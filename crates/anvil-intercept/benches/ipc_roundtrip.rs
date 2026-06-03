#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
#[cfg(unix)]
use std::sync::Arc;
#[cfg(unix)]
use std::time::{Duration, Instant};

#[cfg(unix)]
use anvil_intercept::Shutdown;
#[cfg(unix)]
use anvil_intercept::ipc::{IpcListener, NoopDispatcher, handle_jsonrpc_value_for_benchmark};
#[cfg(unix)]
use anvil_intercept::midedit::ScanBufferService;
#[cfg(unix)]
use serde_json::json;
#[cfg(unix)]
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
#[cfg(unix)]
use tokio::net::UnixStream;

// DSV-006 / Task 16: the in-process `validate_paths` concurrency SLO gate.
#[cfg(unix)]
use anvil_checks::antipattern::check::run_antipattern_check_bytes;
#[cfg(unix)]
use anvil_checks::antipattern::types::AntipatternCheckConfig;
#[cfg(unix)]
use anvil_intercept::confinement::Confinement;
#[cfg(unix)]
use anvil_intercept::ipc::SaveTimeDispatch;
#[cfg(unix)]
use anvil_intercept::save_time::{SaveTimeConn, SaveTimeState, SymbolParser};
#[cfg(unix)]
use anvil_intercept::workspace_pool::{
    DosCaps, ScanCancel, WorkScheduler, run_chunked_scan, walk_capped,
};
#[cfg(unix)]
use anvil_intercept_proto::protocol::{ChangeDescriptor, ChangeKindWire, ValidatePathsRequest};
#[cfg(unix)]
use anvil_kernel_types::{FileSymbols, SymbolKind, SymbolNode, TrustLevel, Visibility};
#[cfg(unix)]
use std::path::Path;
#[cfg(unix)]
use std::sync::Mutex;

#[cfg(unix)]
const SAMPLES: usize = 200;

/// ADR-031 interactive save-time `validation.service` p95 budget (80 ms). The
/// pass/fail SLO for the warm verdict path and the `4 agents + 1 scan` ramp.
#[cfg(unix)]
const SAVE_TIME_SERVICE_P95_BUDGET: Duration = Duration::from_millis(80);

/// RLB-008 / ADR-061 §9: WARN when an interactive request waits >80 ms before
/// service (pre-service queue wait).
#[cfg(unix)]
const QUEUE_WAIT_WARN: Duration = Duration::from_millis(80);

/// Agents in the *gated* concurrency point (ADR-061 §9: "4 agents + 1
/// background scan"). The opt-in `ANVIL_BENCH_VALIDATE_AGENTS` sweep
/// ([`run_agent_sweep`]) varies this to chart the saturation curve.
#[cfg(unix)]
const RAMP_AGENTS: usize = 4;

/// `validate_paths` calls each ramp agent issues.
#[cfg(unix)]
const RAMP_ITERS_PER_AGENT: usize = 50;

#[cfg(unix)]
fn main() {
    let runtime = tokio::runtime::Runtime::new().expect("tokio runtime");
    runtime.block_on(async {
        let dispatcher = Arc::new(NoopDispatcher);
        let scan_buffer = ScanBufferService::default();
        let mut service_samples = Vec::with_capacity(SAMPLES);
        for _ in 0..SAMPLES {
            let request = json!({
                "jsonrpc": "2.0",
                "method": "session.list",
                "id": "service",
            });
            let started = Instant::now();
            let response = handle_jsonrpc_value_for_benchmark(request, &dispatcher, &scan_buffer)
                .await
                .expect("service response");
            service_samples.push(started.elapsed());
            assert!(
                response.get("result").is_some(),
                "unexpected response: {response}"
            );
        }
        report_dimensions("validation.service", "watch", "none");
        report("validation.service", &mut service_samples);

        let tmp = tempfile::tempdir().expect("tempdir");
        std::fs::set_permissions(tmp.path(), std::fs::Permissions::from_mode(0o700))
            .expect("secure tempdir permissions");
        let socket = tmp.path().join("intercept.sock");
        let listener = IpcListener::bind(&socket, NoopDispatcher).expect("bind listener");
        let (shutdown, token) = Shutdown::new();
        let handle = tokio::spawn(async move { listener.serve(token).await });

        let mut roundtrip_samples = Vec::with_capacity(SAMPLES);
        for _ in 0..SAMPLES {
            let client = UnixStream::connect(&socket).await.expect("connect client");
            let mut client = BufReader::new(client);
            let request = "{\"jsonrpc\":\"2.0\",\"method\":\"session.list\",\"id\":\"bench\"}\n";
            let started = Instant::now();
            client
                .get_mut()
                .write_all(request.as_bytes())
                .await
                .expect("write request");
            let mut response = String::new();
            client
                .read_line(&mut response)
                .await
                .expect("read response");
            assert!(
                response.contains("\"result\""),
                "unexpected benchmark response: {response}"
            );
            roundtrip_samples.push(started.elapsed());
        }

        report_dimensions("validation.roundtrip", "watch", "none");
        report("validation.roundtrip", &mut roundtrip_samples);

        shutdown.trigger();
        tokio::time::timeout(Duration::from_secs(1), handle)
            .await
            .expect("listener timeout")
            .expect("listener join")
            .expect("listener ok");
    });

    // DSV-006 / Task 16: the `validate_paths` concurrency SLO gate. Synchronous
    // (rayon + std threads, no tokio), so it runs after the async transport
    // cases. A budget breach exits non-zero — this step IS the CI gate.
    if slo_gate_failed() {
        eprintln!("validate_paths SLO gate failed (see FAIL lines above)");
        std::process::exit(1);
    }
}

/// A deterministic stub [`SymbolParser`] for the bench. The real kernel-backed
/// parser is injected from `anvil-cli` (which the intercept crate must not
/// depend on), so the bench supplies its own: every file parses to one public
/// function, which keeps the warm-cache + certify path live without tree-sitter.
///
/// The optional per-parse `stall` (env `ANVIL_BENCH_VALIDATE_STALL_MS`) inflates
/// the measured verdict latency so CI can prove the SLO gate trips on a
/// synthetic regression (the gate is only credible if a regression fails it).
#[cfg(unix)]
#[derive(Debug)]
struct BenchParser {
    stall: Duration,
}

#[cfg(unix)]
impl SymbolParser for BenchParser {
    fn parse(&self, path: &Path, _bytes: &[u8]) -> Option<FileSymbols> {
        if !self.stall.is_zero() {
            std::thread::sleep(self.stall);
        }
        let file = path.to_string_lossy().into_owned();
        Some(FileSymbols {
            file: file.clone(),
            symbols: vec![SymbolNode {
                id: 1,
                kind: SymbolKind::Function,
                name: "f".to_string(),
                visibility: Visibility::Public,
                file,
                trust_level: TrustLevel::Unknown,
            }],
            imports: Vec::new(),
        })
    }
}

/// Build a shared save-time daemon state with the bench parser wired.
#[cfg(unix)]
fn build_state(stall: Duration) -> SaveTimeState {
    let scheduler = WorkScheduler::new().expect("build work pools");
    SaveTimeState::new(
        scheduler,
        AntipatternCheckConfig::default(),
        Confinement::open_default(),
    )
    .with_parser(Arc::new(BenchParser { stall }))
}

/// A workspace tempdir holding `files` small source files under `src/`.
#[cfg(unix)]
fn make_workspace(files: usize) -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("workspace tempdir");
    std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o700))
        .expect("secure tempdir permissions");
    let src = dir.path().join("src");
    std::fs::create_dir_all(&src).expect("src dir");
    for i in 0..files.max(1) {
        std::fs::write(
            src.join(format!("m{i}.ts")),
            b"export function f() { return 1; }\n",
        )
        .expect("write source file");
    }
    dir
}

/// A `validate_paths` request modifying the workspace's first source file.
#[cfg(unix)]
fn modify_request(root: &Path) -> ValidatePathsRequest {
    ValidatePathsRequest {
        workspace_root: root.to_string_lossy().into_owned(),
        paths: vec![ChangeDescriptor {
            path: "src/m0.ts".to_string(),
            change: ChangeKindWire::Modified,
            content_hash: None,
            mtime: None,
        }],
    }
}

/// Report a pass/fail verdict for one gated p95 against its budget. Returns
/// `true` on breach so the caller can fold it into the overall gate result.
#[cfg(unix)]
fn gate(label: &str, p95: Duration, budget: Duration) -> bool {
    if p95 > budget {
        println!(
            "FAIL: {label} p95 {} exceeds budget {}",
            fmt(p95),
            fmt(budget)
        );
        true
    } else {
        println!(
            "PASS: {label} p95 {} within budget {}",
            fmt(p95),
            fmt(budget)
        );
        false
    }
}

/// Drive `agents` concurrent `validate_paths` clients + 1 background scan and
/// return the agents' interactive latencies (DSV-006 / Task 16, ADR-061 §9).
///
/// Each agent drives its OWN workspace (distinct `WorktreeKey`), so this
/// measures interactive-pool contention, not per-key assurance-lock
/// serialisation. The background scan is a CPU-competing thread standing in for
/// the daemon's background pool (production placement is the background pool;
/// the bench models the core contention it creates). The scanner stops as soon
/// as the agents finish so the scope barrier can complete. `agents` is floored
/// at 1.
#[cfg(unix)]
fn run_concurrency_ramp(state: &SaveTimeState, agents: usize) -> Vec<Duration> {
    let agents = agents.max(1);
    let agent_ws: Vec<tempfile::TempDir> = (0..agents).map(|_| make_workspace(1)).collect();
    let scan_ws = make_workspace(64);
    let caps = DosCaps::default();
    let scan_files = walk_capped(scan_ws.path(), caps.max_walk_depth, caps.max_walk_files);
    let latencies: Mutex<Vec<Duration>> =
        Mutex::new(Vec::with_capacity(agents * RAMP_ITERS_PER_AGENT));
    let cancel = ScanCancel::new();

    std::thread::scope(|s| {
        let scanner = s.spawn(|| {
            while !cancel.is_cancelled() {
                run_chunked_scan(&scan_files, 16, &cancel, |p| {
                    let _ = std::fs::read(p).map(|b| b.len());
                });
            }
        });
        let handles: Vec<_> = agent_ws
            .iter()
            .map(|ws| {
                let req = modify_request(ws.path());
                let latencies = &latencies;
                s.spawn(move || {
                    let mut conn = SaveTimeConn::new(state);
                    let mut local = Vec::with_capacity(RAMP_ITERS_PER_AGENT);
                    for _ in 0..RAMP_ITERS_PER_AGENT {
                        let started = Instant::now();
                        let _ = conn.validate_paths(&req).expect("verdict");
                        local.push(started.elapsed());
                    }
                    latencies.lock().expect("latencies lock").extend(local);
                })
            })
            .collect();
        for a in handles {
            a.join().expect("agent thread");
        }
        cancel.cancel();
        scanner.join().expect("scanner thread");
    });

    latencies.into_inner().expect("ramp latencies")
}

/// The DSV-006 / Task 16 concurrency SLO: warm `validate_paths` p95, the
/// `4 agents + 1 background scan` ramp, the daemon-absent scoped-fallback
/// comparison, and (opt-in) a report-only stepped agent-count sweep
/// ([`run_agent_sweep`]). Returns `true` if any *gated* p95 breached its budget
/// (the name's polarity matches the `if slo_gate_failed() { exit(1) }` call
/// site); the sweep never affects the return.
///
/// This is the **service** harness (ADR-031 `validation.service`): it drives
/// the verdict path in-process on the daemon's interactive pool, the exact work
/// the SLO governs. The transport (`validation.roundtrip`) harness for a real
/// `watch` / MCP driver lands with those clients in DSV-007.
#[cfg(unix)]
fn slo_gate_failed() -> bool {
    let stall = Duration::from_millis(
        std::env::var("ANVIL_BENCH_VALIDATE_STALL_MS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0),
    );
    if !stall.is_zero() {
        println!(
            "note: ANVIL_BENCH_VALIDATE_STALL_MS={} ms injected (synthetic-regression mode)",
            stall.as_millis()
        );
    }
    let state = build_state(stall);

    // --- Case A: warm single-client validation.service p95 (gated ≤ 80 ms) ---
    let warm_ws = make_workspace(1);
    let warm_req = modify_request(warm_ws.path());
    {
        // Warm the pools + cache + page in the file before measuring.
        let mut conn = SaveTimeConn::new(&state);
        for _ in 0..20 {
            let _ = conn.validate_paths(&warm_req).expect("warm-up verdict");
        }
    }
    let mut service = Vec::with_capacity(SAMPLES);
    {
        let mut conn = SaveTimeConn::new(&state);
        for _ in 0..SAMPLES {
            let started = Instant::now();
            let resp = conn.validate_paths(&warm_req).expect("verdict");
            service.push(started.elapsed());
            assert!(!resp.check_families.is_empty(), "verdict scoped a family");
        }
    }
    report_dimensions("validation.service:validate_paths", "save", "default-v1");
    report("validation.service:validate_paths", &mut service);
    let baseline_p50 = service[percentile_index(service.len(), 50)];
    let mut failed = gate(
        "validation.service:validate_paths (warm)",
        service[percentile_index(service.len(), 95)],
        SAVE_TIME_SERVICE_P95_BUDGET,
    );

    // --- Case B: 4 agents + 1 background scan ramp (gated ≤ 80 ms) ---
    let mut ramp = run_concurrency_ramp(&state, RAMP_AGENTS);
    report_dimensions(
        "validation.service:validate_paths:4agents+scan",
        "save",
        "default-v1",
    );
    report("validation.service:validate_paths:4agents+scan", &mut ramp);
    let ramp_p95 = ramp[percentile_index(ramp.len(), 95)];
    failed |= gate(
        "validation.service:validate_paths (4 agents + 1 scan)",
        ramp_p95,
        SAVE_TIME_SERVICE_P95_BUDGET,
    );
    // Pre-service queue-wait proxy: contention overhead above the solo baseline
    // p50 (RLB-008). A true queue-wait probe needs admission instrumentation;
    // this conservative proxy WARNs rather than fails.
    let queue_wait_p95 = ramp_p95.saturating_sub(baseline_p50);
    if queue_wait_p95 > QUEUE_WAIT_WARN {
        println!(
            "WARN: validate_paths pre-service queue wait p95 ~{} exceeds {} (RLB-008)",
            fmt(queue_wait_p95),
            fmt(QUEUE_WAIT_WARN),
        );
    }

    // --- Case C: daemon-absent scoped fallback (RLB-002), report only ---
    // What `watch` runs on daemon absence: a scoped antipattern check over the
    // changed bytes, never `--all`. Background path → report only (ADR-031).
    let bytes = std::fs::read(warm_ws.path().join("src/m0.ts")).expect("read changed file");
    let scanned: Vec<(&str, &[u8])> = vec![("src/m0.ts", bytes.as_slice())];
    let cfg = AntipatternCheckConfig::default();
    let fallback_pool = rayon::ThreadPoolBuilder::new()
        .num_threads(1)
        .build()
        .expect("fallback pool");
    let root = warm_ws.path().to_string_lossy().into_owned();
    let mut fallback = Vec::with_capacity(SAMPLES);
    for _ in 0..SAMPLES {
        let started = Instant::now();
        let _ = run_antipattern_check_bytes(&scanned, &cfg, Some(root.as_str()), &fallback_pool);
        fallback.push(started.elapsed());
    }
    report_dimensions("validation.service:scoped-fallback", "save", "default-v1");
    report(
        "validation.service:scoped-fallback (daemon-absent, RLB-002)",
        &mut fallback,
    );

    // --- Optional Case D: stepped agent-count sweep (opt-in, report-only) ---
    run_agent_sweep(&state);

    failed
}

/// Optional stepped agent-count sweep (DSV-006 diagnostic, opt-in via
/// `ANVIL_BENCH_VALIDATE_AGENTS=1,2,4,8`). Unlike the fixed 4-agent gate (Case
/// B), this is **report-only**: it prints interactive p95 at each concurrency
/// level so the saturation knee — and the headroom against the 80 ms budget —
/// is visible, without failing when a deliberately-overloaded level crosses
/// budget. The default CI run leaves the env unset and stays a single gate
/// point (mirrors how `load-ramp.sh` separates `--smoke` from the full ramp).
#[cfg(unix)]
fn run_agent_sweep(state: &SaveTimeState) {
    let Ok(spec) = std::env::var("ANVIL_BENCH_VALIDATE_AGENTS") else {
        return;
    };
    let levels: Vec<usize> = spec
        .split(',')
        .filter_map(|s| s.trim().parse::<usize>().ok())
        .filter(|&n| n > 0)
        .collect();
    if levels.is_empty() {
        return;
    }
    println!("note: validate_paths agent sweep (report-only) levels={levels:?}");
    for n in levels {
        let mut lat = run_concurrency_ramp(state, n);
        let label = format!("validation.service:validate_paths:sweep:{n}agents+scan");
        report_dimensions(&label, "save", "default-v1");
        report(&label, &mut lat);
        let p95 = lat[percentile_index(lat.len(), 95)];
        let verdict = if p95 <= SAVE_TIME_SERVICE_P95_BUDGET {
            "within"
        } else {
            "OVER"
        };
        println!(
            "sweep: {n} agents + 1 scan -> interactive p95 {} ({verdict} {} budget)",
            fmt(p95),
            fmt(SAVE_TIME_SERVICE_P95_BUDGET),
        );
    }
}

#[cfg(unix)]
fn report_dimensions(boundary: &str, mode: &str, rule_set: &str) {
    println!(
        "dimensions: mode={mode} boundary={boundary} surface=cli-harness contentSource=disk ruleSet={rule_set} fixtureCorpus=synthetic-spike contentSize=0 platform={} daemonState=warm driverProtocol=json-rpc-2.0 debounceMs=0",
        std::env::consts::OS,
    );
}

#[cfg(unix)]
fn report(name: &str, samples: &mut [Duration]) {
    if samples.is_empty() {
        println!("{name}: samples=0 (no measurements)");
        return;
    }
    samples.sort_unstable();
    println!(
        "{name}: samples={} p50={} p95={} p99={}",
        samples.len(),
        fmt(samples[percentile_index(samples.len(), 50)]),
        fmt(samples[percentile_index(samples.len(), 95)]),
        fmt(samples[percentile_index(samples.len(), 99)]),
    );
}

#[cfg(unix)]
fn percentile_index(len: usize, percentile: usize) -> usize {
    ((len.saturating_sub(1)) * percentile) / 100
}

#[cfg(unix)]
fn fmt(duration: Duration) -> String {
    format!("{:.3}ms", duration.as_secs_f64() * 1_000.0)
}

#[cfg(not(unix))]
fn main() {
    println!("ipc_roundtrip benchmark is currently implemented for Unix socket IPC only");
}
