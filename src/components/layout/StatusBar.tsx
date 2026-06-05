import { useKBStore } from "@/stores/useKBStore";
import { useAppStore } from "@/stores/useAppStore";
import { useEffect, useRef, useState } from "react";
import { useEditorStore } from "@/stores/useEditorStore";
import { Circle, GitPullRequestDraft, PanelRightOpen, PanelBottomOpen } from "lucide-react";

interface AgentActivity {
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

function getStatusLabel(status: string): string {
  return STATUS_LABELS[status] || status;
}

function getHealthInfo(stats: {
  health_status: string;
  issue_count?: number;
} | null) {
  if (!stats)
    return { color: "bg-muted", label: "未选择知识库", issueCount: 0 };

  const { health_status, issue_count } = stats;
  const issues = issue_count ?? 0;

  if (health_status === "critical")
    return { color: "bg-destructive", label: `知识库健康度: 严重`, issueCount: issues };
  if (health_status === "warning" || issues > 0)
    return { color: "bg-warning", label: `知识库健康度: 有 ${issues} 个问题`, issueCount: issues };
  return { color: "bg-success", label: "知识库健康度: 正常", issueCount: 0 };
}

export default function StatusBar() {
  const currentKB = useKBStore((s) => s.currentKB);
  const stats = useKBStore((s) => s.stats);
  const setStats = useKBStore((s) => s.setStats);
  const reviewBadgeCount = useAppStore((s) => s.reviewBadgeCount);
  const rightSidebarVisible = useAppStore((s) => s.rightSidebarVisible);
  const bottomPanelVisible = useAppStore((s) => s.bottomPanelVisible);
  const toggleBottomPanel = useAppStore((s) => s.toggleBottomPanel);
  const toggleRightSidebar = useAppStore((s) => s.toggleRightSidebar);
  const openFile = useEditorStore((s) => s.openFile);

  const [time, setTime] = useState("");
  const [agentActivity, setAgentActivity] = useState<AgentActivity | null>(null);
  const activityTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  // Time updater
  useEffect(() => {
    setTime(new Date().toLocaleTimeString("zh-CN"));
    const t = setInterval(() => setTime(new Date().toLocaleTimeString("zh-CN")), 1000);
    return () => clearInterval(t);
  }, []);

  // Agent activity listener (dynamic Tauri import)
  useEffect(() => {
    let unlistenFn: (() => void) | undefined;

    (async () => {
      try {
        const { listen } = await import("@tauri-apps/api/event");
        const unlisten = await listen<AgentActivity>("agent-activity", (event) => {
          // Clear any pending dismiss timer
          if (activityTimerRef.current) {
            clearTimeout(activityTimerRef.current);
          }
          setAgentActivity(event.payload);
          // Auto-dismiss after 60 seconds
          activityTimerRef.current = setTimeout(() => {
            setAgentActivity(null);
          }, 60_000);
        });
        unlistenFn = unlisten;
      } catch {
        // Not running in Tauri context
      }
    })();

    return () => {
      unlistenFn?.();
      if (activityTimerRef.current) {
        clearTimeout(activityTimerRef.current);
      }
    };
  }, []);

  // KB stats listener (dynamic Tauri import, 500ms debounce)
  const lastStatsRefresh = useRef(0);

  // Reset debounce timer on KB switch
  useEffect(() => {
    lastStatsRefresh.current = 0;
  }, [currentKB?.id]);

  useEffect(() => {
    let unlistenFn: (() => void) | undefined;

    (async () => {
      try {
        const { listen } = await import("@tauri-apps/api/event");
        const { invoke } = await import("@tauri-apps/api/core");

        const unlisten = await listen<any>("kb-stats-changed", (event) => {
          const kb = useKBStore.getState().currentKB;
          if (kb && event.payload.kb_id === kb.id) {
            const now = Date.now();
            if (now - lastStatsRefresh.current > 500) {
              lastStatsRefresh.current = now;
              invoke<any>("get_kb_stats", { kbId: kb.id })
                .then((s) => setStats(s))
                .catch((e) => console.error("状态栏刷新统计失败:", e));
            }
          }
        });
        unlistenFn = unlisten;
      } catch {
        // Not running in Tauri context
      }
    })();

    return () => {
      unlistenFn?.();
    };
  }, []);

  const health = getHealthInfo(stats);

  return (
    <div className="h-8 bg-statusbar-bg border-t border-statusbar-border flex items-center px-4 text-xs text-muted-foreground gap-3">
      <span className="font-medium">{currentKB?.name ?? "未选择知识库"}</span>
      <span className="text-muted-foreground">|</span>

      {/* Right sidebar toggle */}
      <button
        type="button"
        onClick={toggleRightSidebar}
        className={`flex items-center gap-1 transition-colors ${
          rightSidebarVisible
            ? "text-primary"
            : "text-muted-foreground hover:text-foreground"
        }`}
        title={rightSidebarVisible ? "关闭右侧面板" : "打开 AI 面板 (上下文/Agent/RAG)"}
      >
        <PanelRightOpen size={13} />
      </button>

      {/* Bottom panel toggle */}
      <button
        type="button"
        onClick={toggleBottomPanel}
        className={`flex items-center gap-1 transition-colors ${
          bottomPanelVisible
            ? "text-warning"
            : "text-muted-foreground hover:text-foreground"
        }`}
        title={bottomPanelVisible ? "关闭审阅面板" : "打开 AI 提案审阅面板"}
      >
        <PanelBottomOpen size={13} />
        {reviewBadgeCount > 0 && (
          <span className="text-[10px] font-medium text-warning">{reviewBadgeCount}</span>
        )}
      </button>

      <span className="text-muted-foreground">|</span>

      {/* Health status dot */}
      <button
        type="button"
        className="flex items-center gap-1 hover:text-foreground transition-colors cursor-pointer"
        onClick={() => openFile({ path: "settings", title: "设置", type: "settings" })}
        title={`${health.label} - 点击打开设置`}
      >
        <Circle
          size={10}
          className={`flex-shrink-0 ${health.color} rounded-full`}
          fill="currentColor"
        />
        <span className="hidden sm:inline">
          {health.color === "bg-success"
            ? "正常"
            : health.color === "bg-warning"
              ? "警告"
              : "异常"}
        </span>
      </button>

      <span className="text-muted-foreground">|</span>

      <span>页面: {stats?.page_count ?? 0}</span>
      <span>Source: {stats?.source_count ?? 0}</span>
      <span>待审: {stats?.review_count ?? 0}</span>

      {/* Review badge */}
      {reviewBadgeCount > 0 && (
        <>
          <span className="text-warning">|</span>
          <button
            type="button"
            className="flex items-center gap-1 text-warning hover:text-warning font-medium transition-colors"
            onClick={toggleBottomPanel}
            title="打开审阅面板"
          >
            <GitPullRequestDraft size={13} />
            <span>{reviewBadgeCount}条待审</span>
          </button>
        </>
      )}

      {/* Agent activity */}
      {agentActivity && (
        <>
          <span className="text-info">|</span>
          <span className="text-info truncate max-w-[300px]">
            [{agentActivity.agent_name}] {getStatusLabel(agentActivity.status)}
            {agentActivity.file_name && ` - ${agentActivity.file_name}`}
            {agentActivity.detail && ` (${agentActivity.detail})`}
          </span>
        </>
      )}

      <span className="ml-auto">v0.2.0-dev | {time}</span>
    </div>
  );
}
