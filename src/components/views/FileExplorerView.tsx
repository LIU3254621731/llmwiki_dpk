import { useEffect, useState, useCallback } from "react";
import { useKBStore } from "@/stores/useKBStore";
import { useEditorStore } from "@/stores/useEditorStore";
import { useFileTreeStore, type FileTreeNode } from "@/stores/useFileTreeStore";
import {
  Folder, FolderOpen, File, FileText,
  ChevronRight, ChevronDown, RefreshCw, ExternalLink,
} from "lucide-react";

function nodePath(n: FileTreeNode): string {
  return n.relative_path || n.path || "";
}

export default function FileExplorerView() {
  const currentKB = useKBStore((s) => s.currentKB);
  const files = useFileTreeStore((s) => s.files);
  const loadFileTree = useFileTreeStore((s) => s.loadFileTree);

  const [expandedDirs, setExpandedDirs] = useState<Set<string>>(new Set());
  const [selectedPath, setSelectedPath] = useState<string | null>(null);

  useEffect(() => {
    if (currentKB) loadFileTree(currentKB.id, currentKB.path);
  }, [currentKB?.id]);

  const toggleDir = (p: string) => {
    setExpandedDirs((prev) => {
      const next = new Set(prev);
      next.has(p) ? next.delete(p) : next.add(p);
      return next;
    });
  };

  const handleSelectNode = useCallback((node: FileTreeNode) => {
    if (node.is_directory) {
      toggleDir(nodePath(node));
      return;
    }
    setSelectedPath(nodePath(node));
    const rp = nodePath(node);
    const { openFile } = useEditorStore.getState();
    openFile({
      path: rp,
      title: node.name,
      type: /\.md$/i.test(rp) ? "wiki" : "file",
    });
  }, []);

  return (
    <div className="h-full flex overflow-hidden">
      {/* Left: File tree */}
      <div className="w-[300px] shrink-0 border-r border-border flex flex-col overflow-hidden bg-sidebar-bg">
        <div className="flex items-center justify-between px-4 py-3 border-b border-border shrink-0">
          <h3 className="text-xs font-medium text-muted-foreground uppercase tracking-wide">文件浏览</h3>
          <button onClick={() => currentKB && loadFileTree(currentKB.id, currentKB.path)} className="p-1 text-muted-foreground hover:text-foreground rounded transition-colors" title="刷新">
            <RefreshCw size={13} />
          </button>
        </div>
        <div className="flex-1 overflow-y-auto py-1">
          {files.length === 0 ? (
            <div className="text-xs text-muted-foreground text-center py-8">暂无文件</div>
          ) : (
            <TreeList
              nodes={files}
              depth={0}
              expanded={expandedDirs}
              selectedPath={selectedPath}
              onToggle={toggleDir}
              onSelect={handleSelectNode}
            />
          )}
        </div>
      </div>

      {/* Right: Hint */}
      <div className="flex-1 flex items-center justify-center bg-background">
        <div className="text-center">
          <ExternalLink size={40} className="mx-auto mb-3 text-muted-foreground/30" />
          <p className="text-sm text-muted-foreground">点击左侧文件在新标签页中打开</p>
        </div>
      </div>
    </div>
  );
}

function TreeList({
  nodes,
  depth,
  expanded,
  selectedPath,
  onToggle,
  onSelect,
}: {
  nodes: FileTreeNode[];
  depth: number;
  expanded: Set<string>;
  selectedPath: string | null;
  onToggle: (path: string) => void;
  onSelect: (node: FileTreeNode) => void;
}) {
  return (
    <>
      {nodes.map((node) => {
        const nPath = nodePath(node);
        const isDir = node.is_directory;
        const isExpanded = expanded.has(nPath);
        const isSelected = selectedPath === nPath;
        const Icon = isDir ? (isExpanded ? FolderOpen : Folder) : nPath.endsWith(".md") ? FileText : File;

        return (
          <div key={nPath}>
            <button
              onClick={() => onSelect(node)}
              className={`w-full flex items-center gap-1.5 px-3 py-1.5 text-xs transition-colors ${
                isSelected ? "bg-primary-subtle text-primary" : "text-foreground-dim hover:bg-card-hover hover:text-foreground"
              }`}
              style={{ paddingLeft: `${12 + depth * 16}px` }}
            >
              {isDir && (
                isExpanded ? <ChevronDown size={12} className="text-muted-foreground shrink-0" /> : <ChevronRight size={12} className="text-muted-foreground shrink-0" />
              )}
              {!isDir && <span className="w-3 shrink-0" />}
              <Icon size={13} className={`shrink-0 ${isDir ? "text-primary" : "text-muted-foreground"}`} />
              <span className="truncate">{node.name}</span>
            </button>
            {isDir && isExpanded && node.children && node.children.length > 0 && (
              <TreeList nodes={node.children} depth={depth + 1} expanded={expanded} selectedPath={selectedPath} onToggle={onToggle} onSelect={onSelect} />
            )}
          </div>
        );
      })}
    </>
  );
}
