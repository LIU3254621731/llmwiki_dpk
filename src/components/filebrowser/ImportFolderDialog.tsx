import { useState, useEffect, useRef, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { UnlistenFn } from "@tauri-apps/api/event";
import { formatSize } from "@/lib/utils";
import type { ImportFolderScanResult, ImportProgressEvent, ImportFolderResult, ImportCandidate } from "@/types/source";
import { FolderOpen, Loader2, CheckCircle2, XCircle, ChevronDown, ChevronRight, FileText } from "lucide-react";

interface ImportFolderDialogProps {
  open: boolean;
  scanResult: ImportFolderScanResult;
  kbId: string;
  kbPath: string;
  onClose: () => void;
  onComplete: () => void;
}

type ImportState = "preview" | "importing" | "completed" | "error_state";

const FILE_TYPE_COLORS: Record<string, string> = {
  pdf: "bg-red-50 text-red-600 border-red-200",
  docx: "bg-blue-50 text-blue-600 border-blue-200",
  doc: "bg-blue-50 text-blue-600 border-blue-200",
  md: "bg-emerald-50 text-emerald-600 border-emerald-200",
  txt: "bg-slate-50 text-slate-600 border-slate-200",
  html: "bg-orange-50 text-orange-600 border-orange-200",
  htm: "bg-orange-50 text-orange-600 border-orange-200",
  png: "bg-violet-50 text-violet-600 border-violet-200",
  jpg: "bg-violet-50 text-violet-600 border-violet-200",
  jpeg: "bg-violet-50 text-violet-600 border-violet-200",
  gif: "bg-violet-50 text-violet-600 border-violet-200",
  webp: "bg-violet-50 text-violet-600 border-violet-200",
};

function getFileTypeStyle(fileType: string): string {
  const ext = fileType.toLowerCase();
  return FILE_TYPE_COLORS[ext] ?? "bg-slate-50 text-slate-500 border-slate-200";
}

export default function ImportFolderDialog({
  open,
  scanResult,
  kbId,
  kbPath,
  onClose,
  onComplete,
}: ImportFolderDialogProps) {
  const [state, setState] = useState<ImportState>("preview");
  const [selectedFiles, setSelectedFiles] = useState<Set<number>>(new Set());
  const [preserveStructure, setPreserveStructure] = useState(true);
  const [showSkipped, setShowSkipped] = useState(false);

  // Importing state
  const [progress, setProgress] = useState<ImportProgressEvent | null>(null);
  const [importResult, setImportResult] = useState<ImportFolderResult | null>(null);
  const [errorMsg, setErrorMsg] = useState("");

  const unlistenRef = useRef<UnlistenFn | null>(null);
  const cancelledRef = useRef(false);

  // Initialize all files as selected
  useEffect(() => {
    if (open && scanResult) {
      const allIndices = new Set<number>();
      scanResult.files.forEach((_, i) => {
        if (scanResult.files[i].is_supported) {
          allIndices.add(i);
        }
      });
      setSelectedFiles(allIndices);
      setState("preview");
      setProgress(null);
      setImportResult(null);
      setErrorMsg("");
      setShowSkipped(false);
      cancelledRef.current = false;
    }
  }, [open, scanResult]);

  // Cleanup listener on unmount
  useEffect(() => {
    return () => {
      if (unlistenRef.current) {
        unlistenRef.current();
        unlistenRef.current = null;
      }
    };
  }, []);

  const toggleFile = (index: number) => {
    setSelectedFiles((prev) => {
      const next = new Set(prev);
      if (next.has(index)) {
        next.delete(index);
      } else {
        next.add(index);
      }
      return next;
    });
  };

  const toggleAll = () => {
    const supportedCount = scanResult.files.filter((f) => f.is_supported).length;
    if (selectedFiles.size === supportedCount) {
      setSelectedFiles(new Set());
    } else {
      const all = new Set<number>();
      scanResult.files.forEach((f, i) => {
        if (f.is_supported) all.add(i);
      });
      setSelectedFiles(all);
    }
  };

  const handleStartImport = async () => {
    if (selectedFiles.size === 0) return;

    setState("importing");
    setErrorMsg("");
    setProgress({ current: 0, total: selectedFiles.size, file_name: "", relative_path: "", status: "importing", success_count: 0, fail_count: 0 });

    // Build list of relative_paths for selected files
    const selectedPaths: string[] = [];
    const selectedIndices = Array.from(selectedFiles).sort((a, b) => a - b);
    for (const idx of selectedIndices) {
      if (idx < scanResult.files.length) {
        selectedPaths.push(scanResult.files[idx].relative_path);
      }
    }

    try {
      // Listen for progress events
      const unlisten = await listen<ImportProgressEvent>("folder-import-progress", (event) => {
        if (cancelledRef.current) return;
        setProgress(event.payload);
      });
      unlistenRef.current = unlisten;

      const result = await invoke<ImportFolderResult>("import_folder", {
        kbId,
        kbPath,
        folderPath: scanResult.folder_path,
        preserveStructure,
        selectedFiles: selectedPaths,
      });

      // Cleanup listener
      unlisten();
      unlistenRef.current = null;

      if (cancelledRef.current) return;

      setImportResult(result);
      setState(result.failed > 0 ? "error_state" : "completed");
    } catch (e) {
      if (cancelledRef.current) return;
      // Cleanup listener on error
      if (unlistenRef.current) {
        unlistenRef.current();
        unlistenRef.current = null;
      }
      setErrorMsg(String(e));
      setState("error_state");
    }
  };

  const handleCancelImport = () => {
    cancelledRef.current = true;
    if (unlistenRef.current) {
      unlistenRef.current();
      unlistenRef.current = null;
    }
    onClose();
  };

  const handleClose = () => {
    if (unlistenRef.current) {
      unlistenRef.current();
      unlistenRef.current = null;
    }
    onClose();
  };

  const handleDone = () => {
    if (unlistenRef.current) {
      unlistenRef.current();
      unlistenRef.current = null;
    }
    onComplete();
  };

  const selectedCount = selectedFiles.size;
  const supportedFiles = scanResult.files.filter((f) => f.is_supported);

  if (!open) return null;

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/30"
      onClick={(e) => {
        // Only allow close by clicking backdrop if not importing
        if (state !== "importing") {
          handleClose();
        }
      }}
    >
      <div
        className="bg-white dark:bg-slate-800 rounded-lg shadow-xl border border-slate-200 dark:border-slate-700 w-[640px] max-h-[80vh] flex flex-col overflow-hidden"
        onClick={(e) => e.stopPropagation()}
      >
        {/* Header */}
        <div className="flex items-center justify-between px-6 py-4 border-b border-slate-200 dark:border-slate-700">
          <h2 className="text-base font-semibold text-slate-900 dark:text-slate-100">
            {state === "preview" && "导入文件夹预览"}
            {state === "importing" && "正在导入..."}
            {(state === "completed" || state === "error_state") && "导入完成"}
          </h2>
          {state === "preview" && (
            <button
              type="button"
              onClick={handleClose}
              className="text-slate-400 hover:text-slate-600 dark:hover:text-slate-300"
            >
              <XCircle size={18} />
            </button>
          )}
        </div>

        {/* Body */}
        <div className="flex-1 overflow-y-auto">
          {/* ---------- Preview State ---------- */}
          {state === "preview" && (
            <div className="p-6 space-y-4">
              {/* Folder info */}
              <div>
                <div className="flex items-center gap-2 text-sm text-slate-700 dark:text-slate-300">
                  <FolderOpen size={16} className="text-slate-400 shrink-0" />
                  <span className="font-medium">{scanResult.folder_name}</span>
                </div>
                <p className="text-xs text-slate-400 dark:text-slate-500 mt-1 pl-6">{scanResult.folder_path}</p>
              </div>

              {/* Stats cards */}
              <div className="grid grid-cols-4 gap-3">
                <div className="bg-slate-50 dark:bg-slate-900 border border-slate-200 dark:border-slate-700 rounded p-3 text-center">
                  <div className="text-lg font-semibold text-slate-900 dark:text-slate-100">{scanResult.total_files}</div>
                  <div className="text-[10px] text-slate-400 dark:text-slate-500 mt-0.5">文件总数</div>
                </div>
                <div className="bg-emerald-50 dark:bg-emerald-900/20 border border-emerald-200 dark:border-emerald-800 rounded p-3 text-center">
                  <div className="text-lg font-semibold text-emerald-700 dark:text-emerald-400">{scanResult.supported_files}</div>
                  <div className="text-[10px] text-slate-400 dark:text-slate-500 mt-0.5">支持文件数</div>
                </div>
                <div className="bg-amber-50 dark:bg-amber-900/20 border border-amber-200 dark:border-amber-800 rounded p-3 text-center">
                  <div className="text-lg font-semibold text-amber-700 dark:text-amber-400">{scanResult.skipped_files}</div>
                  <div className="text-[10px] text-slate-400 dark:text-slate-500 mt-0.5">跳过文件数</div>
                </div>
                <div className="bg-slate-50 dark:bg-slate-900 border border-slate-200 dark:border-slate-700 rounded p-3 text-center">
                  <div className="text-lg font-semibold text-slate-900 dark:text-slate-100">{formatSize(scanResult.total_size)}</div>
                  <div className="text-[10px] text-slate-400 dark:text-slate-500 mt-0.5">总大小</div>
                </div>
              </div>

              {/* Select all */}
              <div className="flex items-center gap-3">
                <label className="flex items-center gap-2 text-xs text-slate-600 dark:text-slate-400 cursor-pointer select-none">
                  <input
                    type="checkbox"
                    checked={selectedFiles.size === supportedFiles.length && supportedFiles.length > 0}
                    onChange={toggleAll}
                    className="w-3.5 h-3.5 rounded border-slate-300 dark:border-slate-600 text-indigo-600 focus:ring-indigo-500 dark:bg-slate-800"
                  />
                  全选 / 取消
                </label>
                <span className="text-[10px] text-slate-400 dark:text-slate-500">
                  已选 {selectedCount} / {supportedFiles.length} 个文件
                </span>
              </div>

              {/* File list */}
              <div className="border border-slate-200 dark:border-slate-700 rounded max-h-[280px] overflow-y-auto">
                <table className="w-full text-xs">
                  <thead className="sticky top-0 bg-slate-50 dark:bg-slate-900">
                    <tr className="border-b border-slate-200 dark:border-slate-700">
                      <th className="w-8 py-2 pl-3 text-left">
                        <span className="sr-only">选择</span>
                      </th>
                      <th className="py-2 text-left font-medium text-slate-400 dark:text-slate-500 uppercase tracking-wide">文件名</th>
                      <th className="py-2 text-left font-medium text-slate-400 dark:text-slate-500 uppercase tracking-wide">类型</th>
                      <th className="py-2 text-left font-medium text-slate-400 dark:text-slate-500 uppercase tracking-wide">大小</th>
                      <th className="py-2 pr-3 text-left font-medium text-slate-400 dark:text-slate-500 uppercase tracking-wide">路径</th>
                    </tr>
                  </thead>
                  <tbody className="divide-y divide-slate-100 dark:divide-slate-800">
                    {scanResult.files.map((file, i) => {
                      const isSelected = selectedFiles.has(i);
                      return (
                        <tr
                          key={i}
                          className={`hover:bg-slate-50 dark:hover:bg-slate-800/50 cursor-pointer ${
                            isSelected ? "bg-slate-50 dark:bg-slate-800/30" : ""
                          } ${!file.is_supported ? "opacity-50" : ""}`}
                          onClick={() => file.is_supported && toggleFile(i)}
                        >
                          <td className="py-2 pl-3">
                            <input
                              type="checkbox"
                              checked={isSelected}
                              disabled={!file.is_supported}
                              onChange={() => file.is_supported && toggleFile(i)}
                              className="w-3.5 h-3.5 rounded border-slate-300 dark:border-slate-600 text-indigo-600 focus:ring-indigo-500 dark:bg-slate-800"
                            />
                          </td>
                          <td className="py-2 text-slate-700 dark:text-slate-300 truncate max-w-[140px]">
                            <div className="flex items-center gap-1.5">
                              <FileText size={12} className="text-slate-400 shrink-0" />
                              <span className="truncate">{file.file_name}</span>
                            </div>
                          </td>
                          <td className="py-2">
                            <span className={`inline-block px-1.5 py-0.5 text-[10px] border rounded ${getFileTypeStyle(file.file_type)}`}>
                              {file.file_type}
                            </span>
                          </td>
                          <td className="py-2 text-slate-400 dark:text-slate-500 whitespace-nowrap">{formatSize(file.file_size)}</td>
                          <td className="py-2 pr-3 text-slate-400 dark:text-slate-500 truncate max-w-[160px] font-mono text-[10px]">{file.relative_path}</td>
                        </tr>
                      );
                    })}
                  </tbody>
                </table>
                {scanResult.files.length === 0 && (
                  <div className="py-8 text-center text-xs text-slate-400 dark:text-slate-500">
                    文件夹为空，没有可导入的文件
                  </div>
                )}
              </div>

              {/* Preserve structure checkbox */}
              <label className="flex items-center gap-2 text-xs text-slate-600 dark:text-slate-400 cursor-pointer select-none">
                <input
                  type="checkbox"
                  checked={preserveStructure}
                  onChange={(e) => setPreserveStructure(e.target.checked)}
                  className="w-3.5 h-3.5 rounded border-slate-300 dark:border-slate-600 text-indigo-600 focus:ring-indigo-500 dark:bg-slate-800"
                />
                保留子目录结构
              </label>

              {/* Skipped files section */}
              {scanResult.skipped_items.length > 0 && (
                <div>
                  <button
                    type="button"
                    onClick={() => setShowSkipped(!showSkipped)}
                    className="flex items-center gap-1 text-xs text-amber-600 dark:text-amber-400 hover:text-amber-700 dark:hover:text-amber-300"
                  >
                    {showSkipped ? <ChevronDown size={14} /> : <ChevronRight size={14} />}
                    跳过的文件 ({scanResult.skipped_items.length})
                  </button>
                  {showSkipped && (
                    <div className="mt-2 border border-amber-200 dark:border-amber-800 rounded bg-amber-50 dark:bg-amber-900/10 overflow-hidden">
                      <table className="w-full text-xs">
                        <thead className="bg-amber-100/50 dark:bg-amber-900/20">
                          <tr>
                            <th className="py-1.5 pl-3 text-left font-medium text-amber-700 dark:text-amber-400">文件名</th>
                            <th className="py-1.5 text-left font-medium text-amber-700 dark:text-amber-400">路径</th>
                            <th className="py-1.5 pr-3 text-left font-medium text-amber-700 dark:text-amber-400">跳过原因</th>
                          </tr>
                        </thead>
                        <tbody className="divide-y divide-amber-100 dark:divide-amber-900/30">
                          {scanResult.skipped_items.map((item, i) => (
                            <tr key={i}>
                              <td className="py-1.5 pl-3 text-slate-600 dark:text-slate-400">{item.file_name}</td>
                              <td className="py-1.5 text-slate-400 dark:text-slate-500 font-mono text-[10px]">{item.relative_path}</td>
                              <td className="py-1.5 pr-3 text-amber-600 dark:text-amber-400">{item.reason}</td>
                            </tr>
                          ))}
                        </tbody>
                      </table>
                    </div>
                  )}
                </div>
              )}
            </div>
          )}

          {/* ---------- Importing State ---------- */}
          {state === "importing" && (
            <div className="p-6 space-y-5">
              {/* Progress bar */}
              <div>
                <div className="flex items-center justify-between text-xs text-slate-500 dark:text-slate-400 mb-2">
                  <span>
                    {progress ? `${progress.current} / ${progress.total}` : "准备中..."}
                  </span>
                  <span>
                    {progress && progress.total > 0
                      ? `${Math.round((progress.current / progress.total) * 100)}%`
                      : "0%"}
                  </span>
                </div>
                <div className="w-full h-2 bg-slate-200 dark:bg-slate-700 rounded-full overflow-hidden">
                  <div
                    className="h-full bg-indigo-600 dark:bg-indigo-500 rounded-full transition-all duration-300 ease-out"
                    style={{
                      width: progress && progress.total > 0
                        ? `${Math.round((progress.current / progress.total) * 100)}%`
                        : "0%",
                    }}
                  />
                </div>
              </div>

              {/* Current file */}
              {progress?.file_name && (
                <div className="flex items-center gap-2.5 px-3 py-2 bg-slate-50 dark:bg-slate-900 border border-slate-200 dark:border-slate-700 rounded text-xs">
                  <Loader2 size={13} className="animate-spin text-indigo-500 shrink-0" />
                  <div className="min-w-0 flex-1">
                    <div className="text-slate-700 dark:text-slate-300 truncate">{progress.file_name}</div>
                    {progress.relative_path && (
                      <div className="text-[10px] text-slate-400 dark:text-slate-500 truncate font-mono">{progress.relative_path}</div>
                    )}
                  </div>
                </div>
              )}

              {/* Success/fail counters */}
              {progress && (
                <div className="flex items-center gap-4 text-xs">
                  <span className="flex items-center gap-1 text-emerald-600 dark:text-emerald-400">
                    <CheckCircle2 size={13} />
                    成功 {progress.success_count}
                  </span>
                  <span className="flex items-center gap-1 text-red-500 dark:text-red-400">
                    <XCircle size={13} />
                    失败 {progress.fail_count}
                  </span>
                </div>
              )}

              <p className="text-[10px] text-slate-400 dark:text-slate-500">
                关闭此对话框不会中断正在进行的导入任务
              </p>
            </div>
          )}

          {/* ---------- Completed / Error State ---------- */}
          {(state === "completed" || state === "error_state") && (
            <div className="p-6 space-y-4">
              {/* Summary */}
              {importResult && (
                <>
                  <div className={`flex items-center gap-3 px-4 py-3 rounded border ${
                    importResult.failed > 0
                      ? "border-amber-200 dark:border-amber-800 bg-amber-50 dark:bg-amber-900/10"
                      : "border-emerald-200 dark:border-emerald-800 bg-emerald-50 dark:bg-emerald-900/10"
                  }`}>
                    {importResult.failed > 0 ? (
                      <XCircle size={18} className="text-amber-600 shrink-0" />
                    ) : (
                      <CheckCircle2 size={18} className="text-emerald-600 shrink-0" />
                    )}
                    <div className="text-sm text-slate-700 dark:text-slate-300">
                      成功导入 <span className="font-semibold text-emerald-600 dark:text-emerald-400">{importResult.success}</span> 个文件
                      {importResult.failed > 0 && (
                        <span>，失败 <span className="font-semibold text-red-500">{importResult.failed}</span> 个</span>
                      )}
                    </div>
                  </div>

                  {/* Failed files */}
                  {importResult.failed > 0 && (
                    <div>
                      <h4 className="text-xs font-medium text-slate-400 dark:text-slate-500 mb-2">
                        失败文件列表
                      </h4>
                      <div className="border border-red-200 dark:border-red-800 rounded bg-red-50 dark:bg-red-900/10 max-h-[200px] overflow-y-auto">
                        <table className="w-full text-xs">
                          <thead className="bg-red-100/50 dark:bg-red-900/20 sticky top-0">
                            <tr>
                              <th className="py-1.5 pl-3 text-left font-medium text-red-700 dark:text-red-400">文件</th>
                              <th className="py-1.5 text-left font-medium text-red-700 dark:text-red-400">路径</th>
                              <th className="py-1.5 pr-3 text-left font-medium text-red-700 dark:text-red-400">错误信息</th>
                            </tr>
                          </thead>
                          <tbody className="divide-y divide-red-100 dark:divide-red-900/30">
                            {importResult.results
                              .filter((r) => r.status === "failed")
                              .map((r, i) => (
                                <tr key={i}>
                                  <td className="py-1.5 pl-3 text-slate-600 dark:text-slate-400">
                                    {r.file_path.split(/[/\\]/).pop() || r.file_path}
                                  </td>
                                  <td className="py-1.5 text-slate-400 dark:text-slate-500 font-mono text-[10px] truncate max-w-[200px]">
                                    {r.relative_path || r.file_path}
                                  </td>
                                  <td className="py-1.5 pr-3 text-red-600 dark:text-red-400 truncate max-w-[200px]">
                                    {r.error || "未知错误"}
                                  </td>
                                </tr>
                              ))}
                          </tbody>
                        </table>
                      </div>
                    </div>
                  )}
                </>
              )}

              {/* Manual error state (invoke failed entirely) */}
              {!importResult && errorMsg && (
                <div className="flex items-center gap-3 px-4 py-3 rounded border border-red-200 dark:border-red-800 bg-red-50 dark:bg-red-900/10">
                  <XCircle size={18} className="text-red-500 shrink-0" />
                  <div className="text-sm text-red-700 dark:text-red-400">{errorMsg}</div>
                </div>
              )}
            </div>
          )}
        </div>

        {/* Footer */}
        <div className="flex items-center justify-end gap-2 px-6 py-4 border-t border-slate-200 dark:border-slate-700 bg-slate-50 dark:bg-slate-900/50">
          {state === "preview" && (
            <>
              <button
                type="button"
                onClick={handleClose}
                className="px-4 py-2 text-xs text-slate-500 dark:text-slate-400 hover:text-slate-700 dark:hover:text-slate-300 hover:bg-slate-100 dark:hover:bg-slate-800 rounded"
              >
                取消
              </button>
              <button
                type="button"
                onClick={handleStartImport}
                disabled={selectedCount === 0}
                className="inline-flex items-center gap-1.5 px-4 py-2 bg-indigo-600 hover:bg-indigo-700 text-white text-xs font-medium rounded disabled:opacity-50 disabled:cursor-not-allowed"
              >
                <FileText size={13} />
                开始导入 ({selectedCount} 个文件)
              </button>
            </>
          )}

          {state === "importing" && (
            <button
              type="button"
              onClick={handleCancelImport}
              className="px-4 py-2 text-xs text-slate-500 dark:text-slate-400 hover:text-slate-700 dark:hover:text-slate-300 hover:bg-slate-100 dark:hover:bg-slate-800 rounded"
            >
              取消
            </button>
          )}

          {(state === "completed" || state === "error_state") && (
            <button
              type="button"
              onClick={handleDone}
              className="inline-flex items-center gap-1.5 px-4 py-2 bg-indigo-600 hover:bg-indigo-700 text-white text-xs font-medium rounded"
            >
              完成
            </button>
          )}
        </div>
      </div>
    </div>
  );
}
