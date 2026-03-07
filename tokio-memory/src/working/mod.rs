//! # Module: Working Memory
//!
//! ## Responsibility
//! Hold the currently active context for an agent session.
//! Bounded, fast, and ephemeral.

pub mod buffer;
pub mod slot;

pub use buffer::{BoundedWorkingMemory, CapacityReport, WorkingMemory};
pub use slot::{Slot, SlotHandle, SlotState};
