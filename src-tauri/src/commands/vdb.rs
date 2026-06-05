use std::sync::Arc;
use tauri::State;

use crate::core::app_kernel::AppKernel;
use crate::embedding::vdb_config::EmbeddingConfig;
use crate::embedding::vdb_status::VdbStatus;

#[tauri::command]
pub async fn get_vdb_status(
    kernel: State<'_, Arc<AppKernel>>,
    kb_id: String,
) -> Result<VdbStatus, String> {
    kernel.vdb.get_status(&kb_id)
}

#[tauri::command]
pub async fn get_embedding_config(
    kernel: State<'_, Arc<AppKernel>>,
) -> Result<EmbeddingConfig, String> {
    kernel.vdb.get_config()
}

#[tauri::command]
pub async fn save_embedding_config(
    kernel: State<'_, Arc<AppKernel>>,
    engine_type: String,
    model_path: Option<String>,
    num_threads: u32,
    graph_opt_level: Option<String>,
    max_seq_len: Option<usize>,
    pooling_strategy: Option<String>,
    l2_normalize: Option<bool>,
) -> Result<(), String> {
    if num_threads < 1 {
        return Err("线程数必须 >= 1".to_string());
    }
    let max_threads = num_cpus::get() as u32;
    if num_threads > max_threads {
        return Err(format!("线程数不能超过 CPU 核心数 ({})", max_threads));
    }

    let config = EmbeddingConfig {
        engine_type,
        model_path,
        num_threads,
        graph_opt_level: graph_opt_level.unwrap_or_else(|| "level3".to_string()),
        max_seq_len: max_seq_len.unwrap_or(512),
        pooling_strategy: pooling_strategy.unwrap_or_else(|| "mean".to_string()),
        l2_normalize: l2_normalize.unwrap_or(true),
    };
    kernel.vdb.save_config(&config)?;
    kernel.vdb.init_engine(&config)?;
    Ok(())
}

#[tauri::command]
pub async fn reindex_vdb(
    kernel: State<'_, Arc<AppKernel>>,
    kb_id: String,
) -> Result<(), String> {
    kernel.vdb.start_reindex(&kb_id)
}

#[tauri::command]
pub async fn flush_vdb(
    kernel: State<'_, Arc<AppKernel>>,
    kb_id: String,
) -> Result<(), String> {
    kernel.vdb.flush(&kb_id)
}
