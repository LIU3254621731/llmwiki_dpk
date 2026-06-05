// ── Macro Canvas: Tag Relationship Network ──
// Uses AntV X6 for interactive drag-and-drop editing with labeled edges.

import { useRef, useCallback, useEffect } from "react";
import { Graph, Shape } from "@antv/x6";
import type { CanvasSchema } from "../types";
import { KnowledgeCanvas, type KnowledgeCanvasHandle } from "../KnowledgeCanvas";
import { useCanvasEngineStore } from "@/stores/useCanvasEngineStore";
import { useKBStore } from "@/stores/useKBStore";
import type { GraphNode, GraphEdge } from "@/types/graph";

// ── Register Macro-specific node/edge shapes ──
function registerMacroShapes(graph: Graph) {
  // Capsule/bubble HTML node
  Shape.HTML.register({
    shape: "macro-node",
    width: 150,
    height: 50,
    effect: ["data"],
    html(cell) {
      const data = cell.getData();
      const label = (data?.label ?? data?.topic ?? cell.id) as string;
      const nodeType = (data?.type ?? "concept") as string;
      const hasMicroMap = (data?.hasMicroMap ?? false) as boolean;

      const colors: Record<string, { bg: string; border: string; text: string }> = {
        concept: { bg: "#fffbeb", border: "#fbbf24", text: "#92400e" },
        entity: { bg: "#ecfdf5", border: "#34d399", text: "#065f46" },
        person: { bg: "#ecfeff", border: "#22d3ee", text: "#155e75" },
        source: { bg: "#fff7ed", border: "#fb923c", text: "#9a3412" },
        wikipage: { bg: "#eef2ff", border: "#818cf8", text: "#3730a3" },
        tag: { bg: "#f5f3ff", border: "#a78bfa", text: "#5b21b6" },
        default: { bg: "#f8fafc", border: "#cbd5e1", text: "#475569" },
      };
      const c = colors[nodeType] ?? colors.default;

      const drillBtn = hasMicroMap
        ? `<div class="macro-drill-btn" title="打开微观导图" style="
            position:absolute;top:-8px;right:-8px;width:18px;height:18px;
            background:#3b82f6;border-radius:50%;display:flex;align-items:center;
            justify-content:center;cursor:pointer;font-size:10px;color:white;
            box-shadow:0 1px 3px rgba(0,0,0,0.2);z-index:10;
          " data-node-id="${cell.id}">+</div>`
        : "";

      return `
        <div style="
          background:linear-gradient(135deg, ${c.bg} 0%, #fff 100%);
          border:2px solid ${c.border};border-radius:20px;
          padding:8px 18px;font-size:12px;font-weight:600;
          color:${c.text};white-space:nowrap;overflow:hidden;
          text-overflow:ellipsis;max-width:180px;cursor:pointer;
          box-shadow:0 2px 6px rgba(0,0,0,0.06);transition:box-shadow 0.15s;
          position:relative;
        " data-node-id="${cell.id}">
          ${drillBtn}
          <span>${escapeHtml(label)}</span>
        </div>
      `;
    },
  });

  // Register a labeled edge type (double-click to edit label)
  Graph.registerEdge(
    "macro-edge",
    {
      inherit: "edge",
      attrs: {
        line: {
          stroke: "#94a3b8",
          strokeWidth: 2,
          targetMarker: { name: "block", width: 12, height: 8 },
        },
      },
      labels: [],
    },
    true,
  );
}

function escapeHtml(text: string): string {
  const el = document.createElement("div");
  el.textContent = text;
  return el.innerHTML;
}

// ── Convert GraphData (from backend) to CanvasSchema ──
export function graphDataToSchema(
  nodes: GraphNode[],
  edges: GraphEdge[],
): CanvasSchema {
  const nodeMap = new Map<string, { x: number; y: number }>();
  const cols = Math.ceil(Math.sqrt(Math.max(nodes.length, 1)));
  nodes.forEach((n, i) => {
    const col = i % cols;
    const row = Math.floor(i / cols);
    nodeMap.set(n.id, { x: col * 220 + 100, y: row * 120 + 80 });
  });

  return {
    version: 1,
    canvasType: "macro",
    canvasId: "default",
    nodes: nodes.map((n) => ({
      id: n.id,
      type: "macro-node",
      x: nodeMap.get(n.id)?.x ?? 200,
      y: nodeMap.get(n.id)?.y ?? 200,
      width: 150,
      height: 50,
      data: {
        id: n.id,
        label: n.label,
        type: n.type,
        tags: n.tags ?? [],
        summary: n.summary,
        hasMicroMap: false,
      },
    })),
    edges: edges.map((e) => ({
      id: e.id,
      source: e.source,
      target: e.target,
      data: {
        id: e.id,
        label: e.relation,
        relation: e.relation,
        arrowType: "single",
      },
    })),
    metadata: {},
  };
}

interface MacroCanvasProps {
  nodes: GraphNode[];
  edges: GraphEdge[];
  onDrillToMicro?: (nodeId: string, nodeLabel: string) => void;
}

export default function MacroCanvas({ nodes, edges, onDrillToMicro }: MacroCanvasProps) {
  const canvasRef = useRef<KnowledgeCanvasHandle>(null);
  const currentKB = useKBStore((s) => s.currentKB);
  const macroSchema = useCanvasEngineStore((s) => s.macroSchema);
  const setMacroSchema = useCanvasEngineStore((s) => s.setMacroSchema);

  const initialSchema = macroSchema ?? graphDataToSchema(nodes, edges);

  const handleGraphReady = useCallback(
    (graph: Graph) => {
      const container = graph.container;
      if (!container) return;

      const handler = (e: MouseEvent) => {
        const target = e.target as HTMLElement;
        if (target.classList.contains("macro-drill-btn") || target.closest(".macro-drill-btn")) {
          const btn = target.classList.contains("macro-drill-btn")
            ? target
            : target.closest(".macro-drill-btn")!;
          const nodeId = btn.getAttribute("data-node-id");
          if (nodeId && onDrillToMicro) {
            const node = graph.getCellById(nodeId);
            const label = (node?.getData()?.label ?? nodeId) as string;
            onDrillToMicro(nodeId, label);
          }
        }
      };
      container.addEventListener("click", handler);
      (graph as any).__macroClickCleanup = () => container.removeEventListener("click", handler);
    },
    [onDrillToMicro],
  );

  const handleGraphChanged = useCallback(
    (schema: CanvasSchema) => {
      setMacroSchema(schema);
    },
    [setMacroSchema],
  );

  useEffect(() => {
    return () => {
      const g = canvasRef.current?.getGraph();
      if (g && (g as any).__macroClickCleanup) {
        (g as any).__macroClickCleanup();
      }
    };
  }, []);

  return (
    <KnowledgeCanvas
      ref={canvasRef}
      canvasId="macro-default"
      canvasType="macro"
      initialSchema={initialSchema}
      registerCustomShapes={registerMacroShapes}
      onGraphReady={handleGraphReady}
      onGraphChanged={handleGraphChanged}
      background="#f8fafc"
      grid={{ size: 20, color: "#e2e8f0" }}
      className="macro-canvas"
    />
  );
}
