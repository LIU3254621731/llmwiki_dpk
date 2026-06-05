use std::sync::Arc;
use tauri::State;
use crate::core::app_kernel::AppKernel;

fn validate_page_path(page_path: &str) -> Result<(), String> {
    if page_path.contains("..") {
        return Err(format!("非法的页面路径（包含 ..）: {}", page_path));
    }
    Ok(())
}

#[tauri::command]
pub async fn list_wiki_pages(
    kernel: State<'_, Arc<AppKernel>>,
    kb_id: String,
) -> Result<Vec<serde_json::Value>, String> {
    let conn = kernel.db.connect()?;
    let mut stmt = conn
        .prepare("SELECT id, title, path, page_type, canonical_name, COALESCE(tags,''), created_at, updated_at FROM wiki_pages WHERE kb_id = ?1 ORDER BY page_type, title")
        .map_err(|e| format!("查询页面失败: {}", e))?;

    let pages = stmt
        .query_map(rusqlite::params![kb_id], |row| {
            Ok(serde_json::json!({
                "id": row.get::<_, String>(0)?,
                "title": row.get::<_, String>(1)?,
                "path": row.get::<_, String>(2)?,
                "page_type": row.get::<_, String>(3)?,
                "canonical_name": row.get::<_, String>(4)?,
                "tags": row.get::<_, String>(5)?,
                "created_at": row.get::<_, String>(6)?,
                "updated_at": row.get::<_, String>(7)?,
            }))
        })
        .map_err(|e| format!("映射页面失败: {}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("收集页面失败: {}", e))?;

    Ok(pages)
}

#[tauri::command]
pub async fn get_wiki_page_content(
    _kernel: State<'_, Arc<AppKernel>>,
    kb_path: String,
    page_path: String,
) -> Result<String, String> {
    validate_page_path(&page_path)?;
    let kb_path_buf = std::path::PathBuf::from(&kb_path);
    let normalized = crate::wiki::path_service::PathService::normalize_workspace_path(&page_path);
    let full_path = crate::wiki::path_service::PathService::resolve_workspace_path(&kb_path_buf, &normalized);

    if !full_path.exists() {
        return Err(format!("页面文件不存在: {} (尝试路径: {})", normalized, full_path.display()));
    }

    std::fs::read_to_string(&full_path)
        .map_err(|e| format!("读取页面失败: {}", e))
}

#[tauri::command]
pub async fn save_wiki_page(
    kernel: State<'_, Arc<AppKernel>>,
    kb_id: String,
    kb_path: String,
    page_type: String,
    title: String,
    content: String,
    page_path: Option<String>,
) -> Result<(), String> {
    if title.trim().is_empty() {
        return Err("页面标题不能为空".to_string());
    }

    let wiki_dir = std::path::PathBuf::from(&kb_path).join("wiki");
    let writer = crate::wiki::wiki_writer::WikiWriter::new(kernel.db.clone());

    // 若指定了已有页面路径，按更新处理
    if let Some(ref existing_path) = page_path {
        if !existing_path.is_empty() {
            validate_page_path(existing_path)?;
            let conn = kernel.db.connect()?;
            let page_exists: bool = match conn.query_row(
                "SELECT COUNT(1) > 0 FROM wiki_pages WHERE kb_id = ?1 AND path = ?2",
                rusqlite::params![kb_id, existing_path],
                |row| row.get(0),
            ) {
                Ok(v) => v,
                Err(rusqlite::Error::QueryReturnedNoRows) => false,
                Err(e) => return Err(format!("查询页面是否存在失败: {}", e)),
            };

            if page_exists {
                writer.update_page(&kb_id, &wiki_dir, existing_path, &content, "manual")?;
                crate::wiki::log_service::LogService::append_log(
                    &wiki_dir, "更新页面", &format!("手动更新: {}", existing_path),
                )?;
                kernel.event_bus.emit_wiki_updated(&kb_id, existing_path);
                kernel.event_bus.emit_kb_stats_changed(&kb_id);
                return Ok(());
            }
        }
    }

    // 检查是否已有同标题页面（兼容旧调用）
    let canonical_name = crate::wiki::wiki_writer::WikiWriter::generate_canonical_name(&title);
    let safe_canonical = crate::wiki::path_service::PathService::generate_safe_name(&canonical_name);
    let conn = kernel.db.connect()?;
    let existing_path: Option<String> = match conn.query_row(
        "SELECT path FROM wiki_pages WHERE kb_id = ?1 AND (title = ?2 OR canonical_name = ?3) LIMIT 1",
        rusqlite::params![kb_id, title, safe_canonical],
        |row| row.get(0),
    ) {
        Ok(p) => Some(p),
        Err(rusqlite::Error::QueryReturnedNoRows) => None,
        Err(e) => return Err(format!("查询已有页面路径失败: {}", e)),
    };

    if let Some(ref path) = existing_path {
        writer.update_page(&kb_id, &wiki_dir, path, &content, "manual")?;
        crate::wiki::log_service::LogService::append_log(
            &wiki_dir, "更新页面", &format!("按标题更新 {} → {}", title, path),
        )?;
    } else {
        writer.create_page(&kb_id, &wiki_dir, &page_type, &title, &canonical_name, &content, "")?;
        crate::wiki::log_service::LogService::append_log(
            &wiki_dir, "创建页面", &format!("创建 {} 页面: {}", page_type, title),
        )?;
    }

    kernel.event_bus.emit_wiki_updated(&kb_id, "");
    kernel.event_bus.emit_kb_stats_changed(&kb_id);
    Ok(())
}

#[tauri::command]
pub async fn delete_wiki_page(
    kernel: State<'_, Arc<AppKernel>>,
    kb_id: String,
    page_path: String,
) -> Result<(), String> {
    let conn = kernel.db.connect()?;
    let kb_path: String = conn
        .query_row("SELECT path FROM knowledge_bases WHERE id = ?1", rusqlite::params![kb_id], |row| row.get(0))
        .map_err(|e| format!("查询知识库路径失败: {}", e))?;
    drop(conn);
    validate_page_path(&page_path)?;
    let normalized = crate::wiki::wiki_writer::WikiWriter::normalize_path(&page_path);
    let kb_path_buf = std::path::PathBuf::from(&kb_path);
    let full_path = crate::wiki::wiki_writer::WikiWriter::resolve_absolute_path(&kb_path_buf, &normalized);
    let wiki_dir = kb_path_buf.join("wiki");

    let conn = kernel.db.connect()?;

    // 将所有级联 DELETE 包装在事务中，防止中途失败导致数据库不一致
    conn.execute("BEGIN TRANSACTION", [])
        .map_err(|e| format!("开始事务失败: {}", e))?;

    let delete_result = (|| -> Result<(), String> {
        // 级联清理：查关联的 knowledge_items
        {
            let mut stmt = conn.prepare(
                "SELECT id FROM knowledge_items WHERE kb_id = ?1 AND (page_path = ?2 OR page_path = ?3)"
            ).map_err(|e| format!("查询关联知识项失败: {}", e))?;
            let ki_ids: Vec<String> = stmt.query_map(
                rusqlite::params![kb_id, normalized, page_path],
                |row| row.get(0),
            ).map_err(|e| format!("查询关联知识项失败: {}", e))?
                .filter_map(|r| r.ok())
                .collect();

            for ki_id in &ki_ids {
                conn.execute("DELETE FROM aliases WHERE item_id = ?1", rusqlite::params![ki_id])
                    .map_err(|e| format!("删除 aliases 失败: {}", e))?;
                conn.execute(
                    "DELETE FROM relationships WHERE kb_id = ?1 AND (source_item_id = ?2 OR target_item_id = ?2)",
                    rusqlite::params![kb_id, ki_id],
                ).map_err(|e| format!("删除 relationships 失败: {}", e))?;
            }
            conn.execute(
                "DELETE FROM knowledge_items WHERE kb_id = ?1 AND (page_path = ?2 OR page_path = ?3)",
                rusqlite::params![kb_id, normalized, page_path],
            ).map_err(|e| format!("删除 knowledge_items 失败: {}", e))?;
        }

        // 清理 graph_edges
        conn.execute(
            "DELETE FROM graph_edges WHERE kb_id = ?1 AND (source_node_id IN (SELECT id FROM graph_nodes WHERE kb_id = ?1 AND path = ?2) OR target_node_id IN (SELECT id FROM graph_nodes WHERE kb_id = ?1 AND path = ?2))",
            rusqlite::params![kb_id, normalized],
        ).map_err(|e| format!("清理 graph_edges 失败: {}", e))?;

        // 清理 graph_nodes
        conn.execute(
            "DELETE FROM graph_nodes WHERE kb_id = ?1 AND path = ?2",
            rusqlite::params![kb_id, normalized],
        ).map_err(|e| format!("清理 graph_nodes 失败: {}", e))?;

        // 清理引用此页面的审阅事件
        conn.execute(
            "DELETE FROM review_item_events WHERE review_item_id IN (SELECT id FROM review_items WHERE review_id IN (SELECT id FROM reviews WHERE kb_id = ?1) AND target_path = ?2)",
            rusqlite::params![kb_id, normalized],
        ).map_err(|e| format!("清理 review_item_events 失败: {}", e))?;

        // 清理引用此页面的审阅项
        conn.execute(
            "DELETE FROM review_items WHERE review_id IN (SELECT id FROM reviews WHERE kb_id = ?1) AND target_path = ?2",
            rusqlite::params![kb_id, normalized],
        ).map_err(|e| format!("清理 review_items 失败: {}", e))?;

        // 清理关联的 operations
        conn.execute(
            "DELETE FROM operations WHERE kb_id = ?1 AND target_path = ?2",
            rusqlite::params![kb_id, normalized],
        ).map_err(|e| format!("清理 operations 失败: {}", e))?;

        // 清理版本快照
        conn.execute(
            "DELETE FROM versions WHERE kb_id = ?1 AND page_path = ?2",
            rusqlite::params![kb_id, normalized],
        ).map_err(|e| format!("清理 versions 失败: {}", e))?;

        conn.execute(
            "DELETE FROM wiki_pages WHERE kb_id = ?1 AND path = ?2",
            rusqlite::params![kb_id, normalized],
        ).or_else(|_| {
            conn.execute(
                "DELETE FROM wiki_pages WHERE kb_id = ?1 AND path = ?2",
                rusqlite::params![kb_id, page_path],
            )
        }).map_err(|e| format!("删除页面记录失败: {}", e))?;

        Ok(())
    })();

    match delete_result {
        Ok(()) => {
            conn.execute("COMMIT", [])
                .map_err(|e| format!("提交事务失败: {}", e))?;
        }
        Err(e) => {
            let _ = conn.execute("ROLLBACK", []);
            return Err(e);
        }
    }

    // 数据库已成功提交后，再删除磁盘文件
    if full_path.exists() {
        if let Err(e) = std::fs::remove_file(&full_path) {
            log::error!("[delete_wiki_page] 删除页面文件失败 ({}): {}", full_path.display(), e);
        }
    } else {
        let alt_path = wiki_dir.join(&normalized);
        if alt_path.exists() {
            if let Err(e) = std::fs::remove_file(&alt_path) {
                log::error!("[delete_wiki_page] 删除页面文件失败 ({}): {}", alt_path.display(), e);
            }
        }
    }

    crate::wiki::log_service::LogService::append_log(
        &wiki_dir, "删除页面", &format!("已删除页面: {}", normalized),
    )?;

    kernel.event_bus.emit_wiki_updated(&kb_id, "");
    kernel.event_bus.emit_kb_stats_changed(&kb_id);
    Ok(())
}

#[tauri::command]
pub async fn get_wiki_page_versions(
    kernel: State<'_, Arc<AppKernel>>,
    kb_id: String,
    page_path: String,
) -> Result<Vec<serde_json::Value>, String> {
    validate_page_path(&page_path)?;
    let normalized = crate::wiki::wiki_writer::WikiWriter::normalize_path(&page_path);
    let vm = crate::wiki::version_manager::VersionManager::new(kernel.db.clone());
    let versions = vm.get_versions(&kb_id, &normalized)?;

    Ok(versions.iter().map(|v| serde_json::json!({
        "id": v.id,
        "content_hash": v.content_hash,
        "task_id": v.task_id,
        "created_at": v.created_at,
    })).collect())
}

#[tauri::command]
pub async fn rollback_wiki_page(
    kernel: State<'_, Arc<AppKernel>>,
    kb_id: String,
    kb_path: String,
    version_id: String,
) -> Result<(), String> {
    let wiki_dir = std::path::PathBuf::from(&kb_path).join("wiki");
    let vm = crate::wiki::version_manager::VersionManager::new(kernel.db.clone());
    vm.rollback(&kb_id, &wiki_dir, &version_id)?;
    // 回滚成功后发出事件，前端 Wiki 列表和 Dashboard 统计实时刷新
    kernel.event_bus.emit_wiki_updated(&kb_id, "");
    kernel.event_bus.emit_kb_stats_changed(&kb_id);
    Ok(())
}

#[tauri::command]
pub async fn list_page_versions(
    kernel: State<'_, Arc<AppKernel>>,
    kb_id: String,
    page_path: String,
) -> Result<Vec<serde_json::Value>, String> {
    validate_page_path(&page_path)?;
    let normalized = crate::wiki::wiki_writer::WikiWriter::normalize_path(&page_path);
    let vm = crate::wiki::version_manager::VersionManager::new(kernel.db.clone());
    let versions = vm.get_versions(&kb_id, &normalized)?;

    Ok(versions.iter().map(|v| serde_json::json!({
        "id": v.id,
        "kb_id": v.kb_id,
        "page_path": v.page_path,
        "content_hash": v.content_hash,
        "snapshot_path": v.snapshot_path,
        "task_id": v.task_id,
        "operation_id": v.operation_id,
        "created_at": v.created_at,
    })).collect())
}

#[tauri::command]
pub async fn get_page_version_snapshot(
    kernel: State<'_, Arc<AppKernel>>,
    kb_id: String,
    version_id: String,
) -> Result<String, String> {
    let conn = kernel.db.connect()?;
    let (kb_path, snapshot_name): (String, String) = conn
        .query_row(
            "SELECT kb.path, v.snapshot_path
             FROM versions v
             JOIN knowledge_bases kb ON kb.id = v.kb_id
             WHERE v.id = ?1 AND v.kb_id = ?2",
            rusqlite::params![version_id, kb_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|e| format!("查询版本快照失败: {}", e))?;

    let snapshot_file = std::path::PathBuf::from(&kb_path)
        .join("versions")
        .join("snapshots")
        .join(&snapshot_name);

    if !snapshot_file.exists() {
        return Err(format!("快照文件不存在: {}", snapshot_file.display()));
    }

    std::fs::read_to_string(&snapshot_file)
        .map_err(|e| format!("读取快照文件失败: {}", e))
}

#[tauri::command]
pub async fn get_index_content(
    kb_path: String,
) -> Result<String, String> {
    let path = std::path::PathBuf::from(&kb_path).join("wiki/index.md");
    std::fs::read_to_string(&path)
        .map_err(|e| format!("读取 index.md 失败: {}", e))
}

#[tauri::command]
pub async fn get_log_content(
    kb_path: String,
) -> Result<String, String> {
    let path = std::path::PathBuf::from(&kb_path).join("wiki/log.md");
    std::fs::read_to_string(&path)
        .map_err(|e| format!("读取 log.md 失败: {}", e))
}

/// Resolve a wikilink text to the actual page path, title, and content.
/// Searches by canonical_name (slugified) and raw title across all page type directories.
#[tauri::command]
pub async fn resolve_wiki_link(
    kernel: State<'_, Arc<AppKernel>>,
    kb_id: String,
    kb_path: String,
    link_text: String,
) -> Result<Option<serde_json::Value>, String> {
    let safe_name = crate::wiki::path_service::PathService::generate_safe_name(&link_text);
    let conn = kernel.db.connect()?;

    // Try matching by canonical_name or title
    let mut stmt = conn
        .prepare(
            "SELECT id, title, path, page_type, canonical_name, tags, created_at, updated_at
             FROM wiki_pages WHERE kb_id = ?1 AND (canonical_name = ?2 OR title = ?3 OR canonical_name LIKE ?4 OR title LIKE ?5)
             LIMIT 1",
        )
        .map_err(|e| format!("查询 wiki_pages 失败: {}", e))?;

    let like_pattern = format!("%{}%", safe_name);
    let result = stmt
        .query_row(
            rusqlite::params![kb_id, safe_name, link_text, like_pattern, like_pattern],
            |row| {
                Ok(serde_json::json!({
                    "id": row.get::<_, String>(0)?,
                    "title": row.get::<_, String>(1)?,
                    "path": row.get::<_, String>(2)?,
                    "page_type": row.get::<_, String>(3)?,
                    "canonical_name": row.get::<_, String>(4)?,
                    "tags": row.get::<_, String>(5).unwrap_or_default(),
                    "created_at": row.get::<_, String>(6)?,
                    "updated_at": row.get::<_, String>(7)?,
                }))
            },
        )
        .map(Some)
        .or_else(|e| {
            if e == rusqlite::Error::QueryReturnedNoRows {
                Ok(None)
            } else {
                Err(e)
            }
        })
        .map_err(|e| format!("解析 wiki link 失败: {}", e))?;

    if let Some(ref page) = result {
        // Read the actual file content
        let page_path = page["path"].as_str().unwrap_or("");
        let full_path = crate::wiki::path_service::PathService::resolve_workspace_path(
            &std::path::PathBuf::from(&kb_path),
            page_path,
        );
        if full_path.exists() {
            if let Ok(content) = std::fs::read_to_string(&full_path) {
                let mut page_with_content = page.clone();
                page_with_content["content"] = serde_json::Value::String(content);
                return Ok(Some(page_with_content));
            }
        }
    }

    Ok(result)
}
