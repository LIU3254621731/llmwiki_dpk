use std::sync::Arc;
use tauri::{State, Emitter};
use walkdir::WalkDir;
use crate::core::app_kernel::AppKernel;
use crate::skills::document_processor::DocumentProcessor;

/// Stage a file into the workspace (copy + DB + text extraction). No AI pipeline trigger.
/// `relative_subdir`: optional subdirectory path under documents/ or images/ root.
async fn stage_file(
    kernel: &AppKernel,
    kb_id: &str,
    kb_path: &str,
    file_path: &str,
    relative_subdir: Option<&str>,
) -> Result<serde_json::Value, String> {
    let src_path = std::path::PathBuf::from(file_path);

    if !src_path.exists() {
        return Err(format!("文件不存在: {}", file_path));
    }

    let file_name = src_path.file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    let extension = DocumentProcessor::get_extension(&file_name);

    if !DocumentProcessor::is_supported(&extension) {
        return Err(format!("不支持的文件类型: .{}", extension));
    }

    DocumentProcessor::check_file_size(&src_path, 50)?;

    // Asset routing
    if DocumentProcessor::is_asset_type(&extension) {
        let dest_dir = if let Some(subdir) = relative_subdir {
            kernel.workspace.get_images_dir(&std::path::PathBuf::from(kb_path)).join(subdir)
        } else {
            kernel.workspace.get_images_dir(&std::path::PathBuf::from(kb_path))
        };
        std::fs::create_dir_all(&dest_dir).map_err(|e| format!("创建资产目录失败: {}", e))?;

        let dest_path = dest_dir.join(&*file_name);
        std::fs::copy(&src_path, &dest_path).map_err(|e| format!("复制资产文件失败: {}", e))?;

        let file_size = DocumentProcessor::check_file_size(&src_path, 50)?;
        let file_hash = DocumentProcessor::compute_file_hash(&dest_path)?;

        let id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();

        let conn = kernel.db.connect()?;
        conn.execute(
            "INSERT INTO assets (id, kb_id, file_name, file_path, file_type, file_size, file_hash, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            rusqlite::params![id, kb_id, file_name, dest_path.to_string_lossy(), extension, file_size as i64, file_hash, now],
        ).map_err(|e| format!("保存资产记录失败: {}", e))?;

        return Ok(serde_json::json!({
            "id": id,
            "type": "asset",
            "file_name": file_name,
            "status": "staged",
        }));
    }

    // Document routing — use a transaction to ensure atomicity
    let conn = kernel.db.connect()?;

    let file_hash = DocumentProcessor::compute_file_hash(&src_path)?;

    // Begin transaction: duplicate check + insert
    conn.execute("BEGIN TRANSACTION", []).map_err(|e| format!("开始事务失败: {}", e))?;

    let existing: Option<String> = match conn.query_row(
        "SELECT id FROM sources WHERE kb_id = ?1 AND file_hash = ?2",
        rusqlite::params![kb_id, file_hash],
        |row| row.get(0),
    ) {
        Ok(id) => Some(id),
        Err(rusqlite::Error::QueryReturnedNoRows) => None,
        Err(e) => {
            if let Err(e) = conn.execute("ROLLBACK", []) {
            log::error!("[stage_file] ROLLBACK failed: {}", e);
        }
            return Err(format!("查询文件去重状态失败: {}", e));
        }
    };

    if let Some(ref existing_id) = existing {
        let _ = conn.execute(
            "UPDATE sources SET status = 'duplicate', updated_at = ?1 WHERE id = ?2 AND status = 'pending'",
            rusqlite::params![chrono::Utc::now().to_rfc3339(), existing_id],
        );
        let _ = conn.execute("COMMIT", []);
        kernel.event_bus.emit_source_updated(kb_id, existing_id);
        kernel.event_bus.emit_kb_stats_changed(kb_id);
        return Ok(serde_json::json!({
            "id": existing_id,
            "type": "document",
            "file_name": file_name,
            "status": "duplicate",
        }));
    }

    // Copy file to workspace
    let docs_dir = if let Some(subdir) = relative_subdir {
        kernel.workspace.get_documents_dir(&std::path::PathBuf::from(kb_path)).join(subdir)
    } else {
        kernel.workspace.get_documents_dir(&std::path::PathBuf::from(kb_path))
    };
    std::fs::create_dir_all(&docs_dir).map_err(|e| {
        if let Err(e) = conn.execute("ROLLBACK", []) {
            log::error!("[stage_file] ROLLBACK failed: {}", e);
        }
        format!("创建文档目录失败: {}", e)
    })?;
    let dest_path = docs_dir.join(&*file_name);
    std::fs::copy(&src_path, &dest_path).map_err(|e| {
        if let Err(e) = conn.execute("ROLLBACK", []) {
            log::error!("[stage_file] ROLLBACK failed: {}", e);
        }
        format!("复制文件失败: {}", e)
    })?;

    let file_size = std::fs::metadata(&dest_path).map(|m| m.len() as i64).unwrap_or_else(|e| {
        log::warn!("[stage_file] 无法获取文件大小 ({}): {}", dest_path.display(), e);
        0
    });
    let source_id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    let insert_result = conn.execute(
        "INSERT INTO sources (id, kb_id, file_name, file_path, file_type, file_size, file_hash, text_length, status, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 0, 'pending', ?8, ?8)",
        rusqlite::params![source_id, kb_id, file_name, dest_path.to_string_lossy(), extension, file_size, file_hash, now],
    );
    if let Err(e) = insert_result {
        if let Err(e) = conn.execute("ROLLBACK", []) {
            log::error!("[stage_file] ROLLBACK failed: {}", e);
        }
        let _ = std::fs::remove_file(&dest_path);
        let err_str = e.to_string();
        if err_str.contains("UNIQUE constraint") && err_str.contains("sources") {
            return Err("该文件已导入过（hash 匹配），请勿重复导入。".to_string());
        }
        return Err(format!("保存源文件记录失败: {}", e));
    }

    // Text extraction
    let parse_result = DocumentProcessor::parse_document(&dest_path, &extension);
    let _ = match &parse_result {
        Ok(result) => {
            let text_len = result.text_length as i64;
            let page_count = result.page_count.map(|p| p as i64);
            if let Err(e) = conn.execute(
                "UPDATE sources SET extracted_text = ?1, text_length = ?2, page_count = ?3 WHERE id = ?4",
                rusqlite::params![result.text, text_len, page_count, source_id],
            ) {
                log::warn!("[stage_file] 保存提取文本到数据库失败: {}", e);
                if let Err(e) = conn.execute("ROLLBACK", []) {
            log::error!("[stage_file] ROLLBACK failed: {}", e);
        }
                let _ = std::fs::remove_file(&dest_path);
                return Err(format!("保存提取文本到数据库失败: {}", e));
            }
            Some(())
        }
        Err(_) => None,
    };

    // Commit transaction — everything succeeded
    conn.execute("COMMIT", []).map_err(|e| {
        if let Err(e) = conn.execute("ROLLBACK", []) {
            log::error!("[stage_file] ROLLBACK failed: {}", e);
        }
        let _ = std::fs::remove_file(&dest_path);
        format!("提交事务失败: {}", e)
    })?;

    kernel.event_bus.emit_source_updated(kb_id, &source_id);
    kernel.event_bus.emit_kb_stats_changed(kb_id);

    Ok(serde_json::json!({
        "id": source_id,
        "type": "document",
        "file_name": file_name,
        "file_type": extension,
        "file_size": file_size,
        "file_hash": file_hash,
        "parse_warning": parse_result.as_ref().err().cloned(),
        "status": "staged",
    }))
}

/// 上传前预检：计算文件哈希、提取元数据、检查重复
#[tauri::command]
pub async fn check_file_hash(
    kernel: State<'_, Arc<AppKernel>>,
    kb_id: String,
    file_path: String,
) -> Result<serde_json::Value, String> {
    let src_path = std::path::PathBuf::from(&file_path);

    if !src_path.exists() {
        return Err(format!("文件不存在: {}", file_path));
    }

    let file_name = src_path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    let extension = DocumentProcessor::get_extension(&file_name);

    if !DocumentProcessor::is_supported(&extension) {
        return Err(format!("不支持的文件类型: .{}", extension));
    }

    DocumentProcessor::check_file_size(&src_path, 50)?;

    let file_hash = DocumentProcessor::compute_file_hash(&src_path)?;
    let file_size = std::fs::metadata(&src_path)
        .map(|m| m.len())
        .unwrap_or(0);

    // 检查是否存在相同哈希的 source
    let conn = kernel.db.connect()?;
    let existing: Option<serde_json::Value> = match conn.query_row(
        "SELECT id, file_name, file_type, status, created_at FROM sources WHERE kb_id = ?1 AND file_hash = ?2",
        rusqlite::params![kb_id, file_hash],
        |row| {
            Ok(serde_json::json!({
                "id": row.get::<_, String>(0)?,
                "file_name": row.get::<_, String>(1)?,
                "file_type": row.get::<_, String>(2)?,
                "status": row.get::<_, String>(3)?,
                "created_at": row.get::<_, String>(4)?,
            }))
        },
    ) {
        Ok(v) => Some(v),
        Err(rusqlite::Error::QueryReturnedNoRows) => None,
        Err(e) => return Err(format!("查询文件去重状态失败: {}", e)),
    };

    // 尝试提取文本用于预估
    let text_length = match DocumentProcessor::parse_document(&src_path, &extension) {
        Ok(result) => result.text_length,
        Err(_) => 0usize,
    };

    Ok(serde_json::json!({
        "file_hash": file_hash,
        "file_size": file_size,
        "text_length": text_length,
        "is_duplicate": existing.is_some(),
        "existing_source": existing,
    }))
}

#[tauri::command]
pub async fn upload_source_file(
    kernel: State<'_, Arc<AppKernel>>,
    kb_id: String,
    file_path: String,
) -> Result<serde_json::Value, String> {
    let conn = kernel.db.connect()?;
    let kb_path: String = conn
        .query_row(
            "SELECT path FROM knowledge_bases WHERE id = ?1",
            rusqlite::params![kb_id],
            |row| row.get(0),
        )
        .map_err(|e| format!("查询知识库路径失败: {}", e))?;
    drop(conn);
    let result = stage_file(&kernel, &kb_id, &kb_path, &file_path, None).await?;

    let file_type = result["type"].as_str().unwrap_or("");
    let status = result["status"].as_str().unwrap_or("");

    if file_type == "asset" {
        let file_name = result["file_name"].as_str().unwrap_or("");
        kernel.event_bus.emit_notification("info", "资产已保存", &format!("{} 已保存为资产，不参与 AI 知识抽取", file_name));
        return Ok(serde_json::json!({
            "id": result["id"],
            "type": "asset",
            "file_name": file_name,
            "message": "图片仅保存为资产，不参与 AI 知识抽取",
        }));
    }

    if status == "duplicate" {
        return Err("该文件已导入过（hash 匹配），请勿重复导入。".to_string());
    }

    // Document — trigger AI pipeline
    let source_id = result["id"].as_str().unwrap_or("").to_string();

    let tq = Arc::new(crate::core::task_queue::TaskQueue::new(kernel.db.clone(), kernel.event_bus.clone()));
    let coordinator = crate::agents::coordinator::CoordinatorAgent::new(
        tq.clone(), kernel.db.clone(), kernel.config.clone(),
        kernel.secrets.clone(), kernel.workspace.clone(), kernel.event_bus.clone(),
        kernel.token_logger.clone(), kernel.vdb.clone(),
    );

    let task_id = coordinator.run_source_ingest(&kb_id, &kb_path, &source_id).await?;

    let conn = kernel.db.connect()?;
    let (extracted_text, created_at): (String, String) = conn.query_row(
        "SELECT COALESCE(extracted_text,''), COALESCE(created_at,'') FROM sources WHERE id = ?1",
        rusqlite::params![source_id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    ).map_err(|e| format!("查询 source 信息失败: {}", e))?;

    Ok(serde_json::json!({
        "id": source_id,
        "task_id": task_id,
        "type": "document",
        "file_name": result["file_name"],
        "file_type": result["file_type"],
        "file_size": result["file_size"],
        "file_hash": result["file_hash"],
        "text_length": if extracted_text.is_empty() { 0 } else { extracted_text.len() },
        "parse_warning": result["parse_warning"],
        "created_at": created_at,
    }))
}

#[tauri::command]
pub async fn list_sources(
    kernel: State<'_, Arc<AppKernel>>,
    kb_id: String,
) -> Result<Vec<serde_json::Value>, String> {
    let conn = kernel.db.connect()?;
    let mut stmt = conn
        .prepare("SELECT s.id, s.file_name, s.file_path, s.file_type, s.file_size, s.file_hash, s.status, s.created_at, s.updated_at, COALESCE(s.ai_summary,''), COALESCE(s.coverage_report,'') FROM sources s WHERE s.kb_id = ?1 ORDER BY s.created_at DESC")
        .map_err(|e| format!("查询 sources 失败: {}", e))?;

    let sources = stmt
        .query_map(rusqlite::params![kb_id], |row| {
            let source_id: String = row.get(0)?;
            Ok((source_id, row.get::<_, String>(1)?, row.get::<_, String>(2)?, row.get::<_, String>(3)?, row.get::<_, i64>(4)?, row.get::<_, String>(5)?, row.get::<_, String>(6)?, row.get::<_, String>(7)?, row.get::<_, String>(8)?, row.get::<_, String>(9)?, row.get::<_, String>(10)?))
        })
        .map_err(|e| format!("映射 sources 失败: {}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("收集 sources 失败: {}", e))?;

    Ok(sources.iter().map(|(sid, fname, fpath, ftype, fsize, fhash, st, ca, _ua, ai_sum, cov)| {
        // 查询关联的 wiki 页数
        let linked_pages: i64 = match conn.query_row(
            "SELECT COUNT(*) FROM wiki_pages WHERE kb_id = ?1 AND id IN (SELECT item_id FROM knowledge_items WHERE kb_id = ?1 AND source_id = ?2)",
            rusqlite::params![kb_id, sid],
            |row| row.get(0),
        ) {
            Ok(c) => c,
            Err(rusqlite::Error::QueryReturnedNoRows) => 0,
            Err(e) => { log::error!("[source] 查询关联页面数失败 (source={}): {}", sid, e); 0 }
        };

        // 查询最近任务
        let recent_task: Option<String> = match conn.query_row(
            "SELECT t.id FROM tasks t WHERE t.kb_id = ?1 AND t.input_ref = ?2 ORDER BY t.created_at DESC LIMIT 1",
            rusqlite::params![kb_id, sid],
            |row| row.get(0),
        ) {
            Ok(id) => Some(id),
            Err(rusqlite::Error::QueryReturnedNoRows) => None,
            Err(e) => { log::error!("[source] 查询最近任务失败 (source={}): {}", sid, e); None }
        };

        // 查询关联 review 数
        let review_count: i64 = match conn.query_row(
            "SELECT COUNT(DISTINCT r.id) FROM reviews r JOIN review_items ri ON ri.review_id = r.id WHERE r.kb_id = ?1 AND ri.source_id = ?2",
            rusqlite::params![kb_id, sid],
            |row| row.get(0),
        ) {
            Ok(c) => c,
            Err(rusqlite::Error::QueryReturnedNoRows) => 0,
            Err(e) => { log::error!("[source] 查询关联审阅数失败 (source={}): {}", sid, e); 0 }
        };

        serde_json::json!({
            "id": sid,
            "file_name": fname,
            "file_path": fpath,
            "file_type": ftype,
            "file_size": fsize,
            "file_hash": fhash,
            "status": st,
            "created_at": ca,
            "updated_at": _ua,
            "ai_summary": ai_sum,
            "coverage_report": cov,
            "linked_pages_count": linked_pages,
            "review_count": review_count,
            "recent_task_id": recent_task,
        })
    }).collect())
}

#[tauri::command]
pub async fn get_source_detail(
    kernel: State<'_, Arc<AppKernel>>,
    source_id: String,
) -> Result<serde_json::Value, String> {
    let conn = kernel.db.connect()?;
    conn.query_row(
        "SELECT id, kb_id, file_name, file_path, file_type, file_size, file_hash, COALESCE(extracted_text,''), status, created_at, updated_at FROM sources WHERE id = ?1",
        rusqlite::params![source_id],
        |row| {
            Ok(serde_json::json!({
                "id": row.get::<_, String>(0)?,
                "kb_id": row.get::<_, String>(1)?,
                "file_name": row.get::<_, String>(2)?,
                "file_path": row.get::<_, String>(3)?,
                "file_type": row.get::<_, String>(4)?,
                "file_size": row.get::<_, i64>(5)?,
                "file_hash": row.get::<_, String>(6)?,
                "extracted_text": row.get::<_, String>(7)?,
                "status": row.get::<_, String>(8)?,
                "created_at": row.get::<_, String>(9)?,
                "updated_at": row.get::<_, String>(10)?,
            }))
        },
    )
    .map_err(|e| format!("获取 source 详情失败: {}", e))
}

#[tauri::command]
pub async fn delete_source(
    kernel: State<'_, Arc<AppKernel>>,
    source_id: String,
) -> Result<(), String> {
    let conn = kernel.db.connect()?;

    // 预先查询所有需要的信息（在事务外）
    let file_path: Option<String> = match conn.query_row(
        "SELECT file_path FROM sources WHERE id = ?1",
        rusqlite::params![source_id],
        |row| row.get(0),
    ) {
        Ok(p) => Some(p),
        Err(rusqlite::Error::QueryReturnedNoRows) => None,
        Err(e) => return Err(format!("查询 Source 文件路径失败: {}", e)),
    };

    let source_kb_id: Option<String> = match conn.query_row(
        "SELECT kb_id FROM sources WHERE id = ?1",
        rusqlite::params![source_id],
        |row| row.get(0),
    ) {
        Ok(k) => Some(k),
        Err(rusqlite::Error::QueryReturnedNoRows) => None,
        Err(e) => return Err(format!("查询 Source KB ID 失败: {}", e)),
    };

    // 收集即将删除的 wiki 页面磁盘路径（在事务外查询，供事务提交后清理）
    let mut page_disk_paths: Vec<(String, String)> = Vec::new(); // (kb_path, wiki_page_path)
    {
        let mut stmt = conn.prepare(
            "SELECT DISTINCT ki.page_id FROM knowledge_items ki WHERE ki.source_id = ?1 AND ki.page_id != '' AND ki.page_id IS NOT NULL"
        ).map_err(|e| format!("查询关联页面失败: {}", e))?;
        let page_ids: Vec<String> = stmt.query_map(rusqlite::params![source_id], |row| row.get(0))
            .map_err(|e| format!("查询关联页面失败: {}", e))?
            .filter_map(|r| r.ok())
            .collect();

        for page_id in &page_ids {
            let remaining: i64 = conn.query_row(
                "SELECT COUNT(*) FROM knowledge_items WHERE page_id = ?1 AND source_id != ?2",
                rusqlite::params![page_id, source_id],
                |row| row.get(0),
            ).unwrap_or(0);
            if remaining > 0 {
                continue;
            }
            if let Ok((wp_path, wp_kb_id)) = conn.query_row(
                "SELECT path, kb_id FROM wiki_pages WHERE id = ?1",
                rusqlite::params![page_id],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            ) {
                if let Ok(kb_path) = conn.query_row(
                    "SELECT path FROM knowledge_bases WHERE id = ?1",
                    rusqlite::params![wp_kb_id],
                    |row| row.get::<_, String>(0),
                ) {
                    page_disk_paths.push((kb_path, wp_path));
                }
            }
        }
    }

    // 将所有级联 DELETE 包装在事务中
    conn.execute("BEGIN TRANSACTION", [])
        .map_err(|e| format!("开始事务失败: {}", e))?;

    let delete_result = (|| -> Result<(), String> {
        // 级联清理：先查关联的 knowledge_items，删除其 aliases/relationships 和自身
        {
            let mut stmt = conn.prepare(
                "SELECT id FROM knowledge_items WHERE source_id = ?1"
            ).map_err(|e| format!("查询关联知识项失败: {}", e))?;
            let ki_ids: Vec<String> = stmt.query_map(rusqlite::params![source_id], |row| row.get(0))
                .map_err(|e| format!("查询关联知识项失败: {}", e))?
                .filter_map(|r| r.ok())
                .collect();

            for ki_id in &ki_ids {
                conn.execute("DELETE FROM aliases WHERE item_id = ?1", rusqlite::params![ki_id])
                    .map_err(|e| format!("删除 aliases 失败: {}", e))?;
                conn.execute(
                    "DELETE FROM relationships WHERE (source_item_id = ?1 OR target_item_id = ?1)",
                    rusqlite::params![ki_id],
                ).map_err(|e| format!("删除 relationships 失败: {}", e))?;
            }
            conn.execute("DELETE FROM knowledge_items WHERE source_id = ?1", rusqlite::params![source_id])
                .map_err(|e| format!("删除 knowledge_items 失败: {}", e))?;
        }

        // 清理仅属于此 source 的 wiki_pages 及其版本快照
        for (_kb_path, wp_path) in &page_disk_paths {
            let (page_id, wp_kb_id) = conn.query_row(
                "SELECT id, kb_id FROM wiki_pages WHERE path = ?1",
                rusqlite::params![wp_path],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            ).map_err(|e| format!("查询 wiki_page 失败: {}", e))?;

            conn.execute(
                "DELETE FROM versions WHERE kb_id = ?1 AND page_path = ?2",
                rusqlite::params![wp_kb_id, wp_path],
            ).map_err(|e| format!("删除 versions 失败: {}", e))?;
            conn.execute(
                "DELETE FROM wiki_pages WHERE id = ?1",
                rusqlite::params![page_id],
            ).map_err(|e| format!("删除 wiki_page 失败: {}", e))?;
        }

        // 清理引用此 source 的审阅项
        conn.execute(
            "DELETE FROM review_items WHERE source_id = ?1",
            rusqlite::params![source_id],
        ).map_err(|e| format!("清理 review_items 失败: {}", e))?;

        // 清理关联的任务
        conn.execute(
            "DELETE FROM tasks WHERE input_ref = ?1",
            rusqlite::params![source_id],
        ).map_err(|e| format!("清理 tasks 失败: {}", e))?;

        // 清理关联的 graph_nodes 和 graph_edges
        if let Some(ref kb) = source_kb_id {
            conn.execute(
                "DELETE FROM graph_edges WHERE kb_id = ?1 AND (source_node_id IN (SELECT id FROM graph_nodes WHERE kb_id = ?1 AND source_id = ?2) OR target_node_id IN (SELECT id FROM graph_nodes WHERE kb_id = ?1 AND source_id = ?2))",
                rusqlite::params![kb, source_id],
            ).map_err(|e| format!("清理 graph_edges 失败: {}", e))?;
            conn.execute(
                "DELETE FROM graph_nodes WHERE kb_id = ?1 AND source_id = ?2",
                rusqlite::params![kb, source_id],
            ).map_err(|e| format!("清理 graph_nodes 失败: {}", e))?;
        }

        conn.execute("DELETE FROM sources WHERE id = ?1", rusqlite::params![source_id])
            .map_err(|e| format!("删除 source 失败: {}", e))?;
        Ok(())
    })();

    match delete_result {
        Ok(()) => {
            conn.execute("COMMIT", [])
                .map_err(|e| format!("提交事务失败: {}", e))?;
        }
        Err(e) => {
            if let Err(e) = conn.execute("ROLLBACK", []) {
            log::error!("[stage_file] ROLLBACK failed: {}", e);
        }
            return Err(e);
        }
    }

    // 数据库已成功提交后，再清理磁盘文件
    if let Some(ref path) = file_path {
        let disk_path = std::path::PathBuf::from(path);
        if disk_path.exists() {
            if let Err(e) = std::fs::remove_file(&disk_path) {
                log::error!("[delete_source] 删除文件失败 ({}): {}", disk_path.display(), e);
            }
        }
    }

    // 清理 wiki 页面磁盘文件
    for (kb_path, wp_path) in &page_disk_paths {
        let wiki_dir = std::path::PathBuf::from(kb_path).join("wiki");
        let disk_path = wiki_dir.join(wp_path);
        if disk_path.exists() {
            if let Err(e) = std::fs::remove_file(&disk_path) {
                log::error!("[delete_source] 删除 wiki 页面文件失败 ({}): {}", disk_path.display(), e);
            }
        }
    }

    // 清理 source_preview 文件
    if let Some(ref kb) = source_kb_id {
        if let Ok(kb_path) = conn.query_row(
            "SELECT path FROM knowledge_bases WHERE id = ?1",
            rusqlite::params![kb],
            |row| row.get::<_, String>(0),
        ) {
            let preview_dir = std::path::PathBuf::from(&kb_path).join(".runtime").join("source_previews");
            if preview_dir.exists() {
                for suffix in &["md", "json"] {
                    let preview_file = preview_dir.join(format!("{}.{}", source_id, suffix));
                    if preview_file.exists() {
                        if let Err(e) = std::fs::remove_file(&preview_file) {
                            log::error!("[delete_source] 删除预览文件失败 ({}): {}", preview_file.display(), e);
                        }
                    }
                }
            }
        }
    }

    // 发出事件通知前端刷新
    if let Some(ref kb) = source_kb_id {
        kernel.event_bus.emit_source_updated(kb, &source_id);
        kernel.event_bus.emit_kb_stats_changed(kb);
    }

    Ok(())
}

#[tauri::command]
pub async fn reimport_source(
    kernel: State<'_, Arc<AppKernel>>,
    source_id: String,
) -> Result<String, String> {
    let conn = kernel.db.connect()?;
    let (kb_id, file_path): (String, String) = conn
        .query_row("SELECT kb_id, file_path FROM sources WHERE id = ?1", rusqlite::params![source_id], |row| Ok((row.get(0)?, row.get(1)?)))
        .map_err(|e| format!("获取 source 信息失败: {}", e))?;

    let kb_path: String = conn
        .query_row("SELECT path FROM knowledge_bases WHERE id = ?1", rusqlite::params![kb_id], |row| row.get(0))
        .map_err(|e| format!("获取知识库路径失败: {}", e))?;

    // 检查原始文件是否仍存在
    let src_path = std::path::PathBuf::from(&file_path);
    if !src_path.exists() {
        return Err(format!("原始文件已不存在: {}。请重新上传文件。", file_path));
    }

    conn.execute("UPDATE sources SET status = 'pending' WHERE id = ?1", rusqlite::params![source_id])
        .map_err(|e| format!("更新状态失败: {}", e))?;

    // 创建 coordinator 并启动任务
    let tq = Arc::new(crate::core::task_queue::TaskQueue::new(kernel.db.clone(), kernel.event_bus.clone()));
    let coordinator = crate::agents::coordinator::CoordinatorAgent::new(
        tq.clone(), kernel.db.clone(), kernel.config.clone(),
        kernel.secrets.clone(), kernel.workspace.clone(), kernel.event_bus.clone(),
        kernel.token_logger.clone(), kernel.vdb.clone(),
    );

    let task_id = coordinator.run_source_ingest(&kb_id, &kb_path, &source_id).await?;
    Ok(task_id)
}

#[tauri::command]
pub async fn get_source_summary(
    kernel: State<'_, Arc<AppKernel>>,
    source_id: String,
) -> Result<String, String> {
    let conn = kernel.db.connect()?;
    let text: String = conn
        .query_row("SELECT COALESCE(extracted_text, '') FROM sources WHERE id = ?1", rusqlite::params![source_id], |row| row.get(0))
        .map_err(|e| format!("获取文本失败: {}", e))?;

    if text.is_empty() {
        return Err("该文档尚未提取文本内容。".to_string());
    }

    Ok(text)
}

#[tauri::command]
pub async fn parse_document_text(
    _kernel: State<'_, Arc<AppKernel>>,
    file_path: String,
    file_type: String,
) -> Result<serde_json::Value, String> {
    let path = std::path::PathBuf::from(&file_path);
    let result = DocumentProcessor::parse_document(&path, &file_type)?;

    Ok(serde_json::json!({
        "text": result.text,
        "text_length": result.text_length,
        "page_count": result.page_count,
        "warnings": result.warnings,
    }))
}

#[tauri::command]
pub async fn get_supported_file_types() -> Result<Vec<serde_json::Value>, String> {
    let types = DocumentProcessor::get_supported_types();
    Ok(types.iter().map(|t| serde_json::json!({
        "extension": t.extension,
        "mime_type": t.mime_type,
        "description": t.description,
        "is_document": t.is_document,
    })).collect())
}

#[tauri::command]
pub async fn batch_import_sources(
    kernel: State<'_, Arc<AppKernel>>,
    kb_id: String,
    kb_path: String,
    file_paths: Vec<String>,
) -> Result<serde_json::Value, String> {
    let mut results = Vec::new();
    let mut success_count = 0;
    let mut fail_count = 0;

    for file_path in &file_paths {
        match upload_single_file(&kernel, &kb_id, &kb_path, file_path, None).await {
            Ok(result) => {
                success_count += 1;
                kernel.event_bus.emit_notification("info", "导入进度", &format!("{}/{} - {}", success_count + fail_count, file_paths.len(), file_path));
                results.push(serde_json::json!({
                    "file_path": file_path,
                    "status": "success",
                    "result": result,
                }));
            }
            Err(e) => {
                fail_count += 1;
                kernel.event_bus.emit_notification("info", "导入进度", &format!("{}/{} - {}", success_count + fail_count, file_paths.len(), file_path));
                results.push(serde_json::json!({
                    "file_path": file_path,
                    "status": "failed",
                    "error": e,
                }));
            }
        }
    }

    // 记录批量操作日志
    let wiki_dir = std::path::PathBuf::from(&kb_path).join("wiki");
    crate::wiki::log_service::LogService::log_batch_operation(
        &wiki_dir, "导入", success_count, if fail_count == 0 { "全部成功" } else { "部分失败" }
    )?;

    Ok(serde_json::json!({
        "total": file_paths.len(),
        "success": success_count,
        "failed": fail_count,
        "results": results,
    }))
}

/// 扫描外部文件夹，返回支持的文件列表（不导入）
/// 用于在导入前预览文件夹内容
#[tauri::command]
pub async fn scan_import_folder(
    folder_path: String,
) -> Result<serde_json::Value, String> {
    let folder = std::path::PathBuf::from(&folder_path);
    if !folder.exists() {
        return Err(format!("文件夹不存在: {}", folder_path));
    }
    if !folder.is_dir() {
        return Err("提供的路径不是一个文件夹".to_string());
    }

    let folder_name = folder
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    let mut total_files: u32 = 0;
    let mut supported_files: u32 = 0;
    let mut skipped_files: u32 = 0;
    let mut directory_count: u32 = 0;
    let mut total_size: u64 = 0;
    let mut files: Vec<serde_json::Value> = Vec::new();
    let mut skipped_items: Vec<serde_json::Value> = Vec::new();

    for entry in WalkDir::new(&folder_path).max_depth(20) {
        if total_files >= 10000 {
            break;
        }
        match entry {
            Ok(entry) => {
                let path = entry.path();

                if path.is_dir() {
                    if path != folder {
                        directory_count += 1;
                    }
                    continue;
                }

                total_files += 1;

                let file_name = path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();

                let relative_path = path
                    .strip_prefix(&folder)
                    .unwrap_or(path)
                    .to_string_lossy()
                    .to_string();

                let extension = DocumentProcessor::get_extension(&file_name);

                if DocumentProcessor::is_supported(&extension) {
                    let file_size = std::fs::metadata(path)
                        .map(|m| m.len())
                        .unwrap_or(0);
                    total_size += file_size;
                    supported_files += 1;
                    files.push(serde_json::json!({
                        "relative_path": relative_path,
                        "file_name": file_name,
                        "file_type": extension,
                        "file_size": file_size,
                        "is_supported": true,
                    }));
                } else {
                    skipped_files += 1;
                    skipped_items.push(serde_json::json!({
                        "relative_path": relative_path,
                        "file_name": file_name,
                        "reason": "unsupported_type",
                    }));
                }
            }
            Err(e) => {
                log::warn!("[scan_import_folder] 遍历文件失败: {}", e);
            }
        }
    }

    if supported_files == 0 {
        return Err("未在所选文件夹中找到支持的文档文件（支持：pdf, docx, md, txt, html, pptx, xlsx, csv, json, xml, png, jpg, jpeg, webp, gif）".to_string());
    }

    Ok(serde_json::json!({
        "folder_name": folder_name,
        "folder_path": folder_path,
        "total_files": total_files,
        "supported_files": supported_files,
        "skipped_files": skipped_files,
        "total_size": total_size,
        "directory_count": directory_count,
        "files": files,
        "skipped_items": skipped_items,
    }))
}

/// 导入整个文件夹中的所有支持文件
/// 逐个文件调用核心导入逻辑，并实时发送进度事件到前端
#[tauri::command]
pub async fn import_folder(
    app_handle: tauri::AppHandle,
    kernel: State<'_, Arc<AppKernel>>,
    kb_id: String,
    folder_path: String,
    preserve_structure: bool,
    selected_files: Option<Vec<String>>,
) -> Result<serde_json::Value, String> {
    if preserve_structure {
        log::info!("[import_folder] preserve_structure=true, 将保留文件夹子目录结构");
    }

    let conn = kernel.db.connect()?;
    let kb_path: String = conn
        .query_row(
            "SELECT path FROM knowledge_bases WHERE id = ?1",
            rusqlite::params![kb_id],
            |row| row.get(0),
        )
        .map_err(|e| format!("查询知识库路径失败: {}", e))?;
    drop(conn);

    let folder = std::path::PathBuf::from(&folder_path);
    if !folder.exists() {
        return Err(format!("文件夹不存在: {}", folder_path));
    }
    if !folder.is_dir() {
        return Err("提供的路径不是一个文件夹".to_string());
    }

    // Scan folder and collect supported file paths
    let mut supported_paths: Vec<(String, String)> = Vec::new(); // (absolute_path, relative_path)

    for entry in WalkDir::new(&folder_path).max_depth(20) {
        match entry {
            Ok(entry) => {
                if entry.file_type().is_dir() {
                    continue;
                }
                let path = entry.path();
                let file_name = path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();
                let extension = DocumentProcessor::get_extension(&file_name);

                if DocumentProcessor::is_supported(&extension) {
                    let relative_path = path
                        .strip_prefix(&folder)
                        .unwrap_or(path)
                        .to_string_lossy()
                        .to_string();
                    supported_paths.push((
                        path.to_string_lossy().to_string(),
                        relative_path,
                    ));
                }
            }
            Err(e) => {
                log::warn!("[import_folder] 遍历文件失败: {}", e);
            }
        }
    }

    if supported_paths.is_empty() {
        return Err("未在所选文件夹中找到支持的文档文件".to_string());
    }

    // Filter by selected files if provided
    if let Some(ref selected) = selected_files {
        if !selected.is_empty() {
            let selected_set: std::collections::HashSet<&str> = selected.iter().map(|s| s.as_str()).collect();
            supported_paths.retain(|(_, rel_path)| selected_set.contains(rel_path.as_str()));
        }
    }

    if supported_paths.is_empty() {
        return Err("所选文件均不在支持的文档类型范围内".to_string());
    }

    let total = supported_paths.len() as u32;
    let mut success_count: u32 = 0;
    let mut fail_count: u32 = 0;
    let mut results: Vec<serde_json::Value> = Vec::new();

    for (index, (file_path_str, relative_path)) in supported_paths.iter().enumerate() {
        let file_name = std::path::Path::new(file_path_str)
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        let current = (index + 1) as u32;
        let is_last = current == total;

        // Emit progress event before importing each file
        let payload = serde_json::json!({
            "current": current,
            "total": total,
            "file_name": file_name,
            "relative_path": relative_path,
            "status": if is_last { "complete" } else { "importing" },
            "success_count": success_count,
            "fail_count": fail_count,
        });
        let _ = app_handle.emit("folder-import-progress", &payload);

        let subdir = if preserve_structure {
            std::path::Path::new(&relative_path).parent().and_then(|p| {
                let s = p.to_string_lossy();
                if s.is_empty() { None } else { Some(s.to_string()) }
            })
        } else {
            None
        };
        match upload_single_file(&kernel, &kb_id, &kb_path, file_path_str, subdir.as_deref()).await {
            Ok(result) => {
                success_count += 1;
                results.push(serde_json::json!({
                    "file_path": file_path_str,
                    "relative_path": relative_path,
                    "status": "success",
                    "result": result,
                }));
            }
            Err(e) => {
                fail_count += 1;
                results.push(serde_json::json!({
                    "file_path": file_path_str,
                    "relative_path": relative_path,
                    "status": "failed",
                    "error": e,
                }));
            }
        }
    }

    // Log batch operation
    let wiki_dir = std::path::PathBuf::from(&kb_path).join("wiki");
    crate::wiki::log_service::LogService::log_batch_operation(
        &wiki_dir,
        "文件夹导入",
        success_count as usize,
        if fail_count == 0 { "全部成功" } else { "部分失败" },
    ).map_err(|e| format!("记录导入日志失败: {}", e))?;

    Ok(serde_json::json!({
        "total": total,
        "success": success_count,
        "failed": fail_count,
        "results": results,
    }))
}

async fn upload_single_file(
    kernel: &AppKernel,
    kb_id: &str,
    kb_path: &str,
    file_path: &str,
    relative_subdir: Option<&str>,
) -> Result<serde_json::Value, String> {
    let result = stage_file(kernel, kb_id, kb_path, file_path, relative_subdir).await?;

    let file_type = result["type"].as_str().unwrap_or("");
    let status = result["status"].as_str().unwrap_or("");

    if file_type == "asset" {
        let file_name = result["file_name"].as_str().unwrap_or("");
        kernel.event_bus.emit_notification("info", "资产已保存", &format!("{} 已保存为资产，不参与 AI 知识抽取", file_name));
        return Ok(serde_json::json!({
            "id": result["id"],
            "type": "asset",
            "file_name": file_name,
            "message": "图片仅保存为资产，不参与 AI 知识抽取",
        }));
    }

    if status == "duplicate" {
        return Ok(serde_json::json!({
            "id": result["id"],
            "status": "duplicate",
            "message": "文件已存在，跳过导入",
        }));
    }

    // Document — trigger AI pipeline
    let source_id = result["id"].as_str().unwrap_or("").to_string();

    let tq = Arc::new(crate::core::task_queue::TaskQueue::new(kernel.db.clone(), kernel.event_bus.clone()));
    let coordinator = crate::agents::coordinator::CoordinatorAgent::new(
        tq.clone(), kernel.db.clone(), kernel.config.clone(),
        kernel.secrets.clone(), kernel.workspace.clone(), kernel.event_bus.clone(),
        kernel.token_logger.clone(), kernel.vdb.clone(),
    );

    let _task_id = coordinator.run_source_ingest(kb_id, kb_path, &source_id).await?;

    let conn = kernel.db.connect()?;
    let extracted_text: Option<String> = match conn.query_row(
        "SELECT extracted_text FROM sources WHERE id = ?1",
        rusqlite::params![source_id],
        |row| row.get(0),
    ) {
        Ok(v) => v,
        Err(e) => {
            log::error!("[upload_single_file] 查询 extracted_text 失败 ({}): {}", source_id, e);
            None
        }
    };

    Ok(serde_json::json!({
        "id": source_id,
        "type": "document",
        "file_name": result["file_name"],
        "file_type": result["file_type"],
        "file_size": result["file_size"],
        "file_hash": result["file_hash"],
        "text_length": extracted_text.map(|t| t.len()).unwrap_or(0),
        "parse_warning": result["parse_warning"],
        "status": "imported",
    }))
}

/// Validate a citation target — checks that the source exists in DB and the physical file is present.
/// Used by the Citation Router when a user clicks a citation tag.
#[derive(Clone, serde::Serialize)]
pub struct CitationTargetInfo {
    pub valid: bool,
    pub file_name: String,
    pub file_path: String,
    pub file_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[tauri::command]
pub async fn validate_citation_target(
    kernel: State<'_, Arc<AppKernel>>,
    source_id: String,
) -> Result<CitationTargetInfo, String> {
    let conn = kernel.db.connect()?;

    let row = conn.query_row(
        "SELECT file_name, file_path, file_type FROM sources WHERE id = ?1",
        rusqlite::params![source_id],
        |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        },
    );

    match row {
        Ok((file_name, file_path, file_type)) => {
            let abs_path = std::path::PathBuf::from(&file_path);
            if abs_path.exists() {
                Ok(CitationTargetInfo {
                    valid: true,
                    file_name,
                    file_path,
                    file_type,
                    reason: None,
                })
            } else {
                Ok(CitationTargetInfo {
                    valid: false,
                    file_name,
                    file_path,
                    file_type,
                    reason: Some("SOURCE_FILE_MISSING".to_string()),
                })
            }
        }
        Err(rusqlite::Error::QueryReturnedNoRows) => {
            Ok(CitationTargetInfo {
                valid: false,
                file_name: String::new(),
                file_path: String::new(),
                file_type: String::new(),
                reason: Some("SOURCE_NOT_FOUND".to_string()),
            })
        }
        Err(e) => Err(format!("查询 source 失败: {}", e)),
    }
}
