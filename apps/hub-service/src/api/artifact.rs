use axum::{routing::get, Router, Json, extract::State};
use serde_json::{json, Value};
use crate::db::Database;
use crate::models::artifact::CreateArtifactRequest;

pub fn routes() -> Router<Database> {
    Router::new()
        .route("/api/artifacts", get(list_artifacts).post(create_artifact))
        .route("/api/artifacts/:id", get(get_artifact))
}

async fn list_artifacts(
    State(db): State<Database>,
) -> Json<Value> {
    let artifacts =
        sqlx::query_as::<_, (String, String, String, Option<String>, String)>(
            "SELECT id, title, artifact_type, mime_type, status FROM artifact ORDER BY created_at DESC LIMIT 50"
        )
        .fetch_all(&db.pool)
        .await;
    
    match artifacts {
        Ok(rows) => {
            let artifacts: Vec<Value> = rows.into_iter().map(|(id, title, typ, mime, status)| {
                json!({
                    "id": id,
                    "title": title,
                    "artifact_type": typ,
                    "mime_type": mime,
                    "status": status
                })
            }).collect();
            Json(json!({"artifacts": artifacts, "total": artifacts.len()}))
        },
        Err(e) => Json(json!({"error": e.to_string()})),
    }
}

async fn create_artifact(
    State(db): State<Database>,
    Json(req): Json<CreateArtifactRequest>,
) -> Json<Value> {
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    
    let result =
        sqlx::query("INSERT INTO artifact (id, workspace_id, agent_id, device_id, title, artifact_type, content, mime_type, sha256, status, created_at, updated_at) VALUES (?, 'default', ?, ?, ?, ?, ?, ?, ?, 'submitted', ?, ?)")
            .bind(&id)
            .bind(&req.agent_id)
            .bind(&req.device_id)
            .bind(&req.title)
            .bind(&req.artifact_type)
            .bind(&req.content)
            .bind(&req.mime_type)
            .bind(&req.sha256)
            .bind(&now)
            .bind(&now)
            .execute(&db.pool)
            .await;
    
    match result {
        Ok(_) => Json(json!({
            "artifact_id": id,
            "status": "submitted"
        })),
        Err(e) => Json(json!({"error": format!("DB insert failed: {}", e)})),
    }
}

async fn get_artifact(
    State(db): State<Database>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Json<Value> {
    let artifact =
        sqlx::query_as::<_, (String, String, String, Option<String>, Option<String>, String)>(
            "SELECT id, title, artifact_type, content, mime_type, status FROM artifact WHERE id = ?"
        )
        .bind(&id)
        .fetch_one(&db.pool)
        .await;
    
    match artifact {
        Ok((id, title, typ, content, mime, status)) => Json(json!({
            "id": id,
            "title": title,
            "artifact_type": typ,
            "content": content,
            "mime_type": mime,
            "status": status
        })),
        Err(_) => Json(json!({"error": "Artifact not found"})),
    }
}
