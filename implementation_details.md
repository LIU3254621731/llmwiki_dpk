实现细节与代码位置映射（关键函数、输入/输出示例）

说明：下面把主要面试问答中会被问到的实现点，映射到仓库内的真实文件与函数，给出输入/输出结构示例，便于你在面试中逐条讲解并展示代码位置。

1) 系统架构与前端入口
- 代码位置：`src/App.tsx`, `src/main.tsx`, `src/pages/`、`src/components/`。
- 功能与 IO：前端通过 Tauri RPC 调用后端命令（见 `src-tauri/src/commands/*.rs`），典型调用例如 `run_query(kb_id, question, scope)`，返回字符串（模型回复）或 JSON 结构。

2) 模型调用入口（核心）
- 代码位置：`src-tauri/src/model/model_gateway.rs`（主网关），`src-tauri/src/model/deepseek_client.rs`（HTTP 客户端实现）。
- 关键函数：
  - `ModelGateway::chat(config: &DeepSeekConfig, messages: Vec<ChatMessage>, use_json_mode: bool) -> Result<ModelResult, String>`
    - 输入（Rust 结构）：
      {
        model: String,
        messages: [{ role: String, content: String }, ...],
        temperature: f64,
        max_tokens: u32,
        stream: bool,
      }
    - 输出（ModelResult）：
      {
        content: String,       // 模型原始返回文本（可能是 JSON 字符串）
        model: String,
        usage: Option<UsageInfo>,
        finish_reason: Option<String>,
      }
  - `ModelGateway::chat_with_content(config, system_prompt, user_content, use_json_mode)`：用于把 system + user 两段合并发送。

3) HTTP 调用与重试策略
- 代码位置：`src-tauri/src/model/deepseek_client.rs`。
- 实现点：使用 `reqwest::Client`，`chat_completion()` 函数负责：构造 POST /v1/chat/completions、加入 `Authorization` 头、超时控制、最大重试（指数退避）、解析 JSON 到 `ChatCompletionResponse`。
- 错误处理：若 JSON 无法解析，返回错误；对 4xx（非 429）不重试；超时与网络错误会重试并延长等待。

4) Prompt 管线（构建与存储）
- 代码位置：`src-tauri/src/prompts/prompt_builder.rs`（构建逻辑），`src-tauri/src/prompts/prompt_registry.rs`（模板注册）。
- 关键方法：
  - `PromptBuilder::build_ingest_prompt(document_text, source_id, existing_pages_summary) -> (system_prompt, user_message)`
  - 返回值：两个 `String`；前端/任务调用会把它们包装到 `ChatMessage{role, content}` 里传给 `ModelGateway`。

5) API Key / 密钥管理
- 代码位置：`src-tauri/src/core/secret_service.rs`。
- 功能：`store_api_key`, `get_api_key`, `has_api_key`；持久化到 `secrets.dat`（XOR+Base64 简单加密）。面试可说明密钥不会被写入通用日志。

6) 任务与中间文件（prompt_*.md、model_raw_response_*）
- 代码位置：`src-tauri/src/commands/task.rs`，方法 `get_task_files` 会读取任务目录下的文件并返回：
  - `ingest_result`（ingest_result.json）、`prompts`（所有 prompt_*.md 的文本）、`model_responses`（model_raw_response_* 文件）、`extracted_text` 等。
- IO 示例（返回 JSON）：
  {
    task_dir: String,
    files: [String],
    ingest_result: String (JSON text),
    prompts: { "prompt_0.md": "...", ... },
    model_responses: { "model_raw_response_0.txt": "..." }
  }

7) 从前端到模型的完整调用链（以问答 run_query 为例）
- 前端：调用 Tauri 命令 `run_query(kb_id, question, scope)`（在前端 JS/TS 中通常是 `window.__TAURI.invoke('run_query', ...)`）。
- 命令实现：`src-tauri/src/commands/task.rs::run_query`。它构造 `CoordinatorAgent` 并调用 `coordinator.run_query(...)`。
- 协调器/代理：相关逻辑在 `src-tauri/src/agents/`（例如 `coordinator.rs`、`query_agent.rs`），代理会使用 `PromptBuilder` 构造 prompt、调用 `ModelGateway::chat_with_content`，并将返回结果写入任务目录（`model_raw_response_*.txt`）与数据库索引，最后返回简短回答给前端。

8) Prompt 优化与评估流程
- 位置参考：提示构建在 `src-tauri/src/prompts/`，评估/repair 在 `src-tauri/src/schema/json_repair.rs`（用于修复模型 JSON 输出）。模型输出质量评估通常通过自动化脚本或人工标注（项目中会在任务目录中保存 `ingest_result.json`、`resolution_result.json` 等用于离线评估）。

9) 常见输入/输出示例
- 发送的 ChatCompletionRequest（JSON 形式）示例：
  {
    "model": "gpt-xxx",
    "messages": [{"role":"system","content":"..."},{"role":"user","content":"..."}],
    "temperature": 0.0,
    "max_tokens": 500,
    "stream": false
  }
- 模型返回（ModelResult.content）可能是纯文本或 JSON 字符串，例如：
  "{\"entities\":[...], \"concepts\": [...]}" 或 "模型自然语言回答文本..."

10) 面试时怎么展示代码（建议步骤）
- 打开并演示：
  - `src-tauri/src/prompts/prompt_builder.rs`：逐行解释如何把上下文合成到 prompt。
  - `src-tauri/src/model/model_gateway.rs`：展示 `chat()` 的输入/输出结构、错误归一化函数 `normalize_error()`。
  - `src-tauri/src/model/deepseek_client.rs`：展示 HTTP 请求、超时和重试逻辑。
  - `src-tauri/src/commands/task.rs::get_task_files`：展示如何把任务中间文件暴露给前端用于调试与复现。

附录：重要文件快速跳转
- 前端入口: [src/App.tsx](src/App.tsx#L1) 、[src/main.tsx](src/main.tsx#L1)
- 模型网关: [src-tauri/src/model/model_gateway.rs](src-tauri/src/model/model_gateway.rs#L1)
- HTTP 客户端: [src-tauri/src/model/deepseek_client.rs](src-tauri/src/model/deepseek_client.rs#L1)
- Prompt 构建: [src-tauri/src/prompts/prompt_builder.rs](src-tauri/src/prompts/prompt_builder.rs#L1)
- 密钥管理: [src-tauri/src/core/secret_service.rs](src-tauri/src/core/secret_service.rs#L1)
- 任务文件读取: [src-tauri/src/commands/task.rs](src-tauri/src/commands/task.rs#L1)

—— 结束 ——
