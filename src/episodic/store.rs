//! # EpisodicStore
//!
//! ## Responsibility
//! Persist and retrieve [`Episode`] records. The in-memory implementation is
//! backed by a [`DashMap`] for O(1) lookup and a [`parking_lot::Mutex`]-protected
//! `Vec<MemoryId>` for insertion-order indexing.
//!
//! ## Guarantees
//! - Thread-safe: all operations are safe under concurrent access.
//! - Non-blocking on the hot path (no async mutex on reads).
//! - `recall_range` filters by the closed interval `[start, end]`.
//!
//! ## NOT Responsible For
//! - Cross-node replication.
//! - Persistence to disk (see `persistence` module).

use std::sync::Arc;

use async_trait::async_trait;
use dashmap::DashMap;

use crate::error::MemoryError;
use crate::id::{AgentId, MemoryId, SessionId, Tag};

use super::episode::Episode;
use super::timeline::TimeRange;

/// Async trait for episodic memory storage.
///
/// Implementors must be `Send + Sync + 'static`.
#[async_trait]
pub trait EpisodicStore: Send + Sync + 'static {
    /// Record a new episode.
    ///
    /// # Returns
    /// - `Ok(())` on success.
    /// - `Err` if the store cannot accept the episode.
    ///
    /// # Panics
    /// Implementations must not panic.
    async fn record(&self, episode: Episode) -> Result<(), MemoryError>;

    /// Recall a single episode by its ID.
    ///
    /// # Returns
    /// - `Ok(Episode)` if found.
    /// - `Err(MemoryError::NotFound)` if no episode with that ID exists.
    ///
    /// # Panics
    /// Implementations must not panic.
    async fn recall(&self, id: &MemoryId) -> Result<Episode, MemoryError>;

    /// Recall all episodes whose timestamps fall within `range`.
    ///
    /// # Panics
    /// Implementations must not panic.
    async fn recall_range(&self, range: &TimeRange) -> Result<Vec<Episode>, MemoryError>;

    /// Recall all episodes recorded during the given session.
    ///
    /// # Panics
    /// Implementations must not panic.
    async fn recall_by_session(&self, session_id: &SessionId) -> Result<Vec<Episode>, MemoryError>;

    /// Recall all episodes that carry at least one of the given tags.
    ///
    /// # Panics
    /// Implementations must not panic.
    async fn recall_by_tag(&self, tag: &Tag) -> Result<Vec<Episode>, MemoryError>;

    /// Return all episodes in insertion order.
    ///
    /// # Panics
    /// Implementations must not panic.
    async fn all(&self) -> Result<Vec<Episode>, MemoryError>;

    /// Return all episodes for a given agent.
    ///
    /// # Panics
    /// Implementations must not panic.
    async fn recall_by_agent(&self, agent_id: &AgentId) -> Result<Vec<Episode>, MemoryError>;
}

// ---------------------------------------------------------------------------
// In-memory implementation
// ---------------------------------------------------------------------------

/// In-memory implementation of [`EpisodicStore`].
///
/// Backed by a [`DashMap`] for O(1) keyed access and a `Mutex<Vec<MemoryId>>`
/// for insertion-order traversal.
#[derive(Debug, Default)]
pub struct InMemoryEpisodicStore {
    map: DashMap<MemoryId, Episode>,
    order: Arc<std::sync::Mutex<Vec<MemoryId>>>,
}

impl InMemoryEpisodicStore {
    /// Create a new, empty [`InMemoryEpisodicStore`].
    pub fn new() -> Self {
        Self {
            map: DashMap::new(),
            order: Arc::new(std::sync::Mutex::new(Vec::new())),
        }
    }

    /// Return the number of stored episodes.
    pub fn len(&self) -> usize {
        self.map.len()
    }

    /// Return `true` if no episodes are stored.
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    /// Collect episodes in insertion order, applying a filter predicate.
    fn collect_filtered<F>(&self, pred: F) -> Vec<Episode>
    where
        F: Fn(&Episode) -> bool,
    {
        let order = self.order.lock().unwrap_or_else(|e| e.into_inner());
        order
            .iter()
            .filter_map(|id| {
                self.map.get(id).and_then(|ep| {
                    if pred(&ep) {
                        Some(ep.clone())
                    } else {
                        None
                    }
                })
            })
            .collect()
    }
}

#[async_trait]
impl EpisodicStore for InMemoryEpisodicStore {
    async fn record(&self, episode: Episode) -> Result<(), MemoryError> {
        let id = episode.id.clone();
        self.map.insert(id.clone(), episode);
        let mut order = self.order.lock().unwrap_or_else(|e| e.into_inner());
        order.push(id);
        Ok(())
    }

    async fn recall(&self, id: &MemoryId) -> Result<Episode, MemoryError> {
        self.map
            .get(id)
            .map(|ep| ep.clone())
            .ok_or_else(|| MemoryError::NotFound(id.clone()))
    }

    async fn recall_range(&self, range: &TimeRange) -> Result<Vec<Episode>, MemoryError> {
        Ok(self.collect_filtered(|ep| range.contains(&ep.timestamp)))
    }

    async fn recall_by_session(&self, session_id: &SessionId) -> Result<Vec<Episode>, MemoryError> {
        Ok(self.collect_filtered(|ep| &ep.session_id == session_id))
    }

    async fn recall_by_tag(&self, tag: &Tag) -> Result<Vec<Episode>, MemoryError> {
        Ok(self.collect_filtered(|ep| ep.tags.contains(tag)))
    }

    async fn all(&self) -> Result<Vec<Episode>, MemoryError> {
        Ok(self.collect_filtered(|_| true))
    }

    async fn recall_by_agent(&self, agent_id: &AgentId) -> Result<Vec<Episode>, MemoryError> {
        Ok(self.collect_filtered(|ep| &ep.agent_id == agent_id))
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::episodic::episode::EpisodeKind;
    use crate::id::{AgentId, SessionId, Tag};
    use chrono::{DateTime, Duration, Utc};

    fn base_ts() -> DateTime<Utc> {
        DateTime::parse_from_rfc3339("2025-06-01T12:00:00Z")
            .unwrap()
            .with_timezone(&Utc)
    }

    fn make_episode(
        agent: AgentId,
        session: SessionId,
        content: &str,
        tags: Vec<Tag>,
        ts: DateTime<Utc>,
    ) -> Episode {
        Episode::with_timestamp(
            agent,
            session,
            EpisodeKind::Observation,
            content,
            0.8,
            tags,
            ts,
        )
        .unwrap()
    }

    #[tokio::test]
    async fn test_episodic_store_record_and_recall_returns_same_episode() {
        let store = InMemoryEpisodicStore::new();
        let agent = AgentId::new();
        let session = SessionId::new();
        let ep = make_episode(agent, session, "hello", vec![], Utc::now());
        let id = ep.id.clone();
        store.record(ep).await.unwrap();
        let recalled = store.recall(&id).await.unwrap();
        assert_eq!(recalled.id, id);
        assert_eq!(recalled.content, "hello");
    }

    #[tokio::test]
    async fn test_episodic_store_recall_missing_id_returns_not_found() {
        let store = InMemoryEpisodicStore::new();
        let missing = MemoryId::new();
        let result = store.recall(&missing).await;
        assert!(matches!(result, Err(MemoryError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_episodic_store_recall_range_filters_correctly() {
        let store = InMemoryEpisodicStore::new();
        let agent = AgentId::new();
        let session = SessionId::new();
        let base = base_ts();

        let ep_in = make_episode(
            agent.clone(),
            session.clone(),
            "in-range",
            vec![],
            base + Duration::minutes(30),
        );
        let ep_out = make_episode(
            agent.clone(),
            session.clone(),
            "out-of-range",
            vec![],
            base + Duration::hours(3),
        );

        store.record(ep_in).await.unwrap();
        store.record(ep_out).await.unwrap();

        let range = TimeRange::new(base, base + Duration::hours(1));
        let results = store.recall_range(&range).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].content, "in-range");
    }

    #[tokio::test]
    async fn test_episodic_store_recall_by_session_filters_correctly() {
        let store = InMemoryEpisodicStore::new();
        let agent = AgentId::new();
        let s1 = SessionId::new();
        let s2 = SessionId::new();

        let ep1 = make_episode(agent.clone(), s1.clone(), "session1", vec![], Utc::now());
        let ep2 = make_episode(agent.clone(), s2.clone(), "session2", vec![], Utc::now());
        store.record(ep1).await.unwrap();
        store.record(ep2).await.unwrap();

        let results = store.recall_by_session(&s1).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].content, "session1");
    }

    #[tokio::test]
    async fn test_episodic_store_recall_by_tag_filters_correctly() {
        let store = InMemoryEpisodicStore::new();
        let agent = AgentId::new();
        let session = SessionId::new();

        let t = Tag::new("important");
        let ep_tagged = make_episode(
            agent.clone(),
            session.clone(),
            "tagged",
            vec![t.clone()],
            Utc::now(),
        );
        let ep_plain = make_episode(agent.clone(), session.clone(), "plain", vec![], Utc::now());

        store.record(ep_tagged).await.unwrap();
        store.record(ep_plain).await.unwrap();

        let results = store.recall_by_tag(&t).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].content, "tagged");
    }

    #[tokio::test]
    async fn test_episodic_store_all_returns_insertion_order() {
        let store = InMemoryEpisodicStore::new();
        let agent = AgentId::new();
        let session = SessionId::new();

        for i in 0..5 {
            let ep = make_episode(
                agent.clone(),
                session.clone(),
                &format!("ep{i}"),
                vec![],
                Utc::now(),
            );
            store.record(ep).await.unwrap();
        }

        let all = store.all().await.unwrap();
        assert_eq!(all.len(), 5);
        for (i, ep) in all.iter().enumerate() {
            assert_eq!(ep.content, format!("ep{i}"));
        }
    }

    #[tokio::test]
    async fn test_episodic_store_recall_by_agent_filters_correctly() {
        let store = InMemoryEpisodicStore::new();
        let a1 = AgentId::new();
        let a2 = AgentId::new();
        let session = SessionId::new();

        store
            .record(make_episode(a1.clone(), session.clone(), "a1", vec![], Utc::now()))
            .await
            .unwrap();
        store
            .record(make_episode(a2.clone(), session.clone(), "a2", vec![], Utc::now()))
            .await
            .unwrap();

        let r = store.recall_by_agent(&a1).await.unwrap();
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].content, "a1");
    }

    #[tokio::test]
    async fn test_episodic_store_len_and_is_empty() {
        let store = InMemoryEpisodicStore::new();
        assert!(store.is_empty());
        let agent = AgentId::new();
        let session = SessionId::new();
        store
            .record(make_episode(agent, session, "x", vec![], Utc::now()))
            .await
            .unwrap();
        assert_eq!(store.len(), 1);
        assert!(!store.is_empty());
    }
}
