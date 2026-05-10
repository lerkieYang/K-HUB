use chrono::Utc;
use crate::models::memory::{Memory, MemoryType, MemorySource, Visibility};

/// 活动记忆收集器
#[allow(dead_code)]
pub struct ActivityCollector {
    workspace_id: String,
}

#[allow(dead_code)]
impl ActivityCollector {
    pub fn new(workspace_id: String) -> Self {
        Self { workspace_id }
    }
    
    /// 收集Agent对话记忆
    pub fn collect_agent_memory(
        &self,
        agent_id: &str,
        session_id: Option<String>,
        title: String,
        content: String,
        tags: Vec<String>,
    ) -> Memory {
        Memory {
            id: uuid::Uuid::new_v4().to_string(),
            workspace_id: self.workspace_id.clone(),
            title,
            content,
            memory_type: MemoryType::Episodic,
            scope: "session".to_string(),
            tags,
            source: MemorySource::Agent {
                session_id,
                agent_id: agent_id.to_string(),
            },
            owner: None,
            confidence: 0.8,
            visibility: Visibility::Agent,
            status: "active".to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            last_used_at: None,
            expires_at: None,
            embedding: None,
            related_to: Vec::new(),
            derived_from: Vec::new(),
        }
    }
    
    /// 收集决策记忆
    pub fn collect_decision(
        &self,
        title: String,
        content: String,
        reason: String,
        agent_id: Option<String>,
        tags: Vec<String>,
    ) -> Memory {
        let full_content = format!(
            "# {}\n\n{}\n\n## 决策原因\n{}",
            title, content, reason
        );
        
        Memory {
            id: uuid::Uuid::new_v4().to_string(),
            workspace_id: self.workspace_id.clone(),
            title,
            content: full_content,
            memory_type: MemoryType::Decision,
            scope: "project".to_string(),
            tags,
            source: MemorySource::Agent {
                session_id: None,
                agent_id: agent_id.unwrap_or_else(|| "unknown".to_string()),
            },
            owner: None,
            confidence: 0.9,
            visibility: Visibility::Project,
            status: "active".to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            last_used_at: None,
            expires_at: None,
            embedding: None,
            related_to: Vec::new(),
            derived_from: Vec::new(),
        }
    }
    
    /// 收集Git提交记忆
    pub fn collect_git_commit(
        &self,
        commit_hash: String,
        message: String,
        author: String,
        files_changed: Vec<String>,
    ) -> Memory {
        let title = format!("Git: {}", message.lines().next().unwrap_or("No message"));
        let content = format!(
            "# Git Commit\n\n**Hash:** {}\n**Author:** {}\n**Message:**\n{}\n\n**Files Changed:**\n{}",
            commit_hash,
            author,
            message,
            files_changed.iter().map(|f| format!("- {}", f)).collect::<Vec<_>>().join("\n")
        );
        
        Memory {
            id: uuid::Uuid::new_v4().to_string(),
            workspace_id: self.workspace_id.clone(),
            title,
            content,
            memory_type: MemoryType::Episodic,
            scope: "project".to_string(),
            tags: vec!["git".to_string(), "commit".to_string()],
            source: MemorySource::Git {
                commit_hash: Some(commit_hash),
            },
            owner: Some(author),
            confidence: 1.0,
            visibility: Visibility::Project,
            status: "active".to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            last_used_at: None,
            expires_at: None,
            embedding: None,
            related_to: Vec::new(),
            derived_from: Vec::new(),
        }
    }
    
    /// 收集浏览器历史记忆
    pub fn collect_browser_visit(
        &self,
        url: String,
        title: String,
        content_excerpt: Option<String>,
    ) -> Memory {
        let content = format!(
            "# 浏览: {}\n\n**URL:** {}\n\n{}",
            title,
            url,
            content_excerpt.unwrap_or_default()
        );
        
        Memory {
            id: uuid::Uuid::new_v4().to_string(),
            workspace_id: self.workspace_id.clone(),
            title: format!("浏览: {}", title),
            content,
            memory_type: MemoryType::Episodic,
            scope: "session".to_string(),
            tags: vec!["browser".to_string(), "web".to_string()],
            source: MemorySource::Browser {
                url: Some(url),
            },
            owner: None,
            confidence: 0.6,
            visibility: Visibility::Private,
            status: "active".to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            last_used_at: None,
            expires_at: None,
            embedding: None,
            related_to: Vec::new(),
            derived_from: Vec::new(),
        }
    }
    
    /// 收集聊天记录记忆
    pub fn collect_chat_message(
        &self,
        platform: &str,  // wechat/feishu/telegram
        message_id: Option<String>,
        sender: String,
        content: String,
        chat_name: Option<String>,
    ) -> Memory {
        let title = format!("{}: {}", platform, chat_name.unwrap_or_else(|| sender.clone()));
        
        let source = match platform {
            "wechat" => MemorySource::Wechat { message_id },
            "feishu" => MemorySource::Feishu { message_id },
            _ => MemorySource::Manual,
        };
        
        Memory {
            id: uuid::Uuid::new_v4().to_string(),
            workspace_id: self.workspace_id.clone(),
            title,
            content,
            memory_type: MemoryType::Episodic,
            scope: "session".to_string(),
            tags: vec![platform.to_string(), "chat".to_string()],
            source,
            owner: Some(sender),
            confidence: 0.7,
            visibility: Visibility::Private,
            status: "active".to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            last_used_at: None,
            expires_at: None,
            embedding: None,
            related_to: Vec::new(),
            derived_from: Vec::new(),
        }
    }
    
    /// 收集偏好设置记忆
    pub fn collect_preference(
        &self,
        key: String,
        value: String,
        context: Option<String>,
    ) -> Memory {
        let content = format!(
            "# 偏好: {}\n\n**值:** {}\n\n{}",
            key,
            value,
            context.map(|c| format!("**上下文:**\n{}", c)).unwrap_or_default()
        );
        
        Memory {
            id: uuid::Uuid::new_v4().to_string(),
            workspace_id: self.workspace_id.clone(),
            title: format!("偏好: {}", key),
            content,
            memory_type: MemoryType::Preference,
            scope: "global".to_string(),
            tags: vec!["preference".to_string(), key.clone()],
            source: MemorySource::Manual,
            owner: None,
            confidence: 1.0,
            visibility: Visibility::Private,
            status: "active".to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            last_used_at: None,
            expires_at: None,
            embedding: None,
            related_to: Vec::new(),
            derived_from: Vec::new(),
        }
    }
}
