import { useMemo, useCallback } from "react";
import ReactFlow, {
  type Node,
  type Edge,
  MiniMap,
  Controls,
  Background,
  MarkerType,
  ReactFlowProvider,
} from "reactflow";
import "reactflow/dist/style.css";
import dagre from "dagre";
import type { GraphNode, GraphEdge } from "@/types/graph";
import HierarchicalNode from "./HierarchicalNode";

// ── Color palette (for MiniMap node coloring) ──
const NODE_COLORS: Record<string, string> = {
  entity:  "#34d399", concept: "#fbbf24", topic:   "#a78bfa",
  person:  "#22d3ee", source:   "#fb923c", question:"#fda4af",
  wikipage:"#818cf8", review:   "#c084fc", dataset: "#6ee7b7",
  method:  "#a78bfa",
  default: "#cbd5e1",
};

const nodeTypes = { hierarchical: HierarchicalNode };

interface MindMapViewProps {
  nodes: GraphNode[];
  edges?: GraphEdge[];
  kbName: string;
  onNodeClick: (node: GraphNode) => void;
  zoomLevel?: number;
  direction?: "TB" | "LR";
}

export default function MindMapView({
  nodes,
  edges = [],
  onNodeClick,
  direction = "TB",
}: MindMapViewProps) {
  // Convert domain data → ReactFlow nodes/edges
  const { rfNodes, rfEdges } = useMemo(() => {
    const ns: Node[] = nodes.map((n) => ({
      id: n.id,
      type: "hierarchical",
      data: {
        label: n.label,
        nodeType: n.type,
        path: n.path,
        summary: n.summary,
        status: n.status,
        aliases: n.aliases,
        tags: n.tags,
        degree: n.inDegree + n.outDegree,
        rawNode: n,
      },
      position: { x: 0, y: 0 },
    }));

    const es: Edge[] = edges.map((e) => ({
      id: e.id,
      source: e.source,
      target: e.target,
      type: "smoothstep",
      animated: e.confidence === "high",
      data: { rawEdge: e },
      markerEnd: {
        type: MarkerType.ArrowClosed,
        width: 16,
        height: 16,
        color: e.relation === "contradicts" ? "#ef4444" : "#94a3b8",
      },
      style: {
        stroke: e.relation === "contradicts" ? "#ef4444" : "#94a3b8",
        strokeWidth: e.confidence === "high" ? 2 : 1,
      },
    }));

    return { rfNodes: ns, rfEdges: es };
  }, [nodes, edges]);

  // dagre hierarchical layout
  const layoutedNodes = useMemo(() => {
    if (rfNodes.length === 0) return [];

    const g = new dagre.graphlib.Graph();
    g.setDefaultEdgeLabel(() => ({}));
    g.setGraph({
      rankdir: direction,
      nodesep: 60,
      ranksep: 100,
      marginx: 50,
      marginy: 50,
    });

    rfNodes.forEach((n) => {
      g.setNode(n.id, { width: 150, height: 68 });
    });
    rfEdges.forEach((e) => {
      if (rfNodes.find((n) => n.id === e.source) && rfNodes.find((n) => n.id === e.target)) {
        g.setEdge(e.source, e.target);
      }
    });

    dagre.layout(g);

    return rfNodes.map((n) => {
      const pos = g.node(n.id);
      return {
        ...n,
        position: pos
          ? { x: pos.x - 75, y: pos.y - 34 }  // center the node on its dagre anchor
          : { x: 0, y: 0 },
      };
    });
  }, [rfNodes, rfEdges, direction]);

  const handleNodeClick = useCallback(
    (_event: React.MouseEvent, node: Node) => {
      const raw = node.data.rawNode as GraphNode;
      if (raw) onNodeClick(raw);
    },
    [onNodeClick],
  );

  return (
    <div style={{ width: "100%", height: "100%", background: "#f8fafc" }}>
      <ReactFlowProvider>
        <ReactFlow
          nodes={layoutedNodes}
          edges={rfEdges}
          onNodeClick={handleNodeClick}
          nodeTypes={nodeTypes}
          fitView
          fitViewOptions={{ padding: 0.2 }}
          minZoom={0.1}
          maxZoom={3}
          attributionPosition="bottom-right"
          defaultEdgeOptions={{
            type: "smoothstep",
          }}
        >
          <Background color="#e2e8f0" gap={20} size={1} />
          <Controls showInteractive={false} />
          <MiniMap
            nodeColor={(n) =>
              NODE_COLORS[n.data?.nodeType] || NODE_COLORS.default
            }
            style={{ width: 160, height: 100 }}
            maskColor="rgba(0,0,0,0.08)"
            pannable
            zoomable
          />
        </ReactFlow>
      </ReactFlowProvider>
    </div>
  );
}
