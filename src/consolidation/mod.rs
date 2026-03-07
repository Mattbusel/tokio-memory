//! # Consolidation
//!
//! Provides the [`ConsolidationPolicy`] trait, the [`FrequencyPolicy`]
//! built-in, and the [`ConsolidationPipeline`] orchestrator that promotes
//! episodes into semantic facts.

pub mod pipeline;
pub mod policy;

pub use pipeline::{ConsolidationPipeline, ConsolidationReport};
pub use policy::{ConsolidationPolicy, FrequencyPolicy};
