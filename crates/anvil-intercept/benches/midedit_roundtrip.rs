//! RTAI-003: mid-edit `scan_buffer` latency benchmark.
//!
//! Measures the daemon-side cost of a single `scan_buffer` mid-edit RPC at
//! both ADR-031 boundaries:
//!
//! - `validation.service` — in-process call into the enforcement pipeline via
//!   [`scan_buffer_with_pipeline`]. This isolates the daemon's rule-evaluation
//!   work from the transport.
//! - `validation.roundtrip` — full Unix-socket round-trip from a synthetic
//!   driver to [`IpcListener`] running the same `ScanBufferService` the
//!   foreground daemon uses. The harness reuses a single persistent connection
//!   across iterations to mirror production drivers (cold-connect cost is
//!   documented in `ipc_roundtrip.rs` and not duplicated here).
//!
//! ### Fixture corpus (ADR-031 § Corpus and harness requirements)
//!
//! ADR-031 mandates the canonical `latency-corpus-v1` cover the dimensions
//! below. Each `BufferCase` entry pins one dimension and is exercised by both
//! criterion groups and the percentile sampler:
//!
//! | Case label                  | Dimension (ADR-031)           | Notes                                   |
//! | --------------------------- | ----------------------------- | --------------------------------------- |
//! | `empty`                     | empty content                 | pins the zero-byte fast path            |
//! | `small_1KiB`                | small representative content  | TypeScript-shaped lines                 |
//! | `medium_64KiB`              | medium representative content | TypeScript-shaped lines                 |
//! | `near_cap_1MiB_minus_1KiB`  | near-cap content              | exercises cap-adjacent slow path        |
//! | `binary_short_circuit`      | binary / binary-like content  | embedded `\0`; expected O(1) at the     |
//! |                             |                               | binary short-circuit in `midedit.rs`    |
//! | `unicode_heavy`             | Unicode-heavy content         | mixed CJK + emoji at ~32 KiB            |
//! | `dirty_secret_match`        | dirty diagnostic path         | embeds a fake AKIA key — exercises the  |
//! |                             |                               | secret-detection rule's full diagnostic |
//! |                             |                               | construction path                       |
//!
//! Two complementary harnesses live in this file:
//!
//! 1. A `criterion` benchmark group (`midedit_service` /
//!    `midedit_roundtrip`) that records per-iteration timings for
//!    regression tracking. This is what `cargo bench` consumes locally.
//!    The `midedit-baseline` job in `.github/workflows/bench.yml` runs the
//!    bench on push to `main` and feeds its percentile-sampler output to
//!    `scripts/check-midedit-baseline.sh` for regression gating.
//! 2. A manual percentile sampler that prints `p50` / `p95` / `p99` and the
//!    ADR-031 dimension labels alongside the criterion run. The percentile
//!    sampler mirrors `ipc_roundtrip.rs` so the round-trip SLO can be
//!    eyeballed without parsing criterion's HTML report. The numbers it
//!    prints are the authoritative SLO evidence; criterion's mean is
//!    reported separately for trend-watching.
//!
//! ### ADR-031 mid-edit interactive buffer SLO (warm daemon, p95)
//!
//! - `validation.service` p95 ≤ **50 ms**
//! - `validation.roundtrip` p95 ≤ **80 ms**
//!
//! ### Recorded baseline machine class
//!
//! The baseline numbers committed alongside this bench are calibrated for:
//!
//! - **Platform:** Linux `x86_64`
//! - **Date:** 2026-04-30
//! - **Daemon state:** warm
//! - **Rule set:** `default-v1` (the `EnforcementPipeline::default()` set)
//! - **Samples:** 500 per case (override via `ANVIL_MIDEDIT_BENCH_SAMPLES`)
//!
//! The recorded p50/p95/p99 per ADR-031 case live in
//! `crates/anvil-intercept/benches/baselines/midedit_roundtrip.json` and are
//! compared against by `scripts/check-midedit-baseline.sh` from the
//! `midedit-baseline` job in `.github/workflows/bench.yml`. The hard-fail
//! gate is the ADR-031 interactive-buffer SLO above; baseline drift past
//! ±15% is a soft warning. Future readers comparing numbers from a different
//! runner class should re-baseline rather than expect parity — the SLO gate
//! is runner-independent and remains the authoritative pass/fail criterion.
//! See `plans/decisions/031-validation-latency-rubric.md`.
//!
//! ### Re-baselining
//!
//! After an intentional latency change ships (or after the runner class
//! changes) re-record the baseline by running the bench locally:
//!
//! ```bash
//! cargo bench -p eddacraft-anvil-intercept --bench midedit_roundtrip \
//!     --features bench-internals 2>&1 | tee midedit-bench.txt
//! ```
//!
//! Copy the seven `validation.service` and seven `validation.roundtrip`
//! `pNN=Xms` rows from the `--- ADR-031 mid-edit warm percentile sampler ---`
//! block into `baselines/midedit_roundtrip.json`, bump
//! `calibration.date`, note the runner class in `calibration.runner`, and
//! commit alongside the change that justifies the new numbers.
//!
//! Run locally with:
//!
//! ```bash
//! cargo bench -p eddacraft-anvil-intercept --bench midedit_roundtrip \
//!     --features bench-internals
//! ```

#[cfg(unix)]
use std::hint::black_box;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
#[cfg(unix)]
use std::path::PathBuf;
#[cfg(unix)]
use std::sync::Arc;
#[cfg(unix)]
use std::sync::LazyLock;
#[cfg(unix)]
use std::time::{Duration, Instant};

#[cfg(unix)]
use anvil_checks::secret::patterns::DEFAULT_COMPILED_PATTERNS;
#[cfg(unix)]
use anvil_intercept::Shutdown;
#[cfg(unix)]
use anvil_intercept::dos::IpcLimits;
#[cfg(unix)]
use anvil_intercept::enforcement::{CONTENT_SIZE_CAP_BYTES_USIZE, EnforcementPipeline};
#[cfg(unix)]
use anvil_intercept::ipc::{IpcListener, NoopDispatcher};
#[cfg(unix)]
use anvil_intercept::midedit::{
    MAX_CONCURRENT_SCAN_BUFFERS, ScanBufferError, ScanBufferMode, ScanBufferRequest,
    ScanBufferService, scan_buffer_with_pipeline,
};
#[cfg(unix)]
use criterion::{Criterion, Throughput, criterion_group, criterion_main};
#[cfg(unix)]
use serde_json::json;
#[cfg(unix)]
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
#[cfg(unix)]
use tokio::net::UnixStream;

#[cfg(unix)]
const SMALL_BYTES: usize = 1024; // 1 KiB
#[cfg(unix)]
const MEDIUM_BYTES: usize = 64 * 1024; // 64 KiB
/// Near-cap fixture: 1 KiB shy of the 1 MiB content cap so the fixture itself
/// stays under the cap once wrapped in a JSON-RPC frame in the round-trip
/// harness, but still exercises the cap-adjacent slow path.
#[cfg(unix)]
const NEAR_CAP_BYTES: usize = CONTENT_SIZE_CAP_BYTES_USIZE - 1024;
/// Binary short-circuit fixture size — small enough to stay in the O(1) path
/// since `midedit::scan_buffer_with_pipeline` returns the moment it sees a
/// `\0` byte. The size is only relevant to the dimension label.
#[cfg(unix)]
const BINARY_BYTES: usize = 4 * 1024; // 4 KiB
/// Unicode-heavy fixture size (~32 KiB of mixed CJK + emoji).
#[cfg(unix)]
const UNICODE_BYTES: usize = 32 * 1024;

/// Default percentile sample count. Set high enough that p99 has at least
/// 5 tail samples (`500 * 0.01 = 5`) and p95 is not noisy. Override via
/// the `ANVIL_MIDEDIT_BENCH_SAMPLES` env var when CI wall-clock is tight.
#[cfg(unix)]
const DEFAULT_PERCENTILE_SAMPLES: usize = 500;

#[cfg(unix)]
fn percentile_sample_count() -> usize {
    std::env::var("ANVIL_MIDEDIT_BENCH_SAMPLES")
        .ok()
        .and_then(|raw| raw.parse::<usize>().ok())
        .filter(|n| *n >= 100)
        .unwrap_or(DEFAULT_PERCENTILE_SAMPLES)
}

/// Buffer-size cases shared across both criterion and percentile harnesses.
/// Each entry pins one ADR-031 corpus dimension; see the module-level table.
#[cfg(unix)]
struct BufferCase {
    label: &'static str,
    bytes: usize,
    /// Builder for the fixture body. Held as a function pointer so a single
    /// `BufferCase` slice can mix the TypeScript, binary, Unicode, and
    /// dirty-path generators without runtime branching at call sites.
    build: fn(usize) -> String,
}

#[cfg(unix)]
const CASES: &[BufferCase] = &[
    // ADR-031 dimension: empty content. Pins the zero-byte fast path.
    BufferCase {
        label: "empty",
        bytes: 0,
        build: make_typescript_fixture,
    },
    // ADR-031 dimension: small representative content.
    BufferCase {
        label: "small_1KiB",
        bytes: SMALL_BYTES,
        build: make_typescript_fixture,
    },
    // ADR-031 dimension: medium representative content.
    BufferCase {
        label: "medium_64KiB",
        bytes: MEDIUM_BYTES,
        build: make_typescript_fixture,
    },
    // ADR-031 dimension: near-cap content.
    BufferCase {
        label: "near_cap_1MiB_minus_1KiB",
        bytes: NEAR_CAP_BYTES,
        build: make_typescript_fixture,
    },
    // ADR-031 dimension: binary / binary-like content. Exercises the
    // `content.contains(&0)` short-circuit at `midedit.rs` line 169 and is
    // expected to be O(1) regardless of `bytes`.
    BufferCase {
        label: "binary_short_circuit",
        bytes: BINARY_BYTES,
        build: make_binary_fixture,
    },
    // ADR-031 dimension: Unicode-heavy content (CJK + emoji mix).
    BufferCase {
        label: "unicode_heavy",
        bytes: UNICODE_BYTES,
        build: make_unicode_fixture,
    },
    // Dirty-path SLO: this fixture embeds a fake AWS access-key ID
    // (`AKIA` + 16 alphanumerics) so the secret-detection rule produces a
    // diagnostic. Pins the dirty-path SLO alongside the clean-path SLO and
    // exercises the diagnostic-emission path that the other cases skip.
    BufferCase {
        label: "dirty_secret_match",
        bytes: MEDIUM_BYTES,
        build: make_dirty_secret_fixture,
    },
];

/// Build a representative TypeScript-shaped fixture of the requested byte
/// length. The content is deliberately rule-clean so we measure the steady
/// scanning cost rather than diagnostic emission cost; mid-edit is dominated
/// by the warm-path scan, not by diagnostic serialisation.
// Roughly 80-byte lines keeps the fixture realistic and avoids degenerate
// "single huge line" cases that distort scanner work.
#[cfg(unix)]
const TYPESCRIPT_LINE: &str =
    "const value: number = compute(left, right) + adjust(seed); // representative line\n";

#[cfg(unix)]
fn make_typescript_fixture(bytes: usize) -> String {
    if bytes == 0 {
        return String::new();
    }
    let mut out = String::with_capacity(bytes + TYPESCRIPT_LINE.len());
    while out.len() < bytes {
        out.push_str(TYPESCRIPT_LINE);
    }
    out.truncate(bytes);
    out
}

/// Build a binary-like fixture with embedded `\0` bytes. The first NUL is
/// near the start so the binary short-circuit triggers immediately.
#[cfg(unix)]
fn make_binary_fixture(bytes: usize) -> String {
    // Start with a short ASCII prefix, then a NUL, then non-NUL bytes. Using
    // valid UTF-8 here (NUL is U+0000) keeps `String` happy.
    let mut out = String::with_capacity(bytes);
    let prefix = "header-bytes:";
    out.push_str(prefix);
    out.push('\0');
    while out.len() < bytes {
        out.push('a');
    }
    out.truncate(bytes);
    out
}

/// Each `UNICODE_LINE` is ~80 bytes of UTF-8 (CJK is 3 bytes/char, the emoji
/// is 4 bytes). The exact width is not load-bearing; the mix is.
#[cfg(unix)]
const UNICODE_LINE: &str = "// 日本語のコメント — 测试用例 — 한국어 주석 — \u{1F680} payload\n";

/// `AKIA` prefix + 16 uppercase alphanumerics. `EXAMPLEKEYBENCH0` is exactly
/// 16 chars and matches the secret-detection regex character class. Embedded
/// in a `const` here for clippy compliance and to keep the fake at module
/// scope so it is unambiguously a fixture and not a real credential.
#[cfg(unix)]
const FAKE_AWS_KEY_LINE: &str = "const AWS_KEY = \"AKIAEXAMPLEKEYBENCH0\";\n";

/// Build a Unicode-heavy fixture mixing CJK ideographs and emoji. The base
/// line is multi-byte throughout so byte-length and char-length diverge,
/// exercising any UTF-8 boundary handling in the scan path.
#[cfg(unix)]
fn make_unicode_fixture(bytes: usize) -> String {
    let mut out = String::with_capacity(bytes + UNICODE_LINE.len());
    while out.len() < bytes {
        out.push_str(UNICODE_LINE);
    }
    // Truncate on a UTF-8 boundary so we never produce invalid Unicode.
    while !out.is_char_boundary(bytes.min(out.len())) {
        out.pop();
    }
    out.truncate(bytes.min(out.len()));
    out
}

/// Build a dirty fixture embedding a fake AWS access-key ID in otherwise
/// clean TypeScript content. The key matches the `AKIA[0-9A-Z]{16}` pattern
/// so it triggers the secret-detection rule's full diagnostic-construction
/// path. The 20-char fake is not a real credential.
#[cfg(unix)]
fn make_dirty_secret_fixture(bytes: usize) -> String {
    let mut out = make_typescript_fixture(bytes);
    if out.len() >= FAKE_AWS_KEY_LINE.len() {
        // Overwrite a slice in the middle of the fixture so the bytes count
        // is preserved and the match is not at offset zero.
        let mid = out.len() / 2;
        // Snap to a UTF-8 boundary; `make_typescript_fixture` is ASCII so
        // this is always already a boundary, but we stay defensive.
        let start = (0..=mid)
            .rev()
            .find(|i| out.is_char_boundary(*i))
            .unwrap_or(0);
        let end = start + FAKE_AWS_KEY_LINE.len();
        if end <= out.len() {
            out.replace_range(start..end, FAKE_AWS_KEY_LINE);
        }
    } else {
        out = FAKE_AWS_KEY_LINE.to_string();
    }
    out
}

#[cfg(unix)]
fn make_request(path: &str, text: String) -> ScanBufferRequest {
    ScanBufferRequest {
        path: PathBuf::from(path),
        text,
        version: 1,
        mode: ScanBufferMode::MidEdit,
        env_agent_tag: None,
    }
}

#[cfg(unix)]
fn print_dimensions(boundary: &str, case: &BufferCase) {
    // ADR-031 required dimensions for every recorded measurement. See
    // `plans/decisions/031-validation-latency-rubric.md` § Required dimensions.
    println!(
        "dimensions: mode=midEdit boundary={boundary} surface=cli-harness \
         contentSource=buffer ruleSet=default-v1 fixtureCorpus=latency-corpus-v1 \
         contentSize={size} platform={platform} daemonState=warm \
         driverProtocol=json-rpc-2.0 debounceMs=0 case={case}",
        size = case.bytes,
        platform = std::env::consts::OS,
        case = case.label,
    );
}

#[cfg(unix)]
fn percentile_index(len: usize, percentile: usize) -> usize {
    ((len.saturating_sub(1)) * percentile) / 100
}

#[cfg(unix)]
fn fmt_ms(duration: Duration) -> String {
    format!("{:.3}ms", duration.as_secs_f64() * 1_000.0)
}

#[cfg(unix)]
fn report_percentiles(boundary: &str, case: &BufferCase, samples: &mut [Duration]) {
    samples.sort_unstable();
    println!(
        "{boundary} {case}: samples={n} p50={p50} p95={p95} p99={p99}",
        case = case.label,
        n = samples.len(),
        p50 = fmt_ms(samples[percentile_index(samples.len(), 50)]),
        p95 = fmt_ms(samples[percentile_index(samples.len(), 95)]),
        p99 = fmt_ms(samples[percentile_index(samples.len(), 99)]),
    );
}

/// Force-initialise expensive lazies and warm the rule pipeline before the
/// criterion harness starts measuring. Without this the first sample of the
/// first case eats the `DEFAULT_COMPILED_PATTERNS` regex compile and the
/// per-rule warm-up.
#[cfg(unix)]
fn warm_up(pipeline: &EnforcementPipeline) {
    LazyLock::force(&DEFAULT_COMPILED_PATTERNS);
    // One service-side scan per case shape so every code path is JIT-warm.
    for case in CASES {
        let request = make_request("src/realtime/buffer.ts", (case.build)(case.bytes));
        let _ = scan_buffer_with_pipeline(&request, pipeline).expect("warm-up scan_buffer");
    }
}

// ---------------------------------------------------------------------------
// Criterion harness
// ---------------------------------------------------------------------------

#[cfg(unix)]
fn bench_validation_service(c: &mut Criterion) {
    let pipeline = EnforcementPipeline::default();
    warm_up(&pipeline);

    let mut group = c.benchmark_group("midedit_service");
    group.measurement_time(Duration::from_secs(5));
    group.sample_size(60);

    for case in CASES {
        let request = make_request("src/realtime/buffer.ts", (case.build)(case.bytes));
        // Throughput is in fixture bytes; criterion plots per-byte scan cost.
        // The binary short-circuit case will look misleadingly fast on this
        // axis — that is the point.
        if case.bytes > 0 {
            group.throughput(Throughput::Bytes(case.bytes as u64));
        }
        group.bench_function(case.label, |b| {
            b.iter(|| {
                let response = scan_buffer_with_pipeline(black_box(&request), black_box(&pipeline))
                    .expect("scan_buffer_with_pipeline");
                black_box(response);
            });
        });
    }

    group.finish();
}

#[cfg(unix)]
fn bench_validation_roundtrip(c: &mut Criterion) {
    // Multi-thread runtime mirrors production: the foreground daemon runs on
    // a multi-thread tokio runtime, and the listener accept loop + scan
    // workers benefit from a real worker pool. 2 worker threads matches
    // `MAX_CONCURRENT_SCAN_BUFFERS`, which is the daemon's concurrency cap
    // for `scan_buffer`. Using `new_current_thread` would serialise accept
    // and worker on a single OS thread and under-report production latency.
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(MAX_CONCURRENT_SCAN_BUFFERS)
        .enable_all()
        .build()
        .expect("tokio runtime");

    // Warm the rule pipeline before timing anything. The harness's own scan
    // service uses an independent pipeline, but the LazyLocks are global.
    warm_up(&EnforcementPipeline::default());

    let mut group = c.benchmark_group("midedit_roundtrip");
    group.measurement_time(Duration::from_secs(8));
    group.sample_size(30);

    for case in CASES {
        // Stand up a fresh daemon per case so the listener's `scan_buffer`
        // semaphore and any criterion warm-up cost are isolated. The client
        // connection is opened once per case and reused across iterations
        // to mirror production drivers (cold-connect cost is documented in
        // `ipc_roundtrip.rs` and not duplicated here).
        let harness = runtime.block_on(async { RoundtripHarness::start() });
        let frame = build_scan_buffer_frame(case);
        let mut client = runtime.block_on(async { harness.connect().await });

        // Warm-up RPC so the per-case first-iteration cost is amortised.
        runtime.block_on(async { harness.run_one(&mut client, &frame).await });

        if case.bytes > 0 {
            group.throughput(Throughput::Bytes(case.bytes as u64));
        }
        group.bench_function(case.label, |b| {
            b.iter(|| {
                runtime.block_on(harness.run_one(&mut client, &frame));
            });
        });

        runtime.block_on(harness.shutdown());
    }

    group.finish();
}

#[cfg(unix)]
fn bench_daemon_burst(c: &mut Criterion) {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(MAX_CONCURRENT_SCAN_BUFFERS)
        .enable_all()
        .build()
        .expect("tokio runtime");
    warm_up(&EnforcementPipeline::default());

    let mut group = c.benchmark_group("daemon_burst");
    group.measurement_time(Duration::from_secs(5));
    group.sample_size(30);

    for &burst_size in &[MAX_CONCURRENT_SCAN_BUFFERS, 8, 32] {
        let service = ScanBufferService::default();
        let request = make_request(
            "src/realtime/burst.ts",
            make_typescript_fixture(SMALL_BYTES),
        );

        group.throughput(Throughput::Elements(burst_size as u64));
        group.bench_function(format!("scan_buffer_burst_{burst_size}"), |b| {
            b.iter(|| {
                runtime.block_on(async {
                    let mut handles = Vec::with_capacity(burst_size);
                    for _ in 0..burst_size {
                        let service = service.clone();
                        let request = request.clone();
                        handles.push(tokio::spawn(
                            async move { service.scan_buffer(request).await },
                        ));
                    }

                    let mut accepted = 0usize;
                    let mut busy = 0usize;
                    for handle in handles {
                        match handle.await.expect("burst task joins") {
                            Ok(_) => accepted += 1,
                            Err(ScanBufferError::Busy) => busy += 1,
                            Err(_) => panic!("unexpected burst error"),
                        }
                    }
                    black_box((accepted, busy));
                });
            });
        });
    }

    group.finish();
}

#[cfg(unix)]
fn build_scan_buffer_frame(case: &BufferCase) -> String {
    let request = json!({
        "jsonrpc": "2.0",
        "method": "scan_buffer",
        "params": {
            "path": "src/realtime/buffer.ts",
            "text": (case.build)(case.bytes),
            "version": 1,
            "mode": "midEdit",
        },
        "id": "midedit-bench",
    });
    let mut serialised = serde_json::to_string(&request).expect("serialise scan_buffer frame");
    serialised.push('\n');
    serialised
}

#[cfg(unix)]
struct RoundtripHarness {
    socket: PathBuf,
    _tmp: tempfile::TempDir,
    shutdown: Shutdown,
    handle: Option<tokio::task::JoinHandle<Result<(), anvil_intercept::ipc::IpcError>>>,
}

#[cfg(unix)]
impl RoundtripHarness {
    /// Must be called from inside a tokio runtime context so the listener
    /// task can be spawned. Both criterion and the percentile sampler call
    /// this via `runtime.block_on(...)`.
    fn start() -> Self {
        let tmp = tempfile::tempdir().expect("tempdir");
        std::fs::set_permissions(tmp.path(), std::fs::Permissions::from_mode(0o700))
            .expect("secure tempdir permissions");
        let socket = tmp.path().join("intercept.sock");
        let scan_buffer = ScanBufferService::default();
        // INTD-016 IPC DoS budgets (100 req/s sustained, 1000 burst)
        // are sized for production drivers, not for the tight loop a
        // criterion bench drives over a single persistent connection.
        // The bench runs thousands of iterations per sample; the
        // production budget trips with `-32005 Server busy`, which the
        // response assertion in `run_one` (below) panics on. Lift the
        // per-connection budget to effectively unbounded so the bench
        // measures the path it cares about (rule evaluation + transport)
        // instead of measuring rate-limit error frames. Production
        // daemon continues using the default budget.
        let bench_limits = IpcLimits {
            rps_sustained: f64::MAX,
            rps_burst: u32::MAX,
            ..IpcLimits::default()
        };
        let listener =
            IpcListener::bind_with_scan_buffer_service(&socket, NoopDispatcher, scan_buffer)
                .expect("bind listener")
                .with_limits(bench_limits);
        let (shutdown, token) = Shutdown::new();
        let handle = tokio::spawn(async move { listener.serve(token).await });
        Self {
            socket,
            _tmp: tmp,
            shutdown,
            handle: Some(handle),
        }
    }

    /// Open a single persistent connection that the caller will reuse across
    /// iterations. Production drivers hold a long-lived connection; baking
    /// connect cost into every p95 number would over-report production
    /// latency.
    async fn connect(&self) -> BufReader<UnixStream> {
        let stream = UnixStream::connect(&self.socket)
            .await
            .expect("connect client");
        BufReader::new(stream)
    }

    async fn run_one(&self, client: &mut BufReader<UnixStream>, frame: &str) {
        client
            .get_mut()
            .write_all(frame.as_bytes())
            .await
            .expect("write request");
        let mut response = String::new();
        client
            .read_line(&mut response)
            .await
            .expect("read response");
        // `assert!` (not `debug_assert!`) — criterion compiles in release and
        // we want harness-validation failures to show up loudly.
        assert!(
            response.contains("\"result\""),
            "unexpected scan_buffer response: {response}",
        );
    }

    async fn shutdown(mut self) {
        self.shutdown.trigger();
        if let Some(handle) = self.handle.take() {
            tokio::time::timeout(Duration::from_secs(2), handle)
                .await
                .expect("listener shutdown timeout")
                .expect("listener join")
                .expect("listener ok");
        }
    }
}

// ---------------------------------------------------------------------------
// Percentile sampler
// ---------------------------------------------------------------------------

/// Standalone percentile sampler — runs once per criterion invocation as the
/// final benchmark group and prints ADR-031-formatted dimension lines plus
/// p50/p95/p99 for both boundaries. Criterion's own output focuses on
/// mean/median; this gives us the SLO numbers directly.
#[cfg(unix)]
fn bench_percentile_sampler(_c: &mut Criterion) {
    let pipeline = Arc::new(EnforcementPipeline::default());
    warm_up(&pipeline);
    let samples_target = percentile_sample_count();

    println!();
    println!("--- ADR-031 mid-edit warm percentile sampler ---");
    println!(
        "interactive buffer SLO (p95): validation.service ≤ 50ms, validation.roundtrip ≤ 80ms"
    );
    println!("samples per case: {samples_target} (override via ANVIL_MIDEDIT_BENCH_SAMPLES)");

    for case in CASES {
        let request = make_request("src/realtime/buffer.ts", (case.build)(case.bytes));

        let mut service_samples = Vec::with_capacity(samples_target);
        for _ in 0..samples_target {
            let started = Instant::now();
            let response =
                scan_buffer_with_pipeline(&request, &pipeline).expect("scan_buffer_with_pipeline");
            service_samples.push(started.elapsed());
            black_box(response);
        }
        print_dimensions("validation.service", case);
        report_percentiles("validation.service", case, &mut service_samples);
    }

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(MAX_CONCURRENT_SCAN_BUFFERS)
        .enable_all()
        .build()
        .expect("tokio runtime");
    runtime.block_on(async {
        for case in CASES {
            let frame = build_scan_buffer_frame(case);
            let harness = RoundtripHarness::start();
            let mut client = harness.connect().await;

            // Single warm-up to amortise listener accept-loop priming.
            harness.run_one(&mut client, &frame).await;

            let mut samples = Vec::with_capacity(samples_target);
            for _ in 0..samples_target {
                let started = Instant::now();
                harness.run_one(&mut client, &frame).await;
                samples.push(started.elapsed());
            }
            print_dimensions("validation.roundtrip", case);
            report_percentiles("validation.roundtrip", case, &mut samples);

            drop(client);
            harness.shutdown().await;
        }
    });
    println!("--- end ADR-031 sampler ---");
}

#[cfg(unix)]
criterion_group!(
    benches,
    bench_validation_service,
    bench_validation_roundtrip,
    bench_daemon_burst,
    bench_percentile_sampler,
);
#[cfg(unix)]
criterion_main!(benches);

#[cfg(not(unix))]
fn main() {
    println!("midedit_roundtrip benchmark currently runs on Unix only (mirrors ipc_roundtrip).");
}
