//! Real-default `anvil watch` CPU/RSS budget bench (RLB-002).
//!
//! The original version of this bench spawned `anvil watch`, let it settle, and
//! sampled the **parent pid** over a window in which **no files changed**. That
//! measured the idle path and reported ~0% while a beta tester saw ~7 cores
//! (GH #2156). The gap was structural: bare `anvil watch` defaults to
//! `--action check`, so every debounced save spawns a per-save `anvil check`
//! child — and the bench drove no saves and never looked at the children.
//!
//! This version closes both gaps:
//! - It drives **sustained churn**: a background thread rewrites repo files on
//!   an interval across the whole measurement window, so the debounced per-save
//!   check actually runs.
//! - It measures the **whole process tree** via [`crate::proc_sampler`], so the
//!   per-save check child's CPU (and any transient RSS) is counted.
//!
//! The verdict is evaluated against [`ResourceBudget::ANVIL_WATCH_CHURN_V1`],
//! the churn-path ceiling, not the idle ceiling.

use std::error::Error;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use tempfile::TempDir;

use crate::budget::{BudgetVerdict, ResourceBudget, evaluate};
use crate::fixture::{RepoSpec, generate_repo};
use crate::proc_sampler::TreeSampler;
use crate::spawn::{ManagedChild, resolve_anvil_binary};

type Result<T> = std::result::Result<T, Box<dyn Error + Send + Sync>>;

const DEFAULT_SETTLE_DURATION: Duration = Duration::from_secs(2);
const DEFAULT_MEASURE_DURATION: Duration = Duration::from_secs(5);
const DEFAULT_SAMPLE_INTERVAL: Duration = Duration::from_millis(200);
const DEFAULT_CHURN_INTERVAL: Duration = Duration::from_millis(250);
const DEFAULT_CHURN_BATCH: usize = 1;

#[derive(Debug, Clone)]
pub struct WatchResourceBudgetConfig {
    pub anvil_bin: PathBuf,
    pub repo_spec: RepoSpec,
    pub settle_duration: Duration,
    pub measure_duration: Duration,
    pub sample_interval: Duration,
    /// How often the churn thread rewrites files. Each rewrite is a debounced
    /// save that triggers a per-save check on the changed path.
    pub churn_interval: Duration,
    /// Number of distinct files rewritten per churn tick.
    pub churn_batch: usize,
    /// Ceiling the measurement is evaluated against.
    pub budget: ResourceBudget,
}

impl WatchResourceBudgetConfig {
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            anvil_bin: resolve_anvil_binary()?,
            repo_spec: RepoSpec::default(),
            settle_duration: DEFAULT_SETTLE_DURATION,
            measure_duration: DEFAULT_MEASURE_DURATION,
            sample_interval: DEFAULT_SAMPLE_INTERVAL,
            churn_interval: DEFAULT_CHURN_INTERVAL,
            churn_batch: DEFAULT_CHURN_BATCH,
            budget: ResourceBudget::ANVIL_WATCH_CHURN_V1,
        })
    }
}

pub fn run(config: &WatchResourceBudgetConfig) -> Result<BudgetVerdict> {
    if !cfg!(target_os = "linux") {
        return Err("watch resource budget sampling requires Linux /proc".into());
    }

    let tempdir = TempDir::new()?;
    let repo = generate_repo(&config.repo_spec, tempdir.path())?;
    let churn_files = collect_churnable_files(repo.root());
    if churn_files.is_empty() {
        return Err("synthetic repo produced no churnable source files".into());
    }

    let child = Command::new(&config.anvil_bin)
        .args(watch_command_args())
        .current_dir(repo.root())
        .env("ANVIL_DISABLE_UPDATE_HINT", "1")
        .env("ANVIL_DEV", "1")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    let mut child = ManagedChild::new(child, "anvil watch");
    let pid = child.id();

    // Let the cold scan settle before we baseline the CPU counters, so the
    // measurement reflects steady-state churn cost, not first-scan cost.
    std::thread::sleep(config.settle_duration);

    // A watcher that died during startup (e.g. the inotify watch limit, a bad
    // repo, a missing binary) would otherwise be measured as a frozen zombie
    // and reported as a happy "0% pass". Refuse to emit a verdict for a corpse.
    child.ensure_running("after settle (watcher failed to start?)")?;

    let mut sampler = TreeSampler::start(pid)?;
    let churn = ChurnDriver::start(churn_files, config.churn_interval, config.churn_batch);
    sampler.sample_for(config.measure_duration, config.sample_interval);
    churn.stop();

    // The watcher must still be alive at the end of the window; a mid-window
    // crash means the measurement covers a partly-dead tree.
    child.ensure_running("after measurement window (watcher crashed mid-run?)")?;
    let sample = sampler.finish()?;

    child.shutdown();

    Ok(evaluate(config.budget, sample))
}

/// Background file-churn driver: rewrites a rotating window of files on an
/// interval until [`ChurnDriver::stop`] is called.
struct ChurnDriver {
    stop: Arc<AtomicBool>,
    handle: Option<std::thread::JoinHandle<()>>,
}

impl ChurnDriver {
    fn start(files: Vec<PathBuf>, interval: Duration, batch: usize) -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let stop_flag = Arc::clone(&stop);
        let handle = std::thread::spawn(move || {
            let mut cursor = 0usize;
            let mut tick: u64 = 0;
            while !stop_flag.load(Ordering::Relaxed) {
                for _ in 0..batch.max(1) {
                    let path = &files[cursor % files.len()];
                    let _ = append_churn_line(path, tick);
                    cursor = cursor.wrapping_add(1);
                }
                tick = tick.wrapping_add(1);
                std::thread::sleep(interval);
            }
        });
        Self {
            stop,
            handle: Some(handle),
        }
    }

    fn stop(mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

/// Append a changing comment line so both content and mtime change, which is
/// what `notify` reports and what a real save looks like.
fn append_churn_line(path: &Path, tick: u64) -> std::io::Result<()> {
    use std::io::Write;
    let mut file = std::fs::OpenOptions::new().append(true).open(path)?;
    writeln!(file, "// churn {tick}")
}

/// Collect rewritable source files (the languages the scanner actually parses)
/// from the synthetic repo. JSON files are skipped — appending a comment line
/// would make them invalid and is not representative of a code save.
fn collect_churnable_files(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            match entry.file_type() {
                Ok(ft) if ft.is_dir() => stack.push(path),
                Ok(ft) if ft.is_file() => {
                    if matches!(
                        path.extension().and_then(|e| e.to_str()),
                        Some("ts" | "js" | "rs")
                    ) {
                        out.push(path);
                    }
                }
                _ => {}
            }
        }
    }
    out.sort();
    out
}

pub fn watch_command_args() -> [&'static str; 5] {
    ["--json", "--no-tui", "watch", "--all", "--debounce=100"]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn watch_command_uses_parseable_non_tui_default_action() {
        // No `--action` override: bare watch defaults to `check` (GH #1913),
        // which is exactly the production save-time path RLB-002 measures.
        let args = watch_command_args();
        assert_eq!(
            args,
            ["--json", "--no-tui", "watch", "--all", "--debounce=100"]
        );
        assert!(
            !args.contains(&"--action"),
            "must measure the default action"
        );
    }

    #[test]
    fn from_env_evaluates_against_churn_budget() {
        // The churn path is gated by the churn ceiling, not the idle ceiling.
        let budget = ResourceBudget::ANVIL_WATCH_CHURN_V1;
        assert_ne!(budget, ResourceBudget::ANVIL_WATCH_V1);
    }

    #[test]
    fn collects_only_scannable_source_files() {
        let dir = tempfile::tempdir().unwrap();
        let repo = generate_repo(&RepoSpec::small(), dir.path()).unwrap();
        let files = collect_churnable_files(repo.root());
        assert!(!files.is_empty(), "expected churnable source files");
        assert!(
            files.iter().all(|p| matches!(
                p.extension().and_then(|e| e.to_str()),
                Some("ts" | "js" | "rs")
            )),
            "json/other files must be excluded from churn"
        );
    }

    #[test]
    fn append_churn_line_changes_content() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a.ts");
        std::fs::write(&path, "const x = 1;\n").unwrap();
        let before = std::fs::metadata(&path).unwrap().len();
        append_churn_line(&path, 7).unwrap();
        let after = std::fs::read_to_string(&path).unwrap();
        assert!(after.contains("// churn 7"));
        assert!(after.len() as u64 > before);
    }

    #[test]
    fn churn_driver_rewrites_until_stopped() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a.ts");
        std::fs::write(&path, "// seed\n").unwrap();
        let driver = ChurnDriver::start(vec![path.clone()], Duration::from_millis(5), 1);
        std::thread::sleep(Duration::from_millis(60));
        driver.stop();
        let churns = std::fs::read_to_string(&path)
            .unwrap()
            .lines()
            .filter(|l| l.starts_with("// churn "))
            .count();
        assert!(churns >= 2, "expected sustained churn, saw {churns} writes");
    }
}
