//! CIB-254: end-to-end honesty matrix for watch save-time routing.
//!
//! Unix-only: this harness uses the real private Unix socket under a temporary
//! `ANVIL_HOME`. Windows named-pipe routing remains covered by the portable
//! planner, transport, payload, and rendering tests; this file is never ignored.

#![cfg(unix)]

use std::fs;
use std::io::{BufRead, BufReader, Read};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use anvil_kernel_types::watch_event::WatchEventPayload;
use anvil_kernel_types::{WatchEventEnvelope, WatchEventType};

const ANVIL_BIN: &str = env!("CARGO_BIN_EXE_anvil");
const WAIT_BUDGET: Duration = Duration::from_secs(12);
const SAVE_COUNT: usize = 3;

fn configure_private_env(command: &mut Command, home: &Path) {
    command
        .env("ANVIL_HOME", home)
        .env("HOME", home)
        .env("USERPROFILE", home)
        .env("XDG_RUNTIME_DIR", home)
        .env_remove("XDG_CONFIG_HOME")
        .env_remove("ANVIL_WATCH_DAEMON")
        .env_remove("ANVIL_NO_PROMPT")
        .env_remove("ANVIL_TOUCH_PROJECT_STATE")
        .env("ANVIL_DEV", "1")
        .env("ANVIL_SKIP_WELCOME", "1")
        .env("ANVIL_DISABLE_UPDATE_HINT", "1");
}

struct DaemonGuard {
    child: Option<Child>,
}

impl DaemonGuard {
    fn spawn(home: &Path) -> Self {
        fs::set_permissions(home, fs::Permissions::from_mode(0o700))
            .expect("secure private ANVIL_HOME");
        let mut command = Command::new(ANVIL_BIN);
        command.args(["intercept", "start", "--foreground"]);
        configure_private_env(&mut command, home);
        let child = command
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn private intercept daemon");
        Self { child: Some(child) }
    }

    fn wait_ready(&mut self, home: &Path) {
        let socket = home.join("intercept.sock");
        let deadline = Instant::now() + WAIT_BUDGET;
        while Instant::now() < deadline {
            if socket.exists() {
                return;
            }
            if self
                .child
                .as_mut()
                .expect("daemon child")
                .try_wait()
                .expect("poll daemon")
                .is_some()
            {
                break;
            }
            thread::sleep(Duration::from_millis(50));
        }

        if let Some(child) = self.child.as_mut() {
            let _ = child.kill();
            let _ = child.wait();
        }
        let mut stderr = String::new();
        if let Some(child) = self.child.as_mut()
            && let Some(mut pipe) = child.stderr.take()
        {
            let _ = pipe.read_to_string(&mut stderr);
        }
        panic!(
            "private daemon did not bind {} within {WAIT_BUDGET:?}; stderr={stderr}",
            socket.display()
        );
    }
}

impl Drop for DaemonGuard {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

fn pump_lines(
    reader: impl Read + Send + 'static,
    sender: mpsc::Sender<String>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        for line in BufReader::new(reader).lines() {
            let Ok(line) = line else {
                break;
            };
            if sender.send(line).is_err() {
                break;
            }
        }
    })
}

struct WatchProcess {
    child: Option<Child>,
    stdout_rx: mpsc::Receiver<String>,
    stderr_rx: mpsc::Receiver<String>,
    readers: Vec<thread::JoinHandle<()>>,
    stdout: Vec<String>,
    stderr: Vec<String>,
}

impl WatchProcess {
    fn spawn(workspace: &Path, home: &Path, json: bool, no_daemon: bool) -> Self {
        let mut command = Command::new(ANVIL_BIN);
        command.arg("--no-tui");
        if json {
            command.arg("--json");
        }
        command.args(["watch", "--debounce", "50"]);
        if no_daemon {
            command.arg("--no-daemon");
        }
        configure_private_env(&mut command, home);
        command
            .current_dir(workspace)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let mut child = command.spawn().expect("spawn anvil watch");
        let stdout = child.stdout.take().expect("piped watch stdout");
        let stderr = child.stderr.take().expect("piped watch stderr");
        let (stdout_tx, stdout_rx) = mpsc::channel();
        let (stderr_tx, stderr_rx) = mpsc::channel();
        let readers = vec![pump_lines(stdout, stdout_tx), pump_lines(stderr, stderr_tx)];
        Self {
            child: Some(child),
            stdout_rx,
            stderr_rx,
            readers,
            stdout: Vec::new(),
            stderr: Vec::new(),
        }
    }

    fn assert_alive(&mut self, context: &str) {
        let status = self
            .child
            .as_mut()
            .expect("watch child")
            .try_wait()
            .expect("poll watch");
        assert!(
            status.is_none(),
            "watch exited while waiting for {context}: status={status:?}; stdout={:?}; stderr={:?}",
            self.stdout,
            self.stderr
        );
    }

    fn drain_stderr(&mut self) {
        while let Ok(line) = self.stderr_rx.try_recv() {
            self.stderr.push(line);
        }
    }

    fn drain_stdout(&mut self) {
        while let Ok(line) = self.stdout_rx.try_recv() {
            self.stdout.push(line);
        }
    }

    fn wait_stdout_after(&mut self, start: usize, context: &str, predicate: impl Fn(&str) -> bool) {
        let deadline = Instant::now() + WAIT_BUDGET;
        loop {
            if self.stdout[start..].iter().any(|line| predicate(line)) {
                return;
            }
            self.drain_stderr();
            self.assert_alive(context);
            let remaining = deadline.saturating_duration_since(Instant::now());
            assert!(
                !remaining.is_zero(),
                "timed out waiting for {context}; stdout={:?}; stderr={:?}",
                self.stdout,
                self.stderr
            );
            match self
                .stdout_rx
                .recv_timeout(remaining.min(Duration::from_millis(50)))
            {
                Ok(line) => self.stdout.push(line),
                Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    panic!(
                        "stdout disconnected waiting for {context}; stdout={:?}; stderr={:?}",
                        self.stdout, self.stderr
                    );
                }
            }
        }
    }

    fn wait_stderr_after(&mut self, start: usize, context: &str, predicate: impl Fn(&str) -> bool) {
        let deadline = Instant::now() + WAIT_BUDGET;
        loop {
            if self.stderr[start..].iter().any(|line| predicate(line)) {
                return;
            }
            self.drain_stdout();
            self.assert_alive(context);
            let remaining = deadline.saturating_duration_since(Instant::now());
            assert!(
                !remaining.is_zero(),
                "timed out waiting for {context}; stdout={:?}; stderr={:?}",
                self.stdout,
                self.stderr
            );
            match self
                .stderr_rx
                .recv_timeout(remaining.min(Duration::from_millis(50)))
            {
                Ok(line) => self.stderr.push(line),
                Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    panic!(
                        "stderr disconnected waiting for {context}; stdout={:?}; stderr={:?}",
                        self.stdout, self.stderr
                    );
                }
            }
        }
    }

    fn wait_ready(&mut self, json: bool) {
        let start = self.stdout.len();
        self.wait_stdout_after(start, "initial watch snapshot", |line| {
            if json {
                serde_json::from_str::<WatchEventEnvelope>(line)
                    .is_ok_and(|event| event.event_type == WatchEventType::Snapshot)
            } else {
                line.contains("initial scan complete")
            }
        });
    }

    fn stop(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        for reader in self.readers.drain(..) {
            let _ = reader.join();
        }
        self.drain_stdout();
        self.drain_stderr();
    }
}

impl Drop for WatchProcess {
    fn drop(&mut self) {
        self.stop();
    }
}

fn new_workspace() -> (tempfile::TempDir, PathBuf) {
    let workspace = tempfile::tempdir().expect("workspace tempdir");
    let source = workspace.path().join("save.ts");
    fs::write(&source, "export const initial = true;\n").expect("seed watched source");
    (workspace, source)
}

fn save_secret(source: &Path, iteration: usize) {
    let key = ["AKIA", "QRSTUVWXYZ", "123456"].concat();
    let harmless = "x".repeat(iteration + 1);
    fs::write(
        source,
        format!("export const accessKey = \"{key}\";\nexport const marker = \"{harmless}\";\n"),
    )
    .expect("write secret fixture");
}

fn exercise_fallback_route(home: &Path, no_daemon: bool) {
    let (workspace, source) = new_workspace();
    let mut watch = WatchProcess::spawn(workspace.path(), home, false, no_daemon);
    watch.wait_ready(false);

    for iteration in 0..SAVE_COUNT {
        let stdout_start = watch.stdout.len();
        let stderr_start = watch.stderr.len();
        save_secret(&source, iteration);
        watch.wait_stdout_after(stdout_start, "secret finding", |line| {
            line.contains("AWS Key")
        });
        watch.wait_stderr_after(stderr_start, "blocking check result", |line| {
            line.contains("Action 'check' exited with code 1")
        });
    }
    watch.stop();

    let secret_findings = watch
        .stdout
        .iter()
        .filter(|line| line.contains("AWS Key"))
        .count();
    assert!(
        secret_findings >= SAVE_COUNT,
        "every scoped fallback save must detect the fixture; stdout={:?}; stderr={:?}",
        watch.stdout,
        watch.stderr
    );
}

#[test]
fn watch_save_time_routes_remain_family_scoped_and_fallback_safe() {
    let live_home = tempfile::tempdir().expect("live ANVIL_HOME");
    let mut daemon = DaemonGuard::spawn(live_home.path());
    daemon.wait_ready(live_home.path());

    let (plain_workspace, plain_source) = new_workspace();
    let mut plain = WatchProcess::spawn(plain_workspace.path(), live_home.path(), false, false);
    plain.wait_ready(false);
    for iteration in 0..SAVE_COUNT {
        let start = plain.stdout.len();
        save_secret(&plain_source, iteration);
        plain.wait_stdout_after(start, "scoped daemon verdict", |line| {
            line.contains("antipattern-only")
        });
    }
    plain.stop();
    assert!(
        plain
            .stdout
            .iter()
            .filter(|line| line.contains("antipattern-only"))
            .count()
            >= SAVE_COUNT,
        "every daemon save must report the family scope; stdout={:?}; stderr={:?}",
        plain.stdout,
        plain.stderr
    );
    assert!(
        plain
            .stdout
            .iter()
            .all(|line| !line.starts_with("anvil watch: clean (0 finding(s))")),
        "daemon output must never make the old unqualified global-clean claim: {:?}",
        plain.stdout
    );

    exercise_fallback_route(live_home.path(), true);

    let absent_home = tempfile::tempdir().expect("absent-daemon ANVIL_HOME");
    exercise_fallback_route(absent_home.path(), false);

    let (json_workspace, json_source) = new_workspace();
    let mut json = WatchProcess::spawn(json_workspace.path(), live_home.path(), true, false);
    json.wait_ready(true);
    let mut daemon_results = Vec::new();
    for iteration in 0..SAVE_COUNT {
        let start = json.stdout.len();
        save_secret(&json_source, iteration);
        json.wait_stdout_after(start, "JSON daemon action_result", |line| {
            serde_json::from_str::<WatchEventEnvelope>(line)
                .is_ok_and(|event| event.event_type == WatchEventType::ActionResult)
        });
        let result = json.stdout[start..]
            .iter()
            .filter_map(|line| serde_json::from_str::<WatchEventEnvelope>(line).ok())
            .find_map(|event| match event.payload {
                WatchEventPayload::ActionResult(result) => Some(result),
                _ => None,
            })
            .expect("matching action_result payload");
        daemon_results.push(result);
    }
    json.stop();

    let envelopes = json
        .stdout
        .iter()
        .map(|line| {
            serde_json::from_str::<WatchEventEnvelope>(line)
                .unwrap_or_else(|err| panic!("stdout must be pure NDJSON: {err}; line={line:?}"))
        })
        .collect::<Vec<_>>();
    let sequences = envelopes.iter().map(|event| event.seq).collect::<Vec<_>>();
    assert!(
        sequences.windows(2).all(|pair| pair[0] < pair[1]),
        "outer sequence numbers must be unique and follow stdout order: {sequences:?}"
    );
    for result in daemon_results {
        let verdict = result.daemon_verdict.expect("structured daemon verdict");
        assert_eq!(verdict.check_families, ["antipattern"]);
        assert_eq!(verdict.finding_count, verdict.diagnostics.len() as u64);
        assert!(
            verdict.assurance_state != "global_clean" && verdict.coverage != "global",
            "daemon assurance must not claim a global scope: {verdict:?}"
        );
    }
}
