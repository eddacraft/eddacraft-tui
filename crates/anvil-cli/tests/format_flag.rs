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
use std::sync::atomic::{AtomicU32, Ordering};

const ANVIL_BIN: &str = env!("CARGO_BIN_EXE_anvil");

/// A fresh, empty working directory so `anvil audit` scans nothing. The path is
/// unique per invocation (process id + monotonic sequence) so concurrent runs
/// of this integration-test binary on the same host cannot remove or recreate
/// each other's working directory.
fn temp_workdir(tag: &str) -> PathBuf {
    static SEQ: AtomicU32 = AtomicU32::new(0);
    let unique = format!(
        "{}-{}",
        std::process::id(),
        SEQ.fetch_add(1, Ordering::Relaxed)
    );
    let dir = std::env::temp_dir().join(format!("anvil-sarifout-001-{tag}-{unique}"));
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
    let doc: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap_or_else(|e| {
        panic!("`audit --format json` stdout must be valid JSON ({e}); got:\n{stdout}")
    });
    assert!(
        doc.is_object(),
        "`audit --format json` should emit a JSON object, got:\n{stdout}"
    );
}

/// SARIFOUT-005: `anvil gate --format sarif` emits a well-formed SARIF 2.1.0
/// document on stdout. Gate is exit-code-neutral under SARIF (it may still exit
/// non-zero when gates fail), so we assert on the document, not the exit code.
#[test]
fn gate_format_sarif_emits_well_formed_document() {
    let dir = temp_workdir("gate-sarif");
    let out = anvil(&dir)
        .args(["--no-tui", "gate", "--format", "sarif"])
        .output()
        .expect("failed to invoke anvil");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let doc: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap_or_else(|e| {
        panic!("`gate --format sarif` stdout must be valid JSON ({e}); got:\n{stdout}")
    });
    assert_eq!(doc["version"], "2.1.0", "SARIF version");
    assert_eq!(
        doc["runs"][0]["tool"]["driver"]["name"], "anvil",
        "tool.driver.name"
    );
    assert!(doc["runs"][0]["results"].is_array(), "results[] present");
}

/// SARIFOUT-003: `anvil check --format sarif` emits a well-formed SARIF 2.1.0
/// document on stdout (envelope + anvil driver), end to end.
#[test]
fn check_format_sarif_emits_well_formed_document() {
    let dir = temp_workdir("check-sarif");
    std::fs::write(dir.join("sample.ts"), "const x: any = 1;\n").expect("write fixture");
    let out = anvil(&dir)
        .args(["--no-tui", "check", "--all", "--format", "sarif"])
        .output()
        .expect("failed to invoke anvil");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let doc: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap_or_else(|e| {
        panic!("`check --format sarif` stdout must be valid JSON ({e}); got:\n{stdout}")
    });
    assert_eq!(doc["version"], "2.1.0", "SARIF version");
    assert_eq!(
        doc["runs"][0]["tool"]["driver"]["name"], "anvil",
        "tool.driver.name"
    );
    assert!(doc["runs"][0]["results"].is_array(), "results[] present");
}

/// SARIFOUT-004: `anvil audit --format sarif` emits a well-formed SARIF 2.1.0
/// document on stdout, end to end (audit scans the cwd, so an empty dir yields
/// an empty-but-valid document).
#[test]
fn audit_format_sarif_emits_well_formed_document() {
    let dir = temp_workdir("audit-sarif");
    let out = anvil(&dir)
        .args(["--no-tui", "audit", "--format", "sarif"])
        .output()
        .expect("failed to invoke anvil");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let doc: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap_or_else(|e| {
        panic!("`audit --format sarif` stdout must be valid JSON ({e}); got:\n{stdout}")
    });
    assert_eq!(doc["version"], "2.1.0", "SARIF version");
    assert_eq!(
        doc["runs"][0]["tool"]["driver"]["name"], "anvil",
        "tool.driver.name"
    );
    assert!(doc["runs"][0]["results"].is_array(), "results[] present");
}

/// SARIFOUT-001 / clawpatch follow-up: a finding command invoked with its own
/// `--format json` (and NO global `--json`) must be treated as machine output
/// by the pre-dispatch auth gate — it emits only the structured `authRequired`
/// envelope and no human chatter / interactive prompt when credentials are
/// missing. Mirrors `init_post_analysis::json_mode_auth_failure_emits_only_json_error`
/// for the per-command `--format` surface that this wiring fixes.
#[test]
fn format_json_auth_failure_emits_only_json_envelope() {
    let dir = temp_workdir("format-json-auth");
    let config_home = temp_workdir("format-json-auth-cfg");

    let output = Command::new(ANVIL_BIN)
        .args(["check", "--all", "--format", "json"])
        .current_dir(&dir)
        .env("ANVIL_SKIP_WELCOME", "1")
        .env("ANVIL_LOG", "off")
        .env("ANVIL_NO_PROMPT", "1")
        .env("XDG_CONFIG_HOME", &config_home)
        .env_remove("ANVIL_DEV")
        .env_remove("ANVIL_LICENSE")
        .output()
        .expect("failed to invoke anvil binary");

    // Action command: auth-required is an expected state → exit 0.
    assert!(
        output.status.success(),
        "auth-required on an action command should exit 0; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr),
    );
    assert!(
        output.stdout.is_empty(),
        "`check --format json` auth failure must not write stdout: {}",
        String::from_utf8_lossy(&output.stdout),
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    let parsed: serde_json::Value = serde_json::from_str(stderr.trim()).unwrap_or_else(|err| {
        panic!("stderr must be one JSON auth envelope, got {stderr:?}: {err}")
    });
    assert_eq!(
        parsed.get("state").and_then(|v| v.as_str()),
        Some("authRequired"),
        "`--format json` must get the structured auth envelope, not human text: {stderr}",
    );
    assert_eq!(
        stderr.trim().lines().count(),
        1,
        "stderr should be only the JSON envelope — no human chatter leaked: {stderr}",
    );
}
