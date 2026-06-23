"""
Orchestrator — LangGraph 多Agent协调器
负责任务分发、并行调度、进度汇总
"""
import json
import time
import asyncio
from pathlib import Path
from typing import Dict, Any, List
from dataclasses import dataclass, field
from datetime import datetime

from langgraph.graph import StateGraph, END
from state import DevState, AgentTask
from agents import AGENTS


PROJECT_ROOT = Path(__file__).resolve().parent.parent
TASK_FILE = Path(__file__).resolve().parent / "task_registry.json"


@dataclass
class Orchestrator:
    """多Agent开发协调器"""
    project_root: Path = PROJECT_ROOT
    agents: Dict = field(default_factory=lambda: AGENTS)
    state: Dict[str, Any] = field(default_factory=dict)
    
    def load_tasks(self) -> List[Dict]:
        """加载任务注册表"""
        with open(TASK_FILE, "r", encoding="utf-8") as f:
            return json.load(f)
    
    def save_tasks(self, tasks: List[Dict]):
        """保存任务状态"""
        with open(TASK_FILE, "w", encoding="utf-8") as f:
            json.dump(tasks, f, ensure_ascii=False, indent=2)
    
    def generate_task_report(self) -> str:
        """生成可读的任务进度报告"""
        tasks = self.load_tasks()
        lines = [
            "=" * 60,
            f"  LLMWiki Dev-Agents — 开发进度报告",
            f"  时间: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}",
            f"  项目: {self.project_root.name}",
            "=" * 60,
            "",
        ]
        
        for phase in tasks.get("phases", []):
            status_icon = {"pending": "⏳", "in_progress": "🔄", "completed": "✅"}.get(phase["status"], "❓")
            lines.append(f"  Phase {phase['phase']}: {phase['name']} {status_icon} [{phase['status']}]")
            
            for agent_id in phase.get("agents", []):
                agent = next((a for a in tasks["agents"] if a["id"] == agent_id), None)
                if agent:
                    s = {"pending": "⏳", "analyzing": "🔍", "coding": "✏️", "reviewing": "👀", "done": "✅", "failed": "❌"}
                    icon = s.get(agent["status"], "❓")
                    lines.append(f"    {icon} {agent['name']} — {agent['status']}")
                    lines.append(f"       Files: {len(agent['files'])} | Priority: P{agent['priority']}")
            lines.append("")
        
        # Summary
        all_agents = tasks.get("agents", [])
        done = sum(1 for a in all_agents if a["status"] == "done")
        failed = sum(1 for a in all_agents if a["status"] == "failed")
        pending = sum(1 for a in all_agents if a["status"] == "pending")
        in_progress = len(all_agents) - done - failed - pending
        
        lines.append(f"  📊 总计: {len(all_agents)} 任务")
        lines.append(f"     ✅ 完成: {done} | 🔄 进行中: {in_progress} | ⏳ 待开始: {pending} | ❌ 失败: {failed}")
        lines.append(f"     完成度: {done / max(len(all_agents), 1) * 100:.0f}%")
        lines.append("=" * 60)
        
        return "\n".join(lines)
    
    def get_parallel_batches(self) -> List[List[str]]:
        """按文件写域隔离，分组并行执行的Agent"""
        tasks = self.load_tasks()
        pending_agents = [a for a in tasks["agents"] if a["status"] == "pending"]
        
        # 按写域分组：同一文件的Agent不能并行
        file_owners: Dict[str, set] = {}
        for agent in pending_agents:
            file_set = frozenset(agent["files"])
            file_owners[agent["id"]] = file_set
        
        batches = []
        remaining = set(a["id"] for a in pending_agents)
        
        while remaining:
            batch = []
            batch_files = set()
            for agent_id in list(remaining):
                agent_files = file_owners[agent_id]
                if not (batch_files & agent_files):  # 无文件冲突
                    batch.append(agent_id)
                    batch_files |= agent_files
                    remaining.discard(agent_id)
            
            if not batch:  # 无法继续分组，剩下的串行
                batch = list(remaining)
                remaining.clear()
            
            batches.append(batch)
        
        return batches


def build_orchestrator_graph() -> StateGraph:
    """构建Orchestrator的LangGraph"""
    workflow = StateGraph(DevState)
    
    def init_state(state: DevState) -> Dict:
        """初始化开发状态"""
        orch = Orchestrator()
        tasks = orch.load_tasks()
        
        agent_tasks = []
        for t in tasks["agents"]:
            agent_tasks.append(AgentTask(
                agent_id=t["id"],
                name=t["name"],
                description=t["description"],
                files=t["files"],
                status=t["status"],
                priority=t["priority"],
                acceptance_criteria=t.get("acceptance", []),
                patch_summary="",
                errors=[],
            ))
        
        return {
            "tasks": agent_tasks,
            "project_root": str(PROJECT_ROOT),
            "project_summary": "LLMWiki Tauri desktop app — LLM-powered knowledge base management",
            "current_phase": 1,
            "phase_status": {1: "in_progress", 2: "pending", 3: "pending", 4: "pending"},
            "messages": [{"orchestrator": "initialized", "agent_count": len(agent_tasks)}],
        }
    
    def compute_batches(state: DevState) -> Dict:
        """计算并行批次"""
        orch = Orchestrator()
        batches = orch.get_parallel_batches()
        return {"messages": [{"orchestrator": "batches_computed", "batches": batches}]}
    
    def dispatch_batch(state: DevState) -> Dict:
        """分发任务给Agent"""
        orch = Orchestrator()
        report = orch.generate_task_report()
        print(report)
        return {"messages": [{"orchestrator": "report_generated"}]}
    
    def finalize(state: DevState) -> Dict:
        """检查完成状态"""
        all_done = all(t["status"] in ("done", "failed") for t in state["tasks"])
        return {"messages": [{"orchestrator": "finalized", "all_done": all_done}]}
    
    workflow.add_node("init", init_state)
    workflow.add_node("compute_batches", compute_batches)
    workflow.add_node("dispatch", dispatch_batch)
    workflow.add_node("finalize", finalize)
    
    workflow.set_entry_point("init")
    workflow.add_edge("init", "compute_batches")
    workflow.add_edge("compute_batches", "dispatch")
    workflow.add_edge("dispatch", "finalize")
    workflow.add_edge("finalize", END)
    
    return workflow


# === 命令行入口 ===
if __name__ == "__main__":
    orch = Orchestrator()
    
    print("\n📋 LLMWiki Dev-Agents 任务注册表")
    print()
    
    tasks = orch.load_tasks()
    batches = orch.get_parallel_batches()
    
    print(f"🔀 并行批次规划 ({len(batches)} 批):")
    for i, batch in enumerate(batches):
        agent_names = [AGENTS[aid]["name"] for aid in batch]
        print(f"  Batch {i+1}: {', '.join(agent_names)}")
    
    print()
    print(orch.generate_task_report())
