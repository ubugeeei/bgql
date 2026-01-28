//! Resource management for query scheduling.
//!
//! This module provides resource tracking and management for
//! priority-based query scheduling.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{RwLock, Semaphore};

/// Resource level for I/O and network operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ResourceLevel {
    #[default]
    Low,
    Medium,
    High,
}

impl ResourceLevel {
    /// Converts to a numeric weight (1-3).
    pub fn weight(&self) -> usize {
        match self {
            Self::Low => 1,
            Self::Medium => 2,
            Self::High => 3,
        }
    }
}

/// Resource requirements for a query or field.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResourceRequirements {
    /// CPU usage (0.0 - 1.0).
    #[serde(default)]
    pub cpu: f64,

    /// Memory in bytes.
    #[serde(default)]
    pub memory: u64,

    /// I/O intensity level.
    #[serde(default)]
    pub io: ResourceLevel,

    /// Network intensity level.
    #[serde(default)]
    pub network: ResourceLevel,

    /// Estimated duration.
    #[serde(default)]
    pub estimated_duration: Option<Duration>,
}

impl ResourceRequirements {
    /// Creates minimal resource requirements.
    pub fn minimal() -> Self {
        Self {
            cpu: 0.01,
            memory: 1024,
            io: ResourceLevel::Low,
            network: ResourceLevel::Low,
            estimated_duration: None,
        }
    }

    /// Creates resource requirements for a CPU-intensive task.
    pub fn cpu_intensive(cpu: f64) -> Self {
        Self {
            cpu,
            memory: 1024 * 1024,
            io: ResourceLevel::Low,
            network: ResourceLevel::Low,
            estimated_duration: None,
        }
    }

    /// Creates resource requirements for an I/O-intensive task.
    pub fn io_intensive() -> Self {
        Self {
            cpu: 0.1,
            memory: 1024 * 1024,
            io: ResourceLevel::High,
            network: ResourceLevel::Low,
            estimated_duration: None,
        }
    }

    /// Creates resource requirements for a network-intensive task.
    pub fn network_intensive() -> Self {
        Self {
            cpu: 0.1,
            memory: 1024 * 1024,
            io: ResourceLevel::Low,
            network: ResourceLevel::High,
            estimated_duration: None,
        }
    }

    /// Creates resource requirements for binary streaming.
    pub fn binary_stream(estimated_size: u64) -> Self {
        Self {
            cpu: 0.05,
            memory: 64 * 1024, // 64KB buffer
            io: ResourceLevel::High,
            network: ResourceLevel::High,
            estimated_duration: Some(Duration::from_secs(estimated_size / (1024 * 1024))),
        }
    }

    /// Calculates a combined score for prioritization.
    pub fn score(&self) -> f64 {
        self.cpu + (self.memory as f64 / (1024 * 1024 * 1024) as f64) // Normalize to GB
            + (self.io.weight() as f64 * 0.1)
            + (self.network.weight() as f64 * 0.1)
    }
}

/// Current resource usage snapshot.
#[derive(Debug, Clone, Default)]
pub struct ResourceUsage {
    /// CPU usage (0.0 - 1.0).
    pub cpu: f64,

    /// Memory usage in bytes.
    pub memory: u64,

    /// Active I/O operations.
    pub io_operations: usize,

    /// Active network connections.
    pub network_connections: usize,

    /// Active binary streams.
    pub binary_streams: usize,
}

impl ResourceUsage {
    /// Returns true if resources are available for the given requirements.
    pub fn can_accommodate(&self, limits: &ResourceLimits, req: &ResourceRequirements) -> bool {
        (self.cpu + req.cpu) <= limits.max_cpu
            && (self.memory + req.memory) <= limits.max_memory
            && (self.io_operations + req.io.weight()) <= limits.max_io_operations
            && (self.network_connections + req.network.weight()) <= limits.max_network_connections
    }
}

/// Resource limits for the system.
#[derive(Debug, Clone)]
pub struct ResourceLimits {
    /// Maximum CPU usage (0.0 - 1.0, usually cores count).
    pub max_cpu: f64,

    /// Maximum memory in bytes.
    pub max_memory: u64,

    /// Maximum concurrent I/O operations.
    pub max_io_operations: usize,

    /// Maximum concurrent network connections.
    pub max_network_connections: usize,

    /// Maximum concurrent binary streams.
    pub max_binary_streams: usize,

    /// Maximum concurrent queries.
    pub max_concurrent_queries: usize,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_cpu: num_cpus::get() as f64,
            max_memory: 1024 * 1024 * 1024, // 1GB
            max_io_operations: 100,
            max_network_connections: 100,
            max_binary_streams: 10,
            max_concurrent_queries: 50,
        }
    }
}

/// Get the number of CPUs (fallback for when num_cpus is not available).
mod num_cpus {
    pub fn get() -> usize {
        std::thread::available_parallelism()
            .map(|p| p.get())
            .unwrap_or(4)
    }
}

/// Manages system resources for query execution.
#[derive(Debug)]
pub struct ResourceManager {
    /// Resource limits.
    limits: ResourceLimits,

    /// Current usage (atomic for lock-free reads).
    cpu_usage: AtomicU64,
    memory_usage: AtomicU64,
    io_operations: AtomicUsize,
    network_connections: AtomicUsize,
    binary_streams: AtomicUsize,

    /// Semaphore for concurrent query limit.
    query_semaphore: Arc<Semaphore>,

    /// Semaphore for binary streams.
    binary_stream_semaphore: Arc<Semaphore>,

    /// Active allocations by execution ID.
    allocations: RwLock<HashMap<String, ResourceAllocation>>,

    /// Statistics.
    stats: RwLock<ResourceStats>,
}

impl ResourceManager {
    /// Creates a new resource manager with default limits.
    pub fn new() -> Self {
        Self::with_limits(ResourceLimits::default())
    }

    /// Creates a resource manager with custom limits.
    pub fn with_limits(limits: ResourceLimits) -> Self {
        let query_semaphore = Arc::new(Semaphore::new(limits.max_concurrent_queries));
        let binary_stream_semaphore = Arc::new(Semaphore::new(limits.max_binary_streams));

        Self {
            limits,
            cpu_usage: AtomicU64::new(0),
            memory_usage: AtomicU64::new(0),
            io_operations: AtomicUsize::new(0),
            network_connections: AtomicUsize::new(0),
            binary_streams: AtomicUsize::new(0),
            query_semaphore,
            binary_stream_semaphore,
            allocations: RwLock::new(HashMap::new()),
            stats: RwLock::new(ResourceStats::default()),
        }
    }

    /// Returns current resource usage.
    pub fn current_usage(&self) -> ResourceUsage {
        ResourceUsage {
            cpu: f64::from_bits(self.cpu_usage.load(Ordering::Relaxed)),
            memory: self.memory_usage.load(Ordering::Relaxed),
            io_operations: self.io_operations.load(Ordering::Relaxed),
            network_connections: self.network_connections.load(Ordering::Relaxed),
            binary_streams: self.binary_streams.load(Ordering::Relaxed),
        }
    }

    /// Returns the resource limits.
    pub fn limits(&self) -> &ResourceLimits {
        &self.limits
    }

    /// Checks if resources can be allocated.
    pub fn can_allocate(&self, req: &ResourceRequirements) -> bool {
        self.current_usage().can_accommodate(&self.limits, req)
    }

    /// Attempts to allocate resources for an execution.
    pub async fn try_allocate<'a>(
        &'a self,
        execution_id: &str,
        req: ResourceRequirements,
    ) -> Option<ResourceGuard<'a>> {
        let usage = self.current_usage();
        if !usage.can_accommodate(&self.limits, &req) {
            return None;
        }

        // Try to acquire query permit
        let permit = self.query_semaphore.clone().try_acquire_owned().ok()?;

        // Allocate resources
        self.cpu_usage
            .fetch_add(req.cpu.to_bits(), Ordering::Relaxed);
        self.memory_usage.fetch_add(req.memory, Ordering::Relaxed);
        self.io_operations
            .fetch_add(req.io.weight(), Ordering::Relaxed);
        self.network_connections
            .fetch_add(req.network.weight(), Ordering::Relaxed);

        let allocation = ResourceAllocation {
            execution_id: execution_id.to_string(),
            requirements: req.clone(),
            allocated_at: Instant::now(),
        };

        {
            let mut allocations = self.allocations.write().await;
            allocations.insert(execution_id.to_string(), allocation);
        }

        {
            let mut stats = self.stats.write().await;
            stats.total_allocations += 1;
        }

        Some(ResourceGuard {
            execution_id: execution_id.to_string(),
            requirements: req,
            _permit: permit,
            manager: self,
        })
    }

    /// Allocates resources, waiting if necessary.
    pub async fn allocate<'a>(
        &'a self,
        execution_id: &str,
        req: ResourceRequirements,
    ) -> ResourceGuard<'a> {
        // Acquire query permit (will wait if at limit)
        let permit = self
            .query_semaphore
            .clone()
            .acquire_owned()
            .await
            .expect("semaphore closed");

        // Wait for resources to become available
        loop {
            let usage = self.current_usage();
            if usage.can_accommodate(&self.limits, &req) {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }

        // Allocate resources
        self.cpu_usage
            .fetch_add(req.cpu.to_bits(), Ordering::Relaxed);
        self.memory_usage.fetch_add(req.memory, Ordering::Relaxed);
        self.io_operations
            .fetch_add(req.io.weight(), Ordering::Relaxed);
        self.network_connections
            .fetch_add(req.network.weight(), Ordering::Relaxed);

        let allocation = ResourceAllocation {
            execution_id: execution_id.to_string(),
            requirements: req.clone(),
            allocated_at: Instant::now(),
        };

        {
            let mut allocations = self.allocations.write().await;
            allocations.insert(execution_id.to_string(), allocation);
        }

        {
            let mut stats = self.stats.write().await;
            stats.total_allocations += 1;
        }

        ResourceGuard {
            execution_id: execution_id.to_string(),
            requirements: req,
            _permit: permit,
            manager: self,
        }
    }

    /// Allocates a binary stream slot.
    pub async fn allocate_binary_stream<'a>(&'a self) -> Option<BinaryStreamGuard<'a>> {
        let permit = self
            .binary_stream_semaphore
            .clone()
            .try_acquire_owned()
            .ok()?;

        self.binary_streams.fetch_add(1, Ordering::Relaxed);

        Some(BinaryStreamGuard {
            _permit: permit,
            manager: self,
        })
    }

    /// Releases allocated resources.
    #[allow(dead_code)]
    async fn release(&self, execution_id: &str, req: &ResourceRequirements) {
        self.cpu_usage
            .fetch_sub(req.cpu.to_bits(), Ordering::Relaxed);
        self.memory_usage.fetch_sub(req.memory, Ordering::Relaxed);
        self.io_operations
            .fetch_sub(req.io.weight(), Ordering::Relaxed);
        self.network_connections
            .fetch_sub(req.network.weight(), Ordering::Relaxed);

        let allocation = {
            let mut allocations = self.allocations.write().await;
            allocations.remove(execution_id)
        };

        if let Some(alloc) = allocation {
            let mut stats = self.stats.write().await;
            stats.total_releases += 1;
            stats.total_execution_time += alloc.allocated_at.elapsed();
        }
    }

    /// Returns resource statistics.
    pub async fn stats(&self) -> ResourceStats {
        self.stats.read().await.clone()
    }

    /// Returns allocation info for an execution.
    pub async fn get_allocation(&self, execution_id: &str) -> Option<ResourceAllocation> {
        let allocations = self.allocations.read().await;
        allocations.get(execution_id).cloned()
    }
}

impl Default for ResourceManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Guard that releases resources when dropped.
pub struct ResourceGuard<'a> {
    execution_id: String,
    requirements: ResourceRequirements,
    _permit: tokio::sync::OwnedSemaphorePermit,
    manager: &'a ResourceManager,
}

impl<'a> ResourceGuard<'a> {
    /// Returns the execution ID.
    pub fn execution_id(&self) -> &str {
        &self.execution_id
    }

    /// Returns the allocated requirements.
    pub fn requirements(&self) -> &ResourceRequirements {
        &self.requirements
    }
}

impl<'a> Drop for ResourceGuard<'a> {
    fn drop(&mut self) {
        // Note: We can't call async release here, so we do sync cleanup
        // The semaphore permit is automatically released
        self.manager
            .cpu_usage
            .fetch_sub(self.requirements.cpu.to_bits(), Ordering::Relaxed);
        self.manager
            .memory_usage
            .fetch_sub(self.requirements.memory, Ordering::Relaxed);
        self.manager
            .io_operations
            .fetch_sub(self.requirements.io.weight(), Ordering::Relaxed);
        self.manager
            .network_connections
            .fetch_sub(self.requirements.network.weight(), Ordering::Relaxed);
    }
}

/// Guard for binary stream allocation.
pub struct BinaryStreamGuard<'a> {
    _permit: tokio::sync::OwnedSemaphorePermit,
    manager: &'a ResourceManager,
}

impl<'a> Drop for BinaryStreamGuard<'a> {
    fn drop(&mut self) {
        self.manager.binary_streams.fetch_sub(1, Ordering::Relaxed);
    }
}

/// Resource allocation record.
#[derive(Debug, Clone)]
pub struct ResourceAllocation {
    /// Execution ID.
    pub execution_id: String,

    /// Allocated requirements.
    pub requirements: ResourceRequirements,

    /// When allocated.
    pub allocated_at: Instant,
}

/// Resource usage statistics.
#[derive(Debug, Clone, Default)]
pub struct ResourceStats {
    /// Total allocations made.
    pub total_allocations: usize,

    /// Total releases made.
    pub total_releases: usize,

    /// Total execution time.
    pub total_execution_time: Duration,

    /// Peak CPU usage.
    pub peak_cpu: f64,

    /// Peak memory usage.
    pub peak_memory: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resource_requirements() {
        let req = ResourceRequirements::minimal();
        assert!(req.score() > 0.0);

        let cpu_req = ResourceRequirements::cpu_intensive(0.8);
        assert!(cpu_req.score() > req.score());
    }

    #[test]
    fn test_resource_usage_can_accommodate() {
        let usage = ResourceUsage {
            cpu: 0.5,
            memory: 512 * 1024 * 1024,
            io_operations: 50,
            network_connections: 50,
            binary_streams: 5,
        };

        let limits = ResourceLimits::default();

        let small_req = ResourceRequirements::minimal();
        assert!(usage.can_accommodate(&limits, &small_req));

        let huge_req = ResourceRequirements {
            cpu: 100.0,
            memory: 100 * 1024 * 1024 * 1024,
            io: ResourceLevel::High,
            network: ResourceLevel::High,
            estimated_duration: None,
        };
        assert!(!usage.can_accommodate(&limits, &huge_req));
    }

    #[tokio::test]
    async fn test_resource_manager_allocation() {
        let manager = ResourceManager::new();

        let req = ResourceRequirements::minimal();
        let guard = manager.try_allocate("exec-1", req.clone()).await;
        assert!(guard.is_some());

        let usage = manager.current_usage();
        assert!(usage.cpu > 0.0);

        drop(guard);

        let usage = manager.current_usage();
        assert_eq!(usage.cpu, 0.0);
    }

    #[tokio::test]
    async fn test_binary_stream_allocation() {
        let limits = ResourceLimits {
            max_binary_streams: 2,
            ..Default::default()
        };
        let manager = ResourceManager::with_limits(limits);

        let _guard1 = manager.allocate_binary_stream().await;
        assert!(_guard1.is_some());

        let _guard2 = manager.allocate_binary_stream().await;
        assert!(_guard2.is_some());

        // Should fail - at limit
        let guard3 = manager.allocate_binary_stream().await;
        assert!(guard3.is_none());
    }
}
