//! # Identity newtypes
//!
//! Strongly-typed identifiers used throughout tokio-memory. Every ID type
//! wraps either a [`uuid::Uuid`] or a [`String`] and is newtype-distinct
//! to prevent accidental mixing (e.g., passing a `SessionId` where an
//! `AgentId` is expected).
//!
//! ## Guarantees
//! - All IDs are cheaply clonable.
//! - UUID-backed IDs are globally unique when created with `new()`.
//! - String-backed IDs are caller-defined; uniqueness is not enforced.

use std::fmt;
use uuid::Uuid;

/// Unique identifier for a single memory item (episode or fact).
///
/// # Example
/// ```rust
/// use tokio_memory::id::MemoryId;
/// let id = MemoryId::new();
/// println!("{id}");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct MemoryId(Uuid);

impl MemoryId {
    /// Create a new random [`MemoryId`].
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Return the underlying [`Uuid`].
    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }
}

impl Default for MemoryId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for MemoryId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<Uuid> for MemoryId {
    fn from(u: Uuid) -> Self {
        Self(u)
    }
}

// ---------------------------------------------------------------------------

/// Unique identifier for an agent.
///
/// # Example
/// ```rust
/// use tokio_memory::id::AgentId;
/// let id = AgentId::new();
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct AgentId(Uuid);

impl AgentId {
    /// Create a new random [`AgentId`].
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for AgentId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for AgentId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

// ---------------------------------------------------------------------------

/// Unique identifier for a session (a bounded interaction window).
///
/// # Example
/// ```rust
/// use tokio_memory::id::SessionId;
/// let id = SessionId::new();
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct SessionId(Uuid);

impl SessionId {
    /// Create a new random [`SessionId`].
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for SessionId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for SessionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

// ---------------------------------------------------------------------------

/// Namespace identifier used by the shared memory space.
///
/// # Example
/// ```rust
/// use tokio_memory::id::NamespaceId;
/// let ns = NamespaceId::new("global");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct NamespaceId(String);

impl NamespaceId {
    /// Create a [`NamespaceId`] from any string-like value.
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    /// Return the inner string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for NamespaceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

// ---------------------------------------------------------------------------

/// Identifier for a semantic entity (subject of a [`crate::semantic::fact::Fact`]).
///
/// # Example
/// ```rust
/// use tokio_memory::id::EntityId;
/// let e = EntityId::new("user:42");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct EntityId(String);

impl EntityId {
    /// Create an [`EntityId`] from any string-like value.
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    /// Return the inner string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for EntityId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

// ---------------------------------------------------------------------------

/// A classification tag attached to an [`crate::episodic::episode::Episode`].
///
/// # Example
/// ```rust
/// use tokio_memory::id::Tag;
/// let t = Tag::new("important");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct Tag(String);

impl Tag {
    /// Create a [`Tag`] from any string-like value.
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    /// Return the inner string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Tag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_id_new_is_unique() {
        let a = MemoryId::new();
        let b = MemoryId::new();
        assert_ne!(a, b);
    }

    #[test]
    fn test_memory_id_display_contains_uuid() {
        let id = MemoryId::new();
        let s = id.to_string();
        // UUID strings are 36 chars (8-4-4-4-12 with dashes)
        assert_eq!(s.len(), 36);
    }

    #[test]
    fn test_memory_id_display_is_stable() {
        let id = MemoryId::new();
        assert_eq!(id.to_string(), id.to_string());
    }

    #[test]
    fn test_agent_id_display_is_stable() {
        let id = AgentId::new();
        assert_eq!(id.to_string(), id.to_string());
    }

    #[test]
    fn test_session_id_display_is_stable() {
        let id = SessionId::new();
        assert_eq!(id.to_string(), id.to_string());
    }

    #[test]
    fn test_namespace_id_display_returns_inner() {
        let ns = NamespaceId::new("test-ns");
        assert_eq!(ns.to_string(), "test-ns");
    }

    #[test]
    fn test_entity_id_display_returns_inner() {
        let e = EntityId::new("user:99");
        assert_eq!(e.to_string(), "user:99");
    }

    #[test]
    fn test_tag_display_returns_inner() {
        let t = Tag::new("critical");
        assert_eq!(t.to_string(), "critical");
    }

    #[test]
    fn test_memory_id_clone_equals_original() {
        let a = MemoryId::new();
        let b = a.clone();
        assert_eq!(a, b);
    }

    #[test]
    fn test_memory_id_from_uuid_roundtrip() {
        let u = Uuid::new_v4();
        let id = MemoryId::from(u);
        assert_eq!(id.as_uuid(), &u);
    }

    #[test]
    fn test_tag_as_str() {
        let t = Tag::new("foo");
        assert_eq!(t.as_str(), "foo");
    }

    #[test]
    fn test_entity_id_as_str() {
        let e = EntityId::new("bar");
        assert_eq!(e.as_str(), "bar");
    }
}
