use std::sync::Arc;
use crate::core::database_service::DatabaseService;
use crate::core::workspace_service::WorkspaceService;
use crate::dedup::dedup_service::DedupService;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HealthCheckItem {
    pub category: String,
    pub severity: String, // critical, warning, info, ok
    pub name: String,
    pub description: String,
    pub suggestion: String,
    pub fix_action: String, // review, sync_graph, rebuild_index, create_page, manual, reanalyze
    pub detail: serde_json::Value,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HealthCheckResult {
    pub timestamp: String,
    pub overall_status: String, // critical, warning, ok
    pub summary: HealthCheckSummary,
    pub items: Vec<HealthCheckItem>,
    pub report_md: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HealthCheckSummary {
    pub page_count: i64,
    pub source_count: i64,
    pub review_count: i64,
    pub knowledge_item_count: i64,
    pub unlinked_ki_count: i64,
    pub graph_node_count: i64,
    pub graph_edge_count: i64,
    pub critical_count: usize,
    pub warning_count: usize,
}

pub struct HealthCheckAgent;

impl HealthCheckAgent {
    pub fn run(
        db: &Arc<DatabaseService>,
        kb_id: &str,
        kb_path: &str,
        workspace: &WorkspaceService,
    ) -> Result<String, String> {
        let result = Self::run_structured(db, kb_id, kb_path, workspace)?;
        Ok(result.report_md)
    }

    pub fn run_structured(
        db: &Arc<DatabaseService>,
        kb_id: &str,
        kb_path: &str,
        workspace: &WorkspaceService,
    ) -> Result<HealthCheckResult, String> {
        let conn = db.connect()?;
        let now = chrono::Utc::now();
        let now_str = now.format("%Y-%m-%d %H:%M:%S").to_string();
        let mut items: Vec<HealthCheckItem> = Vec::new();
        let mut report_md = String::new();

        report_md.push_str(&format!("# 知识库健康检查报告\n\n检查时间: {}\n\n", now_str));

        // ====== 统计 ======
        let page_count: i64 = count_safe(&conn, "SELECT COUNT(*) FROM wiki_pages WHERE kb_id = ?1", kb_id);
        let source_count: i64 = count_safe(&conn, "SELECT COUNT(*) FROM sources WHERE kb_id = ?1", kb_id);
        let review_count: i64 = count_safe(&conn, "SELECT COUNT(*) FROM review_items ri INNER JOIN reviews r ON ri.review_id = r.id WHERE r.kb_id = ?1 AND ri.status = 'pending'", kb_id);
        let ki_count: i64 = count_safe(&conn, "SELECT COUNT(*) FROM knowledge_items WHERE kb_id = ?1", kb_id);
        let graph_node_count: i64 = count_safe(&conn, "SELECT COUNT(*) FROM graph_nodes WHERE kb_id = ?1", kb_id);
        let graph_edge_count: i64 = count_safe(&conn, "SELECT COUNT(*) FROM graph_edges WHERE kb_id = ?1", kb_id);

        report_md.push_str("## 统计\n\n");
        report_md.push_str(&format!("- Wiki 页面数: {}\n", page_count));
        report_md.push_str(&format!("- Source 文件数: {}\n", source_count));
        report_md.push_str(&format!("- 待审阅项: {}\n", review_count));
        report_md.push_str(&format!("- 知识项数: {}\n", ki_count));
        report_md.push_str(&format!("- 图谱节点数: {}\n", graph_node_count));
        report_md.push_str(&format!("- 图谱边数: {}\n\n", graph_edge_count));

        // ====== 检查 1: Source 有文件但无 Wiki 页面 ======
        if source_count > 0 && page_count == 0 {
            items.push(HealthCheckItem {
                category: "core_loop".into(),
                severity: "critical".into(),
                name: "核心闭环未完成".into(),
                description: format!("有 {} 个 Source 文件，但 Wiki 页面为 0，核心闭环未建立", source_count),
                suggestion: "请前往审阅中心处理待审阅项，或从 Source 重新生成分析".into(),
                fix_action: "review".into(),
                detail: serde_json::json!({"source_count": source_count, "page_count": page_count}),
            });
            report_md.push_str("## 核心闭环 ❌\n\n有 Source 文件但无异构 Wiki 页面，核心闭环未建立。\n\n");
        } else if page_count > 0 {
            items.push(HealthCheckItem {
                category: "core_loop".into(),
                severity: "ok".into(),
                name: "核心闭环正常".into(),
                description: format!("{} 个 Source → {} 个 Wiki 页面", source_count, page_count),
                suggestion: "".into(),
                fix_action: "".into(),
                detail: serde_json::json!({}),
            });
            report_md.push_str("## 核心闭环 ✅\n\nSource → Wiki 页面闭环已建立。\n\n");
        }

        // ====== 检查 2: 待审阅项 ======
        if review_count > 0 {
            items.push(HealthCheckItem {
                category: "review".into(),
                severity: if page_count == 0 { "critical".into() } else { "warning".into() },
                name: "有待审阅项".into(),
                description: format!("有 {} 个待审阅修改建议未处理", review_count),
                suggestion: "请前往审阅中心接受或拒绝审阅项".into(),
                fix_action: "review".into(),
                detail: serde_json::json!({"review_count": review_count}),
            });
            report_md.push_str(&format!("## 待审阅 ⚠️\n\n有 {} 个待审阅项未处理。\n\n", review_count));
        } else {
            items.push(HealthCheckItem {
                category: "review".into(),
                severity: "ok".into(),
                name: "无待审阅项".into(),
                description: "所有审阅项已处理完毕".into(),
                suggestion: "".into(),
                fix_action: "".into(),
                detail: serde_json::json!({}),
            });
            report_md.push_str("## 待审阅 ✅\n\n无待审阅项。\n\n");
        }

        // ====== 检查 3: knowledge_items 未关联页面 ======
        let unlinked_ki: i64 = count_safe(&conn,
            "SELECT COUNT(*) FROM knowledge_items WHERE kb_id = ?1 AND (page_id = '' OR page_id IS NULL)", kb_id);

        if unlinked_ki > 0 {
            let severity = if unlinked_ki > 10 { "warning" } else { "info" };
            items.push(HealthCheckItem {
                category: "knowledge".into(),
                severity: severity.to_string(),
                name: "知识项未关联页面".into(),
                description: format!("{} 个知识项尚未关联到 Wiki 页面", unlinked_ki),
                suggestion: "请通过审阅中心接受页面创建建议，将知识项写入 Wiki 页面".into(),
                fix_action: "review".into(),
                detail: serde_json::json!({"unlinked_count": unlinked_ki}),
            });
            report_md.push_str(&format!("## 知识项 ⚠️\n\n{} 个知识项未关联页面。\n\n", unlinked_ki));
        } else if ki_count > 0 {
            items.push(HealthCheckItem {
                category: "knowledge".into(),
                severity: "ok".into(),
                name: "知识项已全部关联".into(),
                description: "所有知识项都已关联到 Wiki 页面".into(),
                suggestion: "".into(),
                fix_action: "".into(),
                detail: serde_json::json!({}),
            });
        }

        // ====== 检查 4: 图谱边为空 ======
        if graph_node_count > 0 && graph_edge_count == 0 {
            items.push(HealthCheckItem {
                category: "graph".into(),
                severity: "warning".into(),
                name: "图谱无边".into(),
                description: format!("有 {} 个图谱节点，但 0 条关系边", graph_node_count),
                suggestion: "完成页面审阅后运行关系分析或重建图谱关系".into(),
                fix_action: "sync_graph".into(),
                detail: serde_json::json!({"node_count": graph_node_count, "edge_count": 0}),
            });
            report_md.push_str(&format!("## 图谱 ⚠️\n\n{} 个节点，0 条边。\n\n", graph_node_count));
        }

        // ====== 检查 5: 图谱节点为空但知识项存在 ======
        if graph_node_count == 0 && ki_count > 0 {
            items.push(HealthCheckItem {
                category: "graph".into(),
                severity: "warning".into(),
                name: "图谱未同步".into(),
                description: format!("有 {} 个知识项，但图谱节点为 0", ki_count),
                suggestion: "请执行图谱同步操作".into(),
                fix_action: "sync_graph".into(),
                detail: serde_json::json!({"ki_count": ki_count, "node_count": 0}),
            });
            report_md.push_str("## 图谱 ⚠️\n\n图谱节点为空，需要同步。\n\n");
        } else if graph_node_count > 0 {
            items.push(HealthCheckItem {
                category: "graph".into(),
                severity: "ok".into(),
                name: "图谱数据正常".into(),
                description: format!("{} 个节点，{} 条边", graph_node_count, graph_edge_count),
                suggestion: "".into(),
                fix_action: "".into(),
                detail: serde_json::json!({}),
            });
        }

        // ====== 检查 6: Review Item 指向不存在的 target_path ======
        if let Ok(mut stmt) = conn.prepare(
            "SELECT ri.id, ri.operation, ri.target_path FROM review_items ri JOIN reviews r ON ri.review_id = r.id WHERE r.kb_id = ?1 AND ri.status = 'pending' AND ri.operation IN ('update', 'append')"
        ) {
            let kb_path_buf = std::path::PathBuf::from(kb_path);
            let mut broken_updates = 0;
            if let Ok(rows) = stmt.query_map(rusqlite::params![kb_id], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?))
            }) {
                for row in rows.filter_map(|r| r.ok()) {
                    let (_id, _op, target_path) = row;
                    let abs_path = crate::wiki::path_service::PathService::resolve_workspace_path(&kb_path_buf, &target_path);
                    if !abs_path.exists() {
                        broken_updates += 1;
                    }
                }
            }
            if broken_updates > 0 {
                items.push(HealthCheckItem {
                    category: "review".into(),
                    severity: "critical".into(),
                    name: "审阅项目标文件缺失".into(),
                    description: format!("{} 个更新操作指向不存在的页面文件", broken_updates),
                    suggestion: "系统已自动将不存在的目标降级为创建页面。请刷新审阅中心查看。".into(),
                    fix_action: "review".into(),
                    detail: serde_json::json!({"broken_updates": broken_updates}),
                });
                report_md.push_str(&format!("## 审阅一致性 ❌\n\n{} 个更新操作目标文件不存在。\n\n", broken_updates));
            }
        }

        // ====== 检查 7: Source 状态一致性 ======
        let source_no_page: i64 = count_safe(&conn,
            "SELECT COUNT(*) FROM sources s WHERE s.kb_id = ?1 AND s.status IN ('analyzed', 'processed') AND (SELECT COUNT(*) FROM knowledge_items ki WHERE ki.source_id = s.id AND ki.page_id != '') = 0", kb_id);

        if source_no_page > 0 {
            items.push(HealthCheckItem {
                category: "source".into(),
                severity: "warning".into(),
                name: "Source 已处理但无页面".into(),
                description: format!("{} 个 Source 状态为 processed 但没有关联 Wiki 页面", source_no_page),
                suggestion: "请检查审阅中心是否有对应的创建页面建议".into(),
                fix_action: "review".into(),
                detail: serde_json::json!({"source_no_page_count": source_no_page}),
            });
            report_md.push_str(&format!("## Source 状态 ⚠️\n\n{} 个已处理 Source 没有关联 Wiki 页面。\n\n", source_no_page));
        }

        // ====== 检查 8: AI摘要缺失 ======
        let no_summary_count: i64 = count_safe(&conn,
            "SELECT COUNT(*) FROM sources WHERE kb_id = ?1 AND (ai_summary = '' OR ai_summary IS NULL) AND status IN ('analyzed', 'processed')", kb_id);

        if no_summary_count > 0 && source_count > 0 {
            items.push(HealthCheckItem {
                category: "source".into(),
                severity: "info".into(),
                name: "Source AI 摘要缺失".into(),
                description: format!("{} 个已处理 Source 没有 AI 摘要", no_summary_count),
                suggestion: "可重新运行分析以生成摘要".into(),
                fix_action: "reanalyze".into(),
                detail: serde_json::json!({"no_summary_count": no_summary_count}),
            });
        }

        // ====== 检查 9: 文件一致性 ======
        let wiki_dir = std::path::PathBuf::from(kb_path).join("wiki");
        let index_missing = !wiki_dir.join("index.md").exists();
        let log_missing = !wiki_dir.join("log.md").exists();

        if index_missing {
            items.push(HealthCheckItem {
                category: "files".into(),
                severity: "warning".into(),
                name: "index.md 缺失".into(),
                description: "Wiki 目录下缺少 index.md".into(),
                suggestion: "请执行重建 Wiki 索引操作".into(),
                fix_action: "rebuild_index".into(),
                detail: serde_json::json!({}),
            });
            report_md.push_str("## 文件一致性 ⚠️\n\nindex.md 缺失。\n\n");
        }
        if log_missing {
            items.push(HealthCheckItem {
                category: "files".into(),
                severity: "info".into(),
                name: "log.md 缺失".into(),
                description: "Wiki 目录下缺少 log.md".into(),
                suggestion: "下次写入操作时会自动创建".into(),
                fix_action: "".into(),
                detail: serde_json::json!({}),
            });
            report_md.push_str("log.md 缺失。\n\n");
        }
        if !index_missing && !log_missing {
            report_md.push_str("## 文件一致性 ✅\n\n核心文件完整。\n\n");
        }

        // ====== 检查 10: 失败任务 ======
        let failed_task_count: i64 = count_safe(&conn,
            "SELECT COUNT(*) FROM tasks WHERE kb_id = ?1 AND status IN ('failed', 'interrupted')", kb_id);

        if failed_task_count > 0 {
            items.push(HealthCheckItem {
                category: "tasks".into(),
                severity: "warning".into(),
                name: "有失败任务".into(),
                description: format!("{} 个任务处于失败或中断状态", failed_task_count),
                suggestion: "请前往任务页面查看详情并重试".into(),
                fix_action: "manual".into(),
                detail: serde_json::json!({"failed_task_count": failed_task_count}),
            });
            report_md.push_str(&format!("## 任务 ⚠️\n\n{} 个失败/中断任务。\n\n", failed_task_count));
        }

        // ====== 检查 11: Source 分析/流水线失败 (v0.2.3) ======
        let failed_source_count: i64 = count_safe(&conn,
            "SELECT COUNT(*) FROM sources WHERE kb_id = ?1 AND status IN ('analysis_failed', 'pipeline_failed')", kb_id);

        if failed_source_count > 0 {
            items.push(HealthCheckItem {
                category: "source".into(),
                severity: "warning".into(),
                name: "Source 处理失败".into(),
                description: format!("{} 个 Source 处于分析失败或流水线失败状态", failed_source_count),
                suggestion: "请检查失败原因并重新导入这些文档".into(),
                fix_action: "reanalyze".into(),
                detail: serde_json::json!({"failed_source_count": failed_source_count}),
            });
            report_md.push_str(&format!("## Source 失败 ⚠️\n\n{} 个 Source 处理失败。\n\n", failed_source_count));
        }

        // ====== 检查 12: 孤立页面（无关系、无来源的页面）(v0.2.3) ======
        let orphan_page_count: i64 = count_safe(&conn,
            "SELECT COUNT(*) FROM wiki_pages wp WHERE wp.kb_id = ?1 AND NOT EXISTS (SELECT 1 FROM graph_edges ge WHERE ge.source_node_id = (SELECT id FROM graph_nodes WHERE page_id = wp.id LIMIT 1) OR ge.target_node_id = (SELECT id FROM graph_nodes WHERE page_id = wp.id LIMIT 1))", kb_id);

        if orphan_page_count > 0 && page_count > 3 {
            let severity = if orphan_page_count as f64 / page_count as f64 > 0.5 { "warning" } else { "info" };
            items.push(HealthCheckItem {
                category: "graph".into(),
                severity: severity.to_string(),
                name: "孤立页面检测".into(),
                description: format!("{} 个 Wiki 页面没有任何关系连接（共 {} 页）", orphan_page_count, page_count),
                suggestion: "可通过关系分析建立页面间链接".into(),
                fix_action: "sync_graph".into(),
                detail: serde_json::json!({"orphan_count": orphan_page_count, "total_pages": page_count}),
            });
            report_md.push_str(&format!("## 孤立页面 ⚠️\n\n{} 个页面无关系连接。\n\n", orphan_page_count));
        }

        // ====== 检查 13: 重复页面检测 (v0.2.1) ======
        if let Ok(duplicates) = DedupService::detect_duplicate_pages(db, kb_id) {
            if !duplicates.is_empty() {
                let total_dup_pages: usize = duplicates.iter().map(|g| g.page_ids.len()).sum();
                items.push(HealthCheckItem {
                    category: "dedup".into(),
                    severity: "warning".into(),
                    name: "重复页面检测".into(),
                    description: format!("发现 {} 组重复页面，共涉及 {} 个页面", duplicates.len(), total_dup_pages),
                    suggestion: "建议合并重复页面或设置别名/重定向".into(),
                    fix_action: "manual".into(),
                    detail: serde_json::json!({
                        "duplicate_groups": duplicates.iter().map(|g| serde_json::json!({
                            "canonical_name": g.canonical_name,
                            "normalized_name": g.normalized_name,
                            "match_type": g.match_type,
                            "pages": g.page_titles,
                            "paths": g.page_paths,
                        })).collect::<Vec<_>>(),
                    }),
                });
                report_md.push_str(&format!("## 重复页面 ⚠️ (v0.2.1)\n\n发现 {} 组重复页面:\n\n", duplicates.len()));
                for (i, group) in duplicates.iter().enumerate() {
                    report_md.push_str(&format!("{}. 组 ({}): {}\n", i+1, group.match_type, group.page_titles.join(", ")));
                    report_md.push_str(&format!("   规范化名称: {}\n", group.normalized_name));
                }
                report_md.push('\n');
            } else {
                items.push(HealthCheckItem {
                    category: "dedup".into(),
                    severity: "ok".into(),
                    name: "无重复页面".into(),
                    description: "未检测到重复页面".into(),
                    suggestion: "".into(),
                    fix_action: "".into(),
                    detail: serde_json::json!({}),
                });
            }
        }

        // ====== 检查 14: 文档解析能力 (MarkItDown/Python) ======
        {
            let python_ok = crate::skills::markitdown_skill::MarkitdownSkill::has_python();
            let md_ok = crate::skills::markitdown_skill::MarkitdownSkill::is_available();
            let doc_sources: i64 = count_safe(&conn,
                "SELECT COUNT(*) FROM sources WHERE kb_id = ?1 AND file_type IN ('pdf','docx','pptx','xlsx','csv','json','xml')", kb_id);

            if !python_ok && doc_sources > 0 {
                items.push(HealthCheckItem {
                    category: "parsing".into(),
                    severity: "warning".into(),
                    name: "Python 未安装".into(),
                    description: format!("有 {} 个文档需要解析，但系统未检测到 Python 运行环境。PDF/DOCX/PPTX 等格式将无法提取文本", doc_sources),
                    suggestion: "请安装 Python 3.10+ (https://python.org) 或 Microsoft Store 搜索 Python 3.12，安装后重启应用。".into(),
                    fix_action: "manual".into(),
                    detail: serde_json::json!({"doc_sources": doc_sources, "python_found": false, "markitdown_available": false}),
                });
                report_md.push_str(&format!("## 文档解析 ⚠️\n\n未检测到 Python，{} 个文档无法解析。请安装 Python 后重启。\n\n", doc_sources));
            } else if !md_ok && doc_sources > 0 {
                items.push(HealthCheckItem {
                    category: "parsing".into(),
                    severity: "info".into(),
                    name: "MarkItDown 未安装".into(),
                    description: format!("有 {} 个文档需要解析，MarkItDown 将在首次使用时自动安装", doc_sources),
                    suggestion: "首次导入 PDF/DOCX 等文件时，系统会自动执行 pip install markitdown。".into(),
                    fix_action: "manual".into(),
                    detail: serde_json::json!({"doc_sources": doc_sources, "python_found": true, "markitdown_available": false}),
                });
                report_md.push_str(&format!("## 文档解析 ℹ️\n\nMarkItDown 将在首次使用时自动安装。（{} 个文档待解析）\n\n", doc_sources));
            } else if md_ok {
                report_md.push_str("## 文档解析 ✅\n\nMarkItDown 可用，支持 PDF/DOCX/PPTX/XLSX/CSV/JSON/XML 等格式。\n\n");
            }
        }

        // ====== 建议汇总 ======
        report_md.push_str("## 建议\n\n");
        let mut has_suggestions = false;
        for item in &items {
            if item.severity != "ok" && !item.suggestion.is_empty() {
                report_md.push_str(&format!("- [{}] {}: {}\n", item.severity, item.name, item.suggestion));
                has_suggestions = true;
            }
        }
        if !has_suggestions {
            report_md.push_str("- 知识库状态良好！🎉\n");
        }

        let critical_count = items.iter().filter(|i| i.severity == "critical").count();
        let warning_count = items.iter().filter(|i| i.severity == "warning").count();

        let overall_status = if critical_count > 0 {
            "critical"
        } else if warning_count > 0 {
            "warning"
        } else {
            "ok"
        };

        // 保存报告
        let tasks_dir = workspace.get_tasks_dir(&std::path::PathBuf::from(kb_path));
        let report_dir = tasks_dir.join("health_check");
        if let Err(e) = std::fs::create_dir_all(&report_dir) { log::error!("[health_check] 创建报告目录失败: {}", e); }
        if let Err(e) = std::fs::write(report_dir.join("report.md"), &report_md) { log::error!("[health_check] 写入报告失败: {}", e); }

        Ok(HealthCheckResult {
            timestamp: now_str,
            overall_status: overall_status.to_string(),
            summary: HealthCheckSummary {
                page_count,
                source_count,
                review_count,
                knowledge_item_count: ki_count,
                unlinked_ki_count: unlinked_ki,
                graph_node_count,
                graph_edge_count,
                critical_count,
                warning_count,
            },
            items,
            report_md,
        })
    }
}

fn count_safe(conn: &rusqlite::Connection, sql: &str, kb_id: &str) -> i64 {
    match conn.query_row(sql, rusqlite::params![kb_id], |row| row.get::<_, i64>(0)) {
        Ok(c) => c,
        Err(rusqlite::Error::QueryReturnedNoRows) => 0,
        Err(e) => {
            log::error!("[health_check] COUNT 查询失败: {}", e);
            0
        }
    }
}
