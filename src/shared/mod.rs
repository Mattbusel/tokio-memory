//! # Shared memory space
//!
//! Provides [`SharedSpace`], [`VersionedFact`], [`ConflictResolver`],
//! [`LastWriteWins`], and [`HighestConfidenceWins`] — the primitives for
//! multi-agent collaborative memory with pluggable conflict resolution.

pub mod conflict;
pub mod space;

pub use conflict::{ConflictResolver, HighestConfidenceWins, LastWriteWins, Resolution};
pub use space::{SharedSpace, VersionedFact};
