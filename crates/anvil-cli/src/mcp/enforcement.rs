//! RTAI-006: enforcement-mode policy for the validate-write tool.
//!
//! INTD-008 will eventually own the project-level `.anvil.yaml` config
//! loader for the daemon. Until then, this module reads the same
//! `enforcement.mode` field directly from the workspace root so the MCP
//! launch shim can honour the contract. When INTD-008 lands, this loader
//! collapses into a thin wrapper over the daemon-shared loader and the
//! semantics here become the canonical client-side mapping table.
//!
//! # Severity-to-decision mapping
//!
//! | Mode    | Diagnostics observed         | Tool decision | `safeDefault`     |
//! | ------- | ---------------------------- | ------------- | ----------------- |
//! | `block` | Any `error`                  | `block`       | `do-not-write`    |
//! | `block` | Mix of `error` and lower     | `block`       | `do-not-write`    |
//! | `block` | Only `warning`/`info`/unknown | `warn`       | (none)            |
//! | `block` | None                         | `allow`       | (none)            |
//! | `warn`  | Any (any severity)           | `warn`        | (none)            |
//! | `warn`  | None                         | `allow`       | (none)            |
//! | `off`   | Any (any severity)           | `allow`       | (none)            |
//! | `off`   | None                         | `allow`       | (none)            |
//!
//! An `Unknown` severity (a value a newer producer emitted; ADR-096) is
//! treated as a warning here: it is not an `error`, so in `block` mode it
//! warns rather than blocking — surfaced, never silently allowed, never
//! over-blocked on a value this consumer cannot interpret.
//!
//! The mixed-severity row makes the `any(Error)` short-circuit explicit:
//! in `block` mode, a single `error` rejects the write even when other
//! diagnostics are lower severity. This matches `decision_for` below.
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

/// Workspace-level enforcement mode for the MCP `validate_write` tool.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EnforcementMode {
    /// Diagnostics with severity `error` reject the write. Lower
    /// severities warn. Default — matches the pre-RTAI-006 behaviour.
    #[default]
    Block,
    /// Diagnostics never reject the write; anything non-empty maps to
    /// `warn`. The agent still sees the diagnostics.
    Warn,
    /// Diagnostics are returned but never affect the decision; the
    /// response is always `allow`. Useful when an operator wants the
    /// agent to see findings without blocking the flow.
    Off,
}

impl EnforcementMode {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Block => "block",
            Self::Warn => "warn",
            Self::Off => "off",
        }
    }

    /// Parse the canonical `.anvil.yaml` `enforcement.mode` value.
    /// Unknown / missing values fall back to the default (`block`). The
    /// loader logs nothing — the MCP server stdout is reserved for
    /// JSON-RPC frames, so config drift is communicated by the agent
    /// observing the `correlation.enforcementMode` field on the
    /// response rather than by a side-channel warning.
    ///
    /// Canonical RTAI-006 terms are `block` / `warn` / `off`; the
    /// aliases `interrupt` / `fence` (→ `Block`) and `advisory` /
    /// `proceed` (→ `Off`) are accepted for forward compatibility with
    /// INTD-008's planned vocabulary (per AD-3 / INTD-008 plan). When
    /// INTD-008 lands and a shared loader replaces this stub, the
    /// canonical-term direction may invert (i.e. `interrupt` becomes
    /// the canonical name and `block` the alias). The alias-direction
    /// reversal will be picked up by the shared loader; callers of
    /// `EnforcementMode` see no churn because the variants stay the
    /// same.
    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "block" | "interrupt" | "fence" => Some(Self::Block),
            "warn" => Some(Self::Warn),
            "off" | "advisory" | "proceed" => Some(Self::Off),
            _ => None,
        }
    }
}

/// Apply the enforcement-mode policy to a diagnostic set.
#[must_use]
pub fn decision_for(diagnostics: &[Diagnostic], mode: EnforcementMode) -> ControlDecision {
    match mode {
        EnforcementMode::Off => ControlDecision::Allow,
        EnforcementMode::Warn => {
            if diagnostics.is_empty() {
                ControlDecision::Allow
            } else {
                ControlDecision::Warn
            }
        }
        EnforcementMode::Block => {
            if diagnostics
                .iter()
                .any(|diagnostic| diagnostic.severity == Severity::Error)
            {
                ControlDecision::Block
            } else if diagnostics.is_empty() {
                ControlDecision::Allow
            } else {
                ControlDecision::Warn
            }
        }
    }
}

/// Resolve the enforcement mode for a workspace.
///
/// Reads `<workspace_root>/.anvil.yaml` via the shared
/// `anvil_intercept_proto::enforcement_config::AnvilConfigFile`
/// deserialiser (INTD-008 owns the wire shape so the daemon and the
/// MCP shim cannot drift on which keys / aliases are accepted).
/// Missing file, missing field, unparseable YAML, and unknown mode
/// strings all fall back to `EnforcementMode::default()` (`block`) —
/// the MCP shim's longstanding fail-closed default. INTD-008's
/// daemon-side loader surfaces a structured `LoadError` on malformed
/// YAML; the MCP shim's stdout is reserved for JSON-RPC frames so we
/// keep the silent-fallback behaviour that pre-dates the shared
/// proto extraction.
#[must_use]
pub fn load_for_workspace(workspace_root: &Path) -> EnforcementMode {
    let path = anvil_yaml_path(workspace_root);
    let Ok(content) = fs::read_to_string(&path) else {
        return EnforcementMode::default();
    };
    let Ok(config) = serde_yaml::from_str::<AnvilConfigFile>(&content) else {
        return EnforcementMode::default();
    };
    config
        .enforcement
        .mode
        .as_deref()
        .and_then(EnforcementMode::parse)
        .unwrap_or_default()
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
    fn default_mode_is_block() {
        assert_eq!(EnforcementMode::default(), EnforcementMode::Block);
    }

    #[test]
    fn parses_canonical_mode_strings() {
        assert_eq!(
            EnforcementMode::parse("block"),
            Some(EnforcementMode::Block)
        );
        assert_eq!(EnforcementMode::parse("warn"), Some(EnforcementMode::Warn));
        assert_eq!(EnforcementMode::parse("off"), Some(EnforcementMode::Off));
    }

    #[test]
    fn parse_is_case_insensitive_and_trims_whitespace() {
        assert_eq!(
            EnforcementMode::parse("  BLOCK  "),
            Some(EnforcementMode::Block)
        );
        assert_eq!(EnforcementMode::parse("Warn"), Some(EnforcementMode::Warn));
    }

    #[test]
    fn parse_accepts_intd_008_aliases() {
        // INTD-008's enforcement vocabulary uses `interrupt`/`fence`
        // for stop-the-write semantics; map them onto our `block`.
        assert_eq!(
            EnforcementMode::parse("interrupt"),
            Some(EnforcementMode::Block)
        );
        assert_eq!(
            EnforcementMode::parse("fence"),
            Some(EnforcementMode::Block)
        );
        // `advisory` / `proceed` are the Off-direction aliases. They
        // map to `Off` so an INTD-008 config using these terms still
        // resolves to "report findings, never block" semantics.
        assert_eq!(
            EnforcementMode::parse("advisory"),
            Some(EnforcementMode::Off)
        );
        assert_eq!(
            EnforcementMode::parse("proceed"),
            Some(EnforcementMode::Off)
        );
        // The parse is case-folded and trimmed (see
        // `parse_is_case_insensitive_and_trims_whitespace`); confirm
        // the alias direction inherits that property.
        assert_eq!(
            EnforcementMode::parse("ADVISORY"),
            Some(EnforcementMode::Off)
        );
        assert_eq!(
            EnforcementMode::parse("  Interrupt  "),
            Some(EnforcementMode::Block)
        );
    }

    #[test]
    fn parse_unknown_returns_none() {
        assert_eq!(EnforcementMode::parse("nope"), None);
        assert_eq!(EnforcementMode::parse(""), None);
        // Typo near-misses must not silently match. `interupt` (single
        // `r`) is a common misspelling of the `interrupt` alias; it
        // returns None and the caller falls back to the default
        // (`block`) per `load_for_workspace`.
        assert_eq!(EnforcementMode::parse("interupt"), None);
        assert_eq!(EnforcementMode::parse("blok"), None);
    }

    #[test]
    fn block_mode_blocks_on_error() {
        let diagnostics = [diagnostic(Severity::Error)];
        assert_eq!(
            decision_for(&diagnostics, EnforcementMode::Block),
            ControlDecision::Block
        );
    }

    #[test]
    fn block_mode_warns_on_warning_only() {
        let diagnostics = [diagnostic(Severity::Warning)];
        assert_eq!(
            decision_for(&diagnostics, EnforcementMode::Block),
            ControlDecision::Warn
        );
    }

    #[test]
    fn block_mode_warns_on_info_only() {
        let diagnostics = [diagnostic(Severity::Info)];
        assert_eq!(
            decision_for(&diagnostics, EnforcementMode::Block),
            ControlDecision::Warn
        );
    }

    #[test]
    fn block_mode_warns_on_unknown_severity_only() {
        // ADR-096: an `Unknown` severity (newer producer) is treated as a
        // warning — in block mode it warns, never blocks (it is not `Error`)
        // and never silently allows. Pins the equality-check behaviour so a
        // future refactor to an exhaustive match cannot regress it.
        let diagnostics = [diagnostic(Severity::Unknown)];
        assert_eq!(
            decision_for(&diagnostics, EnforcementMode::Block),
            ControlDecision::Warn
        );
    }

    #[test]
    fn block_mode_still_blocks_when_error_accompanies_unknown() {
        // An `Error` alongside an `Unknown` still blocks — `Unknown` does not
        // mask a genuine error.
        let diagnostics = [diagnostic(Severity::Unknown), diagnostic(Severity::Error)];
        assert_eq!(
            decision_for(&diagnostics, EnforcementMode::Block),
            ControlDecision::Block
        );
    }

    #[test]
    fn block_mode_allows_when_clean() {
        assert_eq!(
            decision_for(&[], EnforcementMode::Block),
            ControlDecision::Allow
        );
    }

    #[test]
    fn warn_mode_never_blocks_even_on_error() {
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
    fn missing_anvil_yaml_falls_back_to_block() {
        let workspace = tempdir().expect("workspace exists");
        assert_eq!(load_for_workspace(workspace.path()), EnforcementMode::Block);
    }

    #[test]
    fn loads_block_mode_from_anvil_yaml() {
        let workspace = tempdir().expect("workspace exists");
        fs::write(
            workspace.path().join(".anvil.yaml"),
            "enforcement:\n  mode: block\n",
        )
        .expect("write fixture");
        assert_eq!(load_for_workspace(workspace.path()), EnforcementMode::Block);
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
    fn unknown_mode_string_falls_back_to_block() {
        let workspace = tempdir().expect("workspace exists");
        fs::write(
            workspace.path().join(".anvil.yaml"),
            "enforcement:\n  mode: lenient\n",
        )
        .expect("write fixture");
        assert_eq!(load_for_workspace(workspace.path()), EnforcementMode::Block);
    }

    #[test]
    fn malformed_yaml_falls_back_to_block() {
        let workspace = tempdir().expect("workspace exists");
        fs::write(workspace.path().join(".anvil.yaml"), "this is not: yaml: [")
            .expect("write fixture");
        assert_eq!(load_for_workspace(workspace.path()), EnforcementMode::Block);
    }

    #[test]
    fn missing_enforcement_section_falls_back_to_block() {
        let workspace = tempdir().expect("workspace exists");
        fs::write(workspace.path().join(".anvil.yaml"), "version: 1\n").expect("write fixture");
        assert_eq!(load_for_workspace(workspace.path()), EnforcementMode::Block);
    }
}
