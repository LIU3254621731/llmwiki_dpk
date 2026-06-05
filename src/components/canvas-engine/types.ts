// ── Canvas Engine: universal type definitions ──
// These types drive both Macro (relationship network) and Micro (mindmap) canvases.

import type { NodeMetadata, EdgeMetadata, Node, Edge } from "@antv/x6";

// ── Mindmap tree node ──
export interface MindmapNode {
  id: string;
  topic: string;
  children?: MindmapNode[];
  x?: number;
  y?: number;
  widthHint?: number;
}

// ── Macro canvas node data ──
export interface MacroNodeData {
  id: string;
  label: string;
  type: string;
  tags?: string[];
  summary?: string;
  hasMicroMap?: boolean;
  microCanvasId?: string;
}

// ── Macro canvas edge data ──
export interface MacroEdgeData {
  id: string;
  source: string;
  target: string;
  label?: string;
  relation: string;
  arrowType: "none" | "single" | "double";
}

// ── Unified canvas schema (export/import) ──
export interface CanvasSchema {
  version: 1;
  canvasType: "macro" | "micro";
  canvasId: string;
  nodes: CanvasSchemaNode[];
  edges: CanvasSchemaEdge[];
  metadata: Record<string, unknown>;
}

export interface CanvasSchemaNode {
  id: string;
  type: string;
  x: number;
  y: number;
  width?: number;
  height?: number;
  data: Record<string, unknown>;
}

export interface CanvasSchemaEdge {
  id: string;
  source: string;
  target: string;
  sourcePort?: string;
  targetPort?: string;
  router?: string;
  connector?: string;
  data: Record<string, unknown>;
}

// ── Agent action interface ──
export type AgentAction =
  | { type: "add_node"; node: MacroNodeData; position?: { x: number; y: number } }
  | { type: "connect_nodes"; sourceId: string; targetId: string; relation: string; label?: string }
  | { type: "modify_node_text"; nodeId: string; newLabel: string }
  | { type: "delete_node"; nodeId: string }
  | { type: "delete_edge"; edgeId: string }
  | { type: "add_child_node"; parentId: string; topic: string }
  | { type: "reparent_subtree"; nodeId: string; newParentId: string };

// ── Canvas persisted state ──
export interface CanvasPersistState {
  canvasType: "macro" | "micro";
  canvasId: string;
  kbId: string;
  schema: CanvasSchema;
  lastModified: string;
}

// ── X6 conversion helpers ──

export function schemaNodeToX6(sn: CanvasSchemaNode): NodeMetadata {
  return {
    id: sn.id,
    x: sn.x,
    y: sn.y,
    width: sn.width,
    height: sn.height,
    shape: sn.type,
    data: sn.data,
  };
}

export function schemaEdgeToX6(se: CanvasSchemaEdge): EdgeMetadata {
  const source: EdgeMetadata["source"] = se.sourcePort
    ? { cell: se.source, port: se.sourcePort }
    : { cell: se.source };
  const target: EdgeMetadata["target"] = se.targetPort
    ? { cell: se.target, port: se.targetPort }
    : { cell: se.target };

  const meta: EdgeMetadata = {
    id: se.id,
    source,
    target,
    data: se.data,
  };

  if (se.router) {
    (meta as any).router = se.router;
  }
  if (se.connector) {
    (meta as any).connector = se.connector;
  }

  return meta;
}

export function x6NodeToSchema(node: Node): CanvasSchemaNode {
  const pos = node.position();
  const size = node.size();
  return {
    id: node.id,
    type: node.shape ?? "default",
    x: pos.x,
    y: pos.y,
    width: size.width,
    height: size.height,
    data: node.getData(),
  };
}

export function x6EdgeToSchema(edge: Edge): CanvasSchemaEdge {
  const src = edge.getSourceCellId();
  const tgt = edge.getTargetCellId();
  return {
    id: edge.id,
    source: src ?? "",
    target: tgt ?? "",
    data: edge.getData(),
  };
}
