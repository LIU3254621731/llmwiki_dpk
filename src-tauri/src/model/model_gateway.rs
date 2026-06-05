use std::time::Duration;
use serde::{Deserialize, Serialize};
use crate::model::deepseek_client::DeepSeekClient;
use crate::model::provider::{ModelProvider, OpenAiCompatibleProvider, AnthropicProvider};
use crate::core::secret_service::SecretService;
use crate::core::config_service::ProviderConfig;
use crate::core::token_logger::{TokenLogger, TokenContext};
use std::sync::Arc;

/// 模型调用统一网关
/// 所有 LLM 调用必须通过此网关
pub struct ModelGateway {
    client: DeepSeekClient,
    secrets: Arc<SecretService>,
    token_logger: Option<Arc<TokenLogger>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    pub temperature: f64,
    pub max_tokens: u32,
    pub stream: bool,
    pub response_format: Option<ResponseFormat>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseFormat {
    #[serde(rename = "type")]
    pub format_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionResponse {
    pub id: Option<String>,
    pub choices: Vec<Choice>,
    pub usage: Option<UsageInfo>,
    pub model: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Choice {
    pub index: u32,
    pub message: ChoiceMessage,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChoiceMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageInfo {
    pub prompt_tokens: Option<u32>,
    pub completion_tokens: Option<u32>,
    pub total_tokens: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelResult {
    pub content: String,
    pub model: String,
    pub usage: Option<UsageInfo>,
    pub finish_reason: Option<String>,
}

/// 配置状态快照（用于任务详情记录）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigSnapshot {
    pub provider: String,
    pub base_url: String,
    pub chat_model: String,
    pub reasoner_model: String,
    pub config_status: String,
    pub created_with_config_id: String,
}

impl ModelGateway {
    pub fn new(secrets: Arc<SecretService>) -> Self {
        Self {
            client: DeepSeekClient::new(),
            secrets,
            token_logger: None,
        }
    }

    /// 注入 TokenLogger 用于记录每次 API 调用的 Token 消耗
    pub fn with_token_logger(mut self, logger: Arc<TokenLogger>) -> Self {
        self.token_logger = Some(logger);
        self
    }

    /// 根据 ProviderConfig 构造对应的 ModelProvider
    fn resolve_provider(config: &ProviderConfig) -> Box<dyn ModelProvider> {
        match config.provider.as_str() {
            "anthropic" => Box::new(AnthropicProvider::new()),
            _ => Box::new(OpenAiCompatibleProvider::new(
                match config.provider.as_str() {
                    "openai" => "OpenAI",
                    "ollama" => "Ollama",
                    "openwebui" => "OpenWebUI",
                    _ => "DeepSeek",
                },
            )),
        }
    }

    /// 调用模型（非流式）
    pub async fn chat(
        &self,
        config: &ProviderConfig,
        messages: Vec<ChatMessage>,
        use_json_mode: bool,
    ) -> Result<ModelResult, String> {
        self.chat_with_token_ctx(config, messages, use_json_mode, None).await
    }

    /// 调用模型（非流式），附带 Token 上下文用于消耗记录
    pub async fn chat_with_token_ctx(
        &self,
        config: &ProviderConfig,
        messages: Vec<ChatMessage>,
        use_json_mode: bool,
        token_ctx: Option<TokenContext>,
    ) -> Result<ModelResult, String> {
        let provider = Self::resolve_provider(config);
        let api_key = self.secrets.get_api_key(&config.provider)
            .or_else(|| {
                // 回退到 "deepseek" key（兼容旧配置）
                self.secrets.get_api_key("deepseek")
            })
            .ok_or_else(|| format!("{} API Key 未配置，请在设置中配置。", provider.provider_name()))?;

        let model_name = config.chat_model.clone();

        let request = ChatCompletionRequest {
            model: model_name.clone(),
            messages,
            temperature: config.temperature,
            max_tokens: config.max_tokens,
            stream: false,
            response_format: if use_json_mode {
                Some(ResponseFormat {
                    format_type: "json_object".to_string(),
                })
            } else {
                None
            },
        };

        let url = provider.build_chat_url(&config.base_url);
        let headers = provider.build_headers(&api_key);
        let body = provider.build_request_body(&model_name, &request);

        let response = self
            .client
            .chat_completion_generic(
                &url,
                &headers,
                &body,
                Duration::from_secs(config.timeout as u64),
                config.retry_count,
            )
            .await
            .map_err(|e| {
                // 尝试从错误中解析 HTTP 状态码
                if let Some((code, body)) = parse_http_error(&e) {
                    provider.normalize_error(code, &body)
                } else {
                    e
                }
            })?;

        let content = provider.extract_content(&response)?;
        let finish_reason = provider.extract_finish_reason(&response);
        let usage = response
            .get("usage")
            .and_then(|u| {
                Some(UsageInfo {
                    prompt_tokens: u.get("prompt_tokens").and_then(|v| v.as_u64().map(|n| n as u32)),
                    completion_tokens: u.get("completion_tokens").and_then(|v| v.as_u64().map(|n| n as u32)),
                    total_tokens: u.get("total_tokens").and_then(|v| v.as_u64().map(|n| n as u32)),
                })
            });

        // Token 消耗记录（拦截器）
        if let (Some(logger), Some(ctx)) = (&self.token_logger, &token_ctx) {
            if let Some(ref u) = usage {
                let _ = logger.log_usage(
                    ctx,
                    u.prompt_tokens.unwrap_or(0),
                    u.completion_tokens.unwrap_or(0),
                );
            }
        }

        Ok(ModelResult {
            content,
            model: response
                .get("model")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .unwrap_or(model_name),
            usage,
            finish_reason,
        })
    }

    /// 调用模型（流式）
    pub async fn chat_stream(
        &self,
        config: &ProviderConfig,
        messages: Vec<ChatMessage>,
        use_json_mode: bool,
    ) -> Result<tokio::sync::mpsc::Receiver<String>, String> {
        let provider = Self::resolve_provider(config);
        let api_key = self
            .secrets
            .get_api_key(&config.provider)
            .or_else(|| self.secrets.get_api_key("deepseek"))
            .ok_or_else(|| {
                format!("{} API Key 未配置，请在设置中配置。", provider.provider_name())
            })?;

        let model_name = config.chat_model.clone();
        let request = ChatCompletionRequest {
            model: model_name.clone(),
            messages,
            temperature: config.temperature,
            max_tokens: config.max_tokens,
            stream: true,
            response_format: if use_json_mode {
                Some(ResponseFormat {
                    format_type: "json_object".to_string(),
                })
            } else {
                None
            },
        };

        let url = provider.build_chat_url(&config.base_url);
        let headers = provider.build_headers(&api_key);
        let body = provider.build_request_body(&model_name, &request);

        self.client
            .chat_completion_stream(&url, &headers, &body, Duration::from_secs(config.timeout as u64))
            .await
    }

    /// 调用模型（带系统提示和用户文本，专为文档内容设计）
    pub async fn chat_with_content(
        &self,
        config: &ProviderConfig,
        system_prompt: &str,
        user_content: &str,
        use_json_mode: bool,
    ) -> Result<ModelResult, String> {
        self.chat_with_content_and_ctx(config, system_prompt, user_content, use_json_mode, None).await
    }

    /// 调用模型（带系统提示和用户文本），附带 Token 上下文
    pub async fn chat_with_content_and_ctx(
        &self,
        config: &ProviderConfig,
        system_prompt: &str,
        user_content: &str,
        use_json_mode: bool,
        token_ctx: Option<TokenContext>,
    ) -> Result<ModelResult, String> {
        let messages = vec![
            ChatMessage {
                role: "system".to_string(),
                content: system_prompt.to_string(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: user_content.to_string(),
            },
        ];
        self.chat_with_token_ctx(config, messages, use_json_mode, token_ctx).await
    }

    /// 测试连接
    pub async fn test_connection(
        &self,
        config: &ProviderConfig,
        api_key: &str,
    ) -> Result<String, String> {
        let provider = Self::resolve_provider(config);
        let request = ChatCompletionRequest {
            model: config.chat_model.clone(),
            messages: vec![ChatMessage {
                role: "user".to_string(),
                content: "你好，请回复\"连接成功\"".to_string(),
            }],
            temperature: 0.0,
            max_tokens: 50,
            stream: false,
            response_format: None,
        };

        let url = provider.build_chat_url(&config.base_url);
        let headers = provider.build_headers(api_key);
        let body = provider.build_request_body(&config.chat_model, &request);

        let response = self
            .client
            .chat_completion_generic(
                &url,
                &headers,
                &body,
                Duration::from_secs(30),
                1,
            )
            .await
            .map_err(|e| {
                if let Some((code, body)) = parse_http_error(&e) {
                    provider.normalize_error(code, &body)
                } else {
                    e
                }
            })?;

        let content = provider.extract_content(&response)?;
        Ok(format!(
            "连接成功！模型: {}, 回复: {}",
            response.get("model").and_then(|v| v.as_str()).unwrap_or(&config.chat_model),
            content
        ))
    }

    /// 测试 JSON 输出能力
    pub async fn test_json_output(
        &self,
        config: &ProviderConfig,
        api_key: &str,
    ) -> Result<String, String> {
        let provider = Self::resolve_provider(config);
        let request = ChatCompletionRequest {
            model: config.chat_model.clone(),
            messages: vec![
                ChatMessage {
                    role: "system".to_string(),
                    content: "你是一个 JSON 输出助手，请严格按照 JSON 格式输出。".to_string(),
                },
                ChatMessage {
                    role: "user".to_string(),
                    content: "返回一个 JSON 对象，包含 name、version、status 三个字段。".to_string(),
                },
            ],
            temperature: 0.0,
            max_tokens: 200,
            stream: false,
            response_format: Some(ResponseFormat {
                format_type: "json_object".to_string(),
            }),
        };

        let url = provider.build_chat_url(&config.base_url);
        let headers = provider.build_headers(api_key);
        let body = provider.build_request_body(&config.chat_model, &request);

        let response = self
            .client
            .chat_completion_generic(
                &url,
                &headers,
                &body,
                Duration::from_secs(30),
                1,
            )
            .await
            .map_err(|e| {
                if let Some((code, body)) = parse_http_error(&e) {
                    provider.normalize_error(code, &body)
                } else {
                    e
                }
            })?;

        let content = provider.extract_content(&response)?;
        if content.is_empty() {
            return Err("模型返回了空响应，无法测试 JSON 输出".to_string());
        }

        match serde_json::from_str::<serde_json::Value>(&content) {
            Ok(_) => Ok(format!("JSON 输出测试通过！返回内容: {}", content)),
            Err(e) => {
                let repaired = crate::schema::json_repair::repair_json(&content);
                match serde_json::from_str::<serde_json::Value>(&repaired) {
                    Ok(val) => Ok(format!("JSON 输出测试通过（已自动修复）。返回内容: {}", val)),
                    Err(_) => Err(format!(
                        "JSON 输出测试失败。原始返回: {}\nJSON 解析错误: {}",
                        content, e
                    )),
                }
            }
        }
    }

    /// 测试文档附件调用
    pub async fn test_document_attachment(
        &self,
        config: &ProviderConfig,
        api_key: &str,
    ) -> Result<String, String> {
        let provider = Self::resolve_provider(config);
        let sample_doc = "# 测试文档\n\n这是一个测试段落，用于验证模型对文档内容的理解能力。\n\n## 关键概念\n\n- 知识图谱\n- 实体抽取\n- 关系标准化\n\n请用 JSON 格式回复，包含 entities 和 concepts 字段。";

        let request = ChatCompletionRequest {
            model: config.chat_model.clone(),
            messages: vec![
                ChatMessage {
                    role: "system".to_string(),
                    content: "你是一个文档分析助手。请严格按照 JSON 格式输出，不要添加额外解释。".to_string(),
                },
                ChatMessage {
                    role: "user".to_string(),
                    content: format!("分析以下文档并提取实体和概念：\n\n{}", sample_doc),
                },
            ],
            temperature: 0.0,
            max_tokens: 500,
            stream: false,
            response_format: Some(ResponseFormat {
                format_type: "json_object".to_string(),
            }),
        };

        let url = provider.build_chat_url(&config.base_url);
        let headers = provider.build_headers(api_key);
        let body = provider.build_request_body(&config.chat_model, &request);

        let response = self
            .client
            .chat_completion_generic(
                &url,
                &headers,
                &body,
                Duration::from_secs(60),
                1,
            )
            .await
            .map_err(|e| {
                if let Some((code, body)) = parse_http_error(&e) {
                    provider.normalize_error(code, &body)
                } else {
                    e
                }
            })?;

        let content = provider.extract_content(&response)?;
        if content.is_empty() {
            return Err("模型返回了空响应，无法测试文档附件".to_string());
        }

        let usage_info = response
            .get("usage")
            .map(|u| {
                format!(
                    "(输入: {} tokens, 输出: {} tokens)",
                    u.get("prompt_tokens").and_then(|v| v.as_u64()).unwrap_or(0),
                    u.get("completion_tokens").and_then(|v| v.as_u64()).unwrap_or(0)
                )
            })
            .unwrap_or_default();

        match serde_json::from_str::<serde_json::Value>(&content) {
            Ok(_) => Ok(format!(
                "文档附件测试通过！{} 返回: {}...",
                usage_info,
                &content[..content.len().min(200)]
            )),
            Err(e) => {
                let repaired = crate::schema::json_repair::repair_json(&content);
                match serde_json::from_str::<serde_json::Value>(&repaired) {
                    Ok(val) => Ok(format!(
                        "文档附件测试通过（已修复JSON）{} 返回: {}...",
                        usage_info, val
                    )),
                    Err(_) => Err(format!(
                        "文档附件测试失败。JSON 解析错误: {}。原始返回: {}",
                        e,
                        &content[..content.len().min(300)]
                    )),
                }
            }
        }
    }

    /// 生成当前配置快照（用于任务记录）
    pub fn config_snapshot(config: &ProviderConfig, api_key_available: bool) -> ConfigSnapshot {
        ConfigSnapshot {
            provider: config.provider.clone(),
            base_url: config.base_url.clone(),
            chat_model: config.chat_model.clone(),
            reasoner_model: config.reasoner_model.clone(),
            config_status: if api_key_available {
                "configured".to_string()
            } else {
                "missing_api_key".to_string()
            },
            created_with_config_id: String::new(),
        }
    }
}

/// 解析 HTTP 错误字符串，提取状态码和响应体
fn parse_http_error(err: &str) -> Option<(u16, String)> {
    // 错误格式: "HTTP {code}: {body}"
    if let Some(rest) = err.strip_prefix("HTTP ") {
        let parts: Vec<&str> = rest.splitn(2, ": ").collect();
        if let Some(code_str) = parts.first() {
            if let Ok(code) = code_str.parse::<u16>() {
                let body = parts.get(1).map(|s| s.to_string()).unwrap_or_default();
                return Some((code, body));
            }
        }
    }
    None
}
