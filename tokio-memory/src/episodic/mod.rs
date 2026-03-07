//! # Module: Episodic Memory
//!
//! ## Responsibility
//! Store and retrieve temporal sequences of events (what happened, when, why).
//!
//! ## Guarantees
//! - Thread-safe via DashMap
//! - recall_recent returns episodes in reverse chronological order
//! - Non-blocking async operations

pub mod episode;
pub mod event;
pub mod store;

pub use episode::{Episode, EpisodeId};
pub use event::{EventKind, EventMetadata, MemoryEvent};
pub use store::{EpisodicStore, InMemoryEpisodicStore};
