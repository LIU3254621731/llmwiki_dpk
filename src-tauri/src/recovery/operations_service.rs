use std::sync::Arc;
use crate::core::database_service::DatabaseService;

pub struct OperationsService;

impl Default for OperationsService {
    fn default() -> Self {
        Self::new()
    }
}

impl OperationsService {
    pub fn new() -> Self {
        Self
    }

    pub fn check_and_record_operation(
        db: &Arc<DatabaseService>,
        kb_id: &str,
        task_id: &str,
        target_path: &str,
        content: &str,
    ) -> Result<bool, String> {
        let operation_hash = Self::compute_operation_hash(target_path, content);

        let conn = db.connect()?;

        let existing: Option<String> = match conn.query_row(
            "SELECT status FROM operations WHERE kb_id = ?1 AND operation_hash = ?2",
            rusqlite::params![kb_id, operation_hash],
            |row| row.get(0),
        ) {
            Ok(s) => Some(s),
            Err(rusqlite::Error::QueryReturnedNoRows) => None,
            Err(e) => return Err(format!("查询操作记录失败: {}", e)),
        };

        if let Some(status) = existing {
            if status == "completed" {
                return Ok(false);
            }
            return Ok(true);
        }

        let op_id = uuid::Uuid::new_v4().to_string();
        let _now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO operations (id, kb_id, task_id, operation_hash, target_path, status, applied_at)
             VALUES (?1, ?2, ?3, ?4, ?5, 'pending', NULL)",
            rusqlite::params![op_id, kb_id, task_id, operation_hash, target_path],
        ).map_err(|e| format!("记录操作失败: {}", e))?;

        Ok(true)
    }

    pub fn mark_completed(
        db: &Arc<DatabaseService>,
        kb_id: &str,
        _task_id: &str,
        target_path: &str,
        content: &str,
    ) -> Result<(), String> {
        let operation_hash = Self::compute_operation_hash(target_path, content);
        let now = chrono::Utc::now().to_rfc3339();

        let conn = db.connect()?;
        conn.execute(
            "UPDATE operations SET status = 'completed', applied_at = ?1 WHERE kb_id = ?2 AND operation_hash = ?3",
            rusqlite::params![now, kb_id, operation_hash],
        ).map_err(|e| format!("标记操作完成失败: {}", e))?;

        Ok(())
    }

    pub fn is_already_applied(
        db: &Arc<DatabaseService>,
        kb_id: &str,
        target_path: &str,
        content: &str,
    ) -> Result<bool, String> {
        let operation_hash = Self::compute_operation_hash(target_path, content);

        let conn = db.connect()?;
        let status: Option<String> = match conn.query_row(
            "SELECT status FROM operations WHERE kb_id = ?1 AND operation_hash = ?2",
            rusqlite::params![kb_id, operation_hash],
            |row| row.get(0),
        ) {
            Ok(s) => Some(s),
            Err(rusqlite::Error::QueryReturnedNoRows) => None,
            Err(e) => return Err(format!("查询操作状态失败: {}", e)),
        };

        Ok(status.as_deref() == Some("completed"))
    }

    fn compute_operation_hash(target_path: &str, content: &str) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        target_path.hash(&mut hasher);
        content.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }
}