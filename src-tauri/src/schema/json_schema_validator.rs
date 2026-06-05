/// JSON Schema 验证器
/// 验证 LLM 返回的 JSON 是否包含必要字段

#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

/// 验证 Source Ingest 结果
pub fn validate_ingest_result(json: &serde_json::Value) -> ValidationResult {
    let mut errors = Vec::new();
    let mut warnings = Vec::new();

    if !json.is_object() {
        errors.push("根元素必须是 JSON 对象".to_string());
        return ValidationResult { valid: false, errors, warnings };
    }

    // 必需字段
    let required_fields = [
        "source_summary", "coverage_report", "entities",
        "concepts", "topics", "claims", "relationships",
        "proposed_wiki_updates",
    ];

    for field in &required_fields {
        if json.get(field).is_none() {
            errors.push(format!("缺少必需字段: {}", field));
        }
    }

    // 验证 source_summary
    if let Some(summary) = json.get("source_summary") {
        let summary_fields = ["title", "short_summary"];
        for field in &summary_fields {
            if summary.get(field).is_none() {
                warnings.push(format!("source_summary 缺少字段: {}", field));
            }
        }
    }

    // 验证 coverage_report
    if let Some(coverage) = json.get("coverage_report") {
        let cr_fields = ["document_sections_seen", "confidence_in_coverage"];
        for field in &cr_fields {
            if coverage.get(field).is_none() {
                warnings.push(format!("coverage_report 缺少字段: {}", field));
            }
        }
    }

    // 验证 claims 数组
    if let Some(claims) = json.get("claims").and_then(|c| c.as_array()) {
        for (i, claim) in claims.iter().enumerate() {
            let claim_fields = ["claim", "confidence", "citation_status"];
            for field in &claim_fields {
                if claim.get(field).is_none() {
                    warnings.push(format!("claims[{}] 缺少字段: {}", i, field));
                }
            }
        }
    }

    // 验证 relationships
    if let Some(rels) = json.get("relationships").and_then(|r| r.as_array()) {
        for (i, rel) in rels.iter().enumerate() {
            let rel_fields = ["source", "target", "relation", "confidence"];
            for field in &rel_fields {
                if rel.get(field).is_none() {
                    warnings.push(format!("relationships[{}] 缺少字段: {}", i, field));
                }
            }

            // 验证 relation 是否在允许的枚举值中
            if let Some(relation) = rel.get("relation").and_then(|r| r.as_str()) {
                let valid_relations = [
                    "is_a", "part_of", "uses", "depends_on", "improves",
                    "compares_with", "contradicts", "cites", "mentions",
                    "related_to", "has_alias", "belongs_to_topic",
                    "evaluated_on", "proposed_by", "applies_to", "derived_from",
                ];
                if !valid_relations.contains(&relation) {
                    warnings.push(format!(
                        "relationships[{}] 的 relation '{}' 不在标准枚举中",
                        i, relation
                    ));
                }
            }
        }
    }

    let valid = errors.is_empty();

    ValidationResult { valid, errors, warnings }
}

/// 验证 Resolution 结果
pub fn validate_resolution_result(json: &serde_json::Value) -> ValidationResult {
    let mut errors = Vec::new();
    let mut warnings = Vec::new();

    if !json.is_object() {
        errors.push("根元素必须是 JSON 对象".to_string());
        return ValidationResult { valid: false, errors, warnings };
    }

    if json.get("resolutions").is_none() {
        errors.push("缺少必需字段: resolutions".to_string());
    }

    if let Some(resolutions) = json.get("resolutions").and_then(|r| r.as_array()) {
        for (i, res) in resolutions.iter().enumerate() {
            let req_fields = ["input_name", "decision", "confidence"];
            for field in &req_fields {
                if res.get(field).is_none() {
                    warnings.push(format!("resolutions[{}] 缺少字段: {}", i, field));
                }
            }

            if let Some(decision) = res.get("decision").and_then(|d| d.as_str()) {
                let valid_decisions = [
                    "create_new", "update_existing", "append_to_existing",
                    "add_alias", "merge_suggestion", "skip", "needs_user_review",
                ];
                if !valid_decisions.contains(&decision) {
                    warnings.push(format!("resolutions[{}] 的 decision '{}' 不在标准枚举中", i, decision));
                }
            }
        }
    }

    let valid = errors.is_empty();
    ValidationResult { valid, errors, warnings }
}

/// 验证 Wiki Update Plan 结果
pub fn validate_update_plan(json: &serde_json::Value) -> ValidationResult {
    let mut errors = Vec::new();
    let mut warnings = Vec::new();

    if !json.is_object() {
        errors.push("根元素必须是 JSON 对象".to_string());
        return ValidationResult { valid: false, errors, warnings };
    }

    if json.get("wiki_update_plan").is_none() {
        errors.push("缺少必需字段: wiki_update_plan".to_string());
    }

    if let Some(plans) = json.get("wiki_update_plan").and_then(|p| p.as_array()) {
        for (i, plan) in plans.iter().enumerate() {
            let req_fields = ["operation", "title", "risk_level"];
            for field in &req_fields {
                if plan.get(field).is_none() {
                    warnings.push(format!("wiki_update_plan[{}] 缺少字段: {}", i, field));
                }
            }
        }
    }

    let valid = errors.is_empty();
    ValidationResult { valid, errors, warnings }
}

/// 验证 Query 结果
pub fn validate_query_result(json: &serde_json::Value) -> ValidationResult {
    let mut errors = Vec::new();
    let warnings = Vec::new();

    if !json.is_object() {
        errors.push("根元素必须是 JSON 对象".to_string());
        return ValidationResult { valid: false, errors, warnings };
    }

    if json.get("answer").is_none() {
        errors.push("缺少必需字段: answer".to_string());
    }

    let valid = errors.is_empty();
    ValidationResult { valid, errors, warnings }
}
