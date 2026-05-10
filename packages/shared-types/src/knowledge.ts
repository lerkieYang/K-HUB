use serde::{Deserialize, Serialize};

/// Memory级别
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MemoryLevel {
    /// 短期记忆 - 当前会话，自动过期
    #[serde(rename = "short_term")]
    ShortTerm,
    /// 中期记忆 - 项目周期，手动管理
    #[serde(rename = "medium_term")]
    MediumTerm,
    /// 长期记忆 - 永久保存
    #[serde(rename = "long_term")]
    LongTerm,
}

impl MemoryLevel {
    pub fn default_expiry_days(&self) -> Option<i64> {
        match self {
            MemoryLevel::ShortTerm => Some(7),      // 7天过期
            MemoryLevel::MediumTerm => Some(90),    // 90天过期
            MemoryLevel::LongTerm => None,          // 永不过期
        }
    }
    
    pub fn from_scope(scope: &str) -> Self {
        match scope {
            "session" => MemoryLevel::ShortTerm,
            "project" => MemoryLevel::MediumTerm,
            "global" => MemoryLevel::LongTerm,
            _ => MemoryLevel::MediumTerm,
        }
    }
}

/// 知识库分类
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeCategory {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub parent_id: Option<String>,
    pub icon: Option<String>,
    pub color: Option<String>,
}

/// 知识库标签
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeTag {
    pub id: String,
    pub name: String,
    pub color: Option<String>,
}

/// 知识库文档
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeDocument {
    pub id: String,
    pub title: String,
    pub content: String,
    pub category_id: Option<String>,
    pub tags: Vec<String>,
    pub source: String,
    pub source_url: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub version: i32,
    pub metadata: serde_json::Value,
}

/// 导出格式
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExportFormat {
    #[serde(rename = "json")]
    Json,
    #[serde(rename = "markdown")]
    Markdown,
    #[serde(rename = "csv")]
    Csv,
}

/// 导出配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportConfig {
    pub format: ExportFormat,
    pub include_memories: bool,
    pub include_documents: bool,
    pub include_artifacts: bool,
    pub categories: Option<Vec<String>>,
    pub tags: Option<Vec<String>>,
    pub date_range: Option<(String, String)>,
}

/// 导入配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportConfig {
    pub format: ExportFormat,
    pub merge_strategy: MergeStrategy,
    pub deduplicate: bool,
}

/// 合并策略
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MergeStrategy {
    /// 跳过已存在
    #[serde(rename = "skip")]
    Skip,
    /// 覆盖已存在
    #[serde(rename = "overwrite")]
    Overwrite,
    /// 合并内容
    #[serde(rename = "merge")]
    Merge,
    /// 创建新版本
    #[serde(rename = "version")]
    Version,
}
