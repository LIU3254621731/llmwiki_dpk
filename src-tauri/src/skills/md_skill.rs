/// Markdown 文本提取 Skill
/// MD 文件直接读取，去除少量标记，保留主要内容结构
use std::path::Path;

pub struct MdSkill;

impl MdSkill {
    /// 提取 Markdown 文本（保留大部分结构）
    pub fn extract_text(file_path: &Path) -> Result<String, String> {
        let content = std::fs::read_to_string(file_path)
            .map_err(|e| format!("无法读取 Markdown 文件: {}", e))?;

        if content.trim().is_empty() {
            return Err("Markdown 文件内容为空。".to_string());
        }

        Ok(Self::clean_markdown(&content))
    }

    /// 清理 Markdown，保留有意义的文本
    fn clean_markdown(content: &str) -> String {
        content
            .lines()
            .filter(|line| {
                let trimmed = line.trim();
                // 跳过纯分隔线
                if trimmed.chars().all(|c| c == '-' || c == '=' || c == '*' || c == '_' || c == '#') && trimmed.len() > 3 {
                    return false;
                }
                true
            })
            .map(|line| line.trim().to_string())
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// 获取 Markdown 文件 frontmatter
    pub fn extract_frontmatter(content: &str) -> Option<String> {
        if content.trim_start().starts_with("---") {
            let parts: Vec<&str> = content.trim_start().splitn(3, "---").collect();
            if parts.len() >= 2 {
                return Some(parts[1].trim().to_string());
            }
        }
        None
    }
}
