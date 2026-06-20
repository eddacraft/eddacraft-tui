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

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{SyncSender, TrySendError, sync_channel};
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

/// DPO-001: pinned `gate_id` for save-time `validate_paths` verdicts.
/// Distinguishes L2 save-time rows from mid-edit / pre-commit /
/// pre-push / audit rows that share the `gate_evaluated` kind but live
/// at different layers. Unlike the mid-edit path, save-time emits on
/// **both** pass and fail (the producer-coverage gap DPO-001 closes),
/// so a clean save is a queryable `Pass` row, not silence.
pub const SAVE_TIME_GATE_ID: &str = "save-time";

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

/// USAGE-004: fixed placeholder recorded for a nested object/array
/// JSON-RPC param by [`arg_shapes_from_params`]. Using a constant (rather
/// than the serialised value) keeps the coarse length bucket independent
/// of the nested structure's size, so neither nested values nor nested
/// size leak.
const NESTED_PARAM_MARKER: &str = "<nested>";

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
/// are stringified compactly; a nested array/object records a fixed
/// [`NESTED_PARAM_MARKER`] (never its serialised content), so neither the
/// nested values nor the nested structure's *size* leak. A non-object
/// `params` (or `Null`) yields no shapes.
///
/// A sensitive-named nested key (e.g. `params.opts.token`) is not
/// individually marker-redacted — but its value is never captured
/// either, so nothing sensitive leaks; only the top-level key's presence
/// is recorded.
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
            // Nested object/array: feed a FIXED marker, never the
            // serialised content. Measuring the serialised form would let
            // the coarse length bucket leak the nested structure's size
            // (a daemon-only signal the flat CLI argv path cannot produce)
            // and a sensitive key nested one level deep would not be
            // name-redacted. The fixed marker keeps the length bucket
            // constant regardless of nested size; the value is never
            // stored either way. (Council: nested-param redaction.)
            Value::Object(_) | Value::Array(_) => redact_arg(key, Some(NESTED_PARAM_MARKER)),
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

/// DPO-001: convert a save-time `validate_paths` verdict into a
/// Kindling `gate_evaluated` observation. Unlike [`from_midedit_response`]
/// this **always** builds a row — a clean save (empty diagnostics)
/// produces an `Outcome::Pass` row, closing the save-time producer-
/// coverage gap (the mid-edit path stays silent on a pass; save-time
/// does not, because a passing save is the signal operators want to
/// query for coverage).
///
/// The caller supplies the per-evaluation [`ObservationContext`]
/// (session id, timestamp, etc) and the multi-path verdict inputs, so
/// this helper stays a pure converter — testable without a clock or
/// UUID source. `ctx.file_path` is unused on this path (save-time is
/// multi-path); the changed-file set comes from `paths`.
///
/// `include_paths` gates whether the validated paths populate
/// `inputs.changed_files` (paths-only, no content). When `false` the
/// set is empty and only the `file_count` is recorded — the host's
/// privacy posture decides which.
#[must_use]
pub fn from_validate_paths(
    ctx: &ObservationContext<'_>,
    diagnostics: &[Diagnostic],
    // `file_count` is passed separately from `paths` so the caller can skip
    // cloning the path strings on the verdict hot path when paths are not
    // opted in (the default) while still recording the true count.
    file_count: usize,
    paths: &[String],
    include_paths: bool,
) -> GateEvaluatedObservation {
    let outcome = if diagnostics.is_empty() {
        Outcome::Pass
    } else {
        Outcome::Fail
    };
    // `enforcement_for` returns `Informational` on an empty batch and
    // `counts_for` returns `(0, 0)`, so a pass row carries the right
    // zeroed counts without a special case.
    let enforcement = enforcement_for(diagnostics);
    let (violation_count, warning_count) = counts_for(diagnostics);
    let rules_evaluated: Vec<String> = diagnostics
        .iter()
        .map(|d| d.source.rule_id.clone())
        .collect();
    let rules_violated: Vec<String> = diagnostics
        .iter()
        .filter(|d| matches!(d.severity, Severity::Error | Severity::Warning))
        .map(|d| d.source.rule_id.clone())
        .collect();

    GateEvaluatedObservation {
        kind: KIND_GATE_EVALUATED.to_string(),
        session_id: ctx.session_id.to_string(),
        timestamp: ctx.timestamp.to_string(),
        gate_eval_id: ctx.gate_eval_id.to_string(),
        gate_id: SAVE_TIME_GATE_ID.to_string(),
        inputs: ObservationInputs {
            file_count: u32::try_from(file_count).unwrap_or(u32::MAX),
            changed_files: if include_paths {
                paths.to_vec()
            } else {
                Vec::new()
            },
            baseline_hash: None,
        },
        outcome,
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
        // Save-time verdicts run the full path set to completion; the
        // partial-walk flag is an audit-chain concern only.
        partial: false,
    }
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

// DPO-002: fence-engage `constraint_applied` builder --------------
//
// Every successful fence engage is a constraint the daemon applied to
// a worktree. Pre-DPO-002 only the *rate-limited cascade transition*
// surfaced as telemetry; an ordinary engage produced no observation,
// leaving a producer-coverage gap (the council fix). This kind makes
// every engage a queryable row, distinct from `gate_evaluated` so a
// consumer can filter constraints without parsing gate rows.
//
// The row is BEST-EFFORT, not audit-grade (council D): it travels
// through the non-blocking sink boundary and can be dropped on a full
// channel. The authoritative record of which worktrees are fenced is
// the persistent fence-state file, NOT this observation stream.

/// DPO-002: pinned Kindling observation kind for a daemon-applied
/// constraint (a fence engage). Distinct from [`KIND_GATE_EVALUATED`]
/// so consumers query constraints as their own stream.
pub const KIND_CONSTRAINT_APPLIED: &str = "constraint_applied";

/// DPO-002: pinned `gate_id` (and `constraint_id`) for fence-engage
/// rows. Identifies the daemon fence surface as the constraint source.
pub const FENCE_GATE_ID: &str = "daemon.fence";

/// DPO-002: bounded fallback token for any fence reason that is not a
/// known control-lane constant. Operator / rule free-text is never
/// echoed verbatim onto an observation (it could carry path fragments
/// or operator notes); it collapses to this opaque token.
pub const FENCE_REASON_OPERATOR: &str = "operator";

/// DPO-002: normalise a fence `reason` to a bounded, non-leaking token.
///
/// The two known control-lane reasons (cascade engage and spoofed-
/// attribution) are pinned constants and pass through unchanged so a
/// consumer can filter on them. Anything else — an operator-supplied
/// or rule-supplied free-text reason — collapses to
/// [`FENCE_REASON_OPERATOR`] so no free-form text reaches the
/// observation timeline.
#[must_use]
pub fn normalise_fence_reason(reason: &str) -> String {
    // Known control-lane pass-through constants:
    //   - `crate::telemetry::DEGRADED_FENCE_CASCADE`
    //   - `crate::telemetry::DEGRADED_SPOOFED_ATTRIBUTION`
    // This allow-list is exhaustive BY CONVENTION, not enforced by the
    // compiler: adding a new control-lane reason constant that should
    // survive onto the observation timeline REQUIRES adding it here too,
    // otherwise it collapses to `FENCE_REASON_OPERATOR`.
    if reason == crate::telemetry::DEGRADED_FENCE_CASCADE
        || reason == crate::telemetry::DEGRADED_SPOOFED_ATTRIBUTION
    {
        reason.to_string()
    } else {
        FENCE_REASON_OPERATOR.to_string()
    }
}

/// DPO-002: Kindling `constraint_applied` observation payload. Serde
/// JSON wire shape is `snake_case`; emitted once per successful fence
/// engage.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConstraintAppliedObservation {
    pub kind: String,
    pub session_id: String,
    pub timestamp: String,
    /// Stable constraint identifier ([`FENCE_GATE_ID`]); the join key a
    /// consumer groups fence constraints by.
    pub constraint_id: String,
    /// The daemon surface that applied the constraint ([`FENCE_GATE_ID`]).
    pub gate_id: String,
    /// Canonical worktree path the fence was applied to.
    pub worktree: String,
    /// Normalised reason (see [`normalise_fence_reason`]) — never raw
    /// operator free-text.
    pub reason: String,
    /// `true` when this engage also engaged the rate-limited cascade
    /// (5 fences in 60 s); `false` for an ordinary engage.
    pub cascade: bool,
}

/// DPO-002: build a [`ConstraintAppliedObservation`] for a fence
/// engage. Pure converter — the caller owns identity (daemon session
/// id) and the clock (`timestamp`). The `reason` is normalised here so
/// no call site can bypass the bounded-token policy.
///
/// Emitting before the fence-file persist gives best-effort *ordering*
/// (the row tends to precede the persisted state), but the row itself is
/// BEST-EFFORT (council D): it crosses the non-blocking sink boundary and
/// can be dropped on a full channel. Do not treat it as a guaranteed
/// record — the persistent fence-state file is the authoritative source
/// of which worktrees are fenced.
///
/// `include_paths` gates the absolute worktree path (council C): the same
/// `ANVIL_OBSERVATION_INCLUDE_PATHS` posture that suppresses paths on
/// save-time `gate_evaluated` rows applies here. When `false` the
/// `worktree` field is set to [`REDACTED_WORKTREE`] (kept present for
/// schema stability) rather than the real absolute path.
#[must_use]
pub fn from_fence(
    daemon_session_id: &str,
    timestamp: &str,
    worktree: &str,
    reason: &str,
    cascade: bool,
    include_paths: bool,
) -> ConstraintAppliedObservation {
    ConstraintAppliedObservation {
        kind: KIND_CONSTRAINT_APPLIED.to_string(),
        session_id: daemon_session_id.to_string(),
        timestamp: timestamp.to_string(),
        constraint_id: FENCE_GATE_ID.to_string(),
        gate_id: FENCE_GATE_ID.to_string(),
        worktree: if include_paths {
            worktree.to_string()
        } else {
            REDACTED_WORKTREE.to_string()
        },
        reason: normalise_fence_reason(reason),
        cascade,
    }
}

/// DPO-002 (council C): placeholder recorded in the `worktree` field of a
/// `constraint_applied` row when the path-include gate
/// (`ANVIL_OBSERVATION_INCLUDE_PATHS`) is off. The field stays present for
/// wire-schema stability; only its value is suppressed.
pub const REDACTED_WORKTREE: &str = "<redacted>";

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
///
/// `Debug` is a supertrait so a sink can live inside a `#[derive(Debug)]`
/// struct held behind `Arc<dyn KindlingObservationSink>` (e.g.
/// `ForegroundOpts`'s injected sink). `Debug` is REQUIRED of every
/// implementor (not merely a convenience existing sinks happen to
/// satisfy); a new sink MUST derive or implement it. Keep `Debug` cheap
/// and content-free — no row contents should be formatted.
pub trait KindlingObservationSink: std::fmt::Debug + Send + Sync {
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

    /// USAGE-004: deliver a `command.invoked` row produced by the
    /// JSON-RPC dispatch surface for a user-initiated method call.
    /// Defaulted to `Ok(())` for the same reason as
    /// [`Self::try_emit_action_executed`] — existing sinks auto-satisfy
    /// the extended trait; only sinks that consume usage rows override.
    fn try_emit_command_invoked(
        &self,
        _observation: CommandInvokedObservation,
    ) -> Result<(), KindlingSinkError> {
        Ok(())
    }

    /// DPO-002: deliver a `constraint_applied` row produced by the
    /// fence-engage surface. Defaulted to `Ok(())` for the same reason
    /// as the other extension methods — existing sinks (Noop, custom
    /// impls) auto-satisfy the extended trait; only sinks that consume
    /// constraint rows override.
    fn try_emit_constraint_applied(
        &self,
        _observation: ConstraintAppliedObservation,
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
    commands: Mutex<Vec<CommandInvokedObservation>>,
    constraints: Mutex<Vec<ConstraintAppliedObservation>>,
    fail_next: Mutex<Option<KindlingSinkError>>,
    fail_next_action: Mutex<Option<KindlingSinkError>>,
    fail_next_command: Mutex<Option<KindlingSinkError>>,
    fail_next_constraint: Mutex<Option<KindlingSinkError>>,
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

    /// USAGE-004: inject a one-shot failure for the next
    /// [`Self::try_emit_command_invoked`] call so tests can exercise the
    /// dispatch-side sink-error swallow path.
    pub fn fail_next_command_with(&self, error: KindlingSinkError) {
        *self
            .fail_next_command
            .lock()
            .expect("fail_next_command mutex") = Some(error);
    }

    /// USAGE-004: snapshot the recorded `command.invoked` observations in
    /// arrival order.
    #[must_use]
    pub fn recorded_command_invocations(&self) -> Vec<CommandInvokedObservation> {
        self.commands.lock().expect("commands mutex").clone()
    }

    /// USAGE-004: number of `command.invoked` observations recorded.
    #[must_use]
    pub fn commands_len(&self) -> usize {
        self.commands.lock().expect("commands mutex").len()
    }

    /// DPO-002: inject a one-shot failure for the next
    /// [`Self::try_emit_constraint_applied`] call so tests can exercise
    /// the fence-side sink-error swallow path.
    pub fn fail_next_constraint_with(&self, error: KindlingSinkError) {
        *self
            .fail_next_constraint
            .lock()
            .expect("fail_next_constraint mutex") = Some(error);
    }

    /// DPO-002: snapshot the recorded `constraint_applied` observations
    /// in arrival order.
    #[must_use]
    pub fn recorded_constraints(&self) -> Vec<ConstraintAppliedObservation> {
        self.constraints.lock().expect("constraints mutex").clone()
    }

    /// DPO-002: number of `constraint_applied` observations recorded.
    #[must_use]
    pub fn constraints_len(&self) -> usize {
        self.constraints.lock().expect("constraints mutex").len()
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

    /// True when no observations of any kind have been recorded.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
            && self.actions_len() == 0
            && self.commands_len() == 0
            && self.constraints_len() == 0
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

    fn try_emit_command_invoked(
        &self,
        observation: CommandInvokedObservation,
    ) -> Result<(), KindlingSinkError> {
        if let Some(err) = self
            .fail_next_command
            .lock()
            .expect("fail_next_command mutex")
            .take()
        {
            return Err(err);
        }
        self.commands
            .lock()
            .expect("commands mutex")
            .push(observation);
        Ok(())
    }

    fn try_emit_constraint_applied(
        &self,
        observation: ConstraintAppliedObservation,
    ) -> Result<(), KindlingSinkError> {
        if let Some(err) = self
            .fail_next_constraint
            .lock()
            .expect("fail_next_constraint mutex")
            .take()
        {
            return Err(err);
        }
        self.constraints
            .lock()
            .expect("constraints mutex")
            .push(observation);
        Ok(())
    }
}

// DPO-001 (council T2: producer-hot-path boundary) -------------------
//
// The save-time / fence producers run on the daemon's verdict + engage
// paths. A real Kindling sink (the NDJSON sidecar, or a future IPC
// frame) does blocking IO — a slow disk or a contended lock would, with
// a direct sink, back-pressure the verdict response, coupling the hot
// path to sink health. ADR-031 forbids that coupling.
//
// [`NonBlockingObservationSink`] is the decorator that severs it: a
// single background drain thread owns the inner sink, and every producer
// call only does a non-blocking `try_send` onto a bounded channel. When
// the channel is full (the drain is behind) the row is dropped and a
// counter incremented — the producer NEVER blocks and NEVER errors. This
// is the ADR-031 acceptance boundary for DPO-001.

/// DPO-001: a tagged union of every observation kind the daemon
/// produces, so a single bounded channel can carry all of them to the
/// shared drain thread. One variant per `try_emit*` method on
/// [`KindlingObservationSink`].
#[derive(Debug, Clone)]
pub enum ObservationEnvelope {
    /// A `gate_evaluated` row (mid-edit / save-time / audit-chain).
    Gate(GateEvaluatedObservation),
    /// A `command.invoked` row (CLI / JSON-RPC dispatch).
    Command(CommandInvokedObservation),
    /// An `action_executed` row (post-hook surface).
    Action(ActionExecutedObservation),
    /// A `constraint_applied` row (fence engage).
    Constraint(ConstraintAppliedObservation),
}

/// DPO-001: a [`KindlingObservationSink`] decorator that guarantees the
/// producer hot path cannot be back-pressured by a slow or blocking
/// inner sink (council T2 boundary; the ADR-031 acceptance for DPO-001).
///
/// Construction spawns ONE background thread that owns `inner` and drains
/// a bounded [`sync_channel`]. Each `try_emit*` wraps the row in an
/// [`ObservationEnvelope`] and `try_send`s it: on success the drain
/// thread forwards it to the matching `inner.try_emit*` (any error is
/// logged via `tracing::warn!` and swallowed). On a full or disconnected
/// channel the row is dropped, the [`Self::dropped_count`] counter is
/// incremented, and `Ok(())` is returned — the producer never blocks and
/// never sees an error, regardless of inner-sink health.
///
/// No networking and no async runtime: a plain `std::thread` plus an
/// `mpsc` channel, so ADR-064 (the daemon never grows a transport
/// dependency on the hot path) stays clean.
pub struct NonBlockingObservationSink {
    /// `Option` so `Drop` can take (and thereby close) the sender BEFORE
    /// joining the drain thread — closing it is what makes the drain's
    /// `recv()` return `Err` and the loop exit.
    tx: Option<SyncSender<ObservationEnvelope>>,
    dropped: Arc<AtomicU64>,
    /// Join handle for the drain thread; taken on `Drop` so the sink can
    /// join after the sender is closed. `Option` so `Drop` can move it out.
    drain: Option<std::thread::JoinHandle<()>>,
}

impl NonBlockingObservationSink {
    /// Wrap `inner` behind a bounded channel of `capacity` envelopes and
    /// spawn the single drain thread. `capacity` bounds how many rows can
    /// queue while the drain is behind a slow inner sink; beyond it, rows
    /// are dropped (counted) rather than blocking the producer.
    ///
    /// Returns `None` when the drain thread cannot be spawned (e.g. the
    /// host is out of thread handles): a `tracing::warn!` is logged and the
    /// caller is expected to degrade to no observation export rather than
    /// crash the daemon at startup. The old `expect`-on-spawn was a
    /// daemon-crash-at-startup hazard (council G).
    #[must_use]
    pub fn new(inner: Arc<dyn KindlingObservationSink>, capacity: usize) -> Option<Self> {
        let (tx, rx) = sync_channel::<ObservationEnvelope>(capacity);
        let dropped = Arc::new(AtomicU64::new(0));
        let drain = match std::thread::Builder::new()
            .name("anvil-observation-drain".to_owned())
            .spawn(move || {
                // Exits when every sender is dropped (the channel
                // disconnects) — see `Drop`.
                while let Ok(envelope) = rx.recv() {
                    let result = match envelope {
                        ObservationEnvelope::Gate(obs) => inner.try_emit(obs),
                        ObservationEnvelope::Command(obs) => inner.try_emit_command_invoked(obs),
                        ObservationEnvelope::Action(obs) => inner.try_emit_action_executed(obs),
                        ObservationEnvelope::Constraint(obs) => {
                            inner.try_emit_constraint_applied(obs)
                        }
                    };
                    if let Err(err) = result {
                        tracing::warn!(
                            target: "anvil_intercept::kindling_observation",
                            error = %err,
                            "non-blocking observation drain: inner sink rejected a row",
                        );
                    }
                }
            }) {
            Ok(handle) => handle,
            Err(err) => {
                tracing::warn!(
                    target: "anvil_intercept::kindling_observation",
                    error = %err,
                    "could not spawn observation drain thread; observation export disabled",
                );
                return None;
            }
        };
        Some(Self {
            tx: Some(tx),
            dropped,
            drain: Some(drain),
        })
    }

    /// Number of rows dropped because the channel was full or the drain
    /// thread had exited. Tests assert on this to prove the drop-on-full
    /// contract; production may surface it as a degraded-counter signal.
    #[must_use]
    pub fn dropped_count(&self) -> u64 {
        self.dropped.load(Ordering::Relaxed)
    }

    /// Enqueue `envelope`, or count a drop. Never blocks, never errors —
    /// the shared invariant behind every `try_emit*` override below.
    ///
    /// A drop (channel full or disconnected) is RATE-LIMITED-warned (council
    /// E + D): the daemon hot path stays silent on the success path, but a
    /// saturating drain must not be invisible — silent saturation would let
    /// a stuck sidecar quietly shed audit rows. We warn on the first drop
    /// and then every 1024th drop thereafter, keyed off the running counter
    /// so a flood produces a handful of log lines, not one per dropped row.
    fn enqueue(&self, envelope: ObservationEnvelope) {
        // `tx` is `Some` for the whole live span of the sink; it is only
        // taken in `Drop`, after which no `try_emit*` can run. A defensive
        // `None` (impossible in practice) counts a drop, never panics.
        let Some(tx) = self.tx.as_ref() else {
            self.record_drop();
            return;
        };
        if let Err(TrySendError::Full(_) | TrySendError::Disconnected(_)) = tx.try_send(envelope) {
            self.record_drop();
        }
    }

    /// Increment the dropped counter and emit a rate-limited warn so a
    /// saturating drain is visible in the logs without flooding them. Warns
    /// on the 1st drop (count transitions 0 → 1) and every 1024th after.
    fn record_drop(&self) {
        let prior = self.dropped.fetch_add(1, Ordering::Relaxed);
        let total = prior + 1;
        if total == 1 || total.is_multiple_of(1024) {
            tracing::warn!(
                target: "anvil_intercept::kindling_observation",
                dropped_total = total,
                "non-blocking observation sink dropped a row (channel full or drain gone); \
                 observation rows are best-effort and the persistent record lives elsewhere",
            );
        }
    }
}

impl KindlingObservationSink for NonBlockingObservationSink {
    fn try_emit(&self, observation: GateEvaluatedObservation) -> Result<(), KindlingSinkError> {
        self.enqueue(ObservationEnvelope::Gate(observation));
        Ok(())
    }

    fn try_emit_command_invoked(
        &self,
        observation: CommandInvokedObservation,
    ) -> Result<(), KindlingSinkError> {
        self.enqueue(ObservationEnvelope::Command(observation));
        Ok(())
    }

    fn try_emit_action_executed(
        &self,
        observation: ActionExecutedObservation,
    ) -> Result<(), KindlingSinkError> {
        self.enqueue(ObservationEnvelope::Action(observation));
        Ok(())
    }

    fn try_emit_constraint_applied(
        &self,
        observation: ConstraintAppliedObservation,
    ) -> Result<(), KindlingSinkError> {
        self.enqueue(ObservationEnvelope::Constraint(observation));
        Ok(())
    }
}

impl std::fmt::Debug for NonBlockingObservationSink {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NonBlockingObservationSink")
            .field("dropped", &self.dropped_count())
            .finish_non_exhaustive()
    }
}

impl Drop for NonBlockingObservationSink {
    fn drop(&mut self) {
        // Graceful shutdown: close the sender FIRST so the channel
        // disconnects and the drain's `recv()` returns `Err`, ending its
        // loop. Taking `tx` here (the struct holds the only sender) drops
        // it now, before the join below — without this ordering a join
        // would deadlock because the field-drop that closes `tx` runs
        // only after this body returns.
        drop(self.tx.take());
        // Join the drain so any rows already accepted into the channel are
        // forwarded to the inner sink before we return — but with a BOUNDED
        // deadline (council F). An unconditional `join()` would hang the
        // whole shutdown if the inner sink is wedged on a stalled disk; we
        // poll `is_finished()` up to ~2s (matching the tone of the 1s
        // listener-join timeout in `run_foreground`) and then detach. A
        // poisoned / panicked drain's join error is swallowed rather than
        // double-panicking during unwind.
        if let Some(handle) = self.drain.take() {
            let deadline = Instant::now() + Duration::from_secs(2);
            loop {
                if handle.is_finished() {
                    let _ = handle.join();
                    break;
                }
                if Instant::now() >= deadline {
                    // Detach: drop the handle without joining. The drain
                    // thread keeps running (it cannot be force-killed
                    // safely) but shutdown is no longer blocked on it.
                    tracing::warn!(
                        target: "anvil_intercept::kindling_observation",
                        "observation drain did not exit within 2s of shutdown; \
                         detaching (likely a stalled sink/disk)",
                    );
                    break;
                }
                std::thread::sleep(Duration::from_millis(10));
            }
        }
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

// DPO-001: save-time `gate_evaluated` emitter ----------------------
//
// The save-time `validate_paths` verb produces a verdict on every
// editor save. The mid-edit emitter stays silent on a pass (volume
// control); save-time does NOT — a passing save is the coverage signal
// operators query, so it must produce a row. But an unbounded pass
// stream would still flood the sink on a save-heavy workspace, so the
// volume policy is asymmetric:
//
// - A **fail** (non-empty diagnostics) is ALWAYS emitted, never rate-
//   checked. A finding is audit-grade and must not be dropped.
// - A **pass** (empty diagnostics) is admitted only when the pass
//   rate window allows it; throttled passes are dropped (the next save
//   produces a fresh pass row anyway).

/// DPO-001: per-call inputs the IPC handler supplies for each save-time
/// emission. Unlike [`MidEditEmissionRequest`] there is no `file_path` —
/// save-time is multi-path and the path set travels separately.
#[derive(Debug, Clone, Copy)]
pub struct SaveTimeEmissionRequest<'a> {
    /// Unique evaluation id for joining the row back to the
    /// originating telemetry span. Callers derive this from the W3C
    /// `traceparent` (the same `derive_gate_eval_id` the mid-edit path
    /// uses).
    pub gate_eval_id: &'a str,
    /// ISO 8601 datetime — when the daemon observed this verdict
    /// completing.
    pub timestamp: &'a str,
    /// Wall-clock duration the daemon spent on the underlying
    /// `validate_paths` call.
    pub duration_ms: u64,
}

/// DPO-001: default rate-window capacity for the save-time emitter's
/// **pass** stream. Fails bypass the window entirely; this only caps
/// how many clean-save rows reach the sink per window.
pub const DEFAULT_SAVE_TIME_PASS_CAPACITY: usize = 20;

/// DPO-001: default rate-window duration paired with
/// [`DEFAULT_SAVE_TIME_PASS_CAPACITY`] — 20 pass rows per minute. Wider
/// than the mid-edit window because save-time fires far less often than
/// a keystroke storm.
pub const DEFAULT_SAVE_TIME_PASS_WINDOW: Duration = Duration::from_mins(1);

/// DPO-001: daemon-side fan-out that converts a save-time
/// `validate_paths` verdict into a Kindling `gate_evaluated` row,
/// throttles the **pass** stream via a [`RateWindow`], always lets
/// **fail** rows through, and writes through the configured
/// [`KindlingObservationSink`].
///
/// Mirrors [`MidEditObservationEmitter`]'s ownership model: it holds
/// the daemon-stable `session_id` and a sink behind an `Arc<dyn …>`.
/// `include_paths` is fixed at construction (the host's privacy
/// posture) and passed through to [`from_validate_paths`].
pub struct SaveTimeObservationEmitter {
    sink: Arc<dyn KindlingObservationSink>,
    pass_rate_window: RateWindow,
    daemon_session_id: String,
    include_paths: bool,
}

impl SaveTimeObservationEmitter {
    /// Construct an emitter with an explicit sink + pass rate window.
    /// Production callers wire a real sink + a custom cap; tests use
    /// [`Self::with_recorder`] for a recording sink.
    pub fn new(
        sink: Arc<dyn KindlingObservationSink>,
        pass_rate_window: RateWindow,
        daemon_session_id: String,
        include_paths: bool,
    ) -> Self {
        Self {
            sink,
            pass_rate_window,
            daemon_session_id,
            include_paths,
        }
    }

    /// Construct a recording-sink emitter with the default pass rate
    /// window. Returns `(emitter, recorder)` so the test can hold a
    /// clone of the recorder to assert against. Reserved for tests.
    #[must_use]
    pub fn with_recorder(
        daemon_session_id: impl Into<String>,
        include_paths: bool,
    ) -> (Self, Arc<RecordingKindlingObservationSink>) {
        let recorder = Arc::new(RecordingKindlingObservationSink::new());
        let emitter = Self::new(
            Arc::clone(&recorder) as Arc<dyn KindlingObservationSink>,
            RateWindow::new(
                DEFAULT_SAVE_TIME_PASS_CAPACITY,
                DEFAULT_SAVE_TIME_PASS_WINDOW,
            ),
            daemon_session_id.into(),
            include_paths,
        );
        (emitter, recorder)
    }

    /// Daemon-process-stable `session_id` the emitter stamps on every
    /// row.
    #[must_use]
    pub fn daemon_session_id(&self) -> &str {
        &self.daemon_session_id
    }

    /// Whether this emitter records file paths in the row. The IPC caller
    /// reads this to skip cloning the path strings on the verdict hot path
    /// when paths are not opted in (the default privacy posture).
    #[must_use]
    pub fn include_paths(&self) -> bool {
        self.include_paths
    }

    /// Build + emit a `gate_evaluated` row for a completed save-time
    /// verdict.
    ///
    /// A fail (non-empty `diagnostics`) is always emitted. A pass is
    /// admitted only when the pass rate window allows it; on throttle
    /// the row is dropped and [`EmissionOutcome::Throttled`] returned.
    /// Sink errors are logged via `tracing::warn!` and surfaced as
    /// [`EmissionOutcome::SinkError`] — never propagated, so the verdict
    /// response always reaches the client.
    ///
    /// `now` drives the pass rate window; production callers pass
    /// `Instant::now()` (kept as a parameter so tests drive the window
    /// deterministically).
    pub fn try_emit(
        &self,
        request: &SaveTimeEmissionRequest<'_>,
        diagnostics: &[Diagnostic],
        // The true number of changed paths in the verdict, recorded even when
        // `paths` is empty because the caller skipped the clone (paths off).
        file_count: usize,
        paths: &[String],
        now: Instant,
    ) -> EmissionOutcome {
        // A fail is audit-grade — never rate-checked. Only the pass
        // stream consults the window.
        let pending_drops = if diagnostics.is_empty() {
            match self.pass_rate_window.record(now) {
                RateDecision::Throttle { drops } => {
                    tracing::debug!(
                        target: "anvil_intercept::kindling_observation",
                        drops,
                        "save-time pass gate_evaluated emit throttled by rate window"
                    );
                    return EmissionOutcome::Throttled { drops };
                }
                RateDecision::Allow { pending_drops } => {
                    if pending_drops > 0 {
                        tracing::warn!(
                            target: "anvil_intercept::kindling_observation",
                            pending_drops,
                            "save-time pass gate_evaluated emit followed throttle burst",
                        );
                    }
                    pending_drops
                }
            }
        } else {
            // Fails bypass the window; no drops are attributable to a
            // fail emission.
            0
        };

        let ctx = ObservationContext {
            session_id: &self.daemon_session_id,
            timestamp: request.timestamp,
            gate_eval_id: request.gate_eval_id,
            // Unused on the save-time path (multi-path); the changed
            // set comes from `paths`.
            file_path: "",
            duration_ms: request.duration_ms,
        };
        let observation =
            from_validate_paths(&ctx, diagnostics, file_count, paths, self.include_paths);
        match self.sink.try_emit(observation) {
            Ok(()) => EmissionOutcome::Emitted { pending_drops },
            Err(err) => {
                tracing::warn!(
                    target: "anvil_intercept::kindling_observation",
                    error = %err,
                    "save-time gate_evaluated emit dropped: sink failure",
                );
                EmissionOutcome::SinkError
            }
        }
    }
}

impl std::fmt::Debug for SaveTimeObservationEmitter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SaveTimeObservationEmitter")
            .field("daemon_session_id", &self.daemon_session_id)
            .field("pass_rate_window", &self.pass_rate_window)
            .field("include_paths", &self.include_paths)
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

/// USAGE-004: outcome of a `command.invoked` emission attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandEmissionOutcome {
    /// Sink accepted the row.
    Emitted,
    /// Sink returned an error; the row was dropped and a
    /// `tracing::warn!` was logged. Dispatch continues regardless.
    SinkError,
}

/// Per-call inputs the JSON-RPC dispatcher supplies for one
/// `command.invoked` emission. Holds borrowed data so the handler can
/// build one inline from the envelope + params without extra owned
/// allocations.
#[derive(Debug, Clone, Copy)]
pub struct CommandInvokedEmissionRequest<'a> {
    /// The dispatched JSON-RPC method name — becomes the row `command`.
    pub method: &'a str,
    /// The salted-hash principal from the envelope, or `None` when the
    /// caller did not attach one (resolves to `"anonymous"`, parity with
    /// the unauthenticated CLI path). The raw principal is never on the
    /// wire — only this already-hashed value.
    pub principal: Option<&'a str>,
    /// The method's `params` object; redacted to argument *shapes* via
    /// [`arg_shapes_from_params`] — no raw value is ever retained.
    pub params: &'a serde_json::Value,
    /// ISO 8601 datetime the daemon observed the call (caller-minted, so
    /// the emitter stays clock-free and unit-testable).
    pub timestamp: &'a str,
    /// W3C `traceparent` off the envelope — the cross-pipe correlation
    /// key joining this row back to the originating telemetry span.
    pub traceparent: Option<&'a str>,
}

/// JSON-RPC dispatch-side emitter that builds a `command.invoked` row
/// for a user-initiated method call and hands it to the configured
/// sink. Like [`PostHookEmitter`]: exactly one row per call, no rate
/// window, never returns an error (sink failures are logged and the row
/// dropped so dispatch stays uncoupled from sink health).
pub struct CommandInvokedEmitter {
    sink: Arc<dyn KindlingObservationSink>,
    daemon_session_id: String,
    /// USAGE-004: `true` while the sink is in a failing run. Suppresses
    /// the per-call `warn!` after the first failure so a persistently
    /// unwritable sidecar (disk full, permission drift) cannot flood the
    /// trace stream under high-frequency GCTX traffic — one warn on the
    /// edge into failure, one info on recovery. (Council: log-spam.)
    sink_failing: std::sync::atomic::AtomicBool,
}

impl CommandInvokedEmitter {
    /// Construct an emitter with an explicit sink. Production callers
    /// wire a real sink; tests use [`Self::with_recorder`].
    #[must_use]
    pub fn new(sink: Arc<dyn KindlingObservationSink>, daemon_session_id: String) -> Self {
        Self {
            sink,
            daemon_session_id,
            sink_failing: std::sync::atomic::AtomicBool::new(false),
        }
    }

    /// Construct a noop emitter — a handle that discards every row.
    #[must_use]
    pub fn noop(daemon_session_id: impl Into<String>) -> Self {
        Self::new(
            Arc::new(NoopKindlingObservationSink) as Arc<dyn KindlingObservationSink>,
            daemon_session_id.into(),
        )
    }

    /// Construct a recording-sink emitter for tests. Returns
    /// `(emitter, recorder)` so the test can assert against a clone.
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

    /// Daemon-process-stable session id stamped onto every row. The
    /// originating client session is correlated via `traceparent`, not
    /// this field.
    #[must_use]
    pub fn daemon_session_id(&self) -> &str {
        &self.daemon_session_id
    }

    /// Build + emit a `command.invoked` row. Always builds; never
    /// returns an error — sink failures are logged internally so the
    /// dispatch path stays uncoupled from sink health. `flag_set` is
    /// empty on the daemon path (no resolver there), consistent with the
    /// USAGE-001 CLI producer before USAGE-002.
    pub fn try_emit(&self, request: &CommandInvokedEmissionRequest<'_>) -> CommandEmissionOutcome {
        let args = arg_shapes_from_params(request.params);
        let ctx = CommandInvocationContext {
            session_id: &self.daemon_session_id,
            timestamp: request.timestamp,
            command: request.method,
            principal: request.principal.unwrap_or("anonymous"),
            traceparent: request.traceparent,
        };
        let observation = from_command_invocation(&ctx, args, Vec::new());
        match self.sink.try_emit_command_invoked(observation) {
            Ok(()) => {
                // Recovery edge: log once when the sink starts working again.
                if self
                    .sink_failing
                    .swap(false, std::sync::atomic::Ordering::Relaxed)
                {
                    tracing::info!(
                        target: "anvil_intercept::kindling_observation",
                        "command.invoked sink recovered; resuming usage rows",
                    );
                }
                CommandEmissionOutcome::Emitted
            }
            Err(err) => {
                // Failure edge: warn once on entering a failing run, then
                // suppress per-call warns until recovery so a persistently
                // unwritable sidecar cannot flood the trace stream.
                if !self
                    .sink_failing
                    .swap(true, std::sync::atomic::Ordering::Relaxed)
                {
                    tracing::warn!(
                        target: "anvil_intercept::kindling_observation",
                        method = request.method,
                        error = %err,
                        "command.invoked emit dropped: sink failure; \
                         suppressing per-call warnings until the sink recovers",
                    );
                }
                CommandEmissionOutcome::SinkError
            }
        }
    }
}

impl std::fmt::Debug for CommandInvokedEmitter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CommandInvokedEmitter")
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

    /// USAGE-004: the dispatch emitter builds exactly one `command.invoked`
    /// row carrying the method as `command`, the envelope principal, the
    /// echoed `traceparent`, an empty `flag_set`, and redacted args.
    #[test]
    fn command_invoked_emitter_emits_one_row() {
        let (emitter, recorder) = CommandInvokedEmitter::with_recorder("daemon-session-1");
        let params = serde_json::json!({"query": "Foo", "token": "secret"});
        let outcome = emitter.try_emit(&CommandInvokedEmissionRequest {
            method: "anvil/gctx/search_symbols",
            principal: Some("deadbeef0123"),
            params: &params,
            timestamp: "2026-06-18T10:00:00Z",
            traceparent: Some("00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01"),
        });

        assert_eq!(outcome, CommandEmissionOutcome::Emitted);
        let rows = recorder.recorded_command_invocations();
        assert_eq!(rows.len(), 1);
        let row = &rows[0];
        assert_eq!(row.command, "anvil/gctx/search_symbols");
        assert_eq!(row.principal, "deadbeef0123");
        assert_eq!(row.session_id, "daemon-session-1");
        assert!(row.flag_set.is_empty());
        assert_eq!(
            row.traceparent.as_deref(),
            Some("00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01")
        );
        // Privacy: no raw values, sensitive key elided.
        let json = serde_json::to_string(row).expect("serialise");
        assert!(!json.contains("secret"), "raw value leaked: {json}");
        assert!(
            json.contains("<redacted>"),
            "sensitive key not redacted: {json}"
        );
    }

    /// USAGE-004: a missing envelope principal resolves to `anonymous`
    /// (parity with the unauthenticated CLI path).
    #[test]
    fn command_invoked_emitter_defaults_anonymous_principal() {
        let (emitter, recorder) = CommandInvokedEmitter::with_recorder("daemon-session-2");
        let params = serde_json::Value::Null;
        emitter.try_emit(&CommandInvokedEmissionRequest {
            method: "unblock-cascade",
            principal: None,
            params: &params,
            timestamp: "2026-06-18T10:00:00Z",
            traceparent: None,
        });
        assert_eq!(
            recorder.recorded_command_invocations()[0].principal,
            "anonymous"
        );
    }

    /// USAGE-004: a sink failure is swallowed (logged, row dropped) so
    /// the dispatch path is never coupled to sink health.
    #[test]
    fn command_invoked_emitter_swallows_sink_error() {
        let (emitter, recorder) = CommandInvokedEmitter::with_recorder("daemon-session-3");
        recorder.fail_next_command_with(KindlingSinkError::Unavailable("db locked".to_owned()));
        let params = serde_json::Value::Null;
        let outcome = emitter.try_emit(&CommandInvokedEmissionRequest {
            method: "anvil/gctx/find_callers",
            principal: None,
            params: &params,
            timestamp: "2026-06-18T10:00:00Z",
            traceparent: None,
        });
        assert_eq!(outcome, CommandEmissionOutcome::SinkError);
        assert!(recorder.recorded_command_invocations().is_empty());
    }

    /// USAGE-004 (Council: log-spam): repeated sink failures all return
    /// `SinkError` (rows always dropped, dispatch never coupled to sink
    /// health), and a later success resumes recording — the failing-run
    /// state used to throttle per-call warnings clears on recovery.
    #[test]
    fn command_invoked_emitter_recovers_after_failing_run() {
        let (emitter, recorder) = CommandInvokedEmitter::with_recorder("daemon-session-4");
        let params = serde_json::Value::Null;
        let req = |method| CommandInvokedEmissionRequest {
            method,
            principal: None,
            params: &params,
            timestamp: "2026-06-18T10:00:00Z",
            traceparent: None,
        };

        // Two consecutive failures (the throttle suppresses the 2nd warn,
        // but both still drop the row and report SinkError).
        recorder.fail_next_command_with(KindlingSinkError::Unavailable("disk full".to_owned()));
        assert_eq!(
            emitter.try_emit(&req("anvil/gctx/find_callers")),
            CommandEmissionOutcome::SinkError
        );
        recorder.fail_next_command_with(KindlingSinkError::Unavailable("disk full".to_owned()));
        assert_eq!(
            emitter.try_emit(&req("anvil/gctx/find_callers")),
            CommandEmissionOutcome::SinkError
        );
        assert!(recorder.recorded_command_invocations().is_empty());

        // Recovery: the next call succeeds and the row lands.
        assert_eq!(
            emitter.try_emit(&req("anvil/gctx/search_symbols")),
            CommandEmissionOutcome::Emitted
        );
        assert_eq!(recorder.recorded_command_invocations().len(), 1);
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
        assert!(
            json.contains("<redacted>"),
            "sensitive key not redacted: {json}"
        );
        assert!(!json.contains("super-secret"), "raw secret leaked: {json}");
        assert!(
            !json.contains("fn handle_jsonrpc_request"),
            "raw query value leaked: {json}"
        );
    }

    /// USAGE-004 (Council: nested-param redaction): a nested object/array
    /// is recorded as the fixed marker, so neither nested values nor the
    /// nested structure's size leak. Two differently-sized nested objects
    /// must produce identical shape metadata.
    #[test]
    fn arg_shapes_from_params_does_not_leak_nested_content_or_size() {
        let small = arg_shapes_from_params(&serde_json::json!({"filter": {"a": 1}}));
        let large = arg_shapes_from_params(&serde_json::json!({
            "filter": {"token": "supersecretvalue", "a": 1, "b": 2, "c": [1,2,3,4,5]}
        }));
        assert_eq!(small.len(), 1);
        assert_eq!(large.len(), 1);
        // Same coarse shape regardless of nested size → no size leak.
        assert_eq!(small[0].shape, large[0].shape);
        assert_eq!(small[0].length, large[0].length);
        // No nested value or nested key leaks into the serialised row.
        let json = serde_json::to_string(&large).expect("serialise");
        assert!(
            !json.contains("supersecretvalue"),
            "nested value leaked: {json}"
        );
        assert!(!json.contains("token"), "nested key leaked: {json}");
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

    // ----- DPO-001: save-time gate_evaluated builder + emitter -----

    #[test]
    fn from_validate_paths_pass_verdict_emits_pass_outcome() {
        let ctx = sample_ctx();
        // Mirror the hot path when paths are off: the count is passed but
        // the path slice is empty (the caller skipped the clone).
        let obs = from_validate_paths(&ctx, &[], 2, &[], false);
        assert_eq!(obs.outcome, Outcome::Pass);
        assert_eq!(obs.gate_id, SAVE_TIME_GATE_ID);
        assert_eq!(obs.kind, KIND_GATE_EVALUATED);
        assert_eq!(obs.enforcement, Enforcement::Informational);
        assert_eq!(obs.violation_count, Some(0));
        assert_eq!(obs.warning_count, Some(0));
        assert!(obs.rules_violated.is_none());
        assert!(obs.rules_evaluated.is_empty());
        assert_eq!(obs.inputs.file_count, 2);
        assert!(
            obs.inputs.changed_files.is_empty(),
            "include_paths=false must omit the path set",
        );
    }

    #[test]
    fn from_validate_paths_populates_changed_files_when_include_paths() {
        let ctx = sample_ctx();
        let paths = vec!["src/a.rs".to_string(), "src/b.rs".to_string()];
        let obs = from_validate_paths(&ctx, &[], paths.len(), &paths, true);
        assert_eq!(obs.inputs.changed_files, paths);
        assert_eq!(obs.inputs.file_count, 2);
    }

    #[test]
    fn from_validate_paths_fail_verdict_emits_fail_outcome_with_rules() {
        let ctx = sample_ctx();
        let paths = vec!["src/lib.rs".to_string()];
        let diags = vec![
            make_diag("info-1", Severity::Info),
            make_diag("warn-1", Severity::Warning),
            make_diag("err-1", Severity::Error),
        ];
        let obs = from_validate_paths(&ctx, &diags, paths.len(), &paths, true);
        assert_eq!(obs.outcome, Outcome::Fail);
        assert_eq!(obs.gate_id, SAVE_TIME_GATE_ID);
        assert_eq!(obs.enforcement, Enforcement::Blocking);
        assert_eq!(obs.violation_count, Some(1));
        assert_eq!(obs.warning_count, Some(1));
        assert_eq!(
            obs.rules_violated.expect("rules_violated present"),
            vec!["warn-1".to_string(), "err-1".to_string()],
        );
        assert_eq!(
            obs.rules_evaluated,
            vec![
                "info-1".to_string(),
                "warn-1".to_string(),
                "err-1".to_string()
            ],
        );
    }

    fn sample_save_time_request() -> SaveTimeEmissionRequest<'static> {
        SaveTimeEmissionRequest {
            gate_eval_id: "gate-eval-save-1",
            timestamp: "2026-06-19T10:00:00Z",
            duration_ms: 33,
        }
    }

    #[test]
    fn save_time_emitter_emits_pass_and_fail_rows() {
        let (emitter, recorder) = SaveTimeObservationEmitter::with_recorder(SESSION_UUID, true);
        let paths = vec!["src/lib.rs".to_string()];
        // Pass row.
        let pass =
            emitter.try_emit(&sample_save_time_request(), &[], paths.len(), &paths, Instant::now());
        assert_eq!(pass, EmissionOutcome::Emitted { pending_drops: 0 });
        // Fail row.
        let diags = vec![make_diag("err-1", Severity::Error)];
        let fail = emitter.try_emit(
            &sample_save_time_request(),
            &diags,
            paths.len(),
            &paths,
            Instant::now(),
        );
        assert_eq!(fail, EmissionOutcome::Emitted { pending_drops: 0 });

        let rows = recorder.recorded();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].outcome, Outcome::Pass);
        assert_eq!(rows[0].session_id, SESSION_UUID);
        assert_eq!(rows[0].gate_id, SAVE_TIME_GATE_ID);
        assert_eq!(rows[1].outcome, Outcome::Fail);
    }

    #[test]
    fn save_time_emitter_always_emits_fail_even_when_pass_window_saturated() {
        // Capacity-1 pass window over a long window: the first pass admits,
        // every later pass throttles — but fails must bypass the window.
        let recorder = Arc::new(RecordingKindlingObservationSink::new());
        let emitter = SaveTimeObservationEmitter::new(
            Arc::clone(&recorder) as Arc<dyn KindlingObservationSink>,
            RateWindow::new(1, Duration::from_mins(1)),
            SESSION_UUID.to_string(),
            false,
        );
        let paths = vec!["src/lib.rs".to_string()];
        let now = Instant::now();
        // Saturate the pass window.
        assert_eq!(
            emitter.try_emit(&sample_save_time_request(), &[], paths.len(), &paths, now),
            EmissionOutcome::Emitted { pending_drops: 0 },
        );
        assert!(matches!(
            emitter.try_emit(&sample_save_time_request(), &[], paths.len(), &paths, now),
            EmissionOutcome::Throttled { .. },
        ));
        // A fail still flows despite the saturated pass window.
        let diags = vec![make_diag("err-1", Severity::Error)];
        assert_eq!(
            emitter.try_emit(&sample_save_time_request(), &diags, paths.len(), &paths, now),
            EmissionOutcome::Emitted { pending_drops: 0 },
        );
        let rows = recorder.recorded();
        assert_eq!(rows.len(), 2, "one pass + one fail must reach the sink");
        assert_eq!(rows[0].outcome, Outcome::Pass);
        assert_eq!(rows[1].outcome, Outcome::Fail);
    }

    #[test]
    fn save_time_emitter_throttles_passes_after_capacity_within_window() {
        let recorder = Arc::new(RecordingKindlingObservationSink::new());
        let emitter = SaveTimeObservationEmitter::new(
            Arc::clone(&recorder) as Arc<dyn KindlingObservationSink>,
            RateWindow::new(2, Duration::from_mins(1)),
            SESSION_UUID.to_string(),
            false,
        );
        let paths = vec!["src/lib.rs".to_string()];
        let now = Instant::now();
        assert_eq!(
            emitter.try_emit(&sample_save_time_request(), &[], paths.len(), &paths, now),
            EmissionOutcome::Emitted { pending_drops: 0 },
        );
        assert_eq!(
            emitter.try_emit(&sample_save_time_request(), &[], paths.len(), &paths, now),
            EmissionOutcome::Emitted { pending_drops: 0 },
        );
        assert_eq!(
            emitter.try_emit(&sample_save_time_request(), &[], paths.len(), &paths, now),
            EmissionOutcome::Throttled { drops: 1 },
        );
        assert_eq!(
            recorder.len(),
            2,
            "throttled passes must not reach the sink"
        );
    }

    #[test]
    fn save_time_emitter_swallows_sink_error() {
        let (emitter, recorder) = SaveTimeObservationEmitter::with_recorder(SESSION_UUID, true);
        recorder.fail_next_with(KindlingSinkError::Unavailable("db locked".into()));
        let paths = vec!["src/lib.rs".to_string()];
        let diags = vec![make_diag("err-1", Severity::Error)];
        let outcome = emitter.try_emit(
            &sample_save_time_request(),
            &diags,
            paths.len(),
            &paths,
            Instant::now(),
        );
        assert_eq!(outcome, EmissionOutcome::SinkError);
        assert!(recorder.is_empty(), "failed emit must not record the row");
    }

    // ----- DPO-002: fence constraint_applied builder -----

    #[test]
    fn from_fence_preserves_known_reasons() {
        let cascade = from_fence(
            SESSION_UUID,
            "2026-06-19T10:00:00Z",
            "/work/tree",
            crate::telemetry::DEGRADED_FENCE_CASCADE,
            true,
            true,
        );
        assert_eq!(cascade.kind, KIND_CONSTRAINT_APPLIED);
        assert_eq!(cascade.gate_id, FENCE_GATE_ID);
        assert_eq!(cascade.constraint_id, FENCE_GATE_ID);
        assert_eq!(cascade.worktree, "/work/tree");
        assert_eq!(cascade.reason, crate::telemetry::DEGRADED_FENCE_CASCADE);
        assert!(cascade.cascade);

        let spoof = from_fence(
            SESSION_UUID,
            "2026-06-19T10:00:00Z",
            "/work/tree",
            crate::telemetry::DEGRADED_SPOOFED_ATTRIBUTION,
            false,
            true,
        );
        assert_eq!(spoof.reason, crate::telemetry::DEGRADED_SPOOFED_ATTRIBUTION);
        assert!(!spoof.cascade);
    }

    /// DPO-002 (council C): `from_fence` redacts the worktree to the
    /// schema-stable placeholder when `include_paths=false`, and keeps the
    /// real path when `true`.
    #[test]
    fn from_fence_redacts_worktree_when_include_paths_false() {
        let redacted = from_fence(
            SESSION_UUID,
            "2026-06-19T10:00:00Z",
            "/work/tree",
            crate::telemetry::DEGRADED_FENCE_CASCADE,
            false,
            false,
        );
        assert_eq!(redacted.worktree, REDACTED_WORKTREE);
        assert_eq!(redacted.worktree, "<redacted>");

        let present = from_fence(
            SESSION_UUID,
            "2026-06-19T10:00:00Z",
            "/work/tree",
            crate::telemetry::DEGRADED_FENCE_CASCADE,
            false,
            true,
        );
        assert_eq!(present.worktree, "/work/tree");
    }

    #[test]
    fn from_fence_normalises_arbitrary_operator_reason() {
        let row = from_fence(
            SESSION_UUID,
            "2026-06-19T10:00:00Z",
            "/work/tree",
            "manual review: see ticket /home/op/notes.txt",
            false,
            true,
        );
        assert_eq!(
            row.reason, FENCE_REASON_OPERATOR,
            "free-form operator text must collapse to the bounded token",
        );
        let json = serde_json::to_string(&row).expect("serialise");
        assert!(
            !json.contains("notes.txt"),
            "operator free-text leaked into the row: {json}",
        );
    }

    #[test]
    fn normalise_fence_reason_maps_unknown_to_operator() {
        assert_eq!(
            normalise_fence_reason("rule violation"),
            FENCE_REASON_OPERATOR
        );
        assert_eq!(
            normalise_fence_reason(crate::telemetry::DEGRADED_FENCE_CASCADE),
            crate::telemetry::DEGRADED_FENCE_CASCADE,
        );
    }

    #[test]
    fn recording_sink_records_constraint_applied_rows() {
        let sink = RecordingKindlingObservationSink::new();
        let row = from_fence(
            SESSION_UUID,
            "2026-06-19T10:00:00Z",
            "/work/tree",
            crate::telemetry::DEGRADED_FENCE_CASCADE,
            true,
            true,
        );
        sink.try_emit_constraint_applied(row).expect("record");
        let rows = sink.recorded_constraints();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].constraint_id, FENCE_GATE_ID);
        assert!(rows[0].cascade);
        assert!(!sink.is_empty());
    }

    #[test]
    fn recording_sink_constraint_fail_next_is_swallowed_by_emitter_callers() {
        let sink = RecordingKindlingObservationSink::new();
        sink.fail_next_constraint_with(KindlingSinkError::Rejected("dup".into()));
        let row = from_fence(
            SESSION_UUID,
            "2026-06-19T10:00:00Z",
            "/work/tree",
            crate::telemetry::DEGRADED_FENCE_CASCADE,
            false,
            true,
        );
        assert!(sink.try_emit_constraint_applied(row).is_err());
        assert!(sink.recorded_constraints().is_empty());
    }

    // ----- DPO-001: NonBlockingObservationSink (council T2 boundary) -----

    /// A sink whose `try_emit` sleeps a fixed duration, optionally
    /// recording rows. Used to prove the decorator's calls return without
    /// waiting for the (slow) inner sink, and that rows eventually arrive.
    #[derive(Debug)]
    struct SlowSink {
        delay: Duration,
        seen: Arc<AtomicU64>,
    }

    impl SlowSink {
        fn new(delay: Duration) -> (Arc<Self>, Arc<AtomicU64>) {
            let seen = Arc::new(AtomicU64::new(0));
            let sink = Arc::new(Self {
                delay,
                seen: Arc::clone(&seen),
            });
            (sink, seen)
        }
    }

    impl KindlingObservationSink for SlowSink {
        fn try_emit(
            &self,
            _observation: GateEvaluatedObservation,
        ) -> Result<(), KindlingSinkError> {
            std::thread::sleep(self.delay);
            self.seen.fetch_add(1, Ordering::Relaxed);
            Ok(())
        }
    }

    fn sample_gate_row() -> GateEvaluatedObservation {
        from_validate_paths(&sample_ctx(), &[], 1, &["src/lib.rs".to_string()], false)
    }

    /// (a) The decorator's `try_emit` returns in well under the summed
    /// inner latency — proving the producer hot path is never blocked on
    /// the slow inner sink (the ADR-031 acceptance for DPO-001).
    #[test]
    fn non_blocking_sink_returns_without_waiting_on_slow_inner() {
        // 20 calls × 50 ms inner sleep = 1 s of inner work if it were
        // synchronous. Capacity is generous so none are dropped; we only
        // measure that the producer-side calls return fast.
        let (inner, _seen) = SlowSink::new(Duration::from_millis(50));
        let sink = NonBlockingObservationSink::new(inner as Arc<dyn KindlingObservationSink>, 64)
            .expect("spawn");

        let start = Instant::now();
        for _ in 0..20 {
            sink.try_emit(sample_gate_row()).expect("never errors");
        }
        let elapsed = start.elapsed();
        assert!(
            elapsed < Duration::from_millis(25),
            "producer calls must not block on the slow inner sink; took {elapsed:?}",
        );
    }

    /// (b) With a tiny capacity and the drain stuck on a slow inner sink,
    /// the excess calls increment `dropped_count` and STILL return `Ok` —
    /// drop-on-full, never block, never error.
    #[test]
    fn non_blocking_sink_drops_on_full_and_never_errors() {
        // A very slow inner sink keeps the single drain thread busy on the
        // first row, so the bounded channel fills and excess rows drop.
        let (inner, _seen) = SlowSink::new(Duration::from_millis(200));
        let sink = NonBlockingObservationSink::new(inner as Arc<dyn KindlingObservationSink>, 1)
            .expect("spawn");

        // Fire many more than capacity while the drain is blocked.
        for _ in 0..100 {
            sink.try_emit(sample_gate_row()).expect("always returns Ok");
        }
        assert!(
            sink.dropped_count() > 0,
            "excess rows past a full channel must be counted as dropped",
        );
    }

    /// (c) A fast inner sink eventually receives every forwarded row once
    /// the decorator is dropped (Drop closes the sender and joins the
    /// drain, flushing the queue).
    #[test]
    fn non_blocking_sink_forwards_all_rows_to_a_fast_inner() {
        let recorder = Arc::new(RecordingKindlingObservationSink::new());
        let sink = NonBlockingObservationSink::new(
            Arc::clone(&recorder) as Arc<dyn KindlingObservationSink>,
            256,
        )
        .expect("spawn");
        for _ in 0..20 {
            sink.try_emit(sample_gate_row()).expect("ok");
        }
        // Dropping the decorator closes the channel and joins the drain,
        // so every queued row is forwarded before `drop` returns.
        drop(sink);
        assert_eq!(recorder.len(), 20, "fast inner must receive every row");
        assert_eq!(recorder.recorded_constraints().len(), 0);
    }

    /// All four envelope variants route to the matching inner method.
    #[test]
    fn non_blocking_sink_routes_every_kind() {
        let recorder = Arc::new(RecordingKindlingObservationSink::new());
        let sink = NonBlockingObservationSink::new(
            Arc::clone(&recorder) as Arc<dyn KindlingObservationSink>,
            64,
        )
        .expect("spawn");
        sink.try_emit(sample_gate_row()).expect("gate");
        sink.try_emit_command_invoked(from_command_invocation(
            &CommandInvocationContext {
                session_id: SESSION_UUID,
                timestamp: "2026-06-19T10:00:00Z",
                command: "check",
                principal: "anonymous",
                traceparent: None,
            },
            Vec::new(),
            Vec::new(),
        ))
        .expect("command");
        sink.try_emit_action_executed(from_post_hook(
            SESSION_UUID,
            &sample_post_hook_request(PostHookAction::PostCommit),
        ))
        .expect("action");
        sink.try_emit_constraint_applied(from_fence(
            SESSION_UUID,
            "2026-06-19T10:00:00Z",
            "/work/tree",
            crate::telemetry::DEGRADED_FENCE_CASCADE,
            true,
            true,
        ))
        .expect("constraint");
        drop(sink);
        assert_eq!(recorder.len(), 1, "one gate row");
        assert_eq!(recorder.commands_len(), 1, "one command row");
        assert_eq!(recorder.actions_len(), 1, "one action row");
        assert_eq!(recorder.constraints_len(), 1, "one constraint row");
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
