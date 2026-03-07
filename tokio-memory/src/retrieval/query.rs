use serde::{Deserialize, Serialize};
use crate::types::Timestamp;

/// Kind of memory retrieval query.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum QueryKind {
    /// Exact text match.
    Exact,
    /// Fuzzy substring match.
    Fuzzy,
    /// Time range filter.
    Temporal,
    /// Associative (returns all above min_strength).
    Associative,
}

/// A memory retrieval query.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryQuery {
    /// Query kind.
    pub kind: QueryKind,
    /// Text to match.
    pub text: Option<String>,
    /// Time range filter (inclusive).
    pub time_range: Option<(Timestamp, Timestamp)>,
    /// Minimum strength threshold.
    pub min_strength: f32,
    /// Maximum results.
    pub limit: usize,
}

impl MemoryQuery {
    /// Exact text query.
    pub fn exact(text: impl Into<String>, limit: usize) -> Self {
        Self { kind: QueryKind::Exact, text: Some(text.into()), time_range: None, min_strength: 0.0, limit }
    }

    /// Fuzzy text query.
    pub fn fuzzy(text: impl Into<String>, limit: usize) -> Self {
        Self { kind: QueryKind::Fuzzy, text: Some(text.into()), time_range: None, min_strength: 0.0, limit }
    }

    /// Temporal range query.
    pub fn temporal(from: Timestamp, to: Timestamp, limit: usize) -> Self {
        Self { kind: QueryKind::Temporal, text: None, time_range: Some((from, to)), min_strength: 0.0, limit }
    }
}
