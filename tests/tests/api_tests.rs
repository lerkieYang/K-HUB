use reqwest::Client;
use serde_json::{json, Value};
use uuid::Uuid;

const BASE_URL: &str = "http://127.0.0.1:8443";

#[tokio::test]
async fn test_health_check() {
    let client = Client::new();
    let response = client.get(format!("{}/health", BASE_URL))
        .send()
        .await;
    
    // 如果服务未运行，跳过测试
    if response.is_err() {
        println!("Hub service not running, skipping test");
        return;
    }
    
    let response = response.unwrap();
    assert_eq!(response.status(), 200);
    assert_eq!(response.text().await.unwrap(), "ok");
}

#[tokio::test]
async fn test_workspace_crud() {
    let client = Client::new();
    
    // 创建workspace
    let create_response = client.post(format!("{}/api/workspace", BASE_URL))
        .json(&json!({
            "name": "Test Workspace",
            "mode": "standalone"
        }))
        .send()
        .await;
    
    if create_response.is_err() {
        println!("Hub service not running, skipping test");
        return;
    }
    
    let create_response = create_response.unwrap();
    assert_eq!(create_response.status(), 200);
    
    let workspace: Value = create_response.json().await.unwrap();
    let workspace_id = workspace["id"].as_str().unwrap();
    
    // 获取workspace
    let get_response = client.get(format!("{}/api/workspace/{}", BASE_URL, workspace_id))
        .send()
        .await
        .unwrap();
    
    assert_eq!(get_response.status(), 200);
    let fetched: Value = get_response.json().await.unwrap();
    assert_eq!(fetched["name"], "Test Workspace");
}

#[tokio::test]
async fn test_data_source_crud() {
    let client = Client::new();
    
    // 创建数据源
    let create_response = client.post(format!("{}/api/data-sources", BASE_URL))
        .json(&json!({
            "name": "Test Source",
            "path": "C:\\Users\\test\\Documents",
            "source_type": "local"
        }))
        .send()
        .await;
    
    if create_response.is_err() {
        println!("Hub service not running, skipping test");
        return;
    }
    
    let create_response = create_response.unwrap();
    assert_eq!(create_response.status(), 200);
    
    let source: Value = create_response.json().await.unwrap();
    let source_id = source["id"].as_str().unwrap();
    
    // 获取数据源
    let get_response = client.get(format!("{}/api/data-sources/{}", BASE_URL, source_id))
        .send()
        .await
        .unwrap();
    
    assert_eq!(get_response.status(), 200);
    
    // 列表数据源
    let list_response = client.get(format!("{}/api/data-sources", BASE_URL))
        .send()
        .await
        .unwrap();
    
    assert_eq!(list_response.status(), 200);
    let list: Value = list_response.json().await.unwrap();
    assert!(list["data_sources"].as_array().unwrap().len() > 0);
    
    // 删除数据源
    let delete_response = client.delete(format!("{}/api/data-sources/{}", BASE_URL, source_id))
        .send()
        .await
        .unwrap();
    
    assert_eq!(delete_response.status(), 200);
}

#[tokio::test]
async fn test_memory_crud() {
    let client = Client::new();
    
    // 创建memory
    let create_response = client.post(format!("{}/api/memory", BASE_URL))
        .json(&json!({
            "scope": "project",
            "type": "decision",
            "content": "Test memory content",
            "source": "test"
        }))
        .send()
        .await;
    
    if create_response.is_err() {
        println!("Hub service not running, skipping test");
        return;
    }
    
    let create_response = create_response.unwrap();
    assert_eq!(create_response.status(), 200);
    
    let memory: Value = create_response.json().await.unwrap();
    let memory_id = memory["id"].as_str().unwrap();
    
    // 获取memory
    let get_response = client.get(format!("{}/api/memory/{}", BASE_URL, memory_id))
        .send()
        .await
        .unwrap();
    
    assert_eq!(get_response.status(), 200);
    
    // 搜索memory
    let search_response = client.get(format!("{}/api/memory/search?query=Test", BASE_URL))
        .send()
        .await
        .unwrap();
    
    assert_eq!(search_response.status(), 200);
    let search: Value = search_response.json().await.unwrap();
    assert!(search["results"].as_array().unwrap().len() > 0);
    
    // 删除memory
    let delete_response = client.delete(format!("{}/api/memory/{}", BASE_URL, memory_id))
        .send()
        .await
        .unwrap();
    
    assert_eq!(delete_response.status(), 200);
}

#[tokio::test]
async fn test_memory_candidate_flow() {
    let client = Client::new();
    
    // 提交候选memory
    let submit_response = client.post(format!("{}/api/memory/candidates", BASE_URL))
        .json(&json!({
            "agent_id": "test-agent",
            "candidates": [
                {
                    "scope": "project",
                    "type": "decision",
                    "content": "Candidate memory",
                    "reason": "Test reason",
                    "confidence": 0.8
                }
            ]
        }))
        .send()
        .await;
    
    if submit_response.is_err() {
        println!("Hub service not running, skipping test");
        return;
    }
    
    let submit_response = submit_response.unwrap();
    assert_eq!(submit_response.status(), 200);
    
    let result: Value = submit_response.json().await.unwrap();
    assert!(result["accepted"].as_array().unwrap().len() > 0);
    
    let candidate_id = result["accepted"][0]["candidate_id"].as_str().unwrap();
    
    // 获取待审核列表
    let list_response = client.get(format!("{}/api/memory/candidates", BASE_URL))
        .send()
        .await
        .unwrap();
    
    assert_eq!(list_response.status(), 200);
    
    // 批准候选
    let approve_response = client.post(format!("{}/api/memory/candidates/{}/approve", BASE_URL, candidate_id))
        .json(&json!({
            "review_notes": "Approved"
        }))
        .send()
        .await
        .unwrap();
    
    assert_eq!(approve_response.status(), 200);
    
    let approve_result: Value = approve_response.json().await.unwrap();
    assert!(approve_result["success"].as_bool().unwrap());
    assert!(approve_result["memory_id"].as_str().is_some());
}

#[tokio::test]
async fn test_artifact_crud() {
    let client = Client::new();
    
    // 创建artifact
    let create_response = client.post(format!("{}/api/artifacts", BASE_URL))
        .json(&json!({
            "agent_id": "test-agent",
            "title": "Test Artifact",
            "artifact_type": "document",
            "content": "Test content",
            "mime_type": "text/markdown"
        }))
        .send()
        .await;
    
    if create_response.is_err() {
        println!("Hub service not running, skipping test");
        return;
    }
    
    let create_response = create_response.unwrap();
    assert_eq!(create_response.status(), 200);
    
    let artifact: Value = create_response.json().await.unwrap();
    assert!(artifact["artifact_id"].as_str().is_some());
    
    // 列表artifacts
    let list_response = client.get(format!("{}/api/artifacts", BASE_URL))
        .send()
        .await
        .unwrap();
    
    assert_eq!(list_response.status(), 200);
}

#[tokio::test]
async fn test_context_request() {
    let client = Client::new();
    
    // 请求上下文
    let response = client.post(format!("{}/api/context/request", BASE_URL))
        .json(&json!({
            "agent_id": "test-agent",
            "task": "Test task",
            "need": ["documentation"],
            "max_tokens": 1000
        }))
        .send()
        .await;
    
    if response.is_err() {
        println!("Hub service not running, skipping test");
        return;
    }
    
    let response = response.unwrap();
    assert_eq!(response.status(), 200);
    
    let context: Value = response.json().await.unwrap();
    assert!(context["request_id"].as_str().is_some());
    assert!(context["context_pack"].is_object());
}

#[tokio::test]
async fn test_device_registration_flow() {
    let client = Client::new();
    
    // 创建邀请码
    let invite_response = client.post(format!("{}/api/invites", BASE_URL))
        .json(&json!({
            "ttl_seconds": 600
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
    
    // 注册设备
    let register_response = client.post(format!("{}/api/devices/register", BASE_URL))
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
    let device_id = device["device_id"].as_str().unwrap();
    assert_eq!(device["status"], "pending");
    
    // 批准设备
    let approve_response = client.post(format!("{}/api/devices/{}/approve", BASE_URL, device_id))
        .send()
        .await
        .unwrap();
    
    assert_eq!(approve_response.status(), 200);
    
    let approve: Value = approve_response.json().await.unwrap();
    assert_eq!(approve["status"], "approved");
    assert!(approve["device_token"].as_str().is_some());
}

#[tokio::test]
async fn test_agent_discovery() {
    let client = Client::new();
    
    // 扫描agents
    let scan_response = client.get(format!("{}/api/agents/scan", BASE_URL))
        .send()
        .await;
    
    if scan_response.is_err() {
        println!("Hub service not running, skipping test");
        return;
    }
    
    let scan_response = scan_response.unwrap();
    assert_eq!(scan_response.status(), 200);
    
    let scan: Value = scan_response.json().await.unwrap();
    assert!(scan["agents"].as_array().is_some());
    
    // 配置agent
    let configure_response = client.post(format!("{}/api/agents/configure", BASE_URL))
        .json(&json!({
            "agent_id": "test-agent",
            "inject_mcp": true,
            "install_skill": true,
            "create_output_dir": true
        }))
        .send()
        .await
        .unwrap();
    
    assert_eq!(configure_response.status(), 200);
    
    let configure: Value = configure_response.json().await.unwrap();
    assert!(configure["success"].as_bool().unwrap());
    assert!(configure["token"].as_str().is_some());
}

#[tokio::test]
async fn test_ingest_file_events() {
    let client = Client::new();
    
    // 提交文件事件
    let response = client.post(format!("{}/api/ingest/file-events", BASE_URL))
        .json(&json!({
            "device_id": "test-device",
            "events": [
                {
                    "event_id": Uuid::new_v4().to_string(),
                    "data_source_id": "test-source",
                    "event_type": "file_created",
                    "path": "C:\\Users\\test\\Documents\\test.txt",
                    "size_bytes": 1024,
                    "mtime": chrono::Utc::now().to_rfc3339(),
                    "sha256": "abc123"
                }
            ]
        }))
        .send()
        .await;
    
    if response.is_err() {
        println!("Hub service not running, skipping test");
        return;
    }
    
    let response = response.unwrap();
    assert_eq!(response.status(), 200);
    
    let result: Value = response.json().await.unwrap();
    assert!(result["accepted"].as_array().unwrap().len() > 0);
}

#[tokio::test]
async fn test_discovery_info() {
    let client = Client::new();
    
    // 获取发现信息
    let response = client.get(format!("{}/api/discovery/info", BASE_URL))
        .send()
        .await;
    
    if response.is_err() {
        println!("Hub service not running, skipping test");
        return;
    }
    
    let response = response.unwrap();
    assert_eq!(response.status(), 200);
    
    let info: Value = response.json().await.unwrap();
    assert!(info["hub_name"].as_str().is_some());
    assert!(info["hub_url"].as_str().is_some());
    assert!(info["local_ip"].as_str().is_some());
}
