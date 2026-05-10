use crate::db::Database;
use serde_json::{json, Value};

pub struct MemoryService {
    db: Database,
}

impl MemoryService {
    pub fn new(db: Database) -> Self {
        Self { db }
    }
    
    pub async fn search(&self, query: &str, limit: i64) -> Vec<Value> {
        let memories =
            sqlx::query_as::<_, (String, String, String, Option<String>, f64)>(
                "SELECT d.id, COALESCE(me.scope, 'project'), COALESCE(me.memory_type, 'semantic'), d.content, COALESCE(me.confidence, 0.8) FROM doc d LEFT JOIN memory_ext me ON d.id = me.id WHERE d.status = 'active' AND d.content LIKE ? ORDER BY COALESCE(me.confidence, 0.8) DESC LIMIT ?"
            )
            .bind(format!("%{}%", query))
            .bind(limit)
            .fetch_all(&self.db.pool)
            .await;
        
        memories.unwrap_or_default().into_iter().map(|(id, scope, memory_type, content, confidence)| {
            json!({
                "id": id,
                "scope": scope,
                "memory_type": memory_type,
                "content": content,
                "confidence": confidence
            })
        }).collect()
    }
    
    pub async fn get_active_count(&self) -> i64 {
        let count =
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM doc WHERE status = 'active'")
                .fetch_one(&self.db.pool)
                .await;
        
        count.unwrap_or(0)
    }
    
    pub async fn get_pending_count(&self) -> i64 {
        let count =
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM memory_candidate WHERE review_status IN ('candidate', 'needs_review')")
                .fetch_one(&self.db.pool)
                .await;
        
        count.unwrap_or(0)
    }
}
