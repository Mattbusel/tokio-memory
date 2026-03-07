use async_trait::async_trait;
use dashmap::DashMap;
use std::sync::Arc;
use crate::error::MemoryError;
use crate::types::{AgentId, MemoryEntry};
use super::conflict::{Conflict, ConflictResolution, WriteReceipt};
use super::namespace::MemoryNamespace;

/// Trait for cross-agent shared memory.
#[async_trait]
pub trait SharedMemoryStore: Send + Sync {
    /// Write an entry to a namespace.
    ///
    /// # Panics
    /// Never panics.
    async fn write(
        &self,
        ns: &MemoryNamespace,
        key: &str,
        entry: MemoryEntry,
        agent: &AgentId,
    ) -> Result<WriteReceipt, MemoryError>;

    /// Read an entry from a namespace.
    ///
    /// # Panics
    /// Never panics.
    async fn read(&self, ns: &MemoryNamespace, key: &str) -> Result<MemoryEntry, MemoryError>;

    /// Resolve a conflict.
    ///
    /// # Panics
    /// Never panics.
    async fn resolve_conflict(
        &self,
        conflict: Conflict,
        strategy: ConflictResolution,
    ) -> Result<MemoryEntry, MemoryError>;

    /// Create a new namespace.
    ///
    /// # Panics
    /// Never panics.
    async fn create_namespace(&self, ns: MemoryNamespace) -> Result<(), MemoryError>;
}

/// DashMap-backed shared memory store.
pub struct InMemorySharedStore {
    namespaces: Arc<DashMap<String, MemoryNamespace>>,
    entries: Arc<DashMap<String, DashMap<String, (MemoryEntry, AgentId)>>>,
}

impl InMemorySharedStore {
    /// Create a new empty store.
    pub fn new() -> Self {
        Self {
            namespaces: Arc::new(DashMap::new()),
            entries: Arc::new(DashMap::new()),
        }
    }
}

impl Default for InMemorySharedStore {
    fn default() -> Self { Self::new() }
}

#[async_trait]
impl SharedMemoryStore for InMemorySharedStore {
    async fn write(
        &self,
        ns: &MemoryNamespace,
        key: &str,
        entry: MemoryEntry,
        agent: &AgentId,
    ) -> Result<WriteReceipt, MemoryError> {
        if !self.namespaces.contains_key(&ns.id) {
            return Err(MemoryError::NamespaceNotFound { namespace: ns.id.clone() });
        }
        let id = entry.id.clone();
        self.entries
            .entry(ns.id.clone())
            .or_insert_with(DashMap::new)
            .insert(key.to_string(), (entry, agent.clone()));
        Ok(WriteReceipt { id, conflict_resolved: false })
    }

    async fn read(&self, ns: &MemoryNamespace, key: &str) -> Result<MemoryEntry, MemoryError> {
        self.entries
            .get(&ns.id)
            .and_then(|m| m.get(key).map(|e| e.value().0.clone()))
            .ok_or_else(|| MemoryError::NamespaceNotFound { namespace: ns.id.clone() })
    }

    async fn resolve_conflict(
        &self,
        conflict: Conflict,
        strategy: ConflictResolution,
    ) -> Result<MemoryEntry, MemoryError> {
        match strategy {
            ConflictResolution::LastWriteWins => {
                if conflict.incoming.created_at >= conflict.existing.created_at {
                    Ok(conflict.incoming)
                } else {
                    Ok(conflict.existing)
                }
            }
            ConflictResolution::HighestConfidence => {
                if conflict.incoming.strength >= conflict.existing.strength {
                    Ok(conflict.incoming)
                } else {
                    Ok(conflict.existing)
                }
            }
            ConflictResolution::MergeAll => {
                let mut merged = conflict.existing.clone();
                if let (
                    serde_json::Value::Object(mut base),
                    serde_json::Value::Object(extra),
                ) = (merged.content.clone(), conflict.incoming.content)
                {
                    for (k, v) in extra {
                        base.insert(k, v);
                    }
                    merged.content = serde_json::Value::Object(base);
                }
                Ok(merged)
            }
        }
    }

    async fn create_namespace(&self, ns: MemoryNamespace) -> Result<(), MemoryError> {
        self.namespaces.insert(ns.id.clone(), ns.clone());
        self.entries.insert(ns.id, DashMap::new());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{AgentId, MemoryEntry};
    use serde_json::json;

    fn agent(s: &str) -> AgentId { AgentId::new(s) }
    fn ns(id: &str) -> MemoryNamespace {
        MemoryNamespace { id: id.to_string(), shared: true, owners: vec![] }
    }

    #[tokio::test]
    async fn test_shared_store_write_to_unknown_namespace_returns_error() {
        let store = InMemorySharedStore::new();
        let entry = MemoryEntry::new(json!({}), None, None);
        let result = store.write(&ns("unknown"), "k", entry, &agent("a")).await;
        assert!(matches!(result, Err(MemoryError::NamespaceNotFound { .. })));
    }

    #[tokio::test]
    async fn test_shared_store_write_and_read_roundtrip() {
        let store = InMemorySharedStore::new();
        let namespace = ns("shared");
        store.create_namespace(namespace.clone()).await.unwrap();
        let entry = MemoryEntry::new(json!({"data": 42}), None, None);
        store.write(&namespace, "k", entry, &agent("a")).await.unwrap();
        let read = store.read(&namespace, "k").await.unwrap();
        assert_eq!(read.content["data"], json!(42));
    }

    #[tokio::test]
    async fn test_shared_memory_last_write_wins_resolution() {
        let store = InMemorySharedStore::new();
        let mut e1 = MemoryEntry::new(json!({"v": "old"}), None, None);
        let mut e2 = MemoryEntry::new(json!({"v": "new"}), None, None);
        // make e2 newer
        e2.created_at = chrono::Utc::now() + chrono::Duration::seconds(1);
        let conflict = Conflict {
            key: "k".to_string(),
            existing: e1.clone(),
            incoming: e2.clone(),
            existing_agent: agent("a"),
            incoming_agent: agent("b"),
        };
        let result = store.resolve_conflict(conflict, ConflictResolution::LastWriteWins).await.unwrap();
        assert_eq!(result.content["v"], json!("new"));
    }

    #[tokio::test]
    async fn test_shared_memory_highest_confidence_picks_stronger() {
        let store = InMemorySharedStore::new();
        let mut e1 = MemoryEntry::new(json!({"v": "low"}), None, None);
        let mut e2 = MemoryEntry::new(json!({"v": "high"}), None, None);
        e1.strength = 0.3;
        e2.strength = 0.9;
        let conflict = Conflict {
            key: "k".to_string(),
            existing: e1,
            incoming: e2,
            existing_agent: agent("a"),
            incoming_agent: agent("b"),
        };
        let result = store.resolve_conflict(conflict, ConflictResolution::HighestConfidence).await.unwrap();
        assert_eq!(result.content["v"], json!("high"));
    }

    #[tokio::test]
    async fn test_shared_memory_merge_all_combines_keys() {
        let store = InMemorySharedStore::new();
        let e1 = MemoryEntry::new(json!({"a": 1}), None, None);
        let e2 = MemoryEntry::new(json!({"b": 2}), None, None);
        let conflict = Conflict {
            key: "k".to_string(),
            existing: e1,
            incoming: e2,
            existing_agent: agent("a"),
            incoming_agent: agent("b"),
        };
        let result = store.resolve_conflict(conflict, ConflictResolution::MergeAll).await.unwrap();
        assert_eq!(result.content["a"], json!(1));
        assert_eq!(result.content["b"], json!(2));
    }
}
