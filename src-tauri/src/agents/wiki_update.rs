use std::sync::Arc;
use crate::core::database_service::DatabaseService;
use crate::core::task_queue::TaskQueue;
use crate::core::config_service::ConfigService;
use crate::core::event_bus::EventBus;
use crate::model::model_gateway::ModelGateway;
use crate::dedup::dedup_service::DedupService;

pub struct WikiUpdateAgent {
    task_queue: Arc<TaskQueue>,
    db: Arc<DatabaseService>,
    config: Arc<ConfigService>,
    event_bus: Arc<EventBus>,
    model_gateway: Arc<ModelGateway>,
}

impl WikiUpdateAgent {
    pub fn new(
        task_queue: Arc<TaskQueue>,
        db: Arc<DatabaseService>,
        config: Arc<ConfigService>,
        event_bus: Arc<EventBus>,
        model_gateway: Arc<ModelGateway>,
    ) -> Self {
        Self {
            task_queue, db, config, event_bus, model_gateway,
        }
    }

    pub async fn execute(
        &self,
        kb_id: &str,
        task_id: &str,
        resolution_json: &str,
        relationship_json: &str,
    ) -> Result<String, String> {
        self.event_bus.emit_agent_activity(
            "WikiUpdateAgent",
            "update_plan_generating",
            "",
            "正在生成 Wiki 更新计划",
        );

        self.task_queue.update_task_status(task_id, "update_plan_generating", "WikiUpdateAgent", "")?;

        let existing_content = self.get_existing_pages(kb_id)?;

        let (sys_prompt, user_m) = crate::prompts::prompt_builder::PromptBuilder::build_wiki_update_prompt(
            resolution_json,
            relationship_json,
            &existing_content,
        );

        let config = self.config.get_provider_config()?;
        let update_result = self.model_gateway.chat_with_content(
            &config, &sys_prompt, &user_m, true,
        ).await?;

        self.task_queue.update_task_status(task_id, "update_plan_generated", "WikiUpdateAgent", "")?;
        self.event_bus.emit_agent_activity(
            "WikiUpdateAgent",
            "update_plan_generated",
            "",
            "Wiki 更新计划生成完成",
        );

        // v0.2.3: Post-process update plan through dedup service
        let processed_content = self.apply_dedup_to_plan(kb_id, &update_result.content);

        Ok(processed_content)
    }

    /// v0.2.3: Apply dedup checks to the LLM-generated update plan JSON.
    /// Scans each "create" operation against existing pages and adjusts based on similarity:
    ///   >0.95: auto-skip creation, log dedup event
    ///   0.85-0.95: mark as merge_suggestion
    ///   <0.85: proceed with normal create_page
    fn apply_dedup_to_plan(&self, kb_id: &str, plan_json: &str) -> String {
        let dedup = DedupService::new(self.db.clone());

        let mut plan: serde_json::Value = match serde_json::from_str(plan_json) {
            Ok(v) => v,
            Err(e) => {
                log::error!("[wiki_update] Failed to parse update plan JSON: {}", e);
                return plan_json.to_string();
            }
        };

        // Try multiple known array keys
        let array_keys = ["wiki_update_plan", "updates", "wiki_updates", "update_plan", "proposed_wiki_updates"];
        let mut found_key: Option<String> = None;

        for key in &array_keys {
            if plan.get(key).and_then(|v| v.as_array()).is_some() {
                found_key = Some(key.to_string());
                break;
            }
        }

        if let Some(ref key) = found_key {
            if let Some(plans) = plan.get_mut(key).and_then(|v| v.as_array_mut()) {
                for item in plans.iter_mut() {
                    let operation = item.get("operation")
                        .or_else(|| item.get("action"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    let op_type = item.get("operation_type")
                        .or_else(|| item.get("type"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("");

                    // Only check "create" operations
                    let is_create = operation == "create" || op_type == "create" || op_type == "create_page";
                    if !is_create {
                        continue;
                    }

                    let title = item.get("title")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    if title.is_empty() || title == "Untitled" {
                        continue;
                    }

                    let page_type = item.get("page_type")
                        .or_else(|| item.get("type"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("concept");

                    let candidates = dedup.find_duplicate_candidates(kb_id, title, page_type);

                    if let Some(best) = candidates.first() {
                        if best.similarity_score > 0.95 {
                            // Auto-skip: this is essentially a duplicate
                            log::info!(
                                "[wiki_update] DEDUP SKIP: title='{}', match='{}', score={:.4}",
                                title, best.title, best.similarity_score
                            );
                            self.event_bus.emit_agent_activity(
                                "WikiUpdateAgent",
                                "dedup_skip",
                                &best.id,
                                &format!("Skipping '{}' (duplicate of '{}', similarity={:.2})", title, best.title, best.similarity_score),
                            );
                            // Mark as skipped in plan
                            if let Some(obj) = item.as_object_mut() {
                                obj.insert("operation".to_string(), serde_json::Value::String("skip".to_string()));
                                obj.insert("operation_type".to_string(), serde_json::Value::String("skip_duplicate".to_string()));
                                obj.insert("skip_reason".to_string(), serde_json::Value::String(format!(
                                    "Duplicate of '{}' (similarity={:.2})", best.title, best.similarity_score
                                )));
                                obj.insert("matched_page_id".to_string(), serde_json::Value::String(best.id.clone()));
                            }
                        } else if best.similarity_score >= 0.85 {
                            // Merge suggestion: create review item for human decision
                            log::info!(
                                "[wiki_update] DEDUP MERGE: title='{}', match='{}', score={:.4}",
                                title, best.title, best.similarity_score
                            );
                            if let Some(obj) = item.as_object_mut() {
                                obj.insert("operation".to_string(), serde_json::Value::String("create".to_string()));
                                obj.insert("operation_type".to_string(), serde_json::Value::String("merge_suggestion".to_string()));
                                obj.insert("merge_candidate".to_string(), serde_json::Value::Bool(true));
                                obj.insert("matched_page_id".to_string(), serde_json::Value::String(best.id.clone()));
                                obj.insert("matched_title".to_string(), serde_json::Value::String(best.title.clone()));
                                obj.insert("matched_path".to_string(), serde_json::Value::String(best.path.clone()));
                                obj.insert("similarity_score".to_string(), serde_json::Value::Number(
                                    serde_json::Number::from_f64(best.similarity_score).unwrap_or(serde_json::Number::from(0))
                                ));
                            }
                        }
                        // similarity < 0.85: proceed normally, no changes needed
                    }
                }
            }
        }

        match serde_json::to_string(&plan) {
            Ok(s) => s,
            Err(e) => {
                log::error!("[wiki_update] Failed to serialize dedup-processed plan: {}", e);
                plan_json.to_string()
            }
        }
    }

    fn get_existing_pages(&self, kb_id: &str) -> Result<String, String> {
        let conn = self.db.connect()?;
        let mut stmt = conn.prepare(
            "SELECT title, canonical_name, page_type, path FROM wiki_pages WHERE kb_id = ?1"
        ).map_err(|e| format!("准备查询失败: {}", e))?;

        let mut pages = Vec::new();
        let mut rows = stmt.query(rusqlite::params![kb_id])
            .map_err(|e| format!("查询失败: {}", e))?;
        while let Some(row) = rows.next().map_err(|e| format!("读取行失败: {}", e))? {
            let page_type: String = row.get(2).map_err(|e| format!("获取字段失败: {}", e))?;
            let title: String = row.get(0).map_err(|e| format!("获取字段失败: {}", e))?;
            let canonical_name: String = row.get(1).map_err(|e| format!("获取字段失败: {}", e))?;
            let path: String = row.get(3).map_err(|e| format!("获取字段失败: {}", e))?;
            pages.push(format!("- [{}] {} ({}): {}", page_type, title, canonical_name, path));
        }

        Ok(pages.join("\n"))
    }
}
