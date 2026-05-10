use axum::{routing::{get, post}, Router, Json, extract::{State, Query, Path}};
use serde_json::{json, Value};
use crate::db::Database;
use crate::models::memory::{CreateMemoryRequest, UpdateMemoryRequest, MemorySearchParams, Visibility};

/// Escape special characters for SQLite LIKE patterns (% and _).
fn escape_like(s: &str) -> String {
    s.replace('%', "\\%").replace('_', "\\_")
}

/// Map doc.type + doc.source_type to the legacy source_type string for frontend compatibility.
fn doc_type_to_source_type(doc_type: &str, doc_source_type: &str) -> String {
    match doc_type {
        "knowledge" => "knowledge".to_string(),
        "session" => doc_source_type.to_string(), // hermes_session, codex_session, etc.
        _ => doc_source_type.to_string(),          // hermes, openclaw, codex, gemini, manual, etc.
    }
}

pub fn routes() -> Router<Database> {
    Router::new()
        .route("/api/memory", get(list_memory).post(create_memory))
        .route("/api/memory/search", get(search_memory))
        .route("/api/memory/stats", get(memory_stats))
        .route("/api/memory/batch-delete", post(batch_delete_memory))
        .route("/api/memory/candidates", get(list_candidates))
        .route("/api/memory/candidates/:id/approve", post(approve_candidate))
        .route("/api/memory/candidates/:id/reject", post(reject_candidate))
        .route("/api/memory/:id", get(get_memory).put(update_memory).delete(delete_memory))
}

/// 列出所有Memory (from doc table)
async fn list_memory(
    State(db): State<Database>,
    Query(params): Query<MemorySearchParams>,
) -> Json<Value> {
    let limit = params.limit.unwrap_or(100);
    let offset = params.offset.unwrap_or(0);
    let source_type_filter = params.source_type.as_deref();

    // Build WHERE clause based on source_type filter
    // source_type filter maps to doc.type + doc.source_type
    let (where_clause, binds): (String, Vec<String>) = if let Some(st) = source_type_filter {
        if st == "knowledge" {
            ("d.type = 'knowledge'".to_string(), vec![])
        } else if st.ends_with("_session") {
            ("d.type = 'session' AND d.source_type = ?".to_string(), vec![st.to_string()])
        } else {
            ("d.type = 'memory' AND d.source_type = ?".to_string(), vec![st.to_string()])
        }
    } else {
        // Default: memory type only (exclude knowledge and sessions)
        ("d.type = 'memory'".to_string(), vec![])
    };

    let sql = format!(
        r#"SELECT d.id, d.title, SUBSTR(d.content, 1, 200),
           COALESCE(me.memory_type, 'semantic'), COALESCE(me.scope, 'project'), COALESCE(d.metadata, '[]'),
           d.source_type, COALESCE(me.confidence, 0.8), COALESCE(me.visibility, 'private'),
           d.status, d.created_at, d.updated_at
           FROM doc d LEFT JOIN memory_ext me ON d.id = me.id
           WHERE d.status = 'active' AND {}
           ORDER BY d.updated_at DESC LIMIT ? OFFSET ?"#,
        where_clause
    );

    let mut query = sqlx::query_as::<_, (String, String, String, String, String, String, String, f64, String, String, String, String)>(&sql);
    for bind in &binds {
        query = query.bind(bind);
    }
    query = query.bind(limit).bind(offset);
    let memories = query.fetch_all(&db.pool).await;

    match memories {
        Ok(rows) => {
            let items: Vec<Value> = rows.into_iter().map(|(id, title, content_preview, memory_type, scope, tags, source_type, confidence, visibility, status, created_at, updated_at)| {
                let tags_vec: Vec<String> = serde_json::from_str(&tags).unwrap_or_default();
                json!({
                    "id": id,
                    "title": title,
                    "content_preview": content_preview,
                    "memory_type": memory_type,
                    "scope": scope,
                    "tags": tags_vec,
                    "source_type": source_type,
                    "confidence": confidence,
                    "visibility": visibility,
                    "status": status,
                    "created_at": created_at,
                    "updated_at": updated_at
                })
            }).collect();

            let count_sql = format!(
                "SELECT COUNT(*) FROM doc d WHERE d.status = 'active' AND {}",
                where_clause
            );
            let mut count_query = sqlx::query_scalar::<_, i64>(&count_sql);
            for bind in &binds {
                count_query = count_query.bind(bind);
            }
            let total: i64 = count_query.fetch_one(&db.pool).await.unwrap_or(0);

            Json(json!({"memories": items, "total": total}))
        },
        Err(e) => Json(json!({"error": e.to_string()})),
    }
}

/// 创建Memory (into doc + memory_ext)
async fn create_memory(
    State(db): State<Database>,
    Json(req): Json<CreateMemoryRequest>,
) -> Json<Value> {
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    let scope = req.scope.unwrap_or_else(|| "project".to_string());
    let confidence = req.confidence.unwrap_or(0.8);
    let visibility = req.visibility.unwrap_or(Visibility::Private).as_str().to_string();
    let tags = serde_json::to_string(&req.tags.unwrap_or_default()).unwrap_or_else(|_| "[]".to_string());
    let memory_type = req.memory_type.as_str();
    let source_type = req.source_type.clone()
        .or_else(|| req.source.as_ref().map(|s| s.source_type().to_string()))
        .unwrap_or_else(|| "manual".to_string());

    // Insert into doc table
    let result = sqlx::query(
        r#"INSERT INTO doc (id, type, title, content, source_type, status, metadata, created_at, updated_at)
           VALUES (?, 'memory', ?, ?, ?, 'active', ?, ?, ?)"#
    )
    .bind(&id).bind(&req.title).bind(&req.content)
    .bind(&source_type).bind(&tags).bind(&now).bind(&now)
    .execute(&db.pool).await;

    if let Err(e) = result {
        return Json(json!({"error": e.to_string()}));
    }

    // Insert into memory_ext
    let _ = sqlx::query(
        r#"INSERT INTO memory_ext (id, memory_type, scope, confidence, visibility, owner)
           VALUES (?, ?, ?, ?, ?, ?)"#
    )
    .bind(&id).bind(memory_type).bind(&scope)
    .bind(confidence).bind(&visibility).bind("unknown")
    .execute(&db.pool).await;

    Json(json!({"id": id, "title": req.title, "status": "active"}))
}

/// 获取单个Memory (from doc + memory_ext)
async fn get_memory(
    State(db): State<Database>,
    Path(id): Path<String>,
) -> Json<Value> {
    let memory = sqlx::query_as::<_, (String, String, String, String, String, String, String, f64, String, String, String, String)>(
        r#"SELECT d.id, d.title, d.content,
           COALESCE(me.memory_type, 'semantic'), COALESCE(me.scope, 'project'), COALESCE(d.metadata, '[]'),
           d.source_type, COALESCE(me.confidence, 0.8), COALESCE(me.visibility, 'private'),
           d.status, d.created_at, d.updated_at
           FROM doc d LEFT JOIN memory_ext me ON d.id = me.id
           WHERE d.id = ?"#
    )
    .bind(&id)
    .fetch_one(&db.pool)
    .await;

    match memory {
        Ok((id, title, content, memory_type, scope, tags, source_type, confidence, visibility, status, created_at, updated_at)) => {
            let tags_vec: Vec<String> = serde_json::from_str(&tags).unwrap_or_default();
            Json(json!({
                "id": id, "title": title, "content": content,
                "memory_type": memory_type, "scope": scope, "tags": tags_vec,
                "source_type": source_type, "confidence": confidence, "visibility": visibility,
                "status": status, "created_at": created_at, "updated_at": updated_at
            }))
        },
        Err(_) => Json(json!({"error": "Memory not found"})),
    }
}

/// 更新Memory (update doc + memory_ext)
async fn update_memory(
    State(db): State<Database>,
    Path(id): Path<String>,
    Json(req): Json<UpdateMemoryRequest>,
) -> Json<Value> {
    let now = chrono::Utc::now().to_rfc3339();

    // Update doc table fields
    let mut doc_sets: Vec<String> = Vec::new();
    let mut bind_title: Option<String> = None;
    let mut bind_content: Option<String> = None;
    let mut bind_tags: Option<String> = None;
    let mut bind_status: Option<String> = None;

    if let Some(title) = &req.title {
        doc_sets.push("title = ?".to_string());
        bind_title = Some(title.clone());
    }
    if let Some(content) = &req.content {
        doc_sets.push("content = ?".to_string());
        bind_content = Some(content.clone());
    }
    if let Some(tags) = &req.tags {
        doc_sets.push("metadata = ?".to_string());
        bind_tags = Some(serde_json::to_string(tags).unwrap_or_else(|_| "[]".to_string()));
    }
    if let Some(status) = &req.status {
        doc_sets.push("status = ?".to_string());
        bind_status = Some(status.clone());
    }

    if !doc_sets.is_empty() {
        doc_sets.push("updated_at = ?".to_string());
        let sql = format!("UPDATE doc SET {} WHERE id = ?", doc_sets.join(", "));
        let mut query = sqlx::query(&sql);
        if let Some(v) = &bind_title { query = query.bind(v); }
        if let Some(v) = &bind_content { query = query.bind(v); }
        if let Some(v) = &bind_tags { query = query.bind(v); }
        if let Some(v) = &bind_status { query = query.bind(v); }
        query = query.bind(&now).bind(&id);
        if let Err(e) = query.execute(&db.pool).await {
            return Json(json!({"error": format!("Failed to update doc: {}", e)}));
        }
    }

    // Update memory_ext fields
    let mut ext_sets: Vec<String> = Vec::new();
    let mut bind_memory_type: Option<String> = None;
    let mut bind_scope: Option<String> = None;
    let mut bind_confidence: Option<f64> = None;
    let mut bind_visibility: Option<String> = None;
    let mut bind_owner: Option<String> = None;

    if let Some(memory_type) = &req.memory_type {
        ext_sets.push("memory_type = ?".to_string());
        bind_memory_type = Some(memory_type.as_str().to_string());
    }
    if let Some(scope) = &req.scope {
        ext_sets.push("scope = ?".to_string());
        bind_scope = Some(scope.clone());
    }
    if let Some(confidence) = req.confidence {
        ext_sets.push("confidence = ?".to_string());
        bind_confidence = Some(confidence);
    }
    if let Some(visibility) = &req.visibility {
        ext_sets.push("visibility = ?".to_string());
        bind_visibility = Some(visibility.as_str().to_string());
    }
    if let Some(owner) = &req.owner {
        ext_sets.push("owner = ?".to_string());
        bind_owner = Some(owner.clone());
    }

    if !ext_sets.is_empty() {
        let sql = format!("UPDATE memory_ext SET {} WHERE id = ?", ext_sets.join(", "));
        let mut query = sqlx::query(&sql);
        if let Some(v) = &bind_memory_type { query = query.bind(v); }
        if let Some(v) = &bind_scope { query = query.bind(v); }
        if let Some(v) = bind_confidence { query = query.bind(v); }
        if let Some(v) = &bind_visibility { query = query.bind(v); }
        if let Some(v) = &bind_owner { query = query.bind(v); }
        query = query.bind(&id);
        let _ = query.execute(&db.pool).await; // memory_ext may not exist yet
    }

    Json(json!({"success": true, "id": id}))
}

/// 删除Memory (update doc status)
async fn delete_memory(
    State(db): State<Database>,
    Path(id): Path<String>,
) -> Json<Value> {
    let now = chrono::Utc::now().to_rfc3339();

    let record_type: Option<String> = sqlx::query_scalar(
        "SELECT type FROM doc WHERE id = ?"
    ).bind(&id).fetch_one(&db.pool).await.ok();

    let result = sqlx::query("UPDATE doc SET status = 'deleted', updated_at = ? WHERE id = ?")
        .bind(&now).bind(&id).execute(&db.pool).await;

    if record_type.as_deref() == Some("knowledge") {
        let _ = sqlx::query("UPDATE knowledge_ext SET is_current = 0 WHERE id = ?")
            .bind(&id).execute(&db.pool).await;
    }

    match result {
        Ok(r) if r.rows_affected() == 0 => Json(json!({"error": "Memory not found"})),
        Ok(_) => Json(json!({"success": true})),
        Err(e) => Json(json!({"error": format!("Failed to delete memory: {}", e)})),
    }
}

/// 搜索Memory (from doc table)
async fn search_memory(
    State(db): State<Database>,
    Query(params): Query<MemorySearchParams>,
) -> Json<Value> {
    let query = params.query.unwrap_or_default();
    let limit = params.limit.unwrap_or(50);
    let source_type_filter = params.source_type.as_deref();

    let escaped = escape_like(&query);
    let like_query = format!("%{}%", escaped);

    // Build type filter clause
    let type_clause = if let Some(st) = source_type_filter {
        let types: Vec<&str> = st.split(',').collect();
        if types.len() == 1 {
            let t = types[0];
            if t == "knowledge" {
                "d.type = 'knowledge'".to_string()
            } else if t.ends_with("_session") {
                // Strip _session suffix: data stores 'openclaw' not 'openclaw_session'
                let agent = t.strip_suffix("_session").unwrap_or(t);
                format!("d.type = 'session' AND (d.source_type = '{}' OR d.source_type = '{}')", t, agent)
            } else {
                format!("d.type = 'memory' AND d.source_type = '{}'", t)
            }
        } else {
            // Multiple source types - build OR clause
            let clauses: Vec<String> = types.iter().map(|t| {
                if *t == "knowledge" {
                    "d.type = 'knowledge'".to_string()
                } else if t.ends_with("_session") {
                    let agent = t.strip_suffix("_session").unwrap_or(t);
                    format!("(d.type = 'session' AND (d.source_type = '{}' OR d.source_type = '{}'))", t, agent)
                } else {
                    format!("(d.type = 'memory' AND d.source_type = '{}')", t)
                }
            }).collect();
            format!("({})", clauses.join(" OR "))
        }
    } else {
        "1=1".to_string() // no filter
    };

    let sql = format!(
        r#"SELECT d.id, d.title, SUBSTR(d.content, 1, 500),
           COALESCE(me.memory_type, 'semantic'), COALESCE(me.scope, 'project'), COALESCE(d.metadata, '[]'),
           d.source_type, COALESCE(me.confidence, 0.8), COALESCE(me.visibility, 'private'),
           d.status, d.created_at, d.updated_at, COALESCE(d.source_path, ''), d.metadata
           FROM doc d LEFT JOIN memory_ext me ON d.id = me.id
           WHERE d.status = 'active' AND {} AND (d.title LIKE ? ESCAPE '\' OR d.content LIKE ? ESCAPE '\' OR d.metadata LIKE ? ESCAPE '\')
           ORDER BY d.updated_at DESC LIMIT ?"#,
        type_clause
    );

    let memories = sqlx::query_as::<_, (String, String, String, String, String, String, String, f64, String, String, String, String, String, Option<String>)>(&sql)
        .bind(&like_query).bind(&like_query).bind(&like_query).bind(limit)
        .fetch_all(&db.pool).await;

    match memories {
        Ok(rows) => {
            let results: Vec<Value> = rows.into_iter().map(|(id, title, content_preview, memory_type, scope, tags, source_type, confidence, visibility, status, created_at, updated_at, source_file_path, metadata)| {
                let tags_vec: Vec<String> = serde_json::from_str(&tags).unwrap_or_default();
                json!({
                    "id": id, "title": title, "content": content_preview, "content_preview": content_preview,
                    "memory_type": memory_type, "scope": scope, "tags": tags_vec,
                    "source_type": source_type, "source_file_path": source_file_path,
                    "confidence": confidence, "visibility": visibility, "status": status,
                    "created_at": created_at, "updated_at": updated_at, "metadata": metadata
                })
            }).collect();
            Json(json!({"results": results, "total": results.len()}))
        },
        Err(e) => Json(json!({"error": e.to_string()})),
    }
}

/// Memory统计 (from doc table)
async fn memory_stats(
    State(db): State<Database>,
) -> Json<Value> {
    let stats = sqlx::query_as::<_, (String, i64)>(
        r#"SELECT source_type, COUNT(*) FROM doc
           WHERE status = 'active' AND type = 'memory'
           GROUP BY source_type"#
    )
    .fetch_all(&db.pool).await;

    match stats {
        Ok(rows) => {
            let total: i64 = rows.iter().map(|(_, count)| count).sum();
            let by_source: Value = rows.iter().map(|(source, count)| {
                json!({"source": source, "count": count})
            }).collect();

            let embedded_stats = sqlx::query_as::<_, (String, i64)>(
                r#"SELECT source_type, COUNT(*) FROM doc
                   WHERE status = 'active' AND type = 'memory' AND metadata IS NOT NULL AND metadata != '[]' AND metadata != ''
                   GROUP BY source_type"#
            )
            .fetch_all(&db.pool).await;

            let embedded_by_source: std::collections::HashMap<String, i64> = embedded_stats
                .unwrap_or_default().into_iter().collect();

            let memory_embedded: i64 = rows.iter()
                .map(|(s, _)| embedded_by_source.get(s).copied().unwrap_or(0))
                .sum();

            Json(json!({
                "total": total, "by_source": by_source,
                "memory_embedded": memory_embedded, "sessions_embedded": 0,
                "embedded_by_source": embedded_by_source
            }))
        },
        Err(e) => Json(json!({"error": e.to_string()})),
    }
}

/// 批量删除Memory (update doc status)
async fn batch_delete_memory(
    State(db): State<Database>,
    Json(req): Json<BatchDeleteRequest>,
) -> Json<Value> {
    let now = chrono::Utc::now().to_rfc3339();

    let result = if let Some(ids) = &req.ids {
        let placeholders: Vec<String> = ids.iter().enumerate().map(|(i, _)| format!("?{}", i + 1)).collect();
        let sql = format!("UPDATE doc SET status = 'deleted', updated_at = ?{} WHERE id IN ({})", ids.len() + 1, placeholders.join(", "));
        let mut query = sqlx::query(&sql);
        for id in ids { query = query.bind(id); }
        query = query.bind(&now);
        query.execute(&db.pool).await
    } else if let Some(source_type) = &req.source_type {
        sqlx::query("UPDATE doc SET status = 'deleted', updated_at = ? WHERE source_type = ? AND type = 'memory'")
            .bind(&now).bind(source_type).execute(&db.pool).await
    } else if Some(true) == req.delete_all_knowledge {
        sqlx::query("UPDATE doc SET status = 'deleted', updated_at = ? WHERE type = 'knowledge'")
            .bind(&now).execute(&db.pool).await
    } else if Some(true) == req.delete_all_memory {
        sqlx::query("UPDATE doc SET status = 'deleted', updated_at = ? WHERE type = 'memory'")
            .bind(&now).execute(&db.pool).await
    } else {
        return Json(json!({"error": "No delete criteria provided"}));
    };

    match result {
        Ok(r) => Json(json!({"success": true, "deleted_count": r.rows_affected()})),
        Err(e) => Json(json!({"error": e.to_string()})),
    }
}

#[derive(Debug, serde::Deserialize)]
struct BatchDeleteRequest {
    ids: Option<Vec<String>>,
    source_type: Option<String>,
    delete_all_knowledge: Option<bool>,
    delete_all_memory: Option<bool>,
}

/// 列出待审核的候选 Memory (unchanged - uses memory_candidate table)
async fn list_candidates(
    State(db): State<Database>,
    Query(params): Query<MemorySearchParams>,
) -> Json<Value> {
    let limit = params.limit.unwrap_or(50);
    let offset = params.offset.unwrap_or(0);

    let candidates = sqlx::query_as::<_, (String, String, String, String, String, String, String, f64, String, String, String)>(
        r#"SELECT id, title, SUBSTR(content, 1, 500), proposed_type, proposed_scope, reason,
           source_agent_id, confidence, source_agent_id, created_at, reviewed_at
           FROM memory_candidate WHERE review_status = 'needs_review'"
           ORDER BY
             CASE WHEN confidence >= 0.5 THEN 0 ELSE 1 END,
             created_at DESC
           LIMIT ? OFFSET ?"#,
    )
    .bind(limit).bind(offset)
    .fetch_all(&db.pool).await;

    let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM memory_candidate WHERE review_status = 'needs_review'")
        .fetch_one(&db.pool).await.unwrap_or(0);

    match candidates {
        Ok(rows) => {
            let items: Vec<Value> = rows.into_iter().map(|(id, title, content, memory_type, scope, tags, source_type, confidence, agent_id, created_at, updated_at)| {
                let tags_vec: Vec<String> = serde_json::from_str(&tags).unwrap_or_default();
                let ai_reason = tags_vec.iter().find(|t| t.starts_with("ai_rejected:")).map(|t| t.strip_prefix("ai_rejected: ").unwrap_or(t).to_string());
                let clean_tags: Vec<String> = tags_vec.iter().filter(|t| !t.starts_with("ai_rejected:")).cloned().collect();
                json!({
                    "id": id, "title": title, "content_preview": content,
                    "memory_type": memory_type, "scope": scope, "tags": clean_tags,
                    "source_type": source_type, "confidence": confidence,
                    "agent_id": agent_id,
                    "ai_review": if confidence >= 0.5 { "uncertain" } else { "rejected" },
                    "ai_reason": ai_reason,
                    "created_at": created_at, "updated_at": updated_at
                })
            }).collect();
            Json(json!({"candidates": items, "total": total, "pending_count": total}))
        },
        Err(e) => Json(json!({"error": e.to_string()})),
    }
}

/// 批准候选 Memory (insert into doc + memory_ext)
async fn approve_candidate(
    State(db): State<Database>,
    Path(id): Path<String>,
) -> Json<Value> {
    let now = chrono::Utc::now().to_rfc3339();

    let candidate = sqlx::query_as::<_, (String, String, String, String, String, String, String)>(
        "SELECT id, title, content, proposed_type, proposed_scope, source_agent_id, created_at FROM memory_candidate WHERE id = ? AND review_status = 'needs_review'"
    )
    .bind(&id).fetch_optional(&db.pool).await;

    let (c_id, title, content, proposed_type, _scope, agent_id, created_at) = match candidate {
        Ok(Some(row)) => row,
        Ok(None) => return Json(json!({"success": false, "error": "Candidate not found or already reviewed"})),
        Err(e) => return Json(json!({"error": e.to_string()})),
    };

    let mem_id = uuid::Uuid::new_v4().to_string();

    // Insert into doc
    let insert_result = sqlx::query(
        r#"INSERT INTO doc (id, type, title, content, source_type, status, created_at, updated_at)
           VALUES (?, 'memory', ?, ?, ?, 'active', ?, ?)"#
    )
    .bind(&mem_id).bind(&title).bind(&content)
    .bind(&agent_id).bind(&created_at).bind(&now)
    .execute(&db.pool).await;

    if let Err(e) = insert_result {
        return Json(json!({"error": e.to_string()}));
    }

    // Insert into memory_ext
    let _ = sqlx::query(
        r#"INSERT OR IGNORE INTO memory_ext (id, memory_type, scope, confidence, visibility, owner)
           VALUES (?, ?, 'shared', 0.9, 'shared', ?)"#
    )
    .bind(&mem_id).bind(&proposed_type).bind(&agent_id)
    .execute(&db.pool).await;

    let update_result = sqlx::query(
        "UPDATE memory_candidate SET review_status = 'approved', reviewed_at = ? WHERE id = ?"
    ).bind(&now).bind(&c_id).execute(&db.pool).await;

    match update_result {
        Ok(_) => Json(json!({"success": true, "id": c_id, "memory_id": mem_id, "message": "Memory candidate approved"})),
        Err(e) => Json(json!({"error": e.to_string()})),
    }
}

/// 拒绝候选 Memory (unchanged)
async fn reject_candidate(
    State(db): State<Database>,
    Path(id): Path<String>,
) -> Json<Value> {
    let now = chrono::Utc::now().to_rfc3339();
    let result = sqlx::query(
        "UPDATE memory_candidate SET review_status = 'rejected', reviewed_at = ? WHERE id = ? AND review_status = 'needs_review'"
    ).bind(&now).bind(&id).execute(&db.pool).await;

    match result {
        Ok(r) if r.rows_affected() > 0 => Json(json!({"success": true, "id": id, "message": "Memory candidate rejected"})),
        Ok(_) => Json(json!({"success": false, "error": "Candidate not found or already reviewed"})),
        Err(e) => Json(json!({"error": e.to_string()})),
    }
}
