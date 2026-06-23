# LLMWiki —— 大模型驱动的智能双链知识库

[![Version](https://img.shields.io/badge/version-0.2.2-blue)](https://github.com/LIU3254621731/llmwiki_dpk)
[![Tauri 2](https://img.shields.io/badge/Tauri-2.0-FFC131?logo=tauri)](https://tauri.app)
[![Rust](https://img.shields.io/badge/Rust-2021-ed411e?logo=rust)](https://www.rust-lang.org)
[![React](https://img.shields.io/badge/React-18-61DAFB?logo=react)](https://react.dev)
[![License](https://img.shields.io/badge/license-MIT-green)](./LICENSE)

> 你只负责导入文档，AI 自动抽取知识、建立关联、生成 Wiki，你审核即可。

**LLMWiki** 是一个本地优先、大模型驱动的个人知识管理桌面应用。它把软件工程中编译器多阶段管道思想迁移到知识管理领域：你的文档是「源代码」，AI Agent 管道是「编译器」，最终的互联 Markdown Wiki 是你的「编译产物」。

---

## 为什么做这个项目

传统双链笔记（Obsidian、Logseq）始终有一个痛点：**手动整理**。你导入一堆 PDF、网页、Word，然后逐个打标签、建链接、写摘要——「整理熵增」永远在增加。

LLMWiki 把「整理」这个动作从人转移给了 AI：

| | 传统笔记 | LLMWiki |
|---|---|---|
| **文档处理** | 拖进来放着 | AI 自动解析 + 提取知识 |
| **打标签** | 手动一条条打 | AI 自动命名实体、消歧、去重 |
| **建链接** | 手动 `[[]]` | AI 自动发现实体之间的关系 |
| **写 Wiki** | 手动整理 | AI 生成结构化页面，人只需审核 |
| **知识图谱** | 依赖插件，手动维护 | 自动从实体关系派生 |

---

## 核心功能

### 🤖 多 Agent AI 管道
四个专业 Agent 流水线作业：
- **Source Ingest** — 文档解析 + 知识抽取
- **Entity Resolution** — 实体消歧去重
- **Relationship Discovery** — 实体关联发现
- **Wiki Update** — 生成结构化的 Wiki 页面变更提案

### 📄 多格式文档解析
- PDF（含 Windows OCR 扫描件识别）
- Word / DOCX
- HTML / 网页
- Markdown
- 纯文本
- 通过 MarkItDown 支持 PPT、Excel 等格式

### ✅ Git-style 人工审核
- AI 生成的每一处 Wiki 变更都以 **Diff 对比** 呈现
- 支持逐条接受/拒绝、批量操作
- 原子文件写入（tmp + rename）防止数据损坏

### 🕸️ 知识图谱
- 自动从实体关系中构建知识图谱
- ReactFlow 实现交互式可视化
- 支持思维导图布局

### 🔍 全文搜索 + 向量检索
- SQLite FTS 全文索引
- **本地 ONNX Runtime** 运行 Embedding 模型（bge-small-zh-v1.5）
- 混合检索，0 网络消耗

### 🛡️ LLM 输出可靠性
- 自研 JSON Repair 引擎：修复 Python 字面量、单引号、格式偏差
- JSON Schema 校验形成「生成 → 修复 → 校验」三层防线

### 💬 AI 对话侧边栏
- 结合当前知识库上下文的问答
- 支持流式输出

### 🎨 现代化 UI
- 三栏布局：导航栏 + 内容区 + 对话侧边栏
- 暗色/亮色主题
- CodeMirror 6 编辑器（编辑/预览/分屏三模式）
- 标签页式多文档管理

---

## 技术架构

```
用户上传文档 (PDF/Word/HTML/...)
        │
        ▼
┌─ MarkItDown 统一格式转换层 ─┐
        │
        ▼
┌─ ONNX 本地 Embedding ───────┐  (0 Token 消耗)
        │
        ▼
┌─ 多 Agent 知识编译管道 ─────┐  (DeepSeek API)
│  Source Ingest → Resolution │
│  → Relationship → Wiki Update│
        │
        ▼
┌─ JSON Repair + Schema 校验 ─┐
        │
        ▼
┌─ Diff 审核工作台 ───────────┐  (人机交互层)
        │
        ▼
┌─ 互联 Markdown Wiki 输出 ──┐
```

### 桌面端架构

```
┌──────────────┐     ┌──────────────────────┐     ┌──────────────┐
│ IconSidebar  │     │    CenterStage        │     │ ChatSidebar  │
│  (导航栏)     │     │  (Dashboard / 文件     │     │  (AI 对话)    │
│              │     │   Wiki / 审核 / 图谱)   │     │              │
└──────────────┘     └──────────────────────┘     └──────────────┘
                      ┌──────────────────────┐
                      │    StatusBar (底部)    │
                      └──────────────────────┘
        ▲                       │
        │             Tauri IPC invoke()       │
        ▼                       ▼
┌─────────────────────────────────────────────────┐
│              Rust 后端 (Tauri 2)                  │
│  ┌──────────┐ ┌──────────┐ ┌──────────────────┐ │
│  │ AppKernel│ │TaskQueue │ │ ModelGateway     │ │
│  │ (DI 容器) │ │(取消令牌) │ │ (DeepSeek+ONNX)  │ │
│  └──────────┘ └──────────┘ └──────────────────┘ │
│  ┌──────────┐ ┌──────────┐ ┌──────────────────┐ │
│  │ SQLite   │ │ Graph    │ │ Review/Diff      │ │
│  │ (多库隔离) │ │ (petgraph)│ │ Engine          │ │
│  └──────────┘ └──────────┘ └──────────────────┘ │
└─────────────────────────────────────────────────┘
```

---

## 技术栈

| 层 | 技术 |
|---|---|
| **桌面框架** | Tauri 2 (Rust + WebView) |
| **前端** | React 18 + TypeScript + Vite |
| **样式** | Tailwind CSS + Radix UI + Framer Motion |
| **状态管理** | Zustand (14 个独立 Store) |
| **编辑器** | CodeMirror 6 (编辑/预览/分屏) |
| **知识图谱** | ReactFlow (前端) + petgraph (后端) |
| **后端语言** | Rust (Edition 2021) |
| **数据库** | SQLite (rusqlite, bundled, 每知识库独立 DB) |
| **LLM** | DeepSeek API（可切换兼容接口） |
| **本地 Embedding** | ONNX Runtime + tokenizers (bge-small-zh-v1.5) |
| **文档解析** | lopdf (PDF), scraper (HTML), zip (DOCX), Windows OCR |
| **序列化** | serde + serde_json + serde_yaml |
| **Rust 依赖** | tokio (async runtime), reqwest (HTTP), petgraph (图), strsim (相似度) |

---

## 快速开始

### 环境要求

- **Windows 10/11**
- **Node.js** ≥ 18
- **Rust** ≥ 1.75（通过 [rustup](https://rustup.rs) 安装）
- **DeepSeek API Key**（或兼容的 OpenAI 格式 API）

### 安装和运行

```bash
# 1. 克隆仓库
git clone https://github.com/LIU3254621731/llmwiki_dpk.git
cd llmwiki_dpk

# 2. 安装前端依赖
npm install

# 3. 启动开发模式（Tauri + Vite 热更新）
npm run tauri dev

# 4. 生产构建
npm run tauri build
```

首次启动后，在设置页面配置 DeepSeek API Key，即可创建知识库并导入文档。

### 开发命令

| 命令 | 说明 |
|---|---|
| `npm run dev` | 仅前端 Vite 开发服务器 (localhost:1420) |
| `npm run tauri dev` | 完整桌面应用（前端 + Rust 后端） |
| `npm run build` | 前端构建 (`tsc && vite build`) |
| `npm run tauri build` | 生产安装包 |
| `npm run test` | 运行单元测试 (Vitest) |
| `npm run lint` | TypeScript 类型检查 |

---

## 项目结构

```
llmwiki_dpk/
├── src/                          # React 前端
│   ├── components/
│   │   ├── views/                # 6 个主视图
│   │   │   ├── DashboardView     # 仪表盘
│   │   │   ├── FileExplorerView  # 文件浏览器
│   │   │   ├── WikiIndexView     # Wiki 索引
│   │   │   ├── ReviewWorkshopView# 审核工作台
│   │   │   ├── GraphView         # 知识图谱
│   │   │   └── SettingsView      # 设置
│   │   ├── layout/               # 应用壳 (IconSidebar, CenterStage, ChatSidebar)
│   │   ├── editor/               # CodeMirror 编辑器
│   │   ├── graph/                # 知识图谱可视化
│   │   ├── filebrowser/          # 文件树浏览器
│   │   └── common/               # 共享组件 (MarkdownRenderer 等)
│   ├── stores/                   # Zustand 状态管理 (14 个 Store)
│   ├── lib/                      # 类型、工具函数、Tauri 命令封装
│   └── test/                     # Vitest 单元测试
│
├── src-tauri/                    # Rust 后端
│   └── src/
│       ├── agents/               # AI Agent 管道
│       │   ├── coordinator.rs    # 总调度
│       │   ├── source_ingest.rs  # 文档解析 + 知识抽取
│       │   ├── resolution.rs     # 实体消歧
│       │   ├── relationship.rs   # 关系发现
│       │   └── wiki_update.rs    # Wiki 生成
│       ├── core/                 # 核心基础设施
│       │   ├── app_kernel.rs     # DI 容器
│       │   ├── task_queue.rs     # 任务队列 + 取消令牌
│       │   └── event_bus.rs      # 前端事件通知
│       ├── commands/             # Tauri 命令处理器 (100+)
│       ├── model/                # LLM 网关 (DeepSeekClient + ONNX)
│       ├── skills/               # 文档处理器 (PDF/OCR/DOCX/HTML/MD)
│       ├── wiki/                 # Wiki CRUD + 版本管理
│       ├── review/               # Diff 引擎 + 审核工作流
│       ├── graph/                # 知识图谱 (petgraph)
│       ├── search/               # 全文搜索
│       ├── dedup/                # 去重服务
│       ├── schema/               # JSON Schema 校验 + 修复
│       ├── db/                   # SQLite 数据层
│       ├── recovery/             # 崩溃恢复
│       └── prompts/              # Prompt 模板注册表
│
└── docs/                         # 项目文档
```

---

## 核心设计理念

### 1. 编译管道模式
将知识管理类比为代码编译：异构素材（PDF/Word/网页）→ MarkItDown 降维 → 多 Agent 编译 → 结构化 Wiki 输出。

### 2. 人机协作闭环
AI 负责繁重的「抽取 → 消歧 → 关联 → 生成」工作，人只在最终审核环节介入。Git-like Diff 视图让人能快速判断 AI 的建议是否合理。

### 3. 本地优先 + 云端增强
- **本地**：SQLite 存储、ONNX Embedding（0 网络消耗）
- **云端**：需要高推理能力时调用 DeepSeek API

### 4. 防御性工程
- JSON Repair 引擎防止 LLM 输出格式错误
- 原子文件写入（tmp + rename）防止崩溃丢数据
- Panic hook 将崩溃信息写入日志
- 任务取消令牌支持协作式取消
- 工作区健康检查 + 自动修复

---

## 版本历史

| 版本 | 日期 | 主要更新 |
|---|---|---|
| v0.2.2 | 2026-06 | 文档系统完善、dev-agents 编排、DashboardView、dedup 命令、Review Engine 修复 |
| v0.2.1 | 2026-06 | 稳定性版本：15+ bug 修复、JSON repair 增强、Canvas 管道、Web Search |
| v0.2.0 | 2026-05 | 核心管道完整可用 |
| v0.1.x | 2026-04 | 基础架构搭建、AI Agent 原型 |

---

## 许可

MIT License

---

## 作者

**刘汶林** — 独立全栈开发

- GitHub: [@LIU3254621731](https://github.com/LIU3254621731)

---

*如果你觉得这个项目有价值，请给一个 ⭐ Star！*
