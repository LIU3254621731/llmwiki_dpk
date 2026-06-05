use std::path::PathBuf;
use std::sync::Arc;
use tauri::State;
use crate::core::app_kernel::AppKernel;
use crate::core::file_tree_service::FileTreeService;
use crate::core::workspace_file_preview_service::WorkspaceFilePreviewService;

#[tauri::command]
pub async fn scan_workspace_files(
    kernel: State<'_, Arc<AppKernel>>,
    kb_id: String,
    kb_path: String,
) -> Result<serde_json::Value, String> {
    let root_path = std::path::PathBuf::from(&kb_path);
    let result = FileTreeService::scan_workspace(
        &root_path,
        Some(&kernel.db),
        &kb_id,
        true, // update_index
    )?;
    Ok(serde_json::json!(result))
}

#[tauri::command]
pub async fn get_file_tree(
    kernel: State<'_, Arc<AppKernel>>,
    kb_id: String,
    kb_path: String,
) -> Result<serde_json::Value, String> {
    let root_path = std::path::PathBuf::from(&kb_path);
    let result = FileTreeService::scan_workspace(
        &root_path,
        Some(&kernel.db),
        &kb_id,
        false,
    )?;
    Ok(serde_json::json!(result))
}

#[tauri::command]
pub async fn get_file_detail(
    kernel: State<'_, Arc<AppKernel>>,
    kb_id: String,
    kb_path: String,
    relative_path: String,
) -> Result<serde_json::Value, String> {
    let root_path = std::path::PathBuf::from(&kb_path);
    FileTreeService::get_file_detail(&root_path, &relative_path, Some(&kernel.db), &kb_id)
}

#[tauri::command]
pub async fn list_files(
    kernel: State<'_, Arc<AppKernel>>,
    kb_id: String,
    kb_path: String,
    filter: String,
    search: String,
) -> Result<Vec<serde_json::Value>, String> {
    let root_path = std::path::PathBuf::from(&kb_path);
    FileTreeService::list_files(&root_path, Some(&kernel.db), &kb_id, &filter, &search)
}

#[tauri::command]
pub async fn get_workspace_file_preview(
    kernel: State<'_, Arc<AppKernel>>,
    kb_id: String,
    kb_path: String,
    relative_path: String,
) -> Result<serde_json::Value, String> {
    let root_path = std::path::PathBuf::from(&kb_path);
    let preview = WorkspaceFilePreviewService::get_preview(&root_path, &relative_path)?;

    // 查找 source_id
    let source_id = if preview.preview_type == "source_redirect" || relative_path.contains("raw/sources") {
        let file_name = std::path::Path::new(&relative_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");
        let conn = match kernel.db.connect() {
            Ok(c) => c,
            Err(e) => {
                log::error!("[file_tree] 连接数据库失败: {}", e);
                return Ok(serde_json::json!({
                    "relative_path": preview.relative_path,
                    "file_name": preview.file_name,
                    "extension": preview.extension,
                    "size": preview.size,
                    "hash": preview.hash,
                    "modified_at": preview.modified_at,
                    "preview_type": preview.preview_type,
                    "content": preview.content,
                    "render_hint": { "type": "error", "message": "数据库连接失败" },
                    "source_id": null,
                }));
            }
        };
        match conn.query_row(
            "SELECT id FROM sources WHERE kb_id = ?1 AND file_name = ?2 LIMIT 1",
            rusqlite::params![kb_id, file_name],
            |row| row.get::<_, String>(0),
        ) {
            Ok(id) => Some(id),
            Err(rusqlite::Error::QueryReturnedNoRows) => None,
            Err(e) => {
                log::error!("[file_tree] 查询 source_id 失败 (file={}): {}", file_name, e);
                None
            }
        }
    } else {
        None
    };

    Ok(serde_json::json!({
        "relative_path": preview.relative_path,
        "file_name": preview.file_name,
        "extension": preview.extension,
        "size": preview.size,
        "hash": preview.hash,
        "modified_at": preview.modified_at,
        "preview_type": preview.preview_type,
        "content": preview.content,
        "render_hint": {
            "can_render_markdown": preview.render_hint.can_render_markdown,
            "can_show_source": preview.render_hint.can_show_source,
            "can_format_json": preview.render_hint.can_format_json,
            "is_large_file": preview.render_hint.is_large_file,
            "truncated": preview.render_hint.truncated,
            "truncated_length": preview.render_hint.truncated_length,
        },
        "source_id": source_id.unwrap_or_default(),
        "error": preview.error,
    }))
}

#[tauri::command]
pub async fn save_workspace_file(
    kb_path: String,
    relative_path: String,
    content: String,
) -> Result<serde_json::Value, String> {
    let root = std::path::PathBuf::from(&kb_path);
    let full_path = root.join(&relative_path);

    if !full_path.starts_with(&root) {
        return Err("路径越界".to_string());
    }

    if let Some(parent) = full_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("创建目录失败: {}", e))?;
    }

    std::fs::write(&full_path, &content).map_err(|e| format!("保存文件失败: {}", e))?;

    Ok(serde_json::json!({"success": true}))
}

#[tauri::command]
pub fn create_workspace_file(
    kb_path: String,
    relative_path: String,
) -> Result<serde_json::Value, String> {
    let root = std::path::PathBuf::from(&kb_path);
    let target = root.join(&relative_path);

    // 防止路径遍历攻击
    if !target.starts_with(&root) {
        return Err("路径越界".to_string());
    }

    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("创建父目录失败: {}", e))?;
    }
    std::fs::write(&target, "").map_err(|e| format!("创建文件失败: {}", e))?;
    Ok(serde_json::json!({ "success": true }))
}

#[tauri::command]
pub fn create_workspace_folder(
    kb_path: String,
    relative_path: String,
) -> Result<serde_json::Value, String> {
    let root = std::path::PathBuf::from(&kb_path);
    let target = root.join(&relative_path);

    // 防止路径遍历攻击
    if !target.starts_with(&root) {
        return Err("路径越界".to_string());
    }

    std::fs::create_dir_all(&target)
        .map_err(|e| format!("创建文件夹失败: {}", e))?;
    Ok(serde_json::json!({ "success": true }))
}

#[tauri::command]
pub fn delete_workspace_file(
    kb_path: String,
    relative_path: String,
) -> Result<serde_json::Value, String> {
    let root = std::path::PathBuf::from(&kb_path);
    let target = root.join(&relative_path);

    // 防止路径遍历攻击
    if !target.starts_with(&root) {
        return Err("路径越界".to_string());
    }

    if !target.exists() {
        return Err(format!("路径不存在: {}", relative_path));
    }

    if target.is_dir() {
        std::fs::remove_dir_all(&target)
            .map_err(|e| format!("删除文件夹失败: {}", e))?;
    } else {
        std::fs::remove_file(&target)
            .map_err(|e| format!("删除文件失败: {}", e))?;
    }
    Ok(serde_json::json!({ "success": true }))
}

#[tauri::command]
pub fn rename_workspace_file(
    kb_path: String,
    old_relative_path: String,
    new_relative_path: String,
) -> Result<serde_json::Value, String> {
    let root = std::path::PathBuf::from(&kb_path);
    let old_target = root.join(&old_relative_path);
    let new_target = root.join(&new_relative_path);

    // 防止路径遍历攻击：两个路径都必须在工作区根目录内
    if !old_target.starts_with(&root) {
        return Err("路径越界: 源路径超出工作区范围".to_string());
    }
    if !new_target.starts_with(&root) {
        return Err("路径越界: 目标路径超出工作区范围".to_string());
    }

    if !old_target.exists() {
        return Err(format!("源路径不存在: {}", old_relative_path));
    }

    if let Some(parent) = new_target.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("创建目标父目录失败: {}", e))?;
    }

    std::fs::rename(&old_target, &new_target)
        .map_err(|e| format!("重命名失败: {}", e))?;
    Ok(serde_json::json!({ "success": true }))
}

/// 预览本地任意文件（不依赖 KB，传入绝对路径）
#[tauri::command]
pub async fn preview_local_file(
    _kernel: State<'_, Arc<AppKernel>>,
    file_path: String,
) -> Result<serde_json::Value, String> {
    let path = PathBuf::from(&file_path);
    let preview = WorkspaceFilePreviewService::preview_absolute_path(&path)?;
    Ok(serde_json::json!({
        "relative_path": preview.relative_path,
        "file_name": preview.file_name,
        "extension": preview.extension,
        "size": preview.size,
        "hash": preview.hash,
        "modified_at": preview.modified_at,
        "preview_type": preview.preview_type,
        "content": preview.content,
        "render_hint": {
            "can_render_markdown": preview.render_hint.can_render_markdown,
            "can_show_source": preview.render_hint.can_show_source,
            "can_format_json": preview.render_hint.can_format_json,
            "is_large_file": preview.render_hint.is_large_file,
            "truncated": preview.render_hint.truncated,
            "truncated_length": preview.render_hint.truncated_length,
        },
        "source_id": null,
        "error": preview.error,
    }))
}
