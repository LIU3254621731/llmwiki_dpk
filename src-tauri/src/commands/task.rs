use std::sync::Arc;
use tauri::State;
use crate::core::app_kernel::AppKernel;
use crate::wiki::path_service::PathService;

#[tauri::command]
pub async fn list_tasks(
    kernel: State<'_, Arc<AppKernel>>,
    kb_id: String,
) -> Result<Vec<serde_json::Value>, String> {
    let tq = crate::core::task_queue::TaskQueue::new(kernel.db.clone(), kernel.event_bus.clone());
    let tasks = tq.list_tasks(&kb_id)?;

    Ok(tasks.iter().map(|t| serde_json::json!({
        "id": t.id,
        "kb_id": t.kb_id,
        "task_type": t.task_type,
        "status": t.status,
        "current_agent": t.current_agent,
        "error_message": t.error_message,
        "failure_reason": t.failure_reason,
        "recoverable": t.recoverable,
        "resume_from_stage": t.resume_from_stage,
        "last_success_stage": t.last_success_stage,
        "next_action": t.next_action,
        "retry_count": t.retry_count,
        "cancel_reason": t.cancel_reason,
        "created_at": t.created_at,
        "updated_at": t.updated_at,
        "completed_at": t.completed_at,
        "archived_at": t.archived_at,
        "handled_at": t.handled_at,
    })).collect())
}

#[tauri::command]
pub async fn list_tasks_filtered(
    kernel: State<'_, Arc<AppKernel>>,
    kb_id: String,
    status_filter: String,
) -> Result<Vec<serde_json::Value>, String> {
    let tq = crate::core::task_queue::TaskQueue::new(kernel.db.clone(), kernel.event_bus.clone());
    let tasks = tq.list_tasks_filtered(&kb_id, &status_filter)?;

    Ok(tasks.iter().map(|t| serde_json::json!({
        "id": t.id,
        "kb_id": t.kb_id,
        "task_type": t.task_type,
        "status": t.status,
        "current_agent": t.current_agent,
        "error_message": t.error_message,
        "failure_reason": t.failure_reason,
        "recoverable": t.recoverable,
        "resume_from_stage": t.resume_from_stage,
        "last_success_stage": t.last_success_stage,
        "next_action": t.next_action,
        "retry_count": t.retry_count,
        "cancel_reason": t.cancel_reason,
        "created_at": t.created_at,
        "updated_at": t.updated_at,
        "completed_at": t.completed_at,
        "archived_at": t.archived_at,
        "handled_at": t.handled_at,
    })).collect())
}

#[tauri::command]
pub async fn get_task_detail(
    kernel: State<'_, Arc<AppKernel>>,
    task_id: String,
) -> Result<serde_json::Value, String> {
    let tq = crate::core::task_queue::TaskQueue::new(kernel.db.clone(), kernel.event_bus.clone());
    let task = tq.get_task(&task_id)?;

    // 获取关联的 source 元数据
    let source_meta = if !task.input_ref.is_empty() {
        let conn = kernel.db.connect()?;
        match conn.query_row(
            "SELECT file_name, file_type, file_size, file_hash,
                    COALESCE(text_length, LENGTH(COALESCE(extracted_text, ''))),
                    page_count
             FROM sources WHERE id = ?1",
            rusqlite::params![task.input_ref],
            |row| {
                Ok(serde_json::json!({
                    "file_name": row.get::<_, String>(0)?,
                    "file_type": row.get::<_, String>(1)?,
                    "file_size": row.get::<_, i64>(2)?,
                    "file_hash": row.get::<_, String>(3)?,
                    "text_length": row.get::<_, i64>(4)?,
                    "page_count": row.get::<_, Option<i64>>(5)?,
                }))
            },
        ) {
            Ok(v) => Some(v),
            Err(_) => None,
        }
    } else {
        None
    };

    Ok(serde_json::json!({
        "id": task.id,
        "kb_id": task.kb_id,
        "task_type": task.task_type,
        "task_name": task.task_name,
        "status": task.status,
        "current_agent": task.current_agent,
        "input_ref": task.input_ref,
        "output_ref": task.output_ref,
        "review_id": task.review_id,
        "error_message": task.error_message,
        "failure_reason": task.failure_reason,
        "recoverable": task.recoverable,
        "resume_from_stage": task.resume_from_stage,
        "last_success_stage": task.last_success_stage,
        "next_action": task.next_action,
        "retry_count": task.retry_count,
        "cancel_reason": task.cancel_reason,
        "created_at": task.created_at,
        "updated_at": task.updated_at,
        "completed_at": task.completed_at,
        "archived_at": task.archived_at,
        "handled_at": task.handled_at,
        "locked_at": task.locked_at,
        "source_meta": source_meta,
        "review_id": task.review_id,
    }))
}

#[tauri::command]
pub async fn get_task_events(
    kernel: State<'_, Arc<AppKernel>>,
    task_id: String,
) -> Result<Vec<serde_json::Value>, String> {
    let tq = crate::core::task_queue::TaskQueue::new(kernel.db.clone(), kernel.event_bus.clone());
    let events = tq.get_task_events(&task_id)?;

    Ok(events.iter().map(|e| serde_json::json!({
        "id": e.id,
        "task_id": e.task_id,
        "event_type": e.event_type,
        "agent_name": e.agent_name,
        "message": e.message,
        "created_at": e.created_at,
    })).collect())
}

/// 获取任务关联的所有审阅项（供任务详情页 DiffReviewPanel 使用）
#[tauri::command]
pub async fn get_task_review_items(
    kernel: State<'_, Arc<AppKernel>>,
    task_id: String,
) -> Result<Vec<serde_json::Value>, String> {
    let conn = kernel.db.connect()?;
    let mut stmt = conn
        .prepare(
            "SELECT ri.id, ri.operation, ri.target_path, ri.old_content, ri.new_content,
                    ri.status, ri.risk_level, ri.title, ri.summary, ri.page_type, ri.created_at
             FROM review_items ri
             JOIN reviews r ON ri.review_id = r.id
             WHERE r.task_id = ?1
             ORDER BY ri.created_at ASC",
        )
        .map_err(|e| format!("查询审阅项失败: {}", e))?;

    let items = stmt
        .query_map(rusqlite::params![task_id], |row| {
            Ok(serde_json::json!({
                "id": row.get::<_, String>(0)?,
                "operation": row.get::<_, String>(1)?,
                "target_path": row.get::<_, String>(2)?,
                "old_content": row.get::<_, String>(3)?,
                "new_content": row.get::<_, String>(4)?,
                "status": row.get::<_, String>(5)?,
                "risk_level": row.get::<_, String>(6)?,
                "title": row.get::<_, String>(7)?,
                "summary": row.get::<_, String>(8)?,
                "page_type": row.get::<_, String>(9)?,
                "created_at": row.get::<_, String>(10)?,
            }))
        })
        .map_err(|e| format!("映射审阅项失败: {}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("收集审阅项失败: {}", e))?;

    Ok(items)
}

#[tauri::command]
pub async fn retry_task(
    kernel: State<'_, Arc<AppKernel>>,
    task_id: String,
) -> Result<(), String> {
    let tq = crate::core::task_queue::TaskQueue::new(kernel.db.clone(), kernel.event_bus.clone());
    tq.retry_task(&task_id)
}

#[tauri::command]
pub async fn cancel_task(
    kernel: State<'_, Arc<AppKernel>>,
    task_id: String,
) -> Result<(), String> {
    let tq = crate::core::task_queue::TaskQueue::new(kernel.db.clone(), kernel.event_bus.clone());
    tq.cancel_task(&task_id)
}

#[tauri::command]
pub async fn resume_task(
    kernel: State<'_, Arc<AppKernel>>,
    task_id: String,
) -> Result<(), String> {
    let tq = crate::core::task_queue::TaskQueue::new(kernel.db.clone(), kernel.event_bus.clone());
    tq.resume_task(&task_id)
}

#[tauri::command]
pub async fn run_source_ingest(
    kernel: State<'_, Arc<AppKernel>>,
    kb_id: String,
    source_id: String,
) -> Result<String, String> {
    let conn = kernel.db.connect()?;
    let kb_path: String = conn
        .query_row("SELECT path FROM knowledge_bases WHERE id = ?1", rusqlite::params![kb_id], |row| row.get(0))
        .map_err(|e| format!("获取知识库路径失败: {}", e))?;

    let tq = Arc::new(crate::core::task_queue::TaskQueue::new(kernel.db.clone(), kernel.event_bus.clone()));
    let coordinator = crate::agents::coordinator::CoordinatorAgent::new(
        tq, kernel.db.clone(), kernel.config.clone(),
        kernel.secrets.clone(), kernel.workspace.clone(), kernel.event_bus.clone(),
        kernel.token_logger.clone(), kernel.vdb.clone(),
    );

    coordinator.run_source_ingest(&kb_id, &kb_path, &source_id).await
}

#[tauri::command]
pub async fn run_resolution(
    kernel: State<'_, Arc<AppKernel>>,
    kb_id: String,
    task_id: String,
    ingest_result_json: String,
) -> Result<(), String> {
    let tq = Arc::new(crate::core::task_queue::TaskQueue::new(kernel.db.clone(), kernel.event_bus.clone()));
    let coordinator = crate::agents::coordinator::CoordinatorAgent::new(
        tq, kernel.db.clone(), kernel.config.clone(),
        kernel.secrets.clone(), kernel.workspace.clone(), kernel.event_bus.clone(),
        kernel.token_logger.clone(), kernel.vdb.clone(),
    );

    coordinator.run_resolution(&kb_id, &task_id, &ingest_result_json).await
}

#[tauri::command]
pub async fn run_relationship(
    kernel: State<'_, Arc<AppKernel>>,
    kb_id: String,
    task_id: String,
    resolution_json: String,
) -> Result<(), String> {
    let tq = Arc::new(crate::core::task_queue::TaskQueue::new(kernel.db.clone(), kernel.event_bus.clone()));
    let coordinator = crate::agents::coordinator::CoordinatorAgent::new(
        tq, kernel.db.clone(), kernel.config.clone(),
        kernel.secrets.clone(), kernel.workspace.clone(), kernel.event_bus.clone(),
        kernel.token_logger.clone(), kernel.vdb.clone(),
    );

    coordinator.run_relationship(&kb_id, &task_id, &resolution_json).await
}

#[tauri::command]
pub async fn run_wiki_update(
    kernel: State<'_, Arc<AppKernel>>,
    kb_id: String,
    task_id: String,
    resolution_json: String,
    relationship_json: String,
) -> Result<(), String> {
    let tq = Arc::new(crate::core::task_queue::TaskQueue::new(kernel.db.clone(), kernel.event_bus.clone()));
    let coordinator = crate::agents::coordinator::CoordinatorAgent::new(
        tq, kernel.db.clone(), kernel.config.clone(),
        kernel.secrets.clone(), kernel.workspace.clone(), kernel.event_bus.clone(),
        kernel.token_logger.clone(), kernel.vdb.clone(),
    );

    coordinator.run_wiki_update(&kb_id, &task_id, &resolution_json, &relationship_json).await
}

#[tauri::command]
pub async fn run_query(
    kernel: State<'_, Arc<AppKernel>>,
    kb_id: String,
    question: String,
    scope: String,
) -> Result<String, String> {
    let tq = Arc::new(crate::core::task_queue::TaskQueue::new(kernel.db.clone(), kernel.event_bus.clone()));
    let coordinator = crate::agents::coordinator::CoordinatorAgent::new(
        tq, kernel.db.clone(), kernel.config.clone(),
        kernel.secrets.clone(), kernel.workspace.clone(), kernel.event_bus.clone(),
        kernel.token_logger.clone(), kernel.vdb.clone(),
    );

    coordinator.run_query(&kb_id, &question, &scope).await
}

#[tauri::command]
pub async fn save_answer_as_wiki(
    kernel: State<'_, Arc<AppKernel>>,
    kb_id: String,
    kb_path: String,
    title: String,
    content: String,
) -> Result<(), String> {
    let title = title.trim().to_string();
    if title.is_empty() || title.len() > 200 {
        return Err("标题不能为空且不能超过 200 个字符".to_string());
    }

    let wiki_dir = kernel.workspace.get_wiki_dir(&std::path::PathBuf::from(&kb_path));
    let writer = crate::wiki::wiki_writer::WikiWriter::new(kernel.db.clone());
    let conn = kernel.db.connect()?;

    // 检查是否已有同名页面（按 canonical_name 去重）
    let canonical_name = crate::wiki::wiki_writer::WikiWriter::generate_canonical_name(&title);
    let safe_canonical = crate::wiki::path_service::PathService::generate_safe_name(&canonical_name);
    let existing_path: Option<String> = match conn.query_row(
        "SELECT path FROM wiki_pages WHERE kb_id = ?1 AND canonical_name = ?2 LIMIT 1",
        rusqlite::params![kb_id, safe_canonical],
        |row| row.get(0),
    ) {
        Ok(p) => Some(p),
        Err(rusqlite::Error::QueryReturnedNoRows) => None,
        Err(e) => return Err(format!("查询已有页面失败: {}", e)),
    };

    if let Some(ref path) = existing_path {
        writer.update_page(&kb_id, &wiki_dir, path, &content, "qa")?;
        crate::wiki::log_service::LogService::append_log(
            &wiki_dir, "问答更新", &format!("问答已更新 Wiki 页面: {} ({})", title, path),
        )?;
    } else {
        let _page_id = writer.create_page(
            &kb_id, &wiki_dir, "question", &title, &title, &content, "qa",
        )?;
        crate::wiki::log_service::LogService::append_log(
            &wiki_dir, "问答保存", &format!("问答已保存为 Wiki 页面: {}", title),
        )?;
    }

    crate::wiki::index_service::IndexService::new(kernel.db.clone())
        .append_to_index(&wiki_dir, &title, &format!("questions/{}.md", safe_canonical), "question")?;

    kernel.event_bus.emit_wiki_updated(&kb_id, "");
    kernel.event_bus.emit_kb_stats_changed(&kb_id);
    Ok(())
}

#[tauri::command]
pub async fn generate_mindmap(
    kernel: State<'_, Arc<AppKernel>>,
    kb_id: String,
    topic: String,
    context_pages: Option<String>,
) -> Result<crate::agents::mindmap::MindmapTreeNode, String> {
    let gateway = Arc::new(
        crate::model::model_gateway::ModelGateway::new(kernel.secrets.clone())
    );
    crate::agents::mindmap::MindmapAgent::generate(
        &kernel.db,
        &kernel.workspace,
        &kernel.config,
        &gateway,
        &kb_id,
        &topic,
        context_pages.as_deref(),
    ).await
}

#[tauri::command]
pub async fn get_health_snapshot(
    kernel: State<'_, Arc<AppKernel>>,
) -> Result<Option<crate::core::health_monitor::HealthSnapshot>, String> {
    Ok(kernel.health_monitor.get_latest_snapshot())
}

#[tauri::command]
pub async fn run_health_check(
    kernel: State<'_, Arc<AppKernel>>,
    kb_id: String,
) -> Result<String, String> {
    let tq = Arc::new(crate::core::task_queue::TaskQueue::new(kernel.db.clone(), kernel.event_bus.clone()));
    let coordinator = crate::agents::coordinator::CoordinatorAgent::new(
        tq, kernel.db.clone(), kernel.config.clone(),
        kernel.secrets.clone(), kernel.workspace.clone(), kernel.event_bus.clone(),
        kernel.token_logger.clone(), kernel.vdb.clone(),
    );

    coordinator.run_health_check(&kb_id).await
}

#[tauri::command]
pub async fn run_health_check_structured(
    kernel: State<'_, Arc<AppKernel>>,
    kb_id: String,
    kb_path: String,
) -> Result<crate::agents::health_check::HealthCheckResult, String> {
    crate::agents::health_check::HealthCheckAgent::run_structured(
        &kernel.db, &kb_id, &kb_path, &kernel.workspace,
    )
}

/// 运行 LinkSanitizerAgent：检测死链 → 创建占位页 → AI 补全提案
#[tauri::command]
pub async fn run_link_sanitize(
    kernel: State<'_, Arc<AppKernel>>,
    kb_id: String,
    kb_path: String,
) -> Result<crate::agents::link_sanitizer::SanitizeResult, String> {
    let _ = kernel.event_bus.emit_agent_activity(
        "LinkSanitizer", "scanning", &kb_id, "开始扫描死链...",
    );

    let result = crate::agents::link_sanitizer::LinkSanitizerAgent::run(
        &kernel.db,
        &kernel.vdb,
        &kernel.config,
        &kernel.secrets,
        &kernel.event_bus,
        &kb_id,
        &kb_path,
    ).await?;

    let _ = kernel.event_bus.emit_agent_activity(
        "LinkSanitizer", "idle", &kb_id,
        &format!("完成: {} 个死链, {} 个占位页, {} 个AI补全",
            result.broken_links_found, result.placeholders_created, result.ai_completions_proposed),
    );

    Ok(result)
}

/// 查询 link_sanitizer_log
#[tauri::command]
pub async fn get_sanitize_log(
    kernel: State<'_, Arc<AppKernel>>,
    kb_id: String,
) -> Result<Vec<serde_json::Value>, String> {
    let conn = kernel.db.connect()?;
    let mut stmt = conn
        .prepare("SELECT id, link_text, link_type, source_page_path, action, placeholder_path, review_item_id, vdb_max_similarity, details, created_at FROM link_sanitizer_log WHERE kb_id = ?1 ORDER BY created_at DESC LIMIT 100")
        .map_err(|e| format!("查询 link_sanitizer_log 失败: {}", e))?;

    let rows: Vec<serde_json::Value> = stmt
        .query_map(rusqlite::params![kb_id], |row| {
            Ok(serde_json::json!({
                "id": row.get::<_, String>(0)?,
                "link_text": row.get::<_, String>(1)?,
                "link_type": row.get::<_, String>(2)?,
                "source_page_path": row.get::<_, String>(3)?,
                "action": row.get::<_, String>(4)?,
                "placeholder_path": row.get::<_, String>(5)?,
                "review_item_id": row.get::<_, String>(6)?,
                "vdb_max_similarity": row.get::<_, f64>(7)?,
                "details": row.get::<_, String>(8)?,
                "created_at": row.get::<_, String>(9)?,
            }))
        })
        .map_err(|e| format!("映射 link_sanitizer_log 失败: {}", e))?
        .filter_map(|r| r.ok())
        .collect();

    Ok(rows)
}

#[tauri::command]
pub async fn run_reconcile(
    kernel: State<'_, Arc<AppKernel>>,
    kb_id: String,
    kb_path: String,
) -> Result<serde_json::Value, String> {
    let report = crate::recovery::workspace_reconcile::WorkspaceReconcile::run(
        &kernel.db, &kb_id, &kb_path,
    )?;

    Ok(serde_json::json!({
        "issues": report.issues.iter().map(|i| serde_json::json!({
            "severity": i.severity,
            "category": i.category,
            "description": i.description,
            "suggestion": i.suggestion,
            "fixable": i.fixable,
            "detail": i.detail,
        })).collect::<Vec<_>>(),
        "ok_items": report.ok_items,
        "fixed_count": report.fixed_count,
    }))
}

#[tauri::command]
pub async fn run_recovery_check(
    kernel: State<'_, Arc<AppKernel>>,
    kb_id: String,
) -> Result<Vec<String>, String> {
    let tq = Arc::new(crate::core::task_queue::TaskQueue::new(kernel.db.clone(), kernel.event_bus.clone()));
    let kb_path = kernel.db.connect()?
        .query_row("SELECT path FROM knowledge_bases WHERE id = ?1", rusqlite::params![kb_id], |row| row.get::<_, String>(0))
        .unwrap_or_default();
    crate::recovery::recovery_check::RecoveryCheck::run(&kernel.db, &tq, &kb_id, &kb_path)
}

#[tauri::command]
pub async fn get_interrupted_tasks(
    kernel: State<'_, Arc<AppKernel>>,
    kb_id: String,
) -> Result<Vec<serde_json::Value>, String> {
    let tq = crate::core::task_queue::TaskQueue::new(kernel.db.clone(), kernel.event_bus.clone());
    let tasks = tq.get_interrupted_tasks(&kb_id)?;

    Ok(tasks.iter().map(|t| serde_json::json!({
        "id": t.id,
        "task_type": t.task_type,
        "status": t.status,
        "error_message": t.error_message,
        "created_at": t.created_at,
    })).collect())
}

#[tauri::command]
pub async fn get_task_files(
    kernel: State<'_, Arc<AppKernel>>,
    kb_id: String,
    task_id: String,
) -> Result<serde_json::Value, String> {
    let conn = kernel.db.connect()?;
    let kb_path: String = conn
        .query_row("SELECT path FROM knowledge_bases WHERE id = ?1", rusqlite::params![kb_id], |row| row.get(0))
        .map_err(|e| format!("获取知识库路径失败: {}", e))?;

    let kb_path_buf = std::path::PathBuf::from(&kb_path);
    let tasks_dir = kernel.workspace.get_tasks_dir(&kb_path_buf);
    let task_dir = tasks_dir.join(&task_id);

    if !task_dir.exists() {
        return Ok(serde_json::json!({"error": "任务目录不存在", "path": task_dir.to_string_lossy()}));
    }

    let read_file = |name: &str| -> String {
        let path = task_dir.join(name);
        if path.exists() {
            match std::fs::read_to_string(&path) {
                Ok(c) => c,
                Err(e) => {
                    log::error!("[get_task_files] 读取任务文件失败 ({}): {}", name, e);
                    String::new()
                }
            }
        } else {
            String::new()
        }
    };

    let find_files = |pattern: &str| -> Vec<String> {
        let mut results = Vec::new();
        if let Ok(entries) = std::fs::read_dir(&task_dir) {
            for entry in entries.filter_map(|e| e.ok()) {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.starts_with(pattern.trim_end_matches('*')) {
                    results.push(name);
                }
            }
        }
        results.sort();
        results
    };

    let mut prompts = serde_json::json!({});
    for file in find_files("prompt_") {
        prompts[&file] = serde_json::json!(read_file(&file));
    }
    if prompts.as_object().unwrap_or(&serde_json::Map::new()).is_empty() {
        prompts = serde_json::json!({"prompt.md": read_file("prompt.md")});
    }

    let mut model_responses = serde_json::json!({});
    for file in find_files("model_raw_response_") {
        model_responses[&file] = serde_json::json!(read_file(&file));
    }
    if model_responses.as_object().unwrap_or(&serde_json::Map::new()).is_empty() {
        model_responses = serde_json::json!({"model_raw_response.txt": read_file("model_raw_response.txt")});
    }

    Ok(serde_json::json!({
        "task_dir": task_dir.to_string_lossy(),
        "files": find_files("*"),
        "ingest_result": read_file("ingest_result.json"),
        "resolution_result": read_file("resolution_result.json"),
        "relationship_result": read_file("relationship_result.json"),
        "update_plan": read_file("update_plan.json"),
        "prompts": prompts,
        "model_responses": model_responses,
        "extracted_text": read_file("extracted_text.txt"),
    }))
}

// 读取任务目录中指定文件的内容（用于中间文件预览）
#[tauri::command]
pub async fn read_task_file(
    kernel: State<'_, Arc<AppKernel>>,
    kb_id: String,
    task_id: String,
    file_name: String,
) -> Result<serde_json::Value, String> {
    let conn = kernel.db.connect()?;
    let kb_path: String = conn
        .query_row("SELECT path FROM knowledge_bases WHERE id = ?1", rusqlite::params![kb_id], |row| row.get(0))
        .map_err(|e| format!("获取知识库路径失败: {}", e))?;

    let kb_path_buf = std::path::PathBuf::from(&kb_path);
    let tasks_dir = kernel.workspace.get_tasks_dir(&kb_path_buf);
    let task_dir = tasks_dir.join(&task_id);
    let file_path = task_dir.join(&file_name);

    // Prevent directory traversal
    if !file_path.starts_with(&task_dir) {
        return Err("非法文件路径".to_string());
    }

    if !file_path.exists() {
        return Err(format!("文件不存在: {}", file_name));
    }

    let content = std::fs::read_to_string(&file_path)
        .map_err(|e| format!("读取文件失败: {}", e))?;

    let size = file_path.metadata().map(|m| m.len()).unwrap_or(0);

    Ok(serde_json::json!({
        "name": file_name,
        "content": content,
        "size": size,
    }))
}

// ====== v0.1.3 新增：健康检查修复命令 ======

#[tauri::command]
pub async fn recover_page_from_snapshot(
    kernel: State<'_, Arc<AppKernel>>,
    kb_id: String,
    kb_path: String,
    page_path: String,
) -> Result<String, String> {
    crate::recovery::workspace_reconcile::WorkspaceReconcile::recover_from_snapshot(
        &kernel.db, &kb_id, &kb_path, &page_path,
    )?;
    Ok(format!("页面已从快照恢复: {}", PathService::normalize(&page_path)))
}

#[tauri::command]
pub async fn repair_all_wiki_paths(
    kernel: State<'_, Arc<AppKernel>>,
    kb_id: String,
    kb_path: String,
) -> Result<serde_json::Value, String> {
    let fixed = crate::recovery::workspace_reconcile::WorkspaceReconcile::repair_all(
        &kernel.db, &kb_id, &kb_path,
    )?;
    let after = crate::recovery::workspace_reconcile::WorkspaceReconcile::run(
        &kernel.db, &kb_id, &kb_path,
    )?;
    let remaining_total = after.issues.len();
    let remaining_manual = after.issues.iter().filter(|i| !i.fixable).count();
    kernel.event_bus.emit_wiki_updated(&kb_id, "");
    Ok(serde_json::json!({
        "fixed": fixed,
        "remaining_total": remaining_total,
        "remaining_manual": remaining_manual,
    }))
}

#[tauri::command]
pub async fn sync_wiki_index_from_markdown(
    kernel: State<'_, Arc<AppKernel>>,
    kb_id: String,
    kb_path: String,
) -> Result<serde_json::Value, String> {
    let report = crate::wiki::markdown_indexer::MarkdownIndexer::sync_workspace(
        &kernel.db, &kb_id, &kb_path,
    )?;
    kernel.event_bus.emit_wiki_updated(&kb_id, "");
    Ok(serde_json::json!({
        "total_scanned": report.total_scanned,
        "synced": report.synced,
        "created": report.created,
        "updated": report.updated,
        "skipped": report.skipped,
        "skipped_system": report.skipped_system,
        "skipped_invalid": report.skipped_invalid,
        "skipped_errors": report.skipped_errors,
        "warnings": report.warnings,
        "skip_reasons": report.skip_reasons,
    }))
}

#[tauri::command]
pub async fn mark_page_broken(
    kernel: State<'_, Arc<AppKernel>>,
    kb_id: String,
    page_path: String,
) -> Result<(), String> {
    crate::recovery::workspace_reconcile::WorkspaceReconcile::mark_broken(
        &kernel.db, &kb_id, &page_path,
    )
}

#[tauri::command]
pub async fn delete_broken_page_record(
    kernel: State<'_, Arc<AppKernel>>,
    kb_id: String,
    page_id: String,
) -> Result<(), String> {
    crate::recovery::workspace_reconcile::WorkspaceReconcile::delete_broken_record(
        &kernel.db, &kb_id, &page_id,
    )?;

    kernel.event_bus.emit_wiki_updated(&kb_id, "");
    kernel.event_bus.emit_kb_stats_changed(&kb_id);
    Ok(())
}

#[tauri::command]
pub async fn archive_task(
    kernel: State<'_, Arc<AppKernel>>,
    task_id: String,
) -> Result<(), String> {
    let tq = crate::core::task_queue::TaskQueue::new(kernel.db.clone(), kernel.event_bus.clone());
    tq.archive_task(&task_id)
}

#[tauri::command]
pub async fn handle_failed_task(
    kernel: State<'_, Arc<AppKernel>>,
    task_id: String,
) -> Result<(), String> {
    let tq = crate::core::task_queue::TaskQueue::new(kernel.db.clone(), kernel.event_bus.clone());
    tq.handle_failed_task(&task_id)
}

#[tauri::command]
pub async fn get_unhandled_failed_count(
    kernel: State<'_, Arc<AppKernel>>,
    kb_id: String,
) -> Result<i64, String> {
    let tq = crate::core::task_queue::TaskQueue::new(kernel.db.clone(), kernel.event_bus.clone());
    tq.get_unhandled_failed_count(&kb_id)
}
