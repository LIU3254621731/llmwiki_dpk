import { useEffect, useState, useRef, useCallback } from "react";
import { useKBStore } from "@/stores/useKBStore";
import { useReviewStore } from "@/stores/useReviewStore";
import { useAppStore } from "@/stores/useAppStore";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import {
  FileUp, FolderInput, Loader2, X, Check, RefreshCw,
  AlertTriangle, CheckCircle2, GitPullRequestDraft,
  Plus, Pencil, Link, ArrowRight, ChevronDown, FileText,
} from "lucide-react";
import { cn } from "@/lib/utils";
import type { ReviewItem } from "@/types/review";

const OP_LABELS: Record<string, string> = { create: "新建", update: "更新", append: "追加", add_alias: "添加别名", add_relation: "添加关系" };
const OP_COLORS: Record<string, string> = { create: "bg-green-50 text-green-600 dark:bg-green-900/20 dark:text-green-400", update: "bg-blue-50 text-blue-600 dark:bg-blue-900/20 dark:text-blue-400", append: "bg-purple-50 text-purple-600 dark:bg-purple-900/20 dark:text-purple-400", add_alias: "bg-amber-50 text-amber-600 dark:bg-amber-900/20 dark:text-amber-400", add_relation: "bg-pink-50 text-pink-600 dark:bg-pink-900/20 dark:text-pink-400" };

function computeDiff(oldText: string, newText: string) {
  const oldLines = (oldText || "").split("\n");
  const newLines = (newText || "").split("\n");
  const result: Array<{ type: "same" | "added" | "removed"; text: string }> = [];
  const maxLen = Math.max(oldLines.length, newLines.length);
  for (let i = 0; i < maxLen; i++) {
    if (oldLines[i] === undefined) { result.push({ type: "added", text: newLines[i] }); }
    else if (newLines[i] === undefined) { result.push({ type: "removed", text: oldLines[i] }); }
    else if (oldLines[i] === newLines[i]) { result.push({ type: "same", text: oldLines[i] }); }
    else { result.push({ type: "removed", text: oldLines[i] }); result.push({ type: "added", text: newLines[i] }); }
  }
  return result;
}

export default function ImportReviewTab() {
  const currentKB = useKBStore((s) => s.currentKB);
  const pendingItems = useReviewStore((s) => s.pendingItems);
  const pendingCount = useReviewStore((s) => s.pendingCount);
  const reviewLoading = useReviewStore((s) => s.loading);
  const loadPendingReviews = useReviewStore((s) => s.loadPendingReviews);
  const acceptItem = useReviewStore((s) => s.acceptItem);
  const rejectItem = useReviewStore((s) => s.rejectItem);

  const [uploadMsg, setUploadMsg] = useState("");
  const [uploadError, setUploadError] = useState("");
  const [uploadedTaskIds, setUploadedTaskIds] = useState<string[]>([]);
  const [uploading, setUploading] = useState(false);
  const [supportedTypes, setSupportedTypes] = useState<{extension: string; mime_type: string; description: string}[]>([]);

  useEffect(() => {
    if (currentKB) {
      loadPendingReviews(currentKB.id);
      loadSupportedTypes();
    }
  }, [currentKB?.id]);

  useEffect(() => {
    let unlistenFn: (() => void) | undefined;
    (async () => {
      try {
        const { listen: l } = await import("@tauri-apps/api/event");
        const u = await l("review-updated", () => { if (currentKB) loadPendingReviews(currentKB.id); });
        unlistenFn = u;
      } catch { /* */ }
    })();
    return () => { unlistenFn?.(); };
  }, [currentKB]);

  const loadSupportedTypes = async () => {
    try {
      const types = await invoke<{extension: string; mime_type: string; description: string}[]>("get_supported_file_types");
      setSupportedTypes(types);
    } catch { /* */ }
  };

  const handleUpload = async () => {
    if (!currentKB) return;
    try {
      const selected = await open({ multiple: true });
      if (!selected) return;
      setUploading(true); setUploadMsg(""); setUploadError("");
      const files = Array.isArray(selected) ? selected : [selected];
      const taskIds: string[] = [];
      for (const path of files) {
        const result = await invoke<{task_id: string; file_name: string; type: string}>("upload_source_file", { kbId: currentKB.id, filePath: path as string });
        if (result?.task_id) {
          taskIds.push(result.task_id);
        }
      }
      setUploadedTaskIds(taskIds);
      if (taskIds.length > 0) {
        useAppStore.getState().setTaskDetailId(taskIds[0]);
      }
      setUploadMsg(`成功上传 ${files.length} 个文件`);
      loadPendingReviews(currentKB.id);
    } catch (e) { setUploadError(`上传失败: ${e}`); }
    setUploading(false);
  };

  const handleImportFolder = async () => {
    if (!currentKB) return;
    try {
      const selected = await open({ directory: true, multiple: false });
      if (!selected) return;
      setUploading(true); setUploadMsg(""); setUploadError("");
      const scanResult = await invoke<any>("scan_import_folder", { folderPath: selected as string });
      await invoke("import_folder", { kbId: currentKB.id, folderPath: selected as string, preserveStructure: true });
      setUploadMsg(`导入完成: ${scanResult.supported_files} 个文件`);
      loadPendingReviews(currentKB.id);
    } catch (e) { setUploadError(`文件夹导入失败: ${e}`); }
    setUploading(false);
  };

  const handleAcceptAllLowRisk = async () => {
    if (!currentKB || pendingItems.length === 0) return;
    try {
      await invoke("accept_all_low_risk_review", { kbId: currentKB.id });
      setUploadMsg("已接受全部低风险提案");
      loadPendingReviews(currentKB.id);
    } catch (e) { setUploadError(`操作失败: ${e}`); }
  };

  const handleRejectAll = async () => {
    if (!currentKB || pendingItems.length === 0 || !confirm("确定拒绝所有待审提案？")) return;
    try {
      await invoke("reject_all_review", { kbId: currentKB.id });
      setUploadMsg("已拒绝全部提案");
      loadPendingReviews(currentKB.id);
    } catch (e) { setUploadError(`操作失败: ${e}`); }
  };

  const handleAccept = async (item: ReviewItem) => {
    if (!currentKB) return;
    await acceptItem(item.id, currentKB.id, currentKB.path);
  };

  const handleReject = async (item: ReviewItem) => {
    await rejectItem(item.id);
  };

  return (
    <div className="flex-1 flex overflow-hidden">
      {/* Left: Import section */}
      <div className="w-[340px] shrink-0 border-r border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 flex flex-col overflow-hidden">
        <div className="px-4 py-3 border-b border-slate-100 dark:border-slate-800">
          <h3 className="text-xs font-medium text-slate-500 dark:text-slate-400 uppercase tracking-wide mb-2">文件导入</h3>
          <div className="space-y-2">
            <button type="button" onClick={handleUpload} disabled={uploading} className="w-full flex items-center gap-2 px-3 py-2 text-xs border border-slate-200 dark:border-slate-700 text-slate-600 dark:text-slate-400 rounded hover:bg-slate-50 dark:hover:bg-slate-800 disabled:opacity-50">
              {uploading ? <Loader2 size={13} className="animate-spin" /> : <FileUp size={13} />} 上传文件
            </button>
            <button type="button" onClick={handleImportFolder} disabled={uploading} className="w-full flex items-center gap-2 px-3 py-2 text-xs border border-slate-200 dark:border-slate-700 text-slate-600 dark:text-slate-400 rounded hover:bg-slate-50 dark:hover:bg-slate-800 disabled:opacity-50">
              {uploading ? <Loader2 size={13} className="animate-spin" /> : <FolderInput size={13} />} 导入文件夹
            </button>
          </div>
          {uploadMsg && <div className="mt-2 px-3 py-2 text-xs text-green-600 bg-green-50 border border-green-100 rounded">{uploadMsg}</div>}
          {uploadError && <div className="mt-2 px-3 py-2 text-xs text-red-600 bg-red-50 border border-red-100 rounded">{uploadError}</div>}
          {uploadedTaskIds.length > 0 && (
            <div className="mt-2 px-3 py-2 text-xs text-brand-500 bg-brand-50 border border-brand-100 rounded dark:bg-brand-900/20 dark:text-brand-400 dark:border-brand-800 space-y-1">
              {uploadedTaskIds.map((tid, i) => (
                <button
                  key={tid}
                  type="button"
                  onClick={() => useAppStore.getState().setTaskDetailId(tid)}
                  className="block hover:underline cursor-pointer w-full text-left"
                >
                  {"\u4efb\u52a1"} #{tid.slice(0, 8)}... {"\u5df2\u521b\u5efa"}
                  {uploadedTaskIds.length > 1 && ` (${i + 1}/${uploadedTaskIds.length})`}
                </button>
              ))}
            </div>
          )}
          {supportedTypes.length > 0 && (
            <p className="mt-2 text-[10px] text-slate-400">支持: {supportedTypes.map(t => t.extension).join(", ")}</p>
          )}
        </div>
      </div>

      {/* Right: Review section */}
      <div className="flex-1 flex flex-col overflow-hidden bg-slate-50 dark:bg-slate-950">
        <div className="flex items-center justify-between px-4 py-2.5 border-b border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 shrink-0">
          <div className="flex items-center gap-2">
            <GitPullRequestDraft size={15} className="text-slate-500" />
            <span className="text-sm font-medium text-slate-700 dark:text-slate-300">AI 提案审阅</span>
            {pendingCount > 0 && <span className="text-xs text-amber-600 font-medium">{pendingCount} 项待处理</span>}
          </div>
          <div className="flex items-center gap-1">
            {pendingCount > 0 && (
              <>
                <button onClick={handleAcceptAllLowRisk} className="px-2 py-1 text-xs bg-green-500 text-white rounded hover:bg-green-600">接受低风险</button>
                <button onClick={handleRejectAll} className="px-2 py-1 text-xs bg-red-500 text-white rounded hover:bg-red-600">拒绝全部</button>
              </>
            )}
            <button onClick={() => currentKB && loadPendingReviews(currentKB.id)} className="p-1.5 text-slate-400 hover:text-slate-600 rounded" title="刷新"><RefreshCw size={13} className={reviewLoading ? "animate-spin" : ""} /></button>
          </div>
        </div>

        <div className="flex-1 overflow-y-auto p-4 space-y-3">
          {reviewLoading && pendingItems.length === 0 ? (
            <div className="flex items-center justify-center py-20"><Loader2 size={24} className="animate-spin text-slate-400" /></div>
          ) : pendingItems.length === 0 ? (
            <div className="flex flex-col items-center justify-center py-20">
              <CheckCircle2 size={48} className="text-green-400 mb-4" />
              <p className="text-sm text-slate-500">暂无待审阅的 AI 提案</p>
              <p className="text-xs text-slate-400 mt-1">上传文件后 AI 会自动分析并生成提案</p>
            </div>
          ) : (
            pendingItems.map((item) => <ReviewCard key={item.id} item={item} onAccept={() => handleAccept(item)} onReject={() => handleReject(item)} />)
          )}
        </div>
      </div>
    </div>
  );
}

function ReviewCard({ item, onAccept, onReject }: { item: ReviewItem; onAccept: () => void; onReject: () => void }) {
  const [showDiff, setShowDiff] = useState(false);
  const [processing, setProcessing] = useState(false);
  const opType = item.operation_type || "update";
  const diffLines = showDiff ? computeDiff(item.old_content, item.new_content) : [];

  const handleAccept = async () => { setProcessing(true); try { await onAccept(); } finally { setProcessing(false); } };
  const handleReject = async () => { setProcessing(true); try { await onReject(); } finally { setProcessing(false); } };

  return (
    <div className="bg-white dark:bg-slate-800 border border-slate-200 dark:border-slate-700 rounded-lg overflow-hidden">
      <div className="flex items-center gap-2 px-3 py-2 border-b border-slate-100 dark:border-slate-700">
        <span className={cn("inline-flex items-center gap-1 px-1.5 py-0.5 rounded text-[10px] font-medium", OP_COLORS[opType] || "bg-slate-50")}>{opType === "create" ? <Plus size={10} /> : opType === "update" ? <Pencil size={10} /> : opType === "add_alias" ? <Link size={10} /> : <ArrowRight size={10} />}{OP_LABELS[opType] || opType}</span>
        <span className="text-xs font-mono text-slate-500 truncate flex-1">{item.target_path}</span>
        <button onClick={() => setShowDiff(!showDiff)} className="text-xs text-brand-500 hover:text-brand-600 shrink-0">{showDiff ? "收起" : "查看差异"}</button>
      </div>
      {(item.summary || item.reason) && <div className="px-3 py-1.5 text-xs text-slate-600 dark:text-slate-400 bg-slate-50 dark:bg-slate-800/50">{item.summary || item.reason}</div>}
      {showDiff && (
        <div className="border-t border-slate-100 dark:border-slate-700 max-h-64 overflow-y-auto font-mono text-xs leading-5">
          {diffLines.map((line, i) => (
            <div key={i} className={cn("px-3 py-0 whitespace-pre-wrap", line.type === "added" && "bg-green-50 dark:bg-green-900/20 text-green-800 dark:text-green-300", line.type === "removed" && "bg-red-50 dark:bg-red-900/20 text-red-800 dark:text-red-300", line.type === "same" && "text-slate-500 dark:text-slate-400")}>
              <span className="w-4 inline-block text-slate-300 select-none">{line.type === "added" ? "+" : line.type === "removed" ? "-" : " "}</span>{line.text}
            </div>
          ))}
        </div>
      )}
      <div className="flex items-center gap-2 px-3 py-2 border-t border-slate-100 dark:border-slate-700">
        <button onClick={handleAccept} disabled={processing} className="flex items-center gap-1 px-3 py-1 text-xs rounded-md bg-green-500 text-white hover:bg-green-600 disabled:opacity-50">{processing ? <Loader2 size={12} className="animate-spin" /> : <Check size={12} />}接受</button>
        <button onClick={handleReject} disabled={processing} className="flex items-center gap-1 px-3 py-1 text-xs rounded-md bg-red-500 text-white hover:bg-red-600 disabled:opacity-50">{processing ? <Loader2 size={12} className="animate-spin" /> : <X size={12} />}拒绝</button>
        {item.confidence && <span className="ml-auto text-[10px] text-slate-400">可信度: {item.confidence}</span>}
      </div>
    </div>
  );
}
