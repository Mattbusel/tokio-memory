//! # Retrieval
//!
//! Provides [`MemoryQuery`] — a fluent builder for filtering and retrieving
//! episodes from an [`crate::episodic::store::EpisodicStore`].

pub mod query;

pub use query::MemoryQuery;
