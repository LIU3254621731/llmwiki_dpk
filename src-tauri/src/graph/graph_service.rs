// GraphService - 图谱数据服务 (v0.1.1 增强版)

use std::sync::Arc;
use crate::core::database_service::DatabaseService;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GraphData {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
    pub health: GraphHealth,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GraphNode {
    pub id: String,
    pub node_type: String,
    pub label: String,
    pub path: String,
    pub aliases: Vec<String>,
    pub tags: Vec<String>,
    pub summary: String,
    pub source_count: i32,
    pub in_degree: i32,
    pub out_degree: i32,
    pub status: String,
    pub created_at: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GraphEdge {
    pub id: String,
    pub source: String,
    pub target: String,
    pub edge_type: String,
    pub relation: String,
    pub confidence: String,
    pub evidence_source_id: String,
    pub evidence_location: String,
    pub citation_status: String,
    pub created_by_task: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GraphHealth {
    pub node_count: i64,
    pub edge_count: i64,
    pub orphan_count: i64,
    pub low_confidence_count: i64,
    pub conflict_count: i64,
    pub needs_review_count: i64,
    pub uncited_count: i64,
    pub avg_degree: f64,
    pub max_hub_label: String,
    pub max_hub_degree: i64,
}

pub struct GraphService;

impl GraphService {
    pub fn get_graph_data(db: &Arc<DatabaseService>, kb_id: &str) -> Result<GraphData, String> {
        let conn = db.connect()?;

        // Read nodes exclusively from wiki_pages (post-review) ? never from knowledge_items (pre-review)
        let mut node_stmt = conn.prepare(
            "SELECT wp.id, wp.page_type, wp.title, COALESCE(wp.path,''), '' as summary,
                    0 as out_deg, 0 as in_deg,
                    COALESCE(wp.created_at,'')
             FROM wiki_pages wp WHERE wp.kb_id = ?1"
        ).map_err(|e| format!("??????: {}", e))?;

        let node_mapped = node_stmt.query_map(rusqlite::params![kb_id], |row| {
            Ok((row.get::<_,String>(0)?, row.get::<_,String>(1)?, row.get::<_,String>(2)?,
                row.get::<_,String>(3)?, row.get::<_,String>(4)?,
                row.get::<_,i32>(5)?, row.get::<_,i32>(6)?, row.get::<_,String>(7)?))
        }).map_err(|e| format!("??????: {}", e))?;
        let node_raws: Vec<_> = node_mapped
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("?????????: {}", e))?;

        let mut nodes = Vec::new();
        for (id, node_type, label, path, summary, _out_deg, _in_deg, created_at) in &node_raws {
            // Post-review: compute in/out degree from graph_edges
            let (in_deg, out_deg) = match conn.query_row(
                "SELECT
                    (SELECT COUNT(*) FROM graph_edges WHERE kb_id = ?1 AND target_node_id = ?2),
                    (SELECT COUNT(*) FROM graph_edges WHERE kb_id = ?1 AND source_node_id = ?2)",
                rusqlite::params![kb_id, id],
                |row| Ok((row.get::<_,i32>(0)?, row.get::<_,i32>(1)?)),
            ) {
                Ok((i, o)) => (i, o),
                Err(_) => (0, 0),
            };

            nodes.push(GraphNode {
                id: id.clone(), node_type: node_type.clone(), label: label.clone(),
                path: path.clone(), aliases: vec![], tags: vec![],
                summary: summary.clone(), source_count: 0,
                in_degree: in_deg, out_degree: out_deg,
                status: "normal".into(), created_at: created_at.clone(),
            });
        }

        // Read edges exclusively from graph_edges (post-review sync) ? never fall back to pre-review relationships
        let mut edge_stmt = conn.prepare(
            "SELECT e.id, e.source_node_id, e.target_node_id, COALESCE(e.relation, e.edge_type), COALESCE(e.confidence,'medium'),
                    COALESCE(e.evidence_source_id,''), COALESCE(e.evidence_location,''), COALESCE(e.citation_status,'uncited')
             FROM graph_edges e WHERE e.kb_id = ?1"
        )
            .map_err(|e| format!("?????: {}", e))?;

        let edge_mapped = edge_stmt.query_map(rusqlite::params![kb_id], |row| {
            Ok((
                row.get::<_,String>(0)?, row.get::<_,String>(1)?, row.get::<_,String>(2)?,
                row.get::<_,String>(3)?, row.get::<_,String>(4)?,
                row.get::<_,String>(5)?, row.get::<_,String>(6)?, row.get::<_,String>(7)?
            ))
        }).map_err(|e| format!("?????: {}", e))?;
        let edge_raws: Vec<_> = edge_mapped
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("???????: {}", e))?;

        let mut edges = Vec::new();
        let mut low_conf = 0i64; let mut conflict = 0i64; let mut needs_review = 0i64;

        for (id, source, target, relation, confidence, ev_source, ev_location, status) in &edge_raws {
            if confidence == "low" { low_conf += 1; }
            if relation == "contradicts" { conflict += 1; }
            if status == "pending_review" { needs_review += 1; }

            edges.push(GraphEdge {
                id: id.clone(), source: source.clone(), target: target.clone(),
                edge_type: relation.clone(), relation: relation.clone(),
                confidence: confidence.clone(),
                evidence_source_id: ev_source.clone(),
                evidence_location: ev_location.clone(),
                citation_status: if ev_source.is_empty() { "uncited".into() } else { "cited".into() },
                created_by_task: "".into(),
            });
        }

        // ----- ???? -----
        // Compute referenced count from edges (post-review: only graph_edges matter)
        let mut ref_set = std::collections::HashSet::new();
        for e in &edges {
            ref_set.insert(e.source.clone());
            ref_set.insert(e.target.clone());
        }
        let referenced: i64 = ref_set.len() as i64;
        let orphan = nodes.len() as i64 - referenced;
        let avg_deg = if nodes.is_empty() { 0.0 } else {
            let total_deg: i64 = nodes.iter().map(|n| (n.in_degree + n.out_degree) as i64).sum();
            total_deg as f64 / nodes.len() as f64
        };
        let (max_label, max_deg) = nodes.iter()
            .map(|n| (n.label.clone(), (n.in_degree + n.out_degree) as i64))
            .max_by_key(|(_, d)| *d)
            .unwrap_or_default();

        let health = GraphHealth {
            node_count: nodes.len() as i64,
            edge_count: edges.len() as i64,
            orphan_count: orphan.max(0),
            low_confidence_count: low_conf,
            conflict_count: conflict,
            needs_review_count: needs_review,
            uncited_count: edges.iter().filter(|e| e.evidence_source_id.is_empty()).count() as i64,
            avg_degree: avg_deg,
            max_hub_label: max_label,
            max_hub_degree: max_deg,
        };

        Ok(GraphData { nodes, edges, health })
    }

    
    pub fn sync_from_wiki_pages(db: &Arc<DatabaseService>, kb_id: &str) -> Result<(), String> {
        let conn = db.connect()?;

        conn.execute("DELETE FROM graph_nodes WHERE kb_id = ?1", rusqlite::params![kb_id])
            .map_err(|e| format!("???????: {}", e))?;
        conn.execute("DELETE FROM graph_edges WHERE kb_id = ?1", rusqlite::params![kb_id])
            .map_err(|e| format!("??????: {}", e))?;

        // 1. Generate nodes from wiki_pages (post-review)
        {
            let mut wp_stmt = conn.prepare(
                "SELECT id, title, page_type, path, COALESCE(tags,''), created_at FROM wiki_pages WHERE kb_id = ?1"
            ).map_err(|e| format!("?? wiki_pages ????: {}", e))?;

            let wp_items: Vec<(String, String, String, String, String, String)> = wp_stmt
                .query_map(rusqlite::params![kb_id], |row| {
                    Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?))
                }).map_err(|e| format!("?? wiki_pages ??: {}", e))?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| format!("?? wiki_pages ?????: {}", e))?;

            for (page_id, title, page_type, path, tags, created_at) in &wp_items {
                let node_type = match page_type.as_str() {
                    "entity" => "entity",
                    "topic" => "topic",
                    "question" => "question",
                    "source" => "source",
                    "dataset" => "dataset",
                    "method" => "method",
                    "concept" => "concept",
                    _ => "wikipage",
                };
                conn.execute(
                    "INSERT INTO graph_nodes (id, kb_id, node_type, label, path, aliases, tags, summary, source_count, in_degree, out_degree, status, source_id, page_id, confidence, created_at, metadata)
                     VALUES (?1, ?2, ?3, ?4, ?5, '', ?6, '', 0, 0, 0, 'active', '', ?1, 'medium', ?7, '{}')",
                    rusqlite::params![page_id, kb_id, node_type, title, path, tags, created_at],
                ).map_err(|e| format!("?? wiki_page ????: {}", e))?;
            }
        }

        // 2. Generate source nodes from sources table
        {
            let mut src_stmt = conn.prepare(
                "SELECT id, file_name, file_path FROM sources WHERE kb_id = ?1"
            ).map_err(|e| format!("?? sources ????: {}", e))?;

            let src_items: Vec<(String, String, String)> = src_stmt
                .query_map(rusqlite::params![kb_id], |row| {
                    Ok((row.get(0)?, row.get(1)?, row.get(2)?))
                }).map_err(|e| format!("?? sources ??: {}", e))?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| format!("?? source ?????: {}", e))?;

            for (_id, file_name, file_path) in &src_items {
                let existing: i64 = match conn.query_row(
                    "SELECT COUNT(*) FROM graph_nodes WHERE kb_id = ?1 AND label = ?2 AND node_type = 'source'",
                    rusqlite::params![kb_id, file_name],
                    |row| row.get(0),
                ) {
                    Ok(c) => c,
                    Err(rusqlite::Error::QueryReturnedNoRows) => 0,
                    Err(e) => return Err(format!("?? source ???????? (label={}): {}", file_name, e)),
                };

                if existing > 0 { continue; }

                let node_id = uuid::Uuid::new_v4().to_string();
                conn.execute(
                    "INSERT INTO graph_nodes (id, kb_id, node_type, label, path, aliases, tags, summary, source_count, in_degree, out_degree, status, source_id, page_id, confidence, created_at, metadata)
                     VALUES (?1, ?2, 'source', ?3, ?4, '', '', '', 0, 0, 0, 'active', ?5, '', 'medium', '', '{}')",
                    rusqlite::params![node_id, kb_id, file_name, file_path, _id],
                ).map_err(|e| format!("?? source ????: {}", e))?;
            }
        }

        Ok(())
    }

    /// Deprecated: use sync_from_wiki_pages instead. Backward compatibility wrapper.
    pub fn sync_from_knowledge_items(db: &Arc<DatabaseService>, kb_id: &str) -> Result<(), String> {
        Self::sync_from_wiki_pages(db, kb_id)
    }

    pub fn derive_relationships(db: &Arc<DatabaseService>, kb_id: &str) -> Result<usize, String> {
        let conn = db.connect()?;
        let mut created = 0usize;

        // 查询所有已关联到 wiki 页面的知识项
        let mut stmt = conn.prepare(
            "SELECT ki.id, ki.canonical_name, ki.page_path, wp.id as wp_id, wp.title
             FROM knowledge_items ki
             JOIN wiki_pages wp ON wp.kb_id = ki.kb_id AND wp.path = ki.page_path
             WHERE ki.kb_id = ?1 AND ki.page_path != ''"
        ).map_err(|e| format!("查询知识项-页面关联失败: {}", e))?;

        let links: Vec<(String, String, String, String, String)> = stmt
            .query_map(rusqlite::params![kb_id], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?))
            }).map_err(|e| format!("映射知识项-页面关联失败: {}", e))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("读取知识项-页面关联失败: {}", e))?;

        for (ki_id, canonical_name, _page_path, wp_id, wp_title) in &links {
            // 检查是否已存在该关系（去重）
            let existing: i64 = match conn.query_row(
                "SELECT COUNT(*) FROM relationships WHERE kb_id = ?1 AND source_item_id = ?2 AND target_item_id = ?3 AND relation = 'references'",
                rusqlite::params![kb_id, ki_id, wp_id],
                |row| row.get(0),
            ) {
                Ok(c) => c,
                Err(rusqlite::Error::QueryReturnedNoRows) => 0,
                Err(e) => {
                    log::error!("[derive_relationships] 检查关系存在失败 (ki={}→wp={}): {}", ki_id, wp_id, e);
                    continue;
                }
            };
            if existing > 0 { continue; }

            let rel_id = uuid::Uuid::new_v4().to_string();
            let now = chrono::Utc::now().to_rfc3339();
            if let Err(e) = conn.execute(
                "INSERT INTO relationships (id, kb_id, source_item_id, target_item_id, relation, evidence_source_id, confidence, status, created_at)
                 VALUES (?1, ?2, ?3, ?4, 'references', '', 'high', 'active', ?5)",
                rusqlite::params![rel_id, kb_id, ki_id, wp_id, now],
            ) {
                log::error!("[derive_relationships] 插入关系失败 ({}→{}): {}", canonical_name, wp_title, e);
                continue;
            }
            created += 1;
        }

        // 同一 source 的知识项之间创建 "same_source" 关系
        let mut src_stmt = conn.prepare(
            "SELECT ki1.id, ki2.id, ki1.canonical_name, ki2.canonical_name, s.file_name
             FROM knowledge_items ki1
             JOIN knowledge_items ki2 ON ki2.kb_id = ki1.kb_id AND ki2.source_id = ki1.source_id AND ki2.id > ki1.id
             JOIN sources s ON s.id = ki1.source_id
             WHERE ki1.kb_id = ?1 AND ki1.source_id != ''"
        ).map_err(|e| format!("准备同源关系查询失败: {}", e))?;

        let co_links: Vec<(String, String, String, String, String)> = src_stmt
            .query_map(rusqlite::params![kb_id], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?))
            }).map_err(|e| format!("映射同源关系失败: {}", e))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("读取同源关系失败: {}", e))?;

        for (ki1_id, ki2_id, name1, name2, src_name) in &co_links {
            let existing: i64 = match conn.query_row(
                "SELECT COUNT(*) FROM relationships WHERE kb_id = ?1 AND ((source_item_id = ?2 AND target_item_id = ?3) OR (source_item_id = ?3 AND target_item_id = ?2)) AND relation = 'same_source'",
                rusqlite::params![kb_id, ki1_id, ki2_id],
                |row| row.get(0),
            ) {
                Ok(c) => c,
                Err(rusqlite::Error::QueryReturnedNoRows) => 0,
                Err(e) => {
                    log::error!("[derive_relationships] 检查同源关系存在失败: {}", e);
                    continue;
                }
            };
            if existing > 0 { continue; }

            let rel_id = uuid::Uuid::new_v4().to_string();
            let now = chrono::Utc::now().to_rfc3339();
            if let Err(e) = conn.execute(
                "INSERT INTO relationships (id, kb_id, source_item_id, target_item_id, relation, evidence_source_id, confidence, status, created_at)
                 VALUES (?1, ?2, ?3, ?4, 'same_source', ?5, 'medium', 'active', ?6)",
                rusqlite::params![rel_id, kb_id, ki1_id, ki2_id, src_name, now],
            ) {
                log::error!("[derive_relationships] 插入同源关系失败 ({}↔{}): {}", name1, name2, e);
                continue;
            }
            created += 1;
        }

        Ok(created)
    }

    pub fn add_or_update_node(
        db: &Arc<DatabaseService>, kb_id: &str, node_type: &str, label: &str, path: &str,
    ) -> Result<String, String> {
        let conn = db.connect()?;
        let existing_id: Option<String> = match conn.query_row(
            "SELECT id FROM graph_nodes WHERE kb_id = ?1 AND label = ?2 AND node_type = ?3",
            rusqlite::params![kb_id, label, node_type],
            |row| row.get(0),
        ) {
            Ok(id) => Some(id),
            Err(rusqlite::Error::QueryReturnedNoRows) => None,
            Err(e) => return Err(format!("查询图谱节点失败 (label={}, type={}): {}", label, node_type, e)),
        };

        if let Some(id) = existing_id {
            conn.execute("UPDATE graph_nodes SET path = ?1 WHERE id = ?2", rusqlite::params![path, id])
                .map_err(|e| format!("更新节点失败: {}", e))?;
            return Ok(id);
        }
        let node_id = uuid::Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO graph_nodes (id, kb_id, node_type, label, path) VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![node_id, kb_id, node_type, label, path],
        ).map_err(|e| format!("插入节点失败: {}", e))?;
        Ok(node_id)
    }

    pub fn add_relationship(
        db: &Arc<DatabaseService>, kb_id: &str,
        source_label: &str, target_label: &str, relation: &str,
    ) -> Result<(), String> {
        let conn = db.connect()?;
        let source_id: Option<String> = match conn.query_row(
            "SELECT id FROM graph_nodes WHERE kb_id = ?1 AND label = ?2", rusqlite::params![kb_id, source_label], |row| row.get(0),
        ) {
            Ok(id) => Some(id),
            Err(rusqlite::Error::QueryReturnedNoRows) => None,
            Err(e) => return Err(format!("查询 source 图谱节点失败 (label={}): {}", source_label, e)),
        };
        let target_id: Option<String> = match conn.query_row(
            "SELECT id FROM graph_nodes WHERE kb_id = ?1 AND label = ?2", rusqlite::params![kb_id, target_label], |row| row.get(0),
        ) {
            Ok(id) => Some(id),
            Err(rusqlite::Error::QueryReturnedNoRows) => None,
            Err(e) => return Err(format!("查询 target 图谱节点失败 (label={}): {}", target_label, e)),
        };
        if let (Some(s), Some(t)) = (source_id, target_id) {
            let edge_id = uuid::Uuid::new_v4().to_string();
            conn.execute(
                "INSERT INTO graph_edges (id, kb_id, source_node_id, target_node_id, edge_type, metadata) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                rusqlite::params![edge_id, kb_id, s, t, relation, "{}"],
            ).map_err(|e| format!("插入边失败: {}", e))?;
        }
        Ok(())
    }
}
