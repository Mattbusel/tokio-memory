//! # Module: Memory Retrieval
//!
//! ## Responsibility
//! Execute queries against memory stores using multiple retrieval strategies.
//!
//! ## Guarantees
//! - Results always respect the `limit` field

pub mod engine;
pub mod query;

pub use engine::{InMemoryRetrievalEngine, MemoryRetriever};
pub use query::{MemoryQuery, QueryKind};
