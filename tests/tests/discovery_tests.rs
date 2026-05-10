use reqwest::Client;
use serde_json::{json, Value};

/// 测试局域网发现
#[tokio::test]
async fn test_local_discovery() {
    let client = Client::new();
    
    // 尝试访问常见IP的discovery端点
    let common_ips = vec![
        "192.168.1.100",
        "192.168.1.101",
        "192.168.0.100",
        "192.168.0.101",
        "10.0.0.1",
        "127.0.0.1",
    ];
    
    let mut found_hubs = Vec::new();
    
    for ip in common_ips {
        let url = format!("http://{}:8443/api/discovery/info", ip);
        
        match client.get(&url)
            .timeout(std::time::Duration::from_secs(2))
            .send()
            .await
        {
            Ok(response) => {
                if response.status().is_success() {
                    if let Ok(info) = response.json::<Value>().await {
                        println!("Found hub at {}: {:?}", ip, info);
                        found_hubs.push(info);
                    }
                }
            }
            Err(_) => {
                // 忽略连接失败
            }
        }
    }
    
    println!("Total hubs found: {}", found_hubs.len());
}

/// 测试邀请码流程
#[tokio::test]
async fn test_invite_flow() {
    let client = Client::new();
    let base_url = "http://127.0.0.1:8443";
    
    // 1. 创建邀请码
    let invite_response = client.post(format!("{}/api/invites", base_url))
        .json(&json!({
            "ttl_seconds": 600,
            "allowed_role": "client",
            "scopes": ["file_upload", "context_request"]
        }))
        .send()
        .await;
    
    if invite_response.is_err() {
        println!("Hub service not running, skipping test");
        return;
    }
    
    let invite_response = invite_response.unwrap();
    assert_eq!(invite_response.status(), 200);
    
    let invite: Value = invite_response.json().await.unwrap();
    let invite_code = invite["invite_code"].as_str().unwrap();
    
    println!("Invite code: {}", invite_code);
    println!("Expires at: {}", invite["expires_at"]);
    println!("QR payload: {}", invite["qr_payload"]);
    
    // 2. 使用邀请码注册
    let register_response = client.post(format!("{}/api/devices/register", base_url))
        .json(&json!({
            "invite_code": invite_code,
            "device_name": "Test Device",
            "os": "Windows 11",
            "app_version": "0.1.0"
        }))
        .send()
        .await
        .unwrap();
    
    assert_eq!(register_response.status(), 200);
    
    let device: Value = register_response.json().await.unwrap();
    println!("Registered device: {:?}", device);
    
    // 3. 列表设备
    let list_response = client.get(format!("{}/api/devices", base_url))
        .send()
        .await
        .unwrap();
    
    assert_eq!(list_response.status(), 200);
    
    let devices: Value = list_response.json().await.unwrap();
    println!("Total devices: {}", devices["total"]);
}

/// 测试同步API
#[tokio::test]
async fn test_sync_api() {
    let client = Client::new();
    let base_url = "http://127.0.0.1:8443";
    
    // 获取同步状态
    let status_response = client.get(format!("{}/api/sync/status", base_url))
        .send()
        .await;
    
    if status_response.is_err() {
        println!("Hub service not running, skipping test");
        return;
    }
    
    let status_response = status_response.unwrap();
    assert_eq!(status_response.status(), 200);
    
    let status: Value = status_response.json().await.unwrap();
    println!("Sync status: {:?}", status);
    
    // 推送数据
    let push_response = client.post(format!("{}/api/sync/push", base_url))
        .json(&json!({
            "device_id": "test-device",
            "memories": [
                {
                    "id": "mem-001",
                    "scope": "project",
                    "type": "decision",
                    "content": "Test memory from sync",
                    "confidence": 0.9
                }
            ]
        }))
        .send()
        .await
        .unwrap();
    
    assert_eq!(push_response.status(), 200);
    
    let push_result: Value = push_response.json().await.unwrap();
    println!("Sync push result: {:?}", push_result);
    
    // 拉取数据
    let pull_response = client.post(format!("{}/api/sync/pull", base_url))
        .json(&json!({
            "device_id": "test-device",
            "since": "2026-01-01T00:00:00Z"
        }))
        .send()
        .await
        .unwrap();
    
    assert_eq!(pull_response.status(), 200);
    
    let pull_result: Value = pull_response.json().await.unwrap();
    println!("Sync pull result: {:?}", pull_result);
}
