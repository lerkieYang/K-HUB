use serde::{Deserialize, Serialize};

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    pub id: String,
    pub workspace_id: String,
    pub source_file_id: Option<String>,
    pub title: Option<String>,
    pub doc_type: Option<String>,
    pub language: Option<String>,
    pub summary: Option<String>,
    pub owner: Option<String>,
    pub last_updated: Option<String>,
    pub review_frequency: Option<String>,
    pub sensitivity: String,
    pub health_score: Option<f64>,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentChunk {
    pub id: String,
    pub document_id: String,
    pub chunk_index: i32,
    pub content: Option<String>,
    pub token_count: Option<i32>,
    pub metadata: serde_json::Value,
    pub created_at: String,
}
