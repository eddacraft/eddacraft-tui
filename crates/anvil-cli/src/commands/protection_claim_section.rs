//! Shared protection-claim rendering for surfaces that need to report
//! the §14 closed-set vocabulary.
//!
//! MLP2-048 wired `anvil status --json` to emit a typed
//! [`ProtectionClaim`] built from the daemon snapshot via
//! [`build_protection_claim_from_wire`], with a local-only fallback
//! when no snapshot is available. MLP2-051a lifts that wiring out of
//! `commands::status` so `anvil doctor` (and future surfaces) can
//! produce a byte-identical claim from the same inputs.
//!
//! The local fallback derivation deliberately undershoots — it cannot
//! prove per-surface state, so it emits an empty `surfaces` array and
//! a worktree state mapped from the local
//! [`activation::ActivationDiagnostic`]. The daemon-snapshot path
//! enumerates real sessions through [`build_protection_claim_from_wire`].

use std::fmt::Write as _;
use std::path::Path;

use anvil_intercept::status::build_protection_claim_from_wire;
use anvil_intercept_proto::status::DaemonStatusV1;
use anvil_kernel_types::protection_claim::{ProtectionClaim, WorktreeClaimState};

use crate::activation;

/// Build a [`ProtectionClaim`] for `worktree`, preferring the live
/// daemon snapshot when one is available. Falls back to a local-only
/// derivation (empty `surfaces`, worktree state mapped from
/// `activation_diag`) when the daemon is not reachable.
///
/// Output is byte-identical to `commands::status`'s prior in-line
/// implementation; both surfaces now share this helper so the
/// cross-surface parity check (MLP2-051e) only has to certify one
/// path.
#[must_use]
pub fn resolve_protection_claim(
    activation_diag: &activation::ActivationDiagnostic,
    daemon_snapshot: Option<&DaemonStatusV1>,
    worktree: &Path,
) -> ProtectionClaim {
    if let Some(snapshot) = daemon_snapshot {
        return build_protection_claim_from_wire(snapshot, worktree);
    }
    ProtectionClaim::new(derive_local_worktree_state(activation_diag), Vec::new())
}

/// Pick a `WorktreeClaimState` from local activation diagnostics
/// alone. Used when no daemon snapshot is available; deliberately
/// undershoots the closed-set vocabulary so we never over-claim
/// per-surface coverage — see [`resolve_protection_claim`] for the
/// integrated mapping.
#[must_use]
pub fn derive_local_worktree_state(diag: &activation::ActivationDiagnostic) -> WorktreeClaimState {
    use activation::state::ProtectionState as PS;

    match diag.protection_state() {
        PS::Protecting => WorktreeClaimState::PreWriteDaemon,
        PS::Watching => WorktreeClaimState::SaveTimeOnly,
        PS::ReadyRestartRequired => WorktreeClaimState::Warming,
        PS::NeedsAction | PS::Unsupported | PS::Error => WorktreeClaimState::Unprotected,
    }
}

/// Fetch the daemon snapshot + canonical worktree the same way
/// `anvil status --json` does, then build a [`ProtectionClaim`] for
/// the current working directory. Best-effort: a missing or
/// unreachable daemon is mapped to the local-only fallback.
///
/// Used by `anvil doctor` so it emits the same claim shape as
/// `anvil status --json` for the same daemon state.
#[must_use]
pub fn fetch_protection_claim_for_cwd() -> ProtectionClaim {
    let activation_diag = activation::verify(Path::new("."));
    let daemon_snapshot = match crate::commands::intercept::query_daemon_status() {
        Ok(snapshot) => Some(snapshot),
        Err(err) => {
            tracing::debug!(
                error = %err,
                "anvil doctor: daemon IPC unavailable; falling back to local-only protection claim",
            );
            None
        }
    };
    let worktree = std::fs::canonicalize(".").unwrap_or_else(|err| {
        tracing::warn!(
            error = %err,
            "anvil doctor: cwd canonicalise failed; protection claim will not match any daemon-registered session",
        );
        Path::new(".").to_path_buf()
    });
    resolve_protection_claim(&activation_diag, daemon_snapshot.as_ref(), &worktree)
}

/// Render a [`ProtectionClaim`] as the multi-line plain-text section
/// `anvil doctor` and `anvil status` emit. Format:
///
/// ```text
/// protection: <worktree-state>
///   surface <identifier>: <state>
/// ```
///
/// One line per surface, sorted by identifier (already pre-sorted by
/// [`build_protection_claim_from_wire`]). When `surfaces` is empty
/// (local-only fallback / unknown worktree / no sessions) the section
/// is just the headline line.
#[must_use]
pub fn render_protection_claim_plain(claim: &ProtectionClaim) -> String {
    let mut out = String::new();
    // Writes to `String` are infallible; the trait bound is what
    // unlocks `write!` (clippy::format_push_string).
    let _ = writeln!(out, "protection: {}", claim.worktree_state.as_str());
    for surface in &claim.surfaces {
        let _ = writeln!(
            out,
            "  surface {}: {}",
            surface.identifier,
            surface.state.as_str()
        );
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use anvil_kernel_types::protection_claim::{
        SurfaceClaim, SurfaceClaimState, WorktreeClaimState,
    };

    /// Isolated `HOME`/`XDG`/`ANVIL_HOME` so operator MCP entries cannot
    /// promote a missing project to `ReadyRestartRequired` → `Warming`.
    fn isolated_operator_roots() -> tempfile::TempDir {
        let root = tempfile::tempdir().expect("isolated operator roots");
        std::fs::create_dir_all(root.path().join(".config")).expect("xdg config");
        std::fs::create_dir_all(root.path().join("runtime")).expect("xdg runtime");
        root
    }

    fn with_isolated_operator_env<T>(root: &Path, body: impl FnOnce() -> T) -> T {
        let home = root.to_string_lossy().into_owned();
        let config = root.join(".config").to_string_lossy().into_owned();
        let runtime = root.join("runtime").to_string_lossy().into_owned();
        temp_env::with_vars(
            [
                ("HOME", Some(home.as_str())),
                ("XDG_CONFIG_HOME", Some(config.as_str())),
                ("XDG_RUNTIME_DIR", Some(runtime.as_str())),
                ("ANVIL_HOME", Some(home.as_str())),
            ],
            body,
        )
    }

    fn write_restart_required_cursor_fixture(home: &Path) {
        let cursor = home.join(".cursor");
        std::fs::create_dir_all(&cursor).expect("cursor config dir");
        std::fs::write(
            cursor.join("mcp.json"),
            r#"{"mcpServers":{"anvil":{"command":"anvil","args":["mcp","serve","--stdio"],"env":{}}}}"#,
        )
        .expect("cursor mcp fixture");
    }

    /// Local fallback: no daemon snapshot, isolated empty operator
    /// roots, missing project → `Unprotected`, empty surfaces.
    #[test]
    fn local_fallback_yields_unprotected_with_empty_surfaces() {
        let roots = isolated_operator_roots();
        with_isolated_operator_env(roots.path(), || {
            let diag = activation::diagnostic::verify_with_home(
                Path::new("/nonexistent-anvil-pcs-test"),
                Some(roots.path()),
            );
            let claim = resolve_protection_claim(&diag, None, Path::new("/tmp/wt-pcs-fallback"));
            assert_eq!(claim.worktree_state, WorktreeClaimState::Unprotected);
            assert!(claim.surfaces.is_empty());
        });
    }

    /// Same live verify path, but with a representative restart-required
    /// MCP fixture. Pins that isolation is fixture-controlled: host MCP
    /// must not leak in, and a seeded entry must surface as `Warming`.
    #[test]
    fn local_fallback_yields_warming_for_isolated_restart_required_fixture() {
        let roots = isolated_operator_roots();
        write_restart_required_cursor_fixture(roots.path());
        with_isolated_operator_env(roots.path(), || {
            let diag = activation::diagnostic::verify_with_home(
                Path::new("/nonexistent-anvil-pcs-fixture"),
                Some(roots.path()),
            );
            let claim = resolve_protection_claim(&diag, None, Path::new("/tmp/wt-pcs-fixture"));
            assert_eq!(claim.worktree_state, WorktreeClaimState::Warming);
            assert!(claim.surfaces.is_empty());
        });
    }

    /// Headline-only render: empty surfaces emit just the
    /// `protection:` line, no per-surface entries.
    #[test]
    fn plain_render_for_empty_surfaces_is_headline_only() {
        let claim = ProtectionClaim::new(WorktreeClaimState::Unprotected, vec![]);
        let rendered = render_protection_claim_plain(&claim);
        assert_eq!(rendered, "protection: unprotected\n");
    }

    /// Plain render lists one line per surface using the §14
    /// closed-set vocabulary, with surfaces in input order
    /// (sorted-by-identifier in the daemon path).
    #[test]
    fn plain_render_lists_surfaces_with_closed_set_states() {
        let claim = ProtectionClaim::new(
            WorktreeClaimState::DegradedProtection,
            vec![
                SurfaceClaim {
                    identifier: "alpha".to_owned(),
                    state: SurfaceClaimState::Participating,
                },
                SurfaceClaim {
                    identifier: "beta".to_owned(),
                    state: SurfaceClaimState::Quarantined,
                },
            ],
        );
        let rendered = render_protection_claim_plain(&claim);
        assert_eq!(
            rendered,
            "protection: degraded-protection\n  \
             surface alpha: participating\n  \
             surface beta: quarantined\n",
        );
    }

    /// `PreWriteDaemon` headline + a participating surface render in
    /// the documented §14.2 vocabulary — pinned so a future
    /// `WorktreeClaimState::as_str` rename forces an explicit update.
    #[test]
    fn plain_render_pre_write_daemon_uses_pre_write_daemon_token() {
        let claim = ProtectionClaim::new(
            WorktreeClaimState::PreWriteDaemon,
            vec![SurfaceClaim {
                identifier: "sess-test".to_owned(),
                state: SurfaceClaimState::Participating,
            }],
        );
        let rendered = render_protection_claim_plain(&claim);
        assert!(
            rendered.starts_with("protection: pre-write-daemon\n"),
            "headline must use the §14.2 token: {rendered:?}",
        );
        assert!(
            rendered.contains("  surface sess-test: participating\n"),
            "surface line must use §14.1 token: {rendered:?}",
        );
    }
}
