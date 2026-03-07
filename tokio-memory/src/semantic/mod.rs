//! # Module: Semantic Memory
//!
//! ## Responsibility
//! Store structured knowledge as subject-predicate-object facts
//! and support graph traversal for associative recall.
//!
//! ## Guarantees
//! - Fact IDs are deterministic: same (s, p, o) → same ID
//! - Confidence always clamped to 0.0..=1.0

pub mod fact;
pub mod knowledge_graph;
pub mod store;

pub use fact::Fact;
pub use knowledge_graph::KnowledgeGraph;
pub use store::{InMemorySemanticStore, SemanticStore};
