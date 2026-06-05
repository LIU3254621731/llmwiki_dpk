use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::core::database_service::DatabaseService;
use crate::wiki::index_service::IndexService;
use crate::wiki::path_service::PathService;
use crate::wiki::wiki_writer::WikiWriter;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MarkdownSyncReport {
    pub total_scanned: usize,
    pub synced: usize,
    pub created: usize,
    pub updated: usize,
    pub skipped: usize,
    pub skipped_system: usize,
    pub skipped_invalid: usize,
    pub skipped_errors: usize,
    pub warnings: Vec<String>,
    pub skip_reasons: Vec<serde_json::Value>,
}

#[derive(Debug, Clone)]
pub struct MarkdownMetadata {
    pub title: String,
    pub page_type: String,
    pub canonical_name: String,
    pub aliases: Vec<String>,
    pub tags: String,
    pub body: String,
}

pub struct MarkdownIndexer;

impl MarkdownIndexer {
    pub fn sync_workspace(
        db: &Arc<DatabaseService>,
        kb_id: &str,
        kb_path: &str,
    ) -> Result<MarkdownSyncReport, String> {
        let workspace_root = PathBuf::from(kb_path);
        let wiki_dir = workspace_root.join("wiki");
        let mut report = MarkdownSyncReport {
            total_scanned: 0,
            synced: 0,
            created: 0,
            updated: 0,
            skipped: 0,
            skipped_system: 0,
            skipped_invalid: 0,
            skipped_errors: 0,
            warnings: Vec::new(),
            skip_reasons: Vec::new(),
        };

        if !wiki_dir.exists() {
            report.warnings.push(format!("wiki 目录不存在: {}", wiki_dir.display()));
            return Ok(report);
        }

        let mut files = Vec::new();
        Self::collect_markdown_files(&wiki_dir, &mut files)
            .map_err(|e| format!("扫描 wiki markdown 文件失败: {}", e))?;
        files.sort();
        report.total_scanned = files.len();

        let conn = db.connect()?;
        let now = chrono::Utc::now().to_rfc3339();

        for file_path in &files {
            let file_name = file_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_ascii_lowercase();
            if matches!(file_name.as_str(), "index.md" | "log.md" | "overview.md") {
                report.skipped += 1;
                report.skipped_system += 1;
                report.skip_reasons.push(serde_json::json!({
                    "file": file_name,
                    "reason": "system_file_skipped",
                    "detail": "系统保留文件，不纳入 Wiki 页面索引"
                }));
                continue;
            }

            let wiki_relative = match file_path.strip_prefix(&wiki_dir) {
                Ok(p) => p.to_string_lossy().replace('\\', "/"),
                Err(_) => {
                    report.skipped += 1;
                    report.skipped_errors += 1;
                    let warn = format!("跳过 wiki 目录外的文件: {}", file_path.display());
                    report.warnings.push(warn.clone());
                    report.skip_reasons.push(serde_json::json!({
                        "file": file_path.file_name().and_then(|n| n.to_str()).unwrap_or("unknown"),
                        "reason": "outside_wiki_dir",
                        "detail": warn
                    }));
                    continue;
                }
            };
            let relative_path = PathService::normalize_workspace_path(&format!("wiki/{}", wiki_relative));

            let content = match std::fs::read_to_string(file_path) {
                Ok(c) => c,
                Err(e) => {
                    report.skipped += 1;
                    report.skipped_errors += 1;
                    let warn = format!("读取失败 {}: {}", relative_path, e);
                    report.warnings.push(warn.clone());
                    report.skip_reasons.push(serde_json::json!({
                        "file": file_name,
                        "path": relative_path,
                        "reason": "read_error",
                        "detail": warn
                    }));
                    continue;
                }
            };

            let metadata = Self::extract_metadata(&content, &relative_path);
            
            // 检查是否缺少有效标题（既没有 frontmatter title 也没有 H1）
            if metadata.title.is_empty() || metadata.title == "Untitled" {
                // 检查文件名是否看起来像自动生成的ID
                let stem = relative_path.trim_end_matches(".md").split('/').next_back().unwrap_or("");
                if Self::looks_like_generated_page_id(stem) || content.trim().is_empty() {
                    report.skipped += 1;
                    report.skipped_invalid += 1;
                    report.skip_reasons.push(serde_json::json!({
                        "file": file_name,
                        "path": relative_path,
                        "reason": "invalid_markdown",
                        "detail": "缺少有效的 frontmatter.title 和 H1 标题"
                    }));
                    continue;
                }
            }
            let content_hash = PathService::content_hash(&content);
            let existing: Option<String> = match conn.query_row(
                    "SELECT id FROM wiki_pages WHERE kb_id = ?1 AND path = ?2",
                    rusqlite::params![kb_id, relative_path],
                    |row| row.get(0),
                ) {
                    Ok(id) => Some(id),
                    Err(rusqlite::Error::QueryReturnedNoRows) => None,
                    Err(e) => {
                        log::error!("[markdown_indexer] 查询已存在页面失败 (path={}): {}", relative_path, e);
                        None
                    }
                };

            let page_id = if let Some(id) = existing {
                conn.execute(
                    "UPDATE wiki_pages
                     SET title = ?1, page_type = ?2, canonical_name = ?3, tags = ?4, content_hash = ?5, updated_at = ?6
                     WHERE id = ?7",
                    rusqlite::params![
                        metadata.title,
                        metadata.page_type,
                        metadata.canonical_name,
                        metadata.tags,
                        content_hash,
                        now,
                        id
                    ],
                )
                .map_err(|e| format!("update wiki_pages failed for {}: {}", relative_path, e))?;
                report.updated += 1;
                id
            } else {
                let id = uuid::Uuid::new_v4().to_string();
                conn.execute(
                    "INSERT INTO wiki_pages (id, kb_id, title, path, page_type, canonical_name, tags, content_hash, created_at, updated_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?9)",
                    rusqlite::params![
                        id,
                        kb_id,
                        metadata.title,
                        relative_path,
                        metadata.page_type,
                        metadata.canonical_name,
                        metadata.tags,
                        content_hash,
                        now
                    ],
                )
                .map_err(|e| format!("insert wiki_pages failed for {}: {}", relative_path, e))?;
                report.created += 1;
                id
            };

            let summary = Self::plain_summary(&metadata.body);
            let item_id = WikiWriter::upsert_knowledge_item(
                db,
                kb_id,
                &metadata.canonical_name,
                &metadata.page_type,
                &relative_path,
                &summary,
            )?;

            if !metadata.title.trim().is_empty() && metadata.title != metadata.canonical_name {
                if let Err(e) = WikiWriter::upsert_alias(db, &item_id, &metadata.title, "unknown") {
                    log::error!("[markdown_indexer] upsert_alias 标题别名失败 (item={}): {}", item_id, e);
                }
            }
            for alias in &metadata.aliases {
                if !alias.trim().is_empty() {
                    if let Err(e) = WikiWriter::upsert_alias(db, &item_id, alias, "unknown") {
                        log::error!("[markdown_indexer] upsert_alias 失败 (item={}, alias={}): {}", item_id, alias, e);
                    }
                }
            }

            if let Err(e) = conn.execute(
                "UPDATE graph_nodes SET page_id = ?1, path = ?2 WHERE kb_id = ?3 AND label = ?4",
                rusqlite::params![page_id, relative_path, kb_id, metadata.title],
            ) {
                log::error!("[markdown_indexer] graph_nodes UPDATE 失败 (page={}): {}", page_id, e);
            }

            report.synced += 1;
        }

        IndexService::new(db.clone()).rebuild_index(kb_id, &wiki_dir)?;
        crate::graph::graph_service::GraphService::sync_from_knowledge_items(db, kb_id)?;
        Ok(report)
    }

    pub fn extract_metadata(content: &str, relative_path: &str) -> MarkdownMetadata {
        let (frontmatter, body) = Self::split_frontmatter(content);
        let title_from_h1 = Self::extract_h1(&body);
        let path_title = Self::title_from_path(relative_path);

        let fm_title = frontmatter
            .as_ref()
            .and_then(|fm| Self::yaml_string(fm, "title"))
            .filter(|s| !s.trim().is_empty());
        let fm_type = frontmatter
            .as_ref()
            .and_then(|fm| Self::yaml_string(fm, "type"))
            .or_else(|| frontmatter.as_ref().and_then(|fm| Self::yaml_string(fm, "page_type")));
        let fm_canonical = frontmatter
            .as_ref()
            .and_then(|fm| Self::yaml_string(fm, "canonical_name"));

        let canonical_source = fm_canonical
            .clone()
            .filter(|s| !s.trim().is_empty())
            .or_else(|| fm_title.clone())
            .or_else(|| title_from_h1.clone())
            .unwrap_or_else(|| path_title.clone());

        let title = fm_title
            .or(title_from_h1)
            .or_else(|| {
                let trimmed = canonical_source.trim();
                if trimmed.is_empty() || Self::looks_like_generated_page_id(trimmed) {
                    None
                } else {
                    Some(trimmed.to_string())
                }
            })
            .unwrap_or(path_title);

        let page_type = fm_type
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| PathService::path_to_page_type(relative_path).to_string());
        let canonical_name = PathService::generate_safe_name(&canonical_source);
        let aliases = frontmatter
            .as_ref()
            .map(|fm| Self::yaml_string_list(fm, "aliases"))
            .unwrap_or_default();
        let tags = frontmatter
            .as_ref()
            .map(|fm| Self::yaml_string_list(fm, "tags").join(","))
            .unwrap_or_default();

        MarkdownMetadata {
            title,
            page_type,
            canonical_name,
            aliases,
            tags,
            body,
        }
    }

    pub fn best_title(content: &str, relative_path: &str, preferred: Option<&str>) -> String {
        preferred
            .map(str::trim)
            .filter(|s| !s.is_empty() && !Self::looks_like_generated_page_id(s))
            .map(str::to_string)
            .unwrap_or_else(|| Self::extract_metadata(content, relative_path).title)
    }

    pub fn extract_h1(content: &str) -> Option<String> {
        content.lines().find_map(|line| {
            let trimmed = line.trim();
            trimmed
                .strip_prefix("# ")
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string)
        })
    }

    pub fn title_from_path(path: &str) -> String {
        let stem = path
            .trim_end_matches(".md")
            .split('/')
            .next_back()
            .unwrap_or("Untitled")
            .trim();
        if Self::looks_like_generated_page_id(stem) {
            "Untitled".to_string()
        } else {
            stem.replace('-', " ")
        }
    }

    pub fn looks_like_generated_page_id(value: &str) -> bool {
        let lower = value.trim().to_ascii_lowercase();
        lower.starts_with("page-") && lower.len() >= 10
    }

    fn collect_markdown_files(dir: &Path, out: &mut Vec<PathBuf>) -> std::io::Result<()> {
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                Self::collect_markdown_files(&path, out)?;
            } else if path
                .extension()
                .and_then(|ext| ext.to_str())
                .is_some_and(|ext| ext.eq_ignore_ascii_case("md"))
            {
                out.push(path);
            }
        }
        Ok(())
    }

    fn split_frontmatter(content: &str) -> (Option<serde_yaml::Value>, String) {
        let normalized = content.strip_prefix('\u{feff}').unwrap_or(content);
        if let Some(rest) = normalized.strip_prefix("---\n") {
            if let Some(end) = rest.find("\n---\n") {
                let yaml_text = &rest[..end];
                let body = rest[end + 5..].to_string();
                let yaml = serde_yaml::from_str::<serde_yaml::Value>(yaml_text).ok();
                return (yaml, body);
            }
        }
        (None, normalized.to_string())
    }

    fn yaml_string(value: &serde_yaml::Value, key: &str) -> Option<String> {
        value
            .get(key)
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
    }

    fn yaml_string_list(value: &serde_yaml::Value, key: &str) -> Vec<String> {
        match value.get(key) {
            Some(serde_yaml::Value::Sequence(seq)) => seq
                .iter()
                .filter_map(|v| v.as_str())
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string)
                .collect(),
            Some(serde_yaml::Value::String(s)) => s
                .split(',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string)
                .collect(),
            _ => Vec::new(),
        }
    }

    fn plain_summary(body: &str) -> String {
        body.lines()
            .filter(|line| {
                let trimmed = line.trim();
                !trimmed.is_empty() && !trimmed.starts_with('#')
            })
            .collect::<Vec<_>>()
            .join(" ")
            .chars()
            .take(300)
            .collect()
    }
}
