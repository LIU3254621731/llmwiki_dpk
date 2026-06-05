import { Plus, FolderPlus, RefreshCw, FolderDown, FolderTree, ChevronDown, ChevronsDownUp, ChevronsUpDown } from "lucide-react";
import { useState } from "react";
import type { SortMode } from "@/stores/useFileTreeStore";

interface FileTreeHeaderProps {
  kbName?: string;
  sortBy?: SortMode;
  onSortChange?: (sort: SortMode) => void;
  onSearch?: (query: string) => void;
  onNewFile?: () => void;
  onNewFolder?: () => void;
  onRefresh?: () => void;
  onImportFolder?: () => void;
  onExpandAll?: () => void;
  onCollapseAll?: () => void;
}

const SORT_OPTIONS: { value: SortMode; label: string }[] = [
  { value: "name", label: "名称" },
  { value: "modified", label: "修改时间" },
  { value: "type", label: "文件类型" },
];

export default function FileTreeHeader({
  kbName,
  sortBy,
  onSortChange,
  onSearch,
  onNewFile,
  onNewFolder,
  onRefresh,
  onImportFolder,
  onExpandAll,
  onCollapseAll,
}: FileTreeHeaderProps) {
  const [searchQuery, setSearchQuery] = useState("");

  const handleSearchChange = (value: string) => {
    setSearchQuery(value);
    onSearch?.(value);
  };

  return (
    <div className="border-b border-slate-200 dark:border-slate-700">
      {/* KB name header */}
      {kbName && (
        <div className="flex items-center gap-2 px-3 py-2 border-b border-slate-100 dark:border-slate-800">
          <FolderTree size={16} className="text-slate-400 dark:text-slate-500 shrink-0" />
          <span className="text-[13px] font-medium text-slate-600 dark:text-slate-300 truncate">
            {kbName}
          </span>
        </div>
      )}

      {/* Action buttons row */}
      <div className="flex items-center justify-between px-3 py-1.5">
        <div className="flex items-center gap-0.5">
          {/* Sort dropdown */}
          {sortBy && onSortChange && (
            <div className="relative">
              <select
                value={sortBy}
                onChange={(e) => onSortChange(e.target.value as SortMode)}
                className="appearance-none text-[11px] text-slate-400 dark:text-slate-500 bg-transparent border border-slate-200 dark:border-slate-700 rounded pl-1.5 pr-5 py-0.5 focus:outline-none focus:border-slate-300 dark:focus:border-slate-600 cursor-pointer"
              >
                {SORT_OPTIONS.map((opt) => (
                  <option key={opt.value} value={opt.value}>{opt.label}</option>
                ))}
              </select>
              <ChevronDown size={10} className="absolute right-1 top-1/2 -translate-y-1/2 pointer-events-none text-slate-400 dark:text-slate-500" />
            </div>
          )}
        </div>
        <div className="flex items-center gap-0.5">
          {onExpandAll && (
            <button
              type="button"
              onClick={onExpandAll}
              className="p-1 rounded text-slate-400 dark:text-slate-500 hover:text-slate-600 dark:hover:text-slate-300 hover:bg-slate-100 dark:hover:bg-slate-800 transition-colors"
              title="展开全部"
              aria-label="展开全部文件夹"
            >
              <ChevronsDownUp size={14} />
            </button>
          )}
          {onCollapseAll && (
            <button
              type="button"
              onClick={onCollapseAll}
              className="p-1 rounded text-slate-400 dark:text-slate-500 hover:text-slate-600 dark:hover:text-slate-300 hover:bg-slate-100 dark:hover:bg-slate-800 transition-colors"
              title="折叠全部"
              aria-label="折叠全部文件夹"
            >
              <ChevronsUpDown size={14} />
            </button>
          )}
          {onImportFolder && (
            <button
              type="button"
              onClick={onImportFolder}
              className="p-1 rounded text-slate-400 dark:text-slate-500 hover:text-indigo-500 dark:hover:text-indigo-400 hover:bg-slate-100 dark:hover:bg-slate-800 transition-colors"
              title="导入文件夹"
              aria-label="导入文件夹"
            >
              <FolderDown size={14} />
            </button>
          )}
          {onNewFile && (
            <button
              type="button"
              onClick={onNewFile}
              className="p-1 rounded text-slate-400 dark:text-slate-500 hover:text-slate-600 dark:hover:text-slate-300 hover:bg-slate-100 dark:hover:bg-slate-800 transition-colors"
              title="新建文件"
              aria-label="新建文件"
            >
              <Plus size={14} />
            </button>
          )}
          {onNewFolder && (
            <button
              type="button"
              onClick={onNewFolder}
              className="p-1 rounded text-slate-400 dark:text-slate-500 hover:text-slate-600 dark:hover:text-slate-300 hover:bg-slate-100 dark:hover:bg-slate-800 transition-colors"
              title="新建文件夹"
              aria-label="新建文件夹"
            >
              <FolderPlus size={14} />
            </button>
          )}
          {onRefresh && (
            <button
              type="button"
              onClick={onRefresh}
              className="p-1 rounded text-slate-400 dark:text-slate-500 hover:text-slate-600 dark:hover:text-slate-300 hover:bg-slate-100 dark:hover:bg-slate-800 transition-colors"
              title="刷新文件树"
              aria-label="刷新文件树"
            >
              <RefreshCw size={14} />
            </button>
          )}
        </div>
      </div>

      {/* Search input */}
      {onSearch && (
        <div className="px-3 pb-2">
          <input
            type="text"
            value={searchQuery}
            onChange={(e) => handleSearchChange(e.target.value)}
            placeholder="搜索文件..."
            className="w-full px-2 py-1 text-xs border border-slate-200 dark:border-slate-700 rounded bg-slate-50 dark:bg-slate-800 text-slate-700 dark:text-slate-300 placeholder:text-slate-400 dark:placeholder:text-slate-500 focus:outline-none focus:border-slate-300 dark:focus:border-slate-600 focus:bg-white dark:focus:bg-slate-800 transition-colors"
            aria-label="搜索文件"
          />
        </div>
      )}
    </div>
  );
}
