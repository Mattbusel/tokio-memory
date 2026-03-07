//! # Error types for tokio-memory
//!
//! All errors are named, typed, and propagatable. Every variant must have
//! at least one test that triggers it (see module-level tests below).

use crate::id::MemoryId;

/// The unified error type for all tokio-memory operations.
///
/// # Variants
/// Each variant corresponds to a distinct failure mode. Use `?` to propagate
/// these errors; do not use `unwrap()` or `expect()` in production paths.
#[derive(Debug, thiserror::Error)]
pub enum MemoryError {
    /// A memory item with the given [`MemoryId`] was not found in the store.
    #[error("Memory item '{0}' not found")]
    NotFound(MemoryId),

    /// The working memory buffer has reached its configured capacity.
    #[error("Working memory buffer is full (capacity: {capacity})")]
    BufferFull { capacity: usize },

    /// Binary serialization or deserialization failed.
    #[error("Serialization failed: {0}")]
    Serialization(String),

    /// A confidence value was outside the valid range `[0.0, 1.0]`.
    #[error("Invalid confidence value {value}: must be in [0.0, 1.0]")]
    InvalidConfidence { value: f32 },

    /// The consolidation pipeline could not process an episode into facts.
    #[error("Consolidation failed: {reason}")]
    ConsolidationFailed { reason: String },

    /// A conflict resolver rejected the incoming fact; the existing fact is retained.
    #[error("Conflict resolution rejected incoming fact")]
    ConflictRejected,

    /// A persistence-layer operation (snapshot I/O, codec) failed.
    #[error("Persistence error: {0}")]
    Persistence(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::id::MemoryId;

    #[test]
    fn test_not_found_error_displays_id() {
        let id = MemoryId::new();
        let err = MemoryError::NotFound(id.clone());
        let msg = err.to_string();
        assert!(msg.contains(&id.to_string()), "expected id in message: {msg}");
    }

    #[test]
    fn test_buffer_full_error_displays_capacity() {
        let err = MemoryError::BufferFull { capacity: 42 };
        assert!(err.to_string().contains("42"));
    }

    #[test]
    fn test_invalid_confidence_error_displays_value() {
        let err = MemoryError::InvalidConfidence { value: 1.5 };
        assert!(err.to_string().contains("1.5"));
    }

    #[test]
    fn test_consolidation_failed_error_displays_reason() {
        let err = MemoryError::ConsolidationFailed {
            reason: "no policies matched".into(),
        };
        assert!(err.to_string().contains("no policies matched"));
    }

    #[test]
    fn test_conflict_rejected_error_has_message() {
        let err = MemoryError::ConflictRejected;
        assert!(!err.to_string().is_empty());
    }

    #[test]
    fn test_persistence_error_displays_message() {
        let err = MemoryError::Persistence("disk full".into());
        assert!(err.to_string().contains("disk full"));
    }

    #[test]
    fn test_serialization_error_display() {
        let mem_err = MemoryError::Serialization("postcard: unexpected end of input".into());
        assert!(mem_err.to_string().contains("Serialization"));
    }
}
