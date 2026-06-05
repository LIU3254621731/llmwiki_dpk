use tauri::{AppHandle, Emitter};
use serde::Serialize;

pub struct EventBus {
    app: AppHandle,
}

impl EventBus {
    pub fn new(app: AppHandle) -> Self {
        Self { app }
    }

    /// 发送任务更新事件
    pub fn emit_task_updated(&self, _kb_id: &str, task: &impl Serialize) {
        if let Ok(payload) = serde_json::to_value(task) {
            if let Err(e) = self.app.emit("task-updated", payload) {
                log::error!("[EventBus] emit task-updated 失败: {}", e);
            }
        }
    }

    /// 发送任务事件
    pub fn emit_task_event(
        &self,
        task_id: &str,
        event_type: &str,
        agent_name: &str,
        message: &str,
    ) {
        let payload = serde_json::json!({
            "task_id": task_id,
            "event_type": event_type,
            "agent_name": agent_name,
            "message": message,
            "created_at": chrono::Utc::now().to_rfc3339(),
        });
        if let Err(e) = self.app.emit("task-event", payload) {
            log::error!("[EventBus] emit task-event 失败: {}", e);
        }
    }

    /// 发送审阅更新事件
    pub fn emit_review_updated(&self, kb_id: &str, review_id: &str) {
        let payload = serde_json::json!({
            "kb_id": kb_id,
            "review_id": review_id,
        });
        if let Err(e) = self.app.emit("review-updated", payload) {
            log::error!("[EventBus] emit review-updated 失败: {}", e);
        }
    }

    /// 发送 Wiki 页面更新事件
    pub fn emit_wiki_updated(&self, kb_id: &str, page_path: &str) {
        let payload = serde_json::json!({
            "kb_id": kb_id,
            "page_path": page_path,
        });
        if let Err(e) = self.app.emit("wiki-updated", payload) {
            log::error!("[EventBus] emit wiki-updated 失败: {}", e);
        }
    }

    /// 发送 source 更新事件
    pub fn emit_source_updated(&self, kb_id: &str, source_id: &str) {
        let payload = serde_json::json!({
            "kb_id": kb_id,
            "source_id": source_id,
        });
        if let Err(e) = self.app.emit("source-updated", payload) {
            log::error!("[EventBus] emit source-updated 失败: {}", e);
        }
    }

    /// 发送知识库统计变更事件（供 Dashboard 实时刷新）
    pub fn emit_kb_stats_changed(&self, kb_id: &str) {
        let payload = serde_json::json!({
            "kb_id": kb_id,
            "timestamp": chrono::Utc::now().to_rfc3339(),
        });
        if let Err(e) = self.app.emit("kb-stats-changed", payload) {
            log::error!("[EventBus] emit kb-stats-changed 失败: {}", e);
        }
    }

    /// 发送 Agent 活动事件（供状态栏实时展示）
    pub fn emit_agent_activity(
        &self,
        agent_name: &str,
        status: &str,
        file_name: &str,
        detail: &str,
    ) {
        let payload = serde_json::json!({
            "agent_name": agent_name,
            "status": status,
            "file_name": file_name,
            "detail": detail,
            "timestamp": chrono::Utc::now().to_rfc3339(),
        });
        if let Err(e) = self.app.emit("agent-activity", payload) {
            log::error!("[EventBus] emit agent-activity 失败: {}", e);
        }
    }

    /// 发送向量库状态变更事件
    pub fn emit_vdb_status_changed(&self, status: &crate::embedding::vdb_status::VdbStatus) {
        if let Ok(payload) = serde_json::to_value(status) {
            if let Err(e) = self.app.emit("vdb-status-changed", payload) {
                log::error!("[EventBus] emit vdb-status-changed 失败: {}", e);
            }
        }
    }

    /// 发送重建索引进度事件
    pub fn emit_reindex_progress(&self, kb_id: &str, current: u64, total: u64, message: &str) {
        let payload = serde_json::json!({
            "kb_id": kb_id,
            "current": current,
            "total": total,
            "message": message,
            "timestamp": chrono::Utc::now().to_rfc3339(),
        });
        if let Err(e) = self.app.emit("reindex-progress", payload) {
            log::error!("[EventBus] emit reindex-progress 失败: {}", e);
        }
    }

    /// 发送 Agent 流水线状态变更事件（供任务详情页实时更新状态机）
    pub fn emit_agent_status_change(
        &self,
        task_id: &str,
        stage: &str,
        stage_status: &str,
        progress: f64,
        prompt_text: &str,
        response_text: &str,
        log_message: &str,
    ) {
        let payload = serde_json::json!({
            "task_id": task_id,
            "stage": stage,
            "stage_status": stage_status,
            "progress": progress,
            "prompt_text": prompt_text,
            "response_text": response_text,
            "log_message": log_message,
            "timestamp": chrono::Utc::now().to_rfc3339(),
        });
        if let Err(e) = self.app.emit("agent-status-change", payload) {
            log::error!("[EventBus] emit agent-status-change 失败: {}", e);
        }
    }

    /// 发送通用通知
    pub fn emit_notification(&self, level: &str, title: &str, message: &str) {
        let payload = serde_json::json!({
            "level": level,
            "title": title,
            "message": message,
            "timestamp": chrono::Utc::now().to_rfc3339(),
        });
        if let Err(e) = self.app.emit("notification", payload) {
            log::error!("[EventBus] emit notification 失败: {}", e);
        }
    }

    /// Agent 定义变更事件
    pub fn emit_agent_definition_changed(&self, action: &str, agent_name: &str) {
        let payload = serde_json::json!({
            "action": action,
            "agent_name": agent_name,
            "timestamp": chrono::Utc::now().to_rfc3339(),
        });
        if let Err(e) = self.app.emit("agent-definition-changed", payload) {
            log::error!("[EventBus] emit agent-definition-changed 失败: {}", e);
        }
    }

    /// Skill 定义变更事件
    pub fn emit_skill_definition_changed(&self, action: &str, skill_name: &str) {
        let payload = serde_json::json!({
            "action": action,
            "skill_name": skill_name,
            "timestamp": chrono::Utc::now().to_rfc3339(),
        });
        if let Err(e) = self.app.emit("skill-definition-changed", payload) {
            log::error!("[EventBus] emit skill-definition-changed 失败: {}", e);
        }
    }

    /// 健康监测快照事件（供前端健康仪表盘）
    pub fn emit_health_snapshot(&self, payload: &serde_json::Value) -> Result<(), String> {
        self.app
            .emit("health-snapshot", payload)
            .map_err(|e| format!("emit health-snapshot 失败: {}", e))
    }

    /// Agent 链式事件（带深度标记，供 AdminAgent 分发用）
    /// 携带 is_agent_action: true，事件总线消费者可据此过滤，防止级联循环触发。
    pub fn emit_agent_chain_event(
        &self,
        event_type: &str,
        payload: &serde_json::Value,
        source_agent: &str,
        depth: u32,
    ) {
        let wrapped = serde_json::json!({
            "event_type": event_type,
            "payload": payload,
            "source_agent": source_agent,
            "depth": depth,
            "is_agent_action": true,
            "timestamp": chrono::Utc::now().to_rfc3339(),
        });
        if let Err(e) = self.app.emit("agent-chain-event", &wrapped) {
            log::error!("[EventBus] emit agent-chain-event 失败: {}", e);
        }
    }
}
