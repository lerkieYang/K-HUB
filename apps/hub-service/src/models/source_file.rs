use serde::{Deserialize, Serialize};

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceFile {
    pub id: String,
    pub workspace_id: String,
    pub device_id: String,
    pub data_source_id: String,
    pub path: String,
    pub normalized_path: String,
    pub file_name: String,
    pub extension: Option<String>,
    pub mime_type: Option<String>,
    pub size_bytes: Option<i64>,
    pub mtime: Option<String>,
    pub sha256: Option<String>,
    pub status: String,
    pub last_seen_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEvent {
    pub event_id: String,
    pub data_source_id: String,
    pub event_type: String,
    pub path: String,
    pub size_bytes: Option<i64>,
    pub mtime: Option<String>,
    pub sha256: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEventBatch {
    pub device_id: String,
    pub events: Vec<FileEvent>,
}
