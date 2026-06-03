//! DSV-005 Task 8: the save-time `validate_paths` verdict path.
//!
//! This module assembles the [`ValidatePathsResponse`] the daemon returns for a
//! client's change set: classify each path, certify it against the warm
//! `(SymbolGraph, DependencyGraph)` cache, run the antipattern family over the
//! openat2-guarded bytes, and fold the result into the workspace assurance
//! state. The orchestration that wires those steps to the live daemon (auth,
//! the guarded read, the per-worktree cache, the ipc dispatch arm) lands on top
//! of the pure pieces here.
//!
//! This first slice provides the **boundary mappings** between the
//! graph-cache-local / wire vocabularies — the translations DSV-004 deferred to
//! "the daemon boundary (DSV-005)". They are pure and exhaustively tested so the
//! verdict assembly built on them cannot silently mistranslate a reason (a
//! mistranslation on the verdict path is a B2 false-attestation hazard).

use std::path::PathBuf;

use anvil_checks::antipattern::types::{Warning, WarningSeverity};
use anvil_graph_cache::certify::{CertifyStale, ChangeKind};
use anvil_intercept_proto::protocol::{ChangeKindWire, StaleReason};
use anvil_kernel_types::diagnostics::KnownMode;
use anvil_kernel_types::{Category, Diagnostic, DiagnosticSource, Location, Mode, Severity};

use crate::change_class::CanonicalChange;

/// `source_module` stamped on every save-time antipattern diagnostic.
const ANTIPATTERN_SOURCE_MODULE: &str = "anvil-checks::antipattern";

/// Map a wire [`ChangeKindWire`] to the daemon's internal [`CanonicalChange`]
/// (the taxonomy input). A rename carries its prior path through.
#[must_use]
pub fn canonical_change(wire: &ChangeKindWire) -> CanonicalChange {
    match wire {
        ChangeKindWire::Created => CanonicalChange::Create,
        ChangeKindWire::Modified => CanonicalChange::ContentModify,
        ChangeKindWire::Deleted => CanonicalChange::Delete,
        ChangeKindWire::Renamed { from } => CanonicalChange::Rename {
            from: PathBuf::from(from),
        },
    }
}

/// Map a wire [`ChangeKindWire`] to the graph-cache [`ChangeKind`] certify
/// consumes. The rename's `from` path is irrelevant to certifiability and is
/// dropped (cf. [`ChangeKind`]).
#[must_use]
pub fn certify_change_kind(wire: &ChangeKindWire) -> ChangeKind {
    match wire {
        ChangeKindWire::Created => ChangeKind::Create,
        ChangeKindWire::Modified => ChangeKind::ContentModify,
        ChangeKindWire::Deleted => ChangeKind::Delete,
        ChangeKindWire::Renamed { .. } => ChangeKind::Rename,
    }
}

/// Map a graph-cache-local [`CertifyStale`] to the wire [`StaleReason`] (the
/// translation DSV-004 left to "the daemon boundary").
///
/// Direct counterparts map 1:1. Two graph-cache reasons have no dedicated wire
/// variant, so DSV-005 chooses the closest fail-safe wire reason:
/// - [`CertifyStale::ExportSurfaceChange`] → [`StaleReason::CrossFileResolutionNeeded`]:
///   a public/privileged surface change means importers must be revalidated —
///   exactly "needs cross-file resolution the warm cache cannot supply".
/// - [`CertifyStale::UnreliableGraph`] → [`StaleReason::CrossFileResolutionNeeded`]:
///   a failed `update_file` leaves the warm state untrustworthy, so the verdict
///   conservatively asks for a fresh cross-file resolution rather than claiming
///   a more specific cause it cannot stand behind.
#[must_use]
pub fn wire_stale_reason(reason: CertifyStale) -> StaleReason {
    match reason {
        CertifyStale::ImpactSetOverflow => StaleReason::ImpactSetOverflow,
        CertifyStale::Deleted => StaleReason::Deleted,
        CertifyStale::Renamed => StaleReason::Renamed,
        CertifyStale::CrossFileResolutionNeeded => StaleReason::CrossFileResolutionNeeded,
        CertifyStale::ExportSurfaceChange | CertifyStale::UnreliableGraph => {
            StaleReason::CrossFileResolutionNeeded
        }
    }
}

/// Map an antipattern [`WarningSeverity`] to the canonical diagnostic
/// [`Severity`].
#[must_use]
pub fn diagnostic_severity(severity: WarningSeverity) -> Severity {
    match severity {
        WarningSeverity::Error => Severity::Error,
        WarningSeverity::Warning => Severity::Warning,
        WarningSeverity::Info => Severity::Info,
    }
}

/// Convert one antipattern [`Warning`] into a canonical save-time
/// [`Diagnostic`] (`anvil.diagnostic.v1`, category `Antipattern`, mode
/// `SaveTime`). The warning's suggestion, when non-empty, rides as the
/// remediation hint.
#[must_use]
pub fn antipattern_diagnostic(warning: &Warning) -> Diagnostic {
    let location = Location {
        file: warning.location.file.clone(),
        line: u32::try_from(warning.location.line).ok(),
        column: warning.location.column.and_then(|c| u32::try_from(c).ok()),
        end_line: warning
            .location
            .end_line
            .and_then(|l| u32::try_from(l).ok()),
        end_column: warning
            .location
            .end_column
            .and_then(|c| u32::try_from(c).ok()),
    };
    let diagnostic = Diagnostic::new(
        warning.id.clone(),
        diagnostic_severity(warning.severity),
        warning.message.clone(),
        location,
        Category::Antipattern,
        DiagnosticSource {
            rule_id: warning.id.clone(),
            source_module: ANTIPATTERN_SOURCE_MODULE.to_string(),
        },
        Mode::known(KnownMode::SaveTime),
    );
    if warning.suggestion.trim().is_empty() {
        diagnostic
    } else {
        diagnostic.with_remediation_hint(warning.suggestion.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anvil_checks::antipattern::types::{Confidence, Location as WarnLocation, WarningCategory};

    #[test]
    fn canonical_change_maps_every_wire_kind() {
        assert_eq!(
            canonical_change(&ChangeKindWire::Created),
            CanonicalChange::Create
        );
        assert_eq!(
            canonical_change(&ChangeKindWire::Modified),
            CanonicalChange::ContentModify
        );
        assert_eq!(
            canonical_change(&ChangeKindWire::Deleted),
            CanonicalChange::Delete
        );
        assert_eq!(
            canonical_change(&ChangeKindWire::Renamed {
                from: "old/path.ts".to_string()
            }),
            CanonicalChange::Rename {
                from: PathBuf::from("old/path.ts")
            }
        );
    }

    #[test]
    fn certify_change_kind_drops_rename_from() {
        assert_eq!(
            certify_change_kind(&ChangeKindWire::Created),
            ChangeKind::Create
        );
        assert_eq!(
            certify_change_kind(&ChangeKindWire::Modified),
            ChangeKind::ContentModify
        );
        assert_eq!(
            certify_change_kind(&ChangeKindWire::Deleted),
            ChangeKind::Delete
        );
        assert_eq!(
            certify_change_kind(&ChangeKindWire::Renamed {
                from: "old.ts".to_string()
            }),
            ChangeKind::Rename
        );
    }

    #[test]
    fn wire_stale_reason_maps_direct_counterparts() {
        assert_eq!(
            wire_stale_reason(CertifyStale::ImpactSetOverflow),
            StaleReason::ImpactSetOverflow
        );
        assert_eq!(
            wire_stale_reason(CertifyStale::Deleted),
            StaleReason::Deleted
        );
        assert_eq!(
            wire_stale_reason(CertifyStale::Renamed),
            StaleReason::Renamed
        );
        assert_eq!(
            wire_stale_reason(CertifyStale::CrossFileResolutionNeeded),
            StaleReason::CrossFileResolutionNeeded
        );
    }

    #[test]
    fn wire_stale_reason_maps_unwired_reasons_to_cross_file_resolution() {
        // No dedicated wire variant — both conservatively ask for cross-file
        // resolution rather than over-claiming a specific cause.
        assert_eq!(
            wire_stale_reason(CertifyStale::ExportSurfaceChange),
            StaleReason::CrossFileResolutionNeeded
        );
        assert_eq!(
            wire_stale_reason(CertifyStale::UnreliableGraph),
            StaleReason::CrossFileResolutionNeeded
        );
    }

    #[test]
    fn diagnostic_severity_maps_all_levels() {
        assert_eq!(diagnostic_severity(WarningSeverity::Error), Severity::Error);
        assert_eq!(
            diagnostic_severity(WarningSeverity::Warning),
            Severity::Warning
        );
        assert_eq!(diagnostic_severity(WarningSeverity::Info), Severity::Info);
    }

    fn sample_warning(suggestion: &str) -> Warning {
        Warning {
            id: "AP-001".to_string(),
            fingerprint: None,
            category: WarningCategory::AntiPattern,
            severity: WarningSeverity::Warning,
            confidence: Confidence::High,
            title: "Avoid any".to_string(),
            message: "`any` defeats the type checker".to_string(),
            explanation: String::new(),
            suggestion: suggestion.to_string(),
            nudge: None,
            location: WarnLocation {
                file: "src/app.ts".to_string(),
                line: 12,
                column: Some(4),
                end_line: None,
                end_column: None,
            },
            pattern: None,
            suppressed: None,
            family: None,
            definition_ref: None,
            spectrum_position: None,
        }
    }

    #[test]
    fn antipattern_diagnostic_carries_canonical_fields() {
        let d = antipattern_diagnostic(&sample_warning("use a precise type"));
        assert_eq!(d.id, "AP-001");
        assert_eq!(d.severity, Severity::Warning);
        assert_eq!(d.summary, "`any` defeats the type checker");
        assert_eq!(d.location.file, "src/app.ts");
        assert_eq!(d.location.line, Some(12));
        assert_eq!(d.location.column, Some(4));
        assert_eq!(d.category, Category::Antipattern);
        assert_eq!(d.source.rule_id, "AP-001");
        assert_eq!(d.source.source_module, ANTIPATTERN_SOURCE_MODULE);
        assert_eq!(d.mode, Mode::known(KnownMode::SaveTime));
        assert_eq!(d.remediation_hint.as_deref(), Some("use a precise type"));
    }

    #[test]
    fn antipattern_diagnostic_omits_empty_suggestion_hint() {
        let d = antipattern_diagnostic(&sample_warning("   "));
        assert_eq!(
            d.remediation_hint, None,
            "a blank suggestion must not become a placeholder hint"
        );
    }
}
