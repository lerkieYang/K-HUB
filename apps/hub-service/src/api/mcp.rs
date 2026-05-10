use axum::{Router, Json, extract::State, http::StatusCode, routing::{get, post}};
use serde_json::{json, Value};
use crate::db::Database;
use std::collections::HashMap;
use std::sync::Mutex;
use once_cell::sync::Lazy;

/// Agent 活动追踪：agent_id → 最后调用时间
static AGENT_ACTIVITY: Lazy<Mutex<HashMap<String, String>>> = Lazy::new(|| Mutex::new(HashMap::new()));

/// 规范化 agent ID（去掉 -cli 等后缀，统一为 agent type）
fn normalize_agent_id(client_name: &str) -> String {
    let name = client_name.to_lowercase();
    // 常见的 agent ID 映射
    if name.starts_with("gemini") { return "gemini".to_string(); }
    if name.starts_with("codex") { return "codex".to_string(); }
    if name.starts_with("hermes") { return "hermes".to_string(); }
    if name.starts_with("openclaw") { return "openclaw".to_string(); }
    // 其他情况直接返回
    name
}

/// 已初始化的客户端 → agent_id 映射（按 IP 地址）
static AGENT_IP_MAP: Lazy<Mutex<HashMap<String, String>>> = Lazy::new(|| Mutex::new(HashMap::new()));

fn track_agent_activity(agent_id: &str) {
    let normalized = normalize_agent_id(agent_id);
    if let Ok(mut map) = AGENT_ACTIVITY.lock() {
        map.insert(normalized, chrono::Utc::now().to_rfc3339());
    }
}

fn get_agent_from_ip(ip: &str) -> Option<String> {
    AGENT_IP_MAP.lock().ok()?.get(ip).cloned()
}

fn set_agent_for_ip(ip: &str, agent_id: &str) {
    if let Ok(mut map) = AGENT_IP_MAP.lock() {
        map.insert(ip.to_string(), agent_id.to_string());
    }
}

/// 获取所有 Agent 活动
pub fn get_agent_activity() -> Value {
    if let Ok(map) = AGENT_ACTIVITY.lock() {
        let entries: Value = map.iter().map(|(k, v)| {
            json!({"agent_id": k, "last_seen": v})
        }).collect();
        json!({"agents": entries})
    } else {
        json!({"agents": []})
    }
}

async fn agent_activity_handler() -> Json<Value> {
    Json(get_agent_activity())
}

pub fn routes() -> Router<Database> {
    Router::new()
        .route("/mcp", post(mcp_handler))
        .route("/mcp/tools", get(list_tools_get))
        .route("/mcp/agent-activity", get(agent_activity_handler))
}

/// Escape special LIKE characters (% and _) so they are treated literally.
fn escape_like(s: &str) -> String {
    s.replace('%', "\\%").replace('_', "\\_")
}

/// 获取工具列表
fn get_tools_list() -> Value {
    json!([
        {
            "name": "knowledge.search",
            "description": "Search K-HUB knowledge base. Returns matching records with title, content preview, tags, and source info.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Search query (keywords)"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Max results to return (default: 5)"
                    }
                },
                "required": ["query"]
            }
        },
        {
            "name": "context.request",
            "description": "Request a Context Pack from K-HUB. Returns relevant knowledge, memory, and session context for your current task.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "What you need context for"
                    },
                    "include_knowledge": {
                        "type": "boolean",
                        "description": "Include knowledge base results (default: true)"
                    },
                    "include_memory": {
                        "type": "boolean",
                        "description": "Include agent memory results (default: true)"
                    },
                    "include_sessions": {
                        "type": "boolean",
                        "description": "Include session history (default: false)"
                    }
                },
                "required": ["query"]
            }
        },
        {
            "name": "memory.submit_candidate",
            "description": "Submit a candidate memory to K-HUB for review. Use this to share important findings, decisions, or insights from your work.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "title": {
                        "type": "string",
                        "description": "Memory title"
                    },
                    "content": {
                        "type": "string",
                        "description": "Memory content (the actual knowledge/insight)"
                    },
                    "memory_type": {
                        "type": "string",
                        "description": "Type: semantic, episodic, procedural"
                    },
                    "tags": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Tags for categorization"
                    },
                    "agent_id": {
                        "type": "string",
                        "description": "Your agent identifier (e.g., 'gemini', 'hermes', 'codex')"
                    },
                    "device_id": {
                        "type": "string",
                        "description": "Device identifier (optional)"
                    }
                },
                "required": ["title", "content"]
            }
        },
        {
            "name": "artifact.submit",
            "description": "Submit a work artifact to K-HUB (code, documents, configs, etc.)",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "title": {
                        "type": "string",
                        "description": "Artifact title"
                    },
                    "content": {
                        "type": "string",
                        "description": "Artifact content"
                    },
                    "artifact_type": {
                        "type": "string",
                        "description": "Type: code, document, config, etc."
                    },
                    "source_agent": {
                        "type": "string",
                        "description": "Agent that created this artifact"
                    }
                },
                "required": ["title", "content"]
            }
        },
        {
            "name": "policy.check",
            "description": "Check K-HUB policies and configuration (rate limits, permissions, etc.)",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "check_type": {
                        "type": "string",
                        "description": "What to check: rate_limit, permissions, config"
                    }
                },
                "required": ["check_type"]
            }
        }
    ])
}

/// GET /mcp/tools - 返回工具列表
async fn list_tools_get() -> Json<Value> {
    Json(get_tools_list())
}

/// MCP JSON-RPC handler
async fn mcp_handler(
    State(db): State<Database>,
    Json(request): Json<Value>,
) -> (StatusCode, Json<Value>) {
    let method = request.get("method").and_then(|m| m.as_str()).unwrap_or("");
    let id = request.get("id").cloned();
    
    match method {
        // === 握手阶段 ===
        "initialize" => {
            // 提取 agent 名称并记录
            let client_name = request
                .pointer("/params/clientInfo/name")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            set_agent_for_ip("default", client_name);
            track_agent_activity(client_name);
            
            let client_protocol = request
                .pointer("/params/protocolVersion")
                .and_then(|v| v.as_str())
                .unwrap_or("2024-11-05");
            
            (
                StatusCode::OK,
                Json(json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {
                        "protocolVersion": client_protocol,
                        "capabilities": {
                            "tools": {}
                        },
                        "serverInfo": {
                            "name": "k-hub",
                            "version": "0.1.0"
                        }
                    }
                })),
            )
        },
        
        // === 通知（无需 id，无需响应）===
        "notifications/initialized" => {
            (StatusCode::NO_CONTENT, Json(json!({})))
        },
        
        "notifications/cancelled" => {
            (StatusCode::ACCEPTED, Json(json!({})))
        },
        
        // === 工具列表 ===
        "tools/list" => {
            (
                StatusCode::OK,
                Json(json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {
                        "tools": get_tools_list()
                    }
                })),
            )
        },
        
        // === 工具调用 ===
        "tools/call" => {
            let tool_name = request.pointer("/params/name").and_then(|n| n.as_str()).unwrap_or("");
            let args = request.pointer("/params/arguments").cloned().unwrap_or(json!({}));
            
            // 尝试从参数中获取 agent_id 并记录活动
            let agent_id_from_args = args.get("agent_id").and_then(|v| v.as_str()).map(String::from);
            let agent_id_from_ip = get_agent_from_ip("default");
            let agent_id = agent_id_from_args
                .or(agent_id_from_ip)
                .unwrap_or_else(|| "unknown".to_string());
            track_agent_activity(&agent_id);
            
            let result = match tool_name {
                "knowledge.search" => handle_knowledge_search(&db, &args).await,
                "context.request" => handle_context_request(&db, &args).await,
                "memory.submit_candidate" => handle_memory_submit(&db, &args).await,
                "artifact.submit" => handle_artifact_submit(&db, &args).await,
                "policy.check" => handle_policy_check(&args).await,
                _ => {
                    return (
                        StatusCode::OK,
                        Json(json!({
                            "jsonrpc": "2.0",
                            "id": id,
                            "error": {
                                "code": -32601,
                                "message": format!("Unknown tool: {}", tool_name)
                            }
                        })),
                    );
                }
            };
            
            (
                StatusCode::OK,
                Json(json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {
                        "content": [{
                            "type": "text",
                            "text": serde_json::to_string_pretty(&result).unwrap_or_default()
                        }]
                    }
                })),
            )
        },
        
        _ => {
            (
                StatusCode::OK,
                Json(json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "error": {
                        "code": -32601,
                        "message": format!("Unknown method: {}", method)
                    }
                })),
            )
        }
    }
}

/// knowledge.search 处理函数
async fn handle_knowledge_search(db: &Database, args: &Value) -> Value {
    let query = args.get("query").and_then(|q| q.as_str()).unwrap_or("");
    let limit = args.get("limit").and_then(|l| l.as_i64()).unwrap_or(5) as i64;
    
    let like_query = format!("%{}%", escape_like(query));
    
    let results = sqlx::query_as::<_, (String, String, String, String, String)>(
        r#"SELECT id, title, SUBSTR(content, 1, 300), metadata, source_type
           FROM doc
           WHERE status = 'active'
           AND (title LIKE ? ESCAPE '\' OR content LIKE ? ESCAPE '\' OR metadata LIKE ? ESCAPE '\')
           ORDER BY updated_at DESC LIMIT ?"#,
    )
    .bind(&like_query)
    .bind(&like_query)
    .bind(&like_query)
    .bind(limit)
    .fetch_all(&db.pool)
    .await;
    
    match results {
        Ok(rows) => {
            let items: Vec<Value> = rows.into_iter().map(|(id, title, preview, tags, source)| {
                let tags_vec: Vec<String> = serde_json::from_str(&tags).unwrap_or_default();
                json!({
                    "id": id,
                    "title": title,
                    "preview": preview,
                    "tags": tags_vec,
                    "source": source
                })
            }).collect();
            
            json!({
                "count": items.len(),
                "query": query,
                "results": items
            })
        }
        Err(e) => json!({
            "error": e.to_string(),
            "count": 0,
            "results": []
        })
    }
}

/// context.request 处理函数
async fn handle_context_request(db: &Database, args: &Value) -> Value {
    let query = args.get("query").and_then(|q| q.as_str()).unwrap_or("");
    let include_knowledge = args.get("include_knowledge").and_then(|v| v.as_bool()).unwrap_or(true);
    let include_memory = args.get("include_memory").and_then(|v| v.as_bool()).unwrap_or(true);
    let _include_sessions = args.get("include_sessions").and_then(|v| v.as_bool()).unwrap_or(false);
    
    let like_query = format!("%{}%", escape_like(query));
    let mut context = json!({});
    
    // 获取知识库结果
    if include_knowledge {
        let knowledge_results = sqlx::query_as::<_, (String, String, String)>(
            r#"SELECT title, SUBSTR(content, 1, 500), metadata
               FROM doc
               WHERE status = 'active' AND type = 'knowledge'
               AND (title LIKE ? ESCAPE '\' OR content LIKE ? ESCAPE '\')
               ORDER BY updated_at DESC LIMIT 5"#,
        )
        .bind(&like_query)
        .bind(&like_query)
        .fetch_all(&db.pool)
        .await;
        
        if let Ok(rows) = knowledge_results {
            let items: Vec<Value> = rows.into_iter().map(|(title, content, tags)| {
                let tags_vec: Vec<String> = serde_json::from_str(&tags).unwrap_or_default();
                json!({
                    "title": title,
                    "content": content,
                    "tags": tags_vec
                })
            }).collect();
            context["knowledge"] = json!(items);
        }
    }
    
    // 获取 memory 结果
    if include_memory {
        let memory_results = sqlx::query_as::<_, (String, String, String)>(
            r#"SELECT title, SUBSTR(content, 1, 500), metadata
               FROM doc
               WHERE status = 'active' AND type = 'memory' AND source_type = 'hermes'
               AND (title LIKE ? ESCAPE '\' OR content LIKE ? ESCAPE '\')
               ORDER BY updated_at DESC LIMIT 5"#,
        )
        .bind(&like_query)
        .bind(&like_query)
        .fetch_all(&db.pool)
        .await;
        
        if let Ok(rows) = memory_results {
            let items: Vec<Value> = rows.into_iter().map(|(title, content, tags)| {
                let tags_vec: Vec<String> = serde_json::from_str(&tags).unwrap_or_default();
                json!({
                    "title": title,
                    "content": content,
                    "tags": tags_vec
                })
            }).collect();
            context["memory"] = json!(items);
        }
    }
    
    context
}

/// memory.submit_candidate 处理函数
async fn handle_memory_submit(db: &Database, args: &Value) -> Value {
    let title = args.get("title").and_then(|v| v.as_str()).unwrap_or("");
    let content = args.get("content").and_then(|v| v.as_str()).unwrap_or("");
    let memory_type = args.get("memory_type").and_then(|v| v.as_str()).unwrap_or("semantic");
    let tags = args.get("tags")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect::<Vec<String>>())
        .unwrap_or_default();
    let agent_id = args.get("agent_id").and_then(|v| v.as_str()).unwrap_or("unknown");
    
    if title.is_empty() || content.is_empty() {
        return json!({
            "success": false,
            "error": "title and content are required"
        });
    }
    
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    let tags_json = serde_json::to_string(&tags).unwrap_or_else(|_| "[]".to_string());
    
    let result = sqlx::query(
        r#"INSERT INTO memory_candidate 
           (id, title, content, proposed_type, proposed_scope, reason, source_agent_id, confidence, review_status, created_at)
           VALUES (?, ?, ?, ?, ?, ?, ?, ?, 'needs_review', ?)"#,
    )
    .bind(&id)
    .bind(title)
    .bind(content)
    .bind(memory_type)
    .bind("shared")
    .bind("")
    .bind(agent_id)
    .bind(0.5)
    .bind(&now)
    .execute(&db.pool)
    .await;
    
    match result {
        Ok(_) => json!({
            "success": true,
            "id": id,
            "message": format!("Candidate memory '{}' submitted for review", title)
        }),
        Err(e) => json!({
            "success": false,
            "error": e.to_string()
        })
    }
}

/// artifact.submit 处理函数
async fn handle_artifact_submit(db: &Database, args: &Value) -> Value {
    let title = args.get("title").and_then(|v| v.as_str()).unwrap_or("");
    let content = args.get("content").and_then(|v| v.as_str()).unwrap_or("");
    let artifact_type = args.get("artifact_type").and_then(|v| v.as_str()).unwrap_or("document");
    let source_agent = args.get("source_agent").and_then(|v| v.as_str()).unwrap_or("unknown");
    
    if title.is_empty() || content.is_empty() {
        return json!({
            "success": false,
            "error": "title and content are required"
        });
    }
    
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    
    // 存入 artifact 表
    let result = sqlx::query(
        r#"INSERT INTO artifact (id, title, content, artifact_type, source_agent_id, created_at)"
           VALUES (?, ?, ?, ?, ?, ?)"#,
    )
    .bind(&id)
    .bind(title)
    .bind(content)
    .bind(artifact_type)
    .bind(source_agent)
    .bind(&now)
    .execute(&db.pool)
    .await;
    
    match result {
        Ok(_) => json!({
            "success": true,
            "id": id,
            "message": format!("Artifact '{}' submitted", title)
        }),
        Err(e) => json!({
            "success": false,
            "error": e.to_string()
        })
    }
}

/// policy.check 处理函数
async fn handle_policy_check(args: &Value) -> Value {
    let check_type = args.get("check_type").and_then(|v| v.as_str()).unwrap_or("config");
    
    match check_type {
        "rate_limit" => json!({
            "check_type": "rate_limit",
            "status": "ok",
            "limits": {
                "search_per_minute": 60,
                "submit_per_minute": 10,
                "context_per_minute": 30
            }
        }),
        "permissions" => json!({
            "check_type": "permissions",
            "status": "ok",
            "permissions": ["search", "submit", "context", "artifact"]
        }),
        "config" => json!({
            "check_type": "config",
            "status": "ok",
            "server": "k-hub",
            "version": "0.1.0"
        }),
        _ => json!({
            "check_type": check_type,
            "status": "unknown",
            "message": format!("Unknown check type: {}", check_type)
        })
    }
}
