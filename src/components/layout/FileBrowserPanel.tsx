import { useEffect, useState, useMemo, useCallback } from "react";
import { useKBStore } from "@/stores/useKBStore";
import { useFileTreeStore, type FileTreeNode } from "@/stores/useFileTreeStore";
import {
  Search, X, ChevronRight, RefreshCw, Loader2,
  FolderOpen, FileText, BookOpen, Globe, File,
} from "lucide-react";

function getFileIcon(fileType: string, isDirectory: boolean) {
  if (isDirectory) return <FolderOpen size={14} className="text-amber-500 shrink-0" />;
  const ext = fileType.toLowerCase();
  if (["md", "markdown"].includes(ext)) return <BookOpen size={14} className="text-blue-500 shrink-0" />;
  if (ext === "pdf") return <FileText size={14} className="text-red-500 shrink-0" />;
  if (["docx", "doc"].includes(ext)) return <FileText size={14} className="text-blue-500 shrink-0" />;
  if (["html", "htm"].includes(ext)) return <Globe size={14} className="text-orange-500 shrink-0" />;
  if (ext === "txt") return <FileText size={14} className="text-gray-500 shrink-0" />;
  return <File size={14} className="text-slate-400 shrink-0" />;
}

interface FileBrowserPanelProps {
  onPreviewFile: (node: FileTreeNode) => void;
}

export default function FileBrowserPanel({ onPreviewFile }: FileBrowserPanelProps) {
  const currentKB = useKBStore((s) => s.currentKB);
  const files = useFileTreeStore((s) => s.files);
  const expandedFolders = useFileTreeStore((s) => s.expandedFolders);
  const loading = useFileTreeStore((s) => s.loading);
  const error = useFileTreeStore((s) => s.error);
  const loadFileTree = useFileTreeStore((s) => s.loadFileTree);
  const toggleFolder = useFileTreeStore((s) => s.toggleFolder);
  const refreshTree = useFileTreeStore((s) => s.refreshTree);

  const [searchQuery, setSearchQuery] = useState("");

  useEffect(() => {
    if (currentKB) {
      loadFileTree(currentKB.id, currentKB.path);
    }
  }, [currentKB?.id]);

  const filterTree = useCallback(
    (nodes: FileTreeNode[], query: string): FileTreeNode[] => {
      if (!query.trim()) return nodes;
      const lower = query.toLowerCase();
      return nodes.reduce<FileTreeNode[]>((acc, node) => {
        const nameMatch = node.name.toLowerCase().includes(lower);
        if (node.is_directory && node.children) {
          const filtered = filterTree(node.children, query);
          if (filtered.length > 0 || nameMatch) {
            acc.push({ ...node, children: filtered });
          }
        } else if (nameMatch) {
          acc.push(node);
        }
        return acc;
      }, []);
    },
    []
  );

  const displayedFiles = useMemo(
    () => filterTree(files, searchQuery),
    [files, searchQuery, filterTree]
  );

  const renderNode = (node: FileTreeNode, depth: number = 0) => {
    const isExpanded = expandedFolders.has(node.relative_path);
    const ext = (node.file_type || node.extension || "").toLowerCase();

    return (
      <div key={node.relative_path}>
        <div
          className="flex items-center gap-1 py-1 pr-2 rounded cursor-pointer text-xs text-slate-600 dark:text-slate-400 hover:bg-slate-100 dark:hover:bg-slate-800"
          style={{ paddingInlineStart: `${depth * 14 + 8}px` }}
          onClick={() => {
            if (node.is_directory) {
              toggleFolder(node.relative_path);
            } else {
              onPreviewFile(node);
            }
          }}
        >
          {node.is_directory ? (
            <button
              type="button"
              onClick={(e) => { e.stopPropagation(); toggleFolder(node.relative_path); }}
              className="p-0.5 rounded hover:bg-slate-200 dark:hover:bg-slate-700 shrink-0"
            >
              <ChevronRight size={10} className={`text-slate-400 transition-transform ${isExpanded ? "rotate-90" : ""}`} />
            </button>
          ) : (
            <span className="w-4 shrink-0" />
          )}
          {getFileIcon(ext, !!node.is_directory)}
          <span className="truncate flex-1">{node.name}</span>
        </div>
        {node.is_directory && isExpanded && node.children && (
          node.children.map((child) => renderNode(child, depth + 1))
        )}
      </div>
    );
  };

  return (
    <div className="h-full bg-white dark:bg-slate-900 border-r border-slate-200 dark:border-slate-800 flex flex-col">
      {/* Header */}
      <div className="flex items-center justify-between px-3 py-2 border-b border-slate-200 dark:border-slate-800">
        <span className="text-xs font-medium text-slate-500 dark:text-slate-400">文件浏览</span>
        <button
          type="button"
          onClick={() => refreshTree()}
          className="p-1 rounded hover:bg-slate-100 dark:hover:bg-slate-800 text-slate-400"
          title="刷新"
        >
          <RefreshCw size={12} className={loading ? "animate-spin" : ""} />
        </button>
      </div>

      {/* Search */}
      <div className="px-3 py-2">
        <div className="relative">
          <Search size={12} className="absolute left-2 top-1/2 -translate-y-1/2 text-slate-400" />
          <input
            type="text"
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            placeholder="搜索文件..."
            className="w-full pl-6 pr-2 py-1.5 text-xs rounded border border-slate-200 dark:border-slate-700 bg-slate-50 dark:bg-slate-800 text-slate-700 dark:text-slate-300 outline-none focus:border-brand-500"
          />
        </div>
      </div>

      {/* Error message */}
      {error && (
        <div className="text-xs text-destructive px-3 py-2">{error}</div>
      )}

      {/* File tree */}
      <div className="flex-1 overflow-y-auto py-1">
        {loading && files.length === 0 ? (
          <div className="flex items-center justify-center py-12">
            <Loader2 size={16} className="text-slate-400 animate-spin" />
          </div>
        ) : displayedFiles.length === 0 ? (
          <div className="text-xs text-slate-400 text-center py-8">
            {searchQuery ? "无匹配文件" : "暂无文件"}
          </div>
        ) : (
          displayedFiles.map((node) => renderNode(node))
        )}
      </div>
    </div>
  );
}
