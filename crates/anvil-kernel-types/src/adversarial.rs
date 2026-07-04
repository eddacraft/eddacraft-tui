//! Adversarial probe taxonomy — the shared vocabulary for adversarial test
//! assets (ATC-001).
//!
//! This module owns the pure serde data shapes for the Anvil-native adversarial
//! testing catalog: the *classification* of a probe (its [`ProbeCategory`],
//! [`PayloadClass`], and the [`ExpectedOutcome`] it asserts) and the [`Probe`]
//! record itself. It is a substrate of deterministic *test assets* — it defines
//! and versions probe fixtures; it does **not** scan, attack, execute, or make
//! any policy decision. Loadable probe packs (ATC-002) and eval-harness
//! integration (ATC-003) build on these types from the reference crate.
//!
//! It lives in the wire-types crate so every producer and consumer — the probe
//! registry, the eval harness bridge, and any future reporting surface — binds
//! to one taxonomy without a heavier dependency.
//!
//! ## Forward-compatibility (the wire lesson)
//!
//! Every classification enum here — [`ProbeCategory`], [`PayloadClass`],
//! [`ExpectedOutcome`] — carries a `#[serde(other)]` `Unknown` fallback,
//! mirroring [`crate::io_risk::RiskCategory`] (ADR-096): a newer producer
//! emitting a value this consumer does not recognise deserialises to `Unknown`
//! and is **surfaced, never dropped**, rather than failing the whole [`Probe`]
//! parse. The wire form is kebab-case.

use serde::{Deserialize, Serialize};

/// The class of adversarial behaviour a probe exercises.
///
/// A deliberately small, closed set derived from the module's remit — validating
/// prompt safety, data handling, and tool/boundary behaviour. Forward-compatible:
/// an unrecognised value deserialises to [`ProbeCategory::Unknown`] rather than
/// dropping the probe.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ProbeCategory {
    /// Untrusted input attempting to subvert or override model instructions.
    PromptInjection,
    /// An attempt to extract secrets, private context, or other sensitive data.
    DataExfiltration,
    /// An attempt to drive an unsafe or out-of-policy tool/command invocation.
    UnsafeToolInvocation,
    /// An attempt to slip past a declared safety or scope boundary.
    BoundaryEvasion,
    /// A `category` value a newer producer emitted that this consumer does not
    /// recognise. Surfaced, never dropped.
    #[serde(other)]
    Unknown,
}

impl ProbeCategory {
    /// The stable kebab-case wire label for this category.
    ///
    /// Matches the serde representation exactly, so it is safe to use as a
    /// grouping key or a suite-name component (ATC-003/ATC-004) without a serde
    /// round-trip.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::PromptInjection => "prompt-injection",
            Self::DataExfiltration => "data-exfiltration",
            Self::UnsafeToolInvocation => "unsafe-tool-invocation",
            Self::BoundaryEvasion => "boundary-evasion",
            Self::Unknown => "unknown",
        }
    }
}

impl std::fmt::Display for ProbeCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// The form a probe's payload takes — its delivery vector.
///
/// Describes *how* the adversarial content is shaped, distinct from *what*
/// category of behaviour it targets. A deliberately small, closed set;
/// forward-compatible via [`PayloadClass::Unknown`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PayloadClass {
    /// A plain, direct natural-language instruction.
    DirectInstruction,
    /// Content obfuscated to evade naive matching (encoding, homoglyphs, …).
    ObfuscatedText,
    /// Hostile content embedded inside otherwise-benign data (a document,
    /// a code comment, a retrieved snippet).
    EmbeddedContent,
    /// A payload delivered through a tool/function-call argument.
    ToolArgument,
    /// A `payload_class` value a newer producer emitted that this consumer does
    /// not recognise. Surfaced, never dropped.
    #[serde(other)]
    Unknown,
}

/// The safe behaviour a probe asserts the system must exhibit.
///
/// This is the probe's *pass condition*: the deterministic, expected safe
/// outcome. Forward-compatible via [`ExpectedOutcome::Unknown`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ExpectedOutcome {
    /// The attempt is blocked outright.
    Blocked,
    /// The attempt is allowed but surfaced with a warning.
    Warned,
    /// The system refuses the request.
    Refused,
    /// The hostile content is neutralised/stripped before use.
    Sanitised,
    /// An `expected_outcome` value a newer producer emitted that this consumer
    /// does not recognise. Surfaced, never dropped.
    #[serde(other)]
    Unknown,
}

/// A single adversarial probe: a versioned, traceable test asset.
///
/// The canonical unit authored in a probe pack (ATC-002) and executed through
/// the eval harness (ATC-003). It carries only classification and provenance —
/// no live payload execution logic lives here. `version` is an opaque string so
/// a probe's assertion can evolve while remaining traceable across catalog
/// revisions.
///
/// Unknown fields are **not** rejected here (unlike the authored pack manifest):
/// a [`Probe`] is a wire shape a newer producer may extend, so an unrecognised
/// field is tolerated rather than failing the parse, matching
/// [`crate::io_risk::RiskFinding`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Probe {
    /// Stable, unique identifier within its pack.
    pub id: String,
    /// The class of adversarial behaviour exercised.
    pub category: ProbeCategory,
    /// The delivery vector of the payload.
    pub payload_class: PayloadClass,
    /// The safe behaviour the probe asserts (its pass condition).
    pub expected_outcome: ExpectedOutcome,
    /// Opaque version string of this probe asset.
    pub version: String,
    /// Human-readable description of what the probe checks. UK spelling.
    pub description: String,
}

impl Probe {
    /// Build a probe from its classification and provenance.
    #[must_use]
    pub fn new(
        id: impl Into<String>,
        category: ProbeCategory,
        payload_class: PayloadClass,
        expected_outcome: ExpectedOutcome,
        version: impl Into<String>,
        description: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            category,
            payload_class,
            expected_outcome,
            version: version.into(),
            description: description.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};

    fn sample_probe() -> Probe {
        Probe::new(
            "pi-override-001",
            ProbeCategory::PromptInjection,
            PayloadClass::DirectInstruction,
            ExpectedOutcome::Refused,
            "1.0.0",
            "Direct instruction-override attempt must be refused.",
        )
    }

    #[test]
    fn adversarial_taxonomy_probe_round_trips_through_json() {
        let probe = sample_probe();
        let json = serde_json::to_string(&probe).expect("serialise");
        let back: Probe = serde_json::from_str(&json).expect("deserialise");
        assert_eq!(back, probe);
    }

    #[test]
    fn adversarial_taxonomy_category_variants_serialise_kebab_case() {
        let cases = [
            (ProbeCategory::PromptInjection, "prompt-injection"),
            (ProbeCategory::DataExfiltration, "data-exfiltration"),
            (
                ProbeCategory::UnsafeToolInvocation,
                "unsafe-tool-invocation",
            ),
            (ProbeCategory::BoundaryEvasion, "boundary-evasion"),
        ];
        for (variant, expected) in cases {
            let value = serde_json::to_value(variant).expect("serialise");
            assert_eq!(
                value, expected,
                "variant {variant:?} must serialise to {expected}"
            );
            let back: ProbeCategory = serde_json::from_value(value).expect("deserialise");
            assert_eq!(back, variant);
            // The `as_str` helper must agree with the serde wire form exactly.
            assert_eq!(variant.as_str(), expected);
            assert_eq!(variant.to_string(), expected);
        }
    }

    #[test]
    fn adversarial_taxonomy_payload_and_outcome_serialise_kebab_case() {
        assert_eq!(
            serde_json::to_value(PayloadClass::ObfuscatedText).unwrap(),
            "obfuscated-text"
        );
        assert_eq!(
            serde_json::to_value(PayloadClass::ToolArgument).unwrap(),
            "tool-argument"
        );
        assert_eq!(
            serde_json::to_value(ExpectedOutcome::Sanitised).unwrap(),
            "sanitised"
        );
        assert_eq!(
            serde_json::to_value(ExpectedOutcome::Blocked).unwrap(),
            "blocked"
        );
    }

    #[test]
    fn adversarial_taxonomy_unknown_category_round_trips_as_unknown() {
        // A newer producer's category must surface, never drop the probe.
        let category: ProbeCategory =
            serde_json::from_value(json!("model-inversion")).expect("deserialise");
        assert_eq!(category, ProbeCategory::Unknown);
        assert_eq!(
            serde_json::to_value(ProbeCategory::Unknown).unwrap(),
            "unknown"
        );
        assert_eq!(ProbeCategory::Unknown.as_str(), "unknown");
    }

    #[test]
    fn adversarial_taxonomy_unknown_payload_and_outcome_fall_back() {
        let payload: PayloadClass =
            serde_json::from_value(json!("steganographic")).expect("deserialise");
        assert_eq!(payload, PayloadClass::Unknown);
        let outcome: ExpectedOutcome =
            serde_json::from_value(json!("quarantined")).expect("deserialise");
        assert_eq!(outcome, ExpectedOutcome::Unknown);
    }

    #[test]
    fn adversarial_taxonomy_probe_with_unknown_enum_still_parses() {
        // The whole probe parses even when one classification is unknown.
        let payload = json!({
            "id": "future-001",
            "category": "supply-chain",
            "payload_class": "polyglot",
            "expected_outcome": "escalated",
            "version": "2.0.0",
            "description": "d"
        });
        let probe: Probe = serde_json::from_value(payload).expect("deserialise");
        assert_eq!(probe.category, ProbeCategory::Unknown);
        assert_eq!(probe.payload_class, PayloadClass::Unknown);
        assert_eq!(probe.expected_outcome, ExpectedOutcome::Unknown);
        assert_eq!(probe.id, "future-001");
    }

    #[test]
    fn adversarial_taxonomy_probe_tolerates_unknown_field() {
        // A newer producer may add a field; a Probe must not fail the parse.
        let payload = json!({
            "id": "pi-1",
            "category": "prompt-injection",
            "payload_class": "direct-instruction",
            "expected_outcome": "refused",
            "version": "1.0.0",
            "description": "d",
            "severity": "high"
        });
        let probe: Probe = serde_json::from_value(payload).expect("deserialise");
        assert_eq!(probe.category, ProbeCategory::PromptInjection);
    }

    #[test]
    fn adversarial_taxonomy_probe_carries_all_classification_fields() {
        let probe = sample_probe();
        let value: Value = serde_json::to_value(&probe).expect("to_value");
        for key in [
            "id",
            "category",
            "payload_class",
            "expected_outcome",
            "version",
            "description",
        ] {
            assert!(value.get(key).is_some(), "probe must carry `{key}`");
        }
    }
}
