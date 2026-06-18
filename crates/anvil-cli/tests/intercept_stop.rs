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
    let home = tempfile::tempdir().expect("tempdir");
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
