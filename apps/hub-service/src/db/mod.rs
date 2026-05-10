use sqlx::sqlite::SqlitePool;
use crate::config::Config;

#[derive(Clone)]
pub struct Database {
    pub pool: SqlitePool,
    pub is_sqlite: bool,
}

impl Database {
    pub async fn new(config: &Config) -> anyhow::Result<Self> {
        let database_url = &config.database_url;
        
        // 移除sqlite:前缀
        let db_path = database_url.strip_prefix("sqlite:").unwrap_or(database_url);
        
        let pool = SqlitePool::connect(&format!("sqlite:{}?mode=rwc", db_path)).await?;
        
        // 运行迁移
        Self::run_migrations(&pool).await?;
        
        Ok(Self { pool, is_sqlite: true })
    }
    
    async fn run_migrations(pool: &SqlitePool) -> anyhow::Result<()> {
        // Memory table — REMOVED. All data now in doc + ext tables.
        // Drop if exists from legacy installations.
        sqlx::query("DROP TABLE IF EXISTS memory")
            .execute(pool).await?;
        
        // Knowledge Base Config 表
        sqlx::query(r#"
            CREATE TABLE IF NOT EXISTS knowledge_base_config (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                paths TEXT NOT NULL,
                file_patterns TEXT NOT NULL,
                auto_index INTEGER NOT NULL DEFAULT 1,
                auto_embed INTEGER NOT NULL DEFAULT 0,
                embedding_dir TEXT,
                last_indexed_at TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            )
        "#).execute(pool).await?;
        
        // Memory Candidate 表
        sqlx::query(r#"
            CREATE TABLE IF NOT EXISTS memory_candidate (
                id TEXT PRIMARY KEY,
                workspace_id TEXT NOT NULL DEFAULT 'default',
                title TEXT NOT NULL,
                content TEXT,
                proposed_type TEXT NOT NULL,
                proposed_scope TEXT NOT NULL,
                reason TEXT,
                source_agent_id TEXT,
                source_device_id TEXT,
                confidence REAL NOT NULL DEFAULT 0.5,
                review_status TEXT NOT NULL DEFAULT 'needs_review',
                review_notes TEXT,
                created_at TEXT NOT NULL,
                reviewed_at TEXT
            )
        "#).execute(pool).await?;
        
        // Event Log 表
        sqlx::query(r#"
            CREATE TABLE IF NOT EXISTS event_log (
                id TEXT PRIMARY KEY,
                workspace_id TEXT NOT NULL DEFAULT 'default',
                actor_type TEXT NOT NULL,
                actor_id TEXT,
                event_type TEXT NOT NULL,
                entity_type TEXT,
                entity_id TEXT,
                metadata TEXT,
                created_at TEXT NOT NULL
            )
        "#).execute(pool).await?;
        
        // Device 表
        sqlx::query(r#"
            CREATE TABLE IF NOT EXISTS device (
                id TEXT PRIMARY KEY,
                workspace_id TEXT NOT NULL DEFAULT 'default',
                name TEXT NOT NULL,
                device_type TEXT NOT NULL DEFAULT 'desktop',
                platform TEXT,
                os_version TEXT,
                app_version TEXT,
                device_token TEXT,
                invite_code TEXT,
                status TEXT NOT NULL DEFAULT 'pending',
                last_seen_at TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            )
        "#).execute(pool).await?;
        
        // Artifact 表
        sqlx::query(r#"
            CREATE TABLE IF NOT EXISTS artifact (
                id TEXT PRIMARY KEY,
                workspace_id TEXT NOT NULL DEFAULT 'default',
                title TEXT NOT NULL,
                artifact_type TEXT NOT NULL,
                content TEXT,
                content_path TEXT,
                mime_type TEXT,
                source_device_id TEXT,
                source_agent_id TEXT,
                tags TEXT DEFAULT '[]',
                status TEXT NOT NULL DEFAULT 'active',
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            )
        "#).execute(pool).await?;
        
        // Source File 表
        sqlx::query(r#"
            CREATE TABLE IF NOT EXISTS source_file (
                id TEXT PRIMARY KEY,
                workspace_id TEXT NOT NULL DEFAULT 'default',
                path TEXT NOT NULL,
                name TEXT NOT NULL,
                extension TEXT,
                size INTEGER,
                hash TEXT,
                mime_type TEXT,
                data_source_id TEXT,
                indexed_at TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            )
        "#).execute(pool).await?;
        
        // Document 表
        sqlx::query(r#"
            CREATE TABLE IF NOT EXISTS document (
                id TEXT PRIMARY KEY,
                workspace_id TEXT NOT NULL DEFAULT 'default',
                title TEXT NOT NULL,
                content TEXT,
                summary TEXT,
                doc_type TEXT NOT NULL DEFAULT 'text',
                source_file_id TEXT,
                memory_id TEXT,
                embedding TEXT,
                metadata TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            )
        "#).execute(pool).await?;
        
        // Data Source 表
        sqlx::query(r#"
            CREATE TABLE IF NOT EXISTS data_source (
                id TEXT PRIMARY KEY,
                workspace_id TEXT NOT NULL DEFAULT 'default',
                name TEXT NOT NULL,
                source_type TEXT NOT NULL DEFAULT 'folder',
                path TEXT,
                url TEXT,
                config TEXT,
                include_globs TEXT DEFAULT '["*"]',
                exclude_globs TEXT DEFAULT '[]',
                auto_index INTEGER NOT NULL DEFAULT 1,
                last_synced_at TEXT,
                status TEXT NOT NULL DEFAULT 'active',
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            )
        "#).execute(pool).await?;
        
        // Workspace 表
        sqlx::query(r#"
            CREATE TABLE IF NOT EXISTS workspace (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                description TEXT,
                owner_id TEXT,
                settings TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            )
        "#).execute(pool).await?;
        
        // 创建索引 (memory table indexes removed — table dropped)
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_device_status ON device(status)").execute(pool).await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_device_token ON device(device_token)").execute(pool).await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_artifact_type ON artifact(artifact_type)").execute(pool).await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_source_file_path ON source_file(path)").execute(pool).await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_document_type ON document(doc_type)").execute(pool).await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_data_source_type ON data_source(source_type)").execute(pool).await?;

        // ============================================================
        // Core + Extension Architecture Tables
        // ============================================================

        // Extend existing document table with new columns
        sqlx::query("ALTER TABLE document ADD COLUMN type TEXT NOT NULL DEFAULT 'knowledge'")
            .execute(pool).await.ok();
        sqlx::query("ALTER TABLE document ADD COLUMN content_hash TEXT")
            .execute(pool).await.ok();
        sqlx::query("ALTER TABLE document ADD COLUMN status TEXT DEFAULT 'raw'")
            .execute(pool).await.ok();
        sqlx::query("ALTER TABLE document ADD COLUMN quality_score REAL DEFAULT 0.0")
            .execute(pool).await.ok();
        sqlx::query("ALTER TABLE document ADD COLUMN source_type TEXT")
            .execute(pool).await.ok();
        sqlx::query("ALTER TABLE document ADD COLUMN source_path TEXT")
            .execute(pool).await.ok();

        // Doc table (new unified document model)
        sqlx::query(r#"
            CREATE TABLE IF NOT EXISTS doc (
                id TEXT PRIMARY KEY,
                type TEXT NOT NULL DEFAULT 'knowledge',
                title TEXT,
                content TEXT,
                content_hash TEXT,
                status TEXT DEFAULT 'active',
                quality_score REAL DEFAULT 0.0,
                source_type TEXT,
                source_path TEXT,
                embedding TEXT,
                metadata TEXT,
                created_at TEXT,
                updated_at TEXT
            )
        "#).execute(pool).await?;

        // Knowledge extension table
        sqlx::query(r#"
            CREATE TABLE IF NOT EXISTS knowledge_ext (
                id TEXT PRIMARY KEY,
                config_id TEXT,
                file_path TEXT,
                file_hash TEXT,
                version INTEGER DEFAULT 1,
                parent_id TEXT,
                is_current INTEGER DEFAULT 1,
                FOREIGN KEY (id) REFERENCES doc(id) ON DELETE CASCADE
            )
        "#).execute(pool).await?;

        // Memory extension table
        sqlx::query(r#"
            CREATE TABLE IF NOT EXISTS memory_ext (
                id TEXT PRIMARY KEY,
                agent_id TEXT,
                category TEXT,
                owner TEXT,
                visibility TEXT DEFAULT 'shared',
                FOREIGN KEY (id) REFERENCES doc(id) ON DELETE CASCADE
            )
        "#).execute(pool).await?;

        // Add missing columns to memory_ext (for records migrated from memory table)
        sqlx::query("ALTER TABLE memory_ext ADD COLUMN memory_type TEXT NOT NULL DEFAULT 'semantic'")
            .execute(pool).await.ok();
        sqlx::query("ALTER TABLE memory_ext ADD COLUMN scope TEXT NOT NULL DEFAULT 'project'")
            .execute(pool).await.ok();
        sqlx::query("ALTER TABLE memory_ext ADD COLUMN confidence REAL NOT NULL DEFAULT 0.8")
            .execute(pool).await.ok();

        // Session extension table
        sqlx::query(r#"
            CREATE TABLE IF NOT EXISTS session_ext (
                id TEXT PRIMARY KEY,
                agent_id TEXT,
                model TEXT,
                platform TEXT,
                message_count INTEGER DEFAULT 0,
                FOREIGN KEY (id) REFERENCES doc(id) ON DELETE CASCADE
            )
        "#).execute(pool).await?;

        // Indexes for doc table
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_doc_type ON doc(type)").execute(pool).await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_doc_status ON doc(status)").execute(pool).await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_doc_source_type ON doc(source_type)").execute(pool).await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_doc_content_hash ON doc(content_hash)").execute(pool).await?;

        // Indexes for knowledge_ext table
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_knowledge_ext_config_id ON knowledge_ext(config_id)").execute(pool).await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_knowledge_ext_file_path ON knowledge_ext(file_path)").execute(pool).await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_knowledge_ext_is_current ON knowledge_ext(is_current)").execute(pool).await?;

        // ============================================================
        // NOTE: Legacy memory → doc migration removed.
        // All data now writes directly to doc + ext tables.
        // The memory table is no longer used.
        // ============================================================

        // Ensure old memory/session records have status='active' (not 'raw')
        let fix_result = sqlx::query(
            "UPDATE doc SET status = 'active' WHERE status = 'raw' AND type IN ('memory', 'session')"
        ).execute(pool).await;
        if let Ok(r) = fix_result {
            if r.rows_affected() > 0 {
                tracing::info!("Fixed {} old records from status='raw' to 'active'", r.rows_affected());
            }
        }

        // ============================================================
        // One-time data fixes (tracked by migration_marker table)
        // ============================================================
        sqlx::query(r#"
            CREATE TABLE IF NOT EXISTS migration_marker (
                name TEXT PRIMARY KEY,
                applied_at TEXT NOT NULL
            )
        "#).execute(pool).await?;

        // Fix: session source_type correction (moved from main.rs)
        let fix_session_done: Option<(String,)> = sqlx::query_as(
            "SELECT name FROM migration_marker WHERE name = 'fix_session_source_types_20260506'"
        ).fetch_optional(pool).await.unwrap_or(None);

        if fix_session_done.is_none() {
            let mut fixed_total = 0i64;

            // Hermes sessions: title matches date pattern
            match sqlx::query(
                "UPDATE doc SET source_type = 'hermes_session' WHERE source_type = 'hermes' AND type = 'session' AND title GLOB '20[0-9][0-9]*'"
            ).execute(pool).await {
                Ok(r) if r.rows_affected() > 0 => {
                    fixed_total += r.rows_affected() as i64;
                    tracing::info!("Fixed {} hermes session records", r.rows_affected());
                }
                _ => {}
            }

            // Gemini/Codex sessions: title contains "Session"
            for (old_source, new_source, title_pattern) in [
                ("gemini", "gemini_session", "%Session%"),
                ("codex", "codex_session", "%Session%"),
            ] {
                match sqlx::query("UPDATE doc SET source_type = ? WHERE source_type = ? AND type = 'session' AND title LIKE ?")
                    .bind(new_source).bind(old_source).bind(title_pattern)
                    .execute(pool).await
                {
                    Ok(r) if r.rows_affected() > 0 => {
                        fixed_total += r.rows_affected() as i64;
                        tracing::info!("Fixed {} {} session records -> {}", r.rows_affected(), old_source, new_source);
                    }
                    Err(e) => tracing::warn!("Failed to fix session source_types for {}: {}", old_source, e),
                    _ => {}
                }
            }

            if fixed_total > 0 {
                tracing::info!("Total session source_type fixes: {}", fixed_total);
            }
            let _ = sqlx::query("INSERT OR IGNORE INTO migration_marker (name, applied_at) VALUES ('fix_session_source_types_20260506', ?)")
                .bind(chrono::Utc::now().to_rfc3339())
                .execute(pool).await;
        }

        tracing::info!("All migrations and data fixes completed");
        Ok(())
    }
}
