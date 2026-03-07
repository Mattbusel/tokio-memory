//! # ConflictResolver
//!
//! Defines how the shared memory space resolves conflicts between an existing
//! versioned fact and an incoming update.

use crate::semantic::fact::Fact;

use super::space::VersionedFact;

/// The outcome of conflict resolution.
#[derive(Debug, Clone)]
pub enum Resolution {
    /// Accept the incoming fact; replace the existing one.
    Accept,
    /// Reject the incoming fact; keep the existing one.
    Reject,
    /// Merge the two facts into a new synthesised fact.
    Merge(Fact),
}

/// Strategy for resolving conflicts in the shared memory space.
///
/// Implementors receive the existing [`VersionedFact`] and the incoming
/// [`Fact`] and return a [`Resolution`].
pub trait ConflictResolver: Send + Sync + 'static {
    /// Resolve a conflict between an existing fact and an incoming update.
    ///
    /// # Arguments
    /// * `existing` — The currently stored versioned fact.
    /// * `incoming` — The new fact being asserted.
    ///
    /// # Returns
    /// A [`Resolution`] indicating what action to take.
    ///
    /// # Panics
    /// Implementations must not panic.
    fn resolve(&self, existing: &VersionedFact, incoming: &Fact) -> Resolution;
}

// ---------------------------------------------------------------------------
// LastWriteWins
// ---------------------------------------------------------------------------

/// Always accepts the incoming fact, discarding the existing one.
///
/// # Example
/// ```rust
/// use tokio_memory::shared::conflict::{LastWriteWins, ConflictResolver, Resolution};
/// ```
#[derive(Debug, Clone, Default)]
pub struct LastWriteWins;

impl ConflictResolver for LastWriteWins {
    fn resolve(&self, _existing: &VersionedFact, _incoming: &Fact) -> Resolution {
        Resolution::Accept
    }
}

// ---------------------------------------------------------------------------
// HighestConfidenceWins
// ---------------------------------------------------------------------------

/// Accepts the incoming fact only if its confidence exceeds the existing one.
///
/// On a tie the existing fact is retained (`Reject`).
///
/// # Example
/// ```rust
/// use tokio_memory::shared::conflict::{HighestConfidenceWins, ConflictResolver};
/// ```
#[derive(Debug, Clone, Default)]
pub struct HighestConfidenceWins;

impl ConflictResolver for HighestConfidenceWins {
    fn resolve(&self, existing: &VersionedFact, incoming: &Fact) -> Resolution {
        if incoming.confidence > existing.fact.confidence {
            Resolution::Accept
        } else {
            Resolution::Reject
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::id::EntityId;
    use crate::semantic::fact::{Fact, FactValue};

    fn make_fact(confidence: f32) -> Fact {
        Fact::new(
            EntityId::new("e"),
            "p",
            FactValue::Text("v".into()),
            confidence,
        )
        .unwrap()
    }

    fn versioned(fact: Fact) -> VersionedFact {
        VersionedFact { fact, version: 1 }
    }

    #[test]
    fn test_last_write_wins_always_accepts() {
        let resolver = LastWriteWins;
        let existing = versioned(make_fact(0.9));
        let incoming = make_fact(0.1);
        assert!(matches!(resolver.resolve(&existing, &incoming), Resolution::Accept));
    }

    #[test]
    fn test_highest_confidence_wins_accepts_higher_confidence_incoming() {
        let resolver = HighestConfidenceWins;
        let existing = versioned(make_fact(0.5));
        let incoming = make_fact(0.8);
        assert!(matches!(resolver.resolve(&existing, &incoming), Resolution::Accept));
    }

    #[test]
    fn test_shared_space_highest_confidence_wins_rejects_lower() {
        let resolver = HighestConfidenceWins;
        let existing = versioned(make_fact(0.9));
        let incoming = make_fact(0.3);
        assert!(matches!(resolver.resolve(&existing, &incoming), Resolution::Reject));
    }

    #[test]
    fn test_highest_confidence_wins_rejects_equal_confidence() {
        let resolver = HighestConfidenceWins;
        let existing = versioned(make_fact(0.7));
        let incoming = make_fact(0.7);
        assert!(matches!(resolver.resolve(&existing, &incoming), Resolution::Reject));
    }
}
