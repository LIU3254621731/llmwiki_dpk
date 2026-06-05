use std::sync::Arc;
use tauri::State;
use serde_json::Value;

use crate::core::app_kernel::AppKernel;
use crate::local_fs::local_fs_service::LocalFsService;

#[tauri::command]
pub fn scan_local_directory(root: String) -> Result<Vec<Value>, String> {
    let root_path = std::path::Path::new(&root);
    let entries = LocalFsService::scan_local_directory(root_path)?;
    Ok(entries
        .into_iter()
        .map(|e| {
            serde_json::json!({
                "relativePath": e.relative_path,
                "absolutePath": e.absolute_path,
                "title": e.title,
                "snippet": e.snippet,
            })
        })
        .collect())
}

#[tauri::command]
pub fn read_local_file(path: String) -> Result<String, String> {
    let file_path = std::path::Path::new(&path);
    LocalFsService::read_local_file(file_path)
}

#[tauri::command]
pub fn save_wiki_page_local(
    _kernel: State<'_, Arc<AppKernel>>,
    title: String,
    content: String,
    root: Option<String>,
) -> Result<(), String> {
    let local_root = match root {
        Some(r) => std::path::PathBuf::from(&r),
        None => LocalFsService::get_default_local_root(),
    };

    LocalFsService::ensure_local_root(&local_root)?;

    // Generate safe filename from title
    let safe_name = crate::wiki::path_service::PathService::generate_safe_name(&title);
    let relative_path = format!("{}.md", safe_name);

    LocalFsService::write_local_md(&local_root, &relative_path, &content)?;

    log::info!(
        "Local-first save: {} -> {:?}",
        title,
        local_root.join(&relative_path)
    );

    Ok(())
}

#[tauri::command]
pub fn get_default_local_root() -> String {
    LocalFsService::get_default_local_root()
        .to_string_lossy()
        .to_string()
}
