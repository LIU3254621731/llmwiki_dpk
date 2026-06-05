use std::sync::Arc;
use tauri::State;
use crate::core::app_kernel::AppKernel;
use crate::core::source_preview_service::SourcePreviewService;

#[tauri::command]
pub async fn generate_source_preview(
    kernel: State<'_, Arc<AppKernel>>,
    source_id: String,
    kb_path: String,
) -> Result<serde_json::Value, String> {
    let root_path = std::path::PathBuf::from(&kb_path);
    let result = SourcePreviewService::generate_preview(&kernel.db, &root_path, &source_id)?;
    Ok(serde_json::json!(result))
}

#[tauri::command]
pub async fn get_source_preview(
    kernel: State<'_, Arc<AppKernel>>,
    source_id: String,
    kb_path: String,
) -> Result<serde_json::Value, String> {
    let root_path = std::path::PathBuf::from(&kb_path);
    let result = SourcePreviewService::get_preview(&kernel.db, &root_path, &source_id)?;
    Ok(serde_json::json!(result))
}

#[tauri::command]
pub async fn rebuild_all_previews(
    kernel: State<'_, Arc<AppKernel>>,
    kb_id: String,
    kb_path: String,
) -> Result<serde_json::Value, String> {
    let root_path = std::path::PathBuf::from(&kb_path);
    SourcePreviewService::rebuild_all_previews(&kernel.db, &root_path, &kb_id)
}

#[tauri::command]
pub async fn get_source_detail_v2(
    kernel: State<'_, Arc<AppKernel>>,
    source_id: String,
    _kb_path: String,
) -> Result<serde_json::Value, String> {
    let conn = kernel.db.connect()?;

    let (kb_id, file_name, file_path, file_type, file_size, file_hash, extracted_text, status, created_at, updated_at,
         ai_summary, coverage_report, preview_path, preview_status, preview_generated_at, preview_error,
         summary_json_path, coverage_json_path, linked_pages_count, linked_relations_count):
        (String, String, String, String, i64, String, String, String, String, String,
         String, String, String, String, String, String,
         String, String, i64, i64) = conn
        .query_row(
            "SELECT kb_id, file_name, file_path, file_type, COALESCE(file_size,0), COALESCE(file_hash,''),
             COALESCE(extracted_text,''), COALESCE(status,''), COALESCE(created_at,''), COALESCE(updated_at,''),
             COALESCE(ai_summary,''), COALESCE(coverage_report,''), COALESCE(preview_path,''),
             COALESCE(preview_status,''), COALESCE(preview_generated_at,''), COALESCE(preview_error,''),
             COALESCE(summary_json_path,''), COALESCE(coverage_json_path,''),
             COALESCE(linked_pages_count,0), COALESCE(linked_relations_count,0)
             FROM sources WHERE id = ?1",
            rusqlite::params![source_id],
            |row| Ok((
                row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?,
                row.get(6)?, row.get(7)?, row.get(8)?, row.get(9)?,
                row.get(10)?, row.get(11)?, row.get(12)?, row.get(13)?, row.get(14)?, row.get(15)?,
                row.get(16)?, row.get(17)?, row.get(18)?, row.get(19)?,
            )),
        )
        .map_err(|e| format!("获取 source 详情失败: {}", e))?;

    // 查询关联的 Wiki 页面
    let mut linked_wiki_pages: Vec<serde_json::Value> = Vec::new();
    if let Ok(mut stmt) = conn.prepare(
        "SELECT wp.id, wp.title, wp.path, wp.page_type FROM wiki_pages wp
         JOIN knowledge_items ki ON ki.page_path = wp.path
         WHERE ki.source_id = ?1 AND wp.kb_id = ?2"
    ) {
        if let Ok(rows) = stmt.query_map(rusqlite::params![source_id, kb_id], |row| {
            Ok(serde_json::json!({
                "id": row.get::<_, String>(0)?,
                "title": row.get::<_, String>(1)?,
                "path": row.get::<_, String>(2)?,
                "page_type": row.get::<_, String>(3)?,
            }))
        }) {
            linked_wiki_pages = rows.filter_map(|r| r.ok()).collect();
        }
    }

    // 查询关联的 Review
    let review_count: i64 = match conn.query_row(
        "SELECT COUNT(DISTINCT r.id) FROM reviews r JOIN review_items ri ON ri.review_id = r.id WHERE r.kb_id = ?1 AND ri.source_id = ?2",
        rusqlite::params![kb_id, source_id],
        |row| row.get(0),
    ) {
        Ok(c) => c,
        Err(rusqlite::Error::QueryReturnedNoRows) => 0,
        Err(e) => { log::error!("[source_preview] 查询审阅计数失败 (source={}): {}", source_id, e); 0 }
    };

    // 查询关联任务
    let mut tasks: Vec<serde_json::Value> = Vec::new();
    if let Ok(mut stmt) = conn.prepare(
        "SELECT t.id, t.task_type, t.status, t.created_at, t.error_message FROM tasks t WHERE t.kb_id = ?1 AND t.input_ref = ?2 ORDER BY t.created_at DESC LIMIT 10"
    ) {
        if let Ok(rows) = stmt.query_map(rusqlite::params![kb_id, source_id], |row| {
            Ok(serde_json::json!({
                "id": row.get::<_, String>(0)?,
                "task_type": row.get::<_, String>(1)?,
                "status": row.get::<_, String>(2)?,
                "created_at": row.get::<_, String>(3)?,
                "error_message": row.get::<_, String>(4)?,
            }))
        }) {
            tasks = rows.filter_map(|r| r.ok()).collect();
        }
    }

    // 查询关联图谱节点
    let mut graph_nodes: Vec<serde_json::Value> = Vec::new();
    if let Ok(mut stmt) = conn.prepare(
        "SELECT id, label, node_type FROM graph_nodes WHERE kb_id = ?1 AND source_id = ?2 LIMIT 20"
    ) {
        if let Ok(rows) = stmt.query_map(rusqlite::params![kb_id, source_id], |row| {
            Ok(serde_json::json!({
                "id": row.get::<_, String>(0)?,
                "label": row.get::<_, String>(1)?,
                "node_type": row.get::<_, String>(2)?,
            }))
        }) {
            graph_nodes = rows.filter_map(|r| r.ok()).collect();
        }
    }

    // 查询知识项统计
    let (entity_count, concept_count, relation_count): (i64, i64, i64) = match conn.query_row(
        "SELECT
            COALESCE(SUM(CASE WHEN item_type = 'entity' THEN 1 ELSE 0 END), 0),
            COALESCE(SUM(CASE WHEN item_type = 'concept' THEN 1 ELSE 0 END), 0),
            COUNT(ki2.id)
         FROM knowledge_items ki
         LEFT JOIN relationships ki2 ON (ki.id = ki2.source_item_id OR ki.id = ki2.target_item_id) AND ki2.kb_id = ?2
         WHERE ki.kb_id = ?2 AND ki.source_id = ?1",
        rusqlite::params![source_id, kb_id],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    ) {
        Ok(counts) => counts,
        Err(rusqlite::Error::QueryReturnedNoRows) => (0, 0, 0),
        Err(e) => { log::error!("[source_preview] 查询知识项统计失败 (source={}): {}", source_id, e); (0, 0, 0) }
    };

    // 查询知识项详情
    let mut knowledge_items: Vec<serde_json::Value> = Vec::new();
    if let Ok(mut stmt) = conn.prepare(
        "SELECT id, canonical_name, item_type, COALESCE(page_path,''), COALESCE(page_id,''),
                COALESCE(linked_page_path,''), COALESCE(summary,''), COALESCE(source_id,'')
         FROM knowledge_items
         WHERE kb_id = ?1 AND source_id = ?2
         ORDER BY item_type, canonical_name"
    ) {
        if let Ok(rows) = stmt.query_map(rusqlite::params![kb_id, source_id], |row| {
            Ok(serde_json::json!({
                "id": row.get::<_, String>(0)?,
                "canonical_name": row.get::<_, String>(1)?,
                "item_type": row.get::<_, String>(2)?,
                "page_path": row.get::<_, String>(3)?,
                "page_id": row.get::<_, String>(4)?,
                "linked_page_path": row.get::<_, String>(5)?,
                "summary": row.get::<_, String>(6)?,
                "source_id": row.get::<_, String>(7)?,
            }))
        }) {
            knowledge_items = rows.filter_map(|r| r.ok()).collect();
        }
    }

    Ok(serde_json::json!({
        "id": source_id,
        "kb_id": kb_id,
        "file_name": file_name,
        "file_path": file_path,
        "file_type": file_type,
        "file_size": file_size,
        "file_hash": file_hash,
        "extracted_text": extracted_text,
        "status": status,
        "created_at": created_at,
        "updated_at": updated_at,
        "ai_summary": ai_summary,
        "coverage_report": coverage_report,
        "preview_path": preview_path,
        "preview_status": preview_status,
        "preview_generated_at": preview_generated_at,
        "preview_error": preview_error,
        "summary_json_path": summary_json_path,
        "coverage_json_path": coverage_json_path,
        "linked_pages_count": linked_pages_count,
        "linked_relations_count": linked_relations_count,
        "review_count": review_count,
        "entity_count": entity_count,
        "concept_count": concept_count,
        "relation_count": relation_count,
        "linked_wiki_pages": linked_wiki_pages,
        "tasks": tasks,
        "graph_nodes": graph_nodes,
        "knowledge_items": knowledge_items,
    }))
}
