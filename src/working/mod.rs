//! # Working memory
//!
//! Provides [`WorkingBuffer`] — a bounded, priority-ordered scratchpad for
//! in-flight agent reasoning.

pub mod buffer;

pub use buffer::WorkingBuffer;
