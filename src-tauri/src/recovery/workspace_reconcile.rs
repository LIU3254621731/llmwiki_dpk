// WorkspaceReconcile - 检查数据一致性并修复（v0.1.4 增强版）
// v0.1.4: 改进 wiki/wiki 路径检测和自动修复

use std::sync::Arc;
use crate::core::database_service::DatabaseService;
use crate::wiki::path_service::PathService;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ReconcileReport {
    pub issues: Vec<ReconcileIssue>,
    pub ok_items: Vec<String>,
    pub fixed_count: usize,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ReconcileIssue {
    pub severity: String,
    pub category: String,
    pub description: String,
    pub suggestion: String,
    pub fixable: bool,
    pub detail: serde_json::Value,
}

pub struct WorkspaceReconcile;

impl WorkspaceReconcile {
    pub fn run(
        db: &Arc<DatabaseService>,
        kb_id: &str,
        kb_path: &str,
    ) -> Result<ReconcileReport, String> {
        let conn = db.connect()?;
        let mut issues = Vec::new();
        let mut ok_items = Vec::new();

        let workspace_root = std::path::PathBuf::from(kb_path);
        let wiki_dir = workspace_root.join("wiki");

        // 1. 检查 wiki_pages 表中记录的页面文件是否存在
        let mut stmt = conn
            .prepare("SELECT id, path, title, canonical_name FROM wiki_pages WHERE kb_id = ?1")
            .map_err(|e| format!("查询页面失败: {}", e))?;

        let page_rows: Vec<(String, String, String, String)> = stmt
            .query_map(rusqlite::params![kb_id], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
            })
            .map_err(|e| format!("映射页面失败: {}", e))?
            .filter_map(|r| r.ok())
            .collect();

        let mut has_broken = false;

        for (page_id, path, title, canonical_name) in &page_rows {
            // v0.1.4: 检查 wiki/wiki 重复路径
            if path.contains("wiki/wiki/") {
                let repaired = PathService::strip_duplicate_wiki_prefix(path);
                issues.push(ReconcileIssue {
                    severity: "error".to_string(),
                    category: "wiki_wiki_path".to_string(),
                    description: format!("检测到 wiki/wiki 重复路径: {} → {}", path, repaired),
                    suggestion: "可自动修复：去除重复的 wiki/ 前缀".to_string(),
                    fixable: true,
                    detail: serde_json::json!({
                        "action": "repair_path",
                        "page_id": page_id,
                        "old_path": path,
                        "new_path": repaired,
                    }),
                });
            }

            let normalized = PathService::normalize_workspace_path(path);

            // 检查路径是否已修复但仍不规范
            if &normalized != path && !path.contains("wiki/wiki/") {
                let repaired = PathService::repair_path(path);
                issues.push(ReconcileIssue {
                    severity: "warning".to_string(),
                    category: "bad_path".to_string(),
                    description: format!("路径不规范: {} → {}", path, repaired),
                    suggestion: "可自动修复".to_string(),
                    fixable: true,
                    detail: serde_json::json!({
                        "action": "repair_path",
                        "page_id": page_id,
                        "old_path": path,
                        "new_path": repaired,
                    }),
                });
            }

            let full_path = PathService::resolve_workspace_path(&workspace_root, &normalized);
            if !full_path.exists() {
                // 检查是否有版本快照可恢复
                let has_snapshot: bool = match conn.query_row(
                    "SELECT COUNT(*) FROM versions WHERE kb_id = ?1 AND page_path = ?2",
                    rusqlite::params![kb_id, normalized],
                    |row| row.get::<_, i64>(0),
                ) {
                    Ok(c) => c,
                    Err(rusqlite::Error::QueryReturnedNoRows) => 0,
                    Err(e) => return Err(format!("查询版本快照失败 (path={}): {}", normalized, e)),
                } > 0;

                // 检查是否有审阅项可重新应用
                let has_review_item: bool = match conn.query_row(
                    "SELECT COUNT(*) FROM review_items ri JOIN reviews r ON ri.review_id = r.id WHERE r.kb_id = ?1 AND ri.target_path = ?2",
                    rusqlite::params![kb_id, normalized],
                    |row| row.get::<_, i64>(0),
                ) {
                    Ok(c) => c,
                    Err(rusqlite::Error::QueryReturnedNoRows) => 0,
                    Err(e) => return Err(format!("查询审阅项失败 (path={}): {}", normalized, e)),
                } > 0;

                has_broken = true;

                let mut suggestion = if has_snapshot {
                    "可从版本快照恢复".to_string()
                } else {
                    String::new()
                };

                if has_review_item {
                    if !suggestion.is_empty() { suggestion.push_str(" 或 "); }
                    suggestion.push_str("可重新应用审阅项");
                }

                if suggestion.is_empty() {
                    suggestion = "无恢复来源，建议删除数据库记录".to_string();
                }

                let fixable = true;

                issues.push(ReconcileIssue {
                    severity: "error".to_string(),
                    category: "missing_file".to_string(),
                    description: format!("数据库记录的页面文件不存在: {} (标题: {})", normalized, title),
                    suggestion,
                    fixable,
                    detail: serde_json::json!({
                        "action": if has_snapshot { "recover_from_snapshot" } else if has_review_item { "reapply_review" } else { "delete_record" },
                        "page_id": page_id,
                        "path": normalized,
                        "title": title,
                        "canonical_name": canonical_name,
                        "has_snapshot": has_snapshot,
                        "has_review_item": has_review_item,
                    }),
                });
            }
        }

        if !has_broken && !page_rows.is_empty() {
            ok_items.push("所有数据库记录的页面文件均存在".to_string());
        }

        // 2. 检查 Markdown 文件是否有数据库记录
        if let Err(e) = Self::scan_wiki_dir(&wiki_dir, &wiki_dir, kb_id, &conn, &mut issues, &mut ok_items) {
            log::error!("[workspace_reconcile] 扫描 wiki 目录失败 (dir={}): {}", wiki_dir.display(), e);
        }

        let auto_fixable = issues.iter().filter(|i| i.fixable).count();
        Ok(ReconcileReport { issues, ok_items, fixed_count: auto_fixable })
    }

    /// 从版本快照恢复缺失页面
    pub fn recover_from_snapshot(
        db: &Arc<DatabaseService>,
        kb_id: &str,
        kb_path: &str,
        page_path: &str,
    ) -> Result<(), String> {
        let conn = db.connect()?;
        let wiki_dir = std::path::PathBuf::from(kb_path).join("wiki");
        let normalized = PathService::normalize(page_path);

        let (snapshot_name, version_id): (String, String) = conn.query_row(
            "SELECT snapshot_path, id FROM versions WHERE kb_id = ?1 AND page_path = ?2 ORDER BY created_at DESC LIMIT 1",
            rusqlite::params![kb_id, normalized],
            |row| Ok((row.get(0)?, row.get(1)?)),
        ).map_err(|_| format!("未找到页面 '{}' 的版本快照", normalized))?;

        let versions_dir = std::path::PathBuf::from(kb_path).join("versions").join("snapshots");
        let snapshot_file = versions_dir.join(&snapshot_name);

        if !snapshot_file.exists() {
            return Err(format!("版本快照文件不存在: {}", snapshot_name));
        }

        let content = std::fs::read_to_string(&snapshot_file)
            .map_err(|e| format!("读取快照文件失败: {}", e))?;

        let workspace_root = wiki_dir.parent().unwrap_or(&wiki_dir);
        let target_file = PathService::resolve_workspace_path(workspace_root, &normalized);
        if let Some(parent) = target_file.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("创建恢复目录失败: {}", e))?;
        }

        std::fs::write(&target_file, &content)
            .map_err(|e| format!("恢复页面文件失败: {}", e))?;

        // 更新 content_hash
        let hash = PathService::content_hash(&content);
        let now = chrono::Utc::now().format("%Y-%m-%d").to_string();
        conn.execute(
            "UPDATE wiki_pages SET content_hash = ?1, updated_at = ?2 WHERE kb_id = ?3 AND path = ?4",
            rusqlite::params![hash, now, kb_id, normalized],
        ).map_err(|e| format!("更新页面 hash 失败: {}", e))?;

        log::info!("[Reconcile] 从快照恢复页面: {} (版本: {})", normalized, version_id);
        Ok(())
    }

    /// 批量修复所有可修复的路径问题
    pub fn repair_all(db: &Arc<DatabaseService>, kb_id: &str, kb_path: &str) -> Result<usize, String> {
        let mut fixed = 0usize;

        // 修复 wiki_pages 路径
        fixed += PathService::repair_all_paths(db, kb_id)?;

        let sync_report = crate::wiki::markdown_indexer::MarkdownIndexer::sync_workspace(db, kb_id, kb_path)?;
        fixed += sync_report.created + sync_report.updated;

        let report = Self::run(db, kb_id, kb_path)?;

        // 从快照恢复可修复的缺失页面
        for issue in &report.issues {
            if issue.fixable && issue.category == "missing_file" {
                if let Some(path) = issue.detail.get("path").and_then(|p| p.as_str()) {
                    if issue.detail.get("has_snapshot").and_then(|v| v.as_bool()).unwrap_or(false)
                        && Self::recover_from_snapshot(db, kb_id, kb_path, path).is_ok()
                    {
                        fixed += 1;
                    } else if let Some(page_id) = issue.detail.get("page_id").and_then(|p| p.as_str()) {
                        if Self::delete_broken_record(db, kb_id, page_id).is_ok() {
                            fixed += 1;
                        }
                    }
                }
            }
            if issue.fixable && issue.category == "bad_path" {
                // 路径已通过 repair_all_paths 修复
            }
        }

        crate::graph::graph_service::GraphService::sync_from_knowledge_items(db, kb_id)?;

        Ok(fixed)
    }

    /// 删除失效的 wiki_pages 记录
    pub fn delete_broken_record(db: &Arc<DatabaseService>, kb_id: &str, page_id: &str) -> Result<(), String> {
        let conn = db.connect()?;
        conn.execute(
            "DELETE FROM wiki_pages WHERE id = ?1 AND kb_id = ?2",
            rusqlite::params![page_id, kb_id],
        ).map_err(|e| format!("删除页面记录失败: {}", e))?;
        Ok(())
    }

    /// 标记页面为 broken
    pub fn mark_broken(db: &Arc<DatabaseService>, kb_id: &str, page_path: &str) -> Result<(), String> {
        let conn = db.connect()?;
        conn.execute(
            "UPDATE wiki_pages SET status = 'broken' WHERE kb_id = ?1 AND path = ?2",
            rusqlite::params![kb_id, PathService::normalize(page_path)],
        ).map_err(|e| format!("标记页面为 broken 失败: {}", e))?;
        Ok(())
    }

    /// 验证 sources 表：检查文件是否仍然存在，清理失效条目
    /// 返回清理数量
    pub fn validate_sources(db: &Arc<DatabaseService>, kb_id: &str, kb_path: &str) -> Result<usize, String> {
        let conn = db.connect()?;
        let mut cleaned = 0usize;

        let mut stmt = conn
            .prepare("SELECT id, file_path, file_name, file_hash FROM sources WHERE kb_id = ?1")
            .map_err(|e| format!("查询 sources 失败: {}", e))?;

        let sources: Vec<(String, String, String, String)> = stmt
            .query_map(rusqlite::params![kb_id], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
            })
            .map_err(|e| format!("映射 sources 失败: {}", e))?
            .filter_map(|r| r.ok())
            .collect();

        let workspace_root = std::path::PathBuf::from(kb_path);

        for (source_id, file_path, file_name, _file_hash) in &sources {
            // 尝试多种方式定位文件
            let exists = if !file_path.is_empty() {
                std::path::Path::new(file_path).exists()
            } else {
                false
            };

            // 如果 file_path 不存在，尝试在 workspace 中搜索
            let exists_in_workspace = if !exists && !file_name.is_empty() {
                let raw_docs = workspace_root.join("raw").join("sources").join("documents").join(file_name);
                let raw_web = workspace_root.join("raw").join("sources").join("webclips").join(file_name);
                let raw_root = workspace_root.join("raw").join("sources").join(file_name);
                raw_docs.exists() || raw_web.exists() || raw_root.exists()
            } else {
                false
            };

            if !exists && !exists_in_workspace {
                log::warn!(
                    "[Reconcile] 源文件缺失: id={}, path={}, name={}",
                    source_id, file_path, file_name
                );

                // 清理关联数据: source_previews
                if let Err(e) = conn.execute(
                    "DELETE FROM source_previews WHERE source_id = ?1",
                    rusqlite::params![source_id],
                ) {
                    log::error!("[Reconcile] 清理 source_previews 失败 (source={}): {}", source_id, e);
                }

                // 标记关联的 knowledge_items 来源为缺失
                if let Err(e) = conn.execute(
                    "UPDATE knowledge_items SET source_id = '' WHERE source_id = ?1",
                    rusqlite::params![source_id],
                ) {
                    log::error!("[Reconcile] 清理 knowledge_items.source_id 失败 (source={}): {}", source_id, e);
                }

                // 清理关联的任务（input_ref 指向该 source）
                if let Err(e) = conn.execute(
                    "UPDATE tasks SET status = 'failed', error_message = '源文件已缺失', next_action = '' WHERE kb_id = ?1 AND input_ref = ?2 AND status NOT IN ('completed', 'failed')",
                    rusqlite::params![kb_id, source_id],
                ) {
                    log::error!("[Reconcile] 清理 tasks 失败 (source={}): {}", source_id, e);
                }

                // 删除 sources 记录
                if let Err(e) = conn.execute(
                    "DELETE FROM sources WHERE id = ?1",
                    rusqlite::params![source_id],
                ) {
                    log::error!("[Reconcile] 删除失效 source 失败 ({}): {}", source_id, e);
                } else {
                    cleaned += 1;
                }
            }
        }

        if cleaned > 0 {
            log::info!("[Reconcile] 已清理 {} 个失效 source 条目 (kb={})", cleaned, kb_id);
        }
        Ok(cleaned)
    }

    /// 清理 file_index 中指向不存在文件的条目
    pub fn cleanup_stale_file_index(db: &Arc<DatabaseService>, kb_id: &str, kb_path: &str) -> Result<usize, String> {
        let conn = db.connect()?;
        let mut cleaned = 0usize;

        let mut stmt = conn
            .prepare("SELECT id, relative_path FROM file_index WHERE kb_id = ?1")
            .map_err(|e| format!("查询 file_index 失败: {}", e))?;

        let entries: Vec<(String, String)> = stmt
            .query_map(rusqlite::params![kb_id], |row| {
                Ok((row.get(0)?, row.get(1)?))
            })
            .map_err(|e| format!("映射 file_index 失败: {}", e))?
            .filter_map(|r| r.ok())
            .collect();

        let workspace_root = std::path::PathBuf::from(kb_path);

        for (id, relative_path) in &entries {
            let full_path = workspace_root.join(relative_path);
            if !full_path.exists() {
                if let Err(e) = conn.execute(
                    "DELETE FROM file_index WHERE id = ?1",
                    rusqlite::params![id],
                ) {
                    log::error!("[Reconcile] 删除失效 file_index 失败 ({}): {}", id, e);
                } else {
                    cleaned += 1;
                }
            }
        }

        if cleaned > 0 {
            log::info!("[Reconcile] 已清理 {} 个失效 file_index 条目 (kb={})", cleaned, kb_id);
        }
        Ok(cleaned)
    }

    /// 验证 KB 工作区目录是否存在，若不存在则标记 KB 为缺失
    pub fn check_kb_workspace_exists(_db: &Arc<DatabaseService>, _kb_id: &str, kb_path: &str) -> Result<bool, String> {
        let workspace_root = std::path::PathBuf::from(kb_path);
        Ok(workspace_root.exists())
    }

    /// 清理工作区目录已不存在的孤立 KB（仅删 DB 记录，目录已不存在故无需清理磁盘）
    pub fn purge_orphaned_kb(db: &Arc<DatabaseService>, kb_id: &str) -> Result<(), String> {
        let conn = db.connect()?;

        conn.execute("BEGIN TRANSACTION", [])
            .map_err(|e| format!("开始事务失败: {}", e))?;

        let result = (|| -> Result<(), String> {
            conn.execute("DELETE FROM graph_edges WHERE kb_id = ?1", rusqlite::params![kb_id])
                .map_err(|e| format!("删除图谱边失败: {}", e))?;
            conn.execute("DELETE FROM graph_nodes WHERE kb_id = ?1", rusqlite::params![kb_id])
                .map_err(|e| format!("删除图谱节点失败: {}", e))?;
            conn.execute("DELETE FROM operations WHERE kb_id = ?1", rusqlite::params![kb_id])
                .map_err(|e| format!("删除操作记录失败: {}", e))?;
            conn.execute("DELETE FROM versions WHERE kb_id = ?1", rusqlite::params![kb_id])
                .map_err(|e| format!("删除版本快照失败: {}", e))?;
            conn.execute("DELETE FROM review_item_events WHERE review_item_id IN (SELECT id FROM review_items WHERE review_id IN (SELECT id FROM reviews WHERE kb_id = ?1))", rusqlite::params![kb_id])
                .map_err(|e| format!("删除审阅事件失败: {}", e))?;
            conn.execute("DELETE FROM review_items WHERE review_id IN (SELECT id FROM reviews WHERE kb_id = ?1)", rusqlite::params![kb_id])
                .map_err(|e| format!("删除审阅项失败: {}", e))?;
            conn.execute("DELETE FROM reviews WHERE kb_id = ?1", rusqlite::params![kb_id])
                .map_err(|e| format!("删除审阅失败: {}", e))?;
            conn.execute("DELETE FROM task_events WHERE task_id IN (SELECT id FROM tasks WHERE kb_id = ?1)", rusqlite::params![kb_id])
                .map_err(|e| format!("删除任务事件失败: {}", e))?;
            conn.execute("DELETE FROM tasks WHERE kb_id = ?1", rusqlite::params![kb_id])
                .map_err(|e| format!("删除任务失败: {}", e))?;
            conn.execute("DELETE FROM aliases WHERE item_id IN (SELECT id FROM knowledge_items WHERE kb_id = ?1)", rusqlite::params![kb_id])
                .map_err(|e| format!("删除别名失败: {}", e))?;
            conn.execute("DELETE FROM relationships WHERE kb_id = ?1", rusqlite::params![kb_id])
                .map_err(|e| format!("删除关系失败: {}", e))?;
            conn.execute("DELETE FROM knowledge_items WHERE kb_id = ?1", rusqlite::params![kb_id])
                .map_err(|e| format!("删除知识项失败: {}", e))?;
            conn.execute("DELETE FROM wiki_pages WHERE kb_id = ?1", rusqlite::params![kb_id])
                .map_err(|e| format!("删除页面记录失败: {}", e))?;
            conn.execute("DELETE FROM sources WHERE kb_id = ?1", rusqlite::params![kb_id])
                .map_err(|e| format!("删除来源失败: {}", e))?;
            conn.execute("DELETE FROM assets WHERE kb_id = ?1", rusqlite::params![kb_id])
                .map_err(|e| format!("删除资产记录失败: {}", e))?;
            conn.execute("DELETE FROM source_previews WHERE source_id IN (SELECT id FROM sources WHERE kb_id = ?1)", rusqlite::params![kb_id])
                .map_err(|e| format!("删除来源预览失败: {}", e))?;
            conn.execute("DELETE FROM file_index WHERE kb_id = ?1", rusqlite::params![kb_id])
                .map_err(|e| format!("删除文件索引失败: {}", e))?;
            conn.execute("DELETE FROM knowledge_bases WHERE id = ?1", rusqlite::params![kb_id])
                .map_err(|e| format!("删除知识库记录失败: {}", e))?;
            Ok(())
        })();

        match result {
            Ok(()) => {
                conn.execute("COMMIT", [])
                    .map_err(|e| format!("提交事务失败: {}", e))?;
                log::info!("[Reconcile] 已清除孤立 KB 记录: {}", kb_id);
                Ok(())
            }
            Err(e) => {
                let _ = conn.execute("ROLLBACK", []);
                Err(e)
            }
        }
    }

    fn scan_wiki_dir(
        base: &std::path::Path,
        dir: &std::path::Path,
        kb_id: &str,
        conn: &rusqlite::Connection,
        issues: &mut Vec<ReconcileIssue>,
        _ok_items: &mut Vec<String>,
    ) -> std::io::Result<()> {
        if !dir.exists() {
            return Ok(());
        }

        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                Self::scan_wiki_dir(base, &path, kb_id, conn, issues, _ok_items)?;
            } else if path.extension().is_some_and(|e| e == "md") {
                let relative = PathService::normalize_workspace_path(&format!(
                    "wiki/{}",
                    path.strip_prefix(base)
                        .unwrap_or(&path)
                        .to_string_lossy()
                        .replace('\\', "/")
                ));

                if relative == "wiki/index.md" || relative == "wiki/log.md" || relative == "wiki/overview.md" {
                    continue;
                }

                let count: i64 = match conn
                    .query_row(
                        "SELECT COUNT(*) FROM wiki_pages WHERE kb_id = ?1 AND path = ?2",
                        rusqlite::params![kb_id, relative],
                        |row| row.get(0),
                    ) {
                        Ok(c) => c,
                        Err(rusqlite::Error::QueryReturnedNoRows) => 0,
                        Err(e) => return Err(std::io::Error::other(format!("查询 wiki_pages 计数失败 (path={}): {}", relative, e))),
                    };

                if count == 0 {
                    issues.push(ReconcileIssue {
                        severity: "warning".to_string(),
                        category: "orphan_file".to_string(),
                        description: format!("Wiki 文件存在但缺少数据库记录: {}", relative),
                        suggestion: "运行数据库同步或手动创建页面记录".to_string(),
                        fixable: true,
                        detail: serde_json::json!({
                            "action": "sync_markdown_index",
                            "path": relative,
                        }),
                    });
                }
            }
        }

        Ok(())
    }
}
