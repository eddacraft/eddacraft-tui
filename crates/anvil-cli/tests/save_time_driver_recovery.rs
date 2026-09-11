//! JREL-003: the daily command restores a failed save-time driver.
//!
//! Public-boundary regression: a real daemon under a private `ANVIL_HOME`, a
//! real durable registration through bare `anvil`, and a real
//! `anvil watch --save-time-driver` child. The test kills **only the child**,
//! re-runs the actual CLI, and then saves a fixture to obtain a genuine
//! validation result from the restored driver. No membership event is injected
//! in place of CLI registration.
//!
//! Linux-only: the harness reads `/proc` to prove that exactly one driver
//! process serves the worktree, and the driver's inotify watch set is the
//! resource under test.

#![cfg(target_os = "linux")]

use std::fs;
use std::io::Read;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::UnixStream;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use nix::sys::signal::{Signal, kill};
use nix::unistd::Pid;
use serde_json::Value;

const ANVIL_BIN: &str = env!("CARGO_BIN_EXE_anvil");
const WAIT_BUDGET: Duration = Duration::from_secs(20);

fn configure_private_env(command: &mut Command, home: &Path) {
    let config_home = home.join("config");
    let cache_home = home.join("cache");
    let state_home = home.join("state");
    let runtime_home = home.join("runtime");
    let temp_home = home.join("tmp");
    for root in [
        &config_home,
        &cache_home,
        &state_home,
        &runtime_home,
        &temp_home,
    ] {
        fs::create_dir_all(root).expect("create isolated state root");
        fs::set_permissions(root, fs::Permissions::from_mode(0o700))
            .expect("secure isolated state root");
    }
    command
        .env("ANVIL_HOME", home)
        .env("HOME", home)
        .env("USERPROFILE", home)
        .env("XDG_CONFIG_HOME", config_home)
        .env("XDG_CACHE_HOME", cache_home)
        .env("XDG_STATE_HOME", state_home)
        .env("XDG_RUNTIME_DIR", runtime_home)
        .env("TMPDIR", temp_home)
        .env_remove("ANVIL_API_URL")
        .env_remove("ANVIL_WATCH_DAEMON")
        .env_remove("ANVIL_NO_PROMPT")
        .env_remove("ANVIL_NO_DAEMON")
        .env_remove("ANVIL_NO_SAVE_TIME_DRIVER")
        .env_remove("ANVIL_TOUCH_PROJECT_STATE")
        .env("ANVIL_NO_MCP", "1")
        .env("ANVIL_DEV", "1")
        .env("ANVIL_SKIP_WELCOME", "1")
        .env("ANVIL_DISABLE_UPDATE_HINT", "1");
}

fn driver_dir(home: &Path) -> PathBuf {
    home.join("runtime").join("save-time-drivers")
}

/// Owns the private daemon for the test and tears it down deliberately:
/// `anvil intercept stop` first (supervisor-owned child shutdown), then a
/// bounded wait that reaps the owned daemon `Child`. Leftover `*.pid` records
/// are signalled only when their starttime still matches the live process.
struct Harness {
    home: PathBuf,
    child: Option<Child>,
}

impl Harness {
    fn spawn(home: &Path) -> Self {
        Self::spawn_with_binary_and_driver(home, Path::new(ANVIL_BIN), true)
    }

    fn spawn_without_driver(home: &Path) -> Self {
        Self::spawn_with_binary_and_driver(home, Path::new(ANVIL_BIN), false)
    }

    fn spawn_with_binary(home: &Path, binary: &Path) -> Self {
        Self::spawn_with_binary_and_driver(home, binary, true)
    }

    fn spawn_with_binary_and_driver(home: &Path, binary: &Path, driver_enabled: bool) -> Self {
        fs::set_permissions(home, fs::Permissions::from_mode(0o700))
            .expect("secure private ANVIL_HOME");
        let mut command = Command::new(binary);
        command.args(["intercept", "start", "--foreground"]);
        configure_private_env(&mut command, home);
        if !driver_enabled {
            command.env("ANVIL_NO_SAVE_TIME_DRIVER", "1");
        }
        command.process_group(0);
        let child = command
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn private intercept daemon");
        let mut harness = Self {
            home: home.to_path_buf(),
            child: Some(child),
        };
        harness.wait_ready();
        harness
    }

    fn wait_ready(&mut self) {
        let socket = self.home.join("intercept.sock");
        let deadline = Instant::now() + WAIT_BUDGET;
        loop {
            let status = self
                .child
                .as_mut()
                .expect("daemon child")
                .try_wait()
                .expect("poll daemon");
            if let Some(status) = status {
                let stderr = self.drain_stderr();
                panic!(
                    "intercept daemon exited before readiness: status={status}; stderr={stderr}"
                );
            }
            if UnixStream::connect(&socket).is_ok() {
                return;
            }
            assert!(
                Instant::now() < deadline,
                "intercept daemon readiness timeout for {}",
                socket.display()
            );
            thread::sleep(Duration::from_millis(50));
        }
    }

    fn drain_stderr(&mut self) -> String {
        let mut stderr = String::new();
        if let Some(child) = self.child.as_mut()
            && let Some(mut pipe) = child.stderr.take()
        {
            let _ = pipe.read_to_string(&mut stderr);
        }
        stderr
    }

    fn anvil(&self, cwd: &Path, args: &[&str]) -> Output {
        let mut command = Command::new(ANVIL_BIN);
        command.args(args).current_dir(cwd);
        configure_private_env(&mut command, &self.home);
        command.output().expect("invoke anvil")
    }

    fn status_json(&self, cwd: &Path) -> Value {
        let out = self.anvil(cwd, &["intercept", "status", "--json"]);
        assert!(
            out.status.success(),
            "intercept status --json failed: stderr={}",
            String::from_utf8_lossy(&out.stderr)
        );
        serde_json::from_slice(&out.stdout).expect("intercept status --json is JSON")
    }

    fn worktree_entry(&self, cwd: &Path, worktree: &Path) -> Option<Value> {
        self.status_json(cwd)["worktrees"]
            .as_array()
            .into_iter()
            .flatten()
            .find(|entry| entry["worktree"].as_str().map(Path::new) == Some(worktree))
            .cloned()
    }

    /// The single `<stem>.pid` record for the one registered worktree.
    fn driver_pid(&self) -> Option<u32> {
        let dir = driver_dir(&self.home);
        let mut records = fs::read_dir(&dir)
            .ok()?
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| path.extension().is_some_and(|ext| ext == "pid"))
            .collect::<Vec<_>>();
        records.sort();
        assert!(
            records.len() <= 1,
            "one registered worktree must own at most one PID record: {records:?}"
        );
        let record = fs::read_to_string(records.first()?).ok()?;
        record.lines().next()?.trim().parse().ok()
    }

    fn findings_log(&self) -> Option<String> {
        let dir = driver_dir(&self.home);
        fs::read_dir(&dir)
            .ok()?
            .flatten()
            .map(|entry| entry.path())
            .find(|path| path.extension().is_some_and(|ext| ext == "log"))
            .and_then(|path| fs::read_to_string(path).ok())
    }
}

#[test]
fn status_fails_when_selected_save_time_driver_is_absent() {
    let home = tempfile::tempdir().expect("private ANVIL_HOME");
    let workspace = tempfile::tempdir().expect("workspace");
    seed_workspace(workspace.path());
    let worktree = fs::canonicalize(workspace.path()).expect("canonical worktree");
    let harness = Harness::spawn_without_driver(home.path());

    // The daemon accepts durable membership but deliberately does not launch a
    // child. The status process does not inherit that daemon-only opt-out, so
    // save-time coverage remains selected from the caller's perspective.
    let registration = harness.anvil(&worktree, &[]);
    assert_eq!(
        registration.status.code(),
        Some(1),
        "absent selected driver must fail daily activation: stdout={} stderr={}",
        String::from_utf8_lossy(&registration.stdout),
        String::from_utf8_lossy(&registration.stderr),
    );

    let json = harness.anvil(&worktree, &["--json", "status"]);
    assert_eq!(
        json.status.code(),
        Some(1),
        "status must fail when the registered worktree has no selected driver: stdout={} stderr={}",
        String::from_utf8_lossy(&json.stdout),
        String::from_utf8_lossy(&json.stderr),
    );
    let doc: Value = serde_json::from_slice(&json.stdout).expect("status output is JSON");
    assert_eq!(doc["readiness"]["state"], "failed");
    assert_eq!(doc["readiness"]["failing_component"], "save_time");
    assert_eq!(
        doc["readiness"]["components"]["save_time"]["state"],
        "failed"
    );
    assert_eq!(
        doc["next"],
        "run bare `anvil` to restore the failed save-time driver"
    );
    assert_eq!(doc["receipt"]["next"], doc["next"]);

    let human = harness.anvil(&worktree, &["--no-tui", "status"]);
    assert_eq!(human.status.code(), Some(1));
    let stdout = String::from_utf8_lossy(&human.stdout);
    assert!(stdout.contains("Readiness: failed"), "{stdout}");
    assert!(stdout.contains("save_time=failed"), "{stdout}");
    assert_eq!(
        stdout
            .lines()
            .filter(|line| line.trim_start().to_ascii_lowercase().starts_with("next:"))
            .count(),
        1,
        "{stdout}"
    );
    assert!(stdout.contains("bare `anvil`"), "{stdout}");
    assert!(
        !stdout.contains("anvil intercept start --foreground"),
        "{stdout}"
    );
}

#[test]
fn bare_anvil_fails_when_daemon_rejects_worktree_registration() {
    let home = tempfile::tempdir().expect("private ANVIL_HOME");
    let workspace = tempfile::tempdir().expect("workspace");
    seed_workspace(workspace.path());
    let worktree = fs::canonicalize(workspace.path()).expect("canonical worktree");

    let stale_binary = home.path().join("stale-anvil");
    fs::copy(ANVIL_BIN, &stale_binary).expect("copy daemon binary");
    fs::set_permissions(&stale_binary, fs::Permissions::from_mode(0o700))
        .expect("make daemon binary executable");
    let harness = Harness::spawn_with_binary(home.path(), &stale_binary);

    fs::remove_file(&stale_binary).expect("remove daemon executable before driver spawn");

    let plain = harness.anvil(&worktree, &[]);
    assert_eq!(
        plain.status.code(),
        Some(1),
        "a rejected worktree registration must fail bare activation; stdout={} stderr={}",
        String::from_utf8_lossy(&plain.stdout),
        String::from_utf8_lossy(&plain.stderr),
    );
    let stdout = String::from_utf8_lossy(&plain.stdout);
    assert!(stdout.contains("readiness: failed"), "{stdout}");
    assert!(stdout.contains("worktree=failed"), "{stdout}");
    assert_eq!(stdout.matches("next:").count(), 1, "{stdout}");
    assert!(stdout.contains("anvil doctor"), "{stdout}");
    assert!(
        !stdout.contains("anvil intercept start --foreground"),
        "{stdout}"
    );

    let json = harness.anvil(&worktree, &["--json"]);
    assert_eq!(
        json.status.code(),
        Some(1),
        "JSON activation must carry the same failure exit; stdout={} stderr={}",
        String::from_utf8_lossy(&json.stdout),
        String::from_utf8_lossy(&json.stderr),
    );
    let value: Value = serde_json::from_slice(&json.stdout).expect("bare --json is one document");
    assert_eq!(value["readiness"]["state"], "failed");
    assert_eq!(
        value["readiness"]["components"]["worktree"]["state"],
        "failed"
    );
    assert_eq!(
        value["next"],
        "run `anvil doctor` to diagnose the unresolved worktree fault"
    );

    let start = harness.anvil(&worktree, &["start", "--no-tui"]);
    assert_eq!(
        start.status.code(),
        Some(1),
        "anvil start must fail the same rejected registration; stdout={} stderr={}",
        String::from_utf8_lossy(&start.stdout),
        String::from_utf8_lossy(&start.stderr),
    );
    let start_stdout = String::from_utf8_lossy(&start.stdout);
    assert!(start_stdout.contains("readiness: failed"), "{start_stdout}");
    assert!(start_stdout.contains("worktree=failed"), "{start_stdout}");
    assert!(
        start_stdout.contains("failed worktree registration"),
        "{start_stdout}"
    );
    assert_eq!(start_stdout.matches("next:").count(), 1, "{start_stdout}");
    assert!(
        !start_stdout.contains("anvil intercept start --foreground"),
        "{start_stdout}"
    );
}

impl Drop for Harness {
    fn drop(&mut self) {
        let _ = self.anvil(&self.home, &["intercept", "stop"]);
        if let Some(mut child) = self.child.take() {
            reap_owned_daemon(&mut child, Duration::from_secs(2));
        }
        reap_leftover_driver_records(&driver_dir(&self.home));
    }
}

/// Wait for the owned daemon to exit after supervisor stop, then SIGKILL
/// through the unreaped `Child` handle. The PID cannot have been recycled
/// while this process still holds the child.
fn reap_owned_daemon(child: &mut Child, budget: Duration) {
    let deadline = Instant::now() + budget;
    loop {
        match child.try_wait() {
            Ok(Some(_)) => return,
            Ok(None) if Instant::now() < deadline => {
                thread::sleep(Duration::from_millis(50));
            }
            Ok(None) | Err(_) => {
                let _ = child.kill();
                let _ = child.wait();
                return;
            }
        }
    }
}

/// `pid\n[start_time\n]` — the record the supervisor persists.
fn read_pid_record(path: &Path) -> Option<(u32, Option<u64>)> {
    let content = fs::read_to_string(path).ok()?;
    let mut lines = content.lines();
    let pid = lines.next()?.trim().parse().ok()?;
    let start_time = lines.next().and_then(|line| line.trim().parse().ok());
    Some((pid, start_time))
}

/// Signal `pid` only when the recorded start time is present and still
/// matches `/proc`. A missing discriminator is never signalled: the numeric
/// PID may already belong to someone else.
fn signal_recorded_pid_if_same_process(
    pid: u32,
    recorded_start_time: Option<u64>,
    signal: &mut impl FnMut(u32),
) -> bool {
    let Some(recorded) = recorded_start_time else {
        return false;
    };
    if proc_start_time(pid) == Some(recorded) {
        signal(pid);
        true
    } else {
        false
    }
}

fn kill_pid(pid: u32) {
    if let Ok(raw) = i32::try_from(pid) {
        let _ = kill(Pid::from_raw(raw), Signal::SIGKILL);
    }
}

fn reap_leftover_driver_records(dir: &Path) {
    reap_leftover_driver_records_with(dir, kill_pid);
}

fn reap_leftover_driver_records_with(dir: &Path, mut signal: impl FnMut(u32)) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for path in entries.flatten().map(|entry| entry.path()) {
        if path.extension().is_none_or(|ext| ext != "pid") {
            continue;
        }
        let Some((pid, start_time)) = read_pid_record(&path) else {
            continue;
        };
        let _ = signal_recorded_pid_if_same_process(pid, start_time, &mut signal);
    }
}

#[test]
fn harness_drop_does_not_signal_a_recycled_driver_pid() {
    let home = tempfile::tempdir().expect("private ANVIL_HOME");
    let harness = Harness::spawn_without_driver(home.path());

    let mut sentinel = Command::new("sleep")
        .arg("60")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn unrelated sentinel");
    let pid = sentinel.id();
    let start = proc_start_time(pid).expect("sentinel starttime");
    let dir = driver_dir(home.path());
    fs::create_dir_all(&dir).expect("driver dir");
    fs::write(
        dir.join("recycled.pid"),
        format!("{pid}\n{}\n", start.wrapping_add(1)),
    )
    .expect("stale pid record");

    drop(harness);

    let still_running = sentinel.try_wait().expect("poll sentinel").is_none();
    let _ = sentinel.kill();
    let _ = sentinel.wait();
    assert!(
        still_running,
        "teardown must not SIGKILL a live process whose numeric PID was reused in a driver record"
    );
}

#[test]
fn leftover_pid_record_without_start_time_is_never_signalled() {
    let mut sent = Vec::new();
    let signalled =
        signal_recorded_pid_if_same_process(std::process::id(), None, &mut |pid| sent.push(pid));
    assert!(!signalled);
    assert!(sent.is_empty());
}

#[test]
fn leftover_pid_record_with_mismatched_start_time_is_not_signalled() {
    let pid = std::process::id();
    let start = proc_start_time(pid).expect("self starttime");
    let mut sent = Vec::new();
    let signalled =
        signal_recorded_pid_if_same_process(pid, Some(start.wrapping_add(1)), &mut |p| {
            sent.push(p);
        });
    assert!(!signalled);
    assert!(sent.is_empty());
}

#[test]
fn leftover_driver_record_signals_only_verified_identity() {
    let dir = tempfile::tempdir().expect("pid dir");
    let self_pid = std::process::id();
    let start = proc_start_time(self_pid).expect("self starttime");
    fs::write(
        dir.path().join("match.pid"),
        format!("{self_pid}\n{start}\n"),
    )
    .expect("matching record");
    fs::write(
        dir.path().join("stale.pid"),
        format!("{self_pid}\n{}\n", start.wrapping_add(1)),
    )
    .expect("stale record");
    fs::write(dir.path().join("bare.pid"), format!("{self_pid}\n")).expect("bare record");
    fs::write(dir.path().join("junk.pid"), "not a pid\n").expect("malformed record");
    let mut sent = Vec::new();
    reap_leftover_driver_records_with(dir.path(), |pid| sent.push(pid));
    assert_eq!(
        sent,
        vec![self_pid],
        "only the starttime-verified leftover may be signalled"
    );
}

fn wait_until<T>(what: &str, mut probe: impl FnMut() -> Option<T>) -> T {
    let deadline = Instant::now() + WAIT_BUDGET;
    loop {
        if let Some(value) = probe() {
            return value;
        }
        assert!(
            Instant::now() < deadline,
            "timed out after {WAIT_BUDGET:?} waiting for {what}"
        );
        thread::sleep(Duration::from_millis(100));
    }
}

/// `/proc/<pid>/stat` state letter, or `None` once the entry is gone.
fn proc_state(pid: u32) -> Option<char> {
    let stat = fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    stat.rsplit_once(") ")?.1.chars().next()
}

/// `/proc/<pid>/stat` field 22 (`starttime` in clock ticks since boot).
fn proc_start_time(pid: u32) -> Option<u64> {
    let stat = fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    let after_command = stat.rsplit_once(") ")?.1;
    after_command.split_whitespace().nth(19)?.parse().ok()
}

/// Every live (non-zombie) process whose argv is the save-time driver for
/// `worktree`.
fn live_driver_pids(worktree: &Path) -> Vec<u32> {
    let needle = worktree.as_os_str().to_string_lossy().into_owned();
    let mut pids = Vec::new();
    for entry in fs::read_dir("/proc").expect("read /proc").flatten() {
        let Some(pid) = entry
            .file_name()
            .to_str()
            .and_then(|name| name.parse::<u32>().ok())
        else {
            continue;
        };
        let Ok(cmdline) = fs::read(entry.path().join("cmdline")) else {
            continue;
        };
        let argv = cmdline
            .split(|byte| *byte == 0)
            .map(|arg| String::from_utf8_lossy(arg).into_owned())
            .collect::<Vec<_>>();
        if argv.contains(&"--save-time-driver".to_owned())
            && argv.contains(&needle)
            && proc_state(pid).is_some_and(|state| state != 'Z' && state != 'X')
        {
            pids.push(pid);
        }
    }
    pids
}

fn seed_workspace(dir: &Path) {
    let ok = Command::new("git")
        .args(["init", "-q"])
        .current_dir(dir)
        .status()
        .expect("git init")
        .success();
    assert!(ok, "git init failed");
    fs::write(dir.join(".anvil.json"), "{\"checks\":[]}\n").expect("seed activated config");
    fs::write(dir.join("clean.ts"), "export const clean = true;\n").expect("seed source");
}

fn assert_ensure_succeeded(out: &Output, context: &str) {
    assert_eq!(
        out.status.code(),
        Some(0),
        "{context}: bare anvil must exit 0; stdout={}\nstderr={}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}

fn assert_ready_no_mcp_status(harness: &Harness, worktree: &Path) {
    let status = harness.anvil(worktree, &["--json", "status"]);
    assert_eq!(
        status.status.code(),
        Some(0),
        "ready save-time coverage must be a successful no-MCP status: stdout={} stderr={}",
        String::from_utf8_lossy(&status.stdout),
        String::from_utf8_lossy(&status.stderr),
    );
    let status_doc: Value =
        serde_json::from_slice(&status.stdout).expect("ready status output is JSON");
    assert_eq!(status_doc["readiness"]["state"], "ready");
    assert_eq!(
        status_doc["readiness"]["components"]["save_time"]["state"],
        "ready"
    );
    assert_eq!(
        status_doc["readiness"]["components"]["mcp"]["state"],
        "disabled"
    );
}

#[test]
fn bare_anvil_restores_exactly_one_ready_driver_after_child_death() {
    let home = tempfile::tempdir().expect("private ANVIL_HOME");
    let workspace = tempfile::tempdir().expect("workspace");
    seed_workspace(workspace.path());
    let worktree = fs::canonicalize(workspace.path()).expect("canonical worktree");
    let harness = Harness::spawn(home.path());

    // First public registration: a fresh durable membership spawns the driver.
    let first = harness.anvil(&worktree, &[]);
    assert_ensure_succeeded(&first, "first registration");
    let first_stdout = String::from_utf8_lossy(&first.stdout);
    assert!(
        first_stdout.contains("worktree: registered with the save-time daemon"),
        "first run must record a fresh registration:\n{first_stdout}"
    );
    let first_pid = wait_until("the first driver to be attached", || {
        let entry = harness.worktree_entry(&worktree, &worktree)?;
        (entry["save_time_driver"] == "attached").then(|| harness.driver_pid())?
    });
    assert_eq!(
        live_driver_pids(&worktree),
        vec![first_pid],
        "exactly one live driver serves the worktree after the first run"
    );

    // Kill ONLY the child. The daemon keeps running and the durable
    // registration is untouched.
    kill(
        Pid::from_raw(i32::try_from(first_pid).expect("pid fits")),
        Signal::SIGKILL,
    )
    .expect("kill the driver child");
    wait_until("the killed driver to leave the live process table", || {
        matches!(proc_state(first_pid), None | Some('Z' | 'X')).then_some(())
    });
    assert!(
        live_driver_pids(&worktree).is_empty(),
        "no live driver may serve the worktree once the child is dead"
    );

    // Re-run the daily command. Membership is refreshed (not re-created) and
    // the driver must be restored through that public path.
    let second = harness.anvil(&worktree, &[]);
    assert_ensure_succeeded(&second, "re-run after child death");
    let second_stdout = String::from_utf8_lossy(&second.stdout);
    assert!(
        second_stdout.contains("worktree: registration refreshed"),
        "the re-run must refresh durable membership rather than re-register:\n{second_stdout}"
    );
    // The CLI's own bounded wait normally makes this true by the time it
    // exits; under a loaded runner the daemon's snapshot can lag it, so this
    // is retried like the surrounding assertions rather than read once.
    wait_until(
        "the re-run to leave the driver attached, not merely refreshed",
        || {
            let entry = harness
                .worktree_entry(&worktree, &worktree)
                .expect("worktree stays a durable member across the re-run");
            (entry["save_time_driver"] == "attached").then_some(())
        },
    );
    let second_pid = harness
        .driver_pid()
        .expect("a PID record exists for the restored driver");
    assert_ne!(
        second_pid, first_pid,
        "the restored driver must be a new child, not the dead PID re-reported as attached"
    );
    assert_eq!(
        live_driver_pids(&worktree),
        vec![second_pid],
        "exactly one live driver serves the worktree after recovery"
    );

    // Readiness evidence is stronger than "spawned": wait for the restored
    // child to report its watches installed before saving the fixture.
    wait_until("the restored driver to install its watches", || {
        let entry = harness.worktree_entry(&worktree, &worktree)?;
        matches!(
            entry["save_time_driver_evidence"].as_str(),
            Some("watches-installed" | "fresh-activity")
        )
        .then_some(())
    });

    assert_ready_no_mcp_status(&harness, &worktree);

    // Save a fixture the daemon's antipattern lane flags (AP-003 explicit
    // `any`) and obtain a real validation result from the restored driver.
    fs::write(
        worktree.join("planted.ts"),
        "export const planted: any = {};\n",
    )
    .expect("save planted fixture");
    let log = wait_until(
        "the restored driver to log a finding for the saved fixture",
        || {
            harness
                .findings_log()
                .filter(|log| log.contains("planted.ts"))
        },
    );
    // The summary line's wording depends on whether the daemon answered with an
    // assurance scope, so assert on the verdict itself: the AP-003 finding for
    // the saved file.
    assert!(
        log.contains("AP-003"),
        "the findings log must carry the daemon verdict for the save:\n{log}"
    );
    let entry = harness
        .worktree_entry(&worktree, &worktree)
        .expect("worktree entry after the save");
    assert_eq!(
        entry["save_time_driver_evidence"], "fresh-activity",
        "a processed save must be reported as fresh activity; entry={entry}"
    );
    assert_eq!(
        live_driver_pids(&worktree),
        vec![second_pid],
        "the validated save must not have spawned a second driver"
    );
}
