use std::sync::Arc;
use tauri::State;
use crate::core::app_kernel::AppKernel;
use crate::core::token_logger::{
    TokenStats, DailyTokenUsage, PaginatedTokenLogs, DailyTokenLimit, TokenQuotaStatus,
};

#[tauri::command]
pub async fn get_token_statistics(
    kernel: State<'_, Arc<AppKernel>>,
    range: String,
) -> Result<TokenStats, String> {
    kernel.token_logger.get_statistics(&range)
}

#[tauri::command]
pub async fn get_token_daily_trend(
    kernel: State<'_, Arc<AppKernel>>,
) -> Result<Vec<DailyTokenUsage>, String> {
    kernel.token_logger.get_daily_trend()
}

#[tauri::command]
pub async fn get_token_logs(
    kernel: State<'_, Arc<AppKernel>>,
    page: u64,
    page_size: u64,
) -> Result<PaginatedTokenLogs, String> {
    kernel.token_logger.get_logs(page, page_size)
}

#[tauri::command]
pub async fn get_daily_token_limit(
    kernel: State<'_, Arc<AppKernel>>,
) -> Result<DailyTokenLimit, String> {
    kernel.token_logger.get_daily_limit()
}

#[tauri::command]
pub async fn set_daily_token_limit(
    kernel: State<'_, Arc<AppKernel>>,
    enabled: bool,
    limit: u64,
) -> Result<(), String> {
    kernel.token_logger.save_daily_limit(&DailyTokenLimit { enabled, limit })
}

#[tauri::command]
pub async fn check_token_quota(
    kernel: State<'_, Arc<AppKernel>>,
) -> Result<TokenQuotaStatus, String> {
    kernel.token_logger.check_quota()
}
