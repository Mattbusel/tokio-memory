//! # Ebbinghaus Exponential Decay
//!
//! Implements the classic forgetting-curve model:
//!
//! ```text
//! strength(t) = e^(-ln(2) / half_life_secs * elapsed_secs)
//! ```
//!
//! The result is clamped to `[0.0, 1.0]`.

use chrono::{DateTime, Utc};

use super::decay::DecayPolicy;

/// Exponential memory decay parameterised by a half-life duration.
///
/// At `t = 0` strength is `1.0`; at `t = half_life_secs` strength is `0.5`;
/// as `t → ∞` strength approaches `0.0`.
///
/// # Example
/// ```rust
/// use chrono::Utc;
/// use tokio_memory::decay::ebbinghaus::ExponentialDecay;
/// use tokio_memory::decay::decay::DecayPolicy;
///
/// let policy = ExponentialDecay::new(3600.0); // 1-hour half-life
/// let now = Utc::now();
/// let strength = policy.strength_at(now, now);
/// assert!((strength - 1.0).abs() < 1e-6);
/// ```
#[derive(Debug, Clone)]
pub struct ExponentialDecay {
    /// The half-life in seconds. After this many seconds the strength is 0.5.
    pub half_life_secs: f64,
}

impl ExponentialDecay {
    /// Create a new [`ExponentialDecay`] with the given half-life.
    ///
    /// # Arguments
    /// * `half_life_secs` — Duration (in seconds) after which strength halves.
    ///   Must be positive; if `<= 0.0` the policy will return `0.0` for all
    ///   non-zero elapsed times (the formula saturates gracefully).
    ///
    /// # Panics
    /// This function never panics.
    pub fn new(half_life_secs: f64) -> Self {
        Self { half_life_secs }
    }
}

impl DecayPolicy for ExponentialDecay {
    fn strength_at(&self, created: DateTime<Utc>, now: DateTime<Utc>) -> f32 {
        let elapsed_secs = (now - created).num_milliseconds().max(0) as f64 / 1000.0;

        if self.half_life_secs <= 0.0 {
            return if elapsed_secs <= 0.0 { 1.0 } else { 0.0 };
        }

        let lambda = std::f64::consts::LN_2 / self.half_life_secs;
        let strength = (-lambda * elapsed_secs).exp() as f32;
        strength.clamp(0.0, 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[test]
    fn test_exponential_decay_at_zero_elapsed_returns_one() {
        let policy = ExponentialDecay::new(3600.0);
        let now = Utc::now();
        let s = policy.strength_at(now, now);
        assert!((s - 1.0).abs() < 1e-5, "expected ~1.0, got {s}");
    }

    #[test]
    fn test_exponential_decay_at_half_life_returns_point_five() {
        let half_life = 3600.0_f64;
        let policy = ExponentialDecay::new(half_life);
        let created = Utc::now();
        let now = created + Duration::seconds(half_life as i64);
        let s = policy.strength_at(created, now);
        assert!(
            (s - 0.5).abs() < 1e-4,
            "expected ~0.5 at half-life, got {s}"
        );
    }

    #[test]
    fn test_exponential_decay_clamped_to_zero_minimum() {
        let policy = ExponentialDecay::new(1.0); // 1-second half-life
        let created = Utc::now();
        // After 1000 half-lives, strength is effectively 0
        let now = created + Duration::seconds(1_000_000);
        let s = policy.strength_at(created, now);
        assert!(s >= 0.0, "strength must be >= 0.0, got {s}");
        assert!(s < 1e-6, "expected near-zero, got {s}");
    }

    #[test]
    fn test_exponential_decay_decreases_monotonically() {
        let policy = ExponentialDecay::new(60.0);
        let created = Utc::now();
        let s0 = policy.strength_at(created, created);
        let s1 = policy.strength_at(created, created + Duration::seconds(30));
        let s2 = policy.strength_at(created, created + Duration::seconds(60));
        let s3 = policy.strength_at(created, created + Duration::seconds(120));
        assert!(s0 > s1, "s0={s0} s1={s1}");
        assert!(s1 > s2, "s1={s1} s2={s2}");
        assert!(s2 > s3, "s2={s2} s3={s3}");
    }

    #[test]
    fn test_exponential_decay_negative_elapsed_clamped() {
        // now < created would produce negative elapsed; clamp to 0 → strength 1.0
        let policy = ExponentialDecay::new(60.0);
        let created = Utc::now();
        let earlier = created - Duration::seconds(10);
        let s = policy.strength_at(created, earlier);
        assert!((s - 1.0).abs() < 1e-5, "expected 1.0 for negative elapsed, got {s}");
    }

    #[test]
    fn test_exponential_decay_zero_half_life_returns_zero_for_nonzero_elapsed() {
        let policy = ExponentialDecay::new(0.0);
        let created = Utc::now();
        let now = created + Duration::seconds(1);
        let s = policy.strength_at(created, now);
        assert_eq!(s, 0.0);
    }
}
