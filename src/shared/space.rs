//! # SharedSpace
//!
//! ## Responsibility
//! A namespaced key-value store where each value is a [`VersionedFact`].
//! Conflict resolution is pluggable via the [`ConflictResolver`] trait.
//!
//! ## Guarantees
//! - Thread-safe via [`DashMap`].
//! - Version counter is monotonically increasing per key.
//! - `write` delegates to the resolver; on `Accept` the version is incremented.

use dashmap::DashMap;

use crate::error::MemoryError;
use crate::id::NamespaceId;
use crate::semantic::fact::Fact;

use super::conflict::{ConflictResolver, Resolution};

/// A fact stored with a version counter for optimistic conflict detection.
#[derive(Debug, Clone)]
pub struct VersionedFact {
    /// The stored fact.
    pub fact: Fact,
    /// Monotonically increasing version number.
    pub version: u64,
}

/// A namespaced shared memory space.
///
/// Multiple agents can read and write facts into the same namespace.
/// Conflict resolution is determined by the [`ConflictResolver`] passed to
/// [`SharedSpace::write`].
///
/// # Example
/// ```rust
/// use tokio_memory::shared::space::SharedSpace;
/// use tokio_memory::shared::conflict::LastWriteWins;
/// use tokio_memory::id::NamespaceId;
///
/// let space = SharedSpace::new(NamespaceId::new("global"));
/// ```
#[derive(Debug)]
pub struct SharedSpace {
    /// The namespace this space belongs to.
    pub namespace: NamespaceId,
    store: DashMap<String, VersionedFact>,
}

impl SharedSpace {
    /// Create a new, empty [`SharedSpace`] in the given namespace.
    ///
    /// # Panics
    /// This function never panics.
    pub fn new(namespace: NamespaceId) -> Self {
        Self {
            namespace,
            store: DashMap::new(),
        }
    }

    /// Write a fact under `key`, using `resolver` to handle conflicts.
    ///
    /// If no fact exists under `key` the incoming fact is accepted
    /// unconditionally (version 1).
    ///
    /// # Returns
    /// - `Ok(())` if the fact was accepted or merged.
    /// - `Err(MemoryError::ConflictRejected)` if the resolver rejected the
    ///   incoming fact.
    ///
    /// # Panics
    /// This function never panics.
    pub fn write(
        &self,
        key: impl Into<String>,
        incoming: Fact,
        resolver: &dyn ConflictResolver,
    ) -> Result<(), MemoryError> {
        let key = key.into();
        match self.store.get(&key) {
            None => {
                // First write: accept unconditionally.
                self.store.insert(key, VersionedFact { fact: incoming, version: 1 });
                Ok(())
            }
            Some(existing) => {
                match resolver.resolve(&existing, &incoming) {
                    Resolution::Accept => {
                        let new_version = existing.version + 1;
                        drop(existing);
                        self.store.insert(key, VersionedFact { fact: incoming, version: new_version });
                        Ok(())
                    }
                    Resolution::Reject => Err(MemoryError::ConflictRejected),
                    Resolution::Merge(merged) => {
                        let new_version = existing.version + 1;
                        drop(existing);
                        self.store.insert(key, VersionedFact { fact: merged, version: new_version });
                        Ok(())
                    }
                }
            }
        }
    }

    /// Read the current versioned fact stored under `key`.
    ///
    /// # Returns
    /// - `Ok(VersionedFact)` if a fact exists.
    /// - `Err(MemoryError::NotFound)` if no fact is stored under `key`.
    ///
    /// # Panics
    /// This function never panics.
    pub fn read(&self, key: &str) -> Result<VersionedFact, MemoryError> {
        self.store
            .get(key)
            .map(|v| v.clone())
            .ok_or_else(|| {
                // Use a synthetic MemoryId for the NotFound error key.
                MemoryError::Persistence(format!("key '{key}' not found in namespace '{}'", self.namespace))
            })
    }

    /// Return the number of facts stored in this space.
    ///
    /// # Panics
    /// This function never panics.
    pub fn len(&self) -> usize {
        self.store.len()
    }

    /// Return `true` if no facts are stored.
    ///
    /// # Panics
    /// This function never panics.
    pub fn is_empty(&self) -> bool {
        self.store.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::id::{EntityId, NamespaceId};
    use crate::semantic::fact::{Fact, FactValue};
    use crate::shared::conflict::{HighestConfidenceWins, LastWriteWins};

    fn make_fact(confidence: f32) -> Fact {
        Fact::new(
            EntityId::new("e"),
            "p",
            FactValue::Text("v".into()),
            confidence,
        )
        .unwrap()
    }

    #[test]
    fn test_shared_space_first_write_accepted_unconditionally() {
        let space = SharedSpace::new(NamespaceId::new("ns"));
        let resolver = LastWriteWins;
        assert!(space.write("k", make_fact(0.5), &resolver).is_ok());
        assert_eq!(space.len(), 1);
    }

    #[test]
    fn test_shared_space_last_write_wins_accepts_incoming() {
        let space = SharedSpace::new(NamespaceId::new("ns"));
        let resolver = LastWriteWins;
        space.write("k", make_fact(0.9), &resolver).unwrap();
        let result = space.write("k", make_fact(0.1), &resolver);
        assert!(result.is_ok());
        let vf = space.read("k").unwrap();
        assert!((vf.fact.confidence - 0.1).abs() < 1e-5);
        assert_eq!(vf.version, 2);
    }

    #[test]
    fn test_shared_space_highest_confidence_wins_accepts_higher() {
        let space = SharedSpace::new(NamespaceId::new("ns"));
        let resolver = HighestConfidenceWins;
        space.write("k", make_fact(0.5), &resolver).unwrap();
        let result = space.write("k", make_fact(0.9), &resolver);
        assert!(result.is_ok());
        let vf = space.read("k").unwrap();
        assert!((vf.fact.confidence - 0.9).abs() < 1e-5);
    }

    #[test]
    fn test_shared_space_highest_confidence_wins_rejects_lower_confidence() {
        let space = SharedSpace::new(NamespaceId::new("ns"));
        let resolver = HighestConfidenceWins;
        space.write("k", make_fact(0.9), &resolver).unwrap();
        let result = space.write("k", make_fact(0.3), &resolver);
        assert!(matches!(result, Err(MemoryError::ConflictRejected)));
    }

    #[test]
    fn test_shared_space_read_missing_key_returns_error() {
        let space = SharedSpace::new(NamespaceId::new("ns"));
        let result = space.read("nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_shared_space_version_increments_on_accept() {
        let space = SharedSpace::new(NamespaceId::new("ns"));
        let resolver = LastWriteWins;
        space.write("k", make_fact(0.5), &resolver).unwrap();
        space.write("k", make_fact(0.6), &resolver).unwrap();
        space.write("k", make_fact(0.7), &resolver).unwrap();
        let vf = space.read("k").unwrap();
        assert_eq!(vf.version, 3);
    }

    #[test]
    fn test_shared_space_is_empty_initially() {
        let space = SharedSpace::new(NamespaceId::new("ns"));
        assert!(space.is_empty());
    }
}
