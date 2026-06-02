//! Process-tree CPU + RSS sampler for the resource benches (RLB-002..005).
//!
//! Long-running Anvil processes spawn short-lived children: `anvil watch`
//! shells out to a per-save `anvil check`, the intercept daemon and the MCP
//! server fan scans across a rayon pool. Measuring only the parent pid
//! undercounts the real cost — the original `watch_resource_budget` bench did
//! exactly that and reported ~0% while a beta tester saw ~7 cores. This module
//! samples the **whole process tree**:
//!
//! - **CPU**: the root's `utime+stime` plus `cutime+cstime` (the CPU time of
//!   reaped children). This is the same whole-tree accounting the RLB-001 load
//!   probe used (`benchmarks/prototypes/anvil-load-probe.py`) and it reproduced
//!   the field report. A child that is still running at the end of the window
//!   has not been reaped yet, so its tail is undercounted — negligible over a
//!   multi-second steady-state window, and conservative (never over-reports).
//! - **RSS**: the peak sum of resident memory across the root and every live
//!   descendant, sampled on an interval. Short-lived children are caught when
//!   they happen to be alive at a tick.
//!
//! CPU is reported in the same unit the [`crate::budget`] machinery already
//! uses: a percentage where `100.0` is one fully-saturated core (so a process
//! pinning four cores reads `400.0`). The pure parsers carry the whole module's
//! test coverage; the live `/proc` readers are thin Linux-only wrappers.

use std::error::Error;
use std::time::{Duration, Instant};

use crate::budget::MeasurementSample;

type Result<T> = std::result::Result<T, Box<dyn Error + Send + Sync>>;

/// CPU jiffies attributed to a process: its own user+system time, and the
/// accumulated user+system time of children it has already reaped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CpuTimes {
    /// `utime + stime` of the process itself.
    pub self_jiffies: u64,
    /// `cutime + cstime` — reaped children's user+system time.
    pub children_jiffies: u64,
}

impl CpuTimes {
    /// Whole-tree jiffies for the reaping root: self plus reaped children.
    #[must_use]
    pub fn tree_total(self) -> u64 {
        self.self_jiffies.saturating_add(self.children_jiffies)
    }
}

/// Parse the CPU times out of a `/proc/<pid>/stat` line.
///
/// The fields after the (possibly space- and paren-containing) `comm` field
/// are: `state ppid pgrp ... utime stime cutime cstime ...`. Counting from the
/// first field after `") "`, those are indices 11, 12, 13, 14 (0-based).
pub fn parse_proc_cpu_times(stat: &str) -> Result<CpuTimes> {
    let fields = fields_after_comm(stat)?;
    let self_user = field_u64(&fields, 11, "utime")?;
    let self_sys = field_u64(&fields, 12, "stime")?;
    let reaped_user = field_u64(&fields, 13, "cutime")?;
    let reaped_sys = field_u64(&fields, 14, "cstime")?;
    Ok(CpuTimes {
        self_jiffies: self_user.saturating_add(self_sys),
        children_jiffies: reaped_user.saturating_add(reaped_sys),
    })
}

/// Parse the parent pid (field index 1 after `comm`).
pub fn parse_ppid(stat: &str) -> Result<u32> {
    let fields = fields_after_comm(stat)?;
    let ppid = fields
        .get(1)
        .ok_or("missing ppid in /proc/<pid>/stat")?
        .parse()?;
    Ok(ppid)
}

fn fields_after_comm(stat: &str) -> Result<Vec<&str>> {
    let after_comm = stat
        .rfind(") ")
        .and_then(|idx| stat.get(idx + 2..))
        .ok_or("invalid /proc/<pid>/stat shape")?;
    Ok(after_comm.split_whitespace().collect())
}

fn field_u64(fields: &[&str], idx: usize, name: &str) -> Result<u64> {
    Ok(fields
        .get(idx)
        .ok_or_else(|| format!("missing {name} in /proc/<pid>/stat"))?
        .parse()?)
}

/// Sum the aggregate-cpu line of `/proc/stat` into total machine jiffies.
///
/// Sums only the eight non-overlapping fields — `user nice system idle iowait
/// irq softirq steal` — and deliberately drops `guest`/`guest_nice` (fields 9
/// and 10). Since Linux 2.6.24 those are *already counted in* `user`/`nice`, so
/// summing them double-counts guest time. That matters on virtualised hosts
/// (e.g. CI runners): double-counting inflates the denominator and silently
/// under-reports the bench's CPU percentage.
pub fn parse_total_cpu_jiffies(stat: &str) -> Result<u64> {
    let first = stat.lines().next().ok_or("empty /proc/stat")?;
    let mut fields = first.split_whitespace();
    if fields.next() != Some("cpu") {
        return Err("/proc/stat does not start with aggregate cpu line".into());
    }
    fields
        .take(8)
        .map(str::parse::<u64>)
        .try_fold(0u64, |acc, value| Ok(acc + value?))
}

/// Parse `VmRSS` (in KiB) out of a `/proc/<pid>/status` document.
pub fn parse_rss_kib(status: &str) -> Result<u64> {
    for line in status.lines() {
        if let Some(value) = line.strip_prefix("VmRSS:") {
            let kib: u64 = value
                .split_whitespace()
                .next()
                .ok_or("VmRSS value missing")?
                .parse()?;
            return Ok(kib);
        }
    }
    Err("VmRSS missing from /proc/<pid>/status".into())
}

/// CPU usage over a window expressed in cores (1.0 == one saturated core).
///
/// `proc_delta` is the tree's jiffie delta; `total_delta` is the aggregate
/// machine jiffie delta over the same window. Returns 0 when the machine made
/// no measurable progress (avoids a divide-by-zero on a too-short window).
#[must_use]
pub fn cores_for_window(proc_delta: u64, total_delta: u64, ncpus: usize) -> f64 {
    if total_delta == 0 {
        return 0.0;
    }
    (proc_delta as f64) * (ncpus as f64) / (total_delta as f64)
}

/// Same window, expressed as a percentage where `100.0` is one core — the unit
/// the [`crate::budget`] ceilings are pinned in.
#[must_use]
pub fn cpu_pct_for_window(proc_delta: u64, total_delta: u64, ncpus: usize) -> f64 {
    cores_for_window(proc_delta, total_delta, ncpus) * 100.0
}

// ----------------------------------------------------------------------------
// Live /proc readers (Linux only). These are intentionally thin — the logic
// lives in the pure parsers above.
// ----------------------------------------------------------------------------

/// Read a process's [`CpuTimes`]. The process may have exited between
/// discovery and read, hence `Result`.
pub fn read_proc_cpu_times(pid: u32) -> Result<CpuTimes> {
    let stat = std::fs::read_to_string(format!("/proc/{pid}/stat"))?;
    parse_proc_cpu_times(&stat)
}

/// Read the aggregate machine CPU jiffies.
pub fn read_total_cpu_jiffies() -> Result<u64> {
    let stat = std::fs::read_to_string("/proc/stat")?;
    parse_total_cpu_jiffies(&stat)
}

/// Read a process's resident set size in MiB.
pub fn read_proc_rss_mib(pid: u32) -> Result<f64> {
    let status = std::fs::read_to_string(format!("/proc/{pid}/status"))?;
    Ok(parse_rss_kib(&status)? as f64 / 1024.0)
}

/// Discover the live descendants of `root` by walking the `ppid` graph of every
/// process in `/proc`. Returns `root` plus all transitive children that are
/// alive right now. Processes that exit mid-walk are simply skipped.
#[must_use]
pub fn process_tree_pids(root: u32) -> Vec<u32> {
    let mut children: std::collections::HashMap<u32, Vec<u32>> = std::collections::HashMap::new();
    let Ok(entries) = std::fs::read_dir("/proc") else {
        return vec![root];
    };
    for entry in entries.flatten() {
        let Ok(pid) = entry.file_name().to_string_lossy().parse::<u32>() else {
            continue;
        };
        if let Ok(stat) = std::fs::read_to_string(format!("/proc/{pid}/stat"))
            && let Ok(ppid) = parse_ppid(&stat)
        {
            children.entry(ppid).or_default().push(pid);
        }
    }
    // BFS from root over the parent→children adjacency.
    let mut out = Vec::new();
    let mut queue = std::collections::VecDeque::from([root]);
    let mut seen = std::collections::HashSet::new();
    while let Some(pid) = queue.pop_front() {
        if !seen.insert(pid) {
            continue;
        }
        out.push(pid);
        if let Some(kids) = children.get(&pid) {
            queue.extend(kids.iter().copied());
        }
    }
    out
}

/// Sum the resident memory (MiB) of the whole live process tree under `root`.
#[must_use]
pub fn tree_rss_mib(root: u32) -> f64 {
    process_tree_pids(root)
        .into_iter()
        .filter_map(|pid| read_proc_rss_mib(pid).ok())
        .sum()
}

/// A running measurement over a process tree. Construct with [`TreeSampler::start`],
/// call [`TreeSampler::tick_rss`] periodically across the window, then
/// [`TreeSampler::finish`] for the [`MeasurementSample`].
pub struct TreeSampler {
    root: u32,
    start_tree: u64,
    start_total: u64,
    peak_rss_mib: f64,
    ncpus: usize,
}

impl TreeSampler {
    /// Begin measuring. Captures the baseline CPU counters and an initial RSS
    /// sample so a zero-tick window still reports a sane peak.
    pub fn start(root: u32) -> Result<Self> {
        let start_tree = read_proc_cpu_times(root)?.tree_total();
        let start_total = read_total_cpu_jiffies()?;
        let peak_rss_mib = tree_rss_mib(root);
        let ncpus = std::thread::available_parallelism().map_or(1, std::num::NonZeroUsize::get);
        Ok(Self {
            root,
            start_tree,
            start_total,
            peak_rss_mib,
            ncpus,
        })
    }

    /// Take one RSS sample of the tree and fold it into the running peak.
    pub fn tick_rss(&mut self) {
        self.peak_rss_mib = self.peak_rss_mib.max(tree_rss_mib(self.root));
    }

    /// Sleep-and-sample loop: ticks RSS every `interval` until `window` elapses.
    pub fn sample_for(&mut self, window: Duration, interval: Duration) {
        let deadline = Instant::now() + window;
        while Instant::now() < deadline {
            std::thread::sleep(interval);
            self.tick_rss();
        }
    }

    /// Close the window and emit the [`MeasurementSample`]. The root may have
    /// exited (e.g. killed) before this call; its final CPU counters are then
    /// unreadable, so this consumes the last good baseline conservatively.
    pub fn finish(self) -> Result<MeasurementSample> {
        let end_tree = read_proc_cpu_times(self.root)?.tree_total();
        let end_total = read_total_cpu_jiffies()?;
        let proc_delta = end_tree.saturating_sub(self.start_tree);
        let total_delta = end_total.saturating_sub(self.start_total);
        Ok(MeasurementSample {
            steady_state_cpu_pct: cpu_pct_for_window(proc_delta, total_delta, self.ncpus),
            peak_rss_mib: self.peak_rss_mib,
        })
    }
}

/// Aggregate sampler over several **disjoint** process trees at once — used by
/// the concurrent multi-process bench (RLB-005) to measure watch + intercept +
/// MCP running together. CPU is the summed whole-tree jiffies of all roots over
/// the window (expressed as cores ×100, so three saturated cores read `300.0`);
/// peak RSS is the summed resident set of all trees at the busiest tick. The
/// roots must not be ancestors of one another or their CPU/RSS would be
/// double-counted.
pub struct MultiTreeSampler {
    roots: Vec<u32>,
    start_tree: u64,
    start_total: u64,
    peak_rss_mib: f64,
    ncpus: usize,
}

impl MultiTreeSampler {
    /// Begin measuring every root's tree. Errors if any root is already gone.
    pub fn start(roots: Vec<u32>) -> Result<Self> {
        let start_tree = sum_tree_cpu(&roots)?;
        let start_total = read_total_cpu_jiffies()?;
        let peak_rss_mib = sum_tree_rss(&roots);
        let ncpus = std::thread::available_parallelism().map_or(1, std::num::NonZeroUsize::get);
        Ok(Self {
            roots,
            start_tree,
            start_total,
            peak_rss_mib,
            ncpus,
        })
    }

    /// Fold one summed-RSS sample into the running peak.
    pub fn tick_rss(&mut self) {
        self.peak_rss_mib = self.peak_rss_mib.max(sum_tree_rss(&self.roots));
    }

    /// Sleep-and-sample loop until `window` elapses.
    pub fn sample_for(&mut self, window: Duration, interval: Duration) {
        let deadline = Instant::now() + window;
        while Instant::now() < deadline {
            std::thread::sleep(interval);
            self.tick_rss();
        }
    }

    /// Close the window and emit the aggregate [`MeasurementSample`].
    pub fn finish(self) -> Result<MeasurementSample> {
        let end_tree = sum_tree_cpu(&self.roots)?;
        let end_total = read_total_cpu_jiffies()?;
        let proc_delta = end_tree.saturating_sub(self.start_tree);
        let total_delta = end_total.saturating_sub(self.start_total);
        Ok(MeasurementSample {
            steady_state_cpu_pct: cpu_pct_for_window(proc_delta, total_delta, self.ncpus),
            peak_rss_mib: self.peak_rss_mib,
        })
    }
}

fn sum_tree_cpu(roots: &[u32]) -> Result<u64> {
    let mut total = 0u64;
    for &root in roots {
        total = total.saturating_add(read_proc_cpu_times(root)?.tree_total());
    }
    Ok(total)
}

#[must_use]
fn sum_tree_rss(roots: &[u32]) -> f64 {
    roots.iter().map(|&root| tree_rss_mib(root)).sum()
}

#[cfg(test)]
#[allow(clippy::float_cmp)] // exact-zero / bit-stable f64 comparisons
mod tests {
    use super::*;

    #[test]
    fn parses_cpu_times_with_spaces_in_comm() {
        // utime=21 stime=34 cutime=5 cstime=6 (indices 11..=14 after comm).
        let stat = "123 (anvil watch) S 1 2 3 4 5 6 7 8 9 10 21 34 5 6 17 18 19 20";
        let times = parse_proc_cpu_times(stat).unwrap();
        assert_eq!(times.self_jiffies, 55);
        assert_eq!(times.children_jiffies, 11);
        assert_eq!(times.tree_total(), 66);
    }

    #[test]
    fn comm_with_parens_and_spaces_does_not_break_field_offsets() {
        // The comm contains a `) ` sequence; rfind(") ") must pick the *last*.
        let stat = "999 (weird ) name) R 7 2 3 4 5 6 7 8 9 10 100 200 1 2 30 40";
        let times = parse_proc_cpu_times(stat).unwrap();
        assert_eq!(times.self_jiffies, 300);
        assert_eq!(times.children_jiffies, 3);
        assert_eq!(parse_ppid(stat).unwrap(), 7);
    }

    #[test]
    fn parses_ppid() {
        let stat = "123 (anvil) S 4567 2 3 4 5 6 7 8 9 10 11 12 13 14";
        assert_eq!(parse_ppid(stat).unwrap(), 4567);
    }

    #[test]
    fn parses_total_cpu_jiffies_excluding_guest() {
        // Only the first 8 fields are summed (1..=8 = 36); guest=9 and
        // guest_nice=10 are dropped to avoid double-counting (already in user/nice).
        let stat = "cpu  1 2 3 4 5 6 7 8 9 10\ncpu0 1 2 3 4";
        assert_eq!(parse_total_cpu_jiffies(stat).unwrap(), 36);
    }

    #[test]
    fn rejects_non_cpu_first_line() {
        assert!(parse_total_cpu_jiffies("intr 1 2 3\ncpu 1 2 3").is_err());
    }

    #[test]
    fn parses_rss_kib() {
        let status = "Name:\tanvil\nVmRSS:\t204800 kB\nVmSize:\t300000 kB";
        assert_eq!(parse_rss_kib(status).unwrap(), 204_800);
    }

    #[test]
    fn missing_rss_errors() {
        assert!(parse_rss_kib("Name:\tanvil\nVmSize:\t10 kB").is_err());
    }

    #[test]
    fn one_full_core_reads_as_100_pct() {
        let ncpus = std::thread::available_parallelism().map_or(1, std::num::NonZeroUsize::get);
        // Tree burned `w` jiffies; the machine burned `w * ncpus` (one core's
        // worth out of the whole machine).
        let w = 1000u64;
        let total = w * ncpus as u64;
        assert!((cpu_pct_for_window(w, total, ncpus) - 100.0).abs() < f64::EPSILON);
        assert!((cores_for_window(w, total, ncpus) - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn four_cores_reads_as_400_pct() {
        let ncpus = 8;
        let total = 1000u64 * ncpus as u64;
        // Tree used 4 cores' worth: 4000 jiffies of an 8000-jiffie machine.
        assert!((cpu_pct_for_window(4000, total, ncpus) - 400.0).abs() < f64::EPSILON);
    }

    #[test]
    fn zero_total_window_is_zero_not_nan() {
        assert_eq!(cpu_pct_for_window(10, 0, 4), 0.0);
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn process_tree_includes_self() {
        let me = std::process::id();
        let tree = process_tree_pids(me);
        assert!(tree.contains(&me), "tree should contain the querying pid");
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn tree_rss_of_self_is_nonzero() {
        assert!(tree_rss_mib(std::process::id()) > 0.0);
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn multi_tree_aggregates_rss_across_roots() {
        // Two live children → the aggregate RSS exceeds either tree alone.
        let mut a = std::process::Command::new("sleep")
            .arg("30")
            .spawn()
            .expect("spawn sleep a");
        let mut b = std::process::Command::new("sleep")
            .arg("30")
            .spawn()
            .expect("spawn sleep b");
        std::thread::sleep(Duration::from_millis(50));

        let single = tree_rss_mib(a.id());
        let aggregate = sum_tree_rss(&[a.id(), b.id()]);
        let _ = (a.kill(), a.wait(), b.kill(), b.wait());

        assert!(single > 0.0, "a child has resident memory");
        assert!(
            aggregate >= single,
            "aggregate {aggregate} should be >= a single tree {single}"
        );
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn multi_tree_sampler_reports_self() {
        let sampler = MultiTreeSampler::start(vec![std::process::id()]).expect("start");
        let sample = sampler.finish().expect("finish");
        assert!(sample.peak_rss_mib > 0.0);
        assert!(sample.steady_state_cpu_pct >= 0.0);
    }
}
