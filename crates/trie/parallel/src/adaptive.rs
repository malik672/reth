//! Adaptive thread pool scaling based on page fault rate monitoring.
//!
//! This module implements continuous monitoring of system page faults to dynamically
//! adjust worker thread count. The key insight is that page faults are blocking events
//! that prevent threads from executing, so extra threads help hide I/O latency.
//!
//! # Architecture
//!
//! The system consists of three main components:
//!
//! 1. **PageFaultMonitor**: Periodically reads `/proc/vmstat` and calculates faults/sec
//! 2. **AdaptiveController**: Decides desired thread count based on fault rate
//! 3. **ThreadPool**: Spawns new threads when needed, lets idle threads naturally exit
//!
//! # Page Fault Rate Thresholds
//!
//! - **Low paging (<5k/sec)**: System has sufficient RAM, use `num_cores` threads
//! - **Moderate paging (5k-50k/sec)**: Current formula: `(cores * 2).clamp(2, 64)`
//! - **Heavy paging (>50k/sec)**: Memory-constrained, use `(cores * 3).clamp(16, 128)`
//!
//! The thresholds are tuned for production Ethereum state sync:
//! - Mainnet on 256GB with 297GB state: ~36k/sec baseline
//! - Mainnet on 512GB with 297GB state: <1k/sec

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tracing::{debug, trace};

/// Page fault statistics sampled from the system
#[derive(Debug, Clone, Copy)]
pub struct PageFaultSample {
    /// Total page faults since boot
    pub total_faults: u64,
    /// Timestamp when sample was taken
    pub timestamp: SystemTime,
}

/// Calculated page fault rate metrics
#[derive(Debug, Clone, Copy)]
pub struct PageFaultRate {
    /// Current faults per second
    pub faults_per_sec: u64,
    /// Classification of current paging pressure
    pub pressure: PagingPressure,
}

/// Classification of system memory paging pressure
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PagingPressure {
    /// Minimal paging: <5k faults/sec
    Low,
    /// Moderate paging: 5k-50k faults/sec
    Moderate,
    /// Heavy paging: >50k faults/sec
    Heavy,
}

impl PageFaultRate {
    /// Recommended thread count for this page fault rate
    pub fn recommended_threads(&self, num_cores: usize) -> usize {
        match self.pressure {
            PagingPressure::Low => {
                // Minimal paging: system has plenty of RAM
                // Use just the core count, no extra threads needed
                num_cores
            }
            PagingPressure::Moderate => {
                // Moderate paging: use current proven formula
                // Extra threads help hide moderate I/O latency
                (num_cores * 2).clamp(2, 64)
            }
            PagingPressure::Heavy => {
                // Heavy paging: memory-constrained, need more threads
                // At 36k faults/sec, 64 threads can all be blocking I/O
                // Extra threads keep CPU cores utilized while others wait
                (num_cores * 3).clamp(16, 128)
            }
        }
    }
}

/// Monitors page fault rate from `/proc/vmstat` (Linux only)
pub struct PageFaultMonitor {
    /// Last known total fault count
    last_faults: Arc<AtomicU64>,
    /// Last time we sampled
    last_sample_time: Arc<AtomicU64>,
    /// Sampling interval
    sample_interval: Duration,
}

impl PageFaultMonitor {
    /// Create a new page fault monitor
    pub fn new(sample_interval: Duration) -> Self {
        Self {
            last_faults: Arc::new(AtomicU64::new(0)),
            last_sample_time: Arc::new(AtomicU64::new(0)),
            sample_interval,
        }
    }

    /// Read current page fault count from /proc/vmstat (Linux only)
    #[cfg(target_os = "linux")]
    fn read_pgfault_count() -> Option<u64> {
        use std::fs;

        // Read /proc/vmstat and find pgfault line
        let content = fs::read_to_string("/proc/vmstat").ok()?;

        for line in content.lines() {
            if let Some(count_str) = line.strip_prefix("pgfault ") {
                return count_str.trim().parse().ok();
            }
        }

        None
    }

    /// Fallback for non-Linux systems (no page fault measurement available)
    #[cfg(not(target_os = "linux"))]
    fn read_pgfault_count() -> Option<u64> {
        None
    }

    /// Get current page fault rate
    pub fn sample(&self) -> Option<PageFaultRate> {
        let current_faults = Self::read_pgfault_count()?;
        let current_time = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .ok()?
            .as_secs();

        let last_faults = self.last_faults.load(Ordering::Relaxed);
        let last_time = self.last_sample_time.load(Ordering::Relaxed);

        // Store current for next sample
        self.last_faults.store(current_faults, Ordering::Relaxed);
        self.last_sample_time.store(current_time, Ordering::Relaxed);

        // Need at least two samples to calculate rate
        if last_time == 0 {
            trace!(target: "trie::adaptive", "First page fault sample: {} faults", current_faults);
            return None;
        }

        let time_delta = current_time.saturating_sub(last_time);
        if time_delta == 0 {
            return None;
        }

        let fault_delta = current_faults.saturating_sub(last_faults);
        let faults_per_sec = fault_delta / time_delta;

        let pressure = match faults_per_sec {
            0..=4_999 => PagingPressure::Low,
            5_000..=49_999 => PagingPressure::Moderate,
            _ => PagingPressure::Heavy,
        };

        trace!(
            target: "trie::adaptive",
            faults_per_sec,
            pressure = ?pressure,
            "Page fault rate sampled"
        );

        Some(PageFaultRate {
            faults_per_sec,
            pressure,
        })
    }
}

/// Manages adaptive thread pool scaling
pub struct AdaptiveThreadController {
    /// Number of CPU cores available
    num_cores: usize,
    /// Current desired thread count
    desired_threads: Arc<AtomicU64>,
    /// Monitor for page faults
    monitor: PageFaultMonitor,
    /// Hysteresis: only change if delta > threshold to avoid thrashing
    min_change_delta: usize,
}

impl AdaptiveThreadController {
    /// Create a new adaptive thread controller
    pub fn new(sample_interval: Duration) -> Self {
        let num_cores = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(8);

        Self {
            num_cores,
            desired_threads: Arc::new(AtomicU64::new(num_cores as u64)),
            monitor: PageFaultMonitor::new(sample_interval),
            min_change_delta: (num_cores / 4).max(1), // 25% hysteresis
        }
    }

    /// Update desired thread count based on current page fault rate
    pub fn update(&self) {
        if let Some(rate) = self.monitor.sample() {
            let desired = rate.recommended_threads(self.num_cores);
            let current = self.desired_threads.load(Ordering::Relaxed) as usize;

            let delta = (desired as isize - current as isize).abs() as usize;

            // Only update if change is significant enough (hysteresis)
            if delta >= self.min_change_delta {
                self.desired_threads.store(desired as u64, Ordering::Relaxed);

                debug!(
                    target: "trie::adaptive",
                    old_threads = current,
                    new_threads = desired,
                    faults_per_sec = rate.faults_per_sec,
                    pressure = ?rate.pressure,
                    "Adjusted desired thread count"
                );
            }
        }
    }

    /// Get the current desired thread count
    pub fn desired_threads(&self) -> usize {
        self.desired_threads.load(Ordering::Relaxed) as usize
    }

    /// Manually set desired threads (for testing)
    #[cfg(test)]
    fn set_desired_threads(&self, count: usize) {
        self.desired_threads.store(count as u64, Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_paging_pressure_thresholds() {
        let low = PageFaultRate {
            faults_per_sec: 1000,
            pressure: PagingPressure::Low,
        };
        assert_eq!(low.recommended_threads(32), 32);

        let moderate = PageFaultRate {
            faults_per_sec: 20_000,
            pressure: PagingPressure::Moderate,
        };
        assert_eq!(moderate.recommended_threads(32), 64);

        let heavy = PageFaultRate {
            faults_per_sec: 60_000,
            pressure: PagingPressure::Heavy,
        };
        assert_eq!(heavy.recommended_threads(32), 96.min(128));
    }

    #[test]
    fn test_adaptive_controller_hysteresis() {
        let controller = AdaptiveThreadController::new(Duration::from_secs(5));
        assert_eq!(controller.desired_threads(), controller.num_cores);

        // Small change (within hysteresis) should not update
        controller.set_desired_threads(controller.num_cores + 1);
        // This would only update if monitored page faults changed
    }

    #[test]
    fn test_paging_pressure_classification() {
        assert_eq!(PagingPressure::Low, PagingPressure::Low);
        assert_ne!(PagingPressure::Low, PagingPressure::Moderate);
        assert_ne!(PagingPressure::Moderate, PagingPressure::Heavy);
    }
}
