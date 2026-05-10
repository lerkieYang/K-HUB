use axum::{routing::{get, post}, Router, Json, extract::State};
use serde_json::{json, Value};
use crate::db::Database;
use crate::models::workspace::CreateWorkspaceRequest;

pub fn routes() -> Router<Database> {
    Router::new()
        .route("/api/workspace", post(create_workspace))
        .route("/api/workspace/:id", get(get_workspace))
}

async fn create_workspace(
    State(db): State<Database>,
    Json(req): Json<CreateWorkspaceRequest>,
) -> Json<Value> {
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    
    // Schema columns: id, name, description, owner_id, settings, created_at, updated_at
    let result =
        sqlx::query("INSERT INTO workspace (id, name, description, created_at, updated_at) VALUES (?, ?, ?, ?, ?)")
            .bind(&id)
            .bind(&req.name)
            .bind(&req.description)
            .bind(&now)
            .bind(&now)
            .execute(&db.pool)
            .await;
    
    match result {
        Ok(_) => Json(json!({
            "id": id,
            "name": req.name,
            "description": req.description,
            "created_at": now,
            "updated_at": now
        })),
        Err(e) => Json(json!({"error": e.to_string()})),
    }
}

async fn get_workspace(
    State(db): State<Database>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Json<Value> {
    let workspace =
        sqlx::query_as::<_, (String, String, Option<String>, Option<String>, Option<String>, String, String)>("SELECT id, name, description, owner_id, settings, created_at, updated_at FROM workspace WHERE id = ?")
            .bind(&id)
            .fetch_one(&db.pool)
            .await;
    
    match workspace {
        Ok((id, name, description, owner_id, settings, created_at, updated_at)) => Json(json!({
            "id": id,
            "name": name,
            "description": description,
            "owner_id": owner_id,
            "settings": settings,
            "created_at": created_at,
            "updated_at": updated_at
        })),
        Err(_) => Json(json!({"error": "Workspace not found"})),
    }
}
