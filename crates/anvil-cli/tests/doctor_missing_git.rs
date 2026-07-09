//! Issue #1108: `anvil doctor` run in a fresh directory before `git init`
//! must surface a friendly, actionable next-step instead of a hard failure.
//! The `git-repo` check now reports Warn rather than Fail in that case, so
//! doctor exits 0 and the user sees `run: git init` inline.

use std::process::Command;

const ANVIL_BIN: &str = env!("CARGO_BIN_EXE_anvil");

/// Ceiling for git repo discovery. Git only honours a ceiling that is a
/// proper ancestor of the probed directory, so this must be the tempdir's
/// parent — passing the tempdir itself is silently ignored and the walk
/// would still find a stray ancestor repo (e.g. a leftover /tmp/.git).
fn discovery_ceiling(dir: &std::path::Path) -> &std::path::Path {
    dir.parent().expect("tempdir has a parent")
}

/// Plain (`--no-tui`) output: doctor must exit 0 and surface the
/// `git init` next-step inline so a fresh user reading the human-facing
/// output knows what to do.
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
        .env("GIT_CEILING_DIRECTORIES", discovery_ceiling(dir.path()))
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
}

/// Structural (`--json`) assertion: the `git-repo` check must serialise
/// as `status: "warn"` with a `git init` remediation command, and
/// doctor must exit 0. Asserting the schema directly (rather than
/// scraping the human summary line) keeps the test stable if the
/// plain-output formatting ever changes — see PR #1114 review.
#[test]
fn doctor_json_in_dir_without_git_repo_reports_warn_with_remediation() {
    let dir = tempfile::tempdir().unwrap();

    let output = Command::new(ANVIL_BIN)
        .arg("--json")
        .arg("doctor")
        .current_dir(dir.path())
        .env("HOME", dir.path())
        .env("GIT_CEILING_DIRECTORIES", discovery_ceiling(dir.path()))
        .output()
        .expect("failed to invoke anvil binary");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "doctor --json must exit 0 when only the git-repo check is missing\nstatus: {:?}\nstdout: {stdout}\nstderr: {stderr}",
        output.status,
    );

    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("doctor --json stdout must parse as JSON");

    let checks = parsed
        .get("checks")
        .and_then(|v| v.as_array())
        .expect("doctor --json output must carry a `checks` array");

    let git_repo = checks
        .iter()
        .find(|c| c.get("name").and_then(|n| n.as_str()) == Some("git-repo"))
        .expect("`git-repo` check must appear in doctor --json output");

    assert_eq!(
        git_repo.get("status").and_then(|s| s.as_str()),
        Some("warn"),
        "git-repo must serialise as `warn` outside a repo, got: {git_repo}",
    );
    assert_eq!(
        git_repo
            .get("remediation")
            .and_then(|r| r.get("command"))
            .and_then(|c| c.as_str()),
        Some("git init"),
        "git-repo remediation must surface the `git init` command, got: {git_repo}",
    );

    // No `git-repo` entry should be classified as a failure — exactly
    // the regression #1108 was about.
    let failed_count = checks
        .iter()
        .filter(|c| c.get("status").and_then(|s| s.as_str()) == Some("fail"))
        .filter(|c| c.get("name").and_then(|n| n.as_str()) == Some("git-repo"))
        .count();
    assert_eq!(
        failed_count, 0,
        "git-repo must never be `fail` outside a repo"
    );
}
