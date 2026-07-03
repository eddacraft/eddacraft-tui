//! IO risk taxonomy — the shared vocabulary for input/output risk findings
//! (IORISK-001).
//!
//! Provider-agnostic scanners (IORISK-002) produce [`RiskFinding`]s and the
//! policy-engine guidance layer (IORISK-003) consumes them. This module owns
//! only the pure serde data shapes: no scanning logic, no I/O, no policy
//! mapping. It lives in the wire-types crate so every producer and consumer
//! binds to one taxonomy without a heavier dependency.
//!
//! ## Forward-compatibility (the wire lesson)
//!
//! Every classification enum here — [`RiskCategory`], [`RiskSeverity`],
//! [`Confidence`] — carries a `#[serde(other)]` `Unknown` fallback, mirroring
//! [`crate::diagnostics::Severity`]/[`crate::diagnostics::Category`] (ADR-096):
//! a newer producer emitting a value this consumer does not recognise
//! deserialises to `Unknown` and is **surfaced, never dropped**, rather than
//! failing the whole [`RiskFinding`] parse. The wire form is kebab-case.

use serde::{Deserialize, Serialize};

/// The class of IO risk a finding concerns.
///
/// A deliberately small, closed set derived from the module's remit —
/// prompt injection, sensitive-data leakage, and unsafe input/response
/// patterns. Forward-compatible: an unrecognised value deserialises to
/// [`RiskCategory::Unknown`] rather than dropping the finding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RiskCategory {
    /// Untrusted input attempting to subvert model instructions.
    PromptInjection,
    /// Sensitive data (secrets, PII) exposed in an input or output payload.
    SensitiveDataLeakage,
    /// An output/response exhibiting an unsafe pattern.
    UnsafeResponse,
    /// An input exhibiting an unsafe pattern (malformed, hostile, oversized).
    UnsafeInput,
    /// A `category` value a newer producer emitted that this consumer does not
    /// recognise. Surfaced, never dropped.
    #[serde(other)]
    Unknown,
}

/// Severity band of a risk finding.
///
/// Distinct from any enforcement decision: severity describes the finding;
/// the policy/enforcement layer maps severity (plus posture) to a decision
/// (IORISK-003). Forward-compatible via [`RiskSeverity::Unknown`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RiskSeverity {
    /// Advisory; lowest rank.
    Low,
    /// Default operational concern.
    Medium,
    /// Should be addressed before shipping.
    High,
    /// Highest rank.
    Critical,
    /// A `severity` value a newer producer emitted that this consumer does not
    /// recognise. Surfaced, never dropped.
    #[serde(other)]
    Unknown,
}

/// How confident the scanner is in a finding.
///
/// A bounded band, deliberately **not** a float, so the wire form is stable and
/// comparisons are exact. Forward-compatible via [`Confidence::Unknown`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Confidence {
    /// A weak signal; likely needs corroboration.
    Low,
    /// A moderate signal.
    Medium,
    /// A strong signal.
    High,
    /// A `confidence` value a newer producer emitted that this consumer does
    /// not recognise. Surfaced, never dropped.
    #[serde(other)]
    Unknown,
}

/// Where in a payload a finding was located.
///
/// Deliberately generic: a finding may concern a file, a prompt id, a named
/// response stream, or nothing precise. `source` is a free-form label; `start`
/// and `end` are optional payload offsets, populated only when a
/// span-producing scan ran. Offsets are `u32` so the wire schema is stable
/// across 32-/64-bit producers, matching [`crate::diagnostics::Location`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct RiskLocation {
    /// Free-form source label (a file path, a prompt id, `"response"`, …).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    /// Start offset within the scanned payload, when known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start: Option<u32>,
    /// End offset within the scanned payload, when known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end: Option<u32>,
}

/// A single IO risk finding produced by a scanner.
///
/// The canonical unit exchanged between scanners (IORISK-002) and the guidance
/// layer (IORISK-003). `remediation` is required — the taxonomy is
/// remediation-first — while `location` is optional (some findings concern a
/// whole payload with no precise span).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RiskFinding {
    /// The class of risk.
    pub category: RiskCategory,
    /// The severity band.
    pub severity: RiskSeverity,
    /// The scanner's confidence in the finding.
    pub confidence: Confidence,
    /// Where the finding was located, when known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub location: Option<RiskLocation>,
    /// Human-readable description of the finding. UK spelling.
    pub message: String,
    /// How to resolve it — remediation-first guidance.
    pub remediation: String,
}

impl RiskFinding {
    /// Build a finding with no location. Attach one with
    /// [`with_location`](Self::with_location) when a span is known.
    #[must_use]
    pub fn new(
        category: RiskCategory,
        severity: RiskSeverity,
        confidence: Confidence,
        message: impl Into<String>,
        remediation: impl Into<String>,
    ) -> Self {
        Self {
            category,
            severity,
            confidence,
            location: None,
            message: message.into(),
            remediation: remediation.into(),
        }
    }

    /// Attach a location to the finding.
    #[must_use]
    pub fn with_location(mut self, location: RiskLocation) -> Self {
        self.location = Some(location);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};

    fn sample_finding() -> RiskFinding {
        RiskFinding::new(
            RiskCategory::PromptInjection,
            RiskSeverity::High,
            Confidence::Medium,
            "Instruction-override marker detected in untrusted input",
            "Strip or neutralise the marker before forwarding the input.",
        )
        .with_location(RiskLocation {
            source: Some("prompt:user".into()),
            start: Some(12),
            end: Some(48),
        })
    }

    #[test]
    fn io_risk_taxonomy_finding_round_trips_through_json() {
        let finding = sample_finding();
        let json = serde_json::to_string(&finding).expect("serialise");
        let back: RiskFinding = serde_json::from_str(&json).expect("deserialise");
        assert_eq!(back, finding);
    }

    #[test]
    fn io_risk_taxonomy_category_variants_serialise_kebab_case() {
        let cases = [
            (RiskCategory::PromptInjection, "prompt-injection"),
            (RiskCategory::SensitiveDataLeakage, "sensitive-data-leakage"),
            (RiskCategory::UnsafeResponse, "unsafe-response"),
            (RiskCategory::UnsafeInput, "unsafe-input"),
        ];
        for (variant, expected) in cases {
            let json = serde_json::to_value(variant).expect("serialise");
            assert_eq!(
                json, expected,
                "variant {variant:?} must serialise to {expected}"
            );
            let back: RiskCategory = serde_json::from_value(json).expect("deserialise");
            assert_eq!(back, variant);
        }
    }

    #[test]
    fn io_risk_taxonomy_severity_and_confidence_serialise_kebab_case() {
        assert_eq!(serde_json::to_value(RiskSeverity::Low).unwrap(), "low");
        assert_eq!(
            serde_json::to_value(RiskSeverity::Critical).unwrap(),
            "critical"
        );
        assert_eq!(serde_json::to_value(Confidence::Medium).unwrap(), "medium");
        assert_eq!(serde_json::to_value(Confidence::High).unwrap(), "high");
    }

    #[test]
    fn io_risk_taxonomy_unknown_category_round_trips_as_unknown() {
        // A newer producer's category must surface, never drop the finding.
        let category: RiskCategory =
            serde_json::from_value(json!("model-exfiltration")).expect("deserialise");
        assert_eq!(category, RiskCategory::Unknown);
        assert_eq!(
            serde_json::to_value(RiskCategory::Unknown).unwrap(),
            "unknown"
        );
    }

    #[test]
    fn io_risk_taxonomy_unknown_severity_and_confidence_fall_back() {
        let severity: RiskSeverity =
            serde_json::from_value(json!("catastrophic")).expect("deserialise");
        assert_eq!(severity, RiskSeverity::Unknown);
        let confidence: Confidence = serde_json::from_value(json!("certain")).expect("deserialise");
        assert_eq!(confidence, Confidence::Unknown);
    }

    #[test]
    fn io_risk_taxonomy_finding_with_unknown_enum_still_parses() {
        // The whole finding parses even when one classification is unknown.
        let payload = json!({
            "category": "quantum-leak",
            "severity": "fatal",
            "confidence": "definite",
            "message": "m",
            "remediation": "r"
        });
        let finding: RiskFinding = serde_json::from_value(payload).expect("deserialise");
        assert_eq!(finding.category, RiskCategory::Unknown);
        assert_eq!(finding.severity, RiskSeverity::Unknown);
        assert_eq!(finding.confidence, Confidence::Unknown);
    }

    #[test]
    fn io_risk_taxonomy_location_is_optional_and_omitted_when_absent() {
        let finding = RiskFinding::new(
            RiskCategory::UnsafeResponse,
            RiskSeverity::Medium,
            Confidence::Low,
            "m",
            "r",
        );
        let value: Value = serde_json::to_value(&finding).expect("to_value");
        assert!(
            value.get("location").is_none(),
            "absent location must be omitted, got: {value}"
        );
    }

    #[test]
    fn io_risk_taxonomy_location_omits_empty_offsets() {
        let value: Value = serde_json::to_value(RiskLocation {
            source: Some("response".into()),
            start: None,
            end: None,
        })
        .expect("to_value");
        assert_eq!(value["source"], "response");
        assert!(value.get("start").is_none());
        assert!(value.get("end").is_none());
    }
}
