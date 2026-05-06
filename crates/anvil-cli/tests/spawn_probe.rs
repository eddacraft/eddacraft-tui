//! LAUNCH-009.5 integration: `anvil status --verify` runs the MCP
//! initialize handshake probe (observability-only in v1) without
//! breaking the diagnostic surface.
//!
//! The probe spawns `anvil mcp serve --stdio` against the installed
//! entry and observes a JSON-RPC `initialize` exchange within a 1-second
//! budget. In v1, the probe result is logged via `tracing` but does NOT
//! change the user-facing tier (the original LAUNCH-009 spec described
//! a `RestartRequired → ServerStartable` promotion that conflicts with
//! the existing tier ladder where `ServerStartable < RestartRequired`;
//! the semantic alignment is deferred to LAUNCH-009.6).
//!
//! These integration tests verify:
//!
//! 1. The probe runs without breaking the diagnostic surface — install
//!    + verify stays at `restart_required` and renders normally.
//! 2. When no install has happened, the probe is skipped (no spawn
//!    overhead, tier stays at `config_absent`).
//! 3. End-to-end timing — the probe completes within its 1-second
//!    budget plus reasonable slack so an `anvil status --verify` doesn't
//!    block the user for tens of seconds on a broken binary.
//!
//! ## HOME isolation
//!
//! Same convention as `tests/status_verify.rs`: override `HOME` and
//! `USERPROFILE` to a per-test tempdir so the test runs deterministically
//! on developer machines that already have anvil installed.

use std::fs;
use std::path::Path;
use std::process::{Command, Output};

const ANVIL_BIN: &str = env!("CARGO_BIN_EXE_anvil");

fn run_status_verify(workdir: &Path, home: &Path) -> Output {
    Command::new(ANVIL_BIN)
        .arg("--no-tui")
        .arg("status")
        .arg("--verify")
        .current_dir(workdir)
        .env("HOME", home)
        .env("USERPROFILE", home)
        .env_remove("XDG_CONFIG_HOME")
        .env("ANVIL_DEV", "1")
        .env("ANVIL_SKIP_WELCOME", "1")
        .output()
        .expect("failed to invoke anvil binary")
}

/// Pre-populate `~/.cursor/mcp.json` with an anvil entry whose `command`
/// matches the test bin. The probe will spawn this command and run the
/// initialize handshake.
fn install_cursor_entry_pointing_at_test_bin(home: &Path) {
    fs::create_dir_all(home.join(".cursor")).unwrap();
    let cfg = serde_json::json!({
        "mcpServers": {
            "anvil": {
                "command": ANVIL_BIN,
                "args": ["mcp", "serve", "--stdio"],
                "env": {},
            }
        }
    });
    fs::write(
        home.join(".cursor/mcp.json"),
        serde_json::to_string_pretty(&cfg).unwrap(),
    )
    .unwrap();
}

#[test]
fn handshake_against_real_anvil_does_not_break_diagnostic() {
    // Install a Cursor entry that points at the real test binary, then
    // run `status --verify`. The probe should spawn `anvil mcp serve
    // --stdio` against the test bin, drive the JSON-RPC initialize
    // handshake successfully, log it via tracing, and let the rendered
    // tier remain at `restart_required`. The diagnostic must complete
    // and render normally.
    let workdir = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    install_cursor_entry_pointing_at_test_bin(home.path());

    let start = std::time::Instant::now();
    let out = run_status_verify(workdir.path(), home.path());
    let elapsed = start.elapsed();

    assert!(
        out.status.success(),
        "anvil status --verify failed: stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);

    assert!(
        stdout.contains("Cursor: restart_required"),
        "Cursor tier should be restart_required after install (probe is \
         observability-only in v1; tier promotion deferred to \
         LAUNCH-009.6), got:\n{stdout}"
    );
    assert!(
        !stdout.contains("Cursor: server_startable"),
        "Cursor must not be promoted to server_startable in v1 — that \
         direction conflicts with the existing tier ladder, got:\n{stdout}"
    );

    // Performance guard: the probe has a 1-second handshake budget and
    // runs once total. The entire `status --verify` invocation must
    // not exceed a reasonable upper bound, even on slow CI runners.
    // 30 seconds is generous slack to absorb cargo overhead, init,
    // and the file-walk; if this trips we have a runaway probe.
    assert!(
        elapsed < std::time::Duration::from_secs(30),
        "status --verify took {elapsed:?}, suggests the probe didn't \
         honour its 1s timeout"
    );
}

#[test]
fn probe_is_skipped_when_no_install_yet() {
    // Empty home → no installed entry → tier is `config_absent`. The
    // probe-promotion gate only fires for `restart_required` clients,
    // so this exercises the negative path: no spawn happens, no tier
    // change, and the diagnostic remains honest about missing config.
    let workdir = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    // No `.cursor/mcp.json` written.

    let out = run_status_verify(workdir.path(), home.path());
    assert!(
        out.status.success(),
        "anvil status --verify failed: stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);

    assert!(
        stdout.contains("Cursor: config_absent"),
        "Cursor tier must remain config_absent when no install has happened, got:\n{stdout}"
    );
    assert!(
        !stdout.contains("server_startable"),
        "no client should be at server_startable when home is empty, got:\n{stdout}"
    );
}
