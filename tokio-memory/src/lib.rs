//! # tokio-memory
//!
//! Agent memory primitives for production multi-agent systems.
//! Provides episodic, semantic, working memory, consolidation,
//! retrieval, decay, shared memory, and persistence.

pub mod consolidation;
pub mod decay;
pub mod episodic;
pub mod error;
pub mod persistence;
pub mod retrieval;
pub mod semantic;
pub mod shared;
pub mod types;
pub mod working;

pub use error::MemoryError;
pub use types::{AgentId, MemoryEntry, MemoryId, Timestamp};
