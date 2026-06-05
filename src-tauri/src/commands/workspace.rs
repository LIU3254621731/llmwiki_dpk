use std::sync::Arc;
use tauri::State;
use crate::core::app_kernel::AppKernel;

#[tauri::command]
pub async fn create_knowledge_base(
    kernel: State<'_, Arc<AppKernel>>,
    name: String,
    template_name: String,
    base_path: String,
) -> Result<serde_json::Value, String> {
    let name = name.trim().to_string();
    if name.is_empty() {
        return Err("知识库名称不能为空".to_string());
    }
    if name.len() > 100 {
        return Err("知识库名称不能超过100个字符".to_string());
    }
    if name.chars().any(|c| "/\\:*?\"<>|".contains(c)) {
        return Err("知识库名称包含非法字符".to_string());
    }
    let base = std::path::PathBuf::from(&base_path);
    if !base.exists() {
        std::fs::create_dir_all(&base)
            .map_err(|e| format!("创建基础路径失败 ({}): {}", base_path, e))?;
    }
    let kb_id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    let kb_path = std::path::PathBuf::from(&base_path).join(&name);
    let kb_dir_exists = kb_path.exists();
    std::fs::create_dir_all(&kb_path)
        .map_err(|e| format!("创建知识库目录失败: {}", e))?;

    // 若目录已存在且包含旧 workspace 结构，先清理旧数据（防止旧安装残留文件混入新 KB）
    if kb_dir_exists {
        let has_workspace_structure = kb_path.join("raw").exists()
            || kb_path.join("wiki").exists()
            || kb_path.join(".runtime").exists();
        if has_workspace_structure {
            log::info!("[create_kb] 目录已包含旧 workspace 结构，清理旧数据: {:?}", kb_path);
            kernel.workspace.clean_old_workspace_data(&kb_path);
        }
    }

    // 初始化 workspace 目录结构
    kernel.workspace.init_workspace(&kb_path)?;

    // 保存知识库配置
    kernel.config.save_kb_config(
        &kb_path,
        &crate::core::config_service::KbConfig {
            name: name.clone(),
            template_name: template_name.clone(),
            language: "zh-CN".to_string(),
            review_mode: "balanced".to_string(),
            allow_ai_generation: true,
        },
    )?;

    // 保存到数据库
    let conn = kernel.db.connect()?;
    conn.execute(
        "INSERT INTO knowledge_bases (id, name, path, template_name, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?5)",
        rusqlite::params![kb_id, name, kb_path.to_string_lossy(), template_name, now],
    )
    .map_err(|e| format!("保存知识库记录失败: {}", e))?;

    kernel.event_bus.emit_kb_stats_changed(&kb_id);

    Ok(serde_json::json!({
        "id": kb_id,
        "name": name,
        "path": kb_path.to_string_lossy(),
        "template_name": template_name,
        "created_at": now,
    }))
}

#[tauri::command]
pub async fn list_knowledge_bases(
    kernel: State<'_, Arc<AppKernel>>,
) -> Result<Vec<serde_json::Value>, String> {
    let conn = kernel.db.connect()?;
    let mut stmt = conn
        .prepare("SELECT id, name, path, template_name, created_at, updated_at FROM knowledge_bases ORDER BY created_at DESC")
        .map_err(|e| format!("查询知识库失败: {}", e))?;

    let kbs = stmt
        .query_map([], |row| {
            Ok(serde_json::json!({
                "id": row.get::<_, String>(0)?,
                "name": row.get::<_, String>(1)?,
                "path": row.get::<_, String>(2)?,
                "template_name": row.get::<_, String>(3)?,
                "created_at": row.get::<_, String>(4)?,
                "updated_at": row.get::<_, String>(5)?,
            }))
        })
        .map_err(|e| format!("映射知识库失败: {}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("收集知识库失败: {}", e))?;

    Ok(kbs)
}

#[tauri::command]
pub async fn get_kb_stats(
    kernel: State<'_, Arc<AppKernel>>,
    kb_id: String,
) -> Result<serde_json::Value, String> {
    let conn = kernel.db.connect()?;

    let page_count: i64 = count_safe(&conn, "SELECT COUNT(*) FROM wiki_pages WHERE kb_id = ?1", &kb_id);
    let source_count: i64 = count_safe(&conn, "SELECT COUNT(*) FROM sources WHERE kb_id = ?1", &kb_id);
    let review_count: i64 = count_safe(&conn, "SELECT COUNT(*) FROM review_items ri INNER JOIN reviews r ON ri.review_id = r.id WHERE r.kb_id = ?1 AND ri.status = 'pending'", &kb_id);
    let relationship_count: i64 = count_safe(&conn, "SELECT COUNT(*) FROM relationships WHERE kb_id = ?1", &kb_id);
    let broken_page_count: i64 = count_safe(&conn, "SELECT COUNT(*) FROM wiki_pages WHERE kb_id = ?1 AND content_hash = 'broken'", &kb_id);
    let failed_task_count: i64 = count_safe(&conn, "SELECT COUNT(*) FROM tasks WHERE kb_id = ?1 AND status IN ('failed', 'interrupted')", &kb_id);
    let knowledge_item_count: i64 = count_safe(&conn, "SELECT COUNT(*) FROM knowledge_items WHERE kb_id = ?1", &kb_id);
    let graph_node_count: i64 = count_safe(&conn, "SELECT COUNT(*) FROM graph_nodes WHERE kb_id = ?1", &kb_id);

    let kb_path: String = match conn.query_row(
        "SELECT path FROM knowledge_bases WHERE id = ?1", rusqlite::params![kb_id], |row| row.get(0),
    ) {
        Ok(p) => p,
        Err(rusqlite::Error::QueryReturnedNoRows) => String::new(),
        Err(e) => { log::error!("[workspace] 查询 kb_path 失败: {}", e); String::new() }
    };
    let (severe_issue_count, warning_issue_count) = if kb_path.is_empty() {
        (0usize, 0usize)
    } else {
        match crate::recovery::workspace_reconcile::WorkspaceReconcile::run(&kernel.db, &kb_id, &kb_path) {
            Ok(report) => (
                report.issues.iter().filter(|i| i.severity == "error").count(),
                report.issues.iter().filter(|i| i.severity != "error").count(),
            ),
            Err(_) => (1usize, 0usize),
        }
    };
    
    let (language, review_mode, allow_ai_generation) = if kb_path.is_empty() {
        (String::new(), String::new(), true)
    } else {
        match kernel.config.get_kb_config(&std::path::PathBuf::from(&kb_path)) {
            Ok(cfg) => (cfg.language, cfg.review_mode, cfg.allow_ai_generation),
            Err(_) => (String::new(), String::new(), true),
        }
    };

    let health_status = if severe_issue_count > 0 || broken_page_count > 0 || failed_task_count > 0 {
        "critical"
    } else if warning_issue_count > 0 {
        "warning"
    } else if review_count > 0 {
        "review"
    } else if graph_node_count == 0 && knowledge_item_count > 0 {
        "graph_unsynced"
    } else {
        "healthy"
    };

    Ok(serde_json::json!({
        "page_count": page_count,
        "source_count": source_count,
        "review_count": review_count,
        "relationship_count": relationship_count,
        "broken_page_count": broken_page_count,
        "failed_task_count": failed_task_count,
        "knowledge_item_count": knowledge_item_count,
        "graph_node_count": graph_node_count,
        "severe_issue_count": severe_issue_count,
        "warning_issue_count": warning_issue_count,
        "issue_count": severe_issue_count + warning_issue_count,
        "health_status": health_status,
        "language": language,
        "review_mode": review_mode,
        "allow_ai_generation": allow_ai_generation,
    }))
}

#[tauri::command]
pub async fn init_workspace_dirs(
    kernel: State<'_, Arc<AppKernel>>,
    kb_path: String,
) -> Result<(), String> {
    kernel.workspace.init_workspace(&std::path::PathBuf::from(&kb_path))
}

#[tauri::command]
pub async fn update_knowledge_base(
    kernel: State<'_, Arc<AppKernel>>,
    kb_id: String,
    name: String,
    template_name: String,
    language: Option<String>,
    review_mode: Option<String>,
    allow_ai_generation: Option<bool>,
) -> Result<serde_json::Value, String> {
    if name.trim().is_empty() {
        return Err("知识库名称不能为空".to_string());
    }

    let conn = kernel.db.connect()?;
    let now = chrono::Utc::now().to_rfc3339();

    // 获取旧路径
    let old_path: String = conn.query_row(
        "SELECT path FROM knowledge_bases WHERE id = ?1",
        rusqlite::params![kb_id],
        |row| row.get(0),
    ).map_err(|e| format!("查询知识库失败: {}", e))?;

    let old_path = std::path::PathBuf::from(&old_path);
    let new_path = old_path.parent().unwrap_or(&old_path).join(&name);

    // 读取现有配置
    let mut kb_config = kernel.config.get_kb_config(&old_path)?;
    kb_config.name = name.clone();
    kb_config.template_name = template_name.clone();
    if let Some(lang) = &language { kb_config.language = lang.clone(); }
    if let Some(mode) = &review_mode { kb_config.review_mode = mode.clone(); }
    if let Some(allow_gen) = allow_ai_generation { kb_config.allow_ai_generation = allow_gen; }

    // 若名称变更则重命名目录
    if old_path != new_path {
        std::fs::rename(&old_path, &new_path)
            .map_err(|e| format!("重命名知识库目录失败: {}", e))?;
        // 保存配置到新路径
        kernel.config.save_kb_config(&new_path, &kb_config)?;
    } else {
        kernel.config.save_kb_config(&old_path, &kb_config)?;
    }

    // 更新数据库
    conn.execute(
        "UPDATE knowledge_bases SET name = ?1, path = ?2, template_name = ?3, updated_at = ?4 WHERE id = ?5",
        rusqlite::params![name, new_path.to_string_lossy().to_string(), template_name, now, kb_id],
    ).map_err(|e| format!("更新知识库记录失败: {}", e))?;

    kernel.event_bus.emit_kb_stats_changed(&kb_id);

    Ok(serde_json::json!({
        "id": kb_id,
        "name": name,
        "path": new_path.to_string_lossy(),
        "template_name": template_name,
        "language": kb_config.language,
        "review_mode": kb_config.review_mode,
        "allow_ai_generation": kb_config.allow_ai_generation,
        "updated_at": now,
    }))
}

#[tauri::command]
pub async fn delete_knowledge_base(
    kernel: State<'_, Arc<AppKernel>>,
    kb_id: String,
) -> Result<(), String> {
    let conn = kernel.db.connect()?;

    // 先获取路径，在删记录前读取
    let kb_path: String = conn.query_row(
        "SELECT path FROM knowledge_bases WHERE id = ?1",
        rusqlite::params![kb_id],
        |row| row.get(0),
    ).map_err(|e| format!("查询知识库路径失败: {}", e))?;

    // 将所有 DELETE 包装在事务中，防止中途失败导致数据库不一致
    conn.execute("BEGIN TRANSACTION", [])
        .map_err(|e| format!("开始事务失败: {}", e))?;

    let delete_result = (|| -> Result<(), String> {
        conn.execute("DELETE FROM graph_edges WHERE kb_id = ?1", rusqlite::params![kb_id])
            .map_err(|e| format!("删除图谱边失败: {}", e))?;
        conn.execute("DELETE FROM graph_nodes WHERE kb_id = ?1", rusqlite::params![kb_id])
            .map_err(|e| format!("删除图谱节点失败: {}", e))?;
        conn.execute("DELETE FROM operations WHERE kb_id = ?1", rusqlite::params![kb_id])
            .map_err(|e| format!("删除操作记录失败: {}", e))?;
        conn.execute("DELETE FROM versions WHERE kb_id = ?1", rusqlite::params![kb_id])
            .map_err(|e| format!("删除版本快照失败: {}", e))?;
        conn.execute("DELETE FROM review_item_events WHERE review_item_id IN (SELECT id FROM review_items WHERE review_id IN (SELECT id FROM reviews WHERE kb_id = ?1))", rusqlite::params![kb_id])
            .map_err(|e| format!("删除审阅事件失败: {}", e))?;
        conn.execute("DELETE FROM review_items WHERE review_id IN (SELECT id FROM reviews WHERE kb_id = ?1)", rusqlite::params![kb_id])
            .map_err(|e| format!("删除审阅项失败: {}", e))?;
        conn.execute("DELETE FROM reviews WHERE kb_id = ?1", rusqlite::params![kb_id])
            .map_err(|e| format!("删除审阅失败: {}", e))?;
        conn.execute("DELETE FROM task_events WHERE task_id IN (SELECT id FROM tasks WHERE kb_id = ?1)", rusqlite::params![kb_id])
            .map_err(|e| format!("删除任务事件失败: {}", e))?;
        conn.execute("DELETE FROM tasks WHERE kb_id = ?1", rusqlite::params![kb_id])
            .map_err(|e| format!("删除任务失败: {}", e))?;
        conn.execute("DELETE FROM aliases WHERE item_id IN (SELECT id FROM knowledge_items WHERE kb_id = ?1)", rusqlite::params![kb_id])
            .map_err(|e| format!("删除别名失败: {}", e))?;
        conn.execute("DELETE FROM relationships WHERE kb_id = ?1", rusqlite::params![kb_id])
            .map_err(|e| format!("删除关系失败: {}", e))?;
        conn.execute("DELETE FROM knowledge_items WHERE kb_id = ?1", rusqlite::params![kb_id])
            .map_err(|e| format!("删除知识项失败: {}", e))?;
        conn.execute("DELETE FROM wiki_pages WHERE kb_id = ?1", rusqlite::params![kb_id])
            .map_err(|e| format!("删除页面记录失败: {}", e))?;
        conn.execute("DELETE FROM sources WHERE kb_id = ?1", rusqlite::params![kb_id])
            .map_err(|e| format!("删除来源失败: {}", e))?;
        conn.execute("DELETE FROM assets WHERE kb_id = ?1", rusqlite::params![kb_id])
            .map_err(|e| format!("删除资产记录失败: {}", e))?;
        conn.execute("DELETE FROM source_previews WHERE source_id IN (SELECT id FROM sources WHERE kb_id = ?1)", rusqlite::params![kb_id])
            .map_err(|e| format!("删除来源预览失败: {}", e))?;
        conn.execute("DELETE FROM file_index WHERE kb_id = ?1", rusqlite::params![kb_id])
            .map_err(|e| format!("删除文件索引失败: {}", e))?;
        conn.execute("DELETE FROM knowledge_bases WHERE id = ?1", rusqlite::params![kb_id])
            .map_err(|e| format!("删除知识库记录失败: {}", e))?;
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

    // 数据库已成功提交后，再清理磁盘上的工作区目录
    // 如果磁盘清理失败，仅记录错误而不回滚数据库
    let kb_dir = std::path::PathBuf::from(&kb_path);
    if kb_dir.exists() {
        if let Err(e) = std::fs::remove_dir_all(&kb_dir) {
            log::error!("[delete_knowledge_base] 删除工作区目录失败 ({}): {}", kb_dir.display(), e);
        }
    }

    // 发出事件通知前端刷新（KB 列表和 Dashboard 统计需要更新）
    kernel.event_bus.emit_kb_stats_changed(&kb_id);
    kernel.event_bus.emit_notification("info", "知识库已删除", &format!("知识库 {} 已成功删除", kb_id));

    Ok(())
}

/// 重置所有应用数据 — 删除 DB 中所有记录并清理工作区目录，用于清理残留数据
#[tauri::command]
pub async fn reset_all_data(
    kernel: State<'_, Arc<AppKernel>>,
) -> Result<String, String> {
    let conn = kernel.db.connect()?;

    // 先收集所有 KB 路径用于磁盘清理
    let mut kb_paths: Vec<String> = Vec::new();
    {
        let mut stmt = conn.prepare("SELECT path FROM knowledge_bases")
            .map_err(|e| format!("查询 KB 路径失败: {}", e))?;
        let rows = stmt.query_map([], |row| row.get(0))
            .map_err(|e| format!("映射 KB 路径失败: {}", e))?;
        for p in rows.flatten() { kb_paths.push(p); }
    }

    // 将所有 DELETE 包装在事务中
    conn.execute("BEGIN TRANSACTION", [])
        .map_err(|e| format!("开始事务失败: {}", e))?;

    // 安全: tables 列表为硬编码常量，不存在 SQL 注入风险
    // 注意: 顺序很重要 — review_item_events 必须在 review_items 之前
    //          source_previews 必须在 sources 之前
    let tables = [
        "review_item_events", "task_events", "review_items", "operations", "versions",
        "graph_edges", "graph_nodes", "aliases", "relationships",
        "knowledge_items", "wiki_pages", "source_previews", "sources", "assets",
        "file_index", "reviews", "tasks", "knowledge_bases",
    ];
    let delete_result = (|| -> Result<(), String> {
        for table in &tables {
            conn.execute(&format!("DELETE FROM {}", table), [])
                .map_err(|e| format!("清理表 {} 失败: {}", table, e))?;
        }
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

    // 数据库已成功提交后，再清理磁盘
    let mut cleaned = 0usize;
    for p in &kb_paths {
        let dir = std::path::PathBuf::from(p);
        if dir.exists()
            && std::fs::remove_dir_all(&dir).is_ok() { cleaned += 1; }
    }

    Ok(format!("已清理 {} 个知识库记录，删除 {} 个工作区目录", kb_paths.len(), cleaned))
}

fn count_safe(conn: &rusqlite::Connection, sql: &str, kb_id: &str) -> i64 {
    match conn.query_row(sql, rusqlite::params![kb_id], |row| row.get::<_, i64>(0)) {
        Ok(c) => c,
        Err(rusqlite::Error::QueryReturnedNoRows) => 0,
        Err(e) => {
            log::error!("[workspace] COUNT 查询失败: {}", e);
            0
        }
    }
}
