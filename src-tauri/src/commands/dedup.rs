use std::sync::Arc;
use tauri::State;
use crate::core::app_kernel::AppKernel;
use crate::dedup::dedup_service::DedupService;

/// Batch scan all pages in a knowledge base and flag duplicates.
/// Uses strsim::normalized_damerau_levenshtein fuzzy matching with threshold 0.85.
#[tauri::command]
pub async fn dedup_cleanup(
    kernel: State<'_, Arc<AppKernel>>,
    kb_id: String,
) -> Result<Vec<crate::dedup::dedup_service::DuplicatePageGroup>, String> {
    let service = DedupService::new(kernel.db.clone());
    service.dedup_cleanup(&kb_id)
}
