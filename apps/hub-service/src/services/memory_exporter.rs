#![allow(dead_code)]
use chrono::Utc;
use serde::{Serialize, Deserialize};
use crate::models::memory::Memory;

/// 导出格式
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExportFormat {
    #[serde(rename = "json")]
    JSON,
    #[serde(rename = "markdown")]
    Markdown,
    #[serde(rename = "csv")]
    CSV,
}

/// 导出配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportConfig {
    pub format: ExportFormat,
    pub include_knowledge: bool,
    pub include_activity: bool,
    pub source_types: Option<Vec<String>>,
    pub memory_types: Option<Vec<String>>,
    pub output_path: Option<String>,
}

/// memory-export.json 标准格式
#[derive(Debug, Serialize, Deserialize)]
pub struct MemoryExport {
    pub schema_version: String,
    pub export_id: String,
    pub exported_at: String,
    pub workspace_id: String,
    pub total_count: usize,
    pub memories: Vec<MemoryExportItem>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MemoryExportItem {
    pub id: String,
    pub title: String,
    pub content: String,
    pub memory_type: String,
    pub scope: String,
    pub tags: Vec<String>,
    pub source: SourceInfo,
    pub metadata: MetadataInfo,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SourceInfo {
    pub source_type: String,
    pub file_path: Option<String>,
    pub agent_id: Option<String>,
    pub url: Option<String>,
    pub commit_hash: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MetadataInfo {
    pub owner: Option<String>,
    pub confidence: f64,
    pub visibility: String,
    pub status: String,
}

/// Memory导出服务
pub struct MemoryExporter;

impl MemoryExporter {
    /// 导出为JSON格式
    pub fn export_json(memories: &[Memory], config: &ExportConfig) -> Result<String, String> {
        let filtered = Self::filter_memories(memories, config);
        
        let export = MemoryExport {
            schema_version: "2.0".to_string(),
            export_id: uuid::Uuid::new_v4().to_string(),
            exported_at: Utc::now().to_rfc3339(),
            workspace_id: "default".to_string(),
            total_count: filtered.len(),
            memories: filtered.iter().map(|m| MemoryExportItem {
                id: m.id.clone(),
                title: m.title.clone(),
                content: m.content.clone(),
                memory_type: m.memory_type.as_str().to_string(),
                scope: m.scope.clone(),
                tags: m.tags.clone(),
                source: SourceInfo {
                    source_type: m.source.source_type().to_string(),
                    file_path: None, // TODO: 从source提取
                    agent_id: None,
                    url: None,
                    commit_hash: None,
                },
                metadata: MetadataInfo {
                    owner: m.owner.clone(),
                    confidence: m.confidence,
                    visibility: format!("{:?}", m.visibility).to_lowercase(),
                    status: format!("{:?}", m.status).to_lowercase(),
                },
                created_at: m.created_at.to_rfc3339(),
                updated_at: m.updated_at.to_rfc3339(),
            }).collect(),
        };
        
        serde_json::to_string_pretty(&export).map_err(|e| e.to_string())
    }
    
    /// 导出为Markdown格式
    pub fn export_markdown(memories: &[Memory], config: &ExportConfig) -> Result<String, String> {
        let filtered = Self::filter_memories(memories, config);
        
        let mut output = String::new();
        output.push_str("# Memory Export\n\n");
        output.push_str(&format!("**导出时间:** {}\n", Utc::now().format("%Y-%m-%d %H:%M:%S")));
        output.push_str(&format!("**总数量:** {}\n\n", filtered.len()));
        
        // 按类型分组
        let mut by_type: std::collections::HashMap<String, Vec<&Memory>> = std::collections::HashMap::new();
        for memory in &filtered {
            let type_key = memory.memory_type.as_str().to_string();
            by_type.entry(type_key).or_insert_with(Vec::new).push(memory);
        }
        
        for (type_name, type_memories) in &by_type {
            output.push_str(&format!("## {}\n\n", type_name.to_uppercase()));
            
            for memory in type_memories {
                output.push_str(&format!("### {}\n\n", memory.title));
                output.push_str(&format!("**类型:** {}\n", memory.memory_type.as_str()));
                output.push_str(&format!("**范围:** {}\n", memory.scope));
                output.push_str(&format!("**置信度:** {:.0}%\n", memory.confidence * 100.0));
                
                if !memory.tags.is_empty() {
                    output.push_str(&format!("**标签:** {}\n", memory.tags.join(", ")));
                }
                
                if let Some(owner) = &memory.owner {
                    output.push_str(&format!("**负责人:** {}\n", owner));
                }
                
                output.push_str(&format!("**更新时间:** {}\n\n", memory.updated_at.format("%Y-%m-%d %H:%M")));
                output.push_str(&memory.content);
                output.push_str("\n\n---\n\n");
            }
        }
        
        Ok(output)
    }
    
    /// 导出为CSV格式
    pub fn export_csv(memories: &[Memory], config: &ExportConfig) -> Result<String, String> {
        let filtered = Self::filter_memories(memories, config);
        
        let mut output = String::new();
        output.push_str("id,title,type,scope,tags,source_type,confidence,visibility,status,created_at,updated_at\n");
        
        for memory in &filtered {
            let tags_str = memory.tags.join(";");
            let line = format!(
                "{},{},{},{},{},{},{:.2},{},{},{},{}\n",
                memory.id,
                Self::escape_csv(&memory.title),
                memory.memory_type.as_str(),
                memory.scope,
                Self::escape_csv(&tags_str),
                memory.source.source_type(),
                memory.confidence,
                format!("{:?}", memory.visibility).to_lowercase(),
                format!("{:?}", memory.status).to_lowercase(),
                memory.created_at.to_rfc3339(),
                memory.updated_at.to_rfc3339()
            );
            output.push_str(&line);
        }
        
        Ok(output)
    }
    
    /// 过滤Memory
    fn filter_memories<'a>(memories: &'a [Memory], config: &ExportConfig) -> Vec<&'a Memory> {
        memories.iter().filter(|m| {
            // 按来源类型过滤
            if let Some(source_types) = &config.source_types {
                if !source_types.contains(&m.source.source_type().to_string()) {
                    return false;
                }
            }
            
            // 按memory类型过滤
            if let Some(memory_types) = &config.memory_types {
                if !memory_types.contains(&m.memory_type.as_str().to_string()) {
                    return false;
                }
            }
            
            // 按知识库/活动记忆过滤
            let is_knowledge = m.source.is_knowledge();
            if is_knowledge && !config.include_knowledge {
                return false;
            }
            if !is_knowledge && !config.include_activity {
                return false;
            }
            
            true
        }).collect()
    }
    
    /// CSV字段转义
    fn escape_csv(s: &str) -> String {
        if s.contains(',') || s.contains('"') || s.contains('\n') {
            format!("\"{}\"", s.replace('"', "\"\""))
        } else {
            s.to_string()
        }
    }
}

/// memory-export.json 格式解析
pub struct MemoryImport;

impl MemoryImport {
    /// 解析memory-export.json
    pub fn parse_export_json(json_str: &str) -> Result<Vec<Memory>, String> {
        let export: MemoryExport = serde_json::from_str(json_str)
            .map_err(|e| format!("Failed to parse export: {}", e))?;
        
        let mut memories = Vec::new();
        
        for item in export.memories {
            let memory = Memory {
                id: item.id,
                workspace_id: export.workspace_id.clone(),
                title: item.title,
                content: item.content,
                memory_type: serde_json::from_str(&format!("\"{}\"", item.memory_type))
                    .unwrap_or(crate::models::memory::MemoryType::Semantic),
                scope: item.scope,
                tags: item.tags,
                source: crate::models::memory::MemorySource::Manual, // 简化处理
                owner: item.metadata.owner,
                confidence: item.metadata.confidence,
                visibility: serde_json::from_str(&format!("\"{}\"", item.metadata.visibility))
                    .unwrap_or(crate::models::memory::Visibility::Private),
                status: serde_json::from_str(&format!("\"{}\"", item.metadata.status))
                    .unwrap_or("active".to_string()),
                created_at: chrono::DateTime::parse_from_rfc3339(&item.created_at)
                    .map(|dt| dt.with_timezone(&chrono::Utc))
                    .unwrap_or_else(|_| Utc::now()),
                updated_at: chrono::DateTime::parse_from_rfc3339(&item.updated_at)
                    .map(|dt| dt.with_timezone(&chrono::Utc))
                    .unwrap_or_else(|_| Utc::now()),
                last_used_at: None,
                expires_at: None,
                embedding: None,
                related_to: Vec::new(),
                derived_from: Vec::new(),
            };
            memories.push(memory);
        }
        
        Ok(memories)
    }
}
