use std::time::Duration;
use futures::StreamExt;
use reqwest::Client;
use tokio::sync::mpsc;
use crate::model::model_gateway::{ChatCompletionRequest, ChatCompletionResponse};

pub struct DeepSeekClient {
    client: Client,
}

impl Default for DeepSeekClient {
    fn default() -> Self {
        Self::new()
    }
}

impl DeepSeekClient {
    pub fn new() -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(300))
            .build()
            .unwrap_or_else(|e| {
                log::error!("[DeepSeekClient] 构建 HTTP 客户端失败（将使用默认客户端，可能缺少 TLS 超时配置）: {}", e);
                Client::default()
            });
        Self { client }
    }

    /// 通用 HTTP JSON POST（供 ModelGateway provider-agnostic 调用）
    pub async fn chat_completion_generic(
        &self,
        url: &str,
        headers: &[(String, String)],
        body: &serde_json::Value,
        timeout: Duration,
        max_retries: u32,
    ) -> Result<serde_json::Value, String> {
        let mut last_error = String::new();

        for attempt in 0..=max_retries {
            if attempt > 0 {
                let delay = Duration::from_secs(2u64.pow(attempt));
                log::info!("重试第 {} 次，等待 {} 秒...", attempt, delay.as_secs());
                tokio::time::sleep(delay).await;
            }

            let mut req = self.client.post(url).timeout(timeout);
            for (k, v) in headers {
                req = req.header(k.as_str(), v.as_str());
            }
            req = req.json(body);

            match req.send().await {
                Ok(resp) => {
                    let status = resp.status();
                    let resp_text = resp
                        .text()
                        .await
                        .unwrap_or_else(|e| format!("[读取响应体失败: {}]", e));

                    if status.is_success() {
                        match serde_json::from_str::<serde_json::Value>(&resp_text) {
                            Ok(json) => return Ok(json),
                            Err(e) => {
                                last_error = format!(
                                    "解析响应 JSON 失败: {}, 原始响应: {}",
                                    e,
                                    &resp_text[..resp_text.len().min(500)]
                                );
                                continue;
                            }
                        }
                    } else {
                        last_error = format!(
                            "HTTP {}: {}",
                            status.as_u16(),
                            &resp_text[..resp_text.len().min(300)]
                        );
                        if status.as_u16() >= 400 && status.as_u16() < 500 && status.as_u16() != 429 {
                            break;
                        }
                    }
                }
                Err(e) => {
                    last_error = format!("网络请求失败: {}", e);
                    if e.is_timeout() {
                        tokio::time::sleep(Duration::from_secs(5)).await;
                    }
                }
            }
        }

        Err(last_error)
    }

    /// 流式 Chat Completion：发送请求后通过 channel 逐块返回文本增量
    pub async fn chat_completion_stream(
        &self,
        url: &str,
        headers: &[(String, String)],
        body: &serde_json::Value,
        timeout: Duration,
    ) -> Result<mpsc::Receiver<String>, String> {
        let mut stream_body = body.clone();
        stream_body["stream"] = serde_json::json!(true);

        let mut req = self.client.post(url).timeout(timeout);
        for (k, v) in headers {
            req = req.header(k.as_str(), v.as_str());
        }
        req = req.json(&stream_body);

        let resp = req.send().await.map_err(|e| format!("流式请求失败: {}", e))?;
        let status = resp.status();
        if !status.is_success() {
            let resp_text = resp.text().await.unwrap_or_else(|e| format!("[读取响应体失败: {}]", e));
            return Err(format!("HTTP {}: {}", status.as_u16(), &resp_text[..resp_text.len().min(300)]));
        }

        let (tx, rx) = mpsc::channel(64);
        let mut byte_stream = resp.bytes_stream();

        tokio::spawn(async move {
            let mut buf = String::new();
            while let Some(chunk_result) = byte_stream.next().await {
                match chunk_result {
                    Ok(bytes) => {
                        buf.push_str(&String::from_utf8_lossy(&bytes));
                        while let Some(pos) = buf.find("\n\n") {
                            let event = buf[..pos].to_string();
                            buf = buf[pos + 2..].to_string();
                            for line in event.lines() {
                                if let Some(data) = line.strip_prefix("data: ") {
                                    if data == "[DONE]" {
                                        break;
                                    }
                                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(data) {
                                        if let Some(content) = json["choices"][0]["delta"]["content"].as_str() {
                                            if tx.send(content.to_string()).await.is_err() {
                                                return; // receiver dropped
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        log::error!("[DeepSeekClient] 流式读取错误: {}", e);
                        break;
                    }
                }
            }
        });

        Ok(rx)
    }

    /// 发送 Chat Completion 请求（保留为 DeepSeek 兼容路径）
    pub async fn chat_completion(
        &self,
        base_url: &str,
        api_key: &str,
        request: &ChatCompletionRequest,
        timeout: Duration,
        max_retries: u32,
    ) -> Result<ChatCompletionResponse, String> {
        let url = format!("{}/v1/chat/completions", base_url.trim_end_matches('/'));

        let mut last_error = String::new();

        for attempt in 0..=max_retries {
            if attempt > 0 {
                // 指数退避
                let delay = Duration::from_secs(2u64.pow(attempt));
                log::info!("重试第 {} 次，等待 {} 秒...", attempt, delay.as_secs());
                tokio::time::sleep(delay).await;
            }

            let result = self
                .client
                .post(&url)
                .header("Authorization", format!("Bearer {}", api_key))
                .header("Content-Type", "application/json")
                .timeout(timeout)
                .json(request)
                .send()
                .await;

            match result {
                Ok(resp) => {
                    let status = resp.status();

                    // 获取响应文本用于错误诊断
                    let resp_text = resp.text().await.unwrap_or_else(|e| format!("[读取响应体失败: {}]", e));

                    if status.is_success() {
                        match serde_json::from_str::<ChatCompletionResponse>(&resp_text) {
                            Ok(completion) => return Ok(completion),
                            Err(e) => {
                                last_error = format!(
                                    "解析 DeepSeek 响应失败: {}, 原始响应: {}",
                                    e,
                                    &resp_text[..resp_text.len().min(500)]
                                );
                                continue;
                            }
                        }
                    } else {
                        last_error = format!(
                            "HTTP {}: {}",
                            status.as_u16(),
                            &resp_text[..resp_text.len().min(300)]
                        );

                        // 4xx 错误不重试（除 429）
                        if status.as_u16() >= 400 && status.as_u16() < 500 && status.as_u16() != 429 {
                            break;
                        }
                    }
                }
                Err(e) => {
                    last_error = format!("网络请求失败: {}", e);
                    if e.is_timeout() {
                        // 超时可以重试，但延长等待
                        tokio::time::sleep(Duration::from_secs(5)).await;
                    }
                }
            }
        }

        Err(last_error)
    }


    /// 估算 token 数量（简易版：中文约 1.5 字符/token，英文约 4 字符/token）
    pub fn estimate_tokens(text: &str) -> u32 {
        let mut chinese_chars = 0;
        let mut other_chars = 0;

        for c in text.chars() {
            if c as u32 >= 0x4e00 && c as u32 <= 0x9fff {
                chinese_chars += 1;
            } else {
                other_chars += 1;
            }
        }

        // 中文约 1.5 字符/token，英文约 4 字符/token
        let tokens = (chinese_chars as f64 / 1.5) + (other_chars as f64 / 4.0);
        tokens.ceil() as u32
    }
}
