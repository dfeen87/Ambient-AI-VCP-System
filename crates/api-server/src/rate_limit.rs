/// Rate limiting middleware
use crate::error::ApiError;
use axum::{
    body::Body,
    extract::Request,
    http::{HeaderValue, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use ipnet::IpNet;
use std::{
    collections::HashMap,
    net::IpAddr,
    str::FromStr,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::Mutex;
use tracing::{debug, warn};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RateLimitTier {
    Auth,
    NodeRegistration,
    TaskSubmission,
    ProofVerification,
    General,
}

impl RateLimitTier {
    pub fn config(&self) -> (u32, u32) {
        match self {
            Self::Auth => (
                env_u32("RATE_LIMIT_AUTH_RPM", 10),
                env_u32("RATE_LIMIT_AUTH_BURST", 3),
            ),
            Self::NodeRegistration => (
                env_u32("RATE_LIMIT_NODE_RPM", 20),
                env_u32("RATE_LIMIT_NODE_BURST", 5),
            ),
            Self::TaskSubmission => (
                env_u32("RATE_LIMIT_TASK_RPM", 30),
                env_u32("RATE_LIMIT_TASK_BURST", 10),
            ),
            Self::ProofVerification => (
                env_u32("RATE_LIMIT_PROOF_RPM", 15),
                env_u32("RATE_LIMIT_PROOF_BURST", 3),
            ),
            Self::General => (
                env_u32("RATE_LIMIT_GENERAL_RPM", 60),
                env_u32("RATE_LIMIT_GENERAL_BURST", 10),
            ),
        }
    }

    pub fn from_path(path: &str) -> Self {
        if path.contains("/auth/login") || path.contains("/auth/register") {
            Self::Auth
        } else if path.contains("/nodes") && !path.contains("/heartbeat") {
            Self::NodeRegistration
        } else if path.contains("/tasks") {
            Self::TaskSubmission
        } else if path.contains("/proofs/verify") {
            Self::ProofVerification
        } else {
            Self::General
        }
    }
}

fn env_u32(key: &str, default: u32) -> u32 {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

#[derive(Debug, Clone)]
struct TokenBucket {
    tokens: f64,
    capacity: f64,
    refill_rate: f64,
    last_refill: Instant,
}

impl TokenBucket {
    fn new(requests_per_minute: u32, burst_capacity: u32) -> Self {
        Self {
            tokens: burst_capacity as f64,
            capacity: burst_capacity as f64,
            refill_rate: requests_per_minute as f64 / 60.0,
            last_refill: Instant::now(),
        }
    }

    fn try_consume(&mut self) -> bool {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();
        self.tokens = (self.tokens + elapsed * self.refill_rate).min(self.capacity);
        self.last_refill = now;
        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            true
        } else {
            false
        }
    }
}

#[derive(Clone)]
pub struct RateLimiter {
    buckets: Arc<Mutex<HashMap<(IpAddr, RateLimitTier), TokenBucket>>>,
    redis: Option<redis::Client>,
}

impl RateLimiter {
    pub fn new() -> Self {
        let redis = std::env::var("REDIS_URL")
            .ok()
            .and_then(|url| redis::Client::open(url).ok());
        Self {
            buckets: Arc::new(Mutex::new(HashMap::new())),
            redis,
        }
    }

    pub async fn check_rate_limit(
        &self,
        ip: IpAddr,
        tier: RateLimitTier,
    ) -> Result<(), (StatusCode, String)> {
        if self.redis.is_some() {
            return self.check_rate_limit_redis(ip, tier).await;
        }

        let mut buckets = self.buckets.lock().await;
        let (rpm, burst) = tier.config();
        let bucket = buckets
            .entry((ip, tier))
            .or_insert_with(|| TokenBucket::new(rpm, burst));

        if bucket.try_consume() {
            Ok(())
        } else {
            Err((
                StatusCode::TOO_MANY_REQUESTS,
                "Rate limit exceeded".to_string(),
            ))
        }
    }

    async fn check_rate_limit_redis(
        &self,
        ip: IpAddr,
        tier: RateLimitTier,
    ) -> Result<(), (StatusCode, String)> {
        let client = match &self.redis {
            Some(c) => c,
            None => return Ok(()),
        };
        let mut conn = client
            .get_multiplexed_tokio_connection()
            .await
            .map_err(|_| {
                (
                    StatusCode::SERVICE_UNAVAILABLE,
                    "Redis unavailable".to_string(),
                )
            })?;

        let (rpm, burst) = tier.config();
        let key = format!("rl:{}:{:?}", ip, tier);
        let window_secs = 60_u64;
        let script = redis::Script::new(
            r#"
            local current = redis.call('INCR', KEYS[1])
            if current == 1 then
              redis.call('EXPIRE', KEYS[1], ARGV[1])
            end
            if current > tonumber(ARGV[2]) then
              return 0
            end
            return 1
            "#,
        );

        let allowed: i32 = script
            .key(key)
            .arg(window_secs as i32)
            .arg((rpm + burst) as i32)
            .invoke_async(&mut conn)
            .await
            .map_err(|_| {
                (
                    StatusCode::SERVICE_UNAVAILABLE,
                    "Rate limiter backend failed".to_string(),
                )
            })?;

        if allowed == 1 {
            Ok(())
        } else {
            Err((
                StatusCode::TOO_MANY_REQUESTS,
                "Rate limit exceeded".to_string(),
            ))
        }
    }

    pub async fn cleanup(&self) {
        let mut buckets = self.buckets.lock().await;
        buckets.retain(|_, bucket| bucket.last_refill.elapsed() < Duration::from_secs(600));
        debug!("Rate limiter cleanup: {} active buckets", buckets.len());
    }
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::new()
    }
}

static GLOBAL_RATE_LIMITER: tokio::sync::OnceCell<Arc<RateLimiter>> =
    tokio::sync::OnceCell::const_new();

async fn global_rate_limiter() -> &'static Arc<RateLimiter> {
    GLOBAL_RATE_LIMITER
        .get_or_init(|| async { Arc::new(RateLimiter::new()) })
        .await
}

pub async fn rate_limit_middleware(
    request: Request<Body>,
    next: Next,
) -> Result<Response, ApiError> {
    let tier = RateLimitTier::from_path(request.uri().path());
    let ip = extract_client_ip(&request)?;

    // Local node processes can legitimately re-register frequently during startup,
    // integration testing, or rapid restarts. Avoid blocking loopback node
    // registration traffic while preserving limits for all non-loopback clients.
    if tier == RateLimitTier::NodeRegistration && ip.is_loopback() {
        debug!(
            "Skipping node registration rate limit for loopback IP: {}",
            ip
        );
        return Ok(next.run(request).await);
    }

    let rate_limiter = global_rate_limiter().await;

    match rate_limiter.check_rate_limit(ip, tier).await {
        Ok(()) => Ok(next.run(request).await),
        Err((status, message)) => {
            warn!("Rate limit exceeded for IP: {} on tier: {:?}", ip, tier);
            let mut response = ApiError::new("rate_limited", message, status).into_response();
            response
                .headers_mut()
                .insert("Retry-After", HeaderValue::from_static("60"));
            Ok(response)
        }
    }
}

fn trusted_proxy_ranges() -> Vec<IpNet> {
    std::env::var("TRUSTED_PROXY_CIDRS")
        .unwrap_or_default()
        .split(',')
        .filter_map(|cidr| IpNet::from_str(cidr.trim()).ok())
        .collect()
}

fn proxy_is_trusted(remote: IpAddr) -> bool {
    trusted_proxy_ranges()
        .iter()
        .any(|net| net.contains(&remote))
}

fn extract_client_ip(request: &Request<Body>) -> Result<IpAddr, ApiError> {
    let remote_ip = request
        .extensions()
        .get::<std::net::SocketAddr>()
        .map(|s| s.ip())
        .unwrap_or(IpAddr::from([127, 0, 0, 1]));

    if proxy_is_trusted(remote_ip) {
        if let Some(forwarded) = request.headers().get("x-forwarded-for") {
            if let Ok(forwarded_str) = forwarded.to_str() {
                if let Some(first_ip) = forwarded_str.split(',').next() {
                    if let Ok(ip) = first_ip.trim().parse() {
                        return Ok(ip);
                    }
                }
            }
        }

        if let Some(real_ip) = request.headers().get("x-real-ip") {
            if let Ok(ip_str) = real_ip.to_str() {
                if let Ok(ip) = ip_str.parse() {
                    return Ok(ip);
                }
            }
        }
    }

    if remote_ip.is_loopback() || cfg!(debug_assertions) {
        return Ok(remote_ip);
    }

    Err(ApiError::bad_request(
        "Unable to determine trusted client IP address",
    ))
}

pub async fn start_cleanup_task() {
    let rate_limiter = global_rate_limiter().await.clone();

    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(300));
        loop {
            interval.tick().await;
            rate_limiter.cleanup().await;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tier_from_path() {
        assert_eq!(
            RateLimitTier::from_path("/api/v1/auth/login"),
            RateLimitTier::Auth
        );
        assert_eq!(
            RateLimitTier::from_path("/api/v1/proofs/verify"),
            RateLimitTier::ProofVerification
        );
    }

    #[test]
    fn test_proxy_trust() {
        std::env::set_var("TRUSTED_PROXY_CIDRS", "10.0.0.0/8,192.168.0.0/16");
        assert!(proxy_is_trusted(IpAddr::from([10, 1, 2, 3])));
        assert!(!proxy_is_trusted(IpAddr::from([8, 8, 8, 8])));
    }

    /// Verify that the middleware and cleanup task share the same global limiter
    /// instance, so cleanup actually reclaims memory from active rate-limiting state.
    #[tokio::test]
    async fn test_middleware_and_cleanup_share_same_limiter() {
        let limiter_a = global_rate_limiter().await;
        let limiter_b = global_rate_limiter().await;
        assert!(
            Arc::ptr_eq(limiter_a, limiter_b),
            "middleware and cleanup task must share the same RateLimiter instance"
        );
    }
}
