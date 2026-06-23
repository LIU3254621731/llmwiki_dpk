"""
GraphAgent — React前端知识图谱可视化修复
负责: 修复空白画布、渲染节点边、空状态引导
"""
from typing import Dict, Any
from langgraph.graph import StateGraph, END
from state import DevState

AGENT_ID = "agent-beta"
FILES = [
    "src/components/graph/MindMapView.tsx",
    "src/components/views/CanvasView.tsx",
]

INSTRUCTION = """Fix the blank knowledge graph canvas. Current state: backend has graph nodes in database, but frontend renders blank.

1. DEBUG RENDERING PIPELINE in src/components/graph/MindMapView.tsx:
   - Check if graph data is properly fetched via invoke("get_graph_data")
   - Verify nodes/edges are passed to ReactFlow/X6 renderer
   - Ensure ReactFlow component receives proper elements array with position, type, data

2. FIX EMPTY CANVAS in src/components/views/CanvasView.tsx:
   - If graphData.nodes.length === 0: show empty state placeholder
   - If graphData.nodes.length > 0 but nothing renders: check node position defaults
   - Add console.log debugging for data flow

3. NODE RENDERING:
   - Use HierarchicalNode component for each node
   - Apply default layout (dagre or force-directed) if positions are undefined
   - Color nodes by type (entity=blue, concept=green, source=gray)

4. EDGE RENDERING:
   - Render edges from relationships
   - Show edge labels for relationship types

5. INTERACTIONS:
   - onNodeClick: navigate to wiki page or open detail
   - ReactFlow controls: zoom, fit, minimap

6. EMPTY STATE:
   - Show: "No knowledge graph data yet. Import documents and approve review items to build the graph."
   - Add button: "Go to Import"
"""


def analyze_task(state: DevState) -> Dict[str, Any]:
    task = next((t for t in state["tasks"] if t["agent_id"] == AGENT_ID), None)
    if not task:
        return {"messages": [{"agent": AGENT_ID, "status": "no_task"}]}
    
    return {"messages": [{
        "agent": "GraphAgent",
        "status": "analyzing",
        "files": FILES,
        "instruction": INSTRUCTION,
    }]}


def build_graph_agent() -> StateGraph:
    workflow = StateGraph(DevState)
    workflow.add_node("analyze", analyze_task)
    workflow.set_entry_point("analyze")
    workflow.add_edge("analyze", END)
    return workflow
