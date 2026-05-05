use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use anvil_intercept_rules::ChangeKind;
use anvil_kernel_types::diagnostics::KnownMode;
use anvil_kernel_types::{Diagnostic, Mode};
use serde::Serialize;
use thiserror::Error;
use tokio::sync::{Semaphore, TryAcquireError};

use crate::enforcement::{CONTENT_SIZE_CAP_BYTES_USIZE, EnforcementPipeline, ProposedChange};

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
    #[error("unsupported scan_buffer mode; supported modes: midEdit, preWrite")]
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
        }
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
        let pipeline = Arc::clone(&self.pipeline);
        let (sender, receiver) = tokio::sync::oneshot::channel();
        std::thread::Builder::new()
            .name("anvil-scan-buffer".to_owned())
            .spawn(move || {
                let _ = sender.send(scan_buffer_with_pipeline(&request, &pipeline));
            })
            .map_err(|err| ScanBufferError::WorkerFailed(err.to_string()))?;

        match tokio::time::timeout(self.timeout, receiver).await {
            Ok(Ok(result)) => result,
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
