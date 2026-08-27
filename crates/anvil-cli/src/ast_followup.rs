//! GTAO-003 / GTAO-005: CLI-side cheap-catalogue follow-up after a daemon allow.
//!
//! Interactive `scan_buffer` / `validate_paths` verdicts stay regex-only and
//! must not wait. After an allow, this module schedules a coalesced
//! changed-path `anvil check` subprocess (regex + AST). Failure is fail-safe:
//! the original verdict stands, a single skipped diagnostic is recorded, and
//! the parent still exits 0.
//!
//! Kill switch: `ANVIL_AST_FOLLOWUP=0` or `.anvil.yaml` `astFollowup: false`.
//! Env `0` wins over config; env `1` re-enables despite config.

use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
#[cfg(test)]
use std::thread::JoinHandle;

use anvil_intercept_proto::enforcement_config::AnvilConfigFile;

/// Environment kill switch named in ADR-127.
pub(crate) const FOLLOWUP_ENV: &str = "ANVIL_AST_FOLLOWUP";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FollowupInvocation {
    pub program: PathBuf,
    pub args: Vec<String>,
    pub cwd: PathBuf,
}

impl FollowupInvocation {
    /// GTAO-005: a follow-up that shells `check --all` or `gate` is the
    /// ADR-061 save-storm. Scoped `check` of explicit paths is the only
    /// legal shape.
    #[must_use]
    pub(crate) fn is_legal(&self) -> bool {
        is_legal_followup_argv(&self.args)
    }
}

#[must_use]
pub(crate) fn is_legal_followup_argv(args: &[String]) -> bool {
    args.first().map(String::as_str) == Some("check")
        && !args.iter().any(|arg| arg == "--all" || arg == "gate")
        && args.iter().any(|arg| arg == "--")
}

/// Resolve enablement from env then config. Default is on.
#[must_use]
pub(crate) fn followup_enabled(env: Option<&OsStr>, config_enabled: Option<bool>) -> bool {
    match env.and_then(OsStr::to_str).map(str::to_ascii_lowercase) {
        Some(value) if matches!(value.as_str(), "0" | "false" | "off" | "no") => false,
        Some(value) if matches!(value.as_str(), "1" | "true" | "on" | "yes") => true,
        _ => config_enabled.unwrap_or(true),
    }
}

#[must_use]
pub(crate) fn load_config_followup(workspace: &Path) -> Option<bool> {
    let content = std::fs::read_to_string(workspace.join(".anvil.yaml")).ok()?;
    let config: AnvilConfigFile = serde_yaml::from_str(&content).ok()?;
    config.ast_followup
}

#[must_use]
pub(crate) fn build_followup_invocation(
    program: PathBuf,
    workspace: PathBuf,
    paths: &[PathBuf],
) -> Option<FollowupInvocation> {
    let mut files: Vec<PathBuf> = paths
        .iter()
        .filter(|path| path.is_file())
        .cloned()
        .collect();
    files.sort();
    files.dedup();
    if files.is_empty() {
        return None;
    }
    let mut args = vec![
        "check".to_string(),
        "--json".to_string(),
        "--no-tui".to_string(),
        "--".to_string(),
    ];
    args.extend(
        files
            .into_iter()
            .map(|path| path.to_string_lossy().into_owned()),
    );
    Some(FollowupInvocation {
        program,
        args,
        cwd: workspace,
    })
}

pub(crate) trait FollowupRunner: Send + Sync {
    fn run(&self, command: FollowupInvocation) -> Result<(), String>;
}

#[cfg_attr(test, allow(dead_code))]
struct ProcessRunner;

impl FollowupRunner for ProcessRunner {
    fn run(&self, command: FollowupInvocation) -> Result<(), String> {
        if !command.is_legal() {
            return Err("refusing illegal ast follow-up argv".into());
        }
        let mut child = Command::new(&command.program)
            .args(&command.args)
            .current_dir(&command.cwd)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|err| err.to_string())?;
        let status = child.wait().map_err(|err| err.to_string())?;
        if !status.success() {
            tracing::debug!(
                code = ?status.code(),
                "ast follow-up child exited non-zero (findings or skip)"
            );
        }
        Ok(())
    }
}

struct FollowupInner {
    runner: Arc<dyn FollowupRunner>,
    pending: Mutex<BTreeMap<PathBuf, BTreeSet<PathBuf>>>,
    running: AtomicBool,
    #[cfg(test)]
    worker: Mutex<Option<JoinHandle<()>>>,
}

pub(crate) struct FollowupScheduler {
    inner: Arc<FollowupInner>,
}

impl FollowupScheduler {
    pub(crate) fn new(runner: Arc<dyn FollowupRunner>) -> Self {
        Self {
            inner: Arc::new(FollowupInner {
                runner,
                pending: Mutex::new(BTreeMap::new()),
                running: AtomicBool::new(false),
                #[cfg(test)]
                worker: Mutex::new(None),
            }),
        }
    }

    #[cfg_attr(test, allow(dead_code))]
    pub(crate) fn process() -> Self {
        Self::new(Arc::new(ProcessRunner))
    }

    pub(crate) fn schedule(&self, workspace: PathBuf, paths: Vec<PathBuf>) {
        if paths.is_empty() {
            return;
        }
        {
            let mut pending = recover(self.inner.pending.lock());
            pending.entry(workspace).or_default().extend(paths);
        }
        self.ensure_worker();
    }

    fn ensure_worker(&self) {
        if self.inner.running.swap(true, Ordering::SeqCst) {
            return;
        }
        let inner = Arc::clone(&self.inner);
        let handle = std::thread::spawn(move || worker_loop(&inner));
        #[cfg(test)]
        {
            *recover(self.inner.worker.lock()) = Some(handle);
        }
        #[cfg(not(test))]
        {
            let _ = handle;
        }
    }

    #[cfg(test)]
    pub(crate) fn wait_idle(&self) {
        let started = std::time::Instant::now();
        loop {
            let running = self.inner.running.load(Ordering::SeqCst);
            let pending_empty = recover(self.inner.pending.lock()).is_empty();
            if !running && pending_empty {
                if let Some(handle) = recover(self.inner.worker.lock()).take() {
                    let _ = handle.join();
                }
                return;
            }
            assert!(
                started.elapsed() <= std::time::Duration::from_secs(2),
                "follow-up worker did not become idle"
            );
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
    }
}

fn worker_loop(inner: &Arc<FollowupInner>) {
    loop {
        let batch = {
            let mut pending = recover(inner.pending.lock());
            std::mem::take(&mut *pending)
        };
        if batch.is_empty() {
            inner.running.store(false, Ordering::SeqCst);
            let raced = {
                let pending = recover(inner.pending.lock());
                !pending.is_empty()
            };
            if !raced {
                return;
            }
            if inner.running.swap(true, Ordering::SeqCst) {
                return;
            }
            continue;
        }
        for (workspace, paths) in batch {
            let paths: Vec<PathBuf> = paths.into_iter().collect();
            if !followup_enabled(
                std::env::var_os(FOLLOWUP_ENV).as_deref(),
                load_config_followup(&workspace),
            ) {
                continue;
            }
            let Some(program) = resolve_exe() else {
                record_skip("cannot resolve current executable");
                continue;
            };
            let Some(command) = build_followup_invocation(program, workspace, &paths) else {
                record_skip("no on-disk files to scan");
                continue;
            };
            if let Err(err) = inner.runner.run(command) {
                record_skip(&err);
            }
        }
    }
}

fn resolve_exe() -> Option<PathBuf> {
    match std::env::current_exe() {
        Ok(path) => Some(path),
        Err(err) => {
            record_skip(&format!("current_exe failed: {err}"));
            None
        }
    }
}

fn recover<T>(result: std::sync::LockResult<T>) -> T {
    result.unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn record_skip(reason: &str) {
    tracing::warn!(
        target: "anvil.ast_followup",
        skip = true,
        reason,
        "ast follow-up skipped"
    );
}

#[cfg_attr(test, allow(dead_code))]
fn process_scheduler() -> &'static FollowupScheduler {
    static SCHEDULER: OnceLock<FollowupScheduler> = OnceLock::new();
    SCHEDULER.get_or_init(FollowupScheduler::process)
}

/// Fire-and-forget: enqueue changed paths and return without waiting.
///
/// Pre-write MCP (`scan_buffer`) is not a caller: proposed bytes are not on
/// disk yet, so a path-scoped `anvil check` would miss creates and scan stale
/// updates. Golden-path follow-up is post-write `validate_paths` (watch).
pub(crate) fn schedule_paths(workspace: &Path, paths: &[impl AsRef<Path>]) {
    let collected: Vec<PathBuf> = paths
        .iter()
        .map(|path| path.as_ref().to_path_buf())
        .collect();
    #[cfg(test)]
    {
        // Unit tests must not spawn the test binary as `anvil check`.
        if capture_enabled() {
            record_capture(workspace.to_path_buf(), collected);
        }
    }
    #[cfg(not(test))]
    {
        if !followup_enabled(std::env::var_os(FOLLOWUP_ENV).as_deref(), None) {
            return;
        }
        process_scheduler().schedule(workspace.to_path_buf(), collected);
    }
}

#[cfg(test)]
std::thread_local! {
    static CAPTURE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    static CAPTURED: std::cell::RefCell<Vec<(PathBuf, Vec<PathBuf>)>> =
        const { std::cell::RefCell::new(Vec::new()) };
}

#[cfg(test)]
fn capture_enabled() -> bool {
    CAPTURE.with(std::cell::Cell::get)
}

#[cfg(test)]
fn record_capture(workspace: PathBuf, paths: Vec<PathBuf>) {
    CAPTURED.with(|slot| slot.borrow_mut().push((workspace, paths)));
}

#[cfg(test)]
pub(crate) fn capture_schedules() {
    CAPTURE.with(|flag| flag.set(true));
    CAPTURED.with(|slot| slot.borrow_mut().clear());
}

#[cfg(test)]
pub(crate) fn take_scheduled() -> Vec<(PathBuf, Vec<PathBuf>)> {
    CAPTURE.with(|flag| flag.set(false));
    CAPTURED.with(std::cell::RefCell::take)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;
    use std::time::{Duration, Instant};

    struct RecordingRunner {
        calls: Mutex<Vec<FollowupInvocation>>,
        hold: Mutex<Option<mpsc::Receiver<()>>>,
        started: std::sync::atomic::AtomicUsize,
    }

    impl RecordingRunner {
        fn new(hold: Option<mpsc::Receiver<()>>) -> Arc<Self> {
            Arc::new(Self {
                calls: Mutex::new(Vec::new()),
                hold: Mutex::new(hold),
                started: std::sync::atomic::AtomicUsize::new(0),
            })
        }

        fn calls(&self) -> Vec<FollowupInvocation> {
            recover(self.calls.lock()).clone()
        }
    }

    impl FollowupRunner for RecordingRunner {
        fn run(&self, command: FollowupInvocation) -> Result<(), String> {
            self.started.fetch_add(1, Ordering::SeqCst);
            if let Some(rx) = recover(self.hold.lock()).as_ref() {
                let _ = rx.recv_timeout(Duration::from_secs(2));
            }
            recover(self.calls.lock()).push(command);
            Ok(())
        }
    }

    struct FailingRunner;

    impl FollowupRunner for FailingRunner {
        fn run(&self, _command: FollowupInvocation) -> Result<(), String> {
            Err("spawn failed".into())
        }
    }

    fn temp_file(name: &str) -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join(name);
        std::fs::write(&path, "fn main() { let _ = maybe().unwrap(); }\n").expect("write fixture");
        (dir, path)
    }

    #[test]
    fn kill_switch_zero_disables_followup() {
        assert!(!followup_enabled(Some(OsStr::new("0")), Some(true)));
        assert!(!followup_enabled(Some(OsStr::new("false")), None));
        assert!(!followup_enabled(None, Some(false)));
        assert!(followup_enabled(None, None));
        assert!(followup_enabled(Some(OsStr::new("1")), Some(false)));
    }

    #[test]
    fn followup_argv_is_scoped_check_never_all_or_gate() {
        let (_dir, path) = temp_file("lib.rs");
        let invocation = build_followup_invocation(
            PathBuf::from("/usr/bin/anvil"),
            path.parent().expect("parent").to_path_buf(),
            std::slice::from_ref(&path),
        )
        .expect("file exists");
        assert!(invocation.is_legal(), "got {:?}", invocation.args);
        assert_eq!(invocation.args[0], "check");
        assert!(invocation.args.contains(&"--json".to_string()));
        assert!(invocation.args.contains(&"--no-tui".to_string()));
        assert!(
            invocation
                .args
                .iter()
                .any(|arg| arg == path.to_str().unwrap()),
            "must pass the changed file, got {:?}",
            invocation.args
        );
        assert!(!is_legal_followup_argv(&["check".into(), "--all".into()]));
        assert!(!is_legal_followup_argv(&[
            "gate".into(),
            "--profile".into(),
            "ci".into()
        ]));
    }

    #[test]
    fn empty_or_missing_paths_do_not_build_a_command() {
        let dir = tempfile::tempdir().expect("temp dir");
        let missing = dir.path().join("gone.rs");
        assert!(
            build_followup_invocation(
                PathBuf::from("/usr/bin/anvil"),
                dir.path().to_path_buf(),
                &[missing]
            )
            .is_none()
        );
    }

    #[test]
    fn one_schedule_of_many_paths_is_one_scoped_command() {
        let runner = RecordingRunner::new(None);
        let scheduler = FollowupScheduler::new(Arc::clone(&runner) as Arc<dyn FollowupRunner>);
        let (dir_a, a) = temp_file("a.rs");
        let b = dir_a.path().join("b.rs");
        std::fs::write(&b, "pub fn b() {}\n").expect("write b");
        scheduler.schedule(dir_a.path().to_path_buf(), vec![a, b]);
        scheduler.wait_idle();
        let calls = runner.calls();
        assert_eq!(calls.len(), 1, "uncoalesced burst: {calls:?}");
        let joined = calls[0].args.join(" ");
        assert!(
            joined.contains("a.rs") && joined.contains("b.rs"),
            "{joined}"
        );
        assert!(calls[0].is_legal());
    }

    #[test]
    fn in_flight_burst_coalesces_remainder() {
        let (tx, rx) = mpsc::channel();
        let runner = RecordingRunner::new(Some(rx));
        let scheduler = FollowupScheduler::new(Arc::clone(&runner) as Arc<dyn FollowupRunner>);
        let (dir, a) = temp_file("a.rs");
        let b = dir.path().join("b.rs");
        std::fs::write(&b, "pub fn b() {}\n").expect("write b");
        let workspace = dir.path().to_path_buf();
        scheduler.schedule(workspace.clone(), vec![a.clone()]);
        let started = Instant::now();
        while runner.started.load(Ordering::SeqCst) == 0 {
            assert!(
                started.elapsed() <= Duration::from_secs(2),
                "first follow-up never started"
            );
            std::thread::sleep(Duration::from_millis(5));
        }
        scheduler.schedule(workspace, vec![b.clone()]);
        tx.send(()).expect("release first run");
        scheduler.wait_idle();
        let calls = runner.calls();
        assert!(
            calls.len() <= 2,
            "save burst must not spawn per keystroke, got {}",
            calls.len()
        );
        assert!(calls.iter().all(FollowupInvocation::is_legal));
        let joined = calls
            .iter()
            .flat_map(|call| call.args.clone())
            .collect::<Vec<_>>()
            .join(" ");
        assert!(
            joined.contains("a.rs") && joined.contains("b.rs"),
            "{joined}"
        );
    }

    #[test]
    fn schedule_does_not_wait_for_the_runner() {
        let (tx, rx) = mpsc::channel();
        let runner = RecordingRunner::new(Some(rx));
        let scheduler = FollowupScheduler::new(Arc::clone(&runner) as Arc<dyn FollowupRunner>);
        let (dir, path) = temp_file("lib.rs");
        let start = Instant::now();
        scheduler.schedule(dir.path().to_path_buf(), vec![path]);
        let elapsed = start.elapsed();
        assert!(
            elapsed < Duration::from_millis(50),
            "verdict path waited {elapsed:?} for follow-up"
        );
        tx.send(()).expect("release");
        scheduler.wait_idle();
    }

    #[test]
    fn runner_failure_does_not_panic_or_change_caller() {
        let scheduler = FollowupScheduler::new(Arc::new(FailingRunner));
        let (dir, path) = temp_file("lib.rs");
        scheduler.schedule(dir.path().to_path_buf(), vec![path]);
        scheduler.wait_idle();
    }

    #[test]
    fn config_false_disables_without_env() {
        let dir = tempfile::tempdir().expect("temp dir");
        std::fs::write(dir.path().join(".anvil.yaml"), "astFollowup: false\n")
            .expect("write config");
        assert_eq!(load_config_followup(dir.path()), Some(false));
        assert!(!followup_enabled(None, load_config_followup(dir.path())));
    }
}
