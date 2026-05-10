use axum::{routing::{get, post, delete}, Router, Json, extract::State};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use once_cell::sync::Lazy;
use crate::db::Database;
use crate::models::device::{CreateDeviceRequest, CreateInviteRequest, HeartbeatRequest};

static INVITE_CODES: Lazy<Arc<Mutex<HashMap<String, InviteInfo>>>> = Lazy::new(|| Arc::new(Mutex::new(HashMap::new())));

#[allow(dead_code)]
struct InviteInfo {
    expires_at: chrono::DateTime<chrono::Utc>,
    allowed_role: String,
    scopes: Vec<String>,
}

pub fn routes() -> Router<Database> {
    Router::new()
        .route("/api/invites", post(create_invite))
        .route("/api/devices", get(list_devices))
        .route("/api/devices/register", post(register_device))
        .route("/api/devices/:id/approve", post(approve_device))
        .route("/api/devices/heartbeat", post(heartbeat))
        .route("/api/devices/:id", delete(revoke_device))
}

async fn create_invite(
    State(_db): State<Database>,
    Json(req): Json<CreateInviteRequest>,
) -> Json<Value> {
    let invite_id = uuid::Uuid::new_v4().to_string();
    let invite_code = uuid::Uuid::new_v4().to_string();
    let ttl = req.ttl_seconds.unwrap_or(600);
    let expires_at = chrono::Utc::now() + chrono::Duration::seconds(ttl);
    
    // 存储邀请码（使用tokio::sync::Mutex的await）
    let mut codes = INVITE_CODES.lock().await;
    codes.insert(invite_code.clone(), InviteInfo {
        expires_at,
        allowed_role: req.allowed_role.unwrap_or_else(|| "client".to_string()),
        scopes: req.scopes.unwrap_or_else(|| vec!["file_upload".to_string(), "context_request".to_string()]),
    });
    drop(codes);
    
    Json(json!({
        "invite_id": invite_id,
        "invite_code": invite_code,
        "expires_at": expires_at.to_rfc3339(),
        "qr_payload": format!("khub://join?code={}", invite_code)
    }))
}

async fn list_devices(
    State(db): State<Database>,
) -> Json<Value> {
    let devices =
        sqlx::query_as::<_, (String, String, String, String, Option<String>, Option<String>)>(
            "SELECT id, name, device_type, status, platform, last_seen_at FROM device ORDER BY created_at DESC"
        )
        .fetch_all(&db.pool)
        .await;
    
    match devices {
        Ok(rows) => {
            let devices: Vec<Value> = rows.into_iter().map(|(id, name, device_type, status, platform, last_seen)| {
                json!({
                    "id": id,
                    "name": name,
                    "device_type": device_type,
                    "status": status,
                    "platform": platform,
                    "last_seen_at": last_seen
                })
            }).collect();
            Json(json!({"devices": devices, "total": devices.len()}))
        },
        Err(e) => Json(json!({"error": e.to_string()})),
    }
}

async fn register_device(
    State(db): State<Database>,
    Json(req): Json<CreateDeviceRequest>,
) -> Json<Value> {
    // 验证邀请码（使用tokio::sync::Mutex的await）
    {
        let mut codes = INVITE_CODES.lock().await;
        if let Some(info) = codes.get(&req.invite_code) {
            if chrono::Utc::now() > info.expires_at {
                codes.remove(&req.invite_code);
                return Json(json!({"error": "Invite code expired"}));
            }
        } else {
            return Json(json!({"error": "Invalid invite code"}));
        }
    }
    
    let device_id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    
    // Schema: id, workspace_id, name, device_type, platform, os_version, app_version, device_token, invite_code, status, last_seen_at, created_at, updated_at
    let result =
        sqlx::query("INSERT INTO device (id, workspace_id, name, device_type, platform, os_version, app_version, status, invite_code, created_at, updated_at) VALUES (?, 'default', ?, 'desktop', ?, ?, ?, 'pending', ?, ?, ?)")
            .bind(&device_id)
            .bind(&req.device_name)
            .bind(&req.os)
            .bind(&req.app_version)
            .bind(&req.invite_code)
            .bind(&now)
            .bind(&now)
            .execute(&db.pool)
            .await;
    
    match result {
        Ok(_) => {
            // 删除已使用的邀请码
            let mut codes = INVITE_CODES.lock().await;
            codes.remove(&req.invite_code);
            
            Json(json!({
                "device_id": device_id,
                "status": "pending",
                "message": "Waiting for owner approval"
            }))
        },
        Err(e) => Json(json!({"error": e.to_string()})),
    }
}

async fn approve_device(
    State(db): State<Database>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Json<Value> {
    let token = format!("kh_{}", uuid::Uuid::new_v4());
    let now = chrono::Utc::now().to_rfc3339();
    
    let result =
        sqlx::query("UPDATE device SET status = 'approved', device_token = ?, updated_at = ? WHERE id = ?")
            .bind(&token)
            .bind(&now)
            .bind(&id)
            .execute(&db.pool)
            .await;
    
    match result {
        Ok(_) => Json(json!({
            "device_id": id,
            "status": "approved",
            "device_token": token,
            "hub_config": {
                "hub_url": "http://127.0.0.1:8443",
                "mcp_url": "http://127.0.0.1:8443/mcp"
            }
        })),
        Err(e) => Json(json!({"error": e.to_string()})),
    }
}

async fn heartbeat(
    State(db): State<Database>,
    Json(req): Json<HeartbeatRequest>,
) -> Json<Value> {
    let now = chrono::Utc::now().to_rfc3339();
    
    let result =
        sqlx::query("UPDATE device SET last_seen_at = ?, status = 'online' WHERE id = ?")
            .bind(&now)
            .bind(&req.device_id)
            .execute(&db.pool)
            .await;
    
    match result {
        Ok(_) => Json(json!({
            "server_time": now,
            "device_status": "online",
            "commands": []
        })),
        Err(e) => Json(json!({"error": e.to_string()})),
    }
}

async fn revoke_device(
    State(db): State<Database>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Json<Value> {
    let now = chrono::Utc::now().to_rfc3339();
    
    // Schema has no revoked_at column, just update status
    let result =
        sqlx::query("UPDATE device SET status = 'revoked', updated_at = ? WHERE id = ?")
            .bind(&now)
            .bind(&id)
            .execute(&db.pool)
            .await;
    
    match result {
        Ok(_) => Json(json!({"success": true})),
        Err(e) => Json(json!({"error": e.to_string()})),
    }
}
