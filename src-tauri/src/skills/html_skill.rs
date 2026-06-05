/// HTML 文本提取 Skill
/// 从 HTML 中提取纯文本内容
use std::path::Path;

pub struct HtmlSkill;

impl HtmlSkill {
    /// 提取 HTML 全部文本
    pub fn extract_text(file_path: &Path) -> Result<String, String> {
        let content = std::fs::read_to_string(file_path)
            .map_err(|e| format!("无法读取 HTML 文件: {}", e))?;

        let text = Self::html_to_text(&content);

        if text.trim().is_empty() {
            return Err("HTML 文件中未找到可提取的文本内容。".to_string());
        }

        Ok(text)
    }

    /// HTML 转纯文本
    fn html_to_text(html: &str) -> String {
        let mut text = String::new();
        let mut skip = false;
        let mut in_script = false;
        let mut in_style = false;

        let bytes = html.as_bytes();
        let mut i = 0;

        while i < bytes.len() {
            if bytes[i] == b'<' {
                // 检查是否是 script/style 标签开始
                let remaining = &html[i..];
                let remaining_lower = remaining.to_lowercase();

                if remaining_lower.starts_with("<script") {
                    in_script = true;
                } else if remaining_lower.starts_with("</script>") {
                    in_script = false;
                } else if remaining_lower.starts_with("<style") {
                    in_style = true;
                } else if remaining_lower.starts_with("</style>") {
                    in_style = false;
                }

                if !in_script && !in_style {
                    // 标签结束添加空格
                    if bytes.get(i + 1).is_some_and(|&b| {
                        b == b'/' || b == b'p' || b == b'b' || b == b'd' || b == b'h' || b == b'l' || b == b't'
                    })
                        && !text.ends_with('\n') && !text.ends_with(' ') && !text.is_empty() {
                            text.push(' ');
                        }
                }

                skip = true;
                continue;
            }

            if bytes[i] == b'>' {
                skip = false;

                // 块级元素结束添加换行
                let tag_start = html[..i].rfind('<').unwrap_or(0);
                let tag = &html[tag_start..=i].to_lowercase();
                if tag.contains("</p") || tag.contains("</div") || tag.contains("</h") ||
                   tag.contains("</li") || tag.contains("</tr") || tag.contains("<br") {
                    text.push('\n');
                }

                i += 1;
                continue;
            }

            if !skip && !in_script && !in_style {
                text.push(bytes[i] as char);
            }

            i += 1;
        }

        // 清理文本
        let cleaned = text
            .lines()
            .map(|line| line.trim())
            .filter(|line| !line.is_empty())
            .collect::<Vec<_>>()
            .join("\n");

        // 解码 HTML 实体
        

        cleaned
            .replace("&amp;", "&")
            .replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&quot;", "\"")
            .replace("&apos;", "'")
            .replace("&nbsp;", " ")
            .replace("&#39;", "'")
    }
}
