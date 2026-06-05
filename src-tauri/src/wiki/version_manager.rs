// VersionManager - 页面版本快照管理

use std::path::Path;
use sha2::{Digest, Sha256};
use crate::core::database_service::DatabaseService;
use crate::wiki::path_service::PathService;

pub struct VersionManager {
    db: std::sync::Arc<DatabaseService>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PageVersion {
    pub id: String,
    pub kb_id: String,
    pub page_path: String,
    pub content_hash: String,
    pub snapshot_path: String,
    pub task_id: String,
    pub operation_id: String,
    pub created_at: String,
}

impl VersionManager {
    pub fn new(db: std::sync::Arc<DatabaseService>) -> Self {
        Self { db }
    }

    /// 创建页面版本快照
    pub fn create_snapshot(
        &self,
        kb_id: &str,
        wiki_dir: &Path,
        page_path_str: &str,
        task_id: &str,
        operation_id: &str,
    ) -> Result<String, String> {
        let workspace_root = wiki_dir.parent().unwrap_or(wiki_dir);
        let normalized_page_path = PathService::normalize_workspace_path(page_path_str);
        let page_path = PathService::resolve_workspace_path(workspace_root, &normalized_page_path);
        if !page_path.exists() {
            return Err(format!("页面不存在: {}", page_path_str));
        }

        let content = std::fs::read_to_string(&page_path)
            .map_err(|e| format!("读取页面失败: {}", e))?;

        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        let hash = hex::encode(hasher.finalize());

        // 保存快照
        let versions_dir = wiki_dir.parent().unwrap_or(wiki_dir).join("versions").join("snapshots");
        std::fs::create_dir_all(&versions_dir).map_err(|e| format!("创建版本目录失败: {}", e))?;

        let snapshot_name = format!("{}_{}.snap", normalized_page_path.replace(['/', '\\'], "_"), &hash[..8]);
        let snapshot_path = versions_dir.join(&snapshot_name);

        std::fs::write(&snapshot_path, &content)
            .map_err(|e| format!("写入快照失败: {}", e))?;

        // 保存到数据库
        let id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();
        let relative_snapshot = snapshot_path.file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        let conn = self.db.connect()?;
        conn.execute(
            "INSERT INTO versions (id, kb_id, page_path, content_hash, snapshot_path, task_id, operation_id, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            rusqlite::params![id, kb_id, normalized_page_path, hash, relative_snapshot, task_id, operation_id, now],
        )
        .map_err(|e| format!("保存版本记录失败: {}", e))?;

        Ok(hash)
    }

    /// 获取页面版本列表
    pub fn get_versions(&self, kb_id: &str, page_path_str: &str) -> Result<Vec<PageVersion>, String> {
        let conn = self.db.connect()?;
        let mut stmt = conn
            .prepare("SELECT id, kb_id, page_path, content_hash, snapshot_path, COALESCE(task_id,''), COALESCE(operation_id,''), created_at FROM versions WHERE kb_id = ?1 AND page_path = ?2 ORDER BY created_at DESC")
            .map_err(|e| format!("查询版本失败: {}", e))?;

        let versions = stmt
            .query_map(rusqlite::params![kb_id, page_path_str], |row| {
                Ok(PageVersion {
                    id: row.get(0)?,
                    kb_id: row.get(1)?,
                    page_path: row.get(2)?,
                    content_hash: row.get(3)?,
                    snapshot_path: row.get(4)?,
                    task_id: row.get(5)?,
                    operation_id: row.get(6)?,
                    created_at: row.get(7)?,
                })
            })
            .map_err(|e| format!("映射版本失败: {}", e))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("收集版本失败: {}", e))?;

        Ok(versions)
    }

    /// 回滚到指定版本
    pub fn rollback(
        &self,
        kb_id: &str,
        wiki_dir: &Path,
        version_id: &str,
    ) -> Result<(), String> {
        let conn = self.db.connect()?;
        let (page_path, snapshot_path): (String, String) = conn
            .query_row(
                "SELECT page_path, snapshot_path FROM versions WHERE id = ?1 AND kb_id = ?2",
                rusqlite::params![version_id, kb_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .map_err(|e| format!("查询版本失败: {}", e))?;

        let versions_dir = wiki_dir.parent().unwrap_or(wiki_dir).join("versions").join("snapshots");
        let snapshot_file = versions_dir.join(&snapshot_path);

        if !snapshot_file.exists() {
            return Err(format!("版本快照文件不存在: {}", snapshot_path));
        }

        let content = std::fs::read_to_string(&snapshot_file)
            .map_err(|e| format!("读取快照失败: {}", e))?;

        let workspace_root = wiki_dir.parent().unwrap_or(wiki_dir);
        let page_file = PathService::resolve_workspace_path(workspace_root, &page_path);

        // 回滚前保存当前页面状态为 pre-rollback 快照，防止误操作导致数据永久丢失
        if page_file.exists() {
            if let Ok(current) = std::fs::read_to_string(&page_file) {
                let mut hasher = Sha256::new();
                hasher.update(current.as_bytes());
                let pre_hash = hex::encode(&hasher.finalize()[..6]);
                let pre_snapshot_name = format!("{}_pre_rollback_{}.snap",
                    page_path.replace(['/', '\\'], "_"), pre_hash);
                let pre_snapshot_path = versions_dir.join(&pre_snapshot_name);
                if let Err(e) = std::fs::write(&pre_snapshot_path, &current) {
                    log::error!("[VersionManager] 保存回滚前快照失败 (page={}): {}", page_path, e);
                } else {
                    let pre_id = uuid::Uuid::new_v4().to_string();
                    let now = chrono::Utc::now().to_rfc3339();
                    if let Err(e) = conn.execute(
                        "INSERT INTO versions (id, kb_id, page_path, content_hash, snapshot_path, task_id, operation_id, created_at)
                         VALUES (?1, ?2, ?3, ?4, ?5, '', 'pre_rollback', ?6)",
                        rusqlite::params![pre_id, kb_id, page_path, pre_hash, pre_snapshot_name, now],
                    ) {
                        log::error!("[VersionManager] 记录回滚前快照到 DB 失败 (page={}): {}", page_path, e);
                    }
                }
            }
        }

        let tmp = page_file.with_extension("md.rollback");
        std::fs::write(&tmp, &content).map_err(|e| format!("写入回滚文件失败: {}", e))?;
        std::fs::rename(&tmp, &page_file).map_err(|e| format!("回滚失败: {}", e))?;

        Ok(())
    }

    /// 检查内容是否变化（用于决定是否创建快照）
    pub fn has_content_changed(&self, kb_id: &str, page_path_str: &str, content: &str) -> Result<bool, String> {
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        let hash = hex::encode(hasher.finalize());

        let conn = self.db.connect()?;
        let last_hash: Option<String> = match conn.query_row(
            "SELECT content_hash FROM versions WHERE kb_id = ?1 AND page_path = ?2 ORDER BY created_at DESC LIMIT 1",
            rusqlite::params![kb_id, page_path_str],
            |row| row.get(0),
        ) {
            Ok(h) => Some(h),
            Err(rusqlite::Error::QueryReturnedNoRows) => None,
            Err(e) => {
                log::error!("[VersionManager] has_content_changed 查询失败 (page={}): {}", page_path_str, e);
                return Err(format!("查询版本哈希失败: {}", e));
            }
        };

        Ok(last_hash.is_none_or(|h| h != hash))
    }

    /// 为多个页面创建快照（批量操作前使用）
    pub fn create_snapshots_for_pages(
        &self,
        kb_id: &str,
        wiki_dir: &Path,
        page_paths: &[String],
        task_id: &str,
        operation_id: &str,
    ) -> Result<Vec<String>, String> {
        let mut results = Vec::new();
        for page_path in page_paths {
            match self.create_snapshot(kb_id, wiki_dir, page_path, task_id, operation_id) {
                Ok(hash) => results.push(format!("{}:{}", page_path, hash)),
                Err(e) => results.push(format!("{}:ERROR({})", page_path, e)),
            }
        }
        Ok(results)
    }

    /// 获取页面的最新版本
    pub fn get_latest_version(&self, kb_id: &str, page_path_str: &str) -> Result<Option<PageVersion>, String> {
        let conn = self.db.connect()?;
        let version = match conn.query_row(
            "SELECT id, kb_id, page_path, content_hash, snapshot_path, COALESCE(task_id,''), COALESCE(operation_id,''), created_at FROM versions WHERE kb_id = ?1 AND page_path = ?2 ORDER BY created_at DESC LIMIT 1",
            rusqlite::params![kb_id, page_path_str],
            |row| {
                Ok(PageVersion {
                    id: row.get(0)?,
                    kb_id: row.get(1)?,
                    page_path: row.get(2)?,
                    content_hash: row.get(3)?,
                    snapshot_path: row.get(4)?,
                    task_id: row.get(5)?,
                    operation_id: row.get(6)?,
                    created_at: row.get(7)?,
                })
            },
        ) {
            Ok(v) => Some(v),
            Err(rusqlite::Error::QueryReturnedNoRows) => None,
            Err(e) => {
                log::error!("[VersionManager] get_latest_version 查询失败 (page={}): {}", page_path_str, e);
                return Err(format!("查询最新版本失败: {}", e));
            }
        };
        Ok(version)
    }
}
