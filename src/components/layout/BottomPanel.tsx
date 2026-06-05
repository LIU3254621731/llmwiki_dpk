import { useEffect, useRef, useState, useCallback } from "react";
import { useAppStore } from "@/stores/useAppStore";
import { useKBStore } from "@/stores/useKBStore";
import { useReviewStore } from "@/stores/useReviewStore";
import { cn } from "@/lib/utils";
import {
  X, Check, Loader2, ChevronDown,
  FileText, AlertTriangle, Plus, Pencil, Link, ArrowRight, CheckCircle2,
} from "lucide-react";
import type { ReviewItem } from "@/types/review";

const OP_LABELS: Record<string, string> = {
  create: "新建",
  update: "更新",
  append: "追加",
  add_alias: "添加别名",
  add_relation: "添加关系",
};

const OP_COLORS: Record<string, string> = {
  create: "bg-green-50 text-green-600 dark:bg-green-900/20 dark:text-green-400",
  update: "bg-blue-50 text-blue-600 dark:bg-blue-900/20 dark:text-blue-400",
  append: "bg-purple-50 text-purple-600 dark:bg-purple-900/20 dark:text-purple-400",
  add_alias: "bg-amber-50 text-amber-600 dark:bg-amber-900/20 dark:text-amber-400",
  add_relation: "bg-pink-50 text-pink-600 dark:bg-pink-900/20 dark:text-pink-400",
};

// Simple line-level diff
function computeDiff(oldText: string, newText: string): Array<{
  type: "same" | "added" | "removed";
  text: string;
}> {
  const oldLines = (oldText || "").split("\n");
  const newLines = (newText || "").split("\n");
  const result: Array<{ type: "same" | "added" | "removed"; text: string }> = [];

  const maxLen = Math.max(oldLines.length, newLines.length);

  for (let i = 0; i < maxLen; i++) {
    const oldLine = oldLines[i];
    const newLine = newLines[i];

    if (oldLine === undefined) {
      result.push({ type: "added", text: newLines[i] });
    } else if (newLine === undefined) {
      result.push({ type: "removed", text: oldLines[i] });
    } else if (oldLine === newLine) {
      result.push({ type: "same", text: oldLine });
    } else {
      result.push({ type: "removed", text: oldLines[i] });
      result.push({ type: "added", text: newLines[i] });
    }
  }

  return result;
}

function getOpIcon(opType: string) {
  switch (opType) {
    case "create":
      return <Plus size={10} />;
    case "update":
      return <Pencil size={10} />;
    case "add_alias":
      return <Link size={10} />;
    case "add_relation":
      return <ArrowRight size={10} />;
    default:
      return <FileText size={10} />;
  }
}

// ---- Review Item Card ----
function ReviewItemCard({
  item,
  onAccept,
  onReject,
}: {
  item: ReviewItem;
  onAccept: () => void;
  onReject: () => void;
}) {
  const [showDiff, setShowDiff] = useState(false);
  const [processing, setProcessing] = useState(false);
  const [cardError, setCardError] = useState("");
  const opType = item.operation_type || "update";
  const opLabel = OP_LABELS[opType] || opType;
  const opColor = OP_COLORS[opType] || "bg-slate-50 text-slate-600";
  const diffLines = showDiff ? computeDiff(item.old_content, item.new_content) : [];

  const handleAccept = async () => {
    setProcessing(true);
    setCardError("");
    try {
      await onAccept();
    } catch (e) {
      setCardError(`操作失败: ${e}`);
    } finally {
      setProcessing(false);
    }
  };

  const handleReject = async () => {
    setProcessing(true);
    setCardError("");
    try {
      await onReject();
    } catch (e) {
      setCardError(`操作失败: ${e}`);
    } finally {
      setProcessing(false);
    }
  };

  return (
    <div className="bg-white dark:bg-slate-800 border border-slate-200 dark:border-slate-700 rounded-lg overflow-hidden">
      {/* Card header */}
      <div className="flex items-center gap-2 px-3 py-2 border-b border-slate-100 dark:border-slate-700">
        <span
          className={cn(
            "inline-flex items-center gap-1 px-1.5 py-0.5 rounded text-[10px] font-medium",
            opColor
          )}
        >
          {getOpIcon(opType)}
          {opLabel}
        </span>
        <span className="text-xs font-mono text-slate-500 dark:text-slate-400 truncate flex-1">
          {item.target_path}
        </span>
        <button
          type="button"
          onClick={() => setShowDiff(!showDiff)}
          className="text-xs text-brand-500 hover:text-brand-600 shrink-0"
        >
          {showDiff ? "收起" : "查看差异"}
        </button>
      </div>

      {/* Summary / Reason */}
      {(item.summary || item.reason) && (
        <div className="px-3 py-1.5 text-xs text-slate-600 dark:text-slate-400 bg-slate-50 dark:bg-slate-800/50">
          {item.summary || item.reason}
        </div>
      )}

      {/* Diff view */}
      {showDiff && (
        <div className="border-t border-slate-100 dark:border-slate-700">
          <div className="max-h-64 overflow-y-auto">
            <div className="font-mono text-xs leading-5">
              {diffLines.map((line, i) => (
                <div
                  key={i}
                  className={cn(
                    "px-3 py-0 whitespace-pre-wrap",
                    line.type === "added" &&
                      "bg-green-50 dark:bg-green-900/20 text-green-800 dark:text-green-300",
                    line.type === "removed" &&
                      "bg-red-50 dark:bg-red-900/20 text-red-800 dark:text-red-300",
                    line.type === "same" && "text-slate-500 dark:text-slate-400"
                  )}
                >
                  <span className="w-4 inline-block text-slate-300 dark:text-slate-600 select-none shrink-0">
                    {line.type === "added" ? "+" : line.type === "removed" ? "-" : " "}
                  </span>
                  {line.text}
                </div>
              ))}
            </div>
          </div>
        </div>
      )}

      {/* Action buttons */}
      <div className="flex items-center gap-2 px-3 py-2 border-t border-slate-100 dark:border-slate-700">
        <button
          type="button"
          onClick={handleAccept}
          disabled={processing}
          className="flex items-center gap-1 px-3 py-1 text-xs rounded-md bg-green-500 text-white hover:bg-green-600 disabled:opacity-50 transition-colors"
        >
          {processing ? <Loader2 size={12} className="animate-spin" /> : <Check size={12} />}
          接受
        </button>
        <button
          type="button"
          onClick={handleReject}
          disabled={processing}
          className="flex items-center gap-1 px-3 py-1 text-xs rounded-md bg-red-500 text-white hover:bg-red-600 disabled:opacity-50 transition-colors"
        >
          {processing ? <Loader2 size={12} className="animate-spin" /> : <X size={12} />}
          拒绝
        </button>
        {item.confidence && (
          <span className="ml-auto text-[10px] text-slate-400">
            可信度: {item.confidence}
          </span>
        )}
      </div>

      {/* Error message */}
      {cardError && (
        <div className="px-3 py-1.5 text-[10px] text-red-600 dark:text-red-400 bg-red-50 dark:bg-red-900/20 border-t border-red-100 dark:border-red-900/30">
          {cardError}
        </div>
      )}
    </div>
  );
}

// ---- Main BottomPanel ----
export default function BottomPanel() {
  const bottomPanelVisible = useAppStore((s) => s.bottomPanelVisible);
  const bottomPanelHeight = useAppStore((s) => s.bottomPanelHeight);
  const reviewBadgeCount = useAppStore((s) => s.reviewBadgeCount);
  const toggleBottomPanel = useAppStore((s) => s.toggleBottomPanel);
  const setBottomPanelHeight = useAppStore((s) => s.setBottomPanelHeight);

  const currentKB = useKBStore((s) => s.currentKB);
  const pendingItems = useReviewStore((s) => s.pendingItems);
  const pendingCount = useReviewStore((s) => s.pendingCount);
  const loading = useReviewStore((s) => s.loading);
  const loadPendingReviews = useReviewStore((s) => s.loadPendingReviews);
  const acceptItem = useReviewStore((s) => s.acceptItem);
  const rejectItem = useReviewStore((s) => s.rejectItem);

  const dragRef = useRef<{ startY: number; startHeight: number } | null>(null);
  const [isDragging, setIsDragging] = useState(false);

  // Load reviews when KB changes
  useEffect(() => {
    if (currentKB?.id) {
      loadPendingReviews(currentKB.id);
    }
  }, [currentKB?.id, loadPendingReviews]);

  // Drag resize handlers
  const handleDragStart = useCallback(
    (e: React.MouseEvent) => {
      e.preventDefault();
      dragRef.current = { startY: e.clientY, startHeight: bottomPanelHeight };
      setIsDragging(true);
    },
    [bottomPanelHeight]
  );

  useEffect(() => {
    if (!isDragging) return;

    const handleMove = (e: MouseEvent) => {
      if (!dragRef.current) return;
      const delta = dragRef.current.startY - e.clientY;
      const newHeight = Math.max(100, Math.min(600, dragRef.current.startHeight + delta));
      setBottomPanelHeight(newHeight);
    };

    const cleanup = () => {
      dragRef.current = null;
      setIsDragging(false);
      document.body.style.userSelect = "";
      document.body.style.cursor = "";
    };

    const handleUp = () => cleanup();
    const handleCancel = () => cleanup();
    const handleBlur = () => cleanup();

    document.body.style.userSelect = "none";
    document.body.style.cursor = "row-resize";

    window.addEventListener("mousemove", handleMove);
    window.addEventListener("mouseup", handleUp);
    window.addEventListener("pointercancel", handleCancel);
    window.addEventListener("blur", handleBlur);
    return () => {
      window.removeEventListener("mousemove", handleMove);
      window.removeEventListener("mouseup", handleUp);
      window.removeEventListener("pointercancel", handleCancel);
      window.removeEventListener("blur", handleBlur);
      document.body.style.userSelect = "";
      document.body.style.cursor = "";
    };
  }, [isDragging, setBottomPanelHeight]);

  const handleAccept = async (item: ReviewItem) => {
    if (!currentKB?.path) return;
    await acceptItem(item.id, currentKB.id, currentKB.path);
  };

  const handleReject = async (item: ReviewItem) => {
    await rejectItem(item.id);
  };

  const handleRefresh = () => {
    if (currentKB) {
      loadPendingReviews(currentKB.id);
    }
  };

  return (
    <>
      {/* Badge overlay when panel is closed and there are pending reviews */}
      {!bottomPanelVisible && reviewBadgeCount > 0 && (
        <div className="absolute bottom-8 right-4 z-20">
          <button
            type="button"
            onClick={toggleBottomPanel}
            className="flex items-center gap-2 px-3 py-1.5 bg-amber-500 text-white rounded-full shadow-lg hover:bg-amber-600 transition-colors text-xs font-medium"
          >
            <AlertTriangle size={12} />
            {reviewBadgeCount}条AI知识提案待确认
          </button>
        </div>
      )}

      {/* Panel */}
      <div
        className={cn(
          "border-t border-slate-200 dark:border-slate-800 bg-slate-50 dark:bg-slate-900 overflow-hidden transition-all duration-300 flex flex-col",
          bottomPanelVisible ? "flex-shrink-0" : "h-0 border-t-0"
        )}
        style={{
          height: bottomPanelVisible ? `${bottomPanelHeight}px` : undefined,
          maxHeight: bottomPanelVisible ? `${bottomPanelHeight}px` : undefined,
        }}
      >
        {/* Drag handle bar */}
        <div
          className="flex items-center justify-between px-4 py-1 border-b border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-800 cursor-ns-resize shrink-0 select-none"
          onMouseDown={handleDragStart}
        >
          <div className="flex items-center gap-2">
            <span className="text-xs font-medium text-slate-600 dark:text-slate-400">
              AI 提案审阅
            </span>
            {loading ? (
              <Loader2 size={12} className="text-slate-400 animate-spin" />
            ) : (
              <span className="text-[10px] text-slate-400">
                {pendingCount} 项待处理
              </span>
            )}
          </div>

          <div className="flex items-center gap-1">
            <button
              type="button"
              onClick={handleRefresh}
              className="p-0.5 hover:bg-slate-100 dark:hover:bg-slate-700 rounded text-slate-400"
              title="刷新"
            >
              <Loader2 size={12} className={loading ? "animate-spin" : ""} />
            </button>
            <button
              type="button"
              onClick={toggleBottomPanel}
              className="p-0.5 hover:bg-slate-100 dark:hover:bg-slate-700 rounded text-slate-400"
              title="关闭面板"
            >
              <ChevronDown size={14} />
            </button>
          </div>
        </div>

        {/* Content */}
        {bottomPanelVisible && (
          <div className="flex-1 overflow-y-auto p-4 space-y-3">
            {loading && pendingItems.length === 0 ? (
              <div className="flex items-center justify-center py-12">
                <Loader2 size={24} className="text-slate-400 animate-spin" />
              </div>
            ) : pendingItems.length === 0 ? (
              <div className="flex flex-col items-center justify-center py-12 text-center">
                <CheckCircle2 size={36} className="text-green-400 mb-3" />
                <p className="text-sm text-slate-500 dark:text-slate-400">
                  暂无待审阅的AI提案
                </p>
                <p className="text-xs text-slate-400 mt-1">
                  所有提案已处理完毕
                </p>
              </div>
            ) : (
              pendingItems.map((item) => (
                <ReviewItemCard
                  key={item.id}
                  item={item}
                  onAccept={() => handleAccept(item)}
                  onReject={() => handleReject(item)}
                />
              ))
            )}
          </div>
        )}
      </div>
    </>
  );
}
