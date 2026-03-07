use chrono::Utc;
use serde::{Deserialize, Serialize};
use crate::types::{AgentId, MemoryId, Timestamp};

/// The kind of event recorded in episodic memory.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum EventKind {
    /// Agent executed an action.
    Action,
    /// Agent received an observation.
    Observation,
    /// Agent made a decision.
    Decision,
    /// Agent communicated with another agent.
    Communication,
    /// An error or unexpected event.
    Error,
    /// Custom event kind.
    Custom(String),
}

/// Metadata for a memory event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventMetadata {
    /// Agent that generated this event.
    pub agent_id: AgentId,
    /// Optional session identifier.
    pub session_id: Option<String>,
    /// Tags for filtering.
    pub tags: Vec<String>,
}

/// A single event recorded in episodic memory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEvent {
    /// Unique identifier.
    pub id: MemoryId,
    /// Event kind.
    pub kind: EventKind,
    /// Human-readable description.
    pub description: String,
    /// Structured event data.
    pub data: serde_json::Value,
    /// When the event occurred.
    pub timestamp: Timestamp,
    /// Event metadata.
    pub metadata: EventMetadata,
}

impl MemoryEvent {
    /// Create a new MemoryEvent with a generated ID.
    ///
    /// # Panics
    /// Never panics.
    pub fn new(
        kind: EventKind,
        description: impl Into<String>,
        data: serde_json::Value,
        metadata: EventMetadata,
    ) -> Self {
        let description = description.into();
        let now = Utc::now();
        let ns = now.timestamp_nanos_opt().unwrap_or_else(|| now.timestamp() * 1_000_000_000);
        let id_input = format!("{}{}{}", description, ns, data);
        Self {
            id: MemoryId::from_content(id_input.as_bytes()),
            kind,
            description,
            data,
            timestamp: now,
            metadata,
        }
    }
}
