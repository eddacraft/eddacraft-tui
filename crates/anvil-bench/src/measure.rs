use std::time::{Duration, Instant};

/// Memory usage snapshot in kilobytes, read from `/proc/self/status`.
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct MemorySnapshot {
    /// Resident Set Size in KiB.
    pub rss_kib: u64,
    /// Virtual memory size in KiB.
    pub vm_size_kib: u64,
}

impl MemorySnapshot {
    /// Take a snapshot of current process memory usage.
    ///
    /// On non-Linux platforms, returns zeros.
    pub fn now() -> Self {
        read_proc_status().unwrap_or(Self {
            rss_kib: 0,
            vm_size_kib: 0,
        })
    }

    /// RSS in megabytes (for display convenience).
    #[must_use]
    pub fn rss_mib(&self) -> f64 {
        self.rss_kib as f64 / 1024.0
    }
}

/// Before/after memory measurement.
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct MemoryDelta {
    pub before: MemorySnapshot,
    pub after: MemorySnapshot,
    pub delta_rss_kib: i64,
}

impl MemoryDelta {
    #[must_use]
    pub fn delta_rss_mib(&self) -> f64 {
        self.delta_rss_kib as f64 / 1024.0
    }
}

/// Guard that captures memory before construction and after `finish()`.
pub struct MemoryGuard {
    before: MemorySnapshot,
}

impl MemoryGuard {
    #[must_use]
    pub fn start() -> Self {
        Self {
            before: MemorySnapshot::now(),
        }
    }

    #[must_use]
    pub fn finish(self) -> MemoryDelta {
        let after = MemorySnapshot::now();
        let delta_rss_kib = i64::try_from(after.rss_kib).unwrap_or(i64::MAX)
            - i64::try_from(self.before.rss_kib).unwrap_or(i64::MAX);
        MemoryDelta {
            before: self.before,
            after,
            delta_rss_kib,
        }
    }
}

/// Timing measurement for a named operation.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TimingResult {
    pub label: String,
    pub duration: Duration,
    pub iterations: u64,
}

impl TimingResult {
    /// Average duration per iteration.
    #[must_use]
    pub fn per_iteration(&self) -> Duration {
        if self.iterations == 0 {
            return Duration::ZERO;
        }
        self.duration / u32::try_from(self.iterations).unwrap_or(u32::MAX)
    }
}

/// Run a closure `iterations` times, returning total elapsed time.
pub fn time_iterations<F>(label: &str, iterations: u64, mut f: F) -> TimingResult
where
    F: FnMut(),
{
    let start = Instant::now();
    for _ in 0..iterations {
        f();
    }
    let duration = start.elapsed();
    TimingResult {
        label: label.to_string(),
        duration,
        iterations,
    }
}

/// Run a closure repeatedly for at least the given duration, returning
/// throughput metrics.
pub fn time_throughput<F>(label: &str, min_duration: Duration, mut f: F) -> TimingResult
where
    F: FnMut(),
{
    let start = Instant::now();
    let mut iterations = 0u64;
    while start.elapsed() < min_duration {
        f();
        iterations += 1;
    }
    let duration = start.elapsed();
    TimingResult {
        label: label.to_string(),
        duration,
        iterations,
    }
}

#[cfg(target_os = "linux")]
fn read_proc_status() -> Option<MemorySnapshot> {
    let content = std::fs::read_to_string("/proc/self/status").ok()?;
    let mut rss_kib = 0u64;
    let mut vm_size_kib = 0u64;

    for line in content.lines() {
        if let Some(val) = line.strip_prefix("VmRSS:") {
            rss_kib = parse_kb_value(val);
        } else if let Some(val) = line.strip_prefix("VmSize:") {
            vm_size_kib = parse_kb_value(val);
        }
    }

    Some(MemorySnapshot {
        rss_kib,
        vm_size_kib,
    })
}

#[cfg(not(target_os = "linux"))]
fn read_proc_status() -> Option<MemorySnapshot> {
    None
}

#[cfg(target_os = "linux")]
fn parse_kb_value(s: &str) -> u64 {
    s.trim()
        .strip_suffix("kB")
        .or_else(|| s.trim().strip_suffix("KB"))
        .unwrap_or(s.trim())
        .trim()
        .parse()
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memory_snapshot_returns_nonzero_on_linux() {
        let snap = MemorySnapshot::now();
        if cfg!(target_os = "linux") {
            assert!(snap.rss_kib > 0, "RSS should be > 0 on Linux");
            assert!(snap.vm_size_kib > 0, "VmSize should be > 0 on Linux");
        }
    }

    #[test]
    fn memory_guard_captures_delta() {
        let guard = MemoryGuard::start();
        // Allocate some memory to make delta detectable
        let _data: Vec<u8> = vec![0u8; 1024 * 1024];
        let delta = guard.finish();
        // Delta could be positive or zero depending on OS behaviour,
        // but should not panic.
        let _ = delta.delta_rss_mib();
    }

    #[test]
    fn time_iterations_counts_correctly() {
        let mut counter = 0u64;
        let result = time_iterations("test", 10, || counter += 1);
        assert_eq!(counter, 10);
        assert_eq!(result.iterations, 10);
        assert_eq!(result.label, "test");
    }

    #[test]
    fn time_throughput_runs_for_minimum_duration() {
        let result = time_throughput("throughput", Duration::from_millis(50), || {
            std::hint::black_box(42);
        });
        assert!(result.duration >= Duration::from_millis(50));
        assert!(result.iterations > 0);
    }

    #[test]
    fn parse_kb_value_handles_variants() {
        assert_eq!(parse_kb_value("  12345 kB"), 12345);
        assert_eq!(parse_kb_value("  67890 KB"), 67890);
        assert_eq!(parse_kb_value("  999  "), 999);
    }
}
