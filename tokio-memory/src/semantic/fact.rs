use chrono::Utc;
use serde::{Deserialize, Serialize};
use crate::types::{AgentId, MemoryId, Timestamp};

/// A structured fact in semantic memory (subject-predicate-object).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fact {
    /// Unique identifier (deterministic from subject+predicate+object).
    pub id: MemoryId,
    /// The subject entity.
    pub subject: String,
    /// The relationship type.
    pub predicate: String,
    /// The object entity.
    pub object: String,
    /// Confidence in this fact (0.0..=1.0).
    pub confidence: f32,
    /// Agent that asserted this fact.
    pub source: AgentId,
    /// When asserted.
    pub asserted_at: Timestamp,
}

impl Fact {
    /// Create a new Fact with a deterministic ID.
    ///
    /// # Arguments
    /// * `subject` — Subject entity.
    /// * `predicate` — Relationship type.
    /// * `object` — Object entity.
    /// * `confidence` — Confidence score (clamped to 0.0..=1.0).
    /// * `source` — Asserting agent.
    ///
    /// # Panics
    /// Never panics.
    pub fn new(
        subject: impl Into<String>,
        predicate: impl Into<String>,
        object: impl Into<String>,
        confidence: f32,
        source: AgentId,
    ) -> Self {
        let subject = subject.into();
        let predicate = predicate.into();
        let object = object.into();
        let id_input = format!("{}{}{}", subject, predicate, object);
        Self {
            id: MemoryId::from_content(id_input.as_bytes()),
            subject,
            predicate,
            object,
            confidence: confidence.clamp(0.0, 1.0),
            source,
            asserted_at: Utc::now(),
        }
    }
}
