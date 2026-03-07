//! # SemanticStore
//!
//! ## Responsibility
//! Persist and query [`Fact`] records keyed by their subject [`EntityId`].
//!
//! ## Guarantees
//! - Thread-safe via [`DashMap`].
//! - `query_subject` returns all facts for a given entity.
//! - `retract_fact` removes exactly the fact with the matching [`MemoryId`].

use async_trait::async_trait;
use dashmap::DashMap;

use crate::error::MemoryError;
use crate::id::{EntityId, MemoryId};

use super::fact::Fact;

/// Async trait for semantic memory storage.
#[async_trait]
pub trait SemanticStore: Send + Sync + 'static {
    /// Assert (store) a fact.
    ///
    /// If a fact with the same ID already exists it is overwritten.
    ///
    /// # Panics
    /// Implementations must not panic.
    async fn assert_fact(&self, fact: Fact) -> Result<(), MemoryError>;

    /// Retract a fact by its [`MemoryId`].
    ///
    /// # Returns
    /// - `Ok(())` if the fact was found and removed.
    /// - `Err(MemoryError::NotFound)` if no fact with that ID exists.
    ///
    /// # Panics
    /// Implementations must not panic.
    async fn retract_fact(&self, id: &MemoryId) -> Result<(), MemoryError>;

    /// Return all facts about the given subject entity.
    ///
    /// # Panics
    /// Implementations must not panic.
    async fn query_subject(&self, subject: &EntityId) -> Result<Vec<Fact>, MemoryError>;

    /// Return every fact in the store, across all subjects.
    ///
    /// # Panics
    /// Implementations must not panic.
    async fn all_facts(&self) -> Result<Vec<Fact>, MemoryError>;
}

// ---------------------------------------------------------------------------
// In-memory implementation
// ---------------------------------------------------------------------------

/// In-memory implementation of [`SemanticStore`].
///
/// Facts are grouped by [`EntityId`] in a `DashMap<EntityId, Vec<Fact>>`.
#[derive(Debug, Default)]
pub struct InMemorySemanticStore {
    map: DashMap<EntityId, Vec<Fact>>,
}

impl InMemorySemanticStore {
    /// Create a new, empty [`InMemorySemanticStore`].
    pub fn new() -> Self {
        Self {
            map: DashMap::new(),
        }
    }
}

#[async_trait]
impl SemanticStore for InMemorySemanticStore {
    async fn assert_fact(&self, fact: Fact) -> Result<(), MemoryError> {
        let mut entry = self.map.entry(fact.subject.clone()).or_default();
        // Replace if same ID exists, otherwise append.
        if let Some(pos) = entry.iter().position(|f| f.id == fact.id) {
            entry[pos] = fact;
        } else {
            entry.push(fact);
        }
        Ok(())
    }

    async fn retract_fact(&self, id: &MemoryId) -> Result<(), MemoryError> {
        for mut entry in self.map.iter_mut() {
            if let Some(pos) = entry.iter().position(|f| &f.id == id) {
                entry.remove(pos);
                return Ok(());
            }
        }
        Err(MemoryError::NotFound(id.clone()))
    }

    async fn query_subject(&self, subject: &EntityId) -> Result<Vec<Fact>, MemoryError> {
        Ok(self
            .map
            .get(subject)
            .map(|v| v.clone())
            .unwrap_or_default())
    }

    async fn all_facts(&self) -> Result<Vec<Fact>, MemoryError> {
        let mut out = Vec::new();
        for entry in self.map.iter() {
            out.extend(entry.clone());
        }
        Ok(out)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::semantic::fact::FactValue;
    use crate::id::EntityId;

    fn make_fact(subject: &str, predicate: &str, val: &str) -> Fact {
        Fact::new(
            EntityId::new(subject),
            predicate,
            FactValue::Text(val.into()),
            0.9,
        )
        .unwrap()
    }

    #[tokio::test]
    async fn test_semantic_store_assert_and_query() {
        let store = InMemorySemanticStore::new();
        let f = make_fact("user:1", "name", "Alice");
        store.assert_fact(f).await.unwrap();
        let facts = store.query_subject(&EntityId::new("user:1")).await.unwrap();
        assert_eq!(facts.len(), 1);
        assert_eq!(facts[0].predicate, "name");
    }

    #[tokio::test]
    async fn test_semantic_store_retract_removes_fact() {
        let store = InMemorySemanticStore::new();
        let f = make_fact("user:2", "role", "admin");
        let id = f.id.clone();
        store.assert_fact(f).await.unwrap();
        store.retract_fact(&id).await.unwrap();
        let facts = store.query_subject(&EntityId::new("user:2")).await.unwrap();
        assert!(facts.is_empty());
    }

    #[tokio::test]
    async fn test_semantic_store_retract_missing_returns_not_found() {
        let store = InMemorySemanticStore::new();
        let missing = MemoryId::new();
        let result = store.retract_fact(&missing).await;
        assert!(matches!(result, Err(MemoryError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_semantic_store_query_missing_subject_returns_empty() {
        let store = InMemorySemanticStore::new();
        let facts = store.query_subject(&EntityId::new("nobody")).await.unwrap();
        assert!(facts.is_empty());
    }

    #[tokio::test]
    async fn test_semantic_store_all_facts_returns_all() {
        let store = InMemorySemanticStore::new();
        store.assert_fact(make_fact("e1", "p", "v1")).await.unwrap();
        store.assert_fact(make_fact("e2", "p", "v2")).await.unwrap();
        let all = store.all_facts().await.unwrap();
        assert_eq!(all.len(), 2);
    }

    #[tokio::test]
    async fn test_semantic_store_assert_same_id_replaces() {
        let store = InMemorySemanticStore::new();
        let mut f = make_fact("e1", "p", "original");
        let id = f.id.clone();
        store.assert_fact(f.clone()).await.unwrap();
        f.object = FactValue::Text("updated".into());
        store.assert_fact(f).await.unwrap();
        let facts = store.query_subject(&EntityId::new("e1")).await.unwrap();
        assert_eq!(facts.len(), 1);
        let updated = facts.iter().find(|f| f.id == id).unwrap();
        assert_eq!(updated.object, FactValue::Text("updated".into()));
    }

    #[tokio::test]
    async fn test_semantic_store_multiple_facts_per_subject() {
        let store = InMemorySemanticStore::new();
        store.assert_fact(make_fact("e1", "name", "Alice")).await.unwrap();
        store.assert_fact(make_fact("e1", "role", "admin")).await.unwrap();
        let facts = store.query_subject(&EntityId::new("e1")).await.unwrap();
        assert_eq!(facts.len(), 2);
    }
}
