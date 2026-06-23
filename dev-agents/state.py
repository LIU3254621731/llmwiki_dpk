"""
Agent 状态定义 — LangGraph 共享状态
"""
from typing import TypedDict, Annotated, List, Dict, Any, Optional, Literal
from operator import add


class AgentTask(TypedDict):
    """单个Agent的开发任务"""
    agent_id: str
    name: str
    description: str
    files: List[str]
    status: Literal["pending", "analyzing", "coding", "reviewing", "done", "failed"]
    priority: int
    acceptance_criteria: List[str]
    patch_summary: str
    errors: List[str]


class DevState(TypedDict):
    """跨Agent共享的开发状态"""
    # 任务注册
    tasks: Annotated[List[AgentTask], add]
    
    # 项目上下文 (共享给所有Agent)
    project_root: str
    project_summary: str
    codebase_stats: Dict[str, Any]
    
    # 进度追踪
    current_phase: int
    phase_status: Dict[int, str]
    
    # Agent 间通信
    messages: Annotated[List[Dict[str, Any]], add]
    
    # 汇总
    completed_tasks: Annotated[List[str], add]
    failed_tasks: Annotated[List[str], add]
