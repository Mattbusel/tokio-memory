//! # tokio-memory
//!
//! Agent memory primitives for Tokio-based async agents.
//!
//! ## Modules
//! - [`episodic`] — Time-ordered event memory (episodes, store, timeline).
//! - [`semantic`] — Subject-predicate-object fact store.
//! - [`working`] — Bounded priority-queue scratchpad.
//! - [`decay`] — Memory strength decay policies (Ebbinghaus exponential).
//! - [`retrieval`] — Fluent query builder for episodic recall.
//! - [`consolidation`] — Policy-driven episode → fact promotion pipeline.
//! - [`shared`] — Namespaced shared memory with conflict resolution.
//! - [`persistence`] — Snapshot serialisation/deserialisation (postcard).
//! - `sqlite` — Durable SQLite store with full-text search (`sqlite` feature).
//! - [`error`] — Unified [`MemoryError`] type.
//! - [`id`] — Strongly-typed ID newtypes.

pub mod consolidation;
pub mod decay;
pub mod episodic;
pub mod error;
pub mod id;
pub mod persistence;
pub mod retrieval;
pub mod semantic;
pub mod shared;
#[cfg(feature = "sqlite")]
pub mod sqlite;
pub mod working;

// Convenience re-exports
pub use error::MemoryError;
pub use id::{AgentId, EntityId, MemoryId, NamespaceId, SessionId, Tag};

#[cfg(test)]
mod property_tests {
    use proptest::prelude::*;

    use crate::decay::decay::DecayPolicy;
    use crate::decay::ebbinghaus::ExponentialDecay;
    use crate::id::{AgentId, MemoryId, SessionId};
    use crate::persistence::{decode_snapshot, encode_snapshot, MemorySnapshot};

    proptest! {
        #[test]
        fn test_decay_strength_always_in_unit_interval(
            half_life_secs in 0.001f64..=86400.0,
            elapsed_secs in 0i64..=1_000_000i64,
        ) {
            let policy = ExponentialDecay::new(half_life_secs);
            let created = chrono::Utc::now();
            let now = created + chrono::Duration::seconds(elapsed_secs);
            let s = policy.strength_at(created, now);
            prop_assert!(s >= 0.0, "strength below 0: {s}");
            prop_assert!(s <= 1.0, "strength above 1: {s}");
        }

        #[test]
        fn test_memory_id_display_is_stable_proptest(
            hi in any::<u64>(),
            lo in any::<u64>(),
        ) {
            let bytes = {
                let mut b = [0u8; 16];
                b[..8].copy_from_slice(&hi.to_le_bytes());
                b[8..].copy_from_slice(&lo.to_le_bytes());
                b
            };
            let uuid = uuid::Uuid::from_bytes(bytes);
            let id = MemoryId::from(uuid);
            let s1 = id.to_string();
            let s2 = id.to_string();
            prop_assert_eq!(s1, s2);
        }

        #[test]
        fn test_snapshot_roundtrip_preserves_all_episodes_proptest(
            episode_count in 0usize..=10usize,
        ) {
            let agent = AgentId::new();
            let session = SessionId::new();
            let episodes: Vec<_> = (0..episode_count).map(|i| {
                crate::episodic::episode::Episode::new(
                    agent.clone(),
                    session.clone(),
                    crate::episodic::episode::EpisodeKind::Observation,
                    format!("episode {i}"),
                    0.8,
                    vec![],
                ).unwrap()
            }).collect();
            let snap = MemorySnapshot::new(agent, episodes, vec![]);
            let bytes = encode_snapshot(&snap).unwrap();
            let decoded = decode_snapshot(&bytes).unwrap();
            prop_assert_eq!(decoded.episodes.len(), episode_count);
        }
    }
}

/// Every Rust example in the README is compiled by `cargo test --doc`.
#[cfg(doctest)]
#[doc = include_str!("../README.md")]
mod readme_examples {}
