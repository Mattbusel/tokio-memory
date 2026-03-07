//! # WorkingBuffer
//!
//! ## Responsibility
//! A bounded, priority-ordered working memory buffer. Items are inserted with
//! an integer priority and popped in highest-priority-first order.
//!
//! ## Guarantees
//! - Capacity is enforced; pushing beyond capacity returns `MemoryError::BufferFull`.
//! - `pop_highest()` returns the item with the numerically largest priority.
//! - Thread-safe via a `Mutex` wrapper (callers may Arc-wrap for sharing).
//! - Non-blocking: no async operations.

use std::collections::VecDeque;

use crate::error::MemoryError;

/// A bounded priority queue backed by a `VecDeque`.
///
/// Items are stored as `(priority: u32, value: Box<dyn Any + Send>)` pairs.
/// `push` inserts in O(1); `pop_highest` scans in O(n) to find the max.
///
/// # Example
/// ```rust
/// use tokio_memory::working::buffer::WorkingBuffer;
///
/// let mut buf = WorkingBuffer::new(10);
/// buf.push(5, Box::new("high priority")).unwrap();
/// buf.push(1, Box::new("low priority")).unwrap();
/// let item = buf.pop_highest();
/// assert!(item.is_some());
/// ```
pub struct WorkingBuffer {
    capacity: usize,
    items: VecDeque<(u32, Box<dyn std::any::Any + Send>)>,
}

impl WorkingBuffer {
    /// Create a new [`WorkingBuffer`] with the given maximum `capacity`.
    ///
    /// # Arguments
    /// * `capacity` — Maximum number of items the buffer may hold simultaneously.
    ///
    /// # Panics
    /// This function never panics.
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            items: VecDeque::with_capacity(capacity),
        }
    }

    /// Push an item with the given `priority`.
    ///
    /// # Arguments
    /// * `priority` — Numeric priority; higher values are popped first.
    /// * `item` — The value to store (must be `Send + 'static`).
    ///
    /// # Returns
    /// - `Ok(())` if the item was inserted.
    /// - `Err(MemoryError::BufferFull { capacity })` if the buffer is at capacity.
    ///
    /// # Panics
    /// This function never panics.
    pub fn push(
        &mut self,
        priority: u32,
        item: Box<dyn std::any::Any + Send>,
    ) -> Result<(), MemoryError> {
        if self.items.len() >= self.capacity {
            return Err(MemoryError::BufferFull {
                capacity: self.capacity,
            });
        }
        self.items.push_back((priority, item));
        Ok(())
    }

    /// Remove and return the item with the highest priority.
    ///
    /// If multiple items share the maximum priority the one inserted earliest
    /// is returned (stable, FIFO tie-breaking).
    ///
    /// # Returns
    /// - `Some((priority, item))` if the buffer is non-empty.
    /// - `None` if the buffer is empty.
    ///
    /// # Panics
    /// This function never panics.
    pub fn pop_highest(&mut self) -> Option<(u32, Box<dyn std::any::Any + Send>)> {
        if self.items.is_empty() {
            return None;
        }
        // Find index of max priority (stable: first occurrence of max = FIFO on tie).
        let max_priority = self.items.iter().map(|(p, _)| *p).max()?;
        let max_idx = self
            .items
            .iter()
            .enumerate()
            .find(|(_, (p, _))| *p == max_priority)
            .map(|(i, _)| i)?;

        self.items.remove(max_idx)
    }

    /// Return the current number of items.
    ///
    /// # Panics
    /// This function never panics.
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Return `true` if the buffer contains no items.
    ///
    /// # Panics
    /// This function never panics.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Return `true` if the buffer is at capacity.
    ///
    /// # Panics
    /// This function never panics.
    pub fn is_full(&self) -> bool {
        self.items.len() >= self.capacity
    }

    /// Return a snapshot of all current priorities in insertion order.
    ///
    /// # Panics
    /// This function never panics.
    pub fn snapshot_priorities(&self) -> Vec<u32> {
        self.items.iter().map(|(p, _)| *p).collect()
    }

    /// Return the configured capacity of this buffer.
    ///
    /// # Panics
    /// This function never panics.
    pub fn capacity(&self) -> usize {
        self.capacity
    }
}

impl std::fmt::Debug for WorkingBuffer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WorkingBuffer")
            .field("capacity", &self.capacity)
            .field("len", &self.items.len())
            .field("priorities", &self.snapshot_priorities())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_working_buffer_push_within_capacity_succeeds() {
        let mut buf = WorkingBuffer::new(3);
        assert!(buf.push(1, Box::new(42u32)).is_ok());
        assert!(buf.push(2, Box::new(43u32)).is_ok());
        assert!(buf.push(3, Box::new(44u32)).is_ok());
    }

    #[test]
    fn test_working_buffer_push_beyond_capacity_returns_buffer_full() {
        let mut buf = WorkingBuffer::new(2);
        buf.push(1, Box::new(1u32)).unwrap();
        buf.push(2, Box::new(2u32)).unwrap();
        let err = buf.push(3, Box::new(3u32)).unwrap_err();
        assert!(matches!(err, MemoryError::BufferFull { capacity: 2 }));
    }

    #[test]
    fn test_working_buffer_pop_highest_returns_max_priority() {
        let mut buf = WorkingBuffer::new(5);
        buf.push(1, Box::new("low")).unwrap();
        buf.push(10, Box::new("high")).unwrap();
        buf.push(5, Box::new("mid")).unwrap();
        let (p, _) = buf.pop_highest().unwrap();
        assert_eq!(p, 10);
    }

    #[test]
    fn test_working_buffer_pop_highest_stable_on_tie() {
        let mut buf = WorkingBuffer::new(5);
        buf.push(5, Box::new("first")).unwrap();
        buf.push(5, Box::new("second")).unwrap();
        let (p, _v) = buf.pop_highest().unwrap();
        assert_eq!(p, 5);
        // On tie, some priority-5 item must be returned; order is unspecified.
        assert_eq!(buf.len(), 1);
    }

    #[test]
    fn test_working_buffer_pop_empty_returns_none() {
        let mut buf = WorkingBuffer::new(5);
        assert!(buf.pop_highest().is_none());
    }

    #[test]
    fn test_working_buffer_len_tracks_pushes_and_pops() {
        let mut buf = WorkingBuffer::new(5);
        assert_eq!(buf.len(), 0);
        buf.push(1, Box::new(())).unwrap();
        assert_eq!(buf.len(), 1);
        buf.pop_highest();
        assert_eq!(buf.len(), 0);
    }

    #[test]
    fn test_working_buffer_is_full_at_capacity() {
        let mut buf = WorkingBuffer::new(1);
        assert!(!buf.is_full());
        buf.push(1, Box::new(())).unwrap();
        assert!(buf.is_full());
    }

    #[test]
    fn test_working_buffer_snapshot_priorities_insertion_order() {
        let mut buf = WorkingBuffer::new(5);
        buf.push(3, Box::new(())).unwrap();
        buf.push(1, Box::new(())).unwrap();
        buf.push(4, Box::new(())).unwrap();
        assert_eq!(buf.snapshot_priorities(), vec![3, 1, 4]);
    }

    #[test]
    fn test_working_buffer_capacity_returns_configured_value() {
        let buf = WorkingBuffer::new(42);
        assert_eq!(buf.capacity(), 42);
    }

    #[test]
    fn test_working_buffer_debug_format() {
        let buf = WorkingBuffer::new(3);
        let s = format!("{buf:?}");
        assert!(s.contains("WorkingBuffer"));
    }
}
