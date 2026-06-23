# LLMWiki（智维 Wiki）功能描述手册

> **版本**: v2.0 | **日期**: 2026-05-21 | **覆盖范围**: 单页工作区重构后的全部功能
>
> 本手册以用户视角对 LLMWiki v2.0 的 Obsidian 风格单页工作区进行系统描述，涵盖布局体系、多标签编辑器、面板系统、AI 管线及全部交互方式与代码实现位置。

---

## 目录

1. [系统概览与工作区布局](#1-系统概览与工作区布局)
2. [标签栏与多标签编辑器](#2-标签栏与多标签编辑器)
3. [左侧边栏——资源管理器](#3-左侧边栏资源管理器)
4. [中央区域——主工作画布](#4-中央区域主工作画布)
5. [右侧边栏——AI 推理中枢](#5-右侧边栏ai-推理中枢)
6. [底部面板——AI 提案审阅](#6-底部面板ai-提案审阅)
7. [Chat 对话系统](#7-chat-对话系统)
8. [知识图谱与思维导图](#8-知识图谱与思维导图)
9. [数据来源与任务管线（后台隐式运行）](#9-数据来源与任务管线后台隐式运行)
10. [设置中心](#10-设置中心)
11. [全局快速导航](#11-全局快速导航)
12. [状态栏](#12-状态栏)
13. [Zustand 状态管理](#13-zustand-状态管理)
14. [附录：全部 IPC 命令索引](#14-附录全部-ipc-命令索引)
15. [代码位置索引](#15-代码位置索引)

---

## 1. 系统概览与工作区布局

### 1.1 设计理念

v2.0 将原先 15 个独立页面路由重构为 Obsidian 风格的单页工作区（Single-Page Workspace）。所有功能——Wiki 编辑、文件浏览、知识图谱、AI 对话、审阅——均在同一窗口内通过标签页与可折叠面板完成，无需页面跳转。

### 1.2 路由

| 路由 | 渲染内容 |
|------|----------|
| `/` | 工作区（主单页仪表盘） |
| `/settings` | 设置中心（含健康检查） |

所有原先的独立路由（`/wiki`、`/sources`、`/tasks`、`/review`、`/chat`、`/search`、`/graph`、`/health`、`/files`、`/editor`）已整合进工作区，以标签页或面板形式呈现。

### 1.3 工作区六区布局

```
┌──────────────────────────────────────────────────────────────────┐
│ TabBar: [🏠 欢迎] [📑 页面.md *] [🤖 Chat] [📊 图谱]            │
├────────────┬──────────────────────────────┬──────────────────────┤
│ 左侧边栏   │ 中央区域（主画布）           │ 右侧边栏（AI 中枢）  │
│ (280px)    │                              │                      │
│            │ MarkdownEditor /             │ Agent 活动           │
│ 🔍 快速搜索│ PDF 查看器 / 图谱 /          │ 本地 RAG             │
│ 📁 文件树  │ Chat / 欢迎仪表盘            │ 上下文面板           │
│ 🌐 技能    │                              │                      │
│            ├──────────────────────────────┤                      │
│            │ 底部面板（Git Diff 审阅）     │                      │
├────────────┴──────────────────────────────┴──────────────────────┤
│ 状态栏: 📚 我的知识库 | 📄 128 页面 | 🟢 健康 | v2.0            │
└──────────────────────────────────────────────────────────────────┘
```

### 1.4 面板控制

| 面板 | 默认状态 | 切换方式 |
|------|----------|----------|
| 左侧边栏 | 可见 | `Ctrl+B` / 点击左侧边缘手柄 |
| 右侧边栏 | 隐藏 | `Ctrl+J` / 点击右侧边缘手柄 / 上下文自动展开 |
| 底部面板 | 隐藏 | `Ctrl+Shift+R` / 点击状态栏审阅徽标 / 代码触发 |
| 底部面板高度 | 200px（可拖拽调节） | 拖拽分隔线 |

### 1.5 代码位置

- 根组件：[src/App.tsx](src/App.tsx#L1)
- 工作区布局：[src/components/layout/WorkspacePage.tsx](src/components/layout/WorkspacePage.tsx)（待创建）
- 左侧边栏：[src/components/layout/LeftSidebar.tsx](src/components/layout/LeftSidebar.tsx)（待创建）
- 右侧边栏：[src/components/layout/RightSidebar.tsx](src/components/layout/RightSidebar.tsx)（待创建）
- 中央区域：[src/components/layout/CenterArea.tsx](src/components/layout/CenterArea.tsx)（待创建）
- 底部面板：[src/components/layout/BottomPanel.tsx](src/components/layout/BottomPanel.tsx)（待创建）
- 状态栏：[src/components/layout/StatusBar.tsx](src/components/layout/StatusBar.tsx)

---

## 2. 标签栏与多标签编辑器

### 2.1 概述

标签栏位于工作区顶部，管理所有打开的标签页，是工作区的核心导航机制。不再通过路由跳转切换功能——所有内容通过标签页承载。

### 2.2 标签类型

| 类型 | TabType 值 | 说明 | 默认视图模式 |
|------|-----------|------|-------------|
| 欢迎 | `welcome` | 始终存在，钉住，显示仪表盘 | preview |
| 编辑器 | `editor` | Markdown 文件编辑标签 | split |
| Wiki 页面 | `wiki` | Wiki 页面编辑标签 | preview |
| 文件预览 | `file` | 普通文件预览标签 | preview |
| PDF 查看器 | `pdf_viewer` | PDF 文档查看标签 | preview |
| 知识图谱 | `graph` | 图谱可视化标签 | preview |
| Chat | `chat` | AI 对话标签 | preview |

### 2.3 标签操作

| 操作 | 方式 | 结果 |
|------|------|------|
| 打开标签 | 双击左侧文件树文件 / 右键菜单"在编辑器中打开" / 点击状态栏快捷入口 | `openFile()` 新建或聚焦已有标签 |
| 关闭标签 | 点击标签 × 按钮 / 右键菜单"关闭" | `closeTab()` 移除标签，如未保存提示 |
| 钉住标签 | 右键菜单"切换固定" | `togglePin()` 标签移至固定区，不可被批量关闭 |
| 关闭其他 | 右键菜单"关闭其他" | 除当前标签外全部关闭 |
| 关闭右侧 | 右键菜单"关闭右侧" | 关闭当前标签右侧所有标签 |
| 拖拽排序 | 鼠标按住标签拖拽 | `moveTab(fromIndex, toIndex)` 重排标签顺序 |
| 切换标签 | 单击标签 / `Ctrl+Tab` 顺序切换 | `setActiveTab()` |
| 左右滚动 | 点击左右箭头按钮（标签过多时） | 水平滚动标签栏 |

### 2.4 欢迎标签

- `tab:welcome` 始终存在，固定在标签栏最左侧
- 不可关闭，不可取消固定
- 渲染内容为欢迎仪表盘（第 4 章详细描述）
- 当所有其他标签关闭时，自动聚焦欢迎标签

### 2.5 编辑器标签视图模式

与 Obsidian 一致，编辑器标签支持三种视图模式：

| 模式 | viewMode 值 | 说明 |
|------|-----------|------|
| 编辑 | `edit` | 纯文本编辑器 + 行号 |
| 预览 | `preview` | 仅渲染 Markdown 内容（只读） |
| 分屏 | `split` | 左侧编辑区 + 右侧实时预览 |

模式切换方式：点击标签内视图模式按钮组，调用 `setTabViewMode(tabId, mode)`。

### 2.6 脏标记与保存

- 修改未保存时标签标题前显示圆点（`isDirty: true`）
- `Ctrl+S` 保存当前标签内容 → 调用 `save_wiki_page` 或 `save_workspace_file`
- 保存成功后调用 `markTabClean(tabId)` 消除脏标记
- 关闭带有脏标记的标签时弹出确认对话框

### 2.7 代码位置

- 标签栏组件：[src/components/editor/TabBar.tsx](src/components/editor/TabBar.tsx)
- 编辑器组件：[src/components/editor/MarkdownEditor.tsx](src/components/editor/MarkdownEditor.tsx)
- 标签状态管理：[src/stores/useEditorStore.ts](src/stores/useEditorStore.ts)

---

## 3. 左侧边栏——资源管理器

### 3.1 概述

左侧边栏宽度 280px（可拖拽调整），以垂直排列方式提供知识库切换、快速搜索、文件树浏览、技能开关四大功能区。

### 3.2 知识库切换器

位于左侧边栏顶部：

- **下拉选择框**：显示当前知识库名称 + 图标
- **下拉菜单**：列出所有已创建的知识库，点击切换
- **新建知识库入口**：菜单底部"＋ 新建知识库"按钮
- 切换知识库时触发：重新加载文件树、刷新仪表盘统计、清空旧审阅数据

**代码引用**：`useKBStore.setCurrentKB(kb)` → 触发各处 `useEffect` 数据重新加载

### 3.3 快速搜索

知识库切换器下方为搜索输入框：

- 输入文本实时过滤文件树（前端过滤，无需 IPC）
- 支持按文件名模糊匹配
- 清空搜索框恢复完整文件树
- 输入框右侧"×"按钮一键清空

### 3.4 文件树

**Header 工具栏**：

| 控件 | 功能 |
|------|------|
| 排序下拉 | 按名称（name）/ 修改时间（modified）/ 类型（type）排序 |
| 全部展开 | `expandAll()` 递归展开所有文件夹 |
| 全部折叠 | `collapseAll()` 折叠所有文件夹 |
| 刷新 | `refreshTree()` 重新加载文件树 |

**文件树组件**：

- 递归树结构，支持无限层级嵌套
- 缩进引导线（虚线边框连接父子节点）
- 16 种文件类型彩色图标：

| 文件类型 | 图标颜色 |
|----------|----------|
| PDF | 红色 |
| DOCX | 蓝色 |
| MD | 紫色 |
| TXT | 灰色 |
| HTML | 橙色 |
| PPTX | 橙红 |
| XLSX | 绿色 |
| CSV | 青绿 |
| JSON | 黄色 |
| XML | 棕色 |
| PNG/JPG/WEBP/GIF | 粉色 |
| 文件夹 | 金色 |

- 文件类型后缀小徽章（如 `.pdf`、`.docx`）

**文件状态指示器**：

| 图标 | 含义 |
|------|------|
| 绿色对勾 ✓ | AI 已分析完毕 |
| 旋转图标 ⟳ | AI 正在编译分析中 |
| 灰色圆点 ● | 未导入（仅在文件系统中，未触发 AI 分析） |

**右键上下文菜单**：

| 菜单项 | 功能 |
|--------|------|
| 在编辑器中打开 | `openFile()` 新建编辑器标签 |
| 查看 AI 编译日志 | 弹出 Popover 显示任务事件时间线 |
| 复制路径 | 复制文件相对路径到剪贴板 |
| 在文件资源管理器中显示 | 调用 `shell_open` 打开系统文件管理器 |

**键盘导航**：

| 按键 | 功能 |
|------|------|
| 上下方向键 | 在同级节点之间移动焦点 |
| 左右方向键 | 折叠/展开文件夹 |
| Enter | 打开选中文件 |
| F2 | 重命名选中文件 |

**悬停操作按钮**（鼠标悬停文件行时显示）：
- 重命名（铅笔图标）
- 删除（垃圾桶图标）

### 3.5 技能区域

文件树下方为技能开关区：

| 技能 | 说明 |
|------|------|
| 联网搜索 | 开关切换（DuckDuckGo），开启后 Chat 可触发网络搜索 |

### 3.6 代码位置

- 左侧边栏：[src/components/layout/LeftSidebar.tsx](src/components/layout/LeftSidebar.tsx)（待创建）
- 文件树：[src/components/filebrowser/FileTree.tsx](src/components/filebrowser/FileTree.tsx)
- 文件树 Header：[src/components/filebrowser/FileTreeHeader.tsx](src/components/filebrowser/FileTreeHeader.tsx)
- 文件树 Store：[src/stores/useFileTreeStore.ts](src/stores/useFileTreeStore.ts)
- 后端命令：[src-tauri/src/commands/file_tree.rs](src-tauri/src/commands/file_tree.rs)

---

## 4. 中央区域——主工作画布

### 4.1 概述

中央区域是内容渲染的主画布，根据当前活跃标签的类型渲染不同内容。使用 `Suspense` + 懒加载保证性能。

### 4.2 标签类型 → 渲染内容映射

| 活跃标签类型 | 渲染组件 | 说明 |
|-------------|----------|------|
| `welcome` | DashboardPage | 欢迎仪表盘 |
| `editor` / `wiki` | MarkdownEditor | Markdown 编辑器 |
| `pdf_viewer` | PDF 查看器 | MarkItDown 转换预览 |
| `graph` | GraphPage | 知识图谱 / 思维导图 |
| `chat` | ChatPage | AI 对话面板 |
| `file` | 文件预览 | 文本/Markdown/图片预览 |

### 4.3 欢迎仪表盘（Welcome Dashboard）

**知识库统计卡片行**（6 个卡片）：

| 卡片 | 数据项 | 点击行为 |
|------|--------|----------|
| Wiki 页面 | `kb.stats.pages` | 打开文件树，聚焦 wiki/ 目录 |
| 数据来源 | `kb.stats.sources` | 打开文件树，聚焦 raw/sources/ 目录 |
| 待审阅 | `kb.stats.pending_reviews` | 打开底部审阅面板 |
| 关系数量 | `kb.stats.relationships` | 打开图谱标签 |
| 图谱节点 | `kb.stats.graph_nodes` | 打开图谱标签 |
| 失败任务 | `kb.stats.failed_tasks` | 弹出任务列表 Popover |

**快捷操作按钮行**：

| 按钮 | 执行动作 |
|------|----------|
| 导入文件 | 触发系统文件选择器 → `upload_source_file` |
| 浏览 Wiki | 在文件树中展开 wiki/ 目录 |
| 开始问答 | 新建 Chat 标签 |
| 查看审阅 | 展开底部审阅面板 |
| 健康检查 | 跳转到设置中心 → 健康 Tab |

**知识库列表网格**：
- 所有知识库卡片（名称、模板类型、创建时间）
- 当前选中的高亮边框
- 点击切换知识库

**推荐操作区**（规划中）：
- 根据知识库当前状态动态推荐下一步操作
- 如："有 3 条 AI 知识提案待审阅"、"2 个页面存在断链"

### 4.4 Markdown 编辑器（编辑 / 预览 / 分屏模式）

当活跃标签类型为 `editor` 或 `wiki` 时渲染。

**编辑模式**（`viewMode: "edit"`）：
- 等宽字体 textarea 编辑器
- 左侧行号显示（自动对齐）
- `Ctrl+S` 保存
- 双链 `[[页面名]]` 语法高亮
- 支持 frontmatter YAML 语法高亮

**预览模式**（`viewMode: "preview"`）：
- 渲染 Markdown 为富文本
- 显示 frontmatter 元数据（标题、类型、别名、标签、来源、置信度、状态）
- 双链 `[[页面名]]` 渲染为可点击链接
- 代码块语法高亮（Shiki / Prism）
- 表格渲染、引用块、任务列表

**分屏模式**（`viewMode: "split"`）：
- 左侧编辑区 + 右侧实时预览
- 中间分隔线可拖拽调节比例
- 编辑区滚动时预览区同步滚动（syncScroll）

**用户操作**：

| 操作 | 方式 | 结果 |
|------|------|------|
| 编辑内容 | 在编辑区输入 | `updateTabContent()` 实时更新，脏标记出现 |
| 保存 | `Ctrl+S` | 调用 `save_wiki_page` 或 `save_workspace_file` |
| 切换视图模式 | 点击模式按钮 | `setTabViewMode(tabId, mode)` |
| 查看版本历史 | 右键菜单 → "版本历史" | 版本列表 Popover |
| 回滚版本 | 点击版本 → "回滚" | `rollback_wiki_page` |

### 4.5 PDF 查看器

当活跃标签类型为 `pdf_viewer` 时渲染：

- 调用 MarkItDown 将 PDF 转换为 Markdown 预览
- 显示转换进度条
- 渲染转换后的 Markdown 内容
- 顶部显示原始文件名 + "在资源管理器中打开"按钮

### 4.6 知识图谱画布

当活跃标签类型为 `graph` 时渲染（第 8 章详细描述）。

### 4.7 Chat 对话面板

当活跃标签类型为 `chat` 时渲染（第 7 章详细描述）。

### 4.8 代码位置

- 中央区域容器：[src/components/layout/CenterArea.tsx](src/components/layout/CenterArea.tsx)（待创建）
- 编辑器组件：[src/components/editor/MarkdownEditor.tsx](src/components/editor/MarkdownEditor.tsx)
- 仪表盘内容：[src/pages/DashboardPage.tsx](src/pages/DashboardPage.tsx)
- Markdown 渲染：[src/components/common/MarkdownRenderer.tsx](src/components/common/MarkdownRenderer.tsx)

---

## 5. 右侧边栏——AI 推理中枢

### 5.1 概述

右侧边栏是 AI 辅助工作的核心信息面板，包含三种子模式。通过顶部 Tab 切换，也可由系统根据活跃标签类型自动适配。

### 5.2 子模式切换

右侧边栏顶部有三个 Tab 按钮：

| Tab | 模式 | RightSidebarMode 值 | 说明 |
|-----|------|---------------------|------|
| 上下文 | Context Panel | `"context"` | 自动适配活跃标签类型 |
| Agent | Agent Activity | `"agent"` | Agent 实时状态与活动日志 |
| RAG | Local RAG | `"rag"` | 检索增强生成片段 |

点击 Tab 切换：`useAppStore.setRightSidebarMode(mode)`

### 5.3 上下文面板（Context Panel）

`RightSidebarMode = "context"` 时，通过 `useContextPanelStore.autoAdapt(tabType)` 自动适配显示内容。

**活跃标签为 Editor / Wiki 页面时**：

| 子面板 | ContextPanelMode | 内容 |
|--------|-----------------|------|
| 大纲 | `outline` | 页面的 Markdown 标题层级树（H1-H6），点击跳转 |
| 反向链接 | `backlinks` | 其他页面中引用当前页面的链接列表，显示匹配片段 |
| 本地图谱 | `local_graph` | 当前页面关联节点的微型图谱 |
| 页面信息 | `info` | Frontmatter 元数据 + 标签列表 + 别名列表 + 来源 + 版本历史 + AI 操作建议 |

**活跃标签为 PDF 查看器时**：

| 内容 |
|------|
| MarkItDown 提取进度条 |
| AI 摘要（source_summary：标题、类型、语言、简短摘要） |
| 关键要点列表 |
| 关联的 Wiki 页面链接 |

**活跃标签为图谱时**：

| 内容 |
|------|
| 选中节点的详细信息（类型、标签、路径、别名、摘要、入度/出度） |
| 选中边的详细信息（关系类型、置信度、Evidence 来源、引用状态） |
| 操作按钮："打开 Wiki 页面" / "设为中心节点" |

**活跃标签为 Chat 时**：

| 内容 |
|------|
| 对话上下文（问答范围：整个知识库 / 当前页面 / 指定标签） |
| 本次引用的 Wiki 页面列表 |
| Token 使用估算 |
| "保存为 Wiki"按钮 |

### 5.4 Agent 活动面板

`RightSidebarMode = "agent"` 时显示：

- **当前 Agent 状态**：Agent 名称、当前执行任务类型、已用时间
- **活动日志（时间线）**：按时间倒序排列的 Agent 事件
  - 事件时间
  - Agent 名称
  - 事件类型（task_started / stage_completed / task_finished / error）
  - 消息摘要
- **运行时统计**：已完成任务数 / 运行中任务数 / 失败任务数

**事件来源**：监听 `task-event` Tauri 事件实时更新。

### 5.5 本地 RAG 面板

`RightSidebarMode = "rag"` 时显示：

- **检索范围**：当前文件 / 当前知识库（可切换）
- **检索片段列表**：每个片段显示：
  - 来源页面标题（可点击跳转）
  - 匹配片段文本（关键词高亮）
  - 相关性分数
- 用于辅助理解当前编辑内容的上下文

### 5.6 代码位置

- 右侧边栏：[src/components/layout/RightSidebar.tsx](src/components/layout/RightSidebar.tsx)（待创建）
- 上下文面板：[src/components/common/RightContextPanel.tsx](src/components/common/RightContextPanel.tsx)
- 上下文 Store：[src/stores/useContextPanelStore.ts](src/stores/useContextPanelStore.ts)
- 应用布局 Store：[src/stores/useAppStore.ts](src/stores/useAppStore.ts)

---

## 6. 底部面板——AI 提案审阅

### 6.1 概述

底部面板以 Git-Diff 风格展示 AI 生成的 Wiki 修改提案，从工作区底部滑出。替代了原先独立的 `/review` 路由页面。

### 6.2 触发方式

| 触发方式 | 说明 |
|----------|------|
| 点击状态栏审阅徽标 | "⚡ N 条 AI 知识提案待确认" |
| `Ctrl+Shift+R` | 快捷键切换底部面板 |
| 编程触发 | `useAppStore.toggleBottomPanel()` |

### 6.3 视图布局

```
┌──────────────────────────────────────────────────────────────┐
│ 底部面板标题栏: [审阅中心] [筛选Tab] [批量操作按钮] [×关闭]  │
├──────────────────────────────────────────────────────────────┤
│ 审阅条目列表（可折叠卡片）                                    │
│ ┌──────────────────────────────────────────────────────────┐ │
│ │ ▼ Review #1  [create wiki]  风险:低  置信度:0.92         │ │
│ │ ┌──────────────────┬───────────────────────────────────┐ │ │
│ │ │ 当前内容 (旧)     │ AI 建议 (新)                      │ │ │
│ │ │                  │ + 新增内容（绿色高亮）             │ │ │
│ │ │                  │ - 删除内容（红色高亮）             │ │ │
│ │ │                  │ [[双链]]（橙色闪烁）               │ │ │
│ │ └──────────────────┴───────────────────────────────────┘ │ │
│ │ [接受] [拒绝] [修改原因] [引用来源: source_id + 位置]     │ │
│ └──────────────────────────────────────────────────────────┘ │
└──────────────────────────────────────────────────────────────┘
```

**面板高度**：默认 200px，可拖拽分隔线自由调节（`setBottomPanelHeight(height)`）。

### 6.4 筛选 Tab

| Tab | 筛选条件 | 对应原路由状态 |
|-----|----------|--------------|
| 待审 | `review_pending` | pending |
| 人工审核 | 高风险（必须人工判断） | manual_review |
| 已应用 | `applied` | applied |
| 已拒绝 | `rejected` | rejected |
| 失败 | `failed` | failed |
| 已跳过 | `skipped` | skipped |
| 全部 | 所有状态 | all |

### 6.5 审阅条目（可折叠卡片）

每张卡片：

**头部（始终可见）**：
- 展开/折叠箭头按钮
- 操作类型徽标（create / update / append / add_alias / add_relation / merge / delete）
- 风险等级标签（低风险绿色 / 中风险黄色 / 高风险红色）
- 置信度标签（数值 + 颜色）

**折叠内容（展开后）**：
- 当前内容 vs AI 建议的双栏 Diff
- 修改摘要
- 修改原因
- 引用来源（Source ID + 引用位置）
- 风险等级说明

**操作按钮**：

| 按钮 | 调用命令 | 后果 |
|------|----------|------|
| 接受 | `accept_review_item` | WikiWriter 写入，状态变更为 applied |
| 拒绝 | `reject_review_item` | 状态变更为 rejected |

**批量操作按钮**：

| 按钮 | 调用命令 | 说明 |
|------|----------|------|
| 接受所有低风险 | `accept_all_low_risk_review` | Balanced 模式下低风险自动应用 |
| 全部拒绝 | `reject_all_review` | 批量拒绝 |
| 重新生成 | `regenerate_review` | 重新触发 WikiUpdate + Review 流程 |

### 6.6 状态栏审阅徽标

状态栏中显示 `⚡ N条AI知识提案待确认`：
- N = `useReviewStore.pendingCount`
- 点击徽标 → 展开底部面板并聚焦审阅 Tab
- 实时更新（监听 `review-updated` 事件）

### 6.7 接受修改的完整流程

```
1. 用户在底部面板点击"接受"按钮
2. 前端调用 invoke("accept_review_item", { item_id, kb_id, kb_path })
3. Rust 后端 WikiWriter:
   a. 获取页面锁（文件互斥锁）
   b. 读取当前页面内容
   c. 检查 base_version_hash（防止覆盖用户手动编辑）
   d. 检查 operation_hash 幂等性（防止重复应用）
   e. 创建版本快照（VersionManager）
   f. 校验 update_plan
   g. 生成目标 Markdown
   h. 写入临时文件 (.tmp)
   i. 原子 rename 替换正式文件
   j. 更新 SQLite（wiki_pages / knowledge_items / aliases / relationships / graph_nodes / graph_edges / operations）
   k. 更新 index.md + 追加 log.md
   l. 释放页面锁
   m. 更新 review_item.status = applied
   n. 推送刷新事件
4. 前端收到事件，刷新审阅列表和状态栏徽标
```

### 6.8 代码位置

- 底部面板：[src/components/layout/BottomPanel.tsx](src/components/layout/BottomPanel.tsx)（待创建）
- 审阅内容组件（复用原 ReviewPage 的审阅卡片逻辑）
- 审阅 Store：[src/stores/useReviewStore.ts](src/stores/useReviewStore.ts)
- 后端命令：[src-tauri/src/commands/review.rs](src-tauri/src/commands/review.rs)
- 审阅引擎：[src-tauri/src/review/review_engine.rs](src-tauri/src/review/review_engine.rs)
- Diff 引擎：[src-tauri/src/review/diff_engine.rs](src-tauri/src/review/diff_engine.rs)

---

## 7. Chat 对话系统

### 7.1 概述

Chat 系统支持两种呈现方式：
1. **作为标签页**：双击左侧文件树 Chat 入口 → 新建 `chat` 类型标签，渲染在中央区域
2. **作为右侧边栏抽屉**：在右侧边栏中嵌入对话面板（宽度跟随右侧边栏）

两种模式共享同一套对话数据和 Store。

### 7.2 对话管理

**对话列表**（左侧树 / 顶部下拉中显示）：
- 对话条目（按更新时间倒序排列）
- "+ 新建对话"按钮
- 右键/点击菜单：重命名 / 删除
- 点击切换对话（加载历史消息）

**对话操作**：

| 操作 | 方式 | 结果 |
|------|------|------|
| 新建对话 | 点击"+ 新建对话" | `create_conversation`，创建新对话并切换 |
| 选择对话 | 点击对话条目 | 加载该对话的全部历史消息 |
| 重命名 | 右键 → 重命名 → 输入新标题 | `update_conversation_title` |
| 删除 | 右键 → 删除 → 确认 | `delete_conversation`，删除对话及所有消息 |

### 7.3 消息交互

**发送消息**：
- 在底部输入框输入问题
- 按 Enter 或点击发送按钮
- 支持流式输出（SSE → 50ms 节流 → IPC Event → 打字机效果）
- 点击"停止"按钮中止生成

**消息气泡**：

| 角色 | 对齐 | 背景 | 内容 |
|------|------|------|------|
| 用户 | 右对齐 | 蓝色气泡 | 纯文本 |
| AI | 左对齐 | 灰色气泡 | Markdown 渲染 + 引用标注 + Token 用量 |

**每条 AI 回复下方的操作按钮**：
- "保存为 Wiki"按钮 → 弹出标题确认 → `save_answer_as_wiki` → 创建 Wiki 页面

### 7.4 联网搜索集成

输入框旁的"联网搜索"开关：
- 开启后，AI 可触发 Web Search 补充上下文
- 搜索结果在下方可折叠区域展示
- 每条搜索结果：标题 + URL + 摘要 + 复选框
- 可 Fetch 网页内容（`fetch_web_page_content`）
- 可将搜索结果保存为 Source（`save_web_result_as_source`）

### 7.5 代码位置

- Chat 页面组件：[src/pages/ChatPage.tsx](src/pages/ChatPage.tsx)（可复用为标签内容或右侧面板内容）
- 后端命令：[src-tauri/src/commands/chat_history.rs](src-tauri/src/commands/chat_history.rs) + [src-tauri/src/commands/task.rs](src-tauri/src/commands/task.rs) → `run_query`, `save_answer_as_wiki`
- 联网搜索：[src-tauri/src/commands/web_search.rs](src-tauri/src/commands/web_search.rs)

---

## 8. 知识图谱与思维导图

### 8.1 概述

知识图谱以 `graph` 类型标签页形式在中央区域渲染。使用 ReactFlow 作为渲染引擎，替代原先独立的 `/graph` 路由。

### 8.2 视图布局

```
┌──────────────────────────────────────────────────────────────┐
│ [视图模式Tab] [搜索节点输入框] [布局算法选择下拉]              │
├──────────┬──────────────────────────────────┬────────────────┤
│ 左侧     │                                  │ 右侧           │
│ 筛选面板 │    图谱/导图画布 (ReactFlow)      │ 节点/边详情    │
│ (可折叠) │                                  │ 面板           │
│          │  - 节点（彩色圆点 + 标签）        │ (可折叠)       │
│ - 节点   │  - 边（带标签箭头）               │                │
│   类型   │  - 缩放/平移/拖拽                │ - 节点类型     │
│   勾选   │  - 单击查看 / 双击打开 / 右键菜单 │ - 关系列表     │
│          │                                  │ - 别名         │
│ - 置信度 │                                  │ - 统计信息     │
│   滑块   │                                  │ - 操作按钮     │
│          │                                  │                │
│ - 健康   │                                  │                │
│   指标   │                                  │                │
└──────────┴──────────────────────────────────┴────────────────┘
```

### 8.3 六种视图模式

| 模式 | 中心节点 | 展开逻辑 | 用途 |
|------|----------|----------|------|
| **全局 (global)** | 知识库 | 按类别分组展开（概念/实体/方法/数据集/来源/问题/Wiki） | 浏览整个知识网络 |
| **邻居 (neighbor)** | 选中节点 | 展开一度/二度邻居节点 | 查看关联上下文 |
| **来源 (source)** | 选中 Source | 展开该 Source 抽取的所有知识 | 追踪文档贡献 |
| **主题 (topic)** | 选中主题 | 展开主题下的所有概念/实体 | 主题聚合 |
| **时间线 (timeline)** | 无/知识库 | 按创建时间排列 | 查看知识增长 |
| **健康 (health)** | 知识库 | 高亮孤立节点/低置信度/缺失引用/冲突 | 知识质量控制 |

### 8.4 五种布局算法

| 算法 | 适用场景 |
|------|----------|
| **Force（力导向）** | 通用，节点自动分散 |
| **Dagre（层次布局）** | 层级关系清晰（is_a / part_of） |
| **Grid（网格布局）** | 整齐排列，适合截图导出 |
| **Radial（放射布局）** | 以某节点为中心向外辐射 |
| **MindMap（思维导图）** | 分类栏目式布局（类似 XMind），以知识库为中心向八个分类展开 |

### 8.5 思维导图模式 (MindMapView)

```
                  ┌── 概念 (concepts)
                  ├── 实体 (entities)
                  ├── 人物 (people)
    [我的知识库]  ├── 方法 (methods)
                  ├── 数据集 (datasets)
                  ├── 来源 (sources)
                  ├── 问题 (questions)
                  └── Wiki页面 (wiki_pages)
```

- 每个栏目有彩色头部标识
- 节点卡片显示类型色点 + Wiki/文件指示图标
- 单击查看详情（右侧面板更新），双击跳转 Wiki 页面（新建 editor 标签）

### 8.6 左侧筛选面板

- **节点类型筛选**（勾选框）：概念 / 实体 / 人物 / 方法 / 数据集 / 来源 / 问题 / Wiki 页面
- **置信度筛选**（滑块）：调节最低置信度阈值
- **健康指标**：节点总数 / 边总数 / 孤立节点数 / 低置信度节点数 / 冲突数 / 待审阅数 / 无引用数 / 平均度数 / Hub 节点

### 8.7 右侧详情面板

**节点详情**：
- 节点类型、标签、路径
- 别名列表
- 标签列表
- 摘要
- 来源数量 / 入度 / 出度
- 状态
- 操作按钮："打开 Wiki 页面"（新建 editor 标签）/ "设为中心节点" / "删除节点"

**边详情**：
- 边类型（关系类型）
- 置信度 + Evidence（来源 Source + 位置）
- 引用状态
- 操作按钮："删除边"

### 8.8 用户操作

| 操作 | 方式 | 结果 |
|------|------|------|
| 切换视图模式 | 点击顶部 Tab | 切换图布局和展开逻辑 |
| 切换布局算法 | 选择布局下拉 | 重新排列节点位置 |
| 搜索节点 | 输入关键词 → 下拉选择 | `search_graph_nodes`，定位并高亮目标节点 |
| 查看节点详情 | 单击节点 | 右侧面板更新节点信息 |
| 打开 Wiki 页面 | 双击节点 | 新建 editor 标签加载页面 |
| 右键菜单 | 右键节点 | 设为中心 / 删除等 |
| 筛选类型 | 勾选/取消节点类型 | 显示/隐藏特定类型节点 |
| 筛选置信度 | 拖动置信度滑块 | 过滤低置信度节点 |
| 添加节点 | "添加节点" → 搜索选择 | `add_graph_node` |
| 添加边 | "添加边" → 源 + 目标 + 关系 | `add_graph_edge` |
| 删除节点/边 | 详情面板删除按钮 | `delete_graph_node` / `delete_graph_edge` |

### 8.9 代码位置

- 图谱页面：[src/pages/GraphPage.tsx](src/pages/GraphPage.tsx)（作为标签内容渲染）
- 思维导图：[src/components/graph/MindMapView.tsx](src/components/graph/MindMapView.tsx)
- 后端命令：[src-tauri/src/commands/graph.rs](src-tauri/src/commands/graph.rs)
- 图谱服务：[src-tauri/src/graph/graph_service.rs](src-tauri/src/graph/graph_service.rs)

---

## 9. 数据来源与任务管线（后台隐式运行）

### 9.1 概述

v2.0 中，数据来源管理和任务管线不再有独立页面路由，而是隐式集成到工作区中：

- **数据来源** → 整合进左侧文件树（文件即 Source，状态指示器显示 AI 分析进度）
- **任务管线** → 在后台静默运行，用户通过 Agent 活动面板和右键"查看 AI 编译日志"感知进度
- **任务详情** → 以 Popover 形式弹出，不再独占页面

### 9.2 文件导入流程

```
1. 用户在左侧边栏点击"导入文件"或拖拽文件到文件树区域
2. 前端调用 invoke("upload_source_file", { kb_id, kb_path, file_path })
3. Rust 后端:
   a. 计算文件 SHA256 hash → 去重检查
   b. 复制文件到 raw/sources/documents/（文档）或 raw/assets/images/（图片）
   c. 创建 source 记录到 SQLite
   d. 文档类 → 创建 source_ingest 任务 → 触发 AI 管线
   e. 图片类 → 标记为 asset_only，不触发 AI
4. 前端刷新文件树，新文件出现并显示旋转图标（AI 编译中）
5. AI 完成分析后，状态指示器变为绿色对勾
```

支持的文件类型（14 种）：PDF / DOCX / MD / TXT / HTML / PPTX / XLSX / CSV / JSON / XML / PNG / JPG / WEBP / GIF

### 9.3 导入文件夹功能

**触发**：左侧边栏"导入文件夹"按钮 → 系统原生文件夹选择器

**预览对话框（ImportFolderDialog）**：
- **状态一（预览）**：文件选择复选框列表 + 统计卡片 + "保留子目录结构"开关 + 被跳过文件列表
- **状态二（导入中）**：进度条 + 当前文件名 + 成功/失败计数器（实时监听 `folder-import-progress` 事件）
- **状态三（完成）**：汇总统计 + 失败文件列表

### 9.4 查看任务详情（Popover）

右键文件 → "查看 AI 编译日志"：

弹出 Popover 显示：
- **12 阶段管线进度条**（每阶段彩色圆点）：

```
created → queued → locked → prompt_built → sent_to_model
  → model_returned → json_validating → json_valid → candidate_searching
  → resolution_running → relationship_running → update_plan_generating
  → review_generating → review_pending → applying → applied
```

- **事件时间线**：按时间倒序排列的事件列表
- **操作按钮**：重试 / 取消 / 继续（根据任务状态动态显示）
- **中间文件**（可折叠）：`input.json` / `prompt.md` / `model_raw_response.txt` / 各阶段输出 JSON / `error.log`

### 9.5 Agent 活动可见性

- 右侧边栏 Agent 模式：实时状态 + 活动日志
- 状态栏 Agent 指示器：有活动任务时显示旋转动画
- 状态栏审阅徽标：有待审阅项时显示数量

### 9.6 中断任务通知

当检测到中断任务时，在工作区顶部显示横幅："有 N 个中断任务需要关注"，提供"查看详情"按钮。

### 9.7 代码位置

- 文件导入：[src-tauri/src/commands/source.rs](src-tauri/src/commands/source.rs)
- 任务管线：[src-tauri/src/commands/task.rs](src-tauri/src/commands/task.rs)
- 任务队列：[src-tauri/src/core/task_queue.rs](src-tauri/src/core/task_queue.rs)
- 导入对话框：[src/components/filebrowser/ImportFolderDialog.tsx](src/components/filebrowser/ImportFolderDialog.tsx)
- Source 预览 Popover：[src/pages/SourcePreviewPage.tsx](src/pages/SourcePreviewPage.tsx)（内容逻辑复用，以 Popover 呈现）

---

## 10. 设置中心

### 10.1 路由

`/settings` — 设置中心（独立路由，非工作区标签）

### 10.2 视图布局

```
┌──────────┬──────────────────────────────────────────┐
│ 设置导航 │      设置内容区（右侧）                   │
│ (左侧)   │                                          │
│          │  每个区域为可折叠卡片                     │
│ 模型配置 │  包含表单字段 + 操作按钮                  │
│ 知识库   │                                          │
│ 网络搜索 │                                          │
│ 文档转换 │                                          │
│ 知识库   │                                          │
│   健康   │  ← NEW: 整合原 /health 路由              │
│ 危险区域 │                                          │
└──────────┴──────────────────────────────────────────┘
```

### 10.3 六大设置区域

**区域 1：模型配置**

| 字段 | 类型 | 说明 |
|------|------|------|
| API Base URL | 文本输入 | 模型服务地址（默认 `https://api.deepseek.com`） |
| API Key | 密码输入 | 存储到 Windows Credential Manager，输入不可见 |
| Chat 模型 | 文本输入 | 对话/写作模型名称 |
| Reasoner 模型 | 文本输入 | 推理模型名称 |
| Temperature | 滑块 (0.0-2.0) | 模型温度参数 |
| Max Tokens | 数字输入 (1-65536) | 最大输出 Token |
| Timeout | 数字输入 (1-600) | 请求超时（秒） |
| Retry Count | 数字输入 | 最大重试次数 |
| Stream | 开关 | 是否启用流式输出 |

**模型配置管理**：
- 保存当前配置 / 另存为配置文件（Model Profile）/ 加载配置 / 删除配置

**连接测试按钮**：
- 测试连接（`test_connection`）/ 测试 JSON 输出（`test_json_output`）/ 测试文档附件（`test_document_attachment`）/ 检查 API 状态（`check_api_key_status`）

**区域 2：知识库设置**

| 字段 | 类型 | 说明 |
|------|------|------|
| 知识库名称 | 文本输入 | 修改 KB 显示名称 |
| 语言 | 下拉选择 | zh-CN / en |
| 审阅模式 | 下拉选择 | Strict（严格）/ Balanced（平衡，默认）/ Auto（自动） |

**区域 3：网络搜索**

| 字段 | 类型 | 说明 |
|------|------|------|
| 搜索引擎 | 下拉选择 | DuckDuckGo / SearXNG / Brave / Bing |
| 最大结果数 | 滑块 (1-20) | 每次搜索返回的结果数量 |
| SearXNG URL | 文本输入 | SearXNG 实例地址 |
| Brave API Key | 密码输入 | Brave Search API 密钥 |
| Bing API Key | 密码输入 | Bing Search API 密钥 |
| Bing Endpoint | 文本输入 | Bing API 服务端点 |

**区域 4：文档转换**

| 功能 | 说明 |
|------|------|
| MarkItDown 状态检查 | 显示是否已安装、Python 是否可用 |
| 安装 MarkItDown | 点击按钮触发 `retry_markitdown_install` |
| 状态描述 | 显示当前可用性描述和帮助信息 |

**区域 5：知识库健康**（NEW — 整合原 `/health` 路由）

双 Tab 子布局：

**Tab 5a — 系统健康 (System Health)**：
- "运行检查"按钮 → 调用 `run_health_check_structured`
- 统计概览条：问题总数 / 严重 / 警告 / 信息
- 问题列表（按严重程度排序，每项含修复按钮）：

| 严重程度 | 标签颜色 | 示例 |
|----------|----------|------|
| error | 红色 | 数据丢失风险、文件缺失且无快照 |
| warning | 橙色 | 数据不一致但可恢复 |
| info | 蓝色 | 统计异常、建议优化 |

**快速修复工具栏**：

| 按钮 | 命令 | 功能 |
|------|------|------|
| 修复 Wiki 路径 | `repair_all_wiki_paths` | 自动修复 wiki/wiki 重复路径 |
| 重建图谱 | `sync_graph_data` | 从 knowledge_items 重建 graph_nodes/edges |
| 重建 Wiki 索引 | `sync_wiki_index_from_markdown` | 扫描 Markdown 文件同步到 wiki_pages 表 |
| 恢复检查 | `run_recovery_check` | 检查中断任务 |
| 重建预览 | `rebuild_all_previews` | 重建所有 Source Preview 缓存 |

**Tab 5b — 数据一致性 (Data Reconciliation)**：
- "运行检查"按钮 → 调用 `run_reconcile`
- 断链页面（文件缺失但 DB 有记录）→ 恢复自快照 / 删除记录 / 标记 broken
- 孤立 Source（文件存在但未关联 Wiki）→ 触发分析
- DB/文件系统不一致 → 修复建议

**区域 6：危险区域**

| 按钮 | 功能 | 确认要求 |
|------|------|----------|
| 重置所有数据 | 删除所有知识库和记录 | 二次确认 + 输入确认文本 |

### 10.4 代码位置

- 设置页面：[src/pages/SettingsPage.tsx](src/pages/SettingsPage.tsx)
- 健康检查组件：[src/pages/HealthCheckPage.tsx](src/pages/HealthCheckPage.tsx)（内容逻辑复用为设置子 Tab）
- 模型配置命令：[src-tauri/src/commands/config.rs](src-tauri/src/commands/config.rs)
- 健康检查命令：[src-tauri/src/commands/task.rs](src-tauri/src/commands/task.rs)
- 密钥服务：[src-tauri/src/core/secret_service.rs](src-tauri/src/core/secret_service.rs)
- 恢复检查：[src-tauri/src/recovery/recovery_check.rs](src-tauri/src/recovery/recovery_check.rs) + [src-tauri/src/recovery/workspace_reconcile.rs](src-tauri/src/recovery/workspace_reconcile.rs)

---

## 11. 全局快速导航

### 11.1 快速切换器 (QuickSwitcher)

**触发**：`Ctrl+O` / `Ctrl+P`

**功能**：
- 全屏居中弹窗模糊搜索对话框
- 同时搜索 Wiki 页面 + 工作区文件
- 输入字符实时过滤（字符级别模糊匹配标题/路径/名称）
- 键盘上下选择，Enter 确认打开：
  - Wiki 页面 → 新建 `wiki` 类型 editor 标签
  - 工作区文件 → 新建 `editor` 或 `file` 类型标签
- 搜索结果分组显示：Wiki 页面组 / 文件组

**数据加载**：
- `invoke("list_wiki_pages")` — 所有 Wiki 页面
- `invoke("get_file_tree")` — 所有文件

### 11.2 命令面板 (CommandPalette)

**触发**：`Ctrl+Shift+P`

**功能**：
- 全屏居中弹窗命令搜索对话框
- 内置命令列表（模糊过滤）：

| 分类 | 命令 |
|------|------|
| 导航 | 仪表盘、设置中心 |
| 标签 | 关闭当前标签、关闭所有标签、重新打开已关闭标签 |
| 面板 | 切换左侧边栏、切换右侧边栏、切换底部审阅面板 |
| 文件 | 导入文件、导入文件夹、新建文件、新建文件夹 |
| 视图 | 切换编辑/预览/分屏模式 |
| 主题 | 切换深色/浅色主题 |
| Chat | 新建对话 |
| 图谱 | 打开知识图谱、全局视图、思维导图视图 |
| 健康 | 运行健康检查 |

### 11.3 全局键盘快捷键

| 快捷键 | 功能 |
|--------|------|
| `Ctrl+O` / `Ctrl+P` | 快速切换器（文件/Wiki 模糊搜索） |
| `Ctrl+Shift+P` | 命令面板 |
| `Ctrl+S` | 保存当前编辑器标签内容 |
| `Ctrl+B` | 切换左侧边栏 |
| `Ctrl+J` | 切换右侧边栏 |
| `Ctrl+Shift+R` | 切换底部审阅面板 |
| `Ctrl+W` | 关闭当前标签 |
| `Ctrl+Tab` | 切换到下一个标签 |

### 11.4 代码位置

- 全局键盘处理：[src/components/quicknav/GlobalKeyboardHandler.tsx](src/components/quicknav/GlobalKeyboardHandler.tsx)
- 快速切换器：[src/components/quicknav/QuickSwitcher.tsx](src/components/quicknav/QuickSwitcher.tsx)
- 命令面板：[src/components/quicknav/CommandPalette.tsx](src/components/quicknav/CommandPalette.tsx)
- Store：[src/stores/useQuickNavStore.ts](src/stores/useQuickNavStore.ts)

---

## 12. 状态栏

### 12.1 概述

状态栏位于工作区底部，固定高度 28px，显示核心信息的精简视图。

### 12.2 信息项（从左到右）

| 信息项 | 数据来源 | 说明 |
|--------|----------|------|
| 📚 知识库名称 | `useKBStore.currentKB.name` | 当前选中的 KB 名称 |
| 📄 页面数 | `useKBStore.stats.pages` | Wiki 页面总数 |
| 📦 Source 数 | `useKBStore.stats.sources` | 数据来源文件总数 |
| 🟢🟡🔴 健康指示器 | `useKBStore.stats.health` | green=健康 / yellow=警告 / red=严重（点击跳转到设置中心健康 Tab） |
| ⚡ N条待审 | `useReviewStore.pendingCount` | 审阅徽标，点击展开底部面板 |
| ⟳ Agent 活动指示器 | 监听 `agent-activity` 事件 | 有活动任务时显示旋转动画 |
| 版本号 | 编译常量 | 如 v2.0 |
| 当前时间 | `new Date()` 定时刷新 | HH:MM:SS 格式 |

### 12.3 数据刷新

- `kb-stats-changed` 事件 → 更新 KB 统计
- `agent-activity` 事件 → 更新 Agent 指示器
- `review-updated` 事件 → 更新审阅徽标计数

### 12.4 代码位置

- 状态栏：[src/components/layout/StatusBar.tsx](src/components/layout/StatusBar.tsx)

---

## 13. Zustand 状态管理

### 13.1 Store 一览

| Store | 文件 | 关键状态 | 用途 |
|-------|------|----------|------|
| `useKBStore` | [src/stores/useKBStore.ts](src/stores/useKBStore.ts) | `currentKB`, `knowledgeBases`, `stats` | 知识库选择与元数据 |
| `useAppStore` | [src/stores/useAppStore.ts](src/stores/useAppStore.ts) | `leftSidebarVisible`, `rightSidebarVisible`, `rightSidebarMode`, `bottomPanelVisible`, `bottomPanelHeight`, `reviewBadgeCount`, `sidebarCollapsed` | 面板可见性与布局 |
| `useEditorStore` | [src/stores/useEditorStore.ts](src/stores/useEditorStore.ts) | `openTabs`, `activeTabId`, `tabPinned`, `viewMode` | 多标签编辑器系统 |
| `useFileTreeStore` | [src/stores/useFileTreeStore.ts](src/stores/useFileTreeStore.ts) | `files`, `expandedFolders`, `selectedFile`, `sortBy` | 文件树状态与操作 |
| `useContextPanelStore` | [src/stores/useContextPanelStore.ts](src/stores/useContextPanelStore.ts) | `visible`, `mode`, `context`, `sourceTabType`, `autoAdapt()` | 右侧上下文面板自动适配 |
| `useReviewStore` | [src/stores/useReviewStore.ts](src/stores/useReviewStore.ts) | `pendingCount`, `pendingItems`, `loading` | 审阅/DIFF 状态管理 |
| `useModelStore` | [src/stores/useModelStore.ts](src/stores/useModelStore.ts) | `config`, `selectedProfile` | 模型配置 |
| `useQuickNavStore` | [src/stores/useQuickNavStore.ts](src/stores/useQuickNavStore.ts) | `quickSwitcherOpen`, `commandPaletteOpen` | 快速导航弹窗开关 |
| `useThemeStore` | [src/stores/useThemeStore.ts](src/stores/useThemeStore.ts) | `theme` | 深色/浅色主题 |

### 13.2 Store 间数据流

```
useKBStore (currentKB 变化)
  ├→ useFileTreeStore.loadFileTree(kbId, kbPath)    // 重新加载文件树
  ├→ useReviewStore.loadPendingReviews(kbId)          // 重新加载审阅
  ├→ useEditorStore.openTabs 中关闭 wiki/file 标签    // 清除非持久标签
  └→ useAppStore.setReviewBadgeCount(0)              // 重置审阅徽标

useAppStore (面板切换)
  ├→ 左侧边栏: leftSidebarVisible → LeftSidebar 组件渲染
  ├→ 右侧边栏: rightSidebarVisible → RightSidebar 组件渲染
  └→ 底部面板: bottomPanelVisible → BottomPanel 组件渲染

useEditorStore (标签操作)
  └→ useContextPanelStore.autoAdapt(tabType)         // 切换标签时自动适配上下文
```

---

## 14. 附录：全部 IPC 命令索引

### 14.1 知识库管理 (Commands: workspace.rs)

| 命令 | 参数 | 返回 | 说明 |
|------|------|------|------|
| `list_knowledge_bases` | 无 | KB 列表 | 列出所有知识库 |
| `create_knowledge_base` | name, template_name, base_path | KB 对象 | 创建新知识库 |
| `delete_knowledge_base` | kb_id | () | 删除知识库及所有数据 |
| `update_knowledge_base` | kb_id, name, template_name, language?, review_mode? | KB 对象 | 更新 KB 元数据 |
| `get_kb_stats` | kb_id | 统计 JSON | 获取 KB 各项计数 |
| `init_workspace_dirs` | kb_path | () | 初始化工作区目录结构 |
| `reset_all_data` | 无 | 成功消息 | 删除所有数据 |

### 14.2 Wiki 页面 (Commands: wiki.rs)

| 命令 | 参数 | 返回 | 说明 |
|------|------|------|------|
| `list_wiki_pages` | kb_id | 页面列表 | 列出所有 Wiki 页面 |
| `get_wiki_page_content` | kb_path, page_path | Markdown 文本 | 读取页面内容 |
| `save_wiki_page` | kb_id, kb_path, page_type, title, content, page_path? | () | 创建/更新页面 |
| `delete_wiki_page` | kb_id, kb_path, page_path | () | 删除页面 |
| `get_wiki_page_versions` | kb_id, page_path | 版本列表 | 获取版本历史 |
| `rollback_wiki_page` | kb_id, kb_path, version_id | () | 回滚到指定版本 |
| `get_index_content` | kb_path | Markdown 文本 | 读取 index.md |
| `get_log_content` | kb_path | Markdown 文本 | 读取 log.md |

### 14.3 文件浏览 (Commands: file_tree.rs)

| 命令 | 参数 | 返回 | 说明 |
|------|------|------|------|
| `get_file_tree` | kb_id, kb_path | 文件树 JSON | 获取文件树 |
| `scan_workspace_files` | kb_id, kb_path | 文件树 JSON | 扫描并更新索引 |
| `list_files` | kb_id, kb_path, filter, search | 文件列表 | 筛选文件 |
| `get_file_detail` | kb_id, kb_path, relative_path | 文件详情 | 单文件元数据 |
| `get_workspace_file_preview` | kb_id, kb_path, relative_path | 预览 JSON | 文件内容预览 |
| `create_workspace_file` | kb_path, relative_path | {success} | 创建文件 |
| `create_workspace_folder` | kb_path, relative_path | {success} | 创建文件夹 |
| `save_workspace_file` | kb_path, relative_path, content | {success} | 保存文件内容 |
| `delete_workspace_file` | kb_path, relative_path | {success} | 删除文件/文件夹 |
| `rename_workspace_file` | kb_path, old_path, new_path | {success} | 重命名/移动 |

### 14.4 Source 管理 (Commands: source.rs)

| 命令 | 参数 | 返回 | 说明 |
|------|------|------|------|
| `upload_source_file` | kb_id, kb_path, file_path | Source 信息 | 上传并导入文件 |
| `batch_import_sources` | kb_id, kb_path, file_paths | 结果统计 | 批量导入 |
| `scan_import_folder` | folder_path | 文件夹扫描结果 | 预览文件夹内容 |
| `import_folder` | kb_id, kb_path, folder_path, preserve_structure, selected_files? | 结果统计 | 导入整个文件夹 |
| `list_sources` | kb_id | Source 列表 | 列出所有源 |
| `get_source_detail` | source_id | Source 详情 | 获取基本信息 |
| `get_source_detail_v2` | source_id | 完整详情 | 获取含关联信息的详情 |
| `get_source_summary` | source_id | 提取文本 | 获取文本内容 |
| `delete_source` | source_id | () | 删除 Source |
| `reimport_source` | source_id | task_id | 重新分析 |
| `parse_document_text` | file_path, file_type | 解析结果 | 解析文档 |
| `get_supported_file_types` | 无 | 类型列表 | 获取支持的文件类型 |

### 14.5 Source Preview (Commands: source_preview.rs)

| 命令 | 参数 | 返回 | 说明 |
|------|------|------|------|
| `get_source_preview` | source_id, kb_path | Preview JSON | 获取预览 |
| `generate_source_preview` | source_id, kb_path | Preview JSON | 生成/刷新预览 |
| `rebuild_all_previews` | kb_id, kb_path | 结果 | 全部重建 |

### 14.6 任务管线 (Commands: task.rs)

| 命令 | 参数 | 返回 | 说明 |
|------|------|------|------|
| `list_tasks` | kb_id | 任务列表 | 列出所有任务 |
| `list_tasks_filtered` | kb_id, status_filter | 任务列表 | 按状态筛选 |
| `get_task_detail` | task_id | 任务详情 | 获取单任务 |
| `get_task_events` | task_id | 事件列表 | 获取事件时间线 |
| `get_task_files` | kb_id, task_id | 文件内容 | 获取中间文件 |
| `get_interrupted_tasks` | kb_id | 任务列表 | 获取中断任务 |
| `get_unhandled_failed_count` | kb_id | 数量 | 未处理失败计数 |
| `run_source_ingest` | kb_id, source_id | task_id | 触发文档摄入 |
| `run_query` | kb_id, question, scope | 回答文本 | 执行问答 |
| `save_answer_as_wiki` | kb_id, kb_path, title, content | () | 保存回答为 Wiki |
| `retry_task` | task_id | () | 重试任务 |
| `cancel_task` | task_id | () | 取消任务 |
| `resume_task` | task_id | () | 恢复任务 |
| `archive_task` | task_id | () | 归档任务 |
| `handle_failed_task` | task_id | () | 标记失败已处理 |

### 14.7 审阅 (Commands: review.rs)

| 命令 | 参数 | 返回 | 说明 |
|------|------|------|------|
| `get_pending_reviews` | kb_id | 审阅列表 | 列出审阅项 |
| `get_review_detail` | review_id | 审阅详情 | 获取单项审阅 |
| `accept_review_item` | item_id, kb_id, kb_path | 结果 | 接受修改 |
| `reject_review_item` | item_id | () | 拒绝修改 |
| `delete_review_item` | item_id | () | 删除审阅项 |
| `accept_all_low_risk_review` | review_id, kb_id, kb_path | 统计 | 批量接受低风险 |
| `reject_all_review` | review_id | () | 全部拒绝 |
| `regenerate_review` | review_id | () | 重新生成审阅 |

### 14.8 对话 (Commands: chat_history.rs)

| 命令 | 参数 | 返回 | 说明 |
|------|------|------|------|
| `list_conversations` | kb_id | 对话列表 | 列出对话 |
| `create_conversation` | kb_id, title? | 对话对象 | 新建对话 |
| `get_conversation_messages` | conversation_id | 消息列表 | 加载消息 |
| `save_message` | conversation_id, role, content, citations? | 消息对象 | 保存消息 |
| `update_conversation_title` | conversation_id, title | () | 重命名 |
| `delete_conversation` | conversation_id | () | 删除对话 |

### 14.9 图谱 (Commands: graph.rs)

| 命令 | 参数 | 返回 | 说明 |
|------|------|------|------|
| `get_graph_data` | kb_id | 图谱数据 | 获取完整图谱 |
| `sync_graph_data` | kb_id | 同步结果 | 从 Wiki 同步 |
| `get_graph_stats` | kb_id | 统计 | 仅统计信息 |
| `search_graph_nodes` | kb_id, keyword | 节点列表 | 搜索节点 |
| `get_node_relations` | kb_id, node_id | 关系列表 | 获取节点关系 |
| `add_graph_node` | kb_id, label, node_type, path? | 节点对象 | 添加节点 |
| `delete_graph_node` | kb_id, node_id | () | 删除节点 |
| `add_graph_edge` | kb_id, source_id, target_id, relation | 边对象 | 添加边 |
| `delete_graph_edge` | kb_id, edge_id | () | 删除边 |

### 14.10 搜索 (Commands: search.rs)

| 命令 | 参数 | 返回 | 说明 |
|------|------|------|------|
| `full_text_search` | kb_id, kb_path, query | 搜索结果 | 全文搜索 |

### 14.11 健康检查 (Commands: task.rs 健康相关)

| 命令 | 参数 | 返回 | 说明 |
|------|------|------|------|
| `run_health_check` | kb_id | 报告文本 | 文本格式检查 |
| `run_health_check_structured` | kb_id, kb_path | 结构化结果 | 结构化检查 |
| `run_reconcile` | kb_id, kb_path | 修复结果 | 一致性检查 |
| `run_recovery_check` | kb_id | 恢复列表 | 恢复检查 |
| `repair_all_wiki_paths` | kb_id, kb_path | 修复统计 | 修复路径 |
| `sync_wiki_index_from_markdown` | kb_id, kb_path | 同步统计 | 同步索引 |
| `recover_page_from_snapshot` | kb_id, kb_path, page_path | 成功消息 | 恢复页面 |
| `delete_broken_page_record` | kb_id, page_id | () | 删除 broken 记录 |
| `mark_page_broken` | kb_id, page_path | () | 标记 broken |

### 14.12 模型配置 (Commands: config.rs)

| 命令 | 参数 | 返回 | 说明 |
|------|------|------|------|
| `get_deepseek_config` | 无 | 配置对象 | 获取当前配置 |
| `save_deepseek_config` | base_url, api_key, chat_model, reasoner_model, temperature, max_tokens, timeout, retry_count, stream | () | 保存配置 |
| `test_connection` | 无 | 测试结果 | 测试连通性 |
| `test_json_output` | 无 | 测试结果 | 测试 JSON 输出 |
| `test_document_attachment` | 无 | 测试结果 | 测试文档附件 |
| `check_api_key_status` | 无 | 状态 JSON | 检查配置完整性 |
| `list_model_profiles` | 无 | 配置列表 | 列出已保存配置 |
| `save_model_profile` | name, provider, base_url, model_name, api_key, role, temperature, max_tokens, timeout, retry_count | profile_id | 另存配置 |
| `apply_model_profile` | profile_id | () | 应用已保存配置 |
| `delete_model_profile` | profile_id | () | 删除已保存配置 |

### 14.13 网络搜索 (Commands: web_search.rs)

| 命令 | 参数 | 返回 | 说明 |
|------|------|------|------|
| `web_search` | query, engine, max_results | 搜索结果 | 执行搜索 |
| `get_web_search_config` | 无 | 配置对象 | 获取搜索配置 |
| `save_web_search_config` | engine, max_results, searxng_url, brave_api_key, bing_api_key, bing_endpoint | () | 保存搜索配置 |
| `fetch_web_page_content` | url | 网页内容 | 抓取网页 |
| `save_web_result_as_source` | kb_id, kb_path, title, content, format | Source 信息 | 保存网页为 Source |

### 14.14 工具 (Commands: utils.rs)

| 命令 | 参数 | 返回 | 说明 |
|------|------|------|------|
| `shell_open` | path | () | 系统打开文件/目录 |
| `get_markitdown_status` | 无 | 状态 JSON | 检查 MarkItDown |
| `retry_markitdown_install` | 无 | 安装结果 | 重新安装 MarkItDown |

### 14.15 Tauri Events（推送通知）

| Event | 触发时机 | 携带数据 |
|-------|----------|----------|
| `kb-stats-changed` | 知识库统计变化 | stats 对象 |
| `agent-activity` | Agent 开始/结束任务 | agent name, status |
| `source-updated` | Source 状态变化 | source_id |
| `task-updated` | 任务状态变化 | task_id, status |
| `task-event` | 任务事件发生 | task_id, event_type, agent_name, message |
| `review-updated` | 审阅状态变化 | review_id |
| `wiki-updated` | Wiki 页面变化 | page_path |
| `folder-import-progress` | 文件夹导入进度 | current, total, filename, success, failed |
| `agent-stream` | 模型流式输出 | task_id, delta, is_finished |

---

## 15. 代码位置索引

### 15.1 布局组件

| 组件 | 文件路径 | 状态 |
|------|----------|------|
| 工作区根布局 | [src/components/layout/WorkspacePage.tsx](src/components/layout/WorkspacePage.tsx) | 待创建 |
| 左侧边栏 | [src/components/layout/LeftSidebar.tsx](src/components/layout/LeftSidebar.tsx) | 待创建 |
| 右侧边栏 | [src/components/layout/RightSidebar.tsx](src/components/layout/RightSidebar.tsx) | 待创建 |
| 中央区域 | [src/components/layout/CenterArea.tsx](src/components/layout/CenterArea.tsx) | 待创建 |
| 底部面板 | [src/components/layout/BottomPanel.tsx](src/components/layout/BottomPanel.tsx) | 待创建 |
| 状态栏 | [src/components/layout/StatusBar.tsx](src/components/layout/StatusBar.tsx) | 已有 |
| 侧边栏（旧版兼容） | [src/components/layout/Sidebar.tsx](src/components/layout/Sidebar.tsx) | 已有 |

### 15.2 编辑器组件

| 组件 | 文件路径 | 状态 |
|------|----------|------|
| 标签栏 | [src/components/editor/TabBar.tsx](src/components/editor/TabBar.tsx) | 已有 |
| Markdown 编辑器 | [src/components/editor/MarkdownEditor.tsx](src/components/editor/MarkdownEditor.tsx) | 已有 |
| Markdown 渲染器 | [src/components/common/MarkdownRenderer.tsx](src/components/common/MarkdownRenderer.tsx) | 已有 |

### 15.3 文件树组件

| 组件 | 文件路径 | 状态 |
|------|----------|------|
| 文件树 | [src/components/filebrowser/FileTree.tsx](src/components/filebrowser/FileTree.tsx) | 已有 |
| 文件树 Header | [src/components/filebrowser/FileTreeHeader.tsx](src/components/filebrowser/FileTreeHeader.tsx) | 已有 |
| 导入文件夹对话框 | [src/components/filebrowser/ImportFolderDialog.tsx](src/components/filebrowser/ImportFolderDialog.tsx) | 已有 |

### 15.4 图谱组件

| 组件 | 文件路径 | 状态 |
|------|----------|------|
| 思维导图视图 | [src/components/graph/MindMapView.tsx](src/components/graph/MindMapView.tsx) | 已有 |

### 15.5 快速导航组件

| 组件 | 文件路径 | 状态 |
|------|----------|------|
| 全局键盘处理 | [src/components/quicknav/GlobalKeyboardHandler.tsx](src/components/quicknav/GlobalKeyboardHandler.tsx) | 已有 |
| 快速切换器 | [src/components/quicknav/QuickSwitcher.tsx](src/components/quicknav/QuickSwitcher.tsx) | 已有 |
| 命令面板 | [src/components/quicknav/CommandPalette.tsx](src/components/quicknav/CommandPalette.tsx) | 已有 |

### 15.6 页面组件

| 组件 | 文件路径 | 说明 |
|------|----------|------|
| 引导页 | [src/pages/OnboardingPage.tsx](src/pages/OnboardingPage.tsx) | 无 KB 时自动显示 |
| 仪表盘 | [src/pages/DashboardPage.tsx](src/pages/DashboardPage.tsx) | 欢迎标签及 `/` 路由内容 |
| 设置中心 | [src/pages/SettingsPage.tsx](src/pages/SettingsPage.tsx) | `/settings` 路由 |
| Wiki 页面（旧版兼容） | [src/pages/WikiPage.tsx](src/pages/WikiPage.tsx) | 内容逻辑供编辑器标签复用 |
| Chat（旧版兼容） | [src/pages/ChatPage.tsx](src/pages/ChatPage.tsx) | 内容逻辑供 Chat 标签/右侧面板复用 |
| 图谱（旧版兼容） | [src/pages/GraphPage.tsx](src/pages/GraphPage.tsx) | 内容逻辑供 Graph 标签复用 |
| 审阅（旧版兼容） | [src/pages/ReviewPage.tsx](src/pages/ReviewPage.tsx) | 审阅卡片逻辑供底部面板复用 |
| Source 预览（旧版兼容） | [src/pages/SourcePreviewPage.tsx](src/pages/SourcePreviewPage.tsx) | 预览/摘要逻辑供 Popover 复用 |
| 任务详情（旧版兼容） | [src/pages/TaskDetailPage.tsx](src/pages/TaskDetailPage.tsx) | 任务详情逻辑供 Popover 复用 |
| 健康检查（旧版兼容） | [src/pages/HealthCheckPage.tsx](src/pages/HealthCheckPage.tsx) | 健康检查逻辑供设置中心 Tab 复用 |
| 文件浏览（旧版兼容） | [src/pages/FileBrowserPage.tsx](src/pages/FileBrowserPage.tsx) | 文件预览逻辑供左侧文件树复用 |
| 搜索（旧版兼容） | [src/pages/SearchPage.tsx](src/pages/SearchPage.tsx) | 搜索逻辑供快速切换器复用 |
| 来源管理（旧版兼容） | [src/pages/SourcesPage.tsx](src/pages/SourcesPage.tsx) | 来源管理逻辑整合进文件树 |
| 导入任务（旧版兼容） | [src/pages/ImportTasksPage.tsx](src/pages/ImportTasksPage.tsx) | 任务列表逻辑整合进 Agent 面板/Popover |
| 编辑器（旧版兼容） | [src/pages/EditorPage.tsx](src/pages/EditorPage.tsx) | 编辑器逻辑迁移至 Editor 标签 |

### 15.7 Store 文件

| Store | 文件路径 | 状态 |
|-------|----------|------|
| useKBStore | [src/stores/useKBStore.ts](src/stores/useKBStore.ts) | 已有 |
| useAppStore | [src/stores/useAppStore.ts](src/stores/useAppStore.ts) | 已有（v2 字段已就绪） |
| useEditorStore | [src/stores/useEditorStore.ts](src/stores/useEditorStore.ts) | 已有（多标签系统已就绪） |
| useFileTreeStore | [src/stores/useFileTreeStore.ts](src/stores/useFileTreeStore.ts) | 已有 |
| useContextPanelStore | [src/stores/useContextPanelStore.ts](src/stores/useContextPanelStore.ts) | 已有（autoAdapt 已就绪） |
| useReviewStore | [src/stores/useReviewStore.ts](src/stores/useReviewStore.ts) | 已有 |
| useModelStore | [src/stores/useModelStore.ts](src/stores/useModelStore.ts) | 已有 |
| useQuickNavStore | [src/stores/useQuickNavStore.ts](src/stores/useQuickNavStore.ts) | 已有 |
| useThemeStore | [src/stores/useThemeStore.ts](src/stores/useThemeStore.ts) | 已有 |

### 15.8 后端核心模块

| 模块 | 文件路径 |
|------|----------|
| 应用内核（DI 容器） | [src-tauri/src/core/app_kernel.rs](src-tauri/src/core/app_kernel.rs) |
| 任务队列 | [src-tauri/src/core/task_queue.rs](src-tauri/src/core/task_queue.rs) |
| 事件总线 | [src-tauri/src/core/event_bus.rs](src-tauri/src/core/event_bus.rs) |
| 数据库 Schema | [src-tauri/src/db/schema.rs](src-tauri/src/db/schema.rs) |
| Wiki 写入器 | [src-tauri/src/wiki/wiki_writer.rs](src-tauri/src/wiki/wiki_writer.rs) |
| 版本管理器 | [src-tauri/src/wiki/version_manager.rs](src-tauri/src/wiki/version_manager.rs) |
| 审阅引擎 | [src-tauri/src/review/review_engine.rs](src-tauri/src/review/review_engine.rs) |
| Diff 引擎 | [src-tauri/src/review/diff_engine.rs](src-tauri/src/review/diff_engine.rs) |
| 图谱服务 | [src-tauri/src/graph/graph_service.rs](src-tauri/src/graph/graph_service.rs) |
| 全文搜索 | [src-tauri/src/search/full_text_search.rs](src-tauri/src/search/full_text_search.rs) |
| 模型网关 | [src-tauri/src/model/model_gateway.rs](src-tauri/src/model/model_gateway.rs) |
| 恢复检查 | [src-tauri/src/recovery/recovery_check.rs](src-tauri/src/recovery/recovery_check.rs) |
| 密钥服务 | [src-tauri/src/core/secret_service.rs](src-tauri/src/core/secret_service.rs) |

---

> **文档结束**
>
> LLMWiki v2.0 将原先 15 个独立页面路由的分散架构重构为 Obsidian 风格的单一工作区。所有功能通过标签页（TabBar）、可折叠面板（左侧边栏 / 右侧边栏 / 底部面板）、Popover 弹窗实现，最大化工作区沉浸感的同时保持全部原有功能可访问。
>
> 本手册覆盖了 v2.0 工作区的 6 区布局、7 种标签类型、3 种右侧边栏子模式、109 个后端 IPC 命令、9 个 Zustand Store、9 个 Tauri Event、所有用户交互操作与完整数据流。
>
> 最后更新: 2026-05-21
