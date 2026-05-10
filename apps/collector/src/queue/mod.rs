pub mod pending;

use sqlx::{sqlite::SqlitePoolOptions, Pool, Sqlite};
use anyhow::Result;

#[derive(Clone)]
pub struct Queue {
    pool: Pool<Sqlite>,
}

impl Queue {
    pub async fn new(data_dir: &str) -> Result<Self> {
        let db_path = format!("{}/pending.sqlite", data_dir);
        let pool = SqlitePoolOptions::new()
            .connect(&format!("sqlite:{}", db_path))
            .await?;
        
        // 创建表
        sqlx::query(r#"
            CREATE TABLE IF NOT EXISTS pending_events (
                id TEXT PRIMARY KEY,
                event_type TEXT NOT NULL,
                path TEXT NOT NULL,
                source_id TEXT,
                device_id TEXT,
                size_bytes INTEGER,
                mtime TEXT,
                sha256 TEXT,
                status TEXT NOT NULL DEFAULT 'pending',
                retry_count INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL,
                uploaded_at TEXT,
                failed_at TEXT
            )
        "#)
        .execute(&pool)
        .await?;
        
        Ok(Self { pool })
    }
    
    pub async fn push(&self, event: PendingEvent) -> Result<()> {
        sqlx::query("INSERT INTO pending_events (id, event_type, path, source_id, device_id, size_bytes, mtime, sha256, status, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, 'pending', ?)")
            .bind(&event.id)
            .bind(&event.event_type)
            .bind(&event.path)
            .bind(&event.source_id)
            .bind(&event.device_id)
            .bind(event.size_bytes)
            .bind(&event.mtime)
            .bind(&event.sha256)
            .bind(&event.created_at)
            .execute(&self.pool)
            .await?;
        
        Ok(())
    }
    
    pub async fn get_pending(&self, limit: i64) -> Result<Vec<PendingEvent>> {
        let events = sqlx::query_as::<_, (String, String, String, Option<String>, Option<String>, Option<i64>, Option<String>, Option<String>, String)>(
            "SELECT id, event_type, path, source_id, device_id, size_bytes, mtime, sha256, created_at FROM pending_events WHERE status = 'pending' ORDER BY created_at LIMIT ?"
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        
        Ok(events.into_iter().map(|(id, event_type, path, source_id, device_id, size_bytes, mtime, sha256, created_at)| {
            PendingEvent {
                id,
                event_type,
                path,
                source_id,
                device_id,
                size_bytes,
                mtime,
                sha256,
                created_at,
            }
        }).collect())
    }
    
    pub async fn mark_uploaded(&self, id: &str) -> Result<()> {
        let now = chrono::Utc::now().to_rfc3339();
        sqlx::query("UPDATE pending_events SET status = 'uploaded', uploaded_at = ? WHERE id = ?")
            .bind(&now)
            .bind(id)
            .execute(&self.pool)
            .await?;
        
        Ok(())
    }
    
    pub async fn mark_failed(&self, id: &str) -> Result<()> {
        let now = chrono::Utc::now().to_rfc3339();
        sqlx::query("UPDATE pending_events SET status = 'failed', failed_at = ? WHERE id = ?")
            .bind(&now)
            .bind(id)
            .execute(&self.pool)
            .await?;
        
        Ok(())
    }
    
    pub async fn get_retry_count(&self, id: &str) -> Result<i32> {
        let count = sqlx::query_scalar::<_, i32>("SELECT retry_count FROM pending_events WHERE id = ?")
            .bind(id)
            .fetch_one(&self.pool)
            .await?;
        
        Ok(count)
    }
    
    pub async fn increment_retry(&self, id: &str) -> Result<()> {
        sqlx::query("UPDATE pending_events SET retry_count = retry_count + 1 WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        
        Ok(())
    }
    
    pub async fn get_pending_count(&self) -> Result<i64> {
        let count = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM pending_events WHERE status = 'pending'")
            .fetch_one(&self.pool)
            .await?;
        
        Ok(count)
    }
}

#[derive(Debug, Clone)]
pub struct PendingEvent {
    pub id: String,
    pub event_type: String,
    pub path: String,
    pub source_id: Option<String>,
    pub device_id: Option<String>,
    pub size_bytes: Option<i64>,
    pub mtime: Option<String>,
    pub sha256: Option<String>,
    pub created_at: String,
}
