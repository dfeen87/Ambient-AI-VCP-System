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
    pub fn sign(
        key_id: impl Into<String>,
        claims: SessionLeaseClaims,
        key_pair: &signature::Ed25519KeyPair,
    ) -> Self {
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

#[derive(Debug, Clone, Default)]
pub struct LocalPolicyCache {
    pub allowed_protocols: HashSet<Protocol>,
    pub egress_policies: HashMap<String, EgressPolicy>,
    pub verification_keys: HashMap<String, Vec<u8>>,
    pub lease_metadata: HashMap<String, SessionLeaseClaims>,
    offline_read_only: bool,
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

    pub fn upsert_verification_key(
        &mut self,
        key_id: impl Into<String>,
        key: Vec<u8>,
    ) -> Result<(), &'static str> {
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

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;
        writeln!(
            file,
            "{}",
            serde_json::to_string(&record)
                .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidData, err))?
        )?;
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

fn hash_record(
    event_type: &str,
    session_id: &str,
    details: &str,
    bytes: u64,
    at: u64,
    prev_hash: &str,
) -> String {
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

    pub fn refresh_state<M: BackhaulMonitor>(
        &mut self,
        control_plane_reachable: bool,
        monitor: &M,
    ) -> NodeState {
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
        self.cache.set_offline_read_only(matches!(
            self.state,
            NodeState::OfflineControlPlane | NodeState::NoUpstream
        ));
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
        let _ = self.audit_queue.append(
            "session_activated",
            lease.claims.session_id,
            "lease accepted",
            0,
            now,
        );
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

    /// Export this node's policy cache as a [`PeerPolicySyncMessage`].
    ///
    /// The resulting message can be serialised and sent to peer nodes so that
    /// they can acquire fresh session policies even when the central API
    /// endpoint is unreachable.
    pub fn export_peer_sync(&self, node_id: impl Into<String>) -> PeerPolicySyncMessage {
        PeerPolicySyncMessage::from_cache(node_id, &self.cache)
    }

    /// Import policies from a [`PeerPolicySyncMessage`] received from a peer.
    ///
    /// Policies are merged **non-destructively**: existing local entries are
    /// kept unchanged; only entries absent from the local cache are added.
    /// This ensures that a compromised or stale peer cannot overwrite policies
    /// that were previously verified by the control plane.
    ///
    /// # Security
    /// Always verify that the message originates from a trusted peer before
    /// calling this method.  The integrity hash guards against accidental
    /// corruption but not against intentional forgery.
    ///
    /// # Returns
    /// The number of new egress policies applied, or an error if the
    /// integrity check fails.
    pub fn import_peer_sync(
        &mut self,
        msg: &PeerPolicySyncMessage,
    ) -> Result<usize, &'static str> {
        if !msg.verify_integrity() {
            return Err("peer sync message failed integrity check");
        }

        let mut applied = 0;

        // Merge egress policies — only add entries not already present locally.
        for policy in &msg.egress_policies {
            if !self.cache.egress_policies.contains_key(&policy.id) {
                self.cache
                    .egress_policies
                    .insert(policy.id.clone(), policy.clone());
                applied += 1;
            }
        }

        // Merge verification keys — same non-destructive rule.
        for (key_id, key) in &msg.verification_keys {
            if !self.cache.verification_keys.contains_key(key_id) {
                self.cache
                    .verification_keys
                    .insert(key_id.clone(), key.clone());
            }
        }

        let _ = self.audit_queue.append(
            "peer_sync_applied",
            &msg.sender_node_id,
            format!("{} new policies imported from peer", applied),
            0,
            now_epoch_secs(),
        );

        Ok(applied)
    }

    pub fn active_sessions(&self) -> usize {
        self.sessions.len()
    }
}

/// A signed, serialisable snapshot of a node's policy cache that can be shared
/// with peer nodes when the central API endpoint is unreachable.
///
/// When a node is in [`NodeState::OfflineControlPlane`] or
/// [`NodeState::NoUpstream`] it can still accept policy updates from a trusted
/// peer node by calling [`LocalSessionManager::import_peer_sync`].  This
/// enables the mesh to stay operational and continue routing internet traffic
/// even without a connection to the central control plane.
///
/// # Integrity
/// A SHA3-256 hash of the message content is embedded and verified on receipt.
/// Only use this with messages from nodes you trust — the hash guards against
/// accidental corruption, not adversarial tampering.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PeerPolicySyncMessage {
    /// Identifier of the sending node.
    pub sender_node_id: String,
    /// Egress policies exported from the sender's cache.
    pub egress_policies: Vec<EgressPolicy>,
    /// Verification keys exported from the sender's cache.
    pub verification_keys: HashMap<String, Vec<u8>>,
    /// Unix epoch seconds when this snapshot was taken.
    pub snapshot_at: u64,
    /// SHA3-256 hash of the serialised content (integrity check).
    pub content_hash: String,
}

impl PeerPolicySyncMessage {
    /// Build a sync message from a [`LocalPolicyCache`].
    pub fn from_cache(sender_node_id: impl Into<String>, cache: &LocalPolicyCache) -> Self {
        let sender_node_id = sender_node_id.into();
        let egress_policies: Vec<EgressPolicy> =
            cache.egress_policies.values().cloned().collect();
        let verification_keys = cache.verification_keys.clone();
        let snapshot_at = now_epoch_secs();
        let content_hash = compute_sync_hash(
            &sender_node_id,
            &egress_policies,
            &verification_keys,
            snapshot_at,
        );
        Self {
            sender_node_id,
            egress_policies,
            verification_keys,
            snapshot_at,
            content_hash,
        }
    }

    /// Verify that the embedded hash matches the message content.
    pub fn verify_integrity(&self) -> bool {
        let expected = compute_sync_hash(
            &self.sender_node_id,
            &self.egress_policies,
            &self.verification_keys,
            self.snapshot_at,
        );
        self.content_hash == expected
    }
}

fn compute_sync_hash(
    sender_node_id: &str,
    egress_policies: &[EgressPolicy],
    verification_keys: &HashMap<String, Vec<u8>>,
    snapshot_at: u64,
) -> String {
    let mut hasher = Sha3_256::new();
    hasher.update(sender_node_id.as_bytes());

    // Sort by ID for determinism, then hash the full policy content so that
    // a tampered allowed_destinations list will invalidate the hash.
    let mut policies: Vec<&EgressPolicy> = egress_policies.iter().collect();
    policies.sort_unstable_by_key(|p| p.id.as_str());
    for policy in &policies {
        hasher.update(policy.id.as_bytes());
        for dest in &policy.allowed_destinations {
            hasher.update(dest.as_bytes());
        }
    }

    // Sort by key ID for determinism, then hash the full key material so that
    // substituted key bytes will invalidate the hash.
    let mut keys: Vec<(&str, &Vec<u8>)> =
        verification_keys.iter().map(|(k, v)| (k.as_str(), v)).collect();
    keys.sort_unstable_by_key(|(id, _)| *id);
    for (id, bytes) in &keys {
        hasher.update(id.as_bytes());
        hasher.update(bytes.as_slice());
    }

    hasher.update(snapshot_at.to_le_bytes());
    format!("{:x}", hasher.finalize())
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
        cache
            .allowed_protocols
            .extend([Protocol::Tcp, Protocol::Https]);
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
        assert_eq!(
            mgr.refresh_state(false, &monitor),
            NodeState::OfflineControlPlane
        );

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
            .record_traffic(
                "s-1",
                Protocol::Udp,
                "https://allowed.example/path",
                1_000,
                141
            )
            .is_err());
        assert!(mgr
            .record_traffic(
                "s-1",
                Protocol::Https,
                "https://blocked.example",
                1_000,
                141
            )
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
        cache
            .allowed_protocols
            .extend([Protocol::Tcp, Protocol::Https]);
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

    // -----------------------------------------------------------------------
    // Peer policy sync tests
    // -----------------------------------------------------------------------

    fn make_manager_with_policy(
        policy_id: &str,
        destination: &str,
        key_id: &str,
        kp: &signature::Ed25519KeyPair,
    ) -> LocalSessionManager {
        let mut cache = LocalPolicyCache::default();
        cache
            .allowed_protocols
            .extend([Protocol::Tcp, Protocol::Https]);
        cache
            .upsert_egress_policy(EgressPolicy {
                id: policy_id.into(),
                allowed_destinations: vec![destination.into()],
            })
            .unwrap();
        cache
            .upsert_verification_key(key_id, kp.public_key().as_ref().to_vec())
            .unwrap();
        let audit_path = env::temp_dir().join(format!("audit-{}.log", uuid::Uuid::new_v4()));
        LocalSessionManager::new(cache, PersistentAuditQueue::new(audit_path))
    }

    #[test]
    fn peer_sync_message_integrity_passes() {
        let kp = key_pair();
        let mgr = make_manager_with_policy("p-1", "https://a.example", "k1", &kp);
        let msg = mgr.export_peer_sync("node-A");
        assert!(msg.verify_integrity());
        assert_eq!(msg.sender_node_id, "node-A");
        assert_eq!(msg.egress_policies.len(), 1);
    }

    #[test]
    fn peer_sync_message_integrity_fails_on_tampered_destinations() {
        let kp = key_pair();
        let mgr = make_manager_with_policy("p-1", "https://a.example", "k1", &kp);
        let mut msg = mgr.export_peer_sync("node-A");
        // Tamper: change the allowed destination without updating the hash.
        msg.egress_policies[0].allowed_destinations[0] = "https://evil.example".to_string();
        assert!(
            !msg.verify_integrity(),
            "tampered destination should invalidate integrity hash"
        );
    }

    #[test]
    fn peer_sync_message_integrity_fails_on_tamper() {
        let kp = key_pair();
        let mgr = make_manager_with_policy("p-1", "https://a.example", "k1", &kp);
        let mut msg = mgr.export_peer_sync("node-A");
        // Tamper: swap the hash.
        msg.content_hash = "deadbeef".to_string();
        assert!(!msg.verify_integrity());
    }

    #[test]
    fn import_peer_sync_adds_missing_policies() {
        let kp = key_pair();

        // Peer node has policy "p-peer".
        let peer_mgr = make_manager_with_policy("p-peer", "https://peer.example", "k-peer", &kp);
        let msg = peer_mgr.export_peer_sync("node-peer");

        // Local node has a different policy "p-local".
        let mut local_mgr =
            make_manager_with_policy("p-local", "https://local.example", "k-local", &kp);

        let applied = local_mgr.import_peer_sync(&msg).unwrap();
        assert_eq!(applied, 1, "one new policy should have been imported");

        // Both policies should now be present.
        assert!(local_mgr.cache.egress_policies.contains_key("p-local"));
        assert!(local_mgr.cache.egress_policies.contains_key("p-peer"));
    }

    #[test]
    fn import_peer_sync_does_not_overwrite_existing_policy() {
        let kp = key_pair();

        // Both nodes have the same policy ID but different destinations.
        let peer_mgr = make_manager_with_policy("p-1", "https://peer.example", "k1", &kp);
        let msg = peer_mgr.export_peer_sync("node-peer");

        let mut local_mgr =
            make_manager_with_policy("p-1", "https://local.example", "k1", &kp);

        let applied = local_mgr.import_peer_sync(&msg).unwrap();
        // No new policies added — policy "p-1" already existed locally.
        assert_eq!(applied, 0);

        // Local destination must be preserved.
        let policy = local_mgr.cache.egress_policies.get("p-1").unwrap();
        assert!(policy.allowed_destinations[0].contains("local.example"));
    }

    #[test]
    fn import_peer_sync_rejects_tampered_message() {
        let kp = key_pair();
        let peer_mgr = make_manager_with_policy("p-1", "https://peer.example", "k1", &kp);
        let mut msg = peer_mgr.export_peer_sync("node-peer");
        msg.content_hash = "invalid".to_string();

        let mut local_mgr =
            make_manager_with_policy("p-local", "https://local.example", "k1", &kp);

        assert!(local_mgr.import_peer_sync(&msg).is_err());
    }

    #[test]
    fn import_peer_sync_works_in_offline_control_plane_state() {
        let kp = key_pair();

        let peer_mgr = make_manager_with_policy("p-peer", "https://peer.example", "k1", &kp);
        let msg = peer_mgr.export_peer_sync("node-peer");

        let mut local_mgr =
            make_manager_with_policy("p-local", "https://local.example", "k1", &kp);

        // Transition local node to OfflineControlPlane — the cache becomes read-only
        // for normal API writes, but peer sync must still be able to import.
        let monitor = StaticBackhaulMonitor {
            paths: vec![BackhaulPath {
                name: "lte".into(),
                is_up: true,
            }],
        };
        assert_eq!(
            local_mgr.refresh_state(false, &monitor),
            NodeState::OfflineControlPlane
        );

        // Normal write should be blocked.
        assert!(local_mgr
            .cache
            .upsert_egress_policy(EgressPolicy {
                id: "p-api".into(),
                allowed_destinations: vec![],
            })
            .is_err());

        // But peer sync should succeed.
        let applied = local_mgr.import_peer_sync(&msg).unwrap();
        assert_eq!(applied, 1);
        assert!(local_mgr.cache.egress_policies.contains_key("p-peer"));
    }

    #[test]
    fn peer_sync_audit_trail_is_recorded() {
        let kp = key_pair();
        let peer_mgr = make_manager_with_policy("p-peer", "https://peer.example", "k1", &kp);
        let msg = peer_mgr.export_peer_sync("node-peer");

        let audit_path = env::temp_dir().join(format!("audit-{}.log", uuid::Uuid::new_v4()));
        let queue = PersistentAuditQueue::new(&audit_path);
        let mut local_mgr =
            make_manager_with_policy("p-local", "https://local.example", "k1", &kp);
        // Redirect audit queue so we can inspect it.
        local_mgr.audit_queue = queue.clone();

        local_mgr.import_peer_sync(&msg).unwrap();

        let records = queue.read_all().unwrap();
        assert!(
            records
                .iter()
                .any(|r| r.event_type == "peer_sync_applied"),
            "audit trail should contain a peer_sync_applied record"
        );
    }
}
