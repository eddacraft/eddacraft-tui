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
    fail_next: Mutex<Option<KindlingSinkError>>,
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

    /// Snapshot the recorded observations in arrival order.
    #[must_use]
    pub fn recorded(&self) -> Vec<GateEvaluatedObservation> {
        self.observations
            .lock()
            .expect("observations mutex")
            .clone()
    }

    /// Number of observations recorded.
    #[must_use]
    pub fn len(&self) -> usize {
        self.observations.lock().expect("observations mutex").len()
    }

    /// True when no observations have been recorded yet.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
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
            rules_sha: None,
        }
    }

    fn response_with(diagnostics: Vec<Diagnostic>) -> ScanBufferResponse {
        ScanBufferResponse {
            version: 1,
            diagnostics,
            truncated: false,
            rules_sha: None,
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
}
