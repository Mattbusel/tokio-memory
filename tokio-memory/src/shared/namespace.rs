use serde::{Deserialize, Serialize};
use crate::types::AgentId;

/// A memory namespace for isolating or sharing memory between agents.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MemoryNamespace {
    /// Namespace identifier.
    pub id: String,
    /// Whether this namespace is shared.
    pub shared: bool,
    /// Owning agents.
    pub owners: Vec<AgentId>,
}

impl MemoryNamespace {
    /// Create a private namespace for a single agent.
    pub fn private(agent: AgentId) -> Self {
        let id = format!("private:{}", agent.0);
        Self { id, shared: false, owners: vec![agent] }
    }

    /// Create a shared namespace.
    pub fn shared(id: impl Into<String>, owners: Vec<AgentId>) -> Self {
        Self { id: id.into(), shared: true, owners }
    }
}
