//! MLP-016: Rust-side `gate_evaluated` observation builder for the
//! mid-edit L1 path.
//!
//! `packages/kindling-integration/src/observation-contract.ts` defines
//! the wire schema for the Kindling observation kinds, including
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

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use anvil_kernel_types::Diagnostic;
use anvil_kernel_types::diagnostics::Severity;
use anvil_observability::TraceContext;
use anvil_observability::redaction::ArgShape;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::midedit::ScanBufferResponse;
use crate::rate_window::{RateDecision, RateWindow};

/// Pinned Kindling observation kind. Schema-matches
/// `GateEvaluatedObservationSchema.kind` in
/// `packages/kindling-integration/src/observation-contract.ts`.
pub const KIND_GATE_EVALUATED: &str = "gate_evaluated";

/// Pinned `gate_id` for mid-edit findings. Distinguishes L1 mid-edit
/// rows from save-time / pre-commit / pre-push / audit rows that
/// share the `gate_evaluated` kind but live at different layers.
pub const MIDEDIT_GATE_ID: &str = "midEdit";

/// MLP2-008: the canonical `gate_eval_id` join key, extracted from a
/// W3C `traceparent`.
///
/// This is the **single source** for the join key shared by the two
/// mid-edit surfaces that need to correlate downstream:
///
/// - the Kindling `gate_evaluated` row (this module), and
/// - the RTAI-007 mid-edit telemetry envelope
///   (`crate::telemetry::NotificationMirror::gate_eval_id`).
///
/// Both call this extractor so a row and its originating telemetry
/// envelope carry **byte-identical** `gate_eval_id` values whenever a
/// valid `traceparent` is in scope, making the join
/// `envelope.mirror.gate_eval_id == row.gate_eval_id` exact. The
/// daemon IPC handler's `derive_gate_eval_id` also delegates here (then
/// applies its own UUID-v4 fallback), so there is exactly one
/// definition of "what the join key is".
///
/// Returns the W3C **parent-id** (the 16 lower-hex-char upstream span
/// id) — the field consumers join against — or `None` when the
/// `traceparent` is absent or unparseable. Callers that must always
/// emit an id (the Kindling row) apply their own fallback; callers for
/// which an unjoinable random id would be worse than absence (the
/// telemetry envelope) leave the field unset on `None`.
///
/// ## Field map: RTAI-007 mid-edit envelope → `gate_evaluated` row
///
/// The explicit contract a subscriber uses to join an
/// `anvil.notification.v1` mid-edit envelope back to its Kindling row:
///
/// | RTAI-007 envelope field                | `gate_evaluated` row field   | Relationship |
/// |----------------------------------------|------------------------------|--------------|
/// | `mirror.gate_eval_id`                  | `gate_eval_id`               | **join key** — both = `traceparent` parent-id via this fn |
/// | `correlation.session_id`               | `session_id`                 | same daemon/edit session |
/// | `notification.context.file`            | `inputs.changed_files[0]`    | the file evaluated (paths-only) |
/// | `mirror.decision` (`warn`/`block`)     | `enforcement`                | same advisory severity class |
/// | `timestamp`                            | `timestamp`                  | producer wall-clock at emit |
///
/// Note: a row exists only for a finding-bearing scan — an `allow`
/// (no-finding) decision emits **no** `gate_evaluated` row (see
/// [`from_midedit_response`]), so there is nothing to join for `allow`.
/// The row's `outcome` is always `Fail` on the mid-edit path (a row
/// only exists when there is a finding); the severity distinction
/// (`warn` vs `block`) lives in `enforcement`, not `outcome`.
///
/// `gate_id` on the row is pinned to [`MIDEDIT_GATE_ID`] (`"midEdit"`),
/// matching the envelope's `mirror.path = midEdit` discriminator, so a
/// joined pair is provably the same mid-edit evaluation.
#[must_use]
pub fn gate_eval_id_from_traceparent(traceparent: Option<&str>) -> Option<String> {
    traceparent
        .and_then(|raw| TraceContext::parse(raw).ok())
        .map(|ctx| ctx.parent_id().to_string())
}

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
    /// MLP2-056 — `true` when the producing audit walk exited early
    /// because a time budget fired. Additive-optional; mid-edit /
    /// pre-push emitters omit the field, so the wire shape stays
    /// byte-compat with pre-MLP2-056 consumers.
    #[serde(default, skip_serializing_if = "is_false")]
    pub partial: bool,
}

#[inline]
// `skip_serializing_if` requires `fn(&T) -> bool`; passing `bool` by
// value would not satisfy serde's contract.
#[allow(clippy::trivially_copy_pass_by_ref)]
const fn is_false(b: &bool) -> bool {
    !*b
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

// ---------------------------------------------------------------------
// USAGE-001: command-invocation observations
// ---------------------------------------------------------------------

/// Pinned Kindling observation kind for a user-initiated command (CLI)
/// or JSON-RPC method invocation. Schema-matches
/// `CommandInvokedObservationSchema.kind` in
/// `packages/kindling-integration/src/observation-contract.ts`.
pub const KIND_COMMAND_INVOKED: &str = "command.invoked";

/// One resolved feature-flag entry captured inline on a usage row, per
/// ADR-041 (`plans/decisions/041-flag-snapshot-usage-join-contract.md`).
///
/// USAGE-001 emits the row with an **empty** `flag_set`; USAGE-002 owns
/// populating it from the resolver at the invocation boundary. The
/// shape is pinned here so the wire contract is stable before the
/// producer fills it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FlagSetEntry {
    /// Canonical manifest `key` — the stable join key (ADR-041 D-2).
    pub key: String,
    /// Resolved variant for this invocation.
    pub variant: String,
    /// Where the value came from: `snapshot` | `override` | `default`.
    pub source: String,
    /// Whether this flag is gate-affecting (ADR-019 boundary).
    pub gate_affecting: bool,
}

/// Per-invocation identity the caller supplies so this module stays a
/// pure converter (no clock, UUID source, or secrets access of its
/// own).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandInvocationContext<'a> {
    /// Per-invocation UUID v4 string (base-shape `session_id`).
    pub session_id: &'a str,
    /// RFC 3339 / ISO 8601 datetime when the invocation was observed.
    pub timestamp: &'a str,
    /// Canonical command or method name (e.g. `check`, `session.list`).
    pub command: &'a str,
    /// Anonymised principal — a one-way hash, or `anonymous` when no
    /// identity is on the call path. The raw principal MUST NOT appear.
    pub principal: &'a str,
    /// W3C `traceparent` for cross-pipe correlation when one was bound
    /// on the invocation; `None` otherwise (ADR-035).
    pub traceparent: Option<&'a str>,
}

/// Kindling `command.invoked` observation payload (USAGE-001). Records
/// *that* a command ran and the redacted *shape* of its arguments —
/// never argument values, results, or output. See the privacy contract
/// at `docs/observability/usage-analytics.md`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandInvokedObservation {
    pub kind: String,
    pub session_id: String,
    pub timestamp: String,
    pub command: String,
    pub principal: String,
    /// Redacted per-argument shapes (names + value shape, no values).
    pub args: Vec<ArgShape>,
    /// Inline resolved flag context (ADR-041). Empty for USAGE-001;
    /// always present (never omitted) so consumers can distinguish
    /// "no flags resolved" from "field missing". Do NOT add
    /// `skip_serializing_if = "Vec::is_empty"` here — the always-present
    /// `flag_set: []` invariant is the contract.
    #[serde(default)]
    pub flag_set: Vec<FlagSetEntry>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub traceparent: Option<String>,
}

/// Build a [`CommandInvokedObservation`] from per-invocation context,
/// already-redacted argument shapes, and the inline flag set.
///
/// Pure: the caller owns identity minting, argument redaction (via
/// [`anvil_observability::redaction::redact_arg`]), and flag
/// resolution, so this helper is testable without a clock, a secrets
/// store, or a resolver.
#[must_use]
pub fn from_command_invocation(
    ctx: &CommandInvocationContext<'_>,
    args: Vec<ArgShape>,
    flag_set: Vec<FlagSetEntry>,
) -> CommandInvokedObservation {
    CommandInvokedObservation {
        kind: KIND_COMMAND_INVOKED.to_string(),
        session_id: ctx.session_id.to_string(),
        timestamp: ctx.timestamp.to_string(),
        command: ctx.command.to_string(),
        principal: ctx.principal.to_string(),
        args,
        flag_set,
        traceparent: ctx.traceparent.map(ToString::to_string),
    }
}

/// USAGE-004: derive redacted argument shapes from a JSON-RPC `params`
/// object, the daemon-path analogue of the CLI's `arg_shapes_from_argv`.
///
/// Each top-level key becomes one [`ArgShape`] via
/// [`anvil_observability::redaction::redact_arg`], so the privacy
/// contract is identical to the CLI path: a sensitive-named key is
/// elided to the `<redacted>` marker, every other value contributes
/// only its coarse shape (type + length bucket + presence), never the
/// raw value. Keys are visited in sorted order so a row is deterministic
/// regardless of the wire object's key order.
///
/// Value mapping: a JSON string feeds `redact_arg` directly; `null` is
/// treated as an absent value (bare-flag shape); scalars (bool/number)
/// are stringified compactly; nested arrays/objects are compact-JSON
/// serialised purely to measure their coarse length bucket — the bucket
/// (never an exact length) is the backstop that keeps structure size
/// from leaking. A non-object `params` (or `Null`) yields no shapes.
#[must_use]
pub fn arg_shapes_from_params(params: &serde_json::Value) -> Vec<ArgShape> {
    use anvil_observability::redaction::redact_arg;
    use serde_json::Value;

    let Some(map) = params.as_object() else {
        return Vec::new();
    };
    let mut keys: Vec<&String> = map.keys().collect();
    keys.sort();
    keys.into_iter()
        .map(|key| match &map[key] {
            Value::Null => redact_arg(key, None),
            Value::String(s) => redact_arg(key, Some(s)),
            Value::Bool(b) => redact_arg(key, Some(if *b { "true" } else { "false" })),
            Value::Number(n) => redact_arg(key, Some(&n.to_string())),
            other => redact_arg(key, Some(&other.to_string())),
        })
        .collect()
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
        // Mid-edit emissions are never partial — the scan either
        // completes or returns no diagnostics.
        partial: false,
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

// MLP2-054: audit-chain `gate_evaluated` builder -----------------------
//
// `anvil audit-chain` (MLP-015) walks a branch's commits and reports any
// that lack a corresponding L3 witness. The builder below converts that
// audit run into a Kindling row so historical drift is queryable through
// the observation timeline. The CLI is responsible for assembling the
// inputs and delivering the row to a sink (see
// `anvil-cli::commands::audit_chain`); the builder itself stays a pure
// converter so it can be exercised without a clock, UUID source, or
// filesystem.

/// Pinned `gate_id` for audit-chain rows. Distinguishes L5 audit-chain
/// findings from mid-edit / pre-commit / pre-push rows that share the
/// `gate_evaluated` kind but live at different layers.
pub const AUDIT_CHAIN_GATE_ID: &str = "audit-chain";

/// Synthetic rule id for "every commit must have a witness." Audit-
/// chain v1 is a witness-presence check (ADR-037 §D-9); each commit
/// without a witness counts as a violation of this rule.
pub const AUDIT_CHAIN_WITNESS_PRESENCE_RULE_ID: &str = "anvil.audit.witness-presence";

/// Synthetic rule id for "witness chain verifies end-to-end." Distinct
/// from `witness-presence` so tamper evidence surfaces as its own
/// violated rule rather than being conflated with simple drift.
pub const AUDIT_CHAIN_CHAIN_INTACT_RULE_ID: &str = "anvil.audit.chain-intact";

/// Per-invocation context the audit-chain CLI supplies to the builder.
/// Kept separate from [`ObservationContext`] because audit-chain rows
/// scope by branch + commit window rather than a single edited file —
/// `changed_files` is empty, and the audit's "file count" is replaced
/// with the commit count via [`AuditChainSummary::commits_walked`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditChainContext<'a> {
    /// Session id for this invocation. UUID v4 per the Zod
    /// `string().uuid()` contract; the CLI generates a fresh one per
    /// audit-chain run.
    pub session_id: &'a str,
    /// ISO 8601 datetime — when the audit completed.
    pub timestamp: &'a str,
    /// Unique evaluation id for joining to traceparent logs.
    pub gate_eval_id: &'a str,
    /// Wall-clock duration of the audit walk in milliseconds.
    pub duration_ms: u64,
}

/// Audit-chain summary the CLI passes to the builder. Decoupled from
/// the CLI's `AuditReport` struct so this crate doesn't depend on
/// `anvil-cli`; the CLI maps `AuditReport` → `AuditChainSummary` at
/// the call site.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditChainSummary<'a> {
    /// Total number of commits the audit walked.
    pub commits_walked: usize,
    /// Number of unwitnessed commits found. Maps to
    /// `violation_count` on the resulting observation.
    pub unwitnessed_count: usize,
    /// `true` when the witness chain verified end-to-end; `false`
    /// means existing witness files failed verification (tamper
    /// evidence per ADR-038 §D-6).
    pub chain_intact: bool,
    /// `true` when the audit walk exited early because a time budget
    /// fired (MLP2-056). Partial runs still emit a row so the timeline
    /// records that the audit ran; the bit is propagated to the wire
    /// observation so consumers can surface it.
    pub partial: bool,
    /// `true` when `unwitnessed_count` meets or exceeds the operator-
    /// configured threshold and the run should surface as degraded.
    pub degraded_audit_drift: bool,
    /// Hex line-hash of the most recent witness line. Maps to
    /// `inputs.baseline_hash`; omitted when the chain is empty.
    pub chain_head_hash: Option<&'a str>,
}

/// Build a Kindling `gate_evaluated` row for an audit-chain run.
///
/// Pure converter — does no IO, no clock reads, no UUID minting. The
/// CLI supplies the context (see [`AuditChainContext`]) and the
/// summary (see [`AuditChainSummary`]).
///
/// Mapping:
///
/// - `outcome`: `Pass` when the chain is intact, drift is below
///   threshold, AND the walk completed; `Fail` otherwise. A partial
///   walk (`summary.partial == true`) maps to `Fail` because the
///   audit didn't actually finish — surfacing it as `Pass` would let
///   a runaway nightly cron silently report green.
/// - `enforcement`: `Blocking` on drift, tamper, or partial-walk
///   conditions; `Informational` when clean and complete. `Warning`
///   is reserved for layers that emit non-blocking advisories —
///   audit-chain is a backstop, so v1 collapses to the binary case.
/// - `rules_violated`: lists [`AUDIT_CHAIN_WITNESS_PRESENCE_RULE_ID`]
///   when drift triggered, plus [`AUDIT_CHAIN_CHAIN_INTACT_RULE_ID`]
///   when the chain failed verification. Omitted when neither failure
///   condition holds (matches the Zod-optional contract). A partial
///   run alone does not add a rule to `rules_violated` — the bit
///   travels via the dedicated `partial` field instead so consumers
///   can distinguish "no rule was broken; we just ran out of time"
///   from a substantive failure.
/// - `inputs.baseline_hash`: copied from
///   [`AuditChainSummary::chain_head_hash`]; the line-hash of the
///   audit's most recent witness is the natural baseline reference.
/// - `inputs.file_count`: set to `commits_walked` so consumers can
///   tell how much history each row covers. `changed_files` is
///   always empty — audit-chain works at commit granularity, not
///   file granularity, and the Zod schema allows empty arrays.
/// - `violation_count`: `unwitnessed_count` (saturating to `u32::MAX`
///   on the absurd-size case).
/// - `partial`: copied from [`AuditChainSummary::partial`] so a
///   downstream consumer tailing the NDJSON stream can route on
///   partial runs without re-deriving them.
#[must_use]
pub fn from_audit_chain(
    ctx: &AuditChainContext<'_>,
    summary: &AuditChainSummary<'_>,
) -> GateEvaluatedObservation {
    // A partial walk counts as a failure for outcome / enforcement
    // purposes — the audit ran out of time before it could prove the
    // chain green. Without this, a runaway nightly cron would report
    // `outcome: pass` while quietly skipping commits.
    let failed = summary.degraded_audit_drift || !summary.chain_intact || summary.partial;
    let outcome = if failed { Outcome::Fail } else { Outcome::Pass };
    let enforcement = if failed {
        Enforcement::Blocking
    } else {
        Enforcement::Informational
    };

    let rules_evaluated = vec![
        AUDIT_CHAIN_WITNESS_PRESENCE_RULE_ID.to_string(),
        AUDIT_CHAIN_CHAIN_INTACT_RULE_ID.to_string(),
    ];
    let mut violated: Vec<String> = Vec::new();
    if summary.degraded_audit_drift {
        violated.push(AUDIT_CHAIN_WITNESS_PRESENCE_RULE_ID.to_string());
    }
    if !summary.chain_intact {
        violated.push(AUDIT_CHAIN_CHAIN_INTACT_RULE_ID.to_string());
    }
    let rules_violated = if violated.is_empty() {
        None
    } else {
        Some(violated)
    };

    let file_count = u32::try_from(summary.commits_walked).unwrap_or(u32::MAX);
    let violation_count = u32::try_from(summary.unwitnessed_count).unwrap_or(u32::MAX);

    GateEvaluatedObservation {
        kind: KIND_GATE_EVALUATED.to_string(),
        session_id: ctx.session_id.to_string(),
        timestamp: ctx.timestamp.to_string(),
        gate_eval_id: ctx.gate_eval_id.to_string(),
        gate_id: AUDIT_CHAIN_GATE_ID.to_string(),
        inputs: ObservationInputs {
            file_count,
            changed_files: Vec::new(),
            baseline_hash: summary.chain_head_hash.map(str::to_string),
        },
        outcome,
        rules_evaluated,
        rules_violated,
        enforcement,
        duration_ms: ctx.duration_ms,
        violation_count: Some(violation_count),
        // Audit-chain has no notion of "warning"; v1 collapses to
        // pass/fail. Omit the field so consumers don't read a
        // misleading zero.
        warning_count: None,
        partial: summary.partial,
    }
}

// MLP2-006: daemon-side notification fan-out for `gate_evaluated` ----
//
// The pieces below wire the [`from_midedit_response`] builder above into
// the daemon's mid-edit scan path. The `KindlingObservationSink` trait
// is the abstraction over "where the row ends up": today the daemon
// ships with [`NoopKindlingObservationSink`] by default and the host
// (CLI / tests / future embed) supplies a real sink at startup. The
// concrete delivery path to the TS-side `kindling-integration` package
// is the deferred follow-up tracked alongside MLP2-007 / MLP2-008 — but
// the trait + emitter contract here is the stable seam those wirings
// snap into without disturbing the scan_buffer hot path.
//
// The emitter:
//
// - **Short-circuits on no-finding** by calling [`from_midedit_response`],
//   which already returns `None` for empty diagnostics (volume-control
//   contract).
// - **Throttles** through a [`RateWindow`] so a keystroke burst cannot
//   flood the sink. The default cap matches the MLP2-009 spec
//   ([`DEFAULT_MIDEDIT_EMIT_CAPACITY`] events per
//   [`DEFAULT_MIDEDIT_EMIT_WINDOW`]).
// - **Never blocks the scan** on sink failure: errors are logged via
//   `tracing::warn!` and surfaced as [`EmissionOutcome::SinkError`] so
//   tests can observe the drop, but the call signature is infallible
//   from the caller's perspective.

/// Errors a [`KindlingObservationSink`] can surface back to the
/// emitter. The emitter logs and drops these — the scan response
/// always succeeds regardless of sink health (MLP2-006 expected
/// outcome: "Failure to write to Kindling is logged at the daemon
/// level but does NOT block the scan response").
#[derive(Debug, Error)]
pub enum KindlingSinkError {
    /// The sink received the observation but rejected it (schema
    /// mismatch, duplicate id, etc).
    #[error("kindling sink rejected observation: {0}")]
    Rejected(String),
    /// The sink could not be reached (IPC closed, DB locked, etc).
    /// Daemon retries are NOT this layer's concern — drop the row
    /// and rely on the next scan to produce a fresh observation.
    #[error("kindling sink unavailable: {0}")]
    Unavailable(String),
}

/// Where the daemon sends a built [`GateEvaluatedObservation`].
/// Implementations must be cheap because the trait is called on the
/// `scan_buffer` hot path.
///
/// The trait is sync on purpose: async sinks should spawn their own
/// background work (channel send, IPC frame enqueue) and return
/// immediately. The emitter does not await sink work — by contract,
/// `try_emit` returns before the row reaches its destination.
pub trait KindlingObservationSink: Send + Sync {
    fn try_emit(&self, observation: GateEvaluatedObservation) -> Result<(), KindlingSinkError>;

    /// MLP2-010: deliver an `action_executed` row produced by the
    /// post-commit / post-merge / post-rewrite hook surface.
    /// Defaulted to `Ok(())` so existing sinks (Noop, future custom
    /// impls) auto-satisfy the extended trait without churn — only
    /// sinks that genuinely need to consume the new row override.
    /// Tests can override via [`RecordingKindlingObservationSink`].
    fn try_emit_action_executed(
        &self,
        _observation: ActionExecutedObservation,
    ) -> Result<(), KindlingSinkError> {
        Ok(())
    }
}

/// Default sink used when the daemon was started without a Kindling
/// integration (embedded fallback, headless tests, hosts that
/// disabled observation export). Always returns `Ok` and discards
/// the row — no error is logged because the absence of a sink is a
/// configuration choice, not a failure mode.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoopKindlingObservationSink;

impl KindlingObservationSink for NoopKindlingObservationSink {
    fn try_emit(&self, _observation: GateEvaluatedObservation) -> Result<(), KindlingSinkError> {
        Ok(())
    }
}

/// Test-only sink that records every observation it receives. The
/// emitter holds the sink behind an `Arc<dyn ...>`, so tests construct
/// one of these inside an `Arc` and keep a clone to assert against.
#[derive(Debug, Default)]
pub struct RecordingKindlingObservationSink {
    observations: Mutex<Vec<GateEvaluatedObservation>>,
    actions: Mutex<Vec<ActionExecutedObservation>>,
    fail_next: Mutex<Option<KindlingSinkError>>,
    fail_next_action: Mutex<Option<KindlingSinkError>>,
}

impl RecordingKindlingObservationSink {
    /// Construct an empty recorder.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Inject a one-shot failure for the next [`Self::try_emit`] call.
    /// Tests use this to drive the sink-error logging path without
    /// reaching for a real failing sink.
    pub fn fail_next_with(&self, error: KindlingSinkError) {
        *self.fail_next.lock().expect("fail_next mutex") = Some(error);
    }

    /// MLP2-010: inject a one-shot failure for the next
    /// [`Self::try_emit_action_executed`] call so tests can exercise
    /// the post-hook sink-error swallow path without reaching for a
    /// real failing sink.
    pub fn fail_next_action_with(&self, error: KindlingSinkError) {
        *self
            .fail_next_action
            .lock()
            .expect("fail_next_action mutex") = Some(error);
    }

    /// Snapshot the recorded `gate_evaluated` observations in arrival
    /// order.
    #[must_use]
    pub fn recorded(&self) -> Vec<GateEvaluatedObservation> {
        self.observations
            .lock()
            .expect("observations mutex")
            .clone()
    }

    /// MLP2-010: snapshot the recorded `action_executed` observations
    /// in arrival order.
    #[must_use]
    pub fn recorded_actions(&self) -> Vec<ActionExecutedObservation> {
        self.actions.lock().expect("actions mutex").clone()
    }

    /// Number of `gate_evaluated` observations recorded.
    #[must_use]
    pub fn len(&self) -> usize {
        self.observations.lock().expect("observations mutex").len()
    }

    /// MLP2-010: number of `action_executed` observations recorded.
    #[must_use]
    pub fn actions_len(&self) -> usize {
        self.actions.lock().expect("actions mutex").len()
    }

    /// True when no observations of either kind have been recorded.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0 && self.actions_len() == 0
    }
}

impl KindlingObservationSink for RecordingKindlingObservationSink {
    fn try_emit(&self, observation: GateEvaluatedObservation) -> Result<(), KindlingSinkError> {
        if let Some(err) = self.fail_next.lock().expect("fail_next mutex").take() {
            return Err(err);
        }
        self.observations
            .lock()
            .expect("observations mutex")
            .push(observation);
        Ok(())
    }

    fn try_emit_action_executed(
        &self,
        observation: ActionExecutedObservation,
    ) -> Result<(), KindlingSinkError> {
        if let Some(err) = self
            .fail_next_action
            .lock()
            .expect("fail_next_action mutex")
            .take()
        {
            return Err(err);
        }
        self.actions
            .lock()
            .expect("actions mutex")
            .push(observation);
        Ok(())
    }
}

/// Per-call inputs the IPC handler supplies for each emission.
/// Holds borrowed strings so the IPC handler can build one inline
/// from the JSON-RPC frame and request without allocating extra
/// owned `String`s.
#[derive(Debug, Clone, Copy)]
pub struct MidEditEmissionRequest<'a> {
    /// Unique evaluation id for joining the row back to the
    /// originating telemetry envelope. Callers derive this from the
    /// W3C `traceparent`'s `span_id` (MLP2-008 contract).
    pub gate_eval_id: &'a str,
    /// File path being evaluated. Becomes
    /// `inputs.changed_files[0]` on the row.
    pub file_path: &'a str,
    /// ISO 8601 datetime — when the daemon observed this scan
    /// completing.
    pub timestamp: &'a str,
    /// Wall-clock duration the daemon spent on the underlying
    /// pipeline call (the `validation.service` boundary per ADR-031).
    pub duration_ms: u64,
}

/// Outcome of [`MidEditObservationEmitter::try_emit`]. Tests assert
/// on the variant; production callers may ignore the return value
/// because the emitter logs throttle / sink-error events itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EmissionOutcome {
    /// Observation built, throttle admitted, sink accepted. If
    /// `pending_drops > 0`, an earlier burst was suppressed and the
    /// drop count is reported here so a future
    /// `degraded:observation-throttled` row (deferred) can carry it.
    Emitted { pending_drops: u32 },
    /// `from_midedit_response` returned `None` (no diagnostics) —
    /// the emitter stayed silent per the volume-control contract.
    SilentNoFinding,
    /// Rate window denied this emission. `drops` is the running
    /// total of suppressed observations since the previous `Allow`.
    Throttled { drops: u32 },
    /// Sink returned an error; the row was dropped and a
    /// `tracing::warn!` was logged. The scan response is unaffected.
    SinkError,
}

/// Default rate-window capacity for the daemon's mid-edit emitter.
/// Matches the MLP2-009 spec note that the same primitive caps
/// observation emit so a keystroke burst cannot flood Kindling.
/// Tuned conservatively — production hosts can override at startup.
pub const DEFAULT_MIDEDIT_EMIT_CAPACITY: usize = 32;

/// Default rate-window duration for the daemon's mid-edit emitter.
/// Pairs with [`DEFAULT_MIDEDIT_EMIT_CAPACITY`] for a 32-events-per-
/// 5-seconds cap.
pub const DEFAULT_MIDEDIT_EMIT_WINDOW: Duration = Duration::from_secs(5);

/// Daemon-side notification fan-out that converts a mid-edit scan
/// response into a Kindling `gate_evaluated` row, throttles via a
/// shared [`RateWindow`], and writes through the configured
/// [`KindlingObservationSink`].
///
/// The emitter owns the daemon-stable `session_id` (a UUID v4 minted
/// once per daemon process by the host). Per-call context is
/// supplied via [`MidEditEmissionRequest`]. The session-id field
/// here is a v1 placeholder for the per-edit session-id work
/// landing with MLP2-023's composite session keys; the wire shape
/// already matches the schema's `string().uuid()` contract.
pub struct MidEditObservationEmitter {
    sink: Arc<dyn KindlingObservationSink>,
    rate_window: RateWindow,
    daemon_session_id: String,
}

impl MidEditObservationEmitter {
    /// Construct an emitter with an explicit sink + rate window.
    /// Production callers wire a real sink + a custom cap; tests
    /// use [`Self::with_recorder`] for a recording sink.
    pub fn new(
        sink: Arc<dyn KindlingObservationSink>,
        rate_window: RateWindow,
        daemon_session_id: String,
    ) -> Self {
        Self {
            sink,
            rate_window,
            daemon_session_id,
        }
    }

    /// Construct a recording-sink emitter with the default rate
    /// window. Returns `(emitter, recorder)` so the test can hold a
    /// clone of the recorder to assert against. Reserved for tests.
    #[must_use]
    pub fn with_recorder(
        daemon_session_id: impl Into<String>,
    ) -> (Self, Arc<RecordingKindlingObservationSink>) {
        let recorder = Arc::new(RecordingKindlingObservationSink::new());
        let emitter = Self::new(
            Arc::clone(&recorder) as Arc<dyn KindlingObservationSink>,
            RateWindow::new(DEFAULT_MIDEDIT_EMIT_CAPACITY, DEFAULT_MIDEDIT_EMIT_WINDOW),
            daemon_session_id.into(),
        );
        (emitter, recorder)
    }

    /// Daemon-process-stable `session_id` the emitter stamps on every
    /// row. Hosts read this when they need to mirror it onto other
    /// daemon-side surfaces (logging, status, etc).
    #[must_use]
    pub fn daemon_session_id(&self) -> &str {
        &self.daemon_session_id
    }

    /// Build + emit a `gate_evaluated` row for a completed scan.
    ///
    /// Returns the [`EmissionOutcome`] so the caller can introspect
    /// what happened — tests assert on the variant; production
    /// callers can ignore the value (logging is internal).
    ///
    /// `now` is the wall-clock instant used to drive the rate
    /// window; production callers pass `Instant::now()` (kept as a
    /// parameter so tests can drive the window deterministically).
    pub fn try_emit(
        &self,
        request: &MidEditEmissionRequest<'_>,
        response: &ScanBufferResponse,
        now: Instant,
    ) -> EmissionOutcome {
        let ctx = ObservationContext {
            session_id: &self.daemon_session_id,
            timestamp: request.timestamp,
            gate_eval_id: request.gate_eval_id,
            file_path: request.file_path,
            duration_ms: request.duration_ms,
        };
        let Some(observation) = from_midedit_response(&ctx, response) else {
            return EmissionOutcome::SilentNoFinding;
        };
        match self.rate_window.record(now) {
            RateDecision::Throttle { drops } => {
                tracing::debug!(
                    target: "anvil_intercept::kindling_observation",
                    drops,
                    "gate_evaluated emit throttled by rate window"
                );
                EmissionOutcome::Throttled { drops }
            }
            RateDecision::Allow { pending_drops } => {
                if pending_drops > 0 {
                    tracing::warn!(
                        target: "anvil_intercept::kindling_observation",
                        pending_drops,
                        "gate_evaluated emit followed throttle burst",
                    );
                }
                match self.sink.try_emit(observation) {
                    Ok(()) => EmissionOutcome::Emitted { pending_drops },
                    Err(err) => {
                        tracing::warn!(
                            target: "anvil_intercept::kindling_observation",
                            error = %err,
                            "gate_evaluated emit dropped: sink failure",
                        );
                        EmissionOutcome::SinkError
                    }
                }
            }
        }
    }
}

impl std::fmt::Debug for MidEditObservationEmitter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MidEditObservationEmitter")
            .field("daemon_session_id", &self.daemon_session_id)
            .field("rate_window", &self.rate_window)
            .finish_non_exhaustive()
    }
}

// MLP2-010: post-hook `action_executed` builder + emitter ----------
//
// The post-commit / post-merge / post-rewrite hooks each fire as a
// short-lived `anvil hook` CLI invocation. Per the MLP-005 deferred
// outcome (and MLP2-010), every successful witness-line append in
// these hooks emits exactly one `action_executed` Kindling row —
// there is no volume-control short-circuit (unlike `gate_evaluated`,
// where pass-no-finding scans stay silent), and there is no rate
// window: a single git operation produces a single hook invocation.
//
// The wire shape matches `ActionExecutedObservationSchema` from
// `packages/kindling-integration/src/observation-contract.ts`. The
// post-hook surface populates the closed-set fields the schema
// requires (`session_id`, `timestamp`, `action_id`, `action_type`,
// `outcome`, `details.working_directory`, `duration_ms`) and the
// witness-line hash plus commit SHA travel inside the optional
// `details.command` field as a deterministic free-text token. Future
// schema extensions (e.g. a typed `details.witness_line_hash`) land
// here without disturbing the existing contract.

/// Pinned Kindling observation kind for post-hook bookkeeping. Schema-
/// matches `ActionExecutedObservationSchema.kind` in the TS contract.
pub const KIND_ACTION_EXECUTED: &str = "action_executed";

/// Pinned `action_type` for post-hook rows. The TS schema's
/// `action_type` enum is `command | tool_invocation | file_write |
/// file_delete | diff_apply`; post-hook witness-append work is a
/// command invoked by the user (`git commit` / `git merge` /
/// `git rebase --continue`), so `command` is the right bucket.
pub const POST_HOOK_ACTION_TYPE: &str = "command";

/// Closed-set vocabulary for the three post-hook surfaces that
/// produce `action_executed` rows. Stamped onto the row's
/// `details.command` field so consumers can filter without parsing
/// arbitrary free text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PostHookAction {
    PostCommit,
    PostMerge,
    PostRewrite,
}

impl PostHookAction {
    /// Stable wire-shape token. Matches the `validation_at` /
    /// `kind` strings already used by the witness-line surface so
    /// downstream joins stay deterministic.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::PostCommit => "post-commit",
            Self::PostMerge => "post-merge",
            Self::PostRewrite => "post-rewrite",
        }
    }
}

/// Closed-set outcome for an `action_executed` row. Matches the TS
/// schema's `outcome` enum (`success | failure | partial`). Post-
/// hook surfaces only ever emit on the success path today (the
/// witness-append failure paths take a separate render branch in the
/// hook module), but the variant is exposed so callers that decide
/// to emit on partial / failure paths in the future do not need to
/// reach into the type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ActionOutcome {
    Success,
    Failure,
    Partial,
}

/// Kindling `action_executed` observation payload. Serde JSON wire
/// shape matches `ActionExecutedObservationSchema` from the TS
/// contract: `snake_case` keys, kebab-case enum values, and the same
/// optional / required field policy. Optional fields the post-hook
/// surface does not populate (`environment_target`, `exit_code`,
/// `governed_by_*`, etc.) are omitted via `skip_serializing_if`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionExecutedObservation {
    pub kind: String,
    pub session_id: String,
    pub timestamp: String,
    pub action_id: String,
    pub action_type: String,
    pub details: ActionExecutedDetails,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub governed_by_gate_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub governed_by_plan_id: Option<String>,
    pub outcome: ActionOutcome,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
    pub duration_ms: u64,
}

/// Nested `details` object on an `action_executed` observation.
/// Matches `ActionExecutedObservationSchema.details` from the TS
/// contract. Post-hook rows populate `command` (the post-hook name
/// plus commit SHA plus witness-line hash, joined deterministically)
/// alongside `working_directory` (the repo root). Optional keys the
/// post-hook surface does not populate (`tool_name`, `file_paths`,
/// `diff_summary`, `environment_target`) are omitted entirely so the
/// wire shape matches what the TS validator expects.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionExecutedDetails {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_paths: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diff_summary: Option<ActionDiffSummary>,
    pub working_directory: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub environment_target: Option<String>,
}

/// Optional diff summary on an `action_executed` row. Reserved for
/// future emitters (post-hook surface does not populate it today).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionDiffSummary {
    pub additions: u32,
    pub deletions: u32,
    pub files_changed: u32,
}

/// Per-call inputs the hook surface supplies for each post-hook
/// emission. Holds borrowed strings so the hook can build one
/// inline without allocating extra owned `String`s.
#[derive(Debug, Clone, Copy)]
pub struct PostHookEmissionRequest<'a> {
    /// Which of the three post-hook surfaces produced this row.
    /// Stamped into the deterministic `details.command` token so a
    /// consumer can filter by surface without scanning free text.
    pub action: PostHookAction,
    /// Commit SHA the hook was invoked for (the rewritten / merged
    /// / committed SHA). Lower-hex; full 40 chars when available,
    /// shorter forms passed through verbatim.
    pub commit_sha: &'a str,
    /// SHA-256 of the witness line just appended. Pulled from
    /// `anvil_witness::compute_line_hash` at the call site so the
    /// `action_executed` row joins back to the chain entry it
    /// records.
    pub witness_line_hash: &'a str,
    /// Repo root the hook was invoked in. Becomes
    /// `details.working_directory` on the row.
    pub working_directory: &'a str,
    /// ISO 8601 datetime — when the hook observed the witness
    /// append completing.
    pub timestamp: &'a str,
    /// Wall-clock duration the hook spent on the action. Zero is
    /// acceptable for hooks that haven't started instrumenting
    /// yet; production hook wiring measures with `Instant::now()`.
    pub duration_ms: u64,
}

/// Build an `ActionExecutedObservation` for a successful post-hook
/// witness-append. Returns the row in the TS-contract shape so the
/// caller can hand it straight to a [`KindlingObservationSink`].
///
/// `daemon_session_id` is the daemon-process-stable session UUID
/// shared with the mid-edit emitter (see
/// [`MidEditObservationEmitter::daemon_session_id`]) — caller-
/// supplied so the hook surface can stay decoupled from the daemon
/// lifecycle.
///
/// `action_id` is the `commit_sha` prefixed with the action name
/// (`"post-commit:abcd1234..."`), giving consumers a deterministic
/// row id that survives a sink-side dedupe pass without needing a
/// separate UUID generator on the hook hot path.
#[must_use]
pub fn from_post_hook(
    daemon_session_id: &str,
    request: &PostHookEmissionRequest<'_>,
) -> ActionExecutedObservation {
    let action_label = request.action.as_str();
    let action_id = format!("{action_label}:{}", request.commit_sha);
    let command = format!(
        "anvil hook {action_label} (commit={}, witness_line_hash={})",
        request.commit_sha, request.witness_line_hash
    );
    ActionExecutedObservation {
        kind: KIND_ACTION_EXECUTED.to_string(),
        session_id: daemon_session_id.to_string(),
        timestamp: request.timestamp.to_string(),
        action_id,
        action_type: POST_HOOK_ACTION_TYPE.to_string(),
        details: ActionExecutedDetails {
            command: Some(command),
            tool_name: None,
            file_paths: None,
            diff_summary: None,
            working_directory: request.working_directory.to_string(),
            environment_target: None,
        },
        governed_by_gate_id: None,
        governed_by_plan_id: None,
        outcome: ActionOutcome::Success,
        exit_code: None,
        duration_ms: request.duration_ms,
    }
}

/// Outcome of [`PostHookEmitter::try_emit`]. Tests assert on the
/// variant; production callers can ignore the return because the
/// emitter logs sink failures internally.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActionEmissionOutcome {
    /// Sink accepted the row.
    Emitted,
    /// Sink returned an error; the row was dropped and a
    /// `tracing::warn!` was logged. The hook continues regardless.
    SinkError,
}

/// Hook-side emitter that builds an `ActionExecutedObservation` and
/// hands it to the configured sink. Distinct from
/// [`MidEditObservationEmitter`] because the post-hook surface has
/// different volume semantics (exactly one row per invocation, no
/// short-circuit, no rate window).
pub struct PostHookEmitter {
    sink: Arc<dyn KindlingObservationSink>,
    daemon_session_id: String,
}

impl PostHookEmitter {
    /// Construct an emitter with an explicit sink. Production
    /// callers wire a real sink; tests use [`Self::with_recorder`].
    #[must_use]
    pub fn new(sink: Arc<dyn KindlingObservationSink>, daemon_session_id: String) -> Self {
        Self {
            sink,
            daemon_session_id,
        }
    }

    /// Construct a noop emitter — useful when the hook needs an
    /// emitter handle but the host has not wired a real sink yet.
    /// The default the production CLI binary binds today.
    #[must_use]
    pub fn noop(daemon_session_id: impl Into<String>) -> Self {
        Self::new(
            Arc::new(NoopKindlingObservationSink) as Arc<dyn KindlingObservationSink>,
            daemon_session_id.into(),
        )
    }

    /// Construct a recording-sink emitter for tests. Returns
    /// `(emitter, recorder)` so the test can hold a clone of the
    /// recorder to assert against.
    #[must_use]
    pub fn with_recorder(
        daemon_session_id: impl Into<String>,
    ) -> (Self, Arc<RecordingKindlingObservationSink>) {
        let recorder = Arc::new(RecordingKindlingObservationSink::new());
        let emitter = Self::new(
            Arc::clone(&recorder) as Arc<dyn KindlingObservationSink>,
            daemon_session_id.into(),
        );
        (emitter, recorder)
    }

    /// Daemon-process-stable session id stamped onto every row.
    #[must_use]
    pub fn daemon_session_id(&self) -> &str {
        &self.daemon_session_id
    }

    /// Build + emit an `action_executed` row. Always builds (no
    /// volume-control short-circuit) and never returns an error —
    /// sink failures are logged internally so the caller's hook
    /// path stays uncoupled from sink health.
    pub fn try_emit(&self, request: &PostHookEmissionRequest<'_>) -> ActionEmissionOutcome {
        let observation = from_post_hook(&self.daemon_session_id, request);
        match self.sink.try_emit_action_executed(observation) {
            Ok(()) => ActionEmissionOutcome::Emitted,
            Err(err) => {
                tracing::warn!(
                    target: "anvil_intercept::kindling_observation",
                    action = request.action.as_str(),
                    commit_sha = request.commit_sha,
                    error = %err,
                    "action_executed emit dropped: sink failure",
                );
                ActionEmissionOutcome::SinkError
            }
        }
    }
}

impl std::fmt::Debug for PostHookEmitter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PostHookEmitter")
            .field("daemon_session_id", &self.daemon_session_id)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anvil_intercept_proto::protocol::DiagnosticEnvelope;
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

    /// USAGE-004: a JSON-RPC `params` object maps to one redacted
    /// `ArgShape` per top-level key, in sorted order, with no raw value
    /// retained and sensitive-named keys elided to the marker.
    #[test]
    fn arg_shapes_from_params_redacts_and_sorts() {
        let params = serde_json::json!({
            "query": "fn handle_jsonrpc_request",
            "token": "super-secret",
            "depth": 3,
            "include_tests": true,
        });
        let shapes = arg_shapes_from_params(&params);

        // Sorted by key: depth, include_tests, query, token.
        let names: Vec<&str> = shapes.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, ["depth", "include_tests", "query", "token"]);

        // Sensitive-named key elided to the marker; no raw values anywhere.
        let json = serde_json::to_string(&shapes).expect("serialise");
        assert!(json.contains("<redacted>"), "sensitive key not redacted: {json}");
        assert!(!json.contains("super-secret"), "raw secret leaked: {json}");
        assert!(
            !json.contains("fn handle_jsonrpc_request"),
            "raw query value leaked: {json}"
        );
    }

    /// USAGE-004: `null` params (and any non-object) yield no shapes —
    /// a notification with no params produces an empty arg list, not a
    /// panic.
    #[test]
    fn arg_shapes_from_params_handles_non_object() {
        assert!(arg_shapes_from_params(&serde_json::Value::Null).is_empty());
        assert!(arg_shapes_from_params(&serde_json::json!("scalar")).is_empty());
        assert!(arg_shapes_from_params(&serde_json::json!([1, 2, 3])).is_empty());
    }

    #[test]
    fn command_invoked_observation_round_trips_and_pins_kind() {
        use anvil_observability::redaction::redact_arg;

        let ctx = CommandInvocationContext {
            session_id: "22222222-2222-4222-8222-222222222222",
            timestamp: "2026-06-14T09:30:00Z",
            command: "check",
            principal: "deadbeef0123",
            traceparent: Some("00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01"),
        };
        let args = vec![
            redact_arg("path", Some("/home/me/repo")),
            redact_arg("json", None),
            redact_arg("token", Some("super-secret")),
        ];
        let obs = from_command_invocation(&ctx, args, Vec::new());

        assert_eq!(obs.kind, KIND_COMMAND_INVOKED);
        assert_eq!(obs.kind, "command.invoked");
        assert_eq!(obs.command, "check");
        assert_eq!(obs.principal, "deadbeef0123");
        // USAGE-001 emits an empty flag_set; USAGE-002 populates it.
        assert!(obs.flag_set.is_empty());

        let json = serde_json::to_string(&obs).expect("serialise");
        // Privacy contract: no raw argument values, ever.
        assert!(!json.contains("/home/me/repo"), "raw value leaked: {json}");
        assert!(
            !json.contains("super-secret"),
            "sensitive value leaked: {json}"
        );
        assert!(
            json.contains("<redacted>"),
            "redaction marker missing: {json}"
        );
        // flag_set is always present (never omitted), even when empty.
        assert!(
            json.contains("\"flag_set\":[]"),
            "flag_set must be present: {json}"
        );

        let back: CommandInvokedObservation = serde_json::from_str(&json).expect("round-trip");
        assert_eq!(back, obs);
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
            rules_sha: None,
            spoof_block: None,
        }
    }

    fn response_with(diagnostics: DiagnosticEnvelope) -> ScanBufferResponse {
        ScanBufferResponse {
            version: 1,
            diagnostics,
            truncated: false,
            rules_sha: None,
            spoof_block: None,
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

    // ----- MLP2-006: emitter integration -----

    fn sample_request() -> MidEditEmissionRequest<'static> {
        MidEditEmissionRequest {
            gate_eval_id: "gate-eval-emit-1",
            file_path: "src/lib.rs",
            timestamp: "2026-05-15T10:00:00Z",
            duration_ms: 17,
        }
    }

    const SESSION_UUID: &str = "11111111-1111-4111-8111-111111111111";

    #[test]
    fn emitter_short_circuits_silently_on_no_finding() {
        let (emitter, recorder) = MidEditObservationEmitter::with_recorder(SESSION_UUID);
        let outcome = emitter.try_emit(&sample_request(), &empty_response(), Instant::now());
        assert_eq!(outcome, EmissionOutcome::SilentNoFinding);
        assert!(
            recorder.is_empty(),
            "no-finding scans must not produce a row (volume control contract)",
        );
    }

    #[test]
    fn emitter_pushes_observation_to_sink_for_finding_bearing_scan() {
        let (emitter, sink) = MidEditObservationEmitter::with_recorder(SESSION_UUID);
        let resp = response_with(vec![make_diag("secrets-aws-key", Severity::Error)]);
        let outcome = emitter.try_emit(&sample_request(), &resp, Instant::now());
        assert_eq!(outcome, EmissionOutcome::Emitted { pending_drops: 0 });
        let recorded = sink.recorded();
        assert_eq!(recorded.len(), 1);
        let obs = &recorded[0];
        assert_eq!(obs.session_id, SESSION_UUID);
        assert_eq!(obs.gate_eval_id, "gate-eval-emit-1");
        assert_eq!(obs.gate_id, MIDEDIT_GATE_ID);
        assert_eq!(obs.kind, KIND_GATE_EVALUATED);
        assert_eq!(obs.enforcement, Enforcement::Blocking);
        assert_eq!(obs.duration_ms, 17);
        assert_eq!(obs.inputs.changed_files, vec!["src/lib.rs".to_string()]);
    }

    #[test]
    fn emitter_throttles_burst_above_capacity() {
        // Cap of 2 per very long window so timestamps cannot age out.
        let recorder = Arc::new(RecordingKindlingObservationSink::new());
        let emitter = MidEditObservationEmitter::new(
            Arc::clone(&recorder) as Arc<dyn KindlingObservationSink>,
            RateWindow::new(2, Duration::from_mins(1)),
            SESSION_UUID.to_string(),
        );
        let resp = response_with(vec![make_diag("warn-1", Severity::Warning)]);
        let now = Instant::now();
        // First two pass; the rest are throttled with cumulative drops.
        assert_eq!(
            emitter.try_emit(&sample_request(), &resp, now),
            EmissionOutcome::Emitted { pending_drops: 0 },
        );
        assert_eq!(
            emitter.try_emit(&sample_request(), &resp, now),
            EmissionOutcome::Emitted { pending_drops: 0 },
        );
        assert_eq!(
            emitter.try_emit(&sample_request(), &resp, now),
            EmissionOutcome::Throttled { drops: 1 },
        );
        assert_eq!(
            emitter.try_emit(&sample_request(), &resp, now),
            EmissionOutcome::Throttled { drops: 2 },
        );
        assert_eq!(
            recorder.len(),
            2,
            "throttled observations must not reach the sink",
        );
    }

    #[test]
    fn emitter_reports_pending_drops_on_first_allow_after_throttle() {
        let recorder = Arc::new(RecordingKindlingObservationSink::new());
        let emitter = MidEditObservationEmitter::new(
            Arc::clone(&recorder) as Arc<dyn KindlingObservationSink>,
            RateWindow::new(1, Duration::from_millis(50)),
            SESSION_UUID.to_string(),
        );
        let resp = response_with(vec![make_diag("err", Severity::Error)]);
        let t0 = Instant::now();
        assert_eq!(
            emitter.try_emit(&sample_request(), &resp, t0),
            EmissionOutcome::Emitted { pending_drops: 0 },
        );
        // Two drops while still inside the 50 ms window.
        assert_eq!(
            emitter.try_emit(&sample_request(), &resp, t0),
            EmissionOutcome::Throttled { drops: 1 },
        );
        assert_eq!(
            emitter.try_emit(&sample_request(), &resp, t0),
            EmissionOutcome::Throttled { drops: 2 },
        );
        // Past the window — next allow carries the cumulative count.
        let t1 = t0 + Duration::from_millis(100);
        assert_eq!(
            emitter.try_emit(&sample_request(), &resp, t1),
            EmissionOutcome::Emitted { pending_drops: 2 },
        );
        assert_eq!(recorder.len(), 2);
    }

    #[test]
    fn emitter_swallows_sink_failures_without_blocking() {
        let recorder = Arc::new(RecordingKindlingObservationSink::new());
        recorder.fail_next_with(KindlingSinkError::Unavailable("db locked".into()));
        let emitter = MidEditObservationEmitter::new(
            Arc::clone(&recorder) as Arc<dyn KindlingObservationSink>,
            RateWindow::new(8, Duration::from_secs(1)),
            SESSION_UUID.to_string(),
        );
        let resp = response_with(vec![make_diag("err", Severity::Error)]);
        // Sink fails on the first call — emitter must report SinkError
        // without panicking and without retaining state that would
        // cripple the next call.
        assert_eq!(
            emitter.try_emit(&sample_request(), &resp, Instant::now()),
            EmissionOutcome::SinkError,
        );
        assert!(recorder.is_empty(), "failed emit must not record the row");
        // Second call succeeds — sink is healthy again.
        let outcome = emitter.try_emit(&sample_request(), &resp, Instant::now());
        assert!(
            matches!(outcome, EmissionOutcome::Emitted { .. }),
            "post-failure emit should still flow, got {outcome:?}",
        );
        assert_eq!(recorder.len(), 1);
    }

    #[test]
    fn noop_sink_drops_observation_without_error() {
        let sink = NoopKindlingObservationSink;
        let resp = response_with(vec![make_diag("err", Severity::Error)]);
        let obs = from_midedit_response(&sample_ctx(), &resp).expect("obs");
        sink.try_emit(obs).expect("noop sink must always succeed");
    }

    #[test]
    fn emitter_exposes_daemon_session_id_for_host_introspection() {
        let (emitter, _) = MidEditObservationEmitter::with_recorder(SESSION_UUID);
        assert_eq!(emitter.daemon_session_id(), SESSION_UUID);
    }

    // ----- MLP2-010: post-hook action_executed emission -----

    fn sample_post_hook_request(action: PostHookAction) -> PostHookEmissionRequest<'static> {
        PostHookEmissionRequest {
            action,
            commit_sha: "abcdef0123456789abcdef0123456789abcdef01",
            witness_line_hash: "deadbeefcafef00ddeadbeefcafef00ddeadbeefcafef00ddeadbeefcafef00d",
            working_directory: "/home/dev/repo",
            timestamp: "2026-05-16T09:00:00Z",
            duration_ms: 5,
        }
    }

    #[test]
    fn from_post_hook_stamps_canonical_kind_and_action_type() {
        let req = sample_post_hook_request(PostHookAction::PostCommit);
        let obs = from_post_hook(SESSION_UUID, &req);
        assert_eq!(obs.kind, KIND_ACTION_EXECUTED);
        assert_eq!(obs.action_type, POST_HOOK_ACTION_TYPE);
        assert_eq!(obs.outcome, ActionOutcome::Success);
        assert_eq!(obs.session_id, SESSION_UUID);
        assert_eq!(obs.duration_ms, 5);
    }

    #[test]
    fn from_post_hook_action_id_combines_action_label_and_commit_sha() {
        for action in [
            PostHookAction::PostCommit,
            PostHookAction::PostMerge,
            PostHookAction::PostRewrite,
        ] {
            let req = sample_post_hook_request(action);
            let obs = from_post_hook(SESSION_UUID, &req);
            assert_eq!(
                obs.action_id,
                format!("{}:{}", action.as_str(), req.commit_sha),
            );
        }
    }

    #[test]
    fn from_post_hook_command_carries_commit_and_witness_line_hash() {
        let req = sample_post_hook_request(PostHookAction::PostMerge);
        let obs = from_post_hook(SESSION_UUID, &req);
        let command = obs.details.command.expect("command populated");
        assert!(
            command.contains("post-merge"),
            "command must name the post-hook surface: {command}"
        );
        assert!(
            command.contains(req.commit_sha),
            "command must carry the commit SHA: {command}"
        );
        assert!(
            command.contains(req.witness_line_hash),
            "command must carry the witness line hash: {command}"
        );
    }

    #[test]
    fn from_post_hook_records_working_directory_in_details() {
        let req = sample_post_hook_request(PostHookAction::PostRewrite);
        let obs = from_post_hook(SESSION_UUID, &req);
        assert_eq!(obs.details.working_directory, "/home/dev/repo");
        // Reserved-for-future fields stay None in v1.
        assert!(obs.details.tool_name.is_none());
        assert!(obs.details.file_paths.is_none());
        assert!(obs.details.diff_summary.is_none());
        assert!(obs.details.environment_target.is_none());
        assert!(obs.governed_by_gate_id.is_none());
        assert!(obs.governed_by_plan_id.is_none());
        assert!(obs.exit_code.is_none());
    }

    #[test]
    fn action_executed_serialises_with_expected_keys_and_omitted_optionals() {
        let req = sample_post_hook_request(PostHookAction::PostCommit);
        let obs = from_post_hook(SESSION_UUID, &req);
        let value: serde_json::Value = serde_json::to_value(&obs).expect("observation serialises");
        assert_eq!(value["kind"], KIND_ACTION_EXECUTED);
        assert_eq!(value["action_type"], POST_HOOK_ACTION_TYPE);
        assert_eq!(value["outcome"], "success");
        assert_eq!(value["details"]["working_directory"], "/home/dev/repo");
        // Optional fields with None must not appear on the wire.
        assert!(
            value.get("exit_code").is_none(),
            "exit_code must be omitted when None"
        );
        assert!(
            value.get("governed_by_gate_id").is_none(),
            "governed_by_gate_id must be omitted when None"
        );
        assert!(
            value["details"].get("tool_name").is_none(),
            "details.tool_name must be omitted when None"
        );
        assert!(
            value["details"].get("file_paths").is_none(),
            "details.file_paths must be omitted when None"
        );
    }

    #[test]
    fn post_hook_emitter_pushes_observation_to_sink() {
        let (emitter, sink) = PostHookEmitter::with_recorder(SESSION_UUID);
        let req = sample_post_hook_request(PostHookAction::PostCommit);
        let outcome = emitter.try_emit(&req);
        assert_eq!(outcome, ActionEmissionOutcome::Emitted);
        let recorded = sink.recorded_actions();
        assert_eq!(recorded.len(), 1);
        let row = &recorded[0];
        assert_eq!(row.session_id, SESSION_UUID);
        assert_eq!(row.action_id, format!("post-commit:{}", req.commit_sha));
        // gate_evaluated bucket stays empty — the kinds are routed
        // independently through the trait.
        assert!(sink.recorded().is_empty());
    }

    #[test]
    fn post_hook_emitter_emits_one_row_per_invocation_with_no_short_circuit() {
        // Unlike gate_evaluated, post-hook surfaces do NOT collapse
        // back-to-back invocations — each post-commit / post-merge /
        // post-rewrite produces exactly one row. Three invocations →
        // three rows.
        let (emitter, recorder) = PostHookEmitter::with_recorder(SESSION_UUID);
        for action in [
            PostHookAction::PostCommit,
            PostHookAction::PostMerge,
            PostHookAction::PostRewrite,
        ] {
            emitter.try_emit(&sample_post_hook_request(action));
        }
        assert_eq!(recorder.actions_len(), 3);
    }

    #[test]
    fn post_hook_emitter_swallows_sink_failures_without_propagating() {
        let recorder = Arc::new(RecordingKindlingObservationSink::new());
        recorder.fail_next_action_with(KindlingSinkError::Unavailable("ipc closed".into()));
        let emitter = PostHookEmitter::new(
            Arc::clone(&recorder) as Arc<dyn KindlingObservationSink>,
            SESSION_UUID.to_string(),
        );
        let req = sample_post_hook_request(PostHookAction::PostMerge);
        // First call: sink fails — emitter must report SinkError without panic.
        assert_eq!(emitter.try_emit(&req), ActionEmissionOutcome::SinkError);
        assert_eq!(recorder.actions_len(), 0, "failed emit must not record");
        // Second call: sink is healthy — emitter recovers, row reaches sink.
        assert_eq!(emitter.try_emit(&req), ActionEmissionOutcome::Emitted);
        assert_eq!(recorder.actions_len(), 1);
    }

    #[test]
    fn noop_sink_default_accepts_action_executed_via_trait_default() {
        // The default trait body is `Ok(())` — older sinks compile
        // against the extended trait without overriding, so
        // `NoopKindlingObservationSink` (which only impls
        // `try_emit`) auto-satisfies the new method via the default.
        let sink = NoopKindlingObservationSink;
        let req = sample_post_hook_request(PostHookAction::PostCommit);
        let obs = from_post_hook(SESSION_UUID, &req);
        sink.try_emit_action_executed(obs)
            .expect("noop default must succeed");
    }

    #[test]
    fn post_hook_action_as_str_matches_witness_line_validation_at_strings() {
        // Joins the row back to the existing witness-line surface
        // by reusing the same `validation_at` / `kind` tokens —
        // consumers can match without per-surface translation.
        assert_eq!(PostHookAction::PostCommit.as_str(), "post-commit");
        assert_eq!(PostHookAction::PostMerge.as_str(), "post-merge");
        assert_eq!(PostHookAction::PostRewrite.as_str(), "post-rewrite");
    }

    // ----- MLP2-054: audit-chain observation builder -------------------

    fn sample_audit_ctx() -> AuditChainContext<'static> {
        AuditChainContext {
            session_id: "22222222-2222-4222-8222-222222222222",
            timestamp: "2026-05-15T03:17:00Z",
            gate_eval_id: "gate-eval-audit-1",
            duration_ms: 123,
        }
    }

    fn clean_summary() -> AuditChainSummary<'static> {
        AuditChainSummary {
            commits_walked: 7,
            unwitnessed_count: 0,
            chain_intact: true,
            partial: false,
            degraded_audit_drift: false,
            chain_head_hash: Some("abcd1234"),
        }
    }

    #[test]
    fn audit_chain_clean_run_emits_pass_informational() {
        let obs = from_audit_chain(&sample_audit_ctx(), &clean_summary());
        assert_eq!(obs.kind, KIND_GATE_EVALUATED);
        assert_eq!(obs.gate_id, AUDIT_CHAIN_GATE_ID);
        assert_eq!(obs.outcome, Outcome::Pass);
        assert_eq!(obs.enforcement, Enforcement::Informational);
        assert!(
            obs.rules_violated.is_none(),
            "clean runs must omit rules_violated to match the Zod optional"
        );
        assert_eq!(obs.violation_count, Some(0));
    }

    #[test]
    fn audit_chain_drift_flips_outcome_and_lists_witness_rule() {
        let summary = AuditChainSummary {
            commits_walked: 12,
            unwitnessed_count: 5,
            chain_intact: true,
            degraded_audit_drift: true,
            ..clean_summary()
        };
        let obs = from_audit_chain(&sample_audit_ctx(), &summary);
        assert_eq!(obs.outcome, Outcome::Fail);
        assert_eq!(obs.enforcement, Enforcement::Blocking);
        let violated = obs.rules_violated.expect("rules_violated present on drift");
        assert_eq!(
            violated,
            vec![AUDIT_CHAIN_WITNESS_PRESENCE_RULE_ID.to_string()],
            "drift-only failures must flag the witness-presence rule"
        );
        assert_eq!(obs.violation_count, Some(5));
    }

    #[test]
    fn audit_chain_chain_broken_flags_chain_intact_rule() {
        // Tamper evidence: chain_intact=false must surface as its own
        // violated rule even when drift is zero.
        let summary = AuditChainSummary {
            commits_walked: 4,
            unwitnessed_count: 0,
            chain_intact: false,
            degraded_audit_drift: false,
            ..clean_summary()
        };
        let obs = from_audit_chain(&sample_audit_ctx(), &summary);
        assert_eq!(obs.outcome, Outcome::Fail);
        assert_eq!(obs.enforcement, Enforcement::Blocking);
        let violated = obs
            .rules_violated
            .expect("rules_violated present on tamper");
        assert_eq!(
            violated,
            vec![AUDIT_CHAIN_CHAIN_INTACT_RULE_ID.to_string()],
            "tamper-only failures must flag the chain-intact rule, not witness-presence"
        );
    }

    #[test]
    fn audit_chain_drift_and_tamper_list_both_rules_in_stable_order() {
        let summary = AuditChainSummary {
            commits_walked: 9,
            unwitnessed_count: 3,
            chain_intact: false,
            degraded_audit_drift: true,
            ..clean_summary()
        };
        let obs = from_audit_chain(&sample_audit_ctx(), &summary);
        let violated = obs.rules_violated.expect("rules_violated present");
        // Stable insertion order: witness-presence first, chain-intact
        // second. Locking this in so downstream Kindling consumers can
        // pattern-match on the array shape.
        assert_eq!(
            violated,
            vec![
                AUDIT_CHAIN_WITNESS_PRESENCE_RULE_ID.to_string(),
                AUDIT_CHAIN_CHAIN_INTACT_RULE_ID.to_string(),
            ]
        );
    }

    #[test]
    fn audit_chain_observation_populates_baseline_hash_from_chain_head() {
        let obs = from_audit_chain(&sample_audit_ctx(), &clean_summary());
        assert_eq!(
            obs.inputs.baseline_hash.as_deref(),
            Some("abcd1234"),
            "audit-chain rows must propagate chain_head_hash into inputs.baseline_hash"
        );
        assert!(
            obs.inputs.changed_files.is_empty(),
            "audit-chain rows scope by commits, not files; changed_files must be empty"
        );
        assert_eq!(
            obs.inputs.file_count, 7,
            "file_count mirrors commits_walked so the row records the audit window size"
        );
    }

    #[test]
    fn audit_chain_observation_omits_baseline_hash_when_chain_is_empty() {
        let summary = AuditChainSummary {
            chain_head_hash: None,
            ..clean_summary()
        };
        let obs = from_audit_chain(&sample_audit_ctx(), &summary);
        assert!(
            obs.inputs.baseline_hash.is_none(),
            "empty-chain runs must omit baseline_hash (matches the Zod optional)"
        );
    }

    #[test]
    fn audit_chain_evaluated_rules_pin_both_synthetic_ids() {
        // Both rules are always "evaluated" — the audit runs both
        // checks every time. Only rules_violated reflects which one
        // actually failed.
        let obs = from_audit_chain(&sample_audit_ctx(), &clean_summary());
        assert_eq!(
            obs.rules_evaluated,
            vec![
                AUDIT_CHAIN_WITNESS_PRESENCE_RULE_ID.to_string(),
                AUDIT_CHAIN_CHAIN_INTACT_RULE_ID.to_string(),
            ]
        );
    }

    #[test]
    fn audit_chain_observation_carries_context_identity_fields() {
        let ctx = sample_audit_ctx();
        let obs = from_audit_chain(&ctx, &clean_summary());
        assert_eq!(obs.session_id, ctx.session_id);
        assert_eq!(obs.timestamp, ctx.timestamp);
        assert_eq!(obs.gate_eval_id, ctx.gate_eval_id);
        assert_eq!(obs.duration_ms, ctx.duration_ms);
    }

    #[test]
    fn audit_chain_partial_walk_flips_outcome_to_fail_and_propagates_bit() {
        // MLP2-056 wiring contract: a partial walk surfaces as
        // outcome=Fail / enforcement=Blocking AND the dedicated
        // `partial` field is true on the wire. Both signals matter —
        // consumers that don't inspect `partial` still see a non-pass
        // outcome, so a runaway nightly cron can't quietly report
        // green.
        let summary = AuditChainSummary {
            partial: true,
            ..clean_summary()
        };
        let obs = from_audit_chain(&sample_audit_ctx(), &summary);
        assert_eq!(obs.outcome, Outcome::Fail);
        assert_eq!(obs.enforcement, Enforcement::Blocking);
        assert!(obs.partial, "partial bit must reach the wire struct");
        // A partial walk with no drift / tamper should NOT add a
        // rule to rules_violated — partial is its own dedicated
        // signal so consumers can distinguish "ran out of time" from
        // a substantive failure.
        assert!(
            obs.rules_violated.is_none(),
            "partial-only failures must not populate rules_violated"
        );
    }

    #[test]
    fn audit_chain_complete_walk_serialises_partial_as_absent() {
        // Forward-compat pin: when partial=false, the field must skip
        // from the JSON wire so pre-MLP2-056 consumers stay byte-
        // compat (matches the `#[serde(skip_serializing_if = ...)]`
        // contract on the struct).
        let obs = from_audit_chain(&sample_audit_ctx(), &clean_summary());
        assert!(!obs.partial);
        let value: serde_json::Value = serde_json::to_value(&obs).expect("serialises");
        assert!(
            value.get("partial").is_none(),
            "partial=false must skip from the wire to preserve byte-compat"
        );
    }

    #[test]
    fn audit_chain_observation_serialises_with_audit_chain_gate_id() {
        let obs = from_audit_chain(&sample_audit_ctx(), &clean_summary());
        let value: serde_json::Value = serde_json::to_value(&obs).expect("audit obs serialises");
        assert_eq!(value["kind"], KIND_GATE_EVALUATED);
        assert_eq!(value["gate_id"], AUDIT_CHAIN_GATE_ID);
        assert_eq!(value["enforcement"], "informational");
        assert_eq!(value["outcome"], "pass");
        // warning_count is None for audit-chain rows; serde must skip
        // it rather than emit `null`.
        assert!(
            value.get("warning_count").is_none(),
            "audit-chain rows must omit warning_count (Some(0) would be misleading)"
        );
    }
}
