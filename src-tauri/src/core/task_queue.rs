use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::collections::HashMap;
use std::sync::Mutex;
use crate::core::database_service::DatabaseService;
use crate::core::event_bus::EventBus;
use rusqlite::params;

/// 任务取消令牌
#[derive(Clone)]
pub struct CancellationToken {
    cancelled: Arc<AtomicBool>,
}

impl Default for CancellationToken {
    fn default() -> Self {
        Self::new()
    }
}

impl CancellationToken {
    pub fn new() -> Self {
        Self { cancelled: Arc::new(AtomicBool::new(false)) }
    }

    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Task {
    pub id: String,
    pub kb_id: String,
    pub task_type: String,
    pub task_name: String,
    pub status: String,
    pub current_agent: String,
    pub model_profile_id: String,
    pub input_ref: String,
    pub output_ref: String,
    pub review_id: String,
    pub error_message: String,
    pub failure_reason: String,
    pub recoverable: bool,
    pub resume_from_stage: String,
    pub last_success_stage: String,
    pub next_action: String,
    pub retry_count: i32,
    pub cancel_reason: String,
    pub created_at: String,
    pub updated_at: String,
    pub locked_at: Option<String>,
    pub completed_at: Option<String>,
    pub archived_at: Option<String>,
    pub handled_at: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TaskEvent {
    pub id: String,
    pub task_id: String,
    pub event_type: String,
    pub agent_name: String,
    pub message: String,
    pub created_at: String,
}

pub struct TaskQueue {
    db: Arc<DatabaseService>,
    event_bus: Arc<EventBus>,
    cancellation_tokens: Arc<Mutex<HashMap<String, CancellationToken>>>,
}

impl TaskQueue {
    pub fn new(db: Arc<DatabaseService>, event_bus: Arc<EventBus>) -> Self {
        Self {
            db,
            event_bus,
            cancellation_tokens: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// 为任务创建取消令牌
    pub fn create_cancellation_token(&self, task_id: &str) -> CancellationToken {
        let token = CancellationToken::new();
        let mut tokens = self.cancellation_tokens.lock().unwrap_or_else(|e| e.into_inner());
        tokens.insert(task_id.to_string(), token.clone());
        token
    }

    /// 获取任务的取消令牌
    pub fn get_cancellation_token(&self, task_id: &str) -> Option<CancellationToken> {
        let tokens = self.cancellation_tokens.lock().unwrap_or_else(|e| e.into_inner());
        tokens.get(task_id).cloned()
    }

    /// 移除任务的取消令牌
    pub fn remove_cancellation_token(&self, task_id: &str) {
        let mut tokens = self.cancellation_tokens.lock().unwrap_or_else(|e| e.into_inner());
        tokens.remove(task_id);
    }

    /// 创建新任务
    pub fn create_task(
        &self,
        kb_id: &str,
        task_type: &str,
        input_ref: &str,
    ) -> Result<Task, String> {
        let conn = self.db.connect()?;
        let id = format!("task_{}", chrono::Utc::now().format("%Y%m%d%H%M%S%3f"));
        let now = chrono::Utc::now().to_rfc3339();

        // 从 source 的 file_name 推导 task_name
        let task_name = match conn.query_row(
            "SELECT file_name FROM sources WHERE id = ?1",
            params![input_ref],
            |row| row.get::<_, String>(0),
        ) {
            Ok(name) => name,
            Err(_) => String::new(),
        };

        conn.execute(
            "INSERT INTO tasks (id, kb_id, task_type, task_name, status, current_agent, input_ref, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, 'created', '', ?5, ?6, ?6)",
            params![id, kb_id, task_type, task_name, input_ref, now],
        )
        .map_err(|e| format!("创建任务失败: {}", e))?;

        let task = self.get_task(&id)?;

        self.add_event(&id, "task_created", "", &format!("任务已创建: {}", task_type))?;
        self.event_bus.emit_task_updated(kb_id, &task);

        Ok(task)
    }

    /// 更新任务状态（参数化查询，安全）
    pub fn update_task_status(
        &self,
        task_id: &str,
        status: &str,
        agent_name: &str,
        error_message: &str,
    ) -> Result<(), String> {
        let conn = self.db.connect()?;
        let now = chrono::Utc::now().to_rfc3339();

        let sql = match status {
            "locked" => {
                conn.execute(
                    "UPDATE tasks SET status = ?1, current_agent = ?2, error_message = ?3, updated_at = ?4, locked_at = ?4 WHERE id = ?5",
                    rusqlite::params![status, agent_name, error_message, now, task_id],
                )
            }
            "applied" | "failed" | "cancelled" => {
                conn.execute(
                    "UPDATE tasks SET status = ?1, current_agent = ?2, error_message = ?3, updated_at = ?4, completed_at = ?4 WHERE id = ?5",
                    rusqlite::params![status, agent_name, error_message, now, task_id],
                )
            }
            _ => {
                conn.execute(
                    "UPDATE tasks SET status = ?1, current_agent = ?2, error_message = ?3, updated_at = ?4 WHERE id = ?5",
                    rusqlite::params![status, agent_name, error_message, now, task_id],
                )
            }
        };

        sql.map_err(|e| format!("更新任务状态失败: {}", e))?;

        let event_msg = match status {
            "failed" => format!("任务失败: {}", error_message),
            _ => format!("状态更新: {} -> {}", agent_name, status),
        };

        self.add_event(task_id, "status_change", agent_name, &event_msg)?;

        if let Ok(task) = self.get_task(task_id) {
            self.event_bus.emit_task_updated(&task.kb_id, &task);
        }

        Ok(())
    }

    /// 更新任务输出引用
    pub fn update_output_ref(&self, task_id: &str, output_ref: &str) -> Result<(), String> {
        let conn = self.db.connect()?;
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE tasks SET output_ref = ?1, updated_at = ?2 WHERE id = ?3",
            params![output_ref, now, task_id],
        )
        .map_err(|e| format!("更新输出引用失败: {}", e))?;
        Ok(())
    }

    /// 设置任务显示名称
    pub fn set_task_name(&self, task_id: &str, name: &str) -> Result<(), String> {
        let conn = self.db.connect()?;
        conn.execute(
            "UPDATE tasks SET task_name = ?1 WHERE id = ?2",
            params![name, task_id],
        )
        .map_err(|e| format!("更新任务名称失败: {}", e))?;
        Ok(())
    }

    /// 设置任务关联的审阅 ID
    pub fn set_review_id(&self, task_id: &str, review_id: &str) -> Result<(), String> {
        let conn = self.db.connect()?;
        conn.execute(
            "UPDATE tasks SET review_id = ?1 WHERE id = ?2",
            params![review_id, task_id],
        )
        .map_err(|e| format!("更新审阅关联失败: {}", e))?;
        Ok(())
    }

    /// 添加任务事件
    pub fn add_event(
        &self,
        task_id: &str,
        event_type: &str,
        agent_name: &str,
        message: &str,
    ) -> Result<TaskEvent, String> {
        let conn = self.db.connect()?;
        let id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();

        conn.execute(
            "INSERT INTO task_events (id, task_id, event_type, agent_name, message, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![id, task_id, event_type, agent_name, message, now],
        )
        .map_err(|e| format!("添加事件失败: {}", e))?;

        self.event_bus.emit_task_event(task_id, event_type, agent_name, message);

        Ok(TaskEvent {
            id,
            task_id: task_id.to_string(),
            event_type: event_type.to_string(),
            agent_name: agent_name.to_string(),
            message: message.to_string(),
            created_at: now,
        })
    }

    /// 获取任务详情
    pub fn get_task(&self, task_id: &str) -> Result<Task, String> {
        let conn = self.db.connect()?;
        conn.query_row(
            "SELECT t.id, t.kb_id, t.task_type, COALESCE(t.task_name,''), t.status, t.current_agent, t.model_profile_id,
                    t.input_ref, t.output_ref, COALESCE(t.review_id,''),
                    t.error_message,
                    COALESCE(t.failure_reason,''), COALESCE(t.recoverable,0),
                    COALESCE(t.resume_from_stage,''), COALESCE(t.last_success_stage,''),
                    COALESCE(t.next_action,''),
                    t.retry_count,
                    COALESCE(t.cancel_reason,''),
                    t.created_at, t.updated_at, t.locked_at, t.completed_at,
                    t.archived_at, t.handled_at
             FROM tasks t WHERE t.id = ?1",
            params![task_id],
            |row| {
                Ok(Task {
                    id: row.get(0)?,
                    kb_id: row.get(1)?,
                    task_type: row.get(2)?,
                    task_name: row.get(3)?,
                    status: row.get(4)?,
                    current_agent: row.get(5)?,
                    model_profile_id: row.get(6)?,
                    input_ref: row.get(7)?,
                    output_ref: row.get(8)?,
                    review_id: row.get(9)?,
                    error_message: row.get(10)?,
                    failure_reason: row.get(11)?,
                    recoverable: row.get::<_, i32>(12)? != 0,
                    resume_from_stage: row.get(13)?,
                    last_success_stage: row.get(14)?,
                    next_action: row.get(15)?,
                    retry_count: row.get(16)?,
                    cancel_reason: row.get(17)?,
                    created_at: row.get(18)?,
                    updated_at: row.get(19)?,
                    locked_at: row.get(20)?,
                    completed_at: row.get(21)?,
                    archived_at: row.get(22)?,
                    handled_at: row.get(23)?,
                })
            },
        )
        .map_err(|e| format!("获取任务失败: {}", e))
    }

    /// 获取任务事件列表
    pub fn get_task_events(&self, task_id: &str) -> Result<Vec<TaskEvent>, String> {
        let conn = self.db.connect()?;
        let mut stmt = conn
            .prepare(
                "SELECT id, task_id, event_type, agent_name, message, created_at
                 FROM task_events WHERE task_id = ?1 ORDER BY created_at ASC",
            )
            .map_err(|e| format!("查询事件失败: {}", e))?;

        let events = stmt
            .query_map(params![task_id], |row| {
                Ok(TaskEvent {
                    id: row.get(0)?,
                    task_id: row.get(1)?,
                    event_type: row.get(2)?,
                    agent_name: row.get(3)?,
                    message: row.get(4)?,
                    created_at: row.get(5)?,
                })
            })
            .map_err(|e| format!("映射事件失败: {}", e))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("收集事件失败: {}", e))?;

        Ok(events)
    }

    /// 获取知识库所有任务
    pub fn list_tasks(&self, kb_id: &str) -> Result<Vec<Task>, String> {
        let conn = self.db.connect()?;
        let mut stmt = conn
            .prepare(
                "SELECT id, kb_id, task_type, COALESCE(task_name,''), status, current_agent, model_profile_id,
                        input_ref, output_ref, COALESCE(review_id,''),
                        error_message,
                        COALESCE(failure_reason,''), COALESCE(recoverable,0),
                        COALESCE(resume_from_stage,''), COALESCE(last_success_stage,''),
                        COALESCE(next_action,''),
                        retry_count,
                        COALESCE(cancel_reason,''),
                        created_at, updated_at, locked_at, completed_at, archived_at, handled_at
                 FROM tasks WHERE kb_id = ?1 ORDER BY created_at DESC",
            )
            .map_err(|e| format!("查询任务列表失败: {}", e))?;

        let tasks = stmt
            .query_map(params![kb_id], |row| {
                Ok(Task {
                    id: row.get(0)?,
                    kb_id: row.get(1)?,
                    task_type: row.get(2)?,
                    task_name: row.get(3)?,
                    status: row.get(4)?,
                    current_agent: row.get(5)?,
                    model_profile_id: row.get(6)?,
                    input_ref: row.get(7)?,
                    output_ref: row.get(8)?,
                    review_id: row.get(9)?,
                    error_message: row.get(10)?,
                    failure_reason: row.get(11)?,
                    recoverable: row.get::<_, i32>(12)? != 0,
                    resume_from_stage: row.get(13)?,
                    last_success_stage: row.get(14)?,
                    next_action: row.get(15)?,
                    retry_count: row.get(16)?,
                    cancel_reason: row.get(17)?,
                    created_at: row.get(18)?,
                    updated_at: row.get(19)?,
                    locked_at: row.get(20)?,
                    completed_at: row.get(21)?,
                    archived_at: row.get(22)?,
                    handled_at: row.get(23)?,
                })
            })
            .map_err(|e| format!("映射任务失败: {}", e))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("收集任务失败: {}", e))?;

        Ok(tasks)
    }

    /// 按状态过滤任务列表
    pub fn list_tasks_filtered(&self, kb_id: &str, status_filter: &str) -> Result<Vec<Task>, String> {
        let conn = self.db.connect()?;
        let sql = match status_filter {
            "all" => "SELECT id FROM tasks WHERE kb_id = ?1 AND archived_at IS NULL ORDER BY created_at DESC".to_string(),
            "running" => "SELECT id FROM tasks WHERE kb_id = ?1 AND status IN ('queued','running','locked','prompt_built','sent_to_model','model_returned','json_validating','json_valid','json_repaired','candidate_searching','resolution_running','relationship_running','update_plan_generating','applying') AND archived_at IS NULL ORDER BY created_at DESC".to_string(),
            "pending_review" => "SELECT id FROM tasks WHERE kb_id = ?1 AND status IN ('review_pending','review_generating') AND archived_at IS NULL ORDER BY created_at DESC".to_string(),
            "failed" => "SELECT id FROM tasks WHERE kb_id = ?1 AND status IN ('failed', 'pipeline_failed') AND handled_at IS NULL AND archived_at IS NULL ORDER BY created_at DESC".to_string(),
            "cancelled" => "SELECT id FROM tasks WHERE kb_id = ?1 AND status IN ('cancelled', 'cancelled_after_model_return', 'cancelling') AND archived_at IS NULL ORDER BY created_at DESC".to_string(),
            "archived" => "SELECT id FROM tasks WHERE kb_id = ?1 AND archived_at IS NOT NULL ORDER BY created_at DESC".to_string(),
            _ => "SELECT id FROM tasks WHERE kb_id = ?1 AND archived_at IS NULL ORDER BY created_at DESC".to_string(),
        };

        let mut stmt = conn.prepare(&sql).map_err(|e| format!("查询失败: {}", e))?;
        let ids: Vec<String> = stmt
            .query_map(params![kb_id], |row| row.get(0))
            .map_err(|e| format!("映射失败: {}", e))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("读取任务 ID 列表失败: {}", e))?;

        let mut tasks = Vec::new();
        for id in ids {
            if let Ok(task) = self.get_task(&id) {
                tasks.push(task);
            }
        }
        Ok(tasks)
    }

    /// 获取未处理失败任务数
    pub fn get_unhandled_failed_count(&self, kb_id: &str) -> Result<i64, String> {
        let conn = self.db.connect()?;
        let count: i64 = match conn.query_row(
            "SELECT COUNT(*) FROM tasks WHERE kb_id = ?1 AND status IN ('failed', 'pipeline_failed', 'interrupted') AND handled_at IS NULL AND archived_at IS NULL",
            params![kb_id],
            |row| row.get(0),
        ) {
            Ok(c) => c,
            Err(rusqlite::Error::QueryReturnedNoRows) => 0,
            Err(e) => return Err(format!("查询未处理失败任务数失败: {}", e)),
        };
        Ok(count)
    }

    /// 获取中断状态的任务
    pub fn get_interrupted_tasks(&self, kb_id: &str) -> Result<Vec<Task>, String> {
        let conn = self.db.connect()?;
        let mut stmt = conn
            .prepare(
                "SELECT id, kb_id, task_type, COALESCE(task_name,''), status, current_agent, model_profile_id,
                        input_ref, output_ref, COALESCE(review_id,''),
                        error_message,
                        COALESCE(failure_reason,''), COALESCE(recoverable,0),
                        COALESCE(resume_from_stage,''), COALESCE(last_success_stage,''),
                        COALESCE(next_action,''),
                        retry_count,
                        COALESCE(cancel_reason,''),
                        created_at, updated_at, locked_at, completed_at, archived_at, handled_at
                 FROM tasks WHERE kb_id = ?1 AND status IN ('queued', 'locked', 'applying', 'interrupted', 'review_pending')
                 AND archived_at IS NULL
                 ORDER BY created_at DESC",
            )
            .map_err(|e| format!("查询中断任务失败: {}", e))?;

        let tasks = stmt
            .query_map(params![kb_id], |row| {
                Ok(Task {
                    id: row.get(0)?,
                    kb_id: row.get(1)?,
                    task_type: row.get(2)?,
                    task_name: row.get(3)?,
                    status: row.get(4)?,
                    current_agent: row.get(5)?,
                    model_profile_id: row.get(6)?,
                    input_ref: row.get(7)?,
                    output_ref: row.get(8)?,
                    review_id: row.get(9)?,
                    error_message: row.get(10)?,
                    failure_reason: row.get(11)?,
                    recoverable: row.get::<_, i32>(12)? != 0,
                    resume_from_stage: row.get(13)?,
                    last_success_stage: row.get(14)?,
                    next_action: row.get(15)?,
                    retry_count: row.get(16)?,
                    cancel_reason: row.get(17)?,
                    created_at: row.get(18)?,
                    updated_at: row.get(19)?,
                    locked_at: row.get(20)?,
                    completed_at: row.get(21)?,
                    archived_at: row.get(22)?,
                    handled_at: row.get(23)?,
                })
            })
            .map_err(|e| format!("映射中断任务失败: {}", e))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("收集中断任务失败: {}", e))?;

        Ok(tasks)
    }

    /// 归档任务
    pub fn archive_task(&self, task_id: &str) -> Result<(), String> {
        let conn = self.db.connect()?;
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE tasks SET archived_at = ?1, updated_at = ?1 WHERE id = ?2",
            params![now, task_id],
        )
        .map_err(|e| format!("归档任务失败: {}", e))?;
        self.add_event(task_id, "task_archived", "", "任务已归档")?;
        Ok(())
    }

    /// 标记失败任务为已处理
    pub fn handle_failed_task(&self, task_id: &str) -> Result<(), String> {
        let conn = self.db.connect()?;
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE tasks SET handled_at = ?1, updated_at = ?1 WHERE id = ?2",
            params![now, task_id],
        )
        .map_err(|e| format!("处理失败任务失败: {}", e))?;
        self.add_event(task_id, "task_handled", "", "失败任务已标记为已处理")?;
        Ok(())
    }

    /// 更新任务失败原因
    pub fn update_failure_reason(
        &self,
        task_id: &str,
        failure_reason: &str,
        recoverable: bool,
        resume_from_stage: &str,
        last_success_stage: &str,
        next_action: &str,
    ) -> Result<(), String> {
        let conn = self.db.connect()?;
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE tasks SET failure_reason = ?1, recoverable = ?2, resume_from_stage = ?3, last_success_stage = ?4, next_action = ?5, updated_at = ?6 WHERE id = ?7",
            params![failure_reason, recoverable as i32, resume_from_stage, last_success_stage, next_action, now, task_id],
        )
        .map_err(|e| format!("更新失败原因失败: {}", e))?;
        Ok(())
    }

    /// 重试任务
    pub fn retry_task(&self, task_id: &str) -> Result<(), String> {
        let conn = self.db.connect()?;
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE tasks SET status = 'queued', retry_count = retry_count + 1, \
             error_message = '', cancel_flag = 0, cancel_reason = '', \
             updated_at = ?1, completed_at = NULL WHERE id = ?2",
            params![now, task_id],
        )
        .map_err(|e| format!("重试任务失败: {}", e))?;

        self.add_event(task_id, "task_retried", "", "任务已重新入队（cancel_flag 已清除）")?;
        Ok(())
    }

    /// 取消任务（v0.2.2: 持久化取消标志到 DB，确保跨 TaskQueue 实例可见）
    pub fn cancel_task(&self, task_id: &str) -> Result<(), String> {
        let now = chrono::Utc::now().to_rfc3339();

        // 1. 尝试设置内存中的取消令牌（如果当前实例有）
        if let Some(token) = self.get_cancellation_token(task_id) {
            token.cancel();
        }

        // 2. 读取当前状态，判断是否有活跃的 pipeline 运行器
        let conn = self.db.connect()?;
        let current_status: String = conn.query_row(
            "SELECT status FROM tasks WHERE id = ?1",
            params![task_id],
            |row| row.get(0),
        ).map_err(|e| format!("读取任务状态失败: {}", e))?;

        // 终态任务不可取消
        let terminal = ["applied", "failed", "pipeline_failed", "cancelled", "cancelled_after_model_return", "interrupted"];
        if terminal.contains(&current_status.as_str()) {
            return Err("任务已处于终态，无法取消".to_string());
        }

        // 无活跃 pipeline 运行器的状态 → 直接标记 cancelled（避免永久卡在 cancelling）
        // review_generating：pipeline 已完成评阅生成，无运行器监听 cancel_flag
        let no_runner_states = ["queued", "review_pending", "review_generating"];
        if no_runner_states.contains(&current_status.as_str()) {
            conn.execute(
                "UPDATE tasks SET status = 'cancelled', cancel_flag = 1, cancel_reason = 'user_cancelled', updated_at = ?1, completed_at = ?2 WHERE id = ?3",
                params![now, now, task_id],
            ).map_err(|e| format!("取消任务失败: {}", e))?;
            self.add_event(task_id, "task_cancelled", "", "任务已取消（无活跃运行器，直接终止）")?;
            self.remove_cancellation_token(task_id);
            return Ok(());
        }

        // 有活跃 pipeline 运行器 → 设置 cancelling 标志，让 pipeline 检测后调用 mark_cancelled
        conn.execute(
            "UPDATE tasks SET status = 'cancelling', cancel_flag = 1, cancel_reason = 'user_cancelled', updated_at = ?1 \
             WHERE id = ?2",
            params![now, task_id],
        ).map_err(|e| format!("取消任务失败: {}", e))?;

        self.add_event(task_id, "cancellation_requested", "", "用户请求取消任务，等待 Pipeline 响应")?;
        Ok(())
    }

    /// v0.2.2: 检查任务是否已被取消（同时检查内存令牌和 DB 持久化标志）
    pub fn is_task_cancelled(&self, task_id: &str) -> bool {
        if let Some(token) = self.get_cancellation_token(task_id) {
            if token.is_cancelled() {
                return true;
            }
        }
        self.check_cancel_flag_db(task_id)
    }

    /// v0.2.2: 从数据库检查 cancel_flag
    fn check_cancel_flag_db(&self, task_id: &str) -> bool {
        let conn = match self.db.connect() {
            Ok(c) => c,
            Err(_) => return false,
        };
        let flag: i32 = match conn.query_row(
            "SELECT COALESCE(cancel_flag, 0) FROM tasks WHERE id = ?1",
            params![task_id],
            |row| row.get(0),
        ) {
            Ok(f) => f,
            Err(rusqlite::Error::QueryReturnedNoRows) => 0,
            Err(e) => { log::error!("[task_queue] 查询 cancel_flag 失败 (task={}): {}", task_id, e); 0 }
        };
        flag != 0
    }

    /// 将任务标记为已取消（由 Pipeline 在检测到取消后调用）
    pub fn mark_cancelled(&self, task_id: &str) -> Result<(), String> {
        self.update_task_status(task_id, "cancelled", "", "用户取消，Pipeline 已停止")
    }

    /// 标记任务为 cancelled_after_model_return
    pub fn mark_cancelled_after_model(&self, task_id: &str) -> Result<(), String> {
        let now = chrono::Utc::now().to_rfc3339();
        let conn = self.db.connect()?;
        conn.execute(
            "UPDATE tasks SET status = 'cancelled_after_model_return', cancel_flag = 1, cancel_reason = '模型返回后已丢弃结果', updated_at = ?1, completed_at = ?2 WHERE id = ?3",
            params![now, now, task_id],
        ).map_err(|e| format!("标记任务状态失败: {}", e))?;
        self.add_event(task_id, "cancelled_after_model", "", "模型已返回但结果被丢弃（任务已取消）")?;
        // 移除取消令牌
        self.remove_cancellation_token(task_id);
        Ok(())
    }

    /// 启动恢复：清理卡在 cancelling 状态的任务（无活跃 pipeline 运行器时直接取消）
    pub fn cleanup_stuck_cancelling(&self) -> Result<usize, String> {
        let conn = self.db.connect()?;
        let now = chrono::Utc::now().to_rfc3339();
        let count = conn.execute(
            "UPDATE tasks SET status = 'cancelled', completed_at = ?1, updated_at = ?2 WHERE status = 'cancelling'",
            params![now, now],
        ).map_err(|e| format!("清理卡住的取消任务失败: {}", e))?;
        if count > 0 {
            log::info!("[recovery] 清理了 {} 个卡在 cancelling 状态的任务", count);
        }
        Ok(count)
    }

    /// 标记为中断
    pub fn mark_interrupted(&self, task_id: &str) -> Result<(), String> {
        self.update_task_status(task_id, "interrupted", "", "应用中断，任务标记为可恢复")
    }

    /// 恢复中断任务
    /// 若审阅项已生成则进入 review_pending，否则回 queued 重跑流水线（断点续传）
    pub fn resume_task(&self, task_id: &str) -> Result<(), String> {
        let conn = self.db.connect()?;
        let has_reviews: bool = match conn.query_row(
            "SELECT COUNT(1) > 0 FROM reviews WHERE task_id = ?1",
            rusqlite::params![task_id],
            |row| row.get(0),
        ) {
            Ok(v) => v,
            Err(rusqlite::Error::QueryReturnedNoRows) => false,
            Err(e) => return Err(format!("查询任务审阅状态失败: {}", e)),
        };

        if has_reviews {
            self.update_task_status(task_id, "review_pending", "", "审阅项已存在，恢复至待审阅状态")
        } else {
            let now = chrono::Utc::now().to_rfc3339();
            conn.execute(
                "UPDATE tasks SET status = 'queued', cancel_flag = 0, cancel_reason = '', \
                 error_message = '从中断状态恢复', updated_at = ?1, completed_at = NULL WHERE id = ?2",
                rusqlite::params![now, task_id],
            ).map_err(|e| format!("恢复任务失败: {}", e))?;
            Ok(())
        }
    }
}
