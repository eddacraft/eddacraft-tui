//! LAUNCH-006: `anvil start` activation entrypoint integration tests.
//!
//! `anvil start` is promoted from a clap alias for `welcome` to its own
//! command that drives the activation orchestration: init if absent,
//! first-scan, and `activation::verify`. The acceptance contract is:
//!
//! - On a fresh temp repo, `anvil start` exits 0 with one literal
//!   `ProtectionState` in stdout (typically `needs_action` until
//!   LAUNCH-009 wires safe MCP install).
//! - `anvil welcome` still runs unchanged.
//! - Idempotent reruns skip init.
//! - `--verify` is a read-only probe (mirrors `anvil status --verify`).
//! - `--json` emits a state literal in the same shape as `anvil status
//!   --verify --json` (LAUNCH-012).
//!
//! Council-locked truthfulness: a fresh repo MUST NEVER claim
//! `protecting`. LAUNCH-009 / LAUNCH-011 land the safe paths to that
//! state.

use std::fs;
use std::process::Command;

const ANVIL_BIN: &str = env!("CARGO_BIN_EXE_anvil");

fn run_start(workdir: &std::path::Path, extra_args: &[&str]) -> std::process::Output {
    let mut cmd = Command::new(ANVIL_BIN);
    cmd.arg("--no-tui")
        .arg("start")
        .args(extra_args)
        .current_dir(workdir)
        .env("ANVIL_DEV", "1")
        .env("ANVIL_SKIP_WELCOME", "1");
    cmd.output().expect("failed to invoke anvil binary")
}

#[test]
fn start_on_fresh_repo_runs_init_and_lands_needs_action() {
    // #1280 review: don't assert on user-facing copy — those strings are
    // owned by LAUNCH-014 and other UX work. Use stable filesystem
    // signals (.anvilrc existence) and the structured `state:` line.
    let dir = tempfile::tempdir().unwrap();
    assert!(
        !dir.path().join(".anvilrc").exists(),
        "pre-condition: fresh temp repo has no .anvilrc"
    );

    let out = run_start(dir.path(), &[]);
    assert!(
        out.status.success(),
        "anvil start failed: stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );

    // Init ran — the only stable proof is .anvilrc on disk.
    assert!(
        dir.path().join(".anvilrc").exists(),
        ".anvilrc must exist after `anvil start` on a fresh repo"
    );

    // Activation diagnostic emitted one literal final state.
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("state: needs_action"),
        "expected `state: needs_action` on fresh repo, got:\n{stdout}"
    );
    // Truthfulness guardrail.
    assert!(
        !stdout.contains("state: protecting"),
        "fresh repo MUST NOT claim protection, got:\n{stdout}"
    );
}

#[test]
fn start_idempotent_rerun_skips_init() {
    // #1280 review: prove idempotency by checking .anvilrc mtime
    // (filesystem-stable) instead of asserting on the absence of a
    // user-facing init banner.
    let dir = tempfile::tempdir().unwrap();
    let first = run_start(dir.path(), &[]);
    assert!(first.status.success());

    let mtime_before = std::fs::metadata(dir.path().join(".anvilrc"))
        .unwrap()
        .modified()
        .unwrap();

    // Sleep past one-second mtime granularity so any rewrite would be
    // detectable on filesystems with HFS+-style coarse mtimes.
    std::thread::sleep(std::time::Duration::from_millis(1100));
    let second = run_start(dir.path(), &[]);
    assert!(
        second.status.success(),
        "second start failed: stderr={}",
        String::from_utf8_lossy(&second.stderr)
    );

    let mtime_after = std::fs::metadata(dir.path().join(".anvilrc"))
        .unwrap()
        .modified()
        .unwrap();
    assert_eq!(
        mtime_before, mtime_after,
        "second start must not rewrite .anvilrc (idempotent rerun)"
    );

    // Diagnostic still emitted on the second run.
    let stdout = String::from_utf8_lossy(&second.stdout);
    assert!(
        stdout.contains("state: needs_action"),
        "second start must still emit the diagnostic, got:\n{stdout}"
    );
}

#[test]
fn start_verify_on_fresh_repo_reports_needs_action() {
    // #1280 review: tighten the contract. `activation::verify` maps
    // ConfigStatus::Absent → ProtectionState::NeedsAction (see
    // diagnostic.rs:protection_state). Asserting either error or
    // needs_action would hide a regression in that mapping.
    let dir = tempfile::tempdir().unwrap();
    let out = run_start(dir.path(), &["--verify"]);
    assert!(
        out.status.success(),
        "anvil start --verify failed: stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );

    // --verify is read-only: .anvilrc must NOT be written.
    assert!(
        !dir.path().join(".anvilrc").exists(),
        "--verify must not write .anvilrc"
    );

    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("state: needs_action"),
        "fresh-repo --verify should report needs_action (config absent → NeedsAction), got:\n{stdout}"
    );
    assert!(
        stdout.contains("config: absent"),
        "config status should be reported as absent, got:\n{stdout}"
    );
}

#[test]
fn start_json_emits_state_literal_in_status_verify_shape() {
    let dir = tempfile::tempdir().unwrap();
    let out = run_start(dir.path(), &["--json"]);
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);

    // Parse and assert structural fields match the LAUNCH-012 shape.
    let json: serde_json::Value =
        serde_json::from_str(&stdout).expect("--json output must be valid JSON");
    assert!(json["state"].is_string(), "state must be present");
    let state = json["state"].as_str().unwrap();
    assert!(
        [
            "protecting",
            "ready_restart_required",
            "watching",
            "needs_action",
            "unsupported",
            "error"
        ]
        .contains(&state),
        "state must be a known ProtectionState literal, got {state}"
    );
    assert!(json["headline"].is_string());
    assert!(json["config"].is_string());
}

#[test]
fn welcome_still_runs_after_start_promotion() {
    // #1280 review: don't assert on welcome's description copy — that's
    // owned by other UX work and likely to change. Just prove the
    // command still resolves and shows its clap-generated usage block.
    let out = Command::new(ANVIL_BIN)
        .arg("welcome")
        .arg("--help")
        .output()
        .expect("failed to invoke anvil binary");
    assert!(
        out.status.success(),
        "anvil welcome --help failed after LAUNCH-006 promotion: stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    // Stable: clap always emits a `Usage:` block with the subcommand
    // name. If the alias-removal regressed, clap would error out before
    // reaching this point (non-zero exit, caught above).
    assert!(
        stdout.contains("Usage:") && stdout.contains("welcome"),
        "welcome --help should emit clap's Usage block, got:\n{stdout}"
    );
}

#[test]
fn start_on_invalid_config_emits_error_state_not_panic() {
    // Adversarial guardrail: a malformed .anvilrc must not panic the
    // start orchestrator. The diagnostic surfaces it as `state: error`.
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join(".anvilrc"),
        "{this is not valid in any format::",
    )
    .unwrap();

    let out = run_start(dir.path(), &[]);
    assert!(
        out.status.success(),
        "anvil start on invalid config failed (should report error state, not exit non-zero): stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("state: error"),
        "expected `state: error` on malformed config, got:\n{stdout}"
    );
}
