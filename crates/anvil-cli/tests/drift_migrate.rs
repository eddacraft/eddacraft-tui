//! CIB-088: `anvil drift migrate` reports partial runs and prunes backups safely.

use std::path::{Path, PathBuf};
use std::process::Command;

use tempfile::tempdir;

const ANVIL_BIN: &str = env!("CARGO_BIN_EXE_anvil");

fn snapshots_dir(root: &Path) -> PathBuf {
    root.join(".anvil").join("snapshots")
}

fn write_snapshot(root: &Path, name: &str, schema_version: &str) -> PathBuf {
    let dir = snapshots_dir(root);
    std::fs::create_dir_all(&dir).expect("mkdir snapshots");
    let path = dir.join(format!("snapshot-{name}.json"));
    let json = serde_json::json!({
        "schema_version": schema_version,
        "created_at": "2026-06-26T00:00:00Z",
        "name": name,
        "metrics": {
            "boundary_violations": 0,
            "antipattern_count": 0,
            "suppression_count": 0,
            "expired_suppressions": 0,
            "files_analysed": 0
        },
        "violations": [],
        "antipatterns": [],
        "suppressions": []
    });
    std::fs::write(&path, serde_json::to_string_pretty(&json).expect("json")).expect("snapshot");
    path
}

fn run_anvil(root: &Path, args: &[&str]) -> std::process::Output {
    let mut cmd = Command::new(ANVIL_BIN);
    cmd.args(args)
        .current_dir(root)
        .env("ANVIL_HOME", root.join("anvil-home"))
        .env("HOME", root)
        .env("USERPROFILE", root)
        .env("ANVIL_DEV", "1")
        .env("ANVIL_SKIP_WELCOME", "1");
    cmd.env_remove("ANVIL_TOUCH_PROJECT_STATE");
    cmd.output().expect("spawn anvil")
}

#[test]
fn json_partial_migrate_emits_valid_json_and_exits_nonzero() {
    let root = tempdir().expect("workspace");
    let dir = snapshots_dir(root.path());
    std::fs::create_dir_all(&dir).expect("mkdir snapshots");
    std::fs::write(dir.join("snapshot-bad.json"), "{not-json").expect("bad snapshot");

    let output = run_anvil(
        root.path(),
        &["--json", "--touch-project-state", "drift", "migrate"],
    );

    assert_eq!(
        output.status.code(),
        Some(1),
        "partial migration should exit 1\nstdout={}\nstderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let payload: serde_json::Value =
        serde_json::from_str(&stdout).unwrap_or_else(|err| panic!("valid JSON: {err}: {stdout}"));
    assert_eq!(payload["partial"], true);
    assert_eq!(payload["skipped"], 1);
    assert_eq!(payload["skipped_by_reason"]["invalid_json"], 1);
    assert!(
        !stdout.contains("warning:"),
        "JSON stdout should be machine-clean: {stdout}"
    );
}

#[test]
fn json_clean_migrate_exits_zero() {
    let root = tempdir().expect("workspace");
    write_snapshot(root.path(), "old", "1.0.0");

    let output = run_anvil(
        root.path(),
        &["--json", "--touch-project-state", "drift", "migrate"],
    );

    assert!(
        output.status.success(),
        "clean migration should exit 0\nstdout={}\nstderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let payload: serde_json::Value = serde_json::from_slice(&output.stdout).expect("json");
    assert_eq!(payload["partial"], false);
    assert_eq!(payload["migrated"], 1);
    assert_eq!(payload["skipped"], 0);
}

#[test]
fn plain_partial_migrate_warns_without_duplicate_generic_error() {
    let root = tempdir().expect("workspace");
    let dir = snapshots_dir(root.path());
    std::fs::create_dir_all(&dir).expect("mkdir snapshots");
    std::fs::write(dir.join("snapshot-bad.json"), "{not-json").expect("bad snapshot");

    let output = run_anvil(
        root.path(),
        &["--no-tui", "--touch-project-state", "drift", "migrate"],
    );

    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stdout.contains("Partial migration"),
        "plain output should explain partial migration: {stdout}"
    );
    assert!(
        !stderr.contains("Error:"),
        "AlreadyReported path must not add a duplicate generic error: {stderr}"
    );
}

#[test]
fn json_partial_migrate_reports_write_failed_without_aborting_prior_work() {
    let root = tempdir().expect("workspace");
    let good = write_snapshot(root.path(), "good", "1.0.0");
    let first = run_anvil(
        root.path(),
        &["--json", "--touch-project-state", "drift", "migrate"],
    );
    assert!(
        first.status.success(),
        "initial migrate should succeed\nstdout={}\nstderr={}",
        String::from_utf8_lossy(&first.stdout),
        String::from_utf8_lossy(&first.stderr)
    );

    let late = write_snapshot(root.path(), "late", "1.0.0");
    let late_before = std::fs::read_to_string(&late).expect("late before");
    let snapshots = snapshots_dir(root.path());
    let mut perms = std::fs::metadata(&snapshots).expect("meta").permissions();
    perms.set_readonly(true);
    std::fs::set_permissions(&snapshots, perms).expect("chmod snapshots");

    let output = run_anvil(
        root.path(),
        &["--json", "--touch-project-state", "drift", "migrate"],
    );

    assert_eq!(
        output.status.code(),
        Some(1),
        "write failure should yield partial exit 1\nstdout={}\nstderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let payload: serde_json::Value =
        serde_json::from_str(&stdout).unwrap_or_else(|err| panic!("valid JSON: {err}: {stdout}"));
    assert_eq!(payload["partial"], true);
    assert_eq!(payload["migrated"], 0);
    assert_eq!(payload["skipped"], 1);
    assert_eq!(payload["skipped_by_reason"]["write_failed"], 1);

    let good_after: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&good).expect("good snapshot"))
            .expect("json");
    assert_ne!(
        good_after["schema_version"], "1.0.0",
        "prior migrated baseline must stay upgraded: {good_after}"
    );
    assert_eq!(
        std::fs::read_to_string(&late).expect("late snapshot"),
        late_before,
        "blocked baseline must remain untouched"
    );
}

#[test]
fn prune_backups_cli_removes_only_eligible_files() {
    let root = tempdir().expect("workspace");
    let live = write_snapshot(root.path(), "prune", "1.1.0");
    let base = live.with_file_name("snapshot-prune.json.bak");
    let bak1 = live.with_file_name("snapshot-prune.json.bak.1");
    let bak2 = live.with_file_name("snapshot-prune.json.bak.2");
    let unrelated = live.with_file_name("snapshot-prune.json.bak.tmp");
    std::fs::write(&base, "bak0").expect("bak0");
    std::fs::write(&bak1, "bak1").expect("bak1");
    std::fs::write(&bak2, "bak2").expect("bak2");
    std::fs::write(&unrelated, "tmp").expect("tmp");

    let output = run_anvil(
        root.path(),
        &[
            "--no-tui",
            "--touch-project-state",
            "drift",
            "migrate",
            "--prune-backups",
        ],
    );

    assert!(
        output.status.success(),
        "prune should exit 0\nstdout={}\nstderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(live.exists(), "live snapshot must survive");
    assert!(!base.exists(), "oldest backup pruned");
    assert!(!bak1.exists(), "middle backup pruned");
    assert_eq!(std::fs::read_to_string(&bak2).expect("latest"), "bak2");
    assert_eq!(
        std::fs::read_to_string(&unrelated).expect("unrelated"),
        "tmp"
    );
}
