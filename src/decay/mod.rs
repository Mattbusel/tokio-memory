//! # Memory decay
//!
//! Provides the [`DecayPolicy`] trait and the [`ExponentialDecay`] Ebbinghaus
//! implementation for computing time-based memory strength reduction.

#[allow(clippy::module_inception)]
pub mod decay;
pub mod ebbinghaus;

pub use decay::DecayPolicy;
pub use ebbinghaus::ExponentialDecay;
