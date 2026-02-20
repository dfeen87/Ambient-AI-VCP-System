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
use tracing::{debug, info, warn};

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

    /// Provision a new session into the gateway's live session store.
    ///
    /// Call this when a connect session is started so the endpoint can
    /// immediately begin relaying traffic through this node.
    pub async fn add_session(&self, session: GatewaySession) {
        let mut sessions = self.sessions.write().await;
        sessions.insert(session.session_id.clone(), session);
    }

    /// Remove a session from the gateway's live session store.
    ///
    /// Call this when a connect session is stopped or expires so the
    /// node stops relaying internet traffic on behalf of the endpoint.
    /// Returns `true` if the session existed and was removed, `false`
    /// if it was not present (already removed or never added).
    pub async fn revoke_session(&self, session_id: &str) -> bool {
        let mut sessions = self.sessions.write().await;
        sessions.remove(session_id).is_some()
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

/// Configuration for the NCSI (Network Connectivity Status Indicator) spoof server.
///
/// When a VCP node provides internet access to connected endpoints, the endpoint's
/// OS may incorrectly report `ERR_INTERNET_DISCONNECTED` because its NCSI probes
/// travel via the broken direct internet path rather than through the VCP tunnel.
///
/// This server listens on a local address and returns valid NCSI-compatible HTTP
/// responses so that OS connectivity checks succeed when routed through the VCP
/// node, preventing false `ERR_INTERNET_DISCONNECTED` errors for connected clients.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NcsiSpoofConfig {
    /// Address and port to listen on for NCSI HTTP probes.
    pub listen_addr: String,
    /// Whether the NCSI spoof server is enabled.
    pub enabled: bool,
}

impl Default for NcsiSpoofConfig {
    fn default() -> Self {
        Self {
            listen_addr: "0.0.0.0:80".to_string(),
            enabled: false,
        }
    }
}

/// Lightweight HTTP server that serves NCSI-compatible responses to prevent
/// false `ERR_INTERNET_DISCONNECTED` errors on clients using this node as their
/// internet gateway.
///
/// Handles the following OS connectivity-check endpoints:
/// - **Windows NCSI**: `GET /connecttest.txt` → `"Microsoft Connect Test"` (HTTP 200)
/// - **Linux NetworkManager/GNOME**: `GET /check_network_status.txt` → `"NetworkManager is online\n"` (HTTP 200)
/// - **Generic / Ubuntu / Apple captive-portal checks**: any other path → HTTP 204 No Content
pub struct NcsiSpoofServer {
    config: NcsiSpoofConfig,
}

impl NcsiSpoofServer {
    pub fn new(config: NcsiSpoofConfig) -> Self {
        Self { config }
    }

    /// Start the NCSI spoof server.  Returns immediately when `config.enabled` is `false`.
    pub async fn run(self) -> Result<()> {
        if !self.config.enabled {
            return Ok(());
        }

        let listener = TcpListener::bind(&self.config.listen_addr)
            .await
            .with_context(|| {
                format!(
                    "failed to bind NCSI spoof server on {}",
                    self.config.listen_addr
                )
            })?;

        info!(listen_addr = %self.config.listen_addr, "NCSI spoof server listening");

        loop {
            let (stream, peer_addr) = listener.accept().await?;
            tokio::spawn(async move {
                if let Err(err) = handle_ncsi_connection(stream).await {
                    warn!(%peer_addr, "NCSI spoof connection error: {err:#}");
                }
            });
        }
    }
}

/// Returns `(HTTP status line, response body)` for a given NCSI request path.
///
/// Covers the main OS connectivity-probe endpoints:
/// - `/connecttest.txt` — Windows NCSI expects the exact string `"Microsoft Connect Test"`
/// - `/check_network_status.txt` — Linux NetworkManager/GNOME connectivity check
/// - all other paths — return HTTP 204 No Content (Ubuntu, Apple captive-portal, etc.)
fn ncsi_response_for_path(path: &str) -> (&'static str, &'static str) {
    match path {
        "/connecttest.txt" => ("200 OK", "Microsoft Connect Test"),
        "/check_network_status.txt" => ("200 OK", "NetworkManager is online\n"),
        _ => ("204 No Content", ""),
    }
}

/// Handle a single NCSI HTTP probe connection.
///
/// Reads the HTTP request line, drains the remaining headers, then writes a
/// minimal HTTP response that satisfies the OS connectivity check.
async fn handle_ncsi_connection(stream: TcpStream) -> Result<()> {
    let (read_half, mut write_half) = stream.into_split();
    let mut reader = BufReader::new(read_half);
    let mut request_line = String::new();

    // Read the HTTP request line with a short timeout so stale connections do
    // not hold resources indefinitely.
    match tokio::time::timeout(Duration::from_secs(5), reader.read_line(&mut request_line)).await {
        Err(_) => {
            debug!("NCSI request timed out; closing connection");
            return Ok(());
        }
        Ok(Err(e)) => {
            debug!("NCSI request read error: {e}; closing connection");
            return Ok(());
        }
        Ok(Ok(0)) => return Ok(()), // EOF before any data
        Ok(Ok(_)) => {}
    }

    // Drain all remaining request headers before sending the response so the
    // HTTP exchange is well-formed and the client reads the full response.
    loop {
        let mut header_line = String::new();
        match tokio::time::timeout(Duration::from_secs(5), reader.read_line(&mut header_line)).await
        {
            Ok(Ok(0)) | Err(_) => break,
            Ok(Err(e)) => {
                debug!("NCSI header drain error: {e}");
                break;
            }
            Ok(Ok(_)) if header_line == "\r\n" || header_line == "\n" => break,
            _ => {}
        }
    }

    // Extract the request path from the first line, e.g. "GET /connecttest.txt HTTP/1.1"
    let path = request_line.split_whitespace().nth(1).unwrap_or("/");

    let (status, body) = ncsi_response_for_path(path);
    let len = body.len();
    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Length: {len}\r\nContent-Type: text/plain\r\nConnection: close\r\n\r\n{body}",
    );

    write_half
        .write_all(response.as_bytes())
        .await
        .context("failed to write NCSI response")?;

    Ok(())
}

/// Configuration for the HTTP CONNECT proxy that allows browsers on offline nodes
/// to route their traffic through a connected relay node, preventing
/// `ERR_INTERNET_DISCONNECTED` errors.
///
/// Configure the browser (or OS) to use `<node-ip>:<listen_port>` as its HTTP
/// proxy.  The browser will issue `CONNECT host:port HTTP/1.1` requests and
/// this server tunnels the underlying TCP stream to the real destination.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpConnectProxyConfig {
    /// Address and port for the proxy to listen on.
    pub listen_addr: String,
    /// Session bearer token required in the `Proxy-Authorization: Bearer <token>`
    /// request header. Set to an empty string to disable authentication
    /// (not recommended outside development).
    pub session_token: String,
    /// Whether the HTTP CONNECT proxy is enabled.
    pub enabled: bool,
    /// Seconds to wait when establishing a connection to the upstream destination.
    pub connect_timeout_secs: u64,
    /// Maximum idle seconds before an established tunnel is torn down.
    pub idle_timeout_secs: u64,
}

impl Default for HttpConnectProxyConfig {
    fn default() -> Self {
        Self {
            listen_addr: "0.0.0.0:3128".to_string(),
            session_token: String::new(),
            enabled: false,
            connect_timeout_secs: 10,
            idle_timeout_secs: 300,
        }
    }
}

/// HTTP CONNECT proxy that enables browsers on offline nodes to tunnel their
/// HTTPS traffic through a connected relay node.
///
/// ## How it works
///
/// 1. The disconnected device's browser is configured to use the relay node as
///    its HTTP proxy (e.g. `Settings → Network → Manual proxy → <relay-ip>:3128`).
/// 2. When the browser opens an HTTPS connection it sends:
///    ```text
///    CONNECT api.example.com:443 HTTP/1.1
///    Proxy-Authorization: Bearer <session_token>
///    ```
/// 3. This server validates the token, opens a TCP connection to
///    `api.example.com:443`, and replies with `200 Connection Established`.
/// 4. Bidirectional I/O is relayed between the browser and the destination so
///    the browser's TLS handshake and all subsequent encrypted traffic pass
///    through unchanged, preventing `ERR_INTERNET_DISCONNECTED`.
///
/// Only the `CONNECT` method is supported.  Unauthenticated plain-HTTP
/// proxying (`GET http://...`) is intentionally not implemented to avoid open
/// relay abuse.
pub struct HttpConnectProxy {
    config: HttpConnectProxyConfig,
}

impl HttpConnectProxy {
    pub fn new(config: HttpConnectProxyConfig) -> Self {
        Self { config }
    }

    /// Start the HTTP CONNECT proxy.  Returns immediately when
    /// `config.enabled` is `false`.
    pub async fn run(self) -> Result<()> {
        if !self.config.enabled {
            return Ok(());
        }

        let listener = TcpListener::bind(&self.config.listen_addr)
            .await
            .with_context(|| {
                format!(
                    "failed to bind HTTP CONNECT proxy on {}",
                    self.config.listen_addr
                )
            })?;

        info!(listen_addr = %self.config.listen_addr, "HTTP CONNECT proxy listening");

        let config = Arc::new(self.config);
        loop {
            let (stream, peer_addr) = listener.accept().await?;
            let cfg = config.clone();
            tokio::spawn(async move {
                if let Err(err) = handle_connect_proxy(stream, cfg).await {
                    warn!(%peer_addr, "HTTP CONNECT proxy connection error: {err:#}");
                }
            });
        }
    }
}

/// Handle one browser connection to the HTTP CONNECT proxy.
async fn handle_connect_proxy(
    stream: TcpStream,
    config: Arc<HttpConnectProxyConfig>,
) -> Result<()> {
    let idle_timeout = Duration::from_secs(config.idle_timeout_secs);
    let (read_half, mut write_half) = stream.into_split();
    let mut reader = BufReader::new(read_half);

    // Read the HTTP request line, e.g. "CONNECT api.example.com:443 HTTP/1.1\r\n"
    let mut request_line = String::new();
    match tokio::time::timeout(idle_timeout, reader.read_line(&mut request_line)).await {
        Err(_) => {
            debug!("HTTP CONNECT request line timed out; closing connection");
            return Ok(());
        }
        Ok(Err(e)) => {
            debug!("HTTP CONNECT request line read error: {e}; closing connection");
            return Ok(());
        }
        Ok(Ok(0)) => return Ok(()), // EOF before any data
        Ok(Ok(_)) => {}
    }

    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or("").to_ascii_uppercase();
    let target = parts.next().unwrap_or("").to_string();

    // Only the CONNECT method is supported.
    if method != "CONNECT" {
        write_half
            .write_all(
                b"HTTP/1.1 405 Method Not Allowed\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
            )
            .await?;
        return Ok(());
    }

    if target.is_empty() {
        write_half
            .write_all(
                b"HTTP/1.1 400 Bad Request\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
            )
            .await?;
        return Ok(());
    }

    // Drain remaining request headers; capture Proxy-Authorization if present.
    let mut proxy_auth: Option<String> = None;
    loop {
        let mut header = String::new();
        match tokio::time::timeout(idle_timeout, reader.read_line(&mut header)).await {
            Ok(Ok(0)) | Err(_) => break,
            Ok(Err(e)) => {
                debug!("HTTP CONNECT header drain error: {e}");
                break;
            }
            Ok(Ok(_)) if header == "\r\n" || header == "\n" => break,
            Ok(Ok(_)) => {
                let lower = header.to_lowercase();
                if let Some(val) = lower.strip_prefix("proxy-authorization:") {
                    proxy_auth = Some(val.trim().to_string());
                }
            }
        }
    }

    // Validate the bearer token when authentication is configured.
    if !config.session_token.is_empty() {
        let provided = proxy_auth
            .as_deref()
            .and_then(|v| v.strip_prefix("bearer "))
            .unwrap_or("");
        if provided != config.session_token {
            write_half
                .write_all(
                    b"HTTP/1.1 407 Proxy Authentication Required\r\nProxy-Authenticate: Bearer\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                )
                .await?;
            return Ok(());
        }
    }

    // Connect to the upstream destination.
    let mut upstream = match tokio::time::timeout(
        Duration::from_secs(config.connect_timeout_secs),
        TcpStream::connect(&target),
    )
    .await
    {
        Ok(Ok(s)) => s,
        Ok(Err(e)) => {
            warn!(target = %target, "HTTP CONNECT upstream connect failed: {e}");
            write_half
                .write_all(
                    b"HTTP/1.1 502 Bad Gateway\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                )
                .await?;
            return Ok(());
        }
        Err(_) => {
            write_half
                .write_all(
                    b"HTTP/1.1 504 Gateway Timeout\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                )
                .await?;
            return Ok(());
        }
    };

    // Inform the browser that the tunnel is established.
    write_half
        .write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n")
        .await
        .context("failed to send 200 Connection Established")?;

    // Reunite the split halves and relay bidirectionally.
    let mut client_stream = reader
        .into_inner()
        .reunite(write_half)
        .context("failed to reunite TCP stream halves")?;

    let bytes_relayed = tokio::time::timeout(
        idle_timeout,
        tokio::io::copy_bidirectional(&mut client_stream, &mut upstream),
    )
    .await
    .context("HTTP CONNECT relay idle timeout")?
    .context("HTTP CONNECT relay I/O failure")?;

    debug!(
        target = %target,
        from_client_bytes = bytes_relayed.0,
        from_upstream_bytes = bytes_relayed.1,
        "HTTP CONNECT tunnel closed"
    );

    Ok(())
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

    #[tokio::test]
    async fn add_session_allows_relay() {
        let gateway = DataPlaneGateway::new(GatewayConfig::default(), vec![]);

        // No sessions initially – unknown session_id should cause a relay failure.
        let sessions_before = gateway.sessions.read().await;
        assert!(sessions_before.is_empty());
        drop(sessions_before);

        // After adding a session the gateway can look it up.
        let session = sample_session();
        gateway.add_session(session.clone()).await;

        let sessions_after = gateway.sessions.read().await;
        assert!(sessions_after.contains_key(&session.session_id));
    }

    #[tokio::test]
    async fn revoke_session_removes_existing() {
        let session = sample_session();
        let gateway = DataPlaneGateway::new(GatewayConfig::default(), vec![session.clone()]);

        // Session present before revocation.
        let before = gateway.sessions.read().await;
        assert!(before.contains_key(&session.session_id));
        drop(before);

        // Revoke returns true and session is gone.
        let removed = gateway.revoke_session(&session.session_id).await;
        assert!(removed);

        let after = gateway.sessions.read().await;
        assert!(!after.contains_key(&session.session_id));
    }

    #[tokio::test]
    async fn revoke_session_returns_false_when_not_present() {
        let gateway = DataPlaneGateway::new(GatewayConfig::default(), vec![]);
        let removed = gateway.revoke_session("nonexistent_session").await;
        assert!(!removed);
    }

    #[tokio::test]
    async fn revoke_session_prevents_relay_after_endpoint_disconnects() {
        // Simulate the lifecycle: session added when endpoint connects, revoked when it leaves.
        let session = sample_session();
        let gateway = DataPlaneGateway::new(GatewayConfig::default(), vec![]);

        gateway.add_session(session.clone()).await;
        assert!(gateway
            .sessions
            .read()
            .await
            .contains_key(&session.session_id));

        // Endpoint disconnects → revoke so the node no longer relays for it.
        let removed = gateway.revoke_session(&session.session_id).await;
        assert!(removed);
        assert!(gateway.sessions.read().await.is_empty());
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

    // -----------------------------------------------------------------------
    // NCSI spoof server tests
    // -----------------------------------------------------------------------

    #[test]
    fn ncsi_response_windows_connecttest() {
        let (status, body) = ncsi_response_for_path("/connecttest.txt");
        assert_eq!(status, "200 OK");
        assert_eq!(body, "Microsoft Connect Test");
    }

    #[test]
    fn ncsi_response_networkmanager_check() {
        let (status, body) = ncsi_response_for_path("/check_network_status.txt");
        assert_eq!(status, "200 OK");
        assert_eq!(body, "NetworkManager is online\n");
    }

    #[test]
    fn ncsi_response_unknown_path_returns_no_content() {
        let (status, body) = ncsi_response_for_path("/");
        assert_eq!(status, "204 No Content");
        assert_eq!(body, "");

        let (status2, body2) = ncsi_response_for_path("/generate_204");
        assert_eq!(status2, "204 No Content");
        assert_eq!(body2, "");
    }

    #[tokio::test]
    async fn ncsi_spoof_server_disabled_exits_immediately() {
        let config = NcsiSpoofConfig {
            listen_addr: "127.0.0.1:0".to_string(),
            enabled: false,
        };
        let server = NcsiSpoofServer::new(config);
        let result = server.run().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn ncsi_spoof_server_serves_windows_ncsi_response() {
        use tokio::io::AsyncReadExt;

        // Bind an ephemeral port to find a free address, then release it so
        // NcsiSpoofServer can bind to it.
        let probe = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = probe.local_addr().unwrap();
        drop(probe);

        let config = NcsiSpoofConfig {
            listen_addr: addr.to_string(),
            enabled: true,
        };

        let server = NcsiSpoofServer::new(config);
        tokio::spawn(async move {
            let _ = server.run().await;
        });

        tokio::time::sleep(Duration::from_millis(100)).await;

        let mut client = TcpStream::connect(addr).await.unwrap();
        let request = b"GET /connecttest.txt HTTP/1.1\r\nHost: www.msftconnecttest.com\r\nConnection: close\r\n\r\n";
        client.write_all(request).await.unwrap();

        let mut response = Vec::new();
        client.read_to_end(&mut response).await.unwrap();
        let response_str = std::str::from_utf8(&response).unwrap();

        assert!(
            response_str.contains("200 OK"),
            "expected HTTP 200, got: {response_str}"
        );
        assert!(
            response_str.contains("Microsoft Connect Test"),
            "expected NCSI body, got: {response_str}"
        );
    }

    #[tokio::test]
    async fn ncsi_spoof_server_serves_networkmanager_response() {
        use tokio::io::AsyncReadExt;

        let probe = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = probe.local_addr().unwrap();
        drop(probe);

        let config = NcsiSpoofConfig {
            listen_addr: addr.to_string(),
            enabled: true,
        };

        let server = NcsiSpoofServer::new(config);
        tokio::spawn(async move {
            let _ = server.run().await;
        });

        tokio::time::sleep(Duration::from_millis(100)).await;

        let mut client = TcpStream::connect(addr).await.unwrap();
        let request = b"GET /check_network_status.txt HTTP/1.1\r\nHost: nmcheck.gnome.org\r\nConnection: close\r\n\r\n";
        client.write_all(request).await.unwrap();

        let mut response = Vec::new();
        client.read_to_end(&mut response).await.unwrap();
        let response_str = std::str::from_utf8(&response).unwrap();

        assert!(
            response_str.contains("200 OK"),
            "expected HTTP 200, got: {response_str}"
        );
        assert!(
            response_str.contains("NetworkManager is online"),
            "expected NM body, got: {response_str}"
        );
    }

    #[tokio::test]
    async fn ncsi_spoof_server_serves_generic_no_content() {
        use tokio::io::AsyncReadExt;

        let probe = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = probe.local_addr().unwrap();
        drop(probe);

        let config = NcsiSpoofConfig {
            listen_addr: addr.to_string(),
            enabled: true,
        };

        let server = NcsiSpoofServer::new(config);
        tokio::spawn(async move {
            let _ = server.run().await;
        });

        tokio::time::sleep(Duration::from_millis(100)).await;

        let mut client = TcpStream::connect(addr).await.unwrap();
        let request =
            b"GET / HTTP/1.1\r\nHost: connectivity-check.ubuntu.com\r\nConnection: close\r\n\r\n";
        client.write_all(request).await.unwrap();

        let mut response = Vec::new();
        client.read_to_end(&mut response).await.unwrap();
        let response_str = std::str::from_utf8(&response).unwrap();

        assert!(
            response_str.contains("204 No Content"),
            "expected HTTP 204, got: {response_str}"
        );
    }

    // -----------------------------------------------------------------------
    // HTTP CONNECT proxy tests
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn http_connect_proxy_disabled_exits_immediately() {
        let config = HttpConnectProxyConfig {
            listen_addr: "127.0.0.1:0".to_string(),
            session_token: String::new(),
            enabled: false,
            connect_timeout_secs: 5,
            idle_timeout_secs: 30,
        };
        let proxy = HttpConnectProxy::new(config);
        let result = proxy.run().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn http_connect_proxy_rejects_non_connect_method() {
        use tokio::io::AsyncReadExt;

        let probe = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = probe.local_addr().unwrap();
        drop(probe);

        let config = HttpConnectProxyConfig {
            listen_addr: addr.to_string(),
            session_token: String::new(),
            enabled: true,
            connect_timeout_secs: 5,
            idle_timeout_secs: 30,
        };
        let proxy = HttpConnectProxy::new(config);
        tokio::spawn(async move {
            let _ = proxy.run().await;
        });
        tokio::time::sleep(Duration::from_millis(50)).await;

        let mut client = TcpStream::connect(addr).await.unwrap();
        client
            .write_all(b"GET http://example.com/ HTTP/1.1\r\nHost: example.com\r\n\r\n")
            .await
            .unwrap();

        let mut response = Vec::new();
        client.read_to_end(&mut response).await.unwrap();
        let response_str = std::str::from_utf8(&response).unwrap();

        assert!(
            response_str.contains("405 Method Not Allowed"),
            "expected 405, got: {response_str}"
        );
    }

    #[tokio::test]
    async fn http_connect_proxy_rejects_missing_auth_token() {
        use tokio::io::AsyncReadExt;

        let probe = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = probe.local_addr().unwrap();
        drop(probe);

        let config = HttpConnectProxyConfig {
            listen_addr: addr.to_string(),
            session_token: "secret-token".to_string(),
            enabled: true,
            connect_timeout_secs: 5,
            idle_timeout_secs: 30,
        };
        let proxy = HttpConnectProxy::new(config);
        tokio::spawn(async move {
            let _ = proxy.run().await;
        });
        tokio::time::sleep(Duration::from_millis(50)).await;

        let mut client = TcpStream::connect(addr).await.unwrap();
        // Send CONNECT without a Proxy-Authorization header.
        client
            .write_all(b"CONNECT example.com:443 HTTP/1.1\r\nHost: example.com\r\n\r\n")
            .await
            .unwrap();

        let mut response = Vec::new();
        client.read_to_end(&mut response).await.unwrap();
        let response_str = std::str::from_utf8(&response).unwrap();

        assert!(
            response_str.contains("407 Proxy Authentication Required"),
            "expected 407, got: {response_str}"
        );
    }

    #[tokio::test]
    async fn http_connect_proxy_rejects_wrong_auth_token() {
        use tokio::io::AsyncReadExt;

        let probe = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = probe.local_addr().unwrap();
        drop(probe);

        let config = HttpConnectProxyConfig {
            listen_addr: addr.to_string(),
            session_token: "secret-token".to_string(),
            enabled: true,
            connect_timeout_secs: 5,
            idle_timeout_secs: 30,
        };
        let proxy = HttpConnectProxy::new(config);
        tokio::spawn(async move {
            let _ = proxy.run().await;
        });
        tokio::time::sleep(Duration::from_millis(50)).await;

        let mut client = TcpStream::connect(addr).await.unwrap();
        client
            .write_all(b"CONNECT example.com:443 HTTP/1.1\r\nProxy-Authorization: Bearer wrong-token\r\n\r\n")
            .await
            .unwrap();

        let mut response = Vec::new();
        client.read_to_end(&mut response).await.unwrap();
        let response_str = std::str::from_utf8(&response).unwrap();

        assert!(
            response_str.contains("407 Proxy Authentication Required"),
            "expected 407, got: {response_str}"
        );
    }

    #[tokio::test]
    async fn http_connect_proxy_tunnels_traffic_end_to_end() {
        use tokio::io::AsyncReadExt;

        // Spawn a simple echo server to act as the upstream destination.
        let echo_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let echo_addr = echo_listener.local_addr().unwrap();
        tokio::spawn(async move {
            if let Ok((mut socket, _)) = echo_listener.accept().await {
                let mut buf = [0u8; 1024];
                let n = socket.read(&mut buf).await.unwrap();
                socket.write_all(&buf[..n]).await.unwrap();
            }
        });

        // Spawn the HTTP CONNECT proxy.
        let probe = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let proxy_addr = probe.local_addr().unwrap();
        drop(probe);

        let config = HttpConnectProxyConfig {
            listen_addr: proxy_addr.to_string(),
            session_token: "proxy-token".to_string(),
            enabled: true,
            connect_timeout_secs: 5,
            idle_timeout_secs: 30,
        };
        let proxy = HttpConnectProxy::new(config);
        tokio::spawn(async move {
            let _ = proxy.run().await;
        });
        tokio::time::sleep(Duration::from_millis(50)).await;

        // Connect as a browser would.
        let mut client = TcpStream::connect(proxy_addr).await.unwrap();
        let connect_request = format!(
            "CONNECT 127.0.0.1:{} HTTP/1.1\r\nProxy-Authorization: Bearer proxy-token\r\nHost: 127.0.0.1\r\n\r\n",
            echo_addr.port()
        );
        client.write_all(connect_request.as_bytes()).await.unwrap();

        // Read the 200 Connection Established response.
        let mut header_buf = [0u8; 256];
        let n = client.read(&mut header_buf).await.unwrap();
        let header_str = std::str::from_utf8(&header_buf[..n]).unwrap();
        assert!(
            header_str.contains("200 Connection Established"),
            "expected 200, got: {header_str}"
        );

        // Send data through the established tunnel and verify the echo.
        client.write_all(b"hello proxy").await.unwrap();
        let mut echoed = vec![0u8; 11];
        client.read_exact(&mut echoed).await.unwrap();
        assert_eq!(&echoed, b"hello proxy");
    }

    #[tokio::test]
    async fn http_connect_proxy_no_auth_required_when_token_empty() {
        use tokio::io::AsyncReadExt;

        // Echo server.
        let echo_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let echo_addr = echo_listener.local_addr().unwrap();
        tokio::spawn(async move {
            if let Ok((mut socket, _)) = echo_listener.accept().await {
                let mut buf = [0u8; 64];
                let n = socket.read(&mut buf).await.unwrap();
                socket.write_all(&buf[..n]).await.unwrap();
            }
        });

        let probe = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let proxy_addr = probe.local_addr().unwrap();
        drop(probe);

        // session_token is empty → no auth check.
        let config = HttpConnectProxyConfig {
            listen_addr: proxy_addr.to_string(),
            session_token: String::new(),
            enabled: true,
            connect_timeout_secs: 5,
            idle_timeout_secs: 30,
        };
        let proxy = HttpConnectProxy::new(config);
        tokio::spawn(async move {
            let _ = proxy.run().await;
        });
        tokio::time::sleep(Duration::from_millis(50)).await;

        let mut client = TcpStream::connect(proxy_addr).await.unwrap();
        let connect_request = format!(
            "CONNECT 127.0.0.1:{} HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n",
            echo_addr.port()
        );
        client.write_all(connect_request.as_bytes()).await.unwrap();

        let mut header_buf = [0u8; 256];
        let n = client.read(&mut header_buf).await.unwrap();
        let header_str = std::str::from_utf8(&header_buf[..n]).unwrap();
        assert!(
            header_str.contains("200 Connection Established"),
            "expected 200 without auth, got: {header_str}"
        );

        client.write_all(b"no-auth").await.unwrap();
        let mut echoed = vec![0u8; 7];
        client.read_exact(&mut echoed).await.unwrap();
        assert_eq!(&echoed, b"no-auth");
    }
}
