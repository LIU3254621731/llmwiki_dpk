use std::sync::Arc;
use tauri::State;
use crate::core::app_kernel::AppKernel;
use crate::core::agent_registry::AgentDefinition;
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
pub async fn list_agent_definitions(
    kernel: State<'_, Arc<AppKernel>>,
) -> Result<Vec<AgentDefinition>, String> {
    Ok(kernel.agent_registry.list_agents())
}

#[tauri::command]
pub async fn create_agent_definition(
    kernel: State<'_, Arc<AppKernel>>,
    definition: AgentDefinition,
) -> Result<AgentDefinition, String> {
    let admin = build_admin(&kernel);

    if kernel.agent_registry.get_agent(&definition.name).is_some() {
        return Err(format!("Agent '{}' 已存在", definition.name));
    }

    admin.validate_agent_change(&definition).map_err(|errors| errors.join("; "))?;
    kernel.agent_registry.create_agent(definition)
}

#[tauri::command]
pub async fn update_agent_definition(
    kernel: State<'_, Arc<AppKernel>>,
    id: String,
    patch: serde_json::Value,
) -> Result<AgentDefinition, String> {
    kernel.agent_registry.update_agent(&id, &patch)
}

#[tauri::command]
pub async fn delete_agent_definition(
    kernel: State<'_, Arc<AppKernel>>,
    id: String,
) -> Result<(), String> {
    // 不允许删除系统内置 Agent
    let existing = kernel.agent_registry.get_agent(&id);
    if let Some(ref agent) = existing {
        let reserved = ["AdminAgent", "CoordinatorAgent", "SourceIngestAgent",
            "ResolutionAgent", "RelationshipAgent", "WikiUpdateAgent",
            "HealthCheckAgent", "QueryAgent"];
        if reserved.contains(&agent.name.as_str()) {
            return Err(format!("系统内置 Agent '{}' 不可删除", agent.name));
        }
    }
    kernel.agent_registry.delete_agent(&id)
}
