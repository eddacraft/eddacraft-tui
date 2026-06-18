//! DLIFE-004: end-to-end guard for the `anvil watch` save-time daemon
//! lifecycle in non-interactive contexts.
//!
//! These tests spawn the real `anvil` binary with a piped (non-TTY) stdout
//! and an isolated, daemon-free `XDG_RUNTIME_DIR`, so the lifecycle planner
//! resolves to the deterministic fallback rather than the interactive offer.
//! They prove the safety properties automation depends on:
//!
//! 1. A non-interactive `anvil watch check` NEVER prompts or hangs — it
//!    emits the honest `daemon:` fallback line on stderr and proceeds into
//!    the watch loop (we wait with a budget; a hang would time out).
//! 2. `--no-daemon` renders the opt-out line, distinct from the
//!    non-interactive context line.
//! 3. In `--json` mode the lifecycle line is suppressed entirely, so stdout
//!    stays pure NDJSON — asserted only after at least one event line has
//!    actually arrived, so the check can never pass vacuously.
//!
//! The interactive *offer* path (`WatchDaemonPlan::Prompt`) cannot be driven
//! from CI (there is no TTY), so it is covered by the pure-planner and render
//! unit tests in `watch.rs`. These integration tests pin the boundary that
//! automation actually hits.
//!
//! Unix-only: the save-time daemon socket and background-launch path are
//! Unix-first (Windows follows DSV-010/011), matching the sibling
//! `watch_json_output.rs` harness.

#![cfg(unix)]

use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

const ANVIL_BIN: &str = env!("CARGO_BIN_EXE_anvil");

/// Generous wall-clock budget. A non-interactive run reaches the lifecycle
/// line within milliseconds; if the planner ever regressed into prompting,
/// `read_line` on a null stdin would block and this budget would expire,
/// failing the test loudly instead of hanging CI forever.
const WAIT_BUDGET: Duration = Duration::from_secs(8);

/// A spawned `anvil watch` whose stdout and stderr are each pumped into a
/// channel so the test can wait for a specific line with a timeout, then kill
/// the long-running watcher.
struct WatchProcess {
    child: Child,
    stdout_rx: mpsc::Receiver<String>,
    stderr_rx: mpsc::Receiver<String>,
    readers: Vec<thread::JoinHandle<()>>,
}

fn pump(
    reader: impl std::io::Read + Send + 'static,
    tx: mpsc::Sender<String>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let buf = BufReader::new(reader);
        for line in buf.lines() {
            match line {
                Ok(l) => {
                    if tx.send(l).is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    })
}

impl WatchProcess {
    fn spawn(workdir: &Path, home: &Path, json: bool, extra_args: &[&str]) -> Self {
        let mut cmd = Command::new(ANVIL_BIN);
        cmd.arg("--no-tui");
        if json {
            cmd.arg("--json");
        }
        cmd.arg("watch").arg("--debounce").arg("50");
        for arg in extra_args {
            cmd.arg(arg);
        }
        cmd.current_dir(workdir)
            .env("HOME", home)
            .env("USERPROFILE", home)
            // A fresh, isolated runtime dir guarantees no save-time daemon
            // answers, so the probe reports "not live" and the lifecycle
            // resolves to a fallback rather than reuse.
            .env("XDG_RUNTIME_DIR", home)
            .env_remove("XDG_CONFIG_HOME")
            // Hermeticity: the spawned binary inherits the parent env, so a
            // developer's (or CI's) `ANVIL_WATCH_DAEMON` would change the
            // planner's routing posture and the asserted copy. Clear it so the
            // default (unset → DefaultOnWhenLive) posture is what these tests
            // actually exercise.
            .env_remove("ANVIL_WATCH_DAEMON")
            // Clear any inherited prompt opt-out so the test genuinely proves
            // the piped-stdout (non-TTY) detection alone forces the
            // non-interactive fallback — not an inherited env signal. (The
            // piped stdout below is the load-bearing signal: a null stdin plus
            // a non-terminal stdout means `watch_is_interactive()` is false.)
            .env_remove("ANVIL_NO_PROMPT")
            .env("ANVIL_DEV", "1")
            .env("ANVIL_SKIP_WELCOME", "1")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let mut child = cmd.spawn().expect("spawn anvil watch");
        let stdout = child.stdout.take().expect("piped stdout");
        let stderr = child.stderr.take().expect("piped stderr");
        let (out_tx, stdout_rx) = mpsc::channel();
        let (err_tx, stderr_rx) = mpsc::channel();
        let readers = vec![pump(stdout, out_tx), pump(stderr, err_tx)];

        Self {
            child,
            stdout_rx,
            stderr_rx,
            readers,
        }
    }

    /// Wait for a line on `rx` matching `predicate`, returning every line
    /// collected so a failure can show the full prefix. Polls `try_wait` so an
    /// early child exit fails fast rather than burning the budget.
    fn collect_until<F>(
        child: &mut Child,
        rx: &mpsc::Receiver<String>,
        budget: Duration,
        mut predicate: F,
    ) -> Vec<String>
    where
        F: FnMut(&str) -> bool,
    {
        let deadline = Instant::now() + budget;
        let mut collected = Vec::new();
        let poll_step = Duration::from_millis(50);
        while Instant::now() < deadline {
            if let Ok(Some(_status)) = child.try_wait() {
                while let Ok(line) = rx.try_recv() {
                    collected.push(line);
                }
                return collected;
            }
            let remaining = deadline.saturating_duration_since(Instant::now());
            match rx.recv_timeout(remaining.min(poll_step)) {
                Ok(line) => {
                    let matched = predicate(&line);
                    collected.push(line);
                    if matched {
                        return collected;
                    }
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            }
        }
        collected
    }

    fn wait_for_stderr<F>(&mut self, budget: Duration, predicate: F) -> Vec<String>
    where
        F: FnMut(&str) -> bool,
    {
        Self::collect_until(&mut self.child, &self.stderr_rx, budget, predicate)
    }

    fn wait_for_stdout<F>(&mut self, budget: Duration, predicate: F) -> Vec<String>
    where
        F: FnMut(&str) -> bool,
    {
        Self::collect_until(&mut self.child, &self.stdout_rx, budget, predicate)
    }

    fn shutdown(mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        for reader in self.readers.drain(..) {
            let _ = reader.join();
        }
    }
}

fn seed_workspace(dir: &Path) {
    std::fs::write(dir.join("seed.ts"), "export const seed: number = 1;\n")
        .expect("seed workspace");
}

#[test]
fn non_interactive_watch_falls_back_without_hanging() {
    let workdir = tempfile::tempdir().expect("tempdir");
    let home = tempfile::tempdir().expect("home tempdir");
    seed_workspace(workdir.path());

    let mut proc = WatchProcess::spawn(workdir.path(), home.path(), /* json = */ false, &[]);
    let lines = proc.wait_for_stderr(WAIT_BUDGET, |line| line.contains("daemon:"));
    proc.shutdown();

    let daemon_line = lines
        .iter()
        .find(|l| l.contains("daemon:"))
        .unwrap_or_else(|| {
            panic!("no daemon lifecycle line on stderr within {WAIT_BUDGET:?}; lines={lines:?}")
        });
    // Deterministic non-interactive fallback: names the context, names the
    // preserved scoped fallback, and does NOT blame the opt-out flag.
    assert!(
        daemon_line.contains("non-interactive"),
        "expected the non-interactive fallback reason, got: {daemon_line}"
    );
    assert!(
        daemon_line.contains("scoped fallback"),
        "fallback line must name the preserved scoped fallback, got: {daemon_line}"
    );
    assert!(
        !daemon_line.contains("--no-daemon"),
        "a context fallback must not blame the opt-out flag, got: {daemon_line}"
    );
}

#[test]
fn no_daemon_flag_renders_opt_out_line() {
    let workdir = tempfile::tempdir().expect("tempdir");
    let home = tempfile::tempdir().expect("home tempdir");
    seed_workspace(workdir.path());

    let mut proc = WatchProcess::spawn(
        workdir.path(),
        home.path(),
        /* json = */ false,
        &["--no-daemon"],
    );
    let lines = proc.wait_for_stderr(WAIT_BUDGET, |line| line.contains("daemon:"));
    proc.shutdown();

    let daemon_line = lines
        .iter()
        .find(|l| l.contains("daemon:"))
        .unwrap_or_else(|| panic!("no daemon line on stderr; lines={lines:?}"));
    assert!(
        daemon_line.contains("--no-daemon"),
        "the explicit opt-out line must name the flag, got: {daemon_line}"
    );
    assert!(
        daemon_line.contains("scoped fallback"),
        "opt-out line must name the preserved scoped fallback, got: {daemon_line}"
    );
}

#[test]
fn json_mode_suppresses_the_lifecycle_line_on_stdout() {
    let workdir = tempfile::tempdir().expect("tempdir");
    let home = tempfile::tempdir().expect("home tempdir");
    seed_workspace(workdir.path());

    let mut proc = WatchProcess::spawn(workdir.path(), home.path(), /* json = */ true, &[]);
    // Wait until the initial scan ships at least one NDJSON event. Without
    // this gate the purity assertions could pass vacuously on an empty stdout.
    let lines = proc.wait_for_stdout(WAIT_BUDGET, |line| line.contains("\"event_type\""));
    proc.shutdown();

    assert!(
        lines.iter().any(|l| l.contains("\"event_type\"")),
        "no NDJSON event arrived on stdout within {WAIT_BUDGET:?}; lines={lines:?}"
    );
    for line in &lines {
        assert!(
            !line.contains("daemon:"),
            "stdout must stay pure NDJSON in --json mode; leaked lifecycle line: {line:?}"
        );
        assert!(
            line.starts_with('{'),
            "stdout line must be a JSON object in --json mode: {line:?}"
        );
    }
}
