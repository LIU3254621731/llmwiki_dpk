/// Prompt 构建器
/// 根据任务上下文动态构建 Prompt
pub struct PromptBuilder;

impl PromptBuilder {
    /// 构建 Source Ingest Prompt
    pub fn build_ingest_prompt(
        document_text: &str,
        source_id: &str,
        existing_pages_summary: &str,
    ) -> (String, String) {
        let system_prompt = crate::prompts::prompt_registry::PromptRegistry::get_ingest_system_prompt()
            .to_string();

        let max_length = 80000;
        let doc_text = if document_text.len() > max_length {
            // floor_char_boundary 防止字节索引落在多字节 UTF-8 字符中间导致 panic
            let boundary = document_text.floor_char_boundary(max_length);
            format!(
                "{}...\n\n[文档内容已截断，原始长度 {} 字节，显示前 {} 字节]",
                &document_text[..boundary],
                document_text.len(),
                boundary
            )
        } else {
            document_text.to_string()
        };

        let existing_info = if existing_pages_summary.is_empty() {
            "暂无已有页面。"
        } else {
            existing_pages_summary
        };

        let user_message = format!(
            "## 任务\n分析以下文档内容，提取结构化知识。\n\n## 文档信息\n- source_id: {}\n- 文档长度: {} 字符\n\n## 已有 Wiki 页面摘要\n{}\n\n## 文档内容\n\n{}\n\n---\n\n请严格按照 JSON 格式输出分析结果。",
            source_id,
            document_text.len(),
            existing_info,
            doc_text,
        );

        (system_prompt, user_message)
    }

    /// 构建 Resolution Prompt
    pub fn build_resolution_prompt(
        items_to_resolve: &str,
        candidates: &str,
    ) -> (String, String) {
        let system_prompt = crate::prompts::prompt_registry::PromptRegistry::get_resolution_system_prompt()
            .to_string();

        let user_message = format!(
            "## 需要消歧的新项目\n\n{}\n\n## 候选已有页面\n\n{}\n\n---\n\n请判断每个新项目的处理决策。",
            items_to_resolve, candidates
        );

        (system_prompt, user_message)
    }

    /// 构建 Query Prompt
    pub fn build_query_prompt(
        question: &str,
        context_content: &str,
        scope_description: &str,
        allow_ai_generation: bool,
    ) -> (String, String) {
        let mut system_prompt = crate::prompts::prompt_registry::PromptRegistry::get_query_system_prompt()
            .to_string();

        if !allow_ai_generation {
            system_prompt.push_str("\n\n## 严格约束：仅基于 Wiki 内容回答\n你被禁止使用训练数据或自有知识生成答案。所有回答必须严格基于下面提供的 Wiki 内容。如果 Wiki 内容不足以回答问题，你必须明确回复「知识库中暂无相关信息」，不得编造、推测或补充任何 Wiki 中不存在的内容。违反此规则将被视为严重错误。");
        }

        let user_message = format!(
            "## 问答范围\n{}\n\n## 相关 Wiki 内容\n\n{}\n\n## 用户问题\n\n{}\n\n---\n\n请基于提供的 Wiki 内容回答问题。",
            scope_description, context_content, question,
        );

        (system_prompt, user_message)
    }

    /// 构建 Relationship 标准化 Prompt
    pub fn build_relationship_prompt(
        candidate_relationships: &str,
    ) -> (String, String) {
        let system_prompt = crate::prompts::prompt_registry::PromptRegistry::get_relationship_system_prompt()
            .to_string();

        let user_message = format!(
            "## 候选关系\n\n{}\n\n请标准化这些关系，确保映射到 canonical Wiki page。",
            candidate_relationships
        );

        (system_prompt, user_message)
    }

    /// 构建 Wiki Update 计划 Prompt
    pub fn build_wiki_update_prompt(
        resolutions: &str,
        relationships: &str,
        existing_page_content: &str,
    ) -> (String, String) {
        let system_prompt = crate::prompts::prompt_registry::PromptRegistry::get_wiki_update_system_prompt()
            .to_string();

        let user_message = format!(
            "## 消歧结果\n\n{}\n\n## 标准化关系\n\n{}\n\n## 已有页面内容（需要更新的页面）\n\n{}\n\n---\n\n请生成 Wiki 更新计划。",
            resolutions, relationships, existing_page_content
        );

        (system_prompt, user_message)
    }
}
