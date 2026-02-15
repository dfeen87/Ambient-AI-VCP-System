/// Rate limiting middleware
///
/// Provides request rate limiting to prevent abuse and ensure fair usage.
/// Uses token bucket algorithm for flexible rate limiting.

use crate::error::ApiError;
use axum::{
    body::Body,
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::Response,
};
use std::{
    collections::HashMap,
    net::IpAddr,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::Mutex;

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

/// Rate limiter state
#[derive(Clone)]
pub struct RateLimiter {
    /// Buckets per IP address
    buckets: Arc<Mutex<HashMap<IpAddr, TokenBucket>>>,
    /// Configuration
    config: RateLimitConfig,
}

impl RateLimiter {
    /// Create a new rate limiter
    pub fn new(config: RateLimitConfig) -> Self {
        Self {
            buckets: Arc::new(Mutex::new(HashMap::new())),
            config,
        }
    }
    
    /// Check if a request should be allowed
    pub async fn check_rate_limit(&self, ip: IpAddr) -> Result<(), (StatusCode, String)> {
        let mut buckets = self.buckets.lock().await;
        
        let bucket = buckets
            .entry(ip)
            .or_insert_with(|| TokenBucket::new(
                self.config.requests_per_minute,
                self.config.burst_capacity,
            ));
        
        if bucket.try_consume() {
            Ok(())
        } else {
            let retry_after = bucket.time_until_next_token();
            Err((
                StatusCode::TOO_MANY_REQUESTS,
                format!("Rate limit exceeded. Retry after {} seconds", retry_after.as_secs()),
            ))
        }
    }
    
    /// Cleanup old buckets (should be called periodically)
    pub async fn cleanup(&self) {
        let mut buckets = self.buckets.lock().await;
        
        // Remove buckets that haven't been used in 10 minutes
        buckets.retain(|_, bucket| {
            bucket.last_refill.elapsed() < Duration::from_secs(600)
        });
        
        tracing::debug!("Rate limiter cleanup: {} active buckets", buckets.len());
    }
}

/// Rate limiting middleware
pub async fn rate_limit_middleware(
    request: Request<Body>,
    next: Next,
) -> Result<Response, ApiError> {
    // Extract IP address from request
    let ip = extract_client_ip(&request)?;
    
    // Get rate limiter from extensions
    let rate_limiter = request
        .extensions()
        .get::<RateLimiter>()
        .ok_or_else(|| ApiError::internal_error("Rate limiter not configured"))?
        .clone();
    
    // Check rate limit
    match rate_limiter.check_rate_limit(ip).await {
        Ok(()) => {
            // Request allowed
            Ok(next.run(request).await)
        }
        Err((status, message)) => {
            // Rate limit exceeded
            tracing::warn!("Rate limit exceeded for IP: {}", ip);
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
    
    // Unable to determine client IP - reject the request
    Err(ApiError::bad_request(
        "Unable to determine client IP address. Ensure proper proxy headers are configured."
    ))
}

/// Start a background task to periodically cleanup old rate limiter buckets
pub fn start_cleanup_task(rate_limiter: RateLimiter) {
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
        let config = RateLimitConfig {
            requests_per_minute: 60,
            burst_capacity: 10,
        };
        
        let limiter = RateLimiter::new(config);
        let ip = IpAddr::from([127, 0, 0, 1]);
        
        // First 10 requests should succeed (burst capacity)
        for _ in 0..10 {
            assert!(limiter.check_rate_limit(ip).await.is_ok());
        }
        
        // 11th request should fail
        assert!(limiter.check_rate_limit(ip).await.is_err());
    }
}
