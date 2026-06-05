use std::sync::Arc;
use tauri::State;
use crate::core::app_kernel::AppKernel;

#[tauri::command]
pub async fn get_graph_data(
    kernel: State<'_, Arc<AppKernel>>,
    kb_id: String,
) -> Result<serde_json::Value, String> {
    let data = crate::graph::graph_service::GraphService::get_graph_data(&kernel.db, &kb_id)?;

    Ok(serde_json::json!({
        "nodes": data.nodes.iter().map(|n| serde_json::json!({
            "id": n.id,
            "type": n.node_type,
            "label": n.label,
            "path": n.path,
            "aliases": n.aliases,
            "tags": n.tags,
            "summary": n.summary,
            "sourceCount": n.source_count,
            "inDegree": n.in_degree,
            "outDegree": n.out_degree,
            "status": n.status,
            "createdAt": n.created_at,
        })).collect::<Vec<_>>(),
        "edges": data.edges.iter().map(|e| serde_json::json!({
            "id": e.id,
            "source": e.source,
            "target": e.target,
            "type": e.edge_type,
            "relation": e.relation,
            "confidence": e.confidence,
            "evidenceSourceId": e.evidence_source_id,
            "evidenceLocation": e.evidence_location,
            "citationStatus": e.citation_status,
            "createdByTask": e.created_by_task,
        })).collect::<Vec<_>>(),
        "health": serde_json::json!({
            "nodeCount": data.health.node_count,
            "edgeCount": data.health.edge_count,
            "orphanCount": data.health.orphan_count,
            "lowConfidenceCount": data.health.low_confidence_count,
            "conflictCount": data.health.conflict_count,
            "needsReviewCount": data.health.needs_review_count,
            "uncitedCount": data.health.uncited_count,
            "avgDegree": data.health.avg_degree,
            "maxHubLabel": data.health.max_hub_label,
            "maxHubDegree": data.health.max_hub_degree,
        }),
    }))
}

#[tauri::command]
pub async fn sync_graph_data(
    kernel: State<'_, Arc<AppKernel>>,
    kb_id: String,
) -> Result<serde_json::Value, String> {
    crate::graph::graph_service::GraphService::sync_from_knowledge_items(&kernel.db, &kb_id)?;
    let rels_created = crate::graph::graph_service::GraphService::derive_relationships(&kernel.db, &kb_id)?;
    Ok(serde_json::json!({ "relationships_created": rels_created }))
}

#[tauri::command]
pub async fn get_graph_stats(
    kernel: State<'_, Arc<AppKernel>>,
    kb_id: String,
) -> Result<serde_json::Value, String> {
    let data = crate::graph::graph_service::GraphService::get_graph_data(&kernel.db, &kb_id)?;
    let h = &data.health;
    Ok(serde_json::json!({
        "nodeCount": h.node_count,
        "edgeCount": h.edge_count,
        "orphanCount": h.orphan_count,
        "lowConfidenceCount": h.low_confidence_count,
        "conflictCount": h.conflict_count,
        "needsReviewCount": h.needs_review_count,
        "uncitedCount": h.uncited_count,
        "avgDegree": h.avg_degree,
        "maxHubLabel": h.max_hub_label,
        "maxHubDegree": h.max_hub_degree,
    }))
}

#[tauri::command]
pub async fn search_graph_nodes(
    kernel: State<'_, Arc<AppKernel>>,
    kb_id: String,
    keyword: String,
) -> Result<Vec<serde_json::Value>, String> {
    let conn = kernel.db.connect()?;
    let pattern = format!("%{}%", keyword);
    let mut stmt = conn.prepare(
        "SELECT id, node_type, label, path FROM graph_nodes WHERE kb_id = ?1 AND label LIKE ?2"
    ).map_err(|e| format!("准备查询失败: {}", e))?;
    let mapped = stmt.query_map(rusqlite::params![kb_id, pattern], |row| {
        Ok(serde_json::json!({
            "id": row.get::<_,String>(0)?,
            "type": row.get::<_,String>(1)?,
            "label": row.get::<_,String>(2)?,
            "path": row.get::<_,String>(3)?,
        }))
    }).map_err(|e| format!("查询失败: {}", e))?;
    mapped.collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("读取搜索节点结果失败: {}", e))
}

#[tauri::command]
pub async fn get_node_relations(
    kernel: State<'_, Arc<AppKernel>>,
    kb_id: String,
    node_id: String,
) -> Result<serde_json::Value, String> {
    let conn = kernel.db.connect()?;

    let mut stmt = conn.prepare(
        "SELECT e.id, e.edge_type, n.id, n.label, n.node_type, n.path FROM graph_edges e JOIN graph_nodes n ON e.target_node_id = n.id WHERE e.kb_id = ?1 AND e.source_node_id = ?2"
    ).map_err(|e| format!("准备查询失败: {}", e))?;
    let mapped = stmt.query_map(rusqlite::params![kb_id, node_id], |row| {
        Ok(serde_json::json!({
            "edgeId": row.get::<_,String>(0)?,
            "relation": row.get::<_,String>(1)?,
            "targetId": row.get::<_,String>(2)?,
            "targetLabel": row.get::<_,String>(3)?,
            "targetType": row.get::<_,String>(4)?,
            "targetPath": row.get::<_,String>(5)?,
        }))
    }).map_err(|e| format!("查询失败: {}", e))?;
    let outgoing: Vec<_> = mapped
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("读取出边关系失败: {}", e))?;

    let mut stmt2 = conn.prepare(
        "SELECT e.id, e.edge_type, n.id, n.label, n.node_type, n.path FROM graph_edges e JOIN graph_nodes n ON e.source_node_id = n.id WHERE e.kb_id = ?1 AND e.target_node_id = ?2"
    ).map_err(|e| format!("准备查询失败: {}", e))?;
    let mapped2 = stmt2.query_map(rusqlite::params![kb_id, node_id], |row| {
        Ok(serde_json::json!({
            "edgeId": row.get::<_,String>(0)?,
            "relation": row.get::<_,String>(1)?,
            "sourceId": row.get::<_,String>(2)?,
            "sourceLabel": row.get::<_,String>(3)?,
            "sourceType": row.get::<_,String>(4)?,
            "sourcePath": row.get::<_,String>(5)?,
        }))
    }).map_err(|e| format!("查询失败: {}", e))?;
    let incoming: Vec<_> = mapped2
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("读取入边关系失败: {}", e))?;

    Ok(serde_json::json!({ "outgoing": outgoing, "incoming": incoming }))
}

#[tauri::command]
pub async fn add_graph_node(
    kernel: State<'_, Arc<AppKernel>>,
    kb_id: String,
    label: String,
    node_type: String,
    path: Option<String>,
) -> Result<serde_json::Value, String> {
    let node_id = crate::graph::graph_service::GraphService::add_or_update_node(
        &kernel.db, &kb_id, &node_type, &label, &path.unwrap_or_default(),
    )?;
    kernel.event_bus.emit_kb_stats_changed(&kb_id);
    Ok(serde_json::json!({ "id": node_id, "label": label, "type": node_type }))
}

#[tauri::command]
pub async fn delete_graph_node(
    kernel: State<'_, Arc<AppKernel>>,
    kb_id: String,
    node_id: String,
) -> Result<(), String> {
    let conn = kernel.db.connect()?;

    conn.execute("BEGIN TRANSACTION", [])
        .map_err(|e| format!("开始事务失败: {}", e))?;

    let delete_result = (|| -> Result<(), String> {
        conn.execute("DELETE FROM graph_edges WHERE kb_id = ?1 AND (source_node_id = ?2 OR target_node_id = ?2)",
            rusqlite::params![kb_id, node_id],
        ).map_err(|e| format!("删除关联边失败: {}", e))?;
        conn.execute("DELETE FROM graph_nodes WHERE kb_id = ?1 AND id = ?2",
            rusqlite::params![kb_id, node_id],
        ).map_err(|e| format!("删除节点失败: {}", e))?;
        Ok(())
    })();

    match delete_result {
        Ok(()) => {
            conn.execute("COMMIT", [])
                .map_err(|e| format!("提交事务失败: {}", e))?;
        }
        Err(e) => {
            let _ = conn.execute("ROLLBACK", []);
            return Err(e);
        }
    }

    kernel.event_bus.emit_kb_stats_changed(&kb_id);
    Ok(())
}

#[tauri::command]
pub async fn add_graph_edge(
    kernel: State<'_, Arc<AppKernel>>,
    kb_id: String,
    source_node_id: String,
    target_node_id: String,
    relation: String,
) -> Result<serde_json::Value, String> {
    let conn = kernel.db.connect()?;

    // Verify both nodes exist before creating the edge
    let source_exists: bool = conn.query_row(
        "SELECT COUNT(*) > 0 FROM graph_nodes WHERE id = ?1 AND kb_id = ?2",
        rusqlite::params![source_node_id, kb_id],
        |row| row.get(0),
    ).map_err(|e| format!("查询源节点失败: {}", e))?;
    if !source_exists {
        return Err("源节点不存在".to_string());
    }

    let target_exists: bool = conn.query_row(
        "SELECT COUNT(*) > 0 FROM graph_nodes WHERE id = ?1 AND kb_id = ?2",
        rusqlite::params![target_node_id, kb_id],
        |row| row.get(0),
    ).map_err(|e| format!("查询目标节点失败: {}", e))?;
    if !target_exists {
        return Err("目标节点不存在".to_string());
    }

    let edge_id = uuid::Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO graph_edges (id, kb_id, source_node_id, target_node_id, edge_type, relation, confidence, metadata) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'medium', '{}')",
        rusqlite::params![edge_id, kb_id, source_node_id, target_node_id, relation, relation],
    ).map_err(|e| format!("创建边失败: {}", e))?;
    kernel.event_bus.emit_kb_stats_changed(&kb_id);
    Ok(serde_json::json!({ "id": edge_id, "source": source_node_id, "target": target_node_id, "relation": relation }))
}

#[tauri::command]
pub async fn delete_graph_edge(
    kernel: State<'_, Arc<AppKernel>>,
    kb_id: String,
    edge_id: String,
) -> Result<(), String> {
    let conn = kernel.db.connect()?;
    conn.execute("DELETE FROM graph_edges WHERE kb_id = ?1 AND id = ?2",
        rusqlite::params![kb_id, edge_id],
    ).map_err(|e| format!("删除边失败: {}", e))?;
    kernel.event_bus.emit_kb_stats_changed(&kb_id);
    Ok(())
}

#[tauri::command]
pub async fn compute_wikilink_graph_layout(
    kb_path: String,
    scan_root: Option<String>,
) -> Result<serde_json::Value, String> {
    let root = match scan_root {
        Some(r) => {
            let p = std::path::Path::new(&kb_path).join(&r);
            if p.exists() {
                p
            } else {
                std::path::PathBuf::from(&r)
            }
        }
        None => {
            // Default: scan the wiki/ directory under kb_path
            let wiki_dir = std::path::Path::new(&kb_path).join("wiki");
            if wiki_dir.exists() {
                wiki_dir
            } else {
                std::path::PathBuf::from(&kb_path)
            }
        }
    };

    let layout = crate::graph::topology_engine::TopologyEngine::compute_layout(&root)?;

    Ok(serde_json::json!({
        "nodes": layout.nodes.iter().map(|n| serde_json::json!({
            "id": n.id,
            "label": n.label,
            "filePath": n.file_path,
            "level": n.level,
            "x": n.x,
            "y": n.y,
            "inDegree": n.in_degree,
            "outDegree": n.out_degree,
        })).collect::<Vec<_>>(),
        "edges": layout.edges.iter().map(|e| serde_json::json!({
            "source": e.source,
            "target": e.target,
            "label": e.label,
        })).collect::<Vec<_>>(),
    }))
}
