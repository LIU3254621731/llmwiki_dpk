// ── Canvas Engine: persistence commands for the dual-layer canvas system ──
// Provides save/load for both Macro (tag relationship network) and Micro (mindmap) canvases.

use std::sync::Arc;
use tauri::State;
use crate::core::app_kernel::AppKernel;

#[tauri::command]
pub async fn save_canvas_state(
    kernel: State<'_, Arc<AppKernel>>,
    kb_id: String,
    canvas_type: String, // "macro" or "micro"
    canvas_id: String,   // "default" for macro, tagId for micro
    schema_json: String,
) -> Result<(), String> {
    let conn = kernel.db.connect()?;
    let now = chrono::Utc::now().to_rfc3339();

    // Validate that it's valid JSON
    let _: serde_json::Value = serde_json::from_str(&schema_json)
        .map_err(|e| format!("Invalid schema JSON: {}", e))?;

    // Upsert: INSERT OR REPLACE
    conn.execute(
        "INSERT INTO canvas_snapshots (id, kb_id, canvas_type, canvas_id, schema_json, metadata_json, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, '{}', ?6, ?7)
         ON CONFLICT(kb_id, canvas_type, canvas_id) DO UPDATE SET
           schema_json = excluded.schema_json,
           updated_at = excluded.updated_at",
        rusqlite::params![
            format!("cs_{}_{}_{}", kb_id, canvas_type, canvas_id),
            kb_id,
            canvas_type,
            canvas_id,
            schema_json,
            now,
            now,
        ],
    )
    .map_err(|e| format!("Save canvas state failed: {}", e))?;

    log::info!("[canvas_engine] Saved {} canvas '{}' for kb={}", canvas_type, canvas_id, kb_id);
    Ok(())
}

#[tauri::command]
pub async fn load_canvas_state(
    kernel: State<'_, Arc<AppKernel>>,
    kb_id: String,
    canvas_type: String,
    canvas_id: String,
) -> Result<Option<serde_json::Value>, String> {
    let conn = kernel.db.connect()?;

    let result = conn.query_row(
        "SELECT schema_json, created_at, updated_at FROM canvas_snapshots
         WHERE kb_id = ?1 AND canvas_type = ?2 AND canvas_id = ?3
         ORDER BY updated_at DESC LIMIT 1",
        rusqlite::params![kb_id, canvas_type, canvas_id],
        |row| {
            Ok(serde_json::json!({
                "canvasType": canvas_type.clone(),
                "canvasId": canvas_id.clone(),
                "kbId": kb_id.clone(),
                "schema": serde_json::from_str::<serde_json::Value>(&row.get::<_, String>(0)?).unwrap_or(serde_json::Value::Null),
                "lastModified": row.get::<_, String>(2).unwrap_or_default(),
            }))
        },
    );

    match result {
        Ok(val) => Ok(Some(val)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(format!("Load canvas state failed: {}", e)),
    }
}

#[tauri::command]
pub async fn delete_canvas_state(
    kernel: State<'_, Arc<AppKernel>>,
    kb_id: String,
    canvas_type: String,
    canvas_id: String,
) -> Result<(), String> {
    let conn = kernel.db.connect()?;

    conn.execute(
        "DELETE FROM canvas_snapshots WHERE kb_id = ?1 AND canvas_type = ?2 AND canvas_id = ?3",
        rusqlite::params![kb_id, canvas_type, canvas_id],
    )
    .map_err(|e| format!("Delete canvas state failed: {}", e))?;

    Ok(())
}

#[tauri::command]
pub async fn list_canvas_states(
    kernel: State<'_, Arc<AppKernel>>,
    kb_id: String,
    canvas_type: Option<String>,
) -> Result<Vec<serde_json::Value>, String> {
    let conn = kernel.db.connect()?;

    let sql = if canvas_type.is_some() {
        "SELECT canvas_type, canvas_id, created_at, updated_at FROM canvas_snapshots
         WHERE kb_id = ?1 AND canvas_type = ?2 ORDER BY updated_at DESC"
    } else {
        "SELECT canvas_type, canvas_id, created_at, updated_at FROM canvas_snapshots
         WHERE kb_id = ?1 ORDER BY updated_at DESC"
    };

    let mut stmt = conn.prepare(sql).map_err(|e| format!("Prepare failed: {}", e))?;

    let params: Vec<Box<dyn rusqlite::types::ToSql>> = if canvas_type.is_some() {
        vec![Box::new(kb_id.clone()), Box::new(canvas_type.unwrap())]
    } else {
        vec![Box::new(kb_id.clone())]
    };

    let param_refs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|p| p.as_ref()).collect();

    let rows = stmt
        .query_map(param_refs.as_slice(), |row| {
            Ok(serde_json::json!({
                "canvasType": row.get::<_, String>(0)?,
                "canvasId": row.get::<_, String>(1)?,
                "createdAt": row.get::<_, String>(2)?,
                "updatedAt": row.get::<_, String>(3)?,
            }))
        })
        .map_err(|e| format!("Query failed: {}", e))?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Collect rows failed: {}", e))
}
