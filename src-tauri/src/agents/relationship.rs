use std::sync::Arc;
use crate::core::database_service::DatabaseService;
use crate::core::task_queue::TaskQueue;
use crate::core::config_service::ConfigService;
use crate::core::event_bus::EventBus;
use crate::model::model_gateway::ModelGateway;
use crate::schema::json_repair;

pub struct RelationshipAgent {
    task_queue: Arc<TaskQueue>,
    db: Arc<DatabaseService>,
    config: Arc<ConfigService>,
    event_bus: Arc<EventBus>,
    model_gateway: Arc<ModelGateway>,
}

impl RelationshipAgent {
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
    ) -> Result<String, String> {
        self.event_bus.emit_agent_activity(
            "RelationshipAgent",
            "relationship_running",
            "",
            "正在执行关系标准化",
        );

        self.task_queue.update_task_status(task_id, "relationship_running", "RelationshipAgent", "")?;

        let (sys_prompt, user_msg) = crate::prompts::prompt_builder::PromptBuilder::build_relationship_prompt(
            resolution_json,
        );

        let config = self.config.get_provider_config()?;
        let rel_result = self.model_gateway.chat_with_content(
            &config, &sys_prompt, &user_msg, true,
        ).await?;

        let rel_json = json_repair::validate_and_repair_json(&rel_result.content)
            .map_err(|e| format!("JSON 校验失败: {}", e))?;

        Self::save_relationships(&self.db, kb_id, &rel_json)?;

        self.task_queue.update_task_status(task_id, "relationship_completed", "RelationshipAgent", "")?;
        self.event_bus.emit_agent_activity(
            "RelationshipAgent",
            "relationship_completed",
            "",
            "关系标准化完成",
        );

        Ok(rel_result.content)
    }

    pub fn save_relationships(
        db: &Arc<DatabaseService>,
        kb_id: &str,
        json: &serde_json::Value,
    ) -> Result<(), String> {
        let conn = db.connect()?;
        let now = chrono::Utc::now().to_rfc3339();

        let valid_relations = vec![
            "describes", "uses", "is_a", "has_a", "related_to",
            "instantiates", "contradicts", "supports", "refutes",
            "part_of", "derives_from", "cites",
        ];

        let mut high_conf_count = 0u32;
        let mut related_to_count = 0u32;
        let max_high_conf: u32 = 20;
        let max_related_to: u32 = 5;

        if let Some(rels) = json.get("relationships").and_then(|r| r.as_array()) {
            for rel in rels {
                let relation = rel.get("relation").and_then(|v| v.as_str()).unwrap_or("related_to");
                let confidence = rel.get("confidence").and_then(|v| v.as_str()).unwrap_or("medium");
                let evidence_source = rel.get("evidence_source_id")
                    .or_else(|| rel.get("evidence").and_then(|e| e.get("source_id")))
                    .and_then(|v| v.as_str()).unwrap_or("");
                let evidence_location = rel.get("evidence_location")
                    .or_else(|| rel.get("evidence").and_then(|e| e.get("location")))
                    .and_then(|v| v.as_str()).unwrap_or("");

                // 固定 enum 校验
                if !valid_relations.contains(&relation) {
                    continue;
                }

                // 无 evidence 不写入
                if evidence_source.is_empty() {
                    continue;
                }

                // related_to 限5条
                if relation == "related_to" {
                    related_to_count += 1;
                    if related_to_count > max_related_to { continue; }
                }

                // 高置信限20条
                if confidence == "high" {
                    high_conf_count += 1;
                    if high_conf_count > max_high_conf { continue; }
                }

                let status = if confidence == "low" { "pending_review" } else { "active" };

                let source_name = rel.get("source_name").or(rel.get("source"))
                    .and_then(|v| v.as_str()).unwrap_or("");
                let target_name = rel.get("target_name").or(rel.get("target"))
                    .and_then(|v| v.as_str()).unwrap_or("");

                if source_name.is_empty() || target_name.is_empty() {
                    continue;
                }

                let source_id: Option<String> = match conn.query_row(
                    "SELECT id FROM knowledge_items WHERE kb_id = ?1 AND canonical_name = ?2",
                    rusqlite::params![kb_id, source_name],
                    |row| row.get(0),
                ) {
                    Ok(id) => Some(id),
                    Err(rusqlite::Error::QueryReturnedNoRows) => None,
                    Err(e) => {
                        log::error!("[relationship] 查询 source knowledge_item 失败 (name={}): {}", source_name, e);
                        return Err(format!("查询关系源节点失败: {}", e));
                    }
                };

                let target_id: Option<String> = match conn.query_row(
                    "SELECT id FROM knowledge_items WHERE kb_id = ?1 AND canonical_name = ?2",
                    rusqlite::params![kb_id, target_name],
                    |row| row.get(0),
                ) {
                    Ok(id) => Some(id),
                    Err(rusqlite::Error::QueryReturnedNoRows) => None,
                    Err(e) => {
                        log::error!("[relationship] 查询 target knowledge_item 失败 (name={}): {}", target_name, e);
                        return Err(format!("查询关系目标节点失败: {}", e));
                    }
                };

                if let (Some(sid), Some(tid)) = (source_id, target_id) {
                    let id = uuid::Uuid::new_v4().to_string();
                    if let Err(e) = conn.execute(
                        "INSERT OR IGNORE INTO relationships (id, kb_id, source_item_id, target_item_id, relation, evidence_source_id, evidence_location, confidence, status, created_at)
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                        rusqlite::params![id, kb_id, sid, tid, relation, evidence_source, evidence_location, confidence, status, now],
                    ) {
                        log::error!("[relationship] 插入关系失败 (relation={}): {}", relation, e);
                    }
                }
            }
        }

        Ok(())
    }
}
