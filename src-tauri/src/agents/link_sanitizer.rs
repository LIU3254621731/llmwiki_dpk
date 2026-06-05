use std::sync::Arc;
use crate::core::database_service::DatabaseService;
use crate::core::config_service::ConfigService;
use crate::core::secret_service::SecretService;
use crate::core::event_bus::EventBus;
use crate::core::token_logger::TokenContext;
use crate::embedding::vdb_service::VdbService;
use crate::model::model_gateway::ModelGateway;
use crate::wiki::path_service::PathService;
use crate::review::review_engine::ReviewEngine;

/// 死链信息
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BrokenLink {
    pub link_text: String,
    pub source_page_path: String,
    pub link_type: String, // "wikilink" | "source_ref"
    pub detected_at: String,
}

/// Sanitize 执行结果
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SanitizeResult {
    pub broken_links_found: usize,
    pub placeholders_created: usize,
    pub ai_completions_proposed: usize,
    pub ai_completions_aborted: usize,
    pub details: Vec<String>,
}

/// AI 生成的结构化内容
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct GeneratedPage {
    title: String,
    content: String,
    summary: String,
    confidence: String,
    risk_level: String,
}

pub struct LinkSanitizerAgent;

impl LinkSanitizerAgent {
    /// 主入口：运行完整 sanitize 周期
    pub async fn run(
        db: &Arc<DatabaseService>,
        vdb: &Arc<VdbService>,
        config: &Arc<ConfigService>,
        secrets: &Arc<SecretService>,
        event_bus: &Arc<EventBus>,
        kb_id: &str,
        kb_path: &str,
    ) -> Result<SanitizeResult, String> {
        // 1. 检测死链
        let broken_links = Self::detect_broken_links(db, kb_id)?;

        let mut placeholders_created = 0usize;
        let mut ai_completions_proposed = 0usize;
        let mut ai_completions_aborted = 0usize;
        let mut details: Vec<String> = Vec::new();

        for link in &broken_links {
            // 去重检查
            if Self::is_duplicate(db, kb_id, link)? {
                details.push(format!("跳过重复死链: [[{}]] (来自 {})", link.link_text, link.source_page_path));
                continue;
            }

            // 轨 1: 创建影子占位页
            match Self::create_shadow_placeholder(db, kb_id, kb_path, link) {
                Ok(path) => {
                    placeholders_created += 1;
                    details.push(format!("创建占位页: {} → {}", link.link_text, path));

                    let _ = event_bus.emit_wiki_updated(kb_id, &path);
                    let _ = event_bus.emit_kb_stats_changed(kb_id);

                    // 轨 2: 尝试 AI 补全
                    match Self::attempt_ai_completion(
                        db, vdb, config, secrets, event_bus, kb_id, kb_path, link, &path,
                    ).await {
                        Ok(Some(review_item_id)) => {
                            ai_completions_proposed += 1;
                            details.push(format!("AI 补全提案已创建: [[{}]] → review_item={}", link.link_text, review_item_id));

                            let _ = event_bus.emit_review_updated(kb_id, "");
                            let _ = event_bus.emit_notification(
                                "info",
                                "AI 自动补全",
                                &format!("「{}」的死链已生成内容提案，请前往审阅中心查看", link.link_text),
                            );
                        }
                        Ok(None) => {
                            ai_completions_aborted += 1;
                            details.push(format!("AI 补全中止 (相似度不足): [[{}]]", link.link_text));

                            let _ = event_bus.emit_notification(
                                "warning",
                                "无法自动补全",
                                &format!("「{}」在知识库中缺乏足够上下文，保留占位页", link.link_text),
                            );
                        }
                        Err(e) => {
                            details.push(format!("AI 补全失败: [[{}]] → {}", link.link_text, e));
                        }
                    }
                }
                Err(e) => {
                    details.push(format!("创建占位页失败: [[{}]] → {}", link.link_text, e));
                }
            }
        }

        let _ = event_bus.emit_kb_stats_changed(kb_id);

        Ok(SanitizeResult {
            broken_links_found: broken_links.len(),
            placeholders_created,
            ai_completions_proposed,
            ai_completions_aborted,
            details,
        })
    }

    /// 扫描所有 wiki 页面中的死链 ([[wikilink]] 指向不存在的页面)
    fn detect_broken_links(
        db: &Arc<DatabaseService>,
        kb_id: &str,
    ) -> Result<Vec<BrokenLink>, String> {
        let conn = db.connect()?;
        let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();

        // 获取所有 wiki 页面路径集合
        let mut stmt = conn
            .prepare("SELECT path FROM wiki_pages WHERE kb_id = ?1")
            .map_err(|e| format!("查询 wiki_pages 失败: {}", e))?;
        let valid_paths: Vec<String> = stmt
            .query_map(rusqlite::params![kb_id], |row| row.get(0))
            .map_err(|e| format!("映射 wiki_pages 失败: {}", e))?
            .filter_map(|r| r.ok())
            .collect();

        let valid_set: std::collections::HashSet<String> = valid_paths.iter().cloned().collect();

        // 获取所有 wiki 页面的完整内容
        let mut page_stmt = conn
            .prepare("SELECT path, title FROM wiki_pages WHERE kb_id = ?1")
            .map_err(|e| format!("查询 wiki_pages 2 失败: {}", e))?;
        let pages: Vec<(String, String)> = page_stmt
            .query_map(rusqlite::params![kb_id], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(|e| format!("映射 wiki_pages 2 失败: {}", e))?
            .filter_map(|r| r.ok())
            .collect();

        let mut broken_links: Vec<BrokenLink> = Vec::new();

        for (page_path, _title) in &pages {
            // 读取磁盘上的 markdown 文件内容
            let kb_path = match conn.query_row(
                "SELECT path FROM knowledge_bases WHERE id = ?1",
                rusqlite::params![kb_id],
                |row| row.get::<_, String>(0),
            ) {
                Ok(p) => p,
                Err(_) => continue,
            };
            let full_path = std::path::PathBuf::from(&kb_path).join(page_path);
            let content = match std::fs::read_to_string(&full_path) {
                Ok(c) => c,
                Err(_) => continue,
            };

            // 提取 [[wikilink]]
            let re = regex::Regex::new(r"\[\[([^\]]+)\]\]").unwrap();
            for cap in re.captures_iter(&content) {
                let link_text = cap[1].trim().to_string();
                // 跳过 URL 和带管道符的链接
                if link_text.contains("://") || link_text.contains('|') {
                    continue;
                }

                // 生成可能的路径 — 检查所有页面类型目录
                let safe_name = PathService::generate_safe_name(&link_text);
                let page_type_dirs = ["concepts", "entities", "topics", "questions", "reviews", "sources", "datasets", "methods"];
                let exists = page_type_dirs.iter().any(|dir| {
                    let path = format!("wiki/{}/{}.md", dir, safe_name);
                    valid_set.contains(&path)
                }) || valid_set.iter().any(|p| p.ends_with(&format!("/{}.md", safe_name)));

                if !exists {
                    // 避免重复
                    let already_found = broken_links.iter().any(|bl| {
                        bl.link_text == link_text && bl.source_page_path == *page_path
                    });
                    if !already_found {
                        broken_links.push(BrokenLink {
                            link_text,
                            source_page_path: page_path.clone(),
                            link_type: "wikilink".to_string(),
                            detected_at: now.clone(),
                        });
                    }
                }
            }
        }

        Ok(broken_links)
    }

    /// 创建影子占位页（轨 1），返回相对路径
    fn create_shadow_placeholder(
        db: &Arc<DatabaseService>,
        kb_id: &str,
        kb_path: &str,
        link: &BrokenLink,
    ) -> Result<String, String> {
        let safe_name = PathService::generate_safe_name(&link.link_text);
        // 使用 wiki/staging/ 子目录
        let relative_path = format!("wiki/staging/{}.md", safe_name);
        let wiki_dir = std::path::PathBuf::from(kb_path).join("wiki");
        let staging_dir = wiki_dir.join("staging");

        std::fs::create_dir_all(&staging_dir)
            .map_err(|e| format!("创建 staging 目录失败: {}", e))?;

        let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
        let placeholder_content = format!(
            r#"---
title: {}
type: concept
canonical_name: {}
status: staging
created: {}
sources: []
tags: auto-generated
confidence: low
---

# {}

> ⚠️ AI 自动补全待审 — 该内容已被删除或正在等待 Agent 补充

**状态**: 待审阅 (staging)
**创建时间**: {}
**来源死链**: [[{}]] (位于 {})

> 此页面为系统自动生成的占位页。当有足够的上下文信息时，AI 将自动生成内容提案并提交审阅中心。
"#,
            link.link_text,
            safe_name,
            now,
            link.link_text,
            now,
            link.link_text,
            link.source_page_path,
        );

        // 写入占位文件
        let file_path = staging_dir.join(format!("{}.md", safe_name));
        std::fs::write(&file_path, &placeholder_content)
            .map_err(|e| format!("写入占位文件失败: {}", e))?;

        // 插入 wiki_pages 记录 (content_hash = "staging" 标记)
        let conn = db.connect()?;
        let page_id = uuid::Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO wiki_pages (id, kb_id, path, title, page_type, content_hash, status, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, 'staging', 'staging', 'staging', ?5, ?5)",
            rusqlite::params![page_id, kb_id, relative_path, link.link_text, now],
        )
        .map_err(|e| format!("插入 wiki_pages 记录失败: {}", e))?;

        // 记录到 link_sanitizer_log
        let log_id = uuid::Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO link_sanitizer_log (id, kb_id, link_text, link_type, source_page_path, action, placeholder_path, created_at) VALUES (?1, ?2, ?3, ?4, ?5, 'placeholder', ?6, ?7)",
            rusqlite::params![log_id, kb_id, link.link_text, link.link_type, link.source_page_path, relative_path, now],
        )
        .map_err(|e| format!("写入 link_sanitizer_log 失败: {}", e))?;

        Ok(relative_path)
    }

    /// AI 补全流程（轨 2）: VDB 召回 → LLM 生成 → Review 提案
    /// 返回 Some(review_item_id) 如果成功创建提案，None 如果熔断中止
    async fn attempt_ai_completion(
        db: &Arc<DatabaseService>,
        vdb: &Arc<VdbService>,
        config: &Arc<ConfigService>,
        secrets: &Arc<SecretService>,
        event_bus: &Arc<EventBus>,
        kb_id: &str,
        kb_path: &str,
        link: &BrokenLink,
        placeholder_path: &str,
    ) -> Result<Option<String>, String> {
        // Step 1: VDB 跨库召回
        let chunks = vdb.search_similar(kb_id, &link.link_text, 10)?;

        if chunks.is_empty() {
            // 无 VDB 数据，熔断
            Self::log_action(db, kb_id, link, "aborted", placeholder_path, "", 0.0, "VDB 中没有相关上下文片段")?;
            return Ok(None);
        }

        let max_similarity = chunks.first().map(|(_, s)| *s).unwrap_or(0.0);

        // 熔断：最高相似度 < 0.6
        if max_similarity < 0.6 {
            Self::log_action(
                db, kb_id, link, "aborted", placeholder_path, "",
                max_similarity,
                &format!("最高相似度 {:.4} < 0.6，熔断中止", max_similarity),
            )?;
            return Ok(None);
        }

        // Step 2: 筛选相似度 >= 0.4 的片段（最多 5 个）
        let relevant_chunks: Vec<&(String, f64)> = chunks
            .iter()
            .filter(|(_, sim)| *sim >= 0.4)
            .take(5)
            .collect();

        let context_snippets = relevant_chunks
            .iter()
            .enumerate()
            .map(|(i, (text, sim))| {
                format!("[片段 {} (相似度: {:.2})]\n{}", i + 1, sim, text)
            })
            .collect::<Vec<_>>()
            .join("\n\n---\n\n");

        // Step 3: 加载 prompt 模板
        let prompt_template = include_str!("../prompts/link_sanitizer.txt");
        let system_prompt = prompt_template
            .replace("{{concept_name}}", &link.link_text)
            .replace("{{context_snippets}}", &context_snippets);

        let user_message = format!(
            "请为缺失的概念「{}」生成 Wiki 页面内容。该概念在页面「{}」中被引用但目标页面不存在。",
            link.link_text, link.source_page_path
        );

        // Step 4: 调用 LLM
        let provider_config = config
            .get_provider_config()
            .map_err(|e| format!("获取 Provider 配置失败: {}", e))?;

        let model_name = provider_config.chat_model.clone();

        let model_gateway = ModelGateway::new(secrets.clone());
        let token_ctx = TokenContext {
            task_id: format!("link_sanitizer_{}", chrono::Utc::now().format("%Y%m%d%H%M%S")),
            task_name: format!("自愈补全: {}", link.link_text),
            agent_name: "LinkSanitizerAgent".to_string(),
            model_name,
            provider: provider_config.provider.clone(),
        };

        let result = model_gateway
            .chat_with_content_and_ctx(
                &provider_config,
                &system_prompt,
                &user_message,
                true, // JSON mode
                Some(token_ctx),
            )
            .await
            .map_err(|e| format!("LLM 调用失败: {}", e))?;

        // Step 5: 解析 LLM 输出
        let json_str = match serde_json::from_str::<GeneratedPage>(&result.content) {
            Ok(g) => Ok(g),
            Err(_) => {
                // Try JSON repair — validate_and_repair_json returns Value
                match crate::schema::json_repair::validate_and_repair_json(&result.content) {
                    Ok(repaired_val) => {
                        let repaired_str = repaired_val.to_string();
                        serde_json::from_str(&repaired_str)
                    }
                    Err(_) => {
                        // Direct parse failed and repair failed, return the original error
                        serde_json::from_str(&result.content)
                    }
                }
            }
        };
        let generated: GeneratedPage = json_str
            .map_err(|e| format!("解析 LLM 输出失败: {} — 原始输出: {}", e, &result.content[..result.content.len().min(300)]))?;

        // 基本内容校验：至少需要有意义的内容
        if generated.content.len() < 50 || generated.title.is_empty() {
            Self::log_action(
                db, kb_id, link, "aborted", placeholder_path, "",
                max_similarity,
                "LLM 生成的内容质量不足（过短或无标题）",
            )?;
            return Ok(None);
        }

        // Step 6: 构建 Review 提案
        let update_plan = serde_json::json!({
            "wiki_update_plan": [{
                "operation": "update",
                "title": generated.title,
                "path": placeholder_path,
                "page_type": "concept",
                "risk_level": generated.risk_level,
                "reason": format!("LinkSanitizerAgent 自动补全死链 [[{}]]", link.link_text),
                "summary": generated.summary,
                "confidence": generated.confidence,
                "citation_status": "uncited",
                "source_id": "",
                "new_markdown": generated.content,
            }]
        });

        let task_id = format!("ls_{}", chrono::Utc::now().format("%Y%m%d%H%M%S%3f"));

        ReviewEngine::generate_review(
            db,
            kb_id,
            &task_id,
            &update_plan,
            kb_path,
        )?;

        // 获取刚创建的 review_item_id (通过最新的 review 查询)
        let conn = db.connect()?;
        let review_item_id: Option<String> = conn
            .query_row(
                "SELECT ri.id FROM review_items ri
                 INNER JOIN reviews r ON ri.review_id = r.id
                 WHERE r.kb_id = ?1 AND r.task_id = ?2
                 ORDER BY ri.created_at DESC LIMIT 1",
                rusqlite::params![kb_id, task_id],
                |row| row.get(0),
            )
            .ok();

        let review_item_id_str = review_item_id.unwrap_or_default();

        // Step 7: 记录到 link_sanitizer_log
        Self::log_action(
            db, kb_id, link, "ai_completion", placeholder_path,
            &review_item_id_str,
            max_similarity,
            &format!("LLM 生成完成，置信度: {}, 风险: {}", generated.confidence, generated.risk_level),
        )?;

        let _ = event_bus.emit_review_updated(kb_id, "");

        Ok(Some(review_item_id_str))
    }

    /// 去重检查：同一死链是否已经处理过
    fn is_duplicate(
        db: &Arc<DatabaseService>,
        kb_id: &str,
        link: &BrokenLink,
    ) -> Result<bool, String> {
        let conn = db.connect()?;

        // 检查 placeholder 是否已存在
        let placeholder_exists: bool = conn
            .query_row(
                "SELECT COUNT(*) > 0 FROM link_sanitizer_log WHERE kb_id = ?1 AND link_text = ?2 AND action = 'placeholder'",
                rusqlite::params![kb_id, link.link_text],
                |row| row.get(0),
            )
            .unwrap_or(false);

        if placeholder_exists {
            return Ok(true);
        }

        // 检查 24h 内的 abort 冷却期
        let recent_abort: bool = conn
            .query_row(
                "SELECT COUNT(*) > 0 FROM link_sanitizer_log WHERE kb_id = ?1 AND link_text = ?2 AND action = 'aborted' AND created_at > datetime('now', '-1 day')",
                rusqlite::params![kb_id, link.link_text],
                |row| row.get(0),
            )
            .unwrap_or(false);

        if recent_abort {
            return Ok(true);
        }

        // 检查是否有 pending review_item 尚未处理
        let pending_review: bool = conn
            .query_row(
                "SELECT COUNT(*) > 0 FROM link_sanitizer_log lsl
                 INNER JOIN review_items ri ON lsl.review_item_id = ri.id
                 WHERE lsl.kb_id = ?1 AND lsl.link_text = ?2 AND lsl.action = 'ai_completion' AND ri.status = 'pending'",
                rusqlite::params![kb_id, link.link_text],
                |row| row.get(0),
            )
            .unwrap_or(false);

        if pending_review {
            return Ok(true);
        }

        Ok(false)
    }

    /// 写入 link_sanitizer_log
    fn log_action(
        db: &Arc<DatabaseService>,
        kb_id: &str,
        link: &BrokenLink,
        action: &str,
        placeholder_path: &str,
        review_item_id: &str,
        vdb_max_similarity: f64,
        details: &str,
    ) -> Result<(), String> {
        let conn = db.connect()?;
        let log_id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();

        conn.execute(
            "INSERT INTO link_sanitizer_log (id, kb_id, link_text, link_type, source_page_path, action, placeholder_path, review_item_id, vdb_max_similarity, details, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            rusqlite::params![
                log_id, kb_id, link.link_text, link.link_type, link.source_page_path,
                action, placeholder_path, review_item_id, vdb_max_similarity, details, now,
            ],
        )
        .map_err(|e| format!("写入 link_sanitizer_log 失败: {}", e))?;

        Ok(())
    }
}
