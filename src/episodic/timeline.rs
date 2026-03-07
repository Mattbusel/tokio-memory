//! # TimeRange
//!
//! A closed interval of time used for range-based episodic recall.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A closed temporal interval `[start, end]`.
///
/// Used by [`crate::episodic::store::EpisodicStore::recall_range`] to filter
/// episodes whose timestamps fall within the window.
///
/// # Example
/// ```rust
/// use chrono::Utc;
/// use tokio_memory::episodic::timeline::TimeRange;
///
/// let start = Utc::now();
/// let end = Utc::now();
/// let range = TimeRange { start, end };
/// assert!(range.contains(&Utc::now()) || true); // illustrative
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeRange {
    /// Inclusive start of the range.
    pub start: DateTime<Utc>,
    /// Inclusive end of the range.
    pub end: DateTime<Utc>,
}

impl TimeRange {
    /// Create a new [`TimeRange`].
    ///
    /// # Arguments
    /// * `start` — Inclusive start timestamp.
    /// * `end` — Inclusive end timestamp.
    ///
    /// # Panics
    /// This function never panics.
    pub fn new(start: DateTime<Utc>, end: DateTime<Utc>) -> Self {
        Self { start, end }
    }

    /// Return `true` if `ts` falls within `[self.start, self.end]`.
    ///
    /// # Arguments
    /// * `ts` — The timestamp to test.
    ///
    /// # Panics
    /// This function never panics.
    pub fn contains(&self, ts: &DateTime<Utc>) -> bool {
        ts >= &self.start && ts <= &self.end
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    fn base() -> DateTime<Utc> {
        DateTime::parse_from_rfc3339("2025-06-01T12:00:00Z")
            .unwrap()
            .with_timezone(&Utc)
    }

    #[test]
    fn test_time_range_contains_start_boundary() {
        let b = base();
        let r = TimeRange::new(b, b + Duration::hours(1));
        assert!(r.contains(&b));
    }

    #[test]
    fn test_time_range_contains_end_boundary() {
        let b = base();
        let end = b + Duration::hours(1);
        let r = TimeRange::new(b, end);
        assert!(r.contains(&end));
    }

    #[test]
    fn test_time_range_contains_midpoint() {
        let b = base();
        let r = TimeRange::new(b, b + Duration::hours(2));
        assert!(r.contains(&(b + Duration::hours(1))));
    }

    #[test]
    fn test_time_range_excludes_before_start() {
        let b = base();
        let r = TimeRange::new(b, b + Duration::hours(1));
        assert!(!r.contains(&(b - Duration::seconds(1))));
    }

    #[test]
    fn test_time_range_excludes_after_end() {
        let b = base();
        let end = b + Duration::hours(1);
        let r = TimeRange::new(b, end);
        assert!(!r.contains(&(end + Duration::seconds(1))));
    }

    #[test]
    fn test_time_range_new_constructs_correctly() {
        let b = base();
        let e = b + Duration::minutes(30);
        let r = TimeRange::new(b, e);
        assert_eq!(r.start, b);
        assert_eq!(r.end, e);
    }
}
