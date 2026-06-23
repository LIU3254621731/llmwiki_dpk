"""
UIAgent — CSS/React UI美化与空状态引导
"""
from typing import Dict, Any
from langgraph.graph import StateGraph, END
from state import DevState

AGENT_ID = "agent-gamma"
FILES = ["src/styles/theme.css", "src/index.css", "src/components/views/DashboardView.tsx", "src/components/views/FileExplorerView.tsx", "src/components/layout/StatusBar.tsx"]

INSTRUCTION = """Polish UI and add empty state guidance across all views.

1. TAILWIND THEME POLISH:
   - Add CSS variables for consistent spacing (--space-xs/sm/md/lg/xl)
   - Smooth transitions: view switching 200ms ease-in-out, hover states 150ms
   - Card components: subtle shadow + rounded corners (shadow-sm, rounded-lg)
   - Empty state containers: centered, muted text, icon + message + action button

2. STATUS BAR: Show version number and KB name. Add dev mode indicator.

3. DASHBOARD EMPTY STATE:
   - No KB: "Select or create a knowledge base to get started"
   - No sources: "Upload documents to begin. Drag & drop PDF, DOCX, or Markdown files."
   - Sources but no wiki: "AI is analyzing your documents. Check Import & Review."

4. FILE EXPLORER EMPTY STATE:
   - No files: "No files imported yet. Click Upload to add documents."

5. CONSISTENT SPACING: All view containers p-6, section headers text-lg font-semibold mb-4, card grids gap-4, button groups gap-2
"""

def analyze_task(state: DevState) -> Dict[str, Any]:
    return {"messages": [{"agent": AGENT_ID, "status": "analyzing", "files": FILES, "instruction": INSTRUCTION}]}

def build_ui_agent() -> StateGraph:
    workflow = StateGraph(DevState)
    workflow.add_node("analyze", analyze_task)
    workflow.set_entry_point("analyze")
    workflow.add_edge("analyze", END)
    return workflow
