//! # Persistence
//!
//! Snapshot-based serialization of agent memory state across sessions.
//! Uses postcard for compact binary encoding.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::episodic::episode::Episode;
use crate::error::MemoryError;
use crate::id::AgentId;
use crate::semantic::fact::Fact;

/// A full point-in-time snapshot of an agent's memory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemorySnapshot {
    pub agent_id: AgentId,
    pub captured_at: DateTime<Utc>,
    pub episodes: Vec<Episode>,
    pub facts: Vec<Fact>,
}

impl MemorySnapshot {
    /// Create a new snapshot for the given agent.
    pub fn new(agent_id: AgentId, episodes: Vec<Episode>, facts: Vec<Fact>) -> Self {
        Self { agent_id, captured_at: Utc::now(), episodes, facts }
    }
}

/// Encode a snapshot to binary using postcard.
pub fn encode_snapshot(snap: &MemorySnapshot) -> Result<Vec<u8>, MemoryError> {
    postcard::to_allocvec(snap).map_err(|e| MemoryError::Serialization(e.to_string()))
}

/// Decode a snapshot from binary using postcard.
pub fn decode_snapshot(bytes: &[u8]) -> Result<MemorySnapshot, MemoryError> {
    postcard::from_bytes(bytes).map_err(|e| MemoryError::Serialization(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::episodic::episode::{Episode, EpisodeKind};
    use crate::id::{AgentId, SessionId};

    #[test]
    fn test_snapshot_encode_decode_roundtrip() {
        let agent_id = AgentId::new();
        let session_id = SessionId::new();
        let ep = Episode::new(
            agent_id.clone(), session_id, EpisodeKind::Observation,
            "test content", 0.8, vec![],
        ).unwrap();
        let snap = MemorySnapshot::new(agent_id, vec![ep], vec![]);
        let bytes = encode_snapshot(&snap).unwrap();
        let decoded = decode_snapshot(&bytes).unwrap();
        assert_eq!(decoded.episodes.len(), 1);
        assert_eq!(decoded.episodes[0].content, "test content");
    }

    #[test]
    fn test_snapshot_decode_invalid_bytes_returns_error() {
        let result = decode_snapshot(b"not valid postcard data !!!");
        assert!(matches!(result, Err(MemoryError::Serialization(_))));
    }

    #[test]
    fn test_snapshot_empty_roundtrip() {
        let agent_id = AgentId::new();
        let snap = MemorySnapshot::new(agent_id, vec![], vec![]);
        let bytes = encode_snapshot(&snap).unwrap();
        let decoded = decode_snapshot(&bytes).unwrap();
        assert!(decoded.episodes.is_empty());
        assert!(decoded.facts.is_empty());
    }
}
