pub mod commands;
pub mod core;
pub mod skills;
pub mod agents;
pub mod model;
pub mod prompts;
pub mod wiki;
pub mod review;
pub mod search;
pub mod graph;
pub mod local_fs;
pub mod schema;
pub mod recovery;
pub mod db;
pub mod dedup;
pub mod embedding;
pub mod canvas;

use core::app_kernel::AppKernel;
use std::sync::Arc;
use tauri::Manager;

pub fn run() {
    // 设置 panic hook 将崩溃信息写入文件，便于诊断闪退
    let panic_log_path = std::env::temp_dir().join("llmwiki_crash.log");
    let panic_log_path_clone = panic_log_path.clone();
    std::panic::set_hook(Box::new(move |info| {
        let msg = format!(
            "LLMWiki 崩溃\r\n时间: {}\r\n原因: {}\r\n",
            chrono::Utc::now().to_rfc3339(),
            info
        );
        let _ = std::fs::write(&panic_log_path_clone, &msg);
    }));

    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp_millis()
        .init();

    log::info!("LLMWiki 启动中... 崩溃日志路径: {:?}", panic_log_path);

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let kernel = Arc::new(AppKernel::new(app.handle())?);
            app.manage(kernel);
            log::info!("LLMWiki 知识库启动完成");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // 工作区
            commands::workspace::create_knowledge_base,
            commands::workspace::list_knowledge_bases,
            commands::workspace::get_kb_stats,
            commands::workspace::init_workspace_dirs,
            commands::workspace::delete_knowledge_base,
            commands::workspace::update_knowledge_base,
            commands::workspace::reset_all_data,
            // 配置
            commands::config::save_provider_config,
            commands::config::get_provider_config,
            commands::config::save_deepseek_config,
            commands::config::get_deepseek_config,
            commands::config::test_connection,
            commands::config::test_json_output,
            commands::config::test_document_attachment,
            commands::config::chat_stream,
            commands::config::check_api_key_status,
            commands::config::list_model_profiles,
            commands::config::save_model_profile,
            commands::config::delete_model_profile,
            commands::config::apply_model_profile,
            commands::config::save_web_search_config,
            commands::config::get_web_search_config,
            // Source 文件
            commands::source::check_file_hash,
            commands::source::upload_source_file,
            commands::source::list_sources,
            commands::source::get_source_detail,
            commands::source::delete_source,
            commands::source::reimport_source,
            commands::source::get_source_summary,
            // 任务
            commands::task::list_tasks,
            commands::task::list_tasks_filtered,
            commands::task::get_task_detail,
            commands::task::get_task_events,
            commands::task::get_task_review_items,
            commands::task::retry_task,
            commands::task::cancel_task,
            commands::task::resume_task,
            commands::task::archive_task,
            commands::task::handle_failed_task,
            commands::task::get_unhandled_failed_count,
            commands::task::run_source_ingest,
            commands::task::run_resolution,
            commands::task::run_relationship,
            commands::task::run_wiki_update,
            // 审阅
            commands::review::get_pending_reviews,
            commands::review::get_review_detail,
            commands::review::accept_review_item,
            commands::review::reject_review_item,
            commands::review::accept_all_low_risk_review,
            commands::review::reject_all_review,
            commands::review::delete_review_item,
            commands::review::regenerate_review,
            // Wiki
            commands::wiki::list_wiki_pages,
            commands::wiki::get_wiki_page_content,
            commands::wiki::save_wiki_page,
            commands::wiki::delete_wiki_page,
            commands::wiki::get_wiki_page_versions,
            commands::wiki::rollback_wiki_page,
            commands::wiki::list_page_versions,
            commands::wiki::get_page_version_snapshot,
            commands::wiki::get_index_content,
            commands::wiki::get_log_content,
            commands::wiki::resolve_wiki_link,
            // 本地文件系统
            commands::local_fs::scan_local_directory,
            commands::local_fs::read_local_file,
            commands::local_fs::save_wiki_page_local,
            commands::local_fs::get_default_local_root,
            // 问答
            commands::task::run_query,
            commands::task::save_answer_as_wiki,
            // 对话历史
            commands::chat_history::list_conversations,
            commands::chat_history::create_conversation,
            commands::chat_history::get_conversation_messages,
            commands::chat_history::save_message,
            commands::chat_history::delete_conversation,
            commands::chat_history::update_conversation_title,
            // 网页搜索
            commands::web_search::web_search,
            commands::web_search::save_web_result_as_source,
            commands::web_search::fetch_web_page_content,
            // 画布 (Canvas)
            commands::canvas::get_canvas_tag_suggestions,
            commands::canvas::check_canvas_scope,
            commands::canvas::generate_canvas_outline,
            commands::canvas::generate_canvas_textbook,
            commands::canvas::get_canvas_node_detail,
            commands::canvas::get_canvas_scopes,
            commands::canvas::save_canvas_scope,
            commands::canvas::delete_canvas_scope,
            commands::canvas::rename_canvas_scope,
            commands::canvas::generate_canvas_outline_from_web,
            commands::canvas::generate_canvas_textbook_from_web,
            commands::canvas::generate_mindmap_from_text,
            // 画布引擎 (Canvas Engine — dual-layer canvas system)
            commands::canvas_engine::save_canvas_state,
            commands::canvas_engine::load_canvas_state,
            commands::canvas_engine::delete_canvas_state,
            commands::canvas_engine::list_canvas_states,
            // 搜索
            commands::search::full_text_search,
            // 图谱
            commands::graph::get_graph_data,
            commands::graph::sync_graph_data,
            commands::graph::get_graph_stats,
            commands::graph::search_graph_nodes,
            commands::graph::get_node_relations,
            commands::graph::add_graph_node,
            commands::graph::delete_graph_node,
            commands::graph::add_graph_edge,
            commands::graph::delete_graph_edge,
            commands::graph::compute_wikilink_graph_layout,
            // 健康检查
            commands::task::generate_mindmap,
            commands::task::get_health_snapshot,
            commands::task::run_health_check,
            commands::task::run_health_check_structured,
            commands::task::run_link_sanitize,
            commands::task::get_sanitize_log,
            commands::task::run_reconcile,
            // 恢复
            commands::task::run_recovery_check,
            commands::task::get_interrupted_tasks,
            commands::task::get_task_files,
            commands::task::read_task_file,
            commands::task::recover_page_from_snapshot,
            commands::task::repair_all_wiki_paths,
            commands::task::sync_wiki_index_from_markdown,
            commands::task::mark_page_broken,
            commands::task::delete_broken_page_record,
            // 文档解析
            commands::source::parse_document_text,
            commands::source::get_supported_file_types,
            commands::source::batch_import_sources,
            commands::source::scan_import_folder,
            commands::source::import_folder,
            commands::source::validate_citation_target,
            // 文件树
            commands::file_tree::scan_workspace_files,
            commands::file_tree::get_file_tree,
            commands::file_tree::get_file_detail,
            commands::file_tree::list_files,
            commands::file_tree::get_workspace_file_preview,
            commands::file_tree::save_workspace_file,
            commands::file_tree::create_workspace_file,
            commands::file_tree::create_workspace_folder,
            commands::file_tree::delete_workspace_file,
            commands::file_tree::rename_workspace_file,
            commands::file_tree::preview_local_file,
            // Source Preview
            commands::source_preview::generate_source_preview,
            commands::source_preview::get_source_preview,
            commands::source_preview::rebuild_all_previews,
            commands::source_preview::get_source_detail_v2,
            // Token 监测
            commands::token::get_token_statistics,
            commands::token::get_token_daily_trend,
            commands::token::get_token_logs,
            commands::token::get_daily_token_limit,
            commands::token::set_daily_token_limit,
            commands::token::check_token_quota,
            // 向量数据库
            commands::vdb::get_vdb_status,
            commands::vdb::get_embedding_config,
            commands::vdb::save_embedding_config,
            commands::vdb::reindex_vdb,
            commands::vdb::flush_vdb,
            // Agent/Skill 管理系统
            commands::agent::list_agent_definitions,
            commands::agent::create_agent_definition,
            commands::agent::update_agent_definition,
            commands::agent::delete_agent_definition,
            commands::skill::list_skill_definitions,
            commands::skill::create_skill_definition,
            commands::skill::update_skill_definition,
            commands::skill::delete_skill_definition,
            commands::skill::validate_skill_schema,
            commands::skill::execute_skill_mock,
            // 系统工具
            commands::utils::shell_open,
            commands::utils::get_markitdown_status,
            commands::utils::retry_markitdown_install,
        ])
        .run(tauri::generate_context!())
        .expect("启动应用失败");
}
