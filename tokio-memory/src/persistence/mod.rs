//! # Module: Persistence
//!
//! ## Responsibility
//! Serialize and deserialize agent memory snapshots for cross-session persistence.

use async_trait::async_trait;
use chrono::Utc;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use crate::error::MemoryError;
use crate::types::{AgentId, MemoryId, Timestamp};

/// Metadata about a saved snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotMeta {
    /// Agent this snapshot belongs to.
    pub agent_id: AgentId,
    /// When taken.
    pub timestamp: Timestamp,
    /// Schema version.
    pub version: u32,
    /// Payload size in bytes.
    pub size_bytes: usize,
}

/// A complete serialized memory snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemorySnapshot {
    /// Schema version.
    pub version: u32,
    /// Agent this snapshot belongs to.
    pub agent_id: AgentId,
    /// When taken.
    pub timestamp: Timestamp,
    /// Serialized payload.
    pub payload: Vec<u8>,
}

impl MemorySnapshot {
    /// Create a new snapshot.
    pub fn new(agent_id: AgentId, payload: Vec<u8>) -> Self {
        Self { version: 1, agent_id, timestamp: Utc::now(), payload }
    }

    /// Return metadata for this snapshot.
    pub fn meta(&self) -> SnapshotMeta {
        SnapshotMeta {
            agent_id: self.agent_id.clone(),
            timestamp: self.timestamp,
            version: self.version,
            size_bytes: self.payload.len(),
        }
    }
}

/// Trait for memory persistence backends.
#[async_trait]
pub trait MemoryBackend: Send + Sync {
    /// Save a snapshot.
    ///
    /// # Panics
    /// Never panics.
    async fn save(&self, snapshot: &MemorySnapshot) -> Result<(), MemoryError>;

    /// Load the most recent snapshot for an agent.
    ///
    /// # Panics
    /// Never panics.
    async fn load(&self, agent_id: &AgentId) -> Result<MemorySnapshot, MemoryError>;

    /// List all snapshot metadata for an agent.
    ///
    /// # Panics
    /// Never panics.
    async fn list_snapshots(&self, agent_id: &AgentId) -> Result<Vec<SnapshotMeta>, MemoryError>;
}

/// In-memory backend (testing and ephemeral use).
pub struct InMemoryBackend {
    snapshots: Arc<DashMap<String, Vec<MemorySnapshot>>>,
}

impl InMemoryBackend {
    /// Create a new empty backend.
    pub fn new() -> Self {
        Self { snapshots: Arc::new(DashMap::new()) }
    }
}

impl Default for InMemoryBackend {
    fn default() -> Self { Self::new() }
}

#[async_trait]
impl MemoryBackend for InMemoryBackend {
    async fn save(&self, snapshot: &MemorySnapshot) -> Result<(), MemoryError> {
        self.snapshots
            .entry(snapshot.agent_id.0.clone())
            .or_insert_with(Vec::new)
            .push(snapshot.clone());
        Ok(())
    }

    async fn load(&self, agent_id: &AgentId) -> Result<MemorySnapshot, MemoryError> {
        self.snapshots
            .get(&agent_id.0)
            .and_then(|v| v.last().cloned())
            .ok_or_else(|| MemoryError::NotFound(MemoryId::from_string(agent_id.0.clone())))
    }

    async fn list_snapshots(&self, agent_id: &AgentId) -> Result<Vec<SnapshotMeta>, MemoryError> {
        Ok(self.snapshots
            .get(&agent_id.0)
            .map(|v| v.iter().map(|s| s.meta()).collect())
            .unwrap_or_default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::AgentId;

    #[tokio::test]
    async fn test_persistence_save_load_roundtrip() {
        let backend = InMemoryBackend::new();
        let agent = AgentId::new("agent");
        backend.save(&MemorySnapshot::new(agent.clone(), b"payload".to_vec())).await.unwrap();
        let loaded = backend.load(&agent).await.unwrap();
        assert_eq!(loaded.payload, b"payload");
    }

    #[tokio::test]
    async fn test_persistence_load_missing_agent_returns_error() {
        let backend = InMemoryBackend::new();
        assert!(matches!(
            backend.load(&AgentId::new("nobody")).await,
            Err(MemoryError::NotFound(_))
        ));
    }

    #[tokio::test]
    async fn test_persistence_list_snapshots_returns_all() {
        let backend = InMemoryBackend::new();
        let agent = AgentId::new("agent");
        for _ in 0..3 {
            backend.save(&MemorySnapshot::new(agent.clone(), vec![])).await.unwrap();
        }
        let list = backend.list_snapshots(&agent).await.unwrap();
        assert_eq!(list.len(), 3);
    }

    #[tokio::test]
    async fn test_persistence_list_snapshots_empty_for_unknown() {
        let backend = InMemoryBackend::new();
        let list = backend.list_snapshots(&AgentId::new("unknown")).await.unwrap();
        assert!(list.is_empty());
    }

    #[tokio::test]
    async fn test_persistence_latest_snapshot_returned() {
        let backend = InMemoryBackend::new();
        let agent = AgentId::new("agent");
        backend.save(&MemorySnapshot::new(agent.clone(), b"v1".to_vec())).await.unwrap();
        backend.save(&MemorySnapshot::new(agent.clone(), b"v2".to_vec())).await.unwrap();
        let loaded = backend.load(&agent).await.unwrap();
        assert_eq!(loaded.payload, b"v2");
    }
}
