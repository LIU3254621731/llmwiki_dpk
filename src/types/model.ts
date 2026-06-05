export interface ProviderConfig {
  provider: string;
  base_url: string;
  chat_model: string;
  reasoner_model: string;
  temperature: number;
  max_tokens: number;
  timeout: number;
  retry_count: number;
  stream: boolean;
  api_key_masked: string;
}

/** @deprecated 保留向后兼容，请使用 ProviderConfig */
export interface DeepSeekConfig {
  base_url: string;
  chat_model: string;
  reasoner_model: string;
  temperature: number;
  max_tokens: number;
  timeout: number;
  retry_count: number;
  stream: boolean;
  api_key_masked: string;
  provider?: string;
}

export interface ModelProfile {
  id: string;
  provider: string;
  name: string;
  model_name: string;
  role: string;
}

export const PROVIDER_DEFAULTS: Record<string, { label: string; baseUrl: string; chatModel: string }> = {
  deepseek: { label: "DeepSeek", baseUrl: "https://api.deepseek.com", chatModel: "deepseek-chat" },
  openai: { label: "OpenAI", baseUrl: "https://api.openai.com/v1", chatModel: "gpt-4o" },
  anthropic: { label: "Anthropic", baseUrl: "https://api.anthropic.com", chatModel: "claude-sonnet-4-6" },
  ollama: { label: "Ollama", baseUrl: "http://localhost:11434/v1", chatModel: "llama3" },
  openwebui: { label: "OpenWebUI", baseUrl: "http://localhost:3000/api", chatModel: "default" },
  custom: { label: "自定义", baseUrl: "", chatModel: "" },
};
