// ReviewEngine - 从 Wiki Update Plan 生成审阅项（v0.2.1 增强版）
// v0.2.1: 集成去重服务、operation_type 严格枚举、事件日志

use std::sync::Arc;
use crate::core::database_service::DatabaseService;
use crate::wiki::path_service::PathService;
use crate::dedup::dedup_service::DedupService;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ReviewItem {
    pub id: String,
    pub review_id: String,
    pub operation: String,
    pub operation_type: String,
    pub target_path: String,
    pub base_version_hash: String,
    pub old_content: String,
    pub new_content: String,
    pub status: String,
    pub risk_level: String,
    pub reason: String,
    pub source_id: String,
    pub citation_status: String,
    pub summary: String,
    pub confidence: String,
    pub created_at: String,
    pub page_type: String,
    pub title: String,
    pub apply_error: String,
    #[serde(default)]
    pub duplicate_candidate: bool,
    #[serde(default)]
    pub missing_target: bool,
    #[serde(default)]
    pub manual_required: bool,
    #[serde(default)]
    pub matched_page: Option<String>,
    #[serde(default)]
    pub matched_path: Option<String>,
    #[serde(default)]
    pub merge_candidate: bool,
    #[serde(default)]
    pub auto_converted_from_update: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Review {
    pub id: String,
    pub kb_id: String,
    pub task_id: String,
    pub status: String,
    pub summary: String,
    pub risk_level: String,
    pub created_at: String,
    pub items: Vec<ReviewItem>,
}

pub struct ReviewEngine;

impl ReviewEngine {
    /// 根据 update plan 生成审阅组（v0.2.1: 集成去重、operation_type 严格枚举）
    pub fn generate_review(
        db: &Arc<DatabaseService>,
        kb_id: &str,
        task_id: &str,
        update_plan: &serde_json::Value,
        kb_path: &str,
    ) -> Result<(), String> {
        let conn = db.connect()?;
        let now = chrono::Utc::now().to_rfc3339();

        let empty_vec = vec![];
        let plans: &[serde_json::Value] = update_plan
            .get("wiki_update_plan")
            .or_else(|| update_plan.get("updates"))
            .or_else(|| update_plan.get("wiki_updates"))
            .or_else(|| update_plan.get("update_plan"))
            .or_else(|| update_plan.get("proposed_wiki_updates"))
            .and_then(|p| p.as_array())
            .unwrap_or(&empty_vec);

        if plans.is_empty() {
            log::error!("[ReviewEngine] 警告: 未在 update_plan JSON 中找到 wiki_update_plan 或其他支持的键名，JSON 内容: {}",
                serde_json::to_string_pretty(update_plan).unwrap_or_default());
            return Err("Wiki 更新计划为空，未找到 wiki_update_plan 字段。请检查 LLM 输出格式。".to_string());
        }

        // v0.2.1: 内存中去重 plans
        let plans = DedupService::dedup_update_plans(plans);

        let review_id = uuid::Uuid::new_v4().to_string();
        let summary = format!("任务 {} 的 Wiki 更新审阅，共 {} 项", task_id, plans.len());

        let max_risk = plans.iter()
            .filter_map(|p| p.get("risk_level").and_then(|r| r.as_str()))
            .max_by_key(|r| match *r {
                "high" => 3, "medium" => 2, _ => 1,
            })
            .unwrap_or("medium");

        conn.execute(
            "INSERT INTO reviews (id, kb_id, task_id, status, summary, risk_level, created_at, updated_at)
             VALUES (?1, ?2, ?3, 'pending', ?4, ?5, ?6, ?6)",
            rusqlite::params![review_id, kb_id, task_id, summary, max_risk, now],
        )
        .map_err(|e| format!("创建审阅失败: {}", e))?;

        let workspace_root = std::path::PathBuf::from(kb_path);

        for plan in &plans {
            let llm_operation = plan.get("operation").and_then(|o| o.as_str()).unwrap_or("create");
            let title = Self::extract_best_title(plan);
            let llm_path = plan.get("path").and_then(|p| p.as_str()).unwrap_or("");
            let risk = plan.get("risk_level").and_then(|r| r.as_str()).unwrap_or("medium");
            let reason = plan.get("reason").and_then(|r| r.as_str()).unwrap_or("");
            let item_summary = plan.get("summary").and_then(|s| s.as_str()).unwrap_or("");
            let source_id = plan.get("source_id").or(plan.get("evidence_source_id")).and_then(|s| s.as_str()).unwrap_or("");
            let confidence = plan.get("confidence").and_then(|c| c.as_str()).unwrap_or("medium");
            let citation_status = plan.get("citation_status").and_then(|c| c.as_str()).unwrap_or("uncited");
            let page_type = plan.get("page_type").and_then(|p| p.as_str()).unwrap_or("concept");
            let new_content = plan.get("new_markdown").or(plan.get("content_blocks"))
                .and_then(|c| c.as_str())
                .unwrap_or("");

            let path = if !llm_path.is_empty() && PathService::is_valid_wiki_path(llm_path) {
                PathService::normalize_workspace_path(llm_path)
            } else {
                PathService::resolve_wiki_page_path(page_type, &title)
            };

            let old_content = if !path.is_empty() {
                let page_file = PathService::resolve_workspace_path(&workspace_root, &path);
                match std::fs::read_to_string(&page_file) {
                    Ok(c) => c,
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
                    Err(e) => {
                        log::error!("[review_engine] 读取现有页面文件失败 ({}): {}", path, e);
                        String::new()
                    }
                }
            } else {
                String::new()
            };

            // v0.2.1: LLM operation 到 operation_type 的严格映射
            let (operation, operation_type, final_reason) = match llm_operation {
                "create" => {
                    // 查重检查
                    if !title.is_empty() && title != "Untitled" {
                        match DedupService::find_duplicates(db, kb_id, &title) {
                            Ok(dedup_result) => {
                                if dedup_result.is_duplicate && dedup_result.suggested_operation == "update_page" {
                                    if let Some(ref best) = dedup_result.best_match {
                                        ("update", "update_page".to_string(),
                                         format!("[去重检测] 与已有页面「{}」(路径: {}, 相似度: {:.0}%) 高度相似，已自动转为更新页面", best.matched_title, best.matched_path, best.similarity * 100.0))
                                    } else {
                                        ("create", "create_page".to_string(), reason.to_string())
                                    }
                                } else if dedup_result.is_duplicate && dedup_result.suggested_operation == "merge_suggestion" {
                                    if let Some(ref best) = dedup_result.best_match {
                                        ("merge_suggestion", "merge_suggestion".to_string(),
                                         format!("[去重检测] 疑似与「{}」(路径: {}, 相似度: {:.0}%) 重复，建议人工合并", best.matched_title, best.matched_path, best.similarity * 100.0))
                                    } else {
                                        ("merge_suggestion", "merge_suggestion".to_string(), reason.to_string())
                                    }
                                } else {
                                    ("create", "create_page".to_string(), reason.to_string())
                                }
                            }
                            Err(_) => ("create", "create_page".to_string(), reason.to_string()),
                        }
                    } else {
                        ("create", "create_page".to_string(), reason.to_string())
                    }
                }
                "update" => {
                    if old_content.is_empty() {
                        ("create", "create_page".to_string(),
                         format!("[自动降级] 原计划更新页面，但目标文件不存在。原原因: {}", reason))
                    } else {
                        ("update", "update_page".to_string(), reason.to_string())
                    }
                }
                "append" => {
                    if old_content.is_empty() {
                        ("create", "create_page".to_string(),
                         format!("[自动降级] 原计划追加内容，但目标文件不存在。原原因: {}", reason))
                    } else {
                        ("append", "append_section".to_string(), reason.to_string())
                    }
                }
                "add_alias" => ("add_alias", "add_alias".to_string(), reason.to_string()),
                "add_relation" => ("add_relation", "add_relation".to_string(), reason.to_string()),
                "merge_suggestion" => ("merge_suggestion", "merge_suggestion".to_string(), reason.to_string()),
                "skip" => ("skip", "skip".to_string(), reason.to_string()),
                "delete" => {
                    if path.is_empty() {
                        ("unresolved", "unresolved".to_string(),
                         format!("[自动降级] 删除操作缺少目标路径。原原因: {}", reason))
                    } else if old_content.is_empty() {
                        ("unresolved", "unresolved".to_string(),
                         format!("[自动降级] 目标页面不存在，无法删除。原原因: {}", reason))
                    } else {
                        ("delete", "delete_page".to_string(), reason.to_string())
                    }
                }
                _ => {
                    // 低质量内容检测
                    if new_content.len() < 50 {
                        ("unresolved", "unresolved".to_string(),
                         format!("内容过短(仅{}字符)，需要人工审核。原操作: {}", new_content.len(), llm_operation))
                    } else {
                        ("create", "create_page".to_string(), format!("未知操作类型「{}」，默认转为创建页面", llm_operation))
                    }
                }
            };

            let base_hash = if old_content.is_empty() {
                String::new()
            } else {
                PathService::content_hash(&old_content)
            };

            let item_id = uuid::Uuid::new_v4().to_string();

            let item_full_new = if !new_content.starts_with("# ") && !new_content.starts_with("---") && operation_type != "add_alias" {
                format!("# {}\n\n{}", title, new_content)
            } else {
                new_content.to_string()
            };

            // v0.2.1: 写入 operation_type 列
            conn.execute(
                "INSERT INTO review_items (id, review_id, operation, operation_type, target_path, base_version_hash, old_content, new_content, status, risk_level, reason, source_id, citation_status, summary, confidence, title, page_type, apply_error, metadata_json, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 'pending', ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, '', '{}', ?17, ?17)",
                rusqlite::params![item_id, review_id, operation, operation_type, path, base_hash, old_content, item_full_new, risk, final_reason, source_id, citation_status, item_summary, confidence, title, page_type, now],
            )
            .map_err(|e| format!("创建审阅项失败: {}", e))?;
        }

        Ok(())
    }

    /// 提取最佳标题：优先 update_plan.title → content_blocks.title → 第一个 H1 → canonical_name → fallback
    fn extract_best_title(plan: &serde_json::Value) -> String {
        plan.get("title").and_then(|t| t.as_str()).filter(|s| !s.is_empty())
            .or_else(|| plan.get("canonical_name").and_then(|c| c.as_str()).filter(|s| !s.is_empty()))
            .map(|s| s.to_string())
            .or_else(|| {
                plan.get("new_markdown").or(plan.get("content_blocks"))
                    .and_then(|c| c.as_str())
                    .and_then(|s| {
                        s.lines().find(|l| l.starts_with("# "))
                            .map(|l| l.trim_start_matches("# ").trim().to_string())
                    })
            })
            .unwrap_or_else(|| {
                plan.get("path").and_then(|p| p.as_str())
                    .map(|p| p.trim_end_matches(".md").split('/').next_back().unwrap_or("Untitled").to_string())
                    .unwrap_or_else(|| "Untitled".to_string())
            })
    }

    /// 从去重检测 reason 文本中提取结构化元数据
    /// 返回 (matched_page, matched_path, merge_candidate, auto_converted_from_update)
    fn parse_dedup_reason(reason: &str) -> (Option<String>, Option<String>, bool, bool) {
        if !reason.contains("去重检测") {
            return (None, None, false, false);
        }
        let auto_converted = reason.contains("已自动转为更新页面");
        let merge_candidate = reason.contains("建议人工合并");
        let matched_page = reason.find('「')
            .and_then(|start| {
                let after_open = &reason[start + '「'.len_utf8()..];
                after_open.find('」').map(|end| after_open[..end].to_string())
            });
        let matched_path = reason.find("路径: ")
            .and_then(|start| {
                let after = &reason[start + "路径: ".len()..];
                after.find([',', ')']).map(|end| after[..end].to_string())
            });
        (matched_page, matched_path, merge_candidate, auto_converted)
    }

    /// 获取待审阅列表
    pub fn get_pending_reviews(db: &Arc<DatabaseService>, kb_id: &str) -> Result<Vec<Review>, String> {
        let conn = db.connect()?;
        let mut stmt = conn
            .prepare("SELECT id, kb_id, task_id, status, COALESCE(summary,''), COALESCE(risk_level,'medium'), created_at FROM reviews WHERE kb_id = ?1 AND status = 'pending' ORDER BY CASE risk_level WHEN 'high' THEN 0 WHEN 'medium' THEN 1 ELSE 2 END, created_at DESC")
            .map_err(|e| format!("查询审阅失败: {}", e))?;

        let mut reviews = Vec::new();
        let rows = stmt
            .query_map(rusqlite::params![kb_id], |row| {
                Ok((
                    row.get::<_, String>(0)?, row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?, row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?, row.get::<_, String>(5)?,
                    row.get::<_, String>(6)?,
                ))
            })
            .map_err(|e| format!("映射审阅失败: {}", e))?;

        // 预查询所有 wiki 页面路径（用于计算 missing_target 标志）
        let wiki_paths: std::collections::HashSet<String> = match conn.prepare(
            "SELECT path FROM wiki_pages WHERE kb_id = ?1",
        ) {
            Ok(mut stmt) => match stmt.query_map(rusqlite::params![kb_id], |row| {
                row.get::<_, String>(0)
            }) {
                Ok(rows) => rows.filter_map(|r| r.ok()).collect(),
                Err(e) => {
                    log::error!("[review_engine] wiki_paths query failed: {}", e);
                    Default::default()
                }
            },
            Err(e) => {
                log::error!("[review_engine] wiki_paths prepare failed: {}", e);
                Default::default()
            }
        };

        for row in rows {
            let (id, kb_id, task_id, status, summary, risk_level, created_at) = row.map_err(|e| format!("读取行失败: {}", e))?;

            // 获取 items
            let mut item_stmt = conn
                .prepare("SELECT id, review_id, operation, COALESCE(operation_type, operation) as operation_type, target_path, COALESCE(base_version_hash,''), COALESCE(old_content,''), COALESCE(new_content,''), status, COALESCE(risk_level,'medium'), COALESCE(reason,''), COALESCE(source_id,''), COALESCE(citation_status,'uncited'), COALESCE(summary,''), COALESCE(confidence,'medium'), created_at, COALESCE(title,''), COALESCE(page_type,''), COALESCE(apply_error,'') FROM review_items WHERE review_id = ?1 ORDER BY CASE risk_level WHEN 'high' THEN 0 WHEN 'medium' THEN 1 ELSE 2 END")
                .map_err(|e| format!("查询审阅项失败: {}", e))?;

            let items = item_stmt
                .query_map(rusqlite::params![id], |row| {
                    let target_path: String = row.get(4)?;
                    let new_content: String = row.get(7)?;
                    let stored_title: String = row.get(16)?;
                    let stored_page_type: String = row.get(17)?;
                    let page_type = if stored_page_type.trim().is_empty() {
                        PathService::path_to_page_type(&target_path).to_string()
                    } else {
                        stored_page_type
                    };
                    let title = crate::wiki::markdown_indexer::MarkdownIndexer::best_title(
                        &new_content,
                        &target_path,
                        Some(&stored_title),
                    );
                    let operation_type: String = row.get(3)?;
                    let reason: String = row.get(10)?;
                    let duplicate_candidate = reason.contains("去重检测");
                    let missing_target = !target_path.is_empty() && !wiki_paths.contains(&target_path);
                    let manual_required = matches!(
                        operation_type.as_str(),
                        "merge_suggestion" | "unresolved" | "invalid"
                    );
                    let (matched_page, matched_path, merge_candidate, auto_converted_from_update) =
                        Self::parse_dedup_reason(&reason);
                    Ok(ReviewItem {
                        id: row.get(0)?,
                        review_id: row.get(1)?,
                        operation: row.get(2)?,
                        operation_type,
                        target_path,
                        base_version_hash: row.get(5)?,
                        old_content: row.get(6)?,
                        new_content,
                        status: row.get(8)?,
                        risk_level: row.get(9)?,
                        reason,
                        source_id: row.get(11)?,
                        citation_status: row.get(12)?,
                        summary: row.get(13)?,
                        confidence: row.get(14)?,
                        created_at: row.get(15)?,
                        page_type,
                        title,
                        apply_error: row.get(18)?,
                        duplicate_candidate,
                        missing_target,
                        manual_required,
                        matched_page,
                        matched_path,
                        merge_candidate,
                        auto_converted_from_update,
                    })
                })
                .map_err(|e| format!("映射审阅项失败: {}", e))?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| format!("收集审阅项失败: {}", e))?;

            reviews.push(Review {
                id,
                kb_id,
                task_id,
                status,
                summary,
                risk_level,
                created_at,
                items,
            });
        }

        Ok(reviews)
    }

    /// 接受审阅项（已废弃 - 请使用 apply_review_item_impl 进行按 operation_type 分发）
    /// v0.2.1: 此方法绕过 operation_type 分发，直接设 applied，可能导致 skip/merge_suggestion 被错误应用
    /// 保留此方法仅为向后兼容，内部会记录警告
    #[deprecated(note = "请通过 commands::review::apply_review_item_impl 接受审阅项，以确保 operation_type 正确分发")]
    pub fn accept_item(db: &Arc<DatabaseService>, item_id: &str) -> Result<(), String> {
        log::error!("[ReviewEngine] 警告: accept_item 被调用但已废弃。该路径不进行 operation_type 分发，可能导致状态不一致。请使用 apply_review_item_impl。item_id={}", item_id);
        let conn = db.connect()?;
        let now = chrono::Utc::now().to_rfc3339();
        let (old_status, operation_type): (String, String) = match conn.query_row(
            "SELECT status, COALESCE(operation_type, operation) FROM review_items WHERE id = ?1",
            rusqlite::params![item_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        ) {
            Ok(v) => v,
            Err(rusqlite::Error::QueryReturnedNoRows) => {
                return Err(format!("审阅项不存在: {}", item_id));
            }
            Err(e) => return Err(format!("查询审阅项失败: {}", e)),
        };
        // 防止 skip/merge_suggestion/unresolved 被误应用
        if ["skip", "merge_suggestion", "unresolved", "invalid"].contains(&operation_type.as_str()) {
            return Err(format!("无法通过 accept_item 处理 operation_type={}，请使用对应的审阅操作", operation_type));
        }
        conn.execute(
            "UPDATE review_items SET status = 'applied', updated_at = ?1 WHERE id = ?2",
            rusqlite::params![now, item_id],
        )
        .map_err(|e| format!("接受审阅项失败: {}", e))?;
        Self::log_item_event(&conn, item_id, &old_status, "applied", "accept_legacy", "通过废弃的 accept_item 接受（警告：绕过 operation_type 分发）", &now)?;
        Ok(())
    }

    /// 拒绝审阅项（v0.2.1: 添加事件日志）
    pub fn reject_item(db: &Arc<DatabaseService>, item_id: &str) -> Result<(), String> {
        let conn = db.connect()?;
        let now = chrono::Utc::now().to_rfc3339();
        let old_status: String = match conn.query_row(
            "SELECT status FROM review_items WHERE id = ?1",
            rusqlite::params![item_id],
            |row| row.get(0),
        ) {
            Ok(v) => v,
            Err(rusqlite::Error::QueryReturnedNoRows) => {
                return Err(format!("审阅项不存在: {}", item_id));
            }
            Err(e) => return Err(format!("查询审阅项失败: {}", e)),
        };
        conn.execute(
            "UPDATE review_items SET status = 'rejected', updated_at = ?1 WHERE id = ?2",
            rusqlite::params![now, item_id],
        )
        .map_err(|e| format!("拒绝审阅项失败: {}", e))?;
        Self::log_item_event(&conn, item_id, &old_status, "rejected", "reject", "用户拒绝审阅项", &now)?;
        Ok(())
    }

    /// 记录审阅项状态变更事件
    fn log_item_event(conn: &rusqlite::Connection, item_id: &str, old_status: &str, new_status: &str, action: &str, reason: &str, now: &str) -> Result<(), String> {
        let event_id = uuid::Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO review_item_events (id, review_item_id, old_status, new_status, action, reason, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![event_id, item_id, old_status, new_status, action, reason, now],
        ).map_err(|e| format!("记录审阅项事件失败: {}", e))?;
        Ok(())
    }

    /// 接受所有低风险项
    pub fn accept_all_low_risk(db: &Arc<DatabaseService>, review_id: &str) -> Result<usize, String> {
        let conn = db.connect()?;
        let now = chrono::Utc::now().to_rfc3339();
        let count = conn.execute(
            "UPDATE review_items SET status = 'accepted', updated_at = ?1 WHERE review_id = ?2 AND risk_level = 'low' AND status = 'pending'",
            rusqlite::params![now, review_id],
        )
        .map_err(|e| format!("接受低风险项失败: {}", e))?;
        Ok(count)
    }
}
