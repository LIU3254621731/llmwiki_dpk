use std::sync::Arc;
use tauri::State;
use crate::core::app_kernel::AppKernel;
use crate::skills::web_search_skill::{WebSearchSkill, EngineConfig};

#[tauri::command]
pub async fn web_search(
    kernel: State<'_, Arc<AppKernel>>,
    query: String,
    engine: String,
    max_results: u32,
) -> Result<Vec<serde_json::Value>, String> {
    if query.trim().is_empty() {
        return Err("搜索关键词不能为空".to_string());
    }

    let max = max_results.clamp(1, 20);

    let ws_config = kernel.config.get_web_search_config()?;
    let engine_config = EngineConfig {
        engine: if engine.is_empty() { ws_config.engine } else { engine },
        max_results: max,
        searxng_url: ws_config.searxng_url,
        brave_api_key: ws_config.brave_api_key,
        bing_api_key: ws_config.bing_api_key,
        bing_endpoint: ws_config.bing_endpoint,
    };

    let results = WebSearchSkill::search(&engine_config, &query).await?;

    Ok(results
        .iter()
        .map(|r| {
            serde_json::json!({
                "title": r.title,
                "url": r.url,
                "snippet": r.snippet,
            })
        })
        .collect())
}

#[tauri::command]
pub async fn save_web_result_as_source(
    kernel: State<'_, Arc<AppKernel>>,
    kb_id: String,
    kb_path: String,
    title: String,
    content: String,
    format: String,
) -> Result<serde_json::Value, String> {
    if title.trim().is_empty() {
        return Err("标题不能为空".to_string());
    }
    if content.trim().is_empty() {
        return Err("内容不能为空".to_string());
    }

    let now = chrono::Utc::now();
    let now_str = now.to_rfc3339();
    let date_str = now.format("%Y%m%d_%H%M%S").to_string();

    let safe_title = title
        .chars()
        .take(40)
        .map(|c| if c.is_alphanumeric() || c == '_' || c == '-' { c } else { '_' })
        .collect::<String>();

    let extension = match format.as_str() {
        "pdf" => "pdf",
        _ => "md",
    };

    let file_name = format!("web_search_{}_{}.{}", safe_title, date_str, extension);
    let kb_path_buf = std::path::PathBuf::from(&kb_path);
    let dest_dir = kb_path_buf.join("raw").join("sources").join("documents");
    std::fs::create_dir_all(&dest_dir)
        .map_err(|e| format!("创建目录失败: {}", e))?;

    let dest_path = dest_dir.join(&file_name);

    let file_content = if extension == "md" {
        format!(
            "---\ntitle: {}\ntype: source\ncreated: {}\nsource: web_search\n---\n\n{}",
            title, now_str, content
        )
    } else {
        content.clone()
    };

    std::fs::write(&dest_path, &file_content)
        .map_err(|e| format!("写入文件失败: {}", e))?;

    let file_size = dest_path
        .metadata()
        .map(|m| m.len())
        .unwrap_or(0);

    let file_hash = crate::skills::document_processor::DocumentProcessor::compute_file_hash(&dest_path)
        .unwrap_or_default();

    let source_id = uuid::Uuid::new_v4().to_string();
    let conn = kernel.db.connect()?;

    let existing: i64 = match conn.query_row(
        "SELECT COUNT(1) FROM sources WHERE kb_id = ?1 AND file_hash = ?2",
        rusqlite::params![kb_id, file_hash],
        |row| row.get(0),
    ) {
        Ok(c) => c,
        Err(rusqlite::Error::QueryReturnedNoRows) => 0,
        Err(e) => return Err(format!("查询重复文件失败: {}", e)),
    };

    if existing > 0 {
        let _ = std::fs::remove_file(&dest_path);
        return Err("相同内容的搜索结果已存在，请勿重复保存。".to_string());
    }

    conn.execute(
        "INSERT INTO sources (id, kb_id, file_name, file_path, file_type, file_size, file_hash, extracted_text, status, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 'pending', ?9, ?9)",
        rusqlite::params![
            source_id, kb_id, file_name,
            dest_path.to_string_lossy(), extension,
            file_size as i64, file_hash,
            content, now_str,
        ],
    )
    .map_err(|e| format!("保存 source 记录失败: {}", e))?;

    kernel.event_bus.emit_source_updated(&kb_id, &source_id);
    kernel.event_bus.emit_kb_stats_changed(&kb_id);

    let tq = Arc::new(crate::core::task_queue::TaskQueue::new(
        kernel.db.clone(),
        kernel.event_bus.clone(),
    ));
    let coordinator = crate::agents::coordinator::CoordinatorAgent::new(
        tq.clone(),
        kernel.db.clone(),
        kernel.config.clone(),
        kernel.secrets.clone(),
        kernel.workspace.clone(),
        kernel.event_bus.clone(),
        kernel.token_logger.clone(), kernel.vdb.clone(),
    );

    let task_id = coordinator
        .run_source_ingest(&kb_id, &kb_path, &source_id)
        .await?;

    Ok(serde_json::json!({
        "source_id": source_id,
        "task_id": task_id,
        "file_name": file_name,
        "file_type": extension,
        "file_size": file_size,
        "created_at": now_str,
    }))
}

#[tauri::command]
pub async fn fetch_web_page_content(
    url: String,
) -> Result<serde_json::Value, String> {
    if url.trim().is_empty() {
        return Err("URL 不能为空".to_string());
    }

    let content = crate::skills::web_search_skill::WebSearchSkill::fetch_page_content(&url).await?;

    Ok(serde_json::json!({
        "title": content.title,
        "url": content.url,
        "content": content.content,
        "content_length": content.content_length,
    }))
}
