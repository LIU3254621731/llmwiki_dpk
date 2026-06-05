// 本地全文搜索（v0.1.4 增强版）
// v0.1.4: 添加模糊搜索、tags 搜索、路径正确解析

use std::sync::Arc;
use crate::core::database_service::DatabaseService;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SearchResult {
    pub title: String,
    pub path: String,
    pub page_type: String,
    pub matched_field: String,
    pub snippet: String,
    pub updated_at: String,
    pub page_id: String,
    pub tags: Vec<String>,
    pub is_broken: bool,
}

pub struct FullTextSearch;

impl FullTextSearch {
    /// 在 Wiki 页面中执行全文搜索（v0.1.4: 增强模糊搜索）
    pub fn search(
        db: &Arc<DatabaseService>,
        kb_id: &str,
        query: &str,
        wiki_dir: &std::path::Path,
    ) -> Result<Vec<SearchResult>, String> {
        let conn = db.connect()?;
        let mut results = Vec::new();
        let query_lower = query.to_lowercase();

        // 辅助函数：检查 path 是否已存在
        let has_path = |results: &Vec<SearchResult>, p: &str| -> bool {
            results.iter().any(|r: &SearchResult| r.path.as_str() == p)
        };

        // 1. 搜索 title 字段
        let like_pattern = format!("%{}%", query_lower);
        {
            let mut stmt = conn
                .prepare("SELECT id, title, path, page_type, COALESCE(tags,''), COALESCE(updated_at,''), COALESCE(status,'active') FROM wiki_pages WHERE kb_id = ?1 AND LOWER(title) LIKE ?2")
                .map_err(|e| format!("搜索失败: {}", e))?;

            let title_results: Vec<(String, String, String, String, String, String, String)> = stmt
                .query_map(rusqlite::params![kb_id, like_pattern], |row| {
                    Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?, row.get(6)?))
                })
                .map_err(|e| format!("映射结果失败: {}", e))?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| format!("收集结果失败: {}", e))?;

            for (page_id, title, path, page_type, tags_str, updated_at, status) in title_results {
                if !has_path(&results, &path) {
                    let tags: Vec<String> = tags_str.split_whitespace().map(|s| s.to_string()).collect();
                    results.push(SearchResult {
                        title: title.clone(), path: path.clone(), page_type: page_type.clone(),
                        matched_field: "title".to_string(), snippet: title,
                        updated_at, page_id, tags, is_broken: status == "broken",
                    });
                }
            }
        }

        // 2. 搜索 canonical_name 字段
        {
            let mut stmt = conn
                .prepare("SELECT id, title, path, page_type, canonical_name, COALESCE(updated_at,''), COALESCE(tags,''), COALESCE(status,'active') FROM wiki_pages WHERE kb_id = ?1 AND LOWER(canonical_name) LIKE ?2")
                .map_err(|e| format!("搜索失败: {}", e))?;

            let cn_results: Vec<(String, String, String, String, String, String, String, String)> = stmt
                .query_map(rusqlite::params![kb_id, like_pattern], |row| {
                    Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?, row.get(6)?, row.get(7)?))
                })
                .map_err(|e| format!("映射结果失败: {}", e))?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| format!("收集结果失败: {}", e))?;

            for (page_id, title, path, page_type, cn, updated_at, tags_str, status) in cn_results {
                if !has_path(&results, &path) {
                    let tags: Vec<String> = tags_str.split_whitespace().map(|s| s.to_string()).collect();
                    results.push(SearchResult {
                        title, path: path.clone(), page_type,
                        matched_field: "canonical_name".to_string(), snippet: cn,
                        updated_at, page_id, tags, is_broken: status == "broken",
                    });
                }
            }
        }

        // 3. 搜索 tags 字段
        {
            let mut stmt = conn
                .prepare("SELECT id, title, path, page_type, COALESCE(tags,''), COALESCE(updated_at,''), COALESCE(status,'active') FROM wiki_pages WHERE kb_id = ?1 AND LOWER(tags) LIKE ?2")
                .map_err(|e| format!("搜索 tags 失败: {}", e))?;

            let tag_results: Vec<(String, String, String, String, String, String, String)> = stmt
                .query_map(rusqlite::params![kb_id, like_pattern], |row| {
                    Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?, row.get(6)?))
                })
                .map_err(|e| format!("映射 tags 失败: {}", e))?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| format!("收集 tags 失败: {}", e))?;

            for (page_id, title, path, page_type, tags_str, updated_at, status) in tag_results {
                if !has_path(&results, &path) {
                    let tags: Vec<String> = tags_str.split_whitespace().map(|s| s.to_string()).collect();
                    results.push(SearchResult {
                        title, path: path.clone(), page_type,
                        matched_field: "tags".to_string(), snippet: format!("标签: {}", tags_str),
                        updated_at, page_id, tags, is_broken: status == "broken",
                    });
                }
            }
        }

        // 4. 搜索实际文件内容（使用正确的路径解析）
        for (title, path, page_type, updated_at, content) in Self::search_file_contents(wiki_dir, &query_lower)? {
            if !has_path(&results, &path) {
                let snippet = Self::make_snippet(&content, &query_lower);
                results.push(SearchResult {
                    title, path, page_type,
                    matched_field: "content".to_string(), snippet,
                    updated_at, page_id: String::new(), tags: vec![], is_broken: false,
                });
            }
        }

        // 5. 搜索 aliases（v0.1.4: 同时通过别名反查页面）
        {
            let mut stmt = conn
                .prepare("SELECT a.normalized_alias, k.canonical_name, k.page_path FROM aliases a JOIN knowledge_items k ON a.item_id = k.id WHERE k.kb_id = ?1 AND LOWER(a.normalized_alias) LIKE ?2")
                .map_err(|e| format!("搜索 aliases 失败: {}", e))?;

            let alias_results: Vec<(String, String, String)> = stmt
                .query_map(rusqlite::params![kb_id, like_pattern], |row| {
                    Ok((row.get::<_,String>(0)?, row.get::<_,String>(1)?, row.get::<_,String>(2)?))
                })
                .map_err(|e| format!("映射 aliases 失败: {}", e))?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| format!("收集 aliases 失败: {}", e))?;

            for (alias, cn, page_path) in alias_results {
                if !has_path(&results, &page_path) {
                    results.push(SearchResult {
                        title: cn.clone(), path: page_path.clone(), page_type: "alias".to_string(),
                        matched_field: "alias".to_string(), snippet: format!("别名: {}", alias),
                        updated_at: String::new(), page_id: String::new(), tags: vec![], is_broken: false,
                    });
                }
            }
        }

        // 6. 模糊搜索：当精确子串搜索无结果时尝试更宽松的匹配
        if results.is_empty() && query_lower.len() >= 2 {
            let fuzzy_pattern = Self::build_fuzzy_pattern(&query_lower);
            {
                let mut stmt = conn
                    .prepare("SELECT id, title, path, page_type, COALESCE(tags,''), COALESCE(updated_at,''), COALESCE(status,'active') FROM wiki_pages WHERE kb_id = ?1 AND (LOWER(title) LIKE ?2 OR LOWER(canonical_name) LIKE ?2)")
                    .map_err(|e| format!("模糊搜索失败: {}", e))?;

                let fuzzy_results: Vec<(String, String, String, String, String, String, String)> = stmt
                    .query_map(rusqlite::params![kb_id, fuzzy_pattern], |row| {
                        Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?, row.get(6)?))
                    })
                    .map_err(|e| format!("映射模糊结果失败: {}", e))?
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|e| format!("收集模糊结果失败: {}", e))?;

                for (page_id, title, path, page_type, tags_str, updated_at, status) in fuzzy_results {
                    if !has_path(&results, &path) {
                        let tags: Vec<String> = tags_str.split_whitespace().map(|s| s.to_string()).collect();
                        results.push(SearchResult {
                            title, path: path.clone(), page_type,
                            matched_field: "fuzzy".to_string(), snippet: "模糊匹配".to_string(),
                            updated_at, page_id, tags, is_broken: status == "broken",
                        });
                    }
                }
            }
        }

        Ok(results)
    }

    /// 构建模糊匹配模式：将连续字符拆分为 %c%h%a%r% 格式
    fn build_fuzzy_pattern(query: &str) -> String {
        let mut pattern = String::with_capacity(query.len() * 2 + 2);
        pattern.push('%');
        for ch in query.chars() {
            pattern.push(ch);
            pattern.push('%');
        }
        pattern
    }

    fn search_file_contents(wiki_dir: &std::path::Path, query: &str) -> Result<Vec<(String, String, String, String, String)>, String> {
        let mut results = Vec::new();
        if let Err(e) = Self::scan_dir(wiki_dir, wiki_dir, query, &mut results) {
            log::error!("[full_text_search] 扫描目录失败 (dir={}): {}", wiki_dir.display(), e);
        }
        Ok(results)
    }

    fn scan_dir(
        base: &std::path::Path,
        dir: &std::path::Path,
        query: &str,
        results: &mut Vec<(String, String, String, String, String)>,
    ) -> std::io::Result<()> {
        if !dir.exists() {
            return Ok(());
        }

        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                if let Err(e) = Self::scan_dir(base, &path, query, results) {
                    log::error!("[full_text_search] 扫描子目录失败 (dir={}): {}", path.display(), e);
                }
            } else if path.extension().is_some_and(|e| e == "md") {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    if content.to_lowercase().contains(query) {
                        let relative = path.strip_prefix(base).unwrap_or(&path).to_string_lossy().to_string();
                        let title = path.file_stem().unwrap_or_default().to_string_lossy().to_string();

                        let page_type = if let Some(rest) = content.strip_prefix("---") {
                            if let Some(end) = rest.find("---") {
                                let fm = &rest[..end];
                                fm.lines()
                                    .find(|l| l.starts_with("type:"))
                                    .and_then(|l| l.strip_prefix("type:"))
                                    .map(|t| t.trim().to_string())
                                    .unwrap_or_else(|| "unknown".to_string())
                            } else {
                                "unknown".to_string()
                            }
                        } else {
                            "unknown".to_string()
                        };

                        results.push((title, relative, page_type, String::new(), content));
                    }
                }
            }
        }

        Ok(())
    }

    fn make_snippet(content: &str, query: &str) -> String {
        let content_lower = content.to_lowercase();
        if let Some(pos) = content_lower.find(query) {
            let start = pos.saturating_sub(50);
            let end = (pos + query.len() + 50).min(content.len());
            let mut snippet = String::new();
            if start > 0 { snippet.push_str("..."); }
            snippet.push_str(&content[start..end]);
            if end < content.len() { snippet.push_str("..."); }
            snippet
        } else {
            content.chars().take(200).collect()
        }
    }
}
