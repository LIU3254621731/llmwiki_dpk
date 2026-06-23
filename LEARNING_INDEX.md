# Agent、RAG、向量检索 完整学习体系 — 总索引

> 快速导航：找到你需要的学习材料

---

## 📚 学习材料总览

| 文档 | 用途 | 阅读时间 | 难度 |
|------|------|---------|------|
| **[AGENT_SKILL_RAG_MCP_GUIDE.md](AGENT_SKILL_RAG_MCP_GUIDE.md)** | 完整的理论教材 | 8-10 小时 | ⭐⭐⭐ |
| **[RAG_IMPLEMENTATION_GUIDE.md](RAG_IMPLEMENTATION_GUIDE.md)** | 实战代码示例 | 4-6 小时 | ⭐⭐⭐⭐ |
| **[LEARNING_ROADMAP.md](LEARNING_ROADMAP.md)** | 8 周学习计划 | 快速阅读 | ⭐ |
| **本文件** | 总索引和导航 | 10 分钟 | ⭐ |

---

## 🚀 快速开始（30 分钟）

### 第一步：理解核心概念（10 分钟）

```
什么是 Agent？
├─ 能够自主决策和行动的 AI 系统
├─ 项目中有 4 个 Agent：
│  ├─ SourceIngestAgent（文档摄入）
│  ├─ ResolutionAgent（知识去重）
│  ├─ RelationshipAgent（关系提取）
│  └─ WikiUpdateAgent（知识库更新）
└─ 通过 CoordinatorAgent 编排

什么是 Skill？
├─ Agent 可以调用的专用工具或模块
├─ 项目中的 Skills：
│  ├─ DocumentProcessor（文档解析）
│  ├─ PDFSkill（PDF 提取）
│  ├─ DOCXSkill（Word 提取）
│  └─ MarkItDownSkill（万能方案）
└─ Skill 设计原则：单一职责、可组合、容错

什么是 RAG？
├─ Retrieval（检索）+ Augmented（增强）+ Generation（生成）
├─ 流程：
│  ├─ 用户问题 → 向量化 → 搜索知识库
│  ├─ 找到相关文档 → 添加到 Prompt
│  └─ LLM 基于增强 Prompt 生成答案
└─ 优势：LLM 能回答知识库中的内容

什么是向量检索？
├─ 将文本转为向量（Embedding）
├─ 比较向量相似度而不是关键词匹配
├─ 优势：理解语义而不是字面意思
└─ 与关键词搜索的混合称为"混合搜索"
```

### 第二步：查看项目代码（10 分钟）

```bash
# 在项目中找到关键文件：

# 1. 查看 Agent 实现
code src-tauri/src/agents/coordinator.rs
code src-tauri/src/agents/source_ingest.rs

# 2. 查看 Skill 实现
code src-tauri/src/skills/document_processor.rs
code src-tauri/src/skills/pdf_skill.rs

# 3. 查看数据库架构
code src-tauri/src/db/schema.rs
code src-tauri/src/db/migrations/

# 4. 查看模型调用
code src-tauri/src/model/model_gateway.rs
```

### 第三步：选择学习路径（10 分钟）

**如果你是初学者：**
```
推荐顺序：
1. 阅读本索引文档（总览）
2. 阅读 AGENT_SKILL_RAG_MCP_GUIDE.md 第 1-3 章（基础）
3. 查看项目代码（实例）
4. 按照 LEARNING_ROADMAP.md 第 1-2 周计划学习
```

**如果你有 LLM 经验但不懂 Rust：**
```
推荐顺序：
1. 先学 Rust 基础（参见 RUST_BASICS_GUIDE.md）
2. 阅读 AGENT_SKILL_RAG_MCP_GUIDE.md 第 4-6 章（RAG 系统）
3. 阅读 RAG_IMPLEMENTATION_GUIDE.md（实践代码）
```

**如果你有 Rust 和 AI 经验：**
```
推荐顺序：
1. 快速扫一遍 AGENT_SKILL_RAG_MCP_GUIDE.md
2. 重点阅读 RAG_IMPLEMENTATION_GUIDE.md
3. 根据 LEARNING_ROADMAP.md 第 5-8 周进行项目改进
```

---

## 🎯 按主题快速查找

### Agent 相关
- 📖 理论：[AGENT_SKILL_RAG_MCP_GUIDE.md - 第 1 章](AGENT_SKILL_RAG_MCP_GUIDE.md#第-1-章agent-系统设计基础)
- 💻 项目代码：`src-tauri/src/agents/coordinator.rs`
- 📚 学习计划：[LEARNING_ROADMAP.md - 第 1-2 周](LEARNING_ROADMAP.md#第-1-2-周基础概念和架构)

### Skill 相关
- 📖 理论：[AGENT_SKILL_RAG_MCP_GUIDE.md - 第 2 章](AGENT_SKILL_RAG_MCP_GUIDE.md#第-2-章skill-模式详解)
- 💻 项目代码：`src-tauri/src/skills/document_processor.rs`
- 📚 学习计划：[LEARNING_ROADMAP.md - 第 1-2 周](LEARNING_ROADMAP.md#周-1---第-4-5-天)

### RAG 系统
- 📖 理论：[AGENT_SKILL_RAG_MCP_GUIDE.md - 第 4 章](AGENT_SKILL_RAG_MCP_GUIDE.md#第-4-章rag-系统检索增强生成)
- 💻 实现：[RAG_IMPLEMENTATION_GUIDE.md - 案例 1](RAG_IMPLEMENTATION_GUIDE.md#案例-1从零开始构建知识库-rag)
- 📚 学习计划：[LEARNING_ROADMAP.md - 第 3-4 周](LEARNING_ROADMAP.md#第-3-4-周rag-系统基础)

### 向量检索
- 📖 理论：[AGENT_SKILL_RAG_MCP_GUIDE.md - 第 5 章](AGENT_SKILL_RAG_MCP_GUIDE.md#第-5-章向量检索embedding--vector-search)
- 💻 实现：[RAG_IMPLEMENTATION_GUIDE.md - 第二部分](RAG_IMPLEMENTATION_GUIDE.md#步骤-3实现检索服务)
- 📚 学习计划：[LEARNING_ROADMAP.md - 第 4 周](LEARNING_ROADMAP.md#周-4---第-1-3-天)

### MCP 协议
- 📖 理论：[AGENT_SKILL_RAG_MCP_GUIDE.md - 第 7 章](AGENT_SKILL_RAG_MCP_GUIDE.md#第-7-章mcp-协议model-context-protocol)
- 📚 学习计划：[LEARNING_ROADMAP.md - 第 7 周](LEARNING_ROADMAP.md#第-7-周mcp-和性能优化)

### 性能优化
- 📖 理论：[AGENT_SKILL_RAG_MCP_GUIDE.md - 第 9 章](AGENT_SKILL_RAG_MCP_GUIDE.md#第-9-章性能优化)
- 💻 实现：[RAG_IMPLEMENTATION_GUIDE.md - 第三部分](RAG_IMPLEMENTATION_GUIDE.md#第三部分性能优化和监控)

---

## 📖 按学习阶段的文档地图

### 阶段 1：基础概念（1-2 周）

```
开始
  ↓
阅读本索引文档（总览）
  ↓
AGENT_SKILL_RAG_MCP_GUIDE.md
  ├─ 第 1 章：Agent 系统设计基础 ⭐
  ├─ 第 2 章：Skill 模式详解 ⭐
  ├─ 第 3 章：Agent 和 Skill 的协作 ⭐
  └─ 第 10 章：项目中的实战模式 ⭐
  ↓
查看项目代码
  ├─ src-tauri/src/agents/coordinator.rs
  ├─ src-tauri/src/skills/document_processor.rs
  └─ src-tauri/src/commands/workspace.rs
  ↓
完成 LEARNING_ROADMAP.md 第 1-2 周任务
```

### 阶段 2：RAG 系统（3-4 周）

```
复习第 1-2 周的内容
  ↓
AGENT_SKILL_RAG_MCP_GUIDE.md
  ├─ 第 4 章：RAG 系统基础 ⭐⭐
  ├─ 第 5 章：向量检索 ⭐⭐
  ├─ 第 6 章：知识库设计
  └─ 第 12 章：实现完整的 RAG 系统
  ↓
RAG_IMPLEMENTATION_GUIDE.md
  ├─ 第二部分：实战应用 ⭐⭐
  └─ 快速开始部分
  ↓
完成 LEARNING_ROADMAP.md 第 3-4 周任务
```

### 阶段 3：高级特性和优化（5-7 周）

```
复习 RAG 系统
  ↓
AGENT_SKILL_RAG_MCP_GUIDE.md
  ├─ 第 7 章：MCP 协议 ⭐
  ├─ 第 8 章：多 Agent 协调
  ├─ 第 9 章：性能优化 ⭐⭐
  └─ 第 10-12 章：项目改进方案
  ↓
RAG_IMPLEMENTATION_GUIDE.md
  ├─ 第三部分：性能优化 ⭐
  └─ 常见问题 (FAQ)
  ↓
完成 LEARNING_ROADMAP.md 第 5-7 周任务
```

### 阶段 4：项目集成（8 周）

```
整合所有知识
  ↓
按照 AGENT_SKILL_RAG_MCP_GUIDE.md
  ├─ 第 10 章改进 SourceIngestAgent
  ├─ 第 11 章添加新 Skill
  └─ 第 12 章完整 RAG 系统
  ↓
参考 RAG_IMPLEMENTATION_GUIDE.md
  ├─ 案例 2：ChatPage 集成
  └─ 案例 3：Embedding 初始化
  ↓
完成 LEARNING_ROADMAP.md 第 8 周任务
```

---

## 💡 常见问题快速解答

### Q: 应该从哪里开始学？
**A:** 
1. 如果你是初学者，从[第一步](#第一步理解核心概念10-分钟)开始
2. 然后按照[阶段 1](#阶段-1基础概念1-2-周)学习
3. 再逐步进入更高阶段

### Q: 需要多长时间学完？
**A:**
- 基础理解：1 周
- 实战项目：3 周
- 精通系统：6 周

### Q: 没有 Rust 基础能学吗？
**A:** 可以，但建议先学 Rust 基础。参考 [RUST_BASICS_GUIDE.md](RUST_BASICS_GUIDE.md)

### Q: 代码示例都是真实的吗？
**A:** 是的，所有代码都基于项目的真实代码或遵循项目的设计模式

### Q: 如何验证学习成果？
**A:** 参考 [LEARNING_ROADMAP.md 的检查清单](LEARNING_ROADMAP.md#-学习检查清单)

---

## 🔗 文档间的交叉引用

```
AGENT_SKILL_RAG_MCP_GUIDE.md
    ├─ 理论基础和设计原则
    ├─ 交叉引用：
    │  ├─ → RAG_IMPLEMENTATION_GUIDE.md 获取代码示例
    │  └─ → RUST_BASICS_GUIDE.md 学习 Rust 细节
    └─ 参考：LEARNING_ROADMAP.md 的学习计划

RAG_IMPLEMENTATION_GUIDE.md
    ├─ 实战代码和案例
    ├─ 交叉引用：
    │  ├─ → AGENT_SKILL_RAG_MCP_GUIDE.md 理解概念
    │  └─ → LEARNING_ROADMAP.md 的任务要求
    └─ 包含：完整的 Rust 代码

LEARNING_ROADMAP.md
    ├─ 8 周学习计划和任务
    ├─ 交叉引用：
    │  ├─ → AGENT_SKILL_RAG_MCP_GUIDE.md 的各章节
    │  └─ → RAG_IMPLEMENTATION_GUIDE.md 的实践任务
    └─ 检查清单和验收标准

RUST_BASICS_GUIDE.md
    ├─ Rust 基础知识
    ├─ 交叉引用：
    │  ├─ → AGENT_SKILL_RAG_MCP_GUIDE.md 的 Rust 代码
    │  └─ → 项目源代码示例
    └─ 项目中的 Rust 模式
```

---

## 📊 知识体系结构图

```
最上层：AI 应用
  ├─ ChatPage（对话界面）
  ├─ ReviewPage（审阅中心）
  └─ GraphPage（知识图谱）
    
    ↓ 使用的服务
    
中间层：AI 服务
  ├─ RAGService（检索增强生成）
  │  ├─ SearchAgent（搜索）
  │  ├─ ChatAgent（对话）
  │  ├─ RetrievalService（检索）
  │  └─ EmbeddingService（向量化）
  │
  ├─ 处理 Agent 系统
  │  ├─ CoordinatorAgent（编排）
  │  ├─ SourceIngestAgent（摄入）
  │  ├─ ResolutionAgent（去重）
  │  ├─ RelationshipAgent（关系）
  │  └─ WikiUpdateAgent（更新）
  │
  └─ Skill 系统
     ├─ DocumentProcessor（文档处理）
     ├─ PDFSkill（PDF）
     ├─ DOCXSkill（Word）
     ├─ MarkItDownSkill（通用）
     └─ ... 其他 Skills
    
    ↓ 基础设施
    
下层：基础设施
  ├─ 数据库（SQLite）
  │  ├─ knowledge_items（知识项）
  │  ├─ graph_nodes/edges（知识图谱）
  │  ├─ tasks（任务队列）
  │  └─ ... 其他表
  │
  ├─ 外部服务
  │  ├─ LLM API（DeepSeek）
  │  ├─ Embedding API（OpenAI）
  │  └─ Web Search（DuckDuckGo）
  │
  └─ 核心基础
     ├─ 异步运行时（Tokio）
     ├─ 事件总线
     └─ 任务队列
```

---

## ✨ 学习收获预期

完成本学习体系后，你将能够：

**理论方面：**
- [ ] 理解 AI Agent 系统的设计原理
- [ ] 理解 RAG 系统的工作机制
- [ ] 理解向量搜索的数学基础
- [ ] 了解 MCP 协议的标准
- [ ] 掌握性能优化的方法

**实践方面：**
- [ ] 能设计和实现新的 Agent
- [ ] 能设计和实现新的 Skill
- [ ] 能构建完整的 RAG 系统
- [ ] 能优化 LLM 应用的性能
- [ ] 能集成多个 AI 服务

**项目方面：**
- [ ] 能改进 LLMWiki 项目
- [ ] 能添加新的功能模块
- [ ] 能诊断和修复问题
- [ ] 能编写高质量代码
- [ ] 能编写完整的测试

---

## 🎓 推荐学习路径

### 路径 A：完全初学者
```
1. 阅读本索引（10 分钟）
2. 学习 RUST_BASICS_GUIDE.md（3 小时）
3. 学习 AGENT_SKILL_RAG_MCP_GUIDE.md（6 小时）
4. 学习 RAG_IMPLEMENTATION_GUIDE.md（4 小时）
5. 按 LEARNING_ROADMAP.md 第 1-4 周学习（2 周）
6. 项目实践（3 周）
总耗时：5-6 周
```

### 路径 B：有 LLM 经验，无 Rust 基础
```
1. 学习 RUST_BASICS_GUIDE.md（2 小时）
2. 学习 AGENT_SKILL_RAG_MCP_GUIDE.md（3 小时）
3. 学习 RAG_IMPLEMENTATION_GUIDE.md（3 小时）
4. 按 LEARNING_ROADMAP.md 第 3-8 周学习（4 周）
5. 项目实践（2 周）
总耗时：3-4 周
```

### 路径 C：有 Rust 经验，无 AI 经验
```
1. 学习 AGENT_SKILL_RAG_MCP_GUIDE.md（5 小时）
2. 学习 RAG_IMPLEMENTATION_GUIDE.md（4 小时）
3. 按 LEARNING_ROADMAP.md 第 2-8 周学习（6 周）
总耗时：2-3 周
```

### 路径 D：全栈工程师
```
1. 快速阅读本索引和 AGENT_SKILL_RAG_MCP_GUIDE.md（2 小时）
2. 详细研究 RAG_IMPLEMENTATION_GUIDE.md（2 小时）
3. 按 LEARNING_ROADMAP.md 第 5-8 周进行高阶学习（2 周）
总耗时：1-2 周
```

---

## 📞 获取帮助

如果在学习过程中遇到问题：

1. **查阅文档**：先在相关文档中搜索关键字
2. **查看代码**：在项目源代码中找实例
3. **查看 FAQ**：在 RAG_IMPLEMENTATION_GUIDE.md 中查找常见问题
4. **学习计划**：在 LEARNING_ROADMAP.md 中查找类似的任务

---

**祝你学习顺利！🎉**

开始阅读你选择的学习路径，把握每一步的学习机会。3 个月后，你将成为 Agent、RAG、向量检索系统的专家！

---

**更新于：2026 年 5 月 19 日**  
**基于项目：LLMWiki v0.2.0**
