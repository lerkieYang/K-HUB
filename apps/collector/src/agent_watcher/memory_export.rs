use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryExport {
    pub schema_version: String,
    pub agent_id: String,
    pub device_id: Option<String>,
    pub session_id: Option<String>,
    pub task_id: Option<String>,
    pub created_at: String,
    pub task_summary: Option<String>,
    pub user_request: Option<String>,
    pub artifacts: Option<Vec<ArtifactExport>>,
    pub memories: Option<Vec<MemoryItem>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactExport {
    pub title: String,
    pub r#type: String,
    pub content_path: Option<String>,
    pub mime_type: Option<String>,
    pub hash: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryItem {
    pub scope: String,
    pub r#type: String,
    pub content: String,
    pub reason: Option<String>,
    pub confidence: Option<f64>,
    pub suggested_visibility: Option<String>,
}

impl MemoryExport {
    pub fn parse(content: &str) -> anyhow::Result<Self> {
        Ok(serde_json::from_str(content)?)
    }
}
