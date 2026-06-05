use std::sync::Arc;
use crate::core::task_queue::TaskQueue;
use crate::core::event_bus::EventBus;
use crate::core::database_service::DatabaseService;
use crate::core::config_service::ConfigService;
use crate::core::secret_service::SecretService;
use crate::core::workspace_service::WorkspaceService;
use crate::core::token_logger::{TokenLogger, TokenContext};
use crate::model::model_gateway::ModelGateway;
use crate::schema::json_repair;

use crate::core::task_queue::CancellationToken;
use crate::embedding::vdb_service::VdbService;

pub struct CoordinatorAgent {
    task_queue: Arc<TaskQueue>,
    db: Arc<DatabaseService>,
    config: Arc<ConfigService>,
    workspace: Arc<WorkspaceService>,
    event_bus: Arc<EventBus>,
    model_gateway: Arc<ModelGateway>,
    token_logger: Arc<TokenLogger>,
    vdb: Arc<VdbService>,
}

impl CoordinatorAgent {
    pub fn new(
        task_queue: Arc<TaskQueue>,
        db: Arc<DatabaseService>,
        config: Arc<ConfigService>,
        secrets: Arc<SecretService>,
        workspace: Arc<WorkspaceService>,
        event_bus: Arc<EventBus>,
        token_logger: Arc<TokenLogger>,
        vdb: Arc<VdbService>,
    ) -> Self {
        let model_gateway = Arc::new(
            ModelGateway::new(secrets.clone()).with_token_logger(token_logger.clone())
        );
        Self {
            task_queue,
            db,
            config,
            workspace,
            event_bus,
            model_gateway,
            token_logger,
            vdb,
        }
    }

    /// 构建 TokenContext（用于 API 调用消耗记录）
    fn build_token_ctx(&self, task_id: &str, task_name: &str, agent_name: &str) -> TokenContext {
        let config = self.config.get_provider_config().unwrap_or_default();
        TokenContext {
            task_id: task_id.to_string(),
            task_name: task_name.to_string(),
            agent_name: agent_name.to_string(),
            model_name: config.chat_model,
            provider: config.provider,
        }
    }

    /// 根据 task_id 查询关联的源文件名
    fn get_source_file_name(&self, task_id: &str) -> Option<String> {
        let conn = self.db.connect().ok()?;
        conn.query_row(
            "SELECT s.file_name FROM sources s INNER JOIN tasks t ON t.input_ref = s.id WHERE t.id = ?1",
            rusqlite::params![task_id],
            |row| row.get(0),
        ).ok()
    }

    /// 熔断检查：每日 Token 配额
    fn check_quota(&self, kb_id: &str) -> Result<(), String> {
        let status = self.token_logger.check_quota()?;
        if !status.allowed {
            self.event_bus.emit_notification("warning", "Token 额度告警", &status.message);
            self.event_bus.emit_agent_activity(
                "TokenQuotaGuard",
                "quota_exceeded",
                kb_id,
                &status.message,
            );
            return Err(status.message);
        }
        Ok(())
    }

    /// 启动一个 Source Ingest 任务
    pub async fn run_source_ingest(
        &self,
        kb_id: &str,
        kb_path: &str,
        source_id: &str,
    ) -> Result<String, String> {
        // 熔断检查
        self.check_quota(kb_id)?;

        let task = self.task_queue.create_task(kb_id, "source_ingest", source_id)?;
        let task_id = task.id.clone();

        let task_clone = task_id.clone();
        let kb_id_owned = kb_id.to_string();
        let kb_path_owned = kb_path.to_string();
        let source_id_owned = source_id.to_string();

        let tq = self.task_queue.clone();
        let db = self.db.clone();
        let config = self.config.clone();
        let ws = self.workspace.clone();
        let mg = self.model_gateway.clone();
        let eb = self.event_bus.clone();
        let vdb = self.vdb.clone();
        let kb_path_for_chain = kb_path.to_string();
        let task_id_for_chain = task_id.clone();

        tokio::spawn(async move {
            // v0.2.1: 创建取消令牌
            let cancel_token = tq.create_cancellation_token(&task_clone);

            let file_name = match db.connect() {
                Ok(conn) => match conn.query_row(
                    "SELECT file_name FROM sources WHERE id = ?1",
                    rusqlite::params![source_id_owned],
                    |row| row.get::<_, String>(0),
                ) {
                    Ok(name) => name,
                    Err(rusqlite::Error::QueryReturnedNoRows) => "未知文件".to_string(),
                    Err(e) => {
                        log::error!("[coordinator] 查询 source file_name 失败 (source={}): {}", source_id_owned, e);
                        "未知文件".to_string()
                    }
                },
                Err(e) => {
                    log::error!("[coordinator] DB 连接失败 (task={}): {}", task_clone, e);
                    "未知文件".to_string()
                }
            };

            // v0.2.2: 检查取消（内存令牌 + DB 双重检查）
            if cancel_token.is_cancelled() || tq.is_task_cancelled(&task_clone) {
                if let Err(e) = tq.mark_cancelled(&task_clone) {
                    log::error!("[coordinator] mark_cancelled 失败 (task={}): {}", task_clone, e);
                }
                // 将 source 标记为 cancelled（在 agent 启动前取消）
                if let Ok(conn) = db.connect() {
                    if let Err(e) = conn.execute(
                        "UPDATE sources SET status = 'cancelled', updated_at = ?1 WHERE id = ?2",
                        rusqlite::params![chrono::Utc::now().to_rfc3339(), source_id_owned],
                    ) {
                        log::error!("[coordinator] 更新 source 状态为 cancelled 失败 (source={}): {}", source_id_owned, e);
                    }
                }
                eb.emit_source_updated(&kb_id_owned, &source_id_owned);
                eb.emit_kb_stats_changed(&kb_id_owned);
                return;
            }

            let agent = crate::agents::source_ingest::SourceIngestAgent::new(
                tq.clone(), db.clone(), config.clone(), eb.clone(), ws.clone(), mg.clone(), file_name,
            );

            let result = agent.execute(
                &kb_id_owned,
                &kb_path_owned,
                &source_id_owned,
                &task_clone,
                &cancel_token,
            ).await;

            match result {
                Ok(_) => {
                    // v0.2.2: 检查取消（内存令牌 + DB 双重检查）
                    if cancel_token.is_cancelled() || tq.is_task_cancelled(&task_clone) {
                        if let Err(e) = tq.mark_cancelled(&task_clone) {
                            log::error!("[coordinator] mark_cancelled 失败 (task={}): {}", task_clone, e);
                        }
                        // 将 source 标记为 cancelled（agent 完成后取消）
                        if let Ok(conn) = db.connect() {
                            if let Err(e) = conn.execute(
                                "UPDATE sources SET status = 'cancelled', updated_at = ?1 WHERE id = ?2",
                                rusqlite::params![chrono::Utc::now().to_rfc3339(), source_id_owned],
                            ) {
                                log::error!("[coordinator] 更新 source 状态为 cancelled 失败 (source={}): {}", source_id_owned, e);
                            }
                        }
                        eb.emit_source_updated(&kb_id_owned, &source_id_owned);
                        eb.emit_kb_stats_changed(&kb_id_owned);
                        return;
                    }

                    if let Err(e) = tq.update_task_status(&task_clone, "review_pending", "SourceIngestAgent", "") {
                        log::error!("[coordinator] 更新 task 状态为 review_pending 失败 (task={}): {}", task_clone, e);
                    }
                    // Auto-index source text into vector DB after extraction
                    if let Err(e) = vdb.index_source(&kb_id_owned, &source_id_owned) {
                        log::warn!("[coordinator] 自动索引失败 (source={}): {}", source_id_owned, e);
                    }
                    // Auto-sync graph nodes from knowledge_items
                    if let Err(e) = crate::graph::graph_service::GraphService::sync_from_knowledge_items(&db, &kb_id_owned) {
                        log::warn!("[coordinator] 自动图谱同步失败: {}", e);
                    }
                    eb.emit_agent_activity(
                        "PipelineCoordinator",
                        "pipeline_starting",
                        "",
                        "开始执行后续处理流水线",
                    );
                    if let Err(e) = Self::run_full_pipeline(
                        &tq, &db, &config, &ws, &mg, &eb,
                        &kb_id_owned, &kb_path_for_chain, &task_id_for_chain, &source_id_owned, &cancel_token,
                    ).await {
                        if cancel_token.is_cancelled() || tq.is_task_cancelled(&task_clone) {
                            if let Err(e) = tq.mark_cancelled(&task_clone) {
                                log::error!("[coordinator] mark_cancelled 失败 (task={}): {}", task_clone, e);
                            }
                            // 将 source 标记为 cancelled（流水线执行中取消）
                            if let Ok(conn) = db.connect() {
                                if let Err(e) = conn.execute(
                                    "UPDATE sources SET status = 'cancelled', updated_at = ?1 WHERE id = ?2",
                                    rusqlite::params![chrono::Utc::now().to_rfc3339(), source_id_owned],
                                ) {
                                    log::error!("[coordinator] 更新 source 状态为 cancelled 失败 (source={}): {}", source_id_owned, e);
                                }
                            }
                            eb.emit_source_updated(&kb_id_owned, &source_id_owned);
                            eb.emit_kb_stats_changed(&kb_id_owned);
                            return;
                        }
                        if let Err(e) = tq.update_task_status(&task_clone, "pipeline_failed", "PipelineCoordinator", &e) {
                            log::error!("[coordinator] 更新 task 状态为 pipeline_failed 失败 (task={}): {}", task_clone, e);
                        }
                        // 更新 source 状态为 pipeline_failed
                        if let Ok(conn) = db.connect() {
                            if let Err(e) = conn.execute(
                                "UPDATE sources SET status = 'pipeline_failed', updated_at = ?1 WHERE id = ?2",
                                rusqlite::params![chrono::Utc::now().to_rfc3339(), source_id_owned],
                            ) {
                                log::error!("[coordinator] 更新 source 状态为 pipeline_failed 失败 (source={}): {}", source_id_owned, e);
                            }
                        }
                        eb.emit_source_updated(&kb_id_owned, &source_id_owned);
                        eb.emit_kb_stats_changed(&kb_id_owned);
                        eb.emit_agent_activity(
                            "PipelineCoordinator",
                            "pipeline_failed",
                            "",
                            &format!("流水线执行失败: {}", e),
                        );
                    }
                }
                Err(e) => {
                    if cancel_token.is_cancelled() || tq.is_task_cancelled(&task_clone) {
                        if let Err(e) = tq.mark_cancelled(&task_clone) {
                            log::error!("[coordinator] mark_cancelled 失败 (task={}): {}", task_clone, e);
                        }
                        // 将 source 标记为 cancelled（agent 执行错误后取消）
                        if let Ok(conn) = db.connect() {
                            if let Err(e) = conn.execute(
                                "UPDATE sources SET status = 'cancelled', updated_at = ?1 WHERE id = ?2",
                                rusqlite::params![chrono::Utc::now().to_rfc3339(), source_id_owned],
                            ) {
                                log::error!("[coordinator] 更新 source 状态为 cancelled 失败 (source={}): {}", source_id_owned, e);
                            }
                        }
                        eb.emit_source_updated(&kb_id_owned, &source_id_owned);
                        eb.emit_kb_stats_changed(&kb_id_owned);
                        return;
                    }
                    if let Err(e) = tq.update_task_status(&task_clone, "failed", "SourceIngestAgent", &e) {
                        log::error!("[coordinator] 更新 task 状态为 failed 失败 (task={}): {}", task_clone, e);
                    }
                    // 更新 source 状态为 analysis_failed
                    if let Ok(conn) = db.connect() {
                        if let Err(e) = conn.execute(
                            "UPDATE sources SET status = 'analysis_failed', updated_at = ?1 WHERE id = ?2",
                            rusqlite::params![chrono::Utc::now().to_rfc3339(), source_id_owned],
                        ) {
                            log::error!("[coordinator] 更新 source 状态为 analysis_failed 失败 (source={}): {}", source_id_owned, e);
                        }
                    }
                    eb.emit_source_updated(&kb_id_owned, &source_id_owned);
                    eb.emit_kb_stats_changed(&kb_id_owned);
                }
            }
        });

        Ok(task_id)
    }

    /// 执行完整的处理流水线：Resolution -> Relationship -> WikiUpdate -> Review
    /// 带有断点续传：如果中间结果文件已存在，则跳过该步骤
    /// v0.2.1: 添加取消令牌检查
    /// v0.2.3: 添加 source_id 参数以更新 source 状态
    async fn run_full_pipeline(
        tq: &Arc<TaskQueue>,
        db: &Arc<DatabaseService>,
        config: &Arc<ConfigService>,
        ws: &Arc<WorkspaceService>,
        mg: &Arc<ModelGateway>,
        eb: &Arc<EventBus>,
        kb_id: &str,
        kb_path: &str,
        task_id: &str,
        source_id: &str,
        cancel_token: &CancellationToken,
    ) -> Result<(), String> {
        let tasks_dir = ws.get_tasks_dir(&std::path::PathBuf::from(kb_path));
        let task_dir = tasks_dir.join(task_id);
        std::fs::create_dir_all(&task_dir).map_err(|e| format!("创建任务目录失败: {}", e))?;

        // 查询源文件名（用于 Token 上下文记录）
        let source_file_name = db.connect().ok().and_then(|conn| {
            conn.query_row(
                "SELECT file_name FROM sources WHERE id = ?1",
                rusqlite::params![source_id],
                |row| row.get::<_, String>(0),
            ).ok()
        }).unwrap_or_else(|| "未知文件".to_string());

        let provider_config = config.get_provider_config()?;
        let model_name = provider_config.chat_model.clone();
        let provider_name = provider_config.provider.clone();

        let ingest_result_path = task_dir.join("ingest_result.json");
        if !ingest_result_path.exists() {
            return Err("未找到 ingest_result.json，无法继续流水线".to_string());
        }
        let ingest_json_str = std::fs::read_to_string(&ingest_result_path)
            .map_err(|e| format!("读取 ingest_result.json 失败: {}", e))?;

        let ingest_json: serde_json::Value = serde_json::from_str(&ingest_json_str)
            .map_err(|e| format!("解析 ingest_result.json 失败: {}", e))?;

        let has_entities = ingest_json.get("entities").and_then(|e| e.as_array()).map(|a| !a.is_empty()).unwrap_or(false);
        let has_concepts = ingest_json.get("concepts").and_then(|c| c.as_array()).map(|a| !a.is_empty()).unwrap_or(false);

        if !has_entities && !has_concepts {
            eb.emit_agent_activity(
                "PipelineCoordinator",
                "pipeline_completed",
                "",
                "未发现新的知识实体，流水线终止",
            );
            let _ = tq.add_event(task_id, "pipeline_done", "PipelineCoordinator", "文档分析未发现新实体，无需后续处理");
            if let Err(e) = tq.update_task_status(task_id, "applied", "PipelineCoordinator", "") {
                log::error!("[coordinator] 更新 task 状态为 applied 失败 (task={}): {}", task_id, e);
            }
            // 更新 source 状态为 processed 并发出事件（与正常流水线完成路径一致）
            if let Ok(conn) = db.connect() {
                if let Err(e) = conn.execute(
                    "UPDATE sources SET status = 'processed', updated_at = ?1 WHERE id = ?2",
                    rusqlite::params![chrono::Utc::now().to_rfc3339(), source_id],
                ) {
                    log::error!("[coordinator] 更新 source 状态为 processed 失败 (source={}): {}", source_id, e);
                }
            }
            eb.emit_source_updated(kb_id, source_id);
            eb.emit_kb_stats_changed(kb_id);
            return Ok(());
        }

        // ====== 步骤1: Resolution (可断点续传) ======
        // v0.2.2: prompt_build 前检查取消（内存令牌 + DB）
        if cancel_token.is_cancelled() || tq.is_task_cancelled(task_id) {
            let _ = tq.add_event(task_id, "cancelled_before_stage", "PipelineCoordinator", "步骤1 Resolution 前检测到取消");
            let _ = tq.mark_cancelled(task_id);
            // 将 source 标记为 cancelled（中途取消）
            if let Ok(conn) = db.connect() {
                if let Err(e) = conn.execute(
                    "UPDATE sources SET status = 'cancelled', updated_at = ?1 WHERE id = ?2",
                    rusqlite::params![chrono::Utc::now().to_rfc3339(), source_id],
                ) {
                    log::error!("[coordinator] 更新 source 状态为 cancelled 失败 (source={}): {}", source_id, e);
                }
            }
            eb.emit_source_updated(kb_id, source_id);
            eb.emit_kb_stats_changed(kb_id);
            return Ok(());
        }

        let resolution_result_path = task_dir.join("resolution_result.json");
        let resolution_json = if resolution_result_path.exists() {
            eb.emit_agent_activity(
                "PipelineCoordinator",
                "resolution_skip",
                "",
                "使用已有的消歧结果，跳过该步骤",
            );
            let _ = tq.add_event(task_id, "step_skip", "PipelineCoordinator", "Resolution 步骤已存在，跳过");
            std::fs::read_to_string(&resolution_result_path)
                .map_err(|e| format!("读取 resolution_result.json 失败: {}", e))?
        } else {
            eb.emit_agent_activity(
                "PipelineCoordinator",
                "resolution_running",
                "",
                "正在执行消歧处理",
            );
            eb.emit_agent_status_change(
                task_id, "resolution_done", "running", 0.4,
                "", "", "正在执行消歧处理",
            );
            if let Err(e) = tq.update_task_status(task_id, "resolution_running", "PipelineCoordinator", "") {
                log::error!("[coordinator] 更新 task 状态为 resolution_running 失败 (task={}): {}", task_id, e);
            }

            let candidates = crate::search::candidate_search::CandidateSearchEngine::search(
                db, kb_id, &ingest_json_str, kb_path,
            )?;

            let candidates_json = serde_json::to_string(&candidates)
                .map_err(|e| format!("序列化候选实体列表失败: {}", e))?;
            let (system_prompt, user_message) = crate::prompts::prompt_builder::PromptBuilder::build_resolution_prompt(
                &ingest_json_str,
                &candidates_json,
            );

            // v0.2.2: send_to_model 前检查取消（内存令牌 + DB）
            if cancel_token.is_cancelled() || tq.is_task_cancelled(task_id) {
                let _ = tq.add_event(task_id, "cancelled_before_model", "PipelineCoordinator", "调用模型前检测到取消");
                let _ = tq.mark_cancelled(task_id);
                // 将 source 标记为 cancelled（中途取消）
                if let Ok(conn) = db.connect() {
                    if let Err(e) = conn.execute(
                        "UPDATE sources SET status = 'cancelled', updated_at = ?1 WHERE id = ?2",
                        rusqlite::params![chrono::Utc::now().to_rfc3339(), source_id],
                    ) {
                        log::error!("[coordinator] 更新 source 状态为 cancelled 失败 (source={}): {}", source_id, e);
                    }
                }
                eb.emit_source_updated(kb_id, source_id);
                eb.emit_kb_stats_changed(kb_id);
                return Ok(());
            }

            // 通知前端：消歧模型调用中，透传完整 Prompt
            let res_full_prompt = format!("{}\n\n---\n\n{}", system_prompt, user_message);
            eb.emit_agent_status_change(
                task_id, "resolution_done", "running", 0.45,
                &res_full_prompt, "", "正在调用 LLM 执行消歧处理",
            );

            let token_ctx = TokenContext {
                task_id: task_id.to_string(),
                task_name: source_file_name.clone(),
                agent_name: "ResolutionAgent".to_string(),
                model_name: model_name.clone(),
                provider: provider_name.clone(),
            };
            let result = mg.chat_with_content_and_ctx(
                &provider_config,
                &system_prompt,
                &user_message,
                true,
                Some(token_ctx),
            ).await?;

            // v0.2.2: model_returned 后检查取消（内存令牌 + DB）
            if cancel_token.is_cancelled() || tq.is_task_cancelled(task_id) {
                // 保存模型返回但丢弃
                if let Err(e) = std::fs::write(&resolution_result_path, &result.content) { log::error!("[coordinator] 写入 resolution_result.json 失败: {}", e); }
                if let Err(e) = tq.mark_cancelled_after_model(task_id) {
                    log::error!("[coordinator] mark_cancelled_after_model 失败 (task={}): {}", task_id, e);
                }
                eb.emit_kb_stats_changed(kb_id);
                return Ok(());
            }

            if let Err(e) = std::fs::write(&resolution_result_path, &result.content) { log::error!("[coordinator] 写入 resolution_result.json 失败: {}", e); }
            let json = json_repair::validate_and_repair_json(&result.content)?;
            crate::schema::json_schema_validator::validate_resolution_result(&json);

            // Save the REPAIRED JSON (not raw LLM output) so downstream steps get clean data
            let repaired_content = serde_json::to_string(&json).unwrap_or_else(|_| result.content.clone());
            if repaired_content != result.content {
                if let Err(e) = std::fs::write(&resolution_result_path, &repaired_content) {
                    log::error!("[coordinator] 写入修复后的 resolution_result.json 失败: {}", e);
                }
            }

            if let Err(e) = tq.update_output_ref(task_id, "resolution_result.json") {
                log::error!("[coordinator] update_output_ref 失败 (task={}): {}", task_id, e);
            }
            repaired_content
        };

        // ====== 步骤2: Relationship (可断点续传) ======
        // v0.2.2: relationship_running 前检查取消（内存令牌 + DB）
        if cancel_token.is_cancelled() || tq.is_task_cancelled(task_id) {
            let _ = tq.add_event(task_id, "cancelled_before_stage", "PipelineCoordinator", "步骤2 Relationship 前检测到取消");
            let _ = tq.mark_cancelled(task_id);
            // 将 source 标记为 cancelled（中途取消）
            if let Ok(conn) = db.connect() {
                if let Err(e) = conn.execute(
                    "UPDATE sources SET status = 'cancelled', updated_at = ?1 WHERE id = ?2",
                    rusqlite::params![chrono::Utc::now().to_rfc3339(), source_id],
                ) {
                    log::error!("[coordinator] 更新 source 状态为 cancelled 失败 (source={}): {}", source_id, e);
                }
            }
            eb.emit_source_updated(kb_id, source_id);
            eb.emit_kb_stats_changed(kb_id);
            return Ok(());
        }

        let relationship_result_path = task_dir.join("relationship_result.json");
        let rel_json = if relationship_result_path.exists() {
            eb.emit_agent_activity(
                "PipelineCoordinator",
                "relationship_skip",
                "",
                "使用已有的关系结果，跳过该步骤",
            );
            let _ = tq.add_event(task_id, "step_skip", "PipelineCoordinator", "Relationship 步骤已存在，跳过");
            std::fs::read_to_string(&relationship_result_path)
                .map_err(|e| format!("读取 relationship_result.json 失败: {}", e))?
        } else {
            eb.emit_agent_activity(
                "PipelineCoordinator",
                "relationship_running",
                "",
                "正在执行关系标准化",
            );
            eb.emit_agent_status_change(
                task_id, "resolution_done", "done", 0.55,
                "", "", "消歧完成，正在执行关系标准化",
            );
            if let Err(e) = tq.update_task_status(task_id, "relationship_running", "PipelineCoordinator", "") {
                log::error!("[coordinator] 更新 task 状态为 relationship_running 失败 (task={}): {}", task_id, e);
            }

            let (sys_prompt, user_msg) = crate::prompts::prompt_builder::PromptBuilder::build_relationship_prompt(
                &resolution_json,
            );

            // v0.2.2: send_to_model 前检查取消（内存令牌 + DB）
            if cancel_token.is_cancelled() || tq.is_task_cancelled(task_id) {
                let _ = tq.add_event(task_id, "cancelled_before_model", "PipelineCoordinator", "关系分析调用模型前检测到取消");
                let _ = tq.mark_cancelled(task_id);
                // 将 source 标记为 cancelled（中途取消）
                if let Ok(conn) = db.connect() {
                    if let Err(e) = conn.execute(
                        "UPDATE sources SET status = 'cancelled', updated_at = ?1 WHERE id = ?2",
                        rusqlite::params![chrono::Utc::now().to_rfc3339(), source_id],
                    ) {
                        log::error!("[coordinator] 更新 source 状态为 cancelled 失败 (source={}): {}", source_id, e);
                    }
                }
                eb.emit_source_updated(kb_id, source_id);
                eb.emit_kb_stats_changed(kb_id);
                return Ok(());
            }

            // 通知前端：关系分析模型调用中，透传完整 Prompt
            let rel_full_prompt = format!("{}\n\n---\n\n{}", sys_prompt, user_msg);
            eb.emit_agent_status_change(
                task_id, "resolution_done", "done", 0.6,
                &rel_full_prompt, "", "消歧完成，正在调用 LLM 执行关系标准化",
            );

            let rel_token_ctx = TokenContext {
                task_id: task_id.to_string(),
                task_name: source_file_name.clone(),
                agent_name: "RelationshipAgent".to_string(),
                model_name: model_name.clone(),
                provider: provider_name.clone(),
            };
            let rel_result = mg.chat_with_content_and_ctx(
                &provider_config,
                &sys_prompt,
                &user_msg,
                true,
                Some(rel_token_ctx),
            ).await?;

            // v0.2.2: model_returned 后检查取消（内存令牌 + DB）
            if cancel_token.is_cancelled() || tq.is_task_cancelled(task_id) {
                if let Err(e) = std::fs::write(&relationship_result_path, &rel_result.content) { log::error!("[coordinator] 写入 relationship_result.json 失败: {}", e); }
                if let Err(e) = tq.mark_cancelled_after_model(task_id) {
                    log::error!("[coordinator] mark_cancelled_after_model 失败 (task={}): {}", task_id, e);
                }
                eb.emit_kb_stats_changed(kb_id);
                return Ok(());
            }

            if let Err(e) = std::fs::write(&relationship_result_path, &rel_result.content) { log::error!("[coordinator] 写入 relationship_result.json 失败: {}", e); }

            // 尝试解析并保存关系；失败则用空结果继续
            let rel_content = match json_repair::validate_and_repair_json(&rel_result.content) {
                Ok(json) => {
                    if let Err(e) = crate::agents::relationship::RelationshipAgent::save_relationships(
                        db, kb_id, &json,
                    ) {
                        log::error!("[PipelineCoordinator] 保存关系失败: {}", e);
                        let _ = tq.add_event(task_id, "relationship_warning", "PipelineCoordinator", &format!("保存关系失败: {}", e));
                    }
                    rel_result.content.clone()
                }
                Err(e) => {
                    log::error!("[PipelineCoordinator] 关系JSON解析失败: {}", e);
                    let _ = tq.add_event(task_id, "relationship_warning", "PipelineCoordinator", &format!("关系JSON解析失败: {}，继续后续流程", e));
                    let fallback = r#"{"relationships":[]}"#.to_string();
                    if let Err(e) = std::fs::write(&relationship_result_path, &fallback) { log::error!("[coordinator] 写入 relationship fallback 失败: {}", e); }
                    fallback
                }
            };

            if let Err(e) = tq.update_output_ref(task_id, "relationship_result.json") {
                log::error!("[coordinator] update_output_ref 失败 (task={}): {}", task_id, e);
            }
            rel_content
        };

        // ====== 步骤3: WikiUpdate (可断点续传) ======
        // v0.2.2: update_plan_generating 前检查取消（内存令牌 + DB）
        if cancel_token.is_cancelled() || tq.is_task_cancelled(task_id) {
            let _ = tq.add_event(task_id, "cancelled_before_stage", "PipelineCoordinator", "步骤3 WikiUpdate 前检测到取消");
            let _ = tq.mark_cancelled(task_id);
            // 将 source 标记为 cancelled（中途取消）
            if let Ok(conn) = db.connect() {
                if let Err(e) = conn.execute(
                    "UPDATE sources SET status = 'cancelled', updated_at = ?1 WHERE id = ?2",
                    rusqlite::params![chrono::Utc::now().to_rfc3339(), source_id],
                ) {
                    log::error!("[coordinator] 更新 source 状态为 cancelled 失败 (source={}): {}", source_id, e);
                }
            }
            eb.emit_source_updated(kb_id, source_id);
            eb.emit_kb_stats_changed(kb_id);
            return Ok(());
        }

        let update_plan_path = task_dir.join("update_plan.json");
        if update_plan_path.exists() {
            // v0.2.2: 跳过路径也检查取消
            if cancel_token.is_cancelled() || tq.is_task_cancelled(task_id) {
                let _ = tq.add_event(task_id, "cancelled_before_review", "PipelineCoordinator", "WikiUpdate 跳过路径，检测到取消，不生成审阅");
                if let Err(e) = tq.mark_cancelled(task_id) {
                    log::error!("[coordinator] mark_cancelled 失败 (task={}): {}", task_id, e);
                }
                // 将 source 标记为 cancelled（WikiUpdate 跳过路径取消）
                if let Ok(conn) = db.connect() {
                    if let Err(e) = conn.execute(
                        "UPDATE sources SET status = 'cancelled', updated_at = ?1 WHERE id = ?2",
                        rusqlite::params![chrono::Utc::now().to_rfc3339(), source_id],
                    ) {
                        log::error!("[coordinator] 更新 source 状态为 cancelled 失败 (source={}): {}", source_id, e);
                    }
                }
                eb.emit_source_updated(kb_id, source_id);
                eb.emit_kb_stats_changed(kb_id);
                return Ok(());
            }

            eb.emit_agent_activity(
                "PipelineCoordinator",
                "update_skip",
                "",
                "使用已有的更新计划，跳过该步骤",
            );
            let _ = tq.add_event(task_id, "step_skip", "PipelineCoordinator", "WikiUpdate 步骤已存在，跳过");

            // 断点续传：检查审阅是否已生成，缺失则补生成
            let existing_review: Option<String> = {
                let conn = match db.connect() {
                    Ok(c) => c,
                    Err(e) => {
                        log::error!("[coordinator] WikiUpdate DB 连接失败 (task={}): {}", task_id, e);
                        return Err(format!("数据库连接失败: {}", e));
                    }
                };
                match conn.query_row(
                    "SELECT id FROM reviews WHERE task_id = ?1",
                    rusqlite::params![task_id],
                    |row| row.get(0),
                ) {
                    Ok(id) => Some(id),
                    Err(rusqlite::Error::QueryReturnedNoRows) => None,
                    Err(e) => {
                        log::error!("[coordinator] 查询 reviews 失败 (task={}): {}", task_id, e);
                        return Err(format!("查询审阅记录失败: {}", e));
                    }
                }
            };

            if existing_review.is_none() {
                // v0.2.2: 补生成前再次检查取消
                if cancel_token.is_cancelled() || tq.is_task_cancelled(task_id) {
                    let _ = tq.add_event(task_id, "cancelled_before_review", "PipelineCoordinator", "补生成审阅前检测到取消");
                    if let Err(e) = tq.mark_cancelled(task_id) {
                        log::error!("[coordinator] mark_cancelled 失败 (task={}): {}", task_id, e);
                    }
                    // 将 source 标记为 cancelled（补生成审阅前检测到取消）
                    if let Ok(conn) = db.connect() {
                        if let Err(e) = conn.execute(
                            "UPDATE sources SET status = 'cancelled', updated_at = ?1 WHERE id = ?2",
                            rusqlite::params![chrono::Utc::now().to_rfc3339(), source_id],
                        ) {
                            log::error!("[coordinator] 更新 source 状态为 cancelled 失败 (source={}): {}", source_id, e);
                        }
                    }
                    eb.emit_source_updated(kb_id, source_id);
                    eb.emit_kb_stats_changed(kb_id);
                    return Ok(());
                }
                eb.emit_agent_activity(
                    "PipelineCoordinator",
                    "review_regenerating",
                    "",
                    "检测到缺少审阅记录，补生成审阅",
                );
                let update_data = std::fs::read_to_string(&update_plan_path)
                    .map_err(|e| format!("读取 update_plan.json 失败: {}", e))?;
                let update_json: serde_json::Value = serde_json::from_str(&update_data)
                    .map_err(|e| format!("解析 update_plan.json 失败: {}", e))?;

                if let Err(e) = crate::review::review_engine::ReviewEngine::generate_review(
                    db, kb_id, task_id, &update_json, kb_path,
                ) {
                    log::error!("[PipelineCoordinator] 补生成审阅失败: {}", e);
                }
            }
        } else {
            eb.emit_agent_activity(
                "PipelineCoordinator",
                "update_plan_generating",
                "",
                "正在生成 Wiki 更新计划",
            );
            eb.emit_agent_status_change(
                task_id, "update_plan_done", "running", 0.7,
                "", "", "正在生成 Wiki 更新计划",
            );
            if let Err(e) = tq.update_task_status(task_id, "update_plan_generating", "PipelineCoordinator", "") {
                log::error!("[coordinator] 更新 task 状态为 update_plan_generating 失败 (task={}): {}", task_id, e);
            }

            let existing_content = {
                let conn = db.connect().map_err(|e| format!("连接数据库失败: {}", e))?;
                let mut stmt = conn.prepare("SELECT title, canonical_name, page_type, path FROM wiki_pages WHERE kb_id = ?1").map_err(|e| format!("准备查询失败: {}", e))?;
                let mut pages = Vec::new();
                let mut rows = stmt.query(rusqlite::params![kb_id]).map_err(|e| format!("查询失败: {}", e))?;
                while let Some(row) = rows.next().map_err(|e| format!("读取行失败: {}", e))? {
                    let page_type: String = row.get(2).map_err(|e| format!("获取字段失败: {}", e))?;
                    let title: String = row.get(0).map_err(|e| format!("获取字段失败: {}", e))?;
                    let canonical_name: String = row.get(1).map_err(|e| format!("获取字段失败: {}", e))?;
                    let path: String = row.get(3).map_err(|e| format!("获取字段失败: {}", e))?;
                    pages.push(format!("- [{}] {} ({}): {}", page_type, title, canonical_name, path));
                }
                pages.join("\n")
            };

            let (sys_p, user_m) = crate::prompts::prompt_builder::PromptBuilder::build_wiki_update_prompt(
                &resolution_json,
                &rel_json,
                &existing_content,
            );

            // v0.2.2: send_to_model 前检查取消（内存令牌 + DB）
            if cancel_token.is_cancelled() || tq.is_task_cancelled(task_id) {
                let _ = tq.add_event(task_id, "cancelled_before_model", "PipelineCoordinator", "Wiki更新调用模型前检测到取消");
                let _ = tq.mark_cancelled(task_id);
                // 将 source 标记为 cancelled（中途取消）
                if let Ok(conn) = db.connect() {
                    if let Err(e) = conn.execute(
                        "UPDATE sources SET status = 'cancelled', updated_at = ?1 WHERE id = ?2",
                        rusqlite::params![chrono::Utc::now().to_rfc3339(), source_id],
                    ) {
                        log::error!("[coordinator] 更新 source 状态为 cancelled 失败 (source={}): {}", source_id, e);
                    }
                }
                eb.emit_source_updated(kb_id, source_id);
                eb.emit_kb_stats_changed(kb_id);
                return Ok(());
            }

            // 通知前端：Wiki 更新模型调用中，透传完整 Prompt
            let wu_full_prompt = format!("{}\n\n---\n\n{}", sys_p, user_m);
            eb.emit_agent_status_change(
                task_id, "update_plan_done", "running", 0.75,
                &wu_full_prompt, "", "正在调用 LLM 生成 Wiki 更新计划",
            );

            let wu_token_ctx = TokenContext {
                task_id: task_id.to_string(),
                task_name: source_file_name.clone(),
                agent_name: "WikiUpdateAgent".to_string(),
                model_name: model_name.clone(),
                provider: provider_name.clone(),
            };
            let update_result = mg.chat_with_content_and_ctx(
                &provider_config,
                &sys_p,
                &user_m,
                true,
                Some(wu_token_ctx),
            ).await?;

            // v0.2.2: model_returned 后检查取消（内存令牌 + DB）
            if cancel_token.is_cancelled() || tq.is_task_cancelled(task_id) {
                if let Err(e) = std::fs::write(&update_plan_path, &update_result.content) { log::error!("[coordinator] 写入 update_plan.json 失败: {}", e); }
                if let Err(e) = tq.mark_cancelled_after_model(task_id) {
                    log::error!("[coordinator] mark_cancelled_after_model 失败 (task={}): {}", task_id, e);
                }
                eb.emit_kb_stats_changed(kb_id);
                return Ok(());
            }

            if let Err(e) = std::fs::write(&update_plan_path, &update_result.content) { log::error!("[coordinator] 写入 update_plan.json 失败: {}", e); }
            let update_json = json_repair::validate_and_repair_json(&update_result.content)?;
            let validation = crate::schema::json_schema_validator::validate_update_plan(&update_json);
            if !validation.errors.is_empty() {
                log::warn!("[PipelineCoordinator] 更新计划 Schema 验证警告: {:?}", validation.errors);
                // Write validation errors for debugging
                let _ = tq.add_event(task_id, "schema_warning", "PipelineCoordinator",
                    &format!("更新计划 Schema 验证有 {} 个警告", validation.errors.len()));
            }

            // Save the REPAIRED JSON so review generation gets clean data
            let repaired_plan = serde_json::to_string(&update_json).unwrap_or_else(|_| update_result.content.clone());
            if repaired_plan != update_result.content {
                if let Err(e) = std::fs::write(&update_plan_path, &repaired_plan) {
                    log::error!("[coordinator] 写入修复后的 update_plan.json 失败: {}", e);
                }
            }

            // v0.2.2: review_generating 前检查取消（内存令牌 + DB）
            if cancel_token.is_cancelled() || tq.is_task_cancelled(task_id) {
                let _ = tq.add_event(task_id, "cancelled_before_review", "PipelineCoordinator", "生成审阅前检测到取消");
                let _ = tq.mark_cancelled(task_id);
                // 将 source 标记为 cancelled（中途取消）
                if let Ok(conn) = db.connect() {
                    if let Err(e) = conn.execute(
                        "UPDATE sources SET status = 'cancelled', updated_at = ?1 WHERE id = ?2",
                        rusqlite::params![chrono::Utc::now().to_rfc3339(), source_id],
                    ) {
                        log::error!("[coordinator] 更新 source 状态为 cancelled 失败 (source={}): {}", source_id, e);
                    }
                }
                eb.emit_source_updated(kb_id, source_id);
                eb.emit_kb_stats_changed(kb_id);
                return Ok(());
            }

            crate::review::review_engine::ReviewEngine::generate_review(
                db, kb_id, task_id, &update_json, kb_path,
            )?;

            if let Err(e) = tq.update_output_ref(task_id, "update_plan.json") {
                log::error!("[coordinator] update_output_ref 失败 (task={}): {}", task_id, e);
            }
        }

        // v0.2.2: 最终状态更新前检查取消
        if cancel_token.is_cancelled() || tq.is_task_cancelled(task_id) {
            let _ = tq.add_event(task_id, "cancelled_before_final", "PipelineCoordinator", "流水线完成前检测到取消");
            let _ = tq.mark_cancelled(task_id);
            // 将 source 标记为 cancelled（中途取消）
            if let Ok(conn) = db.connect() {
                if let Err(e) = conn.execute(
                    "UPDATE sources SET status = 'cancelled', updated_at = ?1 WHERE id = ?2",
                    rusqlite::params![chrono::Utc::now().to_rfc3339(), source_id],
                ) {
                    log::error!("[coordinator] 更新 source 状态为 cancelled 失败 (source={}): {}", source_id, e);
                }
            }
            eb.emit_source_updated(kb_id, source_id);
            eb.emit_kb_stats_changed(kb_id);
            return Ok(());
        }

        if let Err(e) = tq.update_task_status(task_id, "review_generating", "PipelineCoordinator", "") {
        log::error!("[coordinator] 更新 task 状态为 review_generating 失败 (task={}): {}", task_id, e);
    }

        // v0.2.3: 更新 source 状态为 processed（流水线完成，审阅项已生成）
        if let Ok(conn) = db.connect() {
            if let Err(e) = conn.execute(
                "UPDATE sources SET status = 'processed', updated_at = ?1 WHERE id = ?2",
                rusqlite::params![chrono::Utc::now().to_rfc3339(), source_id],
            ) {
                log::error!("[coordinator] 更新 source 状态为 processed 失败 (source={}): {}", source_id, e);
            }
        }
        eb.emit_source_updated(kb_id, source_id);
        eb.emit_kb_stats_changed(kb_id);

        eb.emit_agent_activity(
            "PipelineCoordinator",
            "review_generating",
            "",
            "流水线执行完成，等待审阅",
        );
        eb.emit_agent_status_change(
            task_id, "review_generated", "done", 0.95,
            "", "", "审阅已生成，等待用户确认",
        );

        // 将 review_id 回写到任务
        if let Ok(conn) = db.connect() {
            if let Ok(review_id) = conn.query_row(
                "SELECT id FROM reviews WHERE task_id = ?1 ORDER BY created_at DESC LIMIT 1",
                rusqlite::params![task_id],
                |row| row.get::<_, String>(0),
            ) {
                let _ = tq.set_review_id(task_id, &review_id);
            }
        }

        eb.emit_agent_status_change(
            task_id, "pipeline_complete", "done", 1.0,
            "", "", "流水线执行完成",
        );

        Ok(())
    }

    /// 运行 Resolution（v0.2.2: 添加取消检查）
    pub async fn run_resolution(
        &self,
        kb_id: &str,
        task_id: &str,
        ingest_result_json: &str,
    ) -> Result<(), String> {
        // 熔断检查
        self.check_quota(kb_id)?;

        // v0.2.2: 检查任务是否已取消
        if self.task_queue.is_task_cancelled(task_id) {
            self.task_queue.mark_cancelled(task_id)?;
            self.update_source_status(kb_id, task_id, "cancelled");
            return Err("任务已取消".to_string());
        }

        self.task_queue.update_task_status(task_id, "resolution_running", "ResolutionAgent", "")?;

        let kb_path = self.get_kb_path(kb_id)?;

        let _existing_summary = self.get_existing_pages_summary(kb_id)?;

        let config = self.config.get_provider_config()?;
        let source_file_name = self.get_source_file_name(task_id).unwrap_or_else(|| "未知文件".to_string());

        use crate::prompts::prompt_builder::PromptBuilder;
        let candidates = crate::search::candidate_search::CandidateSearchEngine::search(
            &self.db, kb_id, ingest_result_json, &kb_path,
        )?;

        let candidates_json = serde_json::to_string(&candidates)
            .map_err(|e| format!("序列化候选实体列表失败: {}", e))?;
        let (system_prompt, user_message) = PromptBuilder::build_resolution_prompt(
            ingest_result_json,
            &candidates_json,
        );

        let token_ctx = self.build_token_ctx(task_id, &source_file_name, "ResolutionAgent");
        let result = self.model_gateway.chat_with_content_and_ctx(
            &config,
            &system_prompt,
            &user_message,
            true,
            Some(token_ctx),
        ).await?;

        // 保存原始响应
        let tasks_dir = self.workspace.get_tasks_dir(&std::path::PathBuf::from(&kb_path));
        let task_dir = tasks_dir.join(task_id);
        if let Err(e) = std::fs::create_dir_all(&task_dir) { log::error!("[coordinator] 创建 task_dir 失败: {}", e); }
        if let Err(e) = std::fs::write(task_dir.join("resolution_result.json"), &result.content) { log::error!("[coordinator] 写入 resolution_result.json 失败: {}", e); }

        // 验证 JSON
        let json = json_repair::validate_and_repair_json(&result.content)?;
        crate::schema::json_schema_validator::validate_resolution_result(&json);

        self.task_queue.update_output_ref(task_id, "resolution_result.json")?;
        self.task_queue.update_task_status(task_id, "relationship_running", "ResolutionAgent", "")?;

        Ok(())
    }

    /// 运行关系标准化（v0.2.2: 添加取消检查）
    pub async fn run_relationship(
        &self,
        kb_id: &str,
        task_id: &str,
        resolution_json: &str,
    ) -> Result<(), String> {
        // 熔断检查
        self.check_quota(kb_id)?;

        // v0.2.2: 检查任务是否已取消
        if self.task_queue.is_task_cancelled(task_id) {
            self.task_queue.mark_cancelled(task_id)?;
            self.update_source_status(kb_id, task_id, "cancelled");
            return Err("任务已取消".to_string());
        }

        self.task_queue.update_task_status(task_id, "relationship_running", "RelationshipAgent", "")?;

        let kb_path = self.get_kb_path(kb_id)?;
        let config = self.config.get_provider_config()?;
        let source_file_name = self.get_source_file_name(task_id).unwrap_or_else(|| "未知文件".to_string());

        use crate::prompts::prompt_builder::PromptBuilder;
        let (system_prompt, user_message) = PromptBuilder::build_relationship_prompt(resolution_json);

        let token_ctx = self.build_token_ctx(task_id, &source_file_name, "RelationshipAgent");
        let result = self.model_gateway.chat_with_content_and_ctx(
            &config,
            &system_prompt,
            &user_message,
            true,
            Some(token_ctx),
        ).await?;

        let tasks_dir = self.workspace.get_tasks_dir(&std::path::PathBuf::from(&kb_path));
        let task_dir = tasks_dir.join(task_id);
        if let Err(e) = std::fs::create_dir_all(&task_dir) { log::error!("[coordinator] 创建 task_dir 失败: {}", e); }
        if let Err(e) = std::fs::write(task_dir.join("relationship_result.json"), &result.content) { log::error!("[coordinator] 写入 relationship_result.json 失败: {}", e); }

        let json = json_repair::validate_and_repair_json(&result.content)?;

        // 写入关系数据
        crate::agents::relationship::RelationshipAgent::save_relationships(
            &self.db, kb_id, &json,
        )?;

        self.task_queue.update_output_ref(task_id, "relationship_result.json")?;
        self.task_queue.update_task_status(task_id, "update_plan_generating", "RelationshipAgent", "")?;

        Ok(())
    }

    /// 生成 Wiki 更新计划（v0.2.2: 添加取消检查 + 去重）
    pub async fn run_wiki_update(
        &self,
        kb_id: &str,
        task_id: &str,
        resolution_json: &str,
        relationship_json: &str,
    ) -> Result<(), String> {
        // 熔断检查
        self.check_quota(kb_id)?;

        // v0.2.2: 检查任务是否已取消
        if self.task_queue.is_task_cancelled(task_id) {
            self.task_queue.mark_cancelled(task_id)?;
            self.update_source_status(kb_id, task_id, "cancelled");
            return Err("任务已取消".to_string());
        }

        self.task_queue.update_task_status(task_id, "update_plan_generating", "WikiUpdateAgent", "")?;

        let kb_path = self.get_kb_path(kb_id)?;
        let config = self.config.get_provider_config()?;
        let source_file_name = self.get_source_file_name(task_id).unwrap_or_else(|| "未知文件".to_string());

        let existing_content = self.get_existing_pages_summary(kb_id)?;

        use crate::prompts::prompt_builder::PromptBuilder;
        let (system_prompt, user_message) = PromptBuilder::build_wiki_update_prompt(
            resolution_json,
            relationship_json,
            &existing_content,
        );

        let token_ctx = self.build_token_ctx(task_id, &source_file_name, "WikiUpdateAgent");
        let result = self.model_gateway.chat_with_content_and_ctx(
            &config,
            &system_prompt,
            &user_message,
            true,
            Some(token_ctx),
        ).await?;

        let tasks_dir = self.workspace.get_tasks_dir(&std::path::PathBuf::from(&kb_path));
        let task_dir = tasks_dir.join(task_id);
        if let Err(e) = std::fs::create_dir_all(&task_dir) { log::error!("[coordinator] 创建 task_dir 失败: {}", e); }
        if let Err(e) = std::fs::write(task_dir.join("update_plan.json"), &result.content) { log::error!("[coordinator] 写入 update_plan.json 失败: {}", e); }

        let json = json_repair::validate_and_repair_json(&result.content)?;
        crate::schema::json_schema_validator::validate_update_plan(&json);

        // v0.2.2: 生成审阅前检查取消
        if self.task_queue.is_task_cancelled(task_id) {
            self.task_queue.mark_cancelled(task_id)?;
            self.update_source_status(kb_id, task_id, "cancelled");
            return Err("任务已取消".to_string());
        }

        // 生成审阅项
        crate::review::review_engine::ReviewEngine::generate_review(
            &self.db, kb_id, task_id, &json, &kb_path,
        )?;

        self.task_queue.update_output_ref(task_id, "update_plan.json")?;
        self.task_queue.update_task_status(task_id, "review_generating", "WikiUpdateAgent", "")?;

        // v0.2.3: 更新 source 状态为 processed（手动逐步执行时 run_full_pipeline 不负责此步骤）
        self.update_source_status(kb_id, task_id, "processed");

        Ok(())
    }

    /// 运行问答（v0.2.2: 集成语义搜索 + Rerank）
    pub async fn run_query(
        &self,
        kb_id: &str,
        question: &str,
        scope: &str, // "all" | page_path | "tag:xxx"
    ) -> Result<String, String> {
        // 熔断检查
        self.check_quota(kb_id)?;

        let _kb_path = self.get_kb_path(kb_id)?;
        let config = self.config.get_provider_config()?;

        // 两阶段检索：语义向量搜索 + 关键词回退
        let semantic_context = self.semantic_search(kb_id, question);
        let keyword_context = self.get_query_context(kb_id, scope)?;

        // 合并上下文：语义结果优先，关键词结果补充
        let context = merge_query_contexts(&semantic_context, &keyword_context);

        let scope_desc = match scope {
            "all" => "整个知识库".to_string(),
            s if s.starts_with("tag:") => format!("页面类型: {}", &s[4..]),
            _ => format!("页面: {}", scope),
        };

        use crate::prompts::prompt_builder::PromptBuilder;
        let (system_prompt, user_message) = PromptBuilder::build_query_prompt(
            question,
            &context,
            &scope_desc,
            true,
        );

        let query_id = uuid::Uuid::new_v4().to_string();
        let token_ctx = self.build_token_ctx(&query_id, "问答", "QueryAgent");
        let result = self.model_gateway.chat_with_content_and_ctx(
            &config,
            &system_prompt,
            &user_message,
            true,
            Some(token_ctx),
        ).await?;

        let json = json_repair::validate_and_repair_json(&result.content)?;
        crate::schema::json_schema_validator::validate_query_result(&json);

        Ok(result.content)
    }

    /// 语义搜索：向量化提问 → vdb_chunks 余弦相似度 → Top 20
    fn semantic_search(&self, kb_id: &str, question: &str) -> String {
        let mut ctx = String::new();
        match self.vdb.search_similar(kb_id, question, 20) {
            Ok(results) if !results.is_empty() => {
                // 简易 Rerank：按相似度排序 + 关键词密度加权
                let reranked = rerank_chunks(question, &results);
                ctx.push_str("# 语义检索结果 (Top 3 精排)\n\n");
                for (i, (chunk, score)) in reranked.iter().take(3).enumerate() {
                    ctx.push_str(&format!(
                        "**相关片段 {}** (相关度: {:.2}):\n{}\n\n",
                        i + 1,
                        score,
                        chunk
                    ));
                }
                // 附加剩余 Top 20 作为扩展上下文（最多前 500 字符）
                ctx.push_str("# 扩展参考片段\n\n");
                for (chunk, _) in results.iter().skip(3).take(17) {
                    ctx.push_str(&format!(
                        "- {}...\n",
                        &chunk[..chunk.len().min(200)]
                    ));
                }
            }
            Ok(_) => {
                log::info!("[Coordinator] 语义搜索无结果，回退到关键词检索");
            }
            Err(e) => {
                log::warn!("[Coordinator] 语义搜索失败: {}，回退到关键词检索", e);
            }
        }
        ctx
    }

    /// 运行健康检查
    pub async fn run_health_check(&self, kb_id: &str) -> Result<String, String> {
        let kb_path = self.get_kb_path(kb_id)?;
        crate::agents::health_check::HealthCheckAgent::run(
            &self.db, kb_id, &kb_path, &self.workspace,
        )
    }

    // 辅助方法

    /// 通过 task_id 查找 source_id 并更新 source 状态，发出事件
    fn update_source_status(&self, kb_id: &str, task_id: &str, status: &str) {
        if let Ok(conn) = self.db.connect() {
            let source_id: Option<String> = match conn.query_row(
                "SELECT input_ref FROM tasks WHERE id = ?1 AND task_type = 'source_ingest'",
                rusqlite::params![task_id],
                |row| row.get(0),
            ) {
                Ok(sid) => Some(sid),
                Err(rusqlite::Error::QueryReturnedNoRows) => None,
                Err(e) => {
                    log::error!("[coordinator] 查询任务 input_ref 失败 (task={}): {}", task_id, e);
                    None
                }
            }.filter(|s: &String| !s.is_empty());
            if let Some(ref sid) = source_id {
                if let Err(e) = conn.execute(
                    "UPDATE sources SET status = ?1, updated_at = ?2 WHERE id = ?3",
                    rusqlite::params![status, chrono::Utc::now().to_rfc3339(), sid],
                ) {
                    log::error!("[coordinator] update_source_status 更新 source 状态失败 (source={}, status={}): {}", sid, status, e);
                }
                self.event_bus.emit_source_updated(kb_id, sid);
            }
        }
        self.event_bus.emit_kb_stats_changed(kb_id);
    }

    fn get_kb_path(&self, kb_id: &str) -> Result<String, String> {
        let conn = self.db.connect()?;
        let path: String = conn
            .query_row("SELECT path FROM knowledge_bases WHERE id = ?1", rusqlite::params![kb_id], |row| row.get(0))
            .map_err(|_| "知识库不存在".to_string())?;
        Ok(path)
    }

    fn get_existing_pages_summary(&self, kb_id: &str) -> Result<String, String> {
        let conn = self.db.connect()?;
        let mut stmt = conn
            .prepare("SELECT title, canonical_name, page_type, path FROM wiki_pages WHERE kb_id = ?1")
            .map_err(|e| format!("查询页面失败: {}", e))?;

        let pages: Vec<String> = stmt
            .query_map(rusqlite::params![kb_id], |row| {
                let title: String = row.get(0)?;
                let cn: String = row.get(1)?;
                let pt: String = row.get(2)?;
                let path: String = row.get(3)?;
                Ok(format!("- [{}] {} ({}): {}", pt, title, cn, path))
            })
            .map_err(|e| format!("映射页面失败: {}", e))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("收集页面失败: {}", e))?;

        Ok(pages.join("\n"))
    }

    fn get_query_context(&self, kb_id: &str, scope: &str) -> Result<String, String> {
        let conn = self.db.connect()?;
        let kb_path: String = conn
            .query_row("SELECT path FROM knowledge_bases WHERE id = ?1", rusqlite::params![kb_id], |row| row.get(0))
            .map_err(|_| "知识库不存在".to_string())?;
        build_query_context_from_db(&self.db, &self.workspace, kb_id, scope, &kb_path)
    }
}

/// 简易 Rerank：对语义搜索结果按关键词密度加权重排
/// 当 Cross-Encoder 模型未就绪时，作为轻量级替代方案。
fn rerank_chunks(question: &str, chunks: &[(String, f64)]) -> Vec<(String, f64)> {
    let question_lower = question.to_lowercase();
    let keywords: Vec<&str> = question_lower
        .split(|c: char| !c.is_alphanumeric() && c != '_')
        .filter(|w| w.len() >= 2)
        .collect();

    let mut scored: Vec<(String, f64)> = chunks
        .iter()
        .map(|(chunk, base_sim)| {
            let chunk_lower = chunk.to_lowercase();
            let kw_score: f64 = keywords
                .iter()
                .map(|kw| {
                    let count = chunk_lower.matches(kw).count() as f64;
                    count / (chunk_lower.len().max(1) as f64) * 1000.0
                })
                .sum();
            // 余弦相似度权重 0.7 + 关键词密度权重 0.3
            let combined = base_sim * 0.7 + kw_score * 0.3;
            (chunk.clone(), combined)
        })
        .collect();

    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    scored.truncate(3);
    scored
}

/// 合并语义搜索结果和关键词检索结果，去重
fn merge_query_contexts(semantic: &str, keyword: &str) -> String {
    if semantic.is_empty() {
        return keyword.to_string();
    }
    format!("{}\n\n---\n\n{}", semantic, keyword)
}

/// 公开的查询上下文构建函数，供 chat_stream 等外部调用
pub fn build_query_context_from_db(
    db: &Arc<DatabaseService>,
    workspace: &Arc<WorkspaceService>,
    kb_id: &str,
    scope: &str,
    kb_path: &str,
) -> Result<String, String> {
    let workspace_root = std::path::PathBuf::from(kb_path);
    let wiki_dir = workspace.get_wiki_dir(&workspace_root);
    let mut context = String::new();

    // 收集 index.md 内容
    let index_path = wiki_dir.join("index.md");
    if index_path.exists() {
        if let Ok(content) = std::fs::read_to_string(&index_path) {
            context.push_str(&format!("# 知识库索引\n\n{}\n\n", &content[..content.len().min(5000)]));
        }
    }

    // 收集相关 Wiki 页面 — 统一使用参数化查询
    let conn = db.connect()?;
    let pages: Vec<(String, String)> = match scope {
        "all" => {
            let mut stmt = conn.prepare(
                "SELECT title, path FROM wiki_pages WHERE kb_id = ?1 LIMIT 50"
            ).map_err(|e| format!("QueryAgent 检索上下文失败: {}", e))?;
            let rows = stmt.query_map(rusqlite::params![kb_id], |row| {
                let title: String = row.get(0)?;
                let path: String = row.get(1)?;
                Ok((title, path))
            }).map_err(|e| format!("QueryAgent 映射页面失败: {}", e))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("QueryAgent 收集页面失败: {}", e))?;
            rows
        }
        s if s.starts_with("tag:") => {
            let page_type = &s[4..];
            let mut stmt = conn.prepare(
                "SELECT title, path FROM wiki_pages WHERE kb_id = ?1 AND page_type = ?2 LIMIT 50"
            ).map_err(|e| format!("QueryAgent 页面类型检索失败: {}", e))?;
            let rows = stmt.query_map(rusqlite::params![kb_id, page_type], |row| {
                let title: String = row.get(0)?;
                let path: String = row.get(1)?;
                Ok((title, path))
            }).map_err(|e| format!("QueryAgent 映射标签页面失败: {}", e))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("QueryAgent 收集标签页面失败: {}", e))?;
            rows
        }
        page_path if !page_path.is_empty() => {
            let mut stmt = conn.prepare(
                "SELECT title, path FROM wiki_pages WHERE kb_id = ?1 AND path = ?2 LIMIT 5"
            ).map_err(|e| format!("QueryAgent 页面检索失败: {}", e))?;
            let rows = stmt.query_map(rusqlite::params![kb_id, page_path], |row| {
                let title: String = row.get(0)?;
                let path: String = row.get(1)?;
                Ok((title, path))
            }).map_err(|e| format!("QueryAgent 映射指定页面失败: {}", e))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("QueryAgent 收集指定页面失败: {}", e))?;
            rows
        }
        _ => Vec::new(),
    };

    if pages.is_empty() {
        context.push_str("# 相关页面\n\n暂无匹配页面，请尝试扩大搜索范围或先导入文档。\n\n");
    } else {
        context.push_str(&format!("# 相关页面\n\n{}\n\n",
            pages.iter().map(|(t, p)| format!("- {} ({})", t, p)).collect::<Vec<_>>().join("\n")));
    }

    // 读取实际页面内容（前10个页面，使用 PathService 正确解析路径）
    for (title, path_str) in pages.iter().take(10) {
        let normalized = crate::wiki::path_service::PathService::normalize_workspace_path(path_str);
        let page_file = crate::wiki::path_service::PathService::resolve_workspace_path(&workspace_root, &normalized);
        if page_file.exists() {
            if let Ok(content) = std::fs::read_to_string(&page_file) {
                let page_type = crate::wiki::path_service::PathService::path_to_page_type(&normalized);
                context.push_str(&format!(
                    "\n---\n## {} (类型: {})\n\n{}\n",
                    title, page_type,
                    &content[..content.len().min(3000)]
                ));
            }
        } else {
            context.push_str(&format!("\n---\n## {} (文件缺失: {})\n\n", title, normalized));
        }
    }

    Ok(context)
}
