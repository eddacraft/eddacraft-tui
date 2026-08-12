use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use anvil_intercept_proto::protocol::DiagnosticEnvelope;
use anvil_intercept_rules::ChangeKind;
use anvil_kernel_types::Mode;
use anvil_kernel_types::diagnostics::KnownMode;
use serde::Serialize;
use thiserror::Error;
use tokio::sync::{Semaphore, TryAcquireError};

use crate::enforcement::{CONTENT_SIZE_CAP_BYTES_USIZE, EnforcementPipeline, ProposedChange};
use crate::kindling_observation::MidEditObservationEmitter;
use crate::latency::LatencyAggregator;

pub const SCAN_BUFFER_METHOD: &str = "scan_buffer";
pub const MAX_CONCURRENT_SCAN_BUFFERS: usize = 2;
pub const MAX_SCAN_BUFFER_DIAGNOSTICS: usize = 128;
pub const MAX_SCAN_BUFFER_PATH_BYTES: usize = 4096;
pub const SCAN_BUFFER_TIMEOUT: Duration = Duration::from_secs(2);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScanBufferMode {
    MidEdit,
    PreWrite,
}

impl ScanBufferMode {
    pub fn parse(value: &str) -> Result<Self, ScanBufferError> {
        match value {
            "midEdit" | "mid-edit" => Ok(Self::MidEdit),
            "preWrite" | "pre-write" => Ok(Self::PreWrite),
            _ => Err(ScanBufferError::UnsupportedMode),
        }
    }

    fn diagnostic_mode(self) -> Mode {
        match self {
            Self::MidEdit => Mode::known(KnownMode::MidEdit),
            Self::PreWrite => Mode::Unknown("pre-write".to_string()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScanBufferRequest {
    pub path: PathBuf,
    pub text: String,
    pub version: u64,
    pub mode: ScanBufferMode,
    /// MLP2-025b: env-supplied `AgentTag` from the writer process,
    /// carried as a raw string (the launcher reads its own
    /// `ANVIL_AGENT_TAG` env var and forwards it verbatim). The
    /// daemon decodes via `anvil_attribution::env::agent_tag_from_env_value`
    /// at the boundary so malformed values fold to the same
    /// `Cross::Spoofed` verdict as out-of-lineage forgeries, rather
    /// than surfacing as a deserialisation error.
    ///
    /// `None` for pre-MLP2-025b writers — the cross-check returns
    /// `Cross::Untagged` and enforcement proceeds unchanged. See
    /// spec §3.1 + Q3 in
    /// `plans/specs/2026-05-16-mlp2-025-spoof-cross-check-control-lane.md`.
    pub env_agent_tag: Option<String>,
    /// CLAWP-065: optional authenticated session binding. When the
    /// writer supplies a `session_id`, the daemon's IPC layer resolves
    /// the session that owns the connection's authenticated peer-PID
    /// lineage and rejects the request with a structured JSON-RPC
    /// error when the claimed id is not that session — closing the gap
    /// where `scan_buffer` could not reject a request issued under
    /// another session's identity.
    ///
    /// `None` keeps the legacy unbound path: pre-CLAWP-065 writers (and
    /// today's TS driver, which does not send the field) are
    /// unaffected. The ownership check lives in
    /// [`crate::ipc`]'s `scan_buffer_from_jsonrpc`; the pure pipeline
    /// never reads this field — it mirrors `env_agent_tag`, which is
    /// also consumed only at the IPC boundary.
    pub session_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ScanBufferResponse {
    pub version: u64,
    /// B3 (ADR-061): typed against the proto-owned
    /// [`DiagnosticEnvelope`] (`Vec<Diagnostic>`) so this response and
    /// the Sub-phase A `validate_paths` response share one wire
    /// diagnostic type rather than each re-declaring the shape.
    pub diagnostics: DiagnosticEnvelope,
    pub truncated: bool,
    /// MLP2-002: `rules_sha` pinned at evaluation start. The
    /// scheduler resolves the rule set against the worktree once,
    /// records the sha, and threads it through to the response so
    /// the witness chain (when wired by MLP2-014) can attribute the
    /// evaluation to a specific rule version even when the cache is
    /// invalidated mid-call. `None` for v1 callers that haven't
    /// wired the rule-set cache.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rules_sha: Option<String>,
    /// MLP2-025b: present when the daemon control-lane refused the
    /// write because the env-supplied `AgentTag` did not match any
    /// daemon-issued tag on the writer's PID lineage. The IPC
    /// handler short-circuits before running the rule engine, so
    /// `diagnostics` is empty and `rules_sha` is `None` when this
    /// field is `Some`. Mutually exclusive with diagnostics —
    /// pinned by spec §6 inv-4.
    ///
    /// Wire-additive via `skip_serializing_if`. Legacy clients
    /// ignore the field and see an empty-diagnostics response (which
    /// is correct: the rule engine never ran). MLP2-025b-aware
    /// clients surface the operator-touch trail (worktree-level
    /// fence + reason).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spoof_block: Option<SpoofBlockInfo>,
}

/// MLP2-025b: details of a daemon-side spoof block. Populated on
/// [`ScanBufferResponse::spoof_block`] when the control-lane
/// short-circuits a write because of an out-of-lineage env-tag
/// forgery. See `plans/specs/2026-05-16-mlp2-025-spoof-cross-check-control-lane.md`
/// §3.3.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SpoofBlockInfo {
    /// Always [`crate::telemetry::DEGRADED_SPOOFED_ATTRIBUTION`]
    /// for v1; carried explicitly so structured-log consumers do
    /// not need to look up the constant.
    pub reason: String,
    /// Canonicalised worktree that the daemon fenced as a side
    /// effect of the block. Future operations on this worktree
    /// remain blocked until the operator runs
    /// `anvil intercept unblock <worktree>`.
    pub fenced_worktree: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ScanBufferError {
    #[error(
        "unsupported scan_buffer mode; canonical modes: midEdit, preWrite (aliases: mid-edit, pre-write)"
    )]
    UnsupportedMode,
    #[error("path exceeds {cap} byte cap")]
    PathTooLong { len: usize, cap: usize },
    #[error("path must be a non-empty string without control characters")]
    InvalidPath,
    #[error("content exceeds {cap} byte cap (got {len} bytes)")]
    ContentTooLarge { len: usize, cap: usize },
    /// Buffer contains a NUL byte. Treated as binary / non-text input
    /// and rejected before the rule pipeline runs so clients cannot
    /// mistake an unscanned payload for a clean success (pre-write
    /// callers would otherwise authorise unenforced content).
    #[error("binary content is not supported by scan_buffer (NUL byte present)")]
    BinaryContent,
    #[error("scan_buffer service busy")]
    Busy,
    #[error("scan_buffer timed out")]
    TimedOut,
    #[error("scan_buffer service unavailable")]
    ServiceUnavailable,
    #[error("scan_buffer worker failed: {0}")]
    WorkerFailed(String),
}

#[derive(Clone)]
pub struct ScanBufferService {
    pipeline: Arc<EnforcementPipeline>,
    permits: Arc<Semaphore>,
    timeout: Duration,
    /// INTD-011: sliding-window aggregator for the daemon-handled
    /// portion of mid-edit `scan_buffer` RPCs. Pre-write samples and
    /// IPC-roundtrip costs are deliberately excluded — ADR-031 names
    /// those `validation.roundtrip` and treats them as a separate
    /// dimension owned by the driver-side benchmarks.
    latency: LatencyAggregator,
    /// MLP2-002: count of evaluations currently in flight. Bumped when
    /// a worker is admitted and decremented when that worker finishes
    /// (including stragglers whose caller already timed out). Observable
    /// via [`Self::in_flight`] so the daemon's status surface and
    /// burst-coalescing telemetry can see how many evaluations a
    /// config write would have to wait out without disturbing.
    in_flight: Arc<AtomicUsize>,
    /// MLP2-006: optional Kindling `gate_evaluated` notification
    /// fan-out. The IPC handler reads this via
    /// [`Self::observation_emitter`] and emits one observation per
    /// scan completion. `None` keeps the daemon legacy-quiet — the
    /// CLI / tests / embedded fallback all start without an emitter
    /// until the host wires one in via [`Self::with_observation_emitter`].
    observation_emitter: Option<Arc<MidEditObservationEmitter>>,
}

impl std::fmt::Debug for ScanBufferService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ScanBufferService")
            .field("pipeline", &self.pipeline)
            .field("timeout", &self.timeout)
            .finish_non_exhaustive()
    }
}

impl ScanBufferService {
    #[must_use]
    pub fn new(pipeline: EnforcementPipeline) -> Self {
        Self::with_timeout(pipeline, SCAN_BUFFER_TIMEOUT)
    }

    /// Build a service with a custom timeout. Reserved for tests that
    /// need timeout / straggler-capacity behaviour observable inside a
    /// few hundred milliseconds; production callers stick with
    /// [`Self::new`].
    #[must_use]
    pub fn with_timeout(pipeline: EnforcementPipeline, timeout: Duration) -> Self {
        Self {
            pipeline: Arc::new(pipeline),
            permits: Arc::new(Semaphore::new(MAX_CONCURRENT_SCAN_BUFFERS)),
            timeout,
            latency: LatencyAggregator::new(),
            in_flight: Arc::new(AtomicUsize::new(0)),
            observation_emitter: None,
        }
    }

    /// MLP2-006: install a Kindling notification fan-out. The IPC
    /// handler reads it via [`Self::observation_emitter`] and emits
    /// one `gate_evaluated` row per finding-bearing scan completion.
    /// Calling without this builder leaves the service silent — the
    /// `scan_buffer` wire shape stays byte-compatible.
    #[must_use]
    pub fn with_observation_emitter(mut self, emitter: Arc<MidEditObservationEmitter>) -> Self {
        self.observation_emitter = Some(emitter);
        self
    }

    /// MLP2-006: borrow the configured Kindling notification fan-out.
    /// `None` when no emitter has been installed (default for CLI /
    /// tests / embedded fallback).
    #[must_use]
    pub fn observation_emitter(&self) -> Option<&Arc<MidEditObservationEmitter>> {
        self.observation_emitter.as_ref()
    }

    /// MLP2-002: number of evaluations currently held by a permit.
    /// Intended for burst-coalescing telemetry once MLP2-014 wires
    /// the witness chain; today there is no in-process consumer
    /// reading this value for scheduling decisions.
    ///
    /// Reads with `Acquire` ordering so a value of `0` observed from
    /// any thread provides the visibility guarantee the
    /// `in_flight==0 after exit` test asserts on weakly-ordered
    /// architectures (ARM/POWER). Pairs with `AcqRel` on the
    /// `fetch_add`/`fetch_sub` in [`InFlightGuard`] (Council
    /// 2026-05-14 #C-040). Pinning correctness itself is guaranteed
    /// by each evaluation moving the pinned `rules_sha` into its
    /// frame before the cache mutates — not by reading this number.
    #[must_use]
    pub fn in_flight(&self) -> usize {
        self.in_flight.load(Ordering::Acquire)
    }

    /// Borrow the latency aggregator. The IPC `query_status` handler
    /// uses this to read the rollup without taking ownership of the
    /// service.
    #[must_use]
    pub fn latency(&self) -> &LatencyAggregator {
        &self.latency
    }

    pub async fn scan_buffer(
        &self,
        request: ScanBufferRequest,
    ) -> Result<ScanBufferResponse, ScanBufferError> {
        self.scan_buffer_with_pin(request, None).await
    }

    /// MLP2-002: variant of [`Self::scan_buffer`] that pins the
    /// resolved `rules_sha` for the duration of the evaluation.
    ///
    /// The scheduler (CLI / IPC handler) resolves the active rule
    /// set against the worktree before calling, captures the
    /// `rules_sha` from
    /// [`crate::rule_cache::RuleSetCache::lookup`], and passes it
    /// here. The value is **moved** into the request scope: a
    /// concurrent watcher-driven `invalidate_on_change` may clear
    /// the cache, but the in-flight evaluation keeps operating
    /// against this pinned sha. The next call resolves freshly and
    /// picks up the new set.
    ///
    /// The pinned sha is round-tripped to the response so the
    /// witness-chain layer (MLP2-014) can record which rule set
    /// produced the evaluation without re-querying the cache.
    pub async fn scan_buffer_with_pin(
        &self,
        request: ScanBufferRequest,
        pinned_rules_sha: Option<String>,
    ) -> Result<ScanBufferResponse, ScanBufferError> {
        // Capacity is bound to the *worker* lifetime, not the caller's
        // await. Sync rules cannot be cancelled mid-flight; if a caller
        // times out, the worker keeps running and must continue to
        // occupy its concurrency slot. Releasing the permit on the
        // TimedOut path would let repeated timeouts spawn an unbounded
        // number of OS threads despite MAX_CONCURRENT_SCAN_BUFFERS.
        // Under that failure mode the service correctly reports Busy
        // until stragglers finish (or the process is restarted).
        let permit = self
            .permits
            .clone()
            .try_acquire_owned()
            .map_err(|err| match err {
                TryAcquireError::NoPermits => ScanBufferError::Busy,
                TryAcquireError::Closed => ScanBufferError::ServiceUnavailable,
            })?;
        // MLP2-002: bump the in-flight counter under an RAII guard so
        // the count reflects live workers (including TimedOut
        // stragglers). Moved into the worker alongside the permit;
        // both drop when the worker finishes. If spawn fails, the
        // closure is dropped without running and the guards release.
        let inflight = InFlightGuard::new(Arc::clone(&self.in_flight));
        // INTD-011: capture the request mode before moving `request`
        // into the worker thread. Only `mode = midEdit` samples are
        // recorded — the latency aggregator is the daemon-side
        // `validation.service` boundary per ADR-031, scoped to the
        // mid-edit budget class. Pre-write timings get a separate
        // mode if RTAI-006 ever needs them.
        let request_mode = request.mode;
        let pipeline = Arc::clone(&self.pipeline);
        let (sender, receiver) = tokio::sync::oneshot::channel();
        std::thread::Builder::new()
            .name("anvil-scan-buffer".to_owned())
            .spawn(move || {
                // Hold capacity for the full worker lifetime. Named
                // bindings (not `let _ = ...`) live until the end of
                // this scope; the explicit `drop` below pins that
                // lifetime past the pipeline call and oneshot send so
                // NLL cannot shrink it earlier.
                let permit = permit;
                let inflight = inflight;
                // ADR-031: `validation.service` boundary — start at
                // "daemon has accepted a complete validation request"
                // (the worker thread has been spawned with the parsed
                // `request`), end at "daemon has produced the response
                // payload" (the pipeline call returns). Measuring on
                // the worker thread keeps the aggregator off the IPC
                // event loop's hot path. Not all platforms guarantee
                // monotonic `Instant` under VM clock skew — the
                // aggregator handles backwards steps with
                // `saturating_duration_since`.
                let started = Instant::now();
                let result = scan_buffer_with_pipeline(&request, &pipeline);
                let elapsed = started.elapsed();
                // Send the (result, elapsed) pair so the caller can
                // record into the aggregator AFTER awaiting — the
                // worker thread does not import the aggregator
                // because keeping the recording on the awaiting
                // side keeps the timeout path correct (a timed-out
                // call MUST NOT poison the aggregator with the
                // worker's straggler duration).
                let _ = sender.send((result, elapsed));
                drop((permit, inflight));
            })
            .map_err(|err| ScanBufferError::WorkerFailed(err.to_string()))?;

        match tokio::time::timeout(self.timeout, receiver).await {
            Ok(Ok((result, elapsed))) => {
                // Only record if the request mode is mid-edit — pre-
                // write traffic is intentionally excluded from the
                // mid-edit rollup. The recording happens on the
                // success path here (NOT inside the worker) so a
                // timed-out call does not contribute to the rollup.
                if matches!(request_mode, ScanBufferMode::MidEdit) && result.is_ok() {
                    self.latency.record(Instant::now(), elapsed);
                }
                result.map(|mut r| {
                    r.rules_sha = pinned_rules_sha;
                    r
                })
            }
            Ok(Err(err)) => Err(ScanBufferError::WorkerFailed(err.to_string())),
            Err(_) => {
                eprintln!(
                    "anvil-intercept: scan_buffer timed out after {:?}",
                    self.timeout
                );
                Err(ScanBufferError::TimedOut)
            }
        }
    }
}

/// MLP2-002: RAII guard that increments the in-flight counter on
/// construction and decrements on drop. Held on the worker thread
/// next to the semaphore permit so the counter always reflects
/// "live workers holding capacity", including `TimedOut` stragglers
/// whose caller has already returned.
struct InFlightGuard {
    counter: Arc<AtomicUsize>,
}

impl InFlightGuard {
    fn new(counter: Arc<AtomicUsize>) -> Self {
        // AcqRel pairs with the Acquire load in
        // `ScanBufferService::in_flight` so observers on ARM / POWER
        // see the increment before any work the worker thread does
        // is committed. Council 2026-05-14 #C-040.
        counter.fetch_add(1, Ordering::AcqRel);
        Self { counter }
    }
}

impl Drop for InFlightGuard {
    fn drop(&mut self) {
        // AcqRel decrement so an observer that sees `0` is
        // guaranteed to see the worker's writes that happened-before
        // this drop. Without the release side, `in_flight()==0` could
        // be observed while the worker's effects are still in flight
        // on a weakly-ordered architecture.
        self.counter.fetch_sub(1, Ordering::AcqRel);
    }
}

impl Default for ScanBufferService {
    fn default() -> Self {
        Self::new(EnforcementPipeline::default())
    }
}

pub fn scan_buffer_with_pipeline(
    request: &ScanBufferRequest,
    pipeline: &EnforcementPipeline,
) -> Result<ScanBufferResponse, ScanBufferError> {
    validate_scan_buffer_path(&request.path.to_string_lossy())?;
    let content = request.text.as_bytes();
    if content.len() > CONTENT_SIZE_CAP_BYTES_USIZE {
        return Err(ScanBufferError::ContentTooLarge {
            len: content.len(),
            cap: CONTENT_SIZE_CAP_BYTES_USIZE,
        });
    }
    if content.contains(&0) {
        // Reject rather than returning an empty successful scan: an
        // unscanned NUL buffer must not look clean to pre-write callers.
        return Err(ScanBufferError::BinaryContent);
    }

    let change = ProposedChange {
        path: &request.path,
        change_kind: ChangeKind::Modified,
        content: Some(content),
    };
    let mut diagnostics = pipeline.diagnostics_for_proposed_changes_with_limit(
        &[change],
        &request.mode.diagnostic_mode(),
        MAX_SCAN_BUFFER_DIAGNOSTICS + 1,
    );
    let truncated = diagnostics.len() > MAX_SCAN_BUFFER_DIAGNOSTICS;
    diagnostics.truncate(MAX_SCAN_BUFFER_DIAGNOSTICS);

    Ok(ScanBufferResponse {
        version: request.version,
        diagnostics,
        truncated,
        // MLP2-002: the pure-pipeline path has no notion of the
        // cache; the async [`ScanBufferService::scan_buffer_with_pin`]
        // overwrites this with the pinned value if the caller
        // supplied one.
        rules_sha: None,
        // MLP2-025b: the pipeline path is reached only after the
        // daemon control-lane confirms attribution is acceptable
        // (`Cross::Match` or `Cross::Untagged`); a spoof never
        // touches this function. Always `None` here.
        spoof_block: None,
    })
}

pub fn validate_scan_buffer_path(path: &str) -> Result<(), ScanBufferError> {
    if path.is_empty() || path.chars().any(char::is_control) {
        return Err(ScanBufferError::InvalidPath);
    }
    if path.len() > MAX_SCAN_BUFFER_PATH_BYTES {
        return Err(ScanBufferError::PathTooLong {
            len: path.len(),
            cap: MAX_SCAN_BUFFER_PATH_BYTES,
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::path::Path;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Barrier};

    use anvil_intercept_rules::{InterceptRule, RuleDecision, RuleInput, RuleRegistry};
    use anvil_kernel_types::{Category, Diagnostic, DiagnosticSource, Location, Severity};

    use super::*;
    use crate::enforcement::{
        EnforcementDecision, default_rule_registry, evaluate_proposed_changes,
    };

    fn secret_request() -> ScanBufferRequest {
        ScanBufferRequest {
            path: PathBuf::from("src/auth/client.ts"),
            text: "import { sdk } from './client';\nconst config = { api_key: 'abcdEFGH1234567890' };\nsdk.connect(config);\n".to_string(),
            version: 7,
            mode: ScanBufferMode::MidEdit,
            env_agent_tag: None,
            session_id: None,
        }
    }

    /// MLP2-025b: pin the wire-additive guard on the new
    /// `spoof_block` field. When `None`, the serialised response
    /// must not include the key — pre-MLP2-025b readers see no
    /// change.
    #[test]
    fn scan_buffer_response_omits_spoof_block_when_none() {
        let response = ScanBufferResponse {
            version: 1,
            diagnostics: Vec::new(),
            truncated: false,
            rules_sha: None,
            spoof_block: None,
        };
        let line = serde_json::to_string(&response).expect("serialise");
        assert!(
            !line.contains("\"spoof_block\""),
            "spoof_block omitted when None: got {line}"
        );
    }

    /// MLP2-025b: when populated, `spoof_block` serialises with the
    /// reason string and fenced worktree on the wire. The reason is
    /// always `degraded:spoofed-attribution` for v1.
    #[test]
    fn scan_buffer_response_includes_spoof_block_when_set() {
        let response = ScanBufferResponse {
            version: 1,
            diagnostics: Vec::new(),
            truncated: false,
            rules_sha: None,
            spoof_block: Some(SpoofBlockInfo {
                reason: crate::telemetry::DEGRADED_SPOOFED_ATTRIBUTION.to_string(),
                fenced_worktree: PathBuf::from("/work/wt"),
            }),
        };
        let line = serde_json::to_string(&response).expect("serialise");
        let parsed: serde_json::Value = serde_json::from_str(&line).expect("parse back");
        assert_eq!(
            parsed["spoof_block"]["reason"],
            "degraded:spoofed-attribution"
        );
        assert_eq!(parsed["spoof_block"]["fenced_worktree"], "/work/wt");
    }

    /// B3 (ADR-061 §8 parity): `ScanBufferResponse.diagnostics` is typed
    /// against the proto-owned [`DiagnosticEnvelope`]. The Sub-phase A
    /// `validate_paths` response will type its `diagnostics` field
    /// against the same alias, so the two surfaces serialise diagnostics
    /// byte-for-byte by construction. This pins the `scan_buffer` side of
    /// that contract: a bare envelope and the response's `diagnostics`
    /// field must serialise to identical JSON.
    #[test]
    fn scan_buffer_diagnostics_serialise_via_shared_proto_envelope() {
        let diagnostic = Diagnostic::new(
            "AP-001",
            Severity::Warning,
            "sample finding",
            Location {
                file: "src/lib.rs".to_string(),
                line: Some(12),
                column: Some(3),
                end_line: None,
                end_column: None,
            },
            Category::Antipattern,
            DiagnosticSource {
                rule_id: "AP-001".to_string(),
                source_module: "anvil-checks::antipattern".to_string(),
            },
            Mode::known(KnownMode::MidEdit),
        );

        // Built as the proto envelope type, then assigned into the
        // response field — only compiles while the field shares the
        // proto-owned alias.
        let envelope: DiagnosticEnvelope = vec![diagnostic];
        let response = ScanBufferResponse {
            version: 1,
            diagnostics: envelope.clone(),
            truncated: false,
            rules_sha: None,
            spoof_block: None,
        };

        let response_json = serde_json::to_value(&response).expect("serialise response");
        let envelope_json = serde_json::to_value(&envelope).expect("serialise envelope");
        assert_eq!(response_json["diagnostics"], envelope_json);
    }

    #[test]
    fn scan_buffer_uses_same_rule_registry_as_proposed_content_path() {
        let request = secret_request();
        let registry = default_rule_registry();
        let decision = evaluate_proposed_changes(
            &registry,
            &[ProposedChange {
                path: Path::new("src/auth/client.ts"),
                change_kind: ChangeKind::Modified,
                content: Some(request.text.as_bytes()),
            }],
        );
        let EnforcementDecision::Interrupt(interrupt) = decision else {
            panic!("secret fixture should interrupt")
        };

        let pipeline = EnforcementPipeline::new(default_rule_registry());
        let response = scan_buffer_with_pipeline(&request, &pipeline).expect("scan buffer");

        assert_eq!(response.version, request.version);
        assert_eq!(response.diagnostics.len(), 1);
        assert!(!response.truncated);
        assert_eq!(response.diagnostics[0].source.rule_id, interrupt.rule_id);
        assert_eq!(response.diagnostics[0].location.line, interrupt.line);
        assert_eq!(
            response.diagnostics[0].mode,
            Mode::known(KnownMode::MidEdit)
        );
    }

    #[test]
    fn scan_buffer_pre_write_mode_emits_pre_write_diagnostics() {
        let mut request = secret_request();
        request.mode = ScanBufferMode::PreWrite;

        let pipeline = EnforcementPipeline::default();
        let response = scan_buffer_with_pipeline(&request, &pipeline).expect("scan buffer");

        assert_eq!(response.diagnostics.len(), 1);
        assert_eq!(
            response.diagnostics[0].mode,
            Mode::Unknown("pre-write".to_string())
        );
        assert!(response.diagnostics[0].id.contains("pre_write"));
    }

    #[test]
    fn scan_buffer_rejects_content_above_cap() {
        let request = ScanBufferRequest {
            path: PathBuf::from("src/large.ts"),
            text: "a".repeat(CONTENT_SIZE_CAP_BYTES_USIZE + 1),
            version: 1,
            mode: ScanBufferMode::MidEdit,
            env_agent_tag: None,
            session_id: None,
        };

        let pipeline = EnforcementPipeline::default();
        let err =
            scan_buffer_with_pipeline(&request, &pipeline).expect_err("over cap should reject");

        assert!(matches!(err, ScanBufferError::ContentTooLarge { .. }));
    }

    #[test]
    fn scan_buffer_rejects_binary_content_with_nul() {
        let request = ScanBufferRequest {
            path: PathBuf::from("asset.bin"),
            text: "api_key='abcdEFGH1234567890'\0".to_string(),
            version: 2,
            mode: ScanBufferMode::MidEdit,
            env_agent_tag: None,
            session_id: None,
        };

        let pipeline = EnforcementPipeline::default();
        let err = scan_buffer_with_pipeline(&request, &pipeline)
            .expect_err("NUL-containing content must not return a clean scan");

        assert!(
            matches!(err, ScanBufferError::BinaryContent),
            "expected BinaryContent, got {err:?}"
        );
    }

    #[test]
    fn scan_buffer_pre_write_rejects_nul_even_with_rule_triggering_payload() {
        // Pre-write callers must not treat unscanned NUL content as
        // write-authorised: a clean success would bypass enforcement.
        let request = ScanBufferRequest {
            path: PathBuf::from("src/auth/client.ts"),
            text: "const api_key = 'abcdEFGH1234567890';\0".to_string(),
            version: 9,
            mode: ScanBufferMode::PreWrite,
            env_agent_tag: None,
            session_id: None,
        };

        let pipeline = EnforcementPipeline::new(default_rule_registry());
        let err = scan_buffer_with_pipeline(&request, &pipeline)
            .expect_err("pre-write NUL buffer must be rejected, not cleaned");

        assert!(matches!(err, ScanBufferError::BinaryContent));
    }

    #[test]
    fn scan_buffer_truncates_large_diagnostic_sets() {
        struct ManyDiagnosticsRule {
            seen_limit: Arc<AtomicUsize>,
        }

        impl InterceptRule for ManyDiagnosticsRule {
            fn rule_id(&self) -> &'static str {
                "many-diagnostics"
            }

            fn needs_content(&self) -> bool {
                true
            }

            fn evaluate(&self, _input: &RuleInput<'_>) -> RuleDecision {
                RuleDecision::Allow
            }

            fn diagnostics_with_limit(
                &self,
                input: &RuleInput<'_>,
                mode: &Mode,
                limit: usize,
            ) -> Vec<Diagnostic> {
                self.seen_limit.store(limit, Ordering::SeqCst);
                (0..=limit)
                    .map(|index| {
                        Diagnostic::new(
                            format!("diag_many_{index}"),
                            Severity::Warning,
                            "synthetic diagnostic",
                            Location {
                                file: input.path.to_string_lossy().into_owned(),
                                line: u32::try_from(index + 1).ok(),
                                column: None,
                                end_line: None,
                                end_column: None,
                            },
                            Category::Other,
                            DiagnosticSource {
                                rule_id: "many-diagnostics".to_string(),
                                source_module: "test".to_string(),
                            },
                            mode.clone(),
                        )
                    })
                    .collect()
            }
        }

        let seen_limit = Arc::new(AtomicUsize::new(0));
        let registry = RuleRegistry::with_rules(vec![Box::new(ManyDiagnosticsRule {
            seen_limit: Arc::clone(&seen_limit),
        })])
        .expect("unique rule");
        let pipeline = EnforcementPipeline::new(registry);
        let response = scan_buffer_with_pipeline(&secret_request(), &pipeline).expect("scan");

        assert_eq!(response.diagnostics.len(), MAX_SCAN_BUFFER_DIAGNOSTICS);
        assert!(response.truncated);
        assert_eq!(
            seen_limit.load(Ordering::SeqCst),
            MAX_SCAN_BUFFER_DIAGNOSTICS + 1
        );
    }

    #[tokio::test]
    async fn scan_buffer_rejects_when_workers_are_busy() {
        struct BlockingRule {
            started: Arc<AtomicUsize>,
            barrier: Arc<Barrier>,
        }

        impl InterceptRule for BlockingRule {
            fn rule_id(&self) -> &'static str {
                "blocking-rule"
            }

            fn needs_content(&self) -> bool {
                true
            }

            fn evaluate(&self, _input: &RuleInput<'_>) -> RuleDecision {
                RuleDecision::Allow
            }

            fn diagnostics_with_limit(
                &self,
                _input: &RuleInput<'_>,
                _mode: &Mode,
                _limit: usize,
            ) -> Vec<Diagnostic> {
                self.started.fetch_add(1, Ordering::SeqCst);
                self.barrier.wait();
                Vec::new()
            }
        }

        let started = Arc::new(AtomicUsize::new(0));
        let barrier = Arc::new(Barrier::new(MAX_CONCURRENT_SCAN_BUFFERS + 1));
        let registry = RuleRegistry::with_rules(vec![Box::new(BlockingRule {
            started: Arc::clone(&started),
            barrier: Arc::clone(&barrier),
        })])
        .expect("unique rule");
        let service = ScanBufferService::new(EnforcementPipeline::new(registry));

        let first_service = service.clone();
        let first = tokio::spawn(async move { first_service.scan_buffer(secret_request()).await });
        let second_service = service.clone();
        let second =
            tokio::spawn(async move { second_service.scan_buffer(secret_request()).await });

        for _ in 0..50 {
            if started.load(Ordering::SeqCst) == MAX_CONCURRENT_SCAN_BUFFERS {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        assert_eq!(started.load(Ordering::SeqCst), MAX_CONCURRENT_SCAN_BUFFERS);

        let err = service
            .scan_buffer(secret_request())
            .await
            .expect_err("third concurrent scan should fail fast");
        assert!(matches!(err, ScanBufferError::Busy));

        barrier.wait();
        first.await.expect("first task").expect("first scan");
        second.await.expect("second task").expect("second scan");
    }

    /// INTD-011: the service records `validation.service` durations
    /// for `mode = midEdit` calls. Pre-write calls do NOT contribute
    /// to the rollup — pre-write is a separate budget class per
    /// ADR-031, and mixing the two would muddy the demo trust signal.
    #[tokio::test]
    async fn scan_buffer_records_mid_edit_latency_into_aggregator() {
        let service = ScanBufferService::new(EnforcementPipeline::default());
        // Mid-edit call -> recorded.
        service
            .scan_buffer(secret_request())
            .await
            .expect("scan_buffer ok");
        let snapshot = service.latency().snapshot(Instant::now());
        let snapshot = snapshot.expect("mid-edit call must record at least one sample");
        assert_eq!(snapshot.sample_count, 1);
        assert!(snapshot.p50_ms >= 0.0);
        assert!(snapshot.p95_ms >= 0.0);
    }

    #[tokio::test]
    async fn scan_buffer_timeout_does_not_poison_latency_aggregator() {
        // A timed-out call MUST NOT contribute to the rollup — the
        // recorded duration would be the worker's straggler timing,
        // which is unbounded above and would skew p95 wildly. The
        // recording lives on the success branch of the await, so a
        // TimedOut return path skips the record() call.
        struct SleepingRule;

        impl InterceptRule for SleepingRule {
            fn rule_id(&self) -> &'static str {
                "sleeping-rule"
            }

            fn needs_content(&self) -> bool {
                true
            }

            fn evaluate(&self, _input: &RuleInput<'_>) -> RuleDecision {
                RuleDecision::Allow
            }

            fn diagnostics_with_limit(
                &self,
                _input: &RuleInput<'_>,
                _mode: &Mode,
                _limit: usize,
            ) -> Vec<Diagnostic> {
                std::thread::sleep(std::time::Duration::from_millis(300));
                Vec::new()
            }
        }

        let registry = RuleRegistry::with_rules(vec![Box::new(SleepingRule)]).expect("unique rule");
        let service = ScanBufferService::with_timeout(
            EnforcementPipeline::new(registry),
            std::time::Duration::from_millis(20),
        );
        let outcome = service.scan_buffer(secret_request()).await;
        assert!(matches!(outcome, Err(ScanBufferError::TimedOut)));
        // Aggregator must remain empty.
        assert!(
            service.latency().snapshot(Instant::now()).is_none(),
            "TimedOut path must not record into the aggregator",
        );
    }

    #[tokio::test]
    async fn scan_buffer_does_not_record_pre_write_latency_into_mid_edit_rollup() {
        let service = ScanBufferService::new(EnforcementPipeline::default());
        let mut request = secret_request();
        request.mode = ScanBufferMode::PreWrite;
        service.scan_buffer(request).await.expect("scan_buffer ok");
        // Pre-write samples MUST NOT contribute to the mid-edit
        // rollup — the aggregator is mode-scoped.
        assert!(
            service.latency().snapshot(Instant::now()).is_none(),
            "pre-write must not pollute the mid-edit rollup",
        );
    }

    /// Capacity stays with the worker for its full lifetime. A
    /// `TimedOut` return therefore does NOT free a concurrency slot
    /// while the straggler is still evaluating — subsequent
    /// admissions see `Busy` until the worker completes. This stops
    /// repeated timeouts from spawning unbounded OS threads.
    #[tokio::test]
    async fn scan_buffer_holds_capacity_for_timed_out_straggler_workers() {
        use std::sync::atomic::AtomicBool;

        struct GateRule {
            started: Arc<AtomicUsize>,
            released: Arc<AtomicBool>,
        }

        impl InterceptRule for GateRule {
            fn rule_id(&self) -> &'static str {
                "gate-straggler-rule"
            }

            fn needs_content(&self) -> bool {
                true
            }

            fn evaluate(&self, _input: &RuleInput<'_>) -> RuleDecision {
                RuleDecision::Allow
            }

            fn diagnostics_with_limit(
                &self,
                _input: &RuleInput<'_>,
                _mode: &Mode,
                _limit: usize,
            ) -> Vec<Diagnostic> {
                self.started.fetch_add(1, Ordering::SeqCst);
                // One-shot release flag (not a reusable Barrier): once
                // released, later admissions proceed immediately so the
                // recovery assertion can succeed without re-parking.
                while !self.released.load(Ordering::SeqCst) {
                    std::thread::sleep(std::time::Duration::from_millis(5));
                }
                Vec::new()
            }
        }

        let started = Arc::new(AtomicUsize::new(0));
        let released = Arc::new(AtomicBool::new(false));
        let registry = RuleRegistry::with_rules(vec![Box::new(GateRule {
            started: Arc::clone(&started),
            released: Arc::clone(&released),
        })])
        .expect("unique rule");
        let service = ScanBufferService::with_timeout(
            EnforcementPipeline::new(registry),
            std::time::Duration::from_millis(50),
        );

        // Saturate every slot with calls that will time out while the
        // worker remains parked behind the release flag.
        let mut tasks = Vec::with_capacity(MAX_CONCURRENT_SCAN_BUFFERS);
        for _ in 0..MAX_CONCURRENT_SCAN_BUFFERS {
            let svc = service.clone();
            tasks.push(tokio::spawn(async move {
                svc.scan_buffer(secret_request()).await
            }));
        }
        for _ in 0..100 {
            if started.load(Ordering::SeqCst) == MAX_CONCURRENT_SCAN_BUFFERS {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        assert_eq!(
            started.load(Ordering::SeqCst),
            MAX_CONCURRENT_SCAN_BUFFERS,
            "expected {MAX_CONCURRENT_SCAN_BUFFERS} workers to enter the rule before timeout",
        );

        for task in tasks {
            let outcome = task.await.expect("join");
            assert!(
                matches!(outcome, Err(ScanBufferError::TimedOut)),
                "expected TimedOut, got {outcome:?}",
            );
        }

        // While stragglers still hold capacity, further requests must
        // fail fast as Busy and must NOT spawn additional workers.
        let workers_after_timeout = started.load(Ordering::SeqCst);
        for _ in 0..(MAX_CONCURRENT_SCAN_BUFFERS * 3) {
            let outcome = service.scan_buffer(secret_request()).await;
            assert!(
                matches!(outcome, Err(ScanBufferError::Busy)),
                "straggler workers must keep capacity occupied; got {outcome:?}",
            );
        }
        assert_eq!(
            started.load(Ordering::SeqCst),
            workers_after_timeout,
            "repeated timed-out admissions must not spawn extra workers",
        );
        assert_eq!(
            service.in_flight(),
            MAX_CONCURRENT_SCAN_BUFFERS,
            "in_flight must track live straggler workers after TimedOut",
        );

        // Release stragglers; capacity recovers for a fresh admission.
        released.store(true, Ordering::SeqCst);
        for _ in 0..100 {
            if service.in_flight() == 0 {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        assert_eq!(
            service.in_flight(),
            0,
            "in_flight must clear once straggler workers finish",
        );
        service
            .scan_buffer(secret_request())
            .await
            .expect("capacity recovers after stragglers finish");
    }

    /// MLP2-002: a pinned `rules_sha` survives the round-trip through
    /// the worker thread and is returned to the caller verbatim.
    #[tokio::test]
    async fn scan_buffer_with_pin_returns_pinned_rules_sha() {
        let service = ScanBufferService::new(EnforcementPipeline::new(default_rule_registry()));
        let response = service
            .scan_buffer_with_pin(secret_request(), Some("sha-pinned-v1".into()))
            .await
            .expect("scan_buffer with pin");
        assert_eq!(response.rules_sha.as_deref(), Some("sha-pinned-v1"));
    }

    /// MLP2-002: the default `scan_buffer` (no pin) leaves
    /// `rules_sha` empty so existing call sites stay byte-compatible.
    #[tokio::test]
    async fn scan_buffer_without_pin_omits_rules_sha() {
        let service = ScanBufferService::new(EnforcementPipeline::new(default_rule_registry()));
        let response = service
            .scan_buffer(secret_request())
            .await
            .expect("scan_buffer");
        assert!(response.rules_sha.is_none());
    }

    /// MLP2-002: in-flight counter reflects evaluations holding a
    /// permit. Zero before, one during, zero after.
    #[tokio::test]
    async fn scan_buffer_in_flight_counter_tracks_active_evaluations() {
        // A rule that blocks on a barrier so the test can observe the
        // counter mid-evaluation. Reaches the worker thread because
        // `needs_content` is true; releases when the test calls
        // `barrier.wait()` on the controlling side.
        struct GateRule(Arc<Barrier>);
        impl InterceptRule for GateRule {
            fn rule_id(&self) -> &'static str {
                "test.gate"
            }
            fn needs_content(&self) -> bool {
                true
            }
            fn evaluate(&self, _input: &RuleInput<'_>) -> RuleDecision {
                self.0.wait();
                RuleDecision::allow()
            }
        }

        let barrier = Arc::new(Barrier::new(2));
        let registry = RuleRegistry::with_rules(vec![Box::new(GateRule(Arc::clone(&barrier)))])
            .expect("registry");
        let service = ScanBufferService::new(EnforcementPipeline::new(registry));
        assert_eq!(service.in_flight(), 0);

        let service_clone = service.clone();
        let handle = tokio::spawn(async move { service_clone.scan_buffer(secret_request()).await });

        // Spin until we observe the counter increment — the worker
        // thread may not have started before the await returns, so
        // poll without sleeping.
        let mut observed = 0;
        for _ in 0..200 {
            observed = service.in_flight();
            if observed == 1 {
                break;
            }
            tokio::task::yield_now().await;
        }
        assert_eq!(observed, 1, "in-flight must observe the held permit");

        // Release the barrier so the worker exits.
        barrier.wait();
        let _ = handle.await.expect("join");
        assert_eq!(service.in_flight(), 0, "counter must clear after exit");
    }

    /// MLP2-002 adversarial test (Council 2026-05-14 #C-001 / #C-029 /
    /// #C-036): a config-cache invalidation that fires **while a
    /// `scan_buffer` worker is mid-evaluation** must NOT change the
    /// pinned `rules_sha` the caller receives.
    ///
    /// The earlier version of this test captured the pin into a local
    /// `String` before spawning the invalidator, so the test passed
    /// even if the implementation re-read from the cache mid-call.
    /// This version uses two `Barrier`s — `arrived` to wait until the
    /// worker thread is provably inside `evaluate`, and `release` to
    /// resume the worker after the cache has been invalidated. No
    /// polling, no `yield_now` — deterministic on any executor + any
    /// load level (Council CI fix 2026-05-14: the earlier
    /// `in_flight()`-poll version was racy under CI scheduling).
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn config_invalidation_while_worker_running_does_not_swap_pinned_rules_sha() {
        use crate::rule_cache::{ResolvedRuleSet, RuleSetCache, RuleSetEntry, WorktreeKey};

        struct GateRule {
            arrived: Arc<Barrier>,
            release: Arc<Barrier>,
        }
        impl InterceptRule for GateRule {
            fn rule_id(&self) -> &'static str {
                "test.pin-gate"
            }
            fn needs_content(&self) -> bool {
                true
            }
            fn evaluate(&self, _input: &RuleInput<'_>) -> RuleDecision {
                // Signal "I'm in evaluate()" then park until the test
                // releases. Both barriers are 2-party so the worker
                // thread proceeds only when the test side calls
                // `wait` on the same barrier.
                self.arrived.wait();
                self.release.wait();
                RuleDecision::allow()
            }
        }

        let cache = Arc::new(RuleSetCache::new());
        let dir = tempfile::tempdir().expect("tempdir");
        let key = WorktreeKey::canonicalise(dir.path()).unwrap();
        cache
            .get_or_resolve::<_, ()>(&key, |_| {
                Ok(RuleSetEntry {
                    rules_sha: "v-original".into(),
                    resolved: ResolvedRuleSet {
                        config: serde_json::json!({"mode":"warn"}),
                    },
                })
            })
            .unwrap();

        let pinned = match cache.lookup(&key) {
            crate::rule_cache::CacheOutcome::Hit(entry) => Some(entry.rules_sha),
            crate::rule_cache::CacheOutcome::Miss => None,
        };
        assert_eq!(pinned.as_deref(), Some("v-original"));

        let arrived = Arc::new(Barrier::new(2));
        let release = Arc::new(Barrier::new(2));
        let registry = RuleRegistry::with_rules(vec![Box::new(GateRule {
            arrived: Arc::clone(&arrived),
            release: Arc::clone(&release),
        })])
        .expect("registry");
        let service = ScanBufferService::new(EnforcementPipeline::new(registry));

        let service_clone = service.clone();
        let pin_for_call = pinned.clone();
        let handle = tokio::spawn(async move {
            service_clone
                .scan_buffer_with_pin(secret_request(), pin_for_call)
                .await
        });

        // Block (on a blocking-friendly task) until the worker is
        // provably in `evaluate`. `spawn_blocking` keeps the tokio
        // reactor responsive while the std Barrier parks the calling
        // OS thread.
        let arrived_for_wait = Arc::clone(&arrived);
        tokio::task::spawn_blocking(move || arrived_for_wait.wait())
            .await
            .expect("arrived join");

        // Belt-and-braces: the in-flight counter must read 1 too.
        // This is now guaranteed (not racy) because the worker has
        // already passed the permit-acquire + InFlightGuard::new
        // happens-before the barrier wait it just released.
        assert_eq!(
            service.in_flight(),
            1,
            "worker must be in flight at the gate"
        );

        // Worker is parked inside `GateRule`. Invalidate the cache —
        // if the implementation re-read from the cache here, the
        // response would carry a different (or missing) `rules_sha`.
        assert!(
            cache.invalidate(&key),
            "cache entry must be present before invalidation"
        );
        assert!(cache.is_empty(), "cache must be empty post-invalidation");

        // Release the worker.
        let release_for_wait = Arc::clone(&release);
        tokio::task::spawn_blocking(move || release_for_wait.wait())
            .await
            .expect("release join");

        let response = handle.await.expect("join").expect("scan_buffer_with_pin");

        assert_eq!(
            response.rules_sha.as_deref(),
            Some("v-original"),
            "in-flight pin must survive a mid-evaluation cache invalidation"
        );
        assert_eq!(service.in_flight(), 0, "counter must clear after exit");
    }

    /// MLP2-002: `in_flight()` tracks live workers. Immediately after
    /// a `TimedOut` return the straggler still holds the slot; the
    /// counter clears only when the worker finishes.
    #[tokio::test]
    async fn scan_buffer_in_flight_clears_after_straggler_worker_finishes() {
        struct SlowRule;
        impl InterceptRule for SlowRule {
            fn rule_id(&self) -> &'static str {
                "test.slow"
            }
            fn needs_content(&self) -> bool {
                true
            }
            fn evaluate(&self, _input: &RuleInput<'_>) -> RuleDecision {
                std::thread::sleep(Duration::from_millis(150));
                RuleDecision::allow()
            }
        }
        let registry = RuleRegistry::with_rules(vec![Box::new(SlowRule)]).expect("registry");
        let service = ScanBufferService::with_timeout(
            EnforcementPipeline::new(registry),
            Duration::from_millis(10),
        );
        let outcome = service.scan_buffer(secret_request()).await;
        assert!(
            matches!(outcome, Err(ScanBufferError::TimedOut)),
            "expected TimedOut, got {outcome:?}"
        );
        assert!(
            service.in_flight() > 0,
            "straggler worker must still hold in_flight after TimedOut"
        );
        for _ in 0..50 {
            if service.in_flight() == 0 {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        assert_eq!(
            service.in_flight(),
            0,
            "InFlightGuard must release once the straggler worker finishes"
        );
    }
}
