import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useKBStore } from "@/stores/useKBStore";
import { Loader2, Check, X, GitMerge, Plus, Pencil, Link, ArrowRight } from "lucide-react";
import { cn } from "@/lib/utils";
import { computeDiff } from "@/lib/diff";

interface ReviewItemRaw {
  id: string;
  operation: string;
  target_path: string;
  old_content: string;
  new_content: string;
  status: string;
  risk_level: string;
  title: string;
  summary: string;
  page_type: string;
  created_at: string;
}

interface DiffReviewPanelProps {
  taskId: string;
  isPipelineComplete: boolean;
}

const OP_LABELS: Record<string, string> = {
  create: "新建", update: "更新", append: "追加",
  add_alias: "添加别名", add_relation: "添加关系",
  create_page: "新建页面", update_page: "更新页面",
  append_section: "追加段落", delete_page: "删除页面",
  merge_suggestion: "合并建议", skip: "跳过",
};
const OP_COLORS: Record<string, string> = {
  create: "bg-emerald-500/10 text-emerald-400", create_page: "bg-emerald-500/10 text-emerald-400",
  update: "bg-blue-400/10 text-blue-400", update_page: "bg-blue-400/10 text-blue-400",
  append: "bg-purple-400/10 text-purple-400", append_section: "bg-purple-400/10 text-purple-400",
  add_alias: "bg-amber-500/10 text-amber-400",
  add_relation: "bg-pink-400/10 text-pink-400",
  delete_page: "bg-red-500/10 text-red-400",
};

export default function DiffReviewPanel({ taskId, isPipelineComplete }: DiffReviewPanelProps) {
  const currentKB = useKBStore((s) => s.currentKB);
  const [items, setItems] = useState<ReviewItemRaw[]>([]);
  const [loading, setLoading] = useState(true);
  const [actionMsg, setActionMsg] = useState("");

  useEffect(() => {
    if (!isPipelineComplete || !taskId) return;
    setLoading(true);
    invoke<ReviewItemRaw[]>("get_task_review_items", { taskId })
      .then((data) => setItems(data))
      .catch((e) => setActionMsg(`加载审阅项失败: ${e}`))
      .finally(() => setLoading(false));
  }, [isPipelineComplete, taskId]);

  const handleAccept = async (itemId: string) => {
    if (!currentKB) return;
    try {
      await invoke("accept_review_item", { itemId, kbId: currentKB.id, kbPath: currentKB.path });
      setItems((prev) => prev.map((it) => it.id === itemId ? { ...it, status: "applied" } : it));
      setActionMsg("已接受");
    } catch (e) {
      setActionMsg(`接受失败: ${e}`);
    }
  };

  const handleReject = async (itemId: string) => {
    try {
      await invoke("reject_review_item", { itemId });
      setItems((prev) => prev.map((it) => it.id === itemId ? { ...it, status: "rejected" } : it));
      setActionMsg("已拒绝");
    } catch (e) {
      setActionMsg(`拒绝失败: ${e}`);
    }
  };

  if (!isPipelineComplete) {
    return (
      <div className="flex-1 flex flex-col items-center justify-center gap-3 p-8 text-muted-foreground">
        <Loader2 size={24} className="animate-spin" />
        <span className="text-sm">等待 Agent 流水线完成...</span>
      </div>
    );
  }

  if (loading) {
    return (
      <div className="flex-1 space-y-3 p-4">
        {[1, 2, 3].map((i) => (
          <div key={i} className="animate-pulse bg-card border border-border rounded-lg p-4 h-24" />
        ))}
      </div>
    );
  }

  if (items.length === 0) {
    return (
      <div className="flex-1 flex items-center justify-center p-8 text-sm text-muted-foreground">
        暂无疑议项
      </div>
    );
  }

  return (
    <div className="flex-1 overflow-auto p-4 space-y-3">
      <div className="flex items-center justify-between mb-2">
        <h3 className="text-sm font-semibold flex items-center gap-1.5">
          <GitMerge size={16} /> AI 审阅建议 ({items.length})
        </h3>
        {actionMsg && <span className="text-xs text-muted-foreground">{actionMsg}</span>}
      </div>

      {items.map((item) => {
        const opLabel = OP_LABELS[item.operation] || item.operation;
        const opColor = OP_COLORS[item.operation] || "bg-slate-400/10 text-slate-400";
        const isResolved = item.status === "applied" || item.status === "rejected";
        const diffLines = computeDiff(item.old_content || "", item.new_content || "");
        const hasContent = item.old_content || item.new_content;

        return (
          <div key={item.id} className={cn(
            "bg-card border border-border rounded-lg overflow-hidden",
            isResolved && "opacity-60"
          )}>
            {/* Header */}
            <div className="flex items-center justify-between px-4 py-2.5 border-b border-border bg-muted/30">
              <div className="flex items-center gap-2 text-sm min-w-0">
                <span className={cn("px-2 py-0.5 rounded text-xs font-medium", opColor)}>
                  {opLabel}
                </span>
                <span className="font-medium truncate">{item.title || item.target_path}</span>
                {item.risk_level === "high" && (
                  <span className="text-xs text-red-400 font-medium">高风险</span>
                )}
              </div>
              {!isResolved && (
                <div className="flex items-center gap-1.5 shrink-0 ml-2">
                  <button
                    onClick={() => handleAccept(item.id)}
                    className="inline-flex items-center gap-1 px-2.5 py-1 rounded-md bg-emerald-500/10 text-emerald-400 hover:bg-emerald-500/20 text-xs transition-colors"
                  >
                    <Check size={12} /> 接受
                  </button>
                  <button
                    onClick={() => handleReject(item.id)}
                    className="inline-flex items-center gap-1 px-2.5 py-1 rounded-md bg-red-500/10 text-red-400 hover:bg-red-500/20 text-xs transition-colors"
                  >
                    <X size={12} /> 拒绝
                  </button>
                </div>
              )}
              {isResolved && (
                <span className={cn(
                  "text-xs px-2 py-0.5 rounded",
                  item.status === "applied" ? "bg-emerald-500/10 text-emerald-400" : "bg-red-500/10 text-red-400"
                )}>
                  {item.status === "applied" ? "已应用" : "已拒绝"}
                </span>
              )}
            </div>

            {/* Summary */}
            {item.summary && (
              <div className="px-4 py-2 text-xs text-muted-foreground border-b border-border">
                {item.summary}
              </div>
            )}

            {/* Diff */}
            {hasContent && (
              <div className="text-xs font-mono max-h-64 overflow-auto">
                {diffLines.map((line, i) => (
                  <div
                    key={i}
                    className={cn(
                      "px-4 py-0.5 leading-relaxed whitespace-pre-wrap",
                      line.type === "added" && "bg-emerald-500/10 text-emerald-400",
                      line.type === "removed" && "bg-red-500/10 text-red-400",
                      line.type === "same" && "text-muted-foreground",
                    )}
                  >
                    <span className="inline-block w-5 text-center mr-2 select-none">
                      {line.type === "added" ? "+" : line.type === "removed" ? "-" : " "}
                    </span>
                    {line.text}
                  </div>
                ))}
              </div>
            )}
          </div>
        );
      })}
    </div>
  );
}
