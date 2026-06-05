// ResolutionAgent - 消歧处理 Agent (v0.2.2: 集成 DedupService)
// 从 Coordinator 中提取的独立消歧逻辑

use std::sync::Arc;
use crate::core::database_service::DatabaseService;
use crate::core::task_queue::TaskQueue;
use crate::core::config_service::ConfigService;
use crate::core::event_bus::EventBus;
use crate::model::model_gateway::ModelGateway;
use crate::dedup::dedup_service::DedupService;

pub struct ResolutionAgent {
    task_queue: Arc<TaskQueue>,
    db: Arc<DatabaseService>,
    config: Arc<ConfigService>,
    event_bus: Arc<EventBus>,
    model_gateway: Arc<ModelGateway>,
}

impl ResolutionAgent {
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
        kb_path: &str,
        task_id: &str,
        ingest_json: &str,
    ) -> Result<String, String> {
        self.event_bus.emit_agent_activity(
            "ResolutionAgent",
            "resolution_running",
            "",
            "正在执行消歧处理",
        );

        let candidates = crate::search::candidate_search::CandidateSearchEngine::search(
            &self.db, kb_id, ingest_json, kb_path,
        )?;

        self.task_queue.update_task_status(task_id, "resolution_running", "ResolutionAgent", "")?;
        self.task_queue.add_event(task_id, "resolution_candidates", "ResolutionAgent",
            &format!("找到 {} 个候选实体", candidates.len()))?;

        let candidates_json = serde_json::to_string(&candidates)
            .map_err(|e| format!("序列化候选实体失败: {}", e))?;
        let (system_prompt, user_message) = crate::prompts::prompt_builder::PromptBuilder::build_resolution_prompt(
            ingest_json,
            &candidates_json,
        );

        let config = self.config.get_provider_config()?;
        let result = self.model_gateway.chat_with_content(
            &config, &system_prompt, &user_message, true,
        ).await?;

        self.event_bus.emit_agent_activity(
            "ResolutionAgent",
            "resolution_completed",
            "",
            "消歧处理完成",
        );

        Ok(result.content)
    }
}

/// 本地消歧辅助：基于字符串相似度检查 + DedupService 查重（v0.2.2: 增强版）
pub fn local_alias_check(
    db: &Arc<DatabaseService>,
    kb_id: &str,
    name: &str,
) -> Result<Vec<(String, String, f64)>, String> {
    let conn = db.connect()?;

    // 1. 使用 DedupService 查重（检查 wiki_pages + aliases）
    let dedup_result = DedupService::find_duplicates(db, kb_id, name).map_err(|e| {
        format!("DedupService 查重失败 (name={}): {}", name, e)
    })?;

    let mut matches = Vec::new();

    // 2. 从 DedupService 结果中提取匹配
    for dm in &dedup_result.matches {
        if dm.similarity >= 0.7 {
            matches.push((dm.matched_page_id.clone(), dm.matched_title.clone(), dm.similarity));
        }
    }

    // 3. 同时检查 knowledge_items（DedupService 可能漏查没有 wiki_pages 的知识项）
    let mut stmt = conn
        .prepare("SELECT id, canonical_name FROM knowledge_items WHERE kb_id = ?1")
        .map_err(|e| format!("查询 knowledge_items 失败: {}", e))?;

    let items: Vec<(String, String)> = stmt
        .query_map(rusqlite::params![kb_id], |row| {
            Ok((row.get(0)?, row.get(1)?))
        })
        .map_err(|e| format!("映射 knowledge_items 失败: {}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("收集 knowledge_items 失败: {}", e))?;

    let normalized_name = name.to_lowercase();

    for (id, cn) in &items {
        let normalized_cn = cn.to_lowercase();
        let similarity = strsim::normalized_damerau_levenshtein(&normalized_name, &normalized_cn);
        if similarity > 0.7
            && !matches.iter().any(|(mid, _, _)| mid == id) {
                matches.push((id.clone(), cn.clone(), similarity));
            }
    }

    matches.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));
    Ok(matches)
}

pub fn alias_exists(db: &Arc<DatabaseService>, normalized_alias: &str) -> Result<bool, String> {
    let conn = db.connect()?;
    let count: i64 = match conn.query_row(
        "SELECT COUNT(*) FROM aliases WHERE normalized_alias = ?1",
        rusqlite::params![normalized_alias],
        |row| row.get(0),
    ) {
        Ok(c) => c,
        Err(rusqlite::Error::QueryReturnedNoRows) => 0,
        Err(e) => return Err(format!("查询别名是否存在失败: {}", e)),
    };
    Ok(count > 0)
}
