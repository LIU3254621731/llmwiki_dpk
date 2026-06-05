/// TXT 文本提取 Skill
/// 纯文本文件直接读取
use std::path::Path;

pub struct TxtSkill;

impl TxtSkill {
    /// 提取 TXT 文本
    pub fn extract_text(file_path: &Path) -> Result<String, String> {
        let bytes = std::fs::read(file_path)
            .map_err(|e| format!("无法读取 TXT 文件: {}", e))?;

        // 尝试 UTF-8 解码
        if let Ok(text) = String::from_utf8(bytes.clone()) {
            if text.trim().is_empty() {
                return Err("TXT 文件内容为空。".to_string());
            }
            return Ok(text);
        }

        // 尝试 GBK/GB2312 解码（中文 Windows 常用编码）
        let (cow, _encoding, had_errors) = encoding_rs::Encoding::for_label("gbk".as_bytes())
            .unwrap_or(encoding_rs::UTF_8)
            .decode(&bytes);

        if had_errors {
            // 尝试 GB18030
            let (cow2, _, had_errors2) = encoding_rs::Encoding::for_label("gb18030".as_bytes())
                .unwrap_or(encoding_rs::UTF_8)
                .decode(&bytes);
            if !had_errors2 {
                return Ok(cow2.into_owned());
            }
        }

        let text = cow.into_owned();
        if text.trim().is_empty() {
            return Err("TXT 文件内容为空或无法解码。".to_string());
        }
        Ok(text)
    }
}
