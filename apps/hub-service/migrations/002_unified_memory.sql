-- Knowledge Hub 数据库迁移脚本
-- 版本: V2 - 统一Memory架构（性能优化版 + 自动向量化开关）

-- ============================================
-- 1. 更新Memory表结构
-- ============================================

CREATE TABLE IF NOT EXISTS memory (
    id TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL DEFAULT 'default',
    
    -- 核心内容
    title TEXT NOT NULL,
    content TEXT,
    
    -- 分类
    memory_type TEXT NOT NULL CHECK (memory_type IN (
        'project_background', 'sop', 'style_guide', 'template', 'case_library',
        'decision', 'episodic', 'semantic', 'procedural', 'preference', 'warning'
    )),
    scope TEXT NOT NULL CHECK (scope IN ('global', 'project', 'agent', 'device', 'session')),
    tags TEXT DEFAULT '[]',
    
    -- 来源
    source_type TEXT NOT NULL CHECK (source_type IN ('knowledge', 'agent', 'wechat', 'feishu', 'browser', 'git', 'manual')),
    source_file_path TEXT,
    source_agent_id TEXT,
    source_session_id TEXT,
    source_url TEXT,
    source_commit_hash TEXT,
    source_message_id TEXT,
    
    -- 元数据
    owner TEXT,
    confidence REAL NOT NULL DEFAULT 0.5,
    visibility TEXT NOT NULL CHECK (visibility IN ('private', 'agent', 'project', 'shared')),
    status TEXT NOT NULL CHECK (status IN ('active', 'deprecated', 'archived', 'conflict')),
    
    -- 时间
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    last_used_at TEXT,
    expires_at TEXT,
    
    -- 向量embedding (JSON数组)
    embedding TEXT,
    
    -- 关联
    related_to TEXT DEFAULT '[]',
    derived_from TEXT DEFAULT '[]'
);

-- 创建索引
CREATE INDEX IF NOT EXISTS idx_memory_type ON memory(memory_type);
CREATE INDEX IF NOT EXISTS idx_memory_source ON memory(source_type);
CREATE INDEX IF NOT EXISTS idx_memory_status ON memory(status);
CREATE INDEX IF NOT EXISTS idx_memory_scope ON memory(scope);
CREATE INDEX IF NOT EXISTS idx_memory_file ON memory(source_file_path);
CREATE INDEX IF NOT EXISTS idx_memory_updated ON memory(updated_at);

-- ============================================
-- 2. FTS5全文搜索索引
-- ============================================

CREATE VIRTUAL TABLE IF NOT EXISTS memory_fts USING fts5(
    id,
    title,
    content,
    tags,
    content_rowid='rowid'
);

-- FTS同步触发器
CREATE TRIGGER IF NOT EXISTS memory_ai AFTER INSERT ON memory BEGIN
    INSERT INTO memory_fts(id, title, content, tags) VALUES (new.id, new.title, new.content, new.tags);
END;

CREATE TRIGGER IF NOT EXISTS memory_ad AFTER DELETE ON memory BEGIN
    INSERT INTO memory_fts(memory_fts, id, title, content, tags) VALUES('delete', old.id, old.title, old.content, old.tags);
END;

CREATE TRIGGER IF NOT EXISTS memory_au AFTER UPDATE ON memory BEGIN
    INSERT INTO memory_fts(memory_fts, id, title, content, tags) VALUES('delete', old.id, old.title, old.content, old.tags);
    INSERT INTO memory_fts(id, title, content, tags) VALUES (new.id, new.title, new.content, new.tags);
END;

-- ============================================
-- 3. 知识库配置表（新增auto_embed字段）
-- ============================================

CREATE TABLE IF NOT EXISTS knowledge_base_config (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    paths TEXT NOT NULL,
    file_patterns TEXT NOT NULL,
    auto_index INTEGER NOT NULL DEFAULT 1,
    auto_embed INTEGER NOT NULL DEFAULT 0,  -- 自动向量化开关，默认关闭
    embedding_dir TEXT,                      -- 向量化产出目录
    last_indexed_at TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- ============================================
-- 4. 知识库索引缓存表
-- ============================================

CREATE TABLE IF NOT EXISTS knowledge_index_cache (
    id TEXT PRIMARY KEY,
    config_id TEXT NOT NULL,
    file_path TEXT NOT NULL,
    file_hash TEXT NOT NULL,
    memory_id TEXT,
    has_embedding INTEGER NOT NULL DEFAULT 0,
    indexed_at TEXT NOT NULL,
    FOREIGN KEY (config_id) REFERENCES knowledge_base_config(id)
);

CREATE INDEX IF NOT EXISTS idx_index_cache_config ON knowledge_index_cache(config_id);
CREATE INDEX IF NOT EXISTS idx_index_cache_path ON knowledge_index_cache(file_path);

-- ============================================
-- 5. Embedding缓存表
-- ============================================

CREATE TABLE IF NOT EXISTS embedding_cache (
    id TEXT PRIMARY KEY,
    text_hash TEXT NOT NULL UNIQUE,
    text_preview TEXT,
    embedding TEXT NOT NULL,
    model TEXT NOT NULL,
    created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_embedding_hash ON embedding_cache(text_hash);

-- ============================================
-- 6. Memory候选表
-- ============================================

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
    source_artifact_id TEXT,
    confidence REAL NOT NULL DEFAULT 0.5,
    review_status TEXT NOT NULL CHECK (review_status IN ('candidate', 'needs_review', 'approved', 'rejected', 'merged', 'auto_rejected')),
    review_notes TEXT,
    created_at TEXT NOT NULL,
    reviewed_at TEXT
);

-- ============================================
-- 7. 事件日志表
-- ============================================

CREATE TABLE IF NOT EXISTS event_log (
    id TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL DEFAULT 'default',
    device_id TEXT,
    actor_type TEXT NOT NULL CHECK (actor_type IN ('user', 'device', 'agent', 'system')),
    actor_id TEXT,
    event_type TEXT NOT NULL,
    entity_type TEXT,
    entity_id TEXT,
    metadata TEXT,
    created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_event_created ON event_log(created_at);
CREATE INDEX IF NOT EXISTS idx_event_type ON event_log(event_type);

-- ============================================
-- 8. 插入默认知识库配置
-- ============================================

INSERT OR IGNORE INTO knowledge_base_config (id, name, paths, file_patterns, auto_index, auto_embed, created_at, updated_at)
VALUES (
    'default',
    '默认知识库',
    '[]',
    '["*.md", "*.txt", "*.markdown"]',
    1,
    0,  -- 默认关闭自动向量化
    datetime('now'),
    datetime('now')
);
