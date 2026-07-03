//! IO risk findings → remediation-first, posture-driven guidance (IORISK-003).
//!
//! Turns [`RiskFinding`]s (from the [`crate::io_risk::pipeline`] chain) into
//! [`RiskGuidance`] — a stable, remediation-first shape consumable by policy
//! packs and CI summaries. It follows the same conventions as
//! [`crate::context::guidance`]: a stable kebab-case [`RiskGuidanceCode`], a
//! human `message`, remediation-first `remediation`, clean serde round-trip, and
//! optional fields skip-serialised. It **reuses** the taxonomy vocabulary
//! ([`RiskCategory`]/[`RiskSeverity`]/[`Confidence`]) rather than cloning it.
//!
//! ## Blocking is posture-driven, not band-driven
//!
//! Whether a finding *blocks* is **not** stored on the guidance and **not**
//! hard-derived from its severity band. The band describes the finding; the
//! enforcement layer decides what to do with it under a requested
//! [`EnforcementPosture`]. [`decision_under`] / [`blocks_under`] expose that
//! mapping as a function of `(severity, posture)`, defaulting to warnings-first
//! ([`EnforcementPosture::Warn`], ADR-002). This keeps the "does it block"
//! decision with the caller that owns the posture, and mirrors the diagnostic
//! envelope's separation of severity from the control decision (kernel-types
//! [`ControlDecision`]).
//!
//! [`crate::context::guidance`] (CPOL) shares this [`EnforcementPosture`] type
//! and the same posture-parameterised shape.

use anvil_kernel_types::diagnostics::ControlDecision;
use anvil_kernel_types::io_risk::{Confidence, RiskCategory, RiskFinding, RiskSeverity};
use serde::{Deserialize, Serialize};

use crate::io_risk::pipeline::ScanReport;

/// The enforcement posture a caller applies when turning findings into
/// decisions.
///
/// Shared by IO risk guidance and [`crate::context::guidance`]. The default is
/// [`EnforcementPosture::Warn`] (ADR-002, warnings over blocks): nothing blocks
/// until a caller opts into [`EnforcementPosture::Enforce`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum EnforcementPosture {
    /// Warnings-first: every finding is advisory; nothing blocks (exit 0).
    #[default]
    Warn,
    /// Enforce: high-signal findings block; lower bands stay advisory.
    Enforce,
}

/// Map a [`RiskSeverity`] to a [`ControlDecision`] under a posture.
///
/// Under [`EnforcementPosture::Warn`] every finding is [`ControlDecision::Warn`]
/// (ADR-002). Under [`EnforcementPosture::Enforce`] only the high bands
/// ([`RiskSeverity::High`]/[`RiskSeverity::Critical`]) block; lower bands stay
/// advisory. An unrecognised (forward-compat) [`RiskSeverity::Unknown`] is
/// treated as a warning, never escalated to a block (ADR-096) — a newer
/// producer's band must not silently start blocking an older enforcement layer.
#[must_use]
pub fn decision_under(severity: RiskSeverity, posture: EnforcementPosture) -> ControlDecision {
    match posture {
        EnforcementPosture::Warn => ControlDecision::Warn,
        EnforcementPosture::Enforce => match severity {
            RiskSeverity::High | RiskSeverity::Critical => ControlDecision::Block,
            RiskSeverity::Low | RiskSeverity::Medium | RiskSeverity::Unknown => {
                ControlDecision::Warn
            }
        },
    }
}

/// Whether a finding of this severity blocks under a posture.
#[must_use]
pub fn blocks_under(severity: RiskSeverity, posture: EnforcementPosture) -> bool {
    matches!(decision_under(severity, posture), ControlDecision::Block)
}

/// Stable, machine-readable classification of a risk guidance entry.
///
/// Mirrors the finding's [`RiskCategory`] on the wire (kebab-case), as a
/// dedicated guidance-side code so the guidance contract can evolve
/// independently of the taxonomy. Variants are added, never renamed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RiskGuidanceCode {
    /// Guidance for a prompt-injection finding.
    PromptInjection,
    /// Guidance for a sensitive-data-leakage finding.
    SensitiveDataLeakage,
    /// Guidance for an unsafe-response finding.
    UnsafeResponse,
    /// Guidance for an unsafe-input finding.
    UnsafeInput,
    /// Guidance for a finding whose category this consumer does not recognise
    /// (forward-compat). Surfaced, never dropped.
    Unknown,
}

impl RiskGuidanceCode {
    /// The guidance code for a finding's category.
    #[must_use]
    pub fn for_category(category: RiskCategory) -> Self {
        match category {
            RiskCategory::PromptInjection => Self::PromptInjection,
            RiskCategory::SensitiveDataLeakage => Self::SensitiveDataLeakage,
            RiskCategory::UnsafeResponse => Self::UnsafeResponse,
            RiskCategory::UnsafeInput => Self::UnsafeInput,
            RiskCategory::Unknown => Self::Unknown,
        }
    }
}

/// A remediation-first explanation of a single risk finding.
///
/// Deliberately carries **no** blocking flag: whether it blocks is computed on
/// demand from [`severity`](Self::severity) and a caller-supplied
/// [`EnforcementPosture`] via [`RiskGuidance::decision_under`]. Optional
/// attribution (`source`) is skip-serialised when absent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RiskGuidance {
    /// Stable machine-readable code for the finding's category.
    pub code: RiskGuidanceCode,
    /// The finding's declared severity band (reused taxonomy).
    pub severity: RiskSeverity,
    /// The scanner's confidence in the finding (reused taxonomy).
    pub confidence: Confidence,
    /// The finding's source label, when it named a location.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    /// Human-readable description of the finding. UK spelling.
    pub message: String,
    /// How to resolve it — remediation-first guidance from the finding.
    pub remediation: String,
}

impl RiskGuidance {
    /// The control decision for this guidance under a posture. Posture-driven,
    /// never stored (see the [module docs](self)).
    #[must_use]
    pub fn decision_under(&self, posture: EnforcementPosture) -> ControlDecision {
        decision_under(self.severity, posture)
    }

    /// Whether this guidance blocks under a posture.
    #[must_use]
    pub fn blocks_under(&self, posture: EnforcementPosture) -> bool {
        blocks_under(self.severity, posture)
    }
}

/// Build guidance for a single finding.
#[must_use]
pub fn guidance_for_finding(finding: &RiskFinding) -> RiskGuidance {
    RiskGuidance {
        code: RiskGuidanceCode::for_category(finding.category),
        severity: finding.severity,
        confidence: finding.confidence,
        source: finding
            .location
            .as_ref()
            .and_then(|location| location.source.clone()),
        message: finding.message.clone(),
        remediation: finding.remediation.clone(),
    }
}

/// Build guidance for a set of findings, preserving order.
#[must_use]
pub fn guidance_for_findings(findings: &[RiskFinding]) -> Vec<RiskGuidance> {
    findings.iter().map(guidance_for_finding).collect()
}

/// Build guidance from a [`ScanReport`]'s findings, preserving the report's
/// deterministic order. Scanner faults ([`ScanReport::scanner_errors`]) are an
/// operational concern and are intentionally not mapped to risk guidance here.
#[must_use]
pub fn guidance_for_report(report: &ScanReport) -> Vec<RiskGuidance> {
    guidance_for_findings(&report.findings)
}

#[cfg(test)]
mod tests {
    use super::*;
    use anvil_kernel_types::io_risk::RiskLocation;

    fn finding(severity: RiskSeverity, category: RiskCategory) -> RiskFinding {
        RiskFinding::new(
            category,
            severity,
            Confidence::High,
            "untrusted marker present",
            "Neutralise the flagged content.",
        )
        .with_location(RiskLocation {
            source: Some("prompt:user".into()),
            start: Some(0),
            end: Some(4),
        })
    }

    #[test]
    fn io_risk_guidance_maps_finding_remediation_first() {
        let guidance =
            guidance_for_finding(&finding(RiskSeverity::High, RiskCategory::PromptInjection));
        assert_eq!(guidance.code, RiskGuidanceCode::PromptInjection);
        assert_eq!(guidance.severity, RiskSeverity::High);
        assert_eq!(guidance.source.as_deref(), Some("prompt:user"));
        assert_eq!(guidance.remediation, "Neutralise the flagged content.");
    }

    #[test]
    fn io_risk_guidance_default_posture_is_warnings_first() {
        // ADR-002: nothing blocks under the default posture, whatever the band.
        assert_eq!(EnforcementPosture::default(), EnforcementPosture::Warn);
        for severity in [
            RiskSeverity::Low,
            RiskSeverity::Medium,
            RiskSeverity::High,
            RiskSeverity::Critical,
            RiskSeverity::Unknown,
        ] {
            assert!(!blocks_under(severity, EnforcementPosture::Warn));
            assert_eq!(
                decision_under(severity, EnforcementPosture::Warn),
                ControlDecision::Warn
            );
        }
    }

    #[test]
    fn io_risk_guidance_enforce_blocks_only_high_bands() {
        assert!(blocks_under(
            RiskSeverity::Critical,
            EnforcementPosture::Enforce
        ));
        assert!(blocks_under(
            RiskSeverity::High,
            EnforcementPosture::Enforce
        ));
        assert!(!blocks_under(
            RiskSeverity::Medium,
            EnforcementPosture::Enforce
        ));
        assert!(!blocks_under(
            RiskSeverity::Low,
            EnforcementPosture::Enforce
        ));
    }

    #[test]
    fn io_risk_guidance_unknown_severity_never_blocks_even_under_enforce() {
        // Forward-compat (ADR-096): a newer producer's unrecognised band must
        // not silently start blocking an older enforcement layer.
        assert!(!blocks_under(
            RiskSeverity::Unknown,
            EnforcementPosture::Enforce
        ));
        assert_eq!(
            decision_under(RiskSeverity::Unknown, EnforcementPosture::Enforce),
            ControlDecision::Warn
        );
    }

    #[test]
    fn io_risk_guidance_blocking_is_not_stored_on_guidance() {
        // The guidance carries the band, and computes the decision on demand —
        // no stored blocking flag on the wire.
        let guidance = guidance_for_finding(&finding(
            RiskSeverity::Critical,
            RiskCategory::UnsafeResponse,
        ));
        let json = serde_json::to_value(&guidance).expect("serialise");
        assert!(
            json.get("blocking").is_none(),
            "blocking must not be stored: {json}"
        );
        // Same guidance, different postures, different decisions.
        assert!(!guidance.blocks_under(EnforcementPosture::Warn));
        assert!(guidance.blocks_under(EnforcementPosture::Enforce));
    }

    #[test]
    fn io_risk_guidance_round_trips_and_skips_absent_source() {
        let bare = RiskFinding::new(
            RiskCategory::UnsafeInput,
            RiskSeverity::Medium,
            Confidence::Low,
            "m",
            "r",
        );
        let guidance = guidance_for_finding(&bare);
        let json = serde_json::to_string(&guidance).expect("serialise");
        assert!(
            !json.contains("\"source\""),
            "absent source must be omitted: {json}"
        );
        assert!(json.contains("unsafe-input"), "{json}");
        let restored: RiskGuidance = serde_json::from_str(&json).expect("deserialise");
        assert_eq!(restored, guidance);
    }

    #[test]
    fn io_risk_guidance_for_report_preserves_order_and_ignores_scanner_errors() {
        let report = ScanReport {
            findings: vec![
                finding(RiskSeverity::High, RiskCategory::PromptInjection),
                finding(RiskSeverity::Low, RiskCategory::UnsafeResponse),
            ],
            scanner_errors: vec![],
        };
        let guidance = guidance_for_report(&report);
        assert_eq!(guidance.len(), 2);
        assert_eq!(guidance[0].code, RiskGuidanceCode::PromptInjection);
        assert_eq!(guidance[1].code, RiskGuidanceCode::UnsafeResponse);
    }

    #[test]
    fn io_risk_guidance_unknown_category_surfaces_as_unknown_code() {
        // A forward-compat category must still produce guidance, not vanish.
        let unknown: RiskFinding = serde_json::from_value(serde_json::json!({
            "category": "model-exfiltration",
            "severity": "high",
            "confidence": "high",
            "message": "m",
            "remediation": "r"
        }))
        .expect("deserialise");
        let guidance = guidance_for_finding(&unknown);
        assert_eq!(guidance.code, RiskGuidanceCode::Unknown);
    }
}
