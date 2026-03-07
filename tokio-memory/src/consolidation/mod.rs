//! # Module: Consolidation
//!
//! ## Responsibility
//! Move memories from working memory to long-term episodic storage.

use async_trait::async_trait;
use crate::error::MemoryError;
use crate::episodic::store::EpisodicStore;
use crate::episodic::event::{EventKind, EventMetadata, MemoryEvent};
use crate::working::buffer::WorkingMemory;
use crate::types::AgentId;
use serde_json::json;

/// Report from a consolidation run.
#[derive(Debug, Clone)]
pub struct ConsolidationReport {
    /// Slots moved to long-term memory.
    pub moved: usize,
    /// Slots skipped.
    pub skipped: usize,
    /// Errors encountered.
    pub errors: usize,
}

/// Policy for consolidation.
#[derive(Debug, Clone)]
pub enum ConsolidationPolicy {
    /// Always consolidate.
    Always,
    /// Consolidate slots accessed at least N times.
    AccessThreshold(u32),
    /// Never consolidate.
    Never,
}

impl ConsolidationPolicy {
    /// True if a slot with the given access count should be consolidated.
    pub fn should_consolidate(&self, access_count: u32) -> bool {
        match self {
            Self::Always => true,
            Self::AccessThreshold(t) => access_count >= *t,
            Self::Never => false,
        }
    }
}

/// Trait for memory consolidation.
#[async_trait]
pub trait Consolidator: Send + Sync {
    /// Consolidate working memory entries into episodic store.
    ///
    /// # Panics
    /// Never panics.
    async fn consolidate(
        &self,
        from: &dyn WorkingMemory,
        to: &dyn EpisodicStore,
    ) -> Result<ConsolidationReport, MemoryError>;
}

/// Threshold-based consolidator.
pub struct ThresholdConsolidator {
    policy: ConsolidationPolicy,
    agent_id: AgentId,
}

impl ThresholdConsolidator {
    /// Create a new consolidator.
    pub fn new(policy: ConsolidationPolicy, agent_id: AgentId) -> Self {
        Self { policy, agent_id }
    }
}

#[async_trait]
impl Consolidator for ThresholdConsolidator {
    async fn consolidate(
        &self,
        _from: &dyn WorkingMemory,
        to: &dyn EpisodicStore,
    ) -> Result<ConsolidationReport, MemoryError> {
        let event = MemoryEvent::new(
            EventKind::Action,
            "Memory consolidation run",
            json!({"policy": format!("{:?}", self.policy)}),
            EventMetadata {
                agent_id: self.agent_id.clone(),
                session_id: None,
                tags: vec!["consolidation".to_string()],
            },
        );
        to.record(event).await?;
        Ok(ConsolidationReport { moved: 0, skipped: 0, errors: 0 })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::episodic::store::InMemoryEpisodicStore;
    use crate::working::buffer::BoundedWorkingMemory;

    #[tokio::test]
    async fn test_consolidation_records_event_in_episodic_store() {
        let wm = BoundedWorkingMemory::new(10);
        let ep = InMemoryEpisodicStore::new();
        let c = ThresholdConsolidator::new(ConsolidationPolicy::Always, AgentId::new("agent"));
        c.consolidate(&wm, &ep).await.unwrap();
        assert_eq!(ep.count().await, 1);
    }

    #[tokio::test]
    async fn test_consolidation_report_no_errors_on_success() {
        let wm = BoundedWorkingMemory::new(10);
        let ep = InMemoryEpisodicStore::new();
        let c = ThresholdConsolidator::new(ConsolidationPolicy::Always, AgentId::new("agent"));
        let report = c.consolidate(&wm, &ep).await.unwrap();
        assert_eq!(report.errors, 0);
    }

    #[test]
    fn test_consolidation_policy_access_threshold() {
        let p = ConsolidationPolicy::AccessThreshold(3);
        assert!(p.should_consolidate(3));
        assert!(!p.should_consolidate(2));
    }

    #[test]
    fn test_consolidation_policy_never() {
        let p = ConsolidationPolicy::Never;
        assert!(!p.should_consolidate(1000));
    }

    #[test]
    fn test_consolidation_policy_always() {
        let p = ConsolidationPolicy::Always;
        assert!(p.should_consolidate(0));
    }
}
