//! Integration test: snapshot encode → decode, all data preserved.

use tokio_memory::episodic::episode::{Episode, EpisodeKind};
use tokio_memory::id::{AgentId, EntityId, SessionId, Tag};
use tokio_memory::persistence::{decode_snapshot, encode_snapshot, MemorySnapshot};
use tokio_memory::semantic::fact::{Fact, FactValue};

fn make_episode(agent: &AgentId, session: &SessionId, content: &str) -> Episode {
    Episode::new(
        agent.clone(),
        session.clone(),
        EpisodeKind::Interaction,
        content,
        0.85,
        vec![Tag::new("integration-test")],
    )
    .unwrap()
}

fn make_fact(subject: &str, predicate: &str, value: &str) -> Fact {
    Fact::new(
        EntityId::new(subject),
        predicate,
        FactValue::Text(value.into()),
        0.95,
    )
    .unwrap()
}

#[test]
fn test_persistence_roundtrip_all_data_preserved() {
    let agent = AgentId::new();
    let session = SessionId::new();

    let episodes: Vec<Episode> = (0..5)
        .map(|i| make_episode(&agent, &session, &format!("event {i}")))
        .collect();

    let facts: Vec<Fact> = vec![
        make_fact("user:1", "name", "Alice"),
        make_fact("user:2", "role", "admin"),
    ];

    let snap = MemorySnapshot::new(agent.clone(), episodes, facts);
    let bytes = encode_snapshot(&snap).unwrap();
    let decoded = decode_snapshot(&bytes).unwrap();

    assert_eq!(decoded.agent_id, agent);
    assert_eq!(decoded.episodes.len(), 5);
    assert_eq!(decoded.facts.len(), 2);

    for (i, ep) in decoded.episodes.iter().enumerate() {
        assert_eq!(ep.content, format!("event {i}"));
        assert_eq!(ep.tags.len(), 1);
        assert_eq!(ep.tags[0].as_str(), "integration-test");
    }

    let names: Vec<&str> = decoded.facts.iter().map(|f| f.predicate.as_str()).collect();
    assert!(names.contains(&"name"));
    assert!(names.contains(&"role"));
}

#[test]
fn test_persistence_roundtrip_empty_snapshot_preserved() {
    let agent = AgentId::new();
    let snap = MemorySnapshot::new(agent.clone(), vec![], vec![]);
    let bytes = encode_snapshot(&snap).unwrap();
    let decoded = decode_snapshot(&bytes).unwrap();
    assert_eq!(decoded.agent_id, agent);
    assert!(decoded.episodes.is_empty());
    assert!(decoded.facts.is_empty());
}

#[test]
fn test_persistence_roundtrip_episode_strength_preserved() {
    let agent = AgentId::new();
    let session = SessionId::new();
    let ep = Episode::new(
        agent.clone(),
        session,
        EpisodeKind::Observation,
        "precise strength",
        0.732,
        vec![],
    )
    .unwrap();
    let snap = MemorySnapshot::new(agent, vec![ep], vec![]);
    let bytes = encode_snapshot(&snap).unwrap();
    let decoded = decode_snapshot(&bytes).unwrap();
    assert!((decoded.episodes[0].strength - 0.732).abs() < 1e-5);
}

#[test]
fn test_persistence_decode_bad_bytes_returns_error() {
    let result = decode_snapshot(b"\xFF\xFE\x00\x01bad bytes");
    assert!(result.is_err());
}
