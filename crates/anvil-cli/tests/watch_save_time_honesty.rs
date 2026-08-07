//! CIB-254: end-to-end honesty matrix for watch save-time routing.
//!
//! Unix-only: this harness uses the real private Unix socket under a temporary
//! `ANVIL_HOME`. Windows named-pipe routing remains covered by the portable
//! planner, transport, payload, and rendering tests; this file is never ignored.

#![cfg(unix)]

use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, mpsc};
use std::thread;
use std::time::{Duration, Instant};

use anvil_kernel_types::watch_event::WatchActionResult;
use serde_json::{Value, json};

const ANVIL_BIN: &str = env!("CARGO_BIN_EXE_anvil");
const WAIT_BUDGET: Duration = Duration::from_secs(12);
const SAVE_COUNT: usize = 3;
const SPY_SHUTDOWN_SENTINEL: &str = "__cib283_spy_shutdown__\n";

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

/// Best-effort `SO_RCVTIMEO` / `SO_SNDTIMEO` on a Unix-domain stream.
///
/// On macOS aarch64, `setsockopt` can return `EINVAL` (`InvalidInput`) for these
/// options on some UDS edge states (notably a peer that has already written and
/// closed). That is a harness platform edge, not a product contract failure.
/// Continue without a socket timeout so the spy can still forward frames or
/// observe the shutdown sentinel; outer `WAIT_BUDGET` deadlines still bound the
/// test.
fn try_set_socket_timeouts(stream: &UnixStream, label: &str) {
    for (side, result) in [
        ("read", stream.set_read_timeout(Some(WAIT_BUDGET))),
        ("write", stream.set_write_timeout(Some(WAIT_BUDGET))),
    ] {
        if let Err(err) = result {
            if err.kind() == std::io::ErrorKind::InvalidInput {
                continue;
            }
            panic!("bound daemon RPC spy {label} {side}: {err}");
        }
    }
}

fn forward_rpc_connection(
    client: &UnixStream,
    mut client_reader: BufReader<UnixStream>,
    mut request: String,
    upstream_socket: &Path,
    request_frames: &AtomicUsize,
) -> std::io::Result<()> {
    try_set_socket_timeouts(client, "client");
    let mut client_writer = client.try_clone()?;

    let upstream = UnixStream::connect(upstream_socket)?;
    try_set_socket_timeouts(&upstream, "upstream");
    let mut upstream_writer = upstream.try_clone()?;
    let mut upstream_reader = BufReader::new(upstream);

    loop {
        request_frames.fetch_add(1, Ordering::SeqCst);
        upstream_writer.write_all(request.as_bytes())?;
        upstream_writer.flush()?;

        let mut response = String::new();
        if upstream_reader.read_line(&mut response)? == 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "daemon closed before replying to the spy",
            ));
        }
        client_writer.write_all(response.as_bytes())?;
        client_writer.flush()?;

        request.clear();
        if client_reader.read_line(&mut request)? == 0 {
            return Ok(());
        }
    }
}

struct DaemonRpcSpy {
    socket: PathBuf,
    accepted_connections: Arc<AtomicUsize>,
    request_frames: Arc<AtomicUsize>,
    shutdown: Arc<AtomicBool>,
    worker: Option<thread::JoinHandle<()>>,
    stopped_rx: mpsc::Receiver<()>,
}

impl DaemonRpcSpy {
    fn spawn(home: &Path, upstream_socket: &Path) -> Self {
        fs::set_permissions(home, fs::Permissions::from_mode(0o700))
            .expect("secure spy ANVIL_HOME");
        let socket = home.join("intercept.sock");
        let listener = UnixListener::bind(&socket).expect("bind daemon RPC spy");
        fs::set_permissions(&socket, fs::Permissions::from_mode(0o600))
            .expect("secure daemon RPC spy socket");

        let accepted_connections = Arc::new(AtomicUsize::new(0));
        let request_frames = Arc::new(AtomicUsize::new(0));
        let shutdown = Arc::new(AtomicBool::new(false));
        let (stopped_tx, stopped_rx) = mpsc::channel();
        let accepted_worker = Arc::clone(&accepted_connections);
        let frames_worker = Arc::clone(&request_frames);
        let shutdown_worker = Arc::clone(&shutdown);
        let upstream_socket = upstream_socket.to_path_buf();
        let worker = thread::spawn(move || {
            loop {
                let (client, _) = listener.accept().expect("accept daemon RPC spy client");
                try_set_socket_timeouts(&client, "accepted client");
                let mut client_reader =
                    BufReader::new(client.try_clone().expect("clone daemon RPC spy client"));
                let mut first_request = String::new();
                client_reader
                    .read_line(&mut first_request)
                    .expect("read first daemon RPC spy frame");
                if shutdown_worker.load(Ordering::SeqCst) && first_request == SPY_SHUTDOWN_SENTINEL
                {
                    break;
                }
                accepted_worker.fetch_add(1, Ordering::SeqCst);
                if !first_request.is_empty() {
                    forward_rpc_connection(
                        &client,
                        client_reader,
                        first_request,
                        &upstream_socket,
                        &frames_worker,
                    )
                    .expect("forward daemon RPC frame");
                }
            }
            let _ = stopped_tx.send(());
        });

        Self {
            socket,
            accepted_connections,
            request_frames,
            shutdown,
            worker: Some(worker),
            stopped_rx,
        }
    }

    fn accepted_connections(&self) -> usize {
        self.accepted_connections.load(Ordering::SeqCst)
    }

    fn request_frames(&self) -> usize {
        self.request_frames.load(Ordering::SeqCst)
    }

    fn reset_counts(&self) {
        self.accepted_connections.store(0, Ordering::SeqCst);
        self.request_frames.store(0, Ordering::SeqCst);
    }

    fn request_shutdown(&self) {
        self.shutdown.store(true, Ordering::SeqCst);
        if let Ok(mut stream) = UnixStream::connect(&self.socket) {
            let _ = stream.write_all(SPY_SHUTDOWN_SENTINEL.as_bytes());
            let _ = stream.flush();
        }
    }

    fn stop(&mut self) {
        let Some(worker) = self.worker.take() else {
            return;
        };
        self.request_shutdown();
        match self.stopped_rx.recv_timeout(WAIT_BUDGET) {
            // Ok: clean shutdown signal. Disconnected: worker already exited
            // (or panicked before sending) — join to surface any panic.
            Ok(()) | Err(mpsc::RecvTimeoutError::Disconnected) => {}
            Err(mpsc::RecvTimeoutError::Timeout) => {
                panic!("daemon RPC spy stopped within budget: Timeout");
            }
        }
        worker.join().expect("join daemon RPC spy");
    }
}

impl Drop for DaemonRpcSpy {
    fn drop(&mut self) {
        if let Some(worker) = self.worker.take() {
            self.request_shutdown();
            match self.stopped_rx.recv_timeout(WAIT_BUDGET) {
                Ok(()) | Err(mpsc::RecvTimeoutError::Disconnected) => {
                    let _ = worker.join();
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {}
            }
        }
    }
}

fn workspace_status_through_spy(socket: &Path, workspace: &Path) -> Value {
    let mut stream = UnixStream::connect(socket).expect("connect daemon RPC spy");
    try_set_socket_timeouts(&stream, "preflight client");
    let frame = json!({
        "jsonrpc": "2.0",
        "method": "anvil/workspace_status",
        "params": { "workspace_root": workspace },
        "id": "cib-283-spy-preflight",
    });
    writeln!(stream, "{frame}").expect("write spy preflight");

    let mut response = String::new();
    BufReader::new(stream)
        .read_line(&mut response)
        .expect("read spy preflight response");
    serde_json::from_str(response.trim_end()).expect("parse spy preflight response")
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
        Self::spawn_with_action(workspace, home, json, no_daemon, "check")
    }

    fn spawn_with_action(
        workspace: &Path,
        home: &Path,
        json: bool,
        no_daemon: bool,
        action: &str,
    ) -> Self {
        let mut command = Command::new(ANVIL_BIN);
        command.arg("--no-tui");
        if json {
            command.arg("--json");
        }
        command.args(["watch", "--debounce", "50", "--action", action]);
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
                serde_json::from_str::<Value>(line).is_ok_and(|event| {
                    event.get("event_type").and_then(Value::as_str) == Some("snapshot")
                })
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
fn json_fallback_check_and_gate_emit_ordered_action_results_without_daemon_evidence() {
    let home = tempfile::tempdir().expect("private ANVIL_HOME");

    for (action, expected_exit_code) in [("check", 1), ("gate", 2)] {
        let (workspace, source) = new_workspace();
        let mut watch =
            WatchProcess::spawn_with_action(workspace.path(), home.path(), true, true, action);
        watch.wait_ready(true);

        let start = watch.stdout.len();
        save_secret(&source, 0);
        watch.wait_stdout_after(start, "JSON fallback action_result", |line| {
            serde_json::from_str::<Value>(line).is_ok_and(|event| {
                event.get("event_type").and_then(Value::as_str) == Some("action_result")
                    && event.pointer("/payload/action").and_then(Value::as_str) == Some(action)
            })
        });
        watch.stop();

        let events = watch
            .stdout
            .iter()
            .map(|line| {
                serde_json::from_str::<Value>(line).unwrap_or_else(|err| {
                    panic!("stdout must be pure NDJSON: {err}; line={line:?}")
                })
            })
            .collect::<Vec<_>>();
        let sequences = events
            .iter()
            .map(|event| {
                event
                    .get("seq")
                    .and_then(Value::as_u64)
                    .expect("watch event seq must be a u64")
            })
            .collect::<Vec<_>>();
        assert!(
            sequences.windows(2).all(|pair| pair[0] < pair[1]),
            "outer sequence numbers must be unique and follow stdout order for {action}: {sequences:?}"
        );

        let action_event = events
            .iter()
            .skip(start)
            .find(|event| {
                event.get("event_type").and_then(Value::as_str) == Some("action_result")
                    && event.pointer("/payload/action").and_then(Value::as_str) == Some(action)
            })
            .unwrap_or_else(|| panic!("missing fallback action_result for {action}"));
        assert_eq!(
            action_event.get("event_type").and_then(Value::as_str),
            Some("action_result")
        );
        let result: WatchActionResult = serde_json::from_value(action_event["payload"].clone())
            .expect("known action_result payload must match the v1 shape");
        assert_eq!(result.action, action);
        assert_eq!(result.exit_code, Some(expected_exit_code));
        assert_eq!(result.error_detail, None);
        assert_eq!(
            result.daemon_verdict, None,
            "fallback result must not imply daemon assurance: {result:?}"
        );
    }
}

#[test]
fn no_daemon_makes_zero_rpc_calls_to_a_live_daemon_endpoint() {
    let live_home = tempfile::tempdir().expect("live ANVIL_HOME");
    let mut daemon = DaemonGuard::spawn(live_home.path());
    daemon.wait_ready(live_home.path());

    let spy_home = tempfile::tempdir().expect("spy ANVIL_HOME");
    let mut spy = DaemonRpcSpy::spawn(spy_home.path(), &live_home.path().join("intercept.sock"));
    let (workspace, source) = new_workspace();

    let preflight = workspace_status_through_spy(&spy.socket, workspace.path());
    assert!(
        preflight.get("error").is_none(),
        "spy must forward a real workspace_status response: {preflight}"
    );
    assert!(
        preflight
            .pointer("/result/workspace_assurance/state")
            .and_then(Value::as_str)
            .is_some(),
        "spy preflight must receive real daemon assurance: {preflight}"
    );
    assert!(
        spy.accepted_connections() > 0,
        "preflight must prove accepted-connection observability"
    );
    assert!(
        spy.request_frames() > 0,
        "preflight must prove NDJSON request-frame observability"
    );
    spy.reset_counts();

    let mut watch = WatchProcess::spawn(workspace.path(), spy_home.path(), false, true);
    watch.wait_ready(false);
    assert!(
        watch
            .stderr
            .iter()
            .any(|line| line.contains("scoped fallback")),
        "--no-daemon must report the scoped fallback: {:?}",
        watch.stderr
    );
    let stdout_start = watch.stdout.len();
    let stderr_start = watch.stderr.len();
    save_secret(&source, 0);
    watch.wait_stdout_after(stdout_start, "fallback secret finding", |line| {
        line.contains("AWS Key")
    });
    watch.wait_stderr_after(stderr_start, "fallback check result", |line| {
        line.contains("Action 'check' exited with code 1")
    });
    watch.stop();

    let barrier = workspace_status_through_spy(&spy.socket, workspace.path());
    assert!(
        barrier.get("error").is_none(),
        "post-stop barrier must receive a real daemon response: {barrier}"
    );
    assert!(
        barrier
            .pointer("/result/workspace_assurance/state")
            .and_then(Value::as_str)
            .is_some(),
        "post-stop barrier must receive daemon assurance: {barrier}"
    );
    let watch_connections = spy
        .accepted_connections()
        .checked_sub(1)
        .expect("post-stop barrier connection must be counted");
    let watch_frames = spy
        .request_frames()
        .checked_sub(1)
        .expect("post-stop barrier frame must be counted");
    assert_eq!(
        watch_connections, 0,
        "--no-daemon opened the spy before the post-stop barrier"
    );
    assert_eq!(
        watch_frames, 0,
        "--no-daemon sent an RPC frame before the post-stop barrier"
    );
    spy.stop();
}

#[test]
fn json_watch_exits_when_consumer_closes_after_triggering_snapshot() {
    let home = tempfile::tempdir().expect("private ANVIL_HOME");
    let (workspace, source) = new_workspace();
    let mut command = Command::new(ANVIL_BIN);
    command.args([
        "--no-tui",
        "--json",
        "watch",
        "--debounce",
        "50",
        "--no-daemon",
    ]);
    configure_private_env(&mut command, home.path());
    command
        .current_dir(workspace.path())
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    let mut child = command.spawn().expect("spawn JSON watch");
    let stdout = child.stdout.take().expect("piped JSON watch stdout");
    let (snapshot_tx, snapshot_rx) = mpsc::channel();
    let reader = thread::spawn(move || {
        let mut snapshot_count = 0usize;
        for line in BufReader::new(stdout).lines() {
            let line = line.expect("read JSON watch line");
            let event: Value = serde_json::from_str(&line).expect("watch stdout is NDJSON");
            if event.get("event_type").and_then(Value::as_str) == Some("snapshot") {
                snapshot_count += 1;
                snapshot_tx
                    .send(snapshot_count)
                    .expect("report observed snapshot");
                if snapshot_count == 2 {
                    break;
                }
            }
        }
    });

    assert_eq!(
        snapshot_rx
            .recv_timeout(WAIT_BUDGET)
            .expect("initial snapshot within budget"),
        1
    );
    let key = ["AKIA", "QRSTUVWXYZ", "123456"].concat();
    fs::write(
        &source,
        format!(
            "export const accessKey = \"{key}\";\nexport const padding = \"{}\";\n",
            "x".repeat(1 << 20)
        ),
    )
    .expect("write slow-enough triggering save");
    assert_eq!(
        snapshot_rx
            .recv_timeout(WAIT_BUDGET)
            .expect("triggering snapshot within budget"),
        2
    );
    reader.join().expect("close JSON consumer");

    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if let Some(status) = child.try_wait().expect("poll JSON watch") {
            assert!(
                status.success(),
                "BrokenPipe shutdown must be clean: {status}"
            );
            break;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!("JSON watch did not exit after the action_result consumer closed");
        }
        thread::sleep(Duration::from_millis(20));
    }
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
            serde_json::from_str::<Value>(line).is_ok_and(|event| {
                event.get("event_type").and_then(Value::as_str) == Some("action_result")
            })
        });
        let result = json.stdout[start..]
            .iter()
            .filter_map(|line| serde_json::from_str::<Value>(line).ok())
            .find(|event| event.get("event_type").and_then(Value::as_str) == Some("action_result"))
            .map(|event| {
                serde_json::from_value::<WatchActionResult>(event["payload"].clone())
                    .expect("known action_result payload must match the v1 shape")
            })
            .expect("matching action_result payload");
        daemon_results.push(result);
    }
    json.stop();

    let events = json
        .stdout
        .iter()
        .map(|line| {
            serde_json::from_str::<Value>(line)
                .unwrap_or_else(|err| panic!("stdout must be pure NDJSON: {err}; line={line:?}"))
        })
        .collect::<Vec<_>>();
    let sequences = events
        .iter()
        .map(|event| {
            event
                .get("seq")
                .and_then(Value::as_u64)
                .expect("watch event seq must be a u64")
        })
        .collect::<Vec<_>>();
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
