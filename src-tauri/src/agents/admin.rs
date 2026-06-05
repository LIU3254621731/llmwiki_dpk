use std::sync::Arc;
use crate::core::agent_registry::{AgentDefinition, AgentRegistry};
use crate::core::skill_registry::{SkillDefinition, SkillRegistry};
use crate::core::config_service::ConfigService;
use crate::core::event_bus::EventBus;
use crate::model::model_gateway::{ModelGateway, ChatMessage};

const ADMIN_AGENT_NAME: &str = "AdminAgent";
/// 硬性全局最大调用链深度，防止 Agent 间无限循环触发
const MAX_CHAIN_DEPTH: u32 = 3;

/// AdminAgent — 硬编码的中央控制器，不可被前端修改
///
/// 职责：
/// 1. 启动时验证所有 Agent/Skill 的一致性
/// 2. 在 CRUD 操作前进行安全校验
/// 3. 执行 Skill 并分发 Agent 事件
/// 4. 防死循环 (MaxDepth) 和超时保护
pub struct AdminAgent {
    agent_registry: Arc<AgentRegistry>,
    skill_registry: Arc<SkillRegistry>,
    event_bus: Arc<EventBus>,
    model_gateway: Arc<ModelGateway>,
    config: Arc<ConfigService>,
}

impl AdminAgent {
    pub fn new(
        agent_registry: Arc<AgentRegistry>,
        skill_registry: Arc<SkillRegistry>,
        event_bus: Arc<EventBus>,
        model_gateway: Arc<ModelGateway>,
        config: Arc<ConfigService>,
    ) -> Self {
        Self {
            agent_registry,
            skill_registry,
            event_bus,
            model_gateway,
            config,
        }
    }

    /// 启动时执行一致性验证
    pub fn run_boot_validation(&self) -> Vec<String> {
        let mut warnings = Vec::new();
        let agents = self.agent_registry.list_agents();
        let skills = self.skill_registry.list_skills();
        let skill_names: std::collections::HashSet<String> =
            skills.iter().map(|s| s.name.clone()).collect();

        for agent in &agents {
            for skill_name in &agent.allowed_skills {
                if !skill_names.contains(skill_name) {
                    let msg = format!(
                        "Agent '{}' 引用了不存在的 Skill '{}'",
                        agent.name, skill_name
                    );
                    log::warn!("[AdminAgent] {}", msg);
                    warnings.push(msg);
                }
            }
        }

        if !warnings.is_empty() {
            self.event_bus.emit_notification(
                "warning",
                "Agent/Skill 一致性检查",
                &format!("发现 {} 个警告", warnings.len()),
            );
        }

        log::info!(
            "[AdminAgent] 启动验证完成: {} agents, {} skills, {} warnings",
            agents.len(),
            skills.len(),
            warnings.len()
        );
        warnings
    }

    /// 校验 Agent 变更：检查 allowed_skills 引用了真实存在的 Skill
    pub fn validate_agent_change(&self, def: &AgentDefinition) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();
        let skills = self.skill_registry.list_skills();
        let skill_names: std::collections::HashSet<String> =
            skills.iter().map(|s| s.name.clone()).collect();

        for skill_name in &def.allowed_skills {
            if !skill_names.contains(skill_name) {
                errors.push(format!("Skill '{}' 不存在", skill_name));
            }
        }

        if def.name.is_empty() {
            errors.push("Agent 名称不能为空".to_string());
        }

        if def.system_prompt.is_empty() {
            errors.push("Agent 系统提示词不能为空".to_string());
        }

        if def.timeout_secs == 0 {
            errors.push("超时时间必须大于 0".to_string());
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    /// 校验 Skill 变更：检查 parameter_schema 是合法 JSON
    pub fn validate_skill_change(&self, def: &SkillDefinition) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        if def.name.is_empty() {
            errors.push("Skill 名称不能为空".to_string());
        }

        if def.parameter_schema != serde_json::Value::Null {
            if let Err(e) =
                self.skill_registry
                    .validate_parameter_schema(&def.parameter_schema.to_string())
            {
                errors.push(format!("parameter_schema 无效: {}", e));
            }
        }

        let valid_types = ["prompt", "transform", "composite"];
        if !valid_types.contains(&def.skill_type.as_str()) {
            errors.push(format!(
                "skill_type 必须为 prompt/transform/composite，当前值: {}",
                def.skill_type
            ));
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    /// 执行一个 Skill（公共入口，通过 boxing 避免递归 async 问题）
    pub fn execute_skill(
        &self,
        skill_name: &str,
        params: serde_json::Value,
        depth: u32,
        max_depth: u32,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<String, String>> + Send + 'static>> {
        let skill_registry = self.skill_registry.clone();
        let config = self.config.clone();
        let model_gateway = self.model_gateway.clone();
        let skill_name = skill_name.to_string();
        Box::pin(async move {
            do_execute_skill(&skill_registry, &config, &model_gateway, &skill_name, &params, depth, max_depth).await
        })
    }

    /// 根据事件类型分发到匹配的 Agent
    /// 携带 is_agent_action 标记和深度计数器，防止级联循环触发。
    pub async fn dispatch_agent(
        &self,
        event_type: &str,
        payload: serde_json::Value,
        depth: u32,
    ) -> Result<(), String> {
        // 硬性全局深度限制 MAX_CHAIN_DEPTH = 3
        if depth > MAX_CHAIN_DEPTH {
            log::warn!(
                "[AdminAgent] 事件 '{}' 深度 {} 超过全局限制 {}，已拒绝分发",
                event_type, depth, MAX_CHAIN_DEPTH
            );
            return Ok(());
        }

        // 如果事件已携带 is_agent_action 标记，拒绝二次级联触发
        if payload.get("is_agent_action").and_then(|v| v.as_bool()).unwrap_or(false) {
            log::info!(
                "[AdminAgent] 事件 '{}' 携带 is_agent_action 标记，已过滤（防止循环触发）",
                event_type
            );
            return Ok(());
        }

        let candidates = self.agent_registry.find_by_trigger(event_type);

        if candidates.is_empty() {
            return Ok(());
        }

        log::info!(
            "[AdminAgent] 事件 '{}' 匹配 {} 个 Agent (depth={})",
            event_type,
            candidates.len(),
            depth
        );

        for agent in &candidates {
            if agent.name == ADMIN_AGENT_NAME {
                continue;
            }

            // 使用全局 MAX_CHAIN_DEPTH 和 agent.max_depth 中较小者
            let effective_max = MAX_CHAIN_DEPTH.min(agent.max_depth);
            if depth > effective_max {
                let msg = format!(
                    "Agent '{}' 触发深度 {} 超过限制 {}，已跳过",
                    agent.name, depth, effective_max
                );
                log::warn!("[AdminAgent] {}", msg);
                self.event_bus
                    .emit_notification("warning", "Agent 深度限制", &msg);
                continue;
            }

            let timeout_dur = tokio::time::Duration::from_secs(agent.timeout_secs as u64);
            // 在 payload 中注入 is_agent_action 标记，防止下游级联触发
            let mut wrapped_payload = payload.clone();
            if let Some(obj) = wrapped_payload.as_object_mut() {
                obj.insert("is_agent_action".to_string(), serde_json::Value::Bool(true));
            }
            let result = tokio::time::timeout(
                timeout_dur,
                self.run_agent_pipeline(agent, &wrapped_payload, depth),
            )
            .await;

            match result {
                Ok(Ok(_)) => {
                    log::info!("[AdminAgent] Agent '{}' 执行成功", agent.name);
                }
                Ok(Err(e)) => {
                    log::error!("[AdminAgent] Agent '{}' 执行失败: {}", agent.name, e);
                    self.event_bus.emit_notification(
                        "error",
                        &format!("Agent '{}' 错误", agent.name),
                        &e,
                    );
                }
                Err(_elapsed) => {
                    let msg = format!(
                        "Agent '{}' 执行超时 ({}s)",
                        agent.name, agent.timeout_secs
                    );
                    log::error!("[AdminAgent] {}", msg);
                    self.event_bus.emit_notification("error", "Agent 超时", &msg);
                }
            }
        }

        Ok(())
    }

    /// 运行单个 Agent 的流水线：发送系统提示词给 LLM，解析输出中的 Skill 调用
    async fn run_agent_pipeline(
        &self,
        agent: &AgentDefinition,
        payload: &serde_json::Value,
        depth: u32,
    ) -> Result<(), String> {
        let provider_config = self.config.get_provider_config().unwrap_or_default();

        let user_prompt = format!(
            "系统事件: {}\n数据: {}\n\n请根据你的角色和职责处理此事件。",
            agent.trigger_event,
            serde_json::to_string_pretty(payload).unwrap_or_default()
        );

        let messages = vec![
            ChatMessage {
                role: "system".to_string(),
                content: agent.system_prompt.clone(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: user_prompt,
            },
        ];

        let result = self
            .model_gateway
            .chat(&provider_config, messages, false)
            .await
            .map_err(|e| format!("Agent LLM 调用失败: {}", e))?;

        // ReAct 解析：提取 Action/Params 指令
        if let Some((skill_name, skill_params)) = parse_react_action_pair(&result.content) {
            if agent.allowed_skills.contains(&skill_name) {
                let outcome = do_execute_skill(
                    &self.skill_registry,
                    &self.config,
                    &self.model_gateway,
                    &skill_name,
                    &skill_params,
                    depth + 1,
                    agent.max_depth,
                )
                .await?;
                log::info!(
                    "[AdminAgent] Agent '{}' 调用 Skill '{}' 完成: {}",
                    agent.name,
                    skill_name,
                    &outcome[..outcome.len().min(200)]
                );
            } else {
                log::warn!(
                    "[AdminAgent] Agent '{}' 尝试调用未授权 Skill '{}'，已忽略",
                    agent.name,
                    skill_name
                );
            }
        }

        Ok(())
    }
}

// ── 自由函数：Skill 执行核心逻辑（避免 async 递归） ──

async fn do_execute_skill(
    skill_registry: &SkillRegistry,
    config: &ConfigService,
    model_gateway: &ModelGateway,
    skill_name: &str,
    params: &serde_json::Value,
    depth: u32,
    max_depth: u32,
) -> Result<String, String> {
    if depth > max_depth {
        return Err(format!(
            "Skill '{}' 执行深度 {} 超过限制 {}",
            skill_name, depth, max_depth
        ));
    }

    let skill = skill_registry
        .get_skill(skill_name)
        .ok_or_else(|| format!("Skill '{}' 不存在", skill_name))?;

    if skill.status != "active" {
        return Err(format!("Skill '{}' 已被禁用", skill_name));
    }

    match skill.skill_type.as_str() {
        "prompt" => execute_prompt_skill(config, model_gateway, &skill, params).await,
        "transform" => execute_transform_skill(&skill, params),
        "composite" => execute_composite_skill(skill_registry, config, model_gateway, &skill, params, depth + 1, max_depth).await,
        other => Err(format!("未知的 skill_type: {}", other)),
    }
}

async fn execute_prompt_skill(
    config: &ConfigService,
    model_gateway: &ModelGateway,
    skill: &SkillDefinition,
    params: &serde_json::Value,
) -> Result<String, String> {
    let provider_config = config.get_provider_config().unwrap_or_default();

    let config_json: serde_json::Value =
        serde_json::from_str(&skill.code_body).unwrap_or_default();

    let system_prompt = config_json
        .get("system_prompt")
        .and_then(|v| v.as_str())
        .unwrap_or(&skill.description)
        .to_string();

    let user_template = config_json
        .get("user_prompt_template")
        .and_then(|v| v.as_str())
        .unwrap_or("{{input}}");

    let user_prompt = interpolate_template(user_template, params);

    let messages = vec![
        ChatMessage {
            role: "system".to_string(),
            content: system_prompt,
        },
        ChatMessage {
            role: "user".to_string(),
            content: user_prompt,
        },
    ];

    let result = model_gateway
        .chat(&provider_config, messages, false)
        .await
        .map_err(|e| format!("LLM 调用失败: {}", e))?;

    if let Some(action_result) = parse_react_action(&result.content) {
        return Ok(action_result);
    }

    Ok(result.content)
}

fn execute_transform_skill(
    skill: &SkillDefinition,
    params: &serde_json::Value,
) -> Result<String, String> {
    let config_json: serde_json::Value =
        serde_json::from_str(&skill.code_body).unwrap_or_default();

    let function_name = config_json
        .get("function")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Skill code_body 缺少 'function' 字段".to_string())?;

    match function_name {
        "sha256_hash" => {
            let input = params.get("text").and_then(|v| v.as_str()).unwrap_or("");
            use sha2::{Digest, Sha256};
            let mut hasher = Sha256::new();
            hasher.update(input.as_bytes());
            Ok(format!("{:x}", hasher.finalize()))
        }
        "count_words" => {
            let input = params.get("text").and_then(|v| v.as_str()).unwrap_or("");
            Ok(input.split_whitespace().count().to_string())
        }
        _ => {
            log::info!(
                "[AdminAgent] Transform skill '{}' 由系统管道处理",
                function_name
            );
            Ok(format!("[系统Skill: {}] 已调度执行", function_name))
        }
    }
}

async fn execute_composite_skill(
    skill_registry: &SkillRegistry,
    config: &ConfigService,
    model_gateway: &ModelGateway,
    skill: &SkillDefinition,
    params: &serde_json::Value,
    _depth: u32,
    _max_depth: u32,
) -> Result<String, String> {
    let config_json: serde_json::Value =
        serde_json::from_str(&skill.code_body)
            .map_err(|e| format!("Composite Skill code_body 不是合法 JSON: {}", e))?;

    let steps = config_json
        .get("steps")
        .and_then(|v| v.as_array())
        .ok_or_else(|| "Composite Skill 缺少 'steps' 数组".to_string())?;

    let mut last_result = String::new();

    for step in steps {
        let step_name = step
            .get("skill")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "Step 缺少 'skill' 字段".to_string())?;
        let step_params = step
            .get("params")
            .cloned()
            .unwrap_or(serde_json::json!({}));
        let merged_params = merge_params(params, &step_params);

        // Only prompt-type steps supported in composite, avoid recursion
        let step_skill = skill_registry
            .get_skill(step_name)
            .ok_or_else(|| format!("Skill '{}' 不存在", step_name))?;

        if step_skill.status != "active" {
            return Err(format!("Skill '{}' 已被禁用", step_name));
        }

        if step_skill.skill_type != "prompt" {
            return Err(format!(
                "Composite Skill 不支持嵌套 skill_type: {}",
                step_skill.skill_type
            ));
        }

        last_result =
            execute_prompt_skill(config, model_gateway, &step_skill, &merged_params).await?;
    }

    Ok(last_result)
}

// ── 工具函数 ──

fn interpolate_template(template: &str, params: &serde_json::Value) -> String {
    let mut result = template.to_string();
    if let Some(obj) = params.as_object() {
        for (key, value) in obj {
            let placeholder = format!("{{{{{}}}}}", key);
            let replacement = match value {
                serde_json::Value::String(s) => s.clone(),
                other => other.to_string(),
            };
            result = result.replace(&placeholder, &replacement);
        }
    }
    result
}

fn merge_params(base: &serde_json::Value, overlay: &serde_json::Value) -> serde_json::Value {
    let mut merged = base.clone();
    if let (Some(merged_obj), Some(overlay_obj)) = (merged.as_object_mut(), overlay.as_object()) {
        for (key, value) in overlay_obj {
            merged_obj.insert(key.clone(), value.clone());
        }
    }
    merged
}

/// 解析 ReAct 格式输出: `Action: call_skill, Params: [...]`
fn parse_react_action(content: &str) -> Option<String> {
    if !content.contains("Action:") {
        return None;
    }
    if let Some(params_start) = content.find("Params:") {
        let params_str = &content[params_start + 7..].trim();
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(params_str) {
            return Some(val.to_string());
        }
        return Some(params_str.to_string());
    }
    None
}

/// 解析 ReAct 格式输出并提取 (skill_name, params) 对
fn parse_react_action_pair(content: &str) -> Option<(String, serde_json::Value)> {
    let action_tag = "Action: ";
    let params_tag = "Params: ";

    if let Some(action_pos) = content.find(action_tag) {
        let after_action = &content[action_pos + action_tag.len()..];
        let action_end = after_action.find('\n').unwrap_or(after_action.len());
        let action_str = after_action[..action_end].trim();

        let skill_name = action_str
            .strip_prefix("call_skill")
            .map(|s| s.trim().trim_matches(',').trim().to_string())
            .unwrap_or_else(|| action_str.to_string());

        if let Some(params_pos) = after_action.find(params_tag) {
            let params_str = &after_action[params_pos + params_tag.len()..];
            let params_end = params_str.find('\n').unwrap_or(params_str.len());
            let params_val = params_str[..params_end].trim();
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(params_val) {
                return Some((skill_name, val));
            }
        }

        return Some((skill_name, serde_json::json!({})));
    }

    None
}
