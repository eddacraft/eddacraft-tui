//! INTD-009: synchronous embedded-mode API.
//!
//! The intercept daemon's primary deployment shape is a per-user
//! background process talking JSON-RPC over Unix sockets / Windows
//! named pipes. CI, batch tooling, and one-shot validation surfaces
//! (the existing RMCP shim's embedded path under
//! `daemon_status: NotWired`) need an in-process API that produces
//! the same enforcement decision **without** running a daemon. This
//! module is that API.
//!
//! ## What embedded mode is
//!
//! A synchronous function that takes a [`ChangeBatch`] of
//! caller-provided proposed content plus a [`crate::config::Resolved`]
//! enforcement policy and returns an [`crate::enforcement::EnforcementDecision`]
//! produced by the same [`EnforcementPipeline`] the daemon uses. The
//! envelope every consumer sees (the `anvil.diagnostic.v1` shape
//! owned by AIGUARD-002) is byte-identical to the daemon-backed
//! path on the same fixture — that is the load-bearing parity
//! property the existing
//! `local_daemon_client_returns_scan_buffer_diagnostics_with_embedded_parity`
//! test asserts in `anvil-cli` and the new
//! [`tests::embedded_path_emits_same_envelope_as_daemon_path`] mirror
//! asserts here.
//!
//! ## What embedded mode is NOT
//!
//! **Not a silent fallback for a failed daemon.** When the daemon
//! is configured but unreachable, the caller MUST receive a daemon
//! error — embedded mode must never auto-promote from a failed
//! daemon path. The MCP shim's `daemon_status: NotWired` path
//! already enforces this distinction (`Unavailable` → embedded,
//! `OperationalFailure` → propagate); embedded mode in this module
//! is the API for callers who explicitly chose in-process. There
//! is no `try_daemon_then_embedded` helper here, and the council
//! review (security-analyst, M5 lineage) rejected one as a
//! correctness foot-gun.
//!
//! Pinned by [`tests::embedded_does_not_auto_promote_from_failed_daemon_path`]:
//! the API surface accepts only the request and the resolved
//! config, never a daemon failure to "recover from". A future
//! refactor that adds a `from_daemon_failure` constructor would
//! break that test.
//!
//! ## Honouring INTD-008 config
//!
//! Embedded mode must honour the same `enforcement.mode` /
//! `observe_only` semantics the daemon does:
//!
//! | Resolved mode             | Embedded behaviour                                          |
//! | ------------------------- | ----------------------------------------------------------- |
//! | `Mode::Off`               | Always `Allow` with diagnostics — the ADR-098 AD-3 posture; |
//! |                           | projects to always-`Allow` (same embedded behaviour as      |
//! |                           | `Warn`; the daemon never enforces from the embedded path).  |
//! | `Mode::Warn`              | Always `Allow` with diagnostics — the rule engine still     |
//! |                           | produces them, but the decision stays `Allow`.              |
//! | `Mode::Fence`             | Pipeline result returned as-is. The caller (CI, MCP shim)   |
//! |                           | applies the fence decision in its own way; embedded has no  |
//! |                           | fence store.                                                |
//! | `Mode::Interrupt`         | Pipeline result returned as-is. As above for fence — the    |
//! |                           | caller chooses how to enforce. (Embedded mode does not have |
//! |                           | a process group to interrupt.)                              |
//! | `observe_only: true`      | Always `Allow`, regardless of `mode`. Diagnostics still     |
//! |                           | flow on the side channel returned by                        |
//! |                           | [`embedded_evaluate_with_diagnostics`].                     |
//!
//! See `plans/modules/intercept-daemon.aps.md` task INTD-009 and
//! `plans/decisions/015-intercept-loop-enforcement.md` AD-3.

use std::path::PathBuf;

use anvil_intercept_rules::ChangeKind;
use anvil_kernel_types::{Diagnostic, Mode as DiagnosticMode};

use crate::config::{Mode as ConfigMode, Resolved};
use crate::enforcement::{
    EnforcementDecision, EnforcementPipeline, InterruptDecision, ProposedChange,
};

/// Single proposed change, owned. The embedded API takes owned
/// values rather than borrowed ones because typical callers (CI
/// pipelines, the MCP shim) build the request from scratch and
/// have no longer-lived buffer to borrow from. Internally the
/// shape converts to [`ProposedChange`] for the call into
/// [`EnforcementPipeline`].
#[derive(Debug, Clone)]
pub struct ProposedFileChange {
    pub path: PathBuf,
    pub change_kind: ChangeKind,
    /// Caller-provided content. Required: the embedded API does
    /// **not** read from disk. If the caller wants the daemon's
    /// disk-read path, it must use the daemon-backed API.
    /// `None` is reserved for `Removed` changes, where rules with
    /// `needs_content` skip evaluation per INTR-006.
    pub content: Option<Vec<u8>>,
}

/// A coalesced batch of proposed changes — the same shape an
/// agent's edit lands as in the daemon's watcher path, but
/// caller-provided rather than read from disk.
#[derive(Debug, Clone)]
pub struct ChangeBatch {
    pub changes: Vec<ProposedFileChange>,
}

impl ChangeBatch {
    #[must_use]
    pub fn new(changes: Vec<ProposedFileChange>) -> Self {
        Self { changes }
    }

    #[must_use]
    pub fn single(change: ProposedFileChange) -> Self {
        Self {
            changes: vec![change],
        }
    }
}

/// Synchronous embedded-mode evaluation. Takes a caller-provided
/// [`ChangeBatch`], the resolved config from INTD-008, and the
/// enforcement pipeline (carrying the rule registry); returns the
/// decision the daemon would have produced for the same input.
///
/// **Does not** read from disk, and **does not** fence / interrupt
/// — those are caller responsibilities. The embedded API is a
/// synchronous "here is what the rule engine says" answer; the
/// daemon's process-group / fence side effects only fire in
/// daemon-backed mode.
///
/// `pipeline` is taken by reference so callers (CI, MCP shim) can
/// keep a long-lived pipeline. `EnforcementPipeline::default()`
/// builds the canonical registry; tests that want to inject a
/// custom registry construct one directly.
#[must_use]
pub fn embedded_evaluate(
    batch: &ChangeBatch,
    config: &Resolved,
    pipeline: &EnforcementPipeline,
) -> EnforcementDecision {
    embedded_evaluate_with_diagnostics(batch, config, pipeline).decision
}

/// Variant of [`embedded_evaluate`] that also returns the
/// diagnostics the rule engine produced. Useful for callers that
/// want to surface diagnostics even when `observe_only` /
/// `Mode::Warn` would otherwise downgrade the decision to
/// `Allow`.
///
/// The diagnostic envelope is byte-identical to the daemon-backed
/// path on the same fixture — pinned by
/// [`tests::embedded_path_emits_same_envelope_as_daemon_path`].
/// Changing the diagnostics produced here without updating the
/// daemon side (or vice versa) breaks that contract test.
#[must_use]
pub fn embedded_evaluate_with_diagnostics(
    batch: &ChangeBatch,
    config: &Resolved,
    pipeline: &EnforcementPipeline,
) -> EmbeddedOutcome {
    let proposed: Vec<ProposedChange<'_>> = batch
        .changes
        .iter()
        .map(|change| ProposedChange {
            path: change.path.as_path(),
            change_kind: change.change_kind,
            content: change.content.as_deref(),
        })
        .collect();

    let raw_decision = pipeline.evaluate_proposed_changes(&proposed);
    let diagnostics = pipeline.diagnostics_for_proposed_changes(
        &proposed,
        // Mode::Unknown carries the embedded vocabulary on the wire;
        // it does not affect rule evaluation, only diagnostic
        // metadata. The embedded surface always emits the
        // `pre-write` mode label so MCP-shim parity holds.
        &DiagnosticMode::Unknown(EMBEDDED_MODE_LABEL.to_string()),
    );
    let decision = downgrade_decision_if_observe(raw_decision, config);
    EmbeddedOutcome {
        decision,
        diagnostics,
    }
}

/// Wire label for the embedded mode the diagnostic envelope
/// carries. Matches the existing `embedded_validate_pre_write`
/// helper in `anvil-cli` so the wire surface is consistent.
const EMBEDDED_MODE_LABEL: &str = "pre-write";

/// Result of [`embedded_evaluate_with_diagnostics`]. Carries both
/// the enforcement decision (post observe-only / mode downgrade)
/// and the diagnostic stream the rule engine produced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmbeddedOutcome {
    pub decision: EnforcementDecision,
    pub diagnostics: Vec<Diagnostic>,
}

fn downgrade_decision_if_observe(
    raw: EnforcementDecision,
    config: &Resolved,
) -> EnforcementDecision {
    // observe_only takes precedence: a workspace in dry-run never
    // surfaces a fence/interrupt regardless of mode.
    if config.observe_only {
        return enforce_allow_from(raw);
    }
    match config.mode {
        // Off / Warn: rules still ran, but the decision is always
        // Allow. Diagnostics flow on the side channel for operator
        // visibility. `Off` is a real posture since ADR-098 AD-3
        // (projects to always-`Allow`); it lands here alongside `Warn`
        // because the embedded pipeline never enforces — the daemon does.
        ConfigMode::Off | ConfigMode::Warn => enforce_allow_from(raw),
        // Fence / Interrupt: the pipeline output is what the
        // daemon would surface — caller decides how to enforce.
        // Embedded mode itself does not fence; the daemon does.
        ConfigMode::Fence | ConfigMode::Interrupt => raw,
    }
}

fn enforce_allow_from(decision: EnforcementDecision) -> EnforcementDecision {
    let affected_paths = match decision {
        EnforcementDecision::Allow { affected_paths }
        | EnforcementDecision::Interrupt(InterruptDecision { affected_paths, .. }) => {
            affected_paths
        }
    };
    EnforcementDecision::Allow { affected_paths }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{AmbiguousOwnership, Mode as ConfigMode, Resolved};
    use crate::enforcement::{ProposedChange, default_rule_registry};
    use anvil_intercept_rules::{ChangeKind, RuleRegistry};
    use anvil_kernel_types::Mode as DiagnosticMode;

    fn secret_change() -> ProposedFileChange {
        ProposedFileChange {
            path: PathBuf::from("src/secret.ts"),
            change_kind: ChangeKind::Modified,
            content: Some(b"const token = 'ghp_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa';\n".to_vec()),
        }
    }

    fn pipeline_default() -> EnforcementPipeline {
        EnforcementPipeline::new(default_rule_registry())
    }

    /// Compile-time hand-off used by
    /// `embedded_does_not_auto_promote_from_failed_daemon_path`.
    /// If either function's signature ever grows a daemon-failure
    /// parameter, the call below fails to compile because the
    /// argument type stops matching.
    fn take_signatures(
        _eval: fn(&ChangeBatch, &Resolved, &EnforcementPipeline) -> EnforcementDecision,
        _eval_diag: fn(&ChangeBatch, &Resolved, &EnforcementPipeline) -> EmbeddedOutcome,
    ) {
    }

    fn pipeline_empty() -> EnforcementPipeline {
        EnforcementPipeline::new(RuleRegistry::new())
    }

    /// Test (a): embedded mode emits diagnostics byte-identical to
    /// the daemon path. The daemon-backed parity test
    /// (`local_daemon_client_returns_scan_buffer_diagnostics_with_embedded_parity`)
    /// asserts the same property at the JSON-RPC boundary; this
    /// test asserts it at the library boundary so a regression
    /// surfaces here even if the wire test is skipped (Linux-only).
    #[test]
    fn embedded_path_emits_same_envelope_as_daemon_path() {
        let pipeline = pipeline_default();
        let change = secret_change();
        let batch = ChangeBatch::single(change.clone());
        // Default config: Mode::Warn, observe_only=false. We use
        // Mode::Fence so the pipeline returns an Interrupt
        // (otherwise warn-downgrade collapses it to Allow and
        // we cannot compare interrupt-shaped outputs).
        let config = Resolved {
            mode: ConfigMode::Fence,
            on_ambiguous_ownership: AmbiguousOwnership::Warn,
            observe_only: false,
            telemetry_allow_cross_session: false,
            ipc_limits: crate::dos::IpcLimits::default(),
            session_per_worktree_max: 16,
        };

        let outcome = embedded_evaluate_with_diagnostics(&batch, &config, &pipeline);

        // Reference: call the underlying pipeline directly with
        // the same proposed-content path the daemon uses (the
        // shared `evaluate_proposed_changes` plus
        // `diagnostics_for_proposed_changes` helpers). Embedded
        // mode must produce byte-identical diagnostics.
        let proposed = ProposedChange {
            path: change.path.as_path(),
            change_kind: change.change_kind,
            content: change.content.as_deref(),
        };
        let daemon_decision = pipeline.evaluate_proposed_changes(&[proposed]);
        let daemon_diagnostics = pipeline.diagnostics_for_proposed_changes(
            &[proposed],
            &DiagnosticMode::Unknown("pre-write".to_string()),
        );

        assert_eq!(
            outcome.decision, daemon_decision,
            "embedded decision must match the daemon path on the same fixture",
        );
        assert_eq!(
            outcome.diagnostics, daemon_diagnostics,
            "embedded diagnostics envelope must be byte-identical to the daemon path \
             (anvil.diagnostic.v1 parity contract)",
        );
        // Sanity: the secret fixture must produce at least one
        // diagnostic, otherwise the parity assertion is vacuous.
        assert!(
            !outcome.diagnostics.is_empty(),
            "secret fixture must produce diagnostics",
        );
        assert_eq!(outcome.diagnostics[0].source.rule_id, "secret-detection");
    }

    /// Test (b): embedded mode never auto-promotes from a failed
    /// daemon path. The API takes only the request and the
    /// resolved config; there is no `from_daemon_failure`
    /// constructor. The shape itself enforces the contract — this
    /// test pins it as a regression guard so a future refactor
    /// that adds such a constructor breaks here.
    #[test]
    fn embedded_does_not_auto_promote_from_failed_daemon_path() {
        // The function signature is `embedded_evaluate(&ChangeBatch,
        // &Resolved, &EnforcementPipeline)`. There is no parameter
        // that says "the daemon failed — fall back". This test
        // documents that contract.
        //
        // To make the regression guard mechanical, we assert the
        // function signature using a `fn` pointer assignment — if
        // the signature ever grows a daemon-failure-shaped
        // parameter, this assignment fails to compile. The
        // `take_signatures` call is the mechanical hand-off; the
        // call site discards the return without binding so clippy
        // does not flag a no-effect binding.
        take_signatures(embedded_evaluate, embedded_evaluate_with_diagnostics);

        // And the runtime path: even when the rule engine would
        // interrupt, embedded mode still returns. There is no
        // notion of "daemon failure" here — the embedded path
        // is the in-process answer, not a recovery surface.
        let pipeline = pipeline_default();
        let batch = ChangeBatch::single(secret_change());
        let config = Resolved {
            mode: ConfigMode::Fence,
            on_ambiguous_ownership: AmbiguousOwnership::Warn,
            observe_only: false,
            telemetry_allow_cross_session: false,
            ipc_limits: crate::dos::IpcLimits::default(),
            session_per_worktree_max: 16,
        };
        let decision = embedded_evaluate(&batch, &config, &pipeline);
        match decision {
            EnforcementDecision::Interrupt(_) => {}
            EnforcementDecision::Allow { .. } => {
                panic!("secret fixture must produce an Interrupt under Mode::Fence");
            }
        }
    }

    /// Test (c.1): `Mode::Warn` downgrades pipeline interrupts to
    /// `Allow` while preserving diagnostics. Honours INTD-008's
    /// `enforcement.mode` semantic.
    #[test]
    fn warn_mode_downgrades_interrupt_to_allow_but_keeps_diagnostics() {
        let pipeline = pipeline_default();
        let batch = ChangeBatch::single(secret_change());
        let config = Resolved {
            mode: ConfigMode::Warn,
            on_ambiguous_ownership: AmbiguousOwnership::Warn,
            observe_only: false,
            telemetry_allow_cross_session: false,
            ipc_limits: crate::dos::IpcLimits::default(),
            session_per_worktree_max: 16,
        };
        let outcome = embedded_evaluate_with_diagnostics(&batch, &config, &pipeline);
        match outcome.decision {
            EnforcementDecision::Allow { .. } => {}
            EnforcementDecision::Interrupt(_) => {
                panic!("Mode::Warn must downgrade an interrupt to Allow");
            }
        }
        assert!(
            !outcome.diagnostics.is_empty(),
            "Mode::Warn must still surface diagnostics on the side channel",
        );
    }

    /// Test (c.2): `observe_only: true` always returns `Allow`
    /// regardless of `mode`. Honours INTD-008's `observe_only`
    /// dry-run semantic.
    #[test]
    fn observe_only_returns_allow_regardless_of_mode() {
        let pipeline = pipeline_default();
        let batch = ChangeBatch::single(secret_change());
        for mode in [ConfigMode::Warn, ConfigMode::Fence, ConfigMode::Interrupt] {
            let config = Resolved {
                mode,
                on_ambiguous_ownership: AmbiguousOwnership::Warn,
                observe_only: true,
                telemetry_allow_cross_session: false,
                ipc_limits: crate::dos::IpcLimits::default(),
                session_per_worktree_max: 16,
            };
            let decision = embedded_evaluate(&batch, &config, &pipeline);
            assert!(
                matches!(decision, EnforcementDecision::Allow { .. }),
                "observe_only must return Allow under {mode:?}, got {decision:?}",
            );
        }
    }

    /// `Mode::Interrupt` keeps the pipeline result intact — embedded
    /// mode is the inspection surface; the caller decides how to
    /// enforce. Pinned because a future refactor that "helpfully"
    /// downgrades Interrupt → Fence here would silently weaken CI
    /// callers.
    #[test]
    fn interrupt_mode_propagates_pipeline_result_unchanged() {
        let pipeline = pipeline_default();
        let batch = ChangeBatch::single(secret_change());
        let config = Resolved {
            mode: ConfigMode::Interrupt,
            on_ambiguous_ownership: AmbiguousOwnership::Warn,
            observe_only: false,
            telemetry_allow_cross_session: false,
            ipc_limits: crate::dos::IpcLimits::default(),
            session_per_worktree_max: 16,
        };
        let decision = embedded_evaluate(&batch, &config, &pipeline);
        assert!(
            matches!(decision, EnforcementDecision::Interrupt(_)),
            "Mode::Interrupt must propagate the pipeline interrupt verbatim",
        );
    }

    /// An empty rule registry returns Allow with the affected
    /// paths populated. Pinned because it documents the
    /// "embedded mode never invents diagnostics" property.
    #[test]
    fn empty_registry_returns_allow_with_affected_paths() {
        let pipeline = pipeline_empty();
        let path = PathBuf::from("noop.rs");
        let batch = ChangeBatch::single(ProposedFileChange {
            path: path.clone(),
            change_kind: ChangeKind::Modified,
            content: Some(b"// nothing to see here\n".to_vec()),
        });
        let config = Resolved {
            mode: ConfigMode::Fence,
            on_ambiguous_ownership: AmbiguousOwnership::Warn,
            observe_only: false,
            telemetry_allow_cross_session: false,
            ipc_limits: crate::dos::IpcLimits::default(),
            session_per_worktree_max: 16,
        };
        let outcome = embedded_evaluate_with_diagnostics(&batch, &config, &pipeline);
        match outcome.decision {
            EnforcementDecision::Allow { affected_paths } => {
                assert_eq!(affected_paths, vec![path]);
            }
            EnforcementDecision::Interrupt(_) => panic!("empty registry cannot interrupt"),
        }
        assert!(outcome.diagnostics.is_empty());
    }

    /// `Path::starts_with` reaches the `EnforcementPipeline` even
    /// for `Removed` changes — content rules skip them per
    /// INTR-006 and only path-based rules see the change. Pinned
    /// because the embedded API must not require callers to fake
    /// content for delete events.
    #[test]
    fn removed_change_with_no_content_passes_path_rules_only() {
        let pipeline = pipeline_default();
        let path = PathBuf::from("removed.rs");
        let batch = ChangeBatch::single(ProposedFileChange {
            path: path.clone(),
            change_kind: ChangeKind::Removed,
            content: None,
        });
        let config = Resolved {
            mode: ConfigMode::Fence,
            on_ambiguous_ownership: AmbiguousOwnership::Warn,
            observe_only: false,
            telemetry_allow_cross_session: false,
            ipc_limits: crate::dos::IpcLimits::default(),
            session_per_worktree_max: 16,
        };
        let decision = embedded_evaluate(&batch, &config, &pipeline);
        // The default registry only contains content-bearing rules
        // (secret detection + reasoning pattern) so a Removed
        // event with no content cannot trigger them.
        assert!(matches!(decision, EnforcementDecision::Allow { .. }));
    }
}
