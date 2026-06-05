import { useEffect, useState } from "react";
import { useAppStore, type RightSidebarMode } from "@/stores/useAppStore";
import { useKBStore } from "@/stores/useKBStore";
import { useEditorStore } from "@/stores/useEditorStore";
import { useContextPanelStore } from "@/stores/useContextPanelStore";
import { cn } from "@/lib/utils";
import {
  X, PanelRightOpen, Bot, FileText, Loader2,
} from "lucide-react";
import RightContextPanel from "@/components/common/RightContextPanel";

interface AgentLogEntry {
  agent_name: string;
  status: string;
  file_name: string;
  detail: string;
  timestamp: string;
}

const STATUS_LABELS: Record<string, string> = {
  starting: "启动中",
  parsing: "解析中",
  parsed: "已解析",
  parse_failed: "解析失败",
  parsing_skipped: "跳过解析",
  context_gathering: "收集上下文",
  prompt_building: "构建Prompt",
  model_calling: "AI推理中",
  model_returned: "AI已返回",
  json_validating: "校验JSON",
  json_repairing: "修复JSON",
  json_failed: "JSON失败",
  candidate_searching: "候选检索",
  completed: "已完成",
  resolution_running: "消歧处理",
  relationship_running: "关系标准化",
  update_plan_generating: "生成更新计划",
  review_generating: "生成审阅",
  applying: "应用变更",
};

const MODE_LABELS: Record<RightSidebarMode, string> = {
  context: "上下文",
  agent: "Agent",
  rag: "RAG",
  health: "健康",
};

function getStatusLabel(status: string): string {
  return STATUS_LABELS[status] || status;
}

// ---- Agent Panel ----
function AgentPanel({ entries }: { entries: AgentLogEntry[] }) {
  const latest = entries[entries.length - 1];

  return (
    <div className="flex flex-col h-full">
      {/* Current status */}
      {latest ? (
        <div className="p-3 border-b border-slate-100 dark:border-slate-800">
          <div className="flex items-center gap-2 mb-1">
            <span className="w-2 h-2 rounded-full bg-blue-500 animate-pulse" />
            <span className="text-sm font-medium text-slate-700 dark:text-slate-300">
              {latest.agent_name}
            </span>
            <span className="text-xs px-1.5 py-0.5 rounded bg-blue-50 dark:bg-blue-900/20 text-blue-600 dark:text-blue-400">
              {getStatusLabel(latest.status)}
            </span>
          </div>
          {latest.file_name && (
            <div className="text-xs text-slate-500 dark:text-slate-400 truncate">
              文件: {latest.file_name}
            </div>
          )}
          {latest.detail && (
            <div className="text-xs text-slate-400 dark:text-slate-500 mt-0.5">
              {latest.detail}
            </div>
          )}
        </div>
      ) : (
        <div className="p-3 border-b border-slate-100 dark:border-slate-800 text-xs text-slate-400 text-center">
          暂无 Agent 活动
        </div>
      )}

      {/* Activity log */}
      <div className="flex-1 overflow-y-auto p-2">
        <h4 className="text-[10px] font-medium text-slate-400 uppercase mb-2 px-1">
          最近活动
        </h4>
        {entries.length === 0 ? (
          <div className="text-xs text-slate-400 text-center py-4">
            等待 Agent 活动中...
          </div>
        ) : (
          <div className="space-y-1">
            {[...entries].reverse().map((entry, i) => (
              <div
                key={i}
                className="px-2 py-1.5 rounded text-xs bg-slate-50 dark:bg-slate-800/50"
              >
                <div className="flex items-center gap-1.5">
                  <span className="font-medium text-slate-600 dark:text-slate-400">
                    {entry.agent_name}
                  </span>
                  <span className="text-slate-400">-</span>
                  <span className="text-slate-500">{getStatusLabel(entry.status)}</span>
                </div>
                {entry.file_name && (
                  <div className="text-slate-400 truncate mt-0.5">{entry.file_name}</div>
                )}
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}

// ---- RAG Panel ----
function RagPanel() {
  const activeTabId = useEditorStore((s) => s.activeTabId);
  const openTabs = useEditorStore((s) => s.openTabs);
  const activeTab = openTabs.find((t) => t.id === activeTabId);

  const isEditorWithFile = activeTab && activeTab.type === "editor" && activeTab.path;

  if (!isEditorWithFile) {
    return (
      <div className="flex flex-col items-center justify-center h-full p-6 text-center">
        <FileText size={32} className="text-slate-300 dark:text-slate-600 mb-3" />
        <p className="text-sm text-slate-500 dark:text-slate-400">
          打开一个文件以查看局部 RAG 召回片段
        </p>
      </div>
    );
  }

  return (
    <div className="p-3 space-y-4">
      <div className="bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-800 rounded-lg p-3">
        <h4 className="text-xs font-medium text-blue-700 dark:text-blue-300 mb-1">
          局部 RAG
        </h4>
        <p className="text-xs text-blue-600 dark:text-blue-400">
          当前文件已锁定为检索范围
        </p>
      </div>

      <div>
        <h4 className="text-[10px] font-medium text-slate-400 uppercase mb-1">
          检索范围
        </h4>
        <div className="text-xs text-slate-600 dark:text-slate-400 font-mono bg-slate-50 dark:bg-slate-800 rounded px-2 py-1.5 truncate">
          {activeTab.path}
        </div>
      </div>

      <div>
        <h4 className="text-[10px] font-medium text-slate-400 uppercase mb-2">
          召回片段
        </h4>
        <div className="text-xs text-slate-400 dark:text-slate-500 text-center py-6 bg-slate-50 dark:bg-slate-800/50 rounded">
          局部 RAG 检索将在 AI 问答时自动触发
        </div>
      </div>
    </div>
  );
}

// ---- Main RightSidebar ----
export default function RightSidebar() {
  const rightSidebarVisible = useAppStore((s) => s.rightSidebarVisible);
  const rightSidebarMode = useAppStore((s) => s.rightSidebarMode);
  const toggleRightSidebar = useAppStore((s) => s.toggleRightSidebar);
  const setRightSidebarMode = useAppStore((s) => s.setRightSidebarMode);

  const context = useContextPanelStore((s) => s.context);
  const contextMode = useContextPanelStore((s) => s.mode);
  const contextSourceTabType = useContextPanelStore((s) => s.sourceTabType);

  const [agentEntries, setAgentEntries] = useState<AgentLogEntry[]>([]);

  // Map sourceTabType to RightContextPanel type
  const getPanelType = (): string => {
    // If in outline/backlinks/local_graph mode for editor content, use the mode directly
    if (contextMode === "outline" || contextMode === "backlinks" || contextMode === "local_graph") {
      return contextMode;
    }
    // Otherwise map based on tab type
    switch (contextSourceTabType) {
      case "editor": return "file";
      case "pdf_viewer": return "source";
      case "graph": return "graph";
      case "chat": return "chat";
      case "welcome": return "wiki";
      default: return "file";
    }
  };

  // Listen to agent-activity events
  useEffect(() => {
    let unlistenFn: (() => void) | null = null;

    (async () => {
      try {
        const { listen } = await import("@tauri-apps/api/event");
        const unlisten = await listen<AgentLogEntry>("agent-activity", (event) => {
          setAgentEntries((prev) => {
            const next = [...prev, event.payload];
            return next.slice(-100);
          });
        });
        unlistenFn = unlisten;
      } catch {
        // Tauri APIs not available (e.g. in browser dev mode)
      }
    })();

    return () => {
      if (unlistenFn) unlistenFn();
    };
  }, []);

  if (!rightSidebarVisible) return null;

  const handleContextAction = (action: string, payload?: any) => {
    console.log("Context action:", action, payload);
  };

  // Build context for RightContextPanel
  const panelContext = context
    ? {
        type: getPanelType() as any,
        data: context,
      }
    : null;

  return (
    <div className="h-full bg-white dark:bg-slate-900 flex flex-col">
      {/* Header */}
      <div className="flex items-center justify-between px-3 py-2 border-b border-slate-200 dark:border-slate-800 bg-slate-50 dark:bg-slate-800/50">
        <span className="text-xs font-medium text-slate-500 dark:text-slate-400 uppercase">
          {MODE_LABELS[rightSidebarMode]}
        </span>
        <button
          type="button"
          onClick={toggleRightSidebar}
          className="p-0.5 hover:bg-slate-200 dark:hover:bg-slate-700 rounded text-slate-400"
          title="关闭面板"
        >
          <X size={14} />
        </button>
      </div>

      {/* Mode tabs */}
      <div className="flex items-center gap-1 px-2 py-1.5 border-b border-slate-100 dark:border-slate-800" role="tablist">
        {(["context", "agent", "rag"] as RightSidebarMode[]).map((mode) => (
          <button
            key={mode}
            type="button"
            role="tab"
            aria-selected={rightSidebarMode === mode ? "true" : "false"}
            aria-label={`切换到${MODE_LABELS[mode]}面板`}
            onClick={() => setRightSidebarMode(mode)}
            className={cn(
              "flex items-center gap-1 px-2 py-1 rounded text-xs transition-colors",
              rightSidebarMode === mode
                ? "bg-brand-50 dark:bg-brand-900/20 text-brand-600 dark:text-brand-400 font-medium"
                : "text-slate-500 dark:text-slate-400 hover:bg-slate-100 dark:hover:bg-slate-800"
            )}
            title={MODE_LABELS[mode]}
          >
            {mode === "context" && <PanelRightOpen size={12} />}
            {mode === "agent" && <Bot size={12} />}
            {mode === "rag" && <FileText size={12} />}
            <span className="hidden xl:inline">{MODE_LABELS[mode]}</span>
          </button>
        ))}
      </div>

      {/* Content */}
      <div className="flex-1 overflow-y-auto">
        {rightSidebarMode === "context" && (
          <RightContextPanel
            visible={true}
            onClose={toggleRightSidebar}
            context={panelContext}
            onAction={handleContextAction}
          />
        )}
        {rightSidebarMode === "agent" && (
          <AgentPanel entries={agentEntries} />
        )}
        {rightSidebarMode === "rag" && <RagPanel />}
      </div>
    </div>
  );
}
