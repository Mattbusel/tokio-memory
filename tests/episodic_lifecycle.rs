//! Integration test: episodic memory lifecycle
//!
//! Records many episodes, then recalls them by range, by session, and by tag.

use chrono::{Duration, Utc};
use tokio_memory::episodic::episode::{Episode, EpisodeKind};
use tokio_memory::episodic::store::{EpisodicStore, InMemoryEpisodicStore};
use tokio_memory::episodic::timeline::TimeRange;
use tokio_memory::id::{AgentId, SessionId, Tag};

fn base_ts() -> chrono::DateTime<chrono::Utc> {
    chrono::DateTime::parse_from_rfc3339("2025-06-01T00:00:00Z")
        .unwrap()
        .with_timezone(&Utc)
}

#[tokio::test]
async fn test_episodic_lifecycle_record_many_and_recall_range() {
    let store = InMemoryEpisodicStore::new();
    let agent = AgentId::new();
    let session = SessionId::new();
    let base = base_ts();

    // Record 10 episodes spread over 10 hours
    for i in 0..10u32 {
        let ep = Episode::with_timestamp(
            agent.clone(),
            session.clone(),
            EpisodeKind::Observation,
            format!("event {i}"),
            0.8,
            vec![],
            base + Duration::hours(i as i64),
        )
        .unwrap();
        store.record(ep).await.unwrap();
    }

    // Recall only hours 2–5 (inclusive)
    let range = TimeRange::new(base + Duration::hours(2), base + Duration::hours(5));
    let results = store.recall_range(&range).await.unwrap();
    assert_eq!(results.len(), 4, "expected 4 episodes in range h2..h5");
    for ep in &results {
        assert!(ep.content.starts_with("event "));
    }
}

#[tokio::test]
async fn test_episodic_lifecycle_recall_by_session() {
    let store = InMemoryEpisodicStore::new();
    let agent = AgentId::new();
    let s1 = SessionId::new();
    let s2 = SessionId::new();

    for i in 0..5 {
        let ep = Episode::new(
            agent.clone(),
            s1.clone(),
            EpisodeKind::Action,
            format!("s1 event {i}"),
            0.7,
            vec![],
        )
        .unwrap();
        store.record(ep).await.unwrap();
    }
    for i in 0..3 {
        let ep = Episode::new(
            agent.clone(),
            s2.clone(),
            EpisodeKind::Inference,
            format!("s2 event {i}"),
            0.6,
            vec![],
        )
        .unwrap();
        store.record(ep).await.unwrap();
    }

    let s1_eps = store.recall_by_session(&s1).await.unwrap();
    assert_eq!(s1_eps.len(), 5);

    let s2_eps = store.recall_by_session(&s2).await.unwrap();
    assert_eq!(s2_eps.len(), 3);
}

#[tokio::test]
async fn test_episodic_lifecycle_recall_by_tag() {
    let store = InMemoryEpisodicStore::new();
    let agent = AgentId::new();
    let session = SessionId::new();
    let critical = Tag::new("critical");
    let routine = Tag::new("routine");

    // 3 critical, 4 routine
    for i in 0..3 {
        let ep = Episode::new(
            agent.clone(),
            session.clone(),
            EpisodeKind::Observation,
            format!("critical {i}"),
            0.9,
            vec![critical.clone()],
        )
        .unwrap();
        store.record(ep).await.unwrap();
    }
    for i in 0..4 {
        let ep = Episode::new(
            agent.clone(),
            session.clone(),
            EpisodeKind::Observation,
            format!("routine {i}"),
            0.5,
            vec![routine.clone()],
        )
        .unwrap();
        store.record(ep).await.unwrap();
    }

    let critical_eps = store.recall_by_tag(&critical).await.unwrap();
    assert_eq!(critical_eps.len(), 3);

    let routine_eps = store.recall_by_tag(&routine).await.unwrap();
    assert_eq!(routine_eps.len(), 4);
}

#[tokio::test]
async fn test_episodic_lifecycle_all_returns_all_in_insertion_order() {
    let store = InMemoryEpisodicStore::new();
    let agent = AgentId::new();
    let session = SessionId::new();

    for i in 0..7 {
        let ep = Episode::new(
            agent.clone(),
            session.clone(),
            EpisodeKind::Interaction,
            format!("msg {i}"),
            0.8,
            vec![],
        )
        .unwrap();
        store.record(ep).await.unwrap();
    }

    let all = store.all().await.unwrap();
    assert_eq!(all.len(), 7);
    for (i, ep) in all.iter().enumerate() {
        assert_eq!(ep.content, format!("msg {i}"));
    }
}
