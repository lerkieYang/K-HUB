use serde::{Deserialize, Serialize};

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryCandidate {
    pub id: String,
    pub workspace_id: String,
    pub proposed_scope: Option<String>,
    pub proposed_type: Option<String>,
    pub content: Option<String>,
    pub reason: Option<String>,
    pub source_agent_id: Option<String>,
    pub source_device_id: Option<String>,
    pub source_artifact_id: Option<String>,
    pub confidence: f64,
    pub review_status: String,
    pub review_notes: Option<String>,
    pub created_at: String,
    pub reviewed_at: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmitCandidateRequest {
    pub agent_id: String,
    pub device_id: Option<String>,
    pub task_id: Option<String>,
    pub candidates: Vec<CandidateItem>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CandidateItem {
    pub scope: String,
    pub r#type: String,
    pub content: String,
    pub reason: Option<String>,
    pub confidence: Option<f64>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewRequest {
    pub review_notes: Option<String>,
}
