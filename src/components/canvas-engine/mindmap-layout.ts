// ── Mindmap layout algorithm (pure function, framework-agnostic) ──
// Computes absolute (x, y) positions for a tree using:
//   X = depth × (NODE_WIDTH + HORIZONTAL_GAP)
//   Y_parent = (Y_firstChild + Y_lastChild) / 2

import type { MindmapNode } from "./types";

export interface LayoutOptions {
  /** Horizontal spacing between depth levels. Default 240. */
  horizontalGap?: number;
  /** Vertical spacing between sibling leaf nodes. Default 80. */
  verticalGap?: number;
  /** Approximate node width for spacing. Default 160. */
  nodeWidth?: number;
  /** X offset for the root node. Default 0. */
  rootX?: number;
  /** Y offset for the root node subtree. Default 0. */
  rootY?: number;
}

export interface LayoutResult {
  nodes: Array<{
    id: string;
    topic: string;
    x: number;
    y: number;
    depth: number;
    children?: MindmapNode[];
  }>;
}

/**
 * Pure function: compute a left-to-right (horizontal) mindmap layout.
 * Root at x=0, children extend rightward. Sibling nodes centered vertically.
 *
 * Returns a flat list of positioned nodes ready for X6 consumption.
 */
export function computeMindmapLayout(
  root: MindmapNode,
  options: LayoutOptions = {},
): LayoutResult {
  const {
    horizontalGap = 240,
    verticalGap = 80,
    nodeWidth = 160,
    rootX = 0,
    rootY = 0,
  } = options;

  const result: LayoutResult["nodes"] = [];
  let leafY = rootY;

  // ── First pass: post-order traversal to assign Y positions ──
  function assignY(node: MindmapNode): void {
    if (!node.children || node.children.length === 0) {
      // Leaf: place at next available Y slot
      node.y = leafY;
      leafY += verticalGap;
    } else {
      // Recurse children first (post-order)
      for (const child of node.children) {
        assignY(child);
      }
      // Parent Y = midpoint of first and last child
      const firstY = node.children[0].y!;
      const lastY = node.children[node.children.length - 1].y!;
      node.y = (firstY + lastY) / 2;
    }
  }

  assignY(root);

  // ── Second pass: pre-order traversal to assign X and collect nodes ──
  function assignX(node: MindmapNode, depth: number): void {
    node.x = rootX + depth * (nodeWidth + horizontalGap);

    result.push({
      id: node.id,
      topic: node.topic,
      x: node.x,
      y: node.y!,
      depth,
      children: node.children,
    });

    if (node.children) {
      for (const child of node.children) {
        assignX(child, depth + 1);
      }
    }
  }

  assignX(root, 0);

  return { nodes: result };
}

/**
 * Inverse: given a flat list of positioned nodes, reconstruct the tree structure.
 * Useful for serialization after user drag-reparents a subtree.
 */
export function nodesToTree(flatNodes: LayoutResult["nodes"]): MindmapNode {
  const map = new Map<string, MindmapNode>();
  for (const n of flatNodes) {
    map.set(n.id, {
      id: n.id,
      topic: n.topic,
      x: n.x,
      y: n.y,
      children: [],
    });
  }

  // Build parent-child relationships from topological order
  // This works because nodes are listed depth-first in layout output
  const root = map.get(flatNodes[0].id)!;
  const stack: { node: MindmapNode; depth: number; idx: number }[] = [
    { node: root, depth: 0, idx: 0 },
  ];

  for (let i = 1; i < flatNodes.length; i++) {
    const curr = map.get(flatNodes[i].id)!;
    const currDepth = flatNodes[i].depth;

    // Find parent: the last node in stack with depth === currDepth - 1
    while (stack.length > 0 && stack[stack.length - 1].depth >= currDepth) {
      stack.pop();
    }

    if (stack.length > 0) {
      const parent = stack[stack.length - 1].node;
      if (!parent.children) parent.children = [];
      parent.children.push(curr);
    }

    stack.push({ node: curr, depth: currDepth, idx: i });
  }

  return root;
}
