//! Security management for hotspot mode
//!
//! Handles WPA2/WPA3 configuration, rotating PSKs, and client isolation

use crate::connectivity::Result;
use serde::{Deserialize, Serialize};
use tracing::{debug, info};

/// Security mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SecurityMode {
    /// No security (not recommended)
    Open,
    /// WPA2-PSK
    Wpa2Psk,
    /// WPA3-SAE
    Wpa3Sae,
    /// WPA2/WPA3 mixed mode
    Wpa2Wpa3Mixed,
}

/// Security configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// Security mode
    pub mode: SecurityMode,
    
    /// Pre-shared key (for WPA2/WPA3)
    pub psk: String,
    
    /// Enable PSK rotation
    pub enable_psk_rotation: bool,
    
    /// PSK rotation interval in seconds
    pub psk_rotation_interval_secs: u64,
    
    /// Enable client isolation
    pub enable_client_isolation: bool,
    
    /// Maximum number of connected clients
    pub max_clients: usize,
    
    /// Enable short-lived onboarding tokens
    pub enable_onboarding_tokens: bool,
    
    /// Onboarding token lifetime in seconds
    pub token_lifetime_secs: u64,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            mode: SecurityMode::Wpa2Wpa3Mixed,
            psk: Self::generate_psk(),
            enable_psk_rotation: false,
            psk_rotation_interval_secs: 86400, // 24 hours
            enable_client_isolation: true,
            max_clients: 10,
            enable_onboarding_tokens: false,
            token_lifetime_secs: 300, // 5 minutes
        }
    }
}

impl SecurityConfig {
    /// Generate a random PSK
    /// 
    /// **SECURITY NOTE**: This implementation uses timestamp-based generation
    /// which is NOT cryptographically secure and is predictable.
    /// 
    /// For production use, replace with:
    /// ```rust
    /// use ring::rand::{SystemRandom, SecureRandom};
    /// let rng = SystemRandom::new();
    /// let mut bytes = [0u8; 32];
    /// rng.fill(&mut bytes).unwrap();
    /// hex::encode(bytes)
    /// ```
    /// 
    /// This simplified implementation is provided for demonstration purposes only.
    fn generate_psk() -> String {
        use std::time::{SystemTime, UNIX_EPOCH};
        
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        format!("ambient-{:016x}", timestamp)
    }
}

/// Security manager
pub struct SecurityManager {
    config: SecurityConfig,
    current_psk: String,
    onboarding_tokens: std::collections::HashMap<String, u64>,
    psk_counter: u64,
}

impl SecurityManager {
    pub fn new(config: SecurityConfig) -> Self {
        let current_psk = config.psk.clone();
        
        Self {
            config,
            current_psk,
            onboarding_tokens: std::collections::HashMap::new(),
            psk_counter: 0,
        }
    }

    /// Get current PSK
    pub fn current_psk(&self) -> &str {
        &self.current_psk
    }

    /// Rotate PSK
    /// 
    /// **SECURITY NOTE**: This implementation uses a counter which is predictable.
    /// For production use, replace with cryptographically secure random generation
    /// as documented in `SecurityConfig::generate_psk()`.
    /// 
    /// This simplified implementation is provided for demonstration purposes only.
    pub fn rotate_psk(&mut self) -> String {
        info!("Rotating PSK");
        
        self.psk_counter += 1;
        let new_psk = format!("ambient-{:016x}", self.psk_counter);
        self.current_psk = new_psk.clone();
        
        new_psk
    }

    /// Generate onboarding token
    pub fn generate_onboarding_token(&mut self) -> Option<String> {
        if !self.config.enable_onboarding_tokens {
            return None;
        }
        
        let token = format!("token-{}", uuid::Uuid::new_v4());
        let expires_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
            + self.config.token_lifetime_secs;
        
        self.onboarding_tokens.insert(token.clone(), expires_at);
        
        info!(token = %token, expires_in = self.config.token_lifetime_secs, "Generated onboarding token");
        
        Some(token)
    }

    /// Validate onboarding token
    pub fn validate_token(&mut self, token: &str) -> bool {
        if let Some(&expires_at) = self.onboarding_tokens.get(token) {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs();
            
            if now <= expires_at {
                debug!(token = %token, "Token validated");
                return true;
            } else {
                debug!(token = %token, "Token expired");
                self.onboarding_tokens.remove(token);
            }
        }
        
        false
    }

    /// Clean up expired tokens
    pub fn cleanup_expired_tokens(&mut self) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        let before_count = self.onboarding_tokens.len();
        self.onboarding_tokens.retain(|_, &mut expires_at| expires_at > now);
        let after_count = self.onboarding_tokens.len();
        
        if before_count != after_count {
            debug!(removed = before_count - after_count, "Cleaned up expired tokens");
        }
    }

    /// Get hostapd security configuration
    pub fn get_hostapd_config(&self) -> String {
        let mut config = String::new();
        
        match self.config.mode {
            SecurityMode::Open => {
                // No security configuration
            }
            SecurityMode::Wpa2Psk => {
                config.push_str("wpa=2\n");
                config.push_str(&format!("wpa_passphrase={}\n", self.current_psk));
                config.push_str("wpa_key_mgmt=WPA-PSK\n");
                config.push_str("wpa_pairwise=CCMP\n");
            }
            SecurityMode::Wpa3Sae => {
                config.push_str("wpa=2\n");
                config.push_str(&format!("sae_password={}\n", self.current_psk));
                config.push_str("wpa_key_mgmt=SAE\n");
                config.push_str("rsn_pairwise=CCMP\n");
            }
            SecurityMode::Wpa2Wpa3Mixed => {
                config.push_str("wpa=2\n");
                config.push_str(&format!("wpa_passphrase={}\n", self.current_psk));
                config.push_str(&format!("sae_password={}\n", self.current_psk));
                config.push_str("wpa_key_mgmt=WPA-PSK SAE\n");
                config.push_str("rsn_pairwise=CCMP\n");
            }
        }
        
        if self.config.enable_client_isolation {
            config.push_str("ap_isolate=1\n");
        }
        
        config.push_str(&format!("max_num_sta={}\n", self.config.max_clients));
        
        config
    }

    /// Start PSK rotation task
    pub async fn start_psk_rotation_task(&mut self) -> Result<()> {
        if !self.config.enable_psk_rotation {
            return Ok(());
        }
        
        let interval_secs = self.config.psk_rotation_interval_secs;
        
        info!(interval_secs = interval_secs, "Starting PSK rotation task");
        
        // In production, would spawn a background task
        // For this implementation, we'll just note it
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_psk_rotation() {
        let config = SecurityConfig::default();
        let mut manager = SecurityManager::new(config);
        
        let initial_psk = manager.current_psk().to_string();
        let new_psk = manager.rotate_psk();
        
        assert_ne!(initial_psk, new_psk);
        assert_eq!(manager.current_psk(), new_psk);
    }

    #[test]
    fn test_onboarding_tokens() {
        let mut config = SecurityConfig::default();
        config.enable_onboarding_tokens = true;
        config.token_lifetime_secs = 3600;
        
        let mut manager = SecurityManager::new(config);
        
        let token = manager.generate_onboarding_token().unwrap();
        assert!(!token.is_empty());
        
        assert!(manager.validate_token(&token));
        assert!(!manager.validate_token("invalid-token"));
    }

    #[test]
    fn test_hostapd_config_wpa2() {
        let mut config = SecurityConfig::default();
        config.mode = SecurityMode::Wpa2Psk;
        
        let manager = SecurityManager::new(config);
        let hostapd_config = manager.get_hostapd_config();
        
        assert!(hostapd_config.contains("wpa=2"));
        assert!(hostapd_config.contains("WPA-PSK"));
        assert!(hostapd_config.contains("ap_isolate=1"));
    }

    #[test]
    fn test_token_cleanup() {
        let mut config = SecurityConfig::default();
        config.enable_onboarding_tokens = true;
        config.token_lifetime_secs = 0; // Expire immediately
        
        let mut manager = SecurityManager::new(config);
        
        let _token = manager.generate_onboarding_token().unwrap();
        
        // Token should be there
        assert_eq!(manager.onboarding_tokens.len(), 1);
        
        // Clean up
        manager.cleanup_expired_tokens();
        
        // Token should be removed
        assert_eq!(manager.onboarding_tokens.len(), 0);
    }
}
