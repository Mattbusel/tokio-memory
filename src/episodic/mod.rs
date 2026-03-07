//! # Episodic memory
//!
//! Provides [`Episode`], [`EpisodicStore`], [`InMemoryEpisodicStore`], and
//! [`TimeRange`] — the primitives for recording and recalling time-ordered
//! agent events.

pub mod episode;
pub mod store;
pub mod timeline;

pub use episode::{Episode, EpisodeKind};
pub use store::{EpisodicStore, InMemoryEpisodicStore};
pub use timeline::TimeRange;
