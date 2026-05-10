use axum::{routing::{get, post}, Router, Json, extract::State};
use serde::{Deserialize};
use serde_json::{json, Value};
use crate::db::Database;

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct SyncPushRequest {
    pub device_id: String,
    pub memories: Option<Vec<Value>>,
    pub artifacts: Option<Vec<Value>>,
    pub file_events: Option<Vec<Value>>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct SyncPullRequest {
    pub device_id: String,
    pub since: Option<String>,
}

pub fn routes() -> Router<Database> {
    Router::new()
        .route("/api/sync/status", get(sync_status))
        .route("/api/sync/push", post(sync_push))
        .route("/api/sync/pull", post(sync_pull))
}

async fn sync_status(
    State(_db): State<Database>,
) -> Json<Value> {
    Json(json!({
        "status": "ready",
        "last_sync": null,
        "pending_items": 0
    }))
}

async fn sync_push(
    State(db): State<Database>,
    Json(req): Json<SyncPushRequest>,
) -> Json<Value> {
    let now = chrono::Utc::now().to_rfc3339();
    let mut synced = 0;
    
    // 同步memories
    if let Some(memories) = req.memories {
        for memory in memories {
            let id = memory["id"].as_str().unwrap_or_default();
            let scope = memory["scope"].as_str().unwrap_or("project");
            let memory_type = memory["memory_type"].as_str()
                .or_else(|| memory["type"].as_str())  // backward compat
                .unwrap_or("semantic");
            let content = memory["content"].as_str().unwrap_or_default();
            let confidence = memory["confidence"].as_f64().unwrap_or(0.5);
            
            // 检查是否已存在
            let exists =
                sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM doc WHERE id = ?")
                    .bind(id)
                    .fetch_one(&db.pool)
                    .await;
            
            if exists.unwrap_or(0) == 0 {
                let insert_result =
                    sqlx::query("INSERT INTO doc (id, workspace_id, content, source_type, status, created_at, updated_at) VALUES (?, 'default', ?, 'sync', 'active', ?, ?)")
                        .bind(id)
                        .bind(content)
                        .bind(&now)
                        .bind(&now)
                        .execute(&db.pool)
                        .await;
                if insert_result.is_ok() {
                    // Insert memory_ext for scope, memory_type, confidence
                    let _ = sqlx::query("INSERT OR IGNORE INTO memory_ext (id, scope, memory_type, confidence, visibility) VALUES (?, ?, ?, ?, 'shared')")
                        .bind(id)
                        .bind(scope)
                        .bind(memory_type)
                        .bind(confidence)
                        .execute(&db.pool)
                        .await;
                    synced += 1;
                }
            }
        }
    }
    
    // 同步artifacts
    if let Some(artifacts) = req.artifacts {
        for artifact in artifacts {
            let id = artifact["id"].as_str().unwrap_or_default();
            let title = artifact["title"].as_str().unwrap_or_default();
            let typ = artifact["artifact_type"].as_str().unwrap_or_default();
            let content = artifact["content"].as_str().unwrap_or_default();
            
            let exists =
                sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM artifact WHERE id = ?")
                    .bind(id)
                    .fetch_one(&db.pool)
                    .await;
            
            if exists.unwrap_or(0) == 0 {
                let insert_result =
                    sqlx::query("INSERT INTO artifact (id, workspace_id, title, artifact_type, content, status, created_at, updated_at) VALUES (?, 'default', ?, ?, ?, 'indexed', ?, ?)")
                        .bind(id)
                        .bind(title)
                        .bind(typ)
                        .bind(content)
                        .bind(&now)
                        .bind(&now)
                        .execute(&db.pool)
                        .await;
                if insert_result.is_ok() {
                    synced += 1;
                }
            }
        }
    }
    
    Json(json!({
        "success": true,
        "synced": synced,
        "timestamp": now
    }))
}

async fn sync_pull(
    State(db): State<Database>,
    Json(req): Json<SyncPullRequest>,
) -> Json<Value> {
    let since = req.since.unwrap_or_else(|| "1970-01-01T00:00:00Z".to_string());
    
    // 获取更新的memories
    let memories =
        sqlx::query_as::<_, (String, String, String, Option<String>, f64, String)>(
            "SELECT d.id, COALESCE(me.scope, 'project'), COALESCE(me.memory_type, 'semantic'), d.content, COALESCE(me.confidence, 0.8), d.updated_at FROM doc d LEFT JOIN memory_ext me ON d.id = me.id WHERE d.status = 'active' AND d.updated_at > ? ORDER BY d.updated_at DESC LIMIT 100"
        )
        .bind(&since)
        .fetch_all(&db.pool)
        .await;
    
    let memory_list: Vec<Value> = memories.unwrap_or_default().into_iter().map(|(id, scope, memory_type, content, confidence, updated_at)| {
        json!({
            "id": id,
            "scope": scope,
            "memory_type": memory_type,
            "content": content,
            "confidence": confidence,
            "updated_at": updated_at
        })
    }).collect();
    
    Json(json!({
        "memories": memory_list,
        "artifacts": [],
        "has_more": false
    }))
}
