//! # Episode
//!
//! An [`Episode`] is a single timestamped memory event recorded by an agent.
//! Episodes form the raw substrate of episodic memory before consolidation
//! into semantic facts.
//!
//! ## Guarantees
//! - `strength` is validated in the range `[0.0, 1.0]` at construction time
//!   via [`Episode::new`].
//! - All fields are fully serializable for persistence.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::error::MemoryError;
use crate::id::{AgentId, MemoryId, SessionId, Tag};

/// Classifies the nature of an [`Episode`].
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EpisodeKind {
    /// A passive observation made by the agent (sensory input, log read, etc.).
    Observation,
    /// An action performed by the agent.
    Action,
    /// A conclusion drawn from existing knowledge.
    Inference,
    /// An exchange with a user or another agent.
    Interaction,
}

/// A single episodic memory event.
///
/// # Fields
/// * `id` — Unique identifier for this episode.
/// * `agent_id` — The agent that recorded this episode.
/// * `session_id` — The session during which this episode occurred.
/// * `kind` — The nature of the episode (observation, action, etc.).
/// * `content` — Free-text description of what happened.
/// * `timestamp` — When the episode was recorded (UTC).
/// * `strength` — Memory strength in `[0.0, 1.0]`; decays over time.
/// * `tags` — Classification tags for retrieval filtering.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Episode {
    /// Unique identifier.
    pub id: MemoryId,
    /// Owning agent.
    pub agent_id: AgentId,
    /// Session scope.
    pub session_id: SessionId,
    /// Episode classification.
    pub kind: EpisodeKind,
    /// Textual content.
    pub content: String,
    /// Creation timestamp (UTC).
    pub timestamp: DateTime<Utc>,
    /// Memory strength in `[0.0, 1.0]`.
    pub strength: f32,
    /// Classification tags.
    pub tags: Vec<Tag>,
}

impl Episode {
    /// Construct a new [`Episode`], validating `strength` ∈ `[0.0, 1.0]`.
    ///
    /// # Arguments
    /// * `agent_id` — The recording agent.
    /// * `session_id` — The active session.
    /// * `kind` — Classification of the event.
    /// * `content` — Free-text description.
    /// * `strength` — Initial memory strength in `[0.0, 1.0]`.
    /// * `tags` — Classification tags.
    ///
    /// # Returns
    /// - `Ok(Episode)` on success.
    /// - `Err(MemoryError::InvalidConfidence)` if `strength` is outside `[0.0, 1.0]`.
    ///
    /// # Panics
    /// This function never panics.
    pub fn new(
        agent_id: AgentId,
        session_id: SessionId,
        kind: EpisodeKind,
        content: impl Into<String>,
        strength: f32,
        tags: Vec<Tag>,
    ) -> Result<Self, MemoryError> {
        if !(0.0..=1.0).contains(&strength) {
            return Err(MemoryError::InvalidConfidence { value: strength });
        }
        Ok(Self {
            id: MemoryId::new(),
            agent_id,
            session_id,
            kind,
            content: content.into(),
            timestamp: Utc::now(),
            strength,
            tags,
        })
    }

    /// Construct an [`Episode`] with an explicit timestamp (useful for tests).
    ///
    /// # Returns
    /// - `Ok(Episode)` on success.
    /// - `Err(MemoryError::InvalidConfidence)` if `strength` is outside `[0.0, 1.0]`.
    ///
    /// # Panics
    /// This function never panics.
    pub fn with_timestamp(
        agent_id: AgentId,
        session_id: SessionId,
        kind: EpisodeKind,
        content: impl Into<String>,
        strength: f32,
        tags: Vec<Tag>,
        timestamp: DateTime<Utc>,
    ) -> Result<Self, MemoryError> {
        if !(0.0..=1.0).contains(&strength) {
            return Err(MemoryError::InvalidConfidence { value: strength });
        }
        Ok(Self {
            id: MemoryId::new(),
            agent_id,
            session_id,
            kind,
            content: content.into(),
            timestamp,
            strength,
            tags,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_ids() -> (AgentId, SessionId) {
        (AgentId::new(), SessionId::new())
    }

    #[test]
    fn test_episode_new_valid_strength_returns_ok() {
        let (a, s) = make_ids();
        let ep = Episode::new(a, s, EpisodeKind::Observation, "saw something", 0.8, vec![]);
        assert!(ep.is_ok());
    }

    #[test]
    fn test_episode_new_strength_zero_is_valid() {
        let (a, s) = make_ids();
        let ep = Episode::new(a, s, EpisodeKind::Action, "did nothing", 0.0, vec![]);
        assert!(ep.is_ok());
    }

    #[test]
    fn test_episode_new_strength_one_is_valid() {
        let (a, s) = make_ids();
        let ep = Episode::new(a, s, EpisodeKind::Inference, "deduced", 1.0, vec![]);
        assert!(ep.is_ok());
    }

    #[test]
    fn test_confidence_invalid_range_returns_error_above_one() {
        let (a, s) = make_ids();
        let err = Episode::new(a, s, EpisodeKind::Observation, "x", 1.1, vec![]).unwrap_err();
        assert!(matches!(err, MemoryError::InvalidConfidence { value } if (value - 1.1).abs() < 1e-5));
    }

    #[test]
    fn test_confidence_invalid_range_returns_error_negative() {
        let (a, s) = make_ids();
        let err = Episode::new(a, s, EpisodeKind::Observation, "x", -0.1, vec![]).unwrap_err();
        assert!(matches!(err, MemoryError::InvalidConfidence { .. }));
    }

    #[test]
    fn test_episode_content_stored_correctly() {
        let (a, s) = make_ids();
        let ep = Episode::new(a, s, EpisodeKind::Interaction, "hello world", 0.5, vec![])
            .unwrap();
        assert_eq!(ep.content, "hello world");
    }

    #[test]
    fn test_episode_tags_stored_correctly() {
        let (a, s) = make_ids();
        let tags = vec![Tag::new("important"), Tag::new("urgent")];
        let ep = Episode::new(a, s, EpisodeKind::Observation, "x", 0.5, tags.clone()).unwrap();
        assert_eq!(ep.tags, tags);
    }

    #[test]
    fn test_episode_kind_variants_are_distinct() {
        assert_ne!(EpisodeKind::Observation, EpisodeKind::Action);
        assert_ne!(EpisodeKind::Inference, EpisodeKind::Interaction);
    }

    #[test]
    fn test_episode_with_timestamp_stores_timestamp() {
        let (a, s) = make_ids();
        let ts = DateTime::parse_from_rfc3339("2025-01-01T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let ep =
            Episode::with_timestamp(a, s, EpisodeKind::Observation, "x", 0.5, vec![], ts).unwrap();
        assert_eq!(ep.timestamp, ts);
    }
}
