/// PDF 文本提取 Skill
/// 双路径策略：Tier1 lopdf 文本层提取 → Tier2 OCR 回退（扫描件）
use std::path::Path;
use lopdf::Document;
use crate::skills::pdf_ocr::PdfOcr;

pub struct PdfSkill;

impl PdfSkill {
    /// 提取 PDF 全部文本（双路径：文本层 + OCR 回退）
    /// 返回 (文本内容, 警告列表)
    pub fn extract_text(file_path: &Path) -> Result<(String, Vec<String>), String> {
        let doc = Document::load(file_path)
            .map_err(|e| format!("无法打开 PDF 文件: {}", e))?;

        let mut warnings = Vec::new();
        let mut all_text = Vec::new();

        // Tier 1: 使用 lopdf 提取文本层
        for (page_num, _object_id) in doc.page_iter() {
            if let Ok(text) = doc.extract_text(&[page_num]) {
                let cleaned = text.trim().to_string();
                if !cleaned.is_empty() {
                    all_text.push(format!("--- 第 {} 页 ---\n{}", page_num, cleaned));
                }
            }
        }

        if !all_text.is_empty() {
            let combined = all_text.join("\n\n");
            let text_len = combined.len();
            // 检测乱码：高比例 Latin-1 补充区字符暗示 CJK 文本未被正确解码
            if Self::looks_like_garbled_cjk(&combined) {
                warnings.push(
                    "⚠️ 内置文本层包含大量乱码字符，可能是中文字体解码问题。建议启用 MarkItDown 以获得更好的 PDF 提取效果。".to_string()
                );
            }
            if text_len < 200 {
                warnings.push(format!(
                    "文本层内容较少({}字符)，可能为扫描件。已尝试文本层提取。",
                    text_len
                ));
            }
            return Ok((combined, warnings));
        }

        // 文本层为空，尝试另一种 lopdf 提取方式
        let mut text = String::new();
        for page_id in doc.page_iter() {
            if let Ok(page_text) = doc.extract_text(&[page_id.0]) {
                text.push_str(&page_text);
                text.push('\n');
            }
        }

        if !text.trim().is_empty() {
            let cleaned = text.trim().to_string();
            warnings.push("使用备用文本提取方式获得内容。".to_string());
            return Ok((cleaned, warnings));
        }

        // Tier 2: 文本层完全为空，启动 OCR 回退路径
        warnings.push("PDF无文本层（扫描件/图片型PDF），启动Windows OCR识别。".to_string());

        match PdfOcr::ocr_document(file_path) {
            Ok(page_texts) => {
                let ocr_text = page_texts.join("\n\n");
                if ocr_text.trim().is_empty()
                    || ocr_text
                        .lines()
                        .all(|l| l.starts_with("[第") || l.starts_with("[图像"))
                {
                    return Err(format!(
                        "PDF无文本层且OCR未能识别出文字。OCR结果: {}",
                        &ocr_text[..ocr_text.len().min(500)]
                    ));
                }
                warnings.push("文本由OCR识别生成，可能存在识别误差。".to_string());
                Ok((ocr_text, warnings))
            }
            Err(ocr_err) => Err(format!(
                "此PDF文件无文本层（可能为扫描件），且OCR失败: {}",
                ocr_err
            )),
        }
    }

    /// 获取 PDF 页数
    pub fn get_page_count(file_path: &Path) -> Result<usize, String> {
        let doc = Document::load(file_path)
            .map_err(|e| format!("无法打开 PDF 文件: {}", e))?;
        Ok(doc.page_iter().count())
    }

    /// 检测文本是否为乱码 CJK 内容（lopdf 对 CID 字体解码不完整时触发）
    fn looks_like_garbled_cjk(text: &str) -> bool {
        let suspect_count = text
            .chars()
            .filter(|c| (*c as u32) > 0x007F && (*c as u32) < 0x0230)
            .count();
        let total = text.chars().count();
        total > 0 && suspect_count > total / 3
    }

    /// 提取指定页面文本
    pub fn extract_page_text(file_path: &Path, page_num: u32) -> Result<String, String> {
        let doc = Document::load(file_path)
            .map_err(|e| format!("无法打开 PDF 文件: {}", e))?;
        doc.extract_text(&[page_num])
            .map_err(|e| format!("提取第 {} 页文本失败: {}", page_num, e))
    }
}
