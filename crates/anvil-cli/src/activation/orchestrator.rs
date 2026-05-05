//! `anvil start` activation orchestration (LAUNCH-006).
//!
//! Composes only the read-safe / idempotent primitives shipped today:
//!
//! 1. Probe `activation::verify`. If `.anvilrc` is absent, call
//!    `commands::init::run_in` (which writes the default config AND runs
//!    the LAUNCH-004 post-init first-scan inline).
//! 2. Re-probe and return the diagnostic for the caller to render.
//!
//! **Deliberately NOT in v1** (deferred to LAUNCH-009 / LAUNCH-011 — the
//! tasks that own the safe versions of these steps):
//! - **MCP install.** `anvil mcp install` parses the editor's existing
//!   config; on malformed JSON it returns an `anyhow::Error` that would
//!   abort the orchestration with no final state. LAUNCH-009 lands the
//!   safe parse-before-modify path.
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

use std::path::Path;

use anyhow::Context;

use crate::GlobalArgs;
use crate::activation::diagnostic::{ActivationDiagnostic, ConfigStatus, verify};
use crate::commands::init;

/// Run the orchestration on `root` and return the final diagnostic.
///
/// The caller is responsible for rendering — the orchestrator is mute on
/// the activation diagnostic itself so unit tests can assert against the
/// returned struct without parsing stdout. Init's own output (config
/// success copy + first-scan summary) goes to stdout when init runs;
/// re-runs against an existing config produce no init output.
pub fn run(root: &Path, global: &GlobalArgs) -> anyhow::Result<ActivationDiagnostic> {
    let initial = verify(root);
    if matches!(initial.config, ConfigStatus::Absent) {
        let args = init::InitArgs { force: false };
        init::run_in(&args, global, root).context("init step of `anvil start` failed")?;
    }
    Ok(verify(root))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::GlobalArgs;

    fn default_global() -> GlobalArgs {
        GlobalArgs {
            no_tui: true,
            ..Default::default()
        }
    }

    #[test]
    fn orchestrator_writes_config_when_absent() {
        let dir = tempfile::tempdir().unwrap();
        let global = default_global();

        // Pre-condition: no config.
        assert!(matches!(verify(dir.path()).config, ConfigStatus::Absent));

        let diag = run(dir.path(), &global).expect("orchestrator should succeed");

        assert!(
            dir.path().join(".anvilrc").exists(),
            "orchestrator should write .anvilrc on a fresh repo"
        );
        assert!(matches!(diag.config, ConfigStatus::Valid));
    }

    #[test]
    fn orchestrator_skips_init_when_config_valid() {
        let dir = tempfile::tempdir().unwrap();
        let global = default_global();

        // Run once to write config.
        run(dir.path(), &global).unwrap();
        let mtime_before = std::fs::metadata(dir.path().join(".anvilrc"))
            .unwrap()
            .modified()
            .unwrap();

        // Idempotency check: file mtime must not change on a re-run.
        // Sleep a beat to make any rewrite detectable across filesystems
        // with one-second mtime granularity (e.g. HFS+).
        std::thread::sleep(std::time::Duration::from_millis(1100));
        run(dir.path(), &global).unwrap();
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
    fn orchestrator_returns_needs_action_state_on_fresh_repo() {
        // The contract: a fresh repo with no editor wired lands at
        // needs_action. LAUNCH-009 / LAUNCH-011 are the only paths that
        // could legitimately produce ready_restart_required, watching,
        // or protecting; until they ship, needs_action is the
        // expected v1 final state.
        let dir = tempfile::tempdir().unwrap();
        let global = default_global();

        let diag = run(dir.path(), &global).unwrap();
        let state = diag.protection_state();
        assert!(
            matches!(
                state,
                crate::activation::state::ProtectionState::NeedsAction
                    | crate::activation::state::ProtectionState::Unsupported
            ),
            "fresh repo should land at needs_action or unsupported, got {state:?}"
        );
    }
}
