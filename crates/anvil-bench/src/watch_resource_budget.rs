use std::error::Error;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use tempfile::TempDir;

use crate::budget::{BudgetVerdict, ResourceBudget, evaluate};
use crate::fixture::{RepoSpec, generate_repo};

type Result<T> = std::result::Result<T, Box<dyn Error + Send + Sync>>;

const DEFAULT_SETTLE_DURATION: Duration = Duration::from_secs(2);
const DEFAULT_MEASURE_DURATION: Duration = Duration::from_secs(3);
const DEFAULT_SAMPLE_INTERVAL: Duration = Duration::from_millis(200);

#[derive(Debug, Clone)]
pub struct WatchResourceBudgetConfig {
    pub anvil_bin: PathBuf,
    pub repo_spec: RepoSpec,
    pub settle_duration: Duration,
    pub measure_duration: Duration,
    pub sample_interval: Duration,
}

impl WatchResourceBudgetConfig {
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            anvil_bin: resolve_anvil_binary()?,
            repo_spec: RepoSpec::default(),
            settle_duration: DEFAULT_SETTLE_DURATION,
            measure_duration: DEFAULT_MEASURE_DURATION,
            sample_interval: DEFAULT_SAMPLE_INTERVAL,
        })
    }
}

pub fn run(config: &WatchResourceBudgetConfig) -> Result<BudgetVerdict> {
    if !cfg!(target_os = "linux") {
        return Err("watch resource budget sampling requires Linux /proc".into());
    }

    let tempdir = TempDir::new()?;
    let repo = generate_repo(&config.repo_spec, tempdir.path())?;
    let child = Command::new(&config.anvil_bin)
        .args(watch_command_args())
        .current_dir(repo.root())
        .env("ANVIL_DISABLE_UPDATE_HINT", "1")
        .env("ANVIL_DEV", "1")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    let mut child = ChildGuard::new(child);

    let pid = child.id();
    let sample = measure_process(pid, config)?;
    child.shutdown();

    Ok(evaluate(ResourceBudget::ANVIL_WATCH_V1, sample))
}

struct ChildGuard {
    child: Option<std::process::Child>,
}

impl ChildGuard {
    fn new(child: std::process::Child) -> Self {
        Self { child: Some(child) }
    }

    fn id(&self) -> u32 {
        self.child.as_ref().expect("child is live").id()
    }

    fn shutdown(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

impl Drop for ChildGuard {
    fn drop(&mut self) {
        self.shutdown();
    }
}

pub fn resolve_anvil_binary() -> Result<PathBuf> {
    if let Some(path) = std::env::var_os("ANVIL_BENCH_ANVIL_BIN") {
        return resolve_configured_anvil_binary(PathBuf::from(path));
    }

    let candidate = workspace_target_anvil("debug");
    if candidate.exists() {
        return Ok(candidate);
    }

    let candidate = workspace_target_anvil("release");
    if candidate.exists() {
        return Ok(candidate);
    }

    Err("set ANVIL_BENCH_ANVIL_BIN or build target/debug/anvil first".into())
}

fn resolve_configured_anvil_binary(path: PathBuf) -> Result<PathBuf> {
    if path.is_absolute() {
        return Ok(path);
    }

    let from_cwd = std::env::current_dir()?.join(&path);
    if from_cwd.exists() {
        return Ok(from_cwd);
    }

    Ok(Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(path))
}

pub fn watch_command_args() -> [&'static str; 5] {
    ["--json", "--no-tui", "watch", "--all", "--debounce=100"]
}

fn workspace_target_anvil(profile: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("target")
        .join(profile)
        .join("anvil")
}

fn measure_process(
    pid: u32,
    config: &WatchResourceBudgetConfig,
) -> Result<crate::budget::MeasurementSample> {
    std::thread::sleep(config.settle_duration);

    let start_proc = read_process_cpu_ticks(pid)?;
    let start_total = read_total_cpu_ticks()?;
    let mut peak_rss_mib = read_process_rss_mib(pid)?;
    let deadline = Instant::now() + config.measure_duration;

    while Instant::now() < deadline {
        std::thread::sleep(config.sample_interval);
        peak_rss_mib = peak_rss_mib.max(read_process_rss_mib(pid)?);
    }

    let end_proc = read_process_cpu_ticks(pid)?;
    let end_total = read_total_cpu_ticks()?;
    let steady_state_cpu_pct = cpu_pct_for_window(start_proc, end_proc, start_total, end_total);

    Ok(crate::budget::MeasurementSample {
        steady_state_cpu_pct,
        peak_rss_mib,
    })
}

fn read_process_cpu_ticks(pid: u32) -> Result<u64> {
    let stat = std::fs::read_to_string(format!("/proc/{pid}/stat"))?;
    parse_process_cpu_ticks(&stat)
}

fn parse_process_cpu_ticks(stat: &str) -> Result<u64> {
    let after_comm = stat
        .rfind(") ")
        .and_then(|idx| stat.get(idx + 2..))
        .ok_or("invalid /proc/<pid>/stat shape")?;
    let fields: Vec<&str> = after_comm.split_whitespace().collect();
    let utime: u64 = fields
        .get(11)
        .ok_or("missing utime in /proc/<pid>/stat")?
        .parse()?;
    let stime: u64 = fields
        .get(12)
        .ok_or("missing stime in /proc/<pid>/stat")?
        .parse()?;
    Ok(utime + stime)
}

fn read_total_cpu_ticks() -> Result<u64> {
    let stat = std::fs::read_to_string("/proc/stat")?;
    parse_total_cpu_ticks(&stat)
}

fn parse_total_cpu_ticks(stat: &str) -> Result<u64> {
    let first = stat.lines().next().ok_or("empty /proc/stat")?;
    let mut fields = first.split_whitespace();
    if fields.next() != Some("cpu") {
        return Err("/proc/stat does not start with aggregate cpu line".into());
    }
    fields
        .map(str::parse::<u64>)
        .try_fold(0u64, |acc, value| Ok(acc + value?))
}

fn read_process_rss_mib(pid: u32) -> Result<f64> {
    let status = std::fs::read_to_string(format!("/proc/{pid}/status"))?;
    parse_process_rss_mib(&status)
}

fn parse_process_rss_mib(status: &str) -> Result<f64> {
    for line in status.lines() {
        if let Some(value) = line.strip_prefix("VmRSS:") {
            let kib: u64 = value
                .split_whitespace()
                .next()
                .ok_or("VmRSS value missing")?
                .parse()?;
            return Ok(kib as f64 / 1024.0);
        }
    }
    Err("VmRSS missing from /proc/<pid>/status".into())
}

fn cpu_pct_for_window(start_proc: u64, end_proc: u64, start_total: u64, end_total: u64) -> f64 {
    let proc_delta = end_proc.saturating_sub(start_proc) as f64;
    let total_delta = end_total.saturating_sub(start_total) as f64;
    if total_delta == 0.0 {
        return 0.0;
    }
    let cpus = std::thread::available_parallelism().map_or(1.0, |n| n.get() as f64);
    (proc_delta * cpus / total_delta) * 100.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn watch_command_uses_parseable_non_tui_output() {
        assert_eq!(
            watch_command_args(),
            ["--json", "--no-tui", "watch", "--all", "--debounce=100"]
        );
    }

    #[test]
    fn env_binary_path_is_resolved_before_child_changes_dir() {
        let path = resolve_configured_anvil_binary(PathBuf::from("target/debug/anvil")).unwrap();
        assert!(path.is_absolute());
        assert!(path.ends_with("target/debug/anvil"));
    }

    #[test]
    fn parses_process_ticks_when_command_contains_spaces() {
        let stat = "123 (anvil watch) S 1 2 3 4 5 6 7 8 9 10 21 34 17 18 19 20";
        assert_eq!(parse_process_cpu_ticks(stat).unwrap(), 55);
    }

    #[test]
    fn parses_total_cpu_ticks() {
        let stat = "cpu  1 2 3 4 5 6 7 8 9 10\ncpu0 1 2 3 4";
        assert_eq!(parse_total_cpu_ticks(stat).unwrap(), 55);
    }

    #[test]
    fn parses_rss_as_mib() {
        let status = "Name:\tanvil\nVmRSS:\t204800 kB\nVmSize:\t300000 kB";
        assert_eq!(parse_process_rss_mib(status).unwrap(), 200.0);
    }

    #[test]
    fn cpu_window_reports_one_full_core_as_available_parallelism_fraction() {
        let cpus = std::thread::available_parallelism().map_or(1.0, |n| n.get() as f64);
        let pct = cpu_pct_for_window(10, 20, 100, 100 + (10 * cpus as u64));
        assert!((pct - 100.0).abs() < f64::EPSILON);
    }
}
