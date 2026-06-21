//! USAGE-003: dev-investment query views, proven end-to-end against the
//! built `anvil` binary.
//!
//! Each test seeds a fixture usage sidecar under a hermetic `ANVIL_HOME`,
//! runs a real `anvil kindling usage <view> --json` invocation, and
//! asserts the view returns the expected (non-empty) result — the
//! module's validation contract ("a smoke test for each canned view
//! returning a non-empty result against a fixture Kindling state").
//!
//! Note: the producer is wired uniformly in `main`, so running
//! `kindling usage …` itself appends one `kindling` row to the sidecar
//! *before* the view reads it. Assertions therefore check the seeded
//! commands are present (and ranked), tolerating that extra self-row
//! rather than asserting exact totals.

use std::path::{Path, PathBuf};
use std::process::Command;

use tempfile::tempdir;

const ANVIL_BIN: &str = env!("CARGO_BIN_EXE_anvil");
// Retention trims rows older than seven days before each append; keep
// fixtures stable instead of tied to the wall clock.
const FIXTURE_TS_0: &str = "2099-06-14T10:00:00Z";
const FIXTURE_TS_1: &str = "2099-06-14T10:01:00Z";
const FIXTURE_TS_2: &str = "2099-06-14T10:02:00Z";

fn usage_log(anvil_home: &Path) -> PathBuf {
    anvil_home
        .join("user")
        .join("kindling")
        .join("usage.ndjson")
}

/// Seed a usage sidecar with `lines` (already-serialised NDJSON rows).
fn seed_usage_log(home: &Path, lines: &[&str]) {
    let path = usage_log(home);
    std::fs::create_dir_all(path.parent().expect("parent")).expect("create kindling dir");
    std::fs::write(&path, format!("{}\n", lines.join("\n"))).expect("seed usage log");
}

/// Build a minimal valid `command.invoked` NDJSON row.
fn row(command: &str, principal: &str, ts: &str) -> String {
    format!(
        r#"{{"kind":"command.invoked","session_id":"s","timestamp":"{ts}","command":"{command}","principal":"{principal}","args":[],"flag_set":[]}}"#
    )
}

/// Run `anvil <args>` under a hermetic `ANVIL_HOME` and return stdout.
fn run_anvil_stdout(home: &Path, args: &[&str]) -> String {
    let mut cmd = Command::new(ANVIL_BIN);
    cmd.args(args)
        .current_dir(home)
        .env("ANVIL_HOME", home)
        .env("HOME", home)
        .env("USERPROFILE", home)
        .env("ANVIL_DEV", "1")
        .env("ANVIL_SKIP_WELCOME", "1");
    cmd.env_remove("ANVIL_TOUCH_PROJECT_STATE");
    cmd.env_remove("TRACEPARENT");
    let out = cmd.output().expect("spawn anvil");
    assert!(
        out.status.success(),
        "anvil {args:?} exited {:?}\nstdout: {}\nstderr: {}",
        out.status,
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
    String::from_utf8_lossy(&out.stdout).into_owned()
}

#[test]
fn top_view_ranks_seeded_commands() {
    let home = tempdir().expect("home");
    seed_usage_log(
        home.path(),
        &[
            &row("check", "p1", FIXTURE_TS_0),
            &row("check", "p1", FIXTURE_TS_1),
            &row("status", "p2", FIXTURE_TS_2),
        ],
    );

    let stdout = run_anvil_stdout(home.path(), &["kindling", "usage", "top", "--json"]);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("valid JSON view output");
    let arr = parsed.as_array().expect("top is an array");

    // `check` (2) must rank first and ahead of `status` (1).
    let check = arr
        .iter()
        .find(|e| e["command"] == "check")
        .expect("check present");
    assert_eq!(check["count"], 2);
    assert_eq!(arr[0]["command"], "check", "most-invoked ranks first");
    assert!(
        arr.iter().any(|e| e["command"] == "status"),
        "status present: {stdout}"
    );
}

#[test]
fn unused_view_lists_never_invoked_commands() {
    let home = tempdir().expect("home");
    // Seed only `version`; many registered commands remain unused.
    seed_usage_log(home.path(), &[&row("version", "p1", FIXTURE_TS_0)]);

    let stdout = run_anvil_stdout(home.path(), &["kindling", "usage", "unused", "--json"]);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("valid JSON");
    let arr = parsed.as_array().expect("unused is an array");

    assert!(!arr.is_empty(), "some registered commands are unused");
    assert!(
        arr.iter().any(|c| c == "audit"),
        "a known-unused command (audit) is listed: {stdout}"
    );
    assert!(
        !arr.iter().any(|c| c == "version"),
        "version was invoked, so it must not be listed as unused: {stdout}"
    );
}

#[test]
fn flags_view_reports_observed_flag_paths() {
    let home = tempdir().expect("home");
    let gated = format!(
        r#"{{"kind":"command.invoked","session_id":"s","timestamp":"{FIXTURE_TS_0}","command":"status","principal":"p1","args":[],"flag_set":[{{"key":"cli.licence-gate","variant":"enabled","source":"override","gate_affecting":true}}]}}"#
    );
    seed_usage_log(home.path(), &[&gated]);

    let stdout = run_anvil_stdout(home.path(), &["kindling", "usage", "flags", "--json"]);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("valid JSON");
    let arr = parsed.as_array().expect("flags is an array");

    let gate = arr
        .iter()
        .find(|f| f["key"] == "cli.licence-gate")
        .expect("licence-gate observed");
    assert_eq!(gate["gate_affecting"], true);
    assert!(gate["invocations"].as_u64().expect("count") >= 1);
}

#[test]
fn principals_view_ranks_by_activity() {
    let home = tempdir().expect("home");
    seed_usage_log(
        home.path(),
        &[
            &row("check", "alice", FIXTURE_TS_0),
            &row("status", "alice", FIXTURE_TS_1),
            &row("version", "bob", FIXTURE_TS_2),
        ],
    );

    let stdout = run_anvil_stdout(home.path(), &["kindling", "usage", "principals", "--json"]);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("valid JSON");
    let arr = parsed.as_array().expect("principals is an array");

    let alice = arr
        .iter()
        .find(|p| p["principal"] == "alice")
        .expect("alice present");
    assert_eq!(alice["invocations"], 2);
    assert_eq!(arr[0]["principal"], "alice", "most active ranks first");
}

#[test]
fn views_handle_empty_log_cleanly() {
    let home = tempdir().expect("home");
    // No seeding: the sidecar does not exist until the run creates it.
    let stdout = run_anvil_stdout(home.path(), &["kindling", "usage", "top", "--json"]);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("valid JSON for empty log");
    // The view's own invocation may add a `kindling` row before it reads,
    // so the result is a (possibly single-entry) array, never an error.
    assert!(parsed.is_array(), "empty-log view still returns an array");
}
