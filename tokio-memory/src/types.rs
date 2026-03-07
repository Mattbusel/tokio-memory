use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;

/// Unique identifier for a memory entry, derived from content hash.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MemoryId(pub String);

impl MemoryId {
    /// Create a MemoryId by hashing content bytes with blake3.
    ///
    /// # Panics
    /// Never panics.
    pub fn from_content(content: &[u8]) -> Self {
        let hash = blake3::hash(content);
        Self(hash.to_hex().to_string())
    }

    /// Create a MemoryId from a raw string.
    pub fn from_string(s: String) -> Self {
        Self(s)
    }
}

impl fmt::Display for MemoryId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Identifies an agent in the system.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AgentId(pub String);

impl AgentId {
    /// Create an AgentId.
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

impl fmt::Display for AgentId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// UTC timestamp.
pub type Timestamp = DateTime<Utc>;

/// A memory entry stored in any memory tier.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    /// Unique identifier.
    pub id: MemoryId,
    /// Arbitrary JSON content.
    pub content: serde_json::Value,
    /// When created.
    pub created_at: Timestamp,
    /// When last accessed.
    pub accessed_at: Timestamp,
    /// Access count.
    pub access_count: u32,
    /// Memory strength (0.0..=1.0).
    pub strength: f32,
    /// Optional text for search.
    pub text: Option<String>,
    /// Source agent.
    pub source_agent: Option<AgentId>,
}

impl MemoryEntry {
    /// Create a new MemoryEntry with full strength.
    ///
    /// # Panics
    /// Never panics.
    pub fn new(content: serde_json::Value, text: Option<String>, source: Option<AgentId>) -> Self {
        let now = Utc::now();
        let id = MemoryId::from_content(content.to_string().as_bytes());
        Self {
            id,
            content,
            created_at: now,
            accessed_at: now,
            access_count: 0,
            strength: 1.0,
            text,
            source_agent: source,
        }
    }

    /// Record an access, updating accessed_at and access_count.
    pub fn record_access(&mut self) {
        self.accessed_at = Utc::now();
        self.access_count = self.access_count.saturating_add(1);
    }
}
