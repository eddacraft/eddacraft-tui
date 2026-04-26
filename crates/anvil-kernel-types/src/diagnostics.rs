//! Canonical `Diagnostic` shape — `anvil.diagnostic.v1`.
//!
//! Defined per the 2026-04-26 diagnostic envelope coordination spec
//! (`plans/specs/2026-04-26-diagnostic-envelope-coordination.md`).
//!
//! A diagnostic is a finding produced by a rule — a cross-layer import,
//! a leaked secret, a reasoning-pattern violation. The same logical
//! payload travels in three distinct outer envelopes (return-value /
//! broadcast / control); only the wrapper differs. This module owns the
//! inner shape that all four work items (AIGUARD-002, RTAI-007,
//! INTD-013, DRVR-002) consume.
//!
//! `Severity` is intentionally distinct from the control decision
//! (`allow`/`warn`/`block`/`interrupt`). INTD-013 maps severity to a
//! control decision per the project's enforcement configuration; that
//! mapping is the daemon's job, not the diagnostic's.

use serde::{Deserialize, Serialize};

/// Current schema-version string for the inner diagnostic shape.
///
/// Distinct from any outer envelope `schema` field. Bumps to
/// `anvil.diagnostic.v2` only on breaking changes; additive evolution
/// stays on `v1` per the spec's versioning rules.
pub const DIAGNOSTIC_SCHEMA_VERSION: &str = "anvil.diagnostic.v1";

/// Rule severity. Deliberately separate from the control decision
/// (`allow`/`warn`/`block`/`interrupt`); the daemon owns severity →
/// decision mapping.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Severity {
    Info,
    Warning,
    Error,
}

/// Coarse routing/filtering grouping for diagnostics. Closed list per
/// the spec — new values require a spec amendment before producers
/// emit them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Category {
    Secret,
    Antipattern,
    Boundary,
    Policy,
    Reasoning,
    CommandSafety,
    Architecture,
    Other,
}

/// Mode discriminator. Identifies which path produced the diagnostic
/// and the consumer expectation. See the envelope spec's "Mode
/// Discriminator Semantics" table for the per-mode contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Mode {
    SaveTime,
    MidEdit,
    Gate,
    Watch,
}

/// File anchor for a diagnostic. `line`/`column` are 1-based;
/// `end_line`/`end_column` are optional and span the end of the
/// flagged region when present.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Location {
    pub file: String,
    pub line: usize,
    pub column: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end_line: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end_column: Option<usize>,
}

/// Provenance for a diagnostic. `rule_id` uniquely identifies the rule
/// across Anvil; `source_module` is the producing crate or sub-module
/// (e.g. `anvil-checks::secrets`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiagnosticSource {
    pub rule_id: String,
    pub source_module: String,
}

/// Canonical inner diagnostic shape — `anvil.diagnostic.v1`.
///
/// Used in three outer envelopes: AIGUARD-002 gate result, RTAI-007 /
/// INTD-013 telemetry mirror, and DRVR-002 JSON-RPC notification. None
/// of those modules redefines this type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Diagnostic {
    pub schema_version: String,
    pub id: String,
    pub severity: Severity,
    pub summary: String,
    pub location: Location,
    pub category: Category,
    pub source: DiagnosticSource,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remediation_hint: Option<String>,
    pub mode: Mode,
}

impl Diagnostic {
    /// Build a new diagnostic with `schema_version` defaulted to the
    /// current canonical value (`anvil.diagnostic.v1`).
    #[must_use]
    pub fn new(
        id: impl Into<String>,
        severity: Severity,
        summary: impl Into<String>,
        location: Location,
        category: Category,
        source: DiagnosticSource,
        mode: Mode,
    ) -> Self {
        Self {
            schema_version: DIAGNOSTIC_SCHEMA_VERSION.to_string(),
            id: id.into(),
            severity,
            summary: summary.into(),
            location,
            category,
            source,
            remediation_hint: None,
            mode,
        }
    }

    /// Attach a remediation hint. Omit instead of emitting a generic
    /// placeholder when there is no useful guidance.
    #[must_use]
    pub fn with_remediation_hint(mut self, hint: impl Into<String>) -> Self {
        self.remediation_hint = Some(hint.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};

    fn sample_location() -> Location {
        Location {
            file: "src/api/client.ts".into(),
            line: 42,
            column: 18,
            end_line: Some(42),
            end_column: Some(47),
        }
    }

    fn sample_source() -> DiagnosticSource {
        DiagnosticSource {
            rule_id: "secret-aws-access-key".into(),
            source_module: "anvil-checks::secrets".into(),
        }
    }

    fn sample_diagnostic() -> Diagnostic {
        Diagnostic::new(
            "diag_01HW8K6Q4P0X7N9TJ4YA3S0V",
            Severity::Error,
            "Hardcoded API key detected",
            sample_location(),
            Category::Secret,
            sample_source(),
            Mode::SaveTime,
        )
        .with_remediation_hint("Move to environment variable; see docs/guides/secrets.md")
    }

    #[test]
    fn diagnostic_schema_version_constant_matches_spec() {
        assert_eq!(DIAGNOSTIC_SCHEMA_VERSION, "anvil.diagnostic.v1");
    }

    #[test]
    fn diagnostic_schema_version_default_on_new() {
        let diag = sample_diagnostic();
        assert_eq!(diag.schema_version, "anvil.diagnostic.v1");
    }

    #[test]
    fn diagnostic_schema_round_trips_through_json() {
        let diag = sample_diagnostic();
        let json = serde_json::to_string(&diag).expect("serialise");
        let back: Diagnostic = serde_json::from_str(&json).expect("deserialise");
        assert_eq!(back, diag);
    }

    #[test]
    fn diagnostic_schema_serialises_to_spec_field_names() {
        let diag = sample_diagnostic();
        let value: Value = serde_json::to_value(&diag).expect("to_value");

        assert_eq!(value["schema_version"], "anvil.diagnostic.v1");
        assert_eq!(value["id"], "diag_01HW8K6Q4P0X7N9TJ4YA3S0V");
        assert_eq!(value["severity"], "error");
        assert_eq!(value["summary"], "Hardcoded API key detected");
        assert_eq!(value["category"], "secret");
        assert_eq!(value["mode"], "save-time");
        assert_eq!(value["location"]["file"], "src/api/client.ts");
        assert_eq!(value["location"]["line"], 42);
        assert_eq!(value["location"]["column"], 18);
        assert_eq!(value["location"]["end_line"], 42);
        assert_eq!(value["location"]["end_column"], 47);
        assert_eq!(value["source"]["rule_id"], "secret-aws-access-key");
        assert_eq!(value["source"]["source_module"], "anvil-checks::secrets");
        assert_eq!(
            value["remediation_hint"],
            "Move to environment variable; see docs/guides/secrets.md"
        );
    }

    #[test]
    fn diagnostic_schema_severity_variants_serialise_kebab_case() {
        assert_eq!(serde_json::to_value(Severity::Info).unwrap(), "info");
        assert_eq!(serde_json::to_value(Severity::Warning).unwrap(), "warning");
        assert_eq!(serde_json::to_value(Severity::Error).unwrap(), "error");
    }

    #[test]
    fn diagnostic_schema_category_variants_serialise_kebab_case() {
        let cases = [
            (Category::Secret, "secret"),
            (Category::Antipattern, "antipattern"),
            (Category::Boundary, "boundary"),
            (Category::Policy, "policy"),
            (Category::Reasoning, "reasoning"),
            (Category::CommandSafety, "command-safety"),
            (Category::Architecture, "architecture"),
            (Category::Other, "other"),
        ];
        for (variant, expected) in cases {
            let json = serde_json::to_value(variant).expect("serialise");
            assert_eq!(
                json, expected,
                "variant {variant:?} must serialise to {expected}"
            );
            let back: Category = serde_json::from_value(json).expect("deserialise");
            assert_eq!(back, variant);
        }
    }

    #[test]
    fn diagnostic_schema_mode_variants_serialise_kebab_case() {
        let cases = [
            (Mode::SaveTime, "save-time"),
            (Mode::MidEdit, "mid-edit"),
            (Mode::Gate, "gate"),
            (Mode::Watch, "watch"),
        ];
        for (variant, expected) in cases {
            let json = serde_json::to_value(variant).expect("serialise");
            assert_eq!(
                json, expected,
                "variant {variant:?} must serialise to {expected}"
            );
            let back: Mode = serde_json::from_value(json).expect("deserialise");
            assert_eq!(back, variant);
        }
    }

    #[test]
    fn diagnostic_schema_remediation_hint_omitted_when_none() {
        let diag = Diagnostic::new(
            "diag_01HW8K6Q4P0X7N9TJ4YA3S0VAAA",
            Severity::Warning,
            "Something happened",
            sample_location(),
            Category::Antipattern,
            sample_source(),
            Mode::Gate,
        );
        let value: Value = serde_json::to_value(&diag).expect("to_value");
        assert!(
            value.get("remediation_hint").is_none(),
            "remediation_hint must be omitted when None, got: {value}"
        );
    }

    #[test]
    fn diagnostic_schema_location_end_fields_omitted_when_none() {
        let diag = Diagnostic::new(
            "diag_01HW8K6Q4P0X7N9TJ4YA3S0VBBB",
            Severity::Info,
            "Path-only finding",
            Location {
                file: "README.md".into(),
                line: 1,
                column: 1,
                end_line: None,
                end_column: None,
            },
            Category::Other,
            sample_source(),
            Mode::Watch,
        );
        let value: Value = serde_json::to_value(&diag).expect("to_value");
        assert!(value["location"].get("end_line").is_none());
        assert!(value["location"].get("end_column").is_none());
    }

    #[test]
    fn diagnostic_schema_deserialises_minimal_payload() {
        // Mirrors a producer that omits all optional fields — simulates
        // what RTAI-007 / DRVR-002 callers may emit.
        let payload = json!({
            "schema_version": "anvil.diagnostic.v1",
            "id": "diag_01HW8K6Q4P0X7N9TJ4YA3S0VCCC",
            "severity": "info",
            "summary": "Lean payload",
            "location": {
                "file": "src/lib.rs",
                "line": 10,
                "column": 1
            },
            "category": "other",
            "source": {
                "rule_id": "lean-rule",
                "source_module": "anvil-checks::lean"
            },
            "mode": "mid-edit"
        });
        let diag: Diagnostic = serde_json::from_value(payload).expect("deserialise");
        assert_eq!(diag.severity, Severity::Info);
        assert_eq!(diag.category, Category::Other);
        assert_eq!(diag.mode, Mode::MidEdit);
        assert!(diag.remediation_hint.is_none());
        assert!(diag.location.end_line.is_none());
        assert!(diag.location.end_column.is_none());
    }

    #[test]
    fn diagnostic_schema_unknown_severity_value_fails() {
        // Closed enum — unknown variants are a parse error at this layer.
        // Forward-compat mapping (treat unknown severity as warning) is
        // a consumer policy applied above this type.
        let payload = json!({
            "schema_version": "anvil.diagnostic.v1",
            "id": "diag_x",
            "severity": "fatal",
            "summary": "x",
            "location": { "file": "a.rs", "line": 1, "column": 1 },
            "category": "other",
            "source": { "rule_id": "r", "source_module": "m" },
            "mode": "gate"
        });
        let result: Result<Diagnostic, _> = serde_json::from_value(payload);
        assert!(result.is_err());
    }
}
