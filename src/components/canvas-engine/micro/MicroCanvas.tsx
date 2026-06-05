// ── Micro Canvas: Mindmap Tree ──
// Tree-structured mindmap with inline editing, add-child, and subtree reparenting.

import { useRef, useCallback, useEffect, useState } from "react";
import { Graph, Shape } from "@antv/x6";
import type { Node } from "@antv/x6";
import type { CanvasSchema, CanvasSchemaNode, CanvasSchemaEdge } from "../types";
import { KnowledgeCanvas, type KnowledgeCanvasHandle } from "../KnowledgeCanvas";
import { computeMindmapLayout } from "../mindmap-layout";
import type { MindmapNode } from "../types";
import { useCanvasEngineStore } from "@/stores/useCanvasEngineStore";
import { useKBStore } from "@/stores/useKBStore";

const DEPTH_COLORS = [
  { bg: "#dbeafe", border: "#3b82f6", text: "#1e40af" },
  { bg: "#dcfce7", border: "#22c55e", text: "#166534" },
  { bg: "#fff7ed", border: "#f97316", text: "#9a3412" },
  { bg: "#f5f3ff", border: "#8b5cf6", text: "#5b21b6" },
  { bg: "#fce7f3", border: "#ec4899", text: "#9d174d" },
];

function getDepthColor(depth: number) {
  return DEPTH_COLORS[Math.min(depth, DEPTH_COLORS.length - 1)];
}

function registerMicroShapes(_graph: Graph) {
  Shape.HTML.register({
    shape: "micro-node",
    width: 160,
    height: 48,
    effect: ["data"],
    ports: {
      groups: {
        in: {
          position: "left",
          attrs: {
            circle: {
              r: 0,
              magnet: true,
              style: { visibility: "hidden" },
            },
          },
        },
        out: {
          position: "right",
          attrs: {
            circle: {
              r: 0,
              magnet: true,
              style: { visibility: "hidden" },
            },
          },
        },
      },
      items: [
        { id: "in", group: "in" },
        { id: "out", group: "out" },
      ],
    },
    html(cell) {
      const data = cell.getData();
      const topic = (data?.topic ?? cell.id) as string;
      const depth = (data?.depth ?? 0) as number;
      const colors = getDepthColor(depth);

      return `
        <div class="micro-node-wrapper" style="position:relative;display:flex;align-items:center;" data-node-id="${cell.id}">
          <div class="micro-add-btn" title="添加子节点" style="
            position:absolute;left:-10px;top:50%;transform:translateY(-50%);
            width:20px;height:20px;background:#22c55e;border-radius:50%;
            display:flex;align-items:center;justify-content:center;cursor:pointer;
            font-size:12px;color:white;z-index:10;opacity:0;transition:opacity 0.15s;
            box-shadow:0 1px 3px rgba(0,0,0,0.2);
          ">+</div>
          <div class="micro-node-body" style="
            background:linear-gradient(135deg, ${colors.bg} 0%, #fff 100%);
            border:2px solid ${colors.border};border-radius:12px;
            padding:8px 16px;font-size:12px;font-weight:600;
            color:${colors.text};white-space:nowrap;overflow:hidden;
            text-overflow:ellipsis;max-width:156px;cursor:pointer;
            box-shadow:0 2px 6px rgba(0,0,0,0.06);transition:box-shadow 0.15s;
          " data-node-id="${cell.id}">
            <span class="micro-topic-text">${escapeHtml(topic)}</span>
          </div>
        </div>
      `;
    },
  });
}

function escapeHtml(text: string): string {
  const el = document.createElement("div");
  el.textContent = text;
  return el.innerHTML;
}

export function mindmapTreeToSchema(root: MindmapNode, canvasId: string): CanvasSchema {
  const layout = computeMindmapLayout(root, {
    horizontalGap: 240,
    verticalGap: 80,
    nodeWidth: 160,
  });

  const nodes: CanvasSchemaNode[] = layout.nodes.map((n) => ({
    id: n.id,
    type: "micro-node",
    x: n.x,
    y: n.y,
    width: 160,
    height: 48,
    data: { id: n.id, topic: n.topic, depth: n.depth },
  }));

  const edges: CanvasSchemaEdge[] = [];
  function collectEdges(node: MindmapNode, parentId?: string) {
    if (parentId) {
      edges.push({
        id: `edge_${parentId}_${node.id}`,
        source: parentId,
        target: node.id,
        sourcePort: "out",
        targetPort: "in",
        router: "smooth",
        connector: "smooth",
        data: { relation: "parent_child" },
      });
    }
    if (node.children) {
      for (const child of node.children) {
        collectEdges(child, node.id);
      }
    }
  }
  collectEdges(root);

  return {
    version: 1,
    canvasType: "micro",
    canvasId,
    nodes,
    edges,
    metadata: { rootId: root.id },
  };
}

interface MicroCanvasProps {
  tagId: string;
  rootTopic?: string;
  initialTree?: MindmapNode | null;
  onBack?: () => void;
}

export default function MicroCanvas({
  tagId,
  rootTopic = "中心主题",
  initialTree,
  onBack,
}: MicroCanvasProps) {
  const canvasRef = useRef<KnowledgeCanvasHandle>(null);
  const currentKB = useKBStore((s) => s.currentKB);
  const microSchemas = useCanvasEngineStore((s) => s.microSchemas);
  const setMicroSchema = useCanvasEngineStore((s) => s.setMicroSchema);
  const loadMicroCanvas = useCanvasEngineStore((s) => s.loadMicroCanvas);

  useEffect(() => {
    if (currentKB) {
      loadMicroCanvas(currentKB.id, tagId);
    }
  }, [currentKB, tagId, loadMicroCanvas]);

  const storedSchema = microSchemas[tagId];
  const initialSchema =
    storedSchema ??
    mindmapTreeToSchema(
      initialTree ?? { id: "root", topic: rootTopic, children: [] },
      tagId,
    );

  const handleGraphReady = useCallback(
    (graph: Graph) => {
      // ── Double-click to edit node text ──
      graph.on("node:dblclick", ({ node }) => {
        startInlineEdit(graph, node);
      });

      const container = graph.container;
      if (!container) return;

      const clickHandler = (e: MouseEvent) => {
        const target = e.target as HTMLElement;

        // Add child button
        if (
          target.classList.contains("micro-add-btn") ||
          target.closest(".micro-add-btn")
        ) {
          e.stopPropagation();
          const btn = target.classList.contains("micro-add-btn")
            ? target
            : target.closest(".micro-add-btn")!;
          const nodeId = btn.parentElement?.getAttribute("data-node-id");
          if (nodeId) {
            handleAddChild(graph, nodeId);
          }
        }
      };
      container.addEventListener("click", clickHandler);
      (graph as any).__microClickCleanup = () =>
        container.removeEventListener("click", clickHandler);

      // Hover: show add button
      graph.on("node:mouseenter", ({ node }) => {
        const c = graph.container;
        if (!c) return;
        const el = c.querySelector(
          `.micro-node-wrapper[data-node-id="${node.id}"] .micro-add-btn`,
        ) as HTMLElement;
        if (el) el.style.opacity = "1";
      });
      graph.on("node:mouseleave", ({ node }) => {
        const c = graph.container;
        if (!c) return;
        const el = c.querySelector(
          `.micro-node-wrapper[data-node-id="${node.id}"] .micro-add-btn`,
        ) as HTMLElement;
        if (el) el.style.opacity = "0";
      });
    },
    [],
  );

  const handleGraphChanged = useCallback(
    (schema: CanvasSchema) => {
      setMicroSchema(tagId, schema);
    },
    [tagId, setMicroSchema],
  );

  useEffect(() => {
    return () => {
      const g = canvasRef.current?.getGraph();
      if (g && (g as any).__microClickCleanup) {
        (g as any).__microClickCleanup();
      }
    };
  }, []);

  return (
    <div
      className="micro-canvas-container"
      style={{ width: "100%", height: "100%", position: "relative" }}
    >
      {onBack && (
        <button
          type="button"
          onClick={onBack}
          className="absolute top-3 left-3 z-10 px-3 py-1.5 text-xs font-medium bg-white border border-border rounded-lg shadow-sm hover:bg-sidebar-bg transition-colors"
        >
          ← 返回宏观图谱
        </button>
      )}
      <KnowledgeCanvas
        ref={canvasRef}
        canvasId={tagId}
        canvasType="micro"
        initialSchema={initialSchema}
        registerCustomShapes={registerMicroShapes}
        onGraphReady={handleGraphReady}
        onGraphChanged={handleGraphChanged}
        background="#fafbfc"
        grid={{ size: 20, color: "#e2e8f0" }}
        snapline={true}
        selection={false}
        scroller={true}
      />
    </div>
  );
}

// ── Inline editing ──

function startInlineEdit(graph: Graph, node: Node) {
  const data = node.getData();
  const currentTopic = (data?.topic ?? "") as string;
  const container = graph.container;
  if (!container) return;

  const pos = node.position();
  const size = node.size();

  const existing = container.querySelector(".micro-inline-editor");
  if (existing) existing.remove();

  const input = document.createElement("input");
  input.className = "micro-inline-editor";
  input.value = currentTopic;
  input.style.cssText = `
    position:absolute;left:${pos.x}px;top:${pos.y}px;
    width:${size.width}px;height:${size.height}px;
    font-size:12px;font-weight:600;text-align:center;
    border:2px solid #3b82f6;border-radius:12px;outline:none;
    background:white;z-index:100;padding:0 8px;
  `;

  container.appendChild(input);
  input.focus();
  input.select();

  const commitEdit = () => {
    const newTopic = input.value.trim() || currentTopic;
    node.setData({ ...data, topic: newTopic }, { overwrite: true });
    input.remove();
    graph.trigger("cell:change:data", { cell: node });
  };

  input.addEventListener("blur", commitEdit);
  input.addEventListener("keydown", (e) => {
    if (e.key === "Enter") { e.preventDefault(); commitEdit(); }
    if (e.key === "Escape") { input.value = currentTopic; commitEdit(); }
  });
}

// ── Add child node ──

function handleAddChild(graph: Graph, parentId: string) {
  const parentNode = graph.getCellById(parentId);
  if (!parentNode || !parentNode.isNode()) return;

  const childId = `child_${Date.now()}_${Math.random().toString(36).slice(2, 6)}`;
  const parentPos = parentNode.position();
  const parentData = parentNode.getData();
  const parentDepth = ((parentData?.depth as number) ?? 0);
  const childDepth = parentDepth + 1;
  const siblingCount = graph.getEdges().filter(
    (e) => e.getSourceCellId() === parentId,
  ).length;
  const newY = parentPos.y + (siblingCount + 1) * 70;

  graph.addNode({
    id: childId,
    x: parentPos.x + 220,
    y: newY,
    width: 160,
    height: 48,
    shape: "micro-node",
    data: { id: childId, topic: "新节点", depth: childDepth, parentId },
  });

  graph.addEdge({
    id: `edge_${parentId}_${childId}`,
    source: { cell: parentId, port: "out" },
    target: { cell: childId, port: "in" },
    router: { name: "smooth" },
    connector: { name: "smooth" },
    attrs: {
      line: {
        stroke: "#94a3b8",
        strokeWidth: 1.5,
        targetMarker: { name: "block", width: 8, height: 6 },
      },
    },
  });

  setTimeout(() => graph.centerContent(), 100);
}
