//! RTAI-006: enforcement-mode policy for the validate-write tool.
//!
//! INTD-008 will eventually own the project-level `.anvil.yaml` config
//! loader for the daemon. Until then, this module reads the same
//! `enforcement.mode` field directly from the workspace root so the MCP
//! launch shim can honour the contract. When INTD-008 lands, this loader
//! collapses into a thin wrapper over the daemon-shared loader and the
//! semantics here become the canonical client-side mapping table.
//!
//! # The shared posture (ADR-098 AD-3)
//!
//! Since ADR-098 AD-3 the MCP shim shares
//! [`anvil_kernel_types::EnforcementMode`] with the daemon — the old
//! MCP-local `EnforcementMode { Block, Warn, Off }` is gone. Crucially,
//! the lossy `fence`/`interrupt` → `Block` collapse the shim used to do
//! **at parse time** is removed: parsing keeps every posture distinct,
//! and the veto is projected **at action time**. `decision_for` records
//! the *true* decision (`fence` stays `fence`); the write-refusal is then
//! gated on [`ControlDecision::is_veto`] (`Block | Fence | Interrupt`).
//!
//! # Severity-to-decision mapping
//!
//! `V` is the posture's veto decision ([`EnforcementMode::escalated_decision`]):
//! `Fence → fence`, `Interrupt → interrupt`. The MCP default posture is
//! `Interrupt` (the `block` alias) — a no-config workspace vetoes writes
//! that carry an `error`, exactly as the pre-AD-3 `block` default did,
//! but now records the true `interrupt` decision.
//!
//! | Mode        | Diagnostics observed          | Tool decision | `safeDefault`  |
//! | ----------- | ----------------------------- | ------------- | -------------- |
//! | `fence`/`interrupt` | Any `error`           | `V` (veto)    | `do-not-write` |
//! | `fence`/`interrupt` | Only `warning`/`info`/unknown | `warn` | (none)         |
//! | `fence`/`interrupt` | None                  | `allow`       | (none)         |
//! | `warn`      | Any (any severity)            | `warn`        | (none)         |
//! | `warn`      | None                          | `allow`       | (none)         |
//! | `off`       | Any (any severity)            | `allow`       | (none)         |
//! | `off`       | None                          | `allow`       | (none)         |
//!
//! An `Unknown` severity (a value a newer producer emitted; ADR-096) is
//! treated as a warning here: it is not an `error`, so under a veto
//! posture it warns rather than vetoing — surfaced, never silently
//! allowed, never over-blocked on a value this consumer cannot interpret.
//!
//! A single `error` vetoes the write even when other diagnostics are
//! lower severity (`any(Error)` short-circuit). This matches
//! `decision_for` below.
//!
//! Diagnostics are always returned to the caller verbatim, regardless of
//! mode — only the decision flag changes. This matches RTAI's
//! "errors-as-first-class" contract: a `warn` or `off` response still
//! carries the structured diagnostics so the agent can surface them.

use std::fs;
use std::path::{Path, PathBuf};

use anvil_intercept_proto::enforcement_config::AnvilConfigFile;
use anvil_kernel_types::Diagnostic;
use anvil_kernel_types::Severity;
use anvil_kernel_types::diagnostics::ControlDecision;
// ADR-098 AD-3: the MCP shim shares the daemon's posture type. The
// MCP-local `EnforcementMode { Block, Warn, Off }` is retired; re-exported
// here so existing `mcp::enforcement::EnforcementMode` call sites resolve.
pub use anvil_kernel_types::EnforcementMode;

/// The MCP shim's no-config / input-error posture (ADR-098 AD-3).
///
/// The pre-AD-3 shim defaulted to `block` — veto any write carrying an
/// `error`. In the unified vocabulary `block` is an alias for
/// [`EnforcementMode::Interrupt`], so a no-config workspace (and any
/// input-error path that must answer before a posture is resolved) uses
/// `Interrupt`, preserving the historical veto-on-error default. Kept
/// distinct from [`EnforcementMode::default`] (`Warn`, ADR-002): the
/// shared type's default is the daemon-aligned posture, while the MCP
/// surface carries its own stricter fallback.
pub const MCP_DEFAULT_ENFORCEMENT: EnforcementMode = EnforcementMode::Interrupt;

/// Apply the enforcement-mode policy to a diagnostic set, recording the
/// *true* [`ControlDecision`] for the posture (ADR-098 AD-3). Under a
/// veto posture (`fence` / `interrupt`) an `error` escalates to that
/// posture's own decision via [`EnforcementMode::escalated_decision`] —
/// `fence` stays `fence`, with no collapse to `block`. The write-refusal
/// is projected downstream from [`ControlDecision::is_veto`].
#[must_use]
pub fn decision_for(diagnostics: &[Diagnostic], mode: EnforcementMode) -> ControlDecision {
    // `off` never affects the decision; the response is always `allow`.
    if mode == EnforcementMode::Off || diagnostics.is_empty() {
        return ControlDecision::Allow;
    }
    if diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity == Severity::Error)
    {
        // Escalate to the posture's true veto decision. For `warn` this
        // caps at `Warn`; for `fence`/`interrupt` it is the namesake veto.
        mode.escalated_decision()
    } else {
        // Non-error findings warn under any non-`off` posture.
        ControlDecision::Warn
    }
}

/// Resolve the enforcement mode for a workspace.
///
/// Reads `<workspace_root>/.anvil.yaml` via the shared
/// `anvil_intercept_proto::enforcement_config::AnvilConfigFile`
/// deserialiser (INTD-008 owns the wire shape so the daemon and the
/// MCP shim cannot drift on which keys / aliases are accepted).
/// Missing file, missing field, unparseable YAML, and unknown mode
/// strings all fall back to [`MCP_DEFAULT_ENFORCEMENT`] (`interrupt`,
/// the `block` alias) — the MCP shim's longstanding fail-closed
/// veto-on-error default, now recording the true decision. INTD-008's
/// daemon-side loader surfaces a structured `LoadError` on malformed
/// YAML; the MCP shim's stdout is reserved for JSON-RPC frames so we
/// keep the silent-fallback behaviour that pre-dates the shared
/// proto extraction.
#[must_use]
pub fn load_for_workspace(workspace_root: &Path) -> EnforcementMode {
    let path = anvil_yaml_path(workspace_root);
    let Ok(content) = fs::read_to_string(&path) else {
        return MCP_DEFAULT_ENFORCEMENT;
    };
    let Ok(config) = serde_yaml::from_str::<AnvilConfigFile>(&content) else {
        return MCP_DEFAULT_ENFORCEMENT;
    };
    config
        .enforcement
        .mode
        .as_deref()
        .and_then(EnforcementMode::parse)
        .unwrap_or(MCP_DEFAULT_ENFORCEMENT)
}

fn anvil_yaml_path(workspace_root: &Path) -> PathBuf {
    workspace_root.join(".anvil.yaml")
}

#[cfg(test)]
mod tests {
    use super::*;
    use anvil_kernel_types::{Category, Diagnostic, DiagnosticSource, Location, Mode};
    use std::fs;
    use tempfile::tempdir;

    fn diagnostic(severity: Severity) -> Diagnostic {
        Diagnostic::new(
            "diag_test",
            severity,
            "test diagnostic",
            Location {
                file: "src/example.ts".to_string(),
                line: Some(1),
                column: None,
                end_line: None,
                end_column: None,
            },
            Category::Other,
            DiagnosticSource {
                rule_id: "test-rule".to_string(),
                source_module: "test".to_string(),
            },
            Mode::Unknown("pre-write".to_string()),
        )
    }

    #[test]
    fn mcp_default_is_interrupt_veto() {
        // ADR-098 AD-3: the MCP surface's no-config fallback is the
        // `block` alias — `Interrupt` — preserving the pre-AD-3
        // veto-on-error default while recording the true decision. This
        // is distinct from the shared type's `Default` (`Warn`, ADR-002).
        assert_eq!(MCP_DEFAULT_ENFORCEMENT, EnforcementMode::Interrupt);
        assert_eq!(EnforcementMode::default(), EnforcementMode::Warn);
    }

    #[test]
    fn parses_canonical_mode_strings_distinctly() {
        // ADR-098 AD-3: parsing keeps every posture distinct — no
        // parse-time collapse of `fence`/`interrupt` onto a single veto.
        assert_eq!(EnforcementMode::parse("off"), Some(EnforcementMode::Off));
        assert_eq!(EnforcementMode::parse("warn"), Some(EnforcementMode::Warn));
        assert_eq!(
            EnforcementMode::parse("fence"),
            Some(EnforcementMode::Fence)
        );
        assert_eq!(
            EnforcementMode::parse("interrupt"),
            Some(EnforcementMode::Interrupt)
        );
    }

    #[test]
    fn parse_no_longer_collapses_fence_and_interrupt() {
        // The pre-AD-3 shim collapsed `fence` and `interrupt` onto its
        // (retired) `Block` variant at parse time. They are now distinct
        // postures; `block` is the alias for the strictest (`interrupt`).
        assert_ne!(
            EnforcementMode::parse("fence"),
            EnforcementMode::parse("interrupt")
        );
        assert_eq!(
            EnforcementMode::parse("block"),
            Some(EnforcementMode::Interrupt)
        );
        assert_eq!(
            EnforcementMode::parse("  Interrupt  "),
            Some(EnforcementMode::Interrupt)
        );
        // `advisory` / `proceed` join `off` (a real posture now).
        assert_eq!(
            EnforcementMode::parse("advisory"),
            Some(EnforcementMode::Off)
        );
        assert_eq!(
            EnforcementMode::parse("PROCEED"),
            Some(EnforcementMode::Off)
        );
    }

    #[test]
    fn parse_unknown_returns_none() {
        assert_eq!(EnforcementMode::parse("nope"), None);
        assert_eq!(EnforcementMode::parse(""), None);
        assert_eq!(EnforcementMode::parse("interupt"), None);
        assert_eq!(EnforcementMode::parse("blok"), None);
    }

    #[test]
    fn interrupt_mode_vetoes_on_error_with_true_decision() {
        // Default MCP posture (`interrupt`): an error escalates to the
        // true `interrupt` decision, not a collapsed `block`.
        let diagnostics = [diagnostic(Severity::Error)];
        let decision = decision_for(&diagnostics, EnforcementMode::Interrupt);
        assert_eq!(decision, ControlDecision::Interrupt);
        assert!(decision.is_veto());
    }

    #[test]
    fn fence_mode_vetoes_on_error_and_fence_stays_fence() {
        // ADR-098 AD-3 regression: a `fence` posture records the true
        // `fence` decision (no collapse to `block`) and is still a veto.
        let diagnostics = [diagnostic(Severity::Error)];
        let decision = decision_for(&diagnostics, EnforcementMode::Fence);
        assert_eq!(decision, ControlDecision::Fence);
        assert!(decision.is_veto());
    }

    #[test]
    fn veto_modes_warn_on_non_error_only() {
        for mode in [EnforcementMode::Fence, EnforcementMode::Interrupt] {
            assert_eq!(
                decision_for(&[diagnostic(Severity::Warning)], mode),
                ControlDecision::Warn
            );
            assert_eq!(
                decision_for(&[diagnostic(Severity::Info)], mode),
                ControlDecision::Warn
            );
            // ADR-096: an `Unknown` severity warns (it is not `Error`),
            // never vetoes and never silently allows.
            assert_eq!(
                decision_for(&[diagnostic(Severity::Unknown)], mode),
                ControlDecision::Warn
            );
        }
    }

    #[test]
    fn veto_modes_still_veto_when_error_accompanies_unknown() {
        // An `Error` alongside an `Unknown` still vetoes — `Unknown` does
        // not mask a genuine error.
        let diagnostics = [diagnostic(Severity::Unknown), diagnostic(Severity::Error)];
        assert_eq!(
            decision_for(&diagnostics, EnforcementMode::Interrupt),
            ControlDecision::Interrupt
        );
        assert_eq!(
            decision_for(&diagnostics, EnforcementMode::Fence),
            ControlDecision::Fence
        );
    }

    #[test]
    fn veto_modes_allow_when_clean() {
        assert_eq!(
            decision_for(&[], EnforcementMode::Interrupt),
            ControlDecision::Allow
        );
        assert_eq!(
            decision_for(&[], EnforcementMode::Fence),
            ControlDecision::Allow
        );
    }

    #[test]
    fn warn_mode_never_vetoes_even_on_error() {
        let diagnostics = [diagnostic(Severity::Error)];
        assert_eq!(
            decision_for(&diagnostics, EnforcementMode::Warn),
            ControlDecision::Warn
        );
    }

    #[test]
    fn warn_mode_allows_when_clean() {
        assert_eq!(
            decision_for(&[], EnforcementMode::Warn),
            ControlDecision::Allow
        );
    }

    #[test]
    fn off_mode_always_allows() {
        let diagnostics = [diagnostic(Severity::Error)];
        assert_eq!(
            decision_for(&diagnostics, EnforcementMode::Off),
            ControlDecision::Allow
        );
        assert_eq!(
            decision_for(&[], EnforcementMode::Off),
            ControlDecision::Allow
        );
    }

    #[test]
    fn missing_anvil_yaml_falls_back_to_mcp_default() {
        let workspace = tempdir().expect("workspace exists");
        assert_eq!(
            load_for_workspace(workspace.path()),
            MCP_DEFAULT_ENFORCEMENT
        );
    }

    #[test]
    fn loads_block_alias_as_interrupt_from_anvil_yaml() {
        let workspace = tempdir().expect("workspace exists");
        fs::write(
            workspace.path().join(".anvil.yaml"),
            "enforcement:\n  mode: block\n",
        )
        .expect("write fixture");
        assert_eq!(
            load_for_workspace(workspace.path()),
            EnforcementMode::Interrupt
        );
    }

    #[test]
    fn loads_fence_mode_from_anvil_yaml() {
        let workspace = tempdir().expect("workspace exists");
        fs::write(
            workspace.path().join(".anvil.yaml"),
            "enforcement:\n  mode: fence\n",
        )
        .expect("write fixture");
        assert_eq!(load_for_workspace(workspace.path()), EnforcementMode::Fence);
    }

    #[test]
    fn loads_warn_mode_from_anvil_yaml() {
        let workspace = tempdir().expect("workspace exists");
        fs::write(
            workspace.path().join(".anvil.yaml"),
            "enforcement:\n  mode: warn\n",
        )
        .expect("write fixture");
        assert_eq!(load_for_workspace(workspace.path()), EnforcementMode::Warn);
    }

    #[test]
    fn loads_off_mode_from_anvil_yaml() {
        let workspace = tempdir().expect("workspace exists");
        fs::write(
            workspace.path().join(".anvil.yaml"),
            "enforcement:\n  mode: off\n",
        )
        .expect("write fixture");
        assert_eq!(load_for_workspace(workspace.path()), EnforcementMode::Off);
    }

    #[test]
    fn unknown_mode_string_falls_back_to_mcp_default() {
        let workspace = tempdir().expect("workspace exists");
        fs::write(
            workspace.path().join(".anvil.yaml"),
            "enforcement:\n  mode: lenient\n",
        )
        .expect("write fixture");
        assert_eq!(
            load_for_workspace(workspace.path()),
            MCP_DEFAULT_ENFORCEMENT
        );
    }

    #[test]
    fn malformed_yaml_falls_back_to_mcp_default() {
        let workspace = tempdir().expect("workspace exists");
        fs::write(workspace.path().join(".anvil.yaml"), "this is not: yaml: [")
            .expect("write fixture");
        assert_eq!(
            load_for_workspace(workspace.path()),
            MCP_DEFAULT_ENFORCEMENT
        );
    }

    #[test]
    fn missing_enforcement_section_falls_back_to_mcp_default() {
        let workspace = tempdir().expect("workspace exists");
        fs::write(workspace.path().join(".anvil.yaml"), "version: 1\n").expect("write fixture");
        assert_eq!(
            load_for_workspace(workspace.path()),
            MCP_DEFAULT_ENFORCEMENT
        );
    }
}
