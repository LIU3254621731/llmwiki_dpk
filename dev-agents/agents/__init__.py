from .review_agent import build_review_agent
from .graph_agent import build_graph_agent
from .ui_agent import build_ui_agent
from .dedup_agent import build_dedup_agent
from .filetree_agent import build_filetree_agent

__all__ = [
    "build_review_agent",
    "build_graph_agent",
    "build_ui_agent",
    "build_dedup_agent",
    "build_filetree_agent",
]

# Agent Registry
AGENTS = {
    "agent-alpha": {
        "id": "agent-alpha",
        "name": "ReviewAgent",
        "icon": "🔍",
        "description": "Rust后端评审系统可靠性修复",
        "builder": build_review_agent,
        "priority": 1,
        "files": ["src-tauri/src/review/review_engine.rs", "src-tauri/src/commands/review.rs"],
    },
    "agent-beta": {
        "id": "agent-beta",
        "name": "GraphAgent",
        "icon": "🕸️",
        "description": "React前端知识图谱可视化修复",
        "builder": build_graph_agent,
        "priority": 2,
        "files": ["src/components/graph/MindMapView.tsx", "src/components/views/CanvasView.tsx"],
    },
    "agent-gamma": {
        "id": "agent-gamma",
        "name": "UIAgent",
        "icon": "🎨",
        "description": "CSS/React UI美化与空状态引导",
        "builder": build_ui_agent,
        "priority": 3,
        "files": ["src/styles/theme.css", "src/index.css", "src/components/views/DashboardView.tsx", "src/components/views/FileExplorerView.tsx", "src/components/layout/StatusBar.tsx"],
    },
    "agent-delta": {
        "id": "agent-delta",
        "name": "DedupAgent",
        "icon": "🔄",
        "description": "Rust后端去重系统集成",
        "builder": build_dedup_agent,
        "priority": 4,
        "files": ["src-tauri/src/dedup/dedup_service.rs", "src-tauri/src/agents/wiki_update.rs"],
    },
    "agent-epsilon": {
        "id": "agent-epsilon",
        "name": "FileTreeAgent",
        "icon": "📁",
        "description": "React前端文件浏览器增强",
        "builder": build_filetree_agent,
        "priority": 5,
        "files": ["src/components/filebrowser/FileTree.tsx", "src/components/filebrowser/FileTreeHeader.tsx"],
    },
}
