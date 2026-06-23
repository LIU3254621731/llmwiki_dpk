"""
FileTreeAgent — React前端文件浏览器增强
"""
from typing import Dict, Any
from langgraph.graph import StateGraph, END
from state import DevState

AGENT_ID = "agent-epsilon"
FILES = ["src/components/filebrowser/FileTree.tsx", "src/components/filebrowser/FileTreeHeader.tsx"]

INSTRUCTION = """Enhance the file browser with right-click context menu, keyboard navigation, and real-time search.

1. RIGHT-CLICK CONTEXT MENU:
   - Menu items: Open in Editor, View AI Analysis Log, Copy Path, Show in File Explorer, Delete
   - Use @radix-ui/react-context-menu or custom dropdown positioned near cursor

2. KEYBOARD NAVIGATION:
   - Arrow Up/Down: move focus between sibling nodes
   - Arrow Right/Left: expand/collapse folders
   - Enter: open selected file in editor
   - Implement roving tabindex pattern

3. REAL-TIME SEARCH FILTER:
   - Search input in FileTreeHeader with debounce (150ms)
   - Filter tree nodes by file name (case-insensitive fuzzy match)
   - Highlight matching characters in results
   - Show count: "Showing X of Y files"

4. FILE STATUS INDICATORS:
   - Green check = AI analysis complete
   - Spinning icon = AI analyzing
   - Gray dot = Not yet imported to AI
"""

def analyze_task(state: DevState) -> Dict[str, Any]:
    return {"messages": [{"agent": AGENT_ID, "status": "analyzing", "files": FILES, "instruction": INSTRUCTION}]}

def build_filetree_agent() -> StateGraph:
    workflow = StateGraph(DevState)
    workflow.add_node("analyze", analyze_task)
    workflow.set_entry_point("analyze")
    workflow.add_edge("analyze", END)
    return workflow
