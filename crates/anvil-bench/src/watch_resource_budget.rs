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
//! - It drives **sustained churn** via [`crate::churn`]: a background thread
//!   rewrites repo files across the whole measurement window, so the debounced
//!   per-save check actually runs.
//! - It measures the **whole process tree** via [`crate::proc_sampler`], so the
//!   per-save check child's CPU (and any transient RSS) is counted.
//!
//! The verdict is evaluated against [`ResourceBudget::ANVIL_WATCH_CHURN_V1`],
//! the churn-path ceiling, not the idle ceiling.

use std::error::Error;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::Duration;

use tempfile::TempDir;

use crate::budget::{BudgetVerdict, ResourceBudget, evaluate};
use crate::churn::{ChurnDriver, collect_churnable_files};
use crate::fixture::{RepoSpec, generate_repo};
use crate::proc_sampler::TreeSampler;
use crate::spawn::{ManagedChild, in_new_process_group, resolve_anvil_binary};

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

    let mut command = Command::new(&config.anvil_bin);
    command
        .args(watch_command_args())
        .current_dir(repo.root())
        .env("ANVIL_DISABLE_UPDATE_HINT", "1")
        .env("ANVIL_DEV", "1")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    // Own process group so the per-save `anvil check` children die with the
    // watcher on shutdown rather than leaking (reparented to init).
    let child = in_new_process_group(&mut command).spawn()?;
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
}
