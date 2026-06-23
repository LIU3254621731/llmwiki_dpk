# Agent、Skill、MCP、RAG、向量检索 完全学习指南

> 从零学习 AI 系统核心架构，包括项目实战应用和完整代码示例

**学习时间：** 4-6 周（系统学习）  
**前置要求：** 基本的 Rust/Python/TypeScript 编程基础

---

## 📋 完整学习地图

```
第一阶段：基础概念（1-2周）
  ├─ Agent 系统设计
  ├─ Skill 模式
  └─ 项目中的架构

第二阶段：实战应用（1-2周）
  ├─ RAG 系统
  ├─ 向量检索
  └─ 知识库设计

第三阶段：高级特性（1周）
  ├─ MCP 协议
  ├─ 多 Agent 协调
  └─ 性能优化

第四阶段：项目集成（1周）
  ├─ 改进项目 Agent
  ├─ 添加新 Skill
  └─ 实现 RAG 增强
```

---

# 第一阶段：基础概念

## 第 1 章：Agent 系统设计基础

### 1.1 什么是 Agent？

**Agent（代理）** 是能够自主决策和行动的 AI 系统。

```
用户请求
    ↓
[Agent 核心循环]
    ├─ 1. 理解目标（目标分解）
    ├─ 2. 规划步骤（生成执行计划）
    ├─ 3. 调用工具（Skills）
    ├─ 4. 观察结果（反馈处理）
    └─ 5. 递归执行（直到完成）
    ↓
最终结果
```

### 1.2 项目中的 Agent 架构

LLMWiki 中有多个 Agent，通过 **Coordinator** 进行编排：

```
CoordinatorAgent（编排器）
    ├─ [Phase 1] SourceIngestAgent     （文档摄入 + AI 分析）
    ├─ [Phase 2] ResolutionAgent       （知识去重）
    ├─ [Phase 3] RelationshipAgent     （关系提取）
    └─ [Phase 4] WikiUpdateAgent       （知识库更新）

每个 Agent 处理一个特定的阶段，输出被下一个 Agent 消费
```

**特点：**
- ✅ 顺序执行（有依赖关系）
- ✅ 支持任务取消
- ✅ 支持错误恢复
- ✅ 事件审计（所有步骤记录）

### 1.3 Agent 的三个层次

#### 层次 1：简单指令执行（无规划）

```python
# 最简单的 Agent：直接执行指令，无决策
class SimpleAgent:
    def __init__(self, llm):
        self.llm = llm
    
    def execute(self, instruction):
        response = self.llm.call(instruction)
        return response

# 使用
agent = SimpleAgent(deepseek_llm)
result = agent.execute("总结这份文档的要点")
```

**特点：** 快速但缺乏灵活性

#### 层次 2：工具使用（有反馈循环）

```python
# Agent 可以调用工具，并根据结果调整
class ToolUsingAgent:
    def __init__(self, llm, tools):
        self.llm = llm
        self.tools = tools  # 字典：工具名 → 工具函数
    
    def execute(self, goal):
        # 初始规划
        plan = self.llm.call(f"制定计划完成目标: {goal}")
        
        # 执行循环
        steps = self.parse_steps(plan)
        results = []
        
        for step in steps:
            # 决定调用哪个工具
            tool_name = self.llm.call(f"选择工具完成: {step}")
            
            if tool_name in self.tools:
                result = self.tools[tool_name](step)  # 调用 Skill
                results.append(result)
            else:
                print(f"工具 {tool_name} 不存在")
        
        return results

# 定义 Skills（工具）
tools = {
    "extract_text": extract_text_from_pdf,
    "search_kb": search_knowledge_base,
    "summarize": summarize_text,
}

agent = ToolUsingAgent(deepseek_llm, tools)
result = agent.execute("分析这份 PDF，找出关键信息")
```

**特点：** 灵活且高效（项目主要用这一层次）

#### 层次 3：自主规划和反思（最智能）

```python
# Agent 能自主规划、执行、反思、调整
class AutonomousAgent:
    def __init__(self, llm, tools):
        self.llm = llm
        self.tools = tools
        self.max_iterations = 10  # 防止无限循环
    
    def execute(self, goal):
        state = {"goal": goal, "steps": [], "results": []}
        
        for iteration in range(self.max_iterations):
            # 1. 检查是否完成
            if self.is_goal_achieved(state):
                return state["results"]
            
            # 2. 决定下一步
            next_step = self.llm.call(
                f"目标: {goal}\n已完成: {state['steps']}\n下一步是什么?"
            )
            
            # 3. 选择工具
            tool_name = self.llm.call(f"用什么工具执行: {next_step}?")
            
            # 4. 执行
            if tool_name in self.tools:
                result = self.tools[tool_name](next_step)
                state["results"].append(result)
                state["steps"].append(next_step)
            
            # 5. 反思：结果是否令人满意？
            reflection = self.llm.call(
                f"对于目标 '{goal}'，结果 '{result}' 是否满意? 需要继续吗?"
            )
            
            if "满意" in reflection or "完成" in reflection:
                return state["results"]
        
        return state["results"]
```

**特点：** 最聪明但成本最高（需要多次 LLM 调用）

### 1.4 项目中的 Agent 实现

**src-tauri/src/agents/coordinator.rs** — Coordinator 的实现：

```rust
pub struct CoordinatorAgent {
    task_queue: Arc<TaskQueue>,
    db: Arc<DatabaseService>,
    config: Arc<ConfigService>,
    workspace: Arc<WorkspaceService>,
    event_bus: Arc<EventBus>,
    model_gateway: Arc<ModelGateway>,
}

impl CoordinatorAgent {
    /// 启动完整的处理流水线
    pub async fn run_source_ingest(
        &self,
        kb_id: &str,
        kb_path: &str,
        source_id: &str,
    ) -> Result<String, String> {
        // 创建任务
        let task = self.task_queue.create_task(kb_id, "source_ingest", source_id)?;
        let task_id = task.id.clone();

        // 启动异步处理
        let cancel_token = CancellationToken::new();
        
        tokio::spawn(async move {
            // Phase 1: Source Ingest（文档摄入）
            let ingest_agent = SourceIngestAgent::new(...);
            let ingest_result = ingest_agent.execute(
                &kb_id, &kb_path, &source_id, &task_id, &cancel_token
            ).await;
            
            match ingest_result {
                Ok(_) => {
                    // Phase 2: Resolution（去重）
                    let resolution_agent = ResolutionAgent::new(...);
                    let resolution_result = resolution_agent.execute(
                        &kb_id, &task_id, &cancel_token
                    ).await;
                    
                    // ... 继续执行 Phase 3, 4 ...
                }
                Err(e) => {
                    // 错误处理和恢复
                }
            }
        });

        Ok(task_id)
    }
}
```

**关键特性：**
- ✅ 多阶段处理（Phase 1-4）
- ✅ 任务取消支持
- ✅ 错误恢复
- ✅ 事件记录

---

## 第 2 章：Skill 模式详解

### 2.1 什么是 Skill？

**Skill（技能）** 是 Agent 可以调用的专用工具或模块。

```
Agent                    Skills
├─ 决定调用哪个 Skill
├─ 传递参数             ├─ PDF 提取
├─ 获取结果             ├─ DOCX 解析
└─ 继续执行             ├─ Web 搜索
                        ├─ 数据库查询
                        └─ AI 调用
```

### 2.2 Skill 的通用接口

```rust
// 所有 Skill 的统一接口
pub trait Skill {
    /// 检查是否支持该文件类型
    fn supports(&self, file_extension: &str) -> bool;
    
    /// 执行 Skill（核心逻辑）
    async fn execute(&self, input: SkillInput) -> Result<SkillOutput, String>;
    
    /// Skill 的名称
    fn name(&self) -> &str;
    
    /// Skill 的描述
    fn description(&self) -> &str;
}

pub struct SkillInput {
    pub file_path: PathBuf,
    pub parameters: HashMap<String, String>,
}

pub struct SkillOutput {
    pub content: String,
    pub metadata: HashMap<String, String>,
    pub warnings: Vec<String>,
}
```

### 2.3 项目中的 Skill 系统

**DocumentProcessor（文档处理技能）**

```rust
// src-tauri/src/skills/document_processor.rs

pub struct DocumentProcessor;

impl DocumentProcessor {
    /// 统一的文档处理入口
    pub async fn parse_document(
        file_path: &Path,
        kb_id: &str,
    ) -> Result<DocumentParseResult, String> {
        let extension = get_file_extension(file_path)?;
        
        // 根据文件类型选择 Skill
        let result = match extension.to_lowercase().as_str() {
            // 优先使用 MarkItDown（万能方案）
            "pdf" => {
                markitdown_skill::convert(file_path)
                    .await
                    .or_else(|_| pdf_skill::extract_text(file_path))  // 回退
            }
            "docx" => {
                markitdown_skill::convert(file_path)
                    .await
                    .or_else(|_| docx_skill::extract_text(file_path))
            }
            "pptx" | "xlsx" | "csv" | "json" | "xml" => {
                // 这些格式仅通过 MarkItDown
                markitdown_skill::convert(file_path).await
            }
            "md" | "txt" | "html" => {
                // 原生格式，使用轻量级 Skill
                match extension.to_lowercase().as_str() {
                    "md" => md_skill::extract_text(file_path),
                    "txt" => txt_skill::extract_text(file_path),
                    "html" => html_skill::extract_text(file_path),
                    _ => Err("不支持的格式".to_string()),
                }
            }
            _ => Err(format!("不支持的文件类型: {}", extension)),
        }?;
        
        Ok(DocumentParseResult {
            file_name: file_path.file_name().unwrap().to_string_lossy().to_string(),
            file_type: extension,
            text: result.text,
            text_length: result.text.len(),
            page_count: result.page_count,
            warnings: result.warnings,
        })
    }
}
```

**具体的 Skill 实现：PDF 提取**

```rust
// src-tauri/src/skills/pdf_skill.rs

pub struct PdfSkill;

impl PdfSkill {
    /// PDF 文本提取（使用 lopdf）
    pub fn extract_text(file_path: &Path) -> Result<TextExtractionResult, String> {
        // 打开 PDF
        let document = Document::load(file_path)
            .map_err(|e| format!("打开 PDF 失败: {}", e))?;
        
        let mut full_text = String::new();
        let mut page_count = 0;
        let mut warnings = Vec::new();
        
        // 遍历所有页面
        for (page_num, _object_id) in document.page_iter() {
            page_count += 1;
            
            // 尝试提取文本
            match document.extract_text(&[page_num]) {
                Ok(text) => {
                    full_text.push_str(&text);
                    full_text.push('\n');
                }
                Err(e) => {
                    // OCR 回退：尝试识别图片
                    warnings.push(format!("第{}页文本提取失败，尝试 OCR", page_num));
                    
                    match extract_images_from_page(&document, page_num) {
                        Ok(images) => {
                            for image_data in images {
                                if let Ok(ocr_text) = run_windows_ocr(&image_data) {
                                    full_text.push_str(&ocr_text);
                                }
                            }
                        }
                        Err(_) => {
                            warnings.push(format!("第{}页 OCR 也失败了", page_num));
                        }
                    }
                }
            }
        }
        
        Ok(TextExtractionResult {
            text: full_text,
            page_count: Some(page_count),
            warnings,
        })
    }
}
```

### 2.4 Skill 的设计原则

| 原则 | 说明 | 例子 |
|------|------|------|
| **单一职责** | 一个 Skill 做一件事 | PdfSkill 只负责 PDF 提取 |
| **可组合性** | Skill 可以互相调用 | MarkItDown 作为多个格式的回退 |
| **可测试性** | 每个 Skill 都可独立测试 | 单元测试每个 extract 函数 |
| **容错性** | 有多层回退方案 | PDF: MarkItDown → lopdf → OCR |
| **可监控性** | 记录执行过程 | warnings 向量记录所有问题 |

### 2.5 添加新的 Skill

**示例：添加 Excel 数据提取 Skill**

```rust
// src-tauri/src/skills/excel_skill.rs

use xlsx_rust::read;
use std::path::Path;

pub struct ExcelSkill;

impl ExcelSkill {
    pub fn extract_text(file_path: &Path) -> Result<String, String> {
        // 打开 Excel 文件
        let mut file = std::fs::File::open(file_path)
            .map_err(|e| format!("打开文件失败: {}", e))?;
        
        // 解析 Excel（XLSX 本质是 ZIP）
        let reader = read::Reader::new(&mut file)
            .map_err(|e| format!("解析 Excel 失败: {}", e))?;
        
        let mut output = String::new();
        
        // 遍历所有工作表
        for sheet in reader.sheets() {
            output.push_str(&format!("=== {} ===\n", sheet.name));
            
            // 遍历所有行
            for row in sheet.rows() {
                // 遍历所有单元格
                let cells: Vec<String> = row.cells()
                    .iter()
                    .map(|cell| cell.to_string())
                    .collect();
                
                output.push_str(&cells.join(" | "));
                output.push('\n');
            }
        }
        
        Ok(output)
    }
}

// 在 DocumentProcessor 中集成
pub async fn parse_document(file_path: &Path) -> Result<DocumentParseResult, String> {
    match get_file_extension(file_path)?.to_lowercase().as_str() {
        "xlsx" | "xls" => {
            ExcelSkill::extract_text(file_path)
                .or_else(|_| markitdown_skill::convert(file_path).await)  // 回退
        }
        _ => { /* 其他格式 */ }
    }
}
```

---

## 第 3 章：Agent 和 Skill 的协作

### 3.1 Agent → Skill 的调用流程

```
SourceIngestAgent（Agent）
    ↓
[1] 接收文件路径
    ↓
[2] 决定使用哪个 Skill
    │   → DocumentProcessor.parse_document()
    ↓
[3] Skill 执行
    │   → DocumentParseResult { text, warnings, ... }
    ↓
[4] Agent 处理结果
    │   → 分段处理（如果文本过长）
    │   → 调用 AI 模型分析
    │   → 保存到数据库
    ↓
[5] 返回处理结果
```

### 3.2 项目中的完整示例

**SourceIngestAgent 的完整工作流：**

```rust
// src-tauri/src/agents/source_ingest.rs

pub struct SourceIngestAgent {
    db: Arc<DatabaseService>,
    config: Arc<ConfigService>,
    model_gateway: Arc<ModelGateway>,
}

impl SourceIngestAgent {
    pub async fn execute(
        &self,
        kb_id: &str,
        kb_path: &str,
        source_id: &str,
        task_id: &str,
        cancel_token: &CancellationToken,
    ) -> Result<(), String> {
        // ============ Phase 1: 文本提取（使用 Skills）============
        self.record_event(task_id, "开始文本提取")?;
        
        // 获取 Source 信息
        let conn = self.db.connect()?;
        let (file_name, file_path) = self.get_source_path(&conn, source_id)?;
        
        // 调用 DocumentProcessor 一系列 Skills
        let parse_result = DocumentProcessor::parse_document(
            Path::new(&file_path),
            kb_id
        ).await?;
        
        // 检查取消
        if cancel_token.is_cancelled() {
            return Ok(());
        }
        
        self.record_event(task_id, &format!(
            "文本提取完成: {} 字符",
            parse_result.text_length
        ))?;
        
        // ============ Phase 2: 分段处理（长文本分割）============
        let chunks = self.chunk_text(
            &parse_result.text,
            3000,  // 每个块 3000 字符
            500    // 块之间重叠 500 字符
        );
        
        // ============ Phase 3: AI 分析（调用 Model Gateway）============
        let mut knowledge_items = Vec::new();
        
        for (i, chunk) in chunks.iter().enumerate() {
            // 每个块前检查取消
            if cancel_token.is_cancelled() {
                return Ok(());
            }
            
            self.record_event(task_id, &format!(
                "处理分段 {}/{}",
                i + 1,
                chunks.len()
            ))?;
            
            // 调用 AI 分析这个块
            let analysis = self.model_gateway.analyze_chunk(
                &chunk,
                &parse_result.file_name,
            ).await?;
            
            // 创建知识项
            let item = self.create_knowledge_item(
                kb_id,
                &file_name,
                &analysis,
                i,
            );
            
            knowledge_items.push(item);
        }
        
        // ============ Phase 4: 数据库保存============
        self.save_to_database(&conn, kb_id, source_id, &knowledge_items)?;
        
        self.record_event(task_id, "源文件处理完成")?;
        Ok(())
    }
    
    fn chunk_text(&self, text: &str, chunk_size: usize, overlap: usize) -> Vec<String> {
        let mut chunks = Vec::new();
        let mut start = 0;
        
        while start < text.len() {
            let end = std::cmp::min(start + chunk_size, text.len());
            chunks.push(text[start..end].to_string());
            
            // 滑动窗口，保留重叠
            start = end - overlap.min(end - start);
        }
        
        chunks
    }
}
```

---

# 第二阶段：实战应用

## 第 4 章：RAG 系统（检索增强生成）

### 4.1 什么是 RAG？

**RAG = 检索（Retrieval）+ 增强（Augmented）+ 生成（Generation）**

传统 LLM 的问题：
```
问题：刘汶林是谁？
LLM 回答：我不知道
原因：刘汶林的信息不在训练数据中
```

**RAG 的解决方案：**
```
问题：刘汶林是谁？
    ↓
[检索] 从知识库搜索相关文档
    ↓
[增强] 将搜索结果作为上下文
    ↓
[生成] LLM 基于上下文回答
    ↓
回答：根据知识库，刘汶林是...
```

### 4.2 RAG 的三个步骤

#### 步骤 1：检索（Retrieval）

```
用户问题：刘汶林是谁？
    ↓
搜索引擎搜索知识库
    ├─ 关键词匹配：刘汶林
    ├─ 向量相似度：问题 embedding 与知识库对比
    └─ 混合搜索：关键词 + 向量
    ↓
返回 Top-K 相关文档
    ├─ 文档 1：刘汶林简历 (相似度: 0.95)
    ├─ 文档 2：刘汶林项目经验 (相似度: 0.92)
    └─ 文档 3：其他人物介绍 (相似度: 0.45)
```

#### 步骤 2：增强（Augmentation）

```
LLM Prompt = 系统提示 + 检索文档 + 用户问题

示例 Prompt：
---
你是一个知识库助手。
根据以下文档回答用户问题。

【文档 1】
刘汶林，资深全栈工程师...

【文档 2】
项目经验：
- LLMWiki：AI 知识库管理系统
- Open WebUI：开源 LLM 前端

【用户问题】
刘汶林是谁？

【你的回答】
---
```

#### 步骤 3：生成（Generation）

```
LLM 生成回答：
"根据知识库信息，刘汶林是一位资深的全栈工程师，
拥有丰富的 AI 和大模型应用开发经验。
他参与开发了 LLMWiki（知识库管理系统）和 Open WebUI 等项目。"
```

### 4.3 项目中的 RAG 实现

**ChatPage 的网页搜索 RAG：**

```rust
// src-tauri/src/commands/web_search.rs

#[tauri::command]
pub async fn web_search(
    query: String,
    engine: String,
    max_results: u32,
) -> Result<Vec<SearchResult>, String> {
    // 步骤 1: 检索（使用 DuckDuckGo）
    let results = search_engine::search(
        &query,
        engine,
        max_results,
    ).await?;
    
    Ok(results)
}

#[tauri::command]
pub async fn fetch_web_page_content(
    url: String,
) -> Result<WebPageContent, String> {
    // 步骤 1: 检索（爬取网页）
    let client = reqwest::Client::new();
    let response = client
        .get(&url)
        .timeout(Duration::from_secs(20))
        .send()
        .await
        .map_err(|e| format!("网页请求失败: {}", e))?;
    
    let html = response.text().await
        .map_err(|e| format!("读取网页失败: {}", e))?;
    
    // 提取正文
    let document = scraper::Html::parse_document(&html);
    let content = extract_main_content(&document);
    
    Ok(WebPageContent {
        title: extract_title(&document),
        url,
        content,
        content_length: content.len(),
    })
}

// 在 ChatPage 中使用 RAG
#[tauri::command]
pub async fn chat_with_web_search(
    kernel: State<'_, Arc<AppKernel>>,
    kb_id: String,
    query: String,
) -> Result<ChatResponse, String> {
    // 步骤 1: 检索
    let search_results = web_search(query.clone(), "duckduckgo".to_string(), 5).await?;
    
    // 提取网页内容
    let mut context_docs = Vec::new();
    for result in search_results {
        if let Ok(content) = fetch_web_page_content(result.url).await {
            context_docs.push(content.content);
        }
    }
    
    // 步骤 2: 增强（组装 Prompt）
    let augmented_prompt = format!(
        "根据以下网页内容回答问题\n\n【网页内容】\n{}\n\n【问题】\n{}",
        context_docs.join("\n---\n"),
        query
    );
    
    // 步骤 3: 生成
    let response = kernel.model_gateway.chat(
        &config,
        vec![ChatMessage {
            role: "user".to_string(),
            content: augmented_prompt,
        }],
        false,
    ).await?;
    
    Ok(ChatResponse {
        content: response.content,
        model: response.model,
    })
}
```

### 4.4 RAG 的质量指标

| 指标 | 含义 | 如何改进 |
|------|------|---------|
| **Recall（召回率）** | 能找到多少相关文档 | 增加搜索范围 K，改进搜索算法 |
| **Precision（精度）** | 找到的文档有多少是相关的 | 提高相似度阈值，精细调优 |
| **MRR（平均倒数排名）** | 第一个相关文档排名有多好 | 改进排序算法，使用混合搜索 |
| **NDCG（归一化折扣累积增益）** | 排名结果的整体质量 | 学习排序，多轮迭代 |

---

## 第 5 章：向量检索（Embedding + Vector Search）

### 5.1 什么是 Embedding？

**Embedding** 将文本转换成数字向量，使计算机能够理解语义。

```
文本：刘汶林是全栈工程师
    ↓
Embedding 模型（如 text-embedding-3-small）
    ↓
向量：[0.123, 0.456, -0.789, 0.234, ...]  （维度：1536 或更高）
```

**为什么需要 Embedding？**
```
关键词匹配问题：
问题：工程师从事什么工作？
关键词：工程师
搜索结果：包含"工程师"的所有文档（太多，精度低）

向量检索优势：
问题：工程师从事什么工作？
Embedding：[语义向量1]
对比所有文档的 Embedding
    ├─ 文档1（刘汶林做开发）：[语义向量2] → 相似度 0.92 ✅
    ├─ 文档2（工程师招聘）：[语义向量3] → 相似度 0.78
    └─ 文档3（工程机械）：[语义向量4] → 相似度 0.12
结果精准度 > 关键词匹配
```

### 5.2 常见的 Embedding 模型

| 模型 | 维度 | 特点 | 成本 |
|------|------|------|------|
| **text-embedding-3-small** (OpenAI) | 1536 | 性能好 | $0.02/百万 |
| **text-embedding-3-large** (OpenAI) | 3072 | 精度最高 | $0.13/百万 |
| **jina-embeddings-v3** | 768-4096 | 开源，可变维度 | 免费 |
| **bge-large-zh-v1.5** (中文) | 1024 | 中文优化 | 免费 |
| **m3e-base** (中文) | 768 | 轻量级 | 免费 |

### 5.3 向量数据库

**为什么需要向量数据库？**

```
普通数据库：SQL 查询（关键字匹配）
  SELECT * FROM documents WHERE content LIKE '%工程师%'

向量数据库：向量相似度查询
  SELECT * FROM documents 
  WHERE embedding <-> user_query_embedding < 0.5
  ORDER BY distance LIMIT 10
```

**常见的向量数据库：**

| 数据库 | 特点 | 适用场景 |
|--------|------|---------|
| **Pinecone** | 云端，无需维护 | 快速上线 |
| **Weaviate** | 开源，支持多种数据类型 | 通用 RAG |
| **Milvus** | 高性能，支持 GPU | 大规模应用 |
| **ChromaDB** | 轻量级，本地存储 | 开发和测试 |
| **FAISS** | 库，非数据库 | 离线批处理 |
| **PostgreSQL + pgvector** | SQL + 向量 | 现有系统集成 |

### 5.4 项目中的向量检索

**项目当前状态：** 已有基础知识图谱，但向量检索未深度集成

**改进方案：**

```rust
// src-tauri/src/search/vector_search.rs（新文件）

use std::sync::Arc;
use crate::core::config_service::ConfigService;
use crate::model::model_gateway::ModelGateway;

pub struct VectorSearchService {
    model_gateway: Arc<ModelGateway>,
    db: Arc<DatabaseService>,
}

impl VectorSearchService {
    pub fn new(model_gateway: Arc<ModelGateway>, db: Arc<DatabaseService>) -> Self {
        Self { model_gateway, db }
    }
    
    /// 为知识项生成和存储 Embedding
    pub async fn embed_knowledge_item(
        &self,
        kb_id: &str,
        item_id: &str,
        text: &str,
    ) -> Result<(), String> {
        // 调用 Embedding 模型
        let embedding = self.model_gateway.generate_embedding(text).await?;
        
        // 保存到数据库
        let conn = self.db.connect()?;
        conn.execute(
            "INSERT OR REPLACE INTO knowledge_item_embeddings 
             (kb_id, item_id, embedding_vector, embedding_dim, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![
                kb_id,
                item_id,
                serde_json::to_string(&embedding)?,
                embedding.len(),
                chrono::Utc::now().to_rfc3339(),
            ],
        ).map_err(|e| format!("保存 embedding 失败: {}", e))?;
        
        Ok(())
    }
    
    /// 向量相似度搜索
    pub async fn search(
        &self,
        kb_id: &str,
        query: &str,
        top_k: usize,
    ) -> Result<Vec<SearchResult>, String> {
        // 步骤 1: 将查询转为向量
        let query_embedding = self.model_gateway.generate_embedding(query).await?;
        
        // 步骤 2: 从数据库读取所有 item 的 embedding
        let conn = self.db.connect()?;
        let mut stmt = conn.prepare(
            "SELECT item_id, embedding_vector FROM knowledge_item_embeddings 
             WHERE kb_id = ?1"
        ).map_err(|e| format!("查询 embedding 失败: {}", e))?;
        
        let mut similarities = Vec::new();
        
        let items = stmt.query_map(
            rusqlite::params![kb_id],
            |row| {
                let item_id: String = row.get(0)?;
                let embedding_json: String = row.get(1)?;
                Ok((item_id, embedding_json))
            }
        ).map_err(|e| format!("读取结果失败: {}", e))?;
        
        // 步骤 3: 计算相似度（余弦相似度）
        for item in items {
            let (item_id, embedding_json) = item
                .map_err(|e| format!("迭代失败: {}", e))?;
            
            let item_embedding: Vec<f32> = serde_json::from_str(&embedding_json)
                .map_err(|e| format!("解析 embedding 失败: {}", e))?;
            
            let similarity = cosine_similarity(&query_embedding, &item_embedding);
            
            similarities.push((item_id, similarity));
        }
        
        // 步骤 4: 排序并返回 Top-K
        similarities.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        
        let results: Vec<SearchResult> = similarities
            .into_iter()
            .take(top_k)
            .map(|(item_id, score)| SearchResult {
                item_id,
                score,
            })
            .collect();
        
        Ok(results)
    }
}

/// 计算两个向量的余弦相似度
fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let mut dot_product = 0.0;
    let mut norm_a = 0.0;
    let mut norm_b = 0.0;
    
    for (ai, bi) in a.iter().zip(b.iter()) {
        dot_product += ai * bi;
        norm_a += ai * ai;
        norm_b += bi * bi;
    }
    
    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }
    
    dot_product / (norm_a.sqrt() * norm_b.sqrt())
}
```

### 5.5 混合搜索（关键词 + 向量）

```rust
/// 混合搜索：结合关键词和向量相似度
pub async fn hybrid_search(
    &self,
    kb_id: &str,
    query: &str,
    top_k: usize,
) -> Result<Vec<SearchResult>, String> {
    // 关键词搜索
    let keyword_results = self.keyword_search(kb_id, query, top_k * 2)?;
    let keyword_scores: HashMap<String, f32> = keyword_results
        .into_iter()
        .map(|(id, score)| (id, score))
        .collect();
    
    // 向量搜索
    let vector_results = self.search(kb_id, query, top_k * 2).await?;
    let vector_scores: HashMap<String, f32> = vector_results
        .into_iter()
        .map(|(id, score)| (id, score))
        .collect();
    
    // 融合分数（加权平均）
    let mut combined_scores = HashMap::new();
    
    for (id, keyword_score) in keyword_scores {
        let vector_score = vector_scores.get(&id).cloned().unwrap_or(0.0);
        let combined = keyword_score * 0.3 + vector_score * 0.7;  // 权重调整
        combined_scores.insert(id, combined);
    }
    
    // 排序并返回
    let mut results: Vec<_> = combined_scores.into_iter().collect();
    results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    
    Ok(results
        .into_iter()
        .take(top_k)
        .map(|(id, score)| SearchResult { item_id: id, score })
        .collect())
}
```

---

## 第 6 章：知识库设计

### 6.1 知识项的结构

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeItem {
    pub id: String,
    pub kb_id: String,
    pub source_id: String,         // 来源（哪个上传的文件）
    pub title: String,             // 标题
    pub content: String,           // 核心内容
    pub category: String,          // 分类
    pub tags: Vec<String>,         // 标签
    pub embedding: Option<Vec<f32>>,  // 向量表示
    pub created_at: String,
    pub updated_at: String,
}
```

### 6.2 知识项的多维索引

```sql
-- 不同类型的索引加速查询

-- 1. 全文搜索索引（SQLite FTS5）
CREATE VIRTUAL TABLE knowledge_fts USING fts5(
    kb_id,
    title,
    content,
    tags,
    content=knowledge_items,
    content_rowid=id
);

-- 2. 向量索引（使用 pgvector 或自己的 embedding 存储）
CREATE TABLE knowledge_embeddings (
    id TEXT PRIMARY KEY,
    kb_id TEXT NOT NULL,
    embedding BLOB NOT NULL,  -- 向量序列化为 BLOB
    FOREIGN KEY (kb_id) REFERENCES knowledge_bases(id)
);

-- 3. 分类和标签索引
CREATE INDEX idx_category ON knowledge_items(kb_id, category);
CREATE INDEX idx_tags ON knowledge_item_tags(kb_id, tag);

-- 4. 时间索引
CREATE INDEX idx_created_at ON knowledge_items(kb_id, created_at DESC);
```

### 6.3 知识库版本控制

```rust
// 支持知识库多版本，便于回滚和对比

#[derive(Debug, Clone)]
pub struct KnowledgeBaseVersion {
    pub version_id: String,
    pub kb_id: String,
    pub snapshot_data: serde_json::Value,  // 当前版本的知识库快照
    pub created_at: String,
    pub description: String,               // 版本描述（如"修复 bug"）
}

pub struct VersionService {
    db: Arc<DatabaseService>,
}

impl VersionService {
    /// 创建新版本
    pub fn create_version(
        &self,
        kb_id: &str,
        description: &str,
    ) -> Result<String, String> {
        // 读取当前知识库状态
        let snapshot = self.get_kb_snapshot(kb_id)?;
        
        // 保存版本记录
        let version_id = uuid::Uuid::new_v4().to_string();
        let conn = self.db.connect()?;
        
        conn.execute(
            "INSERT INTO kb_versions (id, kb_id, snapshot, created_at, description)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![
                version_id,
                kb_id,
                serde_json::to_string(&snapshot)?,
                chrono::Utc::now().to_rfc3339(),
                description,
            ],
        ).map_err(|e| format!("创建版本失败: {}", e))?;
        
        Ok(version_id)
    }
    
    /// 回滚到某个版本
    pub fn rollback(&self, kb_id: &str, version_id: &str) -> Result<(), String> {
        // 读取历史版本
        let conn = self.db.connect()?;
        let snapshot: serde_json::Value = conn.query_row(
            "SELECT snapshot FROM kb_versions WHERE id = ?1 AND kb_id = ?2",
            rusqlite::params![version_id, kb_id],
            |row| {
                let json_str: String = row.get(0)?;
                Ok(serde_json::from_str(&json_str).unwrap())
            }
        ).map_err(|e| format!("读取版本失败: {}", e))?;
        
        // 恢复数据
        self.restore_kb_from_snapshot(kb_id, &snapshot)?;
        
        Ok(())
    }
}
```

---

# 第三阶段：高级特性

## 第 7 章：MCP 协议（Model Context Protocol）

### 7.1 什么是 MCP？

**MCP（Model Context Protocol）** 是 Anthropic 设计的标准化协议，用于连接 LLM 和外部工具/数据源。

```
传统方式：
LLM 应用 ← → 工具 1（Web API）
         ← → 工具 2（数据库）
         ← → 工具 3（文件系统）
问题：每个工具需要单独实现，没有标准接口

MCP 方式：
LLM 应用 ← MCP 协议 → [MCP 服务器]
                       ├─ Tools（工具集）
                       ├─ Resources（资源）
                       ├─ Prompts（提示模板）
                       └─ Sampling（模型采样）
```

### 7.2 MCP 的核心概念

#### 工具（Tools）

```json
{
  "type": "tool",
  "name": "search_kb",
  "description": "在知识库中搜索信息",
  "inputSchema": {
    "type": "object",
    "properties": {
      "query": {
        "type": "string",
        "description": "搜索关键词"
      },
      "top_k": {
        "type": "integer",
        "description": "返回结果数量"
      }
    },
    "required": ["query"]
  }
}
```

#### 资源（Resources）

```json
{
  "type": "resource",
  "uri": "kb://llmwiki/knowledge_base_1",
  "name": "LLMWiki 知识库",
  "description": "包含 AI 和系统设计相关内容",
  "mimeType": "application/json"
}
```

#### 提示（Prompts）

```json
{
  "type": "prompt",
  "name": "code_review",
  "description": "代码审查提示模板",
  "arguments": [
    {
      "name": "code",
      "description": "要审查的代码"
    },
    {
      "name": "language",
      "description": "编程语言"
    }
  ]
}
```

### 7.3 MCP 服务器实现

```rust
// src-tauri/src/mcp/server.rs

use std::sync::Arc;

pub struct MCPServer {
    db: Arc<DatabaseService>,
    search: Arc<SearchService>,
}

impl MCPServer {
    pub fn new(db: Arc<DatabaseService>, search: Arc<SearchService>) -> Self {
        Self { db, search }
    }
    
    /// 列出所有可用工具
    pub fn list_tools(&self) -> Vec<ToolDefinition> {
        vec![
            ToolDefinition {
                name: "search_kb".to_string(),
                description: "在知识库中搜索相关信息".to_string(),
                inputSchema: json!({
                    "type": "object",
                    "properties": {
                        "kb_id": { "type": "string", "description": "知识库 ID" },
                        "query": { "type": "string", "description": "搜索查询" },
                        "top_k": { "type": "integer", "description": "返回结果数" }
                    },
                    "required": ["kb_id", "query"]
                }),
            },
            ToolDefinition {
                name: "get_knowledge_item".to_string(),
                description: "获取特定知识项的详细信息".to_string(),
                inputSchema: json!({
                    "type": "object",
                    "properties": {
                        "item_id": { "type": "string", "description": "知识项 ID" }
                    },
                    "required": ["item_id"]
                }),
            },
            ToolDefinition {
                name: "list_kb_categories".to_string(),
                description: "列出知识库的所有分类".to_string(),
                inputSchema: json!({
                    "type": "object",
                    "properties": {
                        "kb_id": { "type": "string", "description": "知识库 ID" }
                    },
                    "required": ["kb_id"]
                }),
            },
        ]
    }
    
    /// 执行工具
    pub async fn call_tool(
        &self,
        tool_name: &str,
        input: serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        match tool_name {
            "search_kb" => {
                let kb_id = input["kb_id"].as_str()
                    .ok_or("kb_id required")?;
                let query = input["query"].as_str()
                    .ok_or("query required")?;
                let top_k = input["top_k"].as_u64().unwrap_or(10) as usize;
                
                let results = self.search.hybrid_search(kb_id, query, top_k).await?;
                Ok(serde_json::to_value(results)?)
            }
            
            "get_knowledge_item" => {
                let item_id = input["item_id"].as_str()
                    .ok_or("item_id required")?;
                
                let item = self.get_knowledge_item(item_id)?;
                Ok(serde_json::to_value(item)?)
            }
            
            "list_kb_categories" => {
                let kb_id = input["kb_id"].as_str()
                    .ok_or("kb_id required")?;
                
                let categories = self.get_kb_categories(kb_id)?;
                Ok(serde_json::to_value(categories)?)
            }
            
            _ => Err(format!("未知工具: {}", tool_name)),
        }
    }
    
    /// 列出所有可用资源
    pub fn list_resources(&self) -> Result<Vec<Resource>, String> {
        let conn = self.db.connect()?;
        let mut stmt = conn.prepare(
            "SELECT id, name, path FROM knowledge_bases"
        ).map_err(|e| format!("查询失败: {}", e))?;
        
        let resources = stmt.query_map([], |row| {
            Ok(Resource {
                uri: format!("kb://{}", row.get::<_, String>(0)?),
                name: row.get::<_, String>(1)?,
                mimeType: "application/json".to_string(),
            })
        }).map_err(|e| format!("读取失败: {}", e))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("收集失败: {}", e))?;
        
        Ok(resources)
    }
}
```

### 7.4 集成 MCP 到项目

```rust
// src-tauri/src/lib.rs

// 启用 MCP 服务器
#[cfg(feature = "mcp")]
pub mod mcp {
    pub mod server;
    pub mod protocol;
}

// 在 Tauri setup 中初始化 MCP
#[cfg(feature = "mcp")]
{
    let mcp_server = mcp::server::MCPServer::new(
        kernel.db.clone(),
        // search_service...
    );
    app.manage(mcp_server);
}
```

---

## 第 8 章：多 Agent 协调

### 8.1 Agent 通信模式

```
Agent 1 → EventBus → Agent 2
(生成事件)    ↓    (监听事件)
           Agent 3
           (订阅)
```

### 8.2 项目中的事件驱动

```rust
// src-tauri/src/core/event_bus.rs

pub struct EventBus {
    app: AppHandle,
}

impl EventBus {
    pub fn emit(&self, event: &str, payload: &serde_json::Value) {
        self.app.emit(event, payload).ok();
    }
    
    pub fn emit_notification(&self, level: &str, title: &str, message: &str) {
        self.emit("notification", &json!({
            "level": level,
            "title": title,
            "message": message,
        }));
    }
}

// Agent 之间通过事件通信
// Agent 1 完成后发出事件
event_bus.emit("source_ingest_completed", &json!({
    "source_id": source_id,
    "kb_id": kb_id,
}));

// Agent 2 监听事件并继续处理
// （在 Resolution Agent 中）
listen("source_ingest_completed", |event| {
    // 开始处理下一阶段
});
```

### 8.3 多 Agent 的同步和等待

```rust
// 使用 tokio::join! 等待多个 Agent
let (result1, result2, result3) = tokio::join!(
    agent1.execute(),
    agent2.execute(),
    agent3.execute(),
);

// 或者使用通道（Channel）进行通信
use tokio::sync::mpsc;

let (tx, mut rx) = mpsc::channel(100);

// Agent 1 发送结果
tokio::spawn(async move {
    let result = agent1.execute().await;
    tx.send(result).await.ok();
});

// Agent 2 接收并处理
tokio::spawn(async move {
    while let Some(result) = rx.recv().await {
        agent2.process(result).await;
    }
});
```

---

## 第 9 章：性能优化

### 9.1 缓存策略

```rust
use std::collections::HashMap;
use parking_lot::RwLock;

pub struct CacheService {
    cache: RwLock<HashMap<String, CacheEntry>>,
}

#[derive(Clone)]
struct CacheEntry {
    data: serde_json::Value,
    created_at: Instant,
    ttl: Duration,
}

impl CacheService {
    pub fn get(&self, key: &str) -> Option<serde_json::Value> {
        let cache = self.cache.read();
        
        cache.get(key).and_then(|entry| {
            if entry.created_at.elapsed() < entry.ttl {
                Some(entry.data.clone())
            } else {
                None  // 已过期
            }
        })
    }
    
    pub fn set(&self, key: String, value: serde_json::Value, ttl: Duration) {
        let mut cache = self.cache.write();
        cache.insert(key, CacheEntry {
            data: value,
            created_at: Instant::now(),
            ttl,
        });
    }
    
    /// 定期清理过期缓存
    pub fn cleanup(&self) {
        let mut cache = self.cache.write();
        cache.retain(|_, entry| {
            entry.created_at.elapsed() < entry.ttl
        });
    }
}
```

### 9.2 批量处理优化

```rust
// 不一个个处理，而是批量处理
pub async fn batch_process_items(
    items: Vec<String>,
    batch_size: usize,
) -> Result<Vec<ProcessResult>, String> {
    let mut results = Vec::new();
    
    for batch in items.chunks(batch_size) {
        // 一次性处理一批
        let batch_results = process_batch(batch).await?;
        results.extend(batch_results);
    }
    
    Ok(results)
}

// 并行处理
pub async fn parallel_process_items(
    items: Vec<String>,
    concurrency: usize,
) -> Result<Vec<ProcessResult>, String> {
    use futures::stream::{self, StreamExt};
    
    let results = stream::iter(items)
        .map(|item| process_item(item))
        .buffer_unordered(concurrency)  // 最多并行 concurrency 个
        .collect::<Vec<_>>()
        .await;
    
    Ok(results.into_iter().collect::<Result<Vec<_>, _>>()?)
}
```

### 9.3 索引优化

```sql
-- 分析查询性能
EXPLAIN QUERY PLAN
SELECT * FROM knowledge_items
WHERE kb_id = ? AND category = ?
ORDER BY created_at DESC
LIMIT 10;

-- 添加覆盖索引（包含所需的所有列）
CREATE INDEX idx_kb_category_time ON knowledge_items(
    kb_id,
    category,
    created_at DESC,
    id,      -- 覆盖索引，避免回表
    title,
    content
);
```

---

# 第四阶段：项目集成

## 第 10 章：改进项目 Agent 系统

### 10.1 增强 SourceIngestAgent

**目标：** 添加向量搜索和 RAG 能力

```rust
// src-tauri/src/agents/source_ingest.rs（改进版）

pub struct SourceIngestAgent {
    db: Arc<DatabaseService>,
    config: Arc<ConfigService>,
    model_gateway: Arc<ModelGateway>,
    vector_search: Arc<VectorSearchService>,  // 新增
}

impl SourceIngestAgent {
    pub async fn execute_with_rag(
        &self,
        kb_id: &str,
        kb_path: &str,
        source_id: &str,
        task_id: &str,
        cancel_token: &CancellationToken,
    ) -> Result<(), String> {
        // ... 文本提取 ...
        
        // 新增：基于知识库进行 RAG 增强分析
        let analysis_with_context = self.analyze_with_rag(
            &parse_result.text,
            kb_id,
            &parse_result.file_name,
        ).await?;
        
        // ... 保存结果 ...
    }
    
    async fn analyze_with_rag(
        &self,
        text: &str,
        kb_id: &str,
        file_name: &str,
    ) -> Result<String, String> {
        // 步骤 1: 生成查询关键词
        let keywords = self.extract_keywords(text)?;
        
        // 步骤 2: 从知识库搜索相关内容
        let context_docs = self.vector_search
            .search(kb_id, &keywords.join(" "), 5)
            .await?;
        
        // 步骤 3: 组装 RAG Prompt
        let rag_prompt = format!(
            "文档: {}\n\n相关知识库内容:\n{}\n\n请分析这个文档",
            text,
            context_docs.iter()
                .map(|d| format!("- {}", d))
                .collect::<Vec<_>>()
                .join("\n")
        );
        
        // 步骤 4: 调用 AI 分析
        let analysis = self.model_gateway.chat_with_content(
            &self.config.get_deepseek_config()?,
            "你是一个知识库分析助手",
            &rag_prompt,
            false,
        ).await?;
        
        Ok(analysis.content)
    }
}
```

### 10.2 新增 SearchAgent（搜索 Agent）

```rust
// src-tauri/src/agents/search_agent.rs（新文件）

pub struct SearchAgent {
    vector_search: Arc<VectorSearchService>,
    keyword_search: Arc<KeywordSearchService>,
    model_gateway: Arc<ModelGateway>,
}

impl SearchAgent {
    /// 执行混合搜索
    pub async fn search(
        &self,
        kb_id: &str,
        query: &str,
        search_type: SearchType,
    ) -> Result<SearchResults, String> {
        match search_type {
            SearchType::Keyword => {
                self.keyword_search.search(kb_id, query, 10).await
            }
            SearchType::Vector => {
                self.vector_search.search(kb_id, query, 10).await
            }
            SearchType::Hybrid => {
                self.vector_search.hybrid_search(kb_id, query, 10).await
            }
        }
    }
    
    /// 智能搜索（根据查询内容自动选择搜索策略）
    pub async fn smart_search(
        &self,
        kb_id: &str,
        query: &str,
    ) -> Result<SearchResults, String> {
        // 分析查询类型
        let query_type = self.analyze_query_type(query);
        
        let search_type = match query_type {
            QueryType::Entity => SearchType::Keyword,      // 实体查询用关键词
            QueryType::Semantic => SearchType::Vector,     // 语义查询用向量
            _ => SearchType::Hybrid,                       // 默认混合
        };
        
        self.search(kb_id, query, search_type).await
    }
}

#[derive(Debug)]
enum SearchType {
    Keyword,
    Vector,
    Hybrid,
}

enum QueryType {
    Entity,
    Semantic,
    Question,
}
```

### 10.3 新增 ChatAgent（对话 Agent）

```rust
// src-tauri/src/agents/chat_agent.rs（新文件）

pub struct ChatAgent {
    search_agent: Arc<SearchAgent>,
    model_gateway: Arc<ModelGateway>,
    web_search: Arc<WebSearchService>,
}

impl ChatAgent {
    /// 处理用户对话
    pub async fn chat(
        &self,
        kb_id: &str,
        message: &str,
        enable_web_search: bool,
    ) -> Result<ChatResponse, String> {
        // 步骤 1: 决定是否需要搜索
        let needs_search = self.should_search(message);
        
        let mut context = String::new();
        
        if needs_search {
            // 步骤 2a: 知识库搜索
            let kb_results = self.search_agent
                .smart_search(kb_id, message)
                .await?;
            
            context.push_str("【知识库搜索结果】\n");
            for result in kb_results.items {
                context.push_str(&result.content);
                context.push('\n');
            }
            
            // 步骤 2b: 如果启用，网页搜索
            if enable_web_search {
                let web_results = self.web_search.search(message, 3).await?;
                context.push_str("\n【网页搜索结果】\n");
                for result in web_results {
                    context.push_str(&format!("- {} ({})\n", result.title, result.url));
                }
            }
        }
        
        // 步骤 3: 组装 RAG Prompt
        let prompt = self.build_prompt(message, &context);
        
        // 步骤 4: 调用 LLM
        let response = self.model_gateway.chat(
            &self.config.get_deepseek_config()?,
            vec![ChatMessage {
                role: "user".to_string(),
                content: prompt,
            }],
            false,
        ).await?;
        
        Ok(ChatResponse {
            content: response.content,
            model: response.model,
            sources: self.extract_sources(&context),
        })
    }
    
    fn build_prompt(&self, message: &str, context: &str) -> String {
        format!(
            "你是一个有帮助的助手。\n\n{}\n\n用户问题: {}",
            context,
            message
        )
    }
}
```

---

## 第 11 章：添加新的 Skill

### 11.1 图表识别 Skill

```rust
// src-tauri/src/skills/chart_skill.rs

pub struct ChartSkill;

impl ChartSkill {
    /// 识别和描述图表内容
    pub async fn analyze_chart(image_path: &Path) -> Result<ChartAnalysis, String> {
        // 使用 Vision API（如 Claude Vision）分析图表
        let image_data = std::fs::read(image_path)
            .map_err(|e| format!("读取图表失败: {}", e))?;
        
        let base64_image = base64::encode(&image_data);
        
        // 调用 Vision 模型
        let analysis = call_vision_api(
            &base64_image,
            "请分析这个图表，描述其中的数据、趋势和关键信息",
        ).await?;
        
        Ok(ChartAnalysis {
            chart_type: analysis.chart_type,
            title: analysis.title,
            description: analysis.description,
            key_insights: analysis.key_insights,
            data_summary: analysis.data_summary,
        })
    }
}

#[derive(Debug, Serialize)]
pub struct ChartAnalysis {
    pub chart_type: String,      // 柱状图、折线图等
    pub title: String,
    pub description: String,     // 图表内容描述
    pub key_insights: Vec<String>,  // 关键洞察
    pub data_summary: String,    // 数据摘要
}
```

### 11.2 表格提取 Skill

```rust
// src-tauri/src/skills/table_skill.rs

pub struct TableSkill;

impl TableSkill {
    /// 从图片或 PDF 中提取表格
    pub async fn extract_tables(image_path: &Path) -> Result<Vec<Table>, String> {
        // 使用表格识别模型（如 PaddleOCR Table）
        let tables = table_recognition_model::recognize(image_path).await?;
        
        Ok(tables)
    }
}

#[derive(Debug, Serialize)]
pub struct Table {
    pub headers: Vec<String>,
    pub rows: Vec<Vec<String>>,
    pub markdown: String,  // 转换为 Markdown 格式
}

impl Table {
    pub fn to_markdown(&self) -> String {
        // 转换为 Markdown 表格格式
        let mut md = String::new();
        
        // 表头
        md.push('|');
        for header in &self.headers {
            md.push_str(&format!(" {} |", header));
        }
        md.push('\n');
        
        // 分隔符
        md.push('|');
        for _ in &self.headers {
            md.push_str(" --- |");
        }
        md.push('\n');
        
        // 数据行
        for row in &self.rows {
            md.push('|');
            for cell in row {
                md.push_str(&format!(" {} |", cell));
            }
            md.push('\n');
        }
        
        md
    }
}
```

---

## 第 12 章：实现完整的 RAG 系统

### 12.1 端到端的 RAG 流程

```
用户问题
  ↓
[ChatAgent]
  ├─ 分析问题类型
  ├─ 决定搜索策略
  ↓
[SearchAgent]
  ├─ 关键词搜索
  ├─ 向量搜索
  ├─ 混合排序
  ↓
[DocumentRetriever]
  ├─ 获取完整文档
  ├─ 片段化处理
  ├─ 相关性排序
  ↓
[PromptBuilder]
  ├─ 格式化搜索结果
  ├─ 添加系统提示
  ├─ 组装最终 Prompt
  ↓
[ModelGateway]
  ├─ 调用 LLM
  ├─ 获取回答
  ├─ 记录调用信息
  ↓
[ResponseProcessor]
  ├─ 标记信息来源
  ├─ 提取引用
  ├─ 格式化输出
  ↓
最终回答 + 信息来源
```

### 12.2 完整代码示例

```rust
// src-tauri/src/services/rag_service.rs（新文件）

pub struct RAGService {
    chat_agent: Arc<ChatAgent>,
    search_agent: Arc<SearchAgent>,
    model_gateway: Arc<ModelGateway>,
    document_service: Arc<DocumentService>,
}

impl RAGService {
    /// 完整的 RAG 流程
    pub async fn answer_question(
        &self,
        kb_id: &str,
        question: &str,
        options: RAGOptions,
    ) -> Result<RAGResponse, String> {
        // 步骤 1: 分析问题
        let analysis = self.analyze_question(question)?;
        
        // 步骤 2: 检索相关文档
        let retrieved_docs = self.search_agent
            .smart_search(kb_id, &analysis.search_query)
            .await?;
        
        // 步骤 3: 选择最相关的 K 个文档
        let top_k_docs = self.select_top_documents(
            &retrieved_docs,
            options.top_k
        );
        
        // 步骤 4: 构建增强上下文
        let context = self.build_context(&top_k_docs, &analysis);
        
        // 步骤 5: 生成最终 Prompt
        let prompt = self.generate_prompt(question, &context, &analysis);
        
        // 步骤 6: 调用 LLM
        let response = self.model_gateway.chat(
            &self.config.get_deepseek_config()?,
            vec![ChatMessage {
                role: "user".to_string(),
                content: prompt,
            }],
            false,
        ).await?;
        
        // 步骤 7: 后处理（提取引用）
        let citations = self.extract_citations(&response.content, &top_k_docs);
        
        Ok(RAGResponse {
            answer: response.content,
            citations,
            retrieval_score: top_k_docs.iter().map(|d| d.score).sum::<f32>() / top_k_docs.len() as f32,
            model: response.model,
        })
    }
    
    fn build_context(&self, docs: &[Document], analysis: &QuestionAnalysis) -> String {
        let mut context = String::from("【相关文档】\n");
        
        for (i, doc) in docs.iter().enumerate() {
            context.push_str(&format!(
                "[文档 {}] {}\n相关度: {:.2}%\n内容: {}\n\n",
                i + 1,
                doc.title,
                doc.score * 100.0,
                &doc.content[..std::cmp::min(300, doc.content.len())]
            ));
        }
        
        context
    }
    
    fn generate_prompt(
        &self,
        question: &str,
        context: &str,
        _analysis: &QuestionAnalysis,
    ) -> String {
        format!(
            r#"你是一个有帮助的知识库助手。

{context}

【用户问题】
{question}

【要求】
1. 基于提供的文档回答问题
2. 如果文档中没有相关信息，请说明
3. 对于关键信息，请标注来源（如"[文档 1]"）
4. 保持回答的准确性和相关性

【你的回答】"#
        )
    }
    
    fn extract_citations(
        &self,
        answer: &str,
        docs: &[Document],
    ) -> Vec<Citation> {
        let mut citations = Vec::new();
        
        // 正则表达式匹配 [文档 N]
        let re = regex::Regex::new(r"\[文档\s+(\d+)\]").unwrap();
        
        for cap in re.captures_iter(answer) {
            if let Ok(doc_idx) = cap[1].parse::<usize>() {
                if let Some(doc) = docs.get(doc_idx - 1) {
                    citations.push(Citation {
                        text: cap[0].to_string(),
                        source_id: doc.id.clone(),
                        source_title: doc.title.clone(),
                        score: doc.score,
                    });
                }
            }
        }
        
        citations
    }
}

pub struct RAGOptions {
    pub top_k: usize,
    pub score_threshold: f32,
    pub enable_web_search: bool,
}

pub struct RAGResponse {
    pub answer: String,
    pub citations: Vec<Citation>,
    pub retrieval_score: f32,
    pub model: String,
}

pub struct Citation {
    pub text: String,
    pub source_id: String,
    pub source_title: String,
    pub score: f32,
}
```

---

# 总结和实践路线图

## 学习总结

| 阶段 | 概念 | 项目应用 |
|------|------|---------|
| **第一阶段** | Agent、Skill、系统设计 | CoordinatorAgent + 4 个处理 Agent |
| **第二阶段** | RAG、向量检索 | ChatPage 搜索 + 知识库查询 |
| **第三阶段** | MCP、多 Agent 协调 | 扩展工具集合 |
| **第四阶段** | 项目集成 | SearchAgent + ChatAgent + RAGService |

## 实践路线图（8 周计划）

### 周 1-2：学习 Agent 和 Skill 基础
- [ ] 阅读第 1-2 章
- [ ] 阅读项目代码：`src-tauri/src/agents/`
- [ ] 修改 SourceIngestAgent：添加日志和监控

### 周 3-4：实现 RAG 和向量检索
- [ ] 阅读第 4-6 章
- [ ] 添加 VectorSearchService
- [ ] 为知识项添加 Embedding 存储

### 周 5-6：添加新 Agent 和 Skill
- [ ] 实现 SearchAgent
- [ ] 实现 ChatAgent
- [ ] 添加新的 Skill（表格、图表等）

### 周 7：MCP 和性能优化
- [ ] 学习 MCP 协议（第 7 章）
- [ ] 添加缓存和批处理优化

### 周 8：整合测试
- [ ] 端到端测试 RAG 流程
- [ ] 性能测试和优化
- [ ] 文档编写

---

## 推荐资源

### 官方文档
- **Anthropic Claude API**: https://docs.anthropic.com
- **OpenAI Embeddings**: https://platform.openai.com/docs/guides/embeddings
- **Tauri 文档**: https://tauri.app/docs

### 开源项目
- **LangChain**: Python Agent 框架（参考架构）
- **LlamaIndex**: RAG 系统框架
- **FastAPI**: Python 后端框架示例

### 论文和资源
- **RAG 论文**: "Retrieval-Augmented Generation for Knowledge-Intensive NLP Tasks"
- **向量搜索**: "A Billion-scale Commodity Clustering for the Web"
- **Agent 系统**: "ReAct: Synergizing Reasoning and Acting in Language Models"

---

**恭喜！你现在拥有了 Agent、Skill、RAG、向量检索完整的学习体系和项目实战指导。** 🎉

开始从第一章学习，逐步深入，最后在项目中实现这些能力！
