use std::sync::Arc;
use crate::core::task_queue::TaskQueue;
use crate::core::database_service::DatabaseService;
use crate::core::config_service::ConfigService;
use crate::core::event_bus::EventBus;
use crate::core::workspace_service::WorkspaceService;
use crate::core::task_queue::CancellationToken;
use crate::core::token_logger::TokenContext;
use crate::model::model_gateway::ModelGateway;
use crate::schema::json_repair;
use crate::skills::document_processor::DocumentProcessor;

pub struct SourceIngestAgent {
    task_queue: Arc<TaskQueue>,
    db: Arc<DatabaseService>,
    config: Arc<ConfigService>,
    event_bus: Arc<EventBus>,
    workspace: Arc<WorkspaceService>,
    model_gateway: Arc<ModelGateway>,
    source_file_name: String,
}

impl SourceIngestAgent {
    pub fn new(
        task_queue: Arc<TaskQueue>,
        db: Arc<DatabaseService>,
        config: Arc<ConfigService>,
        event_bus: Arc<EventBus>,
        workspace: Arc<WorkspaceService>,
        model_gateway: Arc<ModelGateway>,
        source_file_name: String,
    ) -> Self {
        Self {
            task_queue, db, config, event_bus,
            workspace, model_gateway,
            source_file_name,
        }
    }

    pub async fn execute(
        &self,
        kb_id: &str,
        kb_path: &str,
        source_id: &str,
        task_id: &str,
        cancel_token: &CancellationToken,
    ) -> Result<(), String> {
        // 0. 发送开始事件
        self.event_bus.emit_agent_activity(
            "SourceIngestAgent",
            "starting",
            &self.source_file_name,
            "开始文档分析任务",
        );
        self.event_bus.emit_agent_status_change(
            task_id, "document_parsed", "running", 0.05,
            "", "", "开始文档分析",
        );

        // 1. 更新 source 状态为 analyzing
        {
            let conn = self.db.connect()?;
            if let Err(e) = conn.execute(
                "UPDATE sources SET status = 'analyzing', updated_at = ?1 WHERE id = ?2",
                rusqlite::params![chrono::Utc::now().to_rfc3339(), source_id],
            ) {
                log::error!("[source_ingest] 更新 source 状态为 analyzing 失败 (source={}): {}", source_id, e);
            }
        }
        self.event_bus.emit_source_updated(kb_id, source_id);
        self.event_bus.emit_kb_stats_changed(kb_id);

        // 2. 更新 task 状态：准备构建 prompt
        self.task_queue.update_task_status(task_id, "prompt_built", "SourceIngestAgent", "")?;
        self.task_queue.add_event(task_id, "agent_start", "SourceIngestAgent", "开始文档分析")?;

        // 3. 获取 source 信息
        let conn = self.db.connect()?;
        let (file_name, file_path, file_type): (String, String, String) = conn
            .query_row(
                "SELECT file_name, file_path, file_type FROM sources WHERE id = ?1",
                rusqlite::params![source_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .map_err(|e| format!("获取 source 信息失败: {}", e))?;

        // 3. 检查是否已有提取文本
        let existing_text: Option<String> = match conn.query_row(
                "SELECT extracted_text FROM sources WHERE id = ?1",
                rusqlite::params![source_id],
                |row| row.get(0),
            ) {
                Ok(text) => Some(text),
                Err(rusqlite::Error::QueryReturnedNoRows) => None,
                Err(e) => {
                    log::error!("[SourceIngestAgent] 查询 source 已提取文本失败 (id={}): {}", source_id, e);
                    None
                }
            }.filter(|t: &String| !t.is_empty());

        // 4. 如果没有提取文本，进行文档解析
        let document_text = if let Some(text) = existing_text {
            self.event_bus.emit_agent_activity(
                "SourceIngestAgent",
                "parsing_skipped",
                &self.source_file_name,
                "使用已提取的文本",
            );
            text
        } else {
            self.event_bus.emit_agent_activity(
                "SourceIngestAgent",
                "parsing",
                &self.source_file_name,
                &format!("正在解析文档: {}", file_name),
            );
            self.task_queue.add_event(task_id, "parsing", "DocumentProcessor", &format!("正在解析文档: {}", file_name))?;

            let source_file = std::path::PathBuf::from(&file_path);
            if !source_file.exists() {
                return Err(format!("源文件不存在: {}", file_path));
            }

            match DocumentProcessor::parse_document(&source_file, &file_type) {
                Ok(result) => {
                    // 保存提取的文本到数据库
                    let conn_update = self.db.connect()?;
                    conn_update.execute(
                        "UPDATE sources SET extracted_text = ?1 WHERE id = ?2",
                        rusqlite::params![result.text, source_id],
                    ).map_err(|e| format!("保存提取文本失败: {}", e))?;

                    // 保存原始文本到任务目录
                    let task_dir = {
                        let kb_path_buf = std::path::PathBuf::from(kb_path);
                        let tasks_dir = self.workspace.get_tasks_dir(&kb_path_buf);
                        tasks_dir.join(task_id)
                    };
                    if let Err(e) = std::fs::create_dir_all(&task_dir) { log::error!("[source_ingest] 创建 task_dir 失败: {}", e); }
                    if let Err(e) = std::fs::write(task_dir.join("extracted_text.txt"), &result.text) { log::error!("[source_ingest] 写入 extracted_text 失败: {}", e); }

                    if !result.warnings.is_empty() {
                        for w in &result.warnings {
                            self.task_queue.add_event(task_id, "warning", "DocumentProcessor", w)?;
                        }
                    }

                    let parse_info = if let Some(pages) = result.page_count {
                        format!("PDF共{}页，已提取文本 {} 字符", pages, result.text_length)
                    } else {
                        format!("已提取文本 {} 字符", result.text_length)
                    };
                    self.event_bus.emit_agent_activity(
                        "SourceIngestAgent",
                        "parsed",
                        &self.source_file_name,
                        &parse_info,
                    );

                    if let Some(pages) = result.page_count {
                        self.task_queue.add_event(
                            task_id, "parse_info", "DocumentProcessor",
                            &format!("PDF共{}页，已提取文本 {} 字符", pages, result.text_length),
                        )?;
                    } else {
                        self.task_queue.add_event(
                            task_id, "parse_info", "DocumentProcessor",
                            &format!("已提取文本 {} 字符", result.text_length),
                        )?;
                    }
                    result.text
                }
                Err(e) => {
                    self.event_bus.emit_agent_activity(
                        "SourceIngestAgent",
                        "parse_failed",
                        &self.source_file_name,
                        &format!("文档解析失败: {}", e),
                    );
                    self.task_queue.add_event(task_id, "parse_error", "DocumentProcessor", &format!("文档解析失败: {}", e))?;
                    return Err(format!("文档解析失败: {}", e));
                }
            }
        };

        // 4.5 检查取消（解析完成后、AI 调用前）
        if cancel_token.is_cancelled() || self.task_queue.is_task_cancelled(task_id) {
            self.task_queue.add_event(task_id, "cancelled", "SourceIngestAgent", "文档解析后检测到取消")?;
            return Ok(());
        }

        // 5. 获取已有页面摘要
        self.event_bus.emit_agent_activity(
            "SourceIngestAgent",
            "context_gathering",
            &self.source_file_name,
            "正在准备已有知识上下文",
        );
        self.task_queue.add_event(task_id, "context_gathering", "SourceIngestAgent", "正在准备已有知识上下文")?;
        let existing_summary = self.get_existing_pages(kb_id)?;

        // 6. 构建 Prompt
        self.event_bus.emit_agent_activity(
            "SourceIngestAgent",
            "prompt_building",
            &self.source_file_name,
            "正在构建分析 Prompt",
        );
        self.event_bus.emit_agent_status_change(
            task_id, "prompt_built", "running", 0.15,
            "", "", "正在构建分析 Prompt",
        );
        self.task_queue.add_event(task_id, "prompt_building", "SourceIngestAgent", "正在构建分析 Prompt")?;
        let kb_path_buf = std::path::PathBuf::from(kb_path);
        let tasks_dir = self.workspace.get_tasks_dir(&kb_path_buf);
        let task_dir = tasks_dir.join(task_id);
        if let Err(e) = std::fs::create_dir_all(&task_dir) { log::error!("[source_ingest] 创建 task_dir 失败: {}", e); }

        // 检查是否需要分段处理
        let estimated_tokens = crate::model::deepseek_client::DeepSeekClient::estimate_tokens(&document_text);
        let max_chunk_tokens = 20000; // 每段最多 20K tokens（留足够空间给 prompt）
        let need_chunking = estimated_tokens > max_chunk_tokens;
        let total_chunks = if need_chunking {
            (estimated_tokens as f64 / max_chunk_tokens as f64).ceil() as u32
        } else {
            1
        };

        if need_chunking {
            self.event_bus.emit_agent_activity(
                "SourceIngestAgent",
                "chunking",
                &self.source_file_name,
                &format!("文档较大 ({} tokens)，分为 {} 段处理", estimated_tokens, total_chunks),
            );
            self.task_queue.add_event(task_id, "chunking", "SourceIngestAgent",
                &format!("文档较大 ({} tokens)，分为 {} 段处理", estimated_tokens, total_chunks))?;
        }

        let mut final_json = serde_json::json!({
            "entities": [],
            "concepts": [],
            "topics": [],
            "source_summary": {"title": "", "short_summary": ""},
        });

        // 按 token 数正确分段
        let chunks: Vec<String> = if need_chunking {
            let paragraphs: Vec<&str> = document_text.split("\n\n").collect();
            let mut chunks = Vec::new();
            let mut current_chunk = String::new();
            let mut current_tokens = 0u32;

            for para in &paragraphs {
                let para_tokens = crate::model::deepseek_client::DeepSeekClient::estimate_tokens(para).max(1);

                if current_tokens + para_tokens > max_chunk_tokens && !current_chunk.is_empty() {
                    chunks.push(current_chunk.trim().to_string());
                    current_chunk = String::new();
                    current_tokens = 0;
                }

                if !current_chunk.is_empty() {
                    current_chunk.push_str("\n\n");
                }
                current_chunk.push_str(para);
                current_tokens += para_tokens;
            }

            if !current_chunk.trim().is_empty() {
                chunks.push(current_chunk.trim().to_string());
            }

            // 更新实际分段数
            let actual_chunks = chunks.len() as u32;
            if actual_chunks != total_chunks {
                self.event_bus.emit_agent_activity(
                    "SourceIngestAgent",
                    "chunking",
                    &self.source_file_name,
                    &format!("文档较大 ({} tokens)，实际分为 {} 段处理", estimated_tokens, actual_chunks),
                );
                self.task_queue.add_event(task_id, "chunking", "SourceIngestAgent",
                    &format!("文档较大 ({} tokens)，实际分为 {} 段处理", estimated_tokens, actual_chunks))?;
            }

            chunks
        } else {
            vec![document_text.to_string()]
        };

        let config = self.config.get_provider_config()?;

        let total_chunks_final = chunks.len() as u32;

        for (idx, chunk) in chunks.iter().enumerate() {
            // 每段开始前检查取消
            if cancel_token.is_cancelled() || self.task_queue.is_task_cancelled(task_id) {
                self.task_queue.add_event(task_id, "cancelled", "SourceIngestAgent",
                    &format!("第 {} 段处理前检测到取消", idx + 1))?;
                return Ok(());
            }

            let chunk_label = if total_chunks_final > 1 {
                format!(" [段 {}/{}]", idx + 1, total_chunks_final)
            } else {
                String::new()
            };

            self.event_bus.emit_agent_activity(
                "SourceIngestAgent",
                "model_calling",
                &self.source_file_name,
                &format!("正在调用 DeepSeek 分析文档{}", chunk_label),
            );
            self.task_queue.update_task_status(task_id, "sent_to_model", "SourceIngestAgent",
                &format!("第 {} 段", idx + 1))?;
            self.task_queue.add_event(task_id, "model_calling", "DeepSeekClient",
                &format!("正在调用 DeepSeek 分析文档{} (预估 token: ~{})",
                    chunk_label,
                    crate::model::deepseek_client::DeepSeekClient::estimate_tokens(chunk) / 1000 * 1000
                ))?;

            let (system_prompt, user_message) = crate::prompts::prompt_builder::PromptBuilder::build_ingest_prompt(
                chunk,
                source_id,
                &existing_summary,
            );

            // 保存 prompt
            if let Err(e) = std::fs::write(task_dir.join(format!("prompt_{}.md", idx)), format!("{}\n\n---\n\n{}", system_prompt, user_message)) {
                log::error!("[SourceIngestAgent] 写入 prompt_{}.md 失败: {}", idx, e);
            }

            let token_ctx = TokenContext {
                task_id: task_id.to_string(),
                task_name: self.source_file_name.clone(),
                agent_name: "SourceIngestAgent".to_string(),
                model_name: config.chat_model.clone(),
                provider: config.provider.clone(),
            };

            // 通知前端：模型调用中，同时透传完整 Prompt
            let full_prompt = format!("{}\n\n---\n\n{}", system_prompt, user_message);
            self.event_bus.emit_agent_status_change(
                task_id, "model_called", "running", 0.3,
                &full_prompt, "", &format!("正在调用 LLM 分析文档{}", chunk_label),
            );

            let result = self.model_gateway.chat_with_content_and_ctx(
                &config, &system_prompt, &user_message, true, Some(token_ctx),
            ).await?;

            self.event_bus.emit_agent_activity(
                "SourceIngestAgent",
                "model_returned",
                &self.source_file_name,
                &format!("DeepSeek 返回，正在处理结果{}", chunk_label),
            );
            // 通知前端：模型返回完成，透传完整响应
            self.event_bus.emit_agent_status_change(
                task_id, "model_returned", "done", 0.7,
                "", &result.content, "LLM 响应已返回",
            );
            self.task_queue.update_task_status(task_id, "model_returned", "SourceIngestAgent", "")?;
            if let Err(e) = std::fs::write(task_dir.join(format!("model_raw_response_{}.txt", idx)), &result.content) { log::error!("[source_ingest] 写入 model_raw_response_{}.txt 失败: {}", idx, e); }

            // JSON 校验
            let parse_result = json_repair::validate_and_repair_json(&result.content);
            match parse_result {
                Ok(chunk_json) => {
                    let entity_count = chunk_json.get("entities").and_then(|e| e.as_array()).map(|a| a.len()).unwrap_or(0);
                    let concept_count = chunk_json.get("concepts").and_then(|e| e.as_array()).map(|a| a.len()).unwrap_or(0);
                    self.task_queue.add_event(task_id, "json_parsed", "SourceIngestAgent",
                        &format!("解析成功{}: {} 实体, {} 概念", chunk_label, entity_count, concept_count))?;
                    // 合并结果
                    if let Some(entities) = chunk_json.get("entities").and_then(|e| e.as_array()) {
                        if let Some(final_entities) = final_json.get_mut("entities").and_then(|e| e.as_array_mut()) {
                            for entity in entities {
                                final_entities.push(entity.clone());
                            }
                        }
                    }
                    if let Some(concepts) = chunk_json.get("concepts").and_then(|e| e.as_array()) {
                        if let Some(final_concepts) = final_json.get_mut("concepts").and_then(|e| e.as_array_mut()) {
                            for concept in concepts {
                                final_concepts.push(concept.clone());
                            }
                        }
                    }
                    if let Some(topics) = chunk_json.get("topics").and_then(|e| e.as_array()) {
                        if let Some(final_topics) = final_json.get_mut("topics").and_then(|e| e.as_array_mut()) {
                            for topic in topics {
                                final_topics.push(topic.clone());
                            }
                        }
                    }
                    // 使用第一段的摘要
                    if idx == 0 {
                        if let Some(summary) = chunk_json.get("source_summary") {
                            final_json["source_summary"] = summary.clone();
                        }
                    }
                }
                Err(e) => {
                    let preview = &result.content[..result.content.len().min(500)];
                    self.task_queue.add_event(task_id, "chunk_error", "SourceIngestAgent",
                        &format!("第 {} 段 JSON 解析失败: {}。原始响应前500字符: {}", idx + 1, e, preview))?;
                }
            }
        }

        // 合并后去重（简单实现：按名称去重）
        {
            let mut seen_names = std::collections::HashSet::new();
            if let Some(arr) = final_json.get_mut("entities").and_then(|e| e.as_array_mut()) {
                arr.retain(|e| {
                    if let Some(name) = e.get("name").and_then(|n| n.as_str()) {
                        seen_names.insert(name.to_string())
                    } else {
                        true
                    }
                });
            }
        }

        // 8. Schema 验证
        self.event_bus.emit_agent_activity(
            "SourceIngestAgent",
            "json_validating",
            &self.source_file_name,
            "正在校验合并后的 JSON",
        );
        self.event_bus.emit_agent_status_change(
            task_id, "json_validated", "running", 0.8,
            "", "", "正在校验合并后的 JSON",
        );
        self.task_queue.update_task_status(task_id, "json_validating", "SourceIngestAgent", "")?;
        self.task_queue.add_event(task_id, "validating", "SchemaValidator", "正在校验合并后的 JSON")?;

        let validation = crate::schema::json_schema_validator::validate_ingest_result(&final_json);
        let warnings_str = validation.warnings.join("; ");
        if !warnings_str.is_empty() {
            self.task_queue.add_event(task_id, "schema_warnings", "SchemaValidator", &warnings_str)?;
        }

        // 9. 保存结果
        let result_json = serde_json::to_string_pretty(&final_json)
            .map_err(|e| format!("序列化 ingest_result.json 失败: {}", e))?;
        if let Err(e) = std::fs::write(task_dir.join("ingest_result.json"), &result_json) { log::error!("[source_ingest] 写入 ingest_result.json 失败: {}", e); }

        // 10. 计算 AI 摘要文本（暂不写入 DB，等 knowledge_items 保存成功后再更新状态）
        let ai_summary: Option<String> = final_json.get("source_summary").map(|summary| {
            let title = summary.get("title").and_then(|v| v.as_str()).unwrap_or("");
            let short = summary.get("short_summary").and_then(|v| v.as_str()).unwrap_or("");
            if short.is_empty() { title.to_string() } else { format!("{}: {}", title, short) }
        });

        // 13. 保存实体/概念到 knowledge_items（事务保护，全部成功或全部回滚）
        self.save_knowledge_items(kb_id, source_id, task_id, &final_json)?;

        // 14. knowledge_items 保存成功后，更新 source 状态为 analyzed
        let now = chrono::Utc::now().to_rfc3339();
        if let Some(ref summary) = ai_summary {
            if let Err(e) = conn.execute(
                "UPDATE sources SET status = 'analyzed', ai_summary = ?2, updated_at = ?3 WHERE id = ?1",
                rusqlite::params![source_id, summary, now],
            ) {
                log::error!("[source_ingest] 更新 source 状态为 analyzed 失败 (source={}): {}", source_id, e);
            }
        } else {
            if let Err(e) = conn.execute(
                "UPDATE sources SET status = 'analyzed', updated_at = ?1 WHERE id = ?2",
                rusqlite::params![now, source_id],
            ) {
                log::error!("[source_ingest] 更新 source 状态为 analyzed 失败 (source={}): {}", source_id, e);
            }
        }
        self.event_bus.emit_source_updated(kb_id, source_id);
        self.event_bus.emit_kb_stats_changed(kb_id);

        // 15. 候选检索
        self.event_bus.emit_agent_activity(
            "SourceIngestAgent",
            "candidate_searching",
            &self.source_file_name,
            "正在进行候选检索",
        );
        self.event_bus.emit_agent_status_change(
            task_id, "resolution_done", "running", 0.85,
            "", "", "正在进行候选检索",
        );
        self.task_queue.update_task_status(task_id, "candidate_searching", "SourceIngestAgent", "")?;

        self.event_bus.emit_agent_activity(
            "SourceIngestAgent",
            "completed",
            &self.source_file_name,
            "文档分析完成，等待后续处理",
        );
        self.event_bus.emit_agent_status_change(
            task_id, "json_validated", "done", 1.0,
            "", "", "文档分析阶段完成",
        );

        Ok(())
    }

    fn get_existing_pages(&self, kb_id: &str) -> Result<String, String> {
        let conn = self.db.connect()?;
        let mut stmt = conn
            .prepare("SELECT canonical_name, item_type, COALESCE(summary, '') FROM knowledge_items WHERE kb_id = ?1")
            .map_err(|e| format!("准备查询失败: {}", e))?;

        let mut items = Vec::new();
        let mut rows = stmt.query(rusqlite::params![kb_id])
            .map_err(|e| format!("查询失败: {}", e))?;
        while let Some(row) = rows.next().map_err(|e| format!("读取行失败: {}", e))? {
            let name: String = row.get(0).map_err(|e| format!("获取字段失败: {}", e))?;
            let itype: String = row.get(1).map_err(|e| format!("获取字段失败: {}", e))?;
            let summary: String = row.get(2).map_err(|e| format!("获取字段失败: {}", e))?;
            items.push(format!("- [{}] {}: {}", itype, name, summary));
        }

        Ok(items.join("\n"))
    }

    fn save_knowledge_items(&self, kb_id: &str, source_id: &str, task_id: &str, json: &serde_json::Value) -> Result<(), String> {
        let conn = self.db.connect()?;
        let now = chrono::Utc::now().to_rfc3339();

        // 事务保护：全部 knowledge_items 插入成功或全部回滚
        conn.execute("BEGIN TRANSACTION", [])
            .map_err(|e| format!("开始事务失败: {}", e))?;

        // v0.2.1: 插入前去重检查，避免重复 knowledge_items
        let mut saved_count = 0u32;
        let mut skipped_duplicate = 0u32;

        let mut try_insert = |name: &str, desc: &str, item_type: &str| -> Result<(), String> {
            if name.trim().is_empty() { return Ok(()); }
            // 查重
            let norm = crate::dedup::dedup_service::DedupService::normalize_name(name);
            let exists: bool = match conn.query_row(
                "SELECT COUNT(*) FROM knowledge_items WHERE kb_id = ?1 AND (normalized_name = ?2 OR canonical_name = ?3) LIMIT 1",
                rusqlite::params![kb_id, norm, name],
                |row| row.get::<_, i64>(0),
            ) {
                Ok(c) => c > 0,
                Err(rusqlite::Error::QueryReturnedNoRows) => false,
                Err(e) => return Err(format!("查重知识项失败(name={}): {}", name, e)),
            };

            if exists {
                skipped_duplicate += 1;
                return Ok(());
            }

            let id = uuid::Uuid::new_v4().to_string();
            conn.execute(
                "INSERT INTO knowledge_items (id, kb_id, canonical_name, normalized_name, item_type, source_id, summary, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8)",
                rusqlite::params![id, kb_id, name, norm, item_type, source_id, desc, now],
            ).map_err(|e| format!("保存知识项失败: {}", e))?;
            saved_count += 1;
            Ok(())
        };

        // 保存 entities
        let result = (|| -> Result<(), String> {
            if let Some(arr) = json.get("entities").and_then(|a| a.as_array()) {
                for entity in arr {
                    let name = entity.get("name").and_then(|v| v.as_str()).unwrap_or("");
                    let desc = entity.get("description").and_then(|v| v.as_str()).unwrap_or("");
                    try_insert(name, desc, "entity")?;
                }
            }

            // 保存 concepts
            if let Some(arr) = json.get("concepts").and_then(|a| a.as_array()) {
                for concept in arr {
                    let name = concept.get("name").and_then(|v| v.as_str()).unwrap_or("");
                    let definition = concept.get("definition").and_then(|v| v.as_str()).unwrap_or("");
                    try_insert(name, definition, "concept")?;
                }
            }

            // 保存 topics
            if let Some(arr) = json.get("topics").and_then(|a| a.as_array()) {
                for topic in arr {
                    let name = topic.get("name").and_then(|v| v.as_str()).unwrap_or("");
                    let desc = topic.get("description").and_then(|v| v.as_str()).unwrap_or("");
                    try_insert(name, desc, "topic")?;
                }
            }
            Ok(())
        })();

        match result {
            Ok(()) => {
                conn.execute("COMMIT", [])
                    .map_err(|e| format!("提交事务失败: {}", e))?;
            }
            Err(e) => {
                let _ = conn.execute("ROLLBACK", []);
                return Err(e);
            }
        }

        if skipped_duplicate > 0 {
            self.task_queue.add_event(task_id, "dedup", "SourceIngestAgent",
                &format!("去重: 保存了 {} 个新知识项，跳过了 {} 个已有项", saved_count, skipped_duplicate))?;
        }

        Ok(())
    }
}
