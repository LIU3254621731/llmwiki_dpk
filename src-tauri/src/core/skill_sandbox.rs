// Skill 动态脚本安全沙箱
// 用户可在 Skill 工作台热修改 JavaScript/Prompt 代码，
// 后端执行时包裹在超时隔离的异步沙箱中，防止死循环/崩溃拖垮主进程。

use tokio::time::Duration;

const SANDBOX_TIMEOUT_SECS: u64 = 5;

/// 沙箱执行结果
#[derive(Debug, Clone, serde::Serialize)]
pub struct SandboxResult {
    pub success: bool,
    pub output: String,
    pub error: Option<String>,
    pub duration_ms: u64,
}

/// 在超时沙箱中执行一段纯文本 Prompt 模板渲染（prompt 类型 Skill）
/// 不涉及用户代码执行，仅做模板变量替换。安全操作，无需隔离。
pub async fn execute_prompt_template(
    template: &str,
    params: &serde_json::Value,
) -> SandboxResult {
    let start = std::time::Instant::now();
    let template_owned = template.to_string();
    let params_owned = params.clone();

    let result = tokio::time::timeout(
        Duration::from_secs(SANDBOX_TIMEOUT_SECS),
        tokio::task::spawn_blocking(move || {
            let mut result = template_owned;
            if let Some(obj) = params_owned.as_object() {
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
        }),
    )
    .await;

    let duration_ms = start.elapsed().as_millis() as u64;
    match result {
        Ok(Ok(output)) => SandboxResult {
            success: true,
            output,
            error: None,
            duration_ms,
        },
        Ok(Err(e)) => SandboxResult {
            success: false,
            output: String::new(),
            error: Some(format!("模板执行异常: {}", e)),
            duration_ms,
        },
        Err(_elapsed) => SandboxResult {
            success: false,
            output: String::new(),
            error: Some(format!(
                "Skill 执行超时 ({}s)，沙箱已自动熔断保护",
                SANDBOX_TIMEOUT_SECS
            )),
            duration_ms,
        },
    }
}

/// 在超时沙箱中执行一段 transform 类型的 Rust 函数（白名单调度）
/// 当前仅支持预注册的系统函数，不允许用户自定义代码执行。
pub async fn execute_transform_sandboxed(
    function_name: &str,
    params: &serde_json::Value,
) -> SandboxResult {
    let start = std::time::Instant::now();
    let fn_name = function_name.to_string();
    let params_owned = params.clone();

    let result = tokio::time::timeout(
        Duration::from_secs(SANDBOX_TIMEOUT_SECS),
        tokio::task::spawn_blocking(move || match fn_name.as_str() {
            "sha256_hash" => {
                let input = params_owned
                    .get("text")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                use sha2::{Digest, Sha256};
                let mut hasher = Sha256::new();
                hasher.update(input.as_bytes());
                Ok(format!("{:x}", hasher.finalize()))
            }
            "count_words" => {
                let input = params_owned
                    .get("text")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                Ok(input.split_whitespace().count().to_string())
            }
            other => Err(format!(
                "未注册的系统函数 '{}'，仅支持白名单内的 transform 操作",
                other
            )),
        }),
    )
    .await;

    let duration_ms = start.elapsed().as_millis() as u64;
    match result {
        Ok(Ok(Ok(output))) => SandboxResult {
            success: true,
            output,
            error: None,
            duration_ms,
        },
        Ok(Ok(Err(e))) => SandboxResult {
            success: false,
            output: String::new(),
            error: Some(e),
            duration_ms,
        },
        Ok(Err(join_err)) => SandboxResult {
            success: false,
            output: String::new(),
            error: Some(format!("Transform Skill 执行异常: {}", join_err)),
            duration_ms,
        },
        Err(_elapsed) => SandboxResult {
            success: false,
            output: String::new(),
            error: Some(format!(
                "Transform Skill 执行超时 ({}s)，沙箱已自动熔断保护",
                SANDBOX_TIMEOUT_SECS
            )),
            duration_ms,
        },
    }
}

/// Mock 运行测试：对 prompt 类型执行模板渲染，对 transform 类型执行白名单函数。
/// 供 Skill 工作台的「Mock 运行测试」按钮调用。
pub async fn mock_execute_skill(
    skill_type: &str,
    code_body: &str,
    params: &serde_json::Value,
) -> SandboxResult {
    match skill_type {
        "prompt" => {
            let config: serde_json::Value =
                serde_json::from_str(code_body).unwrap_or_default();
            let template = config
                .get("user_prompt_template")
                .and_then(|v| v.as_str())
                .unwrap_or(code_body);
            execute_prompt_template(template, params).await
        }
        "transform" => {
            let config: serde_json::Value =
                serde_json::from_str(code_body).unwrap_or_default();
            let function_name = config
                .get("function")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            execute_transform_sandboxed(function_name, params).await
        }
        "composite" => SandboxResult {
            success: false,
            output: String::new(),
            error: Some("Composite Skill 的 Mock 测试暂不支持，请单独测试各子 Skill".to_string()),
            duration_ms: 0,
        },
        other => SandboxResult {
            success: false,
            output: String::new(),
            error: Some(format!("未知的 skill_type: {}", other)),
            duration_ms: 0,
        },
    }
}
