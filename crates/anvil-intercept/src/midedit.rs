use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use anvil_intercept_rules::ChangeKind;
use anvil_kernel_types::diagnostics::KnownMode;
use anvil_kernel_types::{Diagnostic, Mode};
use serde::Serialize;
use thiserror::Error;
use tokio::sync::{Semaphore, TryAcquireError};

use crate::enforcement::{CONTENT_SIZE_CAP_BYTES_USIZE, EnforcementPipeline, ProposedChange};
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
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ScanBufferResponse {
    pub version: u64,
    pub diagnostics: Vec<Diagnostic>,
    pub truncated: bool,
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
    /// need permit-release behaviour observable inside a few hundred
    /// milliseconds; production callers stick with [`Self::new`].
    #[must_use]
    pub fn with_timeout(pipeline: EnforcementPipeline, timeout: Duration) -> Self {
        Self {
            pipeline: Arc::new(pipeline),
            permits: Arc::new(Semaphore::new(MAX_CONCURRENT_SCAN_BUFFERS)),
            timeout,
            latency: LatencyAggregator::new(),
        }
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
        // The permit is held by the *caller* for the lifetime of this
        // future, NOT by the worker thread. If the caller times out,
        // the permit is released here on return so a runaway rule
        // cannot wedge the service into permanent `Busy`. The worker
        // thread keeps running until its pipeline call finishes (sync
        // rules cannot be cancelled mid-flight) and its result is
        // discarded once the caller has dropped the receiver.
        let _permit = self
            .permits
            .clone()
            .try_acquire_owned()
            .map_err(|err| match err {
                TryAcquireError::NoPermits => ScanBufferError::Busy,
                TryAcquireError::Closed => ScanBufferError::ServiceUnavailable,
            })?;
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
                result
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
        return Ok(ScanBufferResponse {
            version: request.version,
            diagnostics: Vec::new(),
            truncated: false,
        });
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
    use anvil_kernel_types::{Category, DiagnosticSource, Location, Severity};

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
        }
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
        };

        let pipeline = EnforcementPipeline::default();
        let err =
            scan_buffer_with_pipeline(&request, &pipeline).expect_err("over cap should reject");

        assert!(matches!(err, ScanBufferError::ContentTooLarge { .. }));
    }

    #[test]
    fn scan_buffer_short_circuits_binary_content() {
        let request = ScanBufferRequest {
            path: PathBuf::from("asset.bin"),
            text: "api_key='abcdEFGH1234567890'\0".to_string(),
            version: 2,
            mode: ScanBufferMode::MidEdit,
        };

        let pipeline = EnforcementPipeline::default();
        let response = scan_buffer_with_pipeline(&request, &pipeline)
            .expect("binary content is a clean short-circuit");

        assert!(response.diagnostics.is_empty());
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

    /// A runaway rule that outlives the caller's timeout MUST NOT
    /// keep its semaphore permit. The permit is held by the caller's
    /// future and dropped on return — including on `TimedOut` — so
    /// `scan_buffer` cannot be wedged into permanent `Busy` by stuck
    /// worker threads. The straggler thread keeps running until its
    /// pipeline call completes; only the permit is freed.
    #[tokio::test]
    async fn scan_buffer_releases_permit_when_caller_times_out() {
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
                std::thread::sleep(std::time::Duration::from_millis(500));
                Vec::new()
            }
        }

        let registry = RuleRegistry::with_rules(vec![Box::new(SleepingRule)]).expect("unique rule");
        let service = ScanBufferService::with_timeout(
            EnforcementPipeline::new(registry),
            std::time::Duration::from_millis(50),
        );

        // Saturate every permit with calls that will time out.
        let mut tasks = Vec::with_capacity(MAX_CONCURRENT_SCAN_BUFFERS);
        for _ in 0..MAX_CONCURRENT_SCAN_BUFFERS {
            let svc = service.clone();
            tasks.push(tokio::spawn(async move {
                svc.scan_buffer(secret_request()).await
            }));
        }
        for task in tasks {
            let outcome = task.await.expect("join");
            assert!(
                matches!(outcome, Err(ScanBufferError::TimedOut)),
                "expected TimedOut, got {outcome:?}",
            );
        }

        // After the timeouts fire, the permits must be available
        // again. The next scan must reach the worker thread (and
        // also time out), not be rejected as Busy.
        let outcome = service.scan_buffer(secret_request()).await;
        assert!(
            matches!(outcome, Err(ScanBufferError::TimedOut)),
            "permit must release on caller timeout; got {outcome:?}",
        );
    }
}
