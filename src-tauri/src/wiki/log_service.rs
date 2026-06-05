// LogService - log.md 维护

use std::path::Path;

pub struct LogService;

impl LogService {
    /// 追加日志条目
    pub fn append_log(
        wiki_dir: &Path,
        event_type: &str,
        description: &str,
    ) -> Result<(), String> {
        let log_path = wiki_dir.join("log.md");

        let mut content = std::fs::read_to_string(&log_path)
            .unwrap_or_else(|_| "# 知识库操作日志\n\n".to_string());

        let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
        let entry = format!("\n## {} - {}\n\n{}\n", now, event_type, description);

        content.push_str(&entry);

        std::fs::write(&log_path, content)
            .map_err(|e| format!("写入 log.md 失败: {}", e))?;

        Ok(())
    }

    /// 获取最近的操作日志
    pub fn get_recent_logs(
        wiki_dir: &Path,
        limit: usize,
    ) -> Result<Vec<serde_json::Value>, String> {
        let log_path = wiki_dir.join("log.md");
        let content = std::fs::read_to_string(&log_path)
            .unwrap_or_default();

        let entries: Vec<serde_json::Value> = {
            let mut filtered: Vec<&str> = content
                .split("## ")
                .filter(|s| s.contains(" - "))
                .collect();
            filtered.reverse();
            filtered.into_iter()
                .take(limit)
                .filter_map(|entry| {
                    let parts: Vec<&str> = entry.splitn(2, " - ").collect();
                    if parts.len() == 2 {
                        let (time, rest) = (parts[0].trim(), parts[1]);
                        let desc_lines: Vec<&str> = rest.split("\n\n").collect();
                        Some(serde_json::json!({
                            "time": time,
                            "type": desc_lines.first().unwrap_or(&"").trim(),
                            "description": desc_lines.get(1).unwrap_or(&"").trim(),
                        }))
                    } else {
                        None
                    }
                })
                .collect()
        };

        Ok(entries)
    }

    /// 记录批量操作开始/结束
    pub fn log_batch_operation(
        wiki_dir: &Path,
        operation: &str,
        count: usize,
        status: &str,
    ) -> Result<(), String> {
        let desc = format!("批量{} {} 个项目 - {}", operation, count, status);
        Self::append_log(wiki_dir, "batch_operation", &desc)
    }
}
