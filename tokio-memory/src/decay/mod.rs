//! # Module: Memory Decay
//!
//! ## Responsibility
//! Model the forgetting of memories over time.
//!
//! ## Guarantees
//! - Strength is always in 0.0..=1.0

pub mod models;

pub use models::{DecayModel, ExponentialDecay, NoDecay, PowerLawDecay};
