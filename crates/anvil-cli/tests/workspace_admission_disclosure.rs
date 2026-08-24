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

/// Phrase the CLI emits when a daemon round-trip exceeds its budget.
/// Mirrors `registration.rs`'s transport error; kept as a constant so the
/// retry helper and the assertions below cannot drift apart.
const DAEMON_TRANSPORT_TIMEOUT: &str = "timed out talking to the daemon";

/// Run a `workspace` subcommand that performs a daemon round-trip, retrying
/// **only** on transport timeout.
///
/// The round-trip budget is 500 ms (`ACTIVATION_DAEMON_QUERY_TIMEOUT`), tuned
/// so an interactive command cannot hang on a sick daemon. The registry waits
/// below get 100 attempts against that budget; these calls got exactly one —
/// and their stdout is what the assertions read. The most load-bearing step
/// had the least tolerance, which is how a contended Windows runner failed
/// `live_json_list_preserves_registered_membership` in CI nightly on
/// 2026-08-09: `list` succeeded because it had ~100 chances, `register` had
/// one and lost the race.
///
/// Deliberately narrow. A transport timeout means the daemon never answered,
/// so no outcome was observed and retrying asks the same question again. Every
/// other result — including the durable-gate refusal these tests exist to
/// cover — is returned from the first response and asserted strictly, because
/// those *are* observed outcomes and retrying could mask a real regression.
///
/// Returns `(exit_ok, stdout, retried)`. The `retried` flag matters because the
/// timeout is **client-side only**: `round_trip` (`registration.rs:562`) writes
/// the request on a spawned thread and gives up on `recv_timeout`, so a timed
/// -out request may still have been processed by the daemon — nothing cancels
/// it. A retry can therefore land on an already-applied state and get the
/// idempotent wording rather than the acting wording. Callers use the flag to
/// widen an assertion for exactly that window and no wider.
fn run_workspace_awaiting_daemon(home: &Path, args: &[&str]) -> (bool, String, bool) {
    let mut last = run_workspace(home, args);
    let mut retried = false;
    for _ in 0..10 {
        if !last.1.contains(DAEMON_TRANSPORT_TIMEOUT) {
            return (last.0, last.1, retried);
        }
        std::thread::sleep(Duration::from_millis(100));
        last = run_workspace(home, args);
        retried = true;
    }
    (last.0, last.1, retried)
}

/// Compare paths after resolving both spellings to the same filesystem identity.
///
/// Windows may expose one temp path in long form and another in 8.3 form; macOS
/// has the equivalent `/var` → `/private/var` alias.
fn listed_path_matches(listed: &serde_json::Value, expected: &Path) -> bool {
    let Some(listed) = listed.as_str() else {
        return false;
    };
    let Ok(listed) = dunce::canonicalize(listed) else {
        return false;
    };
    let Ok(expected) = dunce::canonicalize(expected) else {
        return false;
    };
    listed == expected
}

fn assert_single_path_state(
    entries: &serde_json::Value,
    expected_path: &Path,
    expected_state: &str,
) {
    let entries = entries.as_array().expect("path-state entries array");
    assert_eq!(
        entries.len(),
        1,
        "expected exactly one path-state entry: {entries:?}"
    );
    assert!(
        listed_path_matches(&entries[0]["path"], expected_path),
        "listed path does not resolve to {}: {entries:?}",
        expected_path.display()
    );
    assert_eq!(entries[0]["state"], expected_state);
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

fn wait_for_registered_worktree(home: &Path, worktree: &Path) -> serde_json::Value {
    let mut last = serde_json::Value::Null;
    for _ in 0..100 {
        let output = run_workspace_json(home);
        if output.status.success()
            && let Ok(value) = serde_json::from_slice::<serde_json::Value>(&output.stdout)
        {
            let found = value["registered_worktrees"]["entries"]
                .as_array()
                .is_some_and(|entries| {
                    entries
                        .iter()
                        .any(|entry| listed_path_matches(entry, worktree))
                });
            if found {
                return value;
            }
            last = value;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    panic!("registered worktree did not appear; last workspace list: {last}");
}

fn wait_for_unregistered_worktree(home: &Path, worktree: &Path) -> serde_json::Value {
    let mut last = serde_json::Value::Null;
    for _ in 0..100 {
        let output = run_workspace_json(home);
        if output.status.success()
            && let Ok(value) = serde_json::from_slice::<serde_json::Value>(&output.stdout)
        {
            let found = value["registered_worktrees"]["entries"]
                .as_array()
                .is_some_and(|entries| {
                    entries
                        .iter()
                        .any(|entry| listed_path_matches(entry, worktree))
                });
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
    let worktree = dunce::canonicalize(&worktree).expect("canonical worktree fixture");
    let prefix = dunce::canonicalize(&prefix).expect("canonical prefix fixture");
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
            listed_path_matches(&entry["path"], &worktree)
                && entry["kind"] == "exact"
                && entry["live_registered"].is_null()
        }),
        "exact allow entry preserves unknown live membership: {value}"
    );
    assert!(
        allow_entries.iter().any(|entry| {
            listed_path_matches(&entry["path"], &prefix)
                && entry["kind"] == "prefix"
                && entry["live_registered"].is_null()
        }),
        "prefix allow entry preserves kind and unknown membership: {value}"
    );
    assert_eq!(value["registered_worktrees"]["availability"], "unavailable");
    assert_single_path_state(&value["register_on_start"], &worktree, "unknown");
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

    let mut daemon = spawn_daemon(home.path(), &worktree);
    let empty = wait_for_available_registry(home.path());
    assert_eq!(
        empty["registered_worktrees"]["entries"],
        serde_json::json!([]),
        "available and known-empty must stay distinct from daemon unavailable"
    );

    let worktree = dunce::canonicalize(worktree).expect("canonical worktree");
    let worktree_text = worktree.to_str().expect("utf-8 worktree");
    assert!(run_workspace(home.path(), &["allow", worktree_text]).0);
    // `register` needs no idempotency allowance: a timed-out-but-processed
    // request retries into `WorktreeRegistration::Refreshed`
    // ("Refreshed … (already registered)"), which `live_wire_ok` below already
    // accepts. Only `unregister` changes wording across that boundary.
    let (registered, registration_stdout, _) =
        run_workspace_awaiting_daemon(home.path(), &["register", worktree_text, "--persist"]);
    // `workspace register` exits 0 even when the live daemon outcome is a
    // refusal: `--persist` records intent independently (ACTMO-019).
    assert!(
        registered,
        "register command should exit 0 (persist is independent of live outcome): {registration_stdout}"
    );

    // Separate the two refusal families before asserting on copy. A transport
    // timeout means the daemon never answered, so none of the durable-gate
    // wording below can be present — the phrases at `registration.rs:91/271/
    // 292/328` all sit on paths where the daemon *did* acknowledge. Without
    // this line a timeout fails the durable-gate assertion instead, which
    // reads as a wording defect and sends the next reader after the wrong bug.
    assert!(
        !registration_stdout.contains(DAEMON_TRANSPORT_TIMEOUT),
        "daemon round-trip still timing out after retries — a transport failure, \
         not the durable-gate refusal asserted below: {registration_stdout}"
    );

    let live_wire_ok =
        registration_stdout.contains("Registered") || registration_stdout.contains("Refreshed");
    if !live_wire_ok {
        // CIB-150 / CIB-160: the daemon fails closed on wire durable claims when
        // it cannot prove peer-exe identity (gVisor-style `/proc/<pid>/exe`
        // aliasing; issue #3130). Register is then downgraded to a live lease
        // and the CLI honestly refuses "Registered". Durable membership is
        // still available via in-process `register_on_start` on daemon restart
        // — never via the wire dispatcher. Assert that path instead of
        // requiring a faithful peer-exe sandbox on every CI runner.
        assert!(
            registration_stdout.contains("durable membership")
                && (registration_stdout.contains("Recorded")
                    || registration_stdout.contains("register_on_start")),
            "when live wire register is refused, expect durable-gate honesty plus \
             --persist intent: {registration_stdout}"
        );
        drop(daemon);
        daemon = spawn_daemon(home.path(), &worktree);
        wait_for_available_registry(home.path());
    }

    let value = wait_for_registered_worktree(home.path(), &worktree);
    assert!(
        value["registered_worktrees"]["entries"]
            .as_array()
            .is_some_and(|entries| {
                entries
                    .iter()
                    .any(|entry| listed_path_matches(entry, &worktree))
            }),
        "registered worktree membership is preserved: {value}"
    );
    assert!(
        value["allow_entries"].as_array().is_some_and(|entries| {
            entries.iter().any(|entry| {
                listed_path_matches(&entry["path"], &worktree) && entry["live_registered"] == true
            })
        }),
        "allow entry carries its live-registration annotation: {value}"
    );
    assert_single_path_state(&value["register_on_start"], &worktree, "registered");

    // Same single-shot daemon round-trip as `register` above, asserted on the
    // same way — it would have failed identically on a slow runner, it just
    // has not lost the race yet.
    let (unregistered, unregistration_stdout, unregister_retried) =
        run_workspace_awaiting_daemon(home.path(), &["unregister", worktree_text]);
    // Unlike `register`, unregister's wording changes across a retry: if the
    // timed-out request was in fact processed, the retry finds nothing to do
    // and prints the idempotent "was not registered — nothing to do."
    // (`workspace.rs:261`) instead of "Unregistered". Accept that wording
    // **only when a retry actually happened**, so the ordinary path stays
    // strict and a genuinely silent unregister cannot pass unnoticed.
    //
    // The real proof of unregistration is the state assertion immediately
    // below (`live_registered == false`), which holds either way.
    let unregister_reported = unregistration_stdout.contains("Unregistered")
        || (unregister_retried && unregistration_stdout.contains("was not registered"));
    assert!(
        unregistered && unregister_reported,
        "live unregister succeeds without dropping persisted intent \
         (retried={unregister_retried}): {unregistration_stdout}"
    );

    let value = wait_for_unregistered_worktree(home.path(), &worktree);
    assert!(
        value["allow_entries"].as_array().is_some_and(|entries| {
            entries.iter().any(|entry| {
                listed_path_matches(&entry["path"], &worktree) && entry["live_registered"] == false
            })
        }),
        "allow entry reports that live membership was removed: {value}"
    );
    assert_single_path_state(&value["register_on_start"], &worktree, "not_registered");

    // Keep the daemon alive until the last assertion so list/unregister stay
    // against a live registry (Drop kills it).
    drop(daemon);
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

#[cfg(windows)]
#[test]
fn allow_refuses_windows_drive_relative_git_bash_shape() {
    let home = tempdir().expect("temp home");
    let mut cmd = Command::new(ANVIL_BIN);
    cmd.arg("workspace")
        .args(["allow", "D:some-repo"])
        .arg("--no-tui")
        .current_dir(home.path())
        .env("ANVIL_HOME", home.path())
        .env("HOME", home.path())
        .env("USERPROFILE", home.path())
        .env("ANVIL_DEV", "1")
        .env("ANVIL_SKIP_WELCOME", "1");
    let out = cmd.output().expect("spawn anvil workspace allow");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !out.status.success(),
        "drive-relative allow must fail: stdout={stdout} stderr={stderr}"
    );
    let combined = format!("{stdout}{stderr}");
    assert!(
        combined.contains("drive-relative"),
        "refusal must name drive-relative: {combined}"
    );
}

#[test]
fn list_flags_unresolvable_allow_entries() {
    let home = tempdir().expect("temp home");
    let missing = home.path().join("no-such-workspace-root");
    let (ok, stdout) = run_workspace(
        home.path(),
        &["allow", missing.to_str().expect("utf-8 path")],
    );
    assert!(
        ok,
        "allow of a missing path still stores the entry: {stdout}"
    );

    let (ok, stdout) = run_workspace(home.path(), &["list"]);
    assert!(ok, "list succeeds: {stdout}");
    assert!(
        stdout.contains("unresolvable"),
        "list must flag daemon-dropped entries: {stdout}"
    );
}
