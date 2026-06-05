// ── Generic AntV X6 canvas base component ──
// Provides: Snapline, Selection, Scroller/Zoom, canvas pan, export/import schema.
// Both MacroCanvas and MicroCanvas are built on top of this.

import { useEffect, useRef, useCallback, forwardRef, useImperativeHandle } from "react";
import {
  Graph,
  Shape,
  Scroller,
  Snapline,
  Selection,
  Keyboard,
  History,
} from "@antv/x6";
import type { NodeMetadata, EdgeMetadata, Node, Edge } from "@antv/x6";
import type { CanvasSchema, CanvasSchemaNode, CanvasSchemaEdge } from "./types";
import {
  schemaNodeToX6,
  schemaEdgeToX6,
  x6NodeToSchema,
  x6EdgeToSchema,
} from "./types";

// ── Default node/edge registration ──
let registered = false;
function ensureRegistered() {
  if (registered) return;
  Shape.HTML.register({
    shape: "custom-html",
    width: 160,
    height: 60,
    effect: ["data"],
    html(cell) {
      const data = cell.getData();
      return data?.htmlContent ?? `<div>${cell.id}</div>`;
    },
  });
  registered = true;
}

export interface KnowledgeCanvasHandle {
  exportSchema(): CanvasSchema;
  importSchema(schema: CanvasSchema): void;
  getGraph(): Graph;
  fitToContent(padding?: number): void;
  applyAgentActions(
    actions: Array<{
      type: string;
      [key: string]: unknown;
    }>,
  ): Promise<void>;
}

interface KnowledgeCanvasProps {
  canvasId: string;
  canvasType: "macro" | "micro";
  className?: string;
  initialSchema?: CanvasSchema | null;
  registerCustomShapes?: (graph: Graph) => void;
  onGraphReady?: (graph: Graph) => void;
  onGraphChanged?: (schema: CanvasSchema) => void;
  snapline?: boolean;
  selection?: boolean;
  scroller?: boolean;
  keyboard?: boolean;
  minZoom?: number;
  maxZoom?: number;
  background?: string;
  grid?: boolean | { size: number; color: string };
}

export const KnowledgeCanvas = forwardRef<KnowledgeCanvasHandle, KnowledgeCanvasProps>(
  function KnowledgeCanvas(
    {
      canvasId,
      canvasType,
      className,
      initialSchema,
      registerCustomShapes,
      onGraphReady,
      onGraphChanged,
      snapline = true,
      selection = true,
      scroller = true,
      keyboard = true,
      minZoom = 0.1,
      maxZoom = 3,
      background = "#f8fafc",
      grid,
    },
    ref,
  ) {
    const containerRef = useRef<HTMLDivElement>(null);
    const graphRef = useRef<Graph | null>(null);
    const schemaRef = useRef<CanvasSchema>({
      version: 1,
      canvasType,
      canvasId,
      nodes: [],
      edges: [],
      metadata: {},
    });
    const changeTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);
    const onGraphChangedRef = useRef(onGraphChanged);
    onGraphChangedRef.current = onGraphChanged;

    // Debounced change notification
    const notifyChange = useCallback(() => {
      if (changeTimeoutRef.current) clearTimeout(changeTimeoutRef.current);
      changeTimeoutRef.current = setTimeout(() => {
        if (onGraphChangedRef.current && graphRef.current) {
          const schema = exportCurrentSchema();
          schemaRef.current = schema;
          onGraphChangedRef.current(schema);
        }
      }, 300);
    }, []);

    const exportCurrentSchema = useCallback((): CanvasSchema => {
      const g = graphRef.current;
      if (!g) return schemaRef.current;

      const nodes: CanvasSchemaNode[] = g.getNodes().map((n) => x6NodeToSchema(n));
      const edges: CanvasSchemaEdge[] = g.getEdges().map((e) => x6EdgeToSchema(e));

      return {
        version: 1,
        canvasType,
        canvasId,
        nodes,
        edges,
        metadata: schemaRef.current.metadata,
      };
    }, [canvasType, canvasId]);

    // ── Initialize graph ──
    useEffect(() => {
      if (!containerRef.current) return;

      ensureRegistered();

      const graph = new Graph({
        container: containerRef.current,
        autoResize: true,
        background: { color: background },
        grid: grid
          ? typeof grid === "boolean"
            ? { size: 20, visible: true, type: "dot" as const }
            : {
                size: grid.size,
                visible: true,
                type: "dot" as const,
                args: { color: grid.color, thickness: 1 },
              }
          : { size: 20, visible: false },
        mousewheel: scroller
          ? {
              enabled: true,
              zoomAtMousePosition: true,
              modifiers: "ctrl",
              minScale: minZoom,
              maxScale: maxZoom,
            }
          : { enabled: false },
        panning: {
          enabled: true,
          eventTypes: ["leftMouseDown", "mouseWheel"],
        },
        connecting: {
          router: { name: "manhattan" },
          connector: { name: "rounded", args: { radius: 8 } },
          anchor: "center",
          connectionPoint: "anchor",
          allowBlank: false,
          snap: { radius: 20 },
          createEdge() {
            return new Shape.Edge({
              attrs: {
                line: {
                  stroke: "#94a3b8",
                  strokeWidth: 2,
                  targetMarker: { name: "block", width: 12, height: 8 },
                },
              },
            });
          },
          validateConnection({ sourceCell, targetCell }) {
            if (!sourceCell || !targetCell) return false;
            if (sourceCell === targetCell) return false;
            return true;
          },
        },
      });

      // Install plugins
      if (selection) {
        graph.use(new Selection({ enabled: true, rubberband: true, showNodeSelectionBox: true }));
      }
      if (snapline) {
        graph.use(new Snapline({ enabled: true, sharp: true }));
      }
      if (scroller) {
        graph.use(new Scroller({ enabled: true, pannable: true }));
      }
      if (keyboard) {
        graph.use(new Keyboard({ enabled: true, global: false }));
      }
      graph.use(new History({ enabled: true }));

      // ── Event bindings ──
      graph.on("node:change:position", notifyChange);
      graph.on("node:added", notifyChange);
      graph.on("node:removed", notifyChange);
      graph.on("edge:connected", notifyChange);
      graph.on("edge:added", notifyChange);
      graph.on("edge:removed", notifyChange);
      graph.on("edge:change:attrs", notifyChange);
      graph.on("cell:change:data", notifyChange);

      // Keyboard: Delete key removes selected cells
      graph.bindKey("delete", () => {
        const cells = graph.getSelectedCells();
        if (cells.length > 0) {
          graph.removeCells(cells);
        }
      });
      graph.bindKey("backspace", () => {
        const cells = graph.getSelectedCells();
        if (cells.length > 0) {
          graph.removeCells(cells);
        }
      });

      // Allow custom shapes registration
      if (registerCustomShapes) {
        registerCustomShapes(graph);
      }

      graphRef.current = graph;

      // Load initial schema if provided
      if (initialSchema) {
        importSchemaIntoGraph(graph, initialSchema);
        schemaRef.current = { ...initialSchema };
      }

      // Center content after layout
      setTimeout(() => {
        if (graphRef.current) {
          graphRef.current.centerContent();
        }
      }, 100);

      if (onGraphReady) {
        onGraphReady(graph);
      }

      return () => {
        if (changeTimeoutRef.current) clearTimeout(changeTimeoutRef.current);
        graph.dispose();
        graphRef.current = null;
      };
    }, []);

    // ── Imperative handle ──
    useImperativeHandle(
      ref,
      () => ({
        exportSchema(): CanvasSchema {
          return exportCurrentSchema();
        },
        importSchema(schema: CanvasSchema): void {
          const g = graphRef.current;
          if (!g) return;
          importSchemaIntoGraph(g, schema);
          schemaRef.current = { ...schema };
          g.centerContent();
        },
        getGraph(): Graph {
          return graphRef.current!;
        },
        fitToContent(padding = 40): void {
          const g = graphRef.current;
          if (!g) return;
          g.zoomToFit({ padding });
        },
        async applyAgentActions(actions): Promise<void> {
          const g = graphRef.current;
          if (!g) return;
          for (const action of actions) {
            await applyAgentAction(g, action, canvasType);
          }
          notifyChange();
        },
      }),
      [exportCurrentSchema, notifyChange, canvasType],
    );

    return (
      <div ref={containerRef} className={className} style={{ width: "100%", height: "100%" }} />
    );
  },
);

// ── Helper: import schema into existing graph ──
function importSchemaIntoGraph(graph: Graph, schema: CanvasSchema): void {
  graph.resetCells([]);

  const nodes: NodeMetadata[] = schema.nodes.map(schemaNodeToX6);
  const edges: EdgeMetadata[] = schema.edges.map(schemaEdgeToX6);

  if (nodes.length > 0) graph.addNodes(nodes);
  if (edges.length > 0) graph.addEdges(edges);
}

// ── Helper: apply single agent action ──
async function applyAgentAction(
  graph: Graph,
  action: { type: string; [key: string]: unknown },
  _canvasType: string,
): Promise<void> {
  switch (action.type) {
    case "add_node": {
      const nodeData = (action.node ?? {}) as Record<string, unknown>;
      graph.addNode({
        id: nodeData.id as string | undefined,
        x: (action.position as { x: number; y: number })?.x ?? 200,
        y: (action.position as { x: number; y: number })?.y ?? 200,
        shape: "custom-html",
        data: nodeData,
      });
      break;
    }
    case "connect_nodes": {
      graph.addEdge({
        source: { cell: action.sourceId as string },
        target: { cell: action.targetId as string },
        data: {
          label: (action.label as string) ?? "",
          relation: (action.relation as string) ?? "related_to",
        },
        labels: action.label
          ? [{ position: 0.5, attrs: { text: { text: action.label as string } } }]
          : [],
      });
      break;
    }
    case "modify_node_text": {
      const n = graph.getCellById(action.nodeId as string);
      if (n && n.isNode()) {
        n.setData({ ...n.getData(), label: action.newLabel }, { overwrite: true });
      }
      break;
    }
    case "delete_node": {
      graph.removeCell(action.nodeId as string);
      break;
    }
    case "delete_edge": {
      graph.removeCell(action.edgeId as string);
      break;
    }
    case "add_child_node": {
      const parent = graph.getCellById(action.parentId as string);
      if (parent && parent.isNode()) {
        const childId = (action.topic as string) ?? `child_${Date.now()}`;
        const pos = parent.position();
        graph.addNode({
          id: childId,
          x: pos.x + 220,
          y: pos.y,
          shape: "custom-html",
          data: { id: childId, topic: action.topic ?? "新节点", parentId: action.parentId },
        });
        graph.addEdge({
          source: { cell: action.parentId as string },
          target: { cell: childId },
          attrs: { line: { stroke: "#94a3b8", strokeWidth: 1.5 } },
        });
      }
      break;
    }
    case "reparent_subtree": {
      const edges = graph.getEdges();
      const oldEdge = edges.find((e) => e.getTargetCellId() === action.nodeId);
      if (oldEdge) graph.removeCell(oldEdge);
      graph.addEdge({
        source: { cell: action.newParentId as string },
        target: { cell: action.nodeId as string },
        attrs: { line: { stroke: "#94a3b8", strokeWidth: 1.5 } },
      });
      break;
    }
    default:
      console.warn(`[KnowledgeCanvas] Unknown agent action type: ${action.type}`);
  }
  await new Promise((r) => setTimeout(r, 150));
}

export default KnowledgeCanvas;
