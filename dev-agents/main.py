"""
main.py — LLMWiki Dev-Agents 入口
用法:
  python dev-agents/main.py status    # 查看任务状态
  python dev-agents/main.py dashboard # 启动进度看板
  python dev-agents/main.py run       # 运行Orchestrator
"""
import sys
import subprocess
from pathlib import Path

HERE = Path(__file__).resolve().parent


def cmd_status():
    """打印任务状态报告"""
    from orchestrator import Orchestrator
    orch = Orchestrator()
    print(orch.generate_task_report())
    
    print("\n🔀 并行批次规划:")
    batches = orch.get_parallel_batches()
    for i, batch in enumerate(batches):
        print(f"  Batch {i+1}: {batch}")


def cmd_dashboard():
    """启动进度看板"""
    subprocess.run([sys.executable, str(HERE / "dashboard.py")])


def main():
    if len(sys.argv) < 2:
        print(__doc__)
        return
    
    cmd = sys.argv[1]
    if cmd == "status":
        cmd_status()
    elif cmd == "dashboard":
        cmd_dashboard()
    elif cmd == "run":
        print("Starting orchestrator...")
        from orchestrator import build_orchestrator_graph
        graph = build_orchestrator_graph()
        result = graph.invoke({
            "tasks": [],
            "project_root": str(HERE.parent),
            "project_summary": "",
            "codebase_stats": {},
            "current_phase": 1,
            "phase_status": {},
            "messages": [],
            "completed_tasks": [],
            "failed_tasks": [],
        })
        print("Orchestrator completed.")
        print("Messages:", result.get("messages", []))
    else:
        print(f"Unknown command: {cmd}")
        print(__doc__)


if __name__ == "__main__":
    main()
