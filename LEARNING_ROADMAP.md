# Agent、RAG、向量检索 学习规划和资源清单

> 完整的 8 周学习计划 + 推荐资源 + 实战检查清单

---

## 📊 完整学习规划（8 周）

### 第 1-2 周：基础概念和架构

**目标：** 理解 Agent、Skill 的核心概念，能读懂项目代码

#### 周 1 - 第 1-3 天

**学习内容：**
- [ ] 阅读本指南第 1-2 章（Agent 和 Skill）
- [ ] 运行项目，理解 CoordinatorAgent 的 4 个阶段
- [ ] 阅读 `src-tauri/src/agents/coordinator.rs`

**实践任务：**
```rust
// 任务：给 SourceIngestAgent 添加详细的日志记录
// 修改 src-tauri/src/agents/source_ingest.rs
// 在每个关键步骤添加 log::info!("...")

// 验证：运行项目，查看控制台输出是否有详细日志
```

**预期学习成果：**
- [ ] 能解释 Agent 和 Skill 的区别
- [ ] 能画出项目的 Agent 流水线图
- [ ] 理解任务队列和事件总线的作用

---

#### 周 1 - 第 4-5 天

**学习内容：**
- [ ] 学习项目中的 Skill 系统（DocumentProcessor）
- [ ] 阅读 `src-tauri/src/skills/` 目录下的所有文件
- [ ] 理解 Skill 的多层回退机制（MarkItDown → lopdf → OCR）

**实践任务：**
```rust
// 任务：新增一个简单的 Skill
// 创建 src-tauri/src/skills/summary_skill.rs
// 实现一个"总结 Skill"，用来快速提取文本摘要

pub struct SummarySkill;

impl SummarySkill {
    pub fn extract_summary(text: &str, max_length: usize) -> String {
        // 简单的摘要提取（取前 max_length 个字符）
        text.chars().take(max_length).collect()
    }
}

// 在 DocumentProcessor 中集成这个 Skill
```

**预期学习成果：**
- [ ] 理解 Skill 的设计模式
- [ ] 能添加新的 Skill
- [ ] 了解容错和回退机制

---

#### 周 2 - 第 1-3 天

**学习内容：**
- [ ] 学习 Rust 中的异步编程（tokio）
- [ ] 理解 `async/await` 和 `tokio::spawn`
- [ ] 阅读项目中的异步任务处理代码

**实践任务：**
```rust
// 任务：理解项目中的异步执行流
// 分析 src-tauri/src/agents/coordinator.rs 的异步处理
// 绘制异步流程图，标明各个阶段的并发关系

// 问题：
// 1. SourceIngestAgent 和 ResolutionAgent 是串行还是并行？
// 2. 为什么需要 tokio::spawn？
// 3. 任务取消是如何实现的？
```

**预期学习成果：**
- [ ] 理解 async/await 的基本用法
- [ ] 能解释 tokio 的作用
- [ ] 理解任务队列的实现原理

---

#### 周 2 - 第 4-5 天

**学习内容：**
- [ ] 总结第 1-2 周的内容
- [ ] 做一个小项目：实现一个简单的 Agent

**实践任务：**
```rust
// 任务：实现一个简单的 DocumentAnalysisAgent
// 这个 Agent 的作用：
// 1. 读取一个文档
// 2. 调用 DocumentProcessor (Skill) 提取文本
// 3. 调用 SummarySkill 生成摘要
// 4. 保存结果

pub struct DocumentAnalysisAgent {
    db: Arc<DatabaseService>,
}

impl DocumentAnalysisAgent {
    pub async fn analyze(&self, file_path: &str) -> Result<String, String> {
        // 实现这个方法
        todo!()
    }
}

// 验证：编写测试，确保 Agent 能正常工作
#[tokio::test]
async fn test_document_analysis_agent() {
    // 测试代码
}
```

**预期学习成果：**
- [ ] 能从零设计一个 Agent
- [ ] 理解 Agent 和 Skill 的完整协作流程

---

### 第 3-4 周：RAG 系统基础

**目标：** 理解 RAG 的原理，能实现基础的检索增强

#### 周 3 - 第 1-3 天

**学习内容：**
- [ ] 阅读本指南第 4 章（RAG 系统）
- [ ] 理解检索、增强、生成三个步骤
- [ ] 学习如何组装 Prompt

**实践任务：**
```python
# 任务：用 Python 实现一个简单的 RAG 系统
# 这是为了理解概念，不需要集成到项目

from openai import OpenAI

def simple_rag_query(question: str, documents: list[str]) -> str:
    """
    简单 RAG 实现
    1. 把文档作为上下文
    2. 让 LLM 基于上下文回答问题
    """
    
    # 步骤 1: 组装 Prompt
    context = "\n".join([f"[文档 {i}] {doc}" for i, doc in enumerate(documents)])
    prompt = f"""根据以下文档回答问题：

{context}

问题：{question}

回答："""
    
    # 步骤 2: 调用 LLM
    client = OpenAI(api_key="your-key")
    response = client.chat.completions.create(
        model="gpt-3.5-turbo",
        messages=[{"role": "user", "content": prompt}]
    )
    
    return response.choices[0].message.content

# 测试
docs = [
    "刘汶林是资深全栈工程师，拥有 10 年开发经验",
    "他的主要项目包括 LLMWiki 和 Open WebUI",
]
answer = simple_rag_query("刘汶林是谁？", docs)
print(answer)
```

**预期学习成果：**
- [ ] 理解 RAG 的三个步骤
- [ ] 能手工设计一个 RAG Prompt
- [ ] 理解检索文档的重要性

---

#### 周 3 - 第 4-5 天

**学习内容：**
- [ ] 学习向量和 Embedding 的概念
- [ ] 理解余弦相似度的计算
- [ ] 学习常见的 Embedding 模型

**实践任务：**
```python
# 任务：实现简单的向量相似度计算

import math

def cosine_similarity(a: list[float], b: list[float]) -> float:
    """计算两个向量的余弦相似度"""
    
    # 验证向量长度相同
    assert len(a) == len(b), "向量长度必须相同"
    
    # 计算点积
    dot_product = sum(x * y for x, y in zip(a, b))
    
    # 计算范数
    norm_a = math.sqrt(sum(x * x for x in a))
    norm_b = math.sqrt(sum(y * y for y in b))
    
    # 避免除以零
    if norm_a == 0 or norm_b == 0:
        return 0.0
    
    # 计算相似度
    return dot_product / (norm_a * norm_b)

# 测试
vector_1 = [1.0, 2.0, 3.0]
vector_2 = [1.0, 2.0, 3.0]
vector_3 = [-1.0, -2.0, -3.0]

print(f"相同向量: {cosine_similarity(vector_1, vector_2)}")  # 应该是 1.0
print(f"相反向量: {cosine_similarity(vector_1, vector_3)}")  # 应该是 -1.0
```

**预期学习成果：**
- [ ] 理解向量和 Embedding 的概念
- [ ] 能计算余弦相似度
- [ ] 理解 Embedding 模型的作用

---

#### 周 4 - 第 1-3 天

**学习内容：**
- [ ] 阅读本指南第 5 章（向量检索）
- [ ] 理解向量数据库的作用
- [ ] 学习关键词搜索 vs 向量搜索 vs 混合搜索

**实践任务：**
```rust
// 任务：在项目中实现基础的向量搜索
// 创建 src-tauri/src/search/vector_search.rs

pub struct SimpleVectorSearch {
    db: Arc<DatabaseService>,
}

impl SimpleVectorSearch {
    pub fn search(
        &self,
        kb_id: &str,
        query_vector: &[f32],
        top_k: usize,
    ) -> Result<Vec<SearchResult>, String> {
        // 实现这个方法
        // 1. 从数据库读取所有知识项的向量
        // 2. 计算查询向量与每个知识项的相似度
        // 3. 排序并返回 Top-K
        todo!()
    }
}

struct SearchResult {
    item_id: String,
    title: String,
    score: f32,
}

// 验证：写一个集成测试
#[test]
fn test_vector_search() {
    // 测试代码
}
```

**预期学习成果：**
- [ ] 能在项目中实现向量搜索
- [ ] 理解混合搜索的原理
- [ ] 知道如何调整搜索权重

---

#### 周 4 - 第 4-5 天

**学习内容：**
- [ ] 学习 Embedding 模型的调用（OpenAI API）
- [ ] 理解缓存和批量处理

**实践任务：**
```rust
// 任务：实现 Embedding 缓存层

pub struct EmbeddingCache {
    memory_cache: Arc<parking_lot::RwLock<HashMap<String, Vec<f32>>>>,
    db: Arc<DatabaseService>,
}

impl EmbeddingCache {
    pub async fn get_or_generate(
        &self,
        text: &str,
        api: &OpenAIApi,
    ) -> Result<Vec<f32>, String> {
        // 步骤 1: 检查内存缓存
        {
            let cache = self.memory_cache.read();
            if let Some(embedding) = cache.get(text) {
                return Ok(embedding.clone());
            }
        }
        
        // 步骤 2: 检查数据库缓存
        // （省略实现）
        
        // 步骤 3: 调用 API 生成
        let embedding = api.embed(text).await?;
        
        // 步骤 4: 存储到缓存
        {
            let mut cache = self.memory_cache.write();
            cache.insert(text.to_string(), embedding.clone());
        }
        
        Ok(embedding)
    }
}
```

**预期学习成果：**
- [ ] 理解多层缓存策略
- [ ] 能实现高效的 Embedding 生成

---

### 第 5-6 周：高级 RAG 和多 Agent

**目标：** 实现完整的 RAG 系统，学习 Agent 协调

#### 周 5 - 第 1-3 天

**学习内容：**
- [ ] 学习项目的 ChatPage 实现（第 10-11 章）
- [ ] 理解如何在项目中集成 RAG
- [ ] 学习如何处理大文本（分段、重叠等）

**实践任务：**
```rust
// 任务：实现文本分段功能

pub struct TextChunker;

impl TextChunker {
    /// 将长文本分成块，支持重叠
    pub fn chunk_text(
        text: &str,
        chunk_size: usize,
        overlap: usize,
    ) -> Vec<String> {
        let mut chunks = Vec::new();
        let mut start = 0;
        
        while start < text.len() {
            let end = std::cmp::min(start + chunk_size, text.len());
            chunks.push(text[start..end].to_string());
            
            // 滑动窗口
            start = if end == text.len() {
                break;
            } else {
                end - std::cmp::min(overlap, end - start)
            };
        }
        
        chunks
    }
}

// 测试
#[test]
fn test_text_chunking() {
    let text = "这是一个很长的文本...";
    let chunks = TextChunker::chunk_text(text, 100, 20);
    
    assert!(chunks.len() > 0);
    assert!(chunks[0].len() <= 100);
}
```

**预期学习成果：**
- [ ] 理解长文本处理的必要性
- [ ] 能实现文本分段逻辑
- [ ] 理解重叠的作用

---

#### 周 5 - 第 4-5 天

**学习内容：**
- [ ] 学习 Prompt 工程
- [ ] 理解 Few-shot 和 Chain-of-thought
- [ ] 学习如何优化 RAG Prompt

**实践任务：**
```rust
// 任务：实现可配置的 Prompt 模板系统

pub struct PromptTemplate {
    system: String,
    user: String,
}

impl PromptTemplate {
    pub fn rag_with_context(
        question: &str,
        context: &str,
        style: &str,
    ) -> Self {
        let system = match style {
            "formal" => "你是一个专业的知识库助手。请使用正式的语言回答问题。",
            "casual" => "你是一个友好的助手。请用自然的语言回答问题。",
            "academic" => "你是一个学术顾问。请提供有根据的、详细的回答。",
            _ => "你是一个有帮助的助手。",
        };
        
        let user = format!(
            "根据以下信息回答问题：\n\n【信息】\n{}\n\n【问题】\n{}",
            context, question
        );
        
        Self {
            system: system.to_string(),
            user,
        }
    }
}
```

**预期学习成果：**
- [ ] 理解 Prompt 工程的重要性
- [ ] 能编写有效的 Prompt
- [ ] 能创建可重用的 Prompt 模板

---

#### 周 6 - 第 1-3 天

**学习内容：**
- [ ] 学习 SearchAgent 和 ChatAgent 的设计
- [ ] 理解多 Agent 的协调
- [ ] 学习事件驱动架构

**实践任务：**
```rust
// 任务：实现一个简单的 SearchAgent

pub struct SearchAgent {
    vector_search: Arc<VectorSearchService>,
    keyword_search: Arc<KeywordSearchService>,
}

impl SearchAgent {
    pub async fn smart_search(
        &self,
        kb_id: &str,
        query: &str,
    ) -> Result<SearchResults, String> {
        // 分析查询类型
        let is_entity_query = self.is_entity_query(query);
        
        if is_entity_query {
            // 实体查询用关键词搜索
            self.keyword_search.search(kb_id, query, 10).await
        } else {
            // 语义查询用向量搜索
            self.vector_search.search(kb_id, query, 10).await
        }
    }
    
    fn is_entity_query(&self, query: &str) -> bool {
        // 简单的启发式规则：包含专有名词的查询是实体查询
        // 更复杂的实现可以使用 NLP
        query.chars().any(|c| c.is_uppercase())
    }
}
```

**预期学习成果：**
- [ ] 理解智能搜索的实现
- [ ] 能设计简单的启发式算法
- [ ] 理解 Agent 之间的协调

---

#### 周 6 - 第 4-5 天

**学习内容：**
- [ ] 整合第 5-6 周的所有知识
- [ ] 设计一个完整的 RAG 流程

**实践任务：**
```rust
// 任务：完整的 RAG 系统集成测试

#[tokio::test]
async fn test_complete_rag_system() {
    // 1. 初始化所有服务
    let embedding_service = create_test_embedding_service().await;
    let search_agent = create_test_search_agent();
    let chat_agent = create_test_chat_agent();
    
    // 2. 测试完整流程
    let query = "刘汶林是谁？";
    
    // 步骤 1: 搜索
    let search_results = search_agent.smart_search("kb1", query).await.unwrap();
    assert!(search_results.documents.len() > 0);
    
    // 步骤 2: 对话
    let response = chat_agent.chat("kb1", query, false).await.unwrap();
    assert!(!response.answer.is_empty());
    
    // 步骤 3: 验证引用
    assert!(!response.citations.is_empty());
}
```

**预期学习成果：**
- [ ] 能端到端地实现 RAG 系统
- [ ] 理解各个组件的集成
- [ ] 能编写综合测试

---

### 第 7 周：MCP 和性能优化

**目标：** 学习 MCP 协议，优化 RAG 性能

#### 周 7 - 第 1-3 天

**学习内容：**
- [ ] 阅读第 7 章（MCP 协议）
- [ ] 理解 MCP 的工具、资源和提示
- [ ] 学习如何设计 MCP 服务器

**实践任务：**
```rust
// 任务：实现一个简单的 MCP 工具

pub struct MCPTools;

impl MCPTools {
    pub fn list_tools() -> Vec<ToolDef> {
        vec![
            ToolDef {
                name: "search_kb".to_string(),
                description: "在知识库中搜索".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "query": { "type": "string" },
                        "top_k": { "type": "integer" }
                    }
                }),
            },
        ]
    }
    
    pub async fn call_tool(
        name: &str,
        input: serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        match name {
            "search_kb" => {
                let query = input["query"].as_str().unwrap_or("");
                let top_k = input["top_k"].as_u64().unwrap_or(10);
                // 实现搜索逻辑
                Ok(serde_json::json!({"results": []}))
            }
            _ => Err("未知工具".to_string()),
        }
    }
}

struct ToolDef {
    name: String,
    description: String,
    input_schema: serde_json::Value,
}
```

**预期学习成果：**
- [ ] 理解 MCP 协议的基本结构
- [ ] 能设计简单的 MCP 工具
- [ ] 理解如何扩展工具集

---

#### 周 7 - 第 4-5 天

**学习内容：**
- [ ] 学习性能优化技巧
- [ ] 理解缓存、索引、批处理
- [ ] 学习性能监控

**实践任务：**
```rust
// 任务：实现性能监控系统

pub struct PerformanceMonitor {
    metrics: Arc<parking_lot::RwLock<Metrics>>,
}

#[derive(Clone)]
struct Metrics {
    total_queries: u64,
    total_time_ms: u64,
    max_time_ms: u64,
    min_time_ms: u64,
}

impl PerformanceMonitor {
    pub fn record_query(&self, time_ms: u64) {
        let mut m = self.metrics.write();
        m.total_queries += 1;
        m.total_time_ms += time_ms;
        m.max_time_ms = m.max_time_ms.max(time_ms);
        m.min_time_ms = if m.min_time_ms == 0 {
            time_ms
        } else {
            m.min_time_ms.min(time_ms)
        };
    }
    
    pub fn get_avg_time(&self) -> f64 {
        let m = self.metrics.read();
        if m.total_queries == 0 {
            0.0
        } else {
            m.total_time_ms as f64 / m.total_queries as f64
        }
    }
}
```

**预期学习成果：**
- [ ] 理解性能监控的重要性
- [ ] 能收集和分析性能数据
- [ ] 了解性能优化的方向

---

### 第 8 周：项目集成和交付

**目标：** 将所有学习内容集成到项目中，完成端到端的 RAG 系统

#### 周 8 - 第 1-3 天

**学习内容：**
- [ ] 回顾所有学习的知识
- [ ] 规划项目集成方案
- [ ] 制定测试计划

**实践任务：**
```
任务清单：
□ 在项目中添加 VectorSearchService
□ 在项目中添加 EmbeddingService
□ 在项目中添加 SearchAgent
□ 在项目中添加 ChatAgent
□ 在项目中添加 RAGService
□ 更新 ChatPage 以使用新的 RAG 能力
□ 编写端到端测试
□ 编写性能测试
□ 准备用户文档
```

**预期学习成果：**
- [ ] 清楚地理解项目集成的各个步骤
- [ ] 有具体的实施计划

---

#### 周 8 - 第 4-5 天

**最终项目：** 完整的知识库 RAG 系统

**要求：**
1. ✅ 支持向量搜索、关键词搜索和混合搜索
2. ✅ 支持自动 Embedding 生成和缓存
3. ✅ 集成到 ChatPage，支持基于知识库的回答
4. ✅ 显示信息来源和相关度分数
5. ✅ 性能指标监控
6. ✅ 完整的单元测试和集成测试

**验收标准：**
- [ ] 编译通过（0 errors, 0 warnings）
- [ ] 所有测试通过
- [ ] RAG 查询延迟 < 2 秒
- [ ] 搜索准确率 > 80%（根据人工评测）
- [ ] 代码覆盖率 > 70%

---

## 📚 推荐学习资源

### 官方文档

| 资源 | 链接 | 难度 |
|------|------|------|
| **Tauri 官方文档** | https://tauri.app/docs | ⭐⭐ |
| **Rust 官方书** | https://doc.rust-lang.org/book/ | ⭐⭐⭐ |
| **OpenAI API 文档** | https://platform.openai.com/docs | ⭐ |
| **SQLite 官方文档** | https://www.sqlite.org/docs.html | ⭐⭐ |

### 开源项目参考

| 项目 | 用途 | 学习价值 |
|------|------|---------|
| **LangChain** | Python Agent 框架 | 了解 Agent 设计模式 |
| **LlamaIndex** | RAG 系统框架 | 了解 RAG 工程最佳实践 |
| **FastAPI** | Python 后端框架 | 了解异步编程模式 |
| **Qdrant** | 向量数据库 | 了解向量搜索实现 |

### 学习材料

**书籍：**
- 《Rust 权威指南》（The Rust Programming Language）
- 《深入浅出 LLM》
- 《信息检索导论》

**论文：**
- "Retrieval-Augmented Generation for Knowledge-Intensive NLP Tasks"
- "Dense Passage Retrieval for Open-Domain Question Answering"
- "Scaling Language Models"

**视频课程：**
- DeepLearning.AI 的 RAG 课程
- Andrej Karpathy 的 NLP 讲座
- Tauri 官方视频教程

---

## ✅ 学习检查清单

### 基础概念检查

- [ ] 能解释什么是 Agent
- [ ] 能解释什么是 Skill
- [ ] 能描述 Agent 和 Skill 的关系
- [ ] 能画出项目的 4 阶段处理流程图
- [ ] 能解释任务队列的作用
- [ ] 能解释事件总线的作用

### RAG 系统检查

- [ ] 理解 RAG 的三个步骤
- [ ] 能计算向量相似度
- [ ] 能区分向量搜索和关键词搜索
- [ ] 能实现混合搜索
- [ ] 能解释 Embedding 的作用
- [ ] 能设计 RAG Prompt

### 代码实现检查

- [ ] 能添加新的 Skill
- [ ] 能实现向量搜索
- [ ] 能实现 Embedding 缓存
- [ ] 能集成 SearchAgent
- [ ] 能集成 ChatAgent
- [ ] 能编写端到端测试

### 性能和优化检查

- [ ] 了解多层缓存策略
- [ ] 了解数据库索引的作用
- [ ] 能监控系统性能
- [ ] 能识别性能瓶颈
- [ ] 能优化查询速度

---

## 🎓 认证和验证

### 自我测评

完成以下问题，验证学习成果：

**基础问题：**
1. 什么是 Agent？它和普通函数的区别是什么？
2. 设计一个新的 Skill，需要考虑哪些因素？
3. RAG 系统中，检索的质量如何影响最终答案？

**实践问题：**
4. 如何实现一个向量搜索功能？需要哪些组件？
5. 如何优化 RAG 系统的搜索速度？
6. 如何评估 RAG 系统的准确率？

**设计问题：**
7. 为一个医学知识库设计 RAG 系统，需要考虑哪些特殊需求？
8. 如何处理 Embedding 维度很高的情况？
9. 多 Agent 协作时，如何处理冲突和不一致？

---

## 📊 学习进度追踪

创建一个表格来追踪学习进度：

| 周 | 章节 | 学习完成 | 实践完成 | 笔记 |
|----|------|---------|---------|------|
| 1-2 | Agent & Skill | ✅ | ⏳ | 已完成基本概念 |
| 3-4 | RAG 基础 | ⏳ | | |
| 5-6 | 高级 RAG | | | |
| 7 | MCP & 优化 | | | |
| 8 | 项目集成 | | | |

---

## 🚀 学习完成后的建议

**短期（1 个月后）：**
- [ ] 将 RAG 系统部署到生产环境
- [ ] 收集用户反馈
- [ ] 持续优化搜索准确率

**中期（3 个月后）：**
- [ ] 扩展到多个知识库
- [ ] 添加更多 Skill（图表识别、表格提取等）
- [ ] 实现用户反馈循环

**长期（6 个月后）：**
- [ ] 实现多语言支持
- [ ] 集成专业的向量数据库（Pinecone/Milvus）
- [ ] 构建行业解决方案

---

**祝你学习顺利！🎉**

如有问题，随时查阅本指南中的相关章节，或在项目的 GitHub Issues 中提问。
