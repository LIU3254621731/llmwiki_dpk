use std::fs;
use std::path::Path;
use std::sync::Arc;
use crate::core::database_service::DatabaseService;
use chrono::Utc;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FileTreeNode {
    pub name: String,
    pub relative_path: String,
    pub file_type: String,
    pub file_size: u64,
    pub modified_at: String,
    pub is_directory: bool,
    pub children: Vec<FileTreeNode>,
    pub record_type: String,
    pub linked_record_id: String,
    pub status: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FileScanResult {
    pub root: FileTreeNode,
    pub total_files: usize,
    pub total_dirs: usize,
    pub warnings: Vec<String>,
}

pub struct FileTreeService;

impl FileTreeService {
    /// 扫描 workspace 目录生成文件树
    pub fn scan_workspace(
        root_path: &Path,
        db: Option<&Arc<DatabaseService>>,
        kb_id: &str,
        update_index: bool,
    ) -> Result<FileScanResult, String> {
        let mut warnings = Vec::new();
        let mut total_files = 0usize;
        let mut total_dirs = 0usize;

        let root_node = Self::scan_dir(root_path, root_path, db, kb_id, &mut total_files, &mut total_dirs, &mut warnings)?;

        if update_index {
            if let Some(db) = db {
                if let Ok(conn) = db.connect() {
                    if let Err(e) = conn.execute("DELETE FROM file_index WHERE kb_id = ?1", rusqlite::params![kb_id]) {
                        log::error!("[FileTreeService] 清除文件索引失败(kb={}): {}", kb_id, e);
                    }
                }
            }
        }

        Ok(FileScanResult {
            root: root_node,
            total_files,
            total_dirs,
            warnings,
        })
    }

    fn scan_dir(
        root_path: &Path,
        current_path: &Path,
        db: Option<&Arc<DatabaseService>>,
        kb_id: &str,
        total_files: &mut usize,
        total_dirs: &mut usize,
        warnings: &mut Vec<String>,
    ) -> Result<FileTreeNode, String> {
        let name = current_path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        let rel = current_path
            .strip_prefix(root_path)
            .unwrap_or(current_path)
            .to_string_lossy()
            .to_string();
        let relative_path = if rel.is_empty() { ".".to_string() } else { rel };

        let metadata = match fs::metadata(current_path) {
            Ok(m) => m,
            Err(e) => {
                warnings.push(format!("无法读取元数据 {}: {}", relative_path, e));
                return Ok(FileTreeNode {
                    name,
                    relative_path,
                    file_type: "".to_string(),
                    file_size: 0,
                    modified_at: "".to_string(),
                    is_directory: false,
                    children: vec![],
                    record_type: "error".to_string(),
                    linked_record_id: "".to_string(),
                    status: "error".to_string(),
                });
            }
        };

        if metadata.is_dir() {
            *total_dirs += 1;
            let mut children = Vec::new();
            let mut entries: Vec<_> = match fs::read_dir(current_path) {
                Ok(e) => e.filter_map(|e| e.ok()).collect(),
                Err(_) => Vec::new(),
            };

            entries.sort_by(|a, b| {
                let a_name = a.file_name();
                let b_name = b.file_name();
                let a_is_dir = a.file_type().map(|t| t.is_dir()).unwrap_or(false);
                let b_is_dir = b.file_type().map(|t| t.is_dir()).unwrap_or(false);
                b_is_dir.cmp(&a_is_dir).then(a_name.cmp(&b_name))
            });

            for entry in &entries {
                let child = Self::scan_dir(root_path, &entry.path(), db, kb_id, total_files, total_dirs, warnings)?;
                children.push(child);
            }

            Ok(FileTreeNode {
                name,
                relative_path,
                file_type: "".to_string(),
                file_size: 0,
                modified_at: "".to_string(),
                is_directory: true,
                children,
                record_type: "directory".to_string(),
                linked_record_id: "".to_string(),
                status: "ok".to_string(),
            })
        } else {
            *total_files += 1;
            let file_size = metadata.len();
            let ext = Path::new(&name)
                .extension()
                .unwrap_or_default()
                .to_string_lossy()
                .to_lowercase();
            let modified_at = metadata
                .modified()
                .ok()
                .map(|t| {
                    let dt: chrono::DateTime<Utc> = t.into();
                    dt.to_rfc3339()
                })
                .unwrap_or_default();

            let (record_type, linked_record_id, status) = Self::classify_file(db, kb_id, &relative_path, &ext, &name);

            // 保存到 file_index
            if let Some(db) = db {
                if let Ok(conn) = db.connect() {
                    let content_hash = if metadata.len() < 10 * 1024 * 1024 {
                        match crate::skills::document_processor::DocumentProcessor::compute_file_hash(current_path) {
                            Ok(h) => h,
                            Err(e) => {
                                warnings.push(format!("计算文件 hash 失败 {}: {}", relative_path, e));
                                "".to_string()
                            }
                        }
                    } else {
                        "".to_string()
                    };

                    let id = uuid::Uuid::new_v4().to_string();
                    if let Err(e) = conn.execute(
                        "INSERT OR REPLACE INTO file_index (id, kb_id, relative_path, file_name, file_type, file_size, content_hash, modified_at, record_type, linked_record_id, status)
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                        rusqlite::params![id, kb_id, relative_path, name, ext, file_size as i64, content_hash, modified_at, record_type, linked_record_id, status],
                    ) {
                        log::error!("[FileTreeService] 写入文件索引失败(path={}): {}", relative_path, e);
                    }
                }
            }

            Ok(FileTreeNode {
                name,
                relative_path,
                file_type: ext,
                file_size,
                modified_at,
                is_directory: false,
                children: vec![],
                record_type: record_type.clone(),
                linked_record_id,
                status,
            })
        }
    }

    fn classify_file(
        db: Option<&Arc<DatabaseService>>,
        kb_id: &str,
        relative_path: &str,
        ext: &str,
        file_name: &str,
    ) -> (String, String, String) {
        // 分类文件类型
        if relative_path.starts_with("raw/sources/documents") || relative_path.starts_with("raw/sources/webclips") {
            if let Some(db) = db {
                if let Ok(conn) = db.connect() {
                    // Try matching by full file_path first, then by file_name as fallback
                    let result = conn.query_row(
                        "SELECT id, status FROM sources WHERE kb_id = ?1 AND file_path LIKE ?2 LIMIT 1",
                        rusqlite::params![kb_id, format!("%{}", file_name)],
                        |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
                    );
                    if let Ok((sid, st)) = result {
                        return ("source".to_string(), sid, st);
                    }

                    // Fallback: match by file_name only
                    let result = conn.query_row(
                        "SELECT id, status FROM sources WHERE kb_id = ?1 AND file_name = ?2 LIMIT 1",
                        rusqlite::params![kb_id, file_name],
                        |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
                    );
                    if let Ok((sid, st)) = result {
                        return ("source".to_string(), sid, st);
                    }
                }
            }
            return ("source_orphan".to_string(), "".to_string(), "warning".to_string());
        }

        if relative_path.starts_with("raw/assets/images") || relative_path.starts_with("raw/assets/attachments") {
            if let Some(db) = db {
                if let Ok(conn) = db.connect() {
                    let result = conn.query_row(
                        "SELECT id FROM assets WHERE kb_id = ?1 AND file_name = ?2 LIMIT 1",
                        rusqlite::params![kb_id, file_name],
                        |row| row.get::<_, String>(0),
                    );
                    if let Ok(aid) = result {
                        return ("asset".to_string(), aid, "ok".to_string());
                    }
                }
            }
            return ("asset_orphan".to_string(), "".to_string(), "warning".to_string());
        }

        if relative_path.starts_with("wiki/") && ext == "md" {
            if let Some(db) = db {
                if let Ok(conn) = db.connect() {
                    let result = conn.query_row(
                        "SELECT id, status FROM wiki_pages WHERE kb_id = ?1 AND path = ?2 LIMIT 1",
                        rusqlite::params![kb_id, relative_path],
                        |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
                    );
                    if let Ok((pid, st)) = result {
                        return ("wiki_page".to_string(), pid, st);
                    }
                }
            }
            return ("wiki_orphan".to_string(), "".to_string(), "warning".to_string());
        }

        if relative_path.starts_with("versions/") {
            return ("version".to_string(), "".to_string(), "ok".to_string());
        }

        if relative_path.starts_with(".runtime/tasks") {
            return ("runtime_task".to_string(), "".to_string(), "ok".to_string());
        }

        if relative_path.starts_with(".runtime/source_previews") {
            return ("source_preview".to_string(), "".to_string(), "ok".to_string());
        }

        if relative_path.starts_with(".runtime/logs") {
            return ("runtime_log".to_string(), "".to_string(), "ok".to_string());
        }

        if relative_path.starts_with("schema/") {
            return ("schema".to_string(), "".to_string(), "ok".to_string());
        }

        if relative_path.starts_with("config/") {
            return ("config".to_string(), "".to_string(), "ok".to_string());
        }

        if relative_path.starts_with("db/") {
            return ("database".to_string(), "".to_string(), "ok".to_string());
        }

        ("unknown".to_string(), "".to_string(), "ok".to_string())
    }

    /// 获取单个文件的详细信息
    pub fn get_file_detail(
        root_path: &Path,
        relative_path: &str,
        db: Option<&Arc<DatabaseService>>,
        kb_id: &str,
    ) -> Result<serde_json::Value, String> {
        let abs_path = root_path.join(relative_path);
        if !abs_path.exists() {
            return Err(format!("文件不存在: {}", relative_path));
        }

        let metadata = fs::metadata(&abs_path)
            .map_err(|e| format!("无法读取元数据: {}", e))?;

        let file_name = abs_path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        let ext = Path::new(&file_name)
            .extension()
            .unwrap_or_default()
            .to_string_lossy()
            .to_lowercase();
        let modified_at = metadata
            .modified()
            .ok()
            .map(|t| {
                let dt: chrono::DateTime<Utc> = t.into();
                dt.to_rfc3339()
            })
            .unwrap_or_default();
        let created_at = metadata
            .created()
            .ok()
            .map(|t| {
                let dt: chrono::DateTime<Utc> = t.into();
                dt.to_rfc3339()
            })
            .unwrap_or_default();

        let file_hash = if metadata.len() < 100 * 1024 * 1024 {
            match crate::skills::document_processor::DocumentProcessor::compute_file_hash(&abs_path) {
                Ok(hash) => Some(hash),
                Err(e) => {
                    log::error!("[file_tree_service] 计算文件哈希失败 ({}): {}", abs_path.display(), e);
                    None
                }
            }
        } else {
            None
        };

        let (record_type, linked_record_id, status) = Self::classify_file(db, kb_id, relative_path, &ext, &file_name);

        let mut linked_wiki_pages: Vec<serde_json::Value> = Vec::new();
        let mut linked_tasks: Vec<serde_json::Value> = Vec::new();
        let mut linked_graph_nodes: Vec<serde_json::Value> = Vec::new();

        if let Some(db) = db {
            if let Ok(conn) = db.connect() {
                if record_type == "source" || record_type == "source_orphan" {
                    // 查询关联的 wiki 页面
                    if let Ok(mut stmt) = conn.prepare(
                        "SELECT wp.id, wp.title, wp.path FROM wiki_pages wp
                         JOIN knowledge_items ki ON ki.page_path = wp.path
                         WHERE ki.source_id = ?1 AND wp.kb_id = ?2"
                    ) {
                        if let Ok(rows) = stmt.query_map(rusqlite::params![linked_record_id, kb_id], |row| {
                            Ok(serde_json::json!({
                                "id": row.get::<_, String>(0)?,
                                "title": row.get::<_, String>(1)?,
                                "path": row.get::<_, String>(2)?,
                            }))
                        }) {
                            linked_wiki_pages = rows.filter_map(|r| r.ok()).collect();
                        }
                    }

                    // 关联任务
                    if let Ok(mut stmt) = conn.prepare(
                        "SELECT t.id, t.task_type, t.status FROM tasks t WHERE t.kb_id = ?1 AND t.input_ref = ?2 ORDER BY t.created_at DESC LIMIT 5"
                    ) {
                        if let Ok(rows) = stmt.query_map(rusqlite::params![kb_id, linked_record_id], |row| {
                            Ok(serde_json::json!({
                                "id": row.get::<_, String>(0)?,
                                "task_type": row.get::<_, String>(1)?,
                                "status": row.get::<_, String>(2)?,
                            }))
                        }) {
                            linked_tasks = rows.filter_map(|r| r.ok()).collect();
                        }
                    }
                }

                // 关联图谱节点
                if !linked_record_id.is_empty() {
                    if let Ok(mut stmt) = conn.prepare(
                        "SELECT id, label, node_type FROM graph_nodes WHERE kb_id = ?1 AND (source_id = ?2 OR page_id = ?2) LIMIT 10"
                    ) {
                        if let Ok(rows) = stmt.query_map(rusqlite::params![kb_id, linked_record_id], |row| {
                            Ok(serde_json::json!({
                                "id": row.get::<_, String>(0)?,
                                "label": row.get::<_, String>(1)?,
                                "node_type": row.get::<_, String>(2)?,
                            }))
                        }) {
                            linked_graph_nodes = rows.filter_map(|r| r.ok()).collect();
                        }
                    }
                }
            }
        }

        Ok(serde_json::json!({
            "file_name": file_name,
            "relative_path": relative_path,
            "absolute_path": abs_path.to_string_lossy(),
            "file_type": ext,
            "file_size": metadata.len(),
            "file_hash": file_hash,
            "created_at": created_at,
            "modified_at": modified_at,
            "record_type": record_type,
            "linked_record_id": linked_record_id,
            "status": status,
            "linked_wiki_pages": linked_wiki_pages,
            "linked_tasks": linked_tasks,
            "linked_graph_nodes": linked_graph_nodes,
        }))
    }

    /// 获取文件列表（扁平列表，带过滤）
    pub fn list_files(
        root_path: &Path,
        db: Option<&Arc<DatabaseService>>,
        kb_id: &str,
        filter: &str,
        search: &str,
    ) -> Result<Vec<serde_json::Value>, String> {
        let scan = Self::scan_workspace(root_path, db, kb_id, false)?;
        let mut result = Vec::new();

        fn collect_files(node: &FileTreeNode, result: &mut Vec<serde_json::Value>, filter: &str, search: &str) {
            if !node.is_directory {
                let filter_match = filter.is_empty()
                    || filter == "all"
                    || match filter {
                        "sources" => node.record_type.starts_with("source"),
                        "wiki" => node.record_type.starts_with("wiki"),
                        "versions" => node.record_type == "version",
                        "tasks" => node.record_type == "runtime_task",
                        "assets" => node.record_type.starts_with("asset"),
                        "anomalies" => node.status == "warning" || node.status == "error",
                        _ => true,
                    };

                let search_match = search.is_empty()
                    || node.name.to_lowercase().contains(&search.to_lowercase());

                if filter_match && search_match {
                    result.push(serde_json::json!({
                        "name": node.name,
                        "relative_path": node.relative_path,
                        "file_type": node.file_type,
                        "file_size": node.file_size,
                        "modified_at": node.modified_at,
                        "record_type": node.record_type,
                        "linked_record_id": node.linked_record_id,
                        "status": node.status,
                    }));
                }
            }
            for child in &node.children {
                collect_files(child, result, filter, search);
            }
        }

        collect_files(&scan.root, &mut result, filter, search);
        Ok(result)
    }
}
