use hmac::{Hmac, Mac};
use sha2::Sha256;
use chrono::Utc;
use crate::keys::generate_random_hex;

type HmacSha256 = Hmac<Sha256>;

/// 请求签名
#[derive(Debug, Clone)]
pub struct RequestSignature {
    pub device_id: String,
    pub timestamp: String,
    pub nonce: String,
    pub signature: String,
}

impl RequestSignature {
    pub fn new(device_id: &str, method: &str, path: &str, body: &[u8], secret: &[u8]) -> Self {
        let timestamp = Utc::now().to_rfc3339();
        let nonce = generate_random_hex(16);
        
        // 构建签名原文
        let body_hash = crate::keys::sha256_hex(body);
        let sign_content = format!("{}\n{}\n{}\n{}\n{}", method, path, timestamp, nonce, body_hash);
        
        // 计算HMAC-SHA256
        let mut mac = HmacSha256::new_from_slice(secret)
            .expect("HMAC can take key of any size");
        mac.update(sign_content.as_bytes());
        let signature = hex::encode(mac.finalize().into_bytes());
        
        Self {
            device_id: device_id.to_string(),
            timestamp,
            nonce,
            signature,
        }
    }
    
    pub fn verify(&self, method: &str, path: &str, body: &[u8], secret: &[u8], max_age_seconds: i64) -> bool {
        // 检查时间戳是否在有效期内
        if let Ok(timestamp) = chrono::DateTime::parse_from_rfc3339(&self.timestamp) {
            let age = Utc::now().signed_duration_since(timestamp);
            if age.num_seconds().abs() > max_age_seconds {
                return false;
            }
        } else {
            return false;
        }
        
        // 重新计算签名
        let body_hash = crate::keys::sha256_hex(body);
        let sign_content = format!("{}\n{}\n{}\n{}\n{}", method, path, self.timestamp, self.nonce, body_hash);
        
        let mut mac = HmacSha256::new_from_slice(secret)
            .expect("HMAC can take key of any size");
        mac.update(sign_content.as_bytes());
        let expected_signature = hex::encode(mac.finalize().into_bytes());
        
        // 比较签名
        self.signature == expected_signature
    }
}

/// 生成请求头
pub fn generate_signature_headers(device_id: &str, method: &str, path: &str, body: &[u8], secret: &[u8]) -> Vec<(String, String)> {
    let sig = RequestSignature::new(device_id, method, path, body, secret);
    
    vec![
        ("X-KH-Device-Id".to_string(), sig.device_id),
        ("X-KH-Timestamp".to_string(), sig.timestamp),
        ("X-KH-Nonce".to_string(), sig.nonce),
        ("X-KH-Signature".to_string(), sig.signature),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_signature_creation_and_verification() {
        let secret = b"test-secret-key";
        let device_id = "test-device";
        let method = "POST";
        let path = "/api/test";
        let body = b"test body";
        
        let sig = RequestSignature::new(device_id, method, path, body, secret);
        
        assert!(sig.verify(method, path, body, secret, 300));
        assert!(!sig.verify(method, path, body, b"wrong-secret", 300));
    }
}
