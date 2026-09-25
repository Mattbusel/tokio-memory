# tokio-memory

[![CI](https://github.com/Mattbusel/tokio-memory/actions/workflows/ci.yml/badge.svg)](https://github.com/Mattbusel/tokio-memory/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/tokio-memory.svg)](https://crates.io/crates/tokio-memory)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

Trait-based memory building blocks for async Rust agents: an episodic store, a subject-predicate-object fact store, a query builder, Ebbinghaus-style decay, a pluggable episode-to-fact consolidation pipeline, a shared fact space with conflict resolution, and compact binary snapshots.

Each store is an `async_trait` (`EpisodicStore`, `SemanticStore`), so you can start with the in-memory implementations and later back them with Postgres, SQLite or a vector DB without changing agent code. Strongly typed ids (`AgentId`, `SessionId`, `MemoryId`, `EntityId`, `Tag`) keep agents and sessions from getting mixed up.

## Features

| Module | What it does |
|---|---|
| `episodic` | `Episode` (agent, session, `EpisodeKind`: Observation / Action / Inference / Interaction, content, strength, tags); `EpisodicStore` trait with `record`, `recall`, `recall_range`, `recall_by_session`, `recall_by_tag`, `recall_by_agent`; `InMemoryEpisodicStore` |
| `semantic` | `Fact` triples (`EntityId`, predicate, `FactValue::Text / Number / Bool`, confidence); `SemanticStore` trait with `assert_fact`, `retract_fact`, `query_subject`; `InMemorySemanticStore` |
| `retrieval` | `MemoryQuery` builder: filter by agent, session, tag, kind, `since(duration)`, `limit` |
| `decay` | `DecayPolicy` trait and `ExponentialDecay` with a configurable half-life |
| `consolidation` | `ConsolidationPolicy` trait (decide and extract facts), `FrequencyPolicy` (strength threshold), `ConsolidationPipeline::run_once` |
| `working` | `WorkingBuffer`: bounded priority buffer, `pop_highest` with FIFO tie-breaking |
| `shared` | `SharedSpace`: namespaced, versioned facts with a `ConflictResolver` (`LastWriteWins`, `HighestConfidenceWins`, or your own) |
| `persistence` | `MemorySnapshot` and `encode_snapshot` / `decode_snapshot` using `postcard` |

Strength and confidence are validated to `[0.0, 1.0]` on construction, and property tests (`proptest`) cover decay bounds and snapshot round-trips.

## Quick start

```bash
cargo add tokio-memory
cargo add tokio --features full
```

```rust
use std::sync::Arc;
use std::time::Duration;
use tokio_memory::consolidation::{ConsolidationPipeline, FrequencyPolicy};
use tokio_memory::episodic::{Episode, EpisodeKind, EpisodicStore, InMemoryEpisodicStore};
use tokio_memory::persistence::{decode_snapshot, encode_snapshot, MemorySnapshot};
use tokio_memory::retrieval::MemoryQuery;
use tokio_memory::semantic::{InMemorySemanticStore, SemanticStore};
use tokio_memory::{AgentId, MemoryError, SessionId, Tag};

#[tokio::main]
async fn main() -> Result<(), MemoryError> {
    let agent = AgentId::new();
    let session = SessionId::new();
    let episodic = Arc::new(InMemoryEpisodicStore::new());
    let semantic = Arc::new(InMemorySemanticStore::new());

    episodic.record(Episode::new(agent.clone(), session.clone(), EpisodeKind::Interaction,
        "user said they are vegetarian", 0.9, vec![Tag::new("preference")])?).await?;
    episodic.record(Episode::new(agent.clone(), session.clone(), EpisodeKind::Action,
        "opened the restaurants page", 0.3, vec![Tag::new("browsing")])?).await?;

    // Recall: last hour, this agent, tagged "preference".
    let prefs = MemoryQuery::new()
        .with_agent(agent.clone())
        .with_tag(Tag::new("preference"))
        .since(Duration::from_secs(3600))
        .limit(10)
        .execute(episodic.as_ref())
        .await?;
    println!("{} preference episode(s)", prefs.len());

    // Consolidate strong episodes into facts.
    let mut pipeline = ConsolidationPipeline::new(episodic.clone(), semantic.clone());
    pipeline.add_policy(Box::new(FrequencyPolicy::new(0.7)));
    let report = pipeline.run_once().await?;
    println!("{} episodes -> {} facts", report.episodes_processed, report.facts_extracted);

    // Snapshot everything to bytes and back.
    let snap = MemorySnapshot::new(agent, episodic.all().await?, semantic.all_facts().await?);
    let bytes = encode_snapshot(&snap)?;
    println!("snapshot: {} bytes, {} episodes", bytes.len(), decode_snapshot(&bytes)?.episodes.len());
    Ok(())
}
```

## How it works

```
src/
  id.rs              MemoryId, AgentId, SessionId, NamespaceId, EntityId, Tag
  episodic/          Episode, EpisodicStore, InMemoryEpisodicStore (DashMap + insertion order), TimeRange
  semantic/          Fact, FactValue, SemanticStore, InMemorySemanticStore
  retrieval/query.rs MemoryQuery builder
  decay/             DecayPolicy, ExponentialDecay
  consolidation/     ConsolidationPolicy, FrequencyPolicy, ConsolidationPipeline
  working/buffer.rs  WorkingBuffer
  shared/            SharedSpace, VersionedFact, ConflictResolver implementations
  persistence/       MemorySnapshot, postcard codec
```

## Status and limitations

Version 0.1. Only in-memory store implementations ship with the crate. Retrieval filters on metadata (agent, session, tag, kind, time); there is no text or embedding search. Decay policies compute strength but do not prune stores on their own. The `tokio-memory/` subfolder in the repository is an older copy of the crate and is not part of the build.

For a sibling crate with causal event chains, a concept graph, fuzzy retrieval and a multi-agent broadcast bus, see [tokio-agent-memory](https://github.com/Mattbusel/tokio-agent-memory).

```bash
cargo test
```

## License

MIT, see [LICENSE](LICENSE).

---

Part of a set of Rust crates for LLM agents, see [rust-crates](https://github.com/Mattbusel/rust-crates).
