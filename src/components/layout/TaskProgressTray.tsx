import { useEffect, useState } from "react";
import { Loader2, ChevronUp, X } from "lucide-react";
import { useTaskStore, type TaskInfo } from "@/stores/useTaskStore";
import { cn } from "@/lib/utils";

interface TaskProgressPayload {
  task_id: string;
  stage: string;
  progress: number;
  current_file: string;
}

export default function TaskProgressTray() {
  const { tasks, addTask, updateTask, removeTask, clearCompleted } = useTaskStore();
  const [expanded, setExpanded] = useState(false);

  const activeTasks = tasks.filter((t) => t.status === "running");
  const runningCount = activeTasks.length;

  useEffect(() => {
    let unlistenFn: (() => void) | undefined;

    (async () => {
      try {
        const { listen } = await import("@tauri-apps/api/event");
        const unlisten = await listen<TaskProgressPayload>("task-progress", (event) => {
          const { task_id, stage, progress, current_file } = event.payload;
          const exists = useTaskStore.getState().tasks.some((t) => t.id === task_id);
          if (!exists) {
            addTask({
              id: task_id,
              fileName: current_file,
              stage,
              progress,
              status: "running",
            });
          } else {
            updateTask(task_id, {
              fileName: current_file || undefined,
              stage,
              progress,
              status: progress >= 100 ? "completed" : "running",
            });
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

  if (runningCount === 0 && !expanded) return null;

  return (
    <>
      {/* Floating button */}
      <button
        type="button"
        onClick={() => setExpanded(!expanded)}
        className={cn(
          "fixed bottom-10 left-4 z-40 flex items-center gap-2 px-3 py-2",
          "bg-card/90 backdrop-blur border border-border rounded-lg",
          "text-xs text-foreground-dim hover:text-foreground hover:border-[#404040]",
          "shadow-lg transition-all",
          runningCount > 0 && "text-sidebar-icon-active border-[#a78bfa]/30"
        )}
      >
        <Loader2
          size={14}
          className={cn(runningCount > 0 && "animate-spin text-sidebar-icon-active")}
        />
        {runningCount > 0 ? (
          <span>
            {runningCount} 个任务运行中
          </span>
        ) : (
          <span>任务</span>
        )}
        <ChevronUp
          size={12}
          className={cn(
            "transition-transform duration-200",
            expanded && "rotate-180"
          )}
        />
      </button>

      {/* Expanded tray */}
      {expanded && (
        <div className="fixed bottom-[72px] left-4 z-40 w-80 bg-card/95 backdrop-blur border border-border rounded-lg shadow-xl overflow-hidden">
          {/* Header */}
          <div className="flex items-center justify-between px-3 py-2 border-b border-border">
            <span className="text-xs font-medium text-foreground">
              任务进度
            </span>
            <div className="flex items-center gap-1">
              {tasks.some((t) => t.status === "completed") && (
                <button
                  type="button"
                  onClick={clearCompleted}
                  className="text-[10px] text-foreground-muted hover:text-foreground-dim px-1.5 py-0.5 rounded transition-colors"
                >
                  清除已完成
                </button>
              )}
              <button
                type="button"
                onClick={() => setExpanded(false)}
                className="text-foreground-muted hover:text-foreground p-0.5 rounded transition-colors"
                title="关闭"
              >
                <X size={12} />
              </button>
            </div>
          </div>

          {/* Task list */}
          <div className="max-h-64 overflow-y-auto">
            {tasks.filter((t) => t.status !== "completed").length === 0 &&
            tasks.some((t) => t.status === "completed") ? (
              <div className="px-3 py-6 text-center text-xs text-foreground-muted">
                所有任务已完成
              </div>
            ) : (
              tasks
                .filter((t) => t.status !== "completed")
                .map((task) => (
                  <TaskRow
                    key={task.id}
                    task={task}
                    onDismiss={() => removeTask(task.id)}
                  />
                ))
            )}
          </div>
        </div>
      )}
    </>
  );
}

function TaskRow({ task, onDismiss }: { task: TaskInfo; onDismiss: () => void }) {
  const isRunning = task.status === "running";
  const isFailed = task.status === "failed";

  return (
    <div className="px-3 py-2 border-b border-border/50 last:border-b-0">
      <div className="flex items-center justify-between mb-1">
        <span className="text-xs text-foreground truncate flex-1 mr-2">
          {task.fileName}
        </span>
        <div className="flex items-center gap-1 shrink-0">
          <span
            className={cn(
              "text-[10px]",
              isRunning && "text-sidebar-icon-active",
              isFailed && "text-destructive",
              task.status === "completed" && "text-success"
            )}
          >
            {task.stage}
          </span>
          {isFailed && (
            <button
              type="button"
              onClick={onDismiss}
              className="text-foreground-muted hover:text-foreground p-0.5 rounded transition-colors"
              title="关闭"
            >
              <X size={10} />
            </button>
          )}
        </div>
      </div>
      <div className="w-full h-1 bg-muted rounded-full overflow-hidden">
        <div
          className={cn(
            "h-full rounded-full transition-all duration-500 ease-out",
            isRunning && "bg-sidebar-icon-active",
            isFailed && "bg-destructive",
            task.status === "completed" && "bg-success"
          )}
          style={{ width: `${Math.min(100, Math.max(0, task.progress))}%` }}
        />
      </div>
      {task.error && (
        <p className="text-[10px] text-destructive mt-1 truncate">{task.error}</p>
      )}
    </div>
  );
}
