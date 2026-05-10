use serde::{Deserialize, Serialize};

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataSource {
    pub id: String,
    pub workspace_id: String,
    pub device_id: String,
    pub name: String,
    pub path: String,
    pub source_type: String,
    pub recursive: bool,
    pub include_globs: serde_json::Value,
    pub exclude_globs: serde_json::Value,
    pub scan_mode: String,
    pub sensitivity: String,
    pub enabled: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateDataSourceRequest {
    pub name: String,
    pub path: String,
    pub source_type: String,
    pub recursive: Option<bool>,
    pub include_globs: Option<Vec<String>>,
    pub exclude_globs: Option<Vec<String>>,
    pub scan_mode: Option<String>,
    pub sensitivity: Option<String>,
}
