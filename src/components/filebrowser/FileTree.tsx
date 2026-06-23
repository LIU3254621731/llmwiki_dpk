import { useCallback, useEffect, useMemo } from "react";
import * as ContextMenu from "@radix-ui/react-context-menu";
import {
  Folder, FolderOpen, File, FileText, FileCode, FileSpreadsheet,
  FileArchive, Image, ChevronRight, Pencil, Trash2,
  ExternalLink, Copy, ScrollText,
} from "lucide-react";
import { cn } from "@/lib/utils";
import type { FileTreeNode } from "@/stores/useFileTreeStore";

// ── Props ───────────────────────────────────────────────────

export interface FileTreeProps {
  nodes: FileTreeNode[];
  expandedFolders: Set<string>;
  selectedFile: FileTreeNode | null;
  onToggleFolder: (path: string) => void;
  onSelectFile: (node: FileTreeNode) => void;
  onContextAction: (action: string, node: FileTreeNode) => void;
  onOpenFile?: (node: FileTreeNode) => void;
  level?: number;
  /** Flattened visible nodes for keyboard navigation, computed at root level */
  visibleNodes?: FileTreeNode[];
  /** Current search query for highlighting matches in node names */
  searchQuery?: string;
}

// ── Icon resolution ─────────────────────────────────────────

type IconComp = typeof File;

interface IconInfo {
  Icon: IconComp;
  colorClass: string;
}

function getFileIconInfo(node: FileTreeNode, isExpanded: boolean): IconInfo {
  const dir = !!(node.is_directory || node.is_dir);

  if (dir) {
    return {
      Icon: isExpanded ? FolderOpen : Folder,
      colorClass: "text-amber-500",
    };
  }

  const ext = (node.file_type || node.extension || "").toLowerCase();

  if (ext === "pdf") return { Icon: FileText, colorClass: "text-red-500" };
  if (ext === "docx" || ext === "doc") return { Icon: FileText, colorClass: "text-blue-500" };
  if (ext === "md" || ext === "markdown") return { Icon: FileText, colorClass: "text-emerald-500" };
  if (ext === "txt") return { Icon: FileText, colorClass: "text-slate-400" };
  if (["png", "jpg", "jpeg", "gif", "webp", "svg"].includes(ext))
    return { Icon: Image, colorClass: "text-violet-500" };
  if (["json", "xml", "csv", "yaml", "yml", "toml"].includes(ext))
    return { Icon: FileCode, colorClass: "text-amber-500" };
  if (ext === "html" || ext === "htm") return { Icon: FileCode, colorClass: "text-orange-500" };
  if (["pptx", "xlsx", "xls"].includes(ext))
    return { Icon: FileSpreadsheet, colorClass: "text-green-500" };
  if (["zip", "tar", "gz", "7z"].includes(ext))
    return { Icon: FileArchive, colorClass: "text-yellow-600" };

  return { Icon: File, colorClass: "text-slate-400" };
}

// ── Helpers ─────────────────────────────────────────────────

export function sortTreeNodes(nodes: FileTreeNode[]): FileTreeNode[] {
  return [...nodes].sort((a, b) => {
    const aDir = !!(a.is_directory || a.is_dir);
    const bDir = !!(b.is_directory || b.is_dir);
    if (aDir !== bDir) return aDir ? -1 : 1;
    return a.name.localeCompare(b.name, undefined, { sensitivity: "base" });
  });
}

function getExtBadge(node: FileTreeNode): string | null {
  const ext = (node.file_type || node.extension);
  if (!ext) return null;
  return ext.length <= 6 ? ext : ext.slice(0, 6);
}

function getVisibleNodes(
  nodes: FileTreeNode[],
  expandedFolders: Set<string>,
): FileTreeNode[] {
  const result: FileTreeNode[] = [];
  for (const node of nodes) {
    result.push(node);
    const dir = !!(node.is_directory || node.is_dir);
    if (dir && expandedFolders.has(node.relative_path) && Array.isArray(node.children)) {
      for (const child of getVisibleNodes(node.children, expandedFolders)) {
        result.push(child);
      }
    }
  }
  return result;
}

// ── Highlight helper ────────────────────────────────────────

function TreeNodeName({ name, query }: { name: string; query?: string }) {
  if (!query || !query.trim()) return <>{name}</>;
  const lower = query.toLowerCase();
  const idx = name.toLowerCase().indexOf(lower);
  if (idx === -1) return <>{name}</>;
  return (
    <>
      {name.slice(0, idx)}
      <mark className="bg-yellow-200 dark:bg-yellow-700/60 text-inherit rounded-sm px-0.5">
        {name.slice(idx, idx + query.length)}
      </mark>
      {name.slice(idx + query.length)}
    </>
  );
}

// ── TreeNodeRow ─────────────────────────────────────────────

interface TreeNodeRowProps {
  node: FileTreeNode;
  isExpanded: boolean;
  isSelected: boolean;
  level: number;
  onToggleFolder: (path: string) => void;
  onSelectFile: (node: FileTreeNode) => void;
  onContextAction: (action: string, node: FileTreeNode) => void;
  onOpenFile?: (node: FileTreeNode) => void;
  onNavigate: (direction: "up" | "down" | "right" | "left") => void;
  searchQuery?: string;
}

function TreeNodeRow({
  node, isExpanded, isSelected, level,
  onToggleFolder, onSelectFile, onContextAction, onOpenFile, onNavigate,
  searchQuery,
}: TreeNodeRowProps) {
  const dir = !!(node.is_directory || node.is_dir);
  const { Icon, colorClass } = getFileIconInfo(node, isExpanded);
  const badge = !dir ? getExtBadge(node) : null;

  const handleClick = () => {
    if (dir) { onToggleFolder(node.relative_path); }
    else { onSelectFile(node); }
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    switch (e.key) {
      case "Enter":
        e.preventDefault();
        if (dir) { onToggleFolder(node.relative_path); }
        else { onOpenFile?.(node); }
        break;
      case "ArrowDown": e.preventDefault(); onNavigate("down"); break;
      case "ArrowUp": e.preventDefault(); onNavigate("up"); break;
      case "ArrowRight":
        e.preventDefault();
        if (dir && !isExpanded) onToggleFolder(node.relative_path);
        else if (dir && isExpanded && node.children?.length) onNavigate("right");
        break;
      case "ArrowLeft":
        e.preventDefault();
        if (dir && isExpanded) onToggleFolder(node.relative_path);
        else onNavigate("left");
        break;
    }
  };

  return (
    <ContextMenu.Root>
      <ContextMenu.Trigger asChild>
        <div
          data-file-path={node.relative_path}
          tabIndex={isSelected ? 0 : -1}
          role="treeitem"
          aria-expanded={dir ? isExpanded : undefined}
          aria-selected={isSelected}
          aria-level={level + 1}
          className={cn(
            "group flex items-center gap-1.5 py-1.5 px-2 cursor-pointer select-none outline-none transition-colors",
            isSelected
              ? "bg-indigo-50 dark:bg-indigo-900/20 text-slate-900 dark:text-slate-100 border-l-2 border-indigo-500"
              : "text-slate-700 dark:text-slate-300 hover:bg-slate-100 dark:hover:bg-slate-800 border-l-2 border-transparent",
          )}
          onClick={handleClick}
          onKeyDown={handleKeyDown}
        >
          {/* Chevron for dirs */}
          {dir ? (
            <ChevronRight
              size={13}
              className={cn(
                "shrink-0 text-slate-400 dark:text-slate-500 transition-transform duration-150",
                isExpanded && "rotate-90",
              )}
            />
          ) : (
            <span className="w-[13px] shrink-0" />
          )}

          {/* File-type icon */}
          <Icon size={16} className={cn("shrink-0", colorClass)} />

          {/* Name with optional search highlight */}
          <span className={cn(
            "truncate flex-1 min-w-0 text-[13px]",
            dir ? "font-medium" : "font-normal",
          )}>
            <TreeNodeName name={node.name} query={searchQuery} />
          </span>

          {/* Extension badge */}
          {badge && (
            <span className="text-[10px] font-mono text-slate-400 dark:text-slate-500 shrink-0 opacity-50">
              {badge}
            </span>
          )}

          {/* Hover action buttons */}
          <span className="flex items-center gap-0.5 shrink-0 opacity-0 group-hover:opacity-100 group-focus-visible:opacity-100 transition-opacity">
            <button
              type="button"
              onClick={(e) => { e.stopPropagation(); onContextAction("rename", node); }}
              className="p-0.5 rounded hover:bg-slate-200 dark:hover:bg-slate-700 text-slate-400 dark:text-slate-500 hover:text-slate-600 dark:hover:text-slate-300"
              title="Rename"
              aria-label="Rename"
            >
              <Pencil size={14} />
            </button>
            <button
              type="button"
              onClick={(e) => { e.stopPropagation(); onContextAction("delete", node); }}
              className="p-0.5 rounded hover:bg-red-100 dark:hover:bg-red-900/30 text-slate-400 dark:text-slate-500 hover:text-red-500"
              title="Delete"
              aria-label="Delete"
            >
              <Trash2 size={14} />
            </button>
          </span>
        </div>
      </ContextMenu.Trigger>

      <ContextMenu.Portal>
        <ContextMenu.Content
          className="z-50 min-w-[180px] bg-white dark:bg-slate-800 border border-slate-200 dark:border-slate-700 rounded-md shadow-lg py-1 text-xs"
          
        >
          <ContextMenu.Item
            className="flex items-center gap-2 px-3 py-1.5 text-slate-600 dark:text-slate-300 outline-none cursor-default data-[highlighted]:bg-slate-100 dark:data-[highlighted]:bg-slate-700 rounded-sm mx-1"
            onClick={() => onOpenFile?.(node)}
          >
            <FileText size={13} className="text-slate-400 dark:text-slate-500" />
            Open in Editor
          </ContextMenu.Item>
          <ContextMenu.Item
            className="flex items-center gap-2 px-3 py-1.5 text-slate-600 dark:text-slate-300 outline-none cursor-default data-[highlighted]:bg-slate-100 dark:data-[highlighted]:bg-slate-700 rounded-sm mx-1"
            onClick={() => onContextAction("view_ai_log", node)}
          >
            <ScrollText size={13} className="text-slate-400 dark:text-slate-500" />
            View AI Analysis Log
          </ContextMenu.Item>

          <ContextMenu.Separator className="h-px bg-slate-100 dark:bg-slate-700 my-1 mx-1" />

          <ContextMenu.Item
            className="flex items-center gap-2 px-3 py-1.5 text-slate-600 dark:text-slate-300 outline-none cursor-default data-[highlighted]:bg-slate-100 dark:data-[highlighted]:bg-slate-700 rounded-sm mx-1"
            onClick={() => onContextAction("copy_path", node)}
          >
            <Copy size={13} className="text-slate-400 dark:text-slate-500" />
            Copy Path
          </ContextMenu.Item>
          <ContextMenu.Item
            className="flex items-center gap-2 px-3 py-1.5 text-slate-600 dark:text-slate-300 outline-none cursor-default data-[highlighted]:bg-slate-100 dark:data-[highlighted]:bg-slate-700 rounded-sm mx-1"
            onClick={() => onContextAction("reveal_in_explorer", node)}
          >
            <ExternalLink size={13} className="text-slate-400 dark:text-slate-500" />
            Show in File Explorer
          </ContextMenu.Item>

          <ContextMenu.Separator className="h-px bg-slate-100 dark:bg-slate-700 my-1 mx-1" />

          <ContextMenu.Item
            className="flex items-center gap-2 px-3 py-1.5 text-red-600 dark:text-red-400 outline-none cursor-default data-[highlighted]:bg-red-50 dark:data-[highlighted]:bg-red-900/20 rounded-sm mx-1"
            onClick={() => onContextAction("delete", node)}
          >
            <Trash2 size={13} />
            Delete
          </ContextMenu.Item>
        </ContextMenu.Content>
      </ContextMenu.Portal>
    </ContextMenu.Root>
  );
}

// ── FileTree (main export) ──────────────────────────────────

export default function FileTree({
  nodes, expandedFolders, selectedFile,
  onToggleFolder, onSelectFile, onContextAction, onOpenFile,
  level = 0, visibleNodes: propVisibleNodes, searchQuery,
}: FileTreeProps) {
  const sorted = useMemo(() => sortTreeNodes(nodes), [nodes]);

  const visibleNodes = useMemo(
    () => propVisibleNodes ?? (level === 0 ? getVisibleNodes(sorted, expandedFolders) : null),
    [level, sorted, expandedFolders, propVisibleNodes],
  );

  // Focus the DOM row when selection changes (roving tabindex)
  useEffect(() => {
    if (!selectedFile?.relative_path) return;
    const escaped = selectedFile.relative_path.replace(/\\/g, "\\\\");
    const el = document.querySelector(`[data-file-path="${escaped}"]`);
    if (el instanceof HTMLElement) {
      el.focus({ preventScroll: true });
    }
  }, [selectedFile?.relative_path]);

  // Keyboard navigation between visible nodes
  const handleNavigate = useCallback(
    (direction: "up" | "down" | "right" | "left") => {
      if (!visibleNodes || !selectedFile) return;
      const idx = visibleNodes.findIndex((n) => n.relative_path === selectedFile.relative_path);
      if (idx === -1) return;

      if (direction === "down" && idx < visibleNodes.length - 1) {
        onSelectFile(visibleNodes[idx + 1]);
      } else if (direction === "up" && idx > 0) {
        onSelectFile(visibleNodes[idx - 1]);
      } else if (direction === "right") {
        const node = visibleNodes[idx];
        const isDir = !!(node.is_directory || node.is_dir);
        if (isDir && !expandedFolders.has(node.relative_path)) {
          onToggleFolder(node.relative_path);
        } else if (isDir && node.children?.length) {
          onSelectFile(sortTreeNodes(node.children)[0]);
        }
      } else if (direction === "left") {
        const node = visibleNodes[idx];
        const isDir = !!(node.is_directory || node.is_dir);
        if (isDir && expandedFolders.has(node.relative_path)) {
          onToggleFolder(node.relative_path);
        } else {
          const parts = node.relative_path.split("/");
          if (parts.length > 1) {
            const parentPath = parts.slice(0, -1).join("/");
            const parent = visibleNodes.find((n) => n.relative_path === parentPath);
            if (parent) onSelectFile(parent);
          }
        }
      }
    },
    [visibleNodes, selectedFile, expandedFolders, onSelectFile, onToggleFolder],
  );

  const content = (
    <>
      {sorted.map((node) => {
        const dir = !!(node.is_directory || node.is_dir);
        const isExpanded = expandedFolders.has(node.relative_path);
        const isSelected = selectedFile?.relative_path === node.relative_path;
        const hasKids = dir && isExpanded && node.children && node.children.length > 0;

        const row = (
          <TreeNodeRow
            key={node.relative_path}
            node={node}
            isExpanded={isExpanded}
            isSelected={isSelected}
            level={level}
            onToggleFolder={onToggleFolder}
            onSelectFile={onSelectFile}
            onContextAction={onContextAction}
            onOpenFile={onOpenFile}
            onNavigate={handleNavigate}
            searchQuery={searchQuery}
          />
        );

        if (!hasKids) return <div key={node.relative_path}>{row}</div>;

        return (
          <div key={node.relative_path}>
            {row}
            {/* Indentation guide: dashed vertical line */}
            <div className="ml-3 pl-3 border-l border-dashed border-slate-200 dark:border-slate-700">
              <FileTree
                nodes={node.children!}
                expandedFolders={expandedFolders}
                selectedFile={selectedFile}
                onToggleFolder={onToggleFolder}
                onSelectFile={onSelectFile}
                onContextAction={onContextAction}
                onOpenFile={onOpenFile}
                level={level + 1}
                visibleNodes={visibleNodes ?? undefined}
                searchQuery={searchQuery}
              />
            </div>
          </div>
        );
      })}
    </>
  );

  if (level === 0) {
    return <div className="py-1" role="tree">{content}</div>;
  }
  return <>{content}</>;
}

