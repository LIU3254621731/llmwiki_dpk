use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use crate::core::database_service::DatabaseService;
use crate::skills::document_processor::DocumentProcessor;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SourcePreviewResult {
    pub source_id: String,
    pub preview_path: String,
    pub preview_status: String,
    pub content: String,
    pub file_type: String,
}

pub struct SourcePreviewService;

impl SourcePreviewService {
    /// 根据 source_id 生成 Markdown 预览
    pub fn generate_preview(
        db: &Arc<DatabaseService>,
        kb_path: &Path,
        source_id: &str,
    ) -> Result<SourcePreviewResult, String> {
        let conn = db.connect()?;

        // 获取 source 信息，如果不存在则尝试自动恢复
        let source_info: (String, String, String, String) = match conn.query_row(
            "SELECT file_name, file_path, file_type, kb_id FROM sources WHERE id = ?1",
            rusqlite::params![source_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        ) {
            Ok(info) => info,
            Err(rusqlite::Error::QueryReturnedNoRows) => {
                // Auto-recovery: try to find the file on disk and insert a metadata record
                let recovered = Self::auto_recover_source(&conn, kb_path, source_id)?;
                recovered
            }
            Err(e) => return Err(format!("获取 source 信息失败: {}", e)),
        };

        let (file_name, file_path, file_type, kb_id) = source_info;

        let src_path = PathBuf::from(&file_path);
        if !src_path.exists() {
            return Err(format!("原始文件不存在: {}", file_path));
        }

        let preview_dir = kb_path.join(".runtime/source_previews");
        fs::create_dir_all(&preview_dir)
            .map_err(|e| format!("创建预览目录失败: {}", e))?;

        let preview_file_name = format!("{}.md", source_id);
        let preview_path = preview_dir.join(&preview_file_name);

        let content = match file_type.as_str() {
            "md" | "markdown" => {
                fs::read_to_string(&src_path)
                    .map_err(|e| format!("读取 Markdown 文件失败: {}", e))?
            }
            "txt" => {
                let raw = fs::read_to_string(&src_path)
                    .map_err(|e| format!("读取 TXT 文件失败: {}", e))?;
                Self::txt_to_markdown(&raw, &file_name)
            }
            "html" | "htm" => {
                Self::html_to_markdown(&src_path)?
            }
            "docx" => {
                let parse_result = DocumentProcessor::parse_document(&src_path, "docx")?;
                let mut md = format!("# {}\n\n", file_name);
                md.push_str("> 此预览由 DOCX 文档转换生成，不保证完整版式还原\n\n");
                md.push_str(&Self::plain_text_to_markdown(&parse_result.text));
                md
            }
            "pdf" => {
                // PDF: 优先通过 MarkItDown 提取文本，同时展示 AI 摘要
                let ai_summary: String = conn
                    .query_row(
                        "SELECT COALESCE(ai_summary, '') FROM sources WHERE id = ?1",
                        rusqlite::params![source_id],
                        |row| row.get(0),
                    )
                    .unwrap_or_default();

                let coverage_report: String = conn
                    .query_row(
                        "SELECT COALESCE(coverage_report, '') FROM sources WHERE id = ?1",
                        rusqlite::params![source_id],
                        |row| row.get(0),
                    )
                    .unwrap_or_default();

                let parse_result = DocumentProcessor::parse_document(&src_path, "pdf");

                let mut md = format!("# {} (PDF)\n\n", file_name);
                md.push_str(&format!("- **文件大小**: {} 字节\n", fs::metadata(&src_path).map(|m| m.len()).unwrap_or(0)));

                match parse_result {
                    Ok(result) => {
                        if let Some(pc) = result.page_count {
                            md.push_str(&format!("- **页数**: {}\n", pc));
                        }
                        md.push_str(&format!("- **提取文本**: {} 字符\n\n", result.text_length));

                        for w in &result.warnings {
                            md.push_str(&format!("> ⚠️ {}\n", w));
                        }
                        if !result.warnings.is_empty() {
                            md.push('\n');
                        }

                        // 展示提取的文本（限制预览长度）
                        let preview_len = result.text.len().min(8000);
                        md.push_str("## 文档文本预览\n\n");
                        md.push_str(&result.text[..preview_len]);
                        if result.text.len() > 8000 {
                            md.push_str(&format!("\n\n*... 文本过长，已截断（完整长度 {} 字符）*", result.text.len()));
                        }
                        md.push('\n');
                    }
                    Err(ref e) => {
                        md.push_str(&format!("> ⚠️ 文本提取失败: {}\n\n", e));
                    }
                }

                if !ai_summary.is_empty() {
                    md.push_str("\n## AI 摘要\n\n");
                    md.push_str(&ai_summary);
                    md.push('\n');
                }

                if !coverage_report.is_empty() {
                    md.push_str("\n## 覆盖度报告\n\n");
                    md.push_str(&coverage_report);
                    md.push('\n');
                }

                if ai_summary.is_empty() {
                    md.push_str("\n*该 PDF 文件尚未经过 AI 分析。请通过导入任务进行分析以获取摘要。*\n");
                }

                md
            }
            "pptx" | "xlsx" | "xls" | "csv" | "json" | "xml" => {
                // v0.2.5: MarkItDown 支持的格式，通过 DocumentProcessor 提取文本生成预览
                let parse_result = match DocumentProcessor::parse_document(&src_path, &file_type) {
                    Ok(r) => r,
                    Err(e) => return Err(format!("{} 文件解析失败: {}", file_type.to_uppercase(), e)),
                };
                let mut md = format!("# {} ({})\n\n", file_name, file_type.to_uppercase());
                md.push_str(&format!("> 此预览由 {} 文件通过 MarkItDown 转换生成\n\n", file_type.to_uppercase()));
                md.push_str(&Self::plain_text_to_markdown(&parse_result.text));
                md
            }
            ext if DocumentProcessor::is_asset_type(ext) => {
                // 图片等资产文件
                let asset_info = serde_json::json!({
                    "file_name": file_name,
                    "file_type": file_type,
                    "is_asset": true,
                    "asset_only": true,
                });
                format!("# {} (资产文件)\n\n> 此文件为资产文件（{}），不进行 OCR、不进行图片理解、不参与 AI 知识抽取\n\n{}\n",
                    file_name, file_type,
                    serde_json::to_string_pretty(&asset_info).unwrap_or_default())
            }
            _ => {
                return Err(format!("不支持的文件类型: {}", file_type));
            }
        };

        // 保存预览到 .runtime/source_previews/
        fs::write(&preview_path, &content)
            .map_err(|e| format!("保存预览文件失败: {}", e))?;

        let content_hash = match DocumentProcessor::compute_file_hash(&preview_path) {
            Ok(h) => h,
            Err(e) => {
                log::error!("[SourcePreviewService] 计算预览文件 hash 失败 ({}): {}", preview_path.display(), e);
                String::new()
            }
        };
        let now = chrono::Utc::now().to_rfc3339();

        // 更新 sources 表
        conn.execute(
            "UPDATE sources SET preview_path = ?1, preview_status = ?2, preview_generated_at = ?3, preview_error = '' WHERE id = ?4",
            rusqlite::params![preview_path.to_string_lossy(), "generated", now, source_id],
        )
        .map_err(|e| format!("更新 preview_path 失败: {}", e))?;

        // 更新 source_previews 表
        let preview_id = uuid::Uuid::new_v4().to_string();
        conn.execute(
            "INSERT OR REPLACE INTO source_previews (id, source_id, kb_id, preview_path, preview_status, content_hash, generated_at, error_message)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, '')",
            rusqlite::params![preview_id, source_id, kb_id, preview_path.to_string_lossy(), "generated", content_hash, now],
        )
        .map_err(|e| format!("保存 source_previews 记录失败: {}", e))?;

        Ok(SourcePreviewResult {
            source_id: source_id.to_string(),
            preview_path: preview_path.to_string_lossy().to_string(),
            preview_status: "generated".to_string(),
            content,
            file_type,
        })
    }

    /// Auto-recover: when a source_id is not found in the sources table,
    /// scan the workspace for orphaned files (exist on disk but not in DB)
    /// and insert metadata records. Returns the source info for the recovered file.
    fn auto_recover_source(
        conn: &rusqlite::Connection,
        kb_path: &Path,
        source_id: &str,
    ) -> Result<(String, String, String, String), String> {
        use std::collections::HashSet;
        use crate::skills::document_processor::DocumentProcessor;

        // Try file_index first: check if any file_index entry links to this source_id
        let from_file_index: Option<(String, String)> = match conn.query_row(
            "SELECT relative_path, file_name FROM file_index WHERE linked_record_id = ?1 LIMIT 1",
            rusqlite::params![source_id],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        ) {
            Ok(v) => Some(v),
            Err(rusqlite::Error::QueryReturnedNoRows) => None,
            Err(e) => {
                log::warn!("[auto_recover] file_index 查询失败: {}", e);
                None
            }
        };

        // If file_index has a record, reconstruct the source
        if let Some((relative_path, file_name)) = from_file_index {
            let abs_path = kb_path.join(&relative_path);
            if abs_path.exists() {
                let extension = DocumentProcessor::get_extension(&file_name);
                let file_size = std::fs::metadata(&abs_path).map(|m| m.len() as i64).unwrap_or(0);
                let file_hash = DocumentProcessor::compute_file_hash(&abs_path).unwrap_or_default();
                let kb_id = match conn.query_row(
                    "SELECT kb_id FROM file_index WHERE linked_record_id = ?1 LIMIT 1",
                    rusqlite::params![source_id],
                    |row| row.get::<_, String>(0),
                ) {
                    Ok(id) => id,
                    Err(_) => {
                        // Try to get kb_id from knowledge_bases via path prefix
                        String::new()
                    }
                };

                if !kb_id.is_empty() {
                    let now = chrono::Utc::now().to_rfc3339();
                    let new_source_id = uuid::Uuid::new_v4().to_string();
                    let abs_path_str = abs_path.to_string_lossy().to_string();

                    conn.execute(
                        "INSERT INTO sources (id, kb_id, file_name, file_path, file_type, file_size, file_hash, status, created_at, updated_at)
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'pending', ?8, ?8)",
                        rusqlite::params![new_source_id, kb_id, file_name, &abs_path_str, extension, file_size, file_hash, now],
                    ).map_err(|e| format!("auto_recover: 插入 sources 记录失败: {}", e))?;

                    log::info!("[auto_recover] 从 file_index 恢复 source: {} (原 source_id={}) -> {}", file_name, source_id, new_source_id);
                    return Ok((file_name, abs_path_str, extension, kb_id));
                }
            }
        }

        // Scan workspace documents directory for orphaned files
        let docs_dir = kb_path.join("raw").join("sources").join("documents");
        if !docs_dir.exists() {
            return Err("source 记录不存在，且工作区文档目录不存在，无法自动恢复".to_string());
        }

        // Collect existing file paths from sources table
        let mut existing_paths: HashSet<String> = HashSet::new();
        if let Ok(mut stmt) = conn.prepare("SELECT file_path FROM sources") {
            if let Ok(rows) = stmt.query_map([], |row| row.get::<_, String>(0)) {
                for r in rows.flatten() {
                    existing_paths.insert(r);
                }
            }
        }

        // Scan for orphaned files and auto-insert them
        let mut recovered: Option<(String, String, String, String)> = None;

        if let Ok(entries) = std::fs::read_dir(&docs_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if !path.is_file() {
                    continue;
                }

                let abs_path_str = path.to_string_lossy().to_string();
                if existing_paths.contains(&abs_path_str) {
                    continue;
                }

                let file_name = path.file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();

                let extension = DocumentProcessor::get_extension(&file_name);
                if !DocumentProcessor::is_supported(&extension) {
                    continue;
                }

                let file_size = std::fs::metadata(&path).map(|m| m.len() as i64).unwrap_or(0);
                let file_hash = DocumentProcessor::compute_file_hash(&path).unwrap_or_default();
                let kb_id = match conn.query_row(
                    "SELECT id FROM knowledge_bases LIMIT 1",
                    [],
                    |row| row.get::<_, String>(0),
                ) {
                    Ok(id) => id,
                    Err(_) => continue,
                };

                let now = chrono::Utc::now().to_rfc3339();
                let new_source_id = uuid::Uuid::new_v4().to_string();

                if let Err(e) = conn.execute(
                    "INSERT INTO sources (id, kb_id, file_name, file_path, file_type, file_size, file_hash, status, created_at, updated_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'pending', ?8, ?8)",
                    rusqlite::params![new_source_id, kb_id, file_name, abs_path_str, extension, file_size, file_hash, now],
                ) {
                    log::warn!("[auto_recover] 插入孤儿 source 记录失败 ({}): {}", file_name, e);
                    continue;
                }

                log::info!("[auto_recover] 从磁盘恢复孤儿文件: {} -> source_id={}", file_name, new_source_id);

                // Use the first recovered file as the result
                if recovered.is_none() {
                    recovered = Some((file_name, path.to_string_lossy().to_string(), extension, kb_id));
                }
            }
        }

        match recovered {
            Some(info) => Ok(info),
            None => Err("source 记录不存在，且未在磁盘上找到可恢复的文档文件".to_string()),
        }
    }

    /// 获取已有的预览内容
    pub fn get_preview(
        db: &Arc<DatabaseService>,
        kb_path: &Path,
        source_id: &str,
    ) -> Result<SourcePreviewResult, String> {
        let conn = db.connect()?;

        // 先尝试从数据库获取
        let result: Result<(String, String, String), _> = conn.query_row(
            "SELECT preview_path, preview_status, file_type FROM sources WHERE id = ?1",
            rusqlite::params![source_id],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?)),
        );

        match result {
            Ok((preview_path, preview_status, file_type)) if !preview_path.is_empty() => {
                let preview_file = PathBuf::from(&preview_path);
                if preview_file.exists() {
                    let content = fs::read_to_string(&preview_file)
                        .map_err(|e| format!("读取预览文件失败: {}", e))?;
                    return Ok(SourcePreviewResult {
                        source_id: source_id.to_string(),
                        preview_path,
                        preview_status,
                        content,
                        file_type,
                    });
                }
            }
            _ => {}
        }

        // 没有缓存则重新生成
        Self::generate_preview(db, kb_path, source_id)
    }

    /// 将纯文本转换为 Markdown
    fn txt_to_markdown(raw: &str, file_name: &str) -> String {
        if raw.is_empty() {
            return format!("# {}\n\n*文件内容为空*\n", file_name);
        }

        let mut md = format!("# {}\n\n", file_name);
        let lines: Vec<&str> = raw.lines().collect();

        if lines.len() <= 100 {
            // 小文件直接包裹为代码块风格的段落
            for line in &lines {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    md.push('\n');
                } else if trimmed.len() > 80 && trimmed.chars().all(|c| c.is_ascii_alphanumeric() || c.is_ascii_whitespace()) {
                    md.push_str(&format!("{}\n\n", trimmed));
                } else {
                    md.push_str(&format!("{}\n", trimmed));
                }
            }
        } else {
            // 大文件用代码块包裹
            md.push_str("```\n");
            md.push_str(raw);
            md.push_str("\n```\n");
        }
        md
    }

    /// HTML 转 Markdown
    fn html_to_markdown(file_path: &Path) -> Result<String, String> {
        let parse_result = DocumentProcessor::parse_document(file_path, "html")?;
        let file_name = file_path.file_name().unwrap_or_default().to_string_lossy().to_string();
        let mut md = format!("# {}\n\n", file_name);
        md.push_str("> 此预览由 HTML 文档转换生成\n\n");
        md.push_str(&Self::plain_text_to_markdown(&parse_result.text));
        Ok(md)
    }

    /// 将纯文本转换为基本 Markdown（保留段落结构）
    fn plain_text_to_markdown(text: &str) -> String {
        let mut md = String::new();
    let lines: Vec<&str> = text.lines().collect();
    let mut prev_empty = false;

        for line in &lines {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                if !prev_empty {
                    md.push('\n');
                }
                prev_empty = true;
                continue;
            }
            prev_empty = false;

            // 检测可能的标题（短行、以大写字母或数字开头）
            if trimmed.len() <= 100 && !trimmed.contains('.') && !trimmed.starts_with('-') && !trimmed.starts_with('*')
                && trimmed.len() <= 60 {
                    md.push_str(&format!("## {}\n\n", trimmed));
                    continue;
                }

            md.push_str(&format!("{}\n\n", trimmed));
        }

        md
    }

    /// 重建所有 source 的预览
    pub fn rebuild_all_previews(
        db: &Arc<DatabaseService>,
        kb_path: &Path,
        kb_id: &str,
    ) -> Result<serde_json::Value, String> {
        let conn = db.connect()?;
        let mut stmt = conn
            .prepare("SELECT id, file_type FROM sources WHERE kb_id = ?1 AND status != 'asset_only'")
            .map_err(|e| format!("查询 sources 失败: {}", e))?;

        let sources: Vec<(String, String)> = stmt
            .query_map(rusqlite::params![kb_id], |row| {
                Ok((row.get(0)?, row.get(1)?))
            })
            .map_err(|e| format!("映射 sources 失败: {}", e))?
            .filter_map(|r| r.ok())
            .collect();

        let mut success = 0i64;
        let mut failed = 0i64;

        for (sid, _ftype) in &sources {
            match Self::generate_preview(db, kb_path, sid) {
                Ok(_) => success += 1,
                Err(e) => {
                    log::warn!("重建预览失败 {}: {}", sid, e);
                    failed += 1;
                    if let Err(e2) = conn.execute(
                        "UPDATE sources SET preview_status = 'failed', preview_error = ?1 WHERE id = ?2",
                        rusqlite::params![e, sid],
                    ) {
                        log::error!("[source_preview] 更新 preview_status 失败 (source={}): {}", sid, e2);
                    }
                }
            }
        }

        Ok(serde_json::json!({
            "total": sources.len(),
            "success": success,
            "failed": failed,
        }))
    }
}
