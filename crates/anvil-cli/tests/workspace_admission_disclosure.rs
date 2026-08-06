//! CIB-232: `anvil workspace list` must disclose what the `open` admission mode
//! actually does, proven end-to-end against the built `anvil` binary.
//!
//! A fresh home reports `Admission mode: open` with no allow entries. `open` is
//! the **intentional** factory posture (first-touch adopt), but the mode line
//! alone reads as enforcement that has simply not caught anything yet. These
//! tests pin the disclosure and the non-scope guard: the default stays `open`.
//!
//! `ANVIL_HOME`, `HOME`, and `USERPROFILE` are set on the **child process only**
//! (`Command::env`), never on the test process, so the runs are hermetic and
//! race-free regardless of test parallelism.

use std::path::Path;
use std::process::{Child, Command, Output, Stdio};
use std::time::Duration;

use tempfile::tempdir;

const ANVIL_BIN: &str = env!("CARGO_BIN_EXE_anvil");

struct DaemonChild(Child);

impl Drop for DaemonChild {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// Run `anvil workspace <args> --no-tui` against a fresh install root. Returns
/// `(exit_ok, stdout)`.
///
/// `ANVIL_DEV=1` bypasses the licence auth gate (`workspace` is a gated
/// command); the temp `HOME` keeps default path resolution — including the
/// daemon socket — off the developer's real state.
fn run_workspace(home: &Path, args: &[&str]) -> (bool, String) {
    let mut cmd = Command::new(ANVIL_BIN);
    cmd.arg("workspace")
        .args(args)
        .arg("--no-tui")
        .current_dir(home)
        .env("ANVIL_HOME", home)
        .env("HOME", home)
        .env("USERPROFILE", home)
        .env("ANVIL_DEV", "1")
        .env("ANVIL_SKIP_WELCOME", "1")
        .env_remove("ANVIL_NO_DAEMON");
    let out = cmd.output().expect("spawn anvil workspace");
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
    )
}

fn run_workspace_json(home: &Path) -> Output {
    Command::new(ANVIL_BIN)
        .arg("--json")
        .args(["workspace", "list", "--no-tui"])
        .current_dir(home)
        .env("ANVIL_HOME", home)
        .env("HOME", home)
        .env("USERPROFILE", home)
        .env("ANVIL_DEV", "1")
        .env("ANVIL_SKIP_WELCOME", "1")
        .env_remove("ANVIL_NO_DAEMON")
        .output()
        .expect("spawn anvil --json workspace list")
}

fn spawn_daemon(home: &Path, worktree: &Path) -> DaemonChild {
    let child = Command::new(ANVIL_BIN)
        .args(["intercept", "start", "--foreground"])
        .current_dir(worktree)
        .env("ANVIL_HOME", home)
        .env("HOME", home)
        .env("USERPROFILE", home)
        .env("ANVIL_DEV", "1")
        .env("ANVIL_SKIP_WELCOME", "1")
        .env("ANVIL_DISABLE_UPDATE_HINT", "1")
        .env_remove("ANVIL_NO_DAEMON")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn intercept daemon");
    DaemonChild(child)
}

fn wait_for_available_registry(home: &Path) -> serde_json::Value {
    for _ in 0..100 {
        let output = run_workspace_json(home);
        if output.status.success()
            && let Ok(value) = serde_json::from_slice::<serde_json::Value>(&output.stdout)
            && value["registered_worktrees"]["availability"] == "available"
        {
            return value;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    panic!("daemon registry did not become available");
}

fn wait_for_registered_worktree(home: &Path, worktree: &str) -> serde_json::Value {
    let mut last = serde_json::Value::Null;
    for _ in 0..100 {
        let output = run_workspace_json(home);
        if output.status.success()
            && let Ok(value) = serde_json::from_slice::<serde_json::Value>(&output.stdout)
        {
            let found = value["registered_worktrees"]["entries"]
                .as_array()
                .is_some_and(|entries| entries.iter().any(|entry| entry == worktree));
            if found {
                return value;
            }
            last = value;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    panic!("registered worktree did not appear; last workspace list: {last}");
}

fn wait_for_unregistered_worktree(home: &Path, worktree: &str) -> serde_json::Value {
    let mut last = serde_json::Value::Null;
    for _ in 0..100 {
        let output = run_workspace_json(home);
        if output.status.success()
            && let Ok(value) = serde_json::from_slice::<serde_json::Value>(&output.stdout)
        {
            let found = value["registered_worktrees"]["entries"]
                .as_array()
                .is_some_and(|entries| entries.iter().any(|entry| entry == worktree));
            if value["registered_worktrees"]["availability"] == "available" && !found {
                return value;
            }
            last = value;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    panic!("registered worktree did not disappear; last workspace list: {last}");
}

#[test]
fn fresh_home_json_list_is_one_structured_document() {
    let home = tempdir().expect("temp home");
    let output = run_workspace_json(home.path());
    assert!(
        output.status.success(),
        "workspace list succeeds: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!(
            "stdout must contain exactly one JSON document: {error}; stdout={}",
            String::from_utf8_lossy(&output.stdout)
        )
    });
    assert_eq!(value["schema_version"], "anvil.workspace-list.v1");
    assert_eq!(value["admission_mode"], "open");
    assert_eq!(value["allow_entries"], serde_json::json!([]));
    assert_eq!(
        value["registered_worktrees"],
        serde_json::json!({
            "availability": "unavailable",
            "entries": [],
        })
    );
    assert_eq!(value["register_on_start"], serde_json::json!([]));
}

#[test]
fn configured_json_list_preserves_paths_kinds_and_registration_state() {
    let home = tempdir().expect("temp home");
    let worktree = home.path().join("registered-worktree");
    let prefix = home.path().join("projects");
    std::fs::create_dir(&worktree).expect("create worktree");
    std::fs::create_dir(&prefix).expect("create prefix");
    let git = Command::new("git")
        .args(["init", "--quiet"])
        .current_dir(&worktree)
        .output()
        .expect("initialise test worktree");
    assert!(git.status.success(), "git init failed");

    let worktree_text = worktree.to_str().expect("utf-8 worktree");
    let prefix_text = prefix.to_str().expect("utf-8 prefix");
    assert!(run_workspace(home.path(), &["allow", worktree_text]).0);
    assert!(run_workspace(home.path(), &["allow", prefix_text, "--prefix"]).0);
    assert!(
        run_workspace(home.path(), &["register", worktree_text, "--persist"]).0,
        "persisting register-on-start intent succeeds while daemon is unavailable"
    );

    let output = run_workspace_json(home.path());
    assert!(output.status.success());
    let value: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("workspace list JSON parses");
    let allow_entries = value["allow_entries"]
        .as_array()
        .expect("allow entries array");
    assert!(
        allow_entries.iter().any(|entry| {
            entry["path"] == worktree_text
                && entry["kind"] == "exact"
                && entry["live_registered"].is_null()
        }),
        "exact allow entry preserves unknown live membership: {value}"
    );
    assert!(
        allow_entries.iter().any(|entry| {
            entry["path"] == prefix_text
                && entry["kind"] == "prefix"
                && entry["live_registered"].is_null()
        }),
        "prefix allow entry preserves kind and unknown membership: {value}"
    );
    assert_eq!(value["registered_worktrees"]["availability"], "unavailable");
    assert_eq!(
        value["register_on_start"],
        serde_json::json!([{
            "path": worktree_text,
            "state": "unknown",
        }])
    );
}

#[test]
fn live_json_list_preserves_registered_membership() {
    let home = tempdir().expect("temp home");
    let worktree = home.path().join("live-worktree");
    std::fs::create_dir(&worktree).expect("create worktree");
    let git = Command::new("git")
        .args(["init", "--quiet"])
        .current_dir(&worktree)
        .output()
        .expect("initialise test worktree");
    assert!(git.status.success(), "git init failed");

    let _daemon = spawn_daemon(home.path(), &worktree);
    let empty = wait_for_available_registry(home.path());
    assert_eq!(
        empty["registered_worktrees"]["entries"],
        serde_json::json!([]),
        "available and known-empty must stay distinct from daemon unavailable"
    );

    let worktree = dunce::canonicalize(worktree).expect("canonical worktree");
    let worktree_text = worktree.to_str().expect("utf-8 worktree");
    assert!(run_workspace(home.path(), &["allow", worktree_text]).0);
    let (registered, registration_stdout) =
        run_workspace(home.path(), &["register", worktree_text, "--persist"]);
    assert!(
        registered
            && (registration_stdout.contains("Registered")
                || registration_stdout.contains("Refreshed")),
        "live registration succeeds: {registration_stdout}"
    );

    let value = wait_for_registered_worktree(home.path(), worktree_text);
    assert!(
        value["registered_worktrees"]["entries"]
            .as_array()
            .is_some_and(|entries| entries.iter().any(|entry| entry == worktree_text)),
        "registered worktree membership is preserved: {value}"
    );
    assert!(
        value["allow_entries"].as_array().is_some_and(|entries| {
            entries
                .iter()
                .any(|entry| entry["path"] == worktree_text && entry["live_registered"] == true)
        }),
        "allow entry carries its live-registration annotation: {value}"
    );
    assert_eq!(
        value["register_on_start"],
        serde_json::json!([{
            "path": worktree_text,
            "state": "registered",
        }])
    );

    let (unregistered, unregistration_stdout) =
        run_workspace(home.path(), &["unregister", worktree_text]);
    assert!(
        unregistered && unregistration_stdout.contains("Unregistered"),
        "live unregister succeeds without dropping persisted intent: {unregistration_stdout}"
    );

    let value = wait_for_unregistered_worktree(home.path(), worktree_text);
    assert!(
        value["allow_entries"].as_array().is_some_and(|entries| {
            entries
                .iter()
                .any(|entry| entry["path"] == worktree_text && entry["live_registered"] == false)
        }),
        "allow entry reports that live membership was removed: {value}"
    );
    assert_eq!(
        value["register_on_start"],
        serde_json::json!([{
            "path": worktree_text,
            "state": "not_registered",
        }]),
        "non-persistent unregister must retain persisted start-up intent"
    );
}

#[test]
fn fresh_home_list_discloses_that_open_is_not_confinement() {
    let home = tempdir().expect("temp home");
    let (ok, stdout) = run_workspace(home.path(), &["list"]);
    assert!(ok, "workspace list succeeds on a fresh home: {stdout}");

    // The pre-existing posture line is unchanged: `open` remains the default.
    assert!(
        stdout.contains("Admission mode: open"),
        "fresh home still reports open as the mode: {stdout}"
    );
    // CIB-232: and now says what that means, so an empty allow list cannot be
    // skimmed as enforcement.
    assert!(
        stdout.contains("first touch") && stdout.contains("not confined"),
        "open mode discloses first-touch adopt and that confinement is off: {stdout}"
    );
    assert!(
        stdout.contains("anvil workspace mode allowlist"),
        "the disclosure names the command that confines the daemon: {stdout}"
    );
}

#[test]
fn open_mode_with_allow_entries_still_discloses_they_are_inert() {
    // The worst case to misread: `Allow entries:` followed by real paths, under
    // a mode that never consults them. Populated entries look far more like
    // active enforcement than an empty list does.
    let home = tempdir().expect("temp home");
    let project = home.path().join("proj");
    std::fs::create_dir(&project).expect("create project dir");

    let (ok, stdout) = run_workspace(
        home.path(),
        &["allow", project.to_str().expect("utf-8 path")],
    );
    assert!(ok, "workspace allow succeeds: {stdout}");

    let (ok, stdout) = run_workspace(home.path(), &["list"]);
    assert!(ok, "workspace list succeeds: {stdout}");
    assert!(
        stdout.contains("Admission mode: open"),
        "the mode is still open: {stdout}"
    );
    assert!(
        stdout.contains("proj"),
        "the allow entry is listed: {stdout}"
    );
    assert!(
        stdout.contains("first touch") && stdout.contains("not confined"),
        "listed entries do not suppress the disclosure that they are inert: {stdout}"
    );
}

#[test]
fn allowlist_mode_list_has_no_open_disclosure() {
    let home = tempdir().expect("temp home");
    let (ok, set_stdout) = run_workspace(home.path(), &["mode", "allowlist"]);
    assert!(ok, "workspace mode allowlist succeeds: {set_stdout}");

    let (ok, stdout) = run_workspace(home.path(), &["list"]);
    assert!(ok, "workspace list succeeds under allowlist: {stdout}");
    assert!(
        stdout.contains("Admission mode: allowlist"),
        "the mode switch persisted: {stdout}"
    );
    assert!(
        !stdout.contains("first touch"),
        "the open-mode disclosure is not printed under allowlist: {stdout}"
    );
    // The existing fail-closed copy still carries the allowlist consequence.
    assert!(
        stdout.contains("fail-closed"),
        "an empty allowlist still reports fail-closed: {stdout}"
    );
}

#[test]
fn setting_open_mode_discloses_the_posture_it_restores() {
    let home = tempdir().expect("temp home");
    let (ok, _) = run_workspace(home.path(), &["mode", "allowlist"]);
    assert!(ok, "workspace mode allowlist succeeds");

    let (ok, stdout) = run_workspace(home.path(), &["mode", "open"]);
    assert!(ok, "workspace mode open succeeds: {stdout}");
    assert!(
        stdout.contains("Admission mode set to open"),
        "the mode set is reported: {stdout}"
    );
    assert!(
        stdout.contains("first touch") && stdout.contains("not confined"),
        "switching back to open discloses what it turns off: {stdout}"
    );
}
