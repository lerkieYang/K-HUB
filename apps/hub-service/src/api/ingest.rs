use axum::{routing::post, Router, Json, extract::State};
use serde_json::{json, Value};
use crate::db::Database;
use crate::models::source_file::FileEventBatch;

pub fn routes() -> Router<Database> {
    Router::new()
        .route("/api/ingest/file-events", post(ingest_file_events))
        .route("/api/ingest/artifacts", post(ingest_artifacts))
        .route("/api/ingest/memories", post(ingest_memories))
}

async fn ingest_file_events(
    State(db): State<Database>,
    Json(batch): Json<FileEventBatch>,
) -> Json<Value> {
    let now = chrono::Utc::now().to_rfc3339();
    let mut accepted = Vec::new();
    let mut rejected = Vec::new();
    
    for event in batch.events {
        let id = uuid::Uuid::new_v4().to_string();
        
        let result = if db.is_sqlite {
            sqlx::query("INSERT INTO source_file (id, workspace_id, device_id, data_source_id, path, normalized_path, file_name, extension, size_bytes, mtime, sha256, status, created_at, updated_at) VALUES (?, 'default', ?, ?, ?, ?, ?, ?, ?, ?, ?, 'new', ?, ?)")
                .bind(&id)
                .bind(&batch.device_id)
                .bind(&event.data_source_id)
                .bind(&event.path)
                .bind(&event.path.to_lowercase())
                .bind(std::path::Path::new(&event.path).file_name().unwrap_or_default().to_string_lossy())
                .bind(std::path::Path::new(&event.path).extension().map(|e| e.to_string_lossy().to_string()))
                .bind(event.size_bytes)
                .bind(&event.mtime)
                .bind(&event.sha256)
                .bind(&now)
                .bind(&now)
                .execute(&db.pool)
                .await
        } else {
            sqlx::query("INSERT INTO source_file (id, workspace_id, device_id, data_source_id, path, normalized_path, file_name, extension, size_bytes, mtime, sha256, status, created_at, updated_at) VALUES ($1, 'default', $2, $3, $4, $5, $6, $7, $8, $9, $10, 'new', $11, $12)")
                .bind(&id)
                .bind(&batch.device_id)
                .bind(&event.data_source_id)
                .bind(&event.path)
                .bind(&event.path.to_lowercase())
                .bind(std::path::Path::new(&event.path).file_name().unwrap_or_default().to_string_lossy())
                .bind(std::path::Path::new(&event.path).extension().map(|e| e.to_string_lossy().to_string()))
                .bind(event.size_bytes)
                .bind(&event.mtime)
                .bind(&event.sha256)
                .bind(&now)
                .bind(&now)
                .execute(&db.pool)
                .await
        };
        
        match result {
            Ok(_) => accepted.push(event.event_id),
            Err(_) => rejected.push(event.event_id),
        }
    }
    
    Json(json!({
        "accepted": accepted,
        "rejected": rejected
    }))
}

async fn ingest_artifacts(
    State(db): State<Database>,
    Json(payload): Json<Value>,
) -> Json<Value> {
    let now = chrono::Utc::now().to_rfc3339();
    let mut accepted = Vec::new();
    
    // 提取artifacts数组
    if let Some(artifacts) = payload.get("artifacts").and_then(|a| a.as_array()) {
        for artifact in artifacts {
            let id = uuid::Uuid::new_v4().to_string();
            let agent_id = payload.get("agent_id").and_then(|a| a.as_str()).unwrap_or("unknown");
            let device_id = payload.get("device_id").and_then(|d| d.as_str());
            
            let title = artifact.get("title").and_then(|t| t.as_str()).unwrap_or("Untitled");
            let artifact_type = artifact.get("type").and_then(|t| t.as_str()).unwrap_or("document");
            let content = artifact.get("content").and_then(|c| c.as_str());
            let mime_type = artifact.get("mime_type").and_then(|m| m.as_str());
            
            let result = if db.is_sqlite {
                sqlx::query("INSERT INTO artifact (id, workspace_id, agent_id, device_id, title, artifact_type, content, mime_type, status, created_at, updated_at) VALUES (?, 'default', ?, ?, ?, ?, ?, ?, 'submitted', ?, ?)")
                    .bind(&id)
                    .bind(agent_id)
                    .bind(device_id)
                    .bind(title)
                    .bind(artifact_type)
                    .bind(content)
                    .bind(mime_type)
                    .bind(&now)
                    .bind(&now)
                    .execute(&db.pool)
                    .await
            } else {
                sqlx::query("INSERT INTO artifact (id, workspace_id, agent_id, device_id, title, artifact_type, content, mime_type, status, created_at, updated_at) VALUES ($1, 'default', $2, $3, $4, $5, $6, $7, 'submitted', $8, $9)")
                    .bind(&id)
                    .bind(agent_id)
                    .bind(device_id)
                    .bind(title)
                    .bind(artifact_type)
                    .bind(content)
                    .bind(mime_type)
                    .bind(&now)
                    .bind(&now)
                    .execute(&db.pool)
                    .await
            };
            
            if result.is_ok() {
                accepted.push(id);
            }
        }
    } else {
        // 单个artifact
        let id = uuid::Uuid::new_v4().to_string();
        let agent_id = payload.get("agent_id").and_then(|a| a.as_str()).unwrap_or("unknown");
        let device_id = payload.get("device_id").and_then(|d| d.as_str());
        let title = payload.get("title").and_then(|t| t.as_str()).unwrap_or("Untitled");
        let artifact_type = payload.get("artifact_type").and_then(|t| t.as_str()).unwrap_or("document");
        let content = payload.get("content").and_then(|c| c.as_str());
        let mime_type = payload.get("mime_type").and_then(|m| m.as_str());
        
        let result = if db.is_sqlite {
            sqlx::query("INSERT INTO artifact (id, workspace_id, agent_id, device_id, title, artifact_type, content, mime_type, status, created_at, updated_at) VALUES (?, 'default', ?, ?, ?, ?, ?, ?, 'submitted', ?, ?)")
                .bind(&id)
                .bind(agent_id)
                .bind(device_id)
                .bind(title)
                .bind(artifact_type)
                .bind(content)
                .bind(mime_type)
                .bind(&now)
                .bind(&now)
                .execute(&db.pool)
                .await
        } else {
            sqlx::query("INSERT INTO artifact (id, workspace_id, agent_id, device_id, title, artifact_type, content, mime_type, status, created_at, updated_at) VALUES ($1, 'default', $2, $3, $4, $5, $6, $7, 'submitted', $8, $9)")
                .bind(&id)
                .bind(agent_id)
                .bind(device_id)
                .bind(title)
                .bind(artifact_type)
                .bind(content)
                .bind(mime_type)
                .bind(&now)
                .bind(&now)
                .execute(&db.pool)
                .await
        };
        
        if result.is_ok() {
            accepted.push(id);
        }
    }
    
    Json(json!({
        "accepted": accepted,
        "total": accepted.len()
    }))
}

async fn ingest_memories(
    State(db): State<Database>,
    Json(payload): Json<Value>,
) -> Json<Value> {
    let now = chrono::Utc::now().to_rfc3339();
    let mut accepted = Vec::new();
    
    let agent_id = payload.get("agent_id").and_then(|a| a.as_str()).unwrap_or("unknown");
    let device_id = payload.get("device_id").and_then(|d| d.as_str());
    
    if let Some(memories) = payload.get("memories").and_then(|m| m.as_array()) {
        for memory in memories {
            let id = uuid::Uuid::new_v4().to_string();
            
            let scope = memory.get("scope").and_then(|s| s.as_str()).unwrap_or("project");
            let typ = memory.get("type").and_then(|t| t.as_str()).unwrap_or("semantic");
            let title = memory.get("title").and_then(|t| t.as_str()).unwrap_or("Untitled Memory");
            let content = memory.get("content").and_then(|c| c.as_str()).unwrap_or("");
            let reason = memory.get("reason").and_then(|r| r.as_str());
            let confidence = memory.get("confidence").and_then(|c| c.as_f64()).unwrap_or(0.5);
            
            let result = if db.is_sqlite {
                sqlx::query("INSERT INTO memory_candidate (id, workspace_id, title, proposed_scope, proposed_type, content, reason, source_agent_id, source_device_id, confidence, review_status, created_at) VALUES (?, 'default', ?, ?, ?, ?, ?, ?, ?, ?, 'needs_review', ?)")
                    .bind(&id)
                    .bind(title)
                    .bind(scope)
                    .bind(typ)
                    .bind(content)
                    .bind(reason)
                    .bind(agent_id)
                    .bind(device_id)
                    .bind(confidence)
                    .bind(&now)
                    .execute(&db.pool)
                    .await
            } else {
                sqlx::query("INSERT INTO memory_candidate (id, workspace_id, title, proposed_scope, proposed_type, content, reason, source_agent_id, source_device_id, confidence, review_status, created_at) VALUES ($1, 'default', $2, $3, $4, $5, $6, $7, $8, $9, 'needs_review', $10)")
                    .bind(&id)
                    .bind(title)
                    .bind(scope)
                    .bind(typ)
                    .bind(content)
                    .bind(reason)
                    .bind(agent_id)
                    .bind(device_id)
                    .bind(confidence)
                    .bind(&now)
                    .execute(&db.pool)
                    .await
            };
            
            if result.is_ok() {
                accepted.push(json!({
                    "candidate_id": id,
                    "review_status": "needs_review"
                }));
            }
        }
    }
    
    Json(json!({
        "accepted": accepted,
        "rejected": []
    }))
}
