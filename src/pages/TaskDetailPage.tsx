import { useEffect, useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useAppStore } from "@/stores/useAppStore";
import { useEditorStore } from "@/stores/useEditorStore";
import type { TaskDetail, TaskEvent, SourceMeta, AgentStatusChangePayload } from "@/types/task";
import { ArrowLeft, RefreshCw, ChevronDown, ChevronRight, X } from "lucide-react";
import { formatDateTime } from "@/lib/utils";
import FileMetadataPanel from "@/components/task/FileMetadataPanel";
import PipelineStateMachine, { type StageStatus } from "@/components/task/PipelineStateMachine";
import DiffReviewPanel from "@/components/task/DiffReviewPanel";

interface TaskFiles {
  task_dir: string;
  files: string[];
  ingest_result: string;
  resolution_result: string;
  relationship_result: string;
  update_plan: string;
  prompts: Record<string, string>;
  model_responses: Record<string, string>;
  extracted_text: string;
}

interface TaskDetailPageProps {
  taskId: string;
}

// --- Pipeline stage status computation (from existing logic, consolidated to 8 stages) ---

const PIPELINE_STAGES = [
  "document_parsed", "prompt_built", "model_called", "model_returned",
  "json_validated", "resolution_done", "update_plan_done", "review_generated",
] as const;

function computeStageStatuses(task: TaskDetail | null): Record<string, StageStatus> {
  const result: Record<string, StageStatus> = {};
  const taskStatus = task?.status || "";
  const isCancelled = taskStatus === "cancelled" || taskStatus === "cancelling" || taskStatus === "cancelled_after_model_return";

  const stages = PIPELINE_STAGES as readonly string[];

  // Status ordering - later stages imply earlier ones are done
  const doneAfter: Record<string, string[]> = {
    document_parsed: ["prompt_built", "sent_to_model", "model_returned", "json_validating", "json_valid", "json_repaired", "candidate_searching", "resolution_running", "relationship_running", "relationship_completed", "update_plan_generating", "update_plan_generated", "review_generating", "review_pending", "applying", "applied"],
    prompt_built: ["sent_to_model", "model_returned", "json_validating", "json_valid", "json_repaired", "candidate_searching", "resolution_running", "relationship_running", "relationship_completed", "update_plan_generating", "update_plan_generated", "review_generating", "review_pending", "applying", "applied"],
    model_called: ["model_returned", "json_validating", "json_valid", "json_repaired", "candidate_searching", "resolution_running", "relationship_running", "relationship_completed", "update_plan_generating", "update_plan_generated", "review_generating", "review_pending", "applying", "applied"],
    model_returned: ["json_validating", "json_valid", "json_repaired", "candidate_searching", "resolution_running", "relationship_running", "relationship_completed", "update_plan_generating", "update_plan_generated", "review_generating", "review_pending", "applying", "applied"],
    json_validated: ["candidate_searching", "resolution_running", "relationship_running", "relationship_completed", "update_plan_generating", "update_plan_generated", "review_generating", "review_pending", "applying", "applied"],
    resolution_done: ["relationship_running", "relationship_completed", "update_plan_generating", "update_plan_generated", "review_generating", "review_pending", "applying", "applied"],
    update_plan_done: ["update_plan_generated", "review_generating", "review_pending", "applying", "applied"],
    review_generated: ["applying", "applied"],
  };

  for (const stage of stages) {
    if (isCancelled) {
      result[stage] = "cancelled";
      continue;
    }
    const triggers = doneAfter[stage];
    if (triggers && triggers.includes(taskStatus)) {
      result[stage] = "done";
    } else if (triggers && taskStatus === "failed") {
      result[stage] = "failed";
    } else {
      result[stage] = "pending";
    }
  }

  // Mark current stage as running
  if (taskStatus === "prompt_built") result.prompt_built = "running";
  if (taskStatus === "sent_to_model") result.model_called = "running";
  if (taskStatus === "model_returned") result.model_returned = "running";
  if (taskStatus === "json_validating" || taskStatus === "json_valid") result.json_validated = "running";
  if (taskStatus === "candidate_searching" || taskStatus === "resolution_running") result.resolution_done = "running";
  if (taskStatus === "relationship_running") result.resolution_done = "done";
  if (taskStatus === "update_plan_generating") { result.resolution_done = "done"; result.update_plan_done = "running"; }
  if (taskStatus === "review_generating") { result.update_plan_done = "done"; result.review_generated = "running"; }
  if (taskStatus === "review_pending") result.review_generated = "done";
  if (taskStatus === "applying" || taskStatus === "applied") result.review_generated = "done";

  if (taskStatus === "failed" || taskStatus === "pipeline_failed") {
    // Mark only the stage that failed
    for (const stage of stages) {
      if (result[stage] !== "done") result[stage] = "failed";
    }
  }

  return result;
}

function isPipelineComplete(task: TaskDetail | null): boolean {
  if (!task) return false;
  const complete = ["review_pending", "applying", "applied"];
  return complete.includes(task.status);
}

// --- Main component ---

export default function TaskDetailPage({ taskId }: TaskDetailPageProps) {
  const setTaskDetailId = useAppStore((s) => s.setTaskDetailId);

  const [task, setTask] = useState<TaskDetail | null>(null);
  const [events, setEvents] = useState<TaskEvent[]>([]);
  const [files, setFiles] = useState<TaskFiles | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");
  const [actionMsg, setActionMsg] = useState("");
  const [actionLoading, setActionLoading] = useState<string | null>(null);
  const [expandedSections, setExpandedSections] = useState<Record<string, boolean>>({});

  // File preview state
  const [previewFile, setPreviewFile] = useState<{ name: string; content: string; size: number } | null>(null);
  const [previewLoading, setPreviewLoading] = useState(false);

  const handlePreviewFile = async (fileName: string) => {
    if (!task) return;
    setPreviewLoading(true);
    try {
      const result = await invoke<{ name: string; content: string; size: number }>("read_task_file", {
        kbId: task.kb_id,
        taskId,
        fileName,
      });
      setPreviewFile(result);
    } catch (e) {
      setPreviewFile({ name: fileName, content: `读取失败: ${e}`, size: 0 });
    } finally {
      setPreviewLoading(false);
    }
  };

  // Real-time status state
  const [stageStatuses, setStageStatuses] = useState<Record<string, StageStatus>>({});
  const [selectedStage, setSelectedStage] = useState<string | null>(null);
  const [promptText, setPromptText] = useState("");
  const [responseText, setResponseText] = useState("");
  const [logMessages, setLogMessages] = useState<string[]>([]);

  const loadTask = useCallback(async () => {
    setError("");
    try {
      const t = await invoke<TaskDetail>("get_task_detail", { taskId });
      setTask(t);
      setStageStatuses(computeStageStatuses(t));

      const evts = await invoke<TaskEvent[]>("get_task_events", { taskId });
      setEvents(evts);

      try {
        const f = await invoke<TaskFiles>("get_task_files", { kbId: t.kb_id, taskId });
        setFiles(f);
      } catch { setFiles(null); }
    } catch (e) {
      setError(`加载失败: ${e}`);
    } finally {
      setLoading(false);
    }
  }, [taskId]);

  // Initial load
  useEffect(() => {
    loadTask();
  }, [loadTask]);

  // Real-time event listener with cascade: when stage N becomes running, prior stages → done
  useEffect(() => {
    const unlisten = listen<AgentStatusChangePayload>("agent-status-change", (event) => {
      const payload = event.payload;
      if (payload.task_id !== taskId) return;

      // Map backend stage names that don't match 1:1 to frontend stage keys
      const stageKey = payload.stage;

      setStageStatuses((prev) => {
        const next = { ...prev };
        const stageIdx = PIPELINE_STAGES.indexOf(stageKey as typeof PIPELINE_STAGES[number]);

        if (payload.stage_status === "running") {
          // Cascade: mark all prior stages as done
          if (stageIdx >= 0) {
            for (let i = 0; i < stageIdx; i++) {
              next[PIPELINE_STAGES[i]] = "done";
            }
          }
          next[stageKey] = "running";
        } else if (payload.stage_status === "done") {
          if (stageIdx >= 0) {
            // Mark this and all prior as done
            for (let i = 0; i <= stageIdx; i++) {
              next[PIPELINE_STAGES[i]] = "done";
            }
          } else {
            next[stageKey] = "done";
          }
        }

        return next;
      });

      // Update prompt/response/logs
      if (payload.prompt_text) setPromptText(payload.prompt_text);
      if (payload.response_text) setResponseText(payload.response_text);
      if (payload.log_message) {
        setLogMessages((prev) => [...prev.slice(-50), `${payload.timestamp.slice(11, 19)} ${payload.log_message}`]);
      }

      // Auto-select running stage
      if (payload.stage_status === "running") {
        setSelectedStage(payload.stage);
      }

      // Refresh task data on completion
      if (payload.stage === "pipeline_complete") {
        loadTask();
      }
    });

    return () => { unlisten.then((fn) => fn()); };
  }, [taskId, loadTask]);

  // Polling fallback every 3s if task is running
  useEffect(() => {
    if (!task) return;
    const running = ["created", "queued", "prompt_built", "sent_to_model", "model_returned",
      "json_validating", "candidate_searching",
      "resolution_running", "relationship_running", "relationship_completed", "update_plan_generating", "update_plan_generated", "review_generating"];
    if (!running.includes(task.status)) return;

    const interval = setInterval(loadTask, 3000);
    return () => clearInterval(interval);
  }, [task?.status, loadTask]);

  const handleRetry = async () => {
    setActionLoading("retry");
    setActionMsg("");
    try {
      await invoke("retry_task", { taskId });
      setActionMsg("任务已重新入队");
      loadTask();
    } catch (e) {
      setActionMsg(`重试失败: ${e}`);
    } finally {
      setActionLoading(null);
    }
  };

  const handleCancel = async () => {
    setActionLoading("cancel");
    setActionMsg("");
    try {
      await invoke("cancel_task", { taskId });
      setActionMsg("取消请求已发送");
      loadTask();
    } catch (e) {
      setActionMsg(`取消失败: ${e}`);
    } finally {
      setActionLoading(null);
    }
  };

  const handleResume = async () => {
    setActionLoading("resume");
    setActionMsg("");
    try {
      await invoke("resume_task", { taskId });
      setActionMsg("任务已恢复");
      loadTask();
    } catch (e) {
      setActionMsg(`恢复失败: ${e}`);
    } finally {
      setActionLoading(null);
    }
  };

  const handleArchive = async () => {
    setActionLoading("archive");
    setActionMsg("");
    try {
      await invoke("archive_task", { taskId });
      setActionMsg("任务已归档");
    } catch (e) {
      setActionMsg(`归档失败: ${e}`);
    } finally {
      setActionLoading(null);
    }
  };

  const handlePreview = () => {
    if (!task?.kb_id) return;
    const fileList = files?.files ?? [];
    if (!fileList.includes("extracted_text.txt")) {
      setPreviewFile({
        name: "extracted_text.txt",
        content: "文件尚未生成。请等待文档处理任务完成「源文件提取」阶段后再预览。",
        size: 0,
      });
      return;
    }
    handlePreviewFile("extracted_text.txt");
  };

  const handleBack = () => {
    setTaskDetailId(null);
    const { openFile, closeTab, openTabs } = useEditorStore.getState();
    const taskTab = openTabs.find((t) => t.type === "task_detail");
    if (taskTab) closeTab(taskTab.id);
    openFile({ path: "file_explorer", title: "文件浏览", type: "file_explorer" });
  };

  if (loading) {
    return (
      <div className="flex-1 flex items-center justify-center gap-3 text-muted-foreground">
        <RefreshCw size={20} className="animate-spin" />
        <span className="text-sm">加载任务详情...</span>
      </div>
    );
  }

  if (error) {
    return (
      <div className="flex-1 flex flex-col items-center justify-center gap-4 p-8">
        <div className="text-red-400 text-sm">{error}</div>
        <button onClick={loadTask} className="px-4 py-2 rounded-md bg-card border border-border text-sm hover:bg-accent">
          重试
        </button>
      </div>
    );
  }

  if (!task) {
    return (
      <div className="flex-1 flex items-center justify-center text-muted-foreground text-sm">
        任务不存在
      </div>
    );
  }

  const pipelineComplete = isPipelineComplete(task);
  const statusLabel: Record<string, string> = {
    created: "已创建", queued: "已排队", prompt_built: "Prompt 已构建",
    sent_to_model: "模型调用中", model_returned: "模型已返回",
    json_validating: "JSON 校验中", json_valid: "JSON 有效", json_repaired: "JSON 已修复",
    candidate_searching: "候选检索中", resolution_running: "消歧处理中",
    relationship_running: "关系分析中", update_plan_generating: "生成更新计划中",
    review_generating: "审阅生成中", review_pending: "待审阅",
    applying: "应用中", applied: "已完成",
    failed: "失败", pipeline_failed: "流水线失败",
    cancelled: "已取消", cancelling: "取消中", cancelled_after_model_return: "已取消",
    interrupted: "已中断",
  };

  return (
    <div className="flex-1 flex flex-col overflow-hidden bg-background">
      {/* Header */}
      <div className="flex items-center gap-3 px-5 py-3 border-b border-border bg-card">
        <button onClick={handleBack} className="p-1.5 rounded-md hover:bg-accent transition-colors">
          <ArrowLeft size={18} />
        </button>
        <div className="flex-1 min-w-0">
          <h2 className="text-sm font-semibold truncate">
            {task.task_name || task.id}
          </h2>
          <div className="flex items-center gap-2 mt-0.5">
            <span className="text-xs text-muted-foreground">{task.id}</span>
            <span className={cn(
              "text-xs px-1.5 py-0.5 rounded-full font-medium",
              pipelineComplete && "bg-emerald-500/10 text-emerald-400",
              task.status === "failed" && "bg-red-500/10 text-red-400",
              (task.status === "cancelled" || task.status === "cancelling") && "bg-amber-500/10 text-amber-400",
              !pipelineComplete && task.status !== "failed" && !task.status.startsWith("cancelled") && "bg-blue-500/10 text-blue-400",
            )}>
              {statusLabel[task.status] || task.status}
            </span>
          </div>
        </div>

        {/* Actions */}
        <div className="flex items-center gap-1.5">
          {(task.status === "failed" || task.status === "pipeline_failed" || task.status === "interrupted") && (
            <>
              <button onClick={handleRetry} disabled={actionLoading === "retry"} className="px-2.5 py-1.5 rounded-md bg-blue-500/10 text-blue-400 hover:bg-blue-500/20 text-xs transition-colors disabled:opacity-50">
                {actionLoading === "retry" ? "重试中..." : "重试"}
              </button>
              {task.recoverable && (
                <button onClick={handleResume} disabled={actionLoading === "resume"} className="px-2.5 py-1.5 rounded-md bg-emerald-500/10 text-emerald-400 hover:bg-emerald-500/20 text-xs transition-colors disabled:opacity-50">
                  {actionLoading === "resume" ? "恢复中..." : "断点续传"}
                </button>
              )}
            </>
          )}
          {!["applied", "failed", "pipeline_failed", "cancelled", "cancelled_after_model_return"].includes(task.status) && (
            <button onClick={handleCancel} disabled={actionLoading === "cancel"} className="px-2.5 py-1.5 rounded-md bg-amber-500/10 text-amber-400 hover:bg-amber-500/20 text-xs transition-colors disabled:opacity-50">
              {actionLoading === "cancel" ? "取消中..." : "取消"}
            </button>
          )}
          {["applied", "failed", "cancelled", "cancelled_after_model_return"].includes(task.status) && (
            <button onClick={handleArchive} disabled={actionLoading === "archive"} className="px-2.5 py-1.5 rounded-md bg-card border border-border text-xs hover:bg-accent transition-colors disabled:opacity-50">
              归档
            </button>
          )}
        </div>
      </div>

      {actionMsg && (
        <div className="px-5 py-2 bg-muted/30 border-b border-border text-xs text-muted-foreground">
          {actionMsg}
        </div>
      )}

      {/* Scrollable content */}
      <div className="flex-1 overflow-auto">
        {/* Section 1: File Metadata */}
        <FileMetadataPanel
          sourceMeta={task.source_meta || null}
          onPreview={handlePreview}
        />

        {/* Section 2: Pipeline State Machine */}
        <PipelineStateMachine
          statuses={stageStatuses}
          selectedStage={selectedStage}
          onSelectStage={setSelectedStage}
          promptText={promptText}
          responseText={responseText}
          logMessages={logMessages}
        />

        {/* Section 3: Diff Review Panel */}
        <div className={cn(!pipelineComplete && "h-64")}>
          <DiffReviewPanel
            taskId={taskId}
            isPipelineComplete={pipelineComplete}
          />
        </div>

        {/* Events log (collapsible) */}
        {events.length > 0 && (
          <div className="border-t border-border">
            <button
              onClick={() => setExpandedSections((p) => ({ ...p, events: !p.events }))}
              className="flex items-center gap-1.5 px-5 py-2.5 w-full text-left text-xs font-medium text-muted-foreground hover:bg-muted/30 transition-colors"
            >
              {expandedSections.events ? <ChevronDown size={14} /> : <ChevronRight size={14} />}
              任务事件日志 ({events.length})
            </button>
            {expandedSections.events && (
              <div className="px-5 pb-3 space-y-1 max-h-48 overflow-auto">
                {events.map((evt) => (
                  <div key={evt.id} className="flex gap-3 text-xs">
                    <span className="text-muted-foreground shrink-0">{formatDateTime(evt.created_at)}</span>
                    <span className="text-muted-foreground shrink-0">[{evt.agent_name || evt.event_type}]</span>
                    <span>{evt.message}</span>
                  </div>
                ))}
              </div>
            )}
          </div>
        )}

        {/* Intermediate files (collapsible, for debugging) */}
        {files && files.files.length > 0 && (
          <div className="border-t border-border">
            <button
              onClick={() => setExpandedSections((p) => ({ ...p, files: !p.files }))}
              className="flex items-center gap-1.5 px-5 py-2.5 w-full text-left text-xs font-medium text-muted-foreground hover:bg-muted/30 transition-colors"
            >
              {expandedSections.files ? <ChevronDown size={14} /> : <ChevronRight size={14} />}
              中间文件 ({files.files.length})
            </button>
            {expandedSections.files && (
              <div className="px-5 pb-3 space-y-1 max-h-48 overflow-auto">
                {files.files.map((f) => (
                  <button
                    key={f}
                    type="button"
                    onClick={() => handlePreviewFile(f)}
                    className="block w-full text-left text-xs text-muted-foreground font-mono hover:text-foreground hover:bg-accent px-1 py-0.5 rounded transition-colors truncate"
                    title={f}
                  >
                    {f}
                  </button>
                ))}
              </div>
            )}
          </div>
        )}
      </div>

      {/* File preview modal */}
      {previewFile && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/40" onClick={() => setPreviewFile(null)}>
          <div className="bg-card border border-border rounded-lg shadow-xl w-[720px] max-h-[80vh] flex flex-col" onClick={(e) => e.stopPropagation()}>
            <div className="flex items-center justify-between px-4 py-3 border-b border-border">
              <h3 className="text-sm font-medium truncate">{previewFile.name}</h3>
              <span className="text-xs text-muted-foreground ml-2">{previewFile.size.toLocaleString()} bytes</span>
              <button type="button" onClick={() => setPreviewFile(null)} className="ml-auto p-1 rounded hover:bg-accent transition-colors" title="关闭">
                <X size={16} />
              </button>
            </div>
            <pre className="flex-1 overflow-auto p-4 text-xs font-mono leading-relaxed whitespace-pre-wrap">
              {previewFile.content}
            </pre>
          </div>
        </div>
      )}

      {/* Preview loading overlay */}
      {previewLoading && (
        <div className="fixed inset-0 z-40 flex items-center justify-center bg-black/20">
          <div className="bg-card border border-border rounded-lg px-4 py-3 shadow-lg flex items-center gap-2 text-sm text-muted-foreground">
            <RefreshCw size={16} className="animate-spin" />
            读取文件中...
          </div>
        </div>
      )}
    </div>
  );
}

function cn(...args: (string | boolean | undefined | null)[]): string {
  return args.filter(Boolean).join(" ");
}
