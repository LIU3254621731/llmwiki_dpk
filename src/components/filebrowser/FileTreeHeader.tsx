import { Plus, FolderPlus, RefreshCw, FolderDown, FolderTree, ChevronDown, ChevronsDownUp, ChevronsUpDown, Search, X } from "lucide-react";
import { useState, useEffect, useRef } from "react";
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
  /** Total file count before filtering */
  totalFileCount?: number;
  /** Filtered file count (visible after search) */
  filteredFileCount?: number;
}

const SORT_OPTIONS: { value: SortMode; label: string }[] = [
  { value: "name", label: "Name" },
  { value: "modified", label: "Modified" },
  { value: "type", label: "Type" },
];

/** Debounce a value by a given delay in ms */
function useDebounce<T>(value: T, delay: number): T {
  const [debounced, setDebounced] = useState(value);
  useEffect(() => {
    const timer = setTimeout(() => setDebounced(value), delay);
    return () => clearTimeout(timer);
  }, [value, delay]);
  return debounced;
}

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
  totalFileCount,
  filteredFileCount,
}: FileTreeHeaderProps) {
  const [searchQuery, setSearchQuery] = useState("");
  const debouncedQuery = useDebounce(searchQuery, 150);
  const isFirstRender = useRef(true);

  // Notify parent of debounced search changes (skip initial render)
  useEffect(() => {
    if (isFirstRender.current) {
      isFirstRender.current = false;
      return;
    }
    onSearch?.(debouncedQuery);
  }, [debouncedQuery]);

  const handleClear = () => {
    setSearchQuery("");
  };

  const isSearching = searchQuery.trim().length > 0;
  const hasCounts = totalFileCount !== undefined;

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
              title="Expand all"
              aria-label="Expand all folders"
            >
              <ChevronsDownUp size={14} />
            </button>
          )}
          {onCollapseAll && (
            <button
              type="button"
              onClick={onCollapseAll}
              className="p-1 rounded text-slate-400 dark:text-slate-500 hover:text-slate-600 dark:hover:text-slate-300 hover:bg-slate-100 dark:hover:bg-slate-800 transition-colors"
              title="Collapse all"
              aria-label="Collapse all folders"
            >
              <ChevronsUpDown size={14} />
            </button>
          )}
          {onImportFolder && (
            <button
              type="button"
              onClick={onImportFolder}
              className="p-1 rounded text-slate-400 dark:text-slate-500 hover:text-indigo-500 dark:hover:text-indigo-400 hover:bg-slate-100 dark:hover:bg-slate-800 transition-colors"
              title="Import folder"
              aria-label="Import folder"
            >
              <FolderDown size={14} />
            </button>
          )}
          {onNewFile && (
            <button
              type="button"
              onClick={onNewFile}
              className="p-1 rounded text-slate-400 dark:text-slate-500 hover:text-slate-600 dark:hover:text-slate-300 hover:bg-slate-100 dark:hover:bg-slate-800 transition-colors"
              title="New file"
              aria-label="New file"
            >
              <Plus size={14} />
            </button>
          )}
          {onNewFolder && (
            <button
              type="button"
              onClick={onNewFolder}
              className="p-1 rounded text-slate-400 dark:text-slate-500 hover:text-slate-600 dark:hover:text-slate-300 hover:bg-slate-100 dark:hover:bg-slate-800 transition-colors"
              title="New folder"
              aria-label="New folder"
            >
              <FolderPlus size={14} />
            </button>
          )}
          {onRefresh && (
            <button
              type="button"
              onClick={onRefresh}
              className="p-1 rounded text-slate-400 dark:text-slate-500 hover:text-slate-600 dark:hover:text-slate-300 hover:bg-slate-100 dark:hover:bg-slate-800 transition-colors"
              title="Refresh file tree"
              aria-label="Refresh file tree"
            >
              <RefreshCw size={14} />
            </button>
          )}
        </div>
      </div>

      {/* Search input */}
      {onSearch && (
        <div className="px-3 pb-2">
          <div className="relative">
            <Search size={12} className="absolute left-2 top-1/2 -translate-y-1/2 text-slate-400 dark:text-slate-500" />
            <input
              type="text"
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              placeholder="Search files..."
              className="w-full pl-7 pr-7 py-1 text-xs border border-slate-200 dark:border-slate-700 rounded bg-slate-50 dark:bg-slate-800 text-slate-700 dark:text-slate-300 placeholder:text-slate-400 dark:placeholder:text-slate-500 focus:outline-none focus:border-slate-300 dark:focus:border-slate-600 focus:bg-white dark:focus:bg-slate-800 transition-colors"
              aria-label="Search files"
            />
            {isSearching && (
              <button
                type="button"
                onClick={handleClear}
                className="absolute right-1.5 top-1/2 -translate-y-1/2 p-0.5 rounded text-slate-400 dark:text-slate-500 hover:text-slate-600 dark:hover:text-slate-300 hover:bg-slate-200 dark:hover:bg-slate-700 transition-colors"
                aria-label="Clear search"
              >
                <X size={12} />
              </button>
            )}
          </div>
          {/* Search result count */}
          {hasCounts && debouncedQuery.trim() && (
            <p className="text-[10px] text-slate-400 dark:text-slate-500 mt-1 px-1">
              Showing {filteredFileCount} of {totalFileCount} files
            </p>
          )}
        </div>
      )}
    </div>
  );
}
