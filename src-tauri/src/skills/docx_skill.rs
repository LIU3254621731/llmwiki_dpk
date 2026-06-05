/// DOCX 文本提取 Skill
/// DOCX 本质是 ZIP 压缩包，包含 XML 文件
/// 解压后从 word/document.xml 提取文本
use std::io::Read;
use std::path::Path;
use regex::Regex;

pub struct DocxSkill;

impl DocxSkill {
    /// 提取 DOCX 全部文本
    pub fn extract_text(file_path: &Path) -> Result<String, String> {
        let file = std::fs::File::open(file_path)
            .map_err(|e| format!("无法打开 DOCX 文件: {}", e))?;

        let mut archive = zip::ZipArchive::new(file)
            .map_err(|e| format!("无法解析 DOCX (ZIP) 文件: {}", e))?;

        let mut doc_xml = String::new();
        {
            let mut entry = archive
                .by_name("word/document.xml")
                .map_err(|e| format!("DOCX 文件结构异常，缺少 word/document.xml: {}", e))?;
            entry
                .read_to_string(&mut doc_xml)
                .map_err(|e| format!("读取 DOCX 内容失败: {}", e))?;
        }

        let text = Self::extract_text_from_xml(&doc_xml)?;

        if text.trim().is_empty() {
            return Err("DOCX 文件中未找到文本内容。".to_string());
        }

        Ok(text)
    }

    /// 用正则从 DOCX XML 提取所有段落文本
    fn extract_text_from_xml(xml: &str) -> Result<String, String> {
        let wt_re = Regex::new(r"<w:t[^>]*>([^<]*)</w:t>")
            .map_err(|e| format!("DOCX regex 编译失败: {}", e))?;
        let space_re = Regex::new(r"<w:t[^>]*/>")
            .map_err(|e| format!("DOCX regex 编译失败: {}", e))?;

        // 按段落分割
        let paragraphs: Vec<&str> = xml.split("</w:p>").collect();
        let mut result = Vec::new();

        for para in &paragraphs {
            // 跳过空段落标记
            let cleaned = space_re.replace_all(para, "");
            let mut para_text = String::new();
            for cap in wt_re.captures_iter(&cleaned) {
                para_text.push_str(&cap[1]);
            }
            let trimmed = para_text.trim().to_string();
            if !trimmed.is_empty() {
                result.push(trimmed);
            }
        }

        Ok(result.join("\n"))
    }
}
