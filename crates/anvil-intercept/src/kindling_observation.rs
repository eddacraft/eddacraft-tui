//! MLP-016: Rust-side `gate_evaluated` observation builder for the
//! mid-edit L1 path.
//!
//! `packages/kindling-integration/src/observation-contract.ts` defines
//! the wire schema for the nine Kindling observation kinds, including
//! `gate_evaluated`. The mid-edit driver loop produces a `ScanBufferResponse`
//! every keystroke and the daemon needs a stable Rust-side primitive to
//! convert those responses into observation envelopes. The actual
//! database write happens TS-side (the `kindling-integration` package
//! owns the `SQLite` handle); this module just shapes the payload.
//!
//! ## Volume-control contract
//!
//! Per MLP-016's expected outcome: **pass-no-finding mid-edit calls
//! remain silent.** [`from_midedit_response`] returns `None` when the
//! diagnostics vector is empty. The caller is free to call it on
//! every scan; the helper handles the rate filter so the call site
//! does not have to track that policy independently.
//!
//! ## Severity → enforcement mapping
//!
//! Kindling's `enforcement` field is a closed three-value enum
//! (`blocking` / `warning` / `informational`). Diagnostics carry the
//! richer [`Severity`] vocabulary (`Info` / `Warning` / `Error`). The
//! mapping picks the most severe class in the batch:
//!
//! | Highest severity in batch | Kindling `enforcement` |
//! |---------------------------|------------------------|
//! | `Error`                   | `blocking`             |
//! | `Warning`                 | `warning`              |
//! | `Info`                    | `informational`        |
//!
//! Empty diagnostics never produce an observation (see volume-control
//! contract above), so there is no "no diagnostics" row in the table.
//!
//! ## Deferred follow-ups (not v1)
//!
//! - IPC wiring that emits these observations to the
//!   `packages/kindling-integration` consumer — owned by INTD's
//!   notification fan-out layer when the daemon gains a Kindling
//!   client handle. The observation envelope's `session_id` /
//!   `timestamp` / `gate_eval_id` fields are caller-supplied so the
//!   call site stays in control of the identity rules.
//! - MCP shim mirror in `crates/anvil-cli/src/mcp/validation.rs` —
//!   the MCP path needs the same conversion, but the shim's
//!   diagnostic pipeline is its own surface and is not yet wired to
//!   the kindling-integration package either.
//! - Driver-client (TypeScript) mirror at
//!   `packages/anvil-driver-client/src/` — the editor-side L1
//!   surface needs its own observation builder so the driver can
//!   emit when the daemon is unreachable and the embedded fallback
//!   fires.
//! - Coordination with RTAI-007 telemetry contract — the Kindling
//!   row shape here is compatible with the contract; the explicit
//!   joining lands when RTAI-007 surfaces.
//!
//! See `plans/modules/multilayer-protection.aps.md` task MLP-016.

use anvil_kernel_types::Diagnostic;
use anvil_kernel_types::diagnostics::Severity;
use serde::{Deserialize, Serialize};

use crate::midedit::ScanBufferResponse;

/// Pinned Kindling observation kind. Schema-matches
/// `GateEvaluatedObservationSchema.kind` in
/// `packages/kindling-integration/src/observation-contract.ts`.
pub const KIND_GATE_EVALUATED: &str = "gate_evaluated";

/// Pinned `gate_id` for mid-edit findings. Distinguishes L1 mid-edit
/// rows from save-time / pre-commit / pre-push / audit rows that
/// share the `gate_evaluated` kind but live at different layers.
pub const MIDEDIT_GATE_ID: &str = "midEdit";

/// Kindling `enforcement` field values. Closed three-value enum
/// shared with the TS Zod schema; deliberately not derived from
/// [`Severity`] because the mapping is one-directional and would mask
/// future severity additions if it were.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Enforcement {
    /// At least one `Error` diagnostic in the batch.
    Blocking,
    /// Highest severity in the batch is `Warning`.
    Warning,
    /// All diagnostics in the batch are `Info`.
    Informational,
}

/// Kindling `outcome` field. Closed four-value enum matching the TS
/// schema. v1 of this helper only emits `Fail` because empty
/// diagnostics short-circuit before reaching the builder (see volume-
/// control contract on the module).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Outcome {
    Pass,
    Fail,
    Error,
    Skipped,
}

/// Inputs the caller supplies to scope each observation. Identifies
/// the session, time of emission, evaluation id, and the file being
/// evaluated.
///
/// Kept as a separate struct so the call site can build one once per
/// scan and pass it by reference; `ScanBufferResponse` does not carry
/// these fields because they're per-evaluation context the daemon's
/// notification layer owns (`session_id`, traceparent-derived
/// `gate_eval_id`, etc).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObservationContext<'a> {
    /// Session this observation belongs to. UUID v4 string per the
    /// Zod `string().uuid()` contract.
    pub session_id: &'a str,
    /// ISO 8601 datetime — when the daemon observed this scan
    /// completing.
    pub timestamp: &'a str,
    /// Unique evaluation id for joining to traceparent logs.
    pub gate_eval_id: &'a str,
    /// File path being evaluated. Recorded in
    /// `inputs.changed_files` (paths-only; no content per the Zod
    /// "no sensitive data" sanitisation requirement).
    pub file_path: &'a str,
    /// Wall-clock duration the daemon spent on the underlying
    /// pipeline call (the `validation.service` boundary per ADR-031).
    pub duration_ms: u64,
}

/// Kindling `gate_evaluated` observation payload. Serde JSON wire
/// shape matches `GateEvaluatedObservationSchema` from the TS
/// contract: `snake_case` keys, kebab-case enum values, and the same
/// optional / required field policy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GateEvaluatedObservation {
    pub kind: String,
    pub session_id: String,
    pub timestamp: String,
    pub gate_eval_id: String,
    pub gate_id: String,
    pub inputs: ObservationInputs,
    pub outcome: Outcome,
    pub rules_evaluated: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rules_violated: Option<Vec<String>>,
    pub enforcement: Enforcement,
    pub duration_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub violation_count: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warning_count: Option<u32>,
}

/// Nested `inputs` object on a `gate_evaluated` observation. Matches
/// `GateEvaluatedObservationSchema.inputs` from the TS contract.
/// `baseline_hash` is reserved for save-time / pre-commit emitters
/// and omitted from mid-edit rows.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObservationInputs {
    pub file_count: u32,
    pub changed_files: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub baseline_hash: Option<String>,
}

/// Convert a mid-edit [`ScanBufferResponse`] into a Kindling
/// `gate_evaluated` observation, returning `None` when the response
/// has no diagnostics (volume-control contract).
///
/// The caller supplies the per-evaluation [`ObservationContext`]
/// (session id, timestamp, etc) so this helper stays a pure
/// converter — testable without a clock or UUID source.
#[must_use]
pub fn from_midedit_response(
    ctx: &ObservationContext<'_>,
    response: &ScanBufferResponse,
) -> Option<GateEvaluatedObservation> {
    if response.diagnostics.is_empty() {
        return None;
    }

    let enforcement = enforcement_for(&response.diagnostics);
    let (violation_count, warning_count) = counts_for(&response.diagnostics);
    let rules_evaluated: Vec<String> = response
        .diagnostics
        .iter()
        .map(|d| d.source.rule_id.clone())
        .collect();
    let rules_violated: Vec<String> = response
        .diagnostics
        .iter()
        .filter(|d| matches!(d.severity, Severity::Error | Severity::Warning))
        .map(|d| d.source.rule_id.clone())
        .collect();

    Some(GateEvaluatedObservation {
        kind: KIND_GATE_EVALUATED.to_string(),
        session_id: ctx.session_id.to_string(),
        timestamp: ctx.timestamp.to_string(),
        gate_eval_id: ctx.gate_eval_id.to_string(),
        gate_id: MIDEDIT_GATE_ID.to_string(),
        inputs: ObservationInputs {
            file_count: 1,
            changed_files: vec![ctx.file_path.to_string()],
            baseline_hash: None,
        },
        outcome: Outcome::Fail,
        rules_evaluated,
        rules_violated: if rules_violated.is_empty() {
            None
        } else {
            Some(rules_violated)
        },
        enforcement,
        duration_ms: ctx.duration_ms,
        violation_count: Some(violation_count),
        warning_count: Some(warning_count),
    })
}

fn enforcement_for(diagnostics: &[Diagnostic]) -> Enforcement {
    let mut worst = Enforcement::Informational;
    for diag in diagnostics {
        let level = match diag.severity {
            Severity::Error => Enforcement::Blocking,
            Severity::Warning => Enforcement::Warning,
            Severity::Info => Enforcement::Informational,
        };
        if level_rank(level) > level_rank(worst) {
            worst = level;
        }
    }
    worst
}

const fn level_rank(level: Enforcement) -> u8 {
    match level {
        Enforcement::Informational => 0,
        Enforcement::Warning => 1,
        Enforcement::Blocking => 2,
    }
}

fn counts_for(diagnostics: &[Diagnostic]) -> (u32, u32) {
    let mut violations = 0u32;
    let mut warnings = 0u32;
    for diag in diagnostics {
        match diag.severity {
            Severity::Error => violations += 1,
            Severity::Warning => warnings += 1,
            Severity::Info => {}
        }
    }
    (violations, warnings)
}

#[cfg(test)]
mod tests {
    use super::*;
    use anvil_kernel_types::diagnostics::{Category, DiagnosticSource, KnownMode, Location};
    use anvil_kernel_types::{Diagnostic, Mode};

    fn sample_ctx() -> ObservationContext<'static> {
        ObservationContext {
            session_id: "11111111-1111-4111-8111-111111111111",
            timestamp: "2026-05-13T12:00:00Z",
            gate_eval_id: "gate-eval-abc",
            file_path: "src/lib.rs",
            duration_ms: 42,
        }
    }

    fn make_diag(rule_id: &str, severity: Severity) -> Diagnostic {
        Diagnostic::new(
            format!("diag-{rule_id}"),
            severity,
            "test diagnostic",
            Location {
                file: "src/lib.rs".to_string(),
                line: None,
                column: None,
                end_line: None,
                end_column: None,
            },
            Category::Other,
            DiagnosticSource {
                rule_id: rule_id.to_string(),
                source_module: "anvil-checks::test".to_string(),
            },
            Mode::known(KnownMode::MidEdit),
        )
    }

    fn empty_response() -> ScanBufferResponse {
        ScanBufferResponse {
            version: 1,
            diagnostics: Vec::new(),
            truncated: false,
        }
    }

    fn response_with(diagnostics: Vec<Diagnostic>) -> ScanBufferResponse {
        ScanBufferResponse {
            version: 1,
            diagnostics,
            truncated: false,
        }
    }

    #[test]
    fn empty_response_emits_no_observation() {
        let ctx = sample_ctx();
        assert!(
            from_midedit_response(&ctx, &empty_response()).is_none(),
            "pass-no-finding mid-edit calls must remain silent (MLP-016 volume control)",
        );
    }

    #[test]
    fn error_diagnostic_emits_blocking_enforcement() {
        let ctx = sample_ctx();
        let resp = response_with(vec![make_diag("secrets-aws-key", Severity::Error)]);
        let obs = from_midedit_response(&ctx, &resp).expect("observation");
        assert_eq!(obs.enforcement, Enforcement::Blocking);
        assert_eq!(obs.outcome, Outcome::Fail);
        assert_eq!(obs.violation_count, Some(1));
        assert_eq!(obs.warning_count, Some(0));
    }

    #[test]
    fn warning_diagnostic_emits_warning_enforcement() {
        let ctx = sample_ctx();
        let resp = response_with(vec![make_diag("style-nit", Severity::Warning)]);
        let obs = from_midedit_response(&ctx, &resp).expect("observation");
        assert_eq!(obs.enforcement, Enforcement::Warning);
        assert_eq!(obs.violation_count, Some(0));
        assert_eq!(obs.warning_count, Some(1));
    }

    #[test]
    fn info_only_diagnostic_emits_informational_enforcement() {
        let ctx = sample_ctx();
        let resp = response_with(vec![make_diag("info-note", Severity::Info)]);
        let obs = from_midedit_response(&ctx, &resp).expect("observation");
        assert_eq!(obs.enforcement, Enforcement::Informational);
        assert_eq!(obs.violation_count, Some(0));
        assert_eq!(obs.warning_count, Some(0));
    }

    #[test]
    fn mixed_batch_picks_highest_severity() {
        let ctx = sample_ctx();
        let resp = response_with(vec![
            make_diag("info-1", Severity::Info),
            make_diag("warn-1", Severity::Warning),
            make_diag("err-1", Severity::Error),
        ]);
        let obs = from_midedit_response(&ctx, &resp).expect("observation");
        assert_eq!(obs.enforcement, Enforcement::Blocking);
        assert_eq!(obs.violation_count, Some(1));
        assert_eq!(obs.warning_count, Some(1));
    }

    #[test]
    fn rules_violated_excludes_info_severity() {
        let ctx = sample_ctx();
        let resp = response_with(vec![
            make_diag("info-1", Severity::Info),
            make_diag("warn-1", Severity::Warning),
            make_diag("err-1", Severity::Error),
        ]);
        let obs = from_midedit_response(&ctx, &resp).expect("observation");
        let violated = obs.rules_violated.expect("rules_violated present");
        assert_eq!(violated, vec!["warn-1".to_string(), "err-1".to_string()]);
    }

    #[test]
    fn rules_evaluated_records_every_diagnostic() {
        let ctx = sample_ctx();
        let resp = response_with(vec![
            make_diag("info-1", Severity::Info),
            make_diag("warn-1", Severity::Warning),
            make_diag("err-1", Severity::Error),
        ]);
        let obs = from_midedit_response(&ctx, &resp).expect("observation");
        assert_eq!(
            obs.rules_evaluated,
            vec![
                "info-1".to_string(),
                "warn-1".to_string(),
                "err-1".to_string()
            ],
        );
    }

    #[test]
    fn info_only_batch_omits_rules_violated_field() {
        let ctx = sample_ctx();
        let resp = response_with(vec![make_diag("info-1", Severity::Info)]);
        let obs = from_midedit_response(&ctx, &resp).expect("observation");
        assert!(
            obs.rules_violated.is_none(),
            "info-only batches must omit rules_violated to match the Zod optional"
        );
    }

    #[test]
    fn observation_serialises_with_expected_kind_and_gate_id() {
        let ctx = sample_ctx();
        let resp = response_with(vec![make_diag("warn-1", Severity::Warning)]);
        let obs = from_midedit_response(&ctx, &resp).expect("observation");
        let value: serde_json::Value = serde_json::to_value(&obs).expect("observation serialises");
        assert_eq!(value["kind"], KIND_GATE_EVALUATED);
        assert_eq!(value["gate_id"], MIDEDIT_GATE_ID);
        assert_eq!(value["enforcement"], "warning");
        assert_eq!(value["outcome"], "fail");
    }

    #[test]
    fn observation_records_file_path_in_changed_files() {
        let ctx = sample_ctx();
        let resp = response_with(vec![make_diag("err-1", Severity::Error)]);
        let obs = from_midedit_response(&ctx, &resp).expect("observation");
        assert_eq!(obs.inputs.changed_files, vec!["src/lib.rs".to_string()]);
        assert_eq!(obs.inputs.file_count, 1);
        assert!(
            obs.inputs.baseline_hash.is_none(),
            "mid-edit observations should omit baseline_hash"
        );
    }

    #[test]
    fn observation_carries_caller_supplied_identity_fields() {
        let ctx = sample_ctx();
        let resp = response_with(vec![make_diag("err-1", Severity::Error)]);
        let obs = from_midedit_response(&ctx, &resp).expect("observation");
        assert_eq!(obs.session_id, ctx.session_id);
        assert_eq!(obs.timestamp, ctx.timestamp);
        assert_eq!(obs.gate_eval_id, ctx.gate_eval_id);
        assert_eq!(obs.duration_ms, ctx.duration_ms);
    }
}
