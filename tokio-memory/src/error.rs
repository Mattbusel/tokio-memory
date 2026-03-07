use thiserror::Error;
use crate::types::MemoryId;

/// Errors from tokio-memory operations.
#[derive(Debug, Error)]
pub enum MemoryError {
    /// Memory entry not found.
    #[error("Memory entry '{0}' not found")]
    NotFound(MemoryId),
    /// Working memory at capacity.
    #[error("Working memory capacity exceeded: {capacity} slots full")]
    CapacityExceeded { capacity: usize },
    /// Serialization failure.
    #[error("Serialization failed: {source}")]
    Serialization {
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },
    /// Conflict resolution failure.
    #[error("Conflict resolution failed for key '{key}': {reason}")]
    ConflictResolution { key: String, reason: String },
    /// Namespace not found.
    #[error("Namespace '{namespace}' not found")]
    NamespaceNotFound { namespace: String },
    /// Decay model error.
    #[error("Decay model error: {0}")]
    DecayError(String),
    /// Backend error.
    #[error("Backend error: {0}")]
    Backend(String),
    /// Invalid argument.
    #[error("Invalid argument: {0}")]
    InvalidArgument(String),
}
