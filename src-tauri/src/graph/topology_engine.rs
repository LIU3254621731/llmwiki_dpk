use std::collections::{HashMap, VecDeque};
use std::fs;
use std::path::Path;

use petgraph::graph::{DiGraph, NodeIndex};
use regex::Regex;
use serde::Serialize;
use walkdir::WalkDir;

#[derive(Debug, Clone, Serialize)]
pub struct TopologyLayout {
    pub nodes: Vec<LayoutNode>,
    pub edges: Vec<LayoutEdge>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LayoutNode {
    pub id: String,
    pub label: String,
    #[serde(rename = "filePath")]
    pub file_path: String,
    pub level: usize,
    pub x: f64,
    pub y: f64,
    #[serde(rename = "inDegree")]
    pub in_degree: usize,
    #[serde(rename = "outDegree")]
    pub out_degree: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct LayoutEdge {
    pub source: String,
    pub target: String,
    pub label: String,
}

const RANK_SEP: f64 = 200.0;
const NODE_SEP: f64 = 80.0;
const MAX_CROSSING_ITERS: usize = 5;

pub struct TopologyEngine;

impl TopologyEngine {
    /// Extract `[[wikilink]]` targets from markdown content.
    pub fn extract_wikilinks(content: &str) -> Vec<String> {
        let re = Regex::new(r"\[\[([^\]\n]+?)\]\]").unwrap();
        re.captures_iter(content)
            .map(|cap| cap[1].to_string())
            .collect()
    }

    /// Scan a directory for `.md` files and compute the full topology layout.
    pub fn compute_layout(scan_root: &Path) -> Result<TopologyLayout, String> {
        let graph = Self::build_wikilink_graph(scan_root)?;
        let layers = Self::topological_layers(&graph);
        let optimized_layers = Self::minimize_crossings(&graph, layers);
        let nodes = Self::assign_positions(&graph, &optimized_layers);

        let edges: Vec<LayoutEdge> = graph
            .raw_edges()
            .iter()
            .map(|e| {
                let source = graph[e.source()].clone();
                let target = graph[e.target()].clone();
                let source_node = nodes.iter().find(|n| n.id == source).unwrap();
                let target_node = nodes.iter().find(|n| n.id == target).unwrap();
                LayoutEdge {
                    source,
                    target,
                    label: format!("{} → {}", source_node.label, target_node.label),
                }
            })
            .collect();

        Ok(TopologyLayout { nodes, edges })
    }

    /// Build a petgraph DiGraph from `.md` files in a directory.
    /// Nodes = files (identified by relative path), Edges = wikilinks from one file to another.
    pub fn build_wikilink_graph(
        scan_root: &Path,
    ) -> Result<DiGraph<String, ()>, String> {
        let mut graph = DiGraph::<String, ()>::new();
        // Map from filename stem to NodeIndex for matching wikilinks
        let mut stem_to_node: HashMap<String, NodeIndex> = HashMap::new();
        let mut path_to_node: HashMap<String, NodeIndex> = HashMap::new();

        if !scan_root.exists() {
            return Ok(graph);
        }

        // First pass: collect all md files and create nodes
        let mut md_files: Vec<(String, String)> = Vec::new(); // (relative_path, content)

        for entry in WalkDir::new(scan_root)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            if path.extension().map(|e| e != "md").unwrap_or(true) {
                continue;
            }

            let relative_path = path
                .strip_prefix(scan_root)
                .map(|p| p.to_string_lossy().replace('\\', "/"))
                .unwrap_or_else(|_| path.to_string_lossy().replace('\\', "/"));

            let content = fs::read_to_string(path).unwrap_or_default();
            md_files.push((relative_path, content));
        }

        // Sort for deterministic output
        md_files.sort_by(|a, b| a.0.cmp(&b.0));

        for (relative_path, _content) in &md_files {
            let stem = Path::new(relative_path)
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| relative_path.clone());

            let node_id = graph.add_node(relative_path.clone());

            // Match by file stem (e.g., "Obsidian" matches [[Obsidian]])
            stem_to_node.entry(stem.to_lowercase()).or_insert(node_id);
            // Also match by full path for explicit references
            path_to_node.entry(relative_path.clone()).or_insert(node_id);
            // Match by path without extension
            let without_ext = relative_path
                .strip_suffix(".md")
                .unwrap_or(relative_path);
            path_to_node
                .entry(without_ext.to_string())
                .or_insert(node_id);
        }

        // Second pass: extract wikilinks and create edges
        for (source_path, content) in &md_files {
            let source_stem = Path::new(source_path)
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| source_path.clone());

            let source_idx = path_to_node
                .get(source_path.as_str())
                .or_else(|| stem_to_node.get(&source_stem.to_lowercase()))
                .copied();

            let Some(source_idx) = source_idx else { continue };

            let links = Self::extract_wikilinks(content);
            for link in &links {
                // Try to match the link target
                let target_idx = path_to_node
                    .get(link)
                    .or_else(|| path_to_node.get(&format!("{}.md", link)))
                    .or_else(|| stem_to_node.get(&link.to_lowercase()))
                    .copied();

                if let Some(target_idx) = target_idx {
                    if source_idx != target_idx {
                        // Avoid duplicate edges
                        if !graph.contains_edge(source_idx, target_idx) {
                            graph.add_edge(source_idx, target_idx, ());
                        }
                    }
                }
            }
        }

        Ok(graph)
    }

    /// Assign nodes to topological layers using longest-path algorithm.
    /// Returns layers where each layer is a Vec of NodeIndex.
    pub fn topological_layers(
        graph: &DiGraph<String, ()>,
    ) -> Vec<Vec<NodeIndex>> {
        let node_count = graph.node_count();
        if node_count == 0 {
            return Vec::new();
        }

        let mut in_degree: HashMap<NodeIndex, usize> = HashMap::new();
        let mut layer: HashMap<NodeIndex, usize> = HashMap::new();
        let mut queue: VecDeque<NodeIndex> = VecDeque::new();

        // Compute in-degree for all nodes
        for node in graph.node_indices() {
            let deg = graph
                .neighbors_directed(node, petgraph::Direction::Incoming)
                .count();
            in_degree.insert(node, deg);
            if deg == 0 {
                queue.push_back(node);
                layer.insert(node, 0);
            }
        }

        // Topological sort with BFS assigning layers
        let mut max_layer = 0;
        while let Some(node) = queue.pop_front() {
            let current_layer = *layer.get(&node).unwrap_or(&0);

            for neighbor in graph.neighbors_directed(node, petgraph::Direction::Outgoing) {
                let entry = layer.entry(neighbor).or_insert(0);
                *entry = (*entry).max(current_layer + 1);
                max_layer = max_layer.max(*entry);

                // Decrement in-degree and enqueue when ready
                let deg = in_degree.entry(neighbor).or_insert(0);
                if *deg > 0 {
                    *deg -= 1;
                }
                if *deg == 0 {
                    queue.push_back(neighbor);
                }
            }
        }

        // Handle remaining nodes (those in cycles)
        for node in graph.node_indices() {
            if !layer.contains_key(&node) {
                // Assign to a new layer based on any connected node
                let mut max_pred_layer = 0;
                for pred in
                    graph.neighbors_directed(node, petgraph::Direction::Incoming)
                {
                    max_pred_layer =
                        max_pred_layer.max(*layer.get(&pred).unwrap_or(&0));
                }
                layer.insert(node, max_pred_layer + 1);
                max_layer = max_layer.max(max_pred_layer + 1);
            }
        }

        // Build layer groups
        let mut layers: Vec<Vec<NodeIndex>> = vec![Vec::new(); max_layer + 1];
        for (node, l) in &layer {
            layers[*l].push(*node);
        }

        // Remove empty trailing layers
        while layers.last().map(|l| l.is_empty()).unwrap_or(false) {
            layers.pop();
        }

        layers
    }

    /// Minimize edge crossings between layers using the barycenter heuristic.
    pub fn minimize_crossings(
        graph: &DiGraph<String, ()>,
        mut layers: Vec<Vec<NodeIndex>>,
    ) -> Vec<Vec<NodeIndex>> {
        if layers.len() < 2 {
            return layers;
        }

        for _iter in 0..MAX_CROSSING_ITERS {
            // Forward pass: top to bottom
            for i in 1..layers.len() {
                let (upper, lower) = layers.split_at_mut(i);
                let upper_layer = &upper[i - 1];
                let lower_layer = &mut lower[0];

                // Compute barycenter for each node in lower layer
                let mut barycenters: Vec<(NodeIndex, f64)> = lower_layer
                    .iter()
                    .map(|&node| {
                        let predecessors: Vec<usize> = graph
                            .neighbors_directed(node, petgraph::Direction::Incoming)
                            .filter_map(|pred| {
                                upper_layer.iter().position(|&n| n == pred)
                            })
                            .collect();
                        let bc = if predecessors.is_empty() {
                            f64::MAX // nodes with no predecessors go to the end
                        } else {
                            predecessors.iter().sum::<usize>() as f64
                                / predecessors.len() as f64
                        };
                        (node, bc)
                    })
                    .collect();

                barycenters.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
                *lower_layer = barycenters.into_iter().map(|(n, _)| n).collect();
            }

            // Backward pass: bottom to top
            for i in (0..layers.len() - 1).rev() {
                let (upper, lower) = layers.split_at_mut(i + 1);
                let upper_layer = &mut upper[i];
                let lower_layer = &lower[0];

                let mut barycenters: Vec<(NodeIndex, f64)> = upper_layer
                    .iter()
                    .map(|&node| {
                        let successors: Vec<usize> = graph
                            .neighbors_directed(node, petgraph::Direction::Outgoing)
                            .filter_map(|succ| {
                                lower_layer.iter().position(|&n| n == succ)
                            })
                            .collect();
                        let bc = if successors.is_empty() {
                            f64::MAX
                        } else {
                            successors.iter().sum::<usize>() as f64
                                / successors.len() as f64
                        };
                        (node, bc)
                    })
                    .collect();

                barycenters.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
                *upper_layer = barycenters.into_iter().map(|(n, _)| n).collect();
            }
        }

        layers
    }

    /// Assign (x, y) coordinates to nodes based on their layer and position.
    pub fn assign_positions(
        graph: &DiGraph<String, ()>,
        layers: &[Vec<NodeIndex>],
    ) -> Vec<LayoutNode> {
        let mut nodes = Vec::new();

        for (level, layer_nodes) in layers.iter().enumerate() {
            let layer_width = layer_nodes.len();
            // Center nodes vertically within the layer
            let total_height = (layer_width as f64 - 1.0) * NODE_SEP;
            let start_y = -total_height / 2.0;

            for (pos, &node_idx) in layer_nodes.iter().enumerate() {
                let label = graph[node_idx].clone();
                let file_path = label.clone();
                let id = label.clone();

                let display_label = Path::new(&label)
                    .file_stem()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_else(|| label.clone());

                let in_degree = graph
                    .neighbors_directed(node_idx, petgraph::Direction::Incoming)
                    .count();
                let out_degree = graph
                    .neighbors_directed(node_idx, petgraph::Direction::Outgoing)
                    .count();

                nodes.push(LayoutNode {
                    id,
                    label: display_label,
                    file_path,
                    level,
                    x: level as f64 * RANK_SEP,
                    y: start_y + pos as f64 * NODE_SEP,
                    in_degree,
                    out_degree,
                });
            }
        }

        nodes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_wikilinks_single() {
        let content = "See [[Other Page]] for details.";
        let links = TopologyEngine::extract_wikilinks(content);
        assert_eq!(links, vec!["Other Page"]);
    }

    #[test]
    fn test_extract_wikilinks_multiple() {
        let content = "[[Page A]] and [[Topic/Sub Page]] and [[Page C]]";
        let links = TopologyEngine::extract_wikilinks(content);
        assert_eq!(links.len(), 3);
        assert!(links.contains(&"Page A".to_string()));
        assert!(links.contains(&"Topic/Sub Page".to_string()));
        assert!(links.contains(&"Page C".to_string()));
    }

    #[test]
    fn test_extract_wikilinks_none() {
        let content = "No wikilinks here, just plain text.";
        let links = TopologyEngine::extract_wikilinks(content);
        assert!(links.is_empty());
    }

    #[test]
    fn test_extract_wikilinks_ignores_closed_before_newline() {
        // [[ should not match across lines
        let content = "[[valid link]]\nSome text\nNot a [[\nmultiline\n]] match";
        let links = TopologyEngine::extract_wikilinks(content);
        // The regex [[([^\]\n]+?)]] matches [[valid link]] but not the multiline case
        assert_eq!(links.len(), 1);
        assert_eq!(links[0], "valid link");
    }

    #[test]
    fn test_build_wikilink_graph_linear_chain() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();

        std::fs::write(root.join("a.md"), "[[b]]").unwrap();
        std::fs::write(root.join("b.md"), "[[c]]").unwrap();
        std::fs::write(root.join("c.md"), "").unwrap();

        let graph = TopologyEngine::build_wikilink_graph(root).unwrap();
        assert_eq!(graph.node_count(), 3);
        assert_eq!(graph.edge_count(), 2);
    }

    #[test]
    fn test_build_wikilink_graph_no_self_loops() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();

        std::fs::write(root.join("a.md"), "[[a]]").unwrap();

        let graph = TopologyEngine::build_wikilink_graph(root).unwrap();
        assert_eq!(graph.node_count(), 1);
        assert_eq!(graph.edge_count(), 0);
    }

    #[test]
    fn test_topological_layers_linear_chain() {
        let mut graph = DiGraph::new();
        let a = graph.add_node("a".to_string());
        let b = graph.add_node("b".to_string());
        let c = graph.add_node("c".to_string());
        graph.add_edge(a, b, ());
        graph.add_edge(b, c, ());

        let layers = TopologyEngine::topological_layers(&graph);
        assert_eq!(layers.len(), 3);
        assert_eq!(layers[0], vec![a]);
        assert_eq!(layers[1], vec![b]);
        assert_eq!(layers[2], vec![c]);
    }

    #[test]
    fn test_topological_layers_fork() {
        // a -> b, a -> c  (b and c should be in the same layer)
        let mut graph = DiGraph::new();
        let a = graph.add_node("a".to_string());
        let b = graph.add_node("b".to_string());
        let c = graph.add_node("c".to_string());
        graph.add_edge(a, b, ());
        graph.add_edge(a, c, ());

        let layers = TopologyEngine::topological_layers(&graph);
        assert_eq!(layers.len(), 2);
        assert_eq!(layers[0], vec![a]);
        assert_eq!(layers[1].len(), 2);
        assert!(layers[1].contains(&b));
        assert!(layers[1].contains(&c));
    }

    #[test]
    fn test_topological_layers_handles_cycle() {
        // a -> b -> c -> a (cycle)
        let mut graph = DiGraph::new();
        let a = graph.add_node("a".to_string());
        let b = graph.add_node("b".to_string());
        let c = graph.add_node("c".to_string());
        graph.add_edge(a, b, ());
        graph.add_edge(b, c, ());
        graph.add_edge(c, a, ());

        let layers = TopologyEngine::topological_layers(&graph);
        // Should produce layers without infinite loop
        assert!(!layers.is_empty());
        let total_nodes: usize = layers.iter().map(|l| l.len()).sum();
        assert_eq!(total_nodes, 3);
    }

    #[test]
    fn test_assign_positions() {
        let mut graph = DiGraph::new();
        let a = graph.add_node("a.md".to_string());
        let b = graph.add_node("b.md".to_string());
        graph.add_edge(a, b, ());

        let layers = vec![vec![a], vec![b]];
        let nodes = TopologyEngine::assign_positions(&graph, &layers);

        assert_eq!(nodes.len(), 2);
        assert_eq!(nodes[0].level, 0);
        assert_eq!(nodes[1].level, 1);
        // x should increase with level
        assert!(nodes[1].x > nodes[0].x);
    }

    #[test]
    fn test_compute_layout_integration() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();

        std::fs::create_dir_all(root.join("wiki")).unwrap();
        std::fs::write(root.join("wiki/a.md"), "# A\n\nSee [[b]] and [[c]]").unwrap();
        std::fs::write(root.join("wiki/b.md"), "# B\n\nRelated to [[c]]").unwrap();
        std::fs::write(root.join("wiki/c.md"), "# C\n\nRoot concept").unwrap();

        let layout = TopologyEngine::compute_layout(root.join("wiki").as_path()).unwrap();

        assert_eq!(layout.nodes.len(), 3);
        assert!(layout.edges.len() >= 2);

        // All nodes should have x and y coordinates
        for node in &layout.nodes {
            assert!(node.x >= 0.0);
            assert!(node.label.len() > 0);
        }
    }
}
