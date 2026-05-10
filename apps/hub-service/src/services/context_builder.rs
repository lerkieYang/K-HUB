use crate::db::Database;
use serde_json::{json, Value};

#[allow(dead_code)]
pub struct ContextBuilder {
    db: Database,
}

#[allow(dead_code)]
impl ContextBuilder {
    pub fn new(db: Database) -> Self {
        Self { db }
    }
    
    pub async fn build(&self, task: &str, agent_id: &str, _max_tokens: i64) -> Value {
        let request_id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now();
        let expires_at = (now + chrono::Duration::hours(1)).to_rfc3339();
        
        // 搜索相关memory
        let memories = self.search_memories(task, 10).await;
        
        // 搜索相关文档
        let docs = self.search_docs(task, 5).await;
        
        // 获取规则和模板
        let rules = self.get_rules().await;
        let warnings = self.get_warnings(task).await;
        
        json!({
            "request_id": request_id,
            "agent_id": agent_id,
            "task_summary": task,
            "matched_intent": self.detect_intent(task),
            "context_pack": {
                "relevant_memory": memories,
                "relevant_docs": docs,
                "templates": [],
                "sop_checklist": [],
                "rules": rules,
                "warnings": warnings,
                "forbidden_actions": []
            },
            "sources": [],
            "confidence": 0.85,
            "expires_at": expires_at
        })
    }
    
    async fn search_memories(&self, query: &str, limit: i64) -> Vec<Value> {
        let memories =
            sqlx::query_as::<_, (String, String, String, Option<String>, f64)>(
                "SELECT d.id, COALESCE(me.scope, 'project'), COALESCE(me.memory_type, 'semantic'), d.content, COALESCE(me.confidence, 0.8) FROM doc d LEFT JOIN memory_ext me ON d.id = me.id WHERE d.status = 'active' AND d.content LIKE ? ORDER BY COALESCE(me.confidence, 0.8) DESC LIMIT ?"
            )
            .bind(format!("%{}%", query))
            .bind(limit)
            .fetch_all(&self.db.pool)
            .await;

        memories.unwrap_or_default().into_iter().map(|(id, _scope, typ, content, confidence)| {
            json!({
                "memory_id": id,
                "type": typ,
                "content": content,
                "confidence": confidence
            })
        }).collect()
    }
    
    async fn search_docs(&self, query: &str, limit: i64) -> Vec<Value> {
        let docs =
            sqlx::query_as::<_, (String, Option<String>, Option<String>)>(
                "SELECT id, title, summary FROM document WHERE title LIKE ? OR summary LIKE ? LIMIT ?"
            )
            .bind(format!("%{}%", query))
            .bind(format!("%{}%", query))
            .bind(limit)
            .fetch_all(&self.db.pool)
            .await;
        
        docs.unwrap_or_default().into_iter().map(|(id, title, summary)| {
            json!({
                "doc_id": id,
                "title": title,
                "excerpt": summary
            })
        }).collect()
    }
    
    async fn get_rules(&self) -> Vec<String> {
        vec![
            "Agent不得直接写正式memory".to_string(),
            "所有候选memory必须进入Review Queue".to_string(),
        ]
    }
    
    async fn get_warnings(&self, task: &str) -> Vec<String> {
        let mut warnings = Vec::new();
        
        if task.contains("SMB") || task.contains("smb") {
            warnings.push("SMB文件监听可能漏报，必须使用定时扫描兜底".to_string());
        }
        
        warnings
    }
    
    fn detect_intent(&self, task: &str) -> String {
        let task_lower = task.to_lowercase();
        
        if task_lower.contains("设计") || task_lower.contains("design") {
            "technical_design".to_string()
        } else if task_lower.contains("实现") || task_lower.contains("implement") {
            "implementation".to_string()
        } else if task_lower.contains("测试") || task_lower.contains("test") {
            "testing".to_string()
        } else if task_lower.contains("部署") || task_lower.contains("deploy") {
            "deployment".to_string()
        } else {
            "general".to_string()
        }
    }
}
