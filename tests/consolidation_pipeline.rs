//! Integration test: episodic → semantic consolidation pipeline end-to-end.

use std::sync::Arc;

use tokio_memory::consolidation::pipeline::ConsolidationPipeline;
use tokio_memory::consolidation::policy::FrequencyPolicy;
use tokio_memory::episodic::episode::{Episode, EpisodeKind};
use tokio_memory::episodic::store::{EpisodicStore, InMemoryEpisodicStore};
use tokio_memory::id::{AgentId, SessionId};
use tokio_memory::semantic::store::{InMemorySemanticStore, SemanticStore};

fn make_episode(agent: AgentId, session: SessionId, strength: f32, content: &str) -> Episode {
    Episode::new(agent, session, EpisodeKind::Observation, content, strength, vec![]).unwrap()
}

#[tokio::test]
async fn test_consolidation_pipeline_end_to_end_high_strength_episodes_become_facts() {
    let episodic = Arc::new(InMemoryEpisodicStore::new());
    let semantic = Arc::new(InMemorySemanticStore::new());

    let agent = AgentId::new();
    let session = SessionId::new();

    // 3 high-strength (should consolidate), 2 low-strength (should not)
    for content in &["alpha", "beta", "gamma"] {
        episodic
            .record(make_episode(agent.clone(), session.clone(), 0.9, content))
            .await
            .unwrap();
    }
    for content in &["low1", "low2"] {
        episodic
            .record(make_episode(agent.clone(), session.clone(), 0.5, content))
            .await
            .unwrap();
    }

    let mut pipeline = ConsolidationPipeline::new(episodic, semantic.clone());
    pipeline.add_policy(Box::new(FrequencyPolicy::new(0.7)));

    let report = pipeline.run_once().await.unwrap();
    assert_eq!(report.episodes_processed, 5);
    assert_eq!(report.facts_extracted, 3);

    let all_facts = semantic.all_facts().await.unwrap();
    assert_eq!(all_facts.len(), 3);
}

#[tokio::test]
async fn test_consolidation_pipeline_run_twice_is_idempotent_in_count() {
    let episodic = Arc::new(InMemoryEpisodicStore::new());
    let semantic = Arc::new(InMemorySemanticStore::new());

    let agent = AgentId::new();
    let session = SessionId::new();

    episodic
        .record(make_episode(agent.clone(), session.clone(), 0.9, "event"))
        .await
        .unwrap();

    let mut pipeline = ConsolidationPipeline::new(episodic, semantic.clone());
    pipeline.add_policy(Box::new(FrequencyPolicy::new(0.7)));

    let r1 = pipeline.run_once().await.unwrap();
    assert_eq!(r1.facts_extracted, 1);

    let r2 = pipeline.run_once().await.unwrap();
    assert_eq!(r2.facts_extracted, 1);

    let all = semantic.all_facts().await.unwrap();
    // Pipeline re-asserts the same fact each run; semantic store deduplicates by subject+predicate
    assert!(!all.is_empty());
}

#[tokio::test]
async fn test_consolidation_pipeline_no_episodes_produces_empty_report() {
    let episodic = Arc::new(InMemoryEpisodicStore::new());
    let semantic = Arc::new(InMemorySemanticStore::new());

    let mut pipeline = ConsolidationPipeline::new(episodic, semantic);
    pipeline.add_policy(Box::new(FrequencyPolicy::new(0.7)));

    let report = pipeline.run_once().await.unwrap();
    assert_eq!(report.episodes_processed, 0);
    assert_eq!(report.facts_extracted, 0);
}
