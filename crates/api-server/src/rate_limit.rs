/// Rate limiting middleware
///
/// Provides request rate limiting to prevent abuse and ensure fair usage.
/// Uses token bucket algorithm for flexible rate limiting.
use crate::error::ApiError;
use axum::{body::Body, extract::Request, http::StatusCode, middleware::Next, response::Response};
use std::{
    collections::HashMap,
    net::IpAddr,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::Mutex;
use tracing::{debug, warn};

/// Rate limit tier for different endpoint types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RateLimitTier {
    /// Auth endpoints (login, register) - more restrictive
    Auth,
    /// Node registration - moderate limits
    NodeRegistration,
    /// Task submission - moderate limits
    TaskSubmission,
    /// Proof verification - more restrictive (computationally expensive)
    ProofVerification,
    /// General API endpoints
    General,
}

impl RateLimitTier {
    /// Get the rate limit configuration for this tier
    pub fn config(&self) -> (u32, u32) {
        // Returns (requests_per_minute, burst_capacity)
        match self {
            Self::Auth => {
                let rpm = std::env::var("RATE_LIMIT_AUTH_RPM")
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(10); // 10 requests per minute for auth
                let burst = std::env::var("RATE_LIMIT_AUTH_BURST")
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(3); // Burst of 3
                (rpm, burst)
            }
            Self::NodeRegistration => {
                let rpm = std::env::var("RATE_LIMIT_NODE_RPM")
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(20);
                let burst = std::env::var("RATE_LIMIT_NODE_BURST")
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(5);
                (rpm, burst)
            }
            Self::TaskSubmission => {
                let rpm = std::env::var("RATE_LIMIT_TASK_RPM")
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(30);
                let burst = std::env::var("RATE_LIMIT_TASK_BURST")
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(10);
                (rpm, burst)
            }
            Self::ProofVerification => {
                let rpm = std::env::var("RATE_LIMIT_PROOF_RPM")
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(15);
                let burst = std::env::var("RATE_LIMIT_PROOF_BURST")
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(3);
                (rpm, burst)
            }
            Self::General => {
                let rpm = std::env::var("RATE_LIMIT_GENERAL_RPM")
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(60);
                let burst = std::env::var("RATE_LIMIT_GENERAL_BURST")
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(10);
                (rpm, burst)
            }
        }
    }

    /// Determine the tier based on request path
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

/// Rate limiter configuration
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    /// Maximum requests per minute
    pub requests_per_minute: u32,
    /// Burst capacity (additional requests allowed in short bursts)
    pub burst_capacity: u32,
}

impl RateLimitConfig {
    /// Create rate limit config from environment variables
    pub fn from_env() -> Self {
        let requests_per_minute = std::env::var("RATE_LIMIT_REQUESTS_PER_MINUTE")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(60);

        let burst_capacity = std::env::var("RATE_LIMIT_BURST")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(10);

        Self {
            requests_per_minute,
            burst_capacity,
        }
    }

    /// Create config for a specific tier
    pub fn for_tier(tier: RateLimitTier) -> Self {
        let (rpm, burst) = tier.config();
        Self {
            requests_per_minute: rpm,
            burst_capacity: burst,
        }
    }
}

/// Token bucket for rate limiting
#[derive(Debug, Clone)]
struct TokenBucket {
    /// Number of tokens available
    tokens: f64,
    /// Maximum tokens (burst capacity)
    capacity: f64,
    /// Token refill rate per second
    refill_rate: f64,
    /// Last refill timestamp
    last_refill: Instant,
}

impl TokenBucket {
    fn new(requests_per_minute: u32, burst_capacity: u32) -> Self {
        let refill_rate = requests_per_minute as f64 / 60.0; // tokens per second
        let capacity = burst_capacity as f64;

        Self {
            tokens: capacity,
            capacity,
            refill_rate,
            last_refill: Instant::now(),
        }
    }

    /// Try to consume a token, returns true if successful
    fn try_consume(&mut self) -> bool {
        // Refill tokens based on elapsed time
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();

        self.tokens = (self.tokens + elapsed * self.refill_rate).min(self.capacity);
        self.last_refill = now;

        // Try to consume a token
        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            true
        } else {
            false
        }
    }

    /// Get time until next token is available
    fn time_until_next_token(&self) -> Duration {
        if self.tokens >= 1.0 {
            Duration::from_secs(0)
        } else {
            let tokens_needed = 1.0 - self.tokens;
            let seconds = tokens_needed / self.refill_rate;
            Duration::from_secs_f64(seconds.max(1.0))
        }
    }
}

/// Rate limiter state with per-tier buckets
#[derive(Clone)]
pub struct RateLimiter {
    /// Buckets per IP address and tier
    buckets: Arc<Mutex<HashMap<(IpAddr, RateLimitTier), TokenBucket>>>,
}

impl RateLimiter {
    /// Create a new rate limiter
    pub fn new() -> Self {
        Self {
            buckets: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Check if a request should be allowed for a specific tier
    pub async fn check_rate_limit(
        &self,
        ip: IpAddr,
        tier: RateLimitTier,
    ) -> Result<(), (StatusCode, String)> {
        let mut buckets = self.buckets.lock().await;

        let config = RateLimitConfig::for_tier(tier);
        let bucket = buckets
            .entry((ip, tier))
            .or_insert_with(|| TokenBucket::new(config.requests_per_minute, config.burst_capacity));

        if bucket.try_consume() {
            Ok(())
        } else {
            let retry_after = bucket.time_until_next_token();
            Err((
                StatusCode::TOO_MANY_REQUESTS,
                format!(
                    "Rate limit exceeded for {:?} endpoints. Retry after {} seconds",
                    tier,
                    retry_after.as_secs()
                ),
            ))
        }
    }

    /// Cleanup old buckets (should be called periodically)
    pub async fn cleanup(&self) {
        let mut buckets = self.buckets.lock().await;

        // Remove buckets that haven't been used in 10 minutes
        buckets.retain(|_, bucket| bucket.last_refill.elapsed() < Duration::from_secs(600));

        debug!("Rate limiter cleanup: {} active buckets", buckets.len());
    }
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::new()
    }
}

/// Rate limiting middleware with per-tier limits
pub async fn rate_limit_middleware(
    request: Request<Body>,
    next: Next,
) -> Result<Response, ApiError> {
    // Determine the tier based on the request path
    let tier = RateLimitTier::from_path(request.uri().path());

    // Extract IP address from request
    let ip = extract_client_ip(&request)?;

    // Get or create global rate limiter
    static GLOBAL_LIMITER: tokio::sync::OnceCell<Arc<RateLimiter>> =
        tokio::sync::OnceCell::const_new();
    let rate_limiter = GLOBAL_LIMITER
        .get_or_init(|| async { Arc::new(RateLimiter::new()) })
        .await;

    // Check rate limit for this tier
    match rate_limiter.check_rate_limit(ip, tier).await {
        Ok(()) => {
            // Request allowed
            Ok(next.run(request).await)
        }
        Err((status, message)) => {
            // Rate limit exceeded
            warn!("Rate limit exceeded for IP: {} on tier: {:?}", ip, tier);
            Err(ApiError::new("rate_limited", message, status))
        }
    }
}

/// Extract client IP address from request
/// Returns an error if IP cannot be determined
fn extract_client_ip(request: &Request<Body>) -> Result<IpAddr, ApiError> {
    // Try to get IP from X-Forwarded-For header (for reverse proxies)
    if let Some(forwarded) = request.headers().get("x-forwarded-for") {
        if let Ok(forwarded_str) = forwarded.to_str() {
            if let Some(first_ip) = forwarded_str.split(',').next() {
                if let Ok(ip) = first_ip.trim().parse() {
                    return Ok(ip);
                }
            }
        }
    }

    // Try to get IP from X-Real-IP header (used by some proxies)
    if let Some(real_ip) = request.headers().get("x-real-ip") {
        if let Ok(ip_str) = real_ip.to_str() {
            if let Ok(ip) = ip_str.parse() {
                return Ok(ip);
            }
        }
    }

    // Fallback to localhost for development (when headers are not available)
    if cfg!(debug_assertions) {
        warn!("Unable to determine client IP, using localhost fallback");
        return Ok(IpAddr::from([127, 0, 0, 1]));
    }

    // Unable to determine client IP - reject the request in production
    Err(ApiError::bad_request(
        "Unable to determine client IP address. Ensure proper proxy headers are configured.",
    ))
}

/// Start a background task to periodically cleanup old rate limiter buckets
pub async fn start_cleanup_task() {
    static GLOBAL_LIMITER: tokio::sync::OnceCell<Arc<RateLimiter>> =
        tokio::sync::OnceCell::const_new();
    let rate_limiter = GLOBAL_LIMITER
        .get_or_init(|| async { Arc::new(RateLimiter::new()) })
        .await
        .clone();

    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(300)); // Every 5 minutes

        loop {
            interval.tick().await;
            rate_limiter.cleanup().await;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_rate_limiter() {
        let limiter = RateLimiter::new();
        let ip = IpAddr::from([127, 0, 0, 1]);

        // Test Auth tier (10 rpm, burst 3)
        for _ in 0..3 {
            assert!(limiter
                .check_rate_limit(ip, RateLimitTier::Auth)
                .await
                .is_ok());
        }

        // 4th request should fail
        assert!(limiter
            .check_rate_limit(ip, RateLimitTier::Auth)
            .await
            .is_err());
    }

    #[test]
    fn test_tier_from_path() {
        assert_eq!(
            RateLimitTier::from_path("/api/v1/auth/login"),
            RateLimitTier::Auth
        );
        assert_eq!(
            RateLimitTier::from_path("/api/v1/auth/register"),
            RateLimitTier::Auth
        );
        assert_eq!(
            RateLimitTier::from_path("/api/v1/nodes"),
            RateLimitTier::NodeRegistration
        );
        assert_eq!(
            RateLimitTier::from_path("/api/v1/tasks"),
            RateLimitTier::TaskSubmission
        );
        assert_eq!(
            RateLimitTier::from_path("/api/v1/proofs/verify"),
            RateLimitTier::ProofVerification
        );
        assert_eq!(
            RateLimitTier::from_path("/api/v1/health"),
            RateLimitTier::General
        );
    }
}
