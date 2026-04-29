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
//!   foreground daemon uses.
//!
//! Three fixture sizes cover the mid-edit corpus laid out in ADR-031:
//! a **small** buffer (~1 KiB), a **medium** buffer (~64 KiB), and a
//! **near-cap** buffer just below the 1 MiB content cap
//! ([`CONTENT_SIZE_CAP_BYTES_USIZE`]).
//!
//! Two complementary harnesses live in this file:
//!
//! 1. A `criterion` benchmark group (`midedit_service` /
//!    `midedit_roundtrip`) that records per-iteration timings for
//!    regression tracking. This is what `cargo bench` consumes and what CI
//!    feeds into the existing benchmark workflow.
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
//! The bench file documents these inline; automated CI gating against the
//! SLO is a follow-up task and is intentionally not invented here. See
//! `plans/decisions/031-validation-latency-rubric.md`.
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
use std::time::{Duration, Instant};

#[cfg(unix)]
use anvil_intercept::Shutdown;
#[cfg(unix)]
use anvil_intercept::enforcement::{CONTENT_SIZE_CAP_BYTES_USIZE, EnforcementPipeline};
#[cfg(unix)]
use anvil_intercept::ipc::{IpcListener, NoopDispatcher};
#[cfg(unix)]
use anvil_intercept::midedit::{
    ScanBufferMode, ScanBufferRequest, ScanBufferService, scan_buffer_with_pipeline,
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

#[cfg(unix)]
const PERCENTILE_SAMPLES: usize = 200;

/// Buffer-size cases shared across both criterion and percentile harnesses.
#[cfg(unix)]
struct BufferCase {
    label: &'static str,
    bytes: usize,
}

#[cfg(unix)]
const CASES: &[BufferCase] = &[
    BufferCase {
        label: "small_1KiB",
        bytes: SMALL_BYTES,
    },
    BufferCase {
        label: "medium_64KiB",
        bytes: MEDIUM_BYTES,
    },
    BufferCase {
        label: "near_cap_1MiB_minus_1KiB",
        bytes: NEAR_CAP_BYTES,
    },
];

/// Build a representative TypeScript-shaped fixture of the requested byte
/// length. The content is deliberately rule-clean so we measure the steady
/// scanning cost rather than diagnostic emission cost; mid-edit is dominated
/// by the warm-path scan, not by diagnostic serialisation.
#[cfg(unix)]
fn make_fixture(bytes: usize) -> String {
    // Roughly 80-byte lines keeps the fixture realistic and avoids degenerate
    // "single huge line" cases that distort scanner work.
    const LINE: &str =
        "const value: number = compute(left, right) + adjust(seed); // representative line\n";
    let mut out = String::with_capacity(bytes + LINE.len());
    while out.len() < bytes {
        out.push_str(LINE);
    }
    out.truncate(bytes);
    out
}

#[cfg(unix)]
fn make_request(path: &str, text: String) -> ScanBufferRequest {
    ScanBufferRequest {
        path: PathBuf::from(path),
        text,
        version: 1,
        mode: ScanBufferMode::MidEdit,
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

// ---------------------------------------------------------------------------
// Criterion harness
// ---------------------------------------------------------------------------

#[cfg(unix)]
fn bench_validation_service(c: &mut Criterion) {
    let pipeline = EnforcementPipeline::default();
    let mut group = c.benchmark_group("midedit_service");
    group.measurement_time(Duration::from_secs(5));
    group.sample_size(60);

    for case in CASES {
        let request = make_request("src/realtime/buffer.ts", make_fixture(case.bytes));
        group.throughput(Throughput::Bytes(case.bytes as u64));
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
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("tokio runtime");

    let mut group = c.benchmark_group("midedit_roundtrip");
    group.measurement_time(Duration::from_secs(8));
    group.sample_size(30);

    for case in CASES {
        // Stand up a fresh daemon per case so the listener's `scan_buffer`
        // semaphore and any criterion warm-up cost are isolated.
        let harness = runtime.block_on(async { RoundtripHarness::start() });
        let frame = build_scan_buffer_frame(case);

        group.throughput(Throughput::Bytes(case.bytes as u64));
        group.bench_function(case.label, |b| {
            b.iter(|| {
                runtime.block_on(harness.run_one(&frame));
            });
        });

        runtime.block_on(harness.shutdown());
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
            "text": make_fixture(case.bytes),
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
        let listener = IpcListener::bind_with_scan_buffer_service(
            &socket,
            NoopDispatcher,
            scan_buffer,
        )
        .expect("bind listener");
        let (shutdown, token) = Shutdown::new();
        let handle = tokio::spawn(async move { listener.serve(token).await });
        Self {
            socket,
            _tmp: tmp,
            shutdown,
            handle: Some(handle),
        }
    }

    async fn run_one(&self, frame: &str) {
        let stream = UnixStream::connect(&self.socket)
            .await
            .expect("connect client");
        let mut client = BufReader::new(stream);
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
        debug_assert!(
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

    println!();
    println!("--- ADR-031 mid-edit warm percentile sampler ---");
    println!("interactive buffer SLO (p95): validation.service ≤ 50ms, validation.roundtrip ≤ 80ms");

    for case in CASES {
        let request = make_request("src/realtime/buffer.ts", make_fixture(case.bytes));

        let mut service_samples = Vec::with_capacity(PERCENTILE_SAMPLES);
        for _ in 0..PERCENTILE_SAMPLES {
            let started = Instant::now();
            let response = scan_buffer_with_pipeline(&request, &pipeline)
                .expect("scan_buffer_with_pipeline");
            service_samples.push(started.elapsed());
            black_box(response);
        }
        print_dimensions("validation.service", case);
        report_percentiles("validation.service", case, &mut service_samples);
    }

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("tokio runtime");
    runtime.block_on(async {
        for case in CASES {
            let frame = build_scan_buffer_frame(case);
            let harness = RoundtripHarness::start();

            // Single warm-up to amortise listener accept-loop priming.
            harness.run_one(&frame).await;

            let mut samples = Vec::with_capacity(PERCENTILE_SAMPLES);
            for _ in 0..PERCENTILE_SAMPLES {
                let started = Instant::now();
                harness.run_one(&frame).await;
                samples.push(started.elapsed());
            }
            print_dimensions("validation.roundtrip", case);
            report_percentiles("validation.roundtrip", case, &mut samples);

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
    bench_percentile_sampler,
);
#[cfg(unix)]
criterion_main!(benches);

#[cfg(not(unix))]
fn main() {
    println!(
        "midedit_roundtrip benchmark currently runs on Unix only (mirrors ipc_roundtrip)."
    );
}
