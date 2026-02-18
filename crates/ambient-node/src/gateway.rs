use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    net::{IpAddr, SocketAddr},
    path::Path,
    str::FromStr,
    sync::Arc,
    time::Duration,
};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::{TcpListener, TcpStream},
    sync::RwLock,
};
use tracing::{info, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewaySession {
    pub session_id: String,
    pub session_token: String,
    pub egress_profile: String,
    pub destination_policy_id: String,
    pub allowed_destinations: Vec<String>,
    pub expires_at_epoch_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayConfig {
    pub listen_addr: String,
    pub connect_timeout_seconds: u64,
    pub idle_timeout_seconds: u64,
}

impl Default for GatewayConfig {
    fn default() -> Self {
        Self {
            listen_addr: "0.0.0.0:7000".to_string(),
            connect_timeout_seconds: 5,
            idle_timeout_seconds: 600,
        }
    }
}

#[derive(Debug, Clone)]
pub struct DataPlaneGateway {
    config: GatewayConfig,
    sessions: Arc<RwLock<HashMap<String, GatewaySession>>>,
}

#[derive(Debug, Serialize, Deserialize)]
struct HandshakeRequest {
    session_id: String,
    session_token: String,
    destination: String,
}

impl DataPlaneGateway {
    pub fn new(config: GatewayConfig, sessions: Vec<GatewaySession>) -> Self {
        let map = sessions
            .into_iter()
            .map(|session| (session.session_id.clone(), session))
            .collect();
        Self {
            config,
            sessions: Arc::new(RwLock::new(map)),
        }
    }

    pub async fn from_sessions_file(
        config: GatewayConfig,
        sessions_file: impl AsRef<Path>,
    ) -> Result<Self> {
        let data = tokio::fs::read_to_string(sessions_file)
            .await
            .context("failed to read gateway sessions file")?;
        let sessions: Vec<GatewaySession> =
            serde_json::from_str(&data).context("failed to parse gateway sessions JSON")?;
        Ok(Self::new(config, sessions))
    }

    pub async fn run(self) -> Result<()> {
        let listener = TcpListener::bind(&self.config.listen_addr)
            .await
            .with_context(|| {
                format!(
                    "failed to bind gateway listener on {}",
                    self.config.listen_addr
                )
            })?;

        info!(listen_addr = %self.config.listen_addr, "data-plane gateway listening");

        loop {
            let (stream, peer_addr) = listener.accept().await?;
            let gateway = self.clone();
            tokio::spawn(async move {
                if let Err(err) = gateway.handle_connection(stream, peer_addr).await {
                    warn!(%peer_addr, "gateway connection terminated: {err:#}");
                }
            });
        }
    }

    async fn handle_connection(&self, mut stream: TcpStream, peer_addr: SocketAddr) -> Result<()> {
        let idle_timeout = Duration::from_secs(self.config.idle_timeout_seconds);

        let mut reader = BufReader::new(&mut stream);
        let mut handshake_line = String::new();

        tokio::time::timeout(idle_timeout, reader.read_line(&mut handshake_line))
            .await
            .context("handshake timeout")?
            .context("failed to read handshake")?;

        let handshake: HandshakeRequest =
            serde_json::from_str(handshake_line.trim()).context("invalid handshake JSON")?;

        let session = {
            let sessions = self.sessions.read().await;
            sessions
                .get(&handshake.session_id)
                .cloned()
                .context("unknown session_id")?
        };

        validate_session(&session, &handshake.session_token)?;
        validate_destination(&session, &handshake.destination)?;

        let destination = handshake.destination.clone();
        let mut upstream = tokio::time::timeout(
            Duration::from_secs(self.config.connect_timeout_seconds),
            TcpStream::connect(&destination),
        )
        .await
        .context("upstream connect timeout")?
        .with_context(|| format!("failed to connect upstream destination {destination}"))?;

        stream
            .write_all(b"OK\n")
            .await
            .context("failed to send handshake ack")?;

        let bytes_relayed = tokio::time::timeout(
            idle_timeout,
            tokio::io::copy_bidirectional(&mut stream, &mut upstream),
        )
        .await
        .context("relay idle timeout")?
        .context("relay I/O failure")?;

        info!(
            %peer_addr,
            session_id = %session.session_id,
            destination = %destination,
            from_client_bytes = bytes_relayed.0,
            from_upstream_bytes = bytes_relayed.1,
            "relay session completed"
        );

        Ok(())
    }
}

fn validate_session(session: &GatewaySession, provided_token: &str) -> Result<()> {
    if session.session_token != provided_token {
        anyhow::bail!("invalid session token");
    }

    let now = chrono::Utc::now().timestamp() as u64;
    if now >= session.expires_at_epoch_seconds {
        anyhow::bail!("session expired");
    }

    Ok(())
}

fn validate_destination(session: &GatewaySession, destination: &str) -> Result<()> {
    let (host, _port) = split_host_port(destination)?;

    if matches!(session.egress_profile.as_str(), "allowlist_domains") {
        let allowed = session
            .allowed_destinations
            .iter()
            .any(|rule| host_matches_rule(&host, rule));

        if !allowed {
            anyhow::bail!(
                "destination {} is not allowed by policy {}",
                destination,
                session.destination_policy_id
            );
        }
    }

    if host.parse::<IpAddr>().is_ok() && session.egress_profile == "protocol_limited" {
        anyhow::bail!("protocol_limited profile requires DNS hostname destination");
    }

    Ok(())
}

fn split_host_port(destination: &str) -> Result<(String, u16)> {
    if let Ok(socket_addr) = SocketAddr::from_str(destination) {
        return Ok((socket_addr.ip().to_string(), socket_addr.port()));
    }

    let (host, port_str) = destination
        .rsplit_once(':')
        .context("destination must be host:port")?;

    let host = host.trim().to_lowercase();
    if host.is_empty() {
        anyhow::bail!("destination host cannot be empty");
    }

    let port = port_str
        .parse::<u16>()
        .context("destination port must be numeric")?;
    if port == 0 {
        anyhow::bail!("destination port must be > 0");
    }

    Ok((host, port))
}

fn host_matches_rule(host: &str, rule: &str) -> bool {
    let rule = rule.trim().to_lowercase();
    if rule.is_empty() {
        return false;
    }

    if let Some(suffix) = rule.strip_prefix("*.") {
        return host == suffix || host.ends_with(&format!(".{suffix}"));
    }

    host == rule
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    fn sample_session() -> GatewaySession {
        GatewaySession {
            session_id: "sess_123".to_string(),
            session_token: "cs_token".to_string(),
            egress_profile: "allowlist_domains".to_string(),
            destination_policy_id: "policy_web_basic_v1".to_string(),
            allowed_destinations: vec!["127.0.0.1".to_string(), "*.example.com".to_string()],
            expires_at_epoch_seconds: (chrono::Utc::now().timestamp() as u64) + 300,
        }
    }

    #[test]
    fn validates_allowlist_destinations() {
        let session = sample_session();
        assert!(validate_destination(&session, "127.0.0.1:8080").is_ok());
        assert!(validate_destination(&session, "api.example.com:443").is_ok());
        assert!(validate_destination(&session, "evil.com:443").is_err());
    }

    #[tokio::test]
    async fn relays_traffic_end_to_end() {
        let echo_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let echo_addr = echo_listener.local_addr().unwrap();

        tokio::spawn(async move {
            if let Ok((mut socket, _)) = echo_listener.accept().await {
                let mut buf = [0u8; 1024];
                let n = socket.read(&mut buf).await.unwrap();
                socket.write_all(&buf[..n]).await.unwrap();
            }
        });

        let mut session = sample_session();
        session.allowed_destinations = vec!["127.0.0.1".to_string()];

        let gateway = DataPlaneGateway::new(
            GatewayConfig {
                listen_addr: "127.0.0.1:0".to_string(),
                connect_timeout_seconds: 5,
                idle_timeout_seconds: 30,
            },
            vec![session],
        );

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let gateway_addr = listener.local_addr().unwrap();
        drop(listener);

        let run_gateway = DataPlaneGateway {
            config: GatewayConfig {
                listen_addr: gateway_addr.to_string(),
                connect_timeout_seconds: 5,
                idle_timeout_seconds: 30,
            },
            sessions: gateway.sessions.clone(),
        };

        tokio::spawn(async move {
            if let Err(err) = run_gateway.run().await {
                tracing::error!("gateway terminated in test: {err:#}");
            }
        });

        tokio::time::sleep(Duration::from_millis(100)).await;

        let mut client = TcpStream::connect(gateway_addr).await.unwrap();
        let handshake = serde_json::json!({
            "session_id": "sess_123",
            "session_token": "cs_token",
            "destination": format!("127.0.0.1:{}", echo_addr.port()),
        })
        .to_string();

        client.write_all(handshake.as_bytes()).await.unwrap();
        client.write_all(b"\n").await.unwrap();

        let mut ack = [0u8; 3];
        client.read_exact(&mut ack).await.unwrap();
        assert_eq!(&ack, b"OK\n");

        client.write_all(b"hello relay").await.unwrap();
        let mut echoed = vec![0u8; 11];
        client.read_exact(&mut echoed).await.unwrap();
        assert_eq!(&echoed, b"hello relay");
    }
}
