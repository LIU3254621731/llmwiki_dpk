use std::sync::Arc;
use tauri::State;
use crate::core::app_kernel::AppKernel;
use crate::search::full_text_search::FullTextSearch;

#[tauri::command]
pub async fn full_text_search(
    kernel: State<'_, Arc<AppKernel>>,
    kb_id: String,
    kb_path: String,
    query: String,
) -> Result<Vec<serde_json::Value>, String> {
    let wiki_dir = std::path::PathBuf::from(&kb_path).join("wiki");
    let results = FullTextSearch::search(&kernel.db, &kb_id, &query, &wiki_dir)?;

    Ok(results.iter().map(|r| serde_json::json!({
        "title": r.title,
        "path": r.path,
        "page_type": r.page_type,
        "matched_field": r.matched_field,
        "snippet": r.snippet,
        "updated_at": r.updated_at,
        "page_id": r.page_id,
        "tags": r.tags,
        "is_broken": r.is_broken,
    })).collect())
}
