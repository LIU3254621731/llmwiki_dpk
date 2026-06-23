"""
DedupAgent — Rust后端去重系统集成
"""
from typing import Dict, Any
from langgraph.graph import StateGraph, END
from state import DevState

AGENT_ID = "agent-delta"
FILES = ["src-tauri/src/dedup/dedup_service.rs", "src-tauri/src/agents/wiki_update.rs"]

INSTRUCTION = """Integrate dedup service into the wiki update pipeline.

1. DEDUP SERVICE ENHANCEMENT (src-tauri/src/dedup/dedup_service.rs):
   - Add find_duplicates(title, page_type) -> Vec<DuplicateCandidate> method
   - Use strsim for fuzzy string matching (normalized_damerau_levenshtein)
   - Configurable threshold (default 0.85)
   - Return matched page info: id, title, path, similarity_score

2. WIKI UPDATE INTEGRATION (src-tauri/src/agents/wiki_update.rs):
   - Before creating new wiki page, call dedup_service.find_duplicates()
   - If duplicate found with score > 0.95: auto-skip creation, log duplication
   - If duplicate found with 0.85-0.95: generate merge_suggestion review item
   - If no duplicate: proceed with normal create_page

3. PAGE LIST DEDUP: Group duplicates in list_wiki_pages command output

4. BATCH CLEANUP: Add dedup_cleanup(kb_id) command that scans all pages and flags duplicates
"""

def analyze_task(state: DevState) -> Dict[str, Any]:
    return {"messages": [{"agent": AGENT_ID, "status": "analyzing", "files": FILES, "instruction": INSTRUCTION}]}

def build_dedup_agent() -> StateGraph:
    workflow = StateGraph(DevState)
    workflow.add_node("analyze", analyze_task)
    workflow.set_entry_point("analyze")
    workflow.add_edge("analyze", END)
    return workflow
