# 《LLM Wiki 驱动的智能双链知识库》— 完整开发手册

> **版本**: v2.1 (综合版) | **日期**: 2026-05-21 | **目标平台**: Windows 10/11
>
> 本手册整合了项目定位、产品需求、系统架构、前后端实现、Agent 管线、数据库设计、UI 规范、安全策略、多模型供应商支持、MarkItDown 通用转换层、OpenWebUI 扩展集成、安装打包与版本演进的全部维度，是 LLMWiki（智维 Wiki）项目的最高开发蓝图与唯一权威参考。

---

## 目录

1. [项目定位与核心方法论](#1-项目定位与核心方法论)
2. [产品边界与强制排除项](#2-产品边界与强制排除项)
3. [技术架构与选型](#3-技术架构与选型)
4. [系统架构与数据流](#4-系统架构与数据流)
5. [前端架构](#5-前端架构)
6. [后端架构 (Rust Core Runtime)](#6-后端架构-rust-core-runtime)
7. [数据库设计](#7-数据库设计)
8. [知识库目录结构](#8-知识库目录结构)
9. [多 Agent 运行时系统](#9-多-agent-运行时系统)
10. [任务状态机与恢复机制](#10-任务状态机与恢复机制)
11. [多模型供应商统一网关](#11-多模型供应商统一网关)
12. [Prompt 工程系统](#12-prompt-工程系统)
13. [JSON Schema 与输出校验](#13-json-schema-与输出校验)
14. [去重消歧系统](#14-去重消歧系统)
15. [关系建立与知识图谱](#15-关系建立与知识图谱)
16. [审阅与 Diff 系统](#16-审阅与-diff-系统)
17. [Wiki 写入与版本安全](#17-wiki-写入与版本安全)
18. [搜索系统](#18-搜索系统)
19. [问答与 Chat 系统](#19-问答与-chat-系统)
20. [健康检查与工作区修复](#20-健康检查与工作区修复)
21. [MarkItDown 通用格式转换系统](#21-markitdown-通用格式转换系统)
22. [文件处理管线](#22-文件处理管线)
23. [Source Preview 系统](#23-source-preview-系统)
24. [思维导图式知识图谱](#24-思维导图式知识图谱)
25. [UI/UX 设计规范](#25-uiux-设计规范)
26. [知识库模板系统](#26-知识库模板系统)
27. [安全与隐私](#27-安全与隐私)
28. [IPC 通信协议](#28-ipc-通信协议)
29. [OpenWebUI 集成与模型功能扩展](#29-openwebui-集成与模型功能扩展)
30. [本地嵌入与混合检索子系统 (可选进阶)](#30-本地嵌入与混合检索子系统-可选进阶)
31. [Windows 安装与打包](#31-windows-安装与打包)
32. [开发工作流](#32-开发工作流)
33. [版本历史与路线图](#33-版本历史与路线图)
34. [验收标准](#34-验收标准)
35. [附录：代码位置速查](#35-附录代码位置速查)

---

## 1. 项目定位与核心方法论

### 1.1 系统定义：基于大模型编译思维的自生长个人双链知识库

**LLMWiki（智维 Wiki）** 是一个 **本地优先的、大模型驱动的、多 Agent 协作式 Markdown Wiki 维护工作台**，本质上是一个由大模型底层驱动的自生长双链知识管理系统。

它打破了传统双链笔记（如 Obsidian）完全依赖人工手动打标签、建链接的"整理熵增"瓶颈，创新性地将软件工程中的"代码编译"思想平移到知识管理领域：

| 编译类比 | 对应实体 |
|----------|----------|
| 源代码 (Source Code) | 用户上传的异构素材 (PDF/Word/网页/PPT/Excel) |
| 知识编译器 (Knowledge Compiler) | 中央多 Agent 协同系统 (Extractor → Linker → Validator) |
| 生产就绪的代码库 (Codebase) | 本地互联双链 Markdown 集合 (长期知识资产) |

**用户不再承担繁重的归档压力，只需输入与审阅，由大模型在后台自动化编织个人的复杂知识网络。**

### 1.2 五步金字塔闭环研发路线

系统的核心控制流与数据流遵循以下五步金字塔闭环：

```
[异构多源素材输入]
       │
       ▼
1. MarkItDown 子进程层：统一"降维"格式化为标准化纯 Markdown 文本
       │
       ▼
2. 本地 Rust 推理与 RAG 层：0 Token 消耗计算离线特征向量，压入混合检索库
       │
       ▼
3. 云端多 Agent 编译层：精准召回局部上下文，多智能体协同生成结构化 Patch JSON 提案
       │
       ▼
4. 前端 Git-like 交互层：两栏高亮 Diff 对比视图呈现，由人类最终确认写盘
       │
       ▼
5. 高阶输出层：一键反向沉淀重新建库，或通过本地 Typst 引擎无损编译出工业级精美 PDF
```

### 1.3 核心业务闭环

```
上传资料
  → MarkItDown 统一格式转换 (PDF/DOCX/PPTX/XLSX/HTML → Markdown)
  → LLM 抽取结构化知识 (Source Ingest)
  → 多 Agent 消歧与关联 (Resolution + Relationship)
  → 生成 Wiki 更新计划 (Wiki Update Plan)
  → 用户审阅 Diff (Review Center)
  → 本地写入 Markdown Wiki (WikiWriter)
  → 版本快照 (VersionManager)
  → 索引与日志更新 (IndexService + LogService)
  → 基于 Wiki 问答 (QueryAgent)
  → 高价值回答沉淀为新 Wiki 页面
  → Wiki 持续演化
```

### 1.4 角色分工

| 角色 | 职责 |
|------|------|
| **用户** | 意图、上传、提问、审阅、确认（Human-in-the-Loop 最终决策者） |
| **Agent Runtime** | 任务拆解、流程编排、结构化协作、状态机管理 |
| **大模型 (多供应商)** | 文档理解、知识抽取、关系判断、内容生成（DeepSeek/OpenAI/Anthropic/...） |
| **本地 Rust 系统** | 存储、校验、审阅、写入、版本控制、安全、离线推理 |
| **MarkItDown 子进程** | 异构格式统一降维（Python sidecar，隔离运行） |

### 1.5 100% 本地数据主权原则

所有原始文件、计算出的向量、SQLite 数据库以及最终生成的 Markdown 笔记，全生命周期 **100% 留存在用户本地物理盘中**。系统在运行期间绝不设立任何云端多租户托管数据库，仅在需要大模型进行高阶推理或编译时，才将经由本地 RAG 精准裁剪后的局部上下文片段（Context Patches）通过 HTTPS 加密通道传输至大模型云端 API。数据在云端处于"内存态"，随请求结束即刻销毁，从物理根源上断绝数据泄露风险。

### 1.6 BYOK 多供应商统一网关

系统采用 **BYOK (Bring Your Own Key)** 模式。软件内部不捆绑任何商业化大模型资费套餐，前端提供统一的网关配置界面，由用户自行填入 **DeepSeek、OpenAI、Anthropic、Ollama 本地模型、或任何 OpenAI API 兼容服务** 的 Standard API Key。由于重度消耗算力的向量化与前置检索全部由本地离线分担，发送至云端的 Token 数量极度可控，从而将用户的知识库维护成本降至极限。

### 1.7 算力切分：本地执行离线嵌入与混合检索，云端执行多 Agent 深度推理

- **本地算力 (Tauri Rust + 离线模型)**：接管多格式前置转换（MarkItDown）、高频文本切片、Tokenizer 分词、ONNX 离线 Embedding 向量计算、以及 SQLite-vec 混合检索召回。
- **云端算力 (API 逻辑大脑)**：接管多 Agent 协同语义理解、概念提取（Extractor）、拓扑双链计算（Linker）与多范式结构化输出（如康奈尔/费曼笔记格式化）。

### 1.8 产品核心表达式

> 交互是入口；本地多 Agent Runtime 是流程；大模型是能力；Markdown Wiki 是长期知识资产。

> 从文档到 Wiki，从问答到知识资产。

> 将用户输入的非结构化异构素材视为"源代码"，将中央多 Agent 协同系统视为"知识编译器"，将最终沉淀在本地的互联双链 Markdown 集合视为"生产就绪的代码库"。

---

## 2. 产品边界与强制排除项

### 2.1 明确排除的功能

以下功能 **不在本项目的开发范围内**，禁止在任何版本中引入相关依赖或实现：

| 编号 | 排除项 | 原因 |
|------|--------|------|
| 1 | 本地 PDF 深度解析 | 复杂度极高，应由大模型处理 |
| 2 | 本地 OCR | 引入 Tesseract/PaddleOCR 等重型依赖 |
| 3 | 多模态图片理解 | 非核心路径 |
| 4 | 图片 caption | 不参与知识抽取 |
| 5 | PDF 页面切片 | 非核心 |
| 6 | 表格视觉识别 | 非核心 |
| 7 | 公式 OCR | 非核心 |
| 8 | LayoutLM/Nougat/MinerU/Marker 等依赖 | 避免依赖膨胀 |
| 9 | 传统向量 RAG 作为核心 | 第一阶段以全文搜索为主 |
| 10 | 自由 Agent 群聊 | Agent 之间必须通过 Coordinator 调度 |
| 11 | Agent/LLM 直接修改本地 Wiki | 所有写入必须通过 WikiWriter |
| 12 | Web 端部署 | 仅桌面端 |
| 13 | 移动端 | 仅桌面端 |
| 14 | 多人协作云端系统 | 本地单用户 |
| 15 | 复杂云同步 | 本地优先 |

### 2.2 核心路径红线

```
Prompt + 文档附件
  → DeepSeek API
  → 严格 JSON 结构化结果
  → 本地 JSON Schema 校验
  → Candidate Search / Resolution
  → Wiki Update Plan
  → Review Diff
  → 用户确认或低风险自动应用
  → WikiWriter 写入本地 Markdown Wiki
  → VersionManager 保存快照
  → IndexService / LogService 更新
```

**绝对不允走的捷径**：大模型直接写 Wiki、Agent 互相自由调用、跳过 JSON 校验、跳过审阅直接落盘。

---

## 3. 技术架构与选型

### 3.1 总体架构

```
┌──────────────────────────────────────────────────┐
│              React 18 + TypeScript                │
│   Tailwind CSS + Radix UI + Zustand               │
│   TanStack Query + React Router v6 + ReactFlow    │
├──────────────────────────────────────────────────┤
│              Tauri 2.x IPC Bridge                  │
│         (Commands + Events + Resources)            │
├──────────────────────────────────────────────────┤
│              Rust Core Runtime                     │
│  ┌──────────┬──────────┬──────────────────────┐  │
│  │ AppKernel│ TaskQueue│ AgentRuntime         │  │
│  ├──────────┼──────────┼──────────────────────┤  │
│  │ ModelGw  │ WikiWriter│ VersionManager       │  │
│  │(多供应商) │          │                      │  │
│  ├──────────┼──────────┼──────────────────────┤  │
│  │ ReviewEng│ DiffEng  │ GraphService         │  │
│  ├──────────┼──────────┼──────────────────────┤  │
│  │ SearchSvc│ EventBus │ MarkItDownManager    │  │
│  ├──────────┼──────────┼──────────────────────┤  │
│  │ SecretSvc│ ConfigSvc│ RecoveryCheck        │  │
│  └──────────┴──────────┴──────────────────────┘  │
├──────────────────────────────────────────────────┤
│          外部 API (多供应商)                       │
│  DeepSeek | OpenAI | Anthropic | Ollama | ...     │
│  OpenWebUI (可选聚合网关)                         │
├──────────────────────────────────────────────────┤
│          外部子进程                                │
│  Python + MarkItDown (异构格式转换)                │
├──────────────────────────────────────────────────┤
│              SQLite (rusqlite/bundled)             │
│              File System (workspace/)              │
└──────────────────────────────────────────────────┘
```

### 3.2 详细选型表

| 层级 | 技术 | 版本 | 用途 |
|------|------|------|------|
| 桌面框架 | Tauri | 2.x | 跨平台桌面壳，Rust 后端 + WebView 前端 |
| 前端框架 | React + TypeScript | 18+ | 单页应用 (SPA) |
| UI 组件 | Tailwind CSS + Radix UI | - | 原子化 CSS + 无样式组件库 |
| 状态管理 | Zustand | - | 轻量全局状态 |
| 数据请求 | TanStack Query | - | 服务端状态缓存与同步 |
| 路由 | React Router | v6 | 前端路由 |
| 图谱 | ReactFlow | - | 知识图谱/思维导图可视化 |
| 编辑器 | Lexical (规划) / MarkdownRenderer | - | 富文本双链编辑 + Markdown 渲染 |
| 后端语言 | Rust | stable | 核心运行时 |
| HTTP 客户端 | reqwest | - | 调用多供应商 LLM API |
| 数据库 | SQLite (rusqlite bundled) | - | 嵌入式关系数据库 |
| 异步运行时 | tokio | - | Rust 异步任务 |
| 序列化 | serde + serde_json | - | JSON 序列化/反序列化 |
| 密钥存储 | Windows Credential Manager | - | API Key 安全存储 |
| 格式转换 | MarkItDown (Python sidecar) | - | 异构文档统一转 Markdown |
| 离线推理 | ONNX Runtime (ort) | - | 本地 Embedding 向量计算（可选进阶） |
| 模型网关 | OpenWebUI (可选集成) | - | 多模型路由/聚合/负载均衡 |
| 安装包 | Tauri Bundler + NSIS/MSI | - | Windows 安装包生成 |

### 3.3 职责划分

**React UI 只负责**：页面渲染、用户交互、表单输入、状态展示、Chat 输入输出、审阅 Diff 展示、图谱展示、设置界面。

**Rust Core Runtime 负责**：AppKernel、Workspace 管理、SQLite 数据库、TaskQueue、AgentRuntime、Coordinator 调度、ModelGateway（多供应商调度）、各供应商 HTTP Client、MarkItDown 子进程管理、Prompt 构建与注册、JSON Schema 校验、Candidate Search、Resolution Engine、Relationship Engine、Review Engine、Diff Engine、WikiWriter、VersionManager、IndexService、LogService、GraphService、SearchService、EventBus、Recovery Check、Workspace Reconcile、文件安全读写、API Key 安全存储。

---

## 4. 系统架构与数据流

### 4.1 整体数据流

```
React UI
  → Tauri Commands / Events
  → Rust AppKernel
  → TaskQueue
  → CoordinatorAgent
  → Specialized Agents (SourceIngest → Resolution → Relationship → WikiUpdate → Review)
  → ModelGateway (多供应商路由)
  → Provider Clients (DeepSeekClient / OpenAIClient / AnthropicClient / OllamaClient / ...)
  → LLM API (DeepSeek / OpenAI / Anthropic / Ollama / OpenWebUI / ...)
  → JsonSchemaValidator
  → CandidateSearchEngine
  → ResolutionEngine
  → ReviewEngine
  → WikiWriter
  → VersionManager
  → Markdown Wiki + SQLite
```

### 4.2 单次文档导入的完整时序

```
1. 用户选择文件 → 前端 invoke("import_source", { kb_id, file_path })
2. Rust 保存文件到 raw/sources/documents/，计算 SHA256 hash
3. 生成 source_id，创建 source_ingest 任务
4. TaskQueue 锁定任务，Coordinator 调度 SourceIngestAgent
5. SourceIngestAgent:
   a. DocumentProcessor 分发文件到对应 Skill
   b. MarkItDown 子进程统一转换异构格式 → 纯净 Markdown
   c. PromptBuilder 构建 Ingest Prompt + 附件
   d. ModelGateway 根据用户配置路由到对应 LLM 供应商
   e. 接收 JSON，JsonSchemaValidator 校验
   f. 保存 ingest_result.json 到 .runtime/tasks/{task_id}/
6. Coordinator 调度 ResolutionAgent:
   a. CandidateSearchEngine 检索已有候选页面
   b. ResolutionAgent 判断 create_new/update_existing/add_alias/skip
   c. 保存 resolution_result.json
7. Coordinator 调度 RelationshipAgent:
   a. 标准化关系三元组 (source, relation, target)
   b. 映射到 canonical Wiki page
   c. 保存 relationship_result.json
8. Coordinator 调度 WikiUpdateAgent:
   a. 生成 wiki_update_plan (create/update/append/merge)
   b. 标记 risk_level 和 requires_review
   c. 保存 update_plan.json
9. Coordinator 调度 ReviewAgent:
   a. 生成 Review 和 Review Items
   b. 生成 Diff 数据
   c. 保存 review_items.json
10. 前端轮询/事件通知任务状态变更
11. 用户进入审阅中心 → 逐项审阅 Diff
12. 用户接受 → invoke("apply_review_item", { review_item_id })
13. WikiWriter:
    a. 获取页面锁
    b. 检查 base_version_hash
    c. 创建版本快照
    d. 写入 Markdown 文件 (原子 rename)
    e. 更新 SQLite (wiki_pages, knowledge_items, aliases, relationships, graph_nodes, graph_edges)
    f. 更新 index.md / log.md
    g. 释放页面锁
14. 前端刷新 Wiki 页面列表、图谱、Dashboard
```

### 4.3 前端-后端通信方式

**Tauri Commands** (请求-响应):
```typescript
import { invoke } from "@tauri-apps/api/core";
const kbs = await invoke<any[]>("list_knowledge_bases");
const result = await invoke<string>("run_query", { kbId, question, scope });
```

**Tauri Events** (推送通知):
```typescript
import { listen } from "@tauri-apps/api/event";
const unlisten = await listen<any>("kb-stats-changed", (event) => { ... });
const unlisten = await listen<any>("task-event", (event) => { ... });
```

---

## 5. 前端架构

### 5.1 目录结构

```
src/
  main.tsx                     # 入口: QueryClient + BrowserRouter
  App.tsx                      # 根组件: KB 初始化, 路由, 懒加载
  pages/                       # 页面组件 (lazy-loaded)
    DashboardPage.tsx          # 首页/知识库管理入口
    WikiPage.tsx               # Wiki 阅读/编辑/源码三模式
    SourcesPage.tsx            # Source 文件管理
    ImportTasksPage.tsx        # Agent Activity Timeline
    ReviewPage.tsx             # 审阅/Diff 工作台
    ChatPage.tsx               # 问答界面
    SearchPage.tsx             # 全文搜索
    GraphPage.tsx              # 思维导图/知识图谱
    SettingsPage.tsx           # 模型/知识库/Prompt/外观设置
    HealthCheckPage.tsx        # 健康检查与修复
    TaskDetailPage.tsx         # 任务详情/中间文件
    SourcePreviewPage.tsx      # 统一 Source 预览
    OnboardingPage.tsx         # 初始化向导
    FileBrowserPage.tsx        # 文件树浏览器
    EditorPage.tsx             # Markdown 编辑器
  components/
    layout/
      Sidebar.tsx              # 左侧导航栏 (可折叠)
      StatusBar.tsx            # 底部状态栏
    common/
      ErrorBoundary.tsx        # 错误边界
      MarkdownRenderer.tsx     # Markdown 渲染组件
      RightContextPanel.tsx    # 可复用右侧上下文面板
    graph/
      MindMapView.tsx          # ReactFlow 思维导图
    editor/
      MarkdownEditor.tsx       # Markdown 编辑组件
      TabBar.tsx               # 编辑器标签页
    contextpanel/
      ContextPanelSwitcher.tsx # 右侧面板内容切换
      OutlineView.tsx          # 大纲视图
      BacklinksPanel.tsx       # 反向链接面板
      LocalGraphView.tsx       # 局部图谱
    quicknav/
      CommandPalette.tsx       # 命令面板 (Ctrl+K)
      QuickSwitcher.tsx        # 快速切换器
      WikiLinkAutocomplete.tsx # Wiki 链接自动补全
      GlobalKeyboardHandler.tsx # 全局键盘快捷键
    filebrowser/
      FileTree.tsx             # 文件树组件
      FileTreeHeader.tsx       # 文件树头部
      ImportFolderDialog.tsx   # 导入文件夹对话框
  stores/                      # Zustand 状态管理
    useKBStore.ts              # 知识库列表 + 当前 KB
    useModelStore.ts           # LLM 模型配置
    useAppStore.ts             # 应用全局状态 (侧栏等)
    useContextPanelStore.ts    # 右侧面板状态
    useEditorStore.ts          # 编辑器状态
    useFileTreeStore.ts        # 文件树状态
    useQuickNavStore.ts        # 快速导航状态
    useThemeStore.ts           # 主题状态
  types/                       # TypeScript 接口
    kb.ts                      # KnowledgeBase
    wiki.ts                    # WikiPage, KnowledgeItem
    task.ts                    # Task, TaskEvent
    source.ts                  # Source, Asset
    review.ts                  # Review, ReviewItem
    graph.ts                   # GraphNode, GraphEdge
    chat.ts                    # ChatMessage, Conversation
    model.ts                   # ModelProfile, DeepSeekConfig
    webSearch.ts               # WebSearchResult
  lib/
    utils.ts                   # cn(), formatSize(), formatDateTime(), formatDate()
```

### 5.2 页面路由

| 路由 | 组件 | 说明 |
|------|------|------|
| `/` | DashboardPage | 首页/知识库管理 |
| `/wiki/:pageId?` | WikiPage | Wiki 页面阅读/编辑 |
| `/sources` | SourcesPage | Source 文件管理 |
| `/import-tasks` | ImportTasksPage | 导入任务/Agent Timeline |
| `/review` | ReviewPage | 审阅/Diff 工作台 |
| `/chat` | ChatPage | 问答界面 |
| `/search` | SearchPage | 全文搜索 |
| `/graph` | GraphPage | 思维导图/知识图谱 |
| `/settings` | SettingsPage | 设置 |
| `/health` | HealthCheckPage | 健康检查 |
| `/task/:taskId` | TaskDetailPage | 任务详情 |
| `/source-preview/:sourceId` | SourcePreviewPage | Source 预览 |
| `/files` | FileBrowserPage | 文件树浏览器 |
| `/onboarding` | OnboardingPage | 初始化向导 |
| `/editor/:pageId?` | EditorPage | Markdown 编辑器 |

### 5.3 全局状态 (Zustand Stores)

**useKBStore**:
```typescript
interface KBStore {
  kbs: KnowledgeBase[];
  currentKB: KnowledgeBase | null;
  setKBs: (kbs: KnowledgeBase[]) => void;
  setCurrentKB: (kb: KnowledgeBase) => void;
  refresh: () => Promise<void>;
}
```

**useModelStore**:
```typescript
interface ModelStore {
  config: DeepSeekConfig | null;
  setConfig: (config: DeepSeekConfig) => void;
  testConnection: () => Promise<boolean>;
  testJsonOutput: () => Promise<boolean>;
  testAttachment: () => Promise<boolean>;
}
```

### 5.4 设计原则

- 所有数据请求经过 TanStack Query，避免手动管理 loading/error 状态
- 页面级组件按路由懒加载 (React.lazy + Suspense)
- 所有用户可见文案使用中文
- 所有 Tauri invoke 调用封装在 pages 或 stores 内，不分散在组件中
- 错误边界 (ErrorBoundary) 包裹每个页面

---

## 6. 后端架构 (Rust Core Runtime)

### 6.1 模块结构

```
src-tauri/src/
  lib.rs                          # Tauri builder, 注册 commands + plugins + state
  main.rs                         # 入口
  core/
    mod.rs
    app_kernel.rs                 # DI 容器: 持有所有 Service 的 Arc
    config_service.rs             # 配置读写 (kb.config.json, app settings)
    secret_service.rs             # API Key 安全存储 (Windows Credential Manager)
    workspace_service.rs          # 知识库目录初始化与管理
    database_service.rs           # SQLite 连接池与管理
    task_queue.rs                 # 任务队列 + CancellationToken 支持
    event_bus.rs                  # 前端事件推送 (Tauri events)
    file_tree_service.rs          # workspace 文件树扫描
    source_preview_service.rs     # Source 文件转 Markdown 预览
    workspace_file_preview_service.rs # 文件预览服务
  db/
    mod.rs
    schema.rs                     # DDL 定义 + 迁移
    migrations.rs                 # 迁移执行器
  commands/                       # Tauri #[command] handlers
    mod.rs
    workspace.rs                  # 知识库 CRUD
    config.rs                     # 模型配置/设置
    source.rs                     # 文件上传/管理
    task.rs                       # 任务管理/查询
    review.rs                     # 审阅管理
    wiki.rs                       # Wiki 页面 CRUD
    chat_history.rs               # 对话历史
    web_search.rs                 # 联网搜索
    search.rs                     # 本地搜索
    graph.rs                      # 图谱数据
    file_tree.rs                  # 文件树
    source_preview.rs             # Source Preview
    utils.rs                      # 工具命令
  agents/
    mod.rs
    coordinator.rs                # 调度器: 判断任务类型、拆解、调度 Agent
    source_ingest.rs              # 文档摄入: 解析 + AI 分析
    resolution.rs                 # 去重消歧
    relationship.rs               # 关系建立
    wiki_update.rs                # Wiki 更新计划生成
    health_check.rs               # 健康检查
  model/
    mod.rs
    model_gateway.rs              # 统一 LLM 调用入口
    deepseek_client.rs            # DeepSeek HTTP 客户端
  prompts/
    mod.rs
    prompt_registry.rs            # Prompt 模板注册
    prompt_builder.rs             # Prompt 动态构建
  skills/                         # 文档处理技能
    mod.rs
    document_processor.rs         # 文档分发器
    pdf_skill.rs                  # PDF 文本提取 (lopdf)
    pdf_ocr.rs                    # Windows OCR 回退 (扫描件 PDF)
    docx_skill.rs                 # DOCX 文本提取
    html_skill.rs                 # HTML → Markdown
    md_skill.rs                   # Markdown 读取
    txt_skill.rs                  # TXT 读取
    pptx_skill.rs                 # PPTX 文本提取
    markitdown_skill.rs           # MarkItDown 统一转换
    web_search_skill.rs           # 联网搜索
  wiki/
    mod.rs
    wiki_writer.rs                # Wiki 统一写入 (页面锁 + 原子写)
    version_manager.rs            # 版本快照与回滚
    index_service.rs              # index.md 维护
    log_service.rs                # log.md 维护
    path_service.rs               # 路径规范化 (slugify, normalize, resolve)
    markdown_indexer.rs           # Markdown 索引
  review/
    mod.rs
    review_engine.rs              # 审阅引擎 (生成/分组/批量操作)
    diff_engine.rs                # Diff 生成 (行级比对)
  graph/
    mod.rs
    graph_service.rs              # 图谱节点/边 CRUD + 重建
  search/
    mod.rs
    full_text_search.rs           # 全文搜索 (标题/正文/alias)
    candidate_search.rs           # 候选页面检索 (消歧用)
  schema/
    mod.rs
    json_schema_validator.rs      # JSON Schema 校验
    json_repair.rs                # JSON 修复
  recovery/
    mod.rs
    recovery_check.rs             # 启动恢复检查
    workspace_reconcile.rs        # 工作区一致性修复
    operations_service.rs         # 操作幂等性检查
  dedup/
    mod.rs
    dedup_service.rs              # 知识项去重服务
```

### 6.2 AppKernel (DI 容器)

```rust
pub struct AppKernel {
    pub db: Arc<DatabaseService>,
    pub config: Arc<ConfigService>,
    pub secrets: Arc<SecretService>,
    pub workspace: Arc<WorkspaceService>,
    pub event_bus: Arc<EventBus>,
    pub task_queue: Arc<TaskQueue>,
    pub model_gateway: Arc<ModelGateway>,
    // ... other services
}
```

在 `lib.rs` 的 `setup` 阶段创建 AppKernel，注入为 Tauri managed state。所有 command handler 通过 `State<'_, Arc<AppKernel>>` 获取。

### 6.3 关键服务说明

| 服务 | 职责 | 关键方法 |
|------|------|----------|
| DatabaseService | SQLite 连接管理，按 kb_id 获取连接 | `connect(kb_id)`, `connect_app_db()` |
| ConfigService | 应用配置 + kb 配置读写 | `get_deepseek_config()`, `get_kb_config(kb_id)` |
| SecretService | API Key 存取 (Windows Credential Manager) | `store_api_key()`, `get_api_key()` |
| WorkspaceService | KB 目录初始化 + 路径解析 | `init_workspace()`, `resolve_path()` |
| TaskQueue | 任务创建/锁定/状态更新/取消 | `create_task()`, `lock_task()`, `update_status()` |
| EventBus | 前端事件推送 | `emit()`, `emit_notification()` |
| ModelGateway | 统一 LLM 调用 | `chat()`, `chat_with_content()`, `chat_with_attachment()` |
| WikiWriter | Wiki 安全写入 | `create_page()`, `update_page()`, `apply_operation()` |
| VersionManager | 版本快照 | `create_snapshot()`, `rollback()`, `list_versions()` |
| PathService | 路径规范化 | `slugify()`, `normalize_wiki_path()`, `resolve_wiki_path()` |

---

## 7. 数据库设计

### 7.1 数据库位置

- **应用数据库**: `%APPDATA%/LLM Knowledge Wiki/app.sqlite` — 存储 KB 列表、全局配置
- **知识库数据库**: `{workspace}/db/app.sqlite` (实际文件名为 `llmwiki.db`) — 存储单个 KB 的所有数据

### 7.2 核心表结构

```sql
-- 知识库
CREATE TABLE knowledge_bases (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    path TEXT NOT NULL,
    template_name TEXT DEFAULT 'general',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- 原始文件
CREATE TABLE sources (
    id TEXT PRIMARY KEY,
    kb_id TEXT NOT NULL,
    file_name TEXT NOT NULL,
    file_path TEXT NOT NULL,
    file_type TEXT NOT NULL,           -- pdf, docx, md, txt, html, png, jpg, ...
    file_size INTEGER NOT NULL,
    file_hash TEXT NOT NULL,           -- SHA256
    status TEXT DEFAULT 'saved',       -- saved, analyzing, analyzed, review_pending, applied, failed, asset_only
    preview_path TEXT,                 -- .runtime/source_previews/{source_id}.md
    preview_status TEXT,               -- generated, ai_summary_only, unavailable, failed, asset_only
    summary_json_path TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (kb_id) REFERENCES knowledge_bases(id)
);

-- 图片资产
CREATE TABLE assets (
    id TEXT PRIMARY KEY,
    kb_id TEXT NOT NULL,
    file_name TEXT NOT NULL,
    file_path TEXT NOT NULL,
    file_type TEXT NOT NULL,
    file_size INTEGER NOT NULL,
    file_hash TEXT NOT NULL,
    created_at TEXT NOT NULL,
    FOREIGN KEY (kb_id) REFERENCES knowledge_bases(id)
);

-- Wiki 页面
CREATE TABLE wiki_pages (
    id TEXT PRIMARY KEY,
    kb_id TEXT NOT NULL,
    title TEXT NOT NULL,
    path TEXT NOT NULL,                -- 相对 workspace 路径, e.g. wiki/concepts/self-attention.md
    page_type TEXT NOT NULL,           -- concept, entity, topic, source, question, dataset, method
    canonical_name TEXT,
    tags TEXT,                         -- JSON array
    content_hash TEXT,
    status TEXT DEFAULT 'active',      -- active, broken, deprecated
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (kb_id) REFERENCES knowledge_bases(id)
);

-- 知识项 (Entity/Concept/Topic)
CREATE TABLE knowledge_items (
    id TEXT PRIMARY KEY,
    kb_id TEXT NOT NULL,
    canonical_name TEXT NOT NULL,
    item_type TEXT NOT NULL,           -- entity, concept, topic
    page_path TEXT,                    -- 关联的 wiki_pages.path
    summary TEXT,
    confidence TEXT DEFAULT 'medium',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (kb_id) REFERENCES knowledge_bases(id)
);

-- 别名
CREATE TABLE aliases (
    id TEXT PRIMARY KEY,
    item_id TEXT NOT NULL,
    alias TEXT NOT NULL,
    normalized_alias TEXT NOT NULL,
    language TEXT,                     -- zh, en, ...
    created_at TEXT NOT NULL,
    FOREIGN KEY (item_id) REFERENCES knowledge_items(id)
);

-- 关系
CREATE TABLE relationships (
    id TEXT PRIMARY KEY,
    kb_id TEXT NOT NULL,
    source_item_id TEXT NOT NULL,
    target_item_id TEXT NOT NULL,
    relation TEXT NOT NULL,            -- is_a, part_of, uses, depends_on, cites, ...
    evidence_source_id TEXT,
    evidence_location TEXT,
    confidence TEXT DEFAULT 'medium',  -- high, medium, low
    status TEXT DEFAULT 'active',
    created_at TEXT NOT NULL,
    FOREIGN KEY (kb_id) REFERENCES knowledge_bases(id)
);

-- 任务
CREATE TABLE tasks (
    id TEXT PRIMARY KEY,
    kb_id TEXT NOT NULL,
    task_type TEXT NOT NULL,           -- source_ingest, resolution, relationship, wiki_update, query, health_check
    status TEXT DEFAULT 'created',     -- created, queued, locked, prompt_built, ..., applied, failed, cancelled
    current_agent TEXT,
    model_profile_id TEXT,
    input_ref TEXT,                    -- 关联 source_id 或自定义输入
    output_ref TEXT,
    error_message TEXT,
    retry_count INTEGER DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    locked_at TEXT,
    completed_at TEXT,
    FOREIGN KEY (kb_id) REFERENCES knowledge_bases(id)
);

-- 任务事件
CREATE TABLE task_events (
    id TEXT PRIMARY KEY,
    task_id TEXT NOT NULL,
    event_type TEXT NOT NULL,
    agent_name TEXT,
    message TEXT,
    created_at TEXT NOT NULL,
    FOREIGN KEY (task_id) REFERENCES tasks(id)
);

-- 审阅
CREATE TABLE reviews (
    id TEXT PRIMARY KEY,
    kb_id TEXT NOT NULL,
    task_id TEXT NOT NULL,
    status TEXT DEFAULT 'pending',     -- pending, in_progress, completed
    summary TEXT,
    risk_level TEXT DEFAULT 'medium',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (kb_id) REFERENCES knowledge_bases(id),
    FOREIGN KEY (task_id) REFERENCES tasks(id)
);

-- 审阅项
CREATE TABLE review_items (
    id TEXT PRIMARY KEY,
    review_id TEXT NOT NULL,
    operation TEXT NOT NULL,           -- create, update, append, add_alias, add_relation, merge_suggestion
    target_path TEXT,
    target_title TEXT,
    page_type TEXT,
    base_version_hash TEXT,
    old_content TEXT,
    new_content TEXT,
    summary TEXT,
    reason TEXT,
    source_id TEXT,
    location TEXT,
    citation_status TEXT DEFAULT 'model_reported',
    confidence TEXT DEFAULT 'medium',
    risk_level TEXT DEFAULT 'medium',
    status TEXT DEFAULT 'pending',     -- pending, accepted, rejected, applied, apply_failed, edited
    requires_review INTEGER DEFAULT 1,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (review_id) REFERENCES reviews(id)
);

-- 版本快照
CREATE TABLE versions (
    id TEXT PRIMARY KEY,
    kb_id TEXT NOT NULL,
    page_path TEXT NOT NULL,
    content_hash TEXT NOT NULL,
    snapshot_path TEXT NOT NULL,       -- 快照文件路径
    task_id TEXT,
    operation_id TEXT,
    created_at TEXT NOT NULL,
    FOREIGN KEY (kb_id) REFERENCES knowledge_bases(id)
);

-- 操作记录 (幂等性保证)
CREATE TABLE operations (
    id TEXT PRIMARY KEY,
    kb_id TEXT NOT NULL,
    task_id TEXT NOT NULL,
    operation_hash TEXT NOT NULL UNIQUE,
    target_path TEXT NOT NULL,
    status TEXT DEFAULT 'pending',     -- pending, applied, failed
    applied_at TEXT,
    FOREIGN KEY (kb_id) REFERENCES knowledge_bases(id)
);

-- 图谱节点
CREATE TABLE graph_nodes (
    id TEXT PRIMARY KEY,
    kb_id TEXT NOT NULL,
    node_type TEXT NOT NULL,           -- source, concept, entity, topic, wiki_page, question, dataset, method
    label TEXT NOT NULL,
    path TEXT,                         -- 关联的文件路径
    source_id TEXT,                    -- 关联的 source
    page_id TEXT,                      -- 关联的 wiki_page
    confidence TEXT DEFAULT 'medium',
    status TEXT DEFAULT 'active',
    metadata TEXT,                     -- JSON
    FOREIGN KEY (kb_id) REFERENCES knowledge_bases(id)
);

-- 图谱边
CREATE TABLE graph_edges (
    id TEXT PRIMARY KEY,
    kb_id TEXT NOT NULL,
    source_node_id TEXT NOT NULL,
    target_node_id TEXT NOT NULL,
    edge_type TEXT NOT NULL,           -- is_a, part_of, uses, cites, mentions, related_to, ...
    confidence TEXT DEFAULT 'medium',
    evidence TEXT,
    metadata TEXT,                     -- JSON
    FOREIGN KEY (kb_id) REFERENCES knowledge_bases(id),
    FOREIGN KEY (source_node_id) REFERENCES graph_nodes(id),
    FOREIGN KEY (target_node_id) REFERENCES graph_nodes(id)
);

-- 模型配置
CREATE TABLE model_profiles (
    id TEXT PRIMARY KEY,
    provider TEXT NOT NULL DEFAULT 'deepseek',
    name TEXT NOT NULL,
    base_url TEXT NOT NULL,
    model_name TEXT NOT NULL,
    encrypted_api_key_ref TEXT NOT NULL,
    role TEXT DEFAULT 'chat',          -- chat, reasoner, writer, conflict_checker
    temperature REAL DEFAULT 0.7,
    max_tokens INTEGER DEFAULT 4096,
    timeout INTEGER DEFAULT 120,
    retry_count INTEGER DEFAULT 3,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- 对话历史
CREATE TABLE conversations (
    id TEXT PRIMARY KEY,
    kb_id TEXT NOT NULL,
    scope TEXT DEFAULT 'kb',           -- kb, page, file, tag, directory
    scope_target TEXT,
    messages TEXT NOT NULL,            -- JSON array
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- 迁移记录
CREATE TABLE schema_migrations (
    version INTEGER PRIMARY KEY,
    applied_at TEXT NOT NULL
);
```

### 7.3 数据库迁移

- 所有迁移在 `db/migrations.rs` 中定义，按版本号顺序执行
- 迁移前自动备份数据库文件
- 迁移版本号记录在 `schema_migrations` 表中
- 新增迁移时，递增版本号，添加迁移 SQL

---

## 8. 知识库目录结构

### 8.1 标准 Workspace 布局

```
workspace/
  raw/                              # 原始文件 (事实来源, 只读)
    sources/
      documents/                    # PDF/DOCX/MD/TXT/HTML
      webclips/                     # 网页抓取内容
    assets/
      images/                       # PNG/JPG/WEBP/GIF
      attachments/                  # 其他附件
  wiki/                             # Markdown Wiki (长期知识资产)
    index.md                        # 全局索引 (自动维护)
    log.md                          # 操作日志 (自动追加)
    overview.md                     # 知识库概览
    concepts/                       # 概念页面
    entities/                       # 实体页面 (人物/组织/产品/...)
    topics/                         # 主题页面
    methods/                        # 方法页面
    datasets/                       # 数据集页面
    sources/                        # 来源摘要页面
    questions/                      # 问答沉淀页面
    reviews/                        # 审阅记录页面
  schema/                           # 知识库模板与规则
    TEMPLATE.yaml                   # 模板定义
    AGENTS.md                       # Agent 规则
    WIKI_SCHEMA.md                  # Wiki 页面规范
    PROMPTS.md                      # Prompt 规则
  drafts/                           # 待审阅草稿
    ingest/                         # 文档抽取草稿
    wiki_updates/                   # Wiki 更新草稿
  versions/                         # 版本快照
    pages/                          # 页面历史版本
    snapshots/                      # 全局快照
  .runtime/                         # 运行时中间产物
    tasks/{task_id}/                # 每个任务的中间文件
      input.json                    # 任务输入
      prompt.md                     # 构建的 Prompt
      model_raw_response.txt        # 模型原始返回
      ingest_result.json            # 文档分析结果
      resolution_result.json        # 消歧结果
      relationship_result.json      # 关系提取结果
      update_plan.json              # 更新计划
      review_items.json             # 审阅项
      error.log                     # 错误日志
    logs/                           # 运行时日志
    source_previews/{source_id}.md  # Source 预览缓存
  db/
    llmwiki.db                      # SQLite 数据库
  config/
    kb.config.json                  # 知识库配置
```

### 8.2 目录权限与规则

- `raw/` — 原始文件，AI 不可修改，是事实来源
- `wiki/` — AI 生成 + 用户确认后的 Markdown Wiki，是长期知识资产
- `schema/` — 模板与规则文件
- `drafts/` — 待审阅草稿，不对外展示
- `versions/` — 版本快照，用于回滚
- `.runtime/` — 任务中间产物，可清理
- `db/` — 数据库文件
- `config/` — 知识库配置

---

## 9. 多 Agent 运行时系统

### 9.1 Agent 定义

```
Agent = Role + Prompt + Tools + Context + Input Schema + Output Schema + State
```

Agent 是任务处理模块，具有明确职责、固定输入输出协议和受控工具权限。

**Agent 不是自由聊天机器人。Agent 不允许自由互相调用。Agent 不允许直接修改本地 Wiki。**

### 9.2 Agent 列表

| Agent | 职责 | 输入 | 输出 |
|-------|------|------|------|
| **CoordinatorAgent** | 判断任务类型 → 拆解 → 调度 → 收集 → 决定下一步 | Task | next_actions, task_events |
| **SourceIngestAgent** | 构建 Ingest Prompt + 附件提交 DeepSeek → 抽取知识 | Source 文件 | source_summary, entities, concepts, topics, claims, relationships |
| **ResolutionAgent** | 判断新抽取项与已有页面是否重复 → 消歧 | 新知识项 + 候选页面 | resolutions (create_new/update_existing/add_alias/skip/...) |
| **RelationshipAgent** | 标准化关系三元组 → 映射 canonical page | 实体/概念列表 | standardized relationships |
| **WikiUpdateAgent** | 生成 Wiki 更新计划 (不写文件) | 消歧知识 + 关系 | wiki_update_plan (create/update/append/merge) |
| **ReviewAgent** | 生成 Review + Review Items + Diff | wiki_update_plan | review + review_items + diff |
| **QueryAgent** | 基于知识库回答 → 带引用 | question + scope | answer + citations |
| **HealthCheckAgent** | 检查一致性问题 → 生成报告 (不修改) | kb_id | health report |
| **IndexService** | 更新 index.md | kb state | updated index.md |
| **LogService** | 追加 log.md | event | updated log.md |

### 9.3 Agent 通信协议

Agent 之间不直接通信。所有通信通过 Coordinator 调度。

```json
{
  "message_id": "uuid",
  "task_id": "task_xxx",
  "from_agent": "SourceIngestAgent",
  "to_agent": "CoordinatorAgent",
  "message_type": "SOURCE_INGEST_RESULT",
  "payload": { ... },
  "created_at": "2026-05-21T..."
}
```

**message_type 枚举**: SOURCE_INGEST_REQUEST, SOURCE_INGEST_RESULT, RESOLUTION_REQUEST, RESOLUTION_RESULT, RELATIONSHIP_REQUEST, RELATIONSHIP_RESULT, WIKI_UPDATE_REQUEST, WIKI_UPDATE_PLAN, REVIEW_REQUEST, REVIEW_CREATED, QUERY_REQUEST, QUERY_RESULT, HEALTH_CHECK_REQUEST, HEALTH_CHECK_REPORT, ERROR

### 9.4 Coordinator 调度逻辑

```rust
impl CoordinatorAgent {
    pub async fn run_pipeline(&self, kb_id: &str, task_id: &str, cancel_token: &CancellationToken) {
        // Phase 1: Source Ingest
        self.run_stage(task_id, "source_ingest", || async {
            let agent = SourceIngestAgent::new(...);
            agent.execute(kb_id, source_id, task_id, cancel_token).await
        }).await?;

        if cancel_token.is_cancelled() { return; }

        // Phase 2: Resolution
        self.run_stage(task_id, "resolution", || async {
            let agent = ResolutionAgent::new(...);
            agent.execute(kb_id, task_id, cancel_token).await
        }).await?;

        if cancel_token.is_cancelled() { return; }

        // Phase 3: Relationship
        self.run_stage(task_id, "relationship", || async {
            let agent = RelationshipAgent::new(...);
            agent.execute(kb_id, task_id, cancel_token).await
        }).await?;

        if cancel_token.is_cancelled() { return; }

        // Phase 4: Wiki Update
        self.run_stage(task_id, "wiki_update", || async {
            let agent = WikiUpdateAgent::new(...);
            agent.execute(kb_id, task_id, cancel_token).await
        }).await?;

        // Phase 5: Review
        self.run_stage(task_id, "review", || async {
            let agent = ReviewAgent::new(...);
            agent.execute(kb_id, task_id).await
        }).await?;
    }
}
```

---

## 10. 任务状态机与恢复机制

### 10.1 任务状态枚举

```
created → queued → locked → prompt_built → sent_to_model
  → model_returned → json_validating → json_valid | json_invalid
  → json_repairing → json_repaired
  → candidate_searching → resolution_running → relationship_running
  → update_plan_generating → review_generating → review_pending
  → applying → applied
  → failed | cancelled | interrupted
```

### 10.2 任务持久化

每个任务必须保存到 SQLite 的 `tasks` 表和文件系统：

```
.runtime/tasks/{task_id}/
  input.json                 # 任务输入参数
  prompt.md                  # 构建的完整 Prompt
  model_raw_response.txt     # 模型原始返回
  ingest_result.json         # SourceIngest 输出
  resolution_result.json     # Resolution 输出
  relationship_result.json   # Relationship 输出
  update_plan.json           # WikiUpdate 输出
  review_items.json          # Review 输出
  error.log                  # 错误日志
```

### 10.3 启动恢复检查 (RecoveryCheck)

软件启动时必须扫描所有 KB 的任务：

1. 扫描 `interrupted` / `queued` / `locked` / `applying` 状态的任务
2. 对模型调用中断的任务标记为 `interrupted`，允许用户重试
3. 对 `review_pending` 任务恢复正常审阅
4. 对 `applying` 中断任务检查 `operation_id` 和版本快照，避免重复写入
5. 允许用户：继续、重试、取消、回滚

### 10.4 任务取消 (CancellationToken)

所有 Agent 执行循环中检查 `cancel_token.is_cancelled()`，支持用户随时取消长时间运行的任务。取消后任务状态设为 `cancelled`，已产生的中间文件保留。

---

## 11. 多模型供应商统一网关

### 11.1 设计理念

LLMWiki 不绑定任何单一模型供应商。系统采用 **BYOK (Bring Your Own Key)** 模式，用户可自由选择 DeepSeek、OpenAI、Anthropic、Ollama 本地模型、或任何兼容 OpenAI API 格式的服务（包括 OpenWebUI 作为聚合网关）。

所有 LLM 调用必须经过统一的 `ModelGateway`，Agent 不感知底层供应商差异。

### 11.2 支持的模型供应商

| 供应商 | Base URL (默认) | 关键模型 | 特点 |
|--------|-----------------|----------|------|
| **DeepSeek** | `https://api.deepseek.com` | deepseek-chat, deepseek-reasoner | 高性价比推理，中文优异，支持文档附件 |
| **OpenAI** | `https://api.openai.com/v1` | gpt-4o, gpt-4o-mini, o1, o3 | 综合能力最强，生态成熟，JSON 模式稳定 |
| **Anthropic** | `https://api.anthropic.com/v1` | claude-sonnet-4-6, claude-opus-4-7, claude-haiku-4-5 | 深度推理能力极强，长文档处理出色 |
| **Ollama (本地)** | `http://localhost:11434` | llama3, qwen3, mistral, gemma3, deepseek-r1 | 完全离线，零 Token 成本，隐私最高 |
| **OpenAI 兼容 API** | 用户自定义 | 任意兼容 `/v1/chat/completions` 的模型 | 极致的供应商灵活性，覆盖国产模型 (Qwen/Moonshot/GLM/...) |
| **OpenWebUI (聚合)** | 用户自定义 | 通过 OpenWebUI 管道路由到多个后端 | 统一管理多模型、负载均衡、Pipeline 预处理 |

### 11.3 模型供应商抽象层

```rust
/// 模型供应商统一特征
pub trait ModelProvider: Send + Sync {
    /// 供应商标识
    fn provider_name(&self) -> &'static str;

    /// 构建聊天请求 URL
    fn build_chat_url(&self, base_url: &str) -> String;

    /// 构建请求头 (Authorization, x-api-key, etc.)
    fn build_headers(&self, api_key: &str) -> Vec<(String, String)>;

    /// 构建请求体 (兼容不同供应商的 JSON 格式差异)
    fn build_request_body(
        &self,
        model: &str,
        messages: &[ChatMessage],
        temperature: f64,
        max_tokens: u32,
        use_json_mode: bool,
    ) -> serde_json::Value;

    /// 从供应商响应中提取文本内容
    fn extract_content(&self, response: &serde_json::Value) -> Result<String, String>;

    /// 从供应商响应中提取 token 用量
    fn extract_usage(&self, response: &serde_json::Value) -> Option<UsageInfo>;

    /// 从供应商响应中提取 finish_reason
    fn extract_finish_reason(&self, response: &serde_json::Value) -> Option<String>;

    /// 供应商特定的错误归一化
    fn normalize_error(&self, status_code: u16, error_body: &str) -> String;
}
```

### 11.4 已实现的供应商客户端

| 客户端 | 文件位置 | 说明 |
|--------|----------|------|
| `DeepSeekClient` | [src-tauri/src/model/deepseek_client.rs](src-tauri/src/model/deepseek_client.rs) | 当前主用客户端，支持 `/v1/chat/completions` + 文档附件 |
| `OpenAIClient` | 规划中 | 标准 OpenAI API 格式，支持 JSON mode、streaming |
| `AnthropicClient` | 规划中 | Anthropic Messages API，支持超长文档 200K context |
| `OllamaClient` | 规划中 | 本地 Ollama 服务，支持 `/api/chat`，零延迟本地推理 |

### 11.5 ModelGateway 统一调度

```
ModelGateway 负责:
  1. 供应商路由 (根据 model_profile.provider 选择对应 Client)
  2. API Key 注入 (根据 provider 从 SecretService 读取对应密钥)
  3. Base URL 管理 (每个 provider 独立配置)
  4. 模型选择 (chat / reasoner / writer / conflict_checker 按任务类型路由)
  5. 请求/响应格式适配 (不同供应商的 JSON 结构差异)
  6. 超时控制
  7. 自动重试 (指数退避，跨供应商通用)
  8. 限流处理 (429 响应，各供应商差异化等待策略)
  9. 错误归一化 (用户可读中文错误，按供应商差异化处理)
  10. 流式输出适配 (不同供应商 SSE 格式归一化)
  11. 文档附件处理 (供应商特定实现)
  12. 原始响应保存 (.runtime/tasks/{task_id}/)
  13. JSON 修复调度
  14. Token 统计 (使用供应商返回的 usage 信息)
```

### 11.6 模型角色与供应商推荐

| 角色 | 推荐供应商 | 推荐模型 | 用途 |
|------|-----------|----------|------|
| **chat** | DeepSeek / OpenAI | deepseek-chat / gpt-4o-mini | 普通问答、Wiki 写作 |
| **reasoner** | DeepSeek / Anthropic | deepseek-reasoner / claude-sonnet-4-6 | 长文档分析、冲突检测、复杂推理 |
| **writer** | Anthropic / OpenAI | claude-sonnet-4-6 / gpt-4o | Wiki 页面高质量写作 |
| **conflict_checker** | Anthropic / DeepSeek | claude-opus-4-7 / deepseek-reasoner | 高精度冲突检测 |
| **local** | Ollama | qwen3 / llama3 / deepseek-r1 | 完全离线场景，隐私敏感内容 |

### 11.7 用户配置界面

前端 Settings 页面提供多供应商配置：

- **供应商选择**：下拉选择 DeepSeek / OpenAI / Anthropic / Ollama / OpenAI 兼容 / 自定义
- **Base URL**：可编辑文本框（预填默认值）
- **API Key**：密码输入框，存储到 Windows Credential Manager（按 provider 隔离）
- **模型名称**：chat 模型 / reasoner 模型 / writer 模型分别配置
- **高级参数**：temperature, max_tokens, timeout, retry_count
- **连接测试**：一键测试当前供应商连通性
- **JSON 输出测试**：验证模型结构化输出能力
- **文档附件测试**：验证模型文档理解能力

### 11.8 模型配置数据模型

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelProviderConfig {
    /// 供应商标识: "deepseek" | "openai" | "anthropic" | "ollama" | "openai_compatible"
    pub provider: String,
    /// API Base URL
    pub base_url: String,
    /// Chat 模型名称 (普通问答、Wiki 写作)
    pub chat_model: String,
    /// 推理模型名称 (长文档分析、冲突检测)
    pub reasoner_model: String,
    /// 写作模型名称 (高质量 Wiki 页面写作)
    pub writer_model: Option<String>,
    /// 冲突检测模型名称
    pub conflict_checker_model: Option<String>,
    /// 温度参数
    pub temperature: f64,
    /// 最大输出 Token
    pub max_tokens: u32,
    /// 请求超时 (秒)
    pub timeout: u32,
    /// 最大重试次数
    pub retry_count: u32,
    /// 是否启用流式输出
    pub stream: bool,
}
```

### 11.9 错误处理 (跨供应商通用)

必须处理的错误类型（所有供应商共享，但错误消息按供应商差异化）：
- `timeout` → 重试，提示"模型响应超时，正在重试..."
- `429 (Rate Limit)` → 指数退避重试，提示"请求过于频繁，等待 X 秒后重试..."
- `5xx (Server Error)` → 重试，提示"{provider} 服务异常，正在重试..."
- `余额不足` → 不重试，提示"API 余额不足，请充值后重试"
- `API Key 无效` → 不重试，提示"API Key 无效，请检查 {provider} 设置"
- `附件过大` → 不重试，提示"文件过大 (X MB)，超过模型限制"
- `上下文过长` → 不重试，提示"上下文超过模型限制 ({provider} 上下文窗口为 X tokens)"
- `网络断开` → 重试，提示"网络连接失败，请检查网络"
- `JSON 非法` → 进入 JSON Repair 流程
- `模型不存在` → 不重试，提示"模型 {model_name} 在 {provider} 中不可用"

### 11.10 HTTP 客户端调用接口

```rust
impl DeepSeekClient {
    pub async fn chat_completion(
        &self,
        base_url: &str,
        api_key: &str,
        request: &ChatCompletionRequest,
        timeout: Duration,
        max_retries: u32,
    ) -> Result<ChatCompletionResponse, String>;
}
```

所有供应商客户端实现统一的内部接口，但允许供应商特定的请求/响应格式适配。

---

## 12. Prompt 工程系统

### 12.1 Prompt 分层

```
System Prompt (不可随便修改, 定义协议和输出要求)
  └─ Template Prompt (由知识库模板决定, e.g. 科研论文 vs 通用)
      └─ User Custom Instruction (用户偏好, 语言, 关注重点)
          └─ Task Prompt (运行时拼接: 上下文 + 候选页面 + 任务目标)
```

### 12.2 核心概念定义 (必须包含在 Prompt 中)

| 概念 | 定义 |
|------|------|
| **Source** | 用户上传原始资料，事实来源，不允许修改 |
| **Entity** | 具有明确指代的名词: 人物、组织、论文、项目、产品、数据集、方法、软件库 |
| **Concept** | 抽象知识点、理论、机制、原则、术语、方法类别 |
| **Topic** | 更高层级的主题集合 |
| **Claim** | 可验证的事实性陈述，需要绑定来源 |
| **Citation** | Claim 对应的来源位置，默认 model_reported |
| **Conflict** | 新 claim 与已有 Wiki 不一致或存在张力 |
| **Wiki Page** | 要创建或更新的 Markdown 页面 |
| **Wiki Update Plan** | 页面创建/修改/合并/跳过计划 |
| **Alias** | 同一实体或概念的等价名称/翻译/缩写/大小写变体 |
| **Relationship** | 标准化关系，必须使用 relation enum |

### 12.3 Prompt 硬性约束

Prompt 必须要求大模型：
1. 不要在未检查 `existing_candidates` 前建议新建页面
2. 不要把相关概念当 alias
3. 不要把子概念当 alias
4. 不要把实现方式当 alias
5. 没有 evidence 的关系不要输出为可写入关系
6. 优先返回 JSON，不要解释性文字
7. 所有输出必须符合指定的 JSON Schema

### 12.4 PromptBuilder 接口

```rust
impl PromptBuilder {
    /// 构建 SourceIngest Prompt
    pub fn build_ingest_prompt(
        &self,
        document_text: &str,
        source_id: &str,
        kb_template: &KbTemplate,
        existing_pages_summary: &str,
        user_custom_instruction: Option<&str>,
    ) -> (String, String);  // (system_prompt, user_message)

    /// 构建 Query Prompt
    pub fn build_query_prompt(
        &self,
        question: &str,
        context_pages: &[WikiPage],
        scope: &QueryScope,
    ) -> (String, String);
}
```

---

## 13. JSON Schema 与输出校验

### 13.1 校验流程

```
模型返回文本
  → 1. 严格 JSON parse
  → 2. 成功? → JsonSchemaValidator 校验
  → 3. 失败? → 提取 ```json ... ``` block → 重试 parse
  → 4. 仍失败? → JsonRepair 修复 (补引号/括号/逗号) → 重试 parse
  → 5. 仍失败? → task failed, 保存 raw output, 用户可查看并重试
```

### 13.2 SourceIngest 输出 Schema

```json
{
  "source_summary": {
    "title": "string",
    "type": "string",
    "language": "string",
    "short_summary": "string",
    "long_summary": "string",
    "key_points": ["string"]
  },
  "coverage_report": {
    "document_sections_seen": ["string"],
    "possibly_missing_sections": ["string"],
    "confidence_in_coverage": "high | medium | low",
    "notes": "string"
  },
  "entities": [{
    "name": "string",
    "type": "string",
    "description": "string",
    "evidence": [{"source_id": "string", "location": "string", "quote": "string"}]
  }],
  "concepts": [{
    "name": "string",
    "definition": "string",
    "related_entities": ["string"],
    "evidence": [{"source_id": "string", "location": "string"}]
  }],
  "topics": [{"name": "string", "description": "string", "related_concepts": ["string"]}],
  "claims": [{
    "claim": "string",
    "confidence": "high | medium | low",
    "source_id": "string",
    "location": "string",
    "citation_status": "model_reported | user_verified | verified | missing"
  }],
  "relationships": [{
    "source": "string", "source_type": "entity | concept | topic | source | wiki_page",
    "target": "string", "target_type": "entity | concept | topic | source | wiki_page",
    "relation": "string",
    "description": "string",
    "evidence": {"source_id": "string", "location": "string"},
    "confidence": "high | medium | low"
  }],
  "proposed_wiki_updates": [{
    "operation": "create | update | merge | skip",
    "page_type": "string", "path": "string", "title": "string",
    "reason": "string", "risk_level": "low | medium | high",
    "requires_review": true
  }],
  "conflicts": [{"description": "string", "new_claim": "string", "existing_claim": "string"}],
  "questions_for_user": ["string"]
}
```

### 13.3 WikiUpdate 输出 Schema

```json
{
  "wiki_update_plan": [{
    "path": "string",
    "operation": "create | update | merge | delete_suggestion | skip",
    "title": "string",
    "summary": "string",
    "content_blocks": {},
    "new_markdown": "string",
    "patch_mode": "full_replace | replace_section | append_section | create_new",
    "target_section": "string",
    "citations": [{"source_id": "string", "location": "string", "citation_status": "model_reported"}],
    "risk_level": "low | medium | high",
    "requires_review": true,
    "reason": "string"
  }],
  "index_updates": [],
  "log_entry": "string",
  "graph_updates": [],
  "review_notes": []
}
```

---

## 14. 去重消歧系统

### 14.1 Resolution 流程

```
新抽取的 entities/concepts/topics
  → CandidateSearchEngine 检索已有候选页面
     依据: title, canonical_name, aliases, normalized_name, slug,
           page_type, source metadata, index.md 摘要, knowledge_items,
           字符串相似度
  → 新抽取项 + 候选页面 一起提交给 ResolutionAgent
  → ResolutionAgent 判断:
     - create_new: 新建页面
     - update_existing: 更新已有页面
     - append_to_existing: 追加到已有页面
     - add_alias: 添加别名到已有页面
     - merge_suggestion: 建议合并 (高风险管理)
     - skip: 跳过
     - needs_user_review: 无法判断，进入审阅
```

### 14.2 Alias 规则

| 关系类型 | 处理方式 |
|----------|----------|
| 完全同义 | 可自动添加 alias |
| 翻译同义 | 可自动或低风险审阅后添加 |
| 缩写同义 | 需要确认 |
| 相近概念 | 不能作为 alias, 只能 related_to |
| 上下位概念 | 不能作为 alias, 只能 is_a / part_of |
| 同主题概念 | 不能作为 alias |
| 实现方式 | 不能作为 alias |

### 14.3 CandidateSearchEngine

```rust
impl CandidateSearchEngine {
    pub fn search_candidates(
        &self,
        kb_id: &str,
        extracted_items: &[ExtractedItem],
    ) -> Result<Vec<CandidateMatch>, String>;
}

struct CandidateMatch {
    pub input_item: ExtractedItem,
    pub candidates: Vec<KnowledgeItem>,  // 按相似度排序
    pub top_similarity: f32,
}
```

---

## 15. 关系建立与知识图谱

### 15.1 关系类型枚举 (固定)

```
is_a          — 上下位关系
part_of       — 部分-整体关系
uses          — 使用关系
depends_on    — 依赖关系
improves      — 改进关系
compares_with — 对比关系
contradicts   — 矛盾关系
cites         — 引用关系
mentions      — 提及关系
related_to    — 一般相关 (需限制数量)
has_alias     — 别名关系
belongs_to_topic — 属于主题
evaluated_on  — 在...上评估
proposed_by   — 由...提出
applies_to    — 适用于
derived_from  — 派生自
```

### 15.2 关系写入规则

1. RelationshipAgent 输出标准化关系三元组
2. 本地将 source 和 target 映射到 canonical Wiki page
3. 映射成功后写入 `graph_edges` 表
4. 同时更新对应 Markdown 页面的 Related Pages section
5. **没有 evidence 的关系不能写入**
6. 低置信度关系进入审阅中心
7. `related_to` 数量必须限制，每次导入最多 5 条
8. 每次导入高置信关系最多 20 条

### 15.3 图谱数据同步

```
graph_nodes 来源:
  - sources → source 节点
  - wiki_pages → wiki_page 节点
  - knowledge_items → concept/entity/topic 节点

graph_edges 来源:
  - relationships → 标准化关系边
  - wiki_pages frontmatter.related_pages → related_to 边
```

提供"重建图谱索引"按钮，从 wiki_pages / knowledge_items / relationships 重新生成 graph_nodes / graph_edges。

---

## 16. 审阅与 Diff 系统

### 16.1 三种维护模式

| 模式 | 低风险 | 中风险 | 高风险 |
|------|--------|--------|--------|
| **Strict** | 审阅 | 审阅 | 审阅 |
| **Balanced** (默认) | 自动 | 审阅 | 审阅 |
| **Auto** | 自动 | 自动 | 审阅 |

### 16.2 风险等级定义

**低风险**：
- 新增 source summary
- 添加 alias
- 添加 related link
- 追加 log.md
- 新建无冲突页面
- 新增标签

**中风险**：
- 修改已有页面 section
- 调整页面结构
- 添加上位/下位关系
- 将问答保存为页面

**高风险**：
- 合并页面
- 删除页面
- 覆盖核心定义
- 修改 canonical_name
- 标记 contradiction
- 重命名页面
- 覆盖旧结论

### 16.3 审阅中心功能

- 按任务分组 / 按页面分组 / 按风险等级分组
- 高风险置顶，默认折叠低风险
- 一键接受低风险
- 逐条接受 / 全部拒绝
- 编辑后接受
- 要求重新生成
- 标记为以后处理
- 查看原始 Prompt
- 查看模型原始输出
- 查看引用来源
- 查看版本快照
- 回滚

### 16.4 每个 AI 修改必须回答四个问题

1. **改了什么？** — 具体页面、section、内容
2. **为什么改？** — AI 的判断依据
3. **基于哪个 source？** — 来源文件和位置
4. **能否撤销？** — 有版本快照即可回滚

### 16.5 DiffEngine

```rust
impl DiffEngine {
    pub fn generate_diff(
        &self,
        old_content: &str,
        new_content: &str,
    ) -> DiffResult;
}

struct DiffResult {
    pub hunks: Vec<DiffHunk>,
    pub stats: DiffStats,  // added_lines, removed_lines, modified_lines
}
```

前端展示两栏对比视图：
- **左栏 (Current Native)**: 当前本地 Markdown 文件
- **右栏 (AI Proposal)**: 绿色 = 新增, 红色 = 删除, 橙色闪烁 = AI 自动引入的双链 `[[概念]]`

---

## 17. Wiki 写入与版本安全

### 17.1 WikiWriter 写入流程

```
1. 获取页面锁 (文件互斥锁)
2. 读取当前页面
3. 检查 base_version_hash (确认页面未被用户手动修改)
4. 检查 operation_hash 是否已应用 (幂等性)
5. 检查 alias 是否已存在
6. 检查 relationship 是否已存在
7. 创建版本快照
8. 校验 update_plan
9. 生成目标 Markdown
10. 写入临时文件 (.tmp)
11. 原子 rename 替换正式文件
12. 更新 SQLite:
    - wiki_pages (title, path, content_hash, updated_at)
    - knowledge_items (canonical_name, summary)
    - aliases
    - relationships
    - graph_nodes / graph_edges
    - operations (operation_id, operation_hash, status)
13. 更新 index.md (IndexService)
14. 追加 log.md (LogService)
15. 释放页面锁
16. 更新 review_item.status = applied
17. EventBus 推送刷新事件
```

### 17.2 写入安全防护

**必须防止**：
1. 并发写同一页面 (页面锁)
2. 写到一半崩溃 (原子 rename)
3. 重复应用同一 operation (operation_hash 幂等检查)
4. 覆盖用户手动编辑 (base_version_hash 比对)
5. 删除未审阅内容
6. 覆盖核心定义
7. index.md / log.md / SQLite 与 Markdown 文件不一致

### 17.3 页面锁机制

```rust
impl WikiWriter {
    fn acquire_page_lock(&self, page_path: &str) -> Result<PageLock, String>;
    fn release_page_lock(&self, lock: PageLock);
}
```

### 17.4 VersionManager

```rust
impl VersionManager {
    /// 创建页面版本快照
    pub fn create_snapshot(&self, kb_id: &str, page_path: &str, content: &str) -> Result<String, String>;

    /// 回滚到指定版本
    pub fn rollback(&self, kb_id: &str, page_path: &str, version_id: &str) -> Result<(), String>;

    /// 列出页面的所有版本
    pub fn list_versions(&self, kb_id: &str, page_path: &str) -> Result<Vec<Version>, String>;
}
```

### 17.5 Wiki 页面模板

所有 Wiki 页面必须使用稳定模板。模型提供结构化内容块，模板渲染页面。

```yaml
---
title: Self-Attention
type: concept
canonical_name: Self-Attention
aliases:
  - self attention
  - 自注意力
sources:
  - src_xxx
tags:
  - attention
  - transformer
confidence: medium
status: active
created: 2026-05-06
updated: 2026-05-06
last_updated_by_task: task_xxx
---
```

稳定 section 结构：
```markdown
# Title

## Summary 

## Definition 

## Key Points 

## Evidence

## Related Pages

## Source Contributions 

## Open Questions 
```

---

## 18. 搜索系统

### 18.1 搜索范围

- `wiki_pages.title`
- `wiki_pages.path`
- `wiki_pages.canonical_name`
- `aliases.alias`
- Markdown 正文
- frontmatter
- `source_summary`
- `tags`

### 18.2 搜索能力

- 精确搜索
- 大小写不敏感
- 中英文搜索
- 子串搜索
- 简单模糊搜索
- 按 page_type 筛选

### 18.3 搜索结果展示

- 标题
- 页面类型
- 路径
- 匹配片段 (高亮)
- 匹配字段
- 更新时间
- 打开按钮
- 在 Chat 中询问
- 在思维导图中定位

### 18.4 空状态处理

- 没有结果: 提示检查拼写或尝试相近词
- 搜索 broken 页面: 显示警告

---

## 19. 问答与 Chat 系统

### 19.1 问答范围

- 整个知识库
- 当前页面
- 指定文件
- 指定标签
- 指定目录

### 19.2 QueryAgent 流程

```
1. 根据范围读取相关 Wiki 页面
2. 读取相关 Source Summary
3. 读取 aliases, related pages
4. 构造 Query Prompt (System + Context + Question)
5. 调用 DeepSeek
6. 返回 answer + citations + related_pages
```

### 19.3 回答要求

- 回答带引用 (wiki page path + section + citation_status)
- 展示本次使用的上下文 (右侧面板)
- 支持流式输出
- 显示当前使用模型和 token usage
- 支持保存回答为 Wiki 页面 (走 Review → WikiWriter → VersionManager)

### 19.4 Chat 右侧上下文面板

- 本次检索到的页面
- 使用的 source
- 置信度
- 可打开页面
- 可查看 Source Preview

---

## 20. 健康检查与工作区修复

### 20.1 检查项

| 类别 | 检查项 |
|------|--------|
| 文件结构 | 目录完整性, 非法文件, 异常大文件 |
| 数据库 | wiki_pages 与文件一致性, knowledge_items 完整性 |
| 页面 | 重复页面, 孤立页面, broken 页面, 内容过短 |
| 引用 | 缺失引用, citation_status 分布 |
| 关系 | relationships 为空, 低置信度过高 |
| 图谱 | graph_nodes 缺失, graph_edges 缺失, 孤立节点 |
| 索引 | index.md 是否包含所有页面 |
| 版本 | 版本快照缺失 |
| Source | 与 Wiki 未关联, Preview 缺失 |
| 路径 | wiki/wiki 重复路径, 非法字符 |

### 20.2 问题严重程度

- **严重错误**: 数据丢失风险, 文件缺失且无快照
- **警告**: 数据不一致但可恢复
- **信息**: 统计异常, 建议优化

### 20.3 修复动作

- 从版本快照恢复缺失页面
- 删除失效 wiki_pages 记录
- 修复 wiki/wiki 重复路径
- 重建 graph_nodes / graph_edges
- 重新生成 index.md
- 重新生成 log.md 摘要
- 修复 knowledge_items 与 wiki_pages 关联
- 重建 Source Preview 缓存
- 一键修复所有可自动修复的问题

---

## 21. MarkItDown 通用格式转换系统

### 21.1 定位与职责

**MarkItDown** 是微软开源的 Python 库（[github.com/microsoft/markitdown](https://github.com/microsoft/markitdown)），负责将异构多源文件格式统一"降维"转换为标准化纯 Markdown 文本。它是 LLMWiki 文件处理管线的**核心枢纽层**，所有非纯文本格式的文档都必须经过 MarkItDown 归一化后，才能进入后续的 AI 分析管线。

```
MarkItDown 职责:
  1. 异构格式统一转换 (PDF/DOCX/PPTX/XLSX/CSV/JSON/XML/图片 → Markdown)
  2. 网页快照清洗 (HTML → 去除 JS/CSS/广告 → 纯净 Markdown)
  3. 复杂表格收敛 (Excel 多级表头 → 三线标准 Markdown 表格)
  4. 作为原生 Rust Skill 的回退方案
  5. 子进程隔离运行 (Python crash 不影响 Tauri 主进程)
```

**MarkItDown 不是**：OCR 引擎、PDF 深度解析器、图片理解工具。

### 21.2 Python Sidecar 子进程架构

MarkItDown 在独立的 Python 子进程中运行，与 Tauri Rust 主进程物理隔离：

```
┌─────────────────────────┐      stdin/stdout JSON-RPC      ┌──────────────────┐
│   Tauri Rust 主进程      │ ◄──────────────────────────────► │  Python 子进程    │
│                         │                                  │                  │
│  markitdown_skill.rs    │  → {"file_path": "/path/to.pdf"} │  bridge.py       │
│                         │  ← {"success": true,             │  + markitdown    │
│  silent_command()       │      "markdown_content": "..."}  │                  │
│  CREATE_NO_WINDOW       │                                  │  独立崩溃域       │
└─────────────────────────┘                                  └──────────────────┘
```

**关键设计决策**：

| 决策 | 原因 |
|------|------|
| 子进程而非嵌入 | Python GIL/内存泄漏/崩溃不波及 Rust 主进程 |
| Stdio 管道通信 | 避免文件路径在跨语言调用时的二次 I/O 开销 |
| JSON-RPC 协议 | 零内存拷贝的结构化输入输出 |
| CREATE_NO_WINDOW | Windows 下抑制 cmd 弹窗，用户无感知 |

### 21.3 核心调用接口

```rust
// src-tauri/src/skills/markitdown_skill.rs

pub struct MarkitdownSkill;

impl MarkitdownSkill {
    /// 将任意支持格式的文件转换为 Markdown 文本
    pub fn convert(file_path: &Path) -> Result<String, String>;

    /// 检查 MarkItDown 是否可用（首次调用时自动尝试安装）
    pub fn is_available() -> bool;

    /// 检查 Python 是否可用
    pub fn has_python() -> bool;

    /// 自动安装 MarkItDown (pip install markitdown)
    pub fn auto_install() -> Result<String, String>;

    /// 清除可用性缓存（允许重新检测）
    pub fn reset_cache();

    /// 重置安装状态（允许重新尝试自动安装）
    pub fn reset_install_state();
}
```

### 21.4 自动安装与首次使用流程

```
is_available() 调用
  → 检查本地缓存 (Mutex<bool>)
  → 缓存命中? → 直接返回
  → 尝试 find_command() (markitdown / python -m markitdown / python3 -m markitdown)
  → 找到? → 更新缓存 = true → 返回
  → 未找到 且 未尝试过安装?:
     → 查找 Python 解释器 (python / python3 / py)
     → 优先本地源码安装 (pip install ./markitdown-main/packages/markitdown)
     → 回退到 PyPI 安装 (pip install markitdown)
     → 重新 find_command()
     → 成功? → 更新缓存 = true → 返回
     → 失败? → 更新缓存 = false → 返回 false
```

**安装策略**：
- 优先使用项目内 `markitdown-main/` 源码（离线可用）
- 回退到 PyPI 在线安装
- 同一进程生命周期仅尝试安装一次（`INSTALL_ATTEMPTED` 原子标志）
- 用户可在设置页面手动触发重新安装（`retry_markitdown_install` → `reset_install_state()`）

### 21.5 编码处理

Windows 中文环境下子进程 stdout 可能出现编码问题，系统实现了双解码策略：

```rust
fn decode_stdout(bytes: &[u8]) -> String {
    // 1. 默认以 UTF-8 解码
    // 2. 检测拉丁-1 补充区字符占比 (>30% → 可能是 GBK)
    // 3. 如果是 GBK，用 encoding_rs::GBK 重新解码
}
```

同时设置 Python 子进程环境变量：
- `PYTHONIOENCODING=utf-8`
- `PYTHONUTF8=1`

### 21.6 在 DocumentProcessor 中的调用策略

`DocumentProcessor` 根据文件类型采用分层调用策略：

| 文件类型 | 主解析方案 | 回退方案 | 说明 |
|----------|-----------|----------|------|
| **MD / TXT** | 原生 Rust Skill | 不需要回退 | 纯文本，原生 Rust 完全够用 |
| **HTML** | 原生 Rust Skill | MarkItDown | HTML→Markdown 轻量转换 |
| **PDF** | **MarkItDown (优先)** | 原生 pdf_skill (lopdf) | MarkItDown 更稳定，内置解析为回退 |
| **DOCX** | **MarkItDown (优先)** | 原生 docx_skill | MarkItDown 格式保留更好 |
| **PPTX** | **MarkItDown (优先)** | 原生 pptx_skill | MarkItDown 文本提取更完整 |
| **XLSX / XLS** | **MarkItDown (必须)** | 无 | 依赖 MarkItDown 将表格转为 Markdown 表格 |
| **CSV** | **MarkItDown (必须)** | 无 | 依赖 MarkItDown 格式化 |
| **JSON / XML** | **MarkItDown (必须)** | 无 | 依赖 MarkItDown 结构化转换 |
| **Image** | 仅保存为资产 | 不转换 | asset_only，不参与知识抽取 |
| **未知格式** | **MarkItDown (尝试)** | 报错 | 最后兜底方案 |

### 21.7 MarkItDown 对核心闭环的支撑

MarkItDown 处于系统的**第一层转换层**，它的输出质量直接影响整个 AI 管线的效果：

```
异构源文件
  → [MarkItDown 收敛层] → 纯净标准化 Markdown
  → [本地 RAG 层] → 切片、分词、向量化
  → [云端 Agent 层] → Extractor → Linker → Validator → Patch JSON
  → [前端 Diff 层] → 用户审阅确认
  → [WikiWriter] → 落盘为长期 Markdown Wiki 资产
```

### 21.8 代码位置

- 核心实现：[src-tauri/src/skills/markitdown_skill.rs](src-tauri/src/skills/markitdown_skill.rs)
- 文档分发器：[src-tauri/src/skills/document_processor.rs](src-tauri/src/skills/document_processor.rs)
- Python 桥接脚本（规划）：`scripts/bridge.py`
- 状态查询命令：[src-tauri/src/commands/utils.rs](src-tauri/src/commands/utils.rs) → `get_markitdown_status`, `retry_markitdown_install`
- 前端设置页：[src/pages/SettingsPage.tsx](src/pages/SettingsPage.tsx) (文档转换区域)

---

## 22. 文件处理管线

### 22.1 文档文件处理

**支持格式**: PDF, DOCX, Markdown, TXT, HTML

**处理流程**:
1. 保存到 `raw/sources/documents/`
2. 生成 `source_id`
3. 计算 SHA256 hash
4. 创建 Source Ingest Task
5. DocumentProcessor 分发到对应 Skill 解析
6. 提取的文本作为 Prompt 附件提交给 DeepSeek

### 22.2 图片文件处理

**支持格式**: PNG, JPG, JPEG, WEBP, GIF

**处理流程**:
1. 保存到 `raw/assets/images/`
2. 生成 `asset_id`
3. 不 OCR, 不视觉理解, 不参与知识抽取
4. 状态设为 `asset_only`
5. UI 明确提示: "图片仅保存为资产，不参与 AI 知识抽取"

### 22.3 DocumentProcessor

```rust
impl DocumentProcessor {
    pub async fn parse_document(file_path: &Path, kb_id: &str) -> Result<DocumentParseResult, String> {
        let extension = get_extension(file_path)?;
        match extension {
            "pdf" => pdf_skill::extract_text(file_path)
                .or_else(|_| markitdown_skill::convert(file_path)),
            "docx" => docx_skill::extract_text(file_path)
                .or_else(|_| markitdown_skill::convert(file_path)),
            "md" => md_skill::extract_text(file_path),
            "txt" => txt_skill::extract_text(file_path),
            "html" => html_skill::extract_text(file_path),
            "pptx" | "xlsx" | "csv" | "json" | "xml" => markitdown_skill::convert(file_path),
            _ => Err(format!("不支持的文件类型: {}", extension)),
        }
    }
}
```

### 22.4 重复文件检测

上传时检查 file_hash:
- 相同 hash → 提示"文件已存在"
- 选项: 跳过 / 重新分析 / 作为新版本导入

---

## 23. Source Preview 系统

### 23.1 定位与边界

Source Preview 是 **预览层**，不是知识抽取主流程。不做 OCR，不做 PDF 深度解析，不替代 DeepSeek 附件分析。

**Source Preview Markdown ≠ Wiki Page Markdown**

### 23.2 不同文件类型的预览规则

| 文件类型 | 预览方式 | preview_status |
|----------|----------|----------------|
| Markdown | 直接读取渲染 | generated |
| TXT | 包装为 Markdown | generated |
| HTML | 轻量 HTML→Markdown 转换 | generated |
| DOCX | 提取普通文本 → Markdown (基础) | generated / unavailable |
| PDF | 显示元信息 + AI source_summary + coverage_report | ai_summary_only |
| 图片 | 显示缩略图 + 资产信息 | asset_only |

### 23.3 Source Preview 页面 Tabs

1. 原始信息 (文件名, 类型, 大小, hash, 路径, 状态)
2. Markdown 预览
3. AI 摘要 (source_summary)
4. 抽取实体 (entities)
5. 抽取概念 (concepts)
6. 关系 (relationships)
7. 关联 Wiki 页面
8. 任务日志

### 23.4 缓存

- 预览文件: `.runtime/source_previews/{source_id}.md`
- 预览状态: generated / ai_summary_only / unavailable / failed / asset_only
- 数据库: `sources.preview_path` + `sources.preview_status`

---

## 24. 思维导图式知识图谱

### 24.1 默认视图

当没有选择中心节点时，以当前知识库为中心:

```
我的知识库
  ├── 概念 (concepts)
  ├── 实体 (entities)
  ├── 数据集 (datasets)
  ├── 方法 (methods)
  ├── 来源文件 (sources)
  ├── 问题 (questions)
  ├── 待审阅 (pending reviews)
  └── 健康问题 (health issues)
```

### 24.2 中心节点类型

- 当前 Wiki 页面
- Source 文件
- Topic / Concept / Entity / Dataset / Method / Question
- 整个知识库
- 搜索结果节点

### 24.3 展开维度

1. **按页面类型**: concepts, entities, datasets, methods, topics, sources, questions
2. **按关系类型**: is_a, part_of, uses, cites, mentions, related_to, contradicts, derived_from
3. **按来源**: 来自哪个 Source, 哪个任务生成
4. **按时间**: 最近创建/更新, 导入批次, 版本变化
5. **按可信度**: high, medium, low, missing citation
6. **按维护状态**: 待审阅, 缺引用, 孤立, 冲突, broken

### 24.4 交互要求

- 单击节点: 右侧显示详情
- 双击节点: 打开 Wiki 页面或 Source Preview
- 右键节点: 上下文菜单
- 点击分支: 展开/折叠
- "设为中心": 以该节点重新生成思维导图
- 搜索并定位节点
- 导出: PNG / SVG / Markdown 大纲 / JSON

### 24.5 空状态处理

- graph_nodes = 0 && knowledge_items > 0: "可从知识项重建图谱"，提供按钮
- graph_nodes > 0 && graph_edges = 0: 显示孤立节点，不显示"暂无数据"
- graph_edges = 0: "当前有知识节点，但关系尚未建立"

---

## 25. UI/UX 设计规范

### 25.1 整体风格

- **简洁** — 信息优先，去除装饰元素
- **浅色** — 默认浅色主题，蓝白灰配色
- **低饱和** — 避免强烈色彩
- **桌面工作台** — 适合长时间使用，信息密度适中
- **卡片化** — 内容以卡片组织
- **专业但不冰冷** — 接近 Chatbox / Notion / Obsidian / Linear 的结合体

### 25.2 禁止的风格

- 炫酷科技风
- 游戏风
- 深色赛博风
- 复杂大屏仪表盘
- 强渐变
- 过大阴影
- 复杂背景纹理
- 夸张动画

### 25.3 布局结构

```
┌──────────┬────────────────────────┬───────────┐
│          │                        │           │
│ 左侧     │    中间主工作区         │  右侧     │
│ 导航栏   │                        │  上下文   │
│          │                        │  面板     │
│          │                        │           │
├──────────┴────────────────────────┴───────────┤
│              底部状态栏                         │
└────────────────────────────────────────────────┘
```

### 25.4 左侧导航栏

- 当前知识库名称 (可切换)
- 首页 (Dashboard)
- Wiki 页面
- Chat 问答
- 文件 / Sources
- 导入队列 (Import Tasks)
- 审阅中心 (Review)
- 搜索 (Search)
- 知识图谱 (Graph)
- 健康检查 (Health Check)
- 设置 (Settings)

### 25.5 底部状态栏

- 当前模型 (deepseek-chat / deepseek-reasoner)
- 当前任务状态 (空闲 / 运行中 / 有错误)
- 当前知识库路径
- 最近保存时间
- Token 使用估算 (今日 / 累计)
- API 连接状态

### 25.6 右侧上下文面板 (可复用)

根据页面类型显示不同内容：

| 页面 | 面板内容 |
|------|----------|
| Wiki | frontmatter, aliases, sources, tags, relationships, backlinks, versions, AI 操作 |
| Source | metadata, AI summary, coverage_report, extracted items, linked wiki pages, tasks |
| Review | 修改摘要, 原因, 风险, evidence, 应用后影响 |
| Graph | 节点详情, 关系详情, evidence, related pages |
| Chat | 使用上下文, 引用页面, 建议保存为 Wiki |
| Search | 搜索结果详情, 匹配字段, 匹配片段 |
| Health | 问题详情, 原因, 影响, 修复动作 |

### 25.7 状态文案规范

| 内部状态 | 显示文案 |
|----------|----------|
| saved | 已保存 |
| analyzing | 分析中 |
| analyzed | 已分析 |
| review_pending | 等待审阅 |
| applied | 已应用 |
| applied_with_warnings | 已应用但有警告 |
| failed | 失败 |
| cancelled | 已取消 |
| asset_only | 仅资产 |
| interrupted | 已中断 |

---

## 26. 知识库模板系统

### 26.1 内置模板

| 模板 | 名称 | wiki 子目录 | 页面类型 |
|------|------|-------------|----------|
| general | 通用知识库 | concepts, entities, topics, sources, questions | concept, entity, topic |
| research_paper | 科研论文知识库 | papers, concepts, methods, datasets, questions | paper, concept, method, dataset |
| course_learning | 课程学习知识库 | courses, concepts, notes, questions | concept, note, question |
| project_management | 项目管理知识库 | projects, tasks, decisions, meetings | project, task, decision, meeting |
| reading_notes | 读书笔记知识库 | books, concepts, quotes, reviews | book, concept, quote |

### 26.2 模板配置 (TEMPLATE.yaml)

```yaml
name: research_paper
display_name: 科研论文知识库
description: 用于论文阅读、综述写作和研究方向管理
default_language: zh-CN

directories:
  - wiki/papers
  - wiki/concepts
  - wiki/methods
  - wiki/datasets
  - wiki/questions
  - wiki/reviews

page_types:
  paper:
    fields: [title, authors, year, venue, research_problem, method, dataset, metrics, results, limitations, sources]
  concept:
    fields: [name, definition, related_papers, related_methods, sources]

review_policy:
  require_review_for_new_page: false
  require_review_for_update: true
  require_review_for_conflict: true

graph:
  node_types: [paper, concept, method, dataset, source]
  edge_types: [cites, uses_method, related_to, contradicts]
```

### 26.3 模板迁移

知识库创建后切换模板必须作为迁移操作处理，需要生成迁移预览，不能直接强制修改已有页面结构。

---

## 27. 安全与隐私

### 27.1 API Key 安全

- 使用 Windows Credential Manager 存储 (SecretService)
- 绝不写入配置文件、数据库、日志、Prompt Snapshot、错误报告
- 日志和 Prompt Snapshot 必须脱敏 API Key 和 Authorization Header

### 27.2 数据安全

- 100% 用户本地物理盘存储
- 大模型云端 API 仅接收局部上下文片段
- 云端数据为"内存态"，请求结束即销毁
- 卸载软件默认不删除用户知识库

### 27.3 日志脱敏

日志中必须脱敏:
1. API Key
2. Authorization Header
3. 用户敏感路径 (可选)

### 27.4 文件安全

- 文件写入使用原子 rename (临时文件 → 正式文件)
- 页面写入前获取文件互斥锁
- 操作进行 operation_hash 幂等性检查
- 版本快照支持回滚

---

## 28. IPC 通信协议

### 28.1 Commands (请求-响应)

```typescript
// 前端调用示例
import { invoke } from "@tauri-apps/api/core";

// 知识库管理
await invoke("list_knowledge_bases");
await invoke("create_knowledge_base", { name, path, templateName });
await invoke("delete_knowledge_base", { kbId });

// 文件管理
await invoke("import_source", { kbId, filePath });
await invoke("list_sources", { kbId });
await invoke("get_source_preview", { sourceId });

// 任务管理
await invoke("create_ingest_task", { kbId, sourceId });
await invoke("get_task_detail", { taskId });
await invoke("get_task_files", { taskId });
await invoke("retry_task", { taskId });
await invoke("cancel_task", { taskId });

// 审阅
await invoke("list_reviews", { kbId });
await invoke("accept_review_item", { reviewItemId });
await invoke("reject_review_item", { reviewItemId });
await invoke("accept_all_low_risk", { reviewId });
await invoke("reject_all", { reviewId });

// Wiki
await invoke("list_wiki_pages", { kbId });
await invoke("get_wiki_page", { kbId, pageId });
await invoke("save_wiki_page", { kbId, pageId, content });
await invoke("create_wiki_page", { kbId, title, pageType, content });

// 问答
await invoke("run_query", { kbId, question, scope });

// 搜索
await invoke("search_kb", { kbId, query });

// 图谱
await invoke("get_graph_data", { kbId });
await invoke("rebuild_graph", { kbId });

// 健康检查
await invoke("run_health_check", { kbId });
await invoke("fix_health_issue", { kbId, issueId });
await invoke("fix_all_auto", { kbId });
```

### 28.2 Events (推送通知)

```typescript
import { listen } from "@tauri-apps/api/event";

// 知识库状态变化
listen("kb-stats-changed", (event) => { ... });

// 任务事件
listen("task-event", (event) => {
  // { task_id, event_type, agent_name, message, created_at }
});

// Agent 流式输出
listen("agent-stream", (event) => {
  // { task_id, delta, is_finished }
});

// 审阅变化
listen("review-changed", (event) => { ... });

// 错误通知
listen("error-notification", (event) => {
  // { level, title, message }
});
```

### 28.3 流式数据节流

大模型云端 API 吐出 Chunk 频率极高 (50-100次/秒)。Rust 后端设立 **50ms 窗口期定时节流阀**，将 50ms 内收到的所有字符片段合并后，再向前端发射一次 IPC Event，保证前端渲染达到 60fps。

---

## 29. OpenWebUI 集成与模型功能扩展

### 29.1 定位

**OpenWebUI** 是一个功能强大的自托管 LLM Web 交互平台，在 LLMWiki 项目中充当**可选的模型聚合网关与能力扩展层**。用户可在本地或服务器上部署 OpenWebUI 实例，将其作为统一的模型接入点，从而获得以下扩展能力：

1. **多模型统一路由**：一个 API 地址代理所有模型供应商
2. **Pipeline 预处理/后处理**：在模型调用前后注入自定义处理逻辑
3. **Tool / Function Calling 集成**：标准化的工具调用机制
4. **MCP 协议支持**：Model Context Protocol 标准工具接入
5. **模型负载均衡**：多实例自动分发请求

### 29.2 集成架构

```
┌─────────────────────────────────────────────┐
│            LLMWiki (Tauri 桌面端)            │
│                                             │
│  ModelGateway → OpenAI-compatible Client     │
│       │                                     │
│       │  POST /v1/chat/completions          │
│       │  (或 /ollama/api/chat)              │
│       ▼                                     │
├─────────────────────────────────────────────┤
│         OpenWebUI 实例 (可选聚合网关)        │
│                                             │
│  ┌─────────┬──────────┬──────────┐          │
│  │ Router  │ Pipeline │ Tool Mgr │          │
│  ├─────────┼──────────┼──────────┤          │
│  │ DeepSeek│ OpenAI   │ Anthropic│ ...      │
│  │ Ollama  │ Gemini   │ Moonshot │          │
│  └─────────┴──────────┴──────────┘          │
└─────────────────────────────────────────────┘
```

### 29.3 OpenWebUI 提供的关键扩展能力

#### 29.3.1 多模型路由与聚合

OpenWebUI 可以同时连接多个模型后端，通过统一的 `/v1/chat/completions` 端点暴露给 LLMWiki。用户在 OpenWebUI 管理界面配置所有模型后，LLMWiki 只需配置一个 Base URL 即可访问所有模型。

**支持的模型后端**：
- OpenAI (GPT-4o, GPT-4, GPT-3.5, o1, o3)
- Anthropic (Claude Opus, Sonnet, Haiku)
- Google (Gemini 2.5, Gemini 2.0)
- DeepSeek (deepseek-chat, deepseek-reasoner)
- Ollama 本地模型 (llama3, qwen3, mistral, gemma3)
- 任何 OpenAI API 兼容供应商

#### 29.3.2 Pipeline 系统

OpenWebUI 的 Pipeline 功能允许在模型调用前后注入自定义处理逻辑：

```
用户请求 → [前置过滤器] → 模型推理 → [后置过滤器] → 返回给 LLMWiki
                │                          │
                ├── 敏感词过滤              ├── JSON 格式修复
                ├── Prompt 增强             ├── 引用提取
                ├── 上下文注入              ├── Markdown 清洗
                └── 格式转换                └── 安全审查
```

**LLMWiki 可利用的 Pipeline 场景**：
- **前置 Pipeline**：注入知识库上下文、拼接 Prompt 模板、文档格式预处理
- **后置 Pipeline**：JSON Schema 校验前置（在到达 Rust 后端前预校验）、格式修复

#### 29.3.3 Tool / Function Calling

OpenWebUI 支持标准的 OpenAI Function Calling 协议，可将 LLMWiki 本地能力暴露为 Tool：

```json
{
  "tools": [{
    "type": "function",
    "function": {
      "name": "search_local_knowledge_base",
      "description": "搜索用户本地知识库中的相关内容",
      "parameters": {
        "type": "object",
        "properties": {
          "query": {"type": "string"},
          "scope": {"type": "string", "enum": ["kb", "page", "tag"]}
        }
      }
    }
  }]
}
```

#### 29.3.4 MCP 协议集成

OpenWebUI 支持 Model Context Protocol (MCP)，允许标准化的工具服务器接入：

- **Filesystem MCP Server**：安全的文件系统访问
- **Web Search MCP Server**：联网搜索能力
- **Database MCP Server**：数据库查询能力
- **自定义 MCP Server**：LLMWiki 可开发专用 MCP 工具暴露本地能力

#### 29.3.5 Web Search 聚合

OpenWebUI 内置多种搜索引擎集成（DuckDuckGo / SearXNG / Brave / Bing / Google PSE / SerpAPI 等），可作为 LLMWiki 联网搜索能力的上游聚合层。

### 29.4 LLMWiki 与 OpenWebUI 的协作模式

| 模式 | 说明 | 适用场景 |
|------|------|----------|
| **模式 A：直连** | LLMWiki 直接调用模型供应商 API | 默认模式，无需额外部署 |
| **模式 B：OpenWebUI 作为透明代理** | LLMWiki → OpenWebUI → 多个模型后端 | 需要多模型灵活切换 |
| **模式 C：OpenWebUI + Pipeline** | 模式 B + 自定义 Pipeline 处理 | 需要预处理/后处理增强 |
| **模式 D：OpenWebUI + MCP** | 全功能集成，MCP 工具链 | 高级用户，需要工具编排 |

### 29.5 在 LLMWiki 中的配置方式

用户在设置页面选择"OpenAI 兼容 API"供应商后：

| 配置项 | 默认值 | 说明 |
|--------|--------|------|
| Base URL | `http://localhost:3000/api` | OpenWebUI 实例地址 |
| API Key | (用户填入) | OpenWebUI 的 API Key |
| Chat 模型 | `gpt-4o` 或其他 | OpenWebUI 中配置的模型 ID |
| Reasoner 模型 | `claude-sonnet-4-6` | 推理模型 ID |

所有配置与标准供应商配置完全一致，因为 OpenWebUI 暴露的是标准 OpenAI API 格式。

### 29.6 联网搜索与工具调用增强（基于 OpenWebUI 方案思想）

当用户勾选"允许联网"或大模型判断当前提问涉及本地知识盲区时：

1. **大模型触发 Tools/Function Calling** → 调用本地注册的搜索工具
2. **Rust `reqwest` 安全沙箱** → 请求搜索引擎，获取 URL
3. **MarkItDown 子进程** → 抓取网页 HTML → 剥离 JS/CSS/广告 → 归一化为纯净 Markdown
4. **纯净 Markdown 并入 Context** → 提交给大模型进行推理

### 29.7 代码位置

- 项目内 OpenWebUI 源码：[open-webui-main/](open-webui-main/)
- 联网搜索 Skill：[src-tauri/src/skills/web_search_skill.rs](src-tauri/src/skills/web_search_skill.rs)
- 联网搜索命令：[src-tauri/src/commands/web_search.rs](src-tauri/src/commands/web_search.rs)
- 模型配置命令：[src-tauri/src/commands/config.rs](src-tauri/src/commands/config.rs)
- 前端设置页：[src/pages/SettingsPage.tsx](src/pages/SettingsPage.tsx)

---

## 30. 本地嵌入与混合检索子系统 (可选进阶)

> 本节描述的是可选的架构升级方向，当前项目第一阶段以 LLM API 全文搜索为主，本节供后续进阶参考。

### 30.1 离线 Embedding 模型

使用 `ort` (ONNX Runtime Rust bindings) 加载量化模型 `bge-small-zh-v1.5.onnx` (约 90MB)。

```rust
pub struct LocalEmbedder {
    session: ort::Session,
    tokenizer: tokenizers::Tokenizer,
}

impl LocalEmbedder {
    pub fn compute_vector(&self, text: &str) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
        // 1. Tokenize → 截断到 512
        // 2. 构造 input_ids + attention_mask
        // 3. ONNX 推理 → last_hidden_state
        // 4. Mean Pooling → L2 Normalization
        // 5. 返回 512 维归一化向量
    }
}
```

### 30.2 混合检索 (RRF)

结合 FTS5 全文搜索与向量相似度搜索，使用倒数排名融合 (RRF) 算法：

$$\text{RRF\_Score}(d) = \frac{1}{60 + \text{Rank}_{\text{fts}}(d)} + \frac{1}{60 + \text{Rank}_{\text{vec}}(d)}$$

### 30.3 MarkItDown 子进程集成 (Python Sidecar)

Rust 通过 `tokio::process::Command` 拉起独立 Python 子进程运行 MarkItDown：

```rust
pub async fn run_markitdown_converter(
    absolute_file_path: &str,
    app_dir: &Path,
) -> Result<String, String> {
    // 1. 启动 Python 子进程
    // 2. 通过 stdin 传递文件路径 JSON
    // 3. 通过 stdout 接收转换后的 Markdown
    // 4. 子进程崩溃不影响主进程
}
```

---

## 31. Windows 安装与打包

### 31.1 目标平台

Windows 10 / Windows 11 (x64)

### 31.2 安装路径

| 类型 | 路径 |
|------|------|
| 应用安装目录 | `C:\Users\<User>\AppData\Local\Programs\LLM Knowledge Wiki\` |
| 用户数据目录 | `C:\Users\<User>\Documents\LLM Knowledge Wiki\` |
| 应用配置目录 | `C:\Users\<User>\AppData\Roaming\LLM Knowledge Wiki\` |

### 31.3 打包要求

- 使用 Tauri Bundler + NSIS/MSI
- 生成 Windows 安装包 (.msi 或 .exe)
- 双击安装，无需命令行
- 安装后创建桌面快捷方式和开始菜单快捷方式
- 用户无需安装 Node.js / Python / Rust / SQLite
- 卸载软件默认不删除用户知识库

### 31.4 Windows 兼容性注意事项

- 中文路径
- 空格路径
- OneDrive 同步路径
- 无管理员权限运行
- 文件被占用处理
- 路径过长 (MAX_PATH)
- 杀毒软件拦截
- 安装目录无写入权限
- AppData 权限问题

### 31.5 构建命令

```bash
npm run tauri build    # 生产构建, 输出在 src-tauri/target/release/bundle/
npm run tauri dev      # 开发模式
```

---

## 32. 开发工作流

### 32.1 前置条件

- Node.js 18+
- Rust stable (latest)
- Windows 10/11
- Visual Studio Build Tools (Windows)

### 32.2 开发命令

```bash
# 前端开发 (浏览器)
npm run dev                # Vite dev server on port 1420

# 完整 Tauri 桌面应用开发
npm run tauri dev          # 启动 Tauri + 前端热更新

# 类型检查 + 构建
npm run build              # tsc + vite build

# 生产构建
npm run tauri build        # 完整 Tauri 打包

# 仅 Rust 检查
cd src-tauri && cargo check
cd src-tauri && cargo clippy
```

### 32.3 开发流程

1. **启动开发环境**: `npm run tauri dev`
2. **前端开发**: 修改 `src/` 文件，Vite HMR 自动刷新
3. **后端开发**: 修改 `src-tauri/src/` 文件，Tauri 自动重编译重启
4. **数据库变更**: 在 `db/migrations.rs` 添加迁移 SQL
5. **新增 Command**: 在 `commands/` 添加 handler，在 `lib.rs` 注册
6. **新增 Agent**: 在 `agents/` 添加模块，在 Coordinator 中注册

### 32.4 代码规范

- Rust: 遵循 `cargo clippy` 建议
- TypeScript: 遵循 tsconfig rules
- 所有路径处理统一走 PathService
- 所有模型调用统一走 ModelGateway
- 所有文件写入统一走 WikiWriter
- 前端 Tauri invoke 封装在 pages/stores 内
- 数据库表名统一小写下划线
- 文件路径统一使用正斜杠 `/` 作为内部表示
- 中文界面文案统一使用简体中文

### 32.5 路径规范 (非常重要)

```
内部路径 (数据库/wiki_pages.path): 统一正斜杠, 相对于 workspace
  ✅ wiki/concepts/self-attention.md
  ✅ wiki/entities/liu-rulin.md

禁止的路径:
  ❌ wiki/wiki/concepts/xxx.md         (重复前缀)
  ❌ wiki\concepts\xxx.md              (反斜杠)
  ❌ concept · wiki\xxx.md             (显示字符串)
  ❌ C:\Users\...\workspace\wiki\...   (绝对路径)

读写转换:
  full_path = workspace_root + wiki_pages.path
  平台路径 = PathService::to_platform_path(internal_path)
```

---

## 33. 版本历史与路线图

### 33.1 版本演进

| 版本 | 状态 | 主要内容 |
|------|------|----------|
| v0.1.0 | ✅ 已完成 | 工程框架搭建: Tauri 2 + React + TypeScript + Tailwind + Rust 核心 |
| v0.1.1 | ✅ 已完成 | UI 页面实现: 所有页面组件, 路由, 布局, 基础组件 |
| v0.1.2 | ✅ 已完成 | 核心功能: 知识库管理, 文件上传, 任务管线, Agent Pipeline, 审阅, Wiki 基础 |
| v0.1.3 | ✅ 已完成 | 闭环修复: 路径规范, WikiWriter 落盘, Chat 正文读取, Search, Graph 孤立节点, Health 修复 |
| v0.1.4 | 🔄 进行中 | UI 升级: 文件树, Source Preview, Markdown 三模式, 思维导图, 右侧面板 |
| v0.1.5 | 📋 规划中 | 性能与体验: 缓存优化, 批量操作, 导入导出, 更好的错误恢复 |
| v0.1.6 | 📋 规划中 | 高级特性: 离线 Embedding, 混合检索, Lexical 双链编辑器 |
| v0.1.7 | 📋 规划中 | 发布准备: 安装包优化, 文档完善, 测试覆盖 |

### 33.2 开发顺序原则

按"正式架构优先，功能逐步落地"的方式开发：
1. 搭建正式工程架构
2. 实现核心 AppKernel + 数据库
3. 实现 ModelGateway 和 DeepSeek 集成
4. 实现 TaskQueue + Agent 管线
5. 实现 Review + WikiWriter + VersionManager
6. 完善 UI 和用户体验
7. 实现高级特性 (Embedding, Hybrid Search, 双链编辑器)
8. 测试、打包、文档

---

## 34. 验收标准

### 34.1 完整验收清单

#### 安装与启动
- [ ] 用户可以下载 Windows 安装包
- [ ] 双击安装后可以直接打开软件
- [ ] 不需要安装 Node.js / Python / Rust / SQLite
- [ ] 第一次启动进入初始化向导

#### 配置
- [ ] 用户可以配置 DeepSeek API Key
- [ ] 软件可以测试 DeepSeek 连接
- [ ] 软件可以测试文档附件能力
- [ ] API Key 使用系统凭据安全存储

#### 知识库管理
- [ ] 用户可以创建本地知识库
- [ ] 用户可以选择知识库模板
- [ ] 软件自动初始化 workspace 目录结构
- [ ] 用户可以创建多个知识库并切换
- [ ] 切换后数据隔离

#### 文件导入
- [ ] 用户可以上传 PDF / DOCX / Markdown / TXT / HTML
- [ ] 文档被保存到 raw/sources/documents/
- [ ] 软件生成 source_id 和 file_hash
- [ ] 图片上传后只保存为 asset, 不参与知识抽取
- [ ] 相同 hash 文件提示已存在

#### Agent Pipeline
- [ ] 软件构建 Prompt 并将文档附件提交给 DeepSeek
- [ ] DeepSeek 返回结构化 JSON
- [ ] 软件校验 JSON Schema
- [ ] 非法 JSON 可进入 repair 流程
- [ ] SourceIngestAgent 输出 source_summary + entities + concepts + claims + relationships
- [ ] ResolutionAgent 执行去重与消歧
- [ ] RelationshipAgent 建立标准化关系
- [ ] WikiUpdateAgent 生成 wiki_update_plan
- [ ] ReviewAgent 生成审阅任务和 Diff
- [ ] Agent Activity Timeline 正常显示

#### 审阅
- [ ] 用户可以在审阅中心查看 Diff
- [ ] 用户可以接受/拒绝/编辑后接受
- [ ] 每个修改说明: 改了什么, 为什么改, 基于哪个 source, 能否撤销
- [ ] 高风险操作必须审阅
- [ ] 审阅统计与实际一致

#### Wiki
- [ ] WikiWriter 写入 Markdown Wiki
- [ ] VersionManager 保存版本快照
- [ ] index.md 自动更新
- [ ] log.md 自动追加
- [ ] 用户可以浏览/编辑/预览 Wiki 页面
- [ ] 点击页面不再 os error 3
- [ ] 支持阅读/编辑/源码三模式

#### 问答
- [ ] 用户可以基于 Wiki 问答
- [ ] 回答带引用
- [ ] 显示使用的上下文
- [ ] 回答可以保存为 Wiki 页面

#### 搜索
- [ ] 支持标题/正文/alias 搜索
- [ ] 中英文搜索
- [ ] 搜索结果可点击打开

#### 图谱
- [ ] 图谱页面显示节点
- [ ] 即使无边也显示孤立节点
- [ ] 支持思维导图默认视图
- [ ] 支持"重建图谱"功能

#### 健康检查
- [ ] 检测一致性问题
- [ ] 支持一键修复可自动修复的问题
- [ ] 修复后问题数量减少

#### 安全
- [ ] 日志脱敏 API Key
- [ ] 软件防止重复应用 operation
- [ ] 软件防止覆盖用户手动编辑
- [ ] 软件支持页面版本查看和回滚
- [ ] 任务中断后可恢复

#### 打包
- [ ] 软件可在普通 Windows 电脑上安装运行
- [ ] 软件数据保存在本地用户目录
- [ ] 卸载软件不默认删除用户知识库
- [ ] 软件 UI 与设计规范一致

---

## 35. 附录：代码位置速查

### 前端关键文件

| 文件 | 用途 |
|------|------|
| [src/App.tsx](src/App.tsx) | 根组件, 路由, KB 初始化 |
| [src/main.tsx](src/main.tsx) | 入口, QueryClient + BrowserRouter |
| [src/pages/DashboardPage.tsx](src/pages/DashboardPage.tsx) | 首页 |
| [src/pages/WikiPage.tsx](src/pages/WikiPage.tsx) | Wiki 阅读/编辑 |
| [src/pages/ReviewPage.tsx](src/pages/ReviewPage.tsx) | 审阅中心 |
| [src/pages/ChatPage.tsx](src/pages/ChatPage.tsx) | 问答 |
| [src/pages/SearchPage.tsx](src/pages/SearchPage.tsx) | 搜索 |
| [src/pages/GraphPage.tsx](src/pages/GraphPage.tsx) | 知识图谱 |
| [src/pages/SourcesPage.tsx](src/pages/SourcesPage.tsx) | 文件管理 |
| [src/pages/ImportTasksPage.tsx](src/pages/ImportTasksPage.tsx) | 任务管线 |
| [src/pages/HealthCheckPage.tsx](src/pages/HealthCheckPage.tsx) | 健康检查 |
| [src/pages/SettingsPage.tsx](src/pages/SettingsPage.tsx) | 设置 |
| [src/pages/TaskDetailPage.tsx](src/pages/TaskDetailPage.tsx) | 任务详情 |
| [src/pages/SourcePreviewPage.tsx](src/pages/SourcePreviewPage.tsx) | Source 预览 |
| [src/pages/OnboardingPage.tsx](src/pages/OnboardingPage.tsx) | 初始化向导 |
| [src/components/layout/Sidebar.tsx](src/components/layout/Sidebar.tsx) | 左侧导航 |
| [src/components/layout/StatusBar.tsx](src/components/layout/StatusBar.tsx) | 底部状态栏 |
| [src/components/common/RightContextPanel.tsx](src/components/common/RightContextPanel.tsx) | 右侧面板 |
| [src/components/common/MarkdownRenderer.tsx](src/components/common/MarkdownRenderer.tsx) | Markdown 渲染 |
| [src/components/graph/MindMapView.tsx](src/components/graph/MindMapView.tsx) | 思维导图 |

### 后端关键文件

| 文件 | 用途 |
|------|------|
| [src-tauri/src/lib.rs](src-tauri/src/lib.rs) | Tauri 入口, 注册 commands + state |
| [src-tauri/src/core/app_kernel.rs](src-tauri/src/core/app_kernel.rs) | DI 容器 |
| [src-tauri/src/core/database_service.rs](src-tauri/src/core/database_service.rs) | SQLite 服务 |
| [src-tauri/src/core/secret_service.rs](src-tauri/src/core/secret_service.rs) | API Key 存储 |
| [src-tauri/src/core/workspace_service.rs](src-tauri/src/core/workspace_service.rs) | KB 目录管理 |
| [src-tauri/src/core/task_queue.rs](src-tauri/src/core/task_queue.rs) | 任务队列 |
| [src-tauri/src/core/event_bus.rs](src-tauri/src/core/event_bus.rs) | 事件推送 |
| [src-tauri/src/model/model_gateway.rs](src-tauri/src/model/model_gateway.rs) | 统一 LLM 网关 |
| [src-tauri/src/model/deepseek_client.rs](src-tauri/src/model/deepseek_client.rs) | DeepSeek HTTP 客户端 |
| [src-tauri/src/agents/coordinator.rs](src-tauri/src/agents/coordinator.rs) | Agent 调度器 |
| [src-tauri/src/agents/source_ingest.rs](src-tauri/src/agents/source_ingest.rs) | 文档摄入 Agent |
| [src-tauri/src/agents/resolution.rs](src-tauri/src/agents/resolution.rs) | 去重消歧 Agent |
| [src-tauri/src/agents/relationship.rs](src-tauri/src/agents/relationship.rs) | 关系建立 Agent |
| [src-tauri/src/agents/wiki_update.rs](src-tauri/src/agents/wiki_update.rs) | Wiki 更新 Agent |
| [src-tauri/src/agents/health_check.rs](src-tauri/src/agents/health_check.rs) | 健康检查 Agent |
| [src-tauri/src/wiki/wiki_writer.rs](src-tauri/src/wiki/wiki_writer.rs) | Wiki 安全写入 |
| [src-tauri/src/wiki/version_manager.rs](src-tauri/src/wiki/version_manager.rs) | 版本管理 |
| [src-tauri/src/wiki/path_service.rs](src-tauri/src/wiki/path_service.rs) | 路径规范化 |
| [src-tauri/src/wiki/index_service.rs](src-tauri/src/wiki/index_service.rs) | index.md 维护 |
| [src-tauri/src/wiki/log_service.rs](src-tauri/src/wiki/log_service.rs) | log.md 维护 |
| [src-tauri/src/review/review_engine.rs](src-tauri/src/review/review_engine.rs) | 审阅引擎 |
| [src-tauri/src/review/diff_engine.rs](src-tauri/src/review/diff_engine.rs) | Diff 引擎 |
| [src-tauri/src/graph/graph_service.rs](src-tauri/src/graph/graph_service.rs) | 图谱服务 |
| [src-tauri/src/search/full_text_search.rs](src-tauri/src/search/full_text_search.rs) | 全文搜索 |
| [src-tauri/src/search/candidate_search.rs](src-tauri/src/search/candidate_search.rs) | 候选检索 |
| [src-tauri/src/schema/json_schema_validator.rs](src-tauri/src/schema/json_schema_validator.rs) | JSON Schema 校验 |
| [src-tauri/src/schema/json_repair.rs](src-tauri/src/schema/json_repair.rs) | JSON 修复 |
| [src-tauri/src/recovery/recovery_check.rs](src-tauri/src/recovery/recovery_check.rs) | 启动恢复 |
| [src-tauri/src/recovery/workspace_reconcile.rs](src-tauri/src/recovery/workspace_reconcile.rs) | 一致性修复 |
| [src-tauri/src/prompts/prompt_builder.rs](src-tauri/src/prompts/prompt_builder.rs) | Prompt 构建 |
| [src-tauri/src/prompts/prompt_registry.rs](src-tauri/src/prompts/prompt_registry.rs) | Prompt 注册 |
| [src-tauri/src/skills/document_processor.rs](src-tauri/src/skills/document_processor.rs) | 文档分发器 |
| [src-tauri/src/db/schema.rs](src-tauri/src/db/schema.rs) | 数据库 Schema |
| [src-tauri/src/db/migrations.rs](src-tauri/src/db/migrations.rs) | 数据库迁移 |

---

> **文档结束**
>
> 本手册 v2.1 整合了 111.md 的系统架构设计（含 MarkItDown 子进程管道、ONNX 离线推理、RRF 混合检索、多 Agent 状态机）、开发文档 1/2/3 的产品需求与实现细节、v0.1.3/v0.1.4 的版本修复与升级记录、CLAUDE.md 的当前项目状态、implementation_details.md 的代码映射、OpenWebUI 扩展集成方案、MarkItDown 通用格式转换系统、多模型供应商统一网关架构、以及所有相关技术文档的完整内容。是 LLMWiki（智维 Wiki）项目从概念到交付的唯一权威参考。
>
> 最后更新: 2026-05-21
