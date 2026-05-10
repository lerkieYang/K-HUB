use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextRequest {
    pub task: String,
    pub agent_id: String,
    pub device_id: Option<String>,
    pub need: Option<Vec<String>>,
    pub max_tokens: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextResponse {
    pub request_id: String,
    pub context_pack: Value,
    pub sources: Vec<String>,
    pub confidence: f64,
    pub expires_at: String,
}
