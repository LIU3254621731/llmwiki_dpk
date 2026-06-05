use rusqlite::Connection;
use crate::db::schema;

pub fn run_migrations(conn: &Connection) -> Result<(), String> {
    // 创建 migration 追踪表
    conn.execute(
        "CREATE TABLE IF NOT EXISTS schema_migrations (
            version INTEGER PRIMARY KEY,
            applied_at TEXT NOT NULL
        )",
        [],
    )
    .map_err(|e| format!("创建迁移表失败: {}", e))?;

    let current_version = schema::get_current_version(conn)?;

    if current_version < 1 {
        apply_migration(conn, 1)?;
    }

    if current_version < 2 {
        apply_migration(conn, 2)?;
    }

    if current_version < 3 {
        apply_migration(conn, 3)?;
    }

    if current_version < 4 {
        apply_migration(conn, 4)?;
    }

    if current_version < 5 {
        apply_migration(conn, 5)?;
    }

    if current_version < 6 {
        apply_migration(conn, 6)?;
    }

    if current_version < 7 {
        apply_migration(conn, 7)?;
    }

    if current_version < 8 {
        apply_migration(conn, 8)?;
    }

    if current_version < 9 {
        apply_migration(conn, 9)?;
    }

    if current_version < 10 {
        apply_migration(conn, 10)?;
    }

    if current_version < 11 {
        apply_migration(conn, 11)?;
    }

    if current_version < 12 {
        apply_migration(conn, 12)?;
    }

    if current_version < 13 {
        apply_migration(conn, 13)?;
    }

    if current_version < 14 {
        apply_migration(conn, 14)?;
    }

    if current_version < 15 {
        apply_migration(conn, 15)?;
    }

    if current_version < 16 {
        apply_migration(conn, 16)?;
    }

    if current_version < 17 {
        apply_migration(conn, 17)?;
    }

    if current_version < 18 {
        apply_migration(conn, 18)?;
    }

    Ok(())
}

fn apply_migration_v6(conn: &Connection) -> Result<(), String> {
    // v6: 任务状态机增强 - 添加 failure_reason, recoverable, resume_from_stage 等字段

    let task_columns = vec![
        ("failure_reason", "TEXT DEFAULT ''"),
        ("recoverable", "INTEGER DEFAULT 0"),
        ("resume_from_stage", "TEXT DEFAULT ''"),
        ("last_success_stage", "TEXT DEFAULT ''"),
        ("next_action", "TEXT DEFAULT ''"),
        ("archived_at", "TEXT DEFAULT NULL"),
        ("handled_at", "TEXT DEFAULT NULL"),
    ];

    for (col, default) in &task_columns {
        match conn.execute(
            &format!("ALTER TABLE tasks ADD COLUMN {} {}", col, default),
            [],
        ) {
            Ok(_) => log::info!("v6 迁移: 添加 tasks.{} 列", col),
            Err(e) if e.to_string().contains("duplicate column") => {
                log::info!("v6 迁移: 跳过已存在的列 tasks.{}", col);
            }
            Err(e) => log::warn!("v6 迁移: 添加 tasks.{} 列: {}", col, e),
        }
    }

    // 更新旧 failed 任务的 recoverable 状态
    // 如果 error_message 包含 "API Key" 或 "配置"，标记为可恢复
    conn.execute(
        "UPDATE tasks SET recoverable = 1, resume_from_stage = 'source_ingest',
         failure_reason = error_message,
         next_action = 'retry'
         WHERE status = 'failed' AND error_message LIKE '%API Key%'",
        [],
    ).unwrap_or_else(|e| { log::error!("v6 迁移: API Key 任务回填失败: {}", e); 0 });

    conn.execute(
        "UPDATE tasks SET recoverable = 1, failure_reason = error_message,
         next_action = 'retry'
         WHERE status = 'failed' AND error_message LIKE '%超时%'",
        [],
    ).unwrap_or_else(|e| { log::error!("v6 迁移: 超时任务回填失败: {}", e); 0 });

    // 把 cancelled 的任务标记 recoverable=0
    conn.execute(
        "UPDATE tasks SET recoverable = 0, failure_reason = '用户取消'
         WHERE status = 'cancelled' AND COALESCE(failure_reason, '') = ''",
        [],
    ).unwrap_or_else(|e| { log::error!("v6 迁移: cancelled 任务回填失败: {}", e); 0 });

    log::info!("Migration v6 完成");
    Ok(())
}

fn apply_migration_v7(conn: &Connection) -> Result<(), String> {
    // v7: knowledge_items 关联 wiki_pages - 添加 page_id 和 linked_page_path 字段

    for (col, default) in &[
        ("page_id", "TEXT DEFAULT ''"),
        ("linked_page_path", "TEXT DEFAULT ''"),
    ] {
        match conn.execute(
            &format!("ALTER TABLE knowledge_items ADD COLUMN {} {}", col, default),
            [],
        ) {
            Ok(_) => log::info!("v7 迁移: 添加 knowledge_items.{} 列", col),
            Err(e) if e.to_string().contains("duplicate column") => {
                log::info!("v7 迁移: 跳过已存在的列 knowledge_items.{}", col);
            }
            Err(e) => log::warn!("v7 迁移: 添加 knowledge_items.{} 列: {}", col, e),
        }
    }

    // 对于已有 page_path 的 knowledge_items，尝试回填 page_id
    conn.execute(
        "UPDATE knowledge_items SET page_id = (SELECT wp.id FROM wiki_pages wp WHERE wp.kb_id = knowledge_items.kb_id AND wp.path = knowledge_items.page_path AND wp.path != '') WHERE COALESCE(page_id, '') = '' AND COALESCE(page_path, '') != ''",
        [],
    ).unwrap_or_else(|e| { log::error!("v7 迁移: knowledge_items page_id 回填失败: {}", e); 0 });

    // 对于已有 page_path 的 knowledge_items，同步 linked_page_path
    conn.execute(
        "UPDATE knowledge_items SET linked_page_path = page_path WHERE COALESCE(linked_page_path, '') = '' AND COALESCE(page_path, '') != ''",
        [],
    ).unwrap_or_else(|e| { log::error!("v7 迁移: knowledge_items linked_page_path 回填失败: {}", e); 0 });

    log::info!("Migration v7 完成");
    Ok(())
}

fn apply_migration_v8(conn: &Connection) -> Result<(), String> {
    // v8: v0.2.1 - 审阅状态机增强、任务取消机制、知识去重
    // 1. 创建 review_item_events 表（审阅项状态变更日志）
    conn.execute(
        "CREATE TABLE IF NOT EXISTS review_item_events (
            id TEXT PRIMARY KEY,
            review_item_id TEXT NOT NULL,
            old_status TEXT NOT NULL DEFAULT '',
            new_status TEXT NOT NULL DEFAULT '',
            action TEXT NOT NULL DEFAULT '',
            reason TEXT NOT NULL DEFAULT '',
            created_at TEXT NOT NULL,
            FOREIGN KEY (review_item_id) REFERENCES review_items(id)
        )",
        [],
    )
    .map_err(|e| format!("v8: 创建 review_item_events 表失败: {}", e))?;

    // 2. 为 review_items 添加 operation_type 列（与现有 operation 区分）
    match conn.execute(
        "ALTER TABLE review_items ADD COLUMN operation_type TEXT DEFAULT ''",
        [],
    ) {
        Ok(_) => log::info!("v8 迁移: 添加 review_items.operation_type 列"),
        Err(e) if e.to_string().contains("duplicate column") => {}
        Err(e) => log::warn!("v8 迁移: 添加 review_items.operation_type 列: {}", e),
    }

    // 3. 为 tasks 添加取消相关列
    for (col, default) in &[
        ("cancelled_requested_at", "TEXT DEFAULT NULL"),
        ("cancel_reason", "TEXT DEFAULT ''"),
        ("cancel_flag", "INTEGER DEFAULT 0"),
    ] {
        match conn.execute(
            &format!("ALTER TABLE tasks ADD COLUMN {} {}", col, default),
            [],
        ) {
            Ok(_) => log::info!("v8 迁移: 添加 tasks.{} 列", col),
            Err(e) if e.to_string().contains("duplicate column") => {
                log::info!("v8 迁移: 跳过已存在的列 tasks.{}", col);
            }
            Err(e) => log::warn!("v8 迁移: 添加 tasks.{} 列: {}", col, e),
        }
    }

    // 4. 为 knowledge_items 添加 normalized_name 列
    match conn.execute(
        "ALTER TABLE knowledge_items ADD COLUMN normalized_name TEXT DEFAULT ''",
        [],
    ) {
        Ok(_) => log::info!("v8 迁移: 添加 knowledge_items.normalized_name 列"),
        Err(e) if e.to_string().contains("duplicate column") => {}
        Err(e) => log::warn!("v8 迁移: 添加 knowledge_items.normalized_name 列: {}", e),
    }

    // 5. 回填 review_items.operation_type（从 operation 字段映射）
    conn.execute(
        "UPDATE review_items SET operation_type = operation WHERE COALESCE(operation_type, '') = ''",
        [],
    ).unwrap_or_else(|e| { log::error!("v8 迁移: review_items operation_type 回填失败: {}", e); 0 });

    // 6. 为已存在任务回填 cancel_reason（针对 cancelled 状态）
    conn.execute(
        "UPDATE tasks SET cancel_reason = 'user_cancelled' WHERE status = 'cancelled' AND COALESCE(cancel_reason, '') = ''",
        [],
    ).unwrap_or_else(|e| { log::error!("v8 迁移: tasks cancel_reason 回填失败: {}", e); 0 });

    // 7. 回填 knowledge_items.normalized_name
    conn.execute(
        "UPDATE knowledge_items SET normalized_name = LOWER(TRIM(canonical_name)) WHERE COALESCE(normalized_name, '') = ''",
        [],
    ).unwrap_or_else(|e| { log::error!("v8 迁移: knowledge_items normalized_name 回填失败: {}", e); 0 });

    // 8. 创建索引
    for (sql, label) in [
        ("CREATE INDEX IF NOT EXISTS idx_review_item_events_item ON review_item_events(review_item_id)", "review_item_events_item"),
        ("CREATE INDEX IF NOT EXISTS idx_knowledge_items_normalized ON knowledge_items(normalized_name)", "knowledge_items_normalized"),
        ("CREATE INDEX IF NOT EXISTS idx_tasks_cancel_flag ON tasks(cancel_flag)", "tasks_cancel_flag"),
    ] {
        if let Err(e) = conn.execute(sql, []) {
            log::warn!("v8 索引创建失败 ({}): {}", label, e);
        }
    }

    log::info!("Migration v8 完成");
    Ok(())
}

fn apply_migration_v9(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        "CREATE INDEX IF NOT EXISTS idx_reviews_task ON reviews(task_id);",
    )
    .map_err(|e| format!("v9: 创建索引失败: {}", e))?;
    log::info!("Migration v9 完成");
    Ok(())
}

fn apply_migration_v10(conn: &Connection) -> Result<(), String> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS chat_conversations (
            id TEXT PRIMARY KEY,
            kb_id TEXT NOT NULL,
            title TEXT NOT NULL DEFAULT '新对话',
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            FOREIGN KEY (kb_id) REFERENCES knowledge_bases(id)
        )",
        [],
    )
    .map_err(|e| format!("v10: 创建 chat_conversations 表失败: {}", e))?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS chat_messages (
            id TEXT PRIMARY KEY,
            conversation_id TEXT NOT NULL,
            role TEXT NOT NULL,
            content TEXT NOT NULL,
            citations TEXT DEFAULT '',
            created_at TEXT NOT NULL,
            FOREIGN KEY (conversation_id) REFERENCES chat_conversations(id)
        )",
        [],
    )
    .map_err(|e| format!("v10: 创建 chat_messages 表失败: {}", e))?;

    for (sql, label) in [
        ("CREATE INDEX IF NOT EXISTS idx_chat_conversations_kb ON chat_conversations(kb_id)", "chat_conversations_kb"),
        ("CREATE INDEX IF NOT EXISTS idx_chat_messages_conv ON chat_messages(conversation_id)", "chat_messages_conv"),
    ] {
        if let Err(e) = conn.execute(sql, []) {
            log::warn!("v10 索引创建失败 ({}): {}", label, e);
        }
    }

    log::info!("Migration v10 完成");
    Ok(())
}

fn apply_migration_v11(conn: &Connection) -> Result<(), String> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS token_logs (
            id TEXT PRIMARY KEY,
            task_id TEXT NOT NULL DEFAULT '',
            task_name TEXT NOT NULL DEFAULT '',
            agent_name TEXT NOT NULL DEFAULT '',
            input_tokens INTEGER NOT NULL DEFAULT 0,
            output_tokens INTEGER NOT NULL DEFAULT 0,
            model_name TEXT NOT NULL DEFAULT '',
            provider TEXT NOT NULL DEFAULT '',
            created_at TEXT NOT NULL
        )",
        [],
    )
    .map_err(|e| format!("v11: 创建 token_logs 表失败: {}", e))?;

    for (sql, label) in [
        ("CREATE INDEX IF NOT EXISTS idx_token_logs_task ON token_logs(task_id)", "token_logs_task"),
        ("CREATE INDEX IF NOT EXISTS idx_token_logs_date ON token_logs(created_at)", "token_logs_date"),
        ("CREATE INDEX IF NOT EXISTS idx_token_logs_agent ON token_logs(agent_name)", "token_logs_agent"),
    ] {
        if let Err(e) = conn.execute(sql, []) {
            log::warn!("v11 索引创建失败 ({}): {}", label, e);
        }
    }

    log::info!("Migration v11 完成");
    Ok(())
}

fn apply_migration_v14(conn: &Connection) -> Result<(), String> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS canvas_scopes (
            id TEXT PRIMARY KEY,
            kb_id TEXT NOT NULL,
            name TEXT NOT NULL,
            tags_json TEXT NOT NULL DEFAULT '[]',
            last_scroll_position INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            FOREIGN KEY (kb_id) REFERENCES knowledge_bases(id)
        )",
        [],
    )
    .map_err(|e| format!("v14: 创建 canvas_scopes 表失败: {}", e))?;

    if let Err(e) = conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_canvas_scopes_kb ON canvas_scopes(kb_id)",
        [],
    ) {
        log::warn!("v14: idx_canvas_scopes_kb 创建失败: {}", e);
    }

    conn.execute(
        "CREATE TABLE IF NOT EXISTS canvas_cache (
            id TEXT PRIMARY KEY,
            kb_id TEXT NOT NULL,
            cache_key TEXT NOT NULL,
            content_type TEXT NOT NULL,
            topic TEXT NOT NULL DEFAULT '',
            content_json TEXT NOT NULL,
            source_file_ids TEXT NOT NULL DEFAULT '[]',
            total_words INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL,
            FOREIGN KEY (kb_id) REFERENCES knowledge_bases(id)
        )",
        [],
    )
    .map_err(|e| format!("v14: 创建 canvas_cache 表失败: {}", e))?;

    for (sql, label) in [
        ("CREATE INDEX IF NOT EXISTS idx_canvas_cache_kb_key ON canvas_cache(kb_id, cache_key)", "canvas_cache_kb_key"),
        ("CREATE INDEX IF NOT EXISTS idx_canvas_cache_type ON canvas_cache(kb_id, cache_key, content_type)", "canvas_cache_type"),
    ] {
        if let Err(e) = conn.execute(sql, []) {
            log::warn!("v14 索引创建失败 ({}): {}", label, e);
        }
    }

    log::info!("Migration v14 完成");
    Ok(())
}

fn apply_migration_v15(conn: &Connection) -> Result<(), String> {
    // v15: 为 sources 表添加 (kb_id, file_hash) 唯一索引，防止并发竞态导致重复导入

    // 先清理已存在的重复项（保留 rowid 最小的那一条）
    let dup_count = conn.execute(
        "DELETE FROM sources WHERE rowid NOT IN (
            SELECT MIN(rowid) FROM sources GROUP BY kb_id, file_hash
        )",
        [],
    ).unwrap_or_else(|e| {
        log::warn!("v15 migration: cleaning duplicates from sources failed: {}", e);
        0
    });
    if dup_count > 0 {
        log::info!("v15 migration: cleaned {} duplicate sources", dup_count);
    }

    // 创建唯一索引
    conn.execute(
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_sources_kb_file_hash ON sources(kb_id, file_hash)",
        [],
    ).map_err(|e| format!("v15: 创建 sources(kb_id, file_hash) 唯一索引失败: {}", e))?;

    log::info!("Migration v15 完成");
    Ok(())
}

fn apply_migration_v16(conn: &Connection) -> Result<(), String> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS canvas_snapshots (
            id TEXT PRIMARY KEY,
            kb_id TEXT NOT NULL,
            canvas_type TEXT NOT NULL,
            canvas_id TEXT NOT NULL,
            schema_json TEXT NOT NULL,
            metadata_json TEXT NOT NULL DEFAULT '{}',
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            FOREIGN KEY (kb_id) REFERENCES knowledge_bases(id),
            UNIQUE(kb_id, canvas_type, canvas_id)
        )",
        [],
    )
    .map_err(|e| format!("v16: 创建 canvas_snapshots 表失败: {}", e))?;

    for (sql, label) in [
        ("CREATE INDEX IF NOT EXISTS idx_canvas_snapshots_kb ON canvas_snapshots(kb_id)", "canvas_snapshots_kb"),
        ("CREATE INDEX IF NOT EXISTS idx_canvas_snapshots_type ON canvas_snapshots(kb_id, canvas_type)", "canvas_snapshots_type"),
    ] {
        if let Err(e) = conn.execute(sql, []) {
            log::warn!("v16 索引创建失败 ({}): {}", label, e);
        }
    }

    log::info!("Migration v16 完成");
    Ok(())
}

fn apply_migration_v17(conn: &Connection) -> Result<(), String> {
    // v17: Agent/Skill 管理系统 — agent_definitions + skill_definitions 表

    conn.execute(
        "CREATE TABLE IF NOT EXISTS agent_definitions (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL UNIQUE,
            role TEXT NOT NULL DEFAULT 'custom',
            trigger_event TEXT NOT NULL DEFAULT 'manual',
            system_prompt TEXT NOT NULL DEFAULT '',
            allowed_skills TEXT NOT NULL DEFAULT '[]',
            status TEXT NOT NULL DEFAULT 'active',
            max_depth INTEGER NOT NULL DEFAULT 5,
            timeout_secs INTEGER NOT NULL DEFAULT 120,
            metadata_json TEXT NOT NULL DEFAULT '{}',
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        )",
        [],
    )
    .map_err(|e| format!("v17: 创建 agent_definitions 表失败: {}", e))?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS skill_definitions (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL UNIQUE,
            description TEXT NOT NULL DEFAULT '',
            code_body TEXT NOT NULL DEFAULT '',
            parameter_schema TEXT NOT NULL DEFAULT '{}',
            skill_type TEXT NOT NULL DEFAULT 'prompt',
            status TEXT NOT NULL DEFAULT 'active',
            metadata_json TEXT NOT NULL DEFAULT '{}',
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        )",
        [],
    )
    .map_err(|e| format!("v17: 创建 skill_definitions 表失败: {}", e))?;

    // 创建索引
    for (sql, label) in [
        ("CREATE INDEX IF NOT EXISTS idx_agent_defs_name ON agent_definitions(name)", "agent_defs_name"),
        ("CREATE INDEX IF NOT EXISTS idx_agent_defs_status ON agent_definitions(status)", "agent_defs_status"),
        ("CREATE INDEX IF NOT EXISTS idx_agent_defs_trigger ON agent_definitions(trigger_event)", "agent_defs_trigger"),
        ("CREATE INDEX IF NOT EXISTS idx_skill_defs_name ON skill_definitions(name)", "skill_defs_name"),
        ("CREATE INDEX IF NOT EXISTS idx_skill_defs_type ON skill_definitions(skill_type)", "skill_defs_type"),
    ] {
        if let Err(e) = conn.execute(sql, []) {
            log::warn!("v17 索引创建失败 ({}): {}", label, e);
        }
    }

    // 种子数据：7 个系统 Agent
    let now = chrono::Utc::now().to_rfc3339();
    let system_agents = [
        ("coordinator", "CoordinatorAgent", "orchestrator", "source_ingested",
         "你是 LLMWiki 的任务协调调度器。你负责接收系统事件，按顺序调度各个子 Agent 执行知识提取流水线：文档解析 → 实体消歧 → 关系发现 → Wiki 更新。你只在流水线需要推进时被触发，不主动生成内容。",
         r#"["DocumentProcessor","WebSearchSkill"]"#),
        ("source_ingest", "SourceIngestAgent", "ingestor", "manual",
         "你是 LLMWiki 的文档解析专家。你的职责是读取用户上传的文档，提取其中的实体、概念、声明和主题，并生成结构化的知识条目（JSON 格式）。你需要关注信息的完整性、准确性和去重。",
         r#"["DocumentProcessor","PdfSkill","DocxSkill","HtmlSkill","MdSkill","TxtSkill","MarkitdownSkill"]"#),
        ("resolution", "ResolutionAgent", "resolver", "ingest_completed",
         "你是 LLMWiki 的实体消歧专家。你的职责是将新提取的知识条目与已有 Wiki 页面进行匹配和消歧，判断每个条目是属于新实体、已有实体的别名、还是对已有实体的补充。你需要输出清晰的消歧决策。",
         "[]"),
        ("relationship", "RelationshipAgent", "connector", "resolution_completed",
         "你是 LLMWiki 的关系发现专家。你的职责是分析消歧后的知识条目，发现实体之间的语义关系（如：描述、使用、是一种、拥有、关联、实例化、矛盾、支持、反驳、组成部分、派生自、引用），并以结构化 JSON 输出关系列表。",
         "[]"),
        ("wiki_update", "WikiUpdateAgent", "writer", "relationship_completed",
         "你是 LLMWiki 的 Wiki 更新专家。你的职责是基于消歧和关系发现的结果，生成 Wiki 页面的更新计划。你需要决定哪些页面需要新建、修改或合并，并为每个变更编写具体的修改内容。",
         "[]"),
        ("health_check", "HealthCheckAgent", "diagnostician", "manual",
         "你是 LLMWiki 的知识库健康诊断专家。你的职责是扫描知识库的一致性，检查断链接、孤立节点、未处理审阅、失败任务等问题，并生成结构化的健康报告和修复建议。",
         "[]"),
        ("query", "QueryAgent", "assistant", "manual",
         "你是 LLMWiki 的智能问答助手。你基于知识库中的 Wiki 页面内容回答用户的问题，提供带有引用的准确答案。当知识库中没有相关信息时，你需要诚实告知用户。",
         r#"["WebSearchSkill"]"#),
    ];

    for (id, name, role, trigger, prompt, skills) in &system_agents {
        if let Err(e) = conn.execute(
            "INSERT OR IGNORE INTO agent_definitions (id, name, role, trigger_event, system_prompt, allowed_skills, status, max_depth, timeout_secs, metadata_json, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'active', 5, 120, '{}', ?7, ?7)",
            rusqlite::params![id, name, role, trigger, prompt, skills, now],
        ) {
            log::warn!("v17: 插入种子 Agent '{}' 失败: {}", name, e);
        }
    }

    // 种子数据：10 个系统 Skill
    let system_skills: &[(&str, &str, &str, &str, &str)] = &[
        ("document_processor", "DocumentProcessor", "统一文档处理分发器，根据文件类型自动选择合适的解析器", "transform",
         r#"{"type":"transform","function":"document_processor","description":"根据文件扩展名自动分发到对应的文档解析技能"}"#),
        ("pdf_skill", "PdfSkill", "使用 lopdf 库提取 PDF 文件中的文本内容", "transform",
         r#"{"type":"transform","function":"pdf_extract_text","description":"从PDF文件中提取纯文本内容"}"#),
        ("docx_skill", "DocxSkill", "解析 DOCX 文件并提取格式化文本", "transform",
         r#"{"type":"transform","function":"docx_extract_text","description":"从DOCX文件中提取文本内容"}"#),
        ("html_skill", "HtmlSkill", "解析 HTML 文件并提取纯文本内容", "transform",
         r#"{"type":"transform","function":"html_extract_text","description":"从HTML文件中提取纯文本，移除标签"}"#),
        ("md_skill", "MdSkill", "读取 Markdown 文件内容", "transform",
         r#"{"type":"transform","function":"md_read_text","description":"读取Markdown文件原始内容"}"#),
        ("txt_skill", "TxtSkill", "读取纯文本文件内容", "transform",
         r#"{"type":"transform","function":"txt_read_text","description":"读取纯文本文件内容"}"#),
        ("markitdown_skill", "MarkitdownSkill", "使用 Microsoft MarkItDown (Python) 将 Office 文档转换为 Markdown", "transform",
         r#"{"type":"transform","function":"markitdown_convert","description":"通过Python MarkItDown将XLSX/CSV/JSON/XML等格式转换为Markdown"}"#),
        ("pdf_ocr", "PdfOcrSkill", "使用 Windows OCR 引擎识别扫描版 PDF 中的文字", "transform",
         r#"{"type":"transform","function":"pdf_ocr_extract","description":"对扫描版PDF使用Windows OCR进行文字识别"}"#),
        ("pptx_skill", "PptxSkill", "解析 PPTX 文件并提取文本内容", "transform",
         r#"{"type":"transform","function":"pptx_extract_text","description":"从PPTX文件中提取文本内容"}"#),
        ("web_search_skill", "WebSearchSkill", "多引擎网页搜索（DuckDuckGo/SearXNG/Brave/Bing）", "transform",
         r#"{"type":"transform","function":"web_search","description":"通过多引擎网页搜索获取外部信息"}"#),
    ];

    for (id, name, desc, skill_type, code_body) in system_skills {
        if let Err(e) = conn.execute(
            "INSERT OR IGNORE INTO skill_definitions (id, name, description, code_body, parameter_schema, skill_type, status, metadata_json, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, '{}', ?5, 'active', '{}', ?6, ?6)",
            rusqlite::params![id, name, desc, code_body, skill_type, now],
        ) {
            log::warn!("v17: 插入种子 Skill '{}' 失败: {}", name, e);
        }
    }

    log::info!("Migration v17 完成");
    Ok(())
}

fn apply_migration_v18(conn: &Connection) -> Result<(), String> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS link_sanitizer_log (
            id TEXT PRIMARY KEY,
            kb_id TEXT NOT NULL,
            link_text TEXT NOT NULL,
            link_type TEXT NOT NULL DEFAULT 'wikilink',
            source_page_path TEXT NOT NULL,
            action TEXT NOT NULL,
            placeholder_path TEXT DEFAULT '',
            review_item_id TEXT DEFAULT '',
            vdb_max_similarity REAL DEFAULT 0.0,
            details TEXT DEFAULT '',
            created_at TEXT NOT NULL,
            FOREIGN KEY (kb_id) REFERENCES knowledge_bases(id)
        )",
        [],
    )
    .map_err(|e| format!("v18: 创建 link_sanitizer_log 表失败: {}", e))?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_link_sanitizer_log_kb ON link_sanitizer_log(kb_id)",
        [],
    )
    .map_err(|e| format!("v18: 创建索引失败: {}", e))?;

    log::info!("Migration v18 完成");
    Ok(())
}

fn apply_migration(conn: &Connection, version: i32) -> Result<(), String> {
    match version {
        1 => {
            log::info!("执行 migration v1: 初始化所有表结构");
            schema::initialize_tables(conn).map_err(|e| format!("创建表失败: {}", e))?;
            schema::create_indexes(conn).map_err(|e| format!("创建索引失败: {}", e))?;
        }
        2 => {
            log::info!("执行 migration v2: 增强审阅项和页面字段");
            apply_migration_v2(conn)?;
        }
        3 => {
            log::info!("执行 migration v3: 修复 wiki/wiki 路径重复 + 审阅状态增强");
            apply_migration_v3(conn)?;
        }
        4 => {
            log::info!("执行 migration v4: v0.1.4 更新 - Source Preview / 文件索引 / 思维导图缓存");
            apply_migration_v4(conn)?;
        }
        5 => {
            log::info!("Running migration v5: wiki/review consistency closure");
            apply_migration_v5(conn)?;
        }
        6 => {
            log::info!("执行 migration v6: 任务状态机增强 - failure_reason/recoverable/handled/archived");
            apply_migration_v6(conn)?;
        }
        7 => {
            log::info!("执行 migration v7: knowledge_items 添加 page_id 关联字段");
            apply_migration_v7(conn)?;
        }
        8 => {
            log::info!("执行 migration v8: v0.2.1 审阅状态机/任务取消/知识去重基础结构");
            apply_migration_v8(conn)?;
        }
        9 => {
            log::info!("执行 migration v9: 添加性能索引 (reviews.task_id)");
            apply_migration_v9(conn)?;
        }
        10 => {
            log::info!("执行 migration v10: 创建对话历史表 (chat_conversations + chat_messages)");
            apply_migration_v10(conn)?;
        }
        11 => {
            log::info!("执行 migration v11: 创建 Token 消耗日志表 (token_logs)");
            apply_migration_v11(conn)?;
        }
        12 => {
            log::info!("执行 migration v12: 创建向量数据库块表 (vdb_chunks)");
            apply_migration_v12(conn)?;
        }
        13 => {
            log::info!("执行 migration v13: 添加任务名称、审阅关联及源文件元数据列");
            apply_migration_v13(conn)?;
        }
        14 => {
            log::info!("执行 migration v14: 创建 Canvas 画布缓存和范围书签表");
            apply_migration_v14(conn)?;
        }
        15 => {
            log::info!("执行 migration v15: 为 sources 表添加 kb_id+file_hash 唯一约束");
            apply_migration_v15(conn)?;
        }
        16 => {
            log::info!("执行 migration v16: 创建 canvas_snapshots 画布持久化表");
            apply_migration_v16(conn)?;
        }
        17 => {
            log::info!("执行 migration v17: Agent/Skill 管理系统表");
            apply_migration_v17(conn)?;
        }
        18 => {
            log::info!("执行 migration v18: LinkSanitizer 死链追踪表");
            apply_migration_v18(conn)?;
        }
        _ => {
            return Err(format!("未知的 migration 版本: {}", version));
        }
    }

    // 记录 migration
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO schema_migrations (version, applied_at) VALUES (?1, ?2)",
        rusqlite::params![version, now],
    )
    .map_err(|e| format!("记录迁移失败: {}", e))?;

    log::info!("Migration v{} 完成", version);
    Ok(())
}

fn apply_migration_v12(conn: &Connection) -> Result<(), String> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS vdb_chunks (
            id TEXT PRIMARY KEY,
            kb_id TEXT NOT NULL,
            chunk_text TEXT NOT NULL,
            embedding_json TEXT NOT NULL,
            source_id TEXT DEFAULT '',
            page_path TEXT DEFAULT '',
            chunk_index INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL,
            FOREIGN KEY (kb_id) REFERENCES knowledge_bases(id)
        )",
        [],
    )
    .map_err(|e| format!("v12: 创建 vdb_chunks 表失败: {}", e))?;

    for (sql, label) in [
        ("CREATE INDEX IF NOT EXISTS idx_vdb_chunks_kb ON vdb_chunks(kb_id)", "vdb_chunks_kb"),
        ("CREATE INDEX IF NOT EXISTS idx_vdb_chunks_source ON vdb_chunks(source_id)", "vdb_chunks_source"),
        ("CREATE INDEX IF NOT EXISTS idx_vdb_chunks_page ON vdb_chunks(page_path)", "vdb_chunks_page"),
    ] {
        if let Err(e) = conn.execute(sql, []) {
            log::warn!("v12 索引创建失败 ({}): {}", label, e);
        }
    }

    log::info!("Migration v12 完成");
    Ok(())
}

fn apply_migration_v13(conn: &Connection) -> Result<(), String> {
    // 为 sources 表添加元数据列
    for (col, default) in &[
        ("text_length", "INTEGER DEFAULT 0"),
        ("page_count", "INTEGER DEFAULT NULL"),
    ] {
        match conn.execute(
            &format!("ALTER TABLE sources ADD COLUMN {} {}", col, default),
            [],
        ) {
            Ok(_) => log::info!("v13 迁移: 添加 sources.{} 列", col),
            Err(e) if e.to_string().contains("duplicate column") => {
                log::info!("v13 迁移: sources.{} 列已存在，跳过", col);
            }
            Err(e) => log::warn!("v13 迁移: 添加 sources.{} 列失败: {}", col, e),
        }
    }

    // 为 tasks 表添加任务名称和审阅关联列
    for (col, default) in &[
        ("task_name", "TEXT DEFAULT ''"),
        ("review_id", "TEXT DEFAULT ''"),
    ] {
        match conn.execute(
            &format!("ALTER TABLE tasks ADD COLUMN {} {}", col, default),
            [],
        ) {
            Ok(_) => log::info!("v13 迁移: 添加 tasks.{} 列", col),
            Err(e) if e.to_string().contains("duplicate column") => {
                log::info!("v13 迁移: tasks.{} 列已存在，跳过", col);
            }
            Err(e) => log::warn!("v13 迁移: 添加 tasks.{} 列失败: {}", col, e),
        }
    }

    // 回填 sources.text_length
    if let Err(e) = conn.execute(
        "UPDATE sources SET text_length = LENGTH(COALESCE(extracted_text, '')) WHERE text_length = 0",
        [],
    ) {
        log::warn!("v13 迁移: sources.text_length 回填失败: {}", e);
    }

    // 回填 tasks.task_name（从 source.file_name 推导）
    if let Err(e) = conn.execute(
        "UPDATE tasks SET task_name = (
            SELECT s.file_name FROM sources s WHERE s.id = tasks.input_ref
        ) WHERE task_name = '' AND input_ref IS NOT NULL AND input_ref != ''",
        [],
    ) {
        log::warn!("v13 迁移: tasks.task_name 回填失败: {}", e);
    }

    log::info!("Migration v13 完成");
    Ok(())
}

fn apply_migration_v2(conn: &Connection) -> Result<(), String> {
    // 为 review_items 添加增强字段
    let columns_to_add = vec![
        "ALTER TABLE review_items ADD COLUMN reason TEXT DEFAULT ''",
        "ALTER TABLE review_items ADD COLUMN source_id TEXT DEFAULT ''",
        "ALTER TABLE review_items ADD COLUMN citation_status TEXT DEFAULT 'uncited'",
        "ALTER TABLE review_items ADD COLUMN summary TEXT DEFAULT ''",
        "ALTER TABLE review_items ADD COLUMN confidence TEXT DEFAULT 'medium'",
    ];

    for sql in &columns_to_add {
        match conn.execute(sql, []) {
            Ok(_) => log::info!("v2 迁移: 成功执行 {}", sql),
            Err(e) => {
                if e.to_string().contains("duplicate column") {
                    log::info!("v2 迁移: 跳过已存在的列 - {}", sql);
                } else {
                    log::warn!("v2 迁移警告: {} - {}", sql, e);
                }
            }
        }
    }

    // 为 review_items 添加索引
    if let Err(e) = conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_review_items_status ON review_items(status)",
        [],
    ) {
        log::warn!("v2 迁移: idx_review_items_status 创建失败: {}", e);
    }

    if let Err(e) = conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_review_items_source ON review_items(source_id)",
        [],
    ) {
        log::warn!("v2 迁移: idx_review_items_source 创建失败: {}", e);
    }

    // 为 wiki_pages 添加 status 列（如果不存在）
    match conn.execute(
        "ALTER TABLE wiki_pages ADD COLUMN status TEXT DEFAULT 'active'",
        [],
    ) {
        Ok(_) => {}
        Err(e) if e.to_string().contains("duplicate column") => {}
        Err(e) => log::warn!("添加 wiki_pages.status 列: {}", e),
    }

    log::info!("Migration v2 完成");
    Ok(())
}

fn apply_migration_v3(conn: &Connection) -> Result<(), String> {
    // v3: 修复 wiki/wiki 路径重复 + 审阅状态增强 + 缺失列补充

    // 1. 修复 wiki_pages 中 wiki/wiki 重复路径
    {
        let mut stmt = conn.prepare("SELECT id, path FROM wiki_pages WHERE path LIKE 'wiki/wiki/%'")
            .map_err(|e| format!("v3: 查询 wiki_pages 失败: {}", e))?;
        let to_fix: Vec<(String, String)> = stmt
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
            .map_err(|e| format!("v3: 映射 wiki_pages 失败: {}", e))?
            .filter_map(|r| r.ok())
            .collect();

        let mut fixed = 0usize;
        for (id, path) in &to_fix {
            let repaired = path.replacen("wiki/wiki/", "wiki/", 1);
            conn.execute("UPDATE wiki_pages SET path = ?1 WHERE id = ?2",
                rusqlite::params![repaired, id],
            ).map_err(|e| format!("v3: 修复 wiki_pages 路径失败: {}", e))?;
            fixed += 1;
        }
        if fixed > 0 {
            log::info!("v3 迁移: 修复了 {} 个 wiki_pages 的 wiki/wiki 重复路径", fixed);
        }
    }

    // 2. 修复 review_items 中的 wiki/wiki 路径
    {
        let mut stmt = conn.prepare("SELECT id, target_path FROM review_items WHERE target_path LIKE 'wiki/wiki/%'")
            .map_err(|e| format!("v3: 查询 review_items 失败: {}", e))?;
        let to_fix: Vec<(String, String)> = stmt
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
            .map_err(|e| format!("v3: 映射 review_items 失败: {}", e))?
            .filter_map(|r| r.ok())
            .collect();

        let mut fixed = 0usize;
        for (id, path) in &to_fix {
            let repaired = path.replacen("wiki/wiki/", "wiki/", 1);
            conn.execute("UPDATE review_items SET target_path = ?1 WHERE id = ?2",
                rusqlite::params![repaired, id],
            ).map_err(|e| format!("v3: 修复 review_items 路径失败: {}", e))?;
            fixed += 1;
        }
        if fixed > 0 {
            log::info!("v3 迁移: 修复了 {} 个 review_items 的 wiki/wiki 重复路径", fixed);
        }
    }

    // 3. 修复 versions 中的 wiki/wiki 路径
    {
        let mut stmt = conn.prepare("SELECT id, page_path FROM versions WHERE page_path LIKE 'wiki/wiki/%'")
            .map_err(|e| format!("v3: 查询 versions 失败: {}", e))?;
        let to_fix: Vec<(String, String)> = stmt
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
            .map_err(|e| format!("v3: 映射 versions 失败: {}", e))?
            .filter_map(|r| r.ok())
            .collect();

        let mut fixed = 0usize;
        for (id, path) in &to_fix {
            let repaired = path.replacen("wiki/wiki/", "wiki/", 1);
            conn.execute("UPDATE versions SET page_path = ?1 WHERE id = ?2",
                rusqlite::params![repaired, id],
            ).map_err(|e| format!("v3: 修复 versions 路径失败: {}", e))?;
            fixed += 1;
        }
        if fixed > 0 {
            log::info!("v3 迁移: 修复了 {} 个 versions 的 wiki/wiki 重复路径", fixed);
        }
    }

    // 4. 为 sources 添加 ai_summary 和 coverage_report 列
    for (col, default) in &[
        ("ai_summary", "TEXT DEFAULT ''"),
        ("coverage_report", "TEXT DEFAULT ''"),
    ] {
        match conn.execute(
            &format!("ALTER TABLE sources ADD COLUMN {} {}", col, default),
            [],
        ) {
            Ok(_) => log::info!("v3 迁移: 添加 sources.{} 列", col),
            Err(e) if e.to_string().contains("duplicate column") => {}
            Err(e) => log::warn!("v3 迁移: 添加 sources.{} 列: {}", col, e),
        }
    }

    // 5. 为 knowledge_items 添加 source_id 列
    match conn.execute(
        "ALTER TABLE knowledge_items ADD COLUMN source_id TEXT DEFAULT ''",
        [],
    ) {
        Ok(_) => log::info!("v3 迁移: 添加 knowledge_items.source_id 列"),
        Err(e) if e.to_string().contains("duplicate column") => {}
        Err(e) => log::warn!("v3 迁移: 添加 source_id 列: {}", e),
    }

    log::info!("Migration v3 完成");
    Ok(())
}

fn apply_migration_v4(conn: &Connection) -> Result<(), String> {
    // v4: v0.1.4 更新 - 增加 source preview、文件索引、思维导图缓存等

    // 1. 为 sources 表增加 source preview 相关列
    for (col, default) in &[
        ("preview_path", "TEXT DEFAULT ''"),
        ("preview_status", "TEXT DEFAULT ''"),
        ("preview_generated_at", "TEXT DEFAULT ''"),
        ("preview_error", "TEXT DEFAULT ''"),
        ("summary_json_path", "TEXT DEFAULT ''"),
        ("coverage_json_path", "TEXT DEFAULT ''"),
        ("linked_pages_count", "INTEGER DEFAULT 0"),
        ("linked_relations_count", "INTEGER DEFAULT 0"),
    ] {
        match conn.execute(
            &format!("ALTER TABLE sources ADD COLUMN {} {}", col, default),
            [],
        ) {
            Ok(_) => log::info!("v4 迁移: 添加 sources.{} 列", col),
            Err(e) if e.to_string().contains("duplicate column") => {}
            Err(e) => log::warn!("v4 迁移: 添加 sources.{} 列: {}", col, e),
        }
    }

    // 2. 创建 source_previews 表
    conn.execute(
        "CREATE TABLE IF NOT EXISTS source_previews (
            id TEXT PRIMARY KEY,
            source_id TEXT NOT NULL,
            kb_id TEXT NOT NULL,
            preview_path TEXT NOT NULL DEFAULT '',
            preview_status TEXT NOT NULL DEFAULT 'generated',
            content_hash TEXT NOT NULL DEFAULT '',
            generated_at TEXT NOT NULL,
            error_message TEXT DEFAULT '',
            FOREIGN KEY (source_id) REFERENCES sources(id),
            FOREIGN KEY (kb_id) REFERENCES knowledge_bases(id)
        )",
        [],
    )
    .map_err(|e| format!("v4: 创建 source_previews 表失败: {}", e))?;

    // 3. 创建 file_index 表
    conn.execute(
        "CREATE TABLE IF NOT EXISTS file_index (
            id TEXT PRIMARY KEY,
            kb_id TEXT NOT NULL,
            relative_path TEXT NOT NULL,
            file_name TEXT NOT NULL,
            file_type TEXT NOT NULL DEFAULT '',
            file_size INTEGER NOT NULL DEFAULT 0,
            content_hash TEXT NOT NULL DEFAULT '',
            modified_at TEXT NOT NULL,
            record_type TEXT NOT NULL DEFAULT 'unknown',
            linked_record_id TEXT DEFAULT '',
            status TEXT NOT NULL DEFAULT 'ok',
            FOREIGN KEY (kb_id) REFERENCES knowledge_bases(id)
        )",
        [],
    )
    .map_err(|e| format!("v4: 创建 file_index 表失败: {}", e))?;

    // 4. 创建 mind_map_cache 表
    conn.execute(
        "CREATE TABLE IF NOT EXISTS mind_map_cache (
            id TEXT PRIMARY KEY,
            kb_id TEXT NOT NULL,
            center_node_id TEXT NOT NULL DEFAULT '',
            dimension_config TEXT NOT NULL DEFAULT '{}',
            depth INTEGER NOT NULL DEFAULT 2,
            cache_json_path TEXT NOT NULL DEFAULT '',
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            FOREIGN KEY (kb_id) REFERENCES knowledge_bases(id)
        )",
        [],
    )
    .map_err(|e| format!("v4: 创建 mind_map_cache 表失败: {}", e))?;

    // 5. 为 graph_nodes 增加新列
    for (col, default) in &[
        ("aliases", "TEXT DEFAULT ''"),
        ("tags", "TEXT DEFAULT ''"),
        ("summary", "TEXT DEFAULT ''"),
        ("source_count", "INTEGER DEFAULT 0"),
        ("in_degree", "INTEGER DEFAULT 0"),
        ("out_degree", "INTEGER DEFAULT 0"),
        ("status", "TEXT DEFAULT 'active'"),
        ("source_id", "TEXT DEFAULT ''"),
        ("page_id", "TEXT DEFAULT ''"),
        ("confidence", "TEXT DEFAULT 'medium'"),
        ("created_at", "TEXT DEFAULT ''"),
    ] {
        match conn.execute(
            &format!("ALTER TABLE graph_nodes ADD COLUMN {} {}", col, default),
            [],
        ) {
            Ok(_) => log::info!("v4 迁移: 添加 graph_nodes.{} 列", col),
            Err(e) if e.to_string().contains("duplicate column") => {}
            Err(e) => log::warn!("v4 迁移: 添加 graph_nodes.{} 列: {}", col, e),
        }
    }

    // 6. 为 graph_edges 增加新列
    for (col, default) in &[
        ("relation", "TEXT DEFAULT 'related_to'"),
        ("confidence", "TEXT DEFAULT 'medium'"),
        ("evidence_source_id", "TEXT DEFAULT ''"),
        ("evidence_location", "TEXT DEFAULT ''"),
        ("citation_status", "TEXT DEFAULT 'uncited'"),
        ("created_by_task", "TEXT DEFAULT ''"),
        ("created_at", "TEXT DEFAULT ''"),
    ] {
        match conn.execute(
            &format!("ALTER TABLE graph_edges ADD COLUMN {} {}", col, default),
            [],
        ) {
            Ok(_) => log::info!("v4 迁移: 添加 graph_edges.{} 列", col),
            Err(e) if e.to_string().contains("duplicate column") => {}
            Err(e) => log::warn!("v4 迁移: 添加 graph_edges.{} 列: {}", col, e),
        }
    }

    // 7. 为 tasks 表增加新列
    match conn.execute(
        "ALTER TABLE tasks ADD COLUMN locked_at TEXT DEFAULT NULL",
        [],
    ) {
        Ok(_) => log::info!("v4 迁移: 添加 tasks.locked_at 列"),
        Err(e) if e.to_string().contains("duplicate column") => {}
        Err(e) => log::warn!("v4 迁移: 添加 tasks.locked_at 列: {}", e),
    }

    // 8. 创建索引
    for (sql, label) in [
        ("CREATE INDEX IF NOT EXISTS idx_file_index_kb ON file_index(kb_id)", "file_index_kb"),
        ("CREATE INDEX IF NOT EXISTS idx_file_index_type ON file_index(record_type)", "file_index_type"),
        ("CREATE INDEX IF NOT EXISTS idx_source_previews_source ON source_previews(source_id)", "source_previews_source"),
        ("CREATE INDEX IF NOT EXISTS idx_mind_map_cache_kb ON mind_map_cache(kb_id)", "mind_map_cache_kb"),
        ("CREATE INDEX IF NOT EXISTS idx_graph_nodes_source ON graph_nodes(source_id)", "graph_nodes_source"),
        ("CREATE INDEX IF NOT EXISTS idx_graph_edges_confidence ON graph_edges(confidence)", "graph_edges_confidence"),
    ] {
        if let Err(e) = conn.execute(sql, []) {
            log::warn!("v4 索引创建失败 ({}): {}", label, e);
        }
    }

    log::info!("Migration v4 完成");
    Ok(())
}

fn apply_migration_v5(conn: &Connection) -> Result<(), String> {
    for (col, default) in &[
        ("title", "TEXT DEFAULT ''"),
        ("page_type", "TEXT DEFAULT ''"),
        ("apply_error", "TEXT DEFAULT ''"),
        ("metadata_json", "TEXT DEFAULT '{}'"),
    ] {
        match conn.execute(
            &format!("ALTER TABLE review_items ADD COLUMN {} {}", col, default),
            [],
        ) {
            Ok(_) => log::info!("v5 migration: added review_items.{}", col),
            Err(e) if e.to_string().contains("duplicate column") => {}
            Err(e) => log::warn!("v5 migration: adding review_items.{} failed: {}", col, e),
        }
    }

    if let Err(e) = conn.execute(
        "UPDATE review_items
         SET page_type = CASE
             WHEN target_path LIKE 'wiki/entities/%' THEN 'entity'
             WHEN target_path LIKE 'wiki/topics/%' THEN 'topic'
             WHEN target_path LIKE 'wiki/questions/%' THEN 'question'
             WHEN target_path LIKE 'wiki/reviews/%' THEN 'review'
             WHEN target_path LIKE 'wiki/sources/%' THEN 'source'
             WHEN target_path LIKE 'wiki/datasets/%' THEN 'dataset'
             WHEN target_path LIKE 'wiki/methods/%' THEN 'method'
             ELSE 'concept'
         END
         WHERE COALESCE(page_type, '') = ''",
        [],
    ) {
        log::warn!("v5 迁移: review_items page_type 回填失败: {}", e);
    }

    for sql in [
        "CREATE INDEX IF NOT EXISTS idx_wiki_pages_kb_path ON wiki_pages(kb_id, path)",
        "CREATE INDEX IF NOT EXISTS idx_knowledge_items_kb_canonical ON knowledge_items(kb_id, canonical_name)",
        "CREATE INDEX IF NOT EXISTS idx_review_items_status ON review_items(status)",
        "CREATE INDEX IF NOT EXISTS idx_graph_nodes_kb_type ON graph_nodes(kb_id, node_type)",
    ] {
        conn.execute(sql, [])
            .map_err(|e| format!("v5 migration: creating index failed: {}", e))?;
    }

    Ok(())
}
