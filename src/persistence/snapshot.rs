//! # MemorySnapshot
//!
//! A point-in-time snapshot of an agent's memory, suitable for serialisation
//! and transport.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::episodic::episode::Episode;
use crate::id::AgentId;
use crate::semantic::fact::Fact;

/// A complete, serialisable snapshot of an agent's memory at a point in time.
///
/// # Fields
/// * `agent_id` — The agent whose memory is captured.
/// * `captured_at` — When the snapshot was taken (UTC).
/// * `episodes` — All episodic memories at capture time.
/// * `facts` — All semantic facts at capture time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemorySnapshot {
    /// The agent whose memory is captured.
    pub agent_id: AgentId,
    /// Snapshot creation timestamp (UTC).
    pub captured_at: DateTime<Utc>,
    /// Episodic memories.
    pub episodes: Vec<Episode>,
    /// Semantic facts.
    pub facts: Vec<Fact>,
}

impl MemorySnapshot {
    /// Create a new [`MemorySnapshot`].
    ///
    /// # Panics
    /// This function never panics.
    pub fn new(agent_id: AgentId, episodes: Vec<Episode>, facts: Vec<Fact>) -> Self {
        Self {
            agent_id,
            captured_at: Utc::now(),
            episodes,
            facts,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::id::{AgentId, SessionId};
    use crate::episodic::episode::{Episode, EpisodeKind};

    #[test]
    fn test_snapshot_new_stores_all_fields() {
        let agent = AgentId::new();
        let ep = Episode::new(
            agent.clone(),
            SessionId::new(),
            EpisodeKind::Observation,
            "test",
            0.8,
            vec![],
        )
        .unwrap();
        let snap = MemorySnapshot::new(agent.clone(), vec![ep], vec![]);
        assert_eq!(snap.agent_id, agent);
        assert_eq!(snap.episodes.len(), 1);
        assert!(snap.facts.is_empty());
    }
}
