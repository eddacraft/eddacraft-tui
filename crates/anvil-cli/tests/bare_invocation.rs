//! ADR-114: bare `anvil` is the daily ensure surface.
//!
//! - With no project config (never activated): exit 1 + recovery naming
//!   `anvil start` / `anvil welcome` (no silent install).
//! - `--help` still lists commands and leads with the first-run pointer.
//! - Former CIB-177 contract (bare always exit 2) is superseded for ensure.
//! - ADR-145 / JSIMP-003: unsigned never-activated entry skips the licence
//!   wall and reuses the existing not-activated ensure document (exit 1).

use std::path::Path;
use std::process::Command;

const ANVIL_BIN: &str = env!("CARGO_BIN_EXE_anvil");

/// Unsigned, hermetic home so credentials cannot resolve.
fn unsigned_bare(cwd: &Path, home: &Path) -> Command {
    let mut cmd = Command::new(ANVIL_BIN);
    cmd.current_dir(cwd)
        .env("HOME", home)
        .env("USERPROFILE", home)
        .env("LOCALAPPDATA", home)
        .env("XDG_CONFIG_HOME", home.join("xdg"))
        .env("ANVIL_HOME", home.join("anvil-home"))
        .env_remove("ANVIL_DEV")
        .env_remove("ANVIL_LICENSE")
        .env("ANVIL_SKIP_WELCOME", "1")
        .env("ANVIL_NO_PROMPT", "1")
        .env("ANVIL_NO_DAEMON", "1")
        .env("ANVIL_NO_MCP", "1");
    cmd
}

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
    std::fs::write(tmp.path().join(".anvil.json"), "{}\n").expect("write config");
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

/// ADR-145 / JSIMP-003: unsigned config-Absent `anvil --json` replaces the
/// auth envelope with the existing not-activated ensure document. No new
/// fields; no writes.
#[test]
fn unsigned_never_activated_json_is_ensure_not_auth_envelope() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let home = tempfile::tempdir().expect("home");
    let out = unsigned_bare(tmp.path(), home.path())
        .arg("--json")
        .output()
        .expect("failed to invoke unsigned bare anvil --json");

    assert_eq!(
        out.status.code(),
        Some(1),
        "unsigned never-activated --json must exit 1; stdout={}\nstderr={}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );

    let stdout = String::from_utf8_lossy(&out.stdout);
    let value: serde_json::Value =
        serde_json::from_str(&stdout).unwrap_or_else(|e| panic!("json stdout: {e}\n{stdout}"));
    assert_eq!(value["surface"], "ensure");
    assert_eq!(value["config"], "absent");
    assert_ne!(
        value.get("state").and_then(|v| v.as_str()),
        Some("authRequired"),
        "must replace the auth envelope, not add pointer fields: {value}"
    );
    assert!(
        !tmp.path().join(".anvil.yaml").exists() && !tmp.path().join(".anvilrc").exists(),
        "never-activated pointer must not write project config"
    );
}

/// ADR-145 / JSIMP-003: unsigned never-activated human path skips the
/// licence wall and names start / welcome without silent install.
#[test]
fn unsigned_never_activated_human_points_at_start_and_welcome() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let home = tempfile::tempdir().expect("home");
    let out = unsigned_bare(tmp.path(), home.path())
        .output()
        .expect("failed to invoke unsigned bare anvil");

    assert_eq!(
        out.status.code(),
        Some(1),
        "unsigned never-activated bare must exit 1; stdout={}\nstderr={}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );

    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !stderr.contains("anvil auth login"),
        "must not hit the licence wall before the pointer:\n{stderr}"
    );
    assert!(
        stderr.contains("anvil start"),
        "recovery must name `anvil start`:\n{stderr}"
    );
    assert!(
        stderr.contains("anvil welcome") || stderr.contains("not activated"),
        "recovery must orient the user:\n{stderr}"
    );
    assert!(
        !tmp.path().join(".anvil.yaml").exists() && !tmp.path().join(".anvilrc").exists(),
        "never-activated pointer must not write project config"
    );
}

/// ADR-145 / JSIMP-003: unsigned but already-activated ensure stays entitled.
#[test]
fn unsigned_config_present_json_stays_auth_envelope() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let home = tempfile::tempdir().expect("home");
    std::fs::write(tmp.path().join(".anvil.json"), "{}\n").expect("write config");
    let out = unsigned_bare(tmp.path(), home.path())
        .arg("--json")
        .output()
        .expect("failed to invoke unsigned activated anvil --json");

    assert_eq!(
        out.status.code(),
        Some(3),
        "unsigned activated --json must stay exit 3; stdout={}\nstderr={}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );

    let stdout = String::from_utf8_lossy(&out.stdout);
    let value: serde_json::Value =
        serde_json::from_str(&stdout).unwrap_or_else(|e| panic!("json stdout: {e}\n{stdout}"));
    assert_eq!(
        value.get("state").and_then(|v| v.as_str()),
        Some("authRequired"),
        "activated unsigned ensure keeps the auth envelope: {value}"
    );
}
