//! Issue #1108: `anvil doctor` run in a fresh directory before `git init`
//! must surface a friendly, actionable next-step instead of a hard failure.
//! The `git-repo` check now reports Warn rather than Fail in that case, so
//! doctor exits 0 and the user sees `run: git init` inline.

use std::process::Command;

const ANVIL_BIN: &str = env!("CARGO_BIN_EXE_anvil");

#[test]
fn doctor_in_dir_without_git_repo_exits_zero_with_guidance() {
    let dir = tempfile::tempdir().unwrap();

    let output = Command::new(ANVIL_BIN)
        .arg("--no-tui")
        .arg("doctor")
        .current_dir(dir.path())
        // `HOME` is overridden so a developer's parent-directory git repo
        // (or any ancestor `.git`) cannot satisfy `git rev-parse --git-dir`
        // and accidentally make the check Pass on a CI runner.
        .env("HOME", dir.path())
        .env("GIT_CEILING_DIRECTORIES", dir.path())
        .output()
        .expect("failed to invoke anvil binary");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "doctor must exit 0 when only the git-repo check is missing\nstatus: {:?}\nstdout: {stdout}\nstderr: {stderr}",
        output.status,
    );

    assert!(
        stdout.contains("git-repo"),
        "expected the git-repo check to appear in stdout, got:\n{stdout}",
    );
    assert!(
        stdout.contains("git init"),
        "expected `git init` guidance in stdout so the user knows the next step, got:\n{stdout}",
    );
    // Per issue #1108 the friendly path is a warning, not a failure: the
    // run-summary tally must reflect that.
    assert!(
        !stdout.contains("0 warnings"),
        "expected at least one warning in the doctor summary, got:\n{stdout}",
    );
}
