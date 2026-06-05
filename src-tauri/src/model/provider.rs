/// ModelProvider trait — 多模型供应商抽象层
/// 每个 LLM 供应商实现此 trait 以处理 API 格式差异
use crate::model::model_gateway::{ChatCompletionRequest, ChatCompletionResponse};

pub trait ModelProvider: Send + Sync {
    /// 供应商标识名
    fn provider_name(&self) -> &'static str;

    /// 构建 Chat Completion API 端点 URL
    fn build_chat_url(&self, base_url: &str) -> String;

    /// 构建请求头列表
    fn build_headers(&self, api_key: &str) -> Vec<(String, String)>;

    /// 构建请求体 JSON
    fn build_request_body(
        &self,
        model: &str,
        request: &ChatCompletionRequest,
    ) -> serde_json::Value;

    /// 从响应 JSON 中提取文本内容
    fn extract_content(&self, response: &serde_json::Value) -> Result<String, String>;

    /// 从响应 JSON 中提取 usage 信息
    fn extract_usage(
        &self,
        response: &serde_json::Value,
    ) -> Option<ChatCompletionResponse>;

    /// 提取 finish_reason
    fn extract_finish_reason(&self, response: &serde_json::Value) -> Option<String>;

    /// 错误归一化：将 HTTP 错误转为用户可读的中文提示
    fn normalize_error(&self, status_code: u16, error_body: &str) -> String;
}

// ============================================================
// OpenAI 兼容 Provider（DeepSeek / OpenAI / Ollama / OpenWebUI / 自定义）
// ============================================================
pub struct OpenAiCompatibleProvider {
    name: &'static str,
}

impl OpenAiCompatibleProvider {
    pub fn new(name: &'static str) -> Self {
        Self { name }
    }
}

impl ModelProvider for OpenAiCompatibleProvider {
    fn provider_name(&self) -> &'static str {
        self.name
    }

    fn build_chat_url(&self, base_url: &str) -> String {
        format!("{}/v1/chat/completions", base_url.trim_end_matches('/'))
    }

    fn build_headers(&self, api_key: &str) -> Vec<(String, String)> {
        vec![
            ("Authorization".to_string(), format!("Bearer {}", api_key)),
            ("Content-Type".to_string(), "application/json".to_string()),
        ]
    }

    fn build_request_body(
        &self,
        model: &str,
        request: &ChatCompletionRequest,
    ) -> serde_json::Value {
        let mut body = serde_json::json!({
            "model": model,
            "messages": request.messages.iter().map(|m| {
                serde_json::json!({
                    "role": m.role,
                    "content": m.content,
                })
            }).collect::<Vec<_>>(),
            "temperature": request.temperature,
            "max_tokens": request.max_tokens,
            "stream": false,
        });

        if let Some(ref rf) = request.response_format {
            body["response_format"] = serde_json::json!({
                "type": rf.format_type,
            });
        }

        body
    }

    fn extract_content(&self, response: &serde_json::Value) -> Result<String, String> {
        response["choices"]
            .as_array()
            .and_then(|a| a.first())
            .and_then(|c| c["message"]["content"].as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| "模型返回了空响应（choices 为空），可能是输入过长或模型异常".to_string())
    }

    fn extract_usage(
        &self,
        _response: &serde_json::Value,
    ) -> Option<ChatCompletionResponse> {
        None // 由调用方处理
    }

    fn extract_finish_reason(&self, response: &serde_json::Value) -> Option<String> {
        response["choices"]
            .as_array()
            .and_then(|a| a.first())
            .and_then(|c| c["finish_reason"].as_str())
            .map(|s| s.to_string())
    }

    fn normalize_error(&self, status_code: u16, error_body: &str) -> String {
        let detail = &error_body[..error_body.len().min(300)];
        match status_code {
            401 => format!("{} API Key 无效，请在设置中重新配置。", self.name),
            402 => format!("{} 账户余额不足，请充值后重试。", self.name),
            403 => format!("{} 返回 403：访问被拒绝，请检查 API Key 权限。", self.name),
            404 => format!("{} 返回 404：请求的资源不存在，请检查 API 地址配置。", self.name),
            422 => format!("{} 返回 422：请求参数有误。{}", self.name, detail),
            429 => format!("{} 返回 429：请求过于频繁，请稍后重试。", self.name),
            500..=599 => format!("{} 服务器错误 (HTTP {})，请稍后重试。", self.name, status_code),
            _ => format!("{} API 错误 (HTTP {}): {}", self.name, status_code, detail),
        }
    }
}

// ============================================================
// Anthropic Provider
// API 文档: https://docs.anthropic.com/en/api/messages
// ============================================================
pub struct AnthropicProvider;

impl AnthropicProvider {
    pub fn new() -> Self {
        Self
    }
}

impl ModelProvider for AnthropicProvider {
    fn provider_name(&self) -> &'static str {
        "Anthropic"
    }

    fn build_chat_url(&self, base_url: &str) -> String {
        format!("{}/v1/messages", base_url.trim_end_matches('/'))
    }

    fn build_headers(&self, api_key: &str) -> Vec<(String, String)> {
        vec![
            ("x-api-key".to_string(), api_key.to_string()),
            ("anthropic-version".to_string(), "2023-06-01".to_string()),
            ("Content-Type".to_string(), "application/json".to_string()),
        ]
    }

    fn build_request_body(
        &self,
        model: &str,
        request: &ChatCompletionRequest,
    ) -> serde_json::Value {
        // Anthropic 用 system 独立字段，messages 中不含 system role
        let system_msg = request
            .messages
            .iter()
            .find(|m| m.role == "system")
            .map(|m| m.content.clone());

        let messages: Vec<_> = request
            .messages
            .iter()
            .filter(|m| m.role != "system")
            .map(|m| {
                serde_json::json!({
                    "role": m.role,
                    "content": m.content,
                })
            })
            .collect();

        let mut body = serde_json::json!({
            "model": model,
            "messages": messages,
            "max_tokens": request.max_tokens,
        });

        if let Some(sys) = system_msg {
            body["system"] = serde_json::json!(sys);
        }
        if request.temperature > 0.0 {
            body["temperature"] = serde_json::json!(request.temperature);
        }

        body
    }

    fn extract_content(&self, response: &serde_json::Value) -> Result<String, String> {
        // Anthropic 响应格式: { "content": [{ "type": "text", "text": "..." }] }
        response["content"]
            .as_array()
            .and_then(|blocks| blocks.first())
            .and_then(|b| b["text"].as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| {
                // 可能是错误响应
                if let Some(err) = response["error"]["message"].as_str() {
                    format!("Anthropic API 错误: {}", err)
                } else {
                    "Anthropic 返回了空响应".to_string()
                }
            })
    }

    fn extract_usage(
        &self,
        _response: &serde_json::Value,
    ) -> Option<ChatCompletionResponse> {
        None
    }

    fn extract_finish_reason(&self, response: &serde_json::Value) -> Option<String> {
        response["stop_reason"].as_str().map(|s| s.to_string())
    }

    fn normalize_error(&self, status_code: u16, error_body: &str) -> String {
        let detail = &error_body[..error_body.len().min(300)];
        // 尝试解析 Anthropic 错误格式
        let msg = serde_json::from_str::<serde_json::Value>(error_body)
            .ok()
            .and_then(|v| v["error"]["message"].as_str().map(|s| s.to_string()))
            .unwrap_or_else(|| detail.to_string());

        match status_code {
            401 => "Anthropic API Key 无效，请在设置中重新配置。".to_string(),
            403 => "Anthropic 返回 403：访问被拒绝，请检查 API Key 权限。".to_string(),
            429 => "Anthropic 返回 429：请求过于频繁，请稍后重试。".to_string(),
            500..=599 => format!("Anthropic 服务器错误 (HTTP {}): {}", status_code, msg),
            _ => format!("Anthropic API 错误 (HTTP {}): {}", status_code, msg),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_openai_build_chat_url() {
        let provider = OpenAiCompatibleProvider::new("DeepSeek");
        let url = provider.build_chat_url("https://api.deepseek.com");
        assert_eq!(url, "https://api.deepseek.com/v1/chat/completions");
    }

    #[test]
    fn test_openai_build_chat_url_trailing_slash() {
        let provider = OpenAiCompatibleProvider::new("OpenWebUI");
        let url = provider.build_chat_url("http://localhost:3000/");
        assert_eq!(url, "http://localhost:3000/v1/chat/completions");
    }

    #[test]
    fn test_openai_build_headers_contains_bearer() {
        let provider = OpenAiCompatibleProvider::new("DeepSeek");
        let headers = provider.build_headers("sk-test-key");
        assert!(headers.iter().any(|(k, v)| k == "Authorization" && v == "Bearer sk-test-key"));
        assert!(headers.iter().any(|(k, v)| k == "Content-Type" && v == "application/json"));
    }

    #[test]
    fn test_anthropic_build_chat_url() {
        let provider = AnthropicProvider::new();
        let url = provider.build_chat_url("https://api.anthropic.com");
        assert_eq!(url, "https://api.anthropic.com/v1/messages");
    }

    #[test]
    fn test_anthropic_build_headers_contains_x_api_key() {
        let provider = AnthropicProvider::new();
        let headers = provider.build_headers("sk-ant-test");
        assert!(headers.iter().any(|(k, v)| k == "x-api-key" && v == "sk-ant-test"));
        assert!(headers.iter().any(|(k, v)| k == "anthropic-version" && v == "2023-06-01"));
    }

    #[test]
    fn test_openai_provider_name() {
        let provider = OpenAiCompatibleProvider::new("CustomProvider");
        assert_eq!(provider.provider_name(), "CustomProvider");
    }

    #[test]
    fn test_anthropic_provider_name() {
        let provider = AnthropicProvider::new();
        assert_eq!(provider.provider_name(), "Anthropic");
    }
}
