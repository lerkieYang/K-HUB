use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc, Duration};
use crate::keys::generate_random_hex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Token {
    pub token: String,
    pub agent_id: String,
    pub device_id: Option<String>,
    pub permissions: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
}

impl Token {
    pub fn new(agent_id: &str, permissions: Vec<String>) -> Self {
        let token = format!("kh_{}_{}", agent_id, generate_random_hex(16));
        
        Self {
            token,
            agent_id: agent_id.to_string(),
            device_id: None,
            permissions,
            created_at: Utc::now(),
            expires_at: Some(Utc::now() + Duration::days(365)),
        }
    }
    
    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            Utc::now() > expires_at
        } else {
            false
        }
    }
    
    pub fn has_permission(&self, permission: &str) -> bool {
        self.permissions.contains(&permission.to_string())
    }
}

/// 验证token格式
pub fn validate_token_format(token: &str) -> bool {
    token.starts_with("kh_") && token.len() > 10
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_creation() {
        let token = Token::new("test-agent", vec!["context_read".to_string()]);
        assert!(token.token.starts_with("kh_test-agent_"));
        assert!(!token.is_expired());
        assert!(token.has_permission("context_read"));
        assert!(!token.has_permission("admin"));
    }

    #[test]
    fn test_validate_token_format() {
        assert!(validate_token_format("kh_agent_abc123"));
        assert!(!validate_token_format("invalid"));
        assert!(!validate_token_format("kh_"));
    }
}
