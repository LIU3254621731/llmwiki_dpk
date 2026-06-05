use std::fs;
use std::path::{Path, PathBuf};

/// 通用模型供应商配置（取代 DeepSeekConfig）
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ProviderConfig {
    pub provider: String,
    pub base_url: String,
    pub chat_model: String,
    pub reasoner_model: String,
    pub temperature: f64,
    pub max_tokens: u32,
    pub timeout: u32,
    pub retry_count: u32,
    pub stream: bool,
}

impl Default for ProviderConfig {
    fn default() -> Self {
        Self {
            provider: "deepseek".to_string(),
            base_url: "https://api.deepseek.com".to_string(),
            chat_model: "deepseek-chat".to_string(),
            reasoner_model: "deepseek-reasoner".to_string(),
            temperature: 0.7,
            max_tokens: 4096,
            timeout: 120,
            retry_count: 3,
            stream: true,
        }
    }
}

/// 保留 DeepSeekConfig 作为兼容别名（自动与 ProviderConfig 互转）
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DeepSeekConfig {
    pub base_url: String,
    pub chat_model: String,
    pub reasoner_model: String,
    pub temperature: f64,
    pub max_tokens: u32,
    pub timeout: u32,
    pub retry_count: u32,
    pub stream: bool,
}

impl DeepSeekConfig {
    pub fn from_provider(p: &ProviderConfig) -> Self {
        Self {
            base_url: p.base_url.clone(),
            chat_model: p.chat_model.clone(),
            reasoner_model: p.reasoner_model.clone(),
            temperature: p.temperature,
            max_tokens: p.max_tokens,
            timeout: p.timeout,
            retry_count: p.retry_count,
            stream: p.stream,
        }
    }
}

impl Default for DeepSeekConfig {
    fn default() -> Self {
        let p = ProviderConfig::default();
        Self::from_provider(&p)
    }
}

impl ProviderConfig {
    pub fn from_deepseek(d: &DeepSeekConfig) -> Self {
        Self {
            provider: "deepseek".to_string(),
            base_url: d.base_url.clone(),
            chat_model: d.chat_model.clone(),
            reasoner_model: d.reasoner_model.clone(),
            temperature: d.temperature,
            max_tokens: d.max_tokens,
            timeout: d.timeout,
            retry_count: d.retry_count,
            stream: d.stream,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct KbConfig {
    pub name: String,
    pub template_name: String,
    pub language: String,
    pub review_mode: String, // strict | balanced | auto
    #[serde(default = "default_allow_ai_generation")]
    pub allow_ai_generation: bool,
}

fn default_allow_ai_generation() -> bool { true }

impl Default for KbConfig {
    fn default() -> Self {
        Self {
            name: "我的知识库".to_string(),
            template_name: "general".to_string(),
            language: "zh-CN".to_string(),
            review_mode: "balanced".to_string(),
            allow_ai_generation: true,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WebSearchConfig {
    pub engine: String,
    pub max_results: u32,
    pub searxng_url: String,
    pub brave_api_key: String,
    pub bing_api_key: String,
    pub bing_endpoint: String,
}

impl Default for WebSearchConfig {
    fn default() -> Self {
        Self {
            engine: "duckduckgo".to_string(),
            max_results: 10,
            searxng_url: String::new(),
            brave_api_key: String::new(),
            bing_api_key: String::new(),
            bing_endpoint: "https://api.bing.microsoft.com/".to_string(),
        }
    }
}

pub struct ConfigService {
    config_dir: PathBuf,
}

impl ConfigService {
    pub fn new(config_dir: &PathBuf) -> Self {
        Self { config_dir: config_dir.clone() }
    }

    // === 通用 Provider 配置 ===

    pub fn save_provider_config(&self, config: &ProviderConfig) -> Result<(), String> {
        let path = self.config_dir.join("provider.json");
        let json = serde_json::to_string_pretty(config)
            .map_err(|e| format!("序列化配置失败: {}", e))?;
        fs::write(&path, json)
            .map_err(|e| format!("保存 Provider 配置失败: {}", e))?;
        // 同时保存 deepseek.json 以兼容旧代码
        self.save_deepseek_config(&DeepSeekConfig::from_provider(config))?;
        Ok(())
    }

    pub fn get_provider_config(&self) -> Result<ProviderConfig, String> {
        let path = self.config_dir.join("provider.json");
        if path.exists() {
            let content = fs::read_to_string(&path)
                .map_err(|e| format!("读取配置失败: {}", e))?;
            serde_json::from_str(&content)
                .map_err(|e| format!("解析配置失败: {}", e))
        } else {
            // 从旧的 deepseek.json 迁移
            self.get_deepseek_config().map(|c| ProviderConfig::from_deepseek(&c))
        }
    }

    // === DeepSeek 配置（保留向后兼容） ===

    pub fn save_deepseek_config(&self, config: &DeepSeekConfig) -> Result<(), String> {
        let path = self.config_dir.join("deepseek.json");
        let json = serde_json::to_string_pretty(config)
            .map_err(|e| format!("序列化配置失败: {}", e))?;
        fs::write(&path, json)
            .map_err(|e| format!("保存 DeepSeek 配置失败: {}", e))?;
        Ok(())
    }

    pub fn get_deepseek_config(&self) -> Result<DeepSeekConfig, String> {
        let path = self.config_dir.join("deepseek.json");
        if path.exists() {
            let content = fs::read_to_string(&path)
                .map_err(|e| format!("读取配置失败: {}", e))?;
            serde_json::from_str(&content)
                .map_err(|e| format!("解析配置失败: {}", e))
        } else {
            Ok(DeepSeekConfig::default())
        }
    }

    // === 知识库配置 ===

    pub fn save_kb_config(&self, kb_path: &Path, config: &KbConfig) -> Result<(), String> {
        let config_path = kb_path.join("config").join("kb.config.json");
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("创建配置目录失败: {}", e))?;
        }
        let json = serde_json::to_string_pretty(config)
            .map_err(|e| format!("序列化失败: {}", e))?;
        fs::write(&config_path, json)
            .map_err(|e| format!("保存知识库配置失败: {}", e))?;
        Ok(())
    }

    pub fn get_kb_config(&self, kb_path: &Path) -> Result<KbConfig, String> {
        let config_path = kb_path.join("config").join("kb.config.json");
        if config_path.exists() {
            let content = fs::read_to_string(&config_path)
                .map_err(|e| format!("读取知识库配置失败: {}", e))?;
            serde_json::from_str(&content)
                .map_err(|e| format!("解析知识库配置失败: {}", e))
        } else {
            Ok(KbConfig::default())
        }
    }

    pub fn save_web_search_config(&self, config: &WebSearchConfig) -> Result<(), String> {
        let path = self.config_dir.join("web_search.json");
        let json = serde_json::to_string_pretty(config)
            .map_err(|e| format!("序列化配置失败: {}", e))?;
        fs::write(&path, json)
            .map_err(|e| format!("保存网页搜索配置失败: {}", e))?;
        log::info!("网页搜索配置已保存");
        Ok(())
    }

    pub fn get_web_search_config(&self) -> Result<WebSearchConfig, String> {
        let path = self.config_dir.join("web_search.json");
        if path.exists() {
            let content = fs::read_to_string(&path)
                .map_err(|e| format!("读取配置失败: {}", e))?;
            serde_json::from_str(&content)
                .map_err(|e| format!("解析配置失败: {}", e))
        } else {
            Ok(WebSearchConfig::default())
        }
    }

    pub fn get_config_dir(&self) -> &Path {
        &self.config_dir
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_config_default() {
        let config = ProviderConfig::default();
        assert_eq!(config.provider, "deepseek");
        assert_eq!(config.base_url, "https://api.deepseek.com");
        assert_eq!(config.chat_model, "deepseek-chat");
        assert_eq!(config.reasoner_model, "deepseek-reasoner");
        assert_eq!(config.temperature, 0.7);
        assert_eq!(config.max_tokens, 4096);
        assert_eq!(config.timeout, 120);
        assert_eq!(config.retry_count, 3);
        assert!(config.stream);
    }

    #[test]
    fn test_deepseek_from_provider() {
        let p = ProviderConfig::default();
        let d = DeepSeekConfig::from_provider(&p);
        assert_eq!(d.base_url, "https://api.deepseek.com");
        assert_eq!(d.chat_model, "deepseek-chat");
        assert_eq!(d.reasoner_model, "deepseek-reasoner");
        assert_eq!(d.temperature, 0.7);
        assert_eq!(d.max_tokens, 4096);
        assert_eq!(d.timeout, 120);
        assert_eq!(d.retry_count, 3);
        assert!(d.stream);
    }

    #[test]
    fn test_deepseek_default_matches_provider_default() {
        let p = ProviderConfig::default();
        let d = DeepSeekConfig::default();
        assert_eq!(d.base_url, p.base_url);
        assert_eq!(d.chat_model, p.chat_model);
        assert_eq!(d.reasoner_model, p.reasoner_model);
        assert_eq!(d.temperature, p.temperature);
        assert_eq!(d.max_tokens, p.max_tokens);
    }

    #[test]
    fn test_kb_config_default() {
        let config = KbConfig::default();
        assert_eq!(config.name, "我的知识库");
        assert_eq!(config.template_name, "general");
        assert_eq!(config.language, "zh-CN");
        assert_eq!(config.review_mode, "balanced");
    }

    #[test]
    fn test_web_search_config_default() {
        let config = WebSearchConfig::default();
        assert_eq!(config.engine, "duckduckgo");
        assert_eq!(config.max_results, 10);
    }
}
