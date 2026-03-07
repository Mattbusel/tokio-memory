//! # DecayPolicy trait
//!
//! Defines the interface for computing the current memory strength of an item
//! given its creation time and the current time.

use chrono::{DateTime, Utc};

/// A memory decay policy.
///
/// Implementors compute a strength value in `[0.0, 1.0]` representing how
/// much of the original memory signal remains after elapsed time.
///
/// # Guarantees
/// - `strength_at` must return a value in `[0.0, 1.0]`.
/// - `strength_at` must never panic.
pub trait DecayPolicy: Send + Sync + 'static {
    /// Compute the remaining memory strength at `now`.
    ///
    /// # Arguments
    /// * `created` — When the memory item was created.
    /// * `now` — The current instant.
    ///
    /// # Returns
    /// A value in `[0.0, 1.0]`; `1.0` means fully intact, `0.0` means fully decayed.
    ///
    /// # Panics
    /// Implementations must not panic.
    fn strength_at(&self, created: DateTime<Utc>, now: DateTime<Utc>) -> f32;
}
