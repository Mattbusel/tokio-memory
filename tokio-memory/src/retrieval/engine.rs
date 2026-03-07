use async_trait::async_trait;
use dashmap::DashMap;
use std::sync::Arc;
use crate::error::MemoryError;
use crate::types::{MemoryEntry, MemoryId};
use super::query::{MemoryQuery, QueryKind};

/// Trait for memory retrieval.
#[async_trait]
pub trait MemoryRetriever: Send + Sync {
    /// Retrieve entries matching the query, ranked by relevance.
    ///
    /// # Panics
    /// Never panics.
    async fn retrieve(&self, query: MemoryQuery) -> Result<Vec<MemoryEntry>, MemoryError>;
}

/// In-memory retrieval engine.
pub struct InMemoryRetrievalEngine {
    store: Arc<DashMap<MemoryId, MemoryEntry>>,
}

impl InMemoryRetrievalEngine {
    /// Create a new engine backed by the given store.
    pub fn new(store: Arc<DashMap<MemoryId, MemoryEntry>>) -> Self {
        Self { store }
    }

    /// Insert an entry into the backing store.
    pub fn insert(&self, entry: MemoryEntry) {
        self.store.insert(entry.id.clone(), entry);
    }
}

#[async_trait]
impl MemoryRetriever for InMemoryRetrievalEngine {
    async fn retrieve(&self, query: MemoryQuery) -> Result<Vec<MemoryEntry>, MemoryError> {
        let mut results: Vec<MemoryEntry> = self
            .store
            .iter()
            .filter(|e| e.value().strength >= query.min_strength)
            .filter(|e| match &query.kind {
                QueryKind::Exact => query.text.as_deref()
                    .and_then(|q| e.value().text.as_deref().map(|t| t == q))
                    .unwrap_or(false),
                QueryKind::Fuzzy => query.text.as_deref()
                    .and_then(|q| e.value().text.as_deref()
                        .map(|t| t.to_lowercase().contains(&q.to_lowercase())))
                    .unwrap_or(false),
                QueryKind::Temporal => query.time_range
                    .map(|(from, to)| e.value().created_at >= from && e.value().created_at <= to)
                    .unwrap_or(true),
                QueryKind::Associative => true,
            })
            .map(|e| e.value().clone())
            .collect();

        results.sort_by(|a, b| b.strength.partial_cmp(&a.strength).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(query.limit);
        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{AgentId, MemoryEntry};
    use chrono::Utc;
    use serde_json::json;

    fn entry(text: &str) -> MemoryEntry {
        MemoryEntry::new(json!({}), Some(text.to_string()), Some(AgentId::new("a")))
    }

    #[tokio::test]
    async fn test_retrieval_exact_match_returns_correct_entry() {
        let store = Arc::new(DashMap::new());
        let engine = InMemoryRetrievalEngine::new(store);
        engine.insert(entry("hello world"));
        engine.insert(entry("goodbye world"));
        let results = engine.retrieve(MemoryQuery::exact("hello world", 10)).await.unwrap();
        assert_eq!(results.len(), 1);
    }

    #[tokio::test]
    async fn test_retrieval_fuzzy_finds_substring() {
        let store = Arc::new(DashMap::new());
        let engine = InMemoryRetrievalEngine::new(store);
        engine.insert(entry("hello world"));
        engine.insert(entry("goodbye"));
        let results = engine.retrieve(MemoryQuery::fuzzy("hello", 10)).await.unwrap();
        assert_eq!(results.len(), 1);
    }

    #[tokio::test]
    async fn test_retrieval_temporal_range_filters_correctly() {
        let store = Arc::new(DashMap::new());
        let engine = InMemoryRetrievalEngine::new(store);
        let before = Utc::now();
        engine.insert(entry("event"));
        let after = Utc::now();
        let results = engine.retrieve(MemoryQuery::temporal(before, after, 10)).await.unwrap();
        assert_eq!(results.len(), 1);
    }

    #[tokio::test]
    async fn test_retrieval_limit_respected() {
        let store = Arc::new(DashMap::new());
        let engine = InMemoryRetrievalEngine::new(store);
        for i in 0..10 {
            engine.insert(entry(&format!("item {}", i)));
        }
        let results = engine.retrieve(MemoryQuery::fuzzy("item", 3)).await.unwrap();
        assert!(results.len() <= 3);
    }

    #[tokio::test]
    async fn test_retrieval_min_strength_filters_weak_memories() {
        let store = Arc::new(DashMap::new());
        let engine = InMemoryRetrievalEngine::new(store);
        let mut weak = entry("weak");
        weak.strength = 0.1;
        let mut strong = entry("strong");
        strong.strength = 0.9;
        engine.insert(weak);
        engine.insert(strong);
        let mut q = MemoryQuery::fuzzy("", 10);
        q.kind = crate::retrieval::query::QueryKind::Associative;
        q.min_strength = 0.5;
        let results = engine.retrieve(q).await.unwrap();
        assert_eq!(results.len(), 1);
    }
}
