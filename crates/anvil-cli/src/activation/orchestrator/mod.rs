//! `anvil start` activation orchestration (LAUNCH-006 / LAUNCH-009).
//!
//! Composes the read-safe / idempotent primitives the activation flow
//! ships today:
//!
//! 1. Probe `activation::verify`. If `.anvilrc` is absent, call
//!    `commands::init::run_in` (which writes the default config AND runs
//!    the LAUNCH-004 post-init first-scan inline).
//! 2. **MCP install (LAUNCH-009 part 2).** Probe each registered MCP
//!    client (Cursor, Claude Code), classify drift, and either prompt
//!    the user with a [`demand`] picker (interactive) or auto-install
//!    the obvious cases (non-interactive). See [`install`] for the
//!    drift policy and atomicity guarantees.
//! 3. Re-probe and return the diagnostic for the caller to render.
//!
//! **Deliberately NOT in v1** (deferred to LAUNCH-009.5 / LAUNCH-011 —
//! the tasks that own the safe versions of these steps):
//! - **Server startable spawn probe.** Spawning `anvil mcp serve --stdio`
//!   and observing a clean handshake promotes a tier from
//!   `RestartRequired` to `ServerStartable`. Out of scope here; lives
//!   in LAUNCH-009.5 once the spawn helper is verified.
//! - **Watch fallback spawn.** LAUNCH-011 owns the in-process / detached
//!   watcher that lets `start` end in the `watching` state. Until then,
//!   `WatchTier::NotRequested` is the honest answer.
//! - **Doctor composition.** `anvil doctor` bails on any check failure;
//!   inside `start`'s composed flow that propagates as a hard error and
//!   strips the user of a `ProtectionState` literal.
//!
//! **First-run marker:** `anvil start` does NOT write
//! `.anvil/first-run`. `anvil welcome` keeps sole ownership of that
//! marker so the two surfaces don't fight for first-run state.

use std::io::IsTerminal;
use std::path::Path;

use anyhow::Context;

use crate::GlobalArgs;
use crate::activation::diagnostic::{ActivationDiagnostic, ConfigStatus, verify_with_home};
use crate::commands::init;

pub mod install;

pub use install::{InstallOutcome, InstallReport, SkipReason};

/// Run the orchestration on `root` and return the final diagnostic
/// alongside the install report.
///
/// The caller is responsible for rendering — the orchestrator is mute on
/// the activation diagnostic itself so unit tests can assert against the
/// returned struct without parsing stdout. Init's own output (config
/// success copy + first-scan summary) goes to stdout when init runs;
/// re-runs against an existing config produce no init output.
pub fn run(
    root: &Path,
    global: &GlobalArgs,
) -> anyhow::Result<(ActivationDiagnostic, InstallReport)> {
    run_with_home(root, dirs::home_dir().as_deref(), global)
}

/// Like [`run`] but with an explicit `home` override.
///
/// Used by tests that need to write MCP configs into a tempdir-scoped
/// home rather than the developer's real `~/.cursor/mcp.json` etc.
/// Crate-private — production callers must go through [`run`] so the
/// `dirs::home_dir()` resolution stays in one place.
pub(crate) fn run_with_home(
    root: &Path,
    home: Option<&Path>,
    global: &GlobalArgs,
) -> anyhow::Result<(ActivationDiagnostic, InstallReport)> {
    // Step 1 — write `.anvilrc` if absent.
    let initial = verify_with_home(root, home);
    if matches!(initial.config, ConfigStatus::Absent) {
        let args = init::InitArgs { force: false };
        init::run_in(&args, global, root).context("init step of `anvil start` failed")?;
    }

    // Step 2 — install MCP entries for the user-selected (or auto-
    // selected) clients. The install module handles drift, picker UX,
    // and atomic writes; failures are folded into the report rather
    // than propagated, so the orchestrator always returns a final
    // diagnostic the user can act on.
    let install_report = match std::env::current_exe() {
        Ok(exe) => {
            let fresh = crate::activation::mcp_client::AnvilEntry::local_stdio(exe);
            let interactive = is_interactive(global);
            install::install_for_clients(root, home, &fresh, interactive)
        }
        Err(e) => {
            // current_exe failed — verify_with_home will also report
            // last_error, so we don't shadow that signal. Skip install
            // entirely.
            tracing::warn!(
                error = %e,
                "orchestrator: could not resolve current_exe; MCP install skipped",
            );
            InstallReport::default()
        }
    };

    // Step 3 — final probe. The diagnostic absorbs the install side
    // effects (e.g. tiers should now read `RestartRequired` for the
    // clients we just wrote) so the caller can render a single source
    // of truth.
    let mut diagnostic = verify_with_home(root, home);

    // Surface every install failure on the diagnostic so
    // `protection_state()` collapses to `Error` and JSON consumers
    // see all simultaneous failures, not just the first one.
    if let Some(err) = install_report.aggregated_failure() {
        diagnostic.last_error = Some(format!("MCP install failed: {err}"));
    }

    Ok((diagnostic, install_report))
}

/// Decide whether to surface the interactive picker. We require:
/// - not `--json` (machine-readable output must not include prompts)
/// - not `--no-tui` (explicit user opt-out)
/// - stdin is a TTY (`demand` reads keystrokes from stdin)
/// - stderr is a TTY (`demand` renders the prompt to stderr; piping
///   stderr to a file would render the prompt invisibly while still
///   consuming keystrokes)
/// - not running under a known non-interactive shell context
///   (`CI=true`, `GIT_DIR` set, `ANVIL_NO_PROMPT`, etc. — see
///   [`crate::is_non_interactive_env`])
///
/// Council remediation: previously checked `stdout.is_terminal()`, which
/// misclassified `anvil start | tee log.txt` (auto-installs silently)
/// and `echo "" | anvil start` (picker hangs on closed stdin). The new
/// check matches the convention in `commands/tutorial.rs:41` and the
/// auth-prompt gate in `main.rs:413`.
fn is_interactive(global: &GlobalArgs) -> bool {
    !global.json
        && !global.no_tui
        && std::io::stdin().is_terminal()
        && std::io::stderr().is_terminal()
        && !crate::is_non_interactive_env()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::GlobalArgs;
    use crate::activation::diagnostic::McpClientId;
    use tempfile::TempDir;

    fn default_global() -> GlobalArgs {
        // `--no-tui` forces the non-interactive auto-install branch
        // so unit tests don't try to summon a picker.
        GlobalArgs {
            no_tui: true,
            ..Default::default()
        }
    }

    fn run_in_isolated(
        root: &Path,
        home: &Path,
        global: &GlobalArgs,
    ) -> (ActivationDiagnostic, InstallReport) {
        run_with_home(root, Some(home), global).expect("orchestrator should succeed")
    }

    #[test]
    fn orchestrator_writes_config_when_absent() {
        let dir = TempDir::new().unwrap();
        let home = TempDir::new().unwrap();
        let global = default_global();

        // Pre-condition: no config.
        let pre = verify_with_home(dir.path(), Some(home.path()));
        assert!(matches!(pre.config, ConfigStatus::Absent));

        let (diag, _report) = run_in_isolated(dir.path(), home.path(), &global);

        assert!(
            dir.path().join(".anvilrc").exists(),
            "orchestrator should write .anvilrc on a fresh repo"
        );
        assert!(matches!(diag.config, ConfigStatus::Valid));
    }

    #[test]
    fn orchestrator_skips_init_when_config_valid() {
        let dir = TempDir::new().unwrap();
        let home = TempDir::new().unwrap();
        let global = default_global();

        // Run once to write config + install.
        run_in_isolated(dir.path(), home.path(), &global);
        let mtime_before = std::fs::metadata(dir.path().join(".anvilrc"))
            .unwrap()
            .modified()
            .unwrap();

        // Idempotency check: file mtime must not change on a re-run.
        // Sleep a beat to make any rewrite detectable across filesystems
        // with one-second mtime granularity (e.g. HFS+).
        std::thread::sleep(std::time::Duration::from_millis(1100));
        run_in_isolated(dir.path(), home.path(), &global);
        let mtime_after = std::fs::metadata(dir.path().join(".anvilrc"))
            .unwrap()
            .modified()
            .unwrap();

        assert_eq!(
            mtime_before, mtime_after,
            "orchestrator must not rewrite .anvilrc on idempotent re-run"
        );
    }

    #[test]
    fn orchestrator_auto_installs_in_no_tui_mode() {
        let dir = TempDir::new().unwrap();
        let home = TempDir::new().unwrap();
        let global = default_global();

        let (_diag, report) = run_in_isolated(dir.path(), home.path(), &global);

        // Both clients should have been auto-installed at home scope.
        assert!(
            matches!(
                report.per_client.get(&McpClientId::Cursor),
                Some(InstallOutcome::Installed { .. })
            ),
            "Cursor must auto-install in --no-tui mode"
        );
        assert!(
            matches!(
                report.per_client.get(&McpClientId::ClaudeCode),
                Some(InstallOutcome::Installed { .. })
            ),
            "Claude Code must auto-install in --no-tui mode"
        );
        assert!(home.path().join(".cursor/mcp.json").exists());
        assert!(home.path().join(".claude.json").exists());
    }

    #[test]
    fn orchestrator_diagnostic_reflects_post_install_state() {
        // After install, the diagnostic re-probe must show the
        // RestartRequired tier — that's the whole point of the install
        // step. Without it, `anvil start` would land on NeedsAction
        // even though we just wired both clients.
        let dir = TempDir::new().unwrap();
        let home = TempDir::new().unwrap();
        let global = default_global();

        let (diag, _report) = run_in_isolated(dir.path(), home.path(), &global);
        let cursor_tier = diag.mcp.get(&McpClientId::Cursor).map(|r| r.tier);
        let claude_tier = diag.mcp.get(&McpClientId::ClaudeCode).map(|r| r.tier);

        assert_eq!(
            cursor_tier,
            Some(crate::activation::diagnostic::McpTier::RestartRequired),
            "Cursor tier should advance to RestartRequired after install"
        );
        assert_eq!(
            claude_tier,
            Some(crate::activation::diagnostic::McpTier::RestartRequired),
            "Claude Code tier should advance to RestartRequired after install"
        );
    }

    #[test]
    fn orchestrator_returns_ready_restart_required_after_install() {
        // The composed flow's headline outcome: a fresh repo with no
        // editor wired ends at `ReadyRestartRequired` once both clients
        // have an entry on disk. (Was `NeedsAction` before LAUNCH-009
        // part 2 — that test moved into `*_skips_install_when_*` cases.)
        let dir = TempDir::new().unwrap();
        let home = TempDir::new().unwrap();
        let global = default_global();

        let (diag, _report) = run_in_isolated(dir.path(), home.path(), &global);
        let state = diag.protection_state();
        assert!(
            matches!(
                state,
                crate::activation::state::ProtectionState::ReadyRestartRequired
                    | crate::activation::state::ProtectionState::Unsupported
            ),
            "post-install fresh repo should land at ready_restart_required \
             (or unsupported when no covered languages), got {state:?}"
        );
    }

    #[test]
    fn orchestrator_does_not_overwrite_unsafe_drift() {
        let dir = TempDir::new().unwrap();
        let home = TempDir::new().unwrap();
        let global = default_global();

        // Pre-populate Cursor with a foreign-tool entry that uses our
        // server name.
        std::fs::create_dir_all(home.path().join(".cursor")).unwrap();
        let cfg = r#"{"mcpServers": {"anvil": {"command": "/bin/bash", "args": ["mcp", "serve", "--stdio"], "env": {}}}}"#;
        std::fs::write(home.path().join(".cursor/mcp.json"), cfg).unwrap();
        let bytes_before = std::fs::read(home.path().join(".cursor/mcp.json")).unwrap();

        let (_diag, report) = run_in_isolated(dir.path(), home.path(), &global);

        match report.per_client.get(&McpClientId::Cursor) {
            Some(InstallOutcome::Skipped {
                reason: SkipReason::UnsafeDrift(_),
            }) => {}
            other => panic!("expected Cursor UnsafeDrift skip, got {other:?}"),
        }

        let bytes_after = std::fs::read(home.path().join(".cursor/mcp.json")).unwrap();
        assert_eq!(bytes_before, bytes_after, "must not overwrite UnsafeDrift");
    }
}
