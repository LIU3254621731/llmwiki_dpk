import { useState } from "react";
import { cn } from "@/lib/utils";
import {
  FileText, Brain, FileJson, Database, BookOpen, ChevronDown, Loader2,
  CheckCircle2, XCircle, SkipForward, Circle,
} from "lucide-react";
import type { AgentStatusChangePayload } from "@/types/task";

export type StageStatus = "pending" | "running" | "done" | "failed" | "skipped" | "cancelled";

export interface PipelineStage {
  key: string;
  label: string;
  icon: typeof FileText;
}

const STAGES: PipelineStage[] = [
  { key: "document_parsed",   label: "文档解析",     icon: FileText },
  { key: "prompt_built",      label: "Prompt 构建",  icon: FileText },
  { key: "model_called",      label: "模型调用",     icon: Brain },
  { key: "model_returned",    label: "模型返回",     icon: Brain },
  { key: "json_validated",    label: "JSON 校验",    icon: FileJson },
  { key: "resolution_done",   label: "消歧去重",     icon: Database },
  { key: "update_plan_done",  label: "生成更新计划",  icon: BookOpen },
  { key: "review_generated",  label: "生成审阅",     icon: FileJson },
];

interface PipelineStateMachineProps {
  statuses: Record<string, StageStatus>;
  selectedStage: string | null;
  onSelectStage: (key: string) => void;
  promptText: string;
  responseText: string;
  logMessages: string[];
}

const statusConfig: Record<StageStatus, { bg: string; border: string; dot: string; animation?: string }> = {
  pending:    { bg: "bg-card", border: "border-border", dot: "bg-slate-300" },
  running:    { bg: "bg-blue-50 dark:bg-blue-950/30", border: "border-blue-400", dot: "bg-blue-500", animation: "animate-pulse" },
  done:       { bg: "bg-emerald-50 dark:bg-emerald-950/30", border: "border-emerald-400", dot: "bg-emerald-500" },
  failed:     { bg: "bg-red-50 dark:bg-red-950/30", border: "border-red-400", dot: "bg-red-500" },
  skipped:    { bg: "bg-card", border: "border-border", dot: "bg-slate-400" },
  cancelled:  { bg: "bg-amber-50 dark:bg-amber-950/30", border: "border-amber-400", dot: "bg-amber-500" },
};

function StatusIcon({ status }: { status: StageStatus }) {
  if (status === "running") return <Loader2 size={16} className="animate-spin text-blue-500" />;
  if (status === "done") return <CheckCircle2 size={16} className="text-emerald-500" />;
  if (status === "failed") return <XCircle size={16} className="text-red-500" />;
  if (status === "skipped") return <SkipForward size={16} className="text-slate-400" />;
  return <Circle size={16} className="text-slate-300" />;
}

export default function PipelineStateMachine({
  statuses, selectedStage, onSelectStage, promptText, responseText, logMessages,
}: PipelineStateMachineProps) {
  const [expanded, setExpanded] = useState(true);

  return (
    <div className="flex flex-col border-b border-border">
      {/* Stage nodes row */}
      <div className="px-5 py-4">
        <div className="flex items-center gap-1 overflow-x-auto">
          {STAGES.map((stage, idx) => {
            const status = statuses[stage.key] || "pending";
            const config = statusConfig[status];
            const Icon = stage.icon;
            const isSelected = selectedStage === stage.key;

            return (
              <div key={stage.key} className="flex items-center">
                <button
                  onClick={() => onSelectStage(stage.key)}
                  className={cn(
                    "flex items-center gap-1.5 px-3 py-2 rounded-lg border text-xs font-medium transition-all shrink-0",
                    config.border, config.bg,
                    isSelected && "ring-2 ring-blue-400",
                    "hover:shadow-sm cursor-pointer"
                  )}
                >
                  <Icon size={14} className={status === "running" ? "text-blue-500" : "text-muted-foreground"} />
                  <span className="hidden sm:inline">{stage.label}</span>
                  <StatusIcon status={status} />
                </button>
                {idx < STAGES.length - 1 && (
                  <div className={cn(
                    "w-4 h-0.5 shrink-0",
                    status === "done" || status === "skipped" ? "bg-emerald-400" : "bg-border"
                  )} />
                )}
              </div>
            );
          })}
        </div>
      </div>

      {/* Expanded detail panel */}
      {selectedStage && (
        <div className="px-5 pb-4 border-t border-border bg-muted/30">
          <button
            onClick={() => setExpanded(!expanded)}
            className="flex items-center gap-1 py-2 text-xs text-muted-foreground hover:text-foreground"
          >
            <ChevronDown size={12} className={cn("transition-transform", expanded && "rotate-180")} />
            {expanded ? "收起" : "展开"} Prompt / 响应详情
          </button>

          {expanded && (
            <div className="space-y-3">
              {promptText && (
                <div>
                  <div className="text-xs font-medium text-muted-foreground mb-1">完整 Prompt (System + User)</div>
                  <pre className="text-xs bg-card border border-border rounded-md p-3 max-h-48 overflow-auto whitespace-pre-wrap font-mono leading-relaxed">
                    {promptText}
                  </pre>
                </div>
              )}
              {responseText && (
                <div>
                  <div className="text-xs font-medium text-muted-foreground mb-1">模型回复</div>
                  <pre className="text-xs bg-card border border-border rounded-md p-3 max-h-48 overflow-auto whitespace-pre-wrap font-mono leading-relaxed">
                    {responseText}
                  </pre>
                </div>
              )}
              {logMessages.length > 0 && (
                <div>
                  <div className="text-xs font-medium text-muted-foreground mb-1">日志</div>
                  <div className="text-xs bg-card border border-border rounded-md p-3 max-h-32 overflow-auto space-y-0.5">
                    {logMessages.map((msg, i) => (
                      <div key={i} className="text-muted-foreground">{msg}</div>
                    ))}
                  </div>
                </div>
              )}
              {!promptText && !responseText && logMessages.length === 0 && (
                <div className="text-xs text-muted-foreground py-2">
                  暂无该阶段的详细信息
                </div>
              )}
            </div>
          )}
        </div>
      )}
    </div>
  );
}
