use std::sync::Arc;
use tauri::State;
use crate::core::app_kernel::AppKernel;
use crate::wiki::path_service::PathService;

#[tauri::command]
pub async fn get_pending_reviews(
    kernel: State<'_, Arc<AppKernel>>,
    kb_id: String,
) -> Result<Vec<serde_json::Value>, String> {
    let reviews = crate::review::review_engine::ReviewEngine::get_pending_reviews(
        &kernel.db, &kb_id,
    )?;

    Ok(reviews.iter().map(|r| serde_json::json!({
        "id": r.id,
        "kb_id": r.kb_id,
        "task_id": r.task_id,
        "status": r.status,
        "summary": r.summary,
        "risk_level": r.risk_level,
        "created_at": r.created_at,
        "items": r.items.iter().map(|i| serde_json::json!({
            "id": i.id,
            "task_id": r.task_id,
            "review_id": r.id,
            "operation": i.operation,
            "operation_type": i.operation_type,
            "target_path": i.target_path,
            "old_content": i.old_content,
            "new_content": i.new_content,
            "status": i.status,
            "risk_level": i.risk_level,
            "reason": i.reason,
            "source_id": i.source_id,
            "citation_status": i.citation_status,
            "summary": i.summary,
            "confidence": i.confidence,
            "created_at": i.created_at,
            "page_type": i.page_type,
            "title": i.title,
            "apply_error": i.apply_error,
            "duplicate_candidate": i.duplicate_candidate,
            "missing_target": i.missing_target,
            "manual_required": i.manual_required,
            "matched_page": i.matched_page,
            "matched_path": i.matched_path,
            "merge_candidate": i.merge_candidate,
            "auto_converted_from_update": i.auto_converted_from_update,
        })).collect::<Vec<_>>(),
    })).collect())
}

#[tauri::command]
pub async fn get_review_detail(
    kernel: State<'_, Arc<AppKernel>>,
    review_id: String,
) -> Result<serde_json::Value, String> {
    let conn = kernel.db.connect()?;
    let review: (String, String, String, String, i64) = conn.query_row(
        "SELECT id, kb_id, task_id, summary, risk_level FROM reviews WHERE id = ?1",
        rusqlite::params![review_id],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
    ).map_err(|e| format!("查询审阅失败: {}", e))?;

    let mut item_stmt = conn.prepare(
        "SELECT id, operation_type, target_path, title, page_type, new_content, old_content, risk_level, status, reason, source_id, confidence
         FROM review_items WHERE review_id = ?1 ORDER BY risk_level DESC, operation_type"
    ).map_err(|e| format!("准备查询失败: {}", e))?;

    let items: Vec<serde_json::Value> = item_stmt.query_map(
        rusqlite::params![review_id],
        |row| Ok(serde_json::json!({
            "id": row.get::<_, String>(0)?,
            "operation_type": row.get::<_, String>(1)?,
            "target_path": row.get::<_, String>(2)?,
            "title": row.get::<_, String>(3)?,
            "page_type": row.get::<_, String>(4)?,
            "new_content": row.get::<_, String>(5)?,
            "old_content": row.get::<_, String>(6)?,
            "risk_level": row.get::<_, String>(7)?,
            "status": row.get::<_, String>(8)?,
            "reason": row.get::<_, String>(9)?,
            "source_id": row.get::<_, String>(10)?,
            "confidence": row.get::<_, f64>(11).unwrap_or(0.0),
        })),
    ).map_err(|e| format!("映射审阅项失败: {}", e))?
    .collect::<Result<Vec<_>, _>>()
    .map_err(|e| format!("读取审阅项列表失败: {}", e))?;

    Ok(serde_json::json!({
        "id": review.0,
        "kb_id": review.1,
        "task_id": review.2,
        "summary": review.3,
        "risk_level": review.4,
        "items": items,
    }))
}

#[tauri::command]
pub async fn accept_review_item(
    kernel: State<'_, Arc<AppKernel>>,
    item_id: String,
    kb_id: String,
    kb_path: String,
) -> Result<serde_json::Value, String> {
    apply_review_item_sync(kernel.inner(), &item_id, &kb_id, &kb_path)
}

fn apply_review_item_sync(
    kernel: &Arc<AppKernel>,
    item_id: &str,
    kb_id: &str,
    kb_path: &str,
) -> Result<serde_json::Value, String> {
    match apply_review_item_impl(kernel, item_id, kb_id, kb_path) {
        Ok(value) => Ok(value),
        Err(err) => {
            if let Ok(conn) = kernel.db.connect() {
                let now = chrono::Utc::now().to_rfc3339();
                if let Err(e) = conn.execute(
                    "UPDATE review_items SET status = 'apply_failed', apply_error = ?1, updated_at = ?2 WHERE id = ?3",
                    rusqlite::params![err, now, item_id],
                ) {
                    log::error!("[review] 更新 review_item apply_failed 失败 (item={}): {}", item_id, e);
                }
            }
            Err(err)
        }
    }
}

fn apply_review_item_impl(
    kernel: &Arc<AppKernel>,
    item_id: &str,
    kb_id: &str,
    kb_path: &str,
) -> Result<serde_json::Value, String> {
    use crate::dedup::dedup_service::DedupService;

    let conn = kernel.db.connect()?;
    let now_ts = chrono::Utc::now().to_rfc3339();

    let (
        review_id,
        task_id,
        _operation,
        operation_type,
        target_path,
        new_content,
        base_version_hash,
        source_id,
        review_status,
        stored_title,
        stored_page_type,
    ): (String, String, String, String, String, String, String, String, String, String, String) = conn
        .query_row(
            "SELECT ri.review_id, r.task_id, ri.operation, COALESCE(ri.operation_type, ri.operation), ri.target_path, COALESCE(ri.new_content,''), COALESCE(ri.base_version_hash,''),
                    COALESCE(ri.source_id,''), ri.status, COALESCE(ri.title,''), COALESCE(ri.page_type,'')
             FROM review_items ri JOIN reviews r ON ri.review_id = r.id
             WHERE ri.id = ?1 AND r.kb_id = ?2",
            rusqlite::params![item_id, kb_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?, row.get(6)?, row.get(7)?, row.get(8)?, row.get(9)?, row.get(10)?)),
        )
        .map_err(|e| format!("读取审阅项失败: {}", e))?;

    // 事件日志辅助函数
    let log_event = |conn: &rusqlite::Connection, item_id: &str, old_status: &str, new_status: &str, action: &str, reason: &str| {
        let event_id = uuid::Uuid::new_v4().to_string();
        if let Err(e) = conn.execute(
            "INSERT INTO review_item_events (id, review_item_id, old_status, new_status, action, reason, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![event_id, item_id, old_status, new_status, action, reason, &now_ts],
        ) {
            log::error!("[Review] 写入审阅项事件失败(item={}): {}", item_id, e);
        }
    };

    // 检查状态：已处理的直接返回
    if review_status == "applied" {
        return Ok(serde_json::json!({"status": "already_applied"}));
    }
    if review_status == "rejected" {
        return Err("该审阅项已被拒绝，不能应用".to_string());
    }
    if review_status == "skipped" {
        return Ok(serde_json::json!({"status": "already_skipped"}));
    }

    let workspace_root = std::path::PathBuf::from(kb_path);
    let wiki_dir = workspace_root.join("wiki");
    let writer = crate::wiki::wiki_writer::WikiWriter::new(kernel.db.clone());
    let normalized_target = PathService::normalize_workspace_path(&target_path);

    // 设置 applying 状态
    if let Err(e) = conn.execute(
        "UPDATE review_items SET status = 'applying', updated_at = ?1 WHERE id = ?2",
        rusqlite::params![now_ts, item_id],
    ) {
        log::error!("[review] 更新 review_item applying 失败 (item={}): {}", item_id, e);
    }
    log_event(&conn, item_id, &review_status, "applying", "apply_start", &format!("开始应用操作: {}", operation_type));

    // v0.2.1: 严格按 operation_type 分发
    let result = match operation_type.as_str() {
        "create_page" | "create" => {
            let title = crate::wiki::markdown_indexer::MarkdownIndexer::best_title(
                &new_content, &normalized_target, Some(&stored_title),
            );
            if title.trim().is_empty() || title == "Untitled"
                || crate::wiki::markdown_indexer::MarkdownIndexer::looks_like_generated_page_id(&title)
            {
                log_event(&conn, item_id, "applying", "needs_manual_review", "apply_blocked", "缺少可用标题");
                conn.execute(
                    "UPDATE review_items SET status = 'needs_manual_review', apply_error = ?1, updated_at = ?2 WHERE id = ?3",
                    rusqlite::params!["缺少可用标题，未自动生成 page-xxx 页面", now_ts, item_id],
                ).map_err(|e| format!("更新审阅项状态失败: {}", e))?;
                return Ok(serde_json::json!({
                    "status": "needs_manual_review",
                    "message": "缺少可用标题，未自动生成 page-xxx 页面"
                }));
            }
            let metadata = crate::wiki::markdown_indexer::MarkdownIndexer::extract_metadata(
                &new_content, &normalized_target,
            );
            let page_type = if stored_page_type.trim().is_empty() {
                if metadata.page_type.trim().is_empty() {
                    PathService::path_to_page_type(&normalized_target).to_string()
                } else { metadata.page_type }
            } else { stored_page_type };
            let canonical = PathService::generate_safe_name(&title);

            // v0.2.1: 创建前再次查重
            if let Ok(dedup) = DedupService::find_duplicates(&kernel.db, kb_id, &title) {
                if dedup.is_duplicate && dedup.suggested_operation == "update_page" {
                    if let Some(ref best) = dedup.best_match {
                        log_event(&conn, item_id, "applying", "needs_manual_review", "duplicate_detected",
                            &format!("创建前检测到重复页面「{}」(相似度: {:.0}%)", best.matched_title, best.similarity * 100.0));
                        conn.execute(
                            "UPDATE review_items SET status = 'needs_manual_review', apply_error = ?1, updated_at = ?2 WHERE id = ?3",
                            rusqlite::params![format!("疑似与已有页面「{}」重复，已暂停创建供您确认", best.matched_title), now_ts, item_id],
                        ).map_err(|e| format!("更新审阅项状态失败: {}", e))?;
                        return Ok(serde_json::json!({
                            "status": "needs_manual_review",
                            "message": format!("疑似与已有页面「{}」重复", best.matched_title),
                            "duplicate_candidate": true,
                            "matched_page": best.matched_title,
                            "matched_path": best.matched_path,
                        }));
                    }
                }
            }

            let content_body = if new_content.trim_start().starts_with("---") {
                metadata.body
            } else { new_content.clone() };
            let aliases_yaml = if metadata.aliases.is_empty() {
                "[]".to_string()
            } else {
                format!("[{}]", metadata.aliases.iter().map(|a| format!("\"{}\"", a)).collect::<Vec<_>>().join(", "))
            };
            let wr = writer.create_page_full(
                kb_id, &wiki_dir, &page_type, &title, &canonical,
                &content_body, &metadata.tags, &aliases_yaml, &task_id,
                if source_id.is_empty() { None } else { Some(source_id.as_str()) },
            )?;
            conn.execute(
                "UPDATE review_items SET target_path = ?1, title = ?2, page_type = ?3 WHERE id = ?4",
                rusqlite::params![wr.relative_path, title, page_type, item_id],
            ).map_err(|e| format!("更新审阅项路径失败: {}", e))?;
            wr
        }
        "update_page" | "update" => {
            if normalized_target.is_empty() {
                return Err("更新操作缺少目标路径".to_string());
            }
            let target_abs = PathService::resolve_workspace_path(&workspace_root, &normalized_target);
            // Atomic read: eliminate TOCTOU race between exists() check and read_to_string()
            let current_content = match std::fs::read_to_string(&target_abs) {
                Ok(content) => content,
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                    // v0.2.1: 目标不存在 → 提示用户，不静默转换
                    log_event(&conn, item_id, "applying", "needs_manual_review", "target_missing",
                        &format!("更新目标文件不存在: {}", normalized_target));
                    conn.execute(
                        "UPDATE review_items SET status = 'needs_manual_review', apply_error = ?1, updated_at = ?2 WHERE id = ?3",
                        rusqlite::params![format!("目标页面「{}」不存在，请确认是否转为创建页面", normalized_target), now_ts, item_id],
                    ).map_err(|e| format!("更新审阅项状态失败: {}", e))?;
                    return Ok(serde_json::json!({
                        "status": "needs_manual_review",
                        "message": format!("目标页面「{}」不存在，是否转为创建页面？", normalized_target),
                        "missing_target": true,
                    }));
                }
                Err(e) => return Err(format!("读取目标页面失败 ({}): {}", normalized_target, e)),
            };
            if !base_version_hash.is_empty() {
                let current_hash = PathService::content_hash(&current_content);
                if current_hash != base_version_hash {
                    return Err(format!("目标页面已被修改，base_version_hash 不匹配: {}", normalized_target));
                }
            }
            let wr = writer.update_page_full(kb_id, &wiki_dir, &normalized_target, &new_content, &task_id)?;

            // LinkSanitizer: 检测 staging 页面的接受 — 标记为已解决
            let was_staging: bool = conn
                .query_row(
                    "SELECT content_hash = 'staging' FROM wiki_pages WHERE kb_id = ?1 AND path = ?2",
                    rusqlite::params![kb_id, normalized_target],
                    |row| row.get(0),
                )
                .map_err(|e| {
                    log::error!("[review] 查询 staging 状态失败 (page={}): {}", normalized_target, e);
                    false
                })
                .unwrap_or(false);
            if was_staging {
                log::info!("[review] staging 页面被接受: {} → 覆写占位文件并激活", normalized_target);
                let _ = conn.execute(
                    "UPDATE link_sanitizer_log SET action = 'resolved', details = details || ' [审批通过]' WHERE placeholder_path = ?1 AND kb_id = ?2 AND action = 'ai_completion'",
                    rusqlite::params![normalized_target, kb_id],
                );
                let _ = kernel.event_bus.emit_notification(
                    "info",
                    "死链已修复",
                    &format!("「{}」的 AI 补全内容已通过审阅并激活", normalized_target),
                );
            }
            wr
        }
        "append_section" | "append" => {
            if normalized_target.is_empty() {
                return Err("追加操作缺少目标路径".to_string());
            }
            let target_abs = PathService::resolve_workspace_path(&workspace_root, &normalized_target);
            // Atomic: try read first to avoid TOCTOU race between exists() and read_to_string()
            let existing = match std::fs::read_to_string(&target_abs) {
                Ok(content) => content,
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                    log_event(&conn, item_id, "applying", "needs_manual_review", "target_missing",
                        &format!("追加目标文件不存在: {}", normalized_target));
                    conn.execute(
                        "UPDATE review_items SET status = 'needs_manual_review', apply_error = ?1, updated_at = ?2 WHERE id = ?3",
                        rusqlite::params![format!("目标页面「{}」不存在，请确认是否转为创建页面", normalized_target), now_ts, item_id],
                    ).map_err(|e| format!("更新审阅项状态失败: {}", e))?;
                    return Ok(serde_json::json!({
                        "status": "needs_manual_review",
                        "message": format!("目标页面「{}」不存在，是否转为创建页面？", normalized_target),
                        "missing_target": true,
                    }));
                }
                Err(e) => return Err(format!("读取目标页面失败 ({}): {}", normalized_target, e)),
            };
            let combined = format!("{}\n\n{}", existing, new_content);
            writer.update_page_full(kb_id, &wiki_dir, &normalized_target, &combined, &task_id)?
        }
        "add_alias" => {
            let alias = new_content.trim().trim_matches('"').to_string();
            if alias.is_empty() {
                return Err("add_alias 缺少别名内容".to_string());
            }
            let canonical = PathService::generate_safe_name(&crate::wiki::markdown_indexer::MarkdownIndexer::best_title(
                "", &normalized_target, Some(&stored_title),
            ));
            let item_id_found: String = conn.query_row(
                "SELECT ki.id FROM knowledge_items ki WHERE ki.kb_id = ?1 AND (ki.page_path = ?2 OR ki.canonical_name = ?3) LIMIT 1",
                rusqlite::params![kb_id, normalized_target, canonical],
                |row| row.get(0),
            ).map_err(|_| format!("未找到目标知识项: {}", normalized_target))?;
            crate::wiki::wiki_writer::WikiWriter::upsert_alias(&kernel.db, &item_id_found, &alias, "unknown")?;
            crate::wiki::wiki_writer::WikiWriteResult {
                page_id: String::new(),
                relative_path: normalized_target.clone(),
                content_hash: String::new(),
                knowledge_item_id: Some(item_id_found),
                operation_id: None,
            }
        }
        "add_relation" => {
            // v0.2.1: 尝试解析和写入关系
            let parsed: Result<serde_json::Value, _> = serde_json::from_str(&new_content);
            if let Ok(rel_json) = parsed {
                if let (Some(source), Some(target)) = (
                    rel_json.get("source").and_then(|s| s.as_str()),
                    rel_json.get("target").and_then(|t| t.as_str()),
                ) {
                    let relation = rel_json.get("relation").and_then(|r| r.as_str()).unwrap_or("related_to");
                    let source_ki: Option<String> = match conn.query_row(
                        "SELECT id FROM knowledge_items WHERE kb_id = ?1 AND canonical_name = ?2 LIMIT 1",
                        rusqlite::params![kb_id, source],
                        |row| row.get(0),
                    ) {
                        Ok(id) => Some(id),
                        Err(rusqlite::Error::QueryReturnedNoRows) => None,
                        Err(e) => { log::error!("[review] 查询 source_ki 失败 (name={}): {}", source, e); return Err(format!("查询关系源节点失败: {}", e)); }
                    };
                    let target_ki: Option<String> = match conn.query_row(
                        "SELECT id FROM knowledge_items WHERE kb_id = ?1 AND canonical_name = ?2 LIMIT 1",
                        rusqlite::params![kb_id, target],
                        |row| row.get(0),
                    ) {
                        Ok(id) => Some(id),
                        Err(rusqlite::Error::QueryReturnedNoRows) => None,
                        Err(e) => { log::error!("[review] 查询 target_ki 失败 (name={}): {}", target, e); return Err(format!("查询关系目标节点失败: {}", e)); }
                    };
                    if let (Some(s), Some(t)) = (source_ki, target_ki) {
                        crate::wiki::wiki_writer::WikiWriter::upsert_relationship(
                            &kernel.db, kb_id, &s, &t, relation, "medium", &source_id,
                        )?;
                    }
                }
            }
            // 关系操作不产生 Wiki 页面
            crate::wiki::wiki_writer::WikiWriteResult {
                page_id: String::new(),
                relative_path: String::new(),
                content_hash: String::new(),
                knowledge_item_id: None,
                operation_id: None,
            }
        }
        "merge_suggestion" => {
            // v0.2.1: 合并建议 → 必须人工处理
            log_event(&conn, item_id, "applying", "needs_manual_review", "merge_manual_required", "合并操作需要人工确认");
            conn.execute(
                "UPDATE review_items SET status = 'needs_manual_review', apply_error = ?1, updated_at = ?2 WHERE id = ?3",
                rusqlite::params!["合并建议需要人工确认后再应用", now_ts, item_id],
            ).map_err(|e| format!("更新审阅项状态失败: {}", e))?;
            return Ok(serde_json::json!({
                "status": "needs_manual_review",
                "message": "合并建议需要人工确认后再应用",
            "manual_required": true,
            }));
        }
        "skip" => {
            // v0.2.1: 确认跳过
            log_event(&conn, item_id, "applying", "skipped", "skip_confirmed", "用户确认跳过该项");
            conn.execute(
                "UPDATE review_items SET status = 'skipped', apply_error = '用户确认跳过', updated_at = ?1 WHERE id = ?2",
                rusqlite::params![now_ts, item_id],
            ).map_err(|e| format!("更新审阅项状态失败: {}", e))?;
            return Ok(serde_json::json!({
                "status": "skipped",
                "message": "已确认跳过该项"
            }));
        }
        "delete_page" | "delete" => {
            if normalized_target.is_empty() {
                return Err("删除操作缺少目标路径".to_string());
            }
            let target_abs = PathService::resolve_workspace_path(&workspace_root, &normalized_target);
            if !target_abs.exists() {
                log_event(&conn, item_id, "applying", "skipped", "delete_missing_target",
                    &format!("删除目标不存在: {}", normalized_target));
                conn.execute(
                    "UPDATE review_items SET status = 'skipped', apply_error = ?1, updated_at = ?2 WHERE id = ?3",
                    rusqlite::params![format!("目标页面「{}」不存在，已跳过删除", normalized_target), now_ts, item_id],
                ).map_err(|e| format!("更新审阅项状态失败: {}", e))?;
                return Ok(serde_json::json!({
                    "status": "skipped",
                    "message": format!("目标页面「{}」不存在，已跳过删除", normalized_target),
                }));
            }
            // 级联清理：查关联的 knowledge_items
            {
                let mut stmt = conn.prepare(
                    "SELECT id FROM knowledge_items WHERE kb_id = ?1 AND (page_path = ?2 OR linked_page_path = ?2)"
                ).map_err(|e| format!("查询关联知识项失败: {}", e))?;
                let ki_ids: Vec<String> = stmt.query_map(
                    rusqlite::params![kb_id, normalized_target],
                    |row| row.get(0),
                ).map_err(|e| format!("查询关联知识项失败: {}", e))?
                    .filter_map(|r| r.ok())
                    .collect();

                for ki_id in &ki_ids {
                    if let Err(e) = conn.execute("DELETE FROM aliases WHERE item_id = ?1", rusqlite::params![ki_id]) {
                        log::error!("[review] delete_page 删除 aliases 失败(ki={}): {}", ki_id, e);
                    }
                    if let Err(e) = conn.execute(
                        "DELETE FROM relationships WHERE kb_id = ?1 AND (source_item_id = ?2 OR target_item_id = ?2)",
                        rusqlite::params![kb_id, ki_id],
                    ) {
                        log::error!("[review] delete_page 删除 relationships 失败(ki={}): {}", ki_id, e);
                    }
                }
                if !ki_ids.is_empty() {
                    if let Err(e) = conn.execute(
                        "DELETE FROM knowledge_items WHERE kb_id = ?1 AND (page_path = ?2 OR linked_page_path = ?2)",
                        rusqlite::params![kb_id, normalized_target],
                    ) {
                        log::error!("[review] delete_page 删除 knowledge_items 失败(page={}): {}", normalized_target, e);
                    }
                }
            }

            // 清理 graph_nodes 中引用此页面路径的节点
            {
                let mut edge_stmt = conn.prepare(
                    "SELECT id FROM graph_edges WHERE kb_id = ?1 AND (source_node_id IN (SELECT id FROM graph_nodes WHERE kb_id = ?1 AND path = ?2) OR target_node_id IN (SELECT id FROM graph_nodes WHERE kb_id = ?1 AND path = ?2))"
                ).map_err(|e| format!("查询关联图谱边失败: {}", e))?;
                let edge_ids: Vec<String> = edge_stmt.query_map(
                    rusqlite::params![kb_id, normalized_target],
                    |row| row.get(0),
                ).map_err(|e| format!("查询关联图谱边失败: {}", e))?
                    .filter_map(|r| r.ok())
                    .collect();
                for edge_id in &edge_ids {
                    if let Err(e) = conn.execute("DELETE FROM graph_edges WHERE id = ?1", rusqlite::params![edge_id]) {
                        log::error!("[review] delete_page 删除 graph_edge 失败(edge={}): {}", edge_id, e);
                    }
                }
                if let Err(e) = conn.execute(
                    "DELETE FROM graph_nodes WHERE kb_id = ?1 AND path = ?2",
                    rusqlite::params![kb_id, normalized_target],
                ) {
                    log::error!("[review] delete_page 删除 graph_nodes 失败(page={}): {}", normalized_target, e);
                }
            }

            // 清理版本快照
            if let Err(e) = conn.execute(
                "DELETE FROM versions WHERE kb_id = ?1 AND page_path = ?2",
                rusqlite::params![kb_id, normalized_target],
            ) {
                log::error!("[review] delete_page 删除 versions 失败(page={}): {}", normalized_target, e);
            }

            // 清理引用此页面的审阅事件 (review_item_events)
            if let Err(e) = conn.execute(
                "DELETE FROM review_item_events WHERE review_item_id IN (SELECT id FROM review_items WHERE review_id IN (SELECT id FROM reviews WHERE kb_id = ?1) AND target_path = ?2)",
                rusqlite::params![kb_id, normalized_target],
            ) {
                log::error!("[review] delete_page 清理 review_item_events 失败(page={}): {}", normalized_target, e);
            }

            // 清理引用此页面的审阅项
            if let Err(e) = conn.execute(
                "DELETE FROM review_items WHERE review_id IN (SELECT id FROM reviews WHERE kb_id = ?1) AND target_path = ?2",
                rusqlite::params![kb_id, normalized_target],
            ) {
                log::error!("[review] delete_page 清理 review_items 失败(page={}): {}", normalized_target, e);
            }

            // 先删除 DB 记录，确保数据库一致性
            conn.execute(
                "DELETE FROM wiki_pages WHERE kb_id = ?1 AND path = ?2",
                rusqlite::params![kb_id, normalized_target],
            ).map_err(|e| format!("删除页面记录失败: {}", e))?;
            // 再删除磁盘文件（如果失败仅记录日志，数据库已保持一致性）
            if let Err(e) = std::fs::remove_file(&target_abs) {
                log::error!("[review] delete_page 删除页面文件失败 ({}): {}", target_abs.display(), e);
            }
            // 记录操作
            let op_id = uuid::Uuid::new_v4().to_string();
            let op_hash = PathService::content_hash(&format!("delete:{}:{}", normalized_target, now_ts));
            if let Err(e) = conn.execute(
                "INSERT INTO operations (id, kb_id, task_id, operation_hash, target_path, status, applied_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, 'applied', ?6)",
                rusqlite::params![op_id, kb_id, task_id, op_hash, normalized_target, now_ts],
            ) {
                log::error!("[review] 记录 delete_page operation 失败: {}", e);
            }
            crate::wiki::wiki_writer::WikiWriteResult {
                page_id: String::new(),
                relative_path: normalized_target.clone(),
                content_hash: String::new(),
                knowledge_item_id: None,
                operation_id: Some(op_id),
            }
        }
        "unresolved" | "invalid" => {
            // v0.2.1: 不可自动应用
            log_event(&conn, item_id, "applying", "needs_manual_review", "unresolved", "该项标记为未解决/无效");
            conn.execute(
                "UPDATE review_items SET status = 'needs_manual_review', apply_error = ?1, updated_at = ?2 WHERE id = ?3",
                rusqlite::params!["该项需要人工评估和处理", now_ts, item_id],
            ).map_err(|e| format!("更新审阅项状态失败: {}", e))?;
            return Ok(serde_json::json!({
                "status": "needs_manual_review",
                "message": "该项需要人工评估和处理"
            }));
        }
        other => return Err(format!("未知操作类型: {}", other)),
    };

    // 后处理：重建索引、日志、版本快照、图谱同步
    crate::wiki::index_service::IndexService::new(kernel.db.clone())
        .rebuild_index(kb_id, &wiki_dir)?;
    crate::wiki::log_service::LogService::append_log(
        &wiki_dir,
        &operation_type,
        &format!("AI {} 页面: {} (task: {}, item: {})", operation_type, result.relative_path, task_id, item_id),
    )?;
    if !result.relative_path.is_empty() {
        let vm = crate::wiki::version_manager::VersionManager::new(kernel.db.clone());
        vm.create_snapshot(kb_id, &wiki_dir, &result.relative_path, "review_accept", item_id)?;
    }

    // 更新 source 关联页面计数
    if !source_id.is_empty() && !result.page_id.is_empty() {
        if let Err(e) = conn.execute(
            "UPDATE sources SET linked_pages_count = (SELECT COUNT(*) FROM knowledge_items ki JOIN wiki_pages wp ON ki.page_id = wp.id WHERE ki.source_id = ?1 AND wp.kb_id = ?2) WHERE id = ?1",
            rusqlite::params![source_id, kb_id],
        ) {
            log::error!("[review] 更新 source linked_pages_count 失败 (source={}): {}", source_id, e);
        }
    }

    // 关联 knowledge_items.page_id
    if let Some(ref ki_id) = result.knowledge_item_id {
        if let Err(e) = conn.execute(
            "UPDATE knowledge_items SET page_id = ?1, normalized_name = LOWER(TRIM(canonical_name)), updated_at = ?2 WHERE id = ?3 AND (page_id = '' OR page_id IS NULL)",
            rusqlite::params![result.page_id, now_ts, ki_id],
        ) {
            log::error!("[review] 更新 knowledge_item page_id 失败 (ki={}): {}", ki_id, e);
        }
    }

    crate::graph::graph_service::GraphService::sync_from_knowledge_items(&kernel.db, kb_id)?;
    // 自动推导关系（knowledge_items → wiki_pages 的 references 关联 + same_source 共现关联）
    if let Err(e) = crate::graph::graph_service::GraphService::derive_relationships(&kernel.db, kb_id) {
        log::error!("[review] derive_relationships 失败 (kb={}): {}", kb_id, e);
    }

    // v0.2.1: 成功 → status = applied（不允许变成 skipped）
    log_event(&conn, item_id, "applying", "applied", "apply_success", "操作成功应用");
    conn.execute(
        "UPDATE review_items SET status = 'applied', apply_error = '', updated_at = ?1 WHERE id = ?2",
        rusqlite::params![now_ts, item_id],
    ).map_err(|e| format!("更新审阅项状态失败: {}", e))?;

    let pending_count: i64 = match conn.query_row(
        "SELECT COUNT(*) FROM review_items WHERE review_id = ?1 AND status IN ('pending', 'pending_manual', 'needs_manual_review', 'apply_failed', 'applying')",
        rusqlite::params![review_id],
        |row| row.get(0),
    ) {
        Ok(c) => c,
        Err(rusqlite::Error::QueryReturnedNoRows) => 0,
        Err(e) => return Err(format!("查询待处理审阅项计数失败: {}", e)),
    };
    if pending_count == 0 {
        conn.execute(
            "UPDATE reviews SET status = 'completed', updated_at = ?1 WHERE id = ?2",
            rusqlite::params![now_ts, review_id],
        ).map_err(|e| format!("更新审阅状态失败: {}", e))?;
        conn.execute(
            "UPDATE tasks SET status = 'applied', completed_at = ?1 WHERE id = ?2",
            rusqlite::params![now_ts, task_id],
        ).map_err(|e| format!("更新任务状态失败: {}", e))?;
        // v0.2.3: 更新关联 sources 状态为 applied
        if let Err(e) = conn.execute(
            "UPDATE sources SET status = 'applied', updated_at = ?1 WHERE id IN (SELECT DISTINCT input_ref FROM tasks WHERE id = ?2) AND status = 'processed'",
            rusqlite::params![now_ts, task_id],
        ) {
            log::error!("[Review] 更新关联 sources 状态为 applied 失败(task={}): {}", task_id, e);
        }
        // 发送 source-updated 事件给每个受影响的 source
        match conn.prepare("SELECT DISTINCT input_ref FROM tasks WHERE id = ?1 AND input_ref != ''") {
            Ok(mut stmt) => {
                match stmt.query_map(rusqlite::params![task_id], |row| row.get::<_, String>(0)) {
                    Ok(rows) => {
                        for sid in rows.filter_map(|r| r.ok()) {
                            kernel.event_bus.emit_source_updated(kb_id, &sid);
                        }
                    }
                    Err(e) => log::error!("[Review] 查询关联 source 失败: {}", e),
                }
            }
            Err(e) => log::error!("[Review] 准备 source 查询失败: {}", e),
        }
    }

    kernel.event_bus.emit_wiki_updated(kb_id, &result.relative_path);
    kernel.event_bus.emit_review_updated(kb_id, &review_id);
    kernel.event_bus.emit_kb_stats_changed(kb_id);

    Ok(serde_json::json!({
        "status": "applied",
        "page_path": result.relative_path,
        "content_hash": result.content_hash,
    }))
}

fn accept_all_low_risk_review_sync(
    kernel: &Arc<AppKernel>,
    review_id: &str,
    kb_id: &str,
    kb_path: &str,
) -> Result<serde_json::Value, String> {
    let conn = kernel.db.connect()?;

    // 读取 KB 配置中的 review_mode
    let review_mode = kernel.config
        .get_kb_config(&std::path::PathBuf::from(kb_path))
        .map(|c| c.review_mode)
        .unwrap_or_else(|_| "balanced".to_string());

    // strict: 不自动接受任何项
    if review_mode == "strict" {
        let pending_count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM review_items WHERE review_id = ?1 AND status = 'pending'",
            rusqlite::params![review_id],
            |row| row.get(0),
        ).unwrap_or(0);
        return Ok(serde_json::json!({
            "applied": 0,
            "failed": 0,
            "skipped": pending_count,
            "needs_manual": 0,
            "warnings": ["审阅模式为「严格」，不自动接受任何项"],
        }));
    }

    // auto: 接受 low + medium；balanced: 仅 low
    let risk_filter = if review_mode == "auto" { "('low','medium')" } else { "('low')" };

    // v0.2.1: 排除不可自动接受的操作类型
    let non_automatable = ["skip", "merge_suggestion", "unresolved", "invalid", "delete_page", "delete"];
    let placeholders = non_automatable.iter().map(|_| "?").collect::<Vec<_>>().join(",");
    let sql = format!(
        "SELECT ri.id FROM review_items ri WHERE ri.review_id = ?1 AND ri.risk_level IN {} AND ri.status = 'pending' AND (COALESCE(ri.operation_type, ri.operation) NOT IN ({}))",
        risk_filter, placeholders
    );

    let mut stmt = conn.prepare(&sql)
        .map_err(|e| format!("查询低风险审阅项失败: {}", e))?;

    let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = vec![Box::new(review_id.to_string())];
    for s in &non_automatable {
        params.push(Box::new(s.to_string()));
    }
    let param_refs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|b| b.as_ref()).collect();

    let item_ids: Vec<String> = stmt
        .query_map(param_refs.as_slice(), |row| row.get(0))
        .map_err(|e| format!("映射低风险审阅项失败: {}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("读取低风险审阅项失败: {}", e))?;

    // v0.2.1: 统计被跳过的不可自动接受项（使用与主查询一致的 risk_filter）
    let skipped_sql = format!(
        "SELECT COUNT(*) FROM review_items ri WHERE ri.review_id = ?1 AND ri.risk_level IN {} AND ri.status = 'pending' AND (COALESCE(ri.operation_type, ri.operation) IN ('skip', 'merge_suggestion', 'unresolved', 'invalid', 'delete_page', 'delete'))",
        risk_filter
    );
    let skipped_count: i64 = match conn.query_row(
        &skipped_sql,
        rusqlite::params![review_id],
        |row| row.get(0),
    ) {
        Ok(c) => c,
        Err(rusqlite::Error::QueryReturnedNoRows) => 0,
        Err(e) => { log::error!("[review] 查询被跳过项计数失败 (review={}): {}", review_id, e); 0 }
    };

    let mut applied = 0usize;
    let mut failed = 0usize;
    let mut needs_manual = 0usize;
    let mut warnings = Vec::new();
    for item_id in item_ids {
        match apply_review_item_sync(kernel, &item_id, kb_id, kb_path) {
            Ok(value) => {
                let status = value.get("status").and_then(|s| s.as_str()).unwrap_or("");
                if status == "applied" {
                    applied += 1;
                } else if status == "needs_manual_review" {
                    needs_manual += 1;
                } else {
                    warnings.push(format!("{}: {}", &item_id[..8], value));
                }
            }
            Err(e) => {
                failed += 1;
                warnings.push(format!("{}: {}", &item_id[..8], e));
            }
        }
    }
    kernel.event_bus.emit_review_updated(kb_id, review_id);
    kernel.event_bus.emit_kb_stats_changed(kb_id);
    Ok(serde_json::json!({
        "applied": applied,
        "failed": failed,
        "skipped": skipped_count,
        "needs_manual": needs_manual,
        "warnings": warnings,
    }))
}

#[tauri::command]
pub async fn reject_review_item(
    kernel: State<'_, Arc<AppKernel>>,
    item_id: String,
) -> Result<(), String> {
    crate::review::review_engine::ReviewEngine::reject_item(&kernel.db, &item_id)?;

    // 发送 review-updated 事件以触发前端刷新
    let conn = kernel.db.connect()?;
    if let Ok((review_id, kb_id)) = conn.query_row(
        "SELECT r.id, r.kb_id FROM reviews r JOIN review_items ri ON ri.review_id = r.id WHERE ri.id = ?1",
        rusqlite::params![item_id],
        |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
    ) {
        kernel.event_bus.emit_review_updated(&kb_id, &review_id);
        kernel.event_bus.emit_kb_stats_changed(&kb_id);
    }
    Ok(())
}

#[tauri::command]
pub async fn accept_all_low_risk_review(
    kernel: State<'_, Arc<AppKernel>>,
    kb_id: String,
) -> Result<serde_json::Value, String> {
    let conn = kernel.db.connect()?;
    // Find the latest pending review for this KB
    let review_id: String = conn
        .query_row(
            "SELECT id FROM reviews WHERE kb_id = ?1 AND status = 'pending' ORDER BY created_at DESC LIMIT 1",
            rusqlite::params![kb_id],
            |row| row.get(0),
        )
        .map_err(|_| "未找到待处理的审阅".to_string())?;
    let kb_path: String = conn
        .query_row("SELECT path FROM knowledge_bases WHERE id = ?1", rusqlite::params![kb_id], |row| row.get(0))
        .map_err(|e| format!("查询知识库路径失败: {}", e))?;
    drop(conn);
    accept_all_low_risk_review_sync(kernel.inner(), &review_id, &kb_id, &kb_path)
}

#[tauri::command]
pub async fn reject_all_review(
    kernel: State<'_, Arc<AppKernel>>,
    kb_id: String,
) -> Result<(), String> {
    let conn = kernel.db.connect()?;
    // Find the latest pending review for this KB
    let review_id: String = conn
        .query_row(
            "SELECT id FROM reviews WHERE kb_id = ?1 AND status = 'pending' ORDER BY created_at DESC LIMIT 1",
            rusqlite::params![kb_id],
            |row| row.get(0),
        )
        .map_err(|_| "未找到待处理的审阅".to_string())?;
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "UPDATE review_items SET status = 'rejected', updated_at = ?1 WHERE review_id = ?2 AND status = 'pending'",
        rusqlite::params![now, review_id],
    ).map_err(|e| format!("拒绝审阅失败: {}", e))?;

    conn.execute(
        "UPDATE reviews SET status = 'rejected', updated_at = ?1 WHERE id = ?2",
        rusqlite::params![now, review_id],
    ).map_err(|e| format!("更新审阅状态失败: {}", e))?;

    let task_id: String = match conn.query_row(
        "SELECT task_id FROM reviews WHERE id = ?1", rusqlite::params![review_id], |row| row.get(0),
    ) {
        Ok(id) => id,
        Err(rusqlite::Error::QueryReturnedNoRows) => String::new(),
        Err(e) => {
            log::error!("[review] 查询 review task_id 失败 (review={}): {}", review_id, e);
            return Err(format!("查询审阅任务ID失败: {}", e));
        }
    };

    if !task_id.is_empty() {
        if let Err(e) = conn.execute(
            "UPDATE tasks SET status = 'rejected', completed_at = ?1 WHERE id = ?2",
            rusqlite::params![now, task_id],
        ) {
            log::error!("[review] 更新 task rejected 状态失败 (task={}): {}", task_id, e);
        }
    }

    if let Ok(kb_id) = conn.query_row(
        "SELECT kb_id FROM reviews WHERE id = ?1", rusqlite::params![review_id], |row| row.get::<_, String>(0),
    ) {
        kernel.event_bus.emit_review_updated(&kb_id, &review_id);
        kernel.event_bus.emit_kb_stats_changed(&kb_id);
    }
    Ok(())
}

#[tauri::command]
pub async fn delete_review_item(
    kernel: State<'_, Arc<AppKernel>>,
    item_id: String,
) -> Result<(), String> {
    let conn = kernel.db.connect()?;
    let (review_id, kb_id): (String, String) = conn.query_row(
        "SELECT r.id, r.kb_id FROM reviews r JOIN review_items ri ON ri.review_id = r.id WHERE ri.id = ?1",
        rusqlite::params![item_id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    ).map_err(|e| format!("查询审阅项所属审阅失败: {}", e))?;
    conn.execute("DELETE FROM review_items WHERE id = ?1", rusqlite::params![item_id])
        .map_err(|e| format!("删除审阅项失败: {}", e))?;
    // 如果审阅组内没有剩余项，标记审阅为 completed
    let remaining: i64 = conn.query_row(
        "SELECT COUNT(*) FROM review_items WHERE review_id = ?1",
        rusqlite::params![review_id],
        |row| row.get(0),
    ).unwrap_or(0);
    if remaining == 0 {
        conn.execute(
            "UPDATE reviews SET status = 'completed', updated_at = ?1 WHERE id = ?2",
            rusqlite::params![chrono::Utc::now().to_rfc3339(), review_id],
        ).map_err(|e| format!("更新审阅状态失败: {}", e))?;
    }
    kernel.event_bus.emit_review_updated(&kb_id, &review_id);
    kernel.event_bus.emit_kb_stats_changed(&kb_id);
    Ok(())
}

#[tauri::command]
pub async fn regenerate_review(
    kernel: State<'_, Arc<AppKernel>>,
    review_id: String,
) -> Result<(), String> {
    let conn = kernel.db.connect()?;

    // 获取 review 信息
    let (kb_id, task_id): (String, String) = conn.query_row(
        "SELECT kb_id, task_id FROM reviews WHERE id = ?1",
        rusqlite::params![review_id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    ).map_err(|e| format!("查询审阅失败: {}", e))?;

    let kb_path: String = conn.query_row(
        "SELECT path FROM knowledge_bases WHERE id = ?1",
        rusqlite::params![kb_id],
        |row| row.get(0),
    ).map_err(|e| format!("查询知识库失败: {}", e))?;

    // 删除旧审阅项
    conn.execute(
        "DELETE FROM review_items WHERE review_id = ?1",
        rusqlite::params![review_id],
    ).map_err(|e| format!("删除旧审阅项失败: {}", e))?;

    // 检查 update_plan.json 是否仍存在
    let kb_path_buf = std::path::PathBuf::from(&kb_path);
    let tasks_dir = kernel.workspace.get_tasks_dir(&kb_path_buf);
    let task_dir = tasks_dir.join(&task_id);
    let update_plan_path = task_dir.join("update_plan.json");

    if update_plan_path.exists() {
        let update_data = std::fs::read_to_string(&update_plan_path)
            .map_err(|e| format!("读取 update_plan.json 失败: {}", e))?;
        let update_json: serde_json::Value = serde_json::from_str(&update_data)
            .map_err(|e| format!("解析 update_plan.json 失败: {}", e))?;

        crate::review::review_engine::ReviewEngine::generate_review(
            &kernel.db, &kb_id, &task_id, &update_json, &kb_path,
        )?;
    } else {
        // update_plan.json 不存在，重置任务回 queued 重新跑流水线
        // v0.2.3: 检查 cancel_flag，防止覆盖正在进行的取消
        let cancel_flag: i32 = conn.query_row(
            "SELECT cancel_flag FROM tasks WHERE id = ?1",
            rusqlite::params![task_id],
            |row| row.get(0),
        ).map_err(|e| format!("查询任务状态失败: {}", e))?;
        if cancel_flag != 0 {
            return Err("任务已被取消，无法重新生成审阅。请先恢复任务后再操作。".to_string());
        }
        conn.execute(
            "UPDATE tasks SET status = 'queued', error_message = '审阅重新生成，等待流水线执行', \
             cancel_flag = 0, cancel_reason = '', updated_at = ?1 WHERE id = ?2",
            rusqlite::params![chrono::Utc::now().to_rfc3339(), task_id],
        ).map_err(|e| format!("重置任务失败: {}", e))?;
    }

    kernel.event_bus.emit_review_updated(&kb_id, &review_id);
    Ok(())
}
