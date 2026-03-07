use async_trait::async_trait;
use dashmap::DashMap;
use std::sync::Arc;
use crate::error::MemoryError;
use crate::types::{MemoryId, Timestamp};
use super::episode::{Episode, EpisodeId};
use super::event::MemoryEvent;

/// Trait for episodic memory storage.
#[async_trait]
pub trait EpisodicStore: Send + Sync {
    /// Record a new event, returning its ID.
    ///
    /// # Panics
    /// Never panics.
    async fn record(&self, event: MemoryEvent) -> Result<MemoryId, MemoryError>;

    /// Recall a specific episode by ID.
    ///
    /// # Panics
    /// Never panics.
    async fn recall_episode(&self, id: &EpisodeId) -> Result<Episode, MemoryError>;

    /// Recall the most recent episodes in reverse chronological order.
    ///
    /// # Panics
    /// Never panics.
    async fn recall_recent(&self, limit: usize) -> Result<Vec<Episode>, MemoryError>;

    /// Recall all episodes since a given timestamp.
    ///
    /// # Panics
    /// Never panics.
    async fn recall_since(&self, since: Timestamp) -> Result<Vec<Episode>, MemoryError>;

    /// Total number of episodes stored.
    ///
    /// # Panics
    /// Never panics.
    async fn count(&self) -> usize;
}

/// In-memory implementation of EpisodicStore backed by DashMap.
pub struct InMemoryEpisodicStore {
    episodes: Arc<DashMap<EpisodeId, Episode>>,
}

impl InMemoryEpisodicStore {
    /// Create a new empty store.
    pub fn new() -> Self {
        Self { episodes: Arc::new(DashMap::new()) }
    }
}

impl Default for InMemoryEpisodicStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl EpisodicStore for InMemoryEpisodicStore {
    async fn record(&self, event: MemoryEvent) -> Result<MemoryId, MemoryError> {
        let event_id = event.id.clone();
        let episode = Episode::from_event(event);
        self.episodes.insert(episode.id.clone(), episode);
        Ok(event_id)
    }

    async fn recall_episode(&self, id: &EpisodeId) -> Result<Episode, MemoryError> {
        self.episodes
            .get(id)
            .map(|e| e.value().clone())
            .ok_or_else(|| MemoryError::NotFound(MemoryId::from_string(id.0.clone())))
    }

    async fn recall_recent(&self, limit: usize) -> Result<Vec<Episode>, MemoryError> {
        let mut episodes: Vec<Episode> =
            self.episodes.iter().map(|e| e.value().clone()).collect();
        episodes.sort_by(|a, b| b.start_time.cmp(&a.start_time));
        episodes.truncate(limit);
        Ok(episodes)
    }

    async fn recall_since(&self, since: Timestamp) -> Result<Vec<Episode>, MemoryError> {
        let mut episodes: Vec<Episode> = self
            .episodes
            .iter()
            .filter(|e| e.value().start_time >= since)
            .map(|e| e.value().clone())
            .collect();
        episodes.sort_by(|a, b| a.start_time.cmp(&b.start_time));
        Ok(episodes)
    }

    async fn count(&self) -> usize {
        self.episodes.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::episodic::event::{EventKind, EventMetadata};
    use crate::types::AgentId;
    use chrono::Utc;

    fn make_event(desc: &str) -> MemoryEvent {
        MemoryEvent::new(
            EventKind::Action,
            desc,
            serde_json::json!({"test": true}),
            EventMetadata { agent_id: AgentId::new("test"), session_id: None, tags: vec![] },
        )
    }

    #[tokio::test]
    async fn test_episodic_store_record_event_returns_valid_id() {
        let store = InMemoryEpisodicStore::new();
        let result = store.record(make_event("action")).await;
        assert!(result.is_ok());
        assert!(!result.unwrap().0.is_empty());
    }

    #[tokio::test]
    async fn test_episodic_store_count_increments_on_record() {
        let store = InMemoryEpisodicStore::new();
        assert_eq!(store.count().await, 0);
        store.record(make_event("e1")).await.unwrap();
        assert_eq!(store.count().await, 1);
    }

    #[tokio::test]
    async fn test_episodic_store_recall_recent_returns_in_order() {
        let store = InMemoryEpisodicStore::new();
        store.record(make_event("first")).await.unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(2)).await;
        store.record(make_event("second")).await.unwrap();
        let recent = store.recall_recent(10).await.unwrap();
        assert_eq!(recent.len(), 2);
        assert!(recent[0].start_time >= recent[1].start_time);
    }

    #[tokio::test]
    async fn test_episodic_store_recall_recent_respects_limit() {
        let store = InMemoryEpisodicStore::new();
        for i in 0..5 {
            store.record(make_event(&format!("e{}", i))).await.unwrap();
        }
        let recent = store.recall_recent(3).await.unwrap();
        assert_eq!(recent.len(), 3);
    }

    #[tokio::test]
    async fn test_episodic_store_recall_since_filters_correctly() {
        let store = InMemoryEpisodicStore::new();
        store.record(make_event("old")).await.unwrap();
        let cutoff = Utc::now();
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        store.record(make_event("new")).await.unwrap();
        let since = store.recall_since(cutoff).await.unwrap();
        assert_eq!(since.len(), 1);
        assert_eq!(since[0].events[0].description, "new");
    }

    #[tokio::test]
    async fn test_episodic_store_recall_episode_not_found_returns_error() {
        let store = InMemoryEpisodicStore::new();
        let result = store.recall_episode(&EpisodeId::new("nonexistent")).await;
        assert!(matches!(result, Err(MemoryError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_episodic_store_recall_episode_found_returns_episode() {
        let store = InMemoryEpisodicStore::new();
        store.record(make_event("findable")).await.unwrap();
        let episodes = store.recall_recent(1).await.unwrap();
        let recalled = store.recall_episode(&episodes[0].id).await.unwrap();
        assert_eq!(recalled.events[0].description, "findable");
    }
}
