/// 文档处理器：统一的文档解析入口
/// 根据文件类型自动选择对应的 Skill 进行文本提取
use std::path::Path;
use crate::skills::{pdf_skill, docx_skill, pptx_skill, html_skill, md_skill, txt_skill, markitdown_skill};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DocumentParseResult {
    pub file_name: String,
    pub file_type: String,
    pub text: String,
    pub text_length: usize,
    pub page_count: Option<usize>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SupportedFileType {
    pub extension: String,
    pub mime_type: String,
    pub description: String,
    pub is_document: bool,  // true=文档类（需提取文本），false=资产类（图片等）
}

pub struct DocumentProcessor;

impl DocumentProcessor {
    /// 获取支持的文件类型列表
    pub fn get_supported_types() -> Vec<SupportedFileType> {
        vec![
            SupportedFileType {
                extension: "pdf".to_string(),
                mime_type: "application/pdf".to_string(),
                description: "PDF 文档".to_string(),
                is_document: true,
            },
            SupportedFileType {
                extension: "docx".to_string(),
                mime_type: "application/vnd.openxmlformats-officedocument.wordprocessingml.document".to_string(),
                description: "Word 文档".to_string(),
                is_document: true,
            },
            SupportedFileType {
                extension: "md".to_string(),
                mime_type: "text/markdown".to_string(),
                description: "Markdown 文档".to_string(),
                is_document: true,
            },
            SupportedFileType {
                extension: "markdown".to_string(),
                mime_type: "text/markdown".to_string(),
                description: "Markdown 文档".to_string(),
                is_document: true,
            },
            SupportedFileType {
                extension: "txt".to_string(),
                mime_type: "text/plain".to_string(),
                description: "纯文本文件".to_string(),
                is_document: true,
            },
            SupportedFileType {
                extension: "html".to_string(),
                mime_type: "text/html".to_string(),
                description: "HTML 文档".to_string(),
                is_document: true,
            },
            SupportedFileType {
                extension: "htm".to_string(),
                mime_type: "text/html".to_string(),
                description: "HTML 文档".to_string(),
                is_document: true,
            },
            SupportedFileType {
                extension: "pptx".to_string(),
                mime_type: "application/vnd.openxmlformats-officedocument.presentationml.presentation".to_string(),
                description: "PowerPoint 演示文稿".to_string(),
                is_document: true,
            },
            SupportedFileType {
                extension: "xlsx".to_string(),
                mime_type: "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet".to_string(),
                description: "Excel 电子表格".to_string(),
                is_document: true,
            },
            SupportedFileType {
                extension: "xls".to_string(),
                mime_type: "application/vnd.ms-excel".to_string(),
                description: "Excel 电子表格 (旧版)".to_string(),
                is_document: true,
            },
            SupportedFileType {
                extension: "csv".to_string(),
                mime_type: "text/csv".to_string(),
                description: "CSV 表格".to_string(),
                is_document: true,
            },
            SupportedFileType {
                extension: "json".to_string(),
                mime_type: "application/json".to_string(),
                description: "JSON 数据".to_string(),
                is_document: true,
            },
            SupportedFileType {
                extension: "xml".to_string(),
                mime_type: "application/xml".to_string(),
                description: "XML 数据".to_string(),
                is_document: true,
            },
            SupportedFileType {
                extension: "png".to_string(),
                mime_type: "image/png".to_string(),
                description: "PNG 图片（仅保存为资产）".to_string(),
                is_document: false,
            },
            SupportedFileType {
                extension: "jpg".to_string(),
                mime_type: "image/jpeg".to_string(),
                description: "JPG 图片（仅保存为资产）".to_string(),
                is_document: false,
            },
            SupportedFileType {
                extension: "jpeg".to_string(),
                mime_type: "image/jpeg".to_string(),
                description: "JPEG 图片（仅保存为资产）".to_string(),
                is_document: false,
            },
            SupportedFileType {
                extension: "webp".to_string(),
                mime_type: "image/webp".to_string(),
                description: "WebP 图片（仅保存为资产）".to_string(),
                is_document: false,
            },
            SupportedFileType {
                extension: "gif".to_string(),
                mime_type: "image/gif".to_string(),
                description: "GIF 图片（仅保存为资产）".to_string(),
                is_document: false,
            },
        ]
    }

    /// 检查文件类型是否支持
    pub fn is_supported(extension: &str) -> bool {
        let ext = extension.to_lowercase();
        Self::get_supported_types()
            .iter()
            .any(|t| t.extension == ext)
    }

    /// 检查是否为文档类型（需要文本提取）
    pub fn is_document_type(extension: &str) -> bool {
        let ext = extension.to_lowercase();
        Self::get_supported_types()
            .iter()
            .any(|t| t.extension == ext && t.is_document)
    }

    /// 检查是否为图片/资产类型（仅保存不提取）
    pub fn is_asset_type(extension: &str) -> bool {
        let ext = extension.to_lowercase();
        Self::get_supported_types()
            .iter()
            .any(|t| t.extension == ext && !t.is_document)
    }

    /// 根据文件类型解析文档文本
    pub fn parse_document(file_path: &Path, file_type: &str) -> Result<DocumentParseResult, String> {
        let file_name = file_path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        let ext = file_type.to_lowercase();

        let mut result = match ext.as_str() {
            // v0.2.4: PDF/DOCX 优先使用 MarkItDown（更稳定），原生 Rust 实现作为回退
            "pdf" => {
                let mut warnings = Vec::new();
                let text = if markitdown_skill::MarkitdownSkill::is_available() {
                    match markitdown_skill::MarkitdownSkill::convert(file_path) {
                        Ok(t) => {
                            warnings.push("PDF 通过 MarkItDown 转换".to_string());
                            t
                        }
                        Err(e) => {
                            warnings.push(format!("MarkItDown PDF 转换失败 ({}), 回退到内置解析", e));
                            let (t, w) = pdf_skill::PdfSkill::extract_text(file_path)?;
                            warnings.extend(w);
                            t
                        }
                    }
                } else {
                    let (t, w) = pdf_skill::PdfSkill::extract_text(file_path)?;
                    warnings.extend(w);
                    t
                };
                let page_count = pdf_skill::PdfSkill::get_page_count(file_path).ok();
                DocumentParseResult {
                    file_name,
                    file_type: "pdf".to_string(),
                    text,
                    text_length: 0,
                    page_count,
                    warnings,
                }
            }
            "docx" => {
                let mut warnings = Vec::new();
                let text = if markitdown_skill::MarkitdownSkill::is_available() {
                    match markitdown_skill::MarkitdownSkill::convert(file_path) {
                        Ok(t) => {
                            warnings.push("DOCX 通过 MarkItDown 转换".to_string());
                            t
                        }
                        Err(e) => {
                            warnings.push(format!("MarkItDown DOCX 转换失败 ({}), 回退到内置解析", e));
                            docx_skill::DocxSkill::extract_text(file_path)?
                        }
                    }
                } else {
                    docx_skill::DocxSkill::extract_text(file_path)?
                };
                DocumentParseResult {
                    file_name,
                    file_type: "docx".to_string(),
                    text,
                    text_length: 0,
                    page_count: None,
                    warnings,
                }
            }
            "pptx" => {
                let mut warnings = Vec::new();
                let text = if markitdown_skill::MarkitdownSkill::is_available() {
                    match markitdown_skill::MarkitdownSkill::convert(file_path) {
                        Ok(t) => {
                            warnings.push("PPTX 通过 MarkItDown 转换".to_string());
                            t
                        }
                        Err(e) => {
                            warnings.push(format!("MarkItDown PPTX 转换失败 ({}), 回退到内置解析", e));
                            pptx_skill::PptxSkill::extract_text(file_path)?
                        }
                    }
                } else {
                    pptx_skill::PptxSkill::extract_text(file_path)?
                };
                DocumentParseResult {
                    file_name,
                    file_type: "pptx".to_string(),
                    text,
                    text_length: 0,
                    page_count: None,
                    warnings,
                }
            }
            "xlsx" | "xls" => {
                if !markitdown_skill::MarkitdownSkill::is_available() {
                    return Err("Excel 文件需要安装 MarkItDown 才能解析: pip install markitdown".to_string());
                }
                let text = markitdown_skill::MarkitdownSkill::convert(file_path)?;
                DocumentParseResult {
                    file_name,
                    file_type: "xlsx".to_string(),
                    text,
                    text_length: 0,
                    page_count: None,
                    warnings: vec!["Excel 通过 MarkItDown 转换为 Markdown 表格".to_string()],
                }
            }
            "csv" => {
                if !markitdown_skill::MarkitdownSkill::is_available() {
                    return Err("CSV 文件需要安装 MarkItDown 才能解析: pip install markitdown".to_string());
                }
                let text = markitdown_skill::MarkitdownSkill::convert(file_path)?;
                DocumentParseResult {
                    file_name,
                    file_type: "csv".to_string(),
                    text,
                    text_length: 0,
                    page_count: None,
                    warnings: vec!["CSV 通过 MarkItDown 转换为 Markdown 表格".to_string()],
                }
            }
            "json" => {
                if !markitdown_skill::MarkitdownSkill::is_available() {
                    return Err("JSON 文件需要安装 MarkItDown 才能解析: pip install markitdown".to_string());
                }
                let text = markitdown_skill::MarkitdownSkill::convert(file_path)?;
                DocumentParseResult {
                    file_name,
                    file_type: "json".to_string(),
                    text,
                    text_length: 0,
                    page_count: None,
                    warnings: vec!["JSON 通过 MarkItDown 转换".to_string()],
                }
            }
            "xml" => {
                if !markitdown_skill::MarkitdownSkill::is_available() {
                    return Err("XML 文件需要安装 MarkItDown 才能解析: pip install markitdown".to_string());
                }
                let text = markitdown_skill::MarkitdownSkill::convert(file_path)?;
                DocumentParseResult {
                    file_name,
                    file_type: "xml".to_string(),
                    text,
                    text_length: 0,
                    page_count: None,
                    warnings: vec!["XML 通过 MarkItDown 转换".to_string()],
                }
            }
            // 纯文本类格式 — 原生 Rust 实现完全够用，不需要 MarkItDown
            "md" | "markdown" => {
                let text = md_skill::MdSkill::extract_text(file_path)?;
                DocumentParseResult {
                    file_name,
                    file_type: "md".to_string(),
                    text,
                    text_length: 0,
                    page_count: None,
                    warnings: Vec::new(),
                }
            }
            "txt" => {
                let text = txt_skill::TxtSkill::extract_text(file_path)?;
                DocumentParseResult {
                    file_name,
                    file_type: "txt".to_string(),
                    text,
                    text_length: 0,
                    page_count: None,
                    warnings: Vec::new(),
                }
            }
            "html" | "htm" => {
                let text = html_skill::HtmlSkill::extract_text(file_path)?;
                DocumentParseResult {
                    file_name,
                    file_type: "html".to_string(),
                    text,
                    text_length: 0,
                    page_count: None,
                    warnings: Vec::new(),
                }
            }
            _ => {
                // 最后尝试 markitdown（覆盖其他未知格式）
                if markitdown_skill::MarkitdownSkill::is_available() {
                    match markitdown_skill::MarkitdownSkill::convert(file_path) {
                        Ok(text) => DocumentParseResult {
                            file_name,
                            file_type: ext.clone(),
                            text,
                            text_length: 0,
                            page_count: None,
                            warnings: vec![format!("通过 MarkItDown 转换 (.{})", ext)],
                        },
                        Err(e) => return Err(format!("不支持的文件类型 .{} 且 MarkItDown 也无法处理: {}", ext, e)),
                    }
                } else {
                    return Err(format!("不支持的文件类型: .{}（安装 MarkItDown 可扩展支持更多格式: pip install markitdown）", ext));
                }
            }
        };

        result.text_length = result.text.len();

        // 如果文本过长，添加警告
        if result.text_length > 100_000 {
            result.warnings.push(format!(
                "文档文本较长（{} 字符），可能超出模型上下文限制，建议分段处理。",
                result.text_length
            ));
        }

        if result.text_length == 0 {
            return Err("提取的文本内容为空。可能该文件不包含可提取的文本。".to_string());
        }

        Ok(result)
    }

    /// 检查文件大小是否超限
    pub fn check_file_size(file_path: &Path, max_size_mb: u64) -> Result<u64, String> {
        let metadata = std::fs::metadata(file_path)
            .map_err(|e| format!("无法获取文件信息: {}", e))?;
        let size = metadata.len();
        let max_bytes = max_size_mb * 1024 * 1024;

        if size > max_bytes {
            return Err(format!(
                "文件大小 {} MB 超过限制 {} MB",
                size / (1024 * 1024),
                max_size_mb
            ));
        }
        Ok(size)
    }

    /// 计算文件 SHA256 hash
    pub fn compute_file_hash(file_path: &Path) -> Result<String, String> {
        use sha2::{Digest, Sha256};
        let mut file = std::fs::File::open(file_path)
            .map_err(|e| format!("无法打开文件: {}", e))?;
        let mut hasher = Sha256::new();
        std::io::copy(&mut file, &mut hasher)
            .map_err(|e| format!("计算 hash 失败: {}", e))?;
        let hash = hasher.finalize();
        Ok(hex::encode(hash))
    }

    /// 获取文件扩展名
    pub fn get_extension(file_name: &str) -> String {
        Path::new(file_name)
            .extension()
            .unwrap_or_default()
            .to_string_lossy()
            .to_lowercase()
    }
}
