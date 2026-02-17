use ring::signature::{self};
use serde::{Deserialize, Serialize};
use sha3::{Digest, Sha3_256};
use std::collections::{HashMap, HashSet};
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Protocol {
    Tcp,
    Udp,
    Http,
    Https,
    Quic,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SessionLeaseClaims {
    pub session_id: String,
    pub egress_policy_id: String,
    pub issued_at: u64,
    pub expires_at: u64,
    pub max_duration_secs: u64,
    pub max_bandwidth_mbps: u64,
    pub allowed_protocols: Vec<Protocol>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SessionLease {
    pub key_id: String,
    pub claims: SessionLeaseClaims,
    pub signature: Vec<u8>,
}

impl SessionLease {
    pub fn sign(key_id: impl Into<String>, claims: SessionLeaseClaims, key_pair: &signature::Ed25519KeyPair) -> Self {
        let payload = serde_json::to_vec(&claims).expect("lease claims should serialize");
        let signature = key_pair.sign(&payload).as_ref().to_vec();
        Self {
            key_id: key_id.into(),
            claims,
            signature,
        }
    }

    pub fn verify(&self, verification_key: &[u8], now_epoch_secs: u64) -> bool {
        if now_epoch_secs >= self.claims.expires_at {
            return false;
        }

        let payload = match serde_json::to_vec(&self.claims) {
            Ok(payload) => payload,
            Err(_) => return false,
        };

        signature::UnparsedPublicKey::new(&signature::ED25519, verification_key)
            .verify(&payload, &self.signature)
            .is_ok()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EgressPolicy {
    pub id: String,
    pub allowed_destinations: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct LocalPolicyCache {
    pub allowed_protocols: HashSet<Protocol>,
    pub egress_policies: HashMap<String, EgressPolicy>,
    pub verification_keys: HashMap<String, Vec<u8>>,
    pub lease_metadata: HashMap<String, SessionLeaseClaims>,
    offline_read_only: bool,
}

impl Default for LocalPolicyCache {
    fn default() -> Self {
        Self {
            allowed_protocols: HashSet::new(),
            egress_policies: HashMap::new(),
            verification_keys: HashMap::new(),
            lease_metadata: HashMap::new(),
            offline_read_only: false,
        }
    }
}

impl LocalPolicyCache {
    pub fn set_offline_read_only(&mut self, value: bool) {
        self.offline_read_only = value;
    }

    pub fn upsert_egress_policy(&mut self, policy: EgressPolicy) -> Result<(), &'static str> {
        if self.offline_read_only {
            return Err("cache is read-only in offline mode");
        }
        self.egress_policies.insert(policy.id.clone(), policy);
        Ok(())
    }

    pub fn upsert_verification_key(&mut self, key_id: impl Into<String>, key: Vec<u8>) -> Result<(), &'static str> {
        if self.offline_read_only {
            return Err("cache is read-only in offline mode");
        }
        self.verification_keys.insert(key_id.into(), key);
        Ok(())
    }

    pub fn store_lease_metadata(&mut self, lease: &SessionLease) -> Result<(), &'static str> {
        if self.offline_read_only {
            return Err("cache is read-only in offline mode");
        }
        self.lease_metadata
            .insert(lease.claims.session_id.clone(), lease.claims.clone());
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AuditRecord {
    pub event_type: String,
    pub session_id: String,
    pub details: String,
    pub bytes: u64,
    pub at: u64,
    pub prev_hash: String,
    pub hash: String,
}

#[derive(Debug, Clone)]
pub struct PersistentAuditQueue {
    path: PathBuf,
}

impl PersistentAuditQueue {
    pub fn new(path: impl AsRef<Path>) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
        }
    }

    pub fn append(
        &self,
        event_type: impl Into<String>,
        session_id: impl Into<String>,
        details: impl Into<String>,
        bytes: u64,
        at: u64,
    ) -> std::io::Result<AuditRecord> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }

        let prev_hash = self.last_hash().unwrap_or_else(|| "GENESIS".to_string());
        let event_type = event_type.into();
        let session_id = session_id.into();
        let details = details.into();
        let hash = hash_record(&event_type, &session_id, &details, bytes, at, &prev_hash);

        let record = AuditRecord {
            event_type,
            session_id,
            details,
            bytes,
            at,
            prev_hash,
            hash,
        };

        let mut file = OpenOptions::new().create(true).append(true).open(&self.path)?;
        writeln!(file, "{}", serde_json::to_string(&record).expect("audit serialization"))?;
        Ok(record)
    }

    pub fn verify_chain(&self) -> std::io::Result<bool> {
        let records = self.read_all()?;
        let mut prev = "GENESIS".to_string();
        for rec in records {
            let expected = hash_record(
                &rec.event_type,
                &rec.session_id,
                &rec.details,
                rec.bytes,
                rec.at,
                &prev,
            );
            if rec.prev_hash != prev || rec.hash != expected {
                return Ok(false);
            }
            prev = rec.hash;
        }
        Ok(true)
    }

    pub fn read_all(&self) -> std::io::Result<Vec<AuditRecord>> {
        if !self.path.exists() {
            return Ok(Vec::new());
        }

        let file = OpenOptions::new().read(true).open(&self.path)?;
        let reader = BufReader::new(file);
        let mut output = Vec::new();

        for line in reader.lines() {
            let line = line?;
            let record: AuditRecord = serde_json::from_str(&line)
                .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidData, err))?;
            output.push(record);
        }

        Ok(output)
    }

    pub fn clear(&self) -> std::io::Result<()> {
        if self.path.exists() {
            fs::remove_file(&self.path)?;
        }
        Ok(())
    }

    fn last_hash(&self) -> Option<String> {
        self.read_all()
            .ok()
            .and_then(|records| records.last().map(|r| r.hash.clone()))
    }
}

fn hash_record(event_type: &str, session_id: &str, details: &str, bytes: u64, at: u64, prev_hash: &str) -> String {
    let mut hasher = Sha3_256::new();
    hasher.update(event_type.as_bytes());
    hasher.update(session_id.as_bytes());
    hasher.update(details.as_bytes());
    hasher.update(bytes.to_le_bytes());
    hasher.update(at.to_le_bytes());
    hasher.update(prev_hash.as_bytes());
    format!("{:x}", hasher.finalize())
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum NodeState {
    OnlineControlPlane,
    OfflineControlPlane,
    NoUpstream,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BackhaulPath {
    pub name: String,
    pub is_up: bool,
}

pub trait BackhaulMonitor {
    fn available_paths(&self) -> Vec<BackhaulPath>;
}

#[derive(Debug, Clone)]
pub struct StaticBackhaulMonitor {
    pub paths: Vec<BackhaulPath>,
}

impl BackhaulMonitor for StaticBackhaulMonitor {
    fn available_paths(&self) -> Vec<BackhaulPath> {
        self.paths.clone()
    }
}

#[derive(Debug, Clone)]
pub struct ActiveSession {
    pub lease: SessionLease,
    pub started_at: u64,
    pub last_heartbeat_at: u64,
    pub bytes_sent: u64,
}

#[derive(Debug)]
pub struct LocalSessionManager {
    pub state: NodeState,
    pub cache: LocalPolicyCache,
    pub audit_queue: PersistentAuditQueue,
    sessions: HashMap<String, ActiveSession>,
}

impl LocalSessionManager {
    pub fn new(cache: LocalPolicyCache, audit_queue: PersistentAuditQueue) -> Self {
        Self {
            state: NodeState::OnlineControlPlane,
            cache,
            audit_queue,
            sessions: HashMap::new(),
        }
    }

    pub fn refresh_state<M: BackhaulMonitor>(&mut self, control_plane_reachable: bool, monitor: &M) -> NodeState {
        let any_upstream = monitor.available_paths().iter().any(|path| path.is_up);
        let next_state = if !any_upstream {
            NodeState::NoUpstream
        } else if control_plane_reachable {
            NodeState::OnlineControlPlane
        } else {
            NodeState::OfflineControlPlane
        };

        if self.state != next_state {
            let _ = self.audit_queue.append(
                "node_state_transition",
                "node",
                format!("{:?} -> {:?}", self.state, next_state),
                0,
                now_epoch_secs(),
            );
        }

        self.state = next_state.clone();
        self.cache
            .set_offline_read_only(matches!(self.state, NodeState::OfflineControlPlane | NodeState::NoUpstream));
        next_state
    }

    pub fn activate_session(&mut self, lease: SessionLease, now: u64) -> Result<(), &'static str> {
        let key = self
            .cache
            .verification_keys
            .get(&lease.key_id)
            .ok_or("missing verification key")?;
        if !lease.verify(key, now) {
            return Err("invalid or expired lease");
        }

        let policy = self
            .cache
            .egress_policies
            .get(&lease.claims.egress_policy_id)
            .ok_or("missing egress policy")?;

        if lease
            .claims
            .allowed_protocols
            .iter()
            .any(|protocol| !self.cache.allowed_protocols.contains(protocol))
        {
            return Err("lease protocol exceeds local allowlist");
        }

        if policy.allowed_destinations.is_empty() {
            return Err("egress policy has no destinations");
        }

        self.sessions.insert(
            lease.claims.session_id.clone(),
            ActiveSession {
                lease: lease.clone(),
                started_at: now,
                last_heartbeat_at: now,
                bytes_sent: 0,
            },
        );
        let _ = self
            .audit_queue
            .append("session_activated", lease.claims.session_id, "lease accepted", 0, now);
        Ok(())
    }

    pub fn record_traffic(
        &mut self,
        session_id: &str,
        protocol: Protocol,
        destination: &str,
        bytes: u64,
        now: u64,
    ) -> Result<(), &'static str> {
        if matches!(self.state, NodeState::NoUpstream) {
            return Err("internet egress disabled without upstream");
        }

        let session = self.sessions.get_mut(session_id).ok_or("unknown session")?;

        if now >= session.lease.claims.expires_at {
            self.sessions.remove(session_id);
            return Err("lease expired");
        }

        if now.saturating_sub(session.started_at) > session.lease.claims.max_duration_secs {
            self.sessions.remove(session_id);
            return Err("session duration exceeded lease");
        }

        if !session.lease.claims.allowed_protocols.contains(&protocol) {
            return Err("protocol not allowed by lease");
        }

        let policy = self
            .cache
            .egress_policies
            .get(&session.lease.claims.egress_policy_id)
            .ok_or("missing egress policy")?;
        if !policy
            .allowed_destinations
            .iter()
            .any(|allowed| destination.starts_with(allowed))
        {
            return Err("destination not allowed");
        }

        let elapsed = now.saturating_sub(session.started_at).max(1);
        let max_bytes = session
            .lease
            .claims
            .max_bandwidth_mbps
            .saturating_mul(125_000)
            .saturating_mul(elapsed);
        let projected = session.bytes_sent.saturating_add(bytes);
        if projected > max_bytes {
            return Err("bandwidth exceeded");
        }

        session.bytes_sent = projected;
        let _ = self.audit_queue.append(
            "traffic_metered",
            session_id,
            format!("{:?} {}", protocol, destination),
            bytes,
            now,
        );
        Ok(())
    }

    pub fn heartbeat(&mut self, session_id: &str, now: u64) -> Result<(), &'static str> {
        let session = self.sessions.get_mut(session_id).ok_or("unknown session")?;
        session.last_heartbeat_at = now;
        let _ = self
            .audit_queue
            .append("heartbeat", session_id, "local heartbeat", 0, now);
        Ok(())
    }

    pub fn expire_stale_sessions(&mut self, now: u64, heartbeat_timeout: Duration) {
        let timeout_secs = heartbeat_timeout.as_secs();
        self.sessions.retain(|session_id, session| {
            let lease_live = now < session.lease.claims.expires_at
                && now.saturating_sub(session.started_at) <= session.lease.claims.max_duration_secs;
            let heartbeat_live = now.saturating_sub(session.last_heartbeat_at) <= timeout_secs;
            let keep = lease_live && heartbeat_live;
            if !keep {
                let _ = self.audit_queue.append(
                    "session_terminated",
                    session_id,
                    "lease/heartbeat timeout",
                    session.bytes_sent,
                    now,
                );
            }
            keep
        });
    }

    pub fn reconcile_on_reconnect(&mut self) -> std::io::Result<Vec<AuditRecord>> {
        if !matches!(self.state, NodeState::OnlineControlPlane) {
            return Ok(Vec::new());
        }

        let records = self.audit_queue.read_all()?;
        self.audit_queue.clear()?;
        Ok(records)
    }

    pub fn active_sessions(&self) -> usize {
        self.sessions.len()
    }
}

fn now_epoch_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time should be monotonic after unix epoch")
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;
    use ring::rand::SystemRandom;
    use ring::signature::KeyPair;
    use std::env;

    fn key_pair() -> signature::Ed25519KeyPair {
        let rng = SystemRandom::new();
        let pkcs8 = signature::Ed25519KeyPair::generate_pkcs8(&rng).unwrap();
        signature::Ed25519KeyPair::from_pkcs8(pkcs8.as_ref()).unwrap()
    }

    fn lease_template(expiry_offset: u64) -> SessionLeaseClaims {
        SessionLeaseClaims {
            session_id: "s-1".into(),
            egress_policy_id: "policy-1".into(),
            issued_at: 100,
            expires_at: 100 + expiry_offset,
            max_duration_secs: 300,
            max_bandwidth_mbps: 10,
            allowed_protocols: vec![Protocol::Tcp, Protocol::Https],
        }
    }

    #[test]
    fn lease_signature_verification_works() {
        let kp = key_pair();
        let claims = lease_template(3600);
        let lease = SessionLease::sign("k1", claims.clone(), &kp);

        assert!(lease.verify(kp.public_key().as_ref(), 150));
        assert!(!lease.verify(kp.public_key().as_ref(), claims.expires_at));

        let other = key_pair();
        assert!(!lease.verify(other.public_key().as_ref(), 150));
    }

    #[test]
    fn offline_mode_enforces_without_control_plane() {
        let kp = key_pair();
        let mut cache = LocalPolicyCache::default();
        cache.allowed_protocols.extend([Protocol::Tcp, Protocol::Https]);
        cache
            .upsert_egress_policy(EgressPolicy {
                id: "policy-1".into(),
                allowed_destinations: vec!["https://allowed.example".into()],
            })
            .unwrap();
        cache
            .upsert_verification_key("k1", kp.public_key().as_ref().to_vec())
            .unwrap();

        let audit_path = env::temp_dir().join(format!("audit-{}.log", uuid::Uuid::new_v4()));
        let queue = PersistentAuditQueue::new(audit_path);
        let mut mgr = LocalSessionManager::new(cache, queue);

        let monitor = StaticBackhaulMonitor {
            paths: vec![BackhaulPath {
                name: "lte".into(),
                is_up: true,
            }],
        };
        assert_eq!(mgr.refresh_state(false, &monitor), NodeState::OfflineControlPlane);

        let lease = SessionLease::sign("k1", lease_template(3600), &kp);
        mgr.activate_session(lease, 120).unwrap();
        assert_eq!(mgr.active_sessions(), 1);

        mgr.record_traffic(
            "s-1",
            Protocol::Https,
            "https://allowed.example/path",
            1_000,
            140,
        )
        .unwrap();
        assert!(mgr
            .record_traffic("s-1", Protocol::Udp, "https://allowed.example/path", 1_000, 141)
            .is_err());
        assert!(mgr
            .record_traffic("s-1", Protocol::Https, "https://blocked.example", 1_000, 141)
            .is_err());

        assert!(mgr
            .cache
            .upsert_egress_policy(EgressPolicy {
                id: "policy-2".into(),
                allowed_destinations: vec!["https://new.example".into()],
            })
            .is_err());
    }

    #[test]
    fn reconciliation_after_reconnect_drains_queue() {
        let kp = key_pair();
        let mut cache = LocalPolicyCache::default();
        cache.allowed_protocols.extend([Protocol::Tcp, Protocol::Https]);
        cache
            .upsert_egress_policy(EgressPolicy {
                id: "policy-1".into(),
                allowed_destinations: vec!["https://allowed.example".into()],
            })
            .unwrap();
        cache
            .upsert_verification_key("k1", kp.public_key().as_ref().to_vec())
            .unwrap();

        let audit_path = env::temp_dir().join(format!("audit-{}.log", uuid::Uuid::new_v4()));
        let queue = PersistentAuditQueue::new(audit_path);
        let mut mgr = LocalSessionManager::new(cache, queue.clone());

        let lease = SessionLease::sign("k1", lease_template(3600), &kp);
        mgr.activate_session(lease, 120).unwrap();
        mgr.heartbeat("s-1", 130).unwrap();

        assert!(queue.verify_chain().unwrap());

        let offline = StaticBackhaulMonitor {
            paths: vec![BackhaulPath {
                name: "fiber".into(),
                is_up: true,
            }],
        };
        mgr.refresh_state(false, &offline);
        let online = StaticBackhaulMonitor {
            paths: vec![BackhaulPath {
                name: "fiber".into(),
                is_up: true,
            }],
        };
        mgr.refresh_state(true, &online);

        let records = mgr.reconcile_on_reconnect().unwrap();
        assert!(!records.is_empty());
        assert!(mgr.audit_queue.read_all().unwrap().is_empty());
    }
}
