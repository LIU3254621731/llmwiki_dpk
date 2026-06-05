/// Web Search Skill — 多引擎搜索集成
/// 支持: DuckDuckGo (免费) / SearXNG (自部署) / Brave (需 API Key) / Bing (需 API Key)
///
/// 参考: OpenWebUI (https://github.com/open-webui/open-webui)
/// 其 retrieval/web/ 模块提供了多引擎搜索的完整实现
use scraper::{Html, Selector};
use std::time::Duration;

#[derive(Debug, Clone, serde::Serialize)]
pub struct SearchResult {
    pub title: String,
    pub url: String,
    pub snippet: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct WebPageContent {
    pub title: String,
    pub url: String,
    pub content: String,
    pub content_length: usize,
}

/// 搜索引擎配置（由调用方传入）
#[derive(Debug, Clone)]
pub struct EngineConfig {
    pub engine: String,
    pub max_results: u32,
    pub searxng_url: String,
    pub brave_api_key: String,
    pub bing_api_key: String,
    pub bing_endpoint: String,
}

pub struct WebSearchSkill;

impl WebSearchSkill {
    fn build_client(timeout_secs: u64) -> Result<reqwest::Client, String> {
        reqwest::Client::builder()
            .timeout(Duration::from_secs(timeout_secs))
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0.0.0 Safari/537.36")
            .build()
            .map_err(|e| format!("创建 HTTP 客户端失败: {}", e))
    }

    // ========================================================================
    // 公共入口：根据引擎名称分发
    // ========================================================================

    pub async fn search(config: &EngineConfig, query: &str) -> Result<Vec<SearchResult>, String> {
        match config.engine.as_str() {
            "searxng" => Self::search_searxng(config, query).await,
            "brave" => Self::search_brave(config, query).await,
            "bing" => Self::search_bing(config, query).await,
            _ => Self::search_duckduckgo(query, config.max_results).await,
        }
    }

    // ========================================================================
    // DuckDuckGo — 多端点自动回退（HTML 抓取 + 官方 API）
    // ========================================================================

    async fn search_duckduckgo(query: &str, max_results: u32) -> Result<Vec<SearchResult>, String> {
        let client = Self::build_client(15)?;

        let endpoints: Vec<(&str, &str, &str)> = vec![
            ("lite", "https://lite.duckduckgo.com/lite/", "html"),
            ("html", "https://html.duckduckgo.com/html/", "html"),
            ("duckduckgo", "https://duckduckgo.com/html/", "html"),
            ("api", "https://api.duckduckgo.com/", "json"),
        ];

        let mut last_err = String::new();
        for (label, base_url, ep_type) in &endpoints {
            let url = if *ep_type == "json" {
                format!("{}?q={}&format=json&no_html=1&skip_disambig=1", base_url, urlencoding(query))
            } else {
                format!("{}?q={}", base_url, urlencoding(query))
            };
            match Self::try_fetch(&client, &url, label).await {
                Ok(body) => {
                    let results = if *ep_type == "json" {
                        Self::parse_ddg_api(&body, max_results)
                    } else if *label == "lite" {
                        Self::parse_lite_results(&body, max_results)
                    } else {
                        Self::parse_html_results(&body, max_results)
                    };
                    if !results.is_empty() {
                        return Ok(results);
                    }
                    last_err = format!("{} 端点返回空结果", label);
                }
                Err(e) => {
                    last_err = e;
                    continue;
                }
            }
        }

        Err(format!("所有 DuckDuckGo 端点均不可用。最后错误: {}", last_err))
    }

    /// 解析 DuckDuckGo 官方 API JSON 响应
    fn parse_ddg_api(json: &str, max_results: u32) -> Vec<SearchResult> {
        let mut results = Vec::new();

        let parsed: serde_json::Value = match serde_json::from_str(json) {
            Ok(v) => v,
            Err(_) => return results,
        };

        // Abstract (Wikipedia 摘要)
        if let (Some(title), Some(url), Some(text)) = (
            parsed.get("Heading").and_then(|v| v.as_str()),
            parsed.get("AbstractURL").and_then(|v| v.as_str()),
            parsed.get("AbstractText").and_then(|v| v.as_str()),
        ) {
            if !text.is_empty() {
                results.push(SearchResult {
                    title: title.to_string(),
                    url: url.to_string(),
                    snippet: text.to_string(),
                });
            }
        }

        // RelatedTopics
        if let Some(topics) = parsed.get("RelatedTopics").and_then(|v| v.as_array()) {
            for topic in topics {
                if results.len() >= max_results as usize {
                    break;
                }
                if let (Some(text), Some(url)) = (
                    topic.get("Text").and_then(|v| v.as_str()),
                    topic.get("FirstURL").and_then(|v| v.as_str()),
                ) {
                    if !text.is_empty() {
                        results.push(SearchResult {
                            title: text.chars().take(80).collect(),
                            url: url.to_string(),
                            snippet: text.to_string(),
                        });
                    }
                }
            }
        }

        results
    }

    // ========================================================================
    // SearXNG — 自部署元搜索引擎，无需 API Key
    // ========================================================================

    async fn search_searxng(config: &EngineConfig, query: &str) -> Result<Vec<SearchResult>, String> {
        let searxng_url = config.searxng_url.trim().trim_end_matches('/');
        if searxng_url.is_empty() {
            return Err("SearXNG 地址未配置，请在设置中填写 SearXNG 实例 URL".to_string());
        }

        let client = Self::build_client(15)?;
        let url = format!(
            "{}/search?q={}&format=json&categories=general",
            searxng_url,
            urlencoding(query)
        );

        let response = client.get(&url).send().await.map_err(|e| {
            if e.is_connect() {
                format!("无法连接 SearXNG ({})，请确认实例地址正确且服务正在运行", searxng_url)
            } else if e.is_timeout() {
                "SearXNG 请求超时 (15s)".to_string()
            } else {
                format!("SearXNG 请求失败: {}", e)
            }
        })?;

        let status = response.status();
        if !status.is_success() {
            return Err(format!("SearXNG 返回 HTTP {}", status));
        }

        let json: serde_json::Value = response.json().await
            .map_err(|e| format!("解析 SearXNG 响应失败: {}", e))?;

        let results_json = json.get("results").and_then(|v| v.as_array())
            .ok_or_else(|| "SearXNG 返回结果格式异常".to_string())?;

        let max = config.max_results as usize;
        let results: Vec<SearchResult> = results_json.iter()
            .take(max)
            .map(|r| SearchResult {
                title: r.get("title").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                url: r.get("url").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                snippet: r.get("content").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            })
            .filter(|r| !r.title.is_empty())
            .collect();

        if results.is_empty() {
            return Err("SearXNG 未返回任何结果".to_string());
        }
        Ok(results)
    }

    // ========================================================================
    // Brave Search — 免费额度 2000 次/月，需 API Key
    // ========================================================================

    async fn search_brave(config: &EngineConfig, query: &str) -> Result<Vec<SearchResult>, String> {
        if config.brave_api_key.trim().is_empty() {
            return Err("Brave Search API Key 未配置，请在设置中填写".to_string());
        }

        let client = Self::build_client(15)?;
        let url = format!(
            "https://api.search.brave.com/res/v1/web/search?q={}&count={}",
            urlencoding(query),
            config.max_results
        );

        let response = client.get(&url)
            .header("Accept", "application/json")
            .header("Accept-Encoding", "gzip")
            .header("X-Subscription-Token", config.brave_api_key.trim())
            .send().await.map_err(|e| {
                if e.is_connect() {
                    "无法连接 Brave Search API".to_string()
                } else if e.is_timeout() {
                    "Brave Search 请求超时 (15s)".to_string()
                } else {
                    format!("Brave Search 请求失败: {}", e)
                }
            })?;

        let status = response.status();
        if status == 429 {
            return Err("Brave Search API 频率限制 (429)，免费套餐限 1 次/秒，请稍后再试".to_string());
        }
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(format!("Brave Search 返回 HTTP {}: {}", status, body));
        }

        let json: serde_json::Value = response.json().await
            .map_err(|e| format!("解析 Brave Search 响应失败: {}", e))?;

        let web_results = json.get("web").and_then(|v| v.get("results"))
            .and_then(|v| v.as_array())
            .ok_or_else(|| "Brave Search 返回结果格式异常".to_string())?;

        let results: Vec<SearchResult> = web_results.iter()
            .map(|r| SearchResult {
                title: r.get("title").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                url: r.get("url").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                snippet: r.get("description").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            })
            .filter(|r| !r.title.is_empty())
            .collect();

        if results.is_empty() {
            return Err("Brave Search 未返回任何结果".to_string());
        }
        Ok(results)
    }

    // ========================================================================
    // Bing Web Search API — Azure 认知服务，需 API Key
    // ========================================================================

    async fn search_bing(config: &EngineConfig, query: &str) -> Result<Vec<SearchResult>, String> {
        if config.bing_api_key.trim().is_empty() {
            return Err("Bing Search API Key 未配置，请在设置中填写".to_string());
        }

        let endpoint = if config.bing_endpoint.trim().is_empty() {
            "https://api.bing.microsoft.com/"
        } else {
            config.bing_endpoint.trim().trim_end_matches('/')
        };

        let client = Self::build_client(15)?;
        let url = format!(
            "{}v7.0/search?q={}&count={}&mkt=zh-CN",
            endpoint,
            urlencoding(query),
            config.max_results.min(50)
        );

        let response = client.get(&url)
            .header("Ocp-Apim-Subscription-Key", config.bing_api_key.trim())
            .send().await.map_err(|e| {
                if e.is_connect() {
                    "无法连接 Bing Search API".to_string()
                } else if e.is_timeout() {
                    "Bing Search 请求超时 (15s)".to_string()
                } else {
                    format!("Bing Search 请求失败: {}", e)
                }
            })?;

        let status = response.status();
        if status == 401 || status == 403 {
            return Err("Bing Search API Key 无效或已过期".to_string());
        }
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(format!("Bing Search 返回 HTTP {}: {}", status, body));
        }

        let json: serde_json::Value = response.json().await
            .map_err(|e| format!("解析 Bing Search 响应失败: {}", e))?;

        let web_pages = json.get("webPages").and_then(|v| v.get("value"))
            .and_then(|v| v.as_array())
            .ok_or_else(|| "Bing Search 未返回网页结果".to_string())?;

        let results: Vec<SearchResult> = web_pages.iter()
            .map(|r| SearchResult {
                title: r.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                url: r.get("url").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                snippet: r.get("snippet").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            })
            .filter(|r| !r.title.is_empty())
            .collect();

        if results.is_empty() {
            return Err("Bing Search 未返回任何结果".to_string());
        }
        Ok(results)
    }

    // ========================================================================
    // 通用 HTTP 工具方法
    // ========================================================================

    async fn try_fetch(client: &reqwest::Client, url: &str, label: &str) -> Result<String, String> {
        let response = client.get(url).send().await.map_err(|e| {
            if e.is_timeout() {
                format!("{} 端点超时 (15s)", label)
            } else if e.is_connect() {
                format!("{} 端点无法连接", label)
            } else {
                format!("{} 端点请求失败: {}", label, e)
            }
        })?;

        let status = response.status();
        if status == 429 {
            return Err(format!("{} 端点频率限制 (HTTP 429)", label));
        }
        if !status.is_success() {
            return Err(format!("{} 端点返回 HTTP {}", label, status));
        }

        response.text().await.map_err(|e| format!("{} 端点读取响应失败: {}", label, e))
    }

    // ========================================================================
    // DuckDuckGo HTML 解析方法
    // ========================================================================

    fn parse_html_results(html: &str, max_results: u32) -> Vec<SearchResult> {
        let document = Html::parse_document(html);
        let result_sel = match Selector::parse(".result") { Ok(s) => s, Err(_) => return vec![] };
        let title_sel = match Selector::parse(".result__title a.result__a") { Ok(s) => s, Err(_) => return vec![] };
        let snippet_sel = match Selector::parse(".result__snippet") { Ok(s) => s, Err(_) => return vec![] };
        let url_sel = match Selector::parse(".result__url") { Ok(s) => s, Err(_) => return vec![] };

        let mut results = Vec::new();
        for result_node in document.select(&result_sel) {
            if results.len() >= max_results as usize { break; }

            let title = result_node.select(&title_sel).next()
                .map(|el| el.text().collect::<Vec<_>>().join("").trim().to_string())
                .unwrap_or_default();

            let url = result_node.select(&url_sel).next()
                .map(|el| el.text().collect::<Vec<_>>().join("").trim().to_string())
                .unwrap_or_default();

            let snippet = result_node.select(&snippet_sel).next()
                .map(|el| el.text().collect::<Vec<_>>().join("").trim().to_string())
                .unwrap_or_default();

            if title.is_empty() { continue; }
            results.push(SearchResult { title, url, snippet });
        }
        results
    }

    fn parse_lite_results(html: &str, max_results: u32) -> Vec<SearchResult> {
        let document = Html::parse_document(html);
        let link_sel = match Selector::parse("a.result-link") { Ok(s) => s, Err(_) => return vec![] };
        let snippet_sel = Selector::parse(".result-snippet").ok();

        let snippets: Vec<String> = snippet_sel.as_ref().map(|s| {
            document.select(s).map(|el| el.text().collect::<Vec<_>>().join("").trim().to_string()).collect()
        }).unwrap_or_default();

        let mut results = Vec::new();
        for (idx, link_el) in document.select(&link_sel).enumerate() {
            if results.len() >= max_results as usize { break; }

            let title = link_el.text().collect::<Vec<_>>().join("").trim().to_string();
            let url = link_el.value().attr("href").unwrap_or("").to_string()
                .trim().trim_start_matches("//").to_string();
            let url = if !url.is_empty() && !url.starts_with("http") {
                format!("https://{}", url)
            } else {
                url
            };

            if title.is_empty() { continue; }

            let snippet = snippets.get(idx).cloned().unwrap_or_default();
            results.push(SearchResult { title, url, snippet });
        }

        if results.is_empty() {
            results = Self::parse_generic_links(&document, max_results);
        }

        results
    }

    fn parse_generic_links(document: &Html, max_results: u32) -> Vec<SearchResult> {
        let a_sel = match Selector::parse("a[href]") { Ok(s) => s, Err(_) => return vec![] };
        let mut results = Vec::new();
        for a_el in document.select(&a_sel) {
            if results.len() >= max_results as usize { break; }
            let href = a_el.value().attr("href").unwrap_or("").to_string();
            if href.starts_with('#') || href.starts_with('/') && !href.starts_with("//")
                || href.is_empty() || href.starts_with("javascript:")
            {
                continue;
            }
            let title = a_el.text().collect::<Vec<_>>().join("").trim().to_string();
            if title.len() < 3 { continue; }
            let url = if !href.starts_with("http") { format!("https://{}", href.trim_start_matches("//")) } else { href };
            results.push(SearchResult { title, url, snippet: String::new() });
        }
        results
    }

    // ========================================================================
    // 网页内容抓取
    // ========================================================================

    pub async fn fetch_page_content(url: &str) -> Result<WebPageContent, String> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(20))
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0.0.0 Safari/537.36")
            .build()
            .map_err(|e| format!("创建 HTTP 客户端失败: {}", e))?;

        let response = client.get(url).send().await.map_err(|e| {
            if e.is_timeout() { "网页请求超时 (20s)".to_string() }
            else if e.is_connect() { format!("无法连接到 {}", url) }
            else { format!("请求失败: {}", e) }
        })?;

        let status = response.status();
        if !status.is_success() {
            return Err(format!("网页返回 HTTP {}", status));
        }

        let html = response.text().await.map_err(|e| format!("读取网页内容失败: {}", e))?;
        let document = Html::parse_document(&html);

        let title = Selector::parse("title").ok()
            .and_then(|s| document.select(&s).next())
            .map(|el| el.text().collect::<Vec<_>>().join("").trim().to_string())
            .or_else(|| {
                Selector::parse("h1").ok()
                    .and_then(|s| document.select(&s).next())
                    .map(|el| el.text().collect::<Vec<_>>().join("").trim().to_string())
            })
            .unwrap_or_else(|| "无标题".to_string());

        let article_sel = Selector::parse("article").ok();
        let main_sel = Selector::parse("main").ok();
        let body_sel = Selector::parse("body").ok();

        let content_root = article_sel.as_ref()
            .and_then(|s| document.select(s).next())
            .or_else(|| main_sel.as_ref().and_then(|s| document.select(s).next()))
            .or_else(|| body_sel.as_ref().and_then(|s| document.select(s).next()))
            .unwrap_or_else(|| document.root_element());

        let raw_text = content_root.text().collect::<Vec<_>>().join(" ");
        let cleaned: String = raw_text
            .lines()
            .map(|l| l.trim())
            .filter(|l| !l.is_empty())
            .collect::<Vec<_>>()
            .join("\n");

        let max_len = 30000usize;
        let content = if cleaned.chars().count() > max_len {
            let truncated: String = cleaned.chars().take(max_len).collect();
            format!("{}...\n\n[内容已截断，原文共 {} 字符]", truncated, cleaned.chars().count())
        } else {
            cleaned
        };

        Ok(WebPageContent {
            title,
            url: url.to_string(),
            content_length: content.len(),
            content,
        })
    }
}

/// URL 编码
fn urlencoding(s: &str) -> String {
    let mut encoded = String::with_capacity(s.len() * 3);
    for byte in s.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9'
            | b'-' | b'_' | b'.' | b'~' => encoded.push(byte as char),
            b' ' => encoded.push('+'),
            _ => encoded.push_str(&format!("%{:02X}", byte)),
        }
    }
    encoded
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_urlencoding_ascii() {
        assert_eq!(urlencoding("hello world"), "hello+world");
    }

    #[test]
    fn test_urlencoding_chinese() {
        let encoded = urlencoding("知识图谱");
        assert!(encoded.contains("%E7%9F%A5%E8%AF%86"));
    }

    #[test]
    fn test_parse_ddg_api() {
        let json = r#"{"Heading":"Test","AbstractURL":"https://example.com","AbstractText":"A test result","RelatedTopics":[{"FirstURL":"https://example.com/1","Text":"Topic one"}]}"#;
        let results = WebSearchSkill::parse_ddg_api(json, 10);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].title, "Test");
        assert_eq!(results[1].title, "Topic one");
    }
}
