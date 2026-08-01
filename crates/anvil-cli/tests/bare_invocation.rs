//! ADR-114: bare `anvil` is the daily ensure surface.
//!
//! - With no project config (never activated): exit 1 + recovery naming
//!   `anvil start` / `anvil welcome` (no silent install).
//! - `--help` still lists commands and leads with the first-run pointer.
//! - Former CIB-177 contract (bare always exit 2) is superseded for ensure.

use std::process::Command;

const ANVIL_BIN: &str = env!("CARGO_BIN_EXE_anvil");

#[test]
fn bare_anvil_help_leads_with_first_run_pointer() {
    let out = Command::new(ANVIL_BIN)
        .args(["--help"])
        .env("ANVIL_DEV", "1")
        .env("ANVIL_SKIP_WELCOME", "1")
        .output()
        .expect("failed to invoke anvil --help");

    assert_eq!(
        out.status.code(),
        Some(0),
        "anvil --help must exit 0; stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );

    let stdout = String::from_utf8_lossy(&out.stdout);
    let welcome = stdout
        .find("anvil welcome")
        .unwrap_or_else(|| panic!("help names `anvil welcome`:\n{stdout}"));
    let start = stdout
        .find("anvil start")
        .unwrap_or_else(|| panic!("help names `anvil start`:\n{stdout}"));
    let commands = stdout
        .find("Commands:")
        .unwrap_or_else(|| panic!("help lists the commands:\n{stdout}"));

    assert!(
        welcome < commands && start < commands,
        "first-run pointer must lead before the command list:\n{stdout}"
    );
    assert!(
        stdout.contains("EXIT CODES:"),
        "exit-codes footer must be preserved:\n{stdout}"
    );
}

#[test]
fn bare_anvil_not_activated_exits_1_with_recovery() {
    let tmp = tempfile::tempdir().expect("tempdir");
    // Fresh directory with no anvil config — config Absent.
    let out = Command::new(ANVIL_BIN)
        .current_dir(tmp.path())
        .env("ANVIL_DEV", "1")
        .env("ANVIL_SKIP_WELCOME", "1")
        .env("ANVIL_NO_DAEMON", "1")
        .env("ANVIL_NO_MCP", "1")
        .output()
        .expect("failed to invoke bare anvil");

    assert_eq!(
        out.status.code(),
        Some(1),
        "never-activated bare ensure must exit 1; stdout={}\nstderr={}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );

    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("anvil start"),
        "recovery must name `anvil start`:\n{stderr}"
    );
    assert!(
        stderr.contains("anvil welcome") || stderr.contains("not activated"),
        "recovery must orient the user:\n{stderr}"
    );
}

#[test]
fn bare_anvil_json_not_activated_is_structured() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let out = Command::new(ANVIL_BIN)
        .arg("--json")
        .current_dir(tmp.path())
        .env("ANVIL_DEV", "1")
        .env("ANVIL_SKIP_WELCOME", "1")
        .env("ANVIL_NO_DAEMON", "1")
        .env("ANVIL_NO_MCP", "1")
        .output()
        .expect("failed to invoke bare anvil --json");

    assert_eq!(
        out.status.code(),
        Some(1),
        "never-activated --json ensure must exit 1; stdout={}\nstderr={}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );

    let stdout = String::from_utf8_lossy(&out.stdout);
    let value: serde_json::Value =
        serde_json::from_str(&stdout).unwrap_or_else(|e| panic!("json stdout: {e}\n{stdout}"));
    assert_eq!(value["surface"], "ensure");
    assert_eq!(value["config"], "absent");
}

#[test]
fn bare_anvil_with_config_outside_git_refuses_worktree() {
    let tmp = tempfile::tempdir().expect("tempdir");
    // Config present but not a git worktree → refuse (worktree validation gate).
    std::fs::write(tmp.path().join(".anvilrc"), "{}\n").expect("write config");
    let out = Command::new(ANVIL_BIN)
        .current_dir(tmp.path())
        .env("ANVIL_DEV", "1")
        .env("ANVIL_SKIP_WELCOME", "1")
        .env("ANVIL_NO_DAEMON", "1")
        .env("ANVIL_NO_MCP", "1")
        .output()
        .expect("failed to invoke bare anvil");

    assert_eq!(
        out.status.code(),
        Some(1),
        "non-worktree ensure must exit 1; stdout={}\nstderr={}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("worktree") || stderr.contains("registerable"),
        "must mention worktree refusal:\n{stderr}"
    );
}
