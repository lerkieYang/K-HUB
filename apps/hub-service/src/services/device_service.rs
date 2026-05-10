use crate::db::Database;

#[allow(dead_code)]
pub struct DeviceService {
    db: Database,
}

#[allow(dead_code)]
impl DeviceService {
    pub fn new(db: Database) -> Self {
        Self { db }
    }
    
    pub async fn get_online_count(&self) -> i64 {
        let count = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM device WHERE status IN ('online', 'approved')")
            .fetch_one(&self.db.pool)
            .await;
        
        count.unwrap_or(0)
    }
    
    pub async fn get_pending_approvals(&self) -> i64 {
        let count = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM device WHERE status = 'pending'")
            .fetch_one(&self.db.pool)
            .await;
        
        count.unwrap_or(0)
    }
    
    pub async fn check_heartbeat_timeout(&self) {
        let timeout = chrono::Utc::now() - chrono::Duration::minutes(5);
        
        sqlx::query("UPDATE device SET status = 'offline' WHERE status = 'online' AND last_seen_at < ?")
            .bind(timeout.to_rfc3339())
            .execute(&self.db.pool)
            .await
            .ok();
    }
}
