//! LAUNCH-006 / LAUNCH-009: `anvil start` activation entrypoint
//! integration tests.
//!
//! `anvil start` drives the activation orchestration: init if absent,
//! MCP install (LAUNCH-009 part 2), then `activation::verify`. The
//! acceptance contract is:
//!
//! - On a fresh temp repo with an empty `HOME` override, `anvil start`
//!   exits 0 with `ready_restart_required` (MCP entries written for
//!   Cursor + Claude Code; user must restart their editor).
//! - On a fresh temp repo with `--verify` (read-only) and empty `HOME`,
//!   the diagnostic reports `needs_action` (config absent, no MCP
//!   detected — nothing has been written).
//! - `anvil welcome` still runs unchanged.
//! - Idempotent reruns skip init AND the install (entries already up
//!   to date).
//! - `--json` emits a state literal in the same shape as `anvil status
//!   --verify --json` (LAUNCH-012).
//!
//! ## HOME isolation
//!
//! Every test that invokes `anvil start` (not `--help`-style metadata
//! tests) overrides `HOME` to a per-test tempdir. Without this, the
//! tests would probe the test runner's real `~/.cursor/mcp.json` /
//! `~/.claude.json` and report whatever state the developer happens
//! to have on their machine — flaky everywhere, broken on developer
//! machines that already have anvil installed. CI runs with an empty
//! home anyway, so the override only changes local-dev behaviour.
//!
//! `dirs::home_dir()` (via `dirs-sys` 0.5) checks `$HOME` first on
//! every Unix platform including macOS, falling back to `getpwuid_r`
//! only when `HOME` is unset or empty. The `env("HOME", …)`
//! subprocess override is therefore sufficient on macOS as well as
//! Linux. On Windows we also set `USERPROFILE`. (The earlier `dirs`
//! 4.x crate used `getpwuid_r` first on macOS — that quirk is gone
//! in the version we ship.)
//!
//! Council-locked truthfulness: a fresh repo MUST NEVER claim
//! `protecting`. LAUNCH-011 lands the only safe path to that state.

use std::fs;
use std::process::Command;

const ANVIL_BIN: &str = env!("CARGO_BIN_EXE_anvil");

fn run_start_with_home(
    workdir: &std::path::Path,
    home: &std::path::Path,
    extra_args: &[&str],
) -> std::process::Output {
    let mut cmd = Command::new(ANVIL_BIN);
    cmd.arg("--no-tui")
        .arg("start")
        .args(extra_args)
        .current_dir(workdir)
        .env("HOME", home)
        // Windows uses USERPROFILE; macOS / Linux use HOME. Set both
        // so the same test bench works across platforms.
        .env("USERPROFILE", home)
        // Strip XDG so dirs::home_dir() doesn't resolve to a user
        // directory through XDG_CONFIG_HOME.
        .env_remove("XDG_CONFIG_HOME")
        // DLIFE-003: pin the daemon socket/PID resolution to the per-test
        // tempdir so the daemon-ensure probe is deterministically isolated
        // from any real daemon on a developer box. The captured (non-TTY)
        // stdout means `anvil start` resolves a non-interactive context and
        // falls back without spawning — no `--no-daemon` needed for
        // hermeticity, so the harness exercises the real default path.
        // Unix-only env (ignored on Windows, which uses a per-user pipe).
        .env("XDG_RUNTIME_DIR", home)
        .env("ANVIL_DEV", "1")
        // ACTMO-012: `anvil start` now only writes a fresh MCP config for
        // editors it actually detects (binary on PATH / pre-existing
        // editor state). These mechanics tests assert both Cursor and
        // Claude Code entries are written, so force the all-clients
        // opt-in — otherwise the result would depend on whether the test
        // host happens to have `cursor` / `claude` on PATH, which is not
        // hermetic. The detection gate has its own dedicated coverage
        // (unit tests in `activation::orchestrator::install` and the
        // `start_without_detected_editor_does_not_write_mcp_config`
        // negative test below).
        .env("ANVIL_ALL_MCP_CLIENTS", "1")
        .env("ANVIL_SKIP_WELCOME", "1");
    cmd.output().expect("failed to invoke anvil binary")
}

#[cfg(not(target_os = "windows"))]
#[test]
fn start_on_fresh_repo_runs_init_and_lands_ready_restart_required() {
    // The composed flow's headline outcome on an empty HOME: init
    // writes `.anvilrc`, the MCP install step writes Cursor + Claude
    // Code entries into HOME, and the diagnostic ends at
    // `ready_restart_required` (the user must restart their editor
    // for the entries to attach).
    let dir = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    assert!(
        !dir.path().join(".anvilrc").exists(),
        "pre-condition: fresh temp repo has no .anvilrc"
    );

    let out = run_start_with_home(dir.path(), home.path(), &[]);
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

    // Install ran — Cursor + Claude Code entries written into HOME.
    assert!(
        home.path().join(".cursor/mcp.json").exists(),
        "Cursor MCP config must exist in HOME after `anvil start`"
    );
    assert!(
        home.path().join(".claude.json").exists(),
        "Claude Code MCP config must exist in HOME after `anvil start`"
    );

    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("state: ready_restart_required"),
        "expected `state: ready_restart_required` after MCP install, got:\n{stdout}"
    );
    // Truthfulness guardrail.
    assert!(
        !stdout.contains("state: protecting"),
        "fresh repo MUST NOT claim protection, got:\n{stdout}"
    );
}

#[cfg(not(target_os = "windows"))]
#[test]
fn start_without_detected_editor_does_not_write_mcp_config() {
    // ACTMO-012 (Matt beta smoke): on a host where no editor is detected
    // — no `cursor` / `claude` binary on PATH, no pre-existing editor
    // state under HOME — and without `--all-mcp-clients`, `anvil start`
    // must NOT create `~/.cursor/mcp.json` or `~/.claude.json`. The spine
    // (`.anvilrc`, daemon, hooks) still activates; MCP is optional.
    //
    // Determinism: PATH is emptied and the AI-tool env hints are removed
    // so detection cannot fire from the test host's real environment.
    let dir = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();

    let mut cmd = Command::new(ANVIL_BIN);
    cmd.arg("--no-tui")
        .arg("start")
        .current_dir(dir.path())
        .env("HOME", home.path())
        .env("USERPROFILE", home.path())
        .env_remove("XDG_CONFIG_HOME")
        .env("XDG_RUNTIME_DIR", home.path())
        .env("ANVIL_DEV", "1")
        .env("ANVIL_SKIP_WELCOME", "1")
        // Force a deterministic "no editor detected" environment.
        .env("PATH", "")
        .env_remove("ANVIL_ALL_MCP_CLIENTS")
        .env_remove("ANTHROPIC_API_KEY")
        .env_remove("CLAUDE_CODE_HOME")
        .env_remove("CURSOR_HOME")
        .env_remove("OPENAI_API_KEY");
    let out = cmd.output().expect("failed to invoke anvil binary");
    assert!(
        out.status.success(),
        "anvil start failed: stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );

    // The spine still activated.
    assert!(
        dir.path().join(".anvilrc").exists(),
        ".anvilrc must exist after `anvil start` even with no editor detected"
    );
    // But no MCP config was written for an editor the user does not have.
    assert!(
        !home.path().join(".cursor/mcp.json").exists(),
        "must NOT write a Cursor MCP config when Cursor is not detected"
    );
    assert!(
        !home.path().join(".claude.json").exists(),
        "must NOT write a Claude Code MCP config when Claude Code is not detected"
    );
}

#[cfg(not(target_os = "windows"))]
#[test]
fn start_in_non_interactive_context_falls_back_without_spawning() {
    // DLIFE-003 (ADR-082 §4, owner-confirmed headless posture): a mutating
    // `anvil start` with captured (non-TTY) stdout — the CI / scripted shape
    // — must NOT auto-start a daemon. It reports the deterministic
    // non-interactive fallback on the `daemon:` line and never makes a
    // protection claim (module honesty contract: lifecycle action only).
    let dir = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();

    let out = run_start_with_home(dir.path(), home.path(), &[]);
    assert!(
        out.status.success(),
        "anvil start failed: stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("daemon: not auto-started (non-interactive"),
        "expected the non-interactive fallback line, got:\n{stdout}"
    );
    assert!(
        stdout.contains("scoped fallback"),
        "the fallback line must name the preserved scoped fallback, got:\n{stdout}"
    );
    // The fallback line must never graduate the protection claim: the
    // diagnostic still owns `state:`.
    assert!(
        !stdout.contains("state: protecting"),
        "the daemon line must not push the protection state to protecting, got:\n{stdout}"
    );
}

#[cfg(not(target_os = "windows"))]
#[test]
fn start_no_daemon_flag_reports_opt_out_end_to_end() {
    // DLIFE-003: the explicit `--no-daemon` opt-out is plumbed through clap
    // and rendered distinctly from the non-interactive fallback, so a user
    // who opted out sees their flag acknowledged rather than a generic
    // context message.
    let dir = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();

    let out = run_start_with_home(dir.path(), home.path(), &["--no-daemon"]);
    assert!(
        out.status.success(),
        "anvil start --no-daemon failed: stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("daemon: not started (--no-daemon)"),
        "expected the --no-daemon opt-out line, got:\n{stdout}"
    );
}

#[cfg(not(target_os = "windows"))]
#[test]
fn start_verify_is_byte_identical_without_a_daemon_line() {
    // DLIFE-003: read-only probes never start a daemon and never emit a
    // `daemon:` lifecycle line — the auto-start is a mutating-path
    // behaviour only. Pins the read-only contract end-to-end.
    let dir = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();

    let out = run_start_with_home(dir.path(), home.path(), &["--verify"]);
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        !stdout.contains("daemon:"),
        "--verify must not emit a daemon lifecycle line, got:\n{stdout}"
    );
}

#[cfg(not(target_os = "windows"))]
#[test]
fn start_idempotent_rerun_skips_init_and_install() {
    // Idempotency contract: a second `anvil start` against the same
    // repo + same HOME must:
    //   1. Not rewrite `.anvilrc` (mtime unchanged).
    //   2. Not rewrite the MCP entries (mtime unchanged on each
    //      target file).
    //   3. Still emit the diagnostic, ending at the same final state.
    let dir = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    let first = run_start_with_home(dir.path(), home.path(), &[]);
    assert!(first.status.success());

    let cursor_path = home.path().join(".cursor/mcp.json");
    let claude_path = home.path().join(".claude.json");
    let anvilrc = dir.path().join(".anvilrc");

    let mtime_anvilrc_before = std::fs::metadata(&anvilrc).unwrap().modified().unwrap();
    let mtime_cursor_before = std::fs::metadata(&cursor_path).unwrap().modified().unwrap();
    let mtime_claude_before = std::fs::metadata(&claude_path).unwrap().modified().unwrap();

    // Sleep past one-second mtime granularity so any rewrite would be
    // detectable on filesystems with HFS+-style coarse mtimes.
    std::thread::sleep(std::time::Duration::from_millis(1100));
    let second = run_start_with_home(dir.path(), home.path(), &[]);
    assert!(
        second.status.success(),
        "second start failed: stderr={}",
        String::from_utf8_lossy(&second.stderr)
    );

    let mtime_anvilrc_after = std::fs::metadata(&anvilrc).unwrap().modified().unwrap();
    let mtime_cursor_after = std::fs::metadata(&cursor_path).unwrap().modified().unwrap();
    let mtime_claude_after = std::fs::metadata(&claude_path).unwrap().modified().unwrap();
    assert_eq!(
        mtime_anvilrc_before, mtime_anvilrc_after,
        "second start must not rewrite .anvilrc (idempotent rerun)"
    );
    assert_eq!(
        mtime_cursor_before, mtime_cursor_after,
        "second start must not rewrite Cursor MCP config when already up to date"
    );
    assert_eq!(
        mtime_claude_before, mtime_claude_after,
        "second start must not rewrite Claude Code MCP config when already up to date"
    );

    // Diagnostic still emitted on the second run.
    let stdout = String::from_utf8_lossy(&second.stdout);
    assert!(
        stdout.contains("state: ready_restart_required"),
        "second start must still emit the diagnostic, got:\n{stdout}"
    );
    assert!(
        stdout.contains("already up to date"),
        "second start must report install was a no-op, got:\n{stdout}"
    );
}

#[cfg(not(target_os = "windows"))]
#[test]
fn start_verify_on_fresh_repo_reports_needs_action() {
    // `activation::verify` is read-only. With an empty HOME override,
    // no MCP entry exists anywhere, so the diagnostic maps
    // ConfigStatus::Absent → ProtectionState::NeedsAction.
    let dir = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    let out = run_start_with_home(dir.path(), home.path(), &["--verify"]);
    assert!(
        out.status.success(),
        "anvil start --verify failed: stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );

    // --verify is read-only: `.anvilrc` must NOT be written, and
    // neither HOME's `.cursor/mcp.json` nor `.claude.json`.
    assert!(
        !dir.path().join(".anvilrc").exists(),
        "--verify must not write .anvilrc"
    );
    assert!(
        !home.path().join(".cursor/mcp.json").exists(),
        "--verify must not install Cursor MCP entry"
    );
    assert!(
        !home.path().join(".claude.json").exists(),
        "--verify must not install Claude Code MCP entry"
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
    // LAUNCH-012 acceptance: `anvil start --json` is read-only — the
    // flag implies `--verify` (see `start.rs` `read_only = verify ||
    // json`). On a fresh repo with an empty HOME override no MCP entry
    // exists and no `.anvilrc` is written, so the diagnostic maps
    // `ConfigStatus::Absent → ProtectionState::NeedsAction` — the same
    // outcome as `start --verify` (covered by
    // `start_verify_on_fresh_repo_reports_needs_action`).
    //
    // Council-locked truthfulness (CLAWP-022): a fresh repo MUST NEVER
    // claim `protecting`, `watching`, or `ready_restart_required` on
    // this read-only path. Accepting any of those would let a
    // regression silently graduate the diagnostic to a stronger claim
    // than read-only evidence supports.
    let dir = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    let out = run_start_with_home(dir.path(), home.path(), &["--json"]);
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);

    // --json implies --verify: no writes should land on disk.
    assert!(
        !dir.path().join(".anvilrc").exists(),
        "--json must not write .anvilrc (read-only)"
    );
    assert!(
        !home.path().join(".cursor/mcp.json").exists(),
        "--json must not install Cursor MCP entry (read-only)"
    );
    assert!(
        !home.path().join(".claude.json").exists(),
        "--json must not install Claude Code MCP entry (read-only)"
    );

    let json: serde_json::Value =
        serde_json::from_str(&stdout).expect("--json output must be valid JSON");
    let state = json["state"]
        .as_str()
        .expect("state must be present as a string");
    // Truthfulness-specific rejection runs first so a regression that
    // graduates the diagnostic to a stronger claim fails with the
    // CLAWP-022-locked message, not the generic equality mismatch.
    for forbidden in ["protecting", "watching", "ready_restart_required"] {
        assert_ne!(
            state, forbidden,
            "fresh repo MUST NOT claim `{forbidden}` on the read-only --json path"
        );
    }
    assert_eq!(
        state, "needs_action",
        "fresh repo + empty HOME under read-only --json must land on needs_action, got {state}"
    );
    assert!(json["headline"].is_string(), "headline must be a string");
    assert!(json["config"].is_string(), "config must be a string");
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
fn start_help_documents_daemon_lifecycle() {
    // DLIFE-005: `anvil start --help` must keep documenting the daemon
    // lifecycle opt-out so the CLI long help stays aligned with the
    // public docs (auto-start + `--no-daemon` / `ANVIL_NO_DAEMON`).
    let out = Command::new(ANVIL_BIN)
        .arg("start")
        .arg("--help")
        .output()
        .expect("failed to invoke anvil binary");
    assert!(
        out.status.success(),
        "anvil start --help failed: stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("--no-daemon"),
        "start --help should list the --no-daemon opt-out, got:\n{stdout}"
    );
    assert!(
        stdout.contains("ANVIL_NO_DAEMON"),
        "start --help should name the ANVIL_NO_DAEMON env opt-out, got:\n{stdout}"
    );
}

#[test]
fn start_on_invalid_config_emits_error_state_not_panic() {
    // Adversarial guardrail: a malformed .anvilrc must not panic the
    // start orchestrator. The diagnostic surfaces it as `state: error`.
    let dir = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join(".anvilrc"),
        "{this is not valid in any format::",
    )
    .unwrap();

    let out = run_start_with_home(dir.path(), home.path(), &[]);
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

// ---- LAUNCH-011: honest watch fallback --------------------------

#[cfg(not(target_os = "windows"))]
#[test]
fn start_verify_on_initialised_repo_surfaces_partial_protection_note() {
    // LAUNCH-011 acceptance: on an initialised repo (config valid)
    // where MCP cannot pre-write attach, the human render must say
    // so explicitly — never let the user infer protection from a
    // weaker tier or from config-only state.
    //
    // The repo is pre-initialised with a valid `.anvilrc` so the
    // diagnostic does not bypass the offer logic via the
    // `ConfigStatus::Absent` suppression (council remediation: the
    // primary action on Absent is `anvil init`, not watch fallback).
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join(".anvilrc"),
        "profile: default\nchecks: []\n",
    )
    .unwrap();
    // Drop a TS file so the language profile reports a supported
    // language and the diagnostic does not collapse to `Unsupported`
    // (which would also suppress the offer).
    fs::write(dir.path().join("index.ts"), "export {};\n").unwrap();
    let home = tempfile::tempdir().unwrap();
    let out = run_start_with_home(dir.path(), home.path(), &["--verify"]);
    assert!(
        out.status.success(),
        "anvil start --verify failed: stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);

    // The diagnostic must include the literal honesty note.
    assert!(
        stdout.contains("MCP pre-write validation is not attached"),
        "initialised-repo --verify must include the partial-protection note, got:\n{stdout}"
    );

    // It must surface the offered watch tier so the user sees the
    // fallback option in the structured output, not just in the prose
    // hint.
    assert!(
        stdout.contains("watch: offered"),
        "initialised-repo --verify must show watch tier as `offered`, got:\n{stdout}"
    );

    // Truthfulness guardrails — the language LAUNCH-011 explicitly
    // forbids must NOT appear anywhere in the rendered output.
    let lower = stdout.to_lowercase();
    assert!(
        !lower.contains("fully protected"),
        "rendered output must never claim `fully protected`, got:\n{stdout}"
    );
    assert!(
        !lower.contains("mcp activated"),
        "rendered output must never claim `MCP activated`, got:\n{stdout}"
    );
    assert!(
        !stdout.contains("state: protecting"),
        "initialised-repo --verify MUST NOT claim `state: protecting`, got:\n{stdout}"
    );
}

#[cfg(not(target_os = "windows"))]
#[test]
fn start_verify_on_fresh_repo_with_absent_config_does_not_advertise_watch() {
    // Council remediation: when config is absent, the user's primary
    // action is `anvil init`, not watch fallback. The note and the
    // `Offered` tier must both be suppressed so the init nudge is
    // not diluted.
    let dir = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    let out = run_start_with_home(dir.path(), home.path(), &["--verify"]);
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("config: absent"),
        "expected config: absent on fresh repo, got:\n{stdout}"
    );
    assert!(
        !stdout.contains("watch: offered"),
        "fresh repo with absent config MUST NOT advertise watch, got:\n{stdout}"
    );
    assert!(
        !stdout.contains("MCP pre-write validation is not attached"),
        "fresh repo with absent config must defer to init copy, got:\n{stdout}"
    );
    assert!(
        stdout.contains("anvil init"),
        "fresh repo with absent config must surface the init nudge, got:\n{stdout}"
    );
}

#[test]
fn start_after_install_communicates_restart_or_daemon_recovery_action() {
    // Council round-2 remediation: at `ready_restart_required`, the
    // headline already conveys the partial state ("restart your
    // editor or agent so the MCP server attaches"). The
    // partial-protection NOTE is suppressed here because:
    //   1. The headline already says MCP isn't yet attached.
    //   2. There is no `watch: offered` line — appending the watch-
    //      fallback note alone would orphan watch copy and nudge the
    //      user toward watch when they should restart.
    // The honesty contract is preserved by the headline / hint. When
    // daemon evidence is already available and says the daemon is not
    // reachable, DLIFE-006 deliberately replaces the generic restart
    // wording with the terminating `anvil intercept start --foreground`
    // recovery path.
    let dir = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    let out = run_start_with_home(dir.path(), home.path(), &[]);
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("state: ready_restart_required"),
        "post-install state should be ready_restart_required, got:\n{stdout}"
    );
    let lower = stdout.to_lowercase();
    assert!(
        lower.contains("restart your editor")
            || lower.contains("restart required")
            || lower.contains("anvil intercept start --foreground"),
        "ready_restart_required render must surface the next concrete \
         recovery action in headline / hint, got:\n{stdout}"
    );
    assert!(
        lower.contains("attach")
            || lower.contains("mcp server")
            || (lower.contains("mcp config") && lower.contains("daemon not reachable")),
        "ready_restart_required render must explain why pre-write \
         protection is not yet live, got:\n{stdout}"
    );
    // The note belongs with `Watching` / `NeedsAction (config valid)` /
    // the `Offered` watch tier. At ready_restart_required, suppress.
    assert!(
        !stdout.contains("MCP pre-write validation is not attached"),
        "ready_restart_required render must NOT include the orphaned \
         watch-fallback note (the headline already conveys the \
         partial state), got:\n{stdout}"
    );
    // Truthfulness guardrails.
    let lower = stdout.to_lowercase();
    assert!(
        !lower.contains("fully protected"),
        "ready_restart_required must never claim fully protected, got:\n{stdout}"
    );
    assert!(
        !stdout.contains("state: protecting"),
        "ready_restart_required must never claim state: protecting, got:\n{stdout}"
    );
}

#[test]
fn start_watch_with_verify_is_rejected() {
    // LAUNCH-011: `--watch` spawns a process; `--verify` is read-only.
    // Combining them would silently downgrade one or synthesise watch
    // state without actually starting it. Reject the combination so
    // the user gets a clear error instead.
    let dir = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    let out = run_start_with_home(dir.path(), home.path(), &["--watch", "--verify"]);
    assert!(
        !out.status.success(),
        "`--watch --verify` must fail, stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("`--watch` and `--verify` are mutually exclusive"),
        "error message must explain the conflict, got:\n{stderr}"
    );
}

#[test]
fn start_verify_with_format_is_rejected() {
    // CIB-051: `--format` pre-writes `.anvil.<ext>` — a durable project
    // mutation — while `--verify` is read-only. The combination used to
    // silently drop `--format`; reject it explicitly like the
    // `--watch` / `--new-identity` siblings so the user gets a clear
    // error instead.
    let dir = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    let out = run_start_with_home(dir.path(), home.path(), &["--verify", "--format", "yaml"]);
    assert!(
        !out.status.success(),
        "`--verify --format` must fail, stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("`--format` is incompatible with `--verify` / `--json` (read-only)"),
        "error message must explain the conflict, got:\n{stderr}"
    );
    // Clean bail before side effects — no config file may be written.
    assert!(
        !dir.path().join(".anvil.yaml").exists(),
        "`--verify --format` must not write .anvil.yaml"
    );
}

#[test]
fn start_json_with_format_is_rejected() {
    // CIB-051: `--json` implies read-only exactly like `--verify`, so
    // the `--format` pre-write is rejected for the same reason. Pin the
    // second half of the read-only condition the way
    // `start_watch_with_json_is_rejected` pins its sibling.
    let dir = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    let mut cmd = Command::new(ANVIL_BIN);
    cmd.arg("--no-tui")
        .arg("--json")
        .arg("start")
        .arg("--format")
        .arg("yaml")
        .current_dir(dir.path())
        .env("HOME", home.path())
        .env("USERPROFILE", home.path())
        .env_remove("XDG_CONFIG_HOME")
        .env("ANVIL_DEV", "1")
        .env("ANVIL_SKIP_WELCOME", "1");
    let out = cmd.output().expect("failed to invoke anvil binary");
    assert!(
        !out.status.success(),
        "`--json` + `--format` must fail, stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("`--format` is incompatible with `--verify` / `--json` (read-only)"),
        "error message must explain the conflict, got:\n{stderr}"
    );
    // Clean bail before side effects — no config file may be written.
    assert!(
        !dir.path().join(".anvil.yaml").exists(),
        "`--json` + `--format` must not write .anvil.yaml"
    );
}

#[test]
fn start_watch_with_json_is_rejected() {
    // LAUNCH-011: the watcher streams event lines; `--json` expects a
    // single parseable document. Reject the combination explicitly.
    let dir = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    let mut cmd = Command::new(ANVIL_BIN);
    cmd.arg("--no-tui")
        .arg("--json")
        .arg("start")
        .arg("--watch")
        .current_dir(dir.path())
        .env("HOME", home.path())
        .env("USERPROFILE", home.path())
        .env_remove("XDG_CONFIG_HOME")
        .env("ANVIL_DEV", "1")
        .env("ANVIL_SKIP_WELCOME", "1");
    let out = cmd.output().expect("failed to invoke anvil binary");
    assert!(
        !out.status.success(),
        "`--watch --json` must fail, stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("`--watch` and `--json` are mutually exclusive"),
        "error message must explain the conflict, got:\n{stderr}"
    );
}

#[test]
fn start_watch_renders_partial_protection_and_starts_watcher() {
    // LAUNCH-011 acceptance: with no supported MCP client live,
    // `anvil start --watch` runs the orchestrator, prints the
    // diagnostic, and lands the user in the kernel watcher. The
    // pre-handoff render must:
    //
    //   1. Not claim `state: protecting` (no LiveValidation evidence).
    //   2. Include the explicit "MCP pre-write validation is not
    //      attached" note.
    //   3. Print the watch hand-off marker so the user sees the
    //      transition into the fallback.
    //
    // Implementation note: the watcher is long-running. We read
    // stdout in a separate thread until we see the hand-off marker
    // or the deadline expires, then SIGKILL the child via
    // `Child::kill` (cross-platform). The test process is the parent
    // and is fine; we deliberately do not assert on graceful shutdown
    // — that path is covered by `commands::watch` unit tests.
    use std::io::Read;
    use std::process::Stdio;
    use std::sync::{Arc, Mutex};
    use std::time::{Duration, Instant};

    let dir = tempfile::tempdir().unwrap();
    // Seed the workspace so the diagnostic does not depend on
    // orchestrator init behaviour — config is `Valid` from the start
    // and a TS file forces a supported-language profile, locking the
    // path that the assertions below exercise.
    fs::write(
        dir.path().join(".anvilrc"),
        "profile: default\nchecks: []\n",
    )
    .unwrap();
    fs::write(dir.path().join("index.ts"), "export {};\n").unwrap();
    let home = tempfile::tempdir().unwrap();

    let mut child = Command::new(ANVIL_BIN)
        .arg("--no-tui")
        .arg("start")
        .arg("--watch")
        .current_dir(dir.path())
        .env("HOME", home.path())
        .env("USERPROFILE", home.path())
        .env_remove("XDG_CONFIG_HOME")
        // DLIFE-003: isolate the daemon socket to the per-test tempdir. The
        // captured (non-TTY) stdout already makes `start` fall back without
        // spawning a daemon; this also keeps the read-only ensure probe from
        // touching a developer box's real daemon.
        .env("XDG_RUNTIME_DIR", home.path())
        .env("ANVIL_DEV", "1")
        .env("ANVIL_SKIP_WELCOME", "1")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn anvil start --watch");

    let mut stdout_handle = child.stdout.take().expect("piped stdout");

    // Drain stdout in a worker thread so the parent can enforce a
    // wall-clock deadline. A blocking read on the main thread would
    // hang if the child wrote nothing for some reason.
    let buf: Arc<Mutex<String>> = Arc::new(Mutex::new(String::new()));
    let buf_clone = Arc::clone(&buf);
    let reader = std::thread::spawn(move || {
        let mut chunk = [0u8; 1024];
        loop {
            match stdout_handle.read(&mut chunk) {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    if let Ok(mut guard) = buf_clone.lock() {
                        guard.push_str(&String::from_utf8_lossy(&chunk[..n]));
                    }
                }
            }
        }
    });

    // Poll the buffer for the hand-off marker.
    let deadline = Instant::now() + Duration::from_secs(20);
    while Instant::now() < deadline {
        if let Ok(guard) = buf.lock()
            && guard.contains("watch: starting save-time fallback")
        {
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
    }

    // Stop the child; the reader thread will exit on EOF.
    let _ = child.kill();
    let _ = child.wait();
    let _ = reader.join();

    let captured = buf.lock().map(|g| g.clone()).unwrap_or_default();

    assert!(
        captured.contains("MCP pre-write validation is not attached"),
        "pre-handoff render must include the partial-protection note, got:\n{captured}"
    );
    assert!(
        captured.contains("watch: starting save-time fallback"),
        "must print the watch hand-off marker before entering the watcher, \
         got:\n{captured}"
    );

    // LAUNCH-011 spec acceptance: the rendered state must literally
    // be `watching`, not `protecting`. The pre-handoff diagnostic
    // synthesises `WatchTier::Running` so the printed `state:` line
    // matches the protection layer about to take over.
    //
    // Two acceptable renderings (depending on the test runner's HOME
    // contents):
    //   - empty HOME → MCP at `ConfigAbsent` → `state: watching`
    //   - HOME with stale anvil entry → MCP at `RestartRequired` →
    //     `state: ready_restart_required` (watch + restart-pending
    //     prefers the stronger label per the diagnostic mapping)
    // Either is honest; the forbidden literal is `state: protecting`.
    assert!(
        captured.contains("state: watching") || captured.contains("state: ready_restart_required"),
        "pre-handoff state must be `watching` or `ready_restart_required` \
         (the only honest options when MCP is below LiveValidation), got:\n{captured}"
    );
    assert!(
        !captured.contains("state: protecting"),
        "fallback path MUST NOT claim `state: protecting`, got:\n{captured}"
    );
    let lower = captured.to_lowercase();
    assert!(
        !lower.contains("fully protected"),
        "fallback path MUST NEVER claim `fully protected`, got:\n{captured}"
    );
}

/// CIB-049: `start --verify` is a read-only local probe and must skip
/// the auth wall through `skips_auth_for_local_probe` itself — NOT via
/// the `ANVIL_DEV` escape hatch the other tests in this file lean on.
/// Unauthenticated human mode runs the probe instead of printing the
/// auth-required message.
#[test]
fn start_verify_runs_probe_unauthenticated_without_dev() {
    let dir = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();

    let mut cmd = Command::new(ANVIL_BIN);
    cmd.arg("--no-tui")
        .arg("start")
        .arg("--verify")
        .current_dir(dir.path())
        .env("HOME", home.path())
        .env("USERPROFILE", home.path())
        // Point XDG at an empty dir so no credentials resolve.
        .env("XDG_CONFIG_HOME", home.path().join("xdg"))
        .env_remove("ANVIL_DEV")
        .env_remove("ANVIL_LICENSE")
        .env("ANVIL_SKIP_WELCOME", "1")
        .env("ANVIL_NO_PROMPT", "1");
    let output = cmd.output().expect("failed to invoke anvil binary");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("Authentication required"),
        "start --verify must skip the auth wall via the local-probe \
         predicate, not ANVIL_DEV: stderr=\n{stderr}",
    );
    assert!(
        stdout.contains("ACTIVATION"),
        "expected the human activation diagnostic on stdout: \
         stdout=\n{stdout}\nstderr=\n{stderr}",
    );
}
