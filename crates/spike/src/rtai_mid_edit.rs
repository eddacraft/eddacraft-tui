//! RTAI-001 Phase-0 spike: prove the in-flight buffer → daemon → rule →
//! diagnostic loop fits under the mid-edit latency budget.
//!
//! What this is:
//! - A standalone binary that simulates a `didChange` mid-edit flow on
//!   a single fixture buffer, sends it to a prototype daemon endpoint
//!   (an in-process worker thread reachable over an mpsc channel — see
//!   the architecture decision recorded in
//!   `plans/specs/2026-04-26-rtai-001-spike-report.md`), runs the
//!   existing `anvil_checks::secret::scan_content` rule against the
//!   buffer, returns a `Vec<Diagnostic>`, and times the round-trip.
//!
//! What this is **not**:
//! - Production code. The spike is intentionally throwaway: the channel
//!   transport stands in for the real IPC INTD-002 will deliver, and
//!   the worker is single-threaded with no batching, debounce, or
//!   cancellation. The point is to measure floor latency for the
//!   simplest possible loop so RTAI-002 has a real number to budget
//!   against.
//!
//! Run: `cargo run -q --release -p anvil-spike --bin spike-rtai-mid-edit`

// Spike binary: percentile reporting and ratio displays cast small ints
// to f64 for human-readable output. Precision loss is acceptable here
// because samples never approach the 2^52 mantissa cap.
#![allow(clippy::cast_precision_loss)]

use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use anvil_checks::secret::{SecretCheckConfig, SecretFinding, scan_content};
use anvil_kernel_types::diagnostics::KnownMode;
use anvil_kernel_types::{Category, Diagnostic, DiagnosticSource, Location, Mode, Severity};

/// Number of round-trips to time. 1024 picks p95 / p99 buckets cleanly
/// without making the spike take noticeably longer than a single test.
const ITERATIONS: usize = 1024;

/// Fixture buffer that mimics what a Cursor / VS Code user is editing
/// when AI assistance suggests a leaked credential. The `api_key='…'`
/// shape matches the default `API Key` rule — a low-confidence pattern
/// that requires an `api_key=` keyword anchor, picked here because the
/// shape is the most common one in the wild. (Bare `AKIA…` tokens used
/// to be silently dropped by the scanner's `looks_like_code` filter;
/// issue #1800 fixed that, so they would also fire now.) Two unrelated
/// lines on either side put the finding off the first scanned line so
/// the scanner's per-line skip path is exercised.
const FIXTURE_BUFFER: &str = "import { sdk } from \"./client\";\n\
const config = { api_key: 'abcdEFGH1234567890' };\n\
sdk.connect(config);\n";

const FIXTURE_PATH: &str = "src/auth/client.ts";

/// Mid-edit envelope from a fake LSP-shaped client. Held to the minimum
/// the rule needs: a path (so diagnostics carry a `Location.file`),
/// the buffer text, and a monotonically increasing version so the
/// daemon can later coalesce stale work. Anything richer (ranges,
/// multi-buffer batches, cancellation tokens) is RTAI-002's problem.
#[derive(Debug, Clone)]
struct DidChange {
    path: String,
    text: String,
    version: u64,
}

/// Reply envelope. Carries the diagnostics for the buffer version that
/// was scanned plus the version itself so a real driver could drop
/// stale replies. The wire shape is the canonical
/// `anvil.diagnostic.v1` payload exported from `anvil-kernel-types`.
#[derive(Debug)]
struct DidChangeReply {
    version: u64,
    diagnostics: Vec<Diagnostic>,
}

fn main() {
    println!("RTAI-001 Phase-0 spike: mid-edit secret-detection round-trip");
    println!("------------------------------------------------------------");

    let (req_tx, req_rx) = mpsc::channel::<(DidChange, mpsc::Sender<DidChangeReply>)>();

    // Prototype daemon endpoint. A single worker thread blocks on the
    // request channel, runs the rule against the buffer text, builds
    // diagnostics, and replies via the per-request reply channel. This
    // mirrors the shape `INTD::run_daemon_loop` will eventually expose
    // without committing to any real IPC transport.
    let worker = thread::spawn(move || {
        let secret_config = SecretCheckConfig::default();
        while let Ok((req, reply)) = req_rx.recv() {
            let findings = scan_content(&req.text, &req.path, &secret_config);
            let diagnostics = findings_to_diagnostics(&req.path, &findings);
            // Drop on send-fail: the requester gave up before us. Same
            // shape as a real daemon under cancellation.
            let _ = reply.send(DidChangeReply {
                version: req.version,
                diagnostics,
            });
        }
    });

    // Warm the worker — first run pays for static-regex compilation and
    // page faults. We discard the warm-up timing.
    let _ = round_trip(&req_tx, FIXTURE_PATH, FIXTURE_BUFFER, 0);

    let mut samples: Vec<Duration> = Vec::with_capacity(ITERATIONS);
    let mut diagnostic_count = 0usize;
    for v in 1..=ITERATIONS as u64 {
        let (elapsed, n) = round_trip(&req_tx, FIXTURE_PATH, FIXTURE_BUFFER, v);
        samples.push(elapsed);
        diagnostic_count += n;
    }

    drop(req_tx); // close the channel so the worker thread can exit.
    // Surface a worker-thread panic — silently swallowing the join
    // result would let the harness print "PASS" alongside a crashed
    // worker (e.g. if a future refactor breaks the rule pipeline).
    worker.join().expect("worker thread panicked");

    samples.sort_unstable();
    let p50 = samples[samples.len() / 2];
    let p95 = samples[(samples.len() * 95) / 100];
    let p99 = samples[(samples.len() * 99) / 100];
    let max = *samples.last().expect("samples non-empty");
    let min = samples[0];

    println!("iterations          : {ITERATIONS}");
    println!("fixture path        : {FIXTURE_PATH}");
    println!("fixture buffer bytes: {}", FIXTURE_BUFFER.len());
    println!(
        "diagnostics emitted : {diagnostic_count} (avg {:.2} per round-trip)",
        diagnostic_count as f64 / ITERATIONS as f64,
    );
    println!();
    println!("Round-trip latency");
    println!("  min  : {:>8} µs", min.as_micros());
    println!("  p50  : {:>8} µs", p50.as_micros());
    println!("  p95  : {:>8} µs", p95.as_micros());
    println!("  p99  : {:>8} µs", p99.as_micros());
    println!("  max  : {:>8} µs", max.as_micros());
    println!();
    println!(
        "Mid-edit p95 budget (ADR-031): 80 ms warm. Spike measured p95 {:.1} ms.",
        p95.as_micros() as f64 / 1_000.0,
    );
    if p95 < Duration::from_millis(80) {
        println!("Result: PASS — spike floor is well inside the warm budget.");
    } else {
        println!(
            "Result: WARN — spike floor is above the warm budget; budget needs widening or transport rework."
        );
    }
}

/// Run a single mid-edit round-trip and return its wall-clock latency
/// plus the diagnostic count (so the harness can confirm the fixture
/// is actually firing the rule).
fn round_trip(
    req_tx: &mpsc::Sender<(DidChange, mpsc::Sender<DidChangeReply>)>,
    path: &str,
    text: &str,
    version: u64,
) -> (Duration, usize) {
    let (reply_tx, reply_rx) = mpsc::channel::<DidChangeReply>();
    let req = DidChange {
        path: path.to_string(),
        text: text.to_string(),
        version,
    };
    let start = Instant::now();
    req_tx.send((req, reply_tx)).expect("worker alive");
    let reply = reply_rx.recv().expect("worker replied");
    let elapsed = start.elapsed();
    // Real assert (not debug_assert!) because the spike is normally run
    // in --release where debug asserts are compiled out.
    assert_eq!(reply.version, version, "version round-trip");
    (elapsed, reply.diagnostics.len())
}

/// Map secret findings to canonical `Diagnostic` payloads. Mirrors the
/// shape `gate::check_result_to_diagnostic` produces but tagged
/// `KnownMode::MidEdit` — the discriminator AIGUARD-002 reserved for
/// the in-flight surface.
fn findings_to_diagnostics(path: &str, findings: &[SecretFinding]) -> Vec<Diagnostic> {
    findings
        .iter()
        .map(|f| {
            // Per AIGUARD-002 the diagnostic `id` is per-finding-instance
            // and distinct from `source.rule_id`. `path:line:pattern`
            // gives a stable, deterministic id without pulling a UUID
            // dependency into the spike.
            let id = format!("diag_midedit_{}:{}:{}", path, f.line, f.pattern_name);
            // `Location.line` is optional. If a usize line number does
            // not fit in u32 the file is too large to be source code on
            // any sensible workflow; emit `None` rather than the
            // sentinel `u32::MAX` so consumers cannot mistake it for a
            // real line.
            let location_line = u32::try_from(f.line).ok();
            Diagnostic::new(
                id,
                Severity::Error,
                format!("Potential secret detected ({})", f.pattern_name),
                Location {
                    file: path.to_string(),
                    line: location_line,
                    column: None,
                    end_line: None,
                    end_column: None,
                },
                Category::Secret,
                DiagnosticSource {
                    rule_id: "secret-detection".to_string(),
                    source_module: "anvil-spike::rtai-mid-edit".to_string(),
                },
                Mode::known(KnownMode::MidEdit),
            )
        })
        .collect()
}
