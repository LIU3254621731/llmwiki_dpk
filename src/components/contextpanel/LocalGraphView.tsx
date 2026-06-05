import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

interface RelatedNode {
  id: string;
  label: string;
  path: string;
  relation: string;
  direction: "outgoing" | "incoming";
}

interface LocalGraphViewProps {
  nodePath: string;
  nodeTitle: string;
  kbId: string;
  depth?: number;
  onNavigate?: (path: string) => void;
}

const CANVAS_HEIGHT = 250;

export default function LocalGraphView({ nodePath, nodeTitle, kbId, depth = 1, onNavigate }: LocalGraphViewProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  const [relatedNodes, setRelatedNodes] = useState<RelatedNode[]>([]);
  const [canvasWidth, setCanvasWidth] = useState(280);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    let cancelled = false;
    async function loadGraph() {
      try {
        setLoading(true);
        const nodes = await invoke<any[]>("search_graph_nodes", { kbId, keyword: nodeTitle });
        if (cancelled) return;

        let nodeId: string | null = null;
        for (const n of nodes) {
          if (n.label === nodeTitle || n.path === nodePath) {
            nodeId = n.id;
            break;
          }
        }
        if (!nodeId && nodes.length > 0) {
          nodeId = nodes[0].id;
        }
        if (!nodeId) {
          if (!cancelled) setLoading(false);
          return;
        }

        const relations = await invoke<{ outgoing: any[]; incoming: any[] }>("get_node_relations", {
          kbId,
          nodeId,
        });
        if (cancelled) return;

        const all: RelatedNode[] = [];
        for (const r of relations.outgoing || []) {
          all.push({
            id: r.targetId,
            label: r.targetLabel,
            path: r.targetPath,
            relation: r.relation,
            direction: "outgoing",
          });
        }
        for (const r of relations.incoming || []) {
          all.push({
            id: r.sourceId,
            label: r.sourceLabel,
            path: r.sourcePath,
            relation: r.relation,
            direction: "incoming",
          });
        }

        if (!cancelled) {
          setRelatedNodes(all);
          setLoading(false);
        }
      } catch {
        if (!cancelled) setLoading(false);
      }
    }
    loadGraph();
    return () => { cancelled = true; };
  }, [kbId, nodeTitle, nodePath]);

  useEffect(() => {
    const el = containerRef.current;
    if (!el) return;
    const obs = new ResizeObserver((entries) => {
      for (const entry of entries) {
        setCanvasWidth(entry.contentRect.width);
      }
    });
    obs.observe(el);
    return () => obs.disconnect();
  }, []);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas || loading) return;

    const dpr = window.devicePixelRatio || 1;
    canvas.width = canvasWidth * dpr;
    canvas.height = CANVAS_HEIGHT * dpr;
    canvas.style.width = canvasWidth + "px";
    canvas.style.height = CANVAS_HEIGHT + "px";

    const ctx = canvas.getContext("2d");
    if (!ctx) return;
    ctx.scale(dpr, dpr);

    const cx = canvasWidth / 2;
    const cy = CANVAS_HEIGHT / 2;

    ctx.clearRect(0, 0, canvasWidth, CANVAS_HEIGHT);

    if (relatedNodes.length === 0) return;

    // Draw center node
    const centerRadius = 28;
    ctx.beginPath();
    ctx.arc(cx, cy, centerRadius, 0, Math.PI * 2);
    ctx.fillStyle = "#3b82f6";
    ctx.fill();
    ctx.strokeStyle = "#2563eb";
    ctx.lineWidth = 2;
    ctx.stroke();

    ctx.fillStyle = "#ffffff";
    ctx.font = "11px sans-serif";
    ctx.textAlign = "center";
    ctx.textBaseline = "middle";
    const centerLabel = nodeTitle.length > 8 ? nodeTitle.slice(0, 8) + ".." : nodeTitle;
    ctx.fillText(centerLabel, cx, cy);

    // Arrange related nodes in a circle
    const orbitRadius = Math.min(canvasWidth, CANVAS_HEIGHT) / 2 - 50;
    const count = Math.min(relatedNodes.length, 12);
    const displayed = relatedNodes.slice(0, count);

    displayed.forEach((node, i) => {
      const angle = (i / count) * Math.PI * 2 - Math.PI / 2;
      const nx = cx + Math.cos(angle) * orbitRadius;
      const ny = cy + Math.sin(angle) * orbitRadius;
      const nodeRadius = 16;

      // Edge line
      ctx.beginPath();
      const edgeStartX = cx + Math.cos(angle) * centerRadius;
      const edgeStartY = cy + Math.sin(angle) * centerRadius;
      const edgeEndX = nx - Math.cos(angle) * nodeRadius;
      const edgeEndY = ny - Math.sin(angle) * nodeRadius;
      ctx.moveTo(edgeStartX, edgeStartY);
      ctx.lineTo(edgeEndX, edgeEndY);
      ctx.strokeStyle = "#cbd5e1";
      ctx.lineWidth = 1;
      ctx.stroke();

      // Relation label at midpoint
      const mx = (cx + nx) / 2;
      const my = (cy + ny) / 2;
      ctx.fillStyle = "#94a3b8";
      ctx.font = "9px sans-serif";
      const relLabel = node.relation.length > 6 ? node.relation.slice(0, 6) : node.relation;
      ctx.fillText(relLabel, mx, my - 6);

      // Related node circle
      ctx.beginPath();
      ctx.arc(nx, ny, nodeRadius, 0, Math.PI * 2);
      ctx.fillStyle = "#cbd5e1";
      ctx.fill();
      ctx.strokeStyle = "#94a3b8";
      ctx.lineWidth = 1;
      ctx.stroke();

      // Related node label
      ctx.fillStyle = "#334155";
      ctx.font = "10px sans-serif";
      const rl = node.label.length > 6 ? node.label.slice(0, 6) : node.label;
      ctx.fillText(rl, nx, ny + 1);
    });

    // Click handler on canvas
    const handleClick = (e: MouseEvent) => {
      const rect = canvas.getBoundingClientRect();
      const mx = e.clientX - rect.left;
      const my = e.clientY - rect.top;

      // Check center node
      const distCenter = Math.sqrt((mx - cx) ** 2 + (my - cy) ** 2);
      if (distCenter <= centerRadius && nodePath) {
        onNavigate?.(nodePath);
        return;
      }

      // Check related nodes
      displayed.forEach((node, i) => {
        const angle = (i / count) * Math.PI * 2 - Math.PI / 2;
        const nx = cx + Math.cos(angle) * orbitRadius;
        const ny = cy + Math.sin(angle) * orbitRadius;
        const dist = Math.sqrt((mx - nx) ** 2 + (my - ny) ** 2);
        if (dist <= 16 && node.path) {
          onNavigate?.(node.path);
        }
      });
    };

    canvas.addEventListener("click", handleClick);
    return () => canvas.removeEventListener("click", handleClick);
  }, [relatedNodes, canvasWidth, nodeTitle, nodePath, loading, onNavigate]);

  if (loading) {
    return (
      <div className="py-4 text-center">
        <span className="text-xs text-slate-400 italic">加载中...</span>
      </div>
    );
  }

  if (relatedNodes.length === 0) {
    return (
      <div className="py-2">
        <p className="text-xs font-medium text-slate-500 mb-2 px-2">关联页面:</p>
        <div className="py-2 text-center">
          <span className="text-xs text-slate-400 italic">无关联节点</span>
        </div>
      </div>
    );
  }

  return (
    <div>
      <div ref={containerRef} className="w-full">
        <canvas
          ref={canvasRef}
          className="w-full cursor-pointer"
          style={{ height: CANVAS_HEIGHT }}
        />
      </div>
      <div className="mt-2">
        <p className="text-xs font-medium text-slate-500 mb-1 px-2">关联页面:</p>
        <div className="space-y-0.5">
          {relatedNodes.slice(0, 12).map((node) => (
            <button
              key={node.id}
              type="button"
              onClick={() => node.path && onNavigate?.(node.path)}
              className="flex items-center gap-1.5 w-full text-left px-2 py-1 rounded text-xs hover:bg-slate-50"
            >
              <span className="px-1 py-0.5 rounded text-[10px] bg-slate-100 text-slate-500 shrink-0">
                {node.direction === "incoming" ? "入" : "出"}
              </span>
              <span className="text-slate-500 text-[10px] shrink-0">{node.relation}</span>
              <span className="text-brand-600 truncate">{node.label}</span>
            </button>
          ))}
        </div>
      </div>
    </div>
  );
}
