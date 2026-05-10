use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

/// Memory类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MemoryType {
    ProjectBackground,
    SOP,
    RulesStandards,
    Template,
    CaseLibrary,
    Decision,
    Episodic,
    Semantic,
    Procedural,
    Preference,
    Warning,
}

impl MemoryType {
    pub fn as_str(&self) -> &'static str {
        match self {
            MemoryType::ProjectBackground => "project_background",
            MemoryType::SOP => "sop",
            MemoryType::RulesStandards => "rules_standards",
            MemoryType::Template => "template",
            MemoryType::CaseLibrary => "case_library",
            MemoryType::Decision => "decision",
            MemoryType::Episodic => "episodic",
            MemoryType::Semantic => "semantic",
            MemoryType::Procedural => "procedural",
            MemoryType::Preference => "preference",
            MemoryType::Warning => "warning",
        }
    }
    
    #[allow(dead_code)]
    pub fn from_str(s: &str) -> Self {
        match s {
            "project_background" => MemoryType::ProjectBackground,
            "sop" => MemoryType::SOP,
            "rules_standards" | "style_guide" => MemoryType::RulesStandards,
            "template" => MemoryType::Template,
            "case_library" => MemoryType::CaseLibrary,
            "decision" => MemoryType::Decision,
            "episodic" => MemoryType::Episodic,
            "semantic" => MemoryType::Semantic,
            "procedural" => MemoryType::Procedural,
            "preference" => MemoryType::Preference,
            "warning" => MemoryType::Warning,
            _ => MemoryType::Semantic,
        }
    }
}

/// Memory可见性
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Visibility {
    Private,
    Agent,
    Project,
    Shared,
}

impl Visibility {
    pub fn as_str(&self) -> &'static str {
        match self {
            Visibility::Private => "private",
            Visibility::Agent => "agent",
            Visibility::Project => "project",
            Visibility::Shared => "shared",
        }
    }
}

/// Memory来源
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MemorySource {
    KnowledgeFile { file_path: String },
    Agent { session_id: Option<String>, agent_id: String },
    Wechat { message_id: Option<String> },
    Feishu { message_id: Option<String> },
    Browser { url: Option<String> },
    Git { commit_hash: Option<String> },
    Manual,
}

impl MemorySource {
    pub fn source_type(&self) -> &'static str {
        match self {
            MemorySource::KnowledgeFile { .. } => "knowledge",
            MemorySource::Agent { .. } => "agent",
            MemorySource::Wechat { .. } => "wechat",
            MemorySource::Feishu { .. } => "feishu",
            MemorySource::Browser { .. } => "browser",
            MemorySource::Git { .. } => "git",
            MemorySource::Manual => "manual",
        }
    }
    
    #[allow(dead_code)]
    pub fn is_knowledge(&self) -> bool {
        matches!(self, MemorySource::KnowledgeFile { .. })
    }
}

/// Memory状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[allow(dead_code)]
pub enum MemoryStatus {
    Active,
    Archived,
    Deleted,
}

impl MemoryStatus {
    #[allow(dead_code)]
    pub fn as_str(&self) -> &'static str {
        match self {
            MemoryStatus::Active => "active",
            MemoryStatus::Archived => "archived",
            MemoryStatus::Deleted => "deleted",
        }
    }
}

/// Memory结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Memory {
    pub id: String,
    pub workspace_id: String,
    pub title: String,
    pub content: String,
    pub memory_type: MemoryType,
    pub scope: String,
    pub tags: Vec<String>,
    pub source: MemorySource,
    pub owner: Option<String>,
    pub confidence: f64,
    pub visibility: Visibility,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_used_at: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
    pub embedding: Option<Vec<f32>>,
    pub related_to: Vec<String>,
    pub derived_from: Vec<String>,
}

/// 创建Memory请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateMemoryRequest {
    pub title: String,
    pub content: String,
    pub memory_type: MemoryType,
    pub scope: Option<String>,
    pub tags: Option<Vec<String>>,
    pub source: Option<MemorySource>,
    /// 直接指定source_type，优先于source字段（用于手动添加memory/knowledge）
    pub source_type: Option<String>,
    pub owner: Option<String>,
    pub confidence: Option<f64>,
    pub visibility: Option<Visibility>,
}

/// 更新Memory请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateMemoryRequest {
    pub title: Option<String>,
    pub content: Option<String>,
    pub memory_type: Option<MemoryType>,
    pub scope: Option<String>,
    pub tags: Option<Vec<String>>,
    pub owner: Option<String>,
    pub confidence: Option<f64>,
    pub visibility: Option<Visibility>,
    pub status: Option<String>,
}

/// Memory搜索参数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemorySearchParams {
    pub query: Option<String>,
    pub memory_type: Option<String>,
    pub source_type: Option<String>,
    pub scope: Option<String>,
    pub tags: Option<Vec<String>>,
    pub status: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}
