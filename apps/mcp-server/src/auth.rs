use axum::http::HeaderMap;
use reqwest::Client;

/// 验证token有效性
pub async fn validate_token(headers: &HeaderMap, hub_url: &str) -> bool {
    if let Some(auth) = headers.get("authorization") {
        if let Ok(auth_str) = auth.to_str() {
            if auth_str.starts_with("Bearer ") {
                let token = &auth_str[7..];
                
                // 向Hub验证token
                let client = Client::new();
                let response = client
                    .post(format!("{}/api/auth/validate", hub_url))
                    .json(&serde_json::json!({ "token": token }))
                    .send()
                    .await;
                
                match response {
                    Ok(resp) => resp.status().is_success(),
                    Err(_) => false,
                }
            } else {
                false
            }
        } else {
            false
        }
    } else {
        false
    }
}

pub fn extract_agent_id(headers: &HeaderMap) -> Option<String> {
    headers.get("x-agent-id")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
}

pub fn extract_device_id(headers: &HeaderMap) -> Option<String> {
    headers.get("x-device-id")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
}

pub fn extract_token(headers: &HeaderMap) -> Option<String> {
    headers.get("authorization")
        .and_then(|v| v.to_str().ok())
        .filter(|s| s.starts_with("Bearer "))
        .map(|s| s[7..].to_string())
}
