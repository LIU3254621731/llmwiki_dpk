use std::sync::Arc;
use crate::core::database_service::DatabaseService;
use crate::core::task_queue::TaskQueue;
use crate::core::config_service::ConfigService;
use crate::core::event_bus::EventBus;
use crate::model::model_gateway::ModelGateway;

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

        Ok(update_result.content)
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
