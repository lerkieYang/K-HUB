use std::path::{Path, PathBuf};
use std::collections::HashMap;
use chrono::Utc;
use sha2::{Sha256, Digest};
use serde::{Serialize, Deserialize};
use crate::models::memory::{Memory, MemoryType, MemorySource, Visibility};
use crate::services::doc_converter::DocConverter;

/// 索引状态
#[derive(Debug, Serialize, Deserialize)]
struct IndexState {
    file_hashes: HashMap<String, String>,
    indexed_files: HashMap<String, String>,
    last_updated: String,
}

/// 知识库索引服务
#[allow(dead_code)]
 pub struct KnowledgeIndexer {
     paths: Vec<PathBuf>,
     embedding_dir: PathBuf,
     cache_dir: PathBuf,
     converter: DocConverter,
     file_hashes: HashMap<String, String>,
     indexed_files: HashMap<String, String>,
     dirty: bool,
     chat_service: Option<crate::services::ai_service::ChatService>,
 }

impl KnowledgeIndexer {
     pub fn new(paths: Vec<String>, embedding_dir: PathBuf, cache_dir: PathBuf) -> Self {
         Self {
             paths: paths.into_iter().map(PathBuf::from).collect(),
             embedding_dir: embedding_dir.clone(),
             cache_dir: cache_dir.clone(),
             converter: DocConverter::new(cache_dir),
             file_hashes: HashMap::new(),
             indexed_files: HashMap::new(),
             dirty: false,
             chat_service: None,
         }
     }
    
    /// 设置 ChatService（用于轻 embedding：关键词/标签/摘要提取）
    #[allow(dead_code)]
    pub fn with_chat_service(mut self, chat_service: crate::services::ai_service::ChatService) -> Self {
        self.chat_service = Some(chat_service);
        self
    }
    
    /// 加载缓存状态
    pub async fn load_state(&mut self) -> Result<(), String> {
        let state_file = self.embedding_dir.join("index_state.json");
        
        if state_file.exists() {
            let content = tokio::fs::read_to_string(&state_file).await
                .map_err(|e| format!("Failed to read state: {}", e))?;
            
            let state: IndexState = serde_json::from_str(&content)
                .map_err(|e| format!("Failed to parse state: {}", e))?;
            
            self.file_hashes = state.file_hashes;
            self.indexed_files = state.indexed_files;
            
            tracing::info!("Loaded index state: {} files", self.indexed_files.len());
        }
        
        Ok(())
    }
    
    /// 保存缓存状态
    pub async fn save_state(&self) -> Result<(), String> {
        tokio::fs::create_dir_all(&self.embedding_dir).await
            .map_err(|e| format!("Failed to create embedding dir: {}", e))?;
        
        let state = IndexState {
            file_hashes: self.file_hashes.clone(),
            indexed_files: self.indexed_files.clone(),
            last_updated: Utc::now().to_rfc3339(),
        };
        
        let content = serde_json::to_string_pretty(&state)
            .map_err(|e| format!("Failed to serialize state: {}", e))?;
        
        let state_file = self.embedding_dir.join("index_state.json");
        tokio::fs::write(&state_file, content).await
            .map_err(|e| format!("Failed to write state: {}", e))?;
        
        Ok(())
    }
    
    /// 扫描所有目录
    #[allow(dead_code)]
    pub async fn scan_all(&mut self) -> Vec<Memory> {
        let mut memories = Vec::new();
        let mut dirs_to_scan: Vec<PathBuf> = self.paths.clone();
        
        while let Some(dir) = dirs_to_scan.pop() {
            if !dir.exists() || !dir.is_dir() {
                continue;
            }
            
            let mut entries = match tokio::fs::read_dir(&dir).await {
                Ok(entries) => entries,
                Err(_) => continue,
            };
            
            while let Ok(Some(entry)) = entries.next_entry().await {
                let path = entry.path();
                
                if path.is_dir() {
                    dirs_to_scan.push(path);
                } else if self.is_supported_file(&path) {
                    if let Some(memory) = self.index_file(&path).await {
                        memories.push(memory);
                    }
                }
            }
        }
        
        if self.dirty {
            if let Err(e) = self.save_state().await {
                tracing::error!("Failed to save state: {}", e);
            }
            self.dirty = false;
        }
        
        memories
    }
    
    /// 检查是否为支持的文件格式（排除 agent memory 文件）
    fn is_supported_file(&self, path: &Path) -> bool {
        if let Some(ext) = path.extension() {
            let ext = ext.to_string_lossy().to_lowercase();
            // 支持所有常见文档格式
            if !matches!(ext.as_str(), 
                // 纯文本/标记
                "md" | "txt" | "markdown" | "rst" | "asciidoc" | "adoc" |
                // 数据格式
                "json" | "yaml" | "yml" | "toml" | "csv" | "tsv" | "xml" |
                // Office 文档
                "docx" | "doc" | "xlsx" | "xls" | "pptx" | "ppt" |
                "odt" | "ods" | "odp" |
                // PDF
                "pdf" |
                // Web 格式
                "html" | "htm" | "mhtml" | "mht" |
                // 电子书
                "epub" |
                // 富文本
                "rtf" |
                // 代码/配置
                "ini" | "cfg" | "conf" | "properties" | "env" |
                "sh" | "bash" | "zsh" | "fish" | "ps1" | "bat" | "cmd" |
                // 其他常见格式
                "log" | "sql" | "graphql" | "proto"
            ) {
                return false;
            }
        } else {
            return false;
        }
        
        // 排除 agent memory 文件（MEMORY.md, USER.md 等）
        let file_name = path.file_name()
            .map(|n| n.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        
        if file_name == "memory.md" || file_name == "user.md" || file_name == "memory.md.lock" || file_name == "user.md.lock" {
            return false;
        }
        
        true
    }
    
    /// 索引单个文件
     pub async fn index_file(&mut self, path: &Path) -> Option<Memory> {
         let path_str = path.to_string_lossy().to_string();
         
         // 对于 docx/xlsx/pptx/pdf 等，先转换为 .md
         // 使用 spawn_blocking 避免阻塞 tokio runtime
         let read_path = if DocConverter::needs_conversion(path) {
             let path_clone = path.to_path_buf();
             let cache_dir_clone = self.cache_dir.clone();
             match tokio::task::spawn_blocking(move || {
                 let converter = DocConverter::new(cache_dir_clone);
                 converter.get_or_convert(&path_clone)
             }).await {
                 Ok(Ok(md_path)) => md_path,
                 Ok(Err(e)) => {
                     tracing::warn!("Failed to convert {}: {}", path_str, e);
                     return None;
                 }
                 Err(e) => {
                     tracing::warn!("Conversion task panicked for {}: {}", path_str, e);
                     return None;
                 }
             }
         } else {
             path.to_path_buf()
         };
        
        // 读取内容
        let content = tokio::fs::read_to_string(&read_path).await.ok()?;
        let hash = self.calculate_hash(&content);
        
        // 检查是否有变化
        if let Some(old_hash) = self.file_hashes.get(&path_str) {
            if old_hash == &hash {
                return None;
            }
        }
        
        self.file_hashes.insert(path_str.clone(), hash.clone());
        self.dirty = true;
        
        let (metadata, body) = self.parse_frontmatter(&content);
        let memory_type = self.infer_type_from_path(path);
        let title = self.extract_title(&metadata, &body, path);
        let mut tags = self.extract_tags(&metadata);
        let owner = metadata.get("owner").cloned();
        
        // 如果有 ChatService，用它提取 AI 关键词（轻 embedding）
        // 加 15s 超时，防止 API 调用卡死
        if let Some(ref chat) = self.chat_service {
            let extract_result = tokio::time::timeout(
                std::time::Duration::from_secs(15),
                chat.extract_keywords(&title, &body, 2000)
            ).await;
            match extract_result {
                Ok(Ok(ai_keywords)) => {
                    let ai_tags: Vec<String> = ai_keywords
                        .split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect();
                    for tag in ai_tags {
                        if !tags.contains(&tag) {
                            tags.push(tag);
                        }
                    }
                    tracing::info!("AI extracted {} keywords for {}", tags.len(), path_str);
                }
                Ok(Err(e)) => {
                    tracing::warn!("Failed to extract keywords for {}: {}", path_str, e);
                }
                Err(_) => {
                    tracing::warn!("AI keyword extraction timed out for {}", path_str);
                }
            }
        }
        
        let memory_id = self.indexed_files.get(&path_str).cloned()
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        self.indexed_files.insert(path_str.clone(), memory_id.clone());
        
        Some(Memory {
            id: memory_id,
            workspace_id: "default".to_string(),
            title,
            content: body,
            memory_type,
            scope: "project".to_string(),
            tags,
            source: MemorySource::KnowledgeFile { file_path: path_str },
            owner,
            confidence: 1.0,
            visibility: Visibility::Shared,
            status: "active".to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            last_used_at: None,
            expires_at: None,
            embedding: None,
            related_to: Vec::new(),
            derived_from: Vec::new(),
        })
    }
    
    fn calculate_hash(&self, content: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        format!("{:x}", hasher.finalize())
    }
    
    fn parse_frontmatter(&self, content: &str) -> (HashMap<String, String>, String) {
        let mut metadata = HashMap::new();
        let body;
        
        if content.starts_with("---") {
            if let Some(end) = content[3..].find("---") {
                let frontmatter = &content[3..end+3];
                body = content[end+6..].trim().to_string();
                
                for line in frontmatter.lines() {
                    if let Some(colon_pos) = line.find(':') {
                        let key = line[..colon_pos].trim().to_string();
                        let value = line[colon_pos+1..].trim().to_string();
                        metadata.insert(key, value);
                    }
                }
            } else {
                body = content.to_string();
            }
        } else {
            body = content.to_string();
        }
        
        (metadata, body)
    }
    
    fn infer_type_from_path(&self, path: &Path) -> MemoryType {
        let file_name = path.file_name()
            .map(|n| n.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        
        // 项目背景类：README、background、intro、overview
        if file_name.contains("readme") || file_name.contains("background") 
            || file_name.contains("intro") || file_name.contains("overview") 
            || file_name.contains("项目背景") || file_name.contains("背景") {
            return MemoryType::ProjectBackground;
        }
        
        // SOP 流程类：sop、process、guide、workflow、流程、步骤
        if file_name.contains("sop") || file_name.contains("process") 
            || file_name.contains("guide") || file_name.contains("workflow")
            || file_name.contains("流程") || file_name.contains("步骤")
            || file_name.contains("deploy") || file_name.contains("发布") {
            return MemoryType::SOP;
        }
        
        // 规则与标准类：rule、standard、style、规范、标准、规则
        if file_name.contains("rule") || file_name.contains("standard") 
            || file_name.contains("style") || file_name.contains("规范")
            || file_name.contains("标准") || file_name.contains("规则")
            || file_name.contains("convention") || file_name.contains("约定") {
            return MemoryType::RulesStandards;
        }
        
        // 模板类：template、模板
        if file_name.contains("template") || file_name.contains("模板") {
            return MemoryType::Template;
        }
        
        // 案例库类：case、example、案例、示例
        if file_name.contains("case") || file_name.contains("example")
            || file_name.contains("案例") || file_name.contains("示例")
            || file_name.contains("样本") {
            return MemoryType::CaseLibrary;
        }
        
        MemoryType::Semantic
    }
    
    fn extract_title(&self, metadata: &HashMap<String, String>, content: &str, path: &Path) -> String {
        if let Some(title) = metadata.get("title") {
            return title.clone();
        }
        
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("# ") {
                return trimmed[2..].trim().to_string();
            }
        }
        
        path.file_stem()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "Untitled".to_string())
    }
    
    fn extract_tags(&self, metadata: &HashMap<String, String>) -> Vec<String> {
        if let Some(tags_str) = metadata.get("tags") {
            tags_str.split(',').map(|t| t.trim().to_string()).filter(|t| !t.is_empty()).collect()
        } else {
            Vec::new()
        }
    }
    
    #[allow(dead_code)]
    pub fn stats(&self) -> serde_json::Value {
        serde_json::json!({
            "total_files": self.indexed_files.len(),
        })
    }
}
