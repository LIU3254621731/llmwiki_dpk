"""
LLMWiki Dev-Agents — 多Agent协作开发系统
使用 LangGraph 编排 5 个专业 Agent 并行开发 LLMWiki 桌面应用。

架构:
  Orchestrator (LangGraph StateGraph)
    ├── ReviewAgent   — Rust后端: 评审系统可靠性修复
    ├── GraphAgent    — React前端: 知识图谱可视化
    ├── UIAgent       — CSS/React: UI美化与空状态
    ├── DedupAgent    — Rust后端: 去重系统集成
    └── FileTreeAgent — React前端: 文件浏览器增强

两个系统完全独立:
  系统A: E:\Code\llmwiki_dpk\          (被开发的目标应用)
  系统B: E:\Code\llmwiki_dpk\dev-agents\ (多Agent开发框架)
"""

__version__ = "0.1.0"
