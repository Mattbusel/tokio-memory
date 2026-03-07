use async_trait::async_trait;
use dashmap::DashMap;
use std::collections::HashMap;
use std::sync::Arc;
use crate::error::MemoryError;
use crate::types::MemoryId;
use super::fact::Fact;
use super::knowledge_graph::KnowledgeGraph;

/// Trait for semantic memory storage.
#[async_trait]
pub trait SemanticStore: Send + Sync {
    /// Assert a fact into semantic memory.
    ///
    /// # Panics
    /// Never panics.
    async fn assert_fact(&self, fact: Fact) -> Result<MemoryId, MemoryError>;

    /// Retract (remove) a fact by ID.
    ///
    /// # Panics
    /// Never panics.
    async fn retract_fact(&self, id: &MemoryId) -> Result<(), MemoryError>;

    /// Query all facts about a subject.
    ///
    /// # Panics
    /// Never panics.
    async fn query_facts(&self, subject: &str) -> Result<Vec<Fact>, MemoryError>;

    /// Traverse the knowledge graph from a node.
    ///
    /// # Panics
    /// Never panics.
    async fn traverse(&self, from: &str, depth: usize) -> Result<KnowledgeGraph, MemoryError>;
}

/// In-memory SemanticStore backed by DashMap.
pub struct InMemorySemanticStore {
    facts: Arc<DashMap<MemoryId, Fact>>,
}

impl InMemorySemanticStore {
    /// Create a new empty store.
    pub fn new() -> Self {
        Self { facts: Arc::new(DashMap::new()) }
    }
}

impl Default for InMemorySemanticStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SemanticStore for InMemorySemanticStore {
    async fn assert_fact(&self, fact: Fact) -> Result<MemoryId, MemoryError> {
        let id = fact.id.clone();
        self.facts.insert(id.clone(), fact);
        Ok(id)
    }

    async fn retract_fact(&self, id: &MemoryId) -> Result<(), MemoryError> {
        self.facts
            .remove(id)
            .map(|_| ())
            .ok_or_else(|| MemoryError::NotFound(id.clone()))
    }

    async fn query_facts(&self, subject: &str) -> Result<Vec<Fact>, MemoryError> {
        Ok(self
            .facts
            .iter()
            .filter(|f| f.value().subject == subject)
            .map(|f| f.value().clone())
            .collect())
    }

    async fn traverse(&self, from: &str, depth: usize) -> Result<KnowledgeGraph, MemoryError> {
        let mut adjacency: HashMap<String, Vec<(String, String)>> = HashMap::new();
        for f in self.facts.iter() {
            let fact = f.value();
            adjacency
                .entry(fact.subject.clone())
                .or_default()
                .push((fact.predicate.clone(), fact.object.clone()));
        }
        Ok(KnowledgeGraph::from_bfs(&adjacency, from, depth))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::AgentId;

    fn agent() -> AgentId { AgentId::new("test") }

    #[tokio::test]
    async fn test_semantic_store_assert_fact_stores_retrievable() {
        let store = InMemorySemanticStore::new();
        let fact = Fact::new("Alice", "knows", "Bob", 0.9, agent());
        store.assert_fact(fact).await.unwrap();
        let facts = store.query_facts("Alice").await.unwrap();
        assert_eq!(facts.len(), 1);
        assert_eq!(facts[0].object, "Bob");
    }

    #[tokio::test]
    async fn test_semantic_store_retract_fact_removes_it() {
        let store = InMemorySemanticStore::new();
        let id = store.assert_fact(Fact::new("A", "rel", "B", 1.0, agent())).await.unwrap();
        store.retract_fact(&id).await.unwrap();
        assert!(store.query_facts("A").await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_semantic_store_retract_nonexistent_returns_error() {
        let store = InMemorySemanticStore::new();
        let fake = MemoryId::from_string("fake".to_string());
        assert!(matches!(store.retract_fact(&fake).await, Err(MemoryError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_semantic_store_query_facts_returns_only_matching_subject() {
        let store = InMemorySemanticStore::new();
        store.assert_fact(Fact::new("Alice", "knows", "Bob", 1.0, agent())).await.unwrap();
        store.assert_fact(Fact::new("Bob", "knows", "Carol", 1.0, agent())).await.unwrap();
        let facts = store.query_facts("Alice").await.unwrap();
        assert_eq!(facts.len(), 1);
    }

    #[tokio::test]
    async fn test_semantic_store_traverse_returns_connected_nodes() {
        let store = InMemorySemanticStore::new();
        store.assert_fact(Fact::new("Alice", "knows", "Bob", 1.0, agent())).await.unwrap();
        store.assert_fact(Fact::new("Bob", "knows", "Carol", 1.0, agent())).await.unwrap();
        let graph = store.traverse("Alice", 2).await.unwrap();
        assert!(graph.nodes.contains(&"Carol".to_string()));
    }

    #[tokio::test]
    async fn test_semantic_store_traverse_depth_limits_nodes() {
        let store = InMemorySemanticStore::new();
        store.assert_fact(Fact::new("A", "l", "B", 1.0, agent())).await.unwrap();
        store.assert_fact(Fact::new("B", "l", "C", 1.0, agent())).await.unwrap();
        store.assert_fact(Fact::new("C", "l", "D", 1.0, agent())).await.unwrap();
        let graph = store.traverse("A", 1).await.unwrap();
        assert!(!graph.nodes.contains(&"C".to_string()));
    }

    #[tokio::test]
    async fn test_fact_confidence_clamped_to_unit_interval() {
        let f1 = Fact::new("x", "p", "y", 2.0, agent());
        let f2 = Fact::new("x", "p", "y", -1.0, agent());
        assert!(f1.confidence <= 1.0);
        assert!(f2.confidence >= 0.0);
    }
}
