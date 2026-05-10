use serde::{Deserialize, Serialize};

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Device {
    pub id: String,
    pub workspace_id: String,
    pub name: String,
    pub role: String,
    pub status: String,
    pub public_key: Option<String>,
    pub fingerprint: Option<String>,
    pub os: Option<String>,
    pub app_version: Option<String>,
    pub last_seen_at: Option<String>,
    pub created_at: String,
    pub revoked_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateDeviceRequest {
    pub invite_code: String,
    pub device_name: String,
    pub os: String,
    pub app_version: String,
    pub public_key: Option<String>,
    pub fingerprint: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateInviteRequest {
    pub ttl_seconds: Option<i64>,
    pub allowed_role: Option<String>,
    pub scopes: Option<Vec<String>>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InviteResponse {
    pub invite_id: String,
    pub invite_code: String,
    pub expires_at: String,
    pub qr_payload: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartbeatRequest {
    pub device_id: String,
    pub app_version: String,
    pub collector_status: String,
    pub pending_upload_count: i32,
    pub watched_source_count: i32,
}
