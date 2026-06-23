"""
ReviewAgent — Rust后端评审系统可靠性修复
负责: 修复Review状态机、create/update判别、原子化写入
"""
from typing import Dict, Any
from langgraph.graph import StateGraph, END
from state import DevState, AgentTask

AGENT_ID = "agent-alpha"
AGENT_NAME = "ReviewAgent"
OWNER = "Rust Backend"

FILES = [
    "src-tauri/src/review/review_engine.rs",
    "src-tauri/src/commands/review.rs",
]

INSTRUCTION = """Review the review system code and fix the following issues:

1. STATE MACHINE: In src-tauri/src/review/review_engine.rs, ensure accept_review_item follows strict state transitions:
   - pending -> applying -> applied (success path)
   - pending -> applying -> apply_failed (failure path, NEVER skipped)
   - Only reject flows can reach rejected/skipped states

2. CREATE/UPDATE DISCRIMINATION: In review_engine.rs and commands/review.rs, fix:
   - If target wiki page does not exist: operation_type MUST be create_page
   - If target page exists: operation_type can be update_page/append_section
   - Auto-convert update_page to create_page when target not found (already has auto_converted_from_update field)

3. ATOMIC WRITE: In accept flow:
   - Write markdown file -> update wiki_pages -> update knowledge_items -> update relationships -> update graph -> update index -> create version snapshot
   - If ANY step fails, rollback and set apply_failed
   - Record operation_id + operation_hash

4. BATCH OPERATIONS: accept_all/reject_all must be transactional

5. UI ERROR DISPLAY: Pass apply_error field to frontend for display
"""


def analyze_task(state: DevState) -> Dict[str, Any]:
    task = next((t for t in state["tasks"] if t["agent_id"] == AGENT_ID), None)
    if not task:
        return {"messages": [{"agent": AGENT_ID, "status": "no_task"}]}
    
    return {"messages": [{
        "agent": AGENT_ID,
        "status": "analyzing",
        "task": task["name"],
        "files": FILES,
        "instruction": INSTRUCTION,
    }]}


def build_review_agent() -> StateGraph:
    workflow = StateGraph(DevState)
    workflow.add_node("analyze", analyze_task)
    workflow.set_entry_point("analyze")
    workflow.add_edge("analyze", END)
    return workflow
