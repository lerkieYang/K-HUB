use serde::{Deserialize, Serialize};

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Artifact {
    pub id: String,
    pub workspace_id: String,
    pub agent_id: Option<String>,
    pub device_id: Option<String>,
    pub title: String,
    pub artifact_type: String,
    pub content: Option<String>,
    pub file_path: Option<String>,
    pub mime_type: Option<String>,
    pub sha256: Option<String>,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateArtifactRequest {
    pub agent_id: String,
    pub device_id: Option<String>,
    pub title: String,
    pub artifact_type: String,
    pub content: Option<String>,
    pub mime_type: Option<String>,
    pub sha256: Option<String>,
}
