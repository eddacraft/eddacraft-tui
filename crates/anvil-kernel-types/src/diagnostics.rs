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
//! (`allow`/`warn`/`block`/`fence`/`interrupt`). INTD-013 maps severity
//! to a control decision per the project's enforcement configuration;
//! that mapping is the daemon's job, not the diagnostic's.

use serde::{Deserialize, Serialize};

/// Current schema-version string for the inner diagnostic shape.
///
/// Distinct from any outer envelope `schema` field. Bumps to
/// `anvil.diagnostic.v2` only on breaking changes; additive evolution
/// stays on `v1` per the spec's versioning rules.
pub const DIAGNOSTIC_SCHEMA_VERSION: &str = "anvil.diagnostic.v1";

/// Rule severity. Deliberately separate from the control decision
/// (`allow`/`warn`/`block`/`fence`/`interrupt`); the daemon owns
/// severity → decision mapping.
///
/// Forward-compatible per the envelope spec ("subscribers MUST treat
/// unknown `severity` values as `warning` … rather than dropping"):
/// deserialising a value this consumer does not recognise yields
/// [`Severity::Unknown`] instead of failing the whole `Diagnostic`
/// parse, and consumers treat `Unknown` as a warning (ADR-096). This
/// `#[serde(other)]` arm mirrors the forward-compat fallback on
/// `MirrorPath` and `AssuranceState` (ADR-085); the sibling [`Mode`]
/// reaches the same tolerance a different way — an untagged
/// `Known | Unknown(String)` shape, not `#[serde(other)]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Severity {
    Info,
    Warning,
    Error,
    /// A `severity` value a newer producer emitted that this consumer
    /// does not recognise. Surfaced (treated as `warning`), never
    /// dropped.
    #[serde(other)]
    Unknown,
}

/// Enforcement decision vocabulary shared by control surfaces. This is
/// deliberately separate from [`Severity`]: rules describe findings with
/// severity, while the caller maps those findings to an enforcement decision.
///
/// This is the canonical outcome axis of the two-axis enforcement model
/// (ADR-098 AD-3): outcome = [`ControlDecision`], posture =
/// [`crate::enforcement::EnforcementMode`]. Every write-vetoing decision
/// ([`ControlDecision::is_veto`]) stops the operation; the surface that
/// consumes the decision projects it onto its own response shape (MCP
/// projects any veto to a write-refusal; the daemon projects `Fence` to
/// a fenced worktree and `Interrupt` to the signal ladder).
///
/// Forward-compatible per ADR-096 / ADR-098 AD-3: an unrecognised
/// decision string a newer producer emitted deserialises to
/// [`ControlDecision::Unknown`] instead of failing the parse, and
/// consumers treat `Unknown` as the safe default (`warn` — never a
/// veto, never an ack-gate). The `#[serde(other)]` arm mirrors the
/// forward-compat fallback on the sibling [`Severity`] and [`Category`]
/// enums. `Unknown` is deliberately the last breaking extension of this
/// enum — cross-binary version skew (anvil-intercept vs anvil are
/// separate binaries) is now observable rather than silently
/// mis-handled.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ControlDecision {
    /// Permit the operation. No finding, or a posture that never blocks.
    Allow,
    /// Surface the finding as a warning; do not stop the operation.
    Warn,
    /// Veto the operation (stop the write) as an outright block. Used by
    /// the pre-write / gate surfaces where there is no worktree to fence
    /// or process to signal.
    Block,
    /// Veto the operation and fence the worktree. On the daemon this
    /// refuses subsequent registrations against the worktree without
    /// signalling active agent processes; on a stateless surface (MCP)
    /// it projects to a write-refusal like any other veto. A distinct
    /// decision from [`ControlDecision::Block`] so the true enforcement
    /// action stays auditable rather than collapsing at parse time
    /// (ADR-098 AD-3).
    Fence,
    /// Veto the operation and issue a process-group interrupt against the
    /// attributing session (the daemon's strictest action), then fence.
    Interrupt,
    /// A decision value a newer producer emitted that this consumer does
    /// not recognise. Treated as the safe default (`warn`): surfaced,
    /// never a veto, never an ack-gate, never dropped (ADR-098 AD-3).
    #[serde(other)]
    Unknown,
}

impl ControlDecision {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Allow => "allow",
            Self::Warn => "warn",
            Self::Block => "block",
            Self::Fence => "fence",
            Self::Interrupt => "interrupt",
            Self::Unknown => "unknown",
        }
    }

    /// Whether this decision vetoes the operation (stops the write).
    ///
    /// [`ControlDecision::Block`], [`ControlDecision::Fence`], and
    /// [`ControlDecision::Interrupt`] are all vetoes — a surface that
    /// gates `isError` / write-refusal on the decision MUST use this
    /// helper rather than a `== Block` comparison, or a fence-vetoed
    /// write silently reports success (ADR-098 AD-3, amendment 1).
    ///
    /// [`ControlDecision::Allow`], [`ControlDecision::Warn`], and
    /// [`ControlDecision::Unknown`] are not vetoes: `Unknown` degrades to
    /// the safe `warn` default and never blocks a write on a value this
    /// consumer cannot interpret.
    #[must_use]
    pub const fn is_veto(self) -> bool {
        matches!(self, Self::Block | Self::Fence | Self::Interrupt)
    }
}

/// Coarse routing/filtering grouping for diagnostics. Spec-defined
/// producers emit one of the named values; the wire type is
/// forward-compatible so a consumer running an older spec can still
/// surface a diagnostic from a newer producer that introduces an
/// additional category (envelope spec: "subscribers MUST treat …
/// unknown `category` values as `other` … rather than dropping").
/// Unrecognised values deserialise to [`Category::Unknown`] instead of
/// failing the parse; a consumer that routes by category would treat it
/// as [`Category::Other`] (no such routing consumer exists today —
/// `category` is carried, not switched on) (ADR-096).
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
    /// A `category` value a newer producer emitted that this consumer
    /// does not recognise. Surfaced (a category-routing consumer would
    /// treat it as [`Category::Other`]), never dropped.
    #[serde(other)]
    Unknown,
}

/// Known mode values. Spec-defined producers emit one of these; the
/// outer [`Mode`] type is what travels on the wire so a consumer
/// running an older spec can still surface a diagnostic produced by a
/// newer producer that introduces an additional mode value (e.g. a
/// future `remote-edit`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum KnownMode {
    SaveTime,
    MidEdit,
    Gate,
    Watch,
}

/// Mode discriminator. Identifies which path produced the diagnostic
/// and the consumer expectation. See the envelope spec's "Mode
/// Discriminator Semantics" table for the per-mode contract.
///
/// Per the envelope spec ("a consumer that doesn't recognise a mode
/// value MUST surface the diagnostic anyway"), this enum accepts
/// unknown mode strings on deserialisation and round-trips them.
/// Consumers branch on `Known(_)` for known values and treat
/// `Unknown(_)` as informational per the spec's forward-compat rule.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Mode {
    Known(KnownMode),
    Unknown(String),
}

impl Mode {
    /// Convenience for the common case of constructing from a known
    /// variant.
    #[must_use]
    pub const fn known(value: KnownMode) -> Self {
        Self::Known(value)
    }
}

impl From<KnownMode> for Mode {
    fn from(value: KnownMode) -> Self {
        Self::Known(value)
    }
}

/// File anchor for a diagnostic. `line`/`column` are 1-based when
/// present; path-only rules and deleted-file diagnostics omit them
/// per the envelope spec ("`line` may be `null`"). `end_line` /
/// `end_column` are optional and span the end of the flagged region
/// when present.
///
/// Integer fields use `u32` rather than `usize` so the wire schema is
/// stable across 32-bit and 64-bit producers and across language
/// bindings — `usize` JSON-serialises as different things depending
/// on platform.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Location {
    pub file: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub column: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end_line: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end_column: Option<u32>,
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
            line: Some(42),
            column: Some(18),
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
            Mode::known(KnownMode::SaveTime),
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
    fn diagnostic_schema_decision_variants_serialise_kebab_case() {
        let cases = [
            (ControlDecision::Allow, "allow"),
            (ControlDecision::Warn, "warn"),
            (ControlDecision::Block, "block"),
            (ControlDecision::Fence, "fence"),
            (ControlDecision::Interrupt, "interrupt"),
            (ControlDecision::Unknown, "unknown"),
        ];

        for (variant, expected) in cases {
            let json = serde_json::to_value(variant).expect("serialise");
            assert_eq!(json, expected);
            let back: ControlDecision = serde_json::from_value(json).expect("deserialise");
            assert_eq!(back, variant);
            assert_eq!(variant.as_str(), expected);
        }
    }

    #[test]
    fn control_decision_unknown_value_deserialises_forward_compat() {
        // ADR-098 AD-3: a decision string a newer producer emits that
        // this consumer does not recognise must deserialise to `Unknown`
        // (the safe `warn` default) rather than failing the parse —
        // cross-binary version skew (anvil-intercept vs anvil) is real.
        let decision: ControlDecision =
            serde_json::from_value(json!("quarantine")).expect("unknown decision deserialises");
        assert_eq!(decision, ControlDecision::Unknown);
        // `Unknown` round-trips through its own `"unknown"` tag.
        assert_eq!(
            serde_json::to_value(ControlDecision::Unknown).unwrap(),
            "unknown"
        );
        assert_eq!(
            serde_json::from_value::<ControlDecision>(json!("unknown")).unwrap(),
            ControlDecision::Unknown
        );
    }

    #[test]
    fn control_decision_is_veto_covers_block_fence_interrupt() {
        // ADR-098 AD-3, amendment 1: Block, Fence, and Interrupt are all
        // vetoes; Allow, Warn, and Unknown are not (Unknown degrades to
        // the safe `warn` default and never blocks a write).
        assert!(ControlDecision::Block.is_veto());
        assert!(ControlDecision::Fence.is_veto());
        assert!(ControlDecision::Interrupt.is_veto());
        assert!(!ControlDecision::Allow.is_veto());
        assert!(!ControlDecision::Warn.is_veto());
        assert!(!ControlDecision::Unknown.is_veto());
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
            (KnownMode::SaveTime, "save-time"),
            (KnownMode::MidEdit, "mid-edit"),
            (KnownMode::Gate, "gate"),
            (KnownMode::Watch, "watch"),
        ];
        for (variant, expected) in cases {
            let mode = Mode::known(variant);
            let json = serde_json::to_value(&mode).expect("serialise");
            assert_eq!(
                json, expected,
                "variant {variant:?} must serialise to {expected}"
            );
            let back: Mode = serde_json::from_value(json).expect("deserialise");
            assert_eq!(back, mode);
        }
    }

    #[test]
    fn diagnostic_schema_unknown_mode_value_round_trips_as_unknown() {
        // Per envelope spec: "a consumer that doesn't recognise a mode
        // value MUST surface the diagnostic anyway". Closed-enum
        // deserialisation would drop the diagnostic; we accept the
        // unknown string and let the consumer apply its forward-compat
        // policy (treat as informational).
        let payload = json!("remote-edit");
        let mode: Mode = serde_json::from_value(payload).expect("deserialise");
        assert_eq!(mode, Mode::Unknown("remote-edit".to_string()));
        let back = serde_json::to_value(&mode).expect("serialise");
        assert_eq!(back, "remote-edit");
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
            Mode::known(KnownMode::Gate),
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
                line: Some(1),
                column: Some(1),
                end_line: None,
                end_column: None,
            },
            Category::Other,
            sample_source(),
            Mode::known(KnownMode::Watch),
        );
        let value: Value = serde_json::to_value(&diag).expect("to_value");
        assert!(value["location"].get("end_line").is_none());
        assert!(value["location"].get("end_column").is_none());
    }

    #[test]
    fn diagnostic_schema_location_omits_line_and_column_when_none() {
        // Spec: "for deleted-file or path-only rules, `line` may be
        // `null`". Mirror that — both line and column are optional.
        let diag = Diagnostic::new(
            "diag_01HW8K6Q4P0X7N9TJ4YA3S0VPATH",
            Severity::Info,
            "Path-only finding",
            Location {
                file: "README.md".into(),
                line: None,
                column: None,
                end_line: None,
                end_column: None,
            },
            Category::Other,
            sample_source(),
            Mode::known(KnownMode::Watch),
        );
        let value: Value = serde_json::to_value(&diag).expect("to_value");
        assert_eq!(value["location"]["file"], "README.md");
        assert!(value["location"].get("line").is_none());
        assert!(value["location"].get("column").is_none());

        // Round-trip a path-only location.
        let back: Diagnostic = serde_json::from_value(value).expect("deserialise");
        assert_eq!(back, diag);
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
        assert_eq!(diag.mode, Mode::known(KnownMode::MidEdit));
        assert!(diag.remediation_hint.is_none());
        assert!(diag.location.end_line.is_none());
        assert!(diag.location.end_column.is_none());
    }

    #[test]
    fn diagnostic_schema_unknown_severity_value_round_trips_as_unknown() {
        // Envelope spec: subscribers MUST treat unknown `severity` values
        // as `warning` rather than dropping the diagnostic (ADR-096). A
        // newer producer's `severity: "fatal"` must deserialise the whole
        // Diagnostic (not error out) with `severity == Unknown`; consumers
        // apply the warning treatment. Mirrors the `Mode` unknown round-trip.
        let severity: Severity =
            serde_json::from_value(json!("fatal")).expect("unknown severity deserialises");
        assert_eq!(severity, Severity::Unknown);

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
        let diag: Diagnostic =
            serde_json::from_value(payload).expect("diagnostic with unknown severity parses");
        assert_eq!(diag.severity, Severity::Unknown);
        // `Unknown` serialises to the `"unknown"` tag and round-trips back.
        assert_eq!(serde_json::to_value(Severity::Unknown).unwrap(), "unknown");
        assert_eq!(
            serde_json::from_value::<Severity>(json!("unknown")).unwrap(),
            Severity::Unknown
        );
    }

    #[test]
    fn diagnostic_schema_unknown_category_value_round_trips_as_unknown() {
        // Envelope spec: unknown `category` values are surfaced (routed as
        // `other`), never dropped (ADR-096).
        let category: Category =
            serde_json::from_value(json!("quantum")).expect("unknown category deserialises");
        assert_eq!(category, Category::Unknown);
        assert_eq!(serde_json::to_value(Category::Unknown).unwrap(), "unknown");
    }
}
