// PathService - 统一路径规范服务
// 确保所有 wiki 页面路径使用正斜杠、规范目录、安全文件名
// v0.1.4: 新增 strip_duplicate_wiki_prefix / normalize_workspace_path 等安全方法

use std::path::{Path, PathBuf};
use sha2::{Digest, Sha256};

pub struct PathService;

impl PathService {
    /// 内部统一使用 `/` 分隔的相对路径；读写时通过此函数转换为平台路径
    pub fn to_platform(normalized: &str) -> PathBuf {
        PathBuf::from(normalized.replace('/', std::path::MAIN_SEPARATOR_STR))
    }

    /// 将平台路径转换为规范相对路径（仅使用 `/`，去除首尾 `/`）
    pub fn normalize(input: &str) -> String {
        let mut s = input.replace('\\', "/").trim_start_matches('/').trim_end_matches('/').to_string();
        s = Self::strip_duplicate_wiki_prefix(&s);
        s
    }

    /// 去除 wiki/wiki 重复前缀，例如:
    ///   wiki/wiki/concepts/xxx.md → wiki/concepts/xxx.md
    ///   wiki/wiki/wiki/entities/a.md → wiki/entities/a.md (递归去除)
    pub fn strip_duplicate_wiki_prefix(path: &str) -> String {
        let mut result = path.to_string();
        while result.starts_with("wiki/wiki/") {
            result = result.replacen("wiki/wiki/", "wiki/", 1);
        }
        result
    }

    /// 规范化 workspace 相对路径（始终相对于 workspace 根目录）
    /// 保证以 wiki/ 前缀且无重复
    pub fn normalize_workspace_path(path: &str) -> String {
        let n = Self::normalize(path);
        let n = Self::strip_duplicate_wiki_prefix(&n);
        if n.starts_with("wiki/") {
            n
        } else {
            Self::repair_path(&n)
        }
    }

    /// 将规范路径解析为绝对文件系统路径
    /// full_path = workspace_root.join(normalize_workspace_path(path))
    pub fn resolve_workspace_path(workspace_root: &Path, relative_path: &str) -> PathBuf {
        let normalized = Self::normalize_workspace_path(relative_path);
        workspace_root.join(&normalized)
    }

    /// 检查是否为合法的 workspace 相对路径
    pub fn is_valid_workspace_relative_path(path: &str) -> bool {
        let n = Self::normalize(path);
        !n.contains("..") && !n.starts_with("wiki/wiki/") && n.starts_with("wiki/") && n.ends_with(".md")
    }

    /// 根据 title + page_type 生成规范 wiki 页面路径（workspace 相对路径），例如:
    ///   wiki/concepts/remote-photoplethysmography-rppg.md
    ///   wiki/entities/openface.md
    pub fn resolve_wiki_page_path(page_type: &str, title: &str) -> String {
        let canonical = Self::generate_safe_name(title);
        let dir = Self::page_type_to_dir(page_type);
        format!("wiki/{}/{}.md", dir, canonical)
    }

    /// 生成 wiki 目录内的相对路径（不含 wiki/ 前缀），例如:
    ///   concepts/remote-photoplethysmography-rppg.md
    pub fn wiki_relative_path(page_type: &str, title: &str) -> String {
        let canonical = Self::generate_safe_name(title);
        let dir = Self::page_type_to_dir(page_type);
        format!("{}/{}.md", dir, canonical)
    }

    /// 生成安全的文件名（slug）
    pub fn generate_safe_name(name: &str) -> String {
        let trimmed = name.trim();
        let has_non_ascii = !trimmed.is_ascii();
        let slug_source = if has_non_ascii {
            deunicode::deunicode(trimmed)
        } else {
            trimmed.to_string()
        };

        let slug = slug_source.to_lowercase()
            .replace([' ', '/', '\\', ':', '*', '?', '"', '<', '>', '|', '(', ')', '[', ']', '\'', '!', ';', ',', '.'], "-");

        // 过滤仅保留 ASCII 字母数字、连字符、下划线
        let filtered: String = slug
            .chars()
            .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
            .collect();

        // 合并连续连字符
        let mut result = String::new();
        for c in filtered.chars() {
            if c == '-' && result.ends_with('-') {
                continue;
            }
            result.push(c);
        }
        let result = result.trim_matches('-').to_string();

        // 中文或过短标题使用短 hash
        if has_non_ascii || result.len() < 3 {
            let mut hasher = Sha256::new();
            hasher.update(trimmed.as_bytes());
            let hash_suffix = hex::encode(&hasher.finalize()[..6]);
            if result.is_empty() {
                format!("page-{}", hash_suffix)
            } else {
                format!("{}-{}", result, hash_suffix)
            }
        } else {
            result
        }
    }

    /// 重名时追加短 hash
    pub fn generate_unique_name(existing_names: &[String], base_name: &str) -> String {
        let safe = Self::generate_safe_name(base_name);
        if !existing_names.iter().any(|n| n == &safe) {
            return safe;
        }
        let mut hasher = Sha256::new();
        hasher.update(base_name.as_bytes());
        hasher.update(chrono::Utc::now().to_rfc3339().as_bytes());
        let short = hex::encode(&hasher.finalize()[..4]);
        format!("{}-{}", safe.trim_end_matches(".md"), short)
    }

    /// page_type 到目录映射
    pub fn page_type_to_dir(page_type: &str) -> &str {
        match page_type {
            "entity"   => "entities",
            "topic"    => "topics",
            "question" => "questions",
            "review"   => "reviews",
            "source"   => "sources",
            "dataset"  => "datasets",
            "method"   => "methods",
            _          => "concepts",
        }
    }

    /// 从路径提取 page_type
    pub fn path_to_page_type(relative_path: &str) -> &str {
        let normalized = Self::normalize(relative_path);
        if normalized.starts_with("wiki/entities/")  { return "entity"; }
        if normalized.starts_with("wiki/topics/")    { return "topic"; }
        if normalized.starts_with("wiki/questions/") { return "question"; }
        if normalized.starts_with("wiki/reviews/")   { return "review"; }
        if normalized.starts_with("wiki/sources/")   { return "source"; }
        if normalized.starts_with("wiki/datasets/")  { return "dataset"; }
        if normalized.starts_with("wiki/methods/")   { return "method"; }
        "concept"
    }

    /// 检查并确保父目录存在
    pub fn ensure_parent_dir(file_path: &Path) -> Result<(), String> {
        if let Some(parent) = file_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("创建目录失败 {}: {}", parent.display(), e))?;
        }
        Ok(())
    }

    /// 计算内容 hash
    pub fn content_hash(content: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        hex::encode(hasher.finalize())
    }

    /// 检查路径是否在 workspace 内且合法
    pub fn is_valid_wiki_path(path: &str) -> bool {
        let n = Self::normalize(path);
        n.starts_with("wiki/") && n.ends_with(".md") && !n.contains("..")
    }

    /// 修复历史杂乱路径：去除 UI 展示前缀、去除 wiki/wiki 重复、统一正斜杠
    /// 可修复的典型错误:
    ///   wiki/wiki/concepts/xxx.md → wiki/concepts/xxx.md
    ///   wiki\wiki\concepts\xxx.md  → wiki/concepts/xxx.md
    ///   concept · wiki\xxx.md      → wiki/concepts/xxx.md
    pub fn repair_path(raw: &str) -> String {
        let cleaned = raw
            .replace('\\', "/")
            .trim_start_matches('/')
            .to_string();

        // 去除 wiki/wiki 重复（核心修复）
        let cleaned = Self::strip_duplicate_wiki_prefix(&cleaned);

        // 去除可能混入的 UI 展示前缀，如 "concept · wiki\xxx.md" -> "wiki/xxx.md"
        let cleaned = if let Some(pos) = cleaned.find(" · ") {
            let after_dot = cleaned[pos + 3..].to_string();
            if after_dot.starts_with("wiki/") {
                after_dot
            } else {
                format!("wiki/{}", after_dot.trim_start_matches("wiki/"))
            }
        } else {
            cleaned
        };

        // 去除任意位置的 wiki/wiki/ 重复
        let cleaned = Self::strip_duplicate_wiki_prefix(&cleaned);

        // 如果路径不以 wiki/ 开头但看起来是 wiki 子路径，补齐前缀
        if !cleaned.starts_with("wiki/") {
            for sub in &["concepts/", "entities/", "topics/", "questions/", "reviews/", "sources/", "datasets/", "methods/"] {
                if cleaned.starts_with(sub) {
                    return format!("wiki/{}", cleaned);
                }
            }
            // 如果路径只有文件名，放到 concepts/
            if !cleaned.contains('/') && cleaned.ends_with(".md") {
                return format!("wiki/concepts/{}", cleaned);
            }
            // 如果看起来是 wiki_file 的路径，补齐 wiki/
            if cleaned.ends_with(".md") {
                return format!("wiki/{}", cleaned);
            }
        }

        Self::normalize(&cleaned)
    }

    /// 批量修复 wiki_pages 表中的路径（同时修复 review_items 和 versions 表）
    pub fn repair_all_paths(db: &std::sync::Arc<crate::core::database_service::DatabaseService>, kb_id: &str) -> Result<usize, String> {
        let conn = db.connect()?;
        let mut stmt = conn.prepare(
            "SELECT id, path FROM wiki_pages WHERE kb_id = ?1"
        ).map_err(|e| format!("查询页面路径失败: {}", e))?;

        let rows: Vec<(String, String)> = stmt.query_map(
            rusqlite::params![kb_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        ).map_err(|e| format!("映射路径失败: {}", e))?
        .filter_map(|r| r.ok())
        .collect();

        let mut fixed = 0usize;

        // 修复 wiki_pages 路径
        for (id, path) in &rows {
            let repaired = Self::repair_path(path);
            if &repaired != path {
                conn.execute(
                    "UPDATE wiki_pages SET path = ?1 WHERE id = ?2",
                    rusqlite::params![repaired, id],
                ).map_err(|e| format!("更新路径失败: {}", e))?;
                log::error!("[PathService] 修复路径: {} → {}", path, repaired);
                fixed += 1;
            }
        }

        // 同步修复 review_items 中的 target_path
        let mut ri_stmt = conn.prepare(
            "SELECT ri.id AS review_item_id, ri.target_path FROM review_items ri JOIN reviews r ON ri.review_id = r.id WHERE r.kb_id = ?1"
        ).map_err(|e| format!("查询审阅项路径失败: {}", e))?;
        let ri_rows: Vec<(String, String)> = ri_stmt.query_map(
            rusqlite::params![kb_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        ).map_err(|e| format!("映射审阅项路径失败: {}", e))?
        .filter_map(|r| r.ok())
        .collect();

        for (id, path) in &ri_rows {
            let repaired = Self::repair_path(path);
            if &repaired != path {
                if let Err(e) = conn.execute(
                    "UPDATE review_items SET target_path = ?1 WHERE id = ?2",
                    rusqlite::params![repaired, id],
                ) {
                    log::error!("[PathService] 修复 review_item({}) 路径失败: {}", id, e);
                }
                fixed += 1;
            }
        }

        // 同步修复 versions 中的 page_path
        let mut v_stmt = conn.prepare(
            "SELECT id, page_path FROM versions WHERE kb_id = ?1"
        ).map_err(|e| format!("查询版本路径失败: {}", e))?;
        let v_rows: Vec<(String, String)> = v_stmt.query_map(
            rusqlite::params![kb_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        ).map_err(|e| format!("映射版本路径失败: {}", e))?
        .filter_map(|r| r.ok())
        .collect();

        for (id, path) in &v_rows {
            let repaired = Self::repair_path(path);
            if &repaired != path {
                if let Err(e) = conn.execute(
                    "UPDATE versions SET page_path = ?1 WHERE id = ?2",
                    rusqlite::params![repaired, id],
                ) {
                    log::error!("[PathService] 修复 version({}) 路径失败: {}", id, e);
                }
                fixed += 1;
            }
        }

        Ok(fixed)
    }
}
