import { useAppStore } from "@/stores/useAppStore";
import TaskDetailPage from "@/pages/TaskDetailPage";

export default function TaskDetailView() {
  const taskDetailId = useAppStore((s) => s.taskDetailId);

  if (!taskDetailId) {
    return (
      <div className="flex-1 flex items-center justify-center text-muted-foreground">
        未选择任务
      </div>
    );
  }

  return <TaskDetailPage taskId={taskDetailId} />;
}
