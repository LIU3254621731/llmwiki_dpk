import { useState } from "react";
import { useCanvasStore } from "@/stores/useCanvasStore";
import { ChevronRight, ChevronDown, FileText } from "lucide-react";
import type { OutlineNode } from "@/types/canvas";

interface TreeNodeProps {
  node: OutlineNode;
  depth: number;
  activeId: string | null;
  expandedIds: Set<string>;
  onToggle: (id: string) => void;
  onClick: (id: string) => void;
}

function TreeNode({ node, depth, activeId, expandedIds, onToggle, onClick }: TreeNodeProps) {
  const isExpanded = expandedIds.has(node.id);
  const isActive = activeId === node.id;
  const hasChildren = node.children && node.children.length > 0;

  return (
    <div>
      <div
        className={`flex items-center gap-1 py-1.5 px-2 cursor-pointer hover:bg-slate-100 dark:hover:bg-slate-800 rounded text-sm transition-colors ${
          isActive
            ? "bg-primary/10 text-primary font-medium border-l-2 border-primary"
            : "text-foreground border-l-2 border-transparent"
        }`}
        style={{ paddingLeft: `${depth * 16 + 8}px` }}
        onClick={() => onClick(node.id)}
      >
        {hasChildren ? (
          <button
            type="button"
            onClick={(e) => {
              e.stopPropagation();
              onToggle(node.id);
            }}
            className="p-0.5 shrink-0"
          >
            {isExpanded ? (
              <ChevronDown size={14} className="text-muted-foreground" />
            ) : (
              <ChevronRight size={14} className="text-muted-foreground" />
            )}
          </button>
        ) : (
          <span className="w-[18px] shrink-0" />
        )}
        <FileText size={12} className="text-muted-foreground shrink-0" />
        <span className="truncate">{node.title}</span>
      </div>
      {hasChildren && isExpanded && (
        <div>
          {node.children.map((child) => (
            <TreeNode
              key={child.id}
              node={child}
              depth={depth + 1}
              activeId={activeId}
              expandedIds={expandedIds}
              onToggle={onToggle}
              onClick={onClick}
            />
          ))}
        </div>
      )}
    </div>
  );
}

export default function CanvasOutlineTree() {
  const outlineNodes = useCanvasStore((s) => s.outlineNodes);
  const activeNodeId = useCanvasStore((s) => s.activeNodeId);
  const setActiveNodeId = useCanvasStore((s) => s.setActiveNodeId);
  const showDetailPanel = useCanvasStore((s) => s.showDetailPanel);
  const generationPhase = useCanvasStore((s) => s.generationPhase);

  const [expandedIds, setExpandedIds] = useState<Set<string>>(() => {
    // Initially expand first-level nodes
    const s = new Set<string>();
    outlineNodes.forEach((node) => s.add(node.id));
    return s;
  });

  const handleToggle = (id: string) => {
    setExpandedIds((prev) => {
      const next = new Set(prev);
      if (next.has(id)) {
        next.delete(id);
      } else {
        next.add(id);
      }
      return next;
    });
  };

  const handleClick = (id: string) => {
    setActiveNodeId(id);
    const node = findNode(outlineNodes, id);
    if (node) {
      showDetailPanel(node.title);
    }
  };

  if (generationPhase === "outline") {
    return (
      <div className="flex items-center justify-center h-full">
        <div className="text-center px-4">
          <div className="w-6 h-6 border-2 border-primary border-t-transparent rounded-full animate-spin mx-auto mb-2" />
          <p className="text-xs text-muted-foreground">生成知识大纲中...</p>
        </div>
      </div>
    );
  }

  if (outlineNodes.length === 0) {
    return (
      <div className="flex items-center justify-center h-full px-4">
        <p className="text-xs text-muted-foreground text-center">
          选择标签并点击"生成教材"以创建知识大纲
        </p>
      </div>
    );
  }

  return (
    <div className="py-2">
      <div className="px-3 py-1.5 text-xs font-semibold text-muted-foreground uppercase tracking-wider">
        知识大纲
      </div>
      {outlineNodes.map((node) => (
        <TreeNode
          key={node.id}
          node={node}
          depth={0}
          activeId={activeNodeId}
          expandedIds={expandedIds}
          onToggle={handleToggle}
          onClick={handleClick}
        />
      ))}
    </div>
  );
}

function findNode(nodes: OutlineNode[], id: string): OutlineNode | null {
  for (const node of nodes) {
    if (node.id === id) return node;
    if (node.children) {
      const found = findNode(node.children, id);
      if (found) return found;
    }
  }
  return null;
}
