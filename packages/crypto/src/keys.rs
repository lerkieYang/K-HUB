use rand::Rng;
use sha2::{Sha256, Digest};

/// 生成随机字节
pub fn generate_random_bytes(length: usize) -> Vec<u8> {
    let mut rng = rand::thread_rng();
    (0..length).map(|_| rng.gen()).collect()
}

/// 生成随机十六进制字符串
pub fn generate_random_hex(length: usize) -> String {
    let bytes = generate_random_bytes(length / 2);
    hex::encode(bytes)
}

/// 计算SHA256哈希
pub fn sha256(data: &[u8]) -> Vec<u8> {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hasher.finalize().to_vec()
}

/// 计算SHA256哈希（十六进制）
pub fn sha256_hex(data: &[u8]) -> String {
    hex::encode(sha256(data))
}

/// 生成设备指纹
pub fn generate_fingerprint(device_name: &str, os: &str) -> String {
    let data = format!("{}:{}", device_name, os);
    sha256_hex(data.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_random_hex() {
        let hex = generate_random_hex(32);
        assert_eq!(hex.len(), 32);
        assert!(hex.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_sha256() {
        let hash = sha256_hex(b"hello");
        assert_eq!(hash.len(), 64);
    }
}
