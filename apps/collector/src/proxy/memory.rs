use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemorySubmitRequest {
    pub agent_id: String,
    pub device_id: Option<String>,
    pub task_id: Option<String>,
    pub candidates: Vec<MemoryCandidate>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryCandidate {
    pub scope: String,
    pub r#type: String,
    pub content: String,
    pub reason: Option<String>,
    pub confidence: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemorySubmitResponse {
    pub accepted: Vec<AcceptedCandidate>,
    pub rejected: Vec<RejectedCandidate>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcceptedCandidate {
    pub candidate_id: String,
    pub review_status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RejectedCandidate {
    pub content: String,
    pub reason: String,
}
