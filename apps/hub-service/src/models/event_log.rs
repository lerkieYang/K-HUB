use serde::{Deserialize, Serialize};

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventLog {
    pub id: String,
    pub workspace_id: String,
    pub device_id: Option<String>,
    pub actor_type: String,
    pub actor_id: Option<String>,
    pub event_type: String,
    pub entity_type: Option<String>,
    pub entity_id: Option<String>,
    pub payload: serde_json::Value,
    pub created_at: String,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateEventRequest {
    pub device_id: Option<String>,
    pub actor_type: String,
    pub actor_id: Option<String>,
    pub event_type: String,
    pub entity_type: Option<String>,
    pub entity_id: Option<String>,
    pub payload: Option<serde_json::Value>,
}
