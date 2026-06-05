import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { X, Clock, ChevronLeft, FileCode, Loader2, Hash, ArrowRight } from "lucide-react";
import type { PageVersion } from "@/types/wiki";
import { formatDateTime } from "@/lib/utils";
import MarkdownRenderer from "@/components/common/MarkdownRenderer";

interface Props {
  kbId: string;
  pagePath: string;
  onClose: () => void;
}

export default function VersionHistoryPanel({ kbId, pagePath, onClose }: Props) {
  const [versions, setVersions] = useState<PageVersion[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");
  const [selectedVersion, setSelectedVersion] = useState<PageVersion | null>(null);
  const [snapshotContent, setSnapshotContent] = useState("");
  const [snapshotLoading, setSnapshotLoading] = useState(false);

  useEffect(() => {
    loadVersions();
  }, [kbId, pagePath]);

  const loadVersions = async () => {
    setLoading(true);
    setError("");
    try {
      const v = await invoke<PageVersion[]>("list_page_versions", { kbId, pagePath });
      setVersions(v);
    } catch (e) {
      setError(String(e));
    }
    setLoading(false);
  };

  const loadSnapshot = async (version: PageVersion) => {
    setSelectedVersion(version);
    setSnapshotLoading(true);
    try {
      const content = await invoke<string>("get_page_version_snapshot", { kbId, versionId: version.id });
      setSnapshotContent(content);
    } catch (e) {
      setSnapshotContent(`加载快照失败: ${e}`);
    }
    setSnapshotLoading(false);
  };

  const showSnapshot = (version: PageVersion) => {
    if (selectedVersion?.id === version.id) {
      setSelectedVersion(null);
      setSnapshotContent("");
    } else {
      loadSnapshot(version);
    }
  };

  return (
    <div className="border-t border-slate-200 dark:border-slate-700 bg-white dark:bg-slate-900 flex h-[520px]">
      {/* Timeline panel */}
      <div className="w-80 shrink-0 border-r border-slate-200 dark:border-slate-700 flex flex-col">
        <div className="flex items-center justify-between px-4 py-2.5 border-b border-slate-100 dark:border-slate-800">
          <h3 className="text-xs font-medium text-slate-400 dark:text-slate-500 uppercase tracking-wide flex items-center gap-1.5">
            <Clock size={13} />
            版本历史
            {!loading && <span className="text-slate-300 dark:text-slate-600">({versions.length})</span>}
          </h3>
          <button
            type="button"
            onClick={onClose}
            className="p-1 hover:bg-slate-100 dark:hover:bg-slate-800 text-slate-400 hover:text-slate-600 dark:hover:text-slate-300"
            title="关闭版本历史"
          >
            <X size={14} />
          </button>
        </div>

        <div className="flex-1 overflow-y-auto">
          {loading ? (
            <div className="flex items-center justify-center py-10">
              <Loader2 size={16} className="animate-spin text-slate-300 dark:text-slate-600 mr-2" />
              <span className="text-xs text-slate-400 dark:text-slate-500">加载版本历史...</span>
            </div>
          ) : error ? (
            <div className="px-4 py-3 text-xs text-red-500">{error}</div>
          ) : versions.length === 0 ? (
            <div className="px-4 py-8 text-center">
              <Clock size={28} className="mx-auto mb-2 text-slate-300 dark:text-slate-600" />
              <p className="text-xs text-slate-400 dark:text-slate-500">暂无历史版本</p>
              <p className="text-xs text-slate-300 dark:text-slate-600 mt-1">页面修改后会自动保存版本快照</p>
            </div>
          ) : (
            <div className="relative pl-10 pr-4 py-2">
              {/* Timeline line */}
              <div className="absolute left-7 top-0 bottom-0 w-px bg-slate-200 dark:bg-slate-700" />

              {versions.map((v, i) => {
                const isSelected = selectedVersion?.id === v.id;
                const isLatest = i === 0;
                return (
                  <button
                    key={v.id}
                    type="button"
                    onClick={() => showSnapshot(v)}
                    className={`w-full text-left relative mb-1 group ${
                      isSelected
                        ? "bg-slate-100 dark:bg-slate-800 ring-1 ring-slate-200 dark:ring-slate-700"
                        : "hover:bg-slate-50 dark:hover:bg-slate-800/50"
                    }`}
                  >
                    {/* Timeline dot */}
                    <div
                      className={`absolute -left-[13px] top-3 w-2.5 h-2.5 rounded-full border-2 ${
                        isLatest
                          ? "bg-slate-800 dark:bg-slate-200 border-slate-800 dark:border-slate-200"
                          : isSelected
                            ? "bg-blue-500 border-blue-500"
                            : "bg-white dark:bg-slate-800 border-slate-300 dark:border-slate-600 group-hover:border-slate-400 dark:group-hover:border-slate-500"
                      }`}
                    />

                    <div className="px-3 py-2.5">
                      <div className="flex items-center gap-2">
                        <span className="text-xs font-medium text-slate-700 dark:text-slate-300">
                          {isLatest ? "当前版本" : `版本 ${versions.length - i}`}
                        </span>
                        {isLatest && (
                          <span className="text-[10px] px-1.5 py-0.5 bg-slate-800 dark:bg-slate-200 text-white dark:text-slate-800">
                            最新
                          </span>
                        )}
                      </div>
                      <div className="text-xs text-slate-400 dark:text-slate-500 mt-1">
                        {formatDateTime(v.created_at)}
                      </div>
                      <div className="flex items-center gap-2 mt-1.5 text-[10px] text-slate-400 dark:text-slate-500">
                        <span className="inline-flex items-center gap-0.5">
                          <Hash size={9} />
                          {v.content_hash.slice(0, 8)}
                        </span>
                        {v.operation_id && (
                          <span className="bg-slate-100 dark:bg-slate-800 px-1 py-0.5 text-slate-500 dark:text-slate-400">
                            {v.operation_id}
                          </span>
                        )}
                      </div>
                      {v.task_id && (
                        <div className="text-[10px] text-slate-300 dark:text-slate-600 mt-0.5 font-mono truncate" title={v.task_id}>
                          {v.task_id.slice(0, 8)}
                        </div>
                      )}
                    </div>
                  </button>
                );
              })}
            </div>
          )}
        </div>
      </div>

      {/* Snapshot preview panel */}
      <div className="flex-1 flex flex-col min-w-0">
        {!selectedVersion ? (
          <div className="flex-1 flex items-center justify-center">
            <div className="text-center">
              <FileCode size={28} className="mx-auto mb-2 text-slate-300 dark:text-slate-600" />
              <p className="text-xs text-slate-400 dark:text-slate-500">选择一个版本查看快照内容</p>
            </div>
          </div>
        ) : snapshotLoading ? (
          <div className="flex-1 flex items-center justify-center">
            <Loader2 size={16} className="animate-spin text-slate-300 dark:text-slate-600 mr-2" />
            <span className="text-xs text-slate-400 dark:text-slate-500">加载快照...</span>
          </div>
        ) : (
          <>
            <div className="flex items-center gap-2 px-4 py-2 border-b border-slate-100 dark:border-slate-800 bg-slate-50 dark:bg-slate-800/50">
              <button
                type="button"
                onClick={() => { setSelectedVersion(null); setSnapshotContent(""); }}
                className="p-1 hover:bg-slate-200 dark:hover:bg-slate-700 text-slate-400 dark:text-slate-500"
                title="返回版本列表"
              >
                <ChevronLeft size={14} />
              </button>
              <span className="text-xs font-medium text-slate-700 dark:text-slate-300">
                {formatDateTime(selectedVersion.created_at)}
              </span>
              <ArrowRight size={10} className="text-slate-300 dark:text-slate-600" />
              <span className="text-[10px] text-slate-400 dark:text-slate-500 font-mono">
                {selectedVersion.content_hash.slice(0, 8)}
              </span>
              {selectedVersion.operation_id && (
                <span className="text-[10px] px-1.5 py-0.5 bg-slate-100 dark:bg-slate-800 text-slate-500 dark:text-slate-400">
                  {selectedVersion.operation_id}
                </span>
              )}
            </div>
            <div className="flex-1 overflow-y-auto p-4 bg-white dark:bg-slate-900">
              <MarkdownRenderer content={snapshotContent} hideFrontmatter={true} />
            </div>
          </>
        )}
      </div>
    </div>
  );
}
