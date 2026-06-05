// RecoveryCheck - 启动时恢复中断任务 + 数据完整性验证

use std::sync::Arc;
use crate::core::database_service::DatabaseService;
use crate::core::task_queue::TaskQueue;
use crate::recovery::workspace_reconcile::WorkspaceReconcile;

pub struct RecoveryCheck;

impl RecoveryCheck {
    pub fn run(
        db: &Arc<DatabaseService>,
        task_queue: &Arc<TaskQueue>,
        kb_id: &str,
        kb_path: &str,
    ) -> Result<Vec<String>, String> {
        let mut actions = Vec::new();

        // === 步骤 0: 检查 KB 工作区目录是否存在，若不存在则自动清理孤立 KB ===
        if !WorkspaceReconcile::check_kb_workspace_exists(db, kb_id, kb_path)? {
            match WorkspaceReconcile::purge_orphaned_kb(db, kb_id) {
                Ok(()) => {
                    actions.push(format!(
                        "知识库工作区目录已不存在 (路径: {})，已自动清理所有关联数据库记录",
                        kb_path
                    ));
                }
                Err(e) => {
                    log::error!("[recovery] 清理孤立 KB 失败 (kb={}): {}", kb_id, e);
                    actions.push(format!("清理孤立知识库失败: {}", e));
                }
            }
            // 已删除 KB，跳过后续所有检查
            return Ok(actions);
        }

        // === 步骤 0a: 验证 sources 文件存在性，清理失效条目 ===
        match WorkspaceReconcile::validate_sources(db, kb_id, kb_path) {
            Ok(count) => {
                if count > 0 {
                    actions.push(format!("已清理 {} 个失效的源文件索引（文件已不存在）", count));
                }
            }
            Err(e) => {
                log::error!("[recovery] 验证 sources 文件失败 (kb={}): {}", kb_id, e);
                actions.push(format!("源文件验证失败: {}", e));
            }
        }

        // === 步骤 0b: 清理 file_index 中指向不存在文件的条目 ===
        match WorkspaceReconcile::cleanup_stale_file_index(db, kb_id, kb_path) {
            Ok(count) => {
                if count > 0 {
                    actions.push(format!("已清理 {} 个失效的文件索引条目", count));
                }
            }
            Err(e) => {
                log::error!("[recovery] 清理 file_index 失败 (kb={}): {}", kb_id, e);
            }
        }

        // 恢复卡在 "applying" 状态的 review_items（进程崩溃导致）
        {
            let conn = db.connect()?;
            let stuck_count: i64 = conn.query_row(
                "SELECT COUNT(*) FROM review_items ri
                 JOIN reviews r ON ri.review_id = r.id
                 WHERE r.kb_id = ?1 AND ri.status = 'applying'",
                rusqlite::params![kb_id],
                |row| row.get(0),
            ).unwrap_or(0);

            if stuck_count > 0 {
                let now = chrono::Utc::now().to_rfc3339();
                conn.execute(
                    "UPDATE review_items SET status = 'pending', apply_error = '应用过程中断（进程崩溃或异常退出），已自动恢复为待处理', updated_at = ?1
                     WHERE id IN (SELECT ri.id FROM review_items ri JOIN reviews r ON ri.review_id = r.id WHERE r.kb_id = ?2 AND ri.status = 'applying')",
                    rusqlite::params![now, kb_id],
                ).map_err(|e| format!("恢复卡住的 review_items 失败: {}", e))?;

                actions.push(format!("发现 {} 个卡在\"applying\"状态的审阅项，已重置为 pending", stuck_count));

                // 同时更新对应的 review 状态为 pending（如果它是 completed 的）
                conn.execute(
                    "UPDATE reviews SET status = 'pending', updated_at = ?1
                     WHERE kb_id = ?2 AND status = 'completed'
                     AND id IN (SELECT DISTINCT ri.review_id FROM review_items ri WHERE ri.status = 'pending' AND ri.updated_at = ?1)",
                    rusqlite::params![now, kb_id],
                ).unwrap_or_else(|e| {
                    log::error!("[recovery] 更新 review 状态失败: {}", e);
                    0
                });
            }
        }

        let tasks = task_queue.get_interrupted_tasks(kb_id)?;

        for task in &tasks {
            match task.status.as_str() {
                "queued" | "locked" => {
                    task_queue.mark_interrupted(&task.id)?;
                    actions.push(format!("任务 {} ({}) 标记为中断，可重试", task.id, task.task_type));
                }
                "review_pending" => {
                    actions.push(format!("任务 {} 有待审阅，恢复审阅状态", task.id));
                }
                "applying" => {
                    // 检查 operation_id 是否已应用
                    task_queue.mark_interrupted(&task.id)?;
                    actions.push(format!("任务 {} 应用中断，标记为可恢复", task.id));
                }
                _ => {}
            }
        }

        Ok(actions)
    }
}
