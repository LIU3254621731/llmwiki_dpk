// WikiWriter - 统一 Wiki 写入入口（v0.1.3 增强版）
// 所有 Wiki 文件写入必须经过 WikiWriter
// 新增: knowledge_items / aliases / relationships / graph_nodes / operations 同步

use std::path::{Path, PathBuf};
use rusqlite::Connection;
use crate::core::database_service::DatabaseService;
use crate::wiki::path_service::PathService;

#[derive(Debug, Clone)]
pub struct WikiWriteResult {
    pub page_id: String,
    pub relative_path: String,
    pub content_hash: String,
    pub knowledge_item_id: Option<String>,
    pub operation_id: Option<String>,
}

pub struct WikiWriter {
    db: std::sync::Arc<DatabaseService>,
}

impl WikiWriter {
    pub fn new(db: std::sync::Arc<DatabaseService>) -> Self {
        Self { db }
    }

    /// 确保 kb_id 下存在 "manual" 系统任务（用于无流水线的手动操作）
    fn ensure_manual_task(&self, kb_id: &str) -> Result<(), String> {
        let conn = self.db.connect()?;
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT OR IGNORE INTO tasks (id, kb_id, task_type, status, created_at, updated_at)
             VALUES ('manual', ?1, 'manual', 'completed', ?2, ?2)",
            rusqlite::params![kb_id, now],
        ).map_err(|e| format!("创建 manual 任务失败: {}", e))?;
        Ok(())
    }

    /// 生成规范的 canonical_name（委托给 PathService）
    pub fn generate_canonical_name(title: &str) -> String {
        PathService::generate_safe_name(title)
    }

    /// 根据 page_type + canonical_name 解析页面绝对路径（委托给 PathService）
    pub fn resolve_page_path(wiki_dir: &Path, page_type: &str, canonical_name: &str) -> std::path::PathBuf {
        let file_name = format!("{}.md", PathService::generate_safe_name(canonical_name));
        let page_dir = Self::page_type_dir(wiki_dir, page_type);
        page_dir.join(&file_name)
    }

    pub fn page_type_dir(wiki_dir: &Path, page_type: &str) -> std::path::PathBuf {
        wiki_dir.join(PathService::page_type_to_dir(page_type))
    }

    /// 计算相对路径（相对于 workspace 根目录，含 wiki/ 前缀）
    pub fn compute_relative_path(wiki_dir: &Path, full_path: &Path) -> String {
        let wiki_parent = wiki_dir.parent().unwrap_or(wiki_dir);
        let rel = full_path.strip_prefix(wiki_parent)
            .unwrap_or(full_path)
            .to_string_lossy()
            .replace('\\', "/");
        PathService::normalize_workspace_path(&rel)
    }

    pub fn resolve_absolute_path(kb_path: &Path, relative_path: &str) -> std::path::PathBuf {
        PathService::resolve_workspace_path(kb_path, relative_path)
    }

    pub fn workspace_root_from_wiki_dir(wiki_dir: &Path) -> PathBuf {
        wiki_dir.parent().unwrap_or(wiki_dir).to_path_buf()
    }

    pub fn path_exists(kb_path: &Path, relative_path: &str) -> bool {
        Self::resolve_absolute_path(kb_path, relative_path).exists()
    }

    pub fn normalize_path(path: &str) -> String {
        PathService::normalize(path)
    }

    /// 创建新 Wiki 页面（基础版，仅写文件和 wiki_pages 记录）
    pub fn create_page(
        &self,
        kb_id: &str,
        wiki_dir: &Path,
        page_type: &str,
        title: &str,
        canonical_name: &str,
        content: &str,
        tags: &str,
    ) -> Result<String, String> {
        self.ensure_manual_task(kb_id)?;
        let result = self.create_page_full(kb_id, wiki_dir, page_type, title, canonical_name, content, tags, "[]", "manual", None)?;
        Ok(result.page_id)
    }

    /// 创建新 Wiki 页面（完整版，包含 knowledge_items / graph_nodes / operations）
    pub fn create_page_full(
        &self,
        kb_id: &str,
        wiki_dir: &Path,
        page_type: &str,
        title: &str,
        canonical_name: &str,
        content: &str,
        tags: &str,
        aliases: &str,
        task_id: &str,
        source_id: Option<&str>,
    ) -> Result<WikiWriteResult, String> {
        let conn = self.db.connect()?;

        let safe_canonical = PathService::generate_safe_name(canonical_name);
        let page_file_name = format!("{}.md", &safe_canonical);
        let page_dir = Self::page_type_dir(wiki_dir, page_type);

        PathService::ensure_parent_dir(&page_dir.join(&page_file_name))?;
        let page_path = page_dir.join(&page_file_name);
        let relative_path = PathService::normalize(
            &page_path.strip_prefix(wiki_dir.parent().unwrap_or(wiki_dir))
                .unwrap_or(&page_path)
                .to_string_lossy()
                .replace('\\', "/")
        );

        let now_fmt = chrono::Utc::now().format("%Y-%m-%d").to_string();
        let full_content = format!(
            "---\ntitle: {}\ntype: {}\ncanonical_name: {}\naliases: {}\nsources: {}\ntags: {}\nconfidence: medium\nstatus: active\ncreated: {}\nupdated: {}\nlast_updated_by_task: {}\n---\n\n{}",
            title, page_type, safe_canonical, aliases,
            source_id.map(|s| format!("[\"{}\"]", s)).unwrap_or_else(|| "[]".to_string()),
            tags, now_fmt, now_fmt, task_id, content
        );

        // Atomic write via temp file + rename (same as update_page_full)
        let tmp_path = page_path.with_extension("md.tmp");
        std::fs::write(&tmp_path, &full_content)
            .map_err(|e| format!("写入临时文件失败: {}", e))?;
        std::fs::rename(&tmp_path, &page_path)
            .map_err(|e| format!("原子替换文件失败: {}", e))?;

        let content_hash = PathService::content_hash(&full_content);
        let page_id = uuid::Uuid::new_v4().to_string();

        conn.execute(
            "INSERT INTO wiki_pages (id, kb_id, title, path, page_type, canonical_name, tags, content_hash, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?9)",
            rusqlite::params![page_id, kb_id, title, relative_path, page_type, safe_canonical, tags, content_hash, now_fmt],
        ).map_err(|e| format!("保存页面记录失败: {}", e))?;

        // 创建 knowledge_item（关联 source_id）
        let ki_id = uuid::Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO knowledge_items (id, kb_id, canonical_name, item_type, page_path, page_id, linked_page_path, source_id, summary, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?5, ?7, ?8, ?9, ?9)",
            rusqlite::params![ki_id, kb_id, safe_canonical, page_type, relative_path, page_id,
                source_id.unwrap_or(""),
                &content[..content.len().min(300)], now_fmt],
        ).map_err(|e| format!("创建 knowledge_item 失败: {}", e))?;

        // 同步 graph_node
        if let Err(e) = crate::graph::graph_service::GraphService::add_or_update_node(
            &self.db, kb_id, page_type, title, &relative_path,
        ) {
            log::error!("[WikiWriter] 同步 graph_node 失败 (page={}): {}", relative_path, e);
        }

        // 记录 operation
        let op_id = uuid::Uuid::new_v4().to_string();
        let op_hash = PathService::content_hash(&format!("create:{}:{}", relative_path, content_hash));
        let now_ts = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO operations (id, kb_id, task_id, operation_hash, target_path, status, applied_at)
             VALUES (?1, ?2, ?3, ?4, ?5, 'applied', ?6)",
            rusqlite::params![op_id, kb_id, task_id, op_hash, relative_path, now_ts],
        ).map_err(|e| format!("记录 operation 失败: {}", e))?;

        Ok(WikiWriteResult {
            page_id, relative_path, content_hash,
            knowledge_item_id: Some(ki_id),
            operation_id: Some(op_id),
        })
    }

    /// 更新已有 Wiki 页面（完整版，包含 knowledge_items 更新 + graph 同步）
    pub fn update_page_full(
        &self,
        kb_id: &str,
        wiki_dir: &Path,
        page_path_str: &str,
        new_content: &str,
        task_id: &str,
    ) -> Result<WikiWriteResult, String> {
        let normalized = PathService::normalize_workspace_path(page_path_str);
        let absolute_path = PathService::resolve_workspace_path(wiki_dir.parent().unwrap_or(wiki_dir), &normalized);

        // 确保父目录存在
        PathService::ensure_parent_dir(&absolute_path)?;

        let title: String;
        let page_type: String;
        let canonical: String;
        let relative_path: String;
        let full_content: String;
        let now_fmt = chrono::Utc::now().format("%Y-%m-%d").to_string();

        let conn = self.db.connect()?;

        if !absolute_path.exists() {
            // 文件不存在，创建新页面
            title = normalized
                .trim_end_matches(".md")
                .split('/').next_back().unwrap_or("Untitled")
                .to_string();
            page_type = PathService::path_to_page_type(&normalized).to_string();
            canonical = PathService::generate_safe_name(&title);
            relative_path = normalized.clone();
            full_content = format!(
                "---\ntitle: {}\ntype: {}\ncanonical_name: {}\naliases: []\nsources: []\ntags: \"\"\nconfidence: medium\nstatus: active\ncreated: {}\nupdated: {}\nlast_updated_by_task: {}\n---\n\n{}",
                title, page_type, canonical, now_fmt, now_fmt, task_id, new_content
            );

            std::fs::write(&absolute_path, &full_content)
                .map_err(|e| format!("创建新页面文件失败: {}", e))?;
        } else {
            // 文件存在，更新内容
            relative_path = normalized.clone();

            let current = std::fs::read_to_string(&absolute_path)
                .map_err(|e| format!("读取页面失败: {}", e))?;

            let (frontmatter, _) = Self::parse_frontmatter(&current);
            let updated_fm = Self::update_frontmatter_field(&frontmatter, "updated", &now_fmt);
            let updated_fm = Self::update_frontmatter_field(&updated_fm, "last_updated_by_task", task_id);

            // 从 frontmatter 提取元信息
            title = frontmatter.lines()
                .find(|l| l.starts_with("title:"))
                .and_then(|l| l.strip_prefix("title:"))
                .map(|t| t.trim().to_string())
                .unwrap_or_else(|| "Untitled".to_string());
            page_type = frontmatter.lines()
                .find(|l| l.starts_with("type:"))
                .and_then(|l| l.strip_prefix("type:"))
                .map(|t| t.trim().to_string())
                .unwrap_or_else(|| "concept".to_string());
            canonical = frontmatter.lines()
                .find(|l| l.starts_with("canonical_name:"))
                .and_then(|l| l.strip_prefix("canonical_name:"))
                .map(|c| c.trim().to_string())
                .unwrap_or_else(|| PathService::generate_safe_name(&title));

            full_content = format!("---\n{}---\n\n{}", updated_fm, new_content);

            let tmp_path = absolute_path.with_extension("md.tmp");
            std::fs::write(&tmp_path, &full_content)
                .map_err(|e| format!("写入临时文件失败: {}", e))?;
            std::fs::rename(&tmp_path, &absolute_path)
                .map_err(|e| format!("原子替换文件失败: {}", e))?;
        }

        // 单一入口：由 upsert_wiki_page_record 统一处理 DB 写入
        let content_hash = PathService::content_hash(&full_content);
        let page_id = Self::upsert_wiki_page_record(
            &conn,
            kb_id,
            &title,
            &relative_path,
            &page_type,
            &canonical,
            "",
            &content_hash,
            &now_fmt,
        )?;

        // 同步 knowledge_items
        let ki_id: String = match conn.query_row(
            "SELECT id FROM knowledge_items WHERE kb_id = ?1 AND canonical_name = ?2",
            rusqlite::params![kb_id, canonical],
            |row| row.get(0),
        ) {
            Ok(id) => id,
            Err(rusqlite::Error::QueryReturnedNoRows) => {
                let new_ki = uuid::Uuid::new_v4().to_string();
                conn.execute(
                    "INSERT INTO knowledge_items (id, kb_id, canonical_name, item_type, page_path, page_id, linked_page_path, summary, created_at, updated_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?5, ?7, ?8, ?8)",
                    rusqlite::params![new_ki, kb_id, canonical, page_type, relative_path, page_id,
                        &full_content[..full_content.len().min(300)], now_fmt],
                ).map_err(|e| format!("插入 knowledge_item 失败 (page={}): {}", relative_path, e))?;
                new_ki
            }
            Err(e) => {
                log::error!("[WikiWriter] 查询 knowledge_item 失败: {}", e);
                return Err(format!("查询 knowledge_item 失败: {}", e));
            }
        };

        // 更新已有 knowledge_items 的 page_id（如果尚未关联）
        if let Err(e) = conn.execute(
            "UPDATE knowledge_items SET page_id = ?1, linked_page_path = ?2, updated_at = ?3 WHERE kb_id = ?4 AND canonical_name = ?5 AND COALESCE(page_id, '') = ''",
            rusqlite::params![page_id, relative_path, now_fmt, kb_id, canonical],
        ) {
            log::error!("[WikiWriter] 更新 knowledge_item page_id 失败: {}", e);
        }

        // 同步 graph_node
        if let Err(e) = crate::graph::graph_service::GraphService::add_or_update_node(
            &self.db, kb_id, &page_type, &title, &relative_path,
        ) {
            log::error!("[WikiWriter] graph_node 同步失败 (page={}): {}", relative_path, e);
        }

        // 记录 operation
        let op_id = uuid::Uuid::new_v4().to_string();
        let op_hash = PathService::content_hash(&format!("update:{}:{}", relative_path, content_hash));
        let now_ts = chrono::Utc::now().to_rfc3339();
        if let Err(e) = conn.execute(
            "INSERT INTO operations (id, kb_id, task_id, operation_hash, target_path, status, applied_at)
             VALUES (?1, ?2, ?3, ?4, ?5, 'applied', ?6)",
            rusqlite::params![op_id, kb_id, task_id, op_hash, relative_path, now_ts],
        ) {
            log::error!("[WikiWriter] 记录 operation 失败: {}", e);
        }

        Ok(WikiWriteResult {
            page_id, relative_path, content_hash,
            knowledge_item_id: Some(ki_id),
            operation_id: Some(op_id),
        })
    }

    /// 更新已有 Wiki 页面（兼容旧接口）
    pub fn update_page(
        &self,
        kb_id: &str,
        wiki_dir: &Path,
        page_path_str: &str,
        new_content: &str,
        task_id: &str,
    ) -> Result<(), String> {
        self.ensure_manual_task(kb_id)?;
        self.update_page_full(kb_id, wiki_dir, page_path_str, new_content, task_id)?;
        Ok(())
    }

    /// 检查页面是否被用户手动修改
    pub fn check_page_unchanged(&self, wiki_dir: &Path, page_path_str: &str, expected_hash: &str) -> Result<bool, String> {
        let page_path = wiki_dir.join(PathService::normalize(page_path_str));
        if !page_path.exists() {
            return Ok(true);
        }
        let content = std::fs::read_to_string(&page_path)
            .map_err(|e| format!("读取页面失败: {}", e))?;
        let current_hash = PathService::content_hash(&content);
        Ok(current_hash == expected_hash)
    }

    /// 同步 knowledge_item 关联
    pub fn upsert_knowledge_item(
        db: &std::sync::Arc<DatabaseService>,
        kb_id: &str,
        canonical_name: &str,
        item_type: &str,
        page_path: &str,
        summary: &str,
    ) -> Result<String, String> {
        let conn = db.connect()?;
        let now = chrono::Utc::now().format("%Y-%m-%d").to_string();
        let existing = match conn.query_row(
            "SELECT id FROM knowledge_items WHERE kb_id = ?1 AND canonical_name = ?2",
            rusqlite::params![kb_id, canonical_name],
            |row| row.get::<_, String>(0),
        ) {
            Ok(id) => Some(id),
            Err(rusqlite::Error::QueryReturnedNoRows) => None,
            Err(e) => return Err(format!("查询知识项失败: {}", e)),
        };

        if let Some(id) = existing {
            conn.execute(
                "UPDATE knowledge_items SET item_type = ?1, page_path = ?2, summary = ?3, updated_at = ?4 WHERE id = ?5",
                rusqlite::params![item_type, page_path, summary, now, id],
            ).map_err(|e| format!("更新知识项失败: {}", e))?;
            Ok(id)
        } else {
            let id = uuid::Uuid::new_v4().to_string();
            conn.execute(
                "INSERT INTO knowledge_items (id, kb_id, canonical_name, item_type, page_path, summary, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)",
                rusqlite::params![id, kb_id, canonical_name, item_type, page_path, summary, now],
            ).map_err(|e| format!("创建知识项失败: {}", e))?;
            Ok(id)
        }
    }

    /// 同步 alias
    pub fn upsert_alias(
        db: &std::sync::Arc<DatabaseService>,
        item_id: &str,
        alias: &str,
        language: &str,
    ) -> Result<(), String> {
        let conn = db.connect()?;
        let normalized = alias.to_lowercase().trim().to_string();
        let existing: i64 = match conn.query_row(
            "SELECT COUNT(*) FROM aliases WHERE item_id = ?1 AND normalized_alias = ?2",
            rusqlite::params![item_id, normalized],
            |row| row.get(0),
        ) {
            Ok(c) => c,
            Err(rusqlite::Error::QueryReturnedNoRows) => 0,
            Err(e) => return Err(format!("查询别名是否存在失败: {}", e)),
        };
        if existing == 0 {
            let id = uuid::Uuid::new_v4().to_string();
            let now = chrono::Utc::now().to_rfc3339();
            conn.execute(
                "INSERT INTO aliases (id, item_id, alias, normalized_alias, language, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                rusqlite::params![id, item_id, alias, normalized, language, now],
            ).map_err(|e| format!("创建别名失败: {}", e))?;
        }
        Ok(())
    }

    /// 同步 relationship
    pub fn upsert_relationship(
        db: &std::sync::Arc<DatabaseService>,
        kb_id: &str,
        source_item_id: &str,
        target_item_id: &str,
        relation: &str,
        confidence: &str,
        evidence_source_id: &str,
    ) -> Result<(), String> {
        let conn = db.connect()?;
        let existing: i64 = match conn.query_row(
            "SELECT COUNT(*) FROM relationships WHERE kb_id = ?1 AND source_item_id = ?2 AND target_item_id = ?3 AND relation = ?4",
            rusqlite::params![kb_id, source_item_id, target_item_id, relation],
            |row| row.get(0),
        ) {
            Ok(c) => c,
            Err(rusqlite::Error::QueryReturnedNoRows) => 0,
            Err(e) => return Err(format!("查询关系是否存在失败: {}", e)),
        };
        if existing == 0 {
            let id = uuid::Uuid::new_v4().to_string();
            let now = chrono::Utc::now().to_rfc3339();
            conn.execute(
                "INSERT INTO relationships (id, kb_id, source_item_id, target_item_id, relation, evidence_source_id, confidence, status, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'active', ?8)",
                rusqlite::params![id, kb_id, source_item_id, target_item_id, relation, evidence_source_id, confidence, now],
            ).map_err(|e| format!("创建关系失败: {}", e))?;
        }
        Ok(())
    }

    /// 检查 operation_hash 是否已应用（幂等检查）
    pub fn is_operation_applied(db: &std::sync::Arc<DatabaseService>, kb_id: &str, op_hash: &str) -> Result<bool, String> {
        let conn = db.connect()?;
        let count: i64 = match conn.query_row(
            "SELECT COUNT(*) FROM operations WHERE kb_id = ?1 AND operation_hash = ?2 AND status = 'applied'",
            rusqlite::params![kb_id, op_hash],
            |row| row.get(0),
        ) {
            Ok(c) => c,
            Err(rusqlite::Error::QueryReturnedNoRows) => 0,
            Err(e) => return Err(format!("查询操作幂等状态失败: {}", e)),
        };
        Ok(count > 0)
    }

    // 辅助函数

    fn upsert_wiki_page_record(
        conn: &Connection,
        kb_id: &str,
        title: &str,
        relative_path: &str,
        page_type: &str,
        canonical_name: &str,
        tags: &str,
        content_hash: &str,
        now: &str,
    ) -> Result<String, String> {
        let existing_id: Option<String> = match conn.query_row(
            "SELECT id FROM wiki_pages WHERE kb_id = ?1 AND path = ?2",
            rusqlite::params![kb_id, relative_path],
            |row| row.get::<_, String>(0),
        ) {
            Ok(id) => Some(id),
            Err(rusqlite::Error::QueryReturnedNoRows) => None,
            Err(e) => return Err(format!("查询页面是否存在失败: {}", e)),
        };
        if let Some(existing_id) = existing_id {
            // 检查 canonical_name 是否被其他路径占用，冲突时自动去重
            let canonical_to_use = if let Ok((collision_id, collision_path)) = conn.query_row(
                "SELECT id, path FROM wiki_pages WHERE kb_id = ?1 AND canonical_name = ?2 AND path != ?3 LIMIT 1",
                rusqlite::params![kb_id, canonical_name, relative_path],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            ) {
                let unique = Self::ensure_unique_canonical(conn, kb_id, canonical_name);
                log::error!("[WikiWriter] canonical_name 冲突: '{}' 已被 {}({}) 使用, 当前页面: {}, 已调整为: {}",
                    canonical_name, collision_path, collision_id, relative_path, unique);
                unique
            } else {
                canonical_name.to_string()
            };
            conn.execute(
                "UPDATE wiki_pages
                 SET title = ?1, page_type = ?2, canonical_name = ?3, tags = ?4, content_hash = ?5, updated_at = ?6
                 WHERE id = ?7",
                rusqlite::params![title, page_type, canonical_to_use, tags, content_hash, now, existing_id],
            ).map_err(|e| format!("更新页面记录失败: {}", e))?;
            Ok(existing_id)
        } else {
            // 确保 canonical_name 在 kb 内唯一
            let unique_canonical = Self::ensure_unique_canonical(conn, kb_id, canonical_name);
            if unique_canonical != canonical_name {
                log::error!("[WikiWriter] canonical_name '{}' 冲突，已自动调整为 '{}'", canonical_name, unique_canonical);
            }
            let page_id = uuid::Uuid::new_v4().to_string();
            conn.execute(
                "INSERT INTO wiki_pages (id, kb_id, title, path, page_type, canonical_name, tags, content_hash, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?9)",
                rusqlite::params![page_id, kb_id, title, relative_path, page_type, unique_canonical, tags, content_hash, now],
            ).map_err(|e| format!("保存页面记录失败: {}", e))?;
            Ok(page_id)
        }
    }

    /// 确保 canonical_name 在知识库内唯一，冲突时追加数字后缀
    fn ensure_unique_canonical(conn: &Connection, kb_id: &str, base_name: &str) -> String {
        let mut candidate = base_name.to_string();
        let mut suffix = 2;
        loop {
            let exists: bool = match conn.query_row(
                "SELECT COUNT(1) > 0 FROM wiki_pages WHERE kb_id = ?1 AND canonical_name = ?2",
                rusqlite::params![kb_id, candidate],
                |row| row.get(0),
            ) {
                Ok(c) => c,
                Err(rusqlite::Error::QueryReturnedNoRows) => false,
                Err(e) => { log::error!("[WikiWriter] canonical_name 唯一性查询失败: {}", e); false }
            };
            if !exists {
                return candidate;
            }
            candidate = format!("{}-{}", base_name, suffix);
            suffix += 1;
            if suffix > 100 {
                log::error!("[WikiWriter] canonical_name 唯一化循环超过 100 次，使用 UUID 兜底");
                return format!("{}-{}", base_name, uuid::Uuid::new_v4().to_string().split('-').next().unwrap_or("x"));
            }
        }
    }

    pub fn slugify(name: &str) -> String {
        PathService::generate_safe_name(name)
    }

    fn parse_frontmatter(content: &str) -> (String, String) {
        if let Some(rest) = content.strip_prefix("---\n") {
            if let Some(end) = rest.find("\n---\n") {
                return (rest[..end].to_string(), rest[end + 5..].to_string());
            }
        }
        (String::new(), content.to_string())
    }

    fn update_frontmatter_field(fm: &str, field: &str, value: &str) -> String {
        let prefix = format!("{}:", field);
        let mut lines: Vec<String> = fm.lines().map(|l| l.to_string()).collect();
        let mut found = false;
        for line in &mut lines {
            if line.trim_start().starts_with(&prefix) {
                *line = format!("{}: {}", field, value);
                found = true;
                break;
            }
        }
        if !found {
            lines.push(format!("{}: {}", field, value));
        }
        lines.join("\n")
    }
}
