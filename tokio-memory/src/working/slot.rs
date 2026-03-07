use serde::{Deserialize, Serialize};
use crate::types::MemoryId;

/// Handle to a working memory slot.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SlotHandle(pub MemoryId);

impl SlotHandle {
    /// Create a SlotHandle from a MemoryId.
    pub fn new(id: MemoryId) -> Self { Self(id) }
}

/// State of a working memory slot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SlotState {
    /// Actively held in working memory.
    Active,
    /// Evicted due to capacity pressure.
    Evicted,
}

/// A slot in working memory.
#[derive(Debug, Clone)]
pub struct Slot {
    /// Handle for this slot.
    pub handle: SlotHandle,
    /// Key associated with this slot.
    pub key: String,
    /// Value stored.
    pub value: serde_json::Value,
    /// Current state.
    pub state: SlotState,
    /// Number of accesses.
    pub access_count: u32,
}

impl Slot {
    /// Create a new active Slot.
    pub fn new(handle: SlotHandle, key: String, value: serde_json::Value) -> Self {
        Self { handle, key, value, state: SlotState::Active, access_count: 0 }
    }

    /// Record an access.
    pub fn touch(&mut self) {
        self.access_count = self.access_count.saturating_add(1);
    }
}
