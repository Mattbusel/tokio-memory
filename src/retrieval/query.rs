//! # MemoryQuery
//!
//! A fluent builder for constructing and executing episodic memory queries.
//!
//! ## Example
//! ```rust,no_run
//! use std::sync::Arc;
//! use tokio_memory::retrieval::query::MemoryQuery;
//! use tokio_memory::episodic::store::InMemoryEpisodicStore;
//! use tokio_memory::id::{AgentId, Tag};
//!
//! # async fn example() {
//! let store = Arc::new(InMemoryEpisodicStore::new());
//! let agent = AgentId::new();
//! let results = MemoryQuery::new()
//!     .with_agent(agent)
//!     .with_tag(Tag::new("important"))
//!     .limit(10)
//!     .execute(&*store)
//!     .await
//!     .unwrap();
//! # }
//! ```

use std::time::Duration;

use crate::episodic::episode::{Episode, EpisodeKind};
use crate::episodic::store::EpisodicStore;
use crate::error::MemoryError;
use crate::id::{AgentId, SessionId, Tag};

/// A composable query for retrieving [`Episode`]s from an [`EpisodicStore`].
///
/// All fields are optional; omitted filters are not applied.
#[derive(Debug, Default, Clone)]
pub struct MemoryQuery {
    /// Filter by owning agent.
    pub agent_id: Option<AgentId>,
    /// Filter by session.
    pub session_id: Option<SessionId>,
    /// Filter: only include episodes bearing at least one of these tags.
    pub tags: Vec<Tag>,
    /// Filter: only include episodes recorded within the last `since` duration.
    pub since: Option<Duration>,
    /// Maximum number of episodes to return (applied after all other filters).
    pub limit: Option<usize>,
    /// Filter by episode kind.
    pub kind: Option<EpisodeKind>,
}

impl MemoryQuery {
    /// Create a new, unconstrained [`MemoryQuery`].
    ///
    /// # Panics
    /// This function never panics.
    pub fn new() -> Self {
        Self::default()
    }

    /// Restrict results to episodes owned by `agent_id`.
    ///
    /// # Panics
    /// This function never panics.
    pub fn with_agent(mut self, agent_id: AgentId) -> Self {
        self.agent_id = Some(agent_id);
        self
    }

    /// Restrict results to episodes from `session_id`.
    ///
    /// # Panics
    /// This function never panics.
    pub fn with_session(mut self, session_id: SessionId) -> Self {
        self.session_id = Some(session_id);
        self
    }

    /// Restrict results to episodes bearing `tag`.
    ///
    /// Multiple calls accumulate tags (OR semantics).
    ///
    /// # Panics
    /// This function never panics.
    pub fn with_tag(mut self, tag: Tag) -> Self {
        self.tags.push(tag);
        self
    }

    /// Restrict results to episodes recorded within the last `duration`.
    ///
    /// # Panics
    /// This function never panics.
    pub fn since(mut self, duration: Duration) -> Self {
        self.since = Some(duration);
        self
    }

    /// Cap the result set at `n` episodes.
    ///
    /// # Panics
    /// This function never panics.
    pub fn limit(mut self, n: usize) -> Self {
        self.limit = Some(n);
        self
    }

    /// Restrict results to episodes of the given `kind`.
    ///
    /// # Panics
    /// This function never panics.
    pub fn with_kind(mut self, kind: EpisodeKind) -> Self {
        self.kind = Some(kind);
        self
    }

    /// Execute the query against the provided store, returning matching episodes.
    ///
    /// Filters are applied in this order:
    /// 1. Agent ID
    /// 2. Session ID
    /// 3. Tags (OR: episode must have at least one matching tag if `tags` is non-empty)
    /// 4. `since` time window
    /// 5. Kind
    /// 6. Limit
    ///
    /// # Returns
    /// - `Ok(Vec<Episode>)` — possibly empty — on success.
    /// - `Err(MemoryError)` if the store returns an error.
    ///
    /// # Panics
    /// This function never panics.
    pub async fn execute<S: EpisodicStore>(
        &self,
        store: &S,
    ) -> Result<Vec<Episode>, MemoryError> {
        let all = store.all().await?;
        let cutoff = self.since.map(|d| {
            chrono::Utc::now()
                - chrono::Duration::from_std(d).unwrap_or(chrono::Duration::MAX)
        });

        let mut results: Vec<Episode> = all
            .into_iter()
            .filter(|ep| {
                if let Some(ref aid) = self.agent_id {
                    if &ep.agent_id != aid {
                        return false;
                    }
                }
                if let Some(ref sid) = self.session_id {
                    if &ep.session_id != sid {
                        return false;
                    }
                }
                if !self.tags.is_empty() && !self.tags.iter().any(|t| ep.tags.contains(t)) {
                    return false;
                }
                if let Some(cutoff_ts) = cutoff {
                    if ep.timestamp < cutoff_ts {
                        return false;
                    }
                }
                if let Some(ref k) = self.kind {
                    if &ep.kind != k {
                        return false;
                    }
                }
                true
            })
            .collect();

        if let Some(lim) = self.limit {
            results.truncate(lim);
        }

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::episodic::episode::EpisodeKind;
    use crate::episodic::store::InMemoryEpisodicStore;
    use crate::id::{AgentId, SessionId, Tag};
    use chrono::Utc;

    fn ep(
        store: &InMemoryEpisodicStore,
        agent: AgentId,
        session: SessionId,
        kind: EpisodeKind,
        tags: Vec<Tag>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + '_>> {
        let ep = crate::episodic::episode::Episode::new(
            agent, session, kind, "content", 0.8, tags,
        )
        .unwrap();
        Box::pin(async move {
            store.record(ep).await.unwrap();
        })
    }

    #[tokio::test]
    async fn test_memory_query_builder_no_filter_returns_all() {
        let store = InMemoryEpisodicStore::new();
        let a = AgentId::new();
        let s = SessionId::new();
        ep(&store, a.clone(), s.clone(), EpisodeKind::Observation, vec![]).await;
        ep(&store, a.clone(), s.clone(), EpisodeKind::Action, vec![]).await;

        let results = MemoryQuery::new().execute(&store).await.unwrap();
        assert_eq!(results.len(), 2);
    }

    #[tokio::test]
    async fn test_memory_query_builder_filters_by_agent() {
        let store = InMemoryEpisodicStore::new();
        let a1 = AgentId::new();
        let a2 = AgentId::new();
        let s = SessionId::new();
        ep(&store, a1.clone(), s.clone(), EpisodeKind::Observation, vec![]).await;
        ep(&store, a2.clone(), s.clone(), EpisodeKind::Observation, vec![]).await;

        let results = MemoryQuery::new()
            .with_agent(a1.clone())
            .execute(&store)
            .await
            .unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].agent_id, a1);
    }

    #[tokio::test]
    async fn test_memory_query_builder_filters_by_tag() {
        let store = InMemoryEpisodicStore::new();
        let a = AgentId::new();
        let s = SessionId::new();
        let t = Tag::new("hot");
        ep(&store, a.clone(), s.clone(), EpisodeKind::Observation, vec![t.clone()]).await;
        ep(&store, a.clone(), s.clone(), EpisodeKind::Observation, vec![]).await;

        let results = MemoryQuery::new()
            .with_tag(t)
            .execute(&store)
            .await
            .unwrap();
        assert_eq!(results.len(), 1);
    }

    #[tokio::test]
    async fn test_memory_query_limit_caps_results() {
        let store = InMemoryEpisodicStore::new();
        let a = AgentId::new();
        let s = SessionId::new();
        for _ in 0..5 {
            ep(&store, a.clone(), s.clone(), EpisodeKind::Observation, vec![]).await;
        }
        let results = MemoryQuery::new().limit(3).execute(&store).await.unwrap();
        assert_eq!(results.len(), 3);
    }

    #[tokio::test]
    async fn test_memory_query_filters_by_kind() {
        let store = InMemoryEpisodicStore::new();
        let a = AgentId::new();
        let s = SessionId::new();
        ep(&store, a.clone(), s.clone(), EpisodeKind::Action, vec![]).await;
        ep(&store, a.clone(), s.clone(), EpisodeKind::Inference, vec![]).await;

        let results = MemoryQuery::new()
            .with_kind(EpisodeKind::Action)
            .execute(&store)
            .await
            .unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].kind, EpisodeKind::Action);
    }

    #[tokio::test]
    async fn test_memory_query_filters_by_session() {
        let store = InMemoryEpisodicStore::new();
        let a = AgentId::new();
        let s1 = SessionId::new();
        let s2 = SessionId::new();
        ep(&store, a.clone(), s1.clone(), EpisodeKind::Observation, vec![]).await;
        ep(&store, a.clone(), s2.clone(), EpisodeKind::Observation, vec![]).await;

        let results = MemoryQuery::new()
            .with_session(s1.clone())
            .execute(&store)
            .await
            .unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].session_id, s1);
    }

    #[tokio::test]
    async fn test_memory_query_since_excludes_old_episodes() {
        let store = InMemoryEpisodicStore::new();
        let a = AgentId::new();
        let s = SessionId::new();

        // Recent episode
        let recent = crate::episodic::episode::Episode::with_timestamp(
            a.clone(),
            s.clone(),
            EpisodeKind::Observation,
            "recent",
            0.8,
            vec![],
            Utc::now(),
        )
        .unwrap();
        // Old episode (10 days ago)
        let old_ts = Utc::now() - chrono::Duration::days(10);
        let old = crate::episodic::episode::Episode::with_timestamp(
            a.clone(),
            s.clone(),
            EpisodeKind::Observation,
            "old",
            0.8,
            vec![],
            old_ts,
        )
        .unwrap();

        store.record(recent).await.unwrap();
        store.record(old).await.unwrap();

        let results = MemoryQuery::new()
            .since(Duration::from_secs(60)) // only last 60 seconds
            .execute(&store)
            .await
            .unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].content, "recent");
    }
}
