//! # Fact
//!
//! A [`Fact`] is a typed subject-predicate-object triple with a confidence
//! score. Facts are the output of the consolidation pipeline and the currency
//! of the semantic memory layer.
//!
//! ## Guarantees
//! - `confidence` is validated to be in `[0.0, 1.0]` at construction time.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::error::MemoryError;
use crate::id::{EntityId, MemoryId};

/// The object component of a semantic triple.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FactValue {
    /// Free-form text value.
    Text(String),
    /// Numeric value.
    Number(f64),
    /// Boolean value.
    Bool(bool),
}

/// A semantic memory triple: `(subject, predicate, object)` with confidence.
///
/// # Fields
/// * `id` — Unique identifier for this fact.
/// * `subject` — The entity this fact is about.
/// * `predicate` — The relation or attribute name.
/// * `object` — The value associated with the predicate.
/// * `confidence` — Certainty score in `[0.0, 1.0]`.
/// * `asserted_at` — When this fact was asserted (UTC).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fact {
    /// Unique identifier.
    pub id: MemoryId,
    /// Subject entity.
    pub subject: EntityId,
    /// Relation or attribute.
    pub predicate: String,
    /// Object value.
    pub object: FactValue,
    /// Confidence in `[0.0, 1.0]`.
    pub confidence: f32,
    /// Assertion timestamp (UTC).
    pub asserted_at: DateTime<Utc>,
}

impl Fact {
    /// Construct a new [`Fact`], validating `confidence` ∈ `[0.0, 1.0]`.
    ///
    /// # Arguments
    /// * `subject` — The entity this fact describes.
    /// * `predicate` — The relation or attribute name.
    /// * `object` — The value.
    /// * `confidence` — Certainty score in `[0.0, 1.0]`.
    ///
    /// # Returns
    /// - `Ok(Fact)` on success.
    /// - `Err(MemoryError::InvalidConfidence)` if `confidence` is outside `[0.0, 1.0]`.
    ///
    /// # Panics
    /// This function never panics.
    pub fn new(
        subject: EntityId,
        predicate: impl Into<String>,
        object: FactValue,
        confidence: f32,
    ) -> Result<Self, MemoryError> {
        if !(0.0..=1.0).contains(&confidence) {
            return Err(MemoryError::InvalidConfidence { value: confidence });
        }
        Ok(Self {
            id: MemoryId::new(),
            subject,
            predicate: predicate.into(),
            object,
            confidence,
            asserted_at: Utc::now(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fact_new_valid_returns_ok() {
        let e = EntityId::new("user:1");
        let f = Fact::new(e, "name", FactValue::Text("Alice".into()), 0.9);
        assert!(f.is_ok());
    }

    #[test]
    fn test_fact_new_invalid_confidence_above_one_returns_error() {
        let e = EntityId::new("user:1");
        let err = Fact::new(e, "age", FactValue::Number(30.0), 1.5).unwrap_err();
        assert!(matches!(err, MemoryError::InvalidConfidence { value } if (value - 1.5).abs() < 1e-5));
    }

    #[test]
    fn test_fact_new_invalid_confidence_negative_returns_error() {
        let e = EntityId::new("user:1");
        let err = Fact::new(e, "age", FactValue::Number(30.0), -0.1).unwrap_err();
        assert!(matches!(err, MemoryError::InvalidConfidence { .. }));
    }

    #[test]
    fn test_fact_value_text_stores_correctly() {
        let e = EntityId::new("e");
        let f = Fact::new(e, "p", FactValue::Text("hello".into()), 1.0).unwrap();
        assert_eq!(f.object, FactValue::Text("hello".into()));
    }

    #[test]
    fn test_fact_value_number_stores_correctly() {
        let e = EntityId::new("e");
        let f = Fact::new(e, "p", FactValue::Number(42.0), 1.0).unwrap();
        assert_eq!(f.object, FactValue::Number(42.0));
    }

    #[test]
    fn test_fact_value_bool_stores_correctly() {
        let e = EntityId::new("e");
        let f = Fact::new(e, "p", FactValue::Bool(true), 1.0).unwrap();
        assert_eq!(f.object, FactValue::Bool(true));
    }

    #[test]
    fn test_fact_predicate_stored_correctly() {
        let e = EntityId::new("e");
        let f = Fact::new(e, "my-predicate", FactValue::Bool(false), 0.5).unwrap();
        assert_eq!(f.predicate, "my-predicate");
    }
}

impl Fact {
    /// Construct a [`Fact`] with an explicit [`MemoryId`] (for deterministic/idempotent upserts).
    pub fn with_id(
        id: MemoryId,
        subject: EntityId,
        predicate: impl Into<String>,
        object: FactValue,
        confidence: f32,
    ) -> Result<Self, MemoryError> {
        if !(0.0..=1.0).contains(&confidence) {
            return Err(MemoryError::InvalidConfidence { value: confidence });
        }
        Ok(Self {
            id,
            subject,
            predicate: predicate.into(),
            object,
            confidence,
            asserted_at: chrono::Utc::now(),
        })
    }
}
