pub mod memory;
pub mod device;
pub mod artifact;
pub mod memory_candidate;
pub mod document;
pub mod source_file;
pub mod data_source;
pub mod workspace;
pub mod event_log;

#[allow(dead_code)]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PaginationParams {
    pub page: Option<i64>,
    pub per_page: Option<i64>,
}
