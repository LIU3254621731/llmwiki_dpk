// IndexService - index.md 维护

use std::path::Path;
use crate::core::database_service::DatabaseService;

pub struct IndexService {
    db: std::sync::Arc<DatabaseService>,
}

impl IndexService {
    pub fn new(db: std::sync::Arc<DatabaseService>) -> Self {
        Self { db }
    }

    /// 重建 index.md
    pub fn rebuild_index(&self, kb_id: &str, wiki_dir: &Path) -> Result<(), String> {
        let conn = self.db.connect()?;
        let mut stmt = conn
            .prepare("SELECT title, path, page_type, canonical_name, COALESCE(tags,''), created_at FROM wiki_pages WHERE kb_id = ?1 ORDER BY page_type, title")
            .map_err(|e| format!("查询页面失败: {}", e))?;

        let pages: Vec<(String, String, String, String, String, String)> = stmt
            .query_map(rusqlite::params![kb_id], |row| {
                Ok((
                    row.get(0)?, row.get(1)?, row.get(2)?,
                    row.get(3)?, row.get(4)?, row.get(5)?,
                ))
            })
            .map_err(|e| format!("映射页面失败: {}", e))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("收集页面失败: {}", e))?;

        let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
        let mut index = format!("# 知识库索引\n\n> 最后更新: {}\n\n", now);

        index.push_str("## 页面列表\n\n");

        let mut current_type = String::new();
        for (title, path, pt, cn, tags, created) in &pages {
            if *pt != current_type {
                current_type = pt.clone();
                index.push_str(&format!("### {}\n\n", current_type));
            }
            let tag_str = if tags.is_empty() { String::new() } else { format!(" `{}`", tags) };
            index.push_str(&format!("- [{}]({}) ({}){}{}\n", title, path, cn, tag_str,
                if created.is_empty() { String::new() } else { format!(" - {}", created) }
            ));
        }

        if pages.is_empty() {
            index.push_str("暂无页面。\n");
        }

        let index_path = wiki_dir.join("index.md");
        std::fs::write(&index_path, index)
            .map_err(|e| format!("写入 index.md 失败: {}", e))?;

        Ok(())
    }

    /// 追加页面到 index.md
    pub fn append_to_index(
        &self,
        wiki_dir: &Path,
        title: &str,
        path: &str,
        page_type: &str,
    ) -> Result<(), String> {
        let index_path = wiki_dir.join("index.md");
        let mut content = std::fs::read_to_string(&index_path)
            .unwrap_or_else(|_| "# 知识库索引\n\n## 页面列表\n\n".to_string());

        let entry = format!("- [{}]({}) ({})\n", title, path, page_type);

        if let Some(pos) = content.find("## 页面列表") {
            let insert_pos = content[pos..].find('\n').map(|p| pos + p + 1).unwrap_or(content.len());
            content.insert_str(insert_pos, &entry);
        } else {
            content.push_str(&format!("\n## 页面列表\n\n{}", entry));
        }

        std::fs::write(&index_path, content)
            .map_err(|e| format!("更新 index.md 失败: {}", e))?;

        Ok(())
    }

    /// 从 index.md 中移除页面
    pub fn remove_from_index(
        &self,
        wiki_dir: &Path,
        path: &str,
    ) -> Result<(), String> {
        let index_path = wiki_dir.join("index.md");
        let content = std::fs::read_to_string(&index_path)
            .unwrap_or_default();

        let search = format!("({})", path);
        let new_content = content.lines()
            .filter(|line| !line.contains(&search))
            .collect::<Vec<_>>()
            .join("\n");

        std::fs::write(&index_path, new_content)
            .map_err(|e| format!("更新 index.md 失败: {}", e))?;

        Ok(())
    }

    /// 获取页面统计信息
    pub fn get_stats(&self, kb_id: &str) -> Result<serde_json::Value, String> {
        let conn = self.db.connect()?;

        let total_pages: i64 = match conn.query_row(
            "SELECT COUNT(*) FROM wiki_pages WHERE kb_id = ?1",
            rusqlite::params![kb_id],
            |row| row.get(0),
        ) {
            Ok(c) => c,
            Err(rusqlite::Error::QueryReturnedNoRows) => 0,
            Err(e) => return Err(format!("查询总页数失败: {}", e)),
        };

        let by_type: Vec<(String, i64)> = {
            let mut stmt = conn.prepare(
                "SELECT page_type, COUNT(*) FROM wiki_pages WHERE kb_id = ?1 GROUP BY page_type"
            ).map_err(|e| format!("准备查询失败: {}", e))?;
            let mapped = stmt.query_map(rusqlite::params![kb_id], |row| {
                Ok((row.get(0)?, row.get(1)?))
            }).map_err(|e| format!("查询失败: {}", e))?;
            mapped.filter_map(|r| r.ok()).collect()
        };

        let mut type_stats = serde_json::Map::new();
        for (pt, count) in by_type {
            type_stats.insert(pt, serde_json::json!(count));
        }

        Ok(serde_json::json!({
            "total_pages": total_pages,
            "by_type": type_stats,
        }))
    }
}
