use async_trait::async_trait;
use dashmap::DashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use chrono::Utc;
use crate::error::MemoryError;
use crate::types::MemoryId;
use super::slot::{Slot, SlotHandle, SlotState};

/// Report on working memory capacity.
#[derive(Debug, Clone)]
pub struct CapacityReport {
    /// Active slot count.
    pub used: usize,
    /// Maximum capacity.
    pub capacity: usize,
}

impl CapacityReport {
    /// True if at capacity.
    pub fn is_full(&self) -> bool { self.used >= self.capacity }
}

/// Trait for working memory.
#[async_trait]
pub trait WorkingMemory: Send + Sync {
    /// Load a value into working memory, returning a handle.
    ///
    /// # Panics
    /// Never panics.
    async fn load(&self, key: String, value: serde_json::Value) -> Result<SlotHandle, MemoryError>;

    /// Read a value from a slot.
    ///
    /// # Panics
    /// Never panics.
    async fn read(&self, handle: &SlotHandle) -> Result<serde_json::Value, MemoryError>;

    /// Evict a slot.
    ///
    /// # Panics
    /// Never panics.
    async fn evict(&self, handle: &SlotHandle) -> Result<(), MemoryError>;

    /// Get capacity report.
    ///
    /// # Panics
    /// Never panics.
    async fn capacity(&self) -> CapacityReport;

    /// Number of active slots.
    ///
    /// # Panics
    /// Never panics.
    async fn active_slots(&self) -> usize;
}

/// Bounded in-memory working memory.
pub struct BoundedWorkingMemory {
    slots: Arc<DashMap<MemoryId, Slot>>,
    capacity: usize,
    count: Arc<AtomicUsize>,
}

impl BoundedWorkingMemory {
    /// Create a new bounded buffer.
    ///
    /// # Arguments
    /// * `capacity` — Maximum slot count.
    ///
    /// # Panics
    /// Never panics.
    pub fn new(capacity: usize) -> Self {
        Self {
            slots: Arc::new(DashMap::new()),
            capacity,
            count: Arc::new(AtomicUsize::new(0)),
        }
    }
}

#[async_trait]
impl WorkingMemory for BoundedWorkingMemory {
    async fn load(&self, key: String, value: serde_json::Value) -> Result<SlotHandle, MemoryError> {
        if self.count.load(Ordering::SeqCst) >= self.capacity {
            return Err(MemoryError::CapacityExceeded { capacity: self.capacity });
        }
        let now = Utc::now();
        let ns = now.timestamp_nanos_opt().unwrap_or_else(|| now.timestamp() * 1_000_000_000);
        let id = MemoryId::from_content(format!("{}{}", key, ns).as_bytes());
        let handle = SlotHandle::new(id.clone());
        self.slots.insert(id, Slot::new(handle.clone(), key, value));
        self.count.fetch_add(1, Ordering::SeqCst);
        Ok(handle)
    }

    async fn read(&self, handle: &SlotHandle) -> Result<serde_json::Value, MemoryError> {
        self.slots
            .get_mut(&handle.0)
            .map(|mut s| { s.touch(); s.value.clone() })
            .ok_or_else(|| MemoryError::NotFound(handle.0.clone()))
    }

    async fn evict(&self, handle: &SlotHandle) -> Result<(), MemoryError> {
        if let Some(mut slot) = self.slots.get_mut(&handle.0) {
            slot.state = SlotState::Evicted;
        }
        self.slots
            .remove(&handle.0)
            .map(|_| { self.count.fetch_sub(1, Ordering::SeqCst); })
            .ok_or_else(|| MemoryError::NotFound(handle.0.clone()))
    }

    async fn capacity(&self) -> CapacityReport {
        CapacityReport { used: self.count.load(Ordering::SeqCst), capacity: self.capacity }
    }

    async fn active_slots(&self) -> usize {
        self.count.load(Ordering::SeqCst)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_working_memory_load_read_roundtrip() {
        let wm = BoundedWorkingMemory::new(10);
        let h = wm.load("k".to_string(), json!({"x": 1})).await.unwrap();
        assert_eq!(wm.read(&h).await.unwrap(), json!({"x": 1}));
    }

    #[tokio::test]
    async fn test_working_memory_capacity_exceeded_returns_error() {
        let wm = BoundedWorkingMemory::new(1);
        wm.load("k1".to_string(), json!(1)).await.unwrap();
        let result = wm.load("k2".to_string(), json!(2)).await;
        assert!(matches!(result, Err(MemoryError::CapacityExceeded { .. })));
    }

    #[tokio::test]
    async fn test_working_memory_evict_removes_slot() {
        let wm = BoundedWorkingMemory::new(10);
        let h = wm.load("k".to_string(), json!("v")).await.unwrap();
        wm.evict(&h).await.unwrap();
        assert!(matches!(wm.read(&h).await, Err(MemoryError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_working_memory_evict_nonexistent_returns_error() {
        let wm = BoundedWorkingMemory::new(10);
        let fake = SlotHandle::new(MemoryId::from_string("fake".to_string()));
        assert!(matches!(wm.evict(&fake).await, Err(MemoryError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_working_memory_active_slots_tracks_correctly() {
        let wm = BoundedWorkingMemory::new(10);
        assert_eq!(wm.active_slots().await, 0);
        let h = wm.load("k".to_string(), json!(1)).await.unwrap();
        assert_eq!(wm.active_slots().await, 1);
        wm.evict(&h).await.unwrap();
        assert_eq!(wm.active_slots().await, 0);
    }

    #[tokio::test]
    async fn test_working_memory_capacity_report_is_full() {
        let wm = BoundedWorkingMemory::new(1);
        assert!(!wm.capacity().await.is_full());
        wm.load("k".to_string(), json!(null)).await.unwrap();
        assert!(wm.capacity().await.is_full());
    }
}
