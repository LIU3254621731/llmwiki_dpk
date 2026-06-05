use std::sync::Arc;
use tauri::State;
use crate::core::app_kernel::AppKernel;
use crate::core::skill_registry::SkillDefinition;
use crate::model::model_gateway::ModelGateway;

fn build_admin(kernel: &AppKernel) -> crate::agents::admin::AdminAgent {
    let mg = Arc::new(
        ModelGateway::new(kernel.secrets.clone())
            .with_token_logger(kernel.token_logger.clone()),
    );
    crate::agents::admin::AdminAgent::new(
        kernel.agent_registry.clone(),
        kernel.skill_registry.clone(),
        kernel.event_bus.clone(),
        mg,
        kernel.config.clone(),
    )
}

#[tauri::command]
pub async fn list_skill_definitions(
    kernel: State<'_, Arc<AppKernel>>,
) -> Result<Vec<SkillDefinition>, String> {
    Ok(kernel.skill_registry.list_skills())
}

#[tauri::command]
pub async fn create_skill_definition(
    kernel: State<'_, Arc<AppKernel>>,
    definition: SkillDefinition,
) -> Result<SkillDefinition, String> {
    let admin = build_admin(&kernel);

    if kernel.skill_registry.get_skill(&definition.name).is_some() {
        return Err(format!("Skill '{}' 已存在", definition.name));
    }

    admin.validate_skill_change(&definition).map_err(|errors| errors.join("; "))?;
    kernel.skill_registry.create_skill(definition)
}

#[tauri::command]
pub async fn update_skill_definition(
    kernel: State<'_, Arc<AppKernel>>,
    id: String,
    patch: serde_json::Value,
) -> Result<SkillDefinition, String> {
    kernel.skill_registry.update_skill(&id, &patch)
}

#[tauri::command]
pub async fn delete_skill_definition(
    kernel: State<'_, Arc<AppKernel>>,
    id: String,
) -> Result<(), String> {
    let reserved = ["DocumentProcessor", "PdfSkill", "DocxSkill", "HtmlSkill",
        "MdSkill", "TxtSkill", "MarkitdownSkill", "PdfOcrSkill",
        "PptxSkill", "WebSearchSkill"];
    if let Some(skill) = kernel.skill_registry.get_skill(&id) {
        if reserved.contains(&skill.name.as_str()) {
            return Err(format!("系统内置 Skill '{}' 不可删除", skill.name));
        }
    }
    kernel.skill_registry.delete_skill(&id)
}

#[tauri::command]
pub async fn validate_skill_schema(
    kernel: State<'_, Arc<AppKernel>>,
    schema_json: String,
) -> Result<(), String> {
    kernel.skill_registry.validate_parameter_schema(&schema_json)
}

/// Mock 运行测试：对 Skill 的 code_body 在安全沙箱中执行，
/// 返回 { success, output, error, duration_ms }
#[tauri::command]
pub async fn execute_skill_mock(
    skill_type: String,
    code_body: String,
    params: serde_json::Value,
) -> Result<crate::core::skill_sandbox::SandboxResult, String> {
    Ok(crate::core::skill_sandbox::mock_execute_skill(
        &skill_type,
        &code_body,
        &params,
    ).await)
}
