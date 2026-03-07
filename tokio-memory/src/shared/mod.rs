//! # Module: Shared Memory
//!
//! ## Responsibility
//! Cross-agent memory sharing with namespace isolation and conflict resolution.

pub mod conflict;
pub mod namespace;
pub mod store;

pub use conflict::{Conflict, ConflictResolution, WriteReceipt};
pub use namespace::MemoryNamespace;
pub use store::{InMemorySharedStore, SharedMemoryStore};
