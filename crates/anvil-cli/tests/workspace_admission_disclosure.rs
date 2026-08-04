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
use std::process::Command;

use tempfile::tempdir;

const ANVIL_BIN: &str = env!("CARGO_BIN_EXE_anvil");

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
        .env("ANVIL_SKIP_WELCOME", "1");
    let out = cmd.output().expect("spawn anvil workspace");
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
    )
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
