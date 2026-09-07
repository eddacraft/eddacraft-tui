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

use nix::sys::signal::{Signal, kill, killpg};
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
/// `anvil intercept stop` first (so the supervisor terminates its children),
/// then any driver PID still recorded on disk, then the daemon process group.
struct Harness {
    home: PathBuf,
    child: Option<Child>,
}

impl Harness {
    fn spawn(home: &Path) -> Self {
        fs::set_permissions(home, fs::Permissions::from_mode(0o700))
            .expect("secure private ANVIL_HOME");
        let mut command = Command::new(ANVIL_BIN);
        command.args(["intercept", "start", "--foreground"]);
        configure_private_env(&mut command, home);
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

impl Drop for Harness {
    fn drop(&mut self) {
        let _ = self.anvil(&self.home, &["intercept", "stop"]);
        if let Ok(entries) = fs::read_dir(driver_dir(&self.home)) {
            for path in entries.flatten().map(|entry| entry.path()) {
                if path.extension().is_some_and(|ext| ext == "pid")
                    && let Ok(record) = fs::read_to_string(&path)
                    && let Some(pid) = record.lines().next().and_then(|l| l.trim().parse().ok())
                {
                    let _ = kill(Pid::from_raw(pid), Signal::SIGKILL);
                }
            }
        }
        if let Some(mut child) = self.child.take() {
            if let Ok(pgid) = i32::try_from(child.id()) {
                let _ = killpg(Pid::from_raw(pgid), Signal::SIGKILL);
            }
            let _ = child.kill();
            let _ = child.wait();
        }
    }
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
    fs::write(dir.join(".anvil.json"), "{}\n").expect("seed activated config");
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
