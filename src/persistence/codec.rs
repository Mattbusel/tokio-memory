//! # Snapshot codec
//!
//! Binary encode/decode for [`MemorySnapshot`] using `postcard`.
//!
//! ## Guarantees
//! - `encode_snapshot` → `decode_snapshot` is a lossless round-trip.
//! - Errors are surfaced as `MemoryError::Serialization`.

use crate::error::MemoryError;

use super::snapshot::MemorySnapshot;

/// Encode a [`MemorySnapshot`] to a binary byte vector using postcard.
///
/// # Returns
/// - `Ok(Vec<u8>)` on success.
/// - `Err(MemoryError::Serialization)` if encoding fails.
///
/// # Panics
/// This function never panics.
pub fn encode_snapshot(snap: &MemorySnapshot) -> Result<Vec<u8>, MemoryError> {
    postcard::to_allocvec(snap).map_err(|e| MemoryError::Serialization(e.to_string()))
}

/// Decode a [`MemorySnapshot`] from a binary byte slice produced by
/// [`encode_snapshot`].
///
/// # Returns
/// - `Ok(MemorySnapshot)` on success.
/// - `Err(MemoryError::Serialization)` if decoding fails.
///
/// # Panics
/// This function never panics.
pub fn decode_snapshot(bytes: &[u8]) -> Result<MemorySnapshot, MemoryError> {
    postcard::from_bytes(bytes).map_err(|e| MemoryError::Serialization(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::episodic::episode::{Episode, EpisodeKind};
    use crate::id::{AgentId, EntityId, SessionId, Tag};
    use crate::persistence::snapshot::MemorySnapshot;
    use crate::semantic::fact::{Fact, FactValue};

    fn make_snapshot() -> MemorySnapshot {
        let agent = AgentId::new();
        let ep = Episode::new(
            agent.clone(),
            SessionId::new(),
            EpisodeKind::Interaction,
            "hello",
            0.75,
            vec![Tag::new("test")],
        )
        .unwrap();
        let fact = Fact::new(
            EntityId::new("user:1"),
            "name",
            FactValue::Text("Alice".into()),
            0.9,
        )
        .unwrap();
        MemorySnapshot::new(agent, vec![ep], vec![fact])
    }

    #[test]
    fn test_snapshot_encode_decode_roundtrip() {
        let snap = make_snapshot();
        let bytes = encode_snapshot(&snap).unwrap();
        let decoded = decode_snapshot(&bytes).unwrap();
        assert_eq!(decoded.agent_id, snap.agent_id);
        assert_eq!(decoded.episodes.len(), snap.episodes.len());
        assert_eq!(decoded.facts.len(), snap.facts.len());
    }

    #[test]
    fn test_snapshot_encode_produces_nonempty_bytes() {
        let snap = make_snapshot();
        let bytes = encode_snapshot(&snap).unwrap();
        assert!(!bytes.is_empty());
    }

    #[test]
    fn test_snapshot_decode_bad_bytes_returns_serialization_error() {
        let bad = vec![0xDE, 0xAD, 0xBE, 0xEF];
        let result = decode_snapshot(&bad);
        assert!(matches!(result, Err(MemoryError::Serialization(_))));
    }

    #[test]
    fn test_snapshot_roundtrip_episode_content_preserved() {
        let snap = make_snapshot();
        let bytes = encode_snapshot(&snap).unwrap();
        let decoded = decode_snapshot(&bytes).unwrap();
        assert_eq!(decoded.episodes[0].content, "hello");
    }

    #[test]
    fn test_snapshot_roundtrip_fact_predicate_preserved() {
        let snap = make_snapshot();
        let bytes = encode_snapshot(&snap).unwrap();
        let decoded = decode_snapshot(&bytes).unwrap();
        assert_eq!(decoded.facts[0].predicate, "name");
    }

    #[test]
    fn test_snapshot_roundtrip_preserves_all_episodes() {
        let agent = AgentId::new();
        let session = SessionId::new();
        let episodes: Vec<Episode> = (0..5)
            .map(|i| {
                Episode::new(
                    agent.clone(),
                    session.clone(),
                    EpisodeKind::Observation,
                    format!("episode {i}"),
                    0.8,
                    vec![],
                )
                .unwrap()
            })
            .collect();
        let snap = MemorySnapshot::new(agent, episodes, vec![]);
        let bytes = encode_snapshot(&snap).unwrap();
        let decoded = decode_snapshot(&bytes).unwrap();
        assert_eq!(decoded.episodes.len(), 5);
        for (i, ep) in decoded.episodes.iter().enumerate() {
            assert_eq!(ep.content, format!("episode {i}"));
        }
    }
}
