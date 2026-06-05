// DedupService - 知识去重与消歧服务 (v0.2.1)
// 实现: 名称规范化、相似度匹配、重复检测、消歧建议

use std::sync::Arc;
use crate::core::database_service::DatabaseService;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DedupMatch {
    pub matched_page_id: String,
    pub matched_title: String,
    pub matched_path: String,
    pub matched_canonical: String,
    pub similarity: f64,
    pub match_type: String, // exact, normalized, alias, slug_base, chinese_contain, token_set, edit_distance
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DedupResult {
    pub normalized_name: String,
    pub matches: Vec<DedupMatch>,
    pub best_match: Option<DedupMatch>,
    pub suggested_operation: String, // create_page, update_page, merge_suggestion
    pub is_duplicate: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DuplicatePageGroup {
    pub canonical_name: String,
    pub normalized_name: String,
    pub page_ids: Vec<String>,
    pub page_titles: Vec<String>,
    pub page_paths: Vec<String>,
    pub match_type: String,
}

pub struct DedupService;

impl DedupService {
    /// 规范化名称 (去除空格、全角半角统一、英文小写、去除多余连字符)
    pub fn normalize_name(name: &str) -> String {
        let mut s = name.trim().to_string();
        // 全角半角统一
        s = s.chars().map(|c| {
            match c {
                'Ａ'..='Ｚ' => ((c as u32 - 'Ａ' as u32 + 'A' as u32) as u8) as char,
                'ａ'..='ｚ' => ((c as u32 - 'ａ' as u32 + 'a' as u32) as u8) as char,
                '０'..='９' => ((c as u32 - '０' as u32 + '0' as u32) as u8) as char,
                '　' => ' ',
                '（' => '(',
                '）' => ')',
                _ => c,
            }
        }).collect();
        // 英文小写
        s = s.to_lowercase();
        // 去除多余空格、下划线、连字符差异 -> 统一为单个空格
        s = s.replace(['_', '-'], " ");
        // 压缩多个空格
        let words: Vec<&str> = s.split_whitespace().collect();
        s = words.join(" ");
        s
    }

    /// 从标题生成 slug base（去除 hash 后缀）
    pub fn slug_base_from_name(name: &str) -> String {
        Self::strip_hash_suffix(name)
    }
}

// 静态方法需要 regex_lite crate，使用简单实现代替
impl DedupService {
    /// 去除 slug 中的 hash 后缀
    fn strip_hash_suffix(slug: &str) -> String {
        let lower = slug.to_lowercase();
        // 查找最后一个 - 后跟6+个十六进制字符的模式
        let chars: Vec<char> = lower.chars().collect();
        let mut best_cut = chars.len();
        for i in (0..chars.len()).rev() {
            if chars[i] == '-' && chars.len() - i > 6 {
                let suffix: String = chars[i+1..].iter().collect();
                if suffix.chars().all(|c| c.is_ascii_hexdigit()) {
                    best_cut = i;
                    break;
                }
            }
        }
        if best_cut < chars.len() {
            chars[..best_cut].iter().collect::<String>().replace('-', " ")
        } else {
            lower.replace('-', " ")
        }
    }

    /// 计算两个字符串的编辑距离（Levenshtein）
    fn edit_distance(a: &str, b: &str) -> usize {
        let a_chars: Vec<char> = a.chars().collect();
        let b_chars: Vec<char> = b.chars().collect();
        let n = a_chars.len();
        let m = b_chars.len();
        let mut dp = vec![vec![0usize; m + 1]; n + 1];
        for i in 0..=n { dp[i][0] = i; }
        for j in 0..=m { dp[0][j] = j; }
        for i in 1..=n {
            for j in 1..=m {
                let cost = if a_chars[i-1] == b_chars[j-1] { 0 } else { 1 };
                dp[i][j] = (dp[i-1][j] + 1).min(dp[i][j-1] + 1).min(dp[i-1][j-1] + cost);
            }
        }
        dp[n][m]
    }

    /// 编辑距离相似度
    fn edit_similarity(a: &str, b: &str) -> f64 {
        let max_len = a.len().max(b.len()).max(1) as f64;
        let dist = Self::edit_distance(a, b) as f64;
        1.0 - (dist / max_len)
    }

    /// Token set 相似度（英文）
    fn token_set_similarity(a: &str, b: &str) -> f64 {
        let tokens_a: std::collections::HashSet<&str> = a.split_whitespace().collect();
        let tokens_b: std::collections::HashSet<&str> = b.split_whitespace().collect();
        if tokens_a.is_empty() || tokens_b.is_empty() { return 0.0; }
        let intersection = tokens_a.intersection(&tokens_b).count() as f64;
        let union = tokens_a.union(&tokens_b).count() as f64;
        intersection / union
    }

    /// 中文包含相似度
    fn chinese_contain_similarity(a: &str, b: &str) -> f64 {
        if a.is_empty() || b.is_empty() { return 0.0; }
        if a.contains(b) || b.contains(a) { return 0.85; }
        // 检查字符级重叠
        let chars_a: std::collections::HashSet<char> = a.chars().filter(|c| !c.is_whitespace()).collect();
        let chars_b: std::collections::HashSet<char> = b.chars().filter(|c| !c.is_whitespace()).collect();
        if chars_a.is_empty() || chars_b.is_empty() { return 0.0; }
        let overlap = chars_a.intersection(&chars_b).count() as f64;
        let min_len = chars_a.len().min(chars_b.len()) as f64;
        if min_len == 0.0 { return 0.0; }
        (overlap / min_len).min(0.85)
    }

    /// 综合相似度计算
    pub fn compute_similarity(candidate_name: &str, existing_name: &str, existing_canonical: &str, existing_aliases: &[String]) -> Vec<(f64, String)> {
        let mut scores: Vec<(f64, String)> = Vec::new();
        let cand_norm = Self::normalize_name(candidate_name);

        // 1. exact match
        if candidate_name == existing_name {
            scores.push((1.0, "exact".into()));
        }

        // 2. normalized exact
        let exist_norm = Self::normalize_name(existing_name);
        if cand_norm == exist_norm {
            scores.push((0.95, "normalized".into()));
        }

        // 3. 与 canonical_name 匹配
        let canon_norm = Self::normalize_name(existing_canonical);
        if cand_norm == canon_norm {
            scores.push((0.95, "normalized_canonical".into()));
        }

        // 4. alias match
        for alias in existing_aliases {
            let alias_norm = Self::normalize_name(alias);
            if cand_norm == alias_norm {
                scores.push((0.92, "alias".into()));
                break;
            }
        }

        // 5. slug base match
        let cand_slug = Self::strip_hash_suffix(candidate_name);
        let exist_slug = Self::strip_hash_suffix(existing_name);
        if cand_slug == exist_slug && !cand_slug.is_empty() {
            scores.push((0.9, "slug_base".into()));
        }

        // 6. 中文完全包含
        let cn_sim = Self::chinese_contain_similarity(&cand_norm, &exist_norm);
        if cn_sim >= 0.85 {
            scores.push((cn_sim, "chinese_contain".into()));
        }

        // 7. token set 相似
        let ts_sim = Self::token_set_similarity(&cand_norm, &exist_norm);
        if ts_sim >= 0.8 {
            scores.push((ts_sim, "token_set".into()));
        }

        // 8. 编辑距离
        let ed_sim = Self::edit_similarity(&cand_norm, &exist_norm);
        if ed_sim >= 0.75 {
            scores.push((ed_sim, "edit_distance".into()));
        }

        scores.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        scores
    }

    /// 在已有页面中查找重复/相似项
    pub fn find_duplicates(
        db: &Arc<DatabaseService>,
        kb_id: &str,
        candidate_name: &str,
    ) -> Result<DedupResult, String> {
        let conn = db.connect()?;
        let mut result = DedupResult {
            normalized_name: Self::normalize_name(candidate_name),
            matches: Vec::new(),
            best_match: None,
            suggested_operation: "create_page".into(),
            is_duplicate: false,
        };

        // 快速精确匹配检查
        let exact_match = match conn.query_row(
            "SELECT wp.id, wp.title, wp.path, wp.canonical_name FROM wiki_pages wp WHERE wp.kb_id = ?1 AND (wp.title = ?2 OR wp.canonical_name = ?3) LIMIT 1",
            rusqlite::params![kb_id, candidate_name, candidate_name],
            |row| Ok((row.get::<_,String>(0)?, row.get::<_,String>(1)?, row.get::<_,String>(2)?, row.get::<_,String>(3)?)),
        ) {
            Ok(v) => Some(v),
            Err(rusqlite::Error::QueryReturnedNoRows) => None,
            Err(e) => return Err(format!("去重精确匹配查询失败: {}", e)),
        };

        if let Some((id, title, path, canonical)) = exact_match {
            result.matches.push(DedupMatch {
                matched_page_id: id, matched_title: title,
                matched_path: path, matched_canonical: canonical,
                similarity: 1.0, match_type: "exact".into(),
            });
            result.best_match = result.matches.last().cloned();
            result.suggested_operation = "update_page".into();
            result.is_duplicate = true;
            return Ok(result);
        }

        // 检查 aliases
        let cand_norm = &result.normalized_name;
        let alias_match = match conn.query_row(
            "SELECT wp.id, wp.title, wp.path, wp.canonical_name FROM wiki_pages wp
             JOIN knowledge_items ki ON ki.page_id = wp.id
             JOIN aliases a ON a.item_id = ki.id
             WHERE wp.kb_id = ?1 AND a.normalized_alias = ?2 LIMIT 1",
            rusqlite::params![kb_id, cand_norm],
            |row| Ok((row.get::<_,String>(0)?, row.get::<_,String>(1)?, row.get::<_,String>(2)?, row.get::<_,String>(3)?)),
        ) {
            Ok(v) => Some(v),
            Err(rusqlite::Error::QueryReturnedNoRows) => None,
            Err(e) => return Err(format!("去重别名匹配查询失败: {}", e)),
        };

        if let Some((id, title, path, canonical)) = alias_match {
            result.matches.push(DedupMatch {
                matched_page_id: id, matched_title: title,
                matched_path: path, matched_canonical: canonical,
                similarity: 0.92, match_type: "alias".into(),
            });
        }

        // 获取所有已有页面进行相似度比较
        let mut stmt = conn.prepare(
            "SELECT wp.id, wp.title, wp.path, wp.canonical_name FROM wiki_pages wp WHERE wp.kb_id = ?1"
        ).map_err(|e| format!("查询已有页面失败: {}", e))?;

        let existing_pages: Vec<(String, String, String, String)> = stmt
            .query_map(rusqlite::params![kb_id], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
            })
            .map_err(|e| format!("映射页面失败: {}", e))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("读取已有页面行失败: {}", e))?;

        for (pid, ptitle, ppath, pcanonical) in &existing_pages {
            // 获取别名
            let mut alias_stmt = conn.prepare(
                "SELECT a.alias FROM aliases a JOIN knowledge_items ki ON a.item_id = ki.id WHERE ki.page_id = ?1"
            ).map_err(|e| format!("准备别名查询失败: {}", e))?;
            let mut aliases: Vec<String> = Vec::new();
            if let Ok(alias_rows) = alias_stmt.query_map(rusqlite::params![pid], |row| row.get::<_,String>(0)) {
                aliases = alias_rows.collect::<Result<Vec<_>, _>>().unwrap_or_else(|e| {
                    log::error!("[dedup] 读取别名行失败 (pid={}): {}", pid, e);
                    Vec::new()
                });
            }

            let scores = Self::compute_similarity(candidate_name, ptitle, pcanonical, &aliases);
            for (sim, match_type) in &scores {
                let exists = result.matches.iter().any(|m| m.matched_page_id == *pid && m.match_type == *match_type);
                if !exists {
                    result.matches.push(DedupMatch {
                        matched_page_id: pid.clone(),
                        matched_title: ptitle.clone(),
                        matched_path: ppath.clone(),
                        matched_canonical: pcanonical.clone(),
                        similarity: *sim,
                        match_type: match_type.clone(),
                    });
                }
            }
        }

        // 排序匹配项
        result.matches.sort_by(|a, b| b.similarity.partial_cmp(&a.similarity).unwrap_or(std::cmp::Ordering::Equal));

        // 确定最佳操作
        if let Some(best) = result.matches.first() {
            result.best_match = Some(best.clone());
            if best.similarity >= 0.9 {
                result.suggested_operation = "update_page".into();
                result.is_duplicate = true;
            } else if best.similarity >= 0.75 {
                result.suggested_operation = "merge_suggestion".into();
                result.is_duplicate = true;
            }
        }

        Ok(result)
    }

    /// 检测知识库中的重复页面组
    pub fn detect_duplicate_pages(
        db: &Arc<DatabaseService>,
        kb_id: &str,
    ) -> Result<Vec<DuplicatePageGroup>, String> {
        let conn = db.connect()?;
        let mut groups: Vec<DuplicatePageGroup> = Vec::new();

        let mut stmt = conn.prepare(
            "SELECT id, title, canonical_name, path FROM wiki_pages WHERE kb_id = ?1 ORDER BY title"
        ).map_err(|e| format!("查询页面失败: {}", e))?;

        let pages: Vec<(String, String, String, String)> = stmt
            .query_map(rusqlite::params![kb_id], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
            })
            .map_err(|e| format!("映射页面失败: {}", e))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("读取页面行失败: {}", e))?;

        let mut processed = std::collections::HashSet::new();
        for i in 0..pages.len() {
            if processed.contains(&pages[i].0) { continue; }
            let (ref pid, ref ptitle, ref pcanonical, ref ppath) = pages[i];
            let norm1 = Self::normalize_name(ptitle);

            let mut group_ids = vec![pid.clone()];
            let mut group_titles = vec![ptitle.clone()];
            let mut group_paths = vec![ppath.clone()];
            let mut match_type = String::new();

            for j in (i+1)..pages.len() {
                if processed.contains(&pages[j].0) { continue; }
                let (ref pid2, ref ptitle2, ref _pcanonical2, ref ppath2) = pages[j];
                let norm2 = Self::normalize_name(ptitle2);

                // 标题完全相同
                if ptitle == ptitle2 {
                    if match_type.is_empty() { match_type = "exact_title".into(); }
                    group_ids.push(pid2.clone());
                    group_titles.push(ptitle2.clone());
                    group_paths.push(ppath2.clone());
                    processed.insert(pid2.clone());
                    continue;
                }

                // normalized 相同
                if norm1 == norm2 && !norm1.is_empty() {
                    if match_type.is_empty() { match_type = "normalized".into(); }
                    group_ids.push(pid2.clone());
                    group_titles.push(ptitle2.clone());
                    group_paths.push(ppath2.clone());
                    processed.insert(pid2.clone());
                    continue;
                }

                // slug base 相同
                let slug1 = Self::strip_hash_suffix(ptitle);
                let slug2 = Self::strip_hash_suffix(ptitle2);
                if slug1 == slug2 && slug1.len() > 3 {
                    if match_type.is_empty() { match_type = "slug_base".into(); }
                    group_ids.push(pid2.clone());
                    group_titles.push(ptitle2.clone());
                    group_paths.push(ppath2.clone());
                    processed.insert(pid2.clone());
                    continue;
                }

                // canonical_name 相同
                if pcanonical == &pages[j].2 && !pcanonical.is_empty() {
                    if match_type.is_empty() { match_type = "canonical".into(); }
                    group_ids.push(pid2.clone());
                    group_titles.push(ptitle2.clone());
                    group_paths.push(ppath2.clone());
                    processed.insert(pid2.clone());
                    continue;
                }
            }

            if group_ids.len() > 1 {
                processed.insert(pid.clone());
                groups.push(DuplicatePageGroup {
                    canonical_name: pcanonical.clone(),
                    normalized_name: norm1,
                    page_ids: group_ids,
                    page_titles: group_titles,
                    page_paths: group_paths,
                    match_type,
                });
            }
        }

        Ok(groups)
    }

    /// 在一次 update_plan 内存中去重
    pub fn dedup_update_plans(plans: &[serde_json::Value]) -> Vec<serde_json::Value> {
        let mut seen_names = std::collections::HashMap::new();
        let mut merged_plans: Vec<serde_json::Value> = Vec::new();

        for plan in plans {
            let title = plan.get("title")
                .or_else(|| plan.get("canonical_name"))
                .and_then(|t| t.as_str())
                .unwrap_or("");
            let norm = Self::normalize_name(title);
            if norm.is_empty() { continue; }

            if let Some(idx) = seen_names.get(&norm) {
                // 合并 evidence 和 key points 到已有项
                let existing: &mut serde_json::Value = &mut merged_plans[*idx];
                if let Some(plan_evidence) = plan.get("evidence") {
                    if let Some(existing_evidence) = existing.get_mut("evidence") {
                        if let (Some(arr1), Some(arr2)) = (existing_evidence.as_array(), plan_evidence.as_array()) {
                            let mut merged = arr1.clone();
                            merged.extend(arr2.clone());
                            *existing_evidence = serde_json::Value::Array(merged);
                        }
                    } else {
                        if let Some(obj) = existing.as_object_mut() {
                            obj.insert("evidence".to_string(), plan.get("evidence").cloned().unwrap_or_default());
                        }
                    }
                }
            } else {
                seen_names.insert(norm, merged_plans.len());
                merged_plans.push(plan.clone());
            }
        }

        merged_plans
    }

    /// v0.2.2: 创建页面前的快速查重，返回建议操作
    /// 返回 (建议操作, 匹配页面ID, 匹配标题, 相似度)
    /// 建议操作: "create" | "update" | "merge_suggestion"
    pub fn check_before_create(
        db: &Arc<DatabaseService>,
        kb_id: &str,
        title: &str,
        canonical_name: &str,
    ) -> Result<(String, Option<String>, Option<String>, f64), String> {
        let title_result = Self::find_duplicates(db, kb_id, title)?;
        if title_result.is_duplicate {
            let best = title_result.best_match.as_ref();
            if let Some(best) = best {
                if best.similarity >= 0.9 {
                    return Ok(("update".into(), Some(best.matched_page_id.clone()), Some(best.matched_title.clone()), best.similarity));
                } else {
                    return Ok(("merge_suggestion".into(), Some(best.matched_page_id.clone()), Some(best.matched_title.clone()), best.similarity));
                }
            }
        }

        let title_norm = Self::normalize_name(title);
        let canon_norm = Self::normalize_name(canonical_name);
        if title_norm != canon_norm {
            let canon_result = Self::find_duplicates(db, kb_id, canonical_name)?;
            if canon_result.is_duplicate {
                let best = canon_result.best_match.as_ref();
                if let Some(best) = best {
                    if best.similarity >= 0.9 {
                        return Ok(("update".into(), Some(best.matched_page_id.clone()), Some(best.matched_title.clone()), best.similarity));
                    } else {
                        return Ok(("merge_suggestion".into(), Some(best.matched_page_id.clone()), Some(best.matched_title.clone()), best.similarity));
                    }
                }
            }
        }

        Ok(("create".into(), None, None, 0.0))
    }

    /// v0.2.2: 快速检查 canonical_name 是否已存在 wiki_pages
    pub fn canonical_exists(db: &Arc<DatabaseService>, kb_id: &str, canonical_name: &str) -> Result<Option<(String, String)>, String> {
        let conn = db.connect()?;
        let result = match conn.query_row(
            "SELECT id, title FROM wiki_pages WHERE kb_id = ?1 AND (canonical_name = ?2 OR title = ?3) LIMIT 1",
            rusqlite::params![kb_id, canonical_name, canonical_name],
            |row| Ok((row.get::<_,String>(0)?, row.get::<_,String>(1)?)),
        ) {
            Ok(r) => Some(r),
            Err(rusqlite::Error::QueryReturnedNoRows) => None,
            Err(e) => return Err(format!("查询 canonical 是否存在失败: {}", e)),
        };
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_name() {
        assert_eq!(DedupService::normalize_name("刘汶林"), "刘汶林");
        assert_eq!(DedupService::normalize_name(" 刘汶林 "), "刘汶林");
        assert_eq!(DedupService::normalize_name("liu wen lin"), "liu wen lin");
        assert_eq!(DedupService::normalize_name("liu-wen-lin"), "liu wen lin");
        assert_eq!(DedupService::normalize_name("LIU_WEN_LIN"), "liu wen lin");
        assert_eq!(DedupService::normalize_name("　刘汶林　"), "刘汶林"); // 全角空格
        assert_eq!(DedupService::normalize_name("rPPG"), "rppg");
    }

    #[test]
    fn test_strip_hash_suffix() {
        let base = DedupService::strip_hash_suffix("liu-wen-lin-63f45abc");
        assert_eq!(base, "liu wen lin");
        let base2 = DedupService::strip_hash_suffix("liu-wen-lin");
        assert_eq!(base2, "liu wen lin");
    }

    #[test]
    fn test_edit_similarity() {
        let sim = DedupService::edit_similarity("刘汶林", "刘文林");
        assert!(sim > 0.5);
    }
}
