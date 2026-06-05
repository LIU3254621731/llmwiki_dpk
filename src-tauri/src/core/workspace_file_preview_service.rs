use std::path::Path;
use crate::wiki::path_service::PathService;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FilePreviewResponse {
    pub relative_path: String,
    pub file_name: String,
    pub extension: String,
    pub size: u64,
    pub hash: String,
    pub modified_at: String,
    pub preview_type: String, // markdown | json | text | image | binary | unsupported | source_redirect
    pub content: String,
    pub render_hint: RenderHint,
    pub source_id: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RenderHint {
    pub can_render_markdown: bool,
    pub can_show_source: bool,
    pub can_format_json: bool,
    pub is_large_file: bool,
    pub truncated: bool,
    pub truncated_length: usize,
}

const MAX_PREVIEW_SIZE: u64 = 2 * 1024 * 1024; // 2MB

pub struct WorkspaceFilePreviewService;

impl WorkspaceFilePreviewService {
    /// 获取 workspace 文件的预览
    pub fn get_preview(
        kb_path: &Path,
        relative_path: &str,
    ) -> Result<FilePreviewResponse, String> {
        // 安全路径校验
        let resolved = PathService::resolve_workspace_path(kb_path, relative_path);
        
        // 防止路径穿越
        let canonical = resolved.canonicalize()
            .map_err(|_| format!("无法解析路径: {}", relative_path))?;
        let kb_canonical = kb_path.canonicalize()
            .map_err(|_| "无法解析知识库路径".to_string())?;
        if !canonical.starts_with(&kb_canonical) {
            return Err("安全校验失败：路径不在工作区内".to_string());
        }

        if !canonical.exists() {
            return Err(format!("文件不存在: {}", relative_path));
        }
        if !canonical.is_file() {
            return Err(format!("路径不是文件: {}", relative_path));
        }

        let metadata = std::fs::metadata(&canonical)
            .map_err(|e| format!("读取文件元数据失败: {}", e))?;
        let size = metadata.len();
        let modified_at = metadata.modified().ok()
            .map(|t| {
                let dt: chrono::DateTime<chrono::Utc> = t.into();
                dt.to_rfc3339()
            })
            .unwrap_or_default();

        let file_name = canonical.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();
        let extension = canonical.extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        let hash = if size < 100 * 1024 * 1024 {
            crate::skills::document_processor::DocumentProcessor::compute_file_hash(&canonical).unwrap_or_default()
        } else {
            String::new()
        };

        // 确定预览类型
        let (preview_type, content, render_hint, source_id, error) = match extension.as_str() {
            "md" | "markdown" => {
                let (content, truncated, truncated_len) = Self::read_text_safe(&canonical, size);
                let hint = RenderHint {
                    can_render_markdown: true,
                    can_show_source: true,
                    can_format_json: false,
                    is_large_file: size > MAX_PREVIEW_SIZE,
                    truncated,
                    truncated_length: truncated_len,
                };
                ("markdown".to_string(), content, hint, None, None)
            }
            "json" => {
                let (content, truncated, truncated_len) = Self::read_text_safe(&canonical, size);
                let is_valid_json = serde_json::from_str::<serde_json::Value>(&content).is_ok();
                let hint = RenderHint {
                    can_render_markdown: false,
                    can_show_source: true,
                    can_format_json: is_valid_json,
                    is_large_file: size > MAX_PREVIEW_SIZE,
                    truncated,
                    truncated_length: truncated_len,
                };
                let error = if !is_valid_json && !content.is_empty() {
                    Some("JSON 格式无效".to_string())
                } else {
                    None
                };
                ("json".to_string(), content, hint, None, error)
            }
            "txt" | "log" | "yaml" | "yml" | "toml" | "rs" | "ts" | "tsx" | "js" | "css" | "html" => {
                let (content, truncated, truncated_len) = Self::read_text_safe(&canonical, size);
                let hint = RenderHint {
                    can_render_markdown: false,
                    can_show_source: true,
                    can_format_json: false,
                    is_large_file: size > MAX_PREVIEW_SIZE,
                    truncated,
                    truncated_length: truncated_len,
                };
                ("text".to_string(), content, hint, None, None)
            }
            "png" | "jpg" | "jpeg" | "webp" | "gif" | "svg" | "ico" | "bmp" => {
                let hint = RenderHint {
                    can_render_markdown: false,
                    can_show_source: false,
                    can_format_json: false,
                    is_large_file: size > 10 * 1024 * 1024,
                    truncated: false,
                    truncated_length: 0,
                };
                ("image".to_string(), String::new(), hint, None, None)
            }
            "docx" | "pdf" | "pptx" | "xlsx" | "xls" => {
                // 通过 DocumentProcessor 提取文本内容生成预览
                let parse_result = crate::skills::document_processor::DocumentProcessor::parse_document(&canonical, &extension);
                match parse_result {
                    Ok(result) => {
                        let mut md = format!("# {}\n\n", file_name);
                        md.push_str(&format!("> 此预览由 {} 文件自动转换生成\n\n", extension.to_uppercase()));
                        for w in &result.warnings {
                            md.push_str(&format!("> ⚠️ {}\n", w));
                        }
                        if !result.warnings.is_empty() {
                            md.push('\n');
                        }
                        let preview_len = result.text.len().min(8000);
                        md.push_str(&result.text[..preview_len]);
                        if result.text.len() > 8000 {
                            md.push_str(&format!("\n\n*... 文本过长，已截断（完整长度 {} 字符）*", result.text.len()));
                        }
                        let hint = RenderHint {
                            can_render_markdown: true,
                            can_show_source: false,
                            can_format_json: false,
                            is_large_file: result.text.len() > 8000,
                            truncated: result.text.len() > 8000,
                            truncated_length: if result.text.len() > 8000 { 8000 } else { 0 },
                        };
                        ("markdown".to_string(), md, hint, None, None)
                    }
                    Err(e) => {
                        let hint = RenderHint {
                            can_render_markdown: false,
                            can_show_source: false,
                            can_format_json: false,
                            is_large_file: false,
                            truncated: false,
                            truncated_length: 0,
                        };
                        ("unsupported".to_string(), String::new(), hint, None, Some(format!("文件解析失败: {}", e)))
                    }
                }
            }
            _ => {
                let hint = RenderHint {
                    can_render_markdown: false,
                    can_show_source: false,
                    can_format_json: false,
                    is_large_file: false,
                    truncated: false,
                    truncated_length: 0,
                };
                ("unsupported".to_string(), String::new(), hint, None, Some("该文件类型暂不支持内置预览".to_string()))
            }
        };

        Ok(FilePreviewResponse {
            relative_path: relative_path.to_string(),
            file_name,
            extension,
            size,
            hash,
            modified_at,
            preview_type,
            content,
            render_hint,
            source_id,
            error,
        })
    }

    /// 预览本地任意绝对路径文件（不依赖 KB）
    pub fn preview_absolute_path(absolute_path: &Path) -> Result<FilePreviewResponse, String> {
        let canonical = absolute_path.canonicalize()
            .map_err(|_| format!("无法解析路径: {}", absolute_path.display()))?;

        if !canonical.is_file() {
            return Err(format!("路径不是文件: {}", absolute_path.display()));
        }

        let metadata = std::fs::metadata(&canonical)
            .map_err(|e| format!("读取文件元数据失败: {}", e))?;
        let size = metadata.len();
        let modified_at = metadata.modified().ok()
            .map(|t| {
                let dt: chrono::DateTime<chrono::Utc> = t.into();
                dt.to_rfc3339()
            })
            .unwrap_or_default();

        let file_name = canonical.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();
        let extension = canonical.extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        let hash = if size < 100 * 1024 * 1024 {
            crate::skills::document_processor::DocumentProcessor::compute_file_hash(&canonical).unwrap_or_default()
        } else {
            String::new()
        };

        // 确定预览类型 (复用与 get_preview 相同的逻辑)
        let (preview_type, content, render_hint, error) = match extension.as_str() {
            "md" | "markdown" => {
                let (content, truncated, truncated_len) = Self::read_text_safe(&canonical, size);
                let hint = RenderHint {
                    can_render_markdown: true,
                    can_show_source: true,
                    can_format_json: false,
                    is_large_file: size > MAX_PREVIEW_SIZE,
                    truncated,
                    truncated_length: truncated_len,
                };
                ("markdown".to_string(), content, hint, None)
            }
            "json" => {
                let (content, truncated, truncated_len) = Self::read_text_safe(&canonical, size);
                let is_valid_json = serde_json::from_str::<serde_json::Value>(&content).is_ok();
                let hint = RenderHint {
                    can_render_markdown: false,
                    can_show_source: true,
                    can_format_json: is_valid_json,
                    is_large_file: size > MAX_PREVIEW_SIZE,
                    truncated,
                    truncated_length: truncated_len,
                };
                let error = if !is_valid_json && !content.is_empty() {
                    Some("JSON 格式无效".to_string())
                } else {
                    None
                };
                ("json".to_string(), content, hint, error)
            }
            "txt" | "log" | "yaml" | "yml" | "toml" | "rs" | "ts" | "tsx" | "js" | "css" | "html" | "htm" | "xml" | "csv" => {
                let (content, truncated, truncated_len) = Self::read_text_safe(&canonical, size);
                let hint = RenderHint {
                    can_render_markdown: false,
                    can_show_source: true,
                    can_format_json: false,
                    is_large_file: size > MAX_PREVIEW_SIZE,
                    truncated,
                    truncated_length: truncated_len,
                };
                ("text".to_string(), content, hint, None)
            }
            "png" | "jpg" | "jpeg" | "webp" | "gif" | "svg" | "ico" | "bmp" => {
                let hint = RenderHint {
                    can_render_markdown: false,
                    can_show_source: false,
                    can_format_json: false,
                    is_large_file: size > 10 * 1024 * 1024,
                    truncated: false,
                    truncated_length: 0,
                };
                ("image".to_string(), String::new(), hint, None)
            }
            "docx" | "pdf" | "pptx" | "xlsx" | "xls" => {
                let parse_result = crate::skills::document_processor::DocumentProcessor::parse_document(&canonical, &extension);
                match parse_result {
                    Ok(result) => {
                        let mut md = format!("# {}\n\n", file_name);
                        md.push_str(&format!("> 此预览由 {} 文件自动转换生成\n\n", extension.to_uppercase()));
                        for w in &result.warnings {
                            md.push_str(&format!("> ⚠️ {}\n", w));
                        }
                        if !result.warnings.is_empty() {
                            md.push('\n');
                        }
                        let preview_len = result.text.len().min(8000);
                        md.push_str(&result.text[..preview_len]);
                        if result.text.len() > 8000 {
                            md.push_str(&format!("\n\n*... 文本过长，已截断（完整长度 {} 字符）*", result.text.len()));
                        }
                        let hint = RenderHint {
                            can_render_markdown: true,
                            can_show_source: false,
                            can_format_json: false,
                            is_large_file: result.text.len() > 8000,
                            truncated: result.text.len() > 8000,
                            truncated_length: if result.text.len() > 8000 { 8000 } else { 0 },
                        };
                        ("markdown".to_string(), md, hint, None)
                    }
                    Err(e) => {
                        let hint = RenderHint {
                            can_render_markdown: false,
                            can_show_source: false,
                            can_format_json: false,
                            is_large_file: false,
                            truncated: false,
                            truncated_length: 0,
                        };
                        ("unsupported".to_string(), String::new(), hint, Some(format!("文件解析失败: {}", e)))
                    }
                }
            }
            _ => {
                let hint = RenderHint {
                    can_render_markdown: false,
                    can_show_source: false,
                    can_format_json: false,
                    is_large_file: false,
                    truncated: false,
                    truncated_length: 0,
                };
                ("unsupported".to_string(), String::new(), hint, Some("该文件类型暂不支持内置预览".to_string()))
            }
        };

        Ok(FilePreviewResponse {
            relative_path: absolute_path.to_string_lossy().to_string(),
            file_name,
            extension,
            size,
            hash,
            modified_at,
            preview_type,
            content,
            render_hint,
            source_id: None,
            error,
        })
    }

    fn read_text_safe(path: &Path, size: u64) -> (String, bool, usize) {
        if size > MAX_PREVIEW_SIZE {
            // 大文件：只读前 256KB
            let mut buffer = vec![0u8; 256 * 1024];
            if let Ok(mut file) = std::fs::File::open(path) {
                use std::io::Read;
                if let Ok(n) = file.read(&mut buffer) {
                    buffer.truncate(n);
                    let content = String::from_utf8_lossy(&buffer).to_string();
                    return (content, true, n);
                }
            }
            return (format!("文件过大无法直接预览（{} MB）", size / 1024 / 1024), true, 0);
        }

        match std::fs::read_to_string(path) {
            Ok(content) => (content, false, 0),
            Err(_) => {
                // 尝试用二进制方式读取
                match std::fs::read(path) {
                    Ok(bytes) => {
                        let content = String::from_utf8_lossy(&bytes).to_string();
                        (content, false, 0)
                    }
                    Err(e) => (format!("读取文件失败: {}", e), false, 0),
                }
            }
        }
    }

}
