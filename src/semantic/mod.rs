//! # Semantic memory
//!
//! Provides [`Fact`], [`FactValue`], [`SemanticStore`], and
//! [`InMemorySemanticStore`] — the subject-predicate-object triple store
//! for consolidated agent knowledge.

pub mod fact;
pub mod store;

pub use fact::{Fact, FactValue};
pub use store::{InMemorySemanticStore, SemanticStore};
