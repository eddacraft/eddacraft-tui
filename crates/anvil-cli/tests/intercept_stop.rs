//! V060F-002: `anvil intercept stop` operator surface.
//!
//! Verifies the idempotent no-daemon paths against an isolated
//! `ANVIL_HOME` (so the PID file resolves under the per-test temp tree
//! and the test never touches a developer's real daemon): a missing PID
//! file reports "not running", and a PID file pointing at a dead process
//! is cleared. The live-signal branch is unit-tested in
//! `anvil_intercept`'s `plan_stop`; signalling a real daemon from CI
//! would be flaky, so it is not exercised here.

#![cfg(unix)]

use std::fs;
use std::path::Path;
use std::process::{Command, Output};

const ANVIL_BIN: &str = env!("CARGO_BIN_EXE_anvil");

fn stop_in_home(home: &Path) -> Output {
    Command::new(ANVIL_BIN)
        .args(["intercept", "stop"])
        // ANVIL_HOME re-roots the PID file under the temp tree (the
        // daemon and CLI agree on `$ANVIL_HOME/intercept.pid`); HOME /
        // XDG_RUNTIME_DIR keep every other user-state probe off the real
        // home. ANVIL_DEV bypasses the beta licence gate.
        .env("HOME", home)
        .env("USERPROFILE", home)
        .env("XDG_RUNTIME_DIR", home)
        .env("ANVIL_HOME", home)
        .env("ANVIL_DEV", "1")
        .env("ANVIL_SKIP_WELCOME", "1")
        .output()
        .expect("run anvil intercept stop")
}

fn stop_with_unresolved_sibling(root: &Path, json: bool) -> Output {
    use std::os::unix::fs::PermissionsExt;

    let runtime = root.join("runtime");
    let canonical_dir = runtime.join("anvil");
    let home = root.join("home");
    let sibling_dir = home.join(".local/state/anvil");
    fs::create_dir_all(&canonical_dir).expect("canonical runtime dir");
    fs::create_dir_all(&sibling_dir).expect("sibling runtime dir");
    for dir in [&canonical_dir, &sibling_dir] {
        fs::set_permissions(dir, fs::Permissions::from_mode(0o700))
            .expect("owner-only runtime dir");
    }
    fs::write(sibling_dir.join("intercept.pid"), "not-a-pid\n")
        .expect("malformed sibling PID file");

    let mut command = Command::new(ANVIL_BIN);
    if json {
        command.arg("--json");
    }
    command
        .args(["intercept", "stop"])
        .env("HOME", &home)
        .env("USERPROFILE", &home)
        .env("XDG_RUNTIME_DIR", &runtime)
        .env_remove("ANVIL_HOME")
        .env("ANVIL_DEV", "1")
        .env("ANVIL_SKIP_WELCOME", "1")
        .output()
        .expect("run split-candidate intercept stop")
}

fn stop_with_writable_canonical_pid(root: &Path) -> Output {
    use std::os::unix::fs::PermissionsExt;

    let runtime = root.join("runtime");
    let canonical_dir = runtime.join("anvil");
    let home = root.join("home");
    let sibling_dir = home.join(".local/state/anvil");
    fs::create_dir_all(&canonical_dir).expect("canonical runtime dir");
    fs::create_dir_all(&sibling_dir).expect("sibling runtime dir");
    for dir in [&canonical_dir, &sibling_dir] {
        fs::set_permissions(dir, fs::Permissions::from_mode(0o700))
            .expect("owner-only runtime dir");
    }
    let canonical_pid = canonical_dir.join("intercept.pid");
    fs::write(&canonical_pid, "2147483646\nstart_time=1\n").expect("write canonical PID file");
    fs::set_permissions(&canonical_pid, fs::Permissions::from_mode(0o620))
        .expect("make canonical PID file group-writable");

    Command::new(ANVIL_BIN)
        .args(["--json", "intercept", "stop"])
        .env("HOME", &home)
        .env("USERPROFILE", &home)
        .env("XDG_RUNTIME_DIR", &runtime)
        .env_remove("ANVIL_HOME")
        .env("ANVIL_DEV", "1")
        .env("ANVIL_SKIP_WELCOME", "1")
        .output()
        .expect("run stop with writable canonical PID file")
}

#[test]
fn stop_with_no_daemon_reports_not_running() {
    let home = tempfile::tempdir().expect("tempdir");
    let out = stop_in_home(home.path());
    assert!(
        out.status.success(),
        "expected exit 0, got {:?}\nstderr: {}",
        out.status,
        String::from_utf8_lossy(&out.stderr),
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("not running"), "stdout was: {stdout}");
}

#[test]
fn stop_clears_a_stale_pid_file() {
    use std::os::unix::fs::PermissionsExt;

    let home = tempfile::tempdir().expect("tempdir");
    // A real `ANVIL_HOME` holding a PID file is owner-only: the daemon's create
    // path tightens the prefix to 0700 before it writes one. A bare tempdir
    // inherits the umask (775 on many boxes), and the stop path refuses to act
    // on a PID file in a group-writable directory because another user could
    // have planted it. Match production rather than the umask.
    fs::set_permissions(home.path(), fs::Permissions::from_mode(0o700))
        .expect("owner-only ANVIL_HOME");
    let pid_file = home.path().join("intercept.pid");
    // A PID far above any plausible `pid_max`, so `existing_pid_status`
    // sees no such process and classifies the record Stale.
    fs::write(&pid_file, "2147483646\n").expect("write stale pid file");

    let out = stop_in_home(home.path());
    assert!(
        out.status.success(),
        "expected exit 0, got {:?}\nstderr: {}",
        out.status,
        String::from_utf8_lossy(&out.stderr),
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("stale"), "stdout was: {stdout}");
    assert!(
        !pid_file.exists(),
        "stale PID file should have been removed",
    );
}

#[test]
fn stop_with_unresolved_sibling_reports_skip_and_exits_zero() {
    let root = tempfile::tempdir().expect("tempdir");
    let out = stop_with_unresolved_sibling(root.path(), false);

    assert!(
        out.status.success(),
        "sibling PID-file record errors must not fail stop; stdout: {}; stderr: {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("skipped"), "stdout was: {stdout}");
}

#[test]
fn stop_json_reports_every_candidate_and_exits_zero_on_unresolved_sibling() {
    let root = tempfile::tempdir().expect("tempdir");
    let out = stop_with_unresolved_sibling(root.path(), true);

    assert!(
        out.status.success(),
        "JSON stop must exit 0 after reporting an unresolved sibling; stdout: {}; stderr: {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
    let document: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("stdout must be one JSON document");
    assert_eq!(document["outcome"], "not-running");
    assert_eq!(document["result"], "partial-failure");
    assert_eq!(document["partial_failure"], true);
    assert_eq!(document["candidates"].as_array().map(Vec::len), Some(2));
    assert_eq!(document["candidates"][0]["outcome"], "not-running");
    assert_eq!(document["candidates"][1]["outcome"], "unresolved");
    assert!(
        out.stderr.is_empty(),
        "JSON partial failure is already reported on stdout: {}",
        String::from_utf8_lossy(&out.stderr),
    );
}

#[test]
fn stop_json_reports_writable_canonical_pid_and_every_sibling() {
    let root = tempfile::tempdir().expect("tempdir");
    let out = stop_with_writable_canonical_pid(root.path());

    assert!(
        !out.status.success(),
        "unsafe canonical PID metadata must exit non-zero; stdout: {}; stderr: {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
    let document: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("stdout must be one JSON document");
    assert_eq!(document["outcome"], "not-running");
    assert_eq!(document["pid"], serde_json::Value::Null);
    assert_eq!(
        document["registered_losing_protection"],
        serde_json::Value::Null
    );
    assert_eq!(document["result"], "partial-failure");
    assert_eq!(document["partial_failure"], true);
    assert_eq!(document["candidates"].as_array().map(Vec::len), Some(2));
    assert_eq!(document["candidates"][0]["outcome"], "unresolved");
    assert_eq!(document["candidates"][1]["outcome"], "not-running");
    assert!(
        out.stderr.is_empty(),
        "JSON refusal is already reported on stdout: {}",
        String::from_utf8_lossy(&out.stderr),
    );
}
