use serde::{Deserialize, Serialize};
use crate::types::{AgentId, MemoryEntry, MemoryId};

/// A conflict between two versions of the same key.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conflict {
    /// The conflicting key.
    pub key: String,
    /// The existing entry.
    pub existing: MemoryEntry,
    /// The incoming entry.
    pub incoming: MemoryEntry,
    /// Agent that wrote the existing entry.
    pub existing_agent: AgentId,
    /// Agent that wrote the incoming entry.
    pub incoming_agent: AgentId,
}

/// Strategy for resolving conflicts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConflictResolution {
    /// Most recently written entry wins.
    LastWriteWins,
    /// Entry with higher strength wins.
    HighestConfidence,
    /// Merge both entries' JSON objects.
    MergeAll,
}

/// Receipt for a successful write operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WriteReceipt {
    /// The memory ID written.
    pub id: MemoryId,
    /// Whether a conflict was resolved.
    pub conflict_resolved: bool,
}
