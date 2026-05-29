//! SARIFOUT-001: the `--format` value-enum surface (ADR-056).
//!
//! `--format` is a per-command flag on the finding-emitting commands
//! (`check` / `gate` / `audit`) rather than a global flag, because `--format`
//! is already a domain flag on `export` / `validate`. These integration tests
//! pin the user-facing contract:
//! - `--format sarif` reaches the SARIF path on a finding command (and, until
//!   the adapters land, reports the pending state).
//! - `--format` does not exist on non-finding commands (clap rejects it).
//! - `--format json` is accepted as the documented `--json` alias.
//!
//! Resolver precedence, `--json`/`--format json` parity, and the
//! never-auto-select-SARIF invariant are unit-tested in `output/mod.rs`.

use std::path::PathBuf;
use std::process::Command;

const ANVIL_BIN: &str = env!("CARGO_BIN_EXE_anvil");

/// A fresh, empty working directory so `anvil audit` scans nothing.
fn temp_workdir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("anvil-sarifout-001-{tag}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create temp workdir");
    dir
}

fn anvil(workdir: &PathBuf) -> Command {
    let mut cmd = Command::new(ANVIL_BIN);
    cmd.current_dir(workdir)
        .env("ANVIL_DEV", "1")
        .env("ANVIL_SKIP_WELCOME", "1");
    cmd
}

#[test]
fn format_flag_is_rejected_on_non_finding_command() {
    let dir = temp_workdir("non-finding");
    let out = anvil(&dir)
        .args(["--no-tui", "drift", "list", "--format", "sarif"])
        .output()
        .expect("failed to invoke anvil");
    assert!(
        !out.status.success(),
        "`drift list --format sarif` must fail — drift has no --format flag"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("unexpected argument") && stderr.contains("--format"),
        "expected clap's unexpected-argument error for --format, got stderr:\n{stderr}"
    );
}

#[test]
fn format_json_accepted_as_alias_on_finding_command() {
    let dir = temp_workdir("json-alias");
    let out = anvil(&dir)
        .args(["--no-tui", "audit", "--format", "json"])
        .output()
        .expect("failed to invoke anvil");
    assert!(
        out.status.success(),
        "`audit --format json` should succeed; stderr:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.trim_start().starts_with('{'),
        "`audit --format json` should emit a JSON object, got:\n{stdout}"
    );
}

/// All three finding-emitting commands reach the SARIF path (and report the
/// pending state) via their own `--format sarif`. `check` and `gate` bail
/// before any file scan / check run, so this needs no project setup.
#[test]
fn format_sarif_reaches_sarif_path_on_each_finding_command() {
    for command in ["check", "gate", "audit"] {
        let dir = temp_workdir(&format!("sarif-{command}"));
        let out = anvil(&dir)
            .args(["--no-tui", command, "--format", "sarif"])
            .output()
            .expect("failed to invoke anvil");
        assert!(
            !out.status.success(),
            "`{command} --format sarif` should report not-yet-available; stdout:\n{}",
            String::from_utf8_lossy(&out.stdout)
        );
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(
            stderr.contains(&format!(
                "SARIF output for `anvil {command}` is not yet available"
            )),
            "expected the SARIF-pending message for `{command}`, got stderr:\n{stderr}"
        );
    }
}
