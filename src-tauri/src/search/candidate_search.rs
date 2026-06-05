// CandidateSearchEngine - 候选页面检索（v0.2.2: 集成 DedupService 查重）
// 从 ingest result 中提取 name，在现有知识项和 wiki_pages 中搜索匹配

use std::sync::Arc;
use crate::core::database_service::DatabaseService;
use crate::dedup::dedup_service::DedupService;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CandidateResult {
    pub input_name: String,
    pub input_type: String,
    pub candidates: Vec<CandidateMatch>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dedup_result: Option<DedupMatchInfo>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CandidateMatch {
    pub item_id: String,
    pub canonical_name: String,
    pub item_type: String,
    pub page_path: String,
    pub similarity: f64,
    pub match_type: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DedupMatchInfo {
    pub is_duplicate: bool,
    pub suggested_operation: String,
    pub best_match_title: Option<String>,
    pub best_match_similarity: Option<f64>,
    pub dedup_reason: String,
}

pub struct CandidateSearchEngine;

impl CandidateSearchEngine {
    /// 对 ingest result 中的每个实体/概念/主题执行候选检索（v0.2.2: 集成 DedupService）
    pub fn search(
        db: &Arc<DatabaseService>,
        kb_id: &str,
        ingest_result_json: &str,
        _kb_path: &str,
    ) -> Result<Vec<CandidateResult>, String> {
        let json: serde_json::Value = serde_json::from_str(ingest_result_json)
            .map_err(|e| format!("解析 ingest result 失败: {}", e))?;

        let mut results = Vec::new();

        let mut names_to_check: Vec<(String, String)> = Vec::new();

        if let Some(arr) = json.get("entities").and_then(|a| a.as_array()) {
            for item in arr {
                if let Some(name) = item.get("name").and_then(|n| n.as_str()) {
                    names_to_check.push((name.to_string(), "entity".to_string()));
                }
            }
        }

        if let Some(arr) = json.get("concepts").and_then(|a| a.as_array()) {
            for item in arr {
                if let Some(name) = item.get("name").and_then(|n| n.as_str()) {
                    names_to_check.push((name.to_string(), "concept".to_string()));
                }
            }
        }

        if let Some(arr) = json.get("topics").and_then(|a| a.as_array()) {
            for item in arr {
                if let Some(name) = item.get("name").and_then(|n| n.as_str()) {
                    names_to_check.push((name.to_string(), "topic".to_string()));
                }
            }
        }

        for (name, ntype) in &names_to_check {
            let candidates = Self::search_name(db, kb_id, name)?;

            // v0.2.2: 集成 DedupService 查重
            let dedup_info = match DedupService::find_duplicates(db, kb_id, name) {
                Ok(dedup_result) => {
                    let dedup_reason = if dedup_result.suggested_operation == "update_page" {
                        format!("与已有页面「{}」高度相似({:.0}%)，建议更新而非创建",
                            dedup_result.best_match.as_ref().map(|m| m.matched_title.as_str()).unwrap_or("?"),
                            dedup_result.best_match.as_ref().map(|m| m.similarity * 100.0).unwrap_or(0.0))
                    } else if dedup_result.suggested_operation == "merge_suggestion" {
                        format!("疑似与已有页面重复({:.0}%)，建议人工审核合并",
                            dedup_result.best_match.as_ref().map(|m| m.similarity * 100.0).unwrap_or(0.0))
                    } else {
                        "未发现重复".to_string()
                    };
                    Some(DedupMatchInfo {
                        is_duplicate: dedup_result.is_duplicate,
                        suggested_operation: dedup_result.suggested_operation.clone(),
                        best_match_title: dedup_result.best_match.as_ref().map(|m| m.matched_title.clone()),
                        best_match_similarity: dedup_result.best_match.as_ref().map(|m| m.similarity),
                        dedup_reason,
                    })
                }
                Err(e) => {
                    log::error!("[CandidateSearch] 去重检查失败(name={}): {}", name, e);
                    None
                }
            };

            results.push(CandidateResult {
                input_name: name.clone(),
                input_type: ntype.clone(),
                candidates,
                dedup_result: dedup_info,
            });
        }

        Ok(results)
    }

    /// v0.2.2: 搜索 knowledge_items 和 wiki_pages
    fn search_name(
        db: &Arc<DatabaseService>,
        kb_id: &str,
        name: &str,
    ) -> Result<Vec<CandidateMatch>, String> {
        let conn = db.connect()?;

        let mut matches = Vec::new();

        // 1. 精确匹配 knowledge_items.canonical_name
        {
            let mut stmt = conn
                .prepare("SELECT id, canonical_name, item_type, COALESCE(page_path,'') FROM knowledge_items WHERE kb_id = ?1 AND canonical_name = ?2")
                .map_err(|e| format!("查询 knowledge_items 失败: {}", e))?;
            let rows = stmt
                .query_map(rusqlite::params![kb_id, name], |row| {
                    Ok((row.get::<_,String>(0)?, row.get::<_,String>(1)?, row.get::<_,String>(2)?, row.get::<_,String>(3)?))
                })
                .map_err(|e| format!("映射 knowledge_items 失败: {}", e))?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| format!("收集 knowledge_items 失败: {}", e))?;
            for (id, cn, itype, path) in &rows {
                matches.push(CandidateMatch {
                    item_id: id.clone(),
                    canonical_name: cn.clone(),
                    item_type: itype.clone(),
                    page_path: path.clone(),
                    similarity: 1.0,
                    match_type: "exact_ki".into(),
                });
            }
        }

        // 2. 精确匹配 wiki_pages.title 或 canonical_name
        {
            let mut stmt = conn
                .prepare("SELECT id, title, canonical_name, page_type, path FROM wiki_pages WHERE kb_id = ?1 AND (title = ?2 OR canonical_name = ?3)")
                .map_err(|e| format!("查询 wiki_pages 失败: {}", e))?;
            let rows = stmt
                .query_map(rusqlite::params![kb_id, name, name], |row| {
                    Ok((row.get::<_,String>(0)?, row.get::<_,String>(1)?, row.get::<_,String>(2)?, row.get::<_,String>(3)?, row.get::<_,String>(4)?))
                })
                .map_err(|e| format!("映射 wiki_pages 失败: {}", e))?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| format!("收集 wiki_pages 失败: {}", e))?;
            for (id, _title, canonical, ptype, path) in &rows {
                matches.push(CandidateMatch {
                    item_id: id.clone(),
                    canonical_name: canonical.clone(),
                    item_type: ptype.clone(),
                    page_path: path.clone(),
                    similarity: 1.0,
                    match_type: "exact_wp".into(),
                });
            }
        }

        // 3. 模糊搜索所有 knowledge_items（相似度 > 0.6）
        let mut ki_stmt = conn
            .prepare("SELECT id, canonical_name, item_type, COALESCE(page_path,'') FROM knowledge_items WHERE kb_id = ?1")
            .map_err(|e| format!("查询 knowledge_items 失败: {}", e))?;

        let all_ki: Vec<(String, String, String, String)> = ki_stmt
            .query_map(rusqlite::params![kb_id], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
            })
            .map_err(|e| format!("映射 knowledge_items 失败: {}", e))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("收集 knowledge_items 失败: {}", e))?;

        let normalized_name = name.to_lowercase();

        for (id, cn, itype, path) in &all_ki {
            let normalized_cn = cn.to_lowercase();
            let sim = strsim::normalized_damerau_levenshtein(&normalized_name, &normalized_cn);
            if sim > 0.6 && sim < 1.0
                && !matches.iter().any(|m| m.item_id == *id) {
                    matches.push(CandidateMatch {
                        item_id: id.clone(),
                        canonical_name: cn.clone(),
                        item_type: itype.clone(),
                        page_path: path.clone(),
                        similarity: sim,
                        match_type: "fuzzy_ki".into(),
                    });
                }
        }

        // 4. 模糊搜索 wiki_pages
        let mut wp_stmt = conn
            .prepare("SELECT id, title, canonical_name, page_type, path FROM wiki_pages WHERE kb_id = ?1")
            .map_err(|e| format!("查询 wiki_pages 失败: {}", e))?;

        let all_wp: Vec<(String, String, String, String, String)> = wp_stmt
            .query_map(rusqlite::params![kb_id], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?))
            })
            .map_err(|e| format!("映射 wiki_pages 失败: {}", e))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("收集 wiki_pages 失败: {}", e))?;

        for (id, title, canonical, ptype, path) in &all_wp {
            // 检查 title
            let title_sim = strsim::normalized_damerau_levenshtein(&normalized_name, &title.to_lowercase());
            if title_sim > 0.6 && title_sim < 1.0
                && !matches.iter().any(|m| m.item_id == *id) {
                    matches.push(CandidateMatch {
                        item_id: id.clone(),
                        canonical_name: canonical.clone(),
                        item_type: ptype.clone(),
                        page_path: path.clone(),
                        similarity: title_sim,
                        match_type: "fuzzy_wp_title".into(),
                    });
                }

            // 检查 canonical_name
            let canon_sim = strsim::normalized_damerau_levenshtein(&normalized_name, &canonical.to_lowercase());
            if canon_sim > 0.6 && canon_sim < 1.0
                && !matches.iter().any(|m| m.item_id == *id) {
                    matches.push(CandidateMatch {
                        item_id: id.clone(),
                        canonical_name: canonical.clone(),
                        item_type: ptype.clone(),
                        page_path: path.clone(),
                        similarity: canon_sim,
                        match_type: "fuzzy_wp_canon".into(),
                    });
                }
        }

        matches.sort_by(|a, b| b.similarity.partial_cmp(&a.similarity).unwrap_or(std::cmp::Ordering::Equal));
        matches.truncate(10);

        Ok(matches)
    }
}
