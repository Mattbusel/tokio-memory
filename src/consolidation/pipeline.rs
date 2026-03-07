//! # ConsolidationPipeline
//!
//! ## Responsibility
//! Run all registered [`ConsolidationPolicy`] instances over all episodes in
//! the episodic store, extract facts, and assert them into the semantic store.
//!
//! ## Guarantees
//! - `run_once` is idempotent in effect: re-asserting the same fact (same ID)
//!   replaces it in the semantic store.
//! - Returns a [`ConsolidationReport`] describing what was processed.

use std::sync::Arc;

use crate::episodic::store::EpisodicStore;
use crate::error::MemoryError;
use crate::semantic::store::SemanticStore;

use super::policy::ConsolidationPolicy;

/// Summary of a single consolidation run.
///
/// # Fields
/// * `episodes_processed` — Number of episodes examined.
/// * `facts_extracted` — Number of facts asserted into the semantic store.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsolidationReport {
    /// Total episodes examined.
    pub episodes_processed: usize,
    /// Total facts extracted and asserted.
    pub facts_extracted: usize,
}

/// Orchestrates episodic-to-semantic consolidation.
///
/// # Example
/// ```rust,no_run
/// use std::sync::Arc;
/// use tokio_memory::consolidation::pipeline::ConsolidationPipeline;
/// use tokio_memory::consolidation::policy::FrequencyPolicy;
/// use tokio_memory::episodic::store::InMemoryEpisodicStore;
/// use tokio_memory::semantic::store::InMemorySemanticStore;
///
/// # async fn example() {
/// let episodic = Arc::new(InMemoryEpisodicStore::new());
/// let semantic = Arc::new(InMemorySemanticStore::new());
/// let mut pipeline = ConsolidationPipeline::new(episodic, semantic);
/// pipeline.add_policy(Box::new(FrequencyPolicy::new(0.7)));
/// let report = pipeline.run_once().await.unwrap();
/// println!("Extracted {} facts", report.facts_extracted);
/// # }
/// ```
pub struct ConsolidationPipeline {
    episodic: Arc<dyn EpisodicStore>,
    semantic: Arc<dyn SemanticStore>,
    policies: Vec<Box<dyn ConsolidationPolicy>>,
}

impl ConsolidationPipeline {
    /// Create a new [`ConsolidationPipeline`] with no policies.
    ///
    /// # Panics
    /// This function never panics.
    pub fn new(
        episodic: Arc<dyn EpisodicStore>,
        semantic: Arc<dyn SemanticStore>,
    ) -> Self {
        Self {
            episodic,
            semantic,
            policies: Vec::new(),
        }
    }

    /// Register a consolidation policy.
    ///
    /// # Panics
    /// This function never panics.
    pub fn add_policy(&mut self, policy: Box<dyn ConsolidationPolicy>) {
        self.policies.push(policy);
    }

    /// Execute one consolidation pass over all episodes.
    ///
    /// For each episode, each policy is consulted. If any policy says to
    /// consolidate, all facts extracted by that policy are asserted into the
    /// semantic store.
    ///
    /// # Returns
    /// - `Ok(ConsolidationReport)` on success.
    /// - `Err(MemoryError::ConsolidationFailed)` if the episodic store fails.
    ///
    /// # Panics
    /// This function never panics.
    pub async fn run_once(&self) -> Result<ConsolidationReport, MemoryError> {
        let episodes = self.episodic.all().await.map_err(|e| {
            MemoryError::ConsolidationFailed {
                reason: e.to_string(),
            }
        })?;

        let episodes_processed = episodes.len();
        let mut facts_extracted = 0usize;

        for episode in &episodes {
            for policy in &self.policies {
                if policy.should_consolidate(episode) {
                    let facts = policy.extract_facts(episode);
                    for fact in facts {
                        self.semantic
                            .assert_fact(fact)
                            .await
                            .map_err(|e| MemoryError::ConsolidationFailed {
                                reason: e.to_string(),
                            })?;
                        facts_extracted += 1;
                    }
                }
            }
        }

        Ok(ConsolidationReport {
            episodes_processed,
            facts_extracted,
        })
    }
}

impl std::fmt::Debug for ConsolidationPipeline {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConsolidationPipeline")
            .field("policy_count", &self.policies.len())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::consolidation::policy::FrequencyPolicy;
    use crate::episodic::episode::{Episode, EpisodeKind};
    use crate::episodic::store::InMemoryEpisodicStore;
    use crate::id::{AgentId, SessionId};
    use crate::semantic::store::InMemorySemanticStore;

    async fn pipeline_with_episodes(
        episodes: Vec<Episode>,
    ) -> (ConsolidationPipeline, Arc<InMemorySemanticStore>) {
        let episodic = Arc::new(InMemoryEpisodicStore::new());
        let semantic = Arc::new(InMemorySemanticStore::new());
        for ep in episodes {
            episodic.record(ep).await.unwrap();
        }
        let sem_clone = semantic.clone();
        let mut pipeline = ConsolidationPipeline::new(episodic, semantic);
        pipeline.add_policy(Box::new(FrequencyPolicy::new(0.7)));
        (pipeline, sem_clone)
    }

    fn make_episode(strength: f32) -> Episode {
        Episode::new(
            AgentId::new(),
            SessionId::new(),
            EpisodeKind::Observation,
            "content",
            strength,
            vec![],
        )
        .unwrap()
    }

    #[tokio::test]
    async fn test_consolidation_pipeline_extracts_facts_from_episodes() {
        let episodes = vec![make_episode(0.9), make_episode(0.5)];
        let (pipeline, semantic) = pipeline_with_episodes(episodes).await;

        let report = pipeline.run_once().await.unwrap();
        // Only the 0.9-strength episode passes the 0.7 threshold
        assert_eq!(report.episodes_processed, 2);
        assert_eq!(report.facts_extracted, 1);

        let all = semantic.all_facts().await.unwrap();
        assert_eq!(all.len(), 1);
    }

    #[tokio::test]
    async fn test_consolidation_pipeline_no_policies_extracts_nothing() {
        let episodic = Arc::new(InMemoryEpisodicStore::new());
        let semantic = Arc::new(InMemorySemanticStore::new());
        episodic.record(make_episode(1.0)).await.unwrap();
        let pipeline = ConsolidationPipeline::new(episodic, semantic.clone());

        let report = pipeline.run_once().await.unwrap();
        assert_eq!(report.facts_extracted, 0);
    }

    #[tokio::test]
    async fn test_consolidation_pipeline_empty_store_produces_empty_report() {
        let episodic = Arc::new(InMemoryEpisodicStore::new());
        let semantic = Arc::new(InMemorySemanticStore::new());
        let mut pipeline = ConsolidationPipeline::new(episodic, semantic);
        pipeline.add_policy(Box::new(FrequencyPolicy::new(0.7)));

        let report = pipeline.run_once().await.unwrap();
        assert_eq!(report.episodes_processed, 0);
        assert_eq!(report.facts_extracted, 0);
    }

    #[tokio::test]
    async fn test_consolidation_pipeline_all_high_strength_all_extracted() {
        let episodes = vec![make_episode(0.9), make_episode(0.8), make_episode(1.0)];
        let (pipeline, semantic) = pipeline_with_episodes(episodes).await;

        let report = pipeline.run_once().await.unwrap();
        assert_eq!(report.episodes_processed, 3);
        assert_eq!(report.facts_extracted, 3);
        let all = semantic.all_facts().await.unwrap();
        assert_eq!(all.len(), 3);
    }

    #[tokio::test]
    async fn test_consolidation_pipeline_run_once_report_struct() {
        let (pipeline, _) = pipeline_with_episodes(vec![make_episode(0.9)]).await;
        let report = pipeline.run_once().await.unwrap();
        assert_eq!(report.episodes_processed, 1);
        assert_eq!(report.facts_extracted, 1);
    }
}
