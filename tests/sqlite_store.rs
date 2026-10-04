//! The SQLite backend: durability, parity with the in-memory store, search.

use std::collections::BTreeSet;

use chrono::{Duration, Utc};
use tokio_memory::episodic::{Episode, EpisodeKind, EpisodicStore, InMemoryEpisodicStore, TimeRange};
use tokio_memory::id::{AgentId, EntityId, SessionId, Tag};
use tokio_memory::semantic::{Fact, FactValue, InMemorySemanticStore, SemanticStore};
use tokio_memory::sqlite::SqliteMemory;

fn episode(agent: &AgentId, session: &SessionId, text: &str, minutes_ago: i64, tags: &[&str]) -> Episode {
    let mut e = Episode::new(
        agent.clone(),
        session.clone(),
        EpisodeKind::Observation,
        text,
        0.5,
        tags.iter().map(|t| Tag::new(*t)).collect(),
    )
    .unwrap();
    e.timestamp = Utc::now() - Duration::minutes(minutes_ago);
    e
}

fn ids(episodes: &[Episode]) -> Vec<String> {
    episodes.iter().map(|e| e.id.to_string()).collect()
}

fn sample() -> (AgentId, AgentId, SessionId, SessionId, Vec<Episode>) {
    let (a1, a2) = (AgentId::new(), AgentId::new());
    let (s1, s2) = (SessionId::new(), SessionId::new());
    let eps = vec![
        episode(&a1, &s1, "User asked for a flight to Lisbon", 50, &["travel", "user"]),
        episode(&a1, &s1, "Found a window seat on TAP 1234", 40, &["travel"]),
        episode(&a1, &s2, "User prefers vegetarian meals", 30, &["preference", "user"]),
        episode(&a2, &s2, "Booked a hotel near Alfama", 20, &["travel"]),
        episode(&a2, &s2, "Payment declined, retry with another card", 10, &["billing"]),
    ];
    (a1, a2, s1, s2, eps)
}

#[tokio::test]
async fn memory_survives_closing_and_reopening_the_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("memory.db");
    let (_, _, _, _, eps) = sample();
    let fact = Fact::new(EntityId::new("user"), "diet", FactValue::Text("vegetarian".into()), 0.9).unwrap();
    {
        let db = SqliteMemory::open(&path).await.unwrap();
        for e in &eps {
            db.record(e.clone()).await.unwrap();
        }
        db.assert_fact(fact.clone()).await.unwrap();
    }
    let db = SqliteMemory::open(&path).await.unwrap();
    assert_eq!(db.episode_count().await.unwrap(), 5);
    let back = db.recall(&eps[2].id).await.unwrap();
    assert_eq!(back.content, eps[2].content);
    assert_eq!(back.timestamp, eps[2].timestamp);
    let facts = db.query_subject(&EntityId::new("user")).await.unwrap();
    assert_eq!(facts.len(), 1);
    assert_eq!(facts[0].object, FactValue::Text("vegetarian".into()));
    assert!(!db.search("lisbon", 5).await.unwrap().is_empty(), "search index persisted too");
}

#[tokio::test]
async fn every_query_matches_the_in_memory_store() {
    let (a1, a2, s1, s2, eps) = sample();
    let db = SqliteMemory::in_memory().await.unwrap();
    let mem = InMemoryEpisodicStore::new();
    for e in &eps {
        db.record(e.clone()).await.unwrap();
        mem.record(e.clone()).await.unwrap();
    }
    let set = |v: Vec<Episode>| ids(&v).into_iter().collect::<BTreeSet<_>>();

    assert_eq!(set(db.all().await.unwrap()), set(mem.all().await.unwrap()));
    for agent in [&a1, &a2] {
        assert_eq!(set(db.recall_by_agent(agent).await.unwrap()), set(mem.recall_by_agent(agent).await.unwrap()));
    }
    for session in [&s1, &s2] {
        assert_eq!(set(db.recall_by_session(session).await.unwrap()), set(mem.recall_by_session(session).await.unwrap()));
    }
    for tag in ["travel", "user", "billing", "none"] {
        let tag = Tag::new(tag);
        assert_eq!(set(db.recall_by_tag(&tag).await.unwrap()), set(mem.recall_by_tag(&tag).await.unwrap()), "{tag}");
    }
    let range = TimeRange::new(Utc::now() - Duration::minutes(45), Utc::now() - Duration::minutes(15));
    let in_range = db.recall_range(&range).await.unwrap();
    assert_eq!(set(in_range.clone()), set(mem.recall_range(&range).await.unwrap()));
    assert_eq!(in_range.len(), 3);
    assert!(in_range.windows(2).all(|w| w[0].timestamp <= w[1].timestamp), "oldest first");

    let missing = tokio_memory::id::MemoryId::new();
    assert!(db.recall(&missing).await.is_err());
}

#[tokio::test]
async fn facts_behave_like_the_in_memory_store() {
    let db = SqliteMemory::in_memory().await.unwrap();
    let mem = InMemorySemanticStore::new();
    let f1 = Fact::new(EntityId::new("lisbon"), "country", FactValue::Text("Portugal".into()), 1.0).unwrap();
    let f2 = Fact::new(EntityId::new("lisbon"), "population", FactValue::Number(545_000.0), 0.8).unwrap();
    for f in [&f1, &f2] {
        db.assert_fact(f.clone()).await.unwrap();
        mem.assert_fact(f.clone()).await.unwrap();
    }
    assert_eq!(db.query_subject(&EntityId::new("lisbon")).await.unwrap().len(), 2);
    db.retract_fact(&f1.id).await.unwrap();
    mem.retract_fact(&f1.id).await.unwrap();
    assert_eq!(db.all_facts().await.unwrap().len(), mem.all_facts().await.unwrap().len());
    assert!(db.retract_fact(&f1.id).await.is_err(), "retracting twice is NotFound");
}

#[tokio::test]
async fn search_ranks_by_relevance_and_follows_updates() {
    let (_, _, _, _, mut eps) = sample();
    let db = SqliteMemory::in_memory().await.unwrap();
    for e in &eps {
        db.record(e.clone()).await.unwrap();
    }
    let hits = db.search("lisbon flight", 3).await.unwrap();
    assert_eq!(hits[0].0.id, eps[0].id);
    assert!(hits.iter().all(|(_, score)| *score > 0.0));
    assert!(db.search("submarine", 3).await.unwrap().is_empty());
    assert_eq!(db.search("\"window seat\"", 3).await.unwrap()[0].0.id, eps[1].id);
    assert_eq!(db.search("veg*", 3).await.unwrap()[0].0.id, eps[2].id);
    assert!(db.search("what's the: (weird) query?!", 3).await.is_ok(), "punctuation is safe");

    // Re-recording an episode replaces its searchable text.
    eps[3].content = "Booked a hostel in Porto".into();
    db.record(eps[3].clone()).await.unwrap();
    assert!(db.search("alfama", 3).await.unwrap().is_empty());
    assert_eq!(db.search("porto", 3).await.unwrap()[0].0.id, eps[3].id);
    assert_eq!(db.episode_count().await.unwrap(), 5, "replaced, not duplicated");

    assert!(db.forget(&eps[3].id).await.unwrap());
    assert!(db.search("porto", 3).await.unwrap().is_empty());
}

#[tokio::test]
async fn concurrent_writers_do_not_lose_episodes() {
    let db = SqliteMemory::in_memory().await.unwrap();
    let (agent, session) = (AgentId::new(), SessionId::new());
    let mut tasks = tokio::task::JoinSet::new();
    for i in 0..200 {
        let (db, agent, session) = (db.clone(), agent.clone(), session.clone());
        tasks.spawn(async move { db.record(episode(&agent, &session, &format!("event {i}"), 0, &[])).await });
    }
    while let Some(r) = tasks.join_next().await {
        r.unwrap().unwrap();
    }
    assert_eq!(db.episode_count().await.unwrap(), 200);
}
