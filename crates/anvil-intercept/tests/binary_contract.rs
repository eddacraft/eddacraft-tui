//! CLAWP-037: process-level contract for the standalone `anvil-intercept`
//! daemon binary.
//!
//! The other suites (`jsonrpc_conformance`, `midedit_contract`, …) drive
//! the in-process dispatcher and never spawn the actual `[[bin]]`
//! entrypoint, so a regression in its clap surface or exit codes had no
//! coverage. These tests exercise the binary as a process via Cargo's
//! `CARGO_BIN_EXE_<name>` env var (no extra dev-dependency), covering the
//! minimum contract: `--help` succeeds and names the surface, and an
//! invalid invocation fails fast with a usage error. The foreground
//! `start` + SIGTERM lifecycle is intentionally out of scope here (it
//! blocks indefinitely; INTD-002 owns the backgrounded-launch story).

use std::process::Command;

const ANVIL_INTERCEPT_BIN: &str = env!("CARGO_BIN_EXE_anvil-intercept");

#[test]
fn help_flag_prints_usage_and_exits_zero() {
    let out = Command::new(ANVIL_INTERCEPT_BIN)
        .arg("--help")
        .output()
        .expect("failed to spawn anvil-intercept");

    assert!(
        out.status.success(),
        "`anvil-intercept --help` must exit 0, got {:?}; stderr=\n{}",
        out.status,
        String::from_utf8_lossy(&out.stderr),
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("anvil-intercept"),
        "usage must name the binary: {stdout}"
    );
    assert!(
        stdout.contains("start"),
        "usage must list the `start` subcommand: {stdout}"
    );
}

#[test]
fn unknown_subcommand_exits_nonzero_with_usage_error() {
    let out = Command::new(ANVIL_INTERCEPT_BIN)
        .arg("definitely-not-a-subcommand")
        .output()
        .expect("failed to spawn anvil-intercept");

    assert!(
        !out.status.success(),
        "an unknown subcommand must exit non-zero, got {:?}",
        out.status,
    );
    // The non-zero exit is the real contract; additionally assert clap
    // emitted a diagnostic to stderr (matching specific clap wording
    // would be brittle across clap versions).
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !stderr.trim().is_empty(),
        "an unknown subcommand must emit a diagnostic on stderr",
    );
}
