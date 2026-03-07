//! # ConsolidationPolicy
//!
//! Defines when and how episodes are promoted into semantic facts.

use crate::episodic::episode::Episode;
use crate::semantic::fact::Fact;

/// Determines whether an episode should be consolidated and how to extract
/// facts from it.
///
/// ## Guarantees
/// - `should_consolidate` must be pure and not mutate state.
/// - `extract_facts` must not panic.
pub trait ConsolidationPolicy: Send + Sync + 'static {
    /// Return `true` if the given episode should be consolidated into facts.
    ///
    /// # Panics
    /// Implementations must not panic.
    fn should_consolidate(&self, episode: &Episode) -> bool;

    /// Extract semantic facts from the given episode.
    ///
    /// Called only when `should_consolidate` returns `true`.
    ///
    /// # Panics
    /// Implementations must not panic.
    fn extract_facts(&self, episode: &Episode) -> Vec<Fact>;
}

// ---------------------------------------------------------------------------
// Built-in: FrequencyPolicy
// ---------------------------------------------------------------------------

/// Consolidates episodes whose `strength` exceeds a configured threshold.
///
/// By default the threshold is `0.7`.
///
/// # Example
/// ```rust
/// use tokio_memory::consolidation::policy::FrequencyPolicy;
/// let policy = FrequencyPolicy::new(0.7);
/// ```
#[derive(Debug, Clone)]
pub struct FrequencyPolicy {
    /// Episodes with `strength > threshold` will be consolidated.
    pub threshold: f32,
}

impl FrequencyPolicy {
    /// Create a new [`FrequencyPolicy`] with the given strength threshold.
    ///
    /// # Panics
    /// This function never panics.
    pub fn new(threshold: f32) -> Self {
        Self { threshold }
    }
}

impl ConsolidationPolicy for FrequencyPolicy {
    fn should_consolidate(&self, episode: &Episode) -> bool {
        episode.strength > self.threshold
    }

    fn extract_facts(&self, episode: &Episode) -> Vec<Fact> {
        use crate::id::EntityId;
        use crate::semantic::fact::FactValue;

        // Derive a stable entity ID from the agent.
        let subject = EntityId::new(format!("agent:{}", episode.agent_id));

        // Produce one fact: the agent performed/observed this episode content.
        let predicate = format!("{:?}", episode.kind).to_lowercase();
        let fact = Fact::new(
            subject,
            predicate,
            FactValue::Text(episode.content.clone()),
            episode.strength,
        );

        match fact {
            Ok(f) => vec![f],
            Err(_) => vec![], // strength already validated, this branch is unreachable in practice
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::episodic::episode::{Episode, EpisodeKind};
    use crate::id::{AgentId, SessionId};

    fn make_episode(strength: f32) -> Episode {
        Episode::new(
            AgentId::new(),
            SessionId::new(),
            EpisodeKind::Observation,
            "test content",
            strength,
            vec![],
        )
        .unwrap()
    }

    #[test]
    fn test_frequency_policy_consolidates_above_threshold() {
        let policy = FrequencyPolicy::new(0.7);
        let ep = make_episode(0.9);
        assert!(policy.should_consolidate(&ep));
    }

    #[test]
    fn test_frequency_policy_does_not_consolidate_at_threshold() {
        let policy = FrequencyPolicy::new(0.7);
        let ep = make_episode(0.7);
        assert!(!policy.should_consolidate(&ep)); // strictly greater than threshold
    }

    #[test]
    fn test_frequency_policy_does_not_consolidate_below_threshold() {
        let policy = FrequencyPolicy::new(0.7);
        let ep = make_episode(0.5);
        assert!(!policy.should_consolidate(&ep));
    }

    #[test]
    fn test_frequency_policy_extract_facts_returns_one_fact() {
        let policy = FrequencyPolicy::new(0.7);
        let ep = make_episode(0.9);
        let facts = policy.extract_facts(&ep);
        assert_eq!(facts.len(), 1);
    }

    #[test]
    fn test_frequency_policy_extracted_fact_content_matches_episode() {
        let policy = FrequencyPolicy::new(0.7);
        let ep = make_episode(0.9);
        let facts = policy.extract_facts(&ep);
        use crate::semantic::fact::FactValue;
        assert_eq!(facts[0].object, FactValue::Text("test content".into()));
    }
}
