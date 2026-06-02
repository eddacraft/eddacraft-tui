//! LAUNCH-009.6 integration: `anvil status --verify` runs the MCP
//! initialize handshake probe and promotes the installed client to the
//! configured-plus-handshake tier without claiming live validation.
//!
//! The probe spawns `anvil mcp serve --stdio` against the installed
//! entry and observes a JSON-RPC `initialize` exchange within a 1-second
//! budget. LAUNCH-009.6 promotes only installed `restart_required`
//! entries whose actual configured command handshakes successfully; it
//! does not reuse `server_startable`, which remains the weaker "server
//! runs but client wiring is not confirmed" tier.
//!
//! These integration tests verify:
//!
//! 1. The probe runs without breaking the diagnostic surface —
//!    `install` followed by `verify` promotes the entry to
//!    `restart_handshake_verified` and renders normally.
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

#[cfg(not(target_os = "windows"))]
use std::fs;
use std::path::Path;
use std::process::{Command, Output};

const ANVIL_BIN: &str = env!("CARGO_BIN_EXE_anvil");

fn run_status_verify(workdir: &Path, home: &Path) -> Output {
    run_status_verify_with_path(workdir, home, None)
}

fn run_status_verify_with_path(workdir: &Path, home: &Path, path: Option<&Path>) -> Output {
    let mut command = Command::new(ANVIL_BIN);
    command
        .arg("--no-tui")
        .arg("status")
        .arg("--verify")
        .current_dir(workdir)
        .env("HOME", home)
        .env("USERPROFILE", home)
        .env_remove("XDG_CONFIG_HOME")
        .env("ANVIL_DEV", "1")
        .env("ANVIL_SKIP_WELCOME", "1");
    if let Some(path) = path {
        command.env("PATH", path);
    }
    command.output().expect("failed to invoke anvil binary")
}

/// Pre-populate `~/.cursor/mcp.json` with an anvil entry whose `command`
/// matches the test bin. The probe will spawn this command and run the
/// initialize handshake.
///
/// Cfg-gated to non-Windows: `dirs::home_dir()` on Windows ignores the
/// HOME / USERPROFILE env overrides this helper relies on, so its only
/// consumers (the two handshake tests) are also `#[cfg(not(target_os = "windows"))]`.
#[cfg(not(target_os = "windows"))]
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
        home.join(".cursor").join("mcp.json"),
        serde_json::to_string_pretty(&cfg).unwrap(),
    )
    .unwrap();
}

#[cfg(not(target_os = "windows"))]
fn install_claude_code_entry_pointing_at_bare_anvil(home: &Path) {
    let cfg = serde_json::json!({
        "mcpServers": {
            "anvil": {
                "type": "stdio",
                "command": "anvil",
                "args": ["mcp", "serve", "--stdio"],
                "env": {},
            }
        }
    });
    fs::write(
        home.join(".claude.json"),
        serde_json::to_string_pretty(&cfg).unwrap(),
    )
    .unwrap();
}

#[cfg(not(target_os = "windows"))]
#[test]
fn handshake_against_real_anvil_promotes_restart_required_client() {
    // Install a Cursor entry that points at the real test binary, then
    // run `status --verify`. The probe should spawn `anvil mcp serve
    // --stdio` against the test bin, drive the JSON-RPC initialize
    // handshake successfully, promote the rendered tier to
    // `restart_handshake_verified`, and still avoid claiming live
    // validation. The diagnostic must complete and render normally.
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
        stdout.contains("Cursor: restart_handshake_verified"),
        "Cursor tier should be restart_handshake_verified after a successful \
         installed-entry handshake, got:\n{stdout}"
    );
    assert!(
        !stdout.contains("Cursor: server_startable"),
        "Cursor must not be promoted to server_startable — that tier is \
         weaker than configured-plus-handshake, got:\n{stdout}"
    );
    assert!(
        !stdout.contains("state: protecting"),
        "handshake verification must not overclaim live validation, got:\n{stdout}"
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
    assert!(
        !stdout.contains("restart_handshake_verified"),
        "no client should be promoted when home is empty, got:\n{stdout}"
    );
}

/// CLAWP-045: write a fake `anvil` onto a test-controlled PATH that
/// ignores its args and sleeps instead of serving MCP, so
/// `anvil mcp serve --stdio` spawns but never emits a JSON-RPC
/// `initialize` response. Paired with a bare-`"anvil"` config entry
/// (below), this is what lets the probe reach the handshake path and
/// then genuinely wedge there until its 1-second timeout fires.
///
/// The script restores a normal PATH and `exec`s `sleep`: the probe
/// runs the stub with PATH set to just `path_dir` (so the bare `anvil`
/// command resolves here), which would otherwise hide `sleep`. `exec`
/// replaces the shell with `sleep` under the same PID, so the probe's
/// timeout-kill lands on the actual sleeping process and leaves no
/// orphan.
#[cfg(not(target_os = "windows"))]
fn write_hanging_anvil_stub(path_dir: &Path) {
    use std::os::unix::fs::PermissionsExt;
    let stub = path_dir.join("anvil");
    fs::write(
        &stub,
        "#!/bin/sh\nPATH=/usr/bin:/bin:$PATH\nexec sleep 60\n",
    )
    .unwrap();
    fs::set_permissions(&stub, fs::Permissions::from_mode(0o755)).unwrap();
}

/// Install a Cursor entry with a bare `"command": "anvil"`. Bare `anvil`
/// is config-*equivalent* to a real install (basename match, see
/// `mcp_client::entries_equivalent`), so it classifies as
/// `RestartRequired` — the only tier the spawn probe actually launches.
/// A full path to a non-canonical `anvil` would instead read as version
/// drift (`ConfigPresent`) and never be spawned.
#[cfg(not(target_os = "windows"))]
fn install_cursor_entry_bare_anvil(home: &Path) {
    fs::create_dir_all(home.join(".cursor")).unwrap();
    let cfg = serde_json::json!({
        "mcpServers": {
            "anvil": {
                "command": "anvil",
                "args": ["mcp", "serve", "--stdio"],
                "env": {},
            }
        }
    });
    fs::write(
        home.join(".cursor").join("mcp.json"),
        serde_json::to_string_pretty(&cfg).unwrap(),
    )
    .unwrap();
}

#[cfg(not(target_os = "windows"))]
#[test]
fn hanging_handshake_times_out_without_promotion() {
    // CLAWP-045: the suite previously timed only the SUCCESSFUL
    // handshake path, so a probe regression that dropped the 1-second
    // budget would be caught on a healthy server but not on a wedged
    // one. Drive the probe at a RestartRequired entry whose `anvil`
    // resolves (via the controlled PATH) to a stub that never answers
    // the initialize request, then assert (a) `status --verify` still
    // returns well within budget and (b) the client is NOT promoted.
    let workdir = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    let path_dir = tempfile::tempdir().unwrap();
    install_cursor_entry_bare_anvil(home.path());
    write_hanging_anvil_stub(path_dir.path());

    let start = std::time::Instant::now();
    let out = run_status_verify_with_path(workdir.path(), home.path(), Some(path_dir.path()));
    let elapsed = start.elapsed();

    assert!(
        out.status.success(),
        "anvil status --verify failed: stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);

    // The wedged server must NOT be promoted — the handshake never
    // completed — and must not land in the weaker server_startable tier.
    assert!(
        !stdout.contains("restart_handshake_verified"),
        "an unresponsive MCP command must not promote to \
         restart_handshake_verified, got:\n{stdout}"
    );
    assert!(
        !stdout.contains("server_startable"),
        "a wedged handshake must not promote to server_startable, got:\n{stdout}"
    );
    assert!(
        stdout.contains("Cursor: restart_required"),
        "a RestartRequired Cursor entry whose handshake wedges should stay \
         at restart_required, got:\n{stdout}"
    );

    // Budget guard: the stub `sleep 60` is far longer than any
    // legitimate `status --verify`, so exceeding this bound means the
    // 1-second probe timeout did not fire and the probe blocked on the
    // child.
    assert!(
        elapsed < std::time::Duration::from_secs(20),
        "status --verify took {elapsed:?} against a hanging MCP command — \
         the 1s probe timeout did not fire"
    );
}

#[cfg(not(target_os = "windows"))]
#[test]
fn handshake_promotion_is_per_client() {
    // Cursor points at the exact test binary and should promote. Claude Code
    // uses a bare `anvil`; with PATH set to an empty tempdir, that installed
    // entry is still equivalent for config matching but cannot handshake, so
    // it must remain at restart_required.
    let workdir = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    let empty_path = tempfile::tempdir().unwrap();
    install_cursor_entry_pointing_at_test_bin(home.path());
    install_claude_code_entry_pointing_at_bare_anvil(home.path());

    let out = run_status_verify_with_path(workdir.path(), home.path(), Some(empty_path.path()));

    assert!(
        out.status.success(),
        "anvil status --verify failed: stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);

    assert!(
        stdout.contains("Cursor: restart_handshake_verified"),
        "Cursor should promote after its installed entry handshakes, got:\n{stdout}"
    );
    assert!(
        stdout.contains("Claude Code: restart_required"),
        "Claude Code should not promote from Cursor's handshake, got:\n{stdout}"
    );
}
