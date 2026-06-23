# Agent、RAG、向量检索 实践指南和案例

> 从理论到实践：完整的代码示例、配置指南和真实案例

---

## 第一部分：快速开始

### 环境配置

#### 1. 后端依赖（Cargo.toml）

```toml
[dependencies]
# 核心框架
tauri = { version = "2", features = ["cli"] }
tokio = { version = "1", features = ["full"] }

# 数据库
rusqlite = { version = "0.31", features = ["bundled"] }

# 网络和序列化
reqwest = { version = "0.12", features = ["json", "stream"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"

# 向量处理
ndarray = "0.15"                    # 数值计算
ndarray-linalg = "0.15"            # 线性代数

# 搜索和 NLP
tantivy = "0.21"                   # 全文搜索库
unicode-normalization = "0.1"      # Unicode 处理

# UUID 和时间
uuid = { version = "1", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde"] }

# 日志
log = "0.4"
env_logger = "0.11"

# 并发工具
parking_lot = "0.12"
dashmap = "5.5"                    # 并发 HashMap
```

#### 2. Python 环境配置（用于 Embedding 和 LLM）

```bash
# 创建虚拟环境
python -m venv venv
source venv/bin/activate  # Linux/Mac
# 或
venv\Scripts\activate  # Windows

# 安装依赖
pip install -r requirements.txt
```

**requirements.txt:**
```
openai>=1.0.0
anthropic>=0.7.0
sentence-transformers>=2.2.0
numpy>=1.23.0
fastapi>=0.100.0
uvicorn>=0.23.0
pydantic>=2.0.0
httpx>=0.24.0
```

---

## 第二部分：实现完整的 RAG 系统

### 案例 1：从零开始构建知识库 RAG

#### 步骤 1：创建知识库项目

```rust
// src-tauri/src/services/mod.rs

pub mod rag_service;
pub mod embedding_service;
pub mod retrieval_service;
pub mod prompt_service;

pub use rag_service::RAGService;
pub use embedding_service::EmbeddingService;
pub use retrieval_service::RetrievalService;
pub use prompt_service::PromptService;
```

#### 步骤 2：实现 Embedding 服务

```rust
// src-tauri/src/services/embedding_service.rs

use std::sync::Arc;
use crate::core::config_service::ConfigService;

pub struct EmbeddingService {
    config: Arc<ConfigService>,
    cache: Arc<parking_lot::RwLock<std::collections::HashMap<String, Vec<f32>>>>,
}

impl EmbeddingService {
    pub fn new(config: Arc<ConfigService>) -> Self {
        Self {
            config,
            cache: Arc::new(parking_lot::RwLock::new(std::collections::HashMap::new())),
        }
    }
    
    /// 生成文本的 Embedding
    pub async fn embed(&self, text: &str) -> Result<Vec<f32>, String> {
        // 检查缓存
        {
            let cache = self.cache.read();
            if let Some(embedding) = cache.get(text) {
                return Ok(embedding.clone());
            }
        }
        
        // 调用 Embedding 模型
        let embedding = self.call_embedding_api(text).await?;
        
        // 存储到缓存
        {
            let mut cache = self.cache.write();
            cache.insert(text.to_string(), embedding.clone());
        }
        
        Ok(embedding)
    }
    
    /// 批量生成 Embedding
    pub async fn embed_batch(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>, String> {
        // 使用 futures 并行处理
        use futures::future::join_all;
        
        let futures = texts.iter().map(|text| self.embed(text));
        let results = join_all(futures).await;
        
        results.into_iter().collect()
    }
    
    /// 调用 OpenAI API（或其他 Embedding 模型）
    async fn call_embedding_api(&self, text: &str) -> Result<Vec<f32>, String> {
        let api_key = std::env::var("OPENAI_API_KEY")
            .map_err(|_| "OPENAI_API_KEY 环境变量未设置".to_string())?;
        
        let client = reqwest::Client::new();
        
        let response = client
            .post("https://api.openai.com/v1/embeddings")
            .header("Authorization", format!("Bearer {}", api_key))
            .json(&serde_json::json!({
                "model": "text-embedding-3-small",
                "input": text,
            }))
            .send()
            .await
            .map_err(|e| format!("API 请求失败: {}", e))?;
        
        let data = response
            .json::<serde_json::Value>()
            .await
            .map_err(|e| format!("解析响应失败: {}", e))?;
        
        let embedding = data["data"][0]["embedding"]
            .as_array()
            .ok_or("无效的 Embedding 响应")?
            .iter()
            .filter_map(|v| v.as_f64())
            .map(|v| v as f32)
            .collect();
        
        Ok(embedding)
    }
    
    /// 计算余弦相似度
    pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
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
}
```

#### 步骤 3：实现检索服务

```rust
// src-tauri/src/services/retrieval_service.rs

use std::sync::Arc;
use crate::core::database_service::DatabaseService;
use crate::services::embedding_service::EmbeddingService;

pub struct RetrievalService {
    db: Arc<DatabaseService>,
    embedding: Arc<EmbeddingService>,
}

impl RetrievalService {
    pub fn new(
        db: Arc<DatabaseService>,
        embedding: Arc<EmbeddingService>,
    ) -> Self {
        Self { db, embedding }
    }
    
    /// 向量相似度搜索
    pub async fn vector_search(
        &self,
        kb_id: &str,
        query: &str,
        top_k: usize,
    ) -> Result<Vec<RetrievedDocument>, String> {
        // 生成查询的 Embedding
        let query_embedding = self.embedding.embed(query).await?;
        
        // 从数据库读取所有文档的 Embedding
        let conn = self.db.connect()?;
        let mut stmt = conn.prepare(
            "SELECT id, title, content, embedding 
             FROM knowledge_items 
             WHERE kb_id = ?1"
        ).map_err(|e| format!("查询失败: {}", e))?;
        
        let mut scores = Vec::new();
        
        let docs = stmt.query_map(
            rusqlite::params![kb_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                ))
            }
        ).map_err(|e| format!("读取失败: {}", e))?;
        
        for doc in docs {
            let (id, title, content, embedding_json) = doc
                .map_err(|e| format!("迭代失败: {}", e))?;
            
            // 反序列化 Embedding
            let doc_embedding: Vec<f32> = serde_json::from_str(&embedding_json)
                .unwrap_or_default();
            
            // 计算相似度
            let similarity = EmbeddingService::cosine_similarity(
                &query_embedding,
                &doc_embedding,
            );
            
            scores.push(RetrievedDocument {
                id,
                title,
                content,
                score: similarity,
            });
        }
        
        // 按相似度排序
        scores.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        
        // 返回 Top-K
        Ok(scores.into_iter().take(top_k).collect())
    }
    
    /// 关键词搜索（全文搜索）
    pub fn keyword_search(
        &self,
        kb_id: &str,
        query: &str,
        top_k: usize,
    ) -> Result<Vec<RetrievedDocument>, String> {
        let conn = self.db.connect()?;
        
        // 使用 SQL LIKE 查询
        let search_pattern = format!("%{}%", query);
        let mut stmt = conn.prepare(
            "SELECT id, title, content, 
                    CASE 
                        WHEN title LIKE ?1 THEN 2
                        WHEN content LIKE ?1 THEN 1
                        ELSE 0
                    END as relevance
             FROM knowledge_items 
             WHERE kb_id = ?2 AND relevance > 0
             ORDER BY relevance DESC
             LIMIT ?3"
        ).map_err(|e| format!("查询失败: {}", e))?;
        
        let docs = stmt.query_map(
            rusqlite::params![&search_pattern, kb_id, top_k],
            |row| {
                Ok(RetrievedDocument {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    content: row.get(2)?,
                    score: row.get::<_, i32>(3)? as f32 / 2.0,
                })
            }
        ).map_err(|e| format!("读取失败: {}", e))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("收集失败: {}", e))?;
        
        Ok(docs)
    }
    
    /// 混合搜索
    pub async fn hybrid_search(
        &self,
        kb_id: &str,
        query: &str,
        top_k: usize,
    ) -> Result<Vec<RetrievedDocument>, String> {
        // 并行执行两种搜索
        let vector_results = self.vector_search(kb_id, query, top_k * 2).await?;
        let keyword_results = self.keyword_search(kb_id, query, top_k * 2)?;
        
        // 融合结果
        let mut combined: std::collections::HashMap<String, (String, String, f32)> = 
            std::collections::HashMap::new();
        
        // 添加向量搜索结果（权重 0.7）
        for doc in vector_results {
            combined.insert(
                doc.id.clone(),
                (doc.title, doc.content, doc.score * 0.7),
            );
        }
        
        // 添加关键词搜索结果（权重 0.3）
        for doc in keyword_results {
            let (title, content, vec_score) = combined
                .entry(doc.id.clone())
                .or_insert((doc.title.clone(), doc.content.clone(), 0.0));
            
            *vec_score += doc.score * 0.3;
        }
        
        // 转换为列表并排序
        let mut results: Vec<_> = combined
            .into_iter()
            .map(|(id, (title, content, score))| RetrievedDocument {
                id,
                title,
                content,
                score,
            })
            .collect();
        
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        
        Ok(results.into_iter().take(top_k).collect())
    }
}

#[derive(Debug, Clone)]
pub struct RetrievedDocument {
    pub id: String,
    pub title: String,
    pub content: String,
    pub score: f32,
}
```

#### 步骤 4：实现 Prompt 生成服务

```rust
// src-tauri/src/services/prompt_service.rs

pub struct PromptService;

impl PromptService {
    /// 生成 RAG Prompt
    pub fn generate_rag_prompt(
        question: &str,
        documents: &[RetrievedDocument],
        system_prompt: Option<&str>,
    ) -> String {
        let default_system = r#"你是一个有帮助的知识库助手。
根据下面提供的文档回答用户的问题。
如果文档中没有相关信息，请说明这一点。
对关键信息请标注来源。"#;
        
        let system = system_prompt.unwrap_or(default_system);
        
        // 格式化检索到的文档
        let documents_text = documents
            .iter()
            .enumerate()
            .map(|(i, doc)| {
                format!(
                    "[文档 {}] 标题: {}\n内容摘要: {}...\n相关度: {:.1}%",
                    i + 1,
                    doc.title,
                    &doc.content[..std::cmp::min(200, doc.content.len())],
                    doc.score * 100.0
                )
            })
            .collect::<Vec<_>>()
            .join("\n\n");
        
        format!(
            r#"{}

【检索到的文档】
{}

【用户问题】
{}

【请你的回答】"#,
            system, documents_text, question
        )
    }
    
    /// 生成对话 Prompt
    pub fn generate_chat_prompt(
        question: &str,
        context: &str,
        history: &[ChatMessage],
    ) -> String {
        let mut prompt = format!("上下文信息:\n{}\n\n", context);
        
        // 添加对话历史
        if !history.is_empty() {
            prompt.push_str("对话历史:\n");
            for msg in history {
                prompt.push_str(&format!("{}: {}\n", msg.role, msg.content));
            }
            prompt.push('\n');
        }
        
        prompt.push_str(&format!("用户: {}\n助手: ", question));
        prompt
    }
}

#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}
```

#### 步骤 5：实现完整的 RAG 服务

```rust
// src-tauri/src/services/rag_service.rs

use std::sync::Arc;
use crate::services::{
    embedding_service::EmbeddingService,
    retrieval_service::RetrievalService,
    prompt_service::PromptService,
};
use crate::model::model_gateway::ModelGateway;

pub struct RAGService {
    embedding: Arc<EmbeddingService>,
    retrieval: Arc<RetrievalService>,
    model_gateway: Arc<ModelGateway>,
}

impl RAGService {
    pub fn new(
        embedding: Arc<EmbeddingService>,
        retrieval: Arc<RetrievalService>,
        model_gateway: Arc<ModelGateway>,
    ) -> Self {
        Self {
            embedding,
            retrieval,
            model_gateway,
        }
    }
    
    /// 执行 RAG 查询
    pub async fn query(
        &self,
        kb_id: &str,
        question: &str,
        search_type: SearchType,
    ) -> Result<RAGResponse, String> {
        // 步骤 1: 检索相关文档
        let documents = match search_type {
            SearchType::Vector => {
                self.retrieval.vector_search(kb_id, question, 5).await?
            }
            SearchType::Keyword => {
                self.retrieval.keyword_search(kb_id, question, 5)?
            }
            SearchType::Hybrid => {
                self.retrieval.hybrid_search(kb_id, question, 5).await?
            }
        };
        
        // 步骤 2: 生成 Prompt
        let prompt = PromptService::generate_rag_prompt(question, &documents, None);
        
        // 步骤 3: 调用 LLM
        let response = self.model_gateway.chat(
            &self.config.get_deepseek_config()?,
            vec![ChatMessage {
                role: "user".to_string(),
                content: prompt,
            }],
            false,
        ).await?;
        
        // 步骤 4: 提取引用
        let citations = self.extract_citations(&response.content, &documents);
        
        Ok(RAGResponse {
            answer: response.content,
            documents,
            citations,
            search_type: search_type.to_string(),
        })
    }
    
    /// 提取文档引用
    fn extract_citations(
        &self,
        text: &str,
        documents: &[RetrievedDocument],
    ) -> Vec<Citation> {
        let mut citations = Vec::new();
        let re = regex::Regex::new(r"\[文档\s+(\d+)\]").unwrap();
        
        for cap in re.captures_iter(text) {
            if let Ok(idx) = cap[1].parse::<usize>() {
                if let Some(doc) = documents.get(idx - 1) {
                    citations.push(Citation {
                        reference: cap[0].to_string(),
                        document_id: doc.id.clone(),
                        title: doc.title.clone(),
                        relevance_score: doc.score,
                    });
                }
            }
        }
        
        citations
    }
}

#[derive(Debug, Clone)]
pub enum SearchType {
    Vector,
    Keyword,
    Hybrid,
}

impl SearchType {
    fn to_string(&self) -> String {
        match self {
            SearchType::Vector => "向量检索".to_string(),
            SearchType::Keyword => "关键词检索".to_string(),
            SearchType::Hybrid => "混合检索".to_string(),
        }
    }
}

pub struct RAGResponse {
    pub answer: String,
    pub documents: Vec<RetrievedDocument>,
    pub citations: Vec<Citation>,
    pub search_type: String,
}

pub struct Citation {
    pub reference: String,
    pub document_id: String,
    pub title: String,
    pub relevance_score: f32,
}
```

---

### 案例 2：ChatPage 集成 RAG

```typescript
// src/pages/ChatPage.tsx 中的 RAG 集成

import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';

interface RAGDocument {
  id: string;
  title: string;
  content: string;
  score: number;
}

interface RAGResponse {
  answer: string;
  documents: RAGDocument[];
  citations: Citation[];
  search_type: string;
}

interface Citation {
  reference: string;
  document_id: string;
  title: string;
  relevance_score: number;
}

export function ChatPage() {
  const [query, setQuery] = useState('');
  const [responses, setResponses] = useState<RAGResponse[]>([]);
  const [loading, setLoading] = useState(false);
  const [searchType, setSearchType] = useState<'vector' | 'keyword' | 'hybrid'>('hybrid');
  const [showRetrieved, setShowRetrieved] = useState(false);

  const handleQuery = async () => {
    if (!query.trim()) return;

    setLoading(true);
    try {
      const response = await invoke<RAGResponse>('rag_query', {
        kb_id: currentKB.id,
        question: query,
        search_type: searchType,
      });

      setResponses([response, ...responses]);
      setQuery('');
    } catch (error) {
      console.error('查询失败:', error);
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="flex flex-col h-full">
      {/* 搜索类型选择 */}
      <div className="p-4 border-b flex gap-4">
        <select
          value={searchType}
          onChange={(e) => setSearchType(e.target.value as any)}
          className="px-3 py-2 border rounded"
        >
          <option value="vector">向量检索</option>
          <option value="keyword">关键词检索</option>
          <option value="hybrid">混合检索</option>
        </select>
        
        <button
          onClick={() => setShowRetrieved(!showRetrieved)}
          className="px-3 py-2 bg-blue-500 text-white rounded"
        >
          {showRetrieved ? '隐藏' : '显示'}检索文档
        </button>
      </div>

      {/* 回答显示 */}
      <div className="flex-1 overflow-y-auto p-4 space-y-4">
        {responses.map((response, idx) => (
          <div key={idx} className="border rounded p-4 bg-white">
            {/* 答案 */}
            <div className="mb-4">
              <h3 className="font-semibold text-lg mb-2">回答</h3>
              <p className="text-gray-700">{response.answer}</p>
            </div>

            {/* 引用 */}
            {response.citations.length > 0 && (
              <div className="mb-4 p-3 bg-blue-50 rounded">
                <h4 className="font-semibold text-sm mb-2">信息来源</h4>
                <ul className="space-y-1">
                  {response.citations.map((citation, i) => (
                    <li key={i} className="text-sm text-blue-700">
                      {citation.reference}: {citation.title} 
                      (相关度: {(citation.relevance_score * 100).toFixed(1)}%)
                    </li>
                  ))}
                </ul>
              </div>
            )}

            {/* 检索文档（可选显示）*/}
            {showRetrieved && (
              <div className="p-3 bg-gray-50 rounded">
                <h4 className="font-semibold text-sm mb-2">
                  检索到的文档 ({response.search_type})
                </h4>
                <div className="space-y-2">
                  {response.documents.map((doc, i) => (
                    <div
                      key={i}
                      className="p-2 bg-white border rounded text-sm"
                    >
                      <div className="font-semibold">{doc.title}</div>
                      <div className="text-gray-600 text-xs mt-1">
                        {doc.content.substring(0, 150)}...
                      </div>
                      <div className="text-blue-600 text-xs mt-1">
                        相关度: {(doc.score * 100).toFixed(1)}%
                      </div>
                    </div>
                  ))}
                </div>
              </div>
            )}
          </div>
        ))}
      </div>

      {/* 查询输入 */}
      <div className="p-4 border-t flex gap-2">
        <input
          type="text"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          onKeyPress={(e) => e.key === 'Enter' && handleQuery()}
          placeholder="输入问题..."
          className="flex-1 px-4 py-2 border rounded"
          disabled={loading}
        />
        <button
          onClick={handleQuery}
          disabled={loading}
          className="px-4 py-2 bg-green-500 text-white rounded disabled:opacity-50"
        >
          {loading ? '处理中...' : '查询'}
        </button>
      </div>
    </div>
  );
}
```

---

### 案例 3：知识项 Embedding 初始化

```rust
// Tauri 命令：初始化知识库的 Embedding

#[tauri::command]
pub async fn initialize_kb_embeddings(
    kernel: State<'_, Arc<AppKernel>>,
    kb_id: String,
) -> Result<String, String> {
    let embedding_service = Arc::new(
        EmbeddingService::new(kernel.config.clone())
    );
    
    // 读取所有知识项
    let conn = kernel.db.connect()?;
    let mut stmt = conn.prepare(
        "SELECT id, title, content FROM knowledge_items WHERE kb_id = ?1"
    ).map_err(|e| format!("查询失败: {}", e))?;
    
    let items: Vec<(String, String, String)> = stmt.query_map(
        rusqlite::params![kb_id],
        |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
            ))
        }
    ).map_err(|e| format!("读取失败: {}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("收集失败: {}", e))?;
    
    let total = items.len();
    
    // 批量生成 Embedding
    for (i, (id, title, content)) in items.iter().enumerate() {
        // 组合标题和内容
        let text = format!("{}\n{}", title, content);
        
        // 生成 Embedding
        let embedding = embedding_service.embed(&text).await?;
        
        // 保存到数据库
        conn.execute(
            "INSERT INTO knowledge_item_embeddings (kb_id, item_id, embedding_vector, embedding_dim, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(kb_id, item_id) DO UPDATE SET embedding_vector = ?3, embedding_dim = ?4",
            rusqlite::params![
                kb_id,
                id,
                serde_json::to_string(&embedding)?,
                embedding.len(),
                chrono::Utc::now().to_rfc3339(),
            ],
        ).map_err(|e| format!("保存失败: {}", e))?;
        
        log::info!("已处理 {}/{}", i + 1, total);
    }
    
    Ok(format!("成功初始化 {} 个知识项的 Embedding", total))
}
```

---

## 第三部分：性能优化和监控

### 监控 RAG 系统性能

```rust
// src-tauri/src/services/monitoring.rs

pub struct RAGMetrics {
    pub query_count: u64,
    pub avg_retrieval_time_ms: f64,
    pub avg_answer_time_ms: f64,
    pub avg_relevance_score: f32,
    pub error_rate: f32,
}

pub struct MetricsCollector {
    metrics: parking_lot::RwLock<RAGMetrics>,
}

impl MetricsCollector {
    pub fn new() -> Self {
        Self {
            metrics: parking_lot::RwLock::new(RAGMetrics {
                query_count: 0,
                avg_retrieval_time_ms: 0.0,
                avg_answer_time_ms: 0.0,
                avg_relevance_score: 0.0,
                error_rate: 0.0,
            }),
        }
    }
    
    pub fn record_query(&self, retrieval_time: f64, answer_time: f64, score: f32) {
        let mut metrics = self.metrics.write();
        
        // 更新计数
        metrics.query_count += 1;
        
        // 更新平均值（移动平均）
        metrics.avg_retrieval_time_ms = 
            (metrics.avg_retrieval_time_ms * 0.9) + (retrieval_time * 0.1);
        metrics.avg_answer_time_ms = 
            (metrics.avg_answer_time_ms * 0.9) + (answer_time * 0.1);
        metrics.avg_relevance_score = 
            (metrics.avg_relevance_score * 0.9) + (score * 0.1);
    }
    
    pub fn get_metrics(&self) -> RAGMetrics {
        self.metrics.read().clone()
    }
}
```

### 缓存优化

```rust
// 多层缓存策略

pub struct CacheLayer {
    l1_cache: Arc<parking_lot::RwLock<lru::LruCache<String, Vec<f32>>>>,  // 内存缓存（热数据）
    l2_cache: Arc<DatabaseService>,  // 数据库缓存（冷数据）
}

impl CacheLayer {
    pub fn new(l1_capacity: usize, db: Arc<DatabaseService>) -> Self {
        Self {
            l1_cache: Arc::new(parking_lot::RwLock::new(
                lru::LruCache::new(std::num::NonZeroUsize::new(l1_capacity).unwrap())
            )),
            l2_cache: db,
        }
    }
    
    pub async fn get_embedding(&self, text: &str) -> Result<Option<Vec<f32>>, String> {
        // L1: 检查内存缓存
        {
            let mut l1 = self.l1_cache.write();
            if let Some(embedding) = l1.get(text) {
                return Ok(Some(embedding.clone()));
            }
        }
        
        // L2: 检查数据库缓存
        // （实现省略）
        
        Ok(None)
    }
    
    pub fn put_embedding(&self, text: String, embedding: Vec<f32>) {
        let mut l1 = self.l1_cache.write();
        l1.put(text, embedding);
    }
}
```

---

## 总结：完整的 RAG 系统架构图

```
┌─────────────────────────────────────────────────────────────┐
│                       用户输入                               │
│                     (问题或查询)                              │
└────────────────────────┬────────────────────────────────────┘
                         │
                         ▼
          ┌──────────────────────────────┐
          │   EmbeddingService           │
          │  将问题转换为向量             │
          └──────────────┬───────────────┘
                         │
                         ▼
          ┌──────────────────────────────┐
          │   RetrievalService           │
          │  ├─ 向量检索                  │
          │  ├─ 关键词检索                │
          │  └─ 混合检索                  │
          └──────────────┬───────────────┘
                         │
                         ▼
          ┌──────────────────────────────┐
          │   MetricsCollector           │
          │  记录检索性能指标             │
          └──────────────┬───────────────┘
                         │
                         ▼
          ┌──────────────────────────────┐
          │   PromptService              │
          │  组装 RAG Prompt             │
          │  (问题 + 检索文档 + 指示)     │
          └──────────────┬───────────────┘
                         │
                         ▼
          ┌──────────────────────────────┐
          │   ModelGateway               │
          │  调用 LLM 生成答案            │
          └──────────────┬───────────────┘
                         │
                         ▼
          ┌──────────────────────────────┐
          │   Citation Extractor         │
          │  从答案中提取文档引用         │
          └──────────────┬───────────────┘
                         │
                         ▼
          ┌──────────────────────────────┐
          │   RAGResponse                │
          │  ├─ 答案文本                  │
          │  ├─ 检索文档                  │
          │  ├─ 引用                      │
          │  └─ 性能指标                  │
          └──────────────┬───────────────┘
                         │
                         ▼
              ┌─────────────────────┐
              │   前端展示           │
              │  显示答案和来源      │
              └─────────────────────┘
```

---

## 常见问题 (FAQ)

### Q1: 如何选择搜索方式（向量 vs 关键词 vs 混合）？

| 查询类型 | 推荐方式 | 原因 |
|---------|--------|------|
| **特定实体** (如"刘汶林") | 关键词 | 精确匹配 |
| **语义查询** ("工程师做什么") | 向量 | 理解意图 |
| **复杂问题** ("对比两个方案") | 混合 | 综合精确和语义 |

### Q2: Embedding 维度如何选择？

| 维度 | 特点 | 适用场景 |
|------|------|---------|
| 384 | 快速、低成本 | 开发测试 |
| 768 | 平衡 | 中等规模 |
| 1536 | 高精度 | 生产环境 |
| 3072 | 最高精度 | 对精度要求极高 |

### Q3: 如何提高 RAG 的准确率？

1. **改进检索**：
   - 使用更好的 Embedding 模型
   - 调整混合搜索权重
   - 添加重排序（reranking）

2. **改进 Prompt**：
   - 更好的系统提示
   - Few-shot 示例
   - 指定输出格式

3. **改进数据**：
   - 清理知识库
   - 添加元数据
   - 分类标签

---

**恭喜！你现在拥有了完整的 RAG 系统实现指南。** 🎉
