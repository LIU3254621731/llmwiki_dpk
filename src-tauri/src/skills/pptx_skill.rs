/// PPTX 文本提取 Skill
/// PPTX 本质是 ZIP 压缩包，包含 XML 幻灯片文件
/// 解压后从 ppt/slides/slide*.xml 提取文本
use std::io::Read;
use std::path::Path;
use regex::Regex;

pub struct PptxSkill;

impl PptxSkill {
    /// 提取 PPTX 全部文本（所有幻灯片）
    pub fn extract_text(file_path: &Path) -> Result<String, String> {
        let file = std::fs::File::open(file_path)
            .map_err(|e| format!("无法打开 PPTX 文件: {}", e))?;

        let mut archive = zip::ZipArchive::new(file)
            .map_err(|e| format!("无法解析 PPTX (ZIP) 文件: {}", e))?;

        let mut all_slides = Vec::new();

        for i in 1.. {
            let entry_path = format!("ppt/slides/slide{}.xml", i);
            let mut slide_xml = String::new();
            match archive.by_name(&entry_path) {
                Ok(mut entry) => {
                    entry
                        .read_to_string(&mut slide_xml)
                        .map_err(|e| format!("读取幻灯片 {} 失败: {}", i, e))?;
                    let slide_text = Self::extract_text_from_slide_xml(&slide_xml)?;
                    if !slide_text.trim().is_empty() {
                        all_slides.push(format!("--- 幻灯片 {} ---\n{}", i, slide_text));
                    }
                }
                Err(_) => break, // 无更多幻灯片
            }
        }

        if all_slides.is_empty() {
            return Err("PPTX 文件中未找到文本内容。".to_string());
        }

        Ok(all_slides.join("\n\n"))
    }

    /// 从单个幻灯片 XML 提取文本
    /// PPTX 绘图命名空间: <a:t>文本</a:t>
    fn extract_text_from_slide_xml(xml: &str) -> Result<String, String> {
        let at_re = Regex::new(r"<a:t[^>]*>([^<]*)</a:t>")
            .map_err(|e| format!("PPTX regex 编译失败: {}", e))?;
        let mut paragraphs = Vec::new();

        for cap in at_re.captures_iter(xml) {
            let text = cap[1].trim().to_string();
            if !text.is_empty() {
                paragraphs.push(text);
            }
        }

        Ok(paragraphs.join("\n"))
    }
}
