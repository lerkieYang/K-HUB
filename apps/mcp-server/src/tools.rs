use serde_json::{json, Value};
use reqwest::Client;

pub async fn context_request(client: &Client, hub_url: &str, params: Value) -> Value {
    let url = format!("{}/api/context/request", hub_url);
    match client.post(&url).json(&params).send().await {
        Ok(resp) => resp.json().await.unwrap_or(json!({"error": "Failed to parse response"})),
        Err(e) => json!({"error": e.to_string()}),
    }
}

pub async fn memory_submit(client: &Client, hub_url: &str, params: Value) -> Value {
    let url = format!("{}/api/memory/candidates", hub_url);
    match client.post(&url).json(&params).send().await {
        Ok(resp) => resp.json().await.unwrap_or(json!({"error": "Failed to parse response"})),
        Err(e) => json!({"error": e.to_string()}),
    }
}

pub async fn artifact_submit(client: &Client, hub_url: &str, params: Value) -> Value {
    let url = format!("{}/api/artifacts", hub_url);
    match client.post(&url).json(&params).send().await {
        Ok(resp) => resp.json().await.unwrap_or(json!({"error": "Failed to parse response"})),
        Err(e) => json!({"error": e.to_string()}),
    }
}

pub fn policy_check(params: Value) -> Value {
    let content = params["content"].as_str().unwrap_or_default();
    let policy_scope = params["policy_scope"].as_str().unwrap_or("general");
    
    let mut findings = Vec::new();
    let mut passed = true;
    
    // 检查敏感信息
    if content.contains("sk-") || content.contains("api_key=") {
        findings.push("Potential API key detected".to_string());
        passed = false;
    }
    
    if content.contains("password=") || content.contains("passwd=") {
        findings.push("Potential password detected".to_string());
        passed = false;
    }
    
    if content.contains("-----BEGIN") {
        findings.push("Private key detected".to_string());
        passed = false;
    }
    
    json!({
        "passed": passed,
        "findings": findings,
        "recommended_changes": []
    })
}
