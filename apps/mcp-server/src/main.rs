mod tools;
mod auth;

use axum::{routing::post, Router, Json, extract::State, http::HeaderMap};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use reqwest::Client;
use std::net::SocketAddr;

#[derive(Clone)]
struct McpState {
    hub_url: String,
    client: Client,
}

#[derive(Debug, Deserialize)]
struct McpRequest {
    jsonrpc: String,
    id: Value,
    method: String,
    params: Option<Value>,
}

#[derive(Debug, Serialize)]
struct McpResponse {
    jsonrpc: String,
    id: Value,
    result: Option<Value>,
    error: Option<Value>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    
    let hub_url = std::env::var("KH_HUB_URL")
        .unwrap_or_else(|_| "http://127.0.0.1:8443".to_string());
    
    let state = McpState {
        hub_url,
        client: Client::new(),
    };
    
    let app = Router::new()
        .route("/mcp", post(handle_mcp))
        .route("/mcp/tools/list", post(list_tools))
        .route("/mcp/tools/call", post(call_tool))
        .with_state(state);
    
    let addr = SocketAddr::from(([127, 0, 0, 1], 8444));
    tracing::info!("MCP Server listening on {}", addr);
    
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await?;
    
    Ok(())
}

async fn handle_mcp(
    State(state): State<McpState>,
    headers: HeaderMap,
    Json(request): Json<McpRequest>,
) -> Json<McpResponse> {
    // 验证token
    if let Some(auth) = headers.get("authorization") {
        if let Ok(auth_str) = auth.to_str() {
            if !auth_str.starts_with("Bearer ") {
                return Json(McpResponse {
                    jsonrpc: "2.0".to_string(),
                    id: request.id,
                    result: None,
                    error: Some(json!({"code": -32000, "message": "Invalid token"})),
                });
            }
        }
    }
    
    let result = match request.method.as_str() {
        "tools/list" => Some(get_tools_list()),
        "tools/call" => {
            if let Some(params) = request.params {
                Some(call_tool_handler(&state, params).await)
            } else {
                Some(json!({"error": "Missing params"}))
            }
        }
        _ => None,
    };
    
    if let Some(result) = result {
        Json(McpResponse {
            jsonrpc: "2.0".to_string(),
            id: request.id,
            result: Some(result),
            error: None,
        })
    } else {
        Json(McpResponse {
            jsonrpc: "2.0".to_string(),
            id: request.id,
            result: None,
            error: Some(json!({"code": -32601, "message": "Method not found"})),
        })
    }
}

async fn list_tools(State(_): State<McpState>) -> Json<Value> {
    Json(get_tools_list())
}

async fn call_tool(
    State(state): State<McpState>,
    Json(params): Json<Value>,
) -> Json<Value> {
    Json(call_tool_handler(&state, params).await)
}

fn get_tools_list() -> Value {
    json!({
        "tools": [
            {
                "name": "context.request",
                "description": "请求上下文信息，获取相关的memory、文档、模板、规则等",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "task": {"type": "string", "description": "当前任务描述"},
                        "agent_id": {"type": "string", "description": "Agent ID"},
                        "device_id": {"type": "string", "description": "设备ID"},
                        "need": {"type": "array", "items": {"type": "string"}, "description": "需要的信息类型"},
                        "max_tokens": {"type": "integer", "description": "最大token数"}
                    },
                    "required": ["task", "agent_id"]
                }
            },
            {
                "name": "memory.submit_candidate",
                "description": "提交候选memory到审核队列",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "agent_id": {"type": "string", "description": "Agent ID"},
                        "device_id": {"type": "string", "description": "设备ID"},
                        "task_id": {"type": "string", "description": "任务ID"},
                        "candidates": {
                            "type": "array",
                            "items": {
                                "type": "object",
                                "properties": {
                                    "scope": {"type": "string"},
                                    "type": {"type": "string"},
                                    "content": {"type": "string"},
                                    "reason": {"type": "string"},
                                    "confidence": {"type": "number"}
                                },
                                "required": ["scope", "type", "content"]
                            }
                        }
                    },
                    "required": ["agent_id", "candidates"]
                }
            },
            {
                "name": "artifact.submit",
                "description": "提交产出物",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "agent_id": {"type": "string", "description": "Agent ID"},
                        "device_id": {"type": "string", "description": "设备ID"},
                        "title": {"type": "string", "description": "产出标题"},
                        "artifact_type": {"type": "string", "description": "产出类型"},
                        "content": {"type": "string", "description": "产出内容"},
                        "mime_type": {"type": "string", "description": "MIME类型"}
                    },
                    "required": ["agent_id", "title", "artifact_type"]
                }
            },
            {
                "name": "policy.check",
                "description": "检查内容是否符合策略",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "content": {"type": "string", "description": "要检查的内容"},
                        "policy_scope": {"type": "string", "description": "策略范围"},
                        "task_type": {"type": "string", "description": "任务类型"}
                    },
                    "required": ["content"]
                }
            }
        ]
    })
}

async fn call_tool_handler(state: &McpState, params: Value) -> Value {
    let tool_name = params["name"].as_str().unwrap_or_default();
    let arguments = params["arguments"].clone();
    
    match tool_name {
        "context.request" => {
            let url = format!("{}/api/context/request", state.hub_url);
            match state.client.post(&url).json(&arguments).send().await {
                Ok(resp) => resp.json().await.unwrap_or(json!({"error": "Failed to parse response"})),
                Err(e) => json!({"error": e.to_string()}),
            }
        }
        "memory.submit_candidate" => {
            let url = format!("{}/api/memory/candidates", state.hub_url);
            match state.client.post(&url).json(&arguments).send().await {
                Ok(resp) => resp.json().await.unwrap_or(json!({"error": "Failed to parse response"})),
                Err(e) => json!({"error": e.to_string()}),
            }
        }
        "artifact.submit" => {
            let url = format!("{}/api/artifacts", state.hub_url);
            match state.client.post(&url).json(&arguments).send().await {
                Ok(resp) => resp.json().await.unwrap_or(json!({"error": "Failed to parse response"})),
                Err(e) => json!({"error": e.to_string()}),
            }
        }
        "policy.check" => {
            // 简单的策略检查
            let content = arguments["content"].as_str().unwrap_or_default();
            let has_api_key = content.contains("sk-") || content.contains("api_key");
            let has_password = content.contains("password") || content.contains("passwd");
            
            json!({
                "passed": !has_api_key && !has_password,
                "findings": if has_api_key { vec!["Potential API key detected"] } else { vec![] },
                "recommended_changes": []
            })
        }
        _ => json!({"error": format!("Unknown tool: {}", tool_name)}),
    }
}
