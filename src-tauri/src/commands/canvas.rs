use std::sync::Arc;
use tauri::{Emitter, State};
use crate::core::app_kernel::AppKernel;
use crate::core::config_service::ProviderConfig;
use crate::canvas::canvas_service;
use crate::schema::json_repair;
use crate::canvas::canvas_prompts::CanvasPrompts;

fn resolve_config(kernel: &AppKernel) -> Result<(ProviderConfig, String), String> {
    let config = kernel.config.get_provider_config()?;
    let api_key = kernel
        .secrets
        .get_api_key(&config.provider)
        .or_else(|| kernel.secrets.get_api_key("deepseek"))
        .ok_or_else(|| format!("{} API Key 未配置，请在设置中配置。", config.provider))?;
    Ok((config, api_key))
}

// ---- Tag Suggestions ----

#[tauri::command]
pub async fn get_canvas_tag_suggestions(
    kernel: State<'_, Arc<AppKernel>>,
    kb_id: String,
    query: String,
) -> Result<Vec<String>, String> {
    let conn = kernel.db.connect()?;
    let query_lower = query.trim().to_lowercase();
    let mut all_tags = std::collections::HashSet::new();

    // Query wiki_pages.tags
    let mut stmt = conn
        .prepare("SELECT DISTINCT tags FROM wiki_pages WHERE kb_id = ?1 AND LOWER(tags) LIKE ?2")
        .map_err(|e| format!("查询 wiki_pages 标签失败: {}", e))?;
    let pattern = format!("%{}%", query_lower);
    if let Ok(rows) = stmt.query_map(rusqlite::params![kb_id, pattern], |row| {
        row.get::<_, String>(0)
    }) {
        for row in rows.flatten() {
            for tag in row.split(',') {
                let t = tag.trim();
                if !t.is_empty() && t.to_lowercase().contains(&query_lower) {
                    all_tags.insert(t.to_string());
                }
            }
        }
    }

    // Query graph_nodes.tags
    let mut stmt2 = conn
        .prepare("SELECT DISTINCT tags FROM graph_nodes WHERE kb_id = ?1 AND LOWER(tags) LIKE ?2")
        .map_err(|e| format!("查询 graph_nodes 标签失败: {}", e))?;
    if let Ok(rows) = stmt2.query_map(rusqlite::params![kb_id, pattern], |row| {
        row.get::<_, String>(0)
    }) {
        for row in rows.flatten() {
            for tag in row.split(',') {
                let t = tag.trim();
                if !t.is_empty() && t.to_lowercase().contains(&query_lower) {
                    all_tags.insert(t.to_string());
                }
            }
        }
    }

    let mut result: Vec<String> = all_tags.into_iter().collect();
    result.sort();
    Ok(result)
}

// ---- Scope Check ----

#[tauri::command]
pub async fn check_canvas_scope(
    kernel: State<'_, Arc<AppKernel>>,
    kb_id: String,
    tags: Vec<String>,
) -> Result<serde_json::Value, String> {
    let conn = kernel.db.connect()?;

    // Find source files matching ALL provided tags
    let mut source_ids = Vec::new();
    let mut stmt = conn
        .prepare("SELECT id, text_length FROM sources WHERE kb_id = ?1")
        .map_err(|e| format!("查询源文件失败: {}", e))?;
    let sources: Vec<(String, i64)> = stmt
        .query_map(rusqlite::params![kb_id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1).unwrap_or(0)))
        })
        .map_err(|e| format!("读取源文件行失败: {}", e))?
        .filter_map(|r| r.ok())
        .collect();

    // For each source, check tag match via wiki_pages linked to that source
    for (source_id, text_length) in &sources {
        // Check if any wiki page with this source's tags matches our filter tags
        let mut tag_stmt = conn
            .prepare(
                "SELECT DISTINCT wp.tags FROM wiki_pages wp
                 INNER JOIN knowledge_items ki ON ki.page_id = wp.id
                 WHERE ki.source_id = ?1 AND wp.kb_id = ?2",
            )
            .map_err(|e| format!("查询页面标签失败: {}", e))?;
        let page_tags: Vec<String> = tag_stmt
            .query_map(rusqlite::params![source_id, kb_id], |row| {
                row.get::<_, String>(0)
            })
            .map_err(|e| format!("读取页面标签失败: {}", e))?
            .filter_map(|r| r.ok())
            .collect();

        let all_page_tags: std::collections::HashSet<String> = page_tags
            .iter()
            .flat_map(|s| {
                s.split(',')
                    .map(|t| t.trim().to_lowercase().to_string())
                    .collect::<Vec<_>>()
            })
            .collect();

        let tags_lower: std::collections::HashSet<String> =
            tags.iter().map(|t| t.trim().to_lowercase().to_string()).collect();

        if tags_lower.iter().all(|t| all_page_tags.contains(t)) {
            source_ids.push((source_id.clone(), *text_length));
        }
    }

    let mut total_words: u64 = 0;
    let mut matched_ids: Vec<String> = Vec::new();
    for (id, text_length) in &source_ids {
        total_words += *text_length as u64;
        matched_ids.push(id.clone());
    }

    const MAX_WORDS: u64 = 200_000;

    if total_words > MAX_WORDS {
        return Ok(serde_json::json!({
            "total_words": total_words,
            "matched_file_count": matched_ids.len(),
            "cache_key": "",
            "blocked": true,
            "message": format!(
                "关联知识过多（{} 字），请增加更精确的标签缩小视域范围。当前限制：{} 字。",
                total_words, MAX_WORDS
            ),
        }));
    }

    let cache_key = canvas_service::compute_cache_key(&tags, &matched_ids);

    Ok(serde_json::json!({
        "total_words": total_words,
        "matched_file_count": matched_ids.len(),
        "cache_key": cache_key,
        "blocked": false,
        "message": null,
    }))
}

// ---- Outline Generation ----

#[tauri::command]
pub async fn generate_canvas_outline(
    kernel: State<'_, Arc<AppKernel>>,
    kb_id: String,
    tags: Vec<String>,
    cache_key: String,
) -> Result<serde_json::Value, String> {
    // Check cache
    let conn = kernel.db.connect()?;
    if let Ok(Some(cached_json)) = conn
        .query_row(
            "SELECT content_json FROM canvas_cache WHERE kb_id = ?1 AND cache_key = ?2 AND content_type = 'outline'",
            rusqlite::params![kb_id, cache_key],
            |row| row.get::<_, String>(0),
        )
        .map(Some)
        .or_else(|e| {
            if e == rusqlite::Error::QueryReturnedNoRows {
                Ok(None)
            } else {
                Err(e)
            }
        })
    {
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(&cached_json) {
            log::info!("[canvas] 使用缓存的 outline, cache_key={}", cache_key);
            return Ok(val);
        }
    }

    // Gather source content
    let (config, _api_key) = resolve_config(&kernel)?;
    let source_content = gather_source_content_for_tags(&conn, &kb_id, &tags)?;

    if source_content.is_empty() {
        return Err("未找到匹配的源文件内容，请调整标签范围".to_string());
    }

    let system_prompt = CanvasPrompts::outline_system_prompt().to_string();
    let user_prompt = format!(
        "请根据以下文档内容生成学科知识大纲树：\n\n【选定的标签】{}\n\n【源文档内容】\n{}",
        tags.join(", "),
        truncate_content(&source_content, 60000),
    );

    let gateway = crate::model::model_gateway::ModelGateway::new(kernel.secrets.clone());
    let messages = vec![
        crate::model::model_gateway::ChatMessage {
            role: "system".to_string(),
            content: system_prompt,
        },
        crate::model::model_gateway::ChatMessage {
            role: "user".to_string(),
            content: user_prompt,
        },
    ];

    let result = gateway.chat(&config, messages, true).await
        .map_err(|e| format!("大纲生成失败: {}", e))?;

    // Parse JSON from response (with repair for common LLM malformations)
    let parsed: serde_json::Value = json_repair::validate_and_repair_json(&result.content)
        .map_err(|e| format!("解析大纲 JSON 失败: {} — 原始响应: {}", e, &result.content[..result.content.len().min(500)]))?;

    // Cache the result
    let now = chrono::Utc::now().to_rfc3339();
    let content_json = serde_json::to_string(&parsed).unwrap_or_default();
    let cache_id = format!("cache_outline_{}", uuid::Uuid::new_v4());
    conn.execute(
        "INSERT INTO canvas_cache (id, kb_id, cache_key, content_type, topic, content_json, source_file_ids, total_words, created_at)
         VALUES (?1, ?2, ?3, 'outline', '', ?4, '[]', 0, ?5)",
        rusqlite::params![cache_id, kb_id, cache_key, content_json, now],
    )
    .map_err(|e| format!("缓存大纲失败: {}", e))?;

    Ok(parsed)
}

// ---- Textbook Generation (streaming) ----

#[tauri::command]
pub async fn generate_canvas_textbook(
    app_handle: tauri::AppHandle,
    kernel: State<'_, Arc<AppKernel>>,
    kb_id: String,
    tags: Vec<String>,
    outline_json: String,
    cache_key: String,
) -> Result<(), String> {
    // Check cache
    let conn = kernel.db.connect()?;
    if let Ok(Some(cached_text)) = conn
        .query_row(
            "SELECT content_json FROM canvas_cache WHERE kb_id = ?1 AND cache_key = ?2 AND content_type = 'textbook'",
            rusqlite::params![kb_id, cache_key],
            |row| row.get::<_, String>(0),
        )
        .map(Some)
        .or_else(|e| {
            if e == rusqlite::Error::QueryReturnedNoRows { Ok(None) } else { Err(e) }
        })
    {
        log::info!("[canvas] 使用缓存的 textbook, cache_key={}", cache_key);
        let _ = app_handle.emit(
            "canvas-stream-chunk",
            serde_json::json!({"chunk": "", "accumulated": cached_text}),
        );
        let _ = app_handle.emit(
            "canvas-stream-done",
            serde_json::json!({"full_text": cached_text}),
        );
        return Ok(());
    }

    let (config, _api_key) = resolve_config(&kernel)?;
    let source_content = gather_source_content_for_tags(&conn, &kb_id, &tags)?;

    let system_prompt = CanvasPrompts::textbook_system_prompt().to_string();
    let user_prompt = format!(
        "【大纲结构】\n{}\n\n【源文档内容】\n{}",
        outline_json,
        truncate_content(&source_content, 50000),
    );

    let gateway = crate::model::model_gateway::ModelGateway::new(kernel.secrets.clone());
    let messages = vec![
        crate::model::model_gateway::ChatMessage {
            role: "system".to_string(),
            content: system_prompt,
        },
        crate::model::model_gateway::ChatMessage {
            role: "user".to_string(),
            content: user_prompt,
        },
    ];

    let mut rx = match gateway.chat_stream(&config, messages, false).await {
        Ok(rx) => rx,
        Err(e) => {
            let _ = app_handle.emit(
                "canvas-stream-error",
                serde_json::json!({"error": e.clone()}),
            );
            return Err(e);
        }
    };

    let mut full_text = String::new();
    while let Some(chunk) = rx.recv().await {
        full_text.push_str(&chunk);
        let _ = app_handle.emit(
            "canvas-stream-chunk",
            serde_json::json!({"chunk": chunk, "accumulated": full_text}),
        );
    }

    let _ = app_handle.emit(
        "canvas-stream-done",
        serde_json::json!({"full_text": full_text}),
    );

    // Cache
    let now = chrono::Utc::now().to_rfc3339();
    let cache_id = format!("cache_textbook_{}", uuid::Uuid::new_v4());
    let _ = conn.execute(
        "INSERT INTO canvas_cache (id, kb_id, cache_key, content_type, topic, content_json, source_file_ids, total_words, created_at)
         VALUES (?1, ?2, ?3, 'textbook', '', ?4, '[]', 0, ?5)",
        rusqlite::params![cache_id, kb_id, cache_key, full_text, now],
    );

    Ok(())
}

// ---- Node Detail ----

#[tauri::command]
pub async fn get_canvas_node_detail(
    kernel: State<'_, Arc<AppKernel>>,
    kb_id: String,
    topic: String,
    tags: Vec<String>,
    cache_key: String,
) -> Result<serde_json::Value, String> {
    // Check cache
    let conn = kernel.db.connect()?;
    if let Ok(Some(cached_json)) = conn
        .query_row(
            "SELECT content_json FROM canvas_cache WHERE kb_id = ?1 AND cache_key = ?2 AND content_type = 'detail' AND topic = ?3",
            rusqlite::params![kb_id, cache_key, topic],
            |row| row.get::<_, String>(0),
        )
        .map(Some)
        .or_else(|e| {
            if e == rusqlite::Error::QueryReturnedNoRows { Ok(None) } else { Err(e) }
        })
    {
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(&cached_json) {
            log::info!("[canvas] 使用缓存的 detail, topic={}", topic);
            return Ok(val);
        }
    }

    let (config, _api_key) = resolve_config(&kernel)?;
    let source_content = gather_source_content_for_tags(&conn, &kb_id, &tags)?;

    let system_prompt = CanvasPrompts::detail_system_prompt().to_string();
    let user_prompt = format!(
        "【知识点】{}\n【关联标签】{}\n【参考文档节选】\n{}",
        topic,
        tags.join(", "),
        truncate_content(&source_content, 15000),
    );

    let gateway = crate::model::model_gateway::ModelGateway::new(kernel.secrets.clone());
    let messages = vec![
        crate::model::model_gateway::ChatMessage {
            role: "system".to_string(),
            content: system_prompt,
        },
        crate::model::model_gateway::ChatMessage {
            role: "user".to_string(),
            content: user_prompt,
        },
    ];

    let result = gateway.chat(&config, messages, true).await
        .map_err(|e| format!("知识详情生成失败: {}", e))?;

    let parsed: serde_json::Value = json_repair::validate_and_repair_json(&result.content)
        .map_err(|e| format!("解析详情 JSON 失败: {} — 原始响应: {}", e, &result.content[..result.content.len().min(500)]))?;
    // Cache
    let now = chrono::Utc::now().to_rfc3339();
    let content_json = serde_json::to_string(&parsed).unwrap_or_default();
    let cache_id = format!("cache_detail_{}", uuid::Uuid::new_v4());
    let _ = conn.execute(
        "INSERT INTO canvas_cache (id, kb_id, cache_key, content_type, topic, content_json, source_file_ids, total_words, created_at)
         VALUES (?1, ?2, ?3, 'detail', ?4, ?5, '[]', 0, ?6)",
        rusqlite::params![cache_id, kb_id, cache_key, topic, content_json, now],
    );

    Ok(parsed)
}

// ---- Saved Scopes CRUD ----

#[tauri::command]
pub async fn get_canvas_scopes(
    kernel: State<'_, Arc<AppKernel>>,
    kb_id: String,
) -> Result<Vec<serde_json::Value>, String> {
    let conn = kernel.db.connect()?;
    let mut stmt = conn
        .prepare(
            "SELECT id, kb_id, name, tags_json, last_scroll_position, created_at, updated_at
             FROM canvas_scopes WHERE kb_id = ?1 ORDER BY updated_at DESC",
        )
        .map_err(|e| format!("查询画布书签失败: {}", e))?;

    let scopes: Vec<serde_json::Value> = stmt
        .query_map(rusqlite::params![kb_id], |row| {
            Ok(serde_json::json!({
                "id": row.get::<_, String>(0)?,
                "kb_id": row.get::<_, String>(1)?,
                "name": row.get::<_, String>(2)?,
                "tags": parse_tags_json(&row.get::<_, String>(3).unwrap_or_default()),
                "last_scroll_position": row.get::<_, i64>(4).unwrap_or(0),
                "created_at": row.get::<_, String>(5)?,
                "updated_at": row.get::<_, String>(6)?,
            }))
        })
        .map_err(|e| format!("读取画布书签行失败: {}", e))?
        .filter_map(|r| r.ok())
        .collect();

    Ok(scopes)
}

#[tauri::command]
pub async fn save_canvas_scope(
    kernel: State<'_, Arc<AppKernel>>,
    kb_id: String,
    name: String,
    tags_json: String,
    scroll_position: i64,
) -> Result<serde_json::Value, String> {
    let conn = kernel.db.connect()?;
    let id = format!("scope_{}", uuid::Uuid::new_v4());
    let now = chrono::Utc::now().to_rfc3339();

    conn.execute(
        "INSERT INTO canvas_scopes (id, kb_id, name, tags_json, last_scroll_position, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        rusqlite::params![id, kb_id, name, tags_json, scroll_position, now, now],
    )
    .map_err(|e| format!("保存画布书签失败: {}", e))?;

    Ok(serde_json::json!({
        "id": id,
        "kb_id": kb_id,
        "name": name,
        "tags": parse_tags_json(&tags_json),
        "last_scroll_position": scroll_position,
        "created_at": now,
        "updated_at": now,
    }))
}

#[tauri::command]
pub async fn delete_canvas_scope(
    kernel: State<'_, Arc<AppKernel>>,
    scope_id: String,
) -> Result<(), String> {
    let conn = kernel.db.connect()?;
    conn.execute("DELETE FROM canvas_scopes WHERE id = ?1", rusqlite::params![scope_id])
        .map_err(|e| format!("删除画布书签失败: {}", e))?;
    Ok(())
}

#[tauri::command]
pub async fn rename_canvas_scope(
    kernel: State<'_, Arc<AppKernel>>,
    scope_id: String,
    name: String,
) -> Result<(), String> {
    let conn = kernel.db.connect()?;
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "UPDATE canvas_scopes SET name = ?1, updated_at = ?2 WHERE id = ?3",
        rusqlite::params![name, now, scope_id],
    )
    .map_err(|e| format!("重命名画布书签失败: {}", e))?;
    Ok(())
}

// ---- Helpers ----

fn gather_source_content_for_tags(
    conn: &rusqlite::Connection,
    kb_id: &str,
    tags: &[String],
) -> Result<String, String> {
    let mut stmt = conn
        .prepare(
            "SELECT s.id, s.extracted_text FROM sources s WHERE s.kb_id = ?1",
        )
        .map_err(|e| format!("查询源文件失败: {}", e))?;
    let sources: Vec<(String, String)> = stmt
        .query_map(rusqlite::params![kb_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1).unwrap_or_default(),
            ))
        })
        .map_err(|e| format!("读取源文件行失败: {}", e))?
        .filter_map(|r| r.ok())
        .filter(|(_, text)| !text.is_empty())
        .collect();

    let tags_lower: std::collections::HashSet<String> =
        tags.iter().map(|t| t.trim().to_lowercase().to_string()).collect();

    // Collect content from sources that have matching tags via wiki_pages
    let mut all_content = String::new();
    for (source_id, text) in &sources {
        let mut tag_stmt = conn
            .prepare(
                "SELECT DISTINCT wp.tags FROM wiki_pages wp
                 INNER JOIN knowledge_items ki ON ki.page_id = wp.id
                 WHERE ki.source_id = ?1 AND wp.kb_id = ?2",
            )
            .map_err(|e| format!("查询页面标签失败: {}", e))?;
        let page_tags: Vec<String> = tag_stmt
            .query_map(rusqlite::params![source_id, kb_id], |row| {
                row.get::<_, String>(0)
            })
            .map_err(|e| format!("读取页面标签失败: {}", e))?
            .filter_map(|r| r.ok())
            .collect();

        let all_page_tags: std::collections::HashSet<String> = page_tags
            .iter()
            .flat_map(|s| {
                s.split(',')
                    .map(|t| t.trim().to_lowercase().to_string())
                    .collect::<Vec<_>>()
            })
            .collect();

        if tags_lower.iter().all(|t| all_page_tags.contains(t)) {
            all_content.push_str(text);
            all_content.push_str("\n\n---\n\n");
        }
    }

    Ok(all_content)
}

fn truncate_content(content: &str, max_chars: usize) -> String {
    if content.len() <= max_chars {
        content.to_string()
    } else {
        format!(
            "{}\n\n... (内容截断，原始长度 {} 字符)",
            &content[..max_chars],
            content.len()
        )
    }
}

fn parse_tags_json(s: &str) -> Vec<String> {
    serde_json::from_str::<Vec<String>>(s).unwrap_or_else(|_| {
        s.split(',')
            .map(|t| t.trim().to_string())
            .filter(|t| !t.is_empty())
            .collect()
    })
}

// ======================================================================
// Web-sourced canvas generation (when local KB has no relevant content)
// ======================================================================

#[derive(Debug, serde::Deserialize)]
pub struct WebSourceItem {
    pub title: String,
    pub url: String,
    pub content: String,
    pub selected: bool,
}

/// Generate outline from web search results
#[tauri::command]
pub async fn generate_canvas_outline_from_web(
    kernel: State<'_, Arc<AppKernel>>,
    kb_id: String,
    tags: Vec<String>,
    web_sources_json: String,
) -> Result<serde_json::Value, String> {
    let web_sources: Vec<WebSourceItem> = serde_json::from_str(&web_sources_json)
        .map_err(|e| format!("解析网页源数据失败: {}", e))?;

    let selected: Vec<&WebSourceItem> = web_sources.iter().filter(|s| s.selected).collect();
    if selected.is_empty() {
        return Err("请至少选择一个网页作为参考数据".to_string());
    }

    let (config, _api_key) = resolve_config(&kernel)?;

    // Build web content block
    let mut web_content = String::new();
    for (i, src) in selected.iter().enumerate() {
        web_content.push_str(&format!(
            "## 参考网页 {}: {}\n来源: {}\n\n{}\n\n",
            i + 1,
            src.title,
            src.url,
            truncate_content(&src.content, 8000),
        ));
    }

    let system_prompt = CanvasPrompts::outline_system_prompt().to_string();
    let user_prompt = format!(
        "请根据以下网页内容生成学科知识大纲树：\n\n【选定的标签】{}\n\n【网页参考资料】\n{}",
        tags.join(", "),
        truncate_content(&web_content, 60000),
    );

    let gateway = crate::model::model_gateway::ModelGateway::new(kernel.secrets.clone());
    let messages = vec![
        crate::model::model_gateway::ChatMessage {
            role: "system".to_string(),
            content: system_prompt,
        },
        crate::model::model_gateway::ChatMessage {
            role: "user".to_string(),
            content: user_prompt,
        },
    ];

    let result = gateway.chat(&config, messages, true).await
        .map_err(|e| format!("大纲生成失败: {}", e))?;

    let parsed: serde_json::Value = json_repair::validate_and_repair_json(&result.content)
        .map_err(|e| format!("解析大纲 JSON 失败: {}", e))?;

    // Save web sources as KB sources for future reference
    let conn = kernel.db.connect()?;
    let kb_path: String = conn
        .query_row(
            "SELECT path FROM knowledge_bases WHERE id = ?1",
            rusqlite::params![kb_id],
            |row| row.get(0),
        )
        .map_err(|e| format!("查询知识库路径失败: {}", e))?;
    for src in &selected {
        let now = chrono::Utc::now().to_rfc3339();
        let safe_title = src.title.chars().take(30).map(|c| if c.is_alphanumeric() || c == '_' || c == '-' { c } else { '_' }).collect::<String>();
        let file_name = format!("web_canvas_{}.md", safe_title);
        let dest_dir = std::path::PathBuf::from(&kb_path).join("raw").join("sources").join("web_canvas");
        let _ = std::fs::create_dir_all(&dest_dir);
        let dest_path = dest_dir.join(&file_name);
        let md_content = format!("---\ntitle: {}\ntype: source\nsource_url: {}\ncreated: {}\ntags: {}\n---\n\n{}", src.title, src.url, now, tags.join(","), src.content);
        if let Err(e) = std::fs::write(&dest_path, &md_content) {
            log::error!("[canvas] 保存网页源失败: {}", e);
            continue;
        }
        let source_id = uuid::Uuid::new_v4().to_string();
        let _ = conn.execute(
            "INSERT INTO sources (id, kb_id, file_name, file_path, file_type, file_size, extracted_text, status, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, 'md', ?5, ?6, 'processed', ?7, ?7)",
            rusqlite::params![source_id, kb_id, file_name, dest_path.to_string_lossy(), src.content.len() as i64, src.content, now],
        );
    }

    Ok(parsed)
}

/// Generate textbook from web search results (streaming)
#[tauri::command]
pub async fn generate_canvas_textbook_from_web(
    app_handle: tauri::AppHandle,
    kernel: State<'_, Arc<AppKernel>>,
    _kb_id: String,
    _tags: Vec<String>,
    outline_json: String,
    web_sources_json: String,
) -> Result<(), String> {
    let web_sources: Vec<WebSourceItem> = serde_json::from_str(&web_sources_json)
        .map_err(|e| format!("解析网页源数据失败: {}", e))?;

    let selected: Vec<&WebSourceItem> = web_sources.iter().filter(|s| s.selected).collect();
    if selected.is_empty() {
        return Err("请至少选择一个网页作为参考数据".to_string());
    }

    let (config, _api_key) = resolve_config(&kernel)?;

    let mut web_content = String::new();
    for (i, src) in selected.iter().enumerate() {
        web_content.push_str(&format!(
            "## 参考网页 {}: {}\n来源: {}\n\n{}\n\n",
            i + 1,
            src.title,
            src.url,
            truncate_content(&src.content, 6000),
        ));
    }

    let system_prompt = CanvasPrompts::textbook_system_prompt().to_string();
    let user_prompt = format!(
        "【大纲结构】\n{}\n\n【网页参考资料】\n{}",
        outline_json,
        truncate_content(&web_content, 50000),
    );

    let gateway = crate::model::model_gateway::ModelGateway::new(kernel.secrets.clone());
    let messages = vec![
        crate::model::model_gateway::ChatMessage {
            role: "system".to_string(),
            content: system_prompt,
        },
        crate::model::model_gateway::ChatMessage {
            role: "user".to_string(),
            content: user_prompt,
        },
    ];

    let mut rx = match gateway.chat_stream(&config, messages, false).await {
        Ok(rx) => rx,
        Err(e) => {
            let _ = app_handle.emit("canvas-stream-error", serde_json::json!({"error": e.clone()}));
            return Err(e);
        }
    };

    let mut full_text = String::new();
    while let Some(chunk) = rx.recv().await {
        full_text.push_str(&chunk);
        let _ = app_handle.emit("canvas-stream-chunk", serde_json::json!({"chunk": chunk, "accumulated": full_text}));
    }

    let _ = app_handle.emit("canvas-stream-done", serde_json::json!({"full_text": full_text}));

    Ok(())
}

/// Generate mindmap from text content (for web-sourced content or any text)
#[tauri::command]
pub async fn generate_mindmap_from_text(
    kernel: State<'_, Arc<AppKernel>>,
    topic: String,
    text_content: String,
) -> Result<serde_json::Value, String> {
    let (config, _api_key) = resolve_config(&kernel)?;

    let system_prompt = r#"你是一个知识结构分析专家。请将提供的文本内容分析并生成思维导图树结构。

返回 JSON 格式：
{
  "topic": "主题",
  "children": [
    {
      "topic": "子主题1",
      "children": [...]
    }
  ]
}

规则：
1. 最多 3 层深度
2. 每个节点 topic 不超过 15 个字
3. 提取最核心的概念和关系
4. 只输出 JSON，不要有其他内容"#;

    let user_prompt = format!(
        "主题: {}\n\n文本内容:\n{}",
        topic,
        truncate_content(&text_content, 15000),
    );

    let gateway = crate::model::model_gateway::ModelGateway::new(kernel.secrets.clone());
    let messages = vec![
        crate::model::model_gateway::ChatMessage {
            role: "system".to_string(),
            content: system_prompt.to_string(),
        },
        crate::model::model_gateway::ChatMessage {
            role: "user".to_string(),
            content: user_prompt,
        },
    ];

    let result = gateway.chat(&config, messages, true).await
        .map_err(|e| format!("思维导图生成失败: {}", e))?;

    let parsed: serde_json::Value = json_repair::validate_and_repair_json(&result.content)
        .map_err(|e| format!("解析思维导图 JSON 失败: {}", e))?;

    Ok(parsed)
}
