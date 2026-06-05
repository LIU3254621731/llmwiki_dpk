use rusqlite::Connection;

pub fn get_current_version(conn: &Connection) -> Result<i32, String> {
    match conn.query_row(
        "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
        [],
        |row| row.get(0),
    ) {
        Ok(v) => Ok(v),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(0),
        Err(e) => Err(format!("查询 schema 版本失败: {}", e)),
    }
}

pub fn initialize_tables(conn: &Connection) -> Result<(), rusqlite::Error> {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS knowledge_bases (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            path TEXT NOT NULL,
            template_name TEXT NOT NULL DEFAULT 'general',
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS sources (
            id TEXT PRIMARY KEY,
            kb_id TEXT NOT NULL,
            file_name TEXT NOT NULL,
            file_path TEXT NOT NULL,
            file_type TEXT NOT NULL,
            file_size INTEGER NOT NULL DEFAULT 0,
            file_hash TEXT NOT NULL DEFAULT '',
            extracted_text TEXT DEFAULT '',
            status TEXT NOT NULL DEFAULT 'pending',
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            FOREIGN KEY (kb_id) REFERENCES knowledge_bases(id)
        );

        CREATE TABLE IF NOT EXISTS assets (
            id TEXT PRIMARY KEY,
            kb_id TEXT NOT NULL,
            file_name TEXT NOT NULL,
            file_path TEXT NOT NULL,
            file_type TEXT NOT NULL,
            file_size INTEGER NOT NULL DEFAULT 0,
            file_hash TEXT NOT NULL DEFAULT '',
            created_at TEXT NOT NULL,
            FOREIGN KEY (kb_id) REFERENCES knowledge_bases(id)
        );

        CREATE TABLE IF NOT EXISTS wiki_pages (
            id TEXT PRIMARY KEY,
            kb_id TEXT NOT NULL,
            title TEXT NOT NULL,
            path TEXT NOT NULL,
            page_type TEXT NOT NULL DEFAULT 'concept',
            canonical_name TEXT NOT NULL,
            tags TEXT DEFAULT '',
            content_hash TEXT NOT NULL DEFAULT '',
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            FOREIGN KEY (kb_id) REFERENCES knowledge_bases(id)
        );

        CREATE TABLE IF NOT EXISTS knowledge_items (
            id TEXT PRIMARY KEY,
            kb_id TEXT NOT NULL,
            canonical_name TEXT NOT NULL,
            item_type TEXT NOT NULL DEFAULT 'entity',
            page_path TEXT DEFAULT '',
            page_id TEXT DEFAULT '',
            linked_page_path TEXT DEFAULT '',
            summary TEXT DEFAULT '',
            source_id TEXT DEFAULT '',
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            FOREIGN KEY (kb_id) REFERENCES knowledge_bases(id)
        );

        CREATE TABLE IF NOT EXISTS aliases (
            id TEXT PRIMARY KEY,
            item_id TEXT NOT NULL,
            alias TEXT NOT NULL,
            normalized_alias TEXT NOT NULL,
            language TEXT DEFAULT 'unknown',
            created_at TEXT NOT NULL,
            FOREIGN KEY (item_id) REFERENCES knowledge_items(id)
        );

        CREATE TABLE IF NOT EXISTS relationships (
            id TEXT PRIMARY KEY,
            kb_id TEXT NOT NULL,
            source_item_id TEXT NOT NULL,
            target_item_id TEXT NOT NULL,
            relation TEXT NOT NULL,
            evidence_source_id TEXT DEFAULT '',
            evidence_location TEXT DEFAULT '',
            confidence TEXT DEFAULT 'medium',
            status TEXT NOT NULL DEFAULT 'active',
            created_at TEXT NOT NULL,
            FOREIGN KEY (kb_id) REFERENCES knowledge_bases(id)
        );

        CREATE TABLE IF NOT EXISTS tasks (
            id TEXT PRIMARY KEY,
            kb_id TEXT NOT NULL,
            task_type TEXT NOT NULL,
            status TEXT NOT NULL DEFAULT 'created',
            current_agent TEXT DEFAULT '',
            model_profile_id TEXT DEFAULT '',
            input_ref TEXT DEFAULT '',
            output_ref TEXT DEFAULT '',
            error_message TEXT DEFAULT '',
            failure_reason TEXT DEFAULT '',
            recoverable INTEGER DEFAULT 0,
            resume_from_stage TEXT DEFAULT '',
            last_success_stage TEXT DEFAULT '',
            next_action TEXT DEFAULT '',
            retry_count INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            locked_at TEXT DEFAULT NULL,
            completed_at TEXT DEFAULT NULL,
            archived_at TEXT DEFAULT NULL,
            handled_at TEXT DEFAULT NULL,
            FOREIGN KEY (kb_id) REFERENCES knowledge_bases(id)
        );

        CREATE TABLE IF NOT EXISTS task_events (
            id TEXT PRIMARY KEY,
            task_id TEXT NOT NULL,
            event_type TEXT NOT NULL,
            agent_name TEXT DEFAULT '',
            message TEXT NOT NULL,
            created_at TEXT NOT NULL,
            FOREIGN KEY (task_id) REFERENCES tasks(id)
        );

        CREATE TABLE IF NOT EXISTS reviews (
            id TEXT PRIMARY KEY,
            kb_id TEXT NOT NULL,
            task_id TEXT NOT NULL,
            status TEXT NOT NULL DEFAULT 'pending',
            summary TEXT DEFAULT '',
            risk_level TEXT DEFAULT 'medium',
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            FOREIGN KEY (kb_id) REFERENCES knowledge_bases(id),
            FOREIGN KEY (task_id) REFERENCES tasks(id)
        );

        CREATE TABLE IF NOT EXISTS review_items (
            id TEXT PRIMARY KEY,
            review_id TEXT NOT NULL,
            operation TEXT NOT NULL,
            target_path TEXT NOT NULL,
            base_version_hash TEXT DEFAULT '',
            old_content TEXT DEFAULT '',
            new_content TEXT DEFAULT '',
            status TEXT NOT NULL DEFAULT 'pending',
            risk_level TEXT DEFAULT 'medium',
            title TEXT DEFAULT '',
            page_type TEXT DEFAULT '',
            apply_error TEXT DEFAULT '',
            metadata_json TEXT DEFAULT '{}',
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            FOREIGN KEY (review_id) REFERENCES reviews(id)
        );

        CREATE TABLE IF NOT EXISTS versions (
            id TEXT PRIMARY KEY,
            kb_id TEXT NOT NULL,
            page_path TEXT NOT NULL,
            content_hash TEXT NOT NULL,
            snapshot_path TEXT NOT NULL,
            task_id TEXT DEFAULT '',
            operation_id TEXT DEFAULT '',
            created_at TEXT NOT NULL,
            FOREIGN KEY (kb_id) REFERENCES knowledge_bases(id)
        );

        CREATE TABLE IF NOT EXISTS operations (
            id TEXT PRIMARY KEY,
            kb_id TEXT NOT NULL,
            task_id TEXT NOT NULL,
            operation_hash TEXT NOT NULL,
            target_path TEXT NOT NULL,
            status TEXT NOT NULL DEFAULT 'pending',
            applied_at TEXT DEFAULT NULL,
            FOREIGN KEY (kb_id) REFERENCES knowledge_bases(id),
            FOREIGN KEY (task_id) REFERENCES tasks(id)
        );

        CREATE TABLE IF NOT EXISTS graph_nodes (
            id TEXT PRIMARY KEY,
            kb_id TEXT NOT NULL,
            node_type TEXT NOT NULL,
            label TEXT NOT NULL,
            path TEXT DEFAULT '',
            metadata TEXT DEFAULT '{}',
            FOREIGN KEY (kb_id) REFERENCES knowledge_bases(id)
        );

        CREATE TABLE IF NOT EXISTS graph_edges (
            id TEXT PRIMARY KEY,
            kb_id TEXT NOT NULL,
            source_node_id TEXT NOT NULL,
            target_node_id TEXT NOT NULL,
            edge_type TEXT NOT NULL,
            metadata TEXT DEFAULT '{}',
            FOREIGN KEY (kb_id) REFERENCES knowledge_bases(id)
        );

        CREATE TABLE IF NOT EXISTS model_profiles (
            id TEXT PRIMARY KEY,
            provider TEXT NOT NULL DEFAULT 'deepseek',
            name TEXT NOT NULL,
            base_url TEXT NOT NULL DEFAULT 'https://api.deepseek.com',
            model_name TEXT NOT NULL DEFAULT 'deepseek-chat',
            encrypted_api_key_ref TEXT NOT NULL DEFAULT '',
            role TEXT NOT NULL DEFAULT 'chat',
            temperature REAL NOT NULL DEFAULT 0.7,
            max_tokens INTEGER NOT NULL DEFAULT 4096,
            timeout INTEGER NOT NULL DEFAULT 120,
            retry_count INTEGER NOT NULL DEFAULT 3,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS agent_definitions (
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
        );

        CREATE TABLE IF NOT EXISTS skill_definitions (
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
        );

        CREATE TABLE IF NOT EXISTS schema_migrations (
            version INTEGER PRIMARY KEY,
            applied_at TEXT NOT NULL
        );
        ",
    )?;
    Ok(())
}

/// 创建索引
pub fn create_indexes(conn: &Connection) -> Result<(), rusqlite::Error> {
    conn.execute_batch(
        "
        CREATE INDEX IF NOT EXISTS idx_sources_kb_id ON sources(kb_id);
        CREATE INDEX IF NOT EXISTS idx_sources_status ON sources(status);
        CREATE INDEX IF NOT EXISTS idx_wiki_pages_kb_id ON wiki_pages(kb_id);
        CREATE INDEX IF NOT EXISTS idx_wiki_pages_kb_path ON wiki_pages(kb_id, path);
        CREATE INDEX IF NOT EXISTS idx_wiki_pages_type ON wiki_pages(page_type);
        CREATE INDEX IF NOT EXISTS idx_knowledge_items_kb ON knowledge_items(kb_id);
        CREATE INDEX IF NOT EXISTS idx_knowledge_items_kb_canonical ON knowledge_items(kb_id, canonical_name);
        CREATE INDEX IF NOT EXISTS idx_aliases_item ON aliases(item_id);
        CREATE INDEX IF NOT EXISTS idx_aliases_normalized ON aliases(normalized_alias);
        CREATE INDEX IF NOT EXISTS idx_relationships_kb ON relationships(kb_id);
        CREATE INDEX IF NOT EXISTS idx_tasks_kb_id ON tasks(kb_id);
        CREATE INDEX IF NOT EXISTS idx_tasks_status ON tasks(status);
        CREATE INDEX IF NOT EXISTS idx_task_events_task ON task_events(task_id);
        CREATE INDEX IF NOT EXISTS idx_reviews_kb ON reviews(kb_id);
        CREATE INDEX IF NOT EXISTS idx_reviews_task ON reviews(task_id);
        CREATE INDEX IF NOT EXISTS idx_review_items_review ON review_items(review_id);
        CREATE INDEX IF NOT EXISTS idx_review_items_status ON review_items(status);
        CREATE INDEX IF NOT EXISTS idx_versions_kb ON versions(kb_id);
        CREATE INDEX IF NOT EXISTS idx_operations_kb ON operations(kb_id);
        CREATE INDEX IF NOT EXISTS idx_graph_nodes_kb ON graph_nodes(kb_id);
        CREATE INDEX IF NOT EXISTS idx_graph_nodes_kb_type ON graph_nodes(kb_id, node_type);
        CREATE INDEX IF NOT EXISTS idx_graph_nodes_kb_path ON graph_nodes(kb_id, path);
        CREATE INDEX IF NOT EXISTS idx_graph_edges_kb ON graph_edges(kb_id);
        CREATE INDEX IF NOT EXISTS idx_graph_edges_kb_source ON graph_edges(kb_id, source_node_id);
        CREATE INDEX IF NOT EXISTS idx_graph_edges_kb_target ON graph_edges(kb_id, target_node_id);
        CREATE INDEX IF NOT EXISTS idx_tasks_input_ref ON tasks(input_ref);
        CREATE UNIQUE INDEX IF NOT EXISTS idx_sources_kb_file_hash ON sources(kb_id, file_hash);
        CREATE INDEX IF NOT EXISTS idx_agent_defs_name ON agent_definitions(name);
        CREATE INDEX IF NOT EXISTS idx_agent_defs_status ON agent_definitions(status);
        CREATE INDEX IF NOT EXISTS idx_agent_defs_trigger ON agent_definitions(trigger_event);
        CREATE INDEX IF NOT EXISTS idx_skill_defs_name ON skill_definitions(name);
        CREATE INDEX IF NOT EXISTS idx_skill_defs_type ON skill_definitions(skill_type);
        ",
    )?;
    Ok(())
}
