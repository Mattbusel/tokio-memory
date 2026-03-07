# tokio-memory

Agent memory primitives — episodic, semantic, and working memory with decay and consolidation, built for Tokio async runtimes.

A higher-level memory system for AI agents that need to remember, forget, and recall information across long-running sessions.

## What's inside

- **EpisodicMemory** — time-ordered event log with temporal and tag-based recall
- **SemanticMemory** — concept store with typed assertions and relation traversal
- **WorkingMemory** — bounded priority slots for active context; evicts on overflow
- **DecayScheduler** — configurable memory decay (linear or exponential) with forget thresholds
- **Consolidator** — background process that promotes working memory to long-term storage
- **MemoryBus** — publish/subscribe channel for cross-agent memory sharing

## Features

- Async-first — all operations are `async` and safe under concurrent agent access
- Decay-aware — memories fade over time unless reinforced
- Consolidation pipeline — short-term → long-term promotion based on importance scores
- Pluggable storage — swap the backend by implementing the `MemoryStore` trait

## Quick start

```rust
use tokio_memory::{MemorySystem, MemoryConfig};

#[tokio::main]
async fn main() {
    let memory = MemorySystem::new(MemoryConfig::default());

    memory.remember("user asked about Paris", &["geography", "user"]).await.unwrap();

    let results = memory.recall_by_tag("geography").await.unwrap();
    println!("Found {} memories", results.len());
}
```

## Add to your project

```toml
[dependencies]
tokio-memory = { git = "https://github.com/Mattbusel/tokio-memory" }
tokio = { version = "1", features = ["full"] }
```

## Test coverage

```bash
cargo test
```

---

> Used inside [tokio-prompt-orchestrator](https://github.com/Mattbusel/tokio-prompt-orchestrator) -- a production Rust orchestration layer for LLM pipelines. See the full [primitive library collection](https://github.com/Mattbusel/rust-crates).