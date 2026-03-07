use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};

/// A graph of subject-predicate-object relationships.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct KnowledgeGraph {
    /// All node labels.
    pub nodes: Vec<String>,
    /// Edges as (subject, predicate, object) triples.
    pub edges: Vec<(String, String, String)>,
}

impl KnowledgeGraph {
    /// Create an empty KnowledgeGraph.
    pub fn new() -> Self {
        Self::default()
    }

    /// Build a graph by BFS from a starting node.
    ///
    /// # Arguments
    /// * `adjacency` — subject → list of (predicate, object) pairs.
    /// * `start` — Starting node.
    /// * `depth` — Maximum traversal depth.
    ///
    /// # Panics
    /// Never panics.
    pub fn from_bfs(
        adjacency: &HashMap<String, Vec<(String, String)>>,
        start: &str,
        depth: usize,
    ) -> Self {
        let mut visited = HashSet::new();
        let mut nodes = Vec::new();
        let mut edges = Vec::new();
        let mut queue: VecDeque<(String, usize)> = VecDeque::new();
        visited.insert(start.to_string());
        nodes.push(start.to_string());
        queue.push_back((start.to_string(), 0));

        while let Some((node, d)) = queue.pop_front() {
            if d >= depth {
                continue;
            }
            if let Some(neighbors) = adjacency.get(&node) {
                for (pred, obj) in neighbors {
                    edges.push((node.clone(), pred.clone(), obj.clone()));
                    if !visited.contains(obj) {
                        visited.insert(obj.clone());
                        nodes.push(obj.clone());
                        queue.push_back((obj.clone(), d + 1));
                    }
                }
            }
        }
        Self { nodes, edges }
    }
}
