use crate::types::Timestamp;

/// Trait for memory decay models.
pub trait DecayModel: Send + Sync {
    /// Compute memory strength at the current time.
    ///
    /// # Arguments
    /// * `created` — When the memory was created.
    /// * `last_accessed` — When last accessed.
    /// * `now` — Current time.
    ///
    /// # Returns
    /// Strength in 0.0..=1.0.
    ///
    /// # Panics
    /// Never panics.
    fn strength_at(&self, created: Timestamp, last_accessed: Timestamp, now: Timestamp) -> f32;

    /// True if the memory should be considered forgotten.
    ///
    /// # Panics
    /// Never panics.
    fn is_forgotten(&self, strength: f32) -> bool;
}

/// Exponential decay: strength halves every `half_life_secs`.
pub struct ExponentialDecay {
    /// Half-life in seconds.
    pub half_life_secs: f64,
    /// Forgetting threshold.
    pub forget_threshold: f32,
}

impl ExponentialDecay {
    /// Create with the given half-life.
    pub fn new(half_life_secs: f64) -> Self {
        Self { half_life_secs, forget_threshold: 0.01 }
    }
}

impl DecayModel for ExponentialDecay {
    fn strength_at(&self, _created: Timestamp, last_accessed: Timestamp, now: Timestamp) -> f32 {
        let elapsed = (now - last_accessed).num_seconds().max(0) as f64;
        let decay = (-elapsed * std::f64::consts::LN_2 / self.half_life_secs).exp();
        (decay as f32).clamp(0.0, 1.0)
    }

    fn is_forgotten(&self, strength: f32) -> bool {
        strength < self.forget_threshold
    }
}

/// Power law decay (Ebbinghaus-inspired).
pub struct PowerLawDecay {
    /// Stability parameter.
    pub stability: f64,
    /// Forgetting threshold.
    pub forget_threshold: f32,
}

impl PowerLawDecay {
    /// Create with the given stability.
    pub fn new(stability: f64) -> Self {
        Self { stability, forget_threshold: 0.01 }
    }
}

impl DecayModel for PowerLawDecay {
    fn strength_at(&self, _created: Timestamp, last_accessed: Timestamp, now: Timestamp) -> f32 {
        let elapsed = (now - last_accessed).num_seconds().max(0) as f64 + 1.0;
        let decay = (elapsed / self.stability).powf(-0.5);
        (decay as f32).clamp(0.0, 1.0)
    }

    fn is_forgotten(&self, strength: f32) -> bool {
        strength < self.forget_threshold
    }
}

/// No decay — memories never fade.
pub struct NoDecay;

impl DecayModel for NoDecay {
    fn strength_at(&self, _: Timestamp, _: Timestamp, _: Timestamp) -> f32 { 1.0 }
    fn is_forgotten(&self, _: f32) -> bool { false }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, Utc};
    use proptest::prelude::*;

    #[test]
    fn test_decay_exponential_strength_decreases_over_time() {
        let model = ExponentialDecay::new(3600.0);
        let now = Utc::now();
        let s_recent = model.strength_at(now, now - Duration::minutes(1), now);
        let s_old = model.strength_at(now, now - Duration::hours(10), now);
        assert!(s_recent > s_old);
    }

    #[test]
    fn test_decay_no_decay_always_one() {
        let model = NoDecay;
        let now = Utc::now();
        assert_eq!(model.strength_at(now, now - Duration::days(1000), now), 1.0);
    }

    #[test]
    fn test_decay_no_decay_never_forgotten() {
        let model = NoDecay;
        assert!(!model.is_forgotten(0.0));
    }

    #[test]
    fn test_decay_exponential_forgotten_below_threshold() {
        let model = ExponentialDecay::new(1.0);
        assert!(model.is_forgotten(0.005));
        assert!(!model.is_forgotten(0.5));
    }

    proptest! {
        #[test]
        fn test_decay_strength_always_in_unit_interval(elapsed in 0i64..1_000_000) {
            let model = ExponentialDecay::new(3600.0);
            let now = Utc::now();
            let s = model.strength_at(now, now - Duration::seconds(elapsed), now);
            prop_assert!(s >= 0.0 && s <= 1.0);
        }

        #[test]
        fn test_decay_power_law_always_in_unit_interval(elapsed in 0i64..1_000_000) {
            let model = PowerLawDecay::new(3600.0);
            let now = Utc::now();
            let s = model.strength_at(now, now - Duration::seconds(elapsed), now);
            prop_assert!(s >= 0.0 && s <= 1.0);
        }
    }
}
