export interface GraphNode {
  id: string;
  type: string;
  label: string;
  path: string;
  aliases: string[];
  tags: string[];
  summary: string;
  sourceCount: number;
  inDegree: number;
  outDegree: number;
  status: string;
  createdAt: string;
}

export interface GraphEdge {
  id: string;
  source: string;
  target: string;
  type: string;
  relation: string;
  confidence: string;
  evidenceSourceId: string;
  evidenceLocation: string;
  citationStatus: string;
  createdByTask: string;
}

export interface GraphHealth {
  nodeCount: number;
  edgeCount: number;
  orphanCount: number;
  lowConfidenceCount: number;
  conflictCount: number;
  needsReviewCount: number;
  uncitedCount: number;
  avgDegree: number;
  maxHubLabel: string;
  maxHubDegree: number;
}

export interface GraphData {
  nodes: GraphNode[];
  edges: GraphEdge[];
  health: GraphHealth;
}

export interface LayoutNode {
  id: string;
  label: string;
  filePath: string;
  level: number;
  x: number;
  y: number;
  inDegree: number;
  outDegree: number;
}

export interface LayoutEdge {
  source: string;
  target: string;
  label: string;
}

export interface TopologyLayout {
  nodes: LayoutNode[];
  edges: LayoutEdge[];
}
