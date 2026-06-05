use std::sync::Arc;
use tauri::{Emitter, State};
use crate::core::app_kernel::AppKernel;
use crate::core::config_service::ProviderConfig;
use crate::agents::coordinator::build_query_context_from_db;
use crate::prompts::prompt_builder::PromptBuilder;

// ============================================================
// Provider 配置（新：多供应商通用）
// ============================================================

#[tauri::command]
pub async fn save_provider_config(
    kernel: State<'_, Arc<AppKernel>>,
    provider: String,
    base_url: String,
    api_key: String,
    chat_model: String,
    reasoner_model: String,
    temperature: f64,
    max_tokens: u32,
    timeout: u32,
    retry_count: u32,
    stream: bool,
) -> Result<(), String> {
    if base_url.trim().is_empty() {
        return Err("Base URL 不能为空".to_string());
    }
    if chat_model.trim().is_empty() {
        return Err("Chat 模型名称不能为空".to_string());
    }
    if !(0.0..=2.0).contains(&temperature) {
        return Err("Temperature 必须在 0.0 ~ 2.0 之间".to_string());
    }
    if max_tokens == 0 || max_tokens > 65536 {
        return Err("Max Tokens 必须在 1 ~ 65536 之间".to_string());
    }
    if timeout == 0 || timeout > 600 {
        return Err("Timeout 必须在 1 ~ 600 秒之间".to_string());
    }

    let valid_providers = ["deepseek", "openai", "anthropic", "ollama", "openwebui", "custom"];
    if !valid_providers.contains(&provider.as_str()) {
        return Err(format!("不支持的供应商: {}。支持: {}", provider, valid_providers.join(", ")));
    }

    kernel.secrets.store_api_key(&provider, &api_key);

    let config = ProviderConfig {
        provider: provider.clone(),
        base_url: base_url.clone(),
        chat_model: chat_model.clone(),
        reasoner_model,
        temperature,
        max_tokens,
        timeout,
        retry_count,
        stream,
    };

    kernel.config.save_provider_config(&config)?;

    // 同时存储 deepseek key 以兼容旧引用
    if provider == "deepseek" {
        kernel.secrets.store_api_key("deepseek", &api_key);
    }

    Ok(())
}

#[tauri::command]
pub async fn get_provider_config(
    kernel: State<'_, Arc<AppKernel>>,
) -> Result<serde_json::Value, String> {
    let config = kernel.config.get_provider_config()?;
    let masked_key = kernel.secrets.mask_api_key(&config.provider)
        .unwrap_or_else(|| "未配置".to_string());

    Ok(serde_json::json!({
        "provider": config.provider,
        "base_url": config.base_url,
        "chat_model": config.chat_model,
        "reasoner_model": config.reasoner_model,
        "temperature": config.temperature,
        "max_tokens": config.max_tokens,
        "timeout": config.timeout,
        "retry_count": config.retry_count,
        "stream": config.stream,
        "api_key_masked": masked_key,
    }))
}

// ============================================================
// DeepSeek 配置（保留向后兼容，内部委托给 ProviderConfig）
// ============================================================

#[tauri::command]
pub async fn save_deepseek_config(
    kernel: State<'_, Arc<AppKernel>>,
    base_url: String,
    api_key: String,
    chat_model: String,
    reasoner_model: String,
    temperature: f64,
    max_tokens: u32,
    timeout: u32,
    retry_count: u32,
    stream: bool,
) -> Result<(), String> {
    // 委托给 save_provider_config，以 deepseek 作为 provider
    save_provider_config(
        kernel,
        "deepseek".to_string(),
        base_url, api_key, chat_model, reasoner_model,
        temperature, max_tokens, timeout, retry_count, stream,
    ).await
}

#[tauri::command]
pub async fn get_deepseek_config(
    kernel: State<'_, Arc<AppKernel>>,
) -> Result<serde_json::Value, String> {
    // 向后兼容：返回与旧格式一致的数据
    let config = kernel.config.get_provider_config()
        .map_err(|e| format!("读取供应商配置失败: {}", e))?;
    let masked_key = kernel.secrets.mask_api_key(&config.provider)
        .or_else(|| kernel.secrets.mask_api_key("deepseek"))
        .unwrap_or_else(|| "未配置".to_string());

    Ok(serde_json::json!({
        "base_url": config.base_url,
        "chat_model": config.chat_model,
        "reasoner_model": config.reasoner_model,
        "temperature": config.temperature,
        "max_tokens": config.max_tokens,
        "timeout": config.timeout,
        "retry_count": config.retry_count,
        "stream": config.stream,
        "api_key_masked": masked_key,
        "provider": config.provider,
    }))
}

// ============================================================
// 连接测试（使用 ProviderConfig）
// ============================================================

fn resolve_provider_config(kernel: &AppKernel) -> Result<(ProviderConfig, String), String> {
    let config = kernel.config.get_provider_config()?;
    let api_key = kernel.secrets.get_api_key(&config.provider)
        .or_else(|| {
            log::warn!("[config] 供应商 '{}' 的 API Key 未找到，回退到 'deepseek'", config.provider);
            kernel.secrets.get_api_key("deepseek")
        })
        .ok_or_else(|| format!("{} API Key 未配置，请在设置中配置。", config.provider))?;
    Ok((config, api_key))
}

#[tauri::command]
pub async fn test_connection(
    kernel: State<'_, Arc<AppKernel>>,
) -> Result<String, String> {
    let (config, api_key) = resolve_provider_config(&kernel)?;
    let gateway = crate::model::model_gateway::ModelGateway::new(kernel.secrets.clone());
    gateway.test_connection(&config, &api_key).await
}

#[tauri::command]
pub async fn test_json_output(
    kernel: State<'_, Arc<AppKernel>>,
) -> Result<String, String> {
    let (config, api_key) = resolve_provider_config(&kernel)?;
    let gateway = crate::model::model_gateway::ModelGateway::new(kernel.secrets.clone());
    gateway.test_json_output(&config, &api_key).await
}

#[tauri::command]
pub async fn test_document_attachment(
    kernel: State<'_, Arc<AppKernel>>,
) -> Result<String, String> {
    let (config, api_key) = resolve_provider_config(&kernel)?;
    let gateway = crate::model::model_gateway::ModelGateway::new(kernel.secrets.clone());
    gateway.test_document_attachment(&config, &api_key).await
}

#[tauri::command]
pub async fn chat_stream(
    app_handle: tauri::AppHandle,
    kernel: State<'_, Arc<AppKernel>>,
    system_prompt: String,
    user_content: String,
    history: Option<Vec<serde_json::Value>>,
    kb_id: Option<String>,
    scope: Option<String>,
    allow_ai_generation: Option<bool>,
) -> Result<(), String> {
    let (config, _) = resolve_provider_config(&kernel)?;
    let gateway = crate::model::model_gateway::ModelGateway::new(kernel.secrets.clone());

    let mut messages = Vec::new();

    // Include conversation history for context
    if let Some(ref history_msgs) = history {
        for msg in history_msgs {
            let role = msg.get("role").and_then(|v| v.as_str()).unwrap_or("user");
            let content = msg.get("content").and_then(|v| v.as_str()).unwrap_or("");
            if !content.is_empty() {
                messages.push(crate::model::model_gateway::ChatMessage {
                    role: role.to_string(),
                    content: content.to_string(),
                });
            }
        }
    } else {
        // Build wiki context if kb_id is provided
        let (final_system_prompt, final_user_content) = if let (Some(ref kb_id), Some(ref scope)) = (&kb_id, &scope) {
            // Query KB path from DB
            let conn = kernel.db.connect().map_err(|e| format!("连接数据库失败: {}", e))?;
            let kb_path: String = conn
                .query_row("SELECT path FROM knowledge_bases WHERE id = ?1", rusqlite::params![kb_id], |row| row.get(0))
                .map_err(|_| "知识库不存在".to_string())?;

            // 两阶段检索：语义搜索 + 关键词检索合并
            let mut wiki_context = String::new();
            match kernel.vdb.search_similar(kb_id, &user_content, 20) {
                Ok(results) if !results.is_empty() => {
                    // 简易 Rerank: 基于关键词密度精排 Top 3
                    let reranked: Vec<(String, f64)> = {
                        let question_lower = user_content.to_lowercase();
                        let keywords: Vec<&str> = question_lower
                            .split(|c: char| !c.is_alphanumeric() && c != '_')
                            .filter(|w| w.len() >= 2)
                            .collect();
                        let mut scored: Vec<(String, f64)> = results.iter().map(|(chunk, base_sim)| {
                            let chunk_lower = chunk.to_lowercase();
                            let kw_score: f64 = keywords.iter().map(|kw| {
                                (chunk_lower.matches(kw).count() as f64) / (chunk_lower.len().max(1) as f64) * 1000.0
                            }).sum();
                            (chunk.clone(), base_sim * 0.7 + kw_score * 0.3)
                        }).collect();
                        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
                        scored.truncate(3);
                        scored
                    };
                    wiki_context.push_str("# 语义检索结果 (Top 3 精排)\n\n");
                    for (i, (chunk, score)) in reranked.iter().enumerate() {
                        wiki_context.push_str(&format!("**相关片段 {}** (相关度: {:.2}):\n{}\n\n", i + 1, score, chunk));
                    }
                }
                Ok(_) => {}
                Err(e) => log::warn!("[chat_stream] 语义搜索失败: {}", e),
            }
            let kw_context = build_query_context_from_db(&kernel.db, &kernel.workspace, kb_id, scope, &kb_path)?;
            wiki_context.push_str(&kw_context);

            let scope_desc = match scope.as_str() {
                "all" => "整个知识库".to_string(),
                s if s.starts_with("tag:") => format!("页面类型: {}", &s[4..]),
                _ => format!("页面: {}", scope),
            };

            let allow_gen = allow_ai_generation.unwrap_or(true);
            let (sys_prompt, user_msg) = PromptBuilder::build_query_prompt(
                &user_content,
                &wiki_context,
                &scope_desc,
                allow_gen,
            );
            (sys_prompt, user_msg)
        } else {
            (system_prompt, user_content)
        };

        // Fallback: build a simple system + user message pair
        if !final_system_prompt.is_empty() {
            messages.push(crate::model::model_gateway::ChatMessage {
                role: "system".to_string(),
                content: final_system_prompt,
            });
        }
        messages.push(crate::model::model_gateway::ChatMessage {
            role: "user".to_string(),
            content: final_user_content,
        });
    }

    let mut rx = match gateway.chat_stream(&config, messages, false).await {
        Ok(rx) => rx,
        Err(e) => {
            let _ = app_handle.emit("chat-stream-error", serde_json::json!({ "error": e.clone() }));
            return Err(e);
        }
    };

    let mut full_text = String::new();
    while let Some(chunk) = rx.recv().await {
        full_text.push_str(&chunk);
        let payload = serde_json::json!({
            "chunk": chunk,
            "accumulated": full_text,
        });
        let _ = app_handle.emit("chat-stream-chunk", payload);
    }

    let _ = app_handle.emit("chat-stream-done", serde_json::json!({ "full_text": full_text }));
    Ok(())
}

#[tauri::command]
pub async fn check_api_key_status(
    kernel: State<'_, Arc<AppKernel>>,
) -> Result<serde_json::Value, String> {
    let config = kernel.config.get_provider_config()?;
    let has_key = kernel.secrets.has_api_key(&config.provider)
        || kernel.secrets.has_api_key("deepseek");
    let has_base_url = !config.base_url.is_empty();
    let has_chat_model = !config.chat_model.is_empty();
    let has_reasoner_model = !config.reasoner_model.is_empty();

    Ok(serde_json::json!({
        "api_key_configured": has_key,
        "base_url_configured": has_base_url,
        "chat_model_configured": has_chat_model,
        "reasoner_model_configured": has_reasoner_model,
        "all_configured": has_key && has_base_url && has_chat_model,
        "provider": config.provider,
        "base_url": config.base_url,
        "chat_model": config.chat_model,
        "reasoner_model": config.reasoner_model,
    }))
}

// ============================================================
// 模型配置方案
// ============================================================

#[tauri::command]
pub async fn list_model_profiles(
    kernel: State<'_, Arc<AppKernel>>,
) -> Result<Vec<serde_json::Value>, String> {
    let conn = kernel.db.connect()?;
    let mut stmt = conn.prepare(
        "SELECT id, provider, name, base_url, model_name, role, temperature, max_tokens, timeout, retry_count, created_at FROM model_profiles ORDER BY created_at DESC"
    ).map_err(|e| format!("准备查询失败: {}", e))?;
    let mapped = stmt.query_map([], |row| {
        Ok(serde_json::json!({
            "id": row.get::<_, String>(0)?,
            "provider": row.get::<_, String>(1)?,
            "name": row.get::<_, String>(2)?,
            "base_url": row.get::<_, String>(3)?,
            "model_name": row.get::<_, String>(4)?,
            "role": row.get::<_, String>(5)?,
            "temperature": row.get::<_, f64>(6)?,
            "max_tokens": row.get::<_, i32>(7)?,
            "timeout": row.get::<_, i32>(8)?,
            "retry_count": row.get::<_, i32>(9)?,
            "created_at": row.get::<_, String>(10)?,
        }))
    }).map_err(|e| format!("查询失败: {}", e))?;
    let profiles: Vec<serde_json::Value> = mapped
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("读取模型配置列表失败: {}", e))?;
    Ok(profiles)
}

#[tauri::command]
pub async fn save_model_profile(
    kernel: State<'_, Arc<AppKernel>>,
    name: String,
    provider: String,
    base_url: String,
    model_name: String,
    api_key: String,
    role: String,
    temperature: f64,
    max_tokens: i32,
    timeout: i32,
    retry_count: i32,
) -> Result<String, String> {
    if name.trim().is_empty() {
        return Err("配置名称不能为空".to_string());
    }
    if name.len() > 100 {
        return Err("配置名称不能超过 100 个字符".to_string());
    }
    if base_url.trim().is_empty() {
        return Err("Base URL 不能为空".to_string());
    }
    if model_name.trim().is_empty() {
        return Err("模型名称不能为空".to_string());
    }
    if !(0.0..=2.0).contains(&temperature) {
        return Err("Temperature 必须在 0.0 ~ 2.0 之间".to_string());
    }
    if !(1..=65536).contains(&max_tokens) {
        return Err("Max Tokens 必须在 1 ~ 65536 之间".to_string());
    }
    if !(1..=600).contains(&timeout) {
        return Err("Timeout 必须在 1 ~ 600 秒之间".to_string());
    }

    let profile_id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    let conn = kernel.db.connect()?;
    conn.execute(
        "INSERT INTO model_profiles (id, provider, name, base_url, model_name, encrypted_api_key_ref, role, temperature, max_tokens, timeout, retry_count, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?12)",
        rusqlite::params![profile_id, provider, name.trim(), base_url.trim(), model_name.trim(), format!("key_ref_{}", profile_id), role, temperature, max_tokens, timeout, retry_count, now],
    ).map_err(|e| format!("保存模型配置失败: {}", e))?;

    if !api_key.is_empty() {
        kernel.secrets.store_api_key(&profile_id, &api_key);
    }

    Ok(profile_id)
}

#[tauri::command]
pub async fn delete_model_profile(
    kernel: State<'_, Arc<AppKernel>>,
    profile_id: String,
) -> Result<(), String> {
    let conn = kernel.db.connect()?;
    conn.execute("DELETE FROM model_profiles WHERE id = ?1", rusqlite::params![profile_id])
        .map_err(|e| format!("删除模型配置失败: {}", e))?;
    kernel.secrets.remove_api_key(&profile_id);
    Ok(())
}

#[tauri::command]
pub async fn apply_model_profile(
    kernel: State<'_, Arc<AppKernel>>,
    profile_id: String,
) -> Result<(), String> {
    let conn = kernel.db.connect()?;
    let (provider, base_url, model_name, role, temperature, max_tokens, timeout, retry_count): (String, String, String, String, f64, i32, i32, i32) = conn.query_row(
        "SELECT provider, base_url, model_name, role, temperature, max_tokens, timeout, retry_count FROM model_profiles WHERE id = ?1",
        rusqlite::params![profile_id],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?, row.get(6)?, row.get(7)?)),
    ).map_err(|e| format!("获取模型配置失败: {}", e))?;

    let api_key = kernel.secrets.get_api_key(&profile_id)
        .ok_or_else(|| "API Key 未找到".to_string())?;

    let current = kernel.config.get_provider_config()
        .map_err(|e| format!("读取现有配置失败: {}", e))?;

    let (chat_model, reasoner_model) = if role == "reasoner" {
        (current.chat_model, model_name)
    } else {
        (model_name, current.reasoner_model)
    };

    let config = ProviderConfig {
        provider: provider.clone(),
        base_url,
        chat_model,
        reasoner_model,
        temperature,
        max_tokens: max_tokens as u32,
        timeout: timeout as u32,
        retry_count: retry_count as u32,
        stream: true,
    };

    kernel.config.save_provider_config(&config)?;
    kernel.secrets.store_api_key(&provider, &api_key);
    // 向后兼容
    if provider == "deepseek" {
        kernel.secrets.store_api_key("deepseek", &api_key);
    }

    Ok(())
}

// ============================================================
// 网页搜索配置
// ============================================================

#[tauri::command]
pub async fn save_web_search_config(
    kernel: State<'_, Arc<AppKernel>>,
    engine: String,
    max_results: u32,
    searxng_url: String,
    brave_api_key: String,
    bing_api_key: String,
    bing_endpoint: String,
) -> Result<(), String> {
    if max_results == 0 || max_results > 20 {
        return Err("最大结果数必须在 1-20 之间".to_string());
    }

    let valid_engines = ["duckduckgo", "searxng", "brave", "bing"];
    if !valid_engines.contains(&engine.as_str()) {
        return Err(format!("不支持的搜索引擎: {}。支持: {}", engine, valid_engines.join(", ")));
    }

    let config = crate::core::config_service::WebSearchConfig {
        engine,
        max_results,
        searxng_url,
        brave_api_key,
        bing_api_key,
        bing_endpoint,
    };

    kernel.config.save_web_search_config(&config)?;
    Ok(())
}

#[tauri::command]
pub async fn get_web_search_config(
    kernel: State<'_, Arc<AppKernel>>,
) -> Result<serde_json::Value, String> {
    let config = kernel.config.get_web_search_config()?;
    Ok(serde_json::json!({
        "engine": config.engine,
        "max_results": config.max_results,
        "searxng_url": config.searxng_url,
        "brave_api_key": config.brave_api_key,
        "bing_api_key": config.bing_api_key,
        "bing_endpoint": config.bing_endpoint,
    }))
}
