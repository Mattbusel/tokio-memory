use serde::{Deserialize, Serialize};
use crate::types::{MemoryId, Timestamp};
use super::event::MemoryEvent;

/// Unique identifier for an episode.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EpisodeId(pub String);

impl EpisodeId {
    /// Create an EpisodeId.
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }
}

/// A grouped collection of related memory events.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Episode {
    /// Unique identifier.
    pub id: EpisodeId,
    /// Events in this episode.
    pub events: Vec<MemoryEvent>,
    /// When the episode started.
    pub start_time: Timestamp,
    /// When the episode ended (None if ongoing).
    pub end_time: Option<Timestamp>,
    /// Optional summary.
    pub summary: Option<String>,
}

impl Episode {
    /// Create an Episode from a single event.
    ///
    /// # Panics
    /// Never panics.
    pub fn from_event(event: MemoryEvent) -> Self {
        let id = EpisodeId::new(format!("ep-{}", event.id));
        let start_time = event.timestamp;
        Self { id, events: vec![event], start_time, end_time: None, summary: None }
    }

    /// Add an event to this episode.
    pub fn add_event(&mut self, event: MemoryEvent) {
        self.end_time = Some(event.timestamp);
        self.events.push(event);
    }

    /// Return event IDs for all events.
    pub fn event_ids(&self) -> Vec<&MemoryId> {
        self.events.iter().map(|e| &e.id).collect()
    }
}
