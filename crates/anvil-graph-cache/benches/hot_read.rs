//! GV2-025: the hot-read latency gate ADR-063 names but never shipped.
//!
//! ADR-063 declares the resident hot-read API "admissible" only if it stays
//! inside the ADR-031 interactive save-time budget, but until now nothing
//! measured it. This bench closes that gap: it times all four [`HotReadApi`]
//! allowlist categories — `resident_symbols`/`symbol_owner` (#1), `known_edge`
//! (#2), `reverse_impact` at depth 1 and at the hard cap (#3), and
//! `boundary_membership` (#4) — plus the A′ verdict entry ([`certify`]) over a
//! versioned resident-graph corpus, reports p50/p95/p99, and **exits non-zero
//! when any op's p95 exceeds the budget** — so the CI step that runs it is the
//! gate, not a report.
//!
//! ## Why `harness = false` (and not a Criterion group)
//!
//! The repo's other ADR-031 latency gate — `anvil-intercept`'s `ipc_roundtrip`
//! — is a hand-rolled `harness = false` `main()` that computes true percentiles
//! and `process::exit(1)` on a breach. Criterion's statistical harness reports
//! mean/median/slope with confidence intervals, not a p95 pass/fail exit code,
//! so it cannot *gate* CI on an absolute budget. This bench follows the proven
//! sibling so the two latency gates read and fail identically.
//!
//! ## Quiet-box / CI-box requirement
//!
//! These are sub-microsecond-to-microsecond in-process reads, three orders of
//! magnitude under the 80 ms budget, so the gate has enormous headroom and is a
//! *regression ceiling*, not a tight SLO — it tolerates a shared runner. Still,
//! p95 is only meaningful on a box that is not paging or fighting for cores: run
//! `cargo bench -p eddacraft-anvil-graph-cache --bench hot_read` on an otherwise
//! idle machine (CI runs it on the dedicated resource-budget runner). The bench
//! is pure in-process (no sockets, no filesystem, no threads), so unlike the
//! watch/IPC benches it carries no inotify or daemon-startup flakiness and runs
//! on every platform.
//!
//! ## Self-test hook
//!
//! Setting `ANVIL_BENCH_HOTREAD_STALL_MS=<n>` injects an `n` ms sleep into every
//! measured read, driving p95 past the budget so CI can prove the gate actually
//! trips on a synthetic regression (mirrors `ipc_roundtrip`'s
//! `ANVIL_BENCH_VALIDATE_STALL_MS`).

use std::hint::black_box;
use std::time::{Duration, Instant};

use anvil_graph_cache::{
    ChangeKind, DependencyGraph, GraphDelta, HotReadApi, MAX_REVERSE_IMPACT_DEPTH, SymbolGraph,
    certify,
};
use anvil_kernel_types::{SymbolIdentity, SymbolKind, SymbolNode, TrustLevel, Visibility};

/// Warm reads measured per operation. Matches `ipc_roundtrip`'s `SAMPLES` so the
/// two latency gates report on the same sample budget.
const SAMPLES: usize = 200;

/// Warm-up reads discarded before measurement, so the first-touch page-in and
/// branch-predictor warm-up never land in the percentiles.
const WARMUP: usize = 20;

/// ADR-031 interactive save-time `validation.service` p95 budget (80 ms). The
/// hot reads are sub-components of that service path, so each must clear it with
/// orders of magnitude to spare; the gate fires on a gross per-op regression.
///
/// This is deliberately a per-op ceiling, not a composed-path SLO: it cannot
/// catch death-by-accumulation (many cheap reads summing past 80 ms in one
/// `validate_paths`). That composed budget is already gated end-to-end by the
/// warm `validation.service:validate_paths` p95 gate in `anvil-intercept`'s
/// `ipc_roundtrip` bench (RLB-008 / DSV-006), which drives the real save-time
/// verdict path. The two gates are complementary: this one localises a
/// component regression to the hot-read API; that one bounds the whole call.
const HOT_READ_P95_BUDGET: Duration = Duration::from_millis(80);

/// Reverse-impact closure cap (files). Sized well above the corpus's depth-cap
/// closure so the walk measures the real traversal cost rather than an early
/// [`anvil_graph_cache::HotReadMiss::ImpactSetOverflow`] short-circuit.
const IMPACT_BUDGET: usize = 4096;

/// Direct importers of the hub file (depth-1 closure size).
const HUB_DIRECT_IMPORTERS: usize = 64;
/// Second-hop importers per direct importer (depth-2 fan-out).
const SECOND_HOP_FANOUT: usize = 4;

fn main() {
    let stall = Duration::from_millis(
        std::env::var("ANVIL_BENCH_HOTREAD_STALL_MS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0),
    );
    if !stall.is_zero() {
        println!(
            "note: ANVIL_BENCH_HOTREAD_STALL_MS={} ms injected (synthetic-regression mode)",
            stall.as_millis()
        );
    }
    println!(
        "note: hot-read corpus=hot-read-corpus-v1 files={CORPUS_FILES} \
         hub-direct-importers={HUB_DIRECT_IMPORTERS} depth-cap={MAX_REVERSE_IMPACT_DEPTH} \
         (run on a quiet/CI box)",
    );

    let (sym, dep) = build_corpus();
    let api = HotReadApi::new(&sym, &dep);
    // A surface-changing ContentModify on the hub so `certify` reaches the
    // `impact_closure` walk over the hub's dependents (a body-only delta would
    // certify self-only and never walk).
    let delta = surface_change_delta(HUB_FILE);

    let mut failed = false;

    // Allowlist #1 — per-file symbol lookup on the hub (its resident symbols).
    failed |= measure_gate("resident_symbols (hub)", stall, || {
        let _ = black_box(api.resident_symbols(black_box(HUB_FILE)));
    });

    // Allowlist #1 — symbol-ownership lookup (which file owns a symbol id), O(1).
    failed |= measure_gate("symbol_owner (hub api)", stall, || {
        let _ = black_box(api.symbol_owner(black_box(HUB_API_SYMBOL_ID)));
    });

    // Allowlist #2 — known-edge existence over the resident dependency index.
    failed |= measure_gate("known_edge (importer→hub)", stall, || {
        let _ = black_box(api.known_edge(black_box(KNOWN_IMPORTER), black_box(HUB_FILE)));
    });

    // Allowlist #4 — precomputed boundary/trust membership for a symbol id, O(1).
    failed |= measure_gate("boundary_membership (hub api)", stall, || {
        let _ = black_box(api.boundary_membership(black_box(HUB_API_SYMBOL_ID)));
    });

    // Allowlist #3 — bounded reverse-impact closure, depth 1 (direct importers).
    failed |= measure_gate("reverse_impact depth=1 (hub)", stall, || {
        let _ = black_box(api.reverse_impact(black_box(HUB_FILE), 1, IMPACT_BUDGET));
    });

    // Allowlist #3 — reverse-impact closure at the hard cap (the deepest walk
    // the hot path ever performs).
    failed |= measure_gate("reverse_impact depth=cap (hub)", stall, || {
        let _ = black_box(api.reverse_impact(
            black_box(HUB_FILE),
            MAX_REVERSE_IMPACT_DEPTH,
            IMPACT_BUDGET,
        ));
    });

    // The A′ verdict entry (GV2-027): the real save-time `certify` call, driven
    // down the surface-changed branch so it walks the certifiability closure.
    failed |= measure_gate("certify ContentModify (hub)", stall, || {
        let _ = black_box(api.certify(
            black_box(&ChangeKind::ContentModify),
            black_box(&delta),
            IMPACT_BUDGET,
            MAX_REVERSE_IMPACT_DEPTH,
        ));
    });

    // Belt-and-braces: the standalone `certify` free function over the same
    // resident graphs, so the gate also covers callers that do not go through
    // the `HotReadApi` wrapper.
    failed |= measure_gate("certify free-fn ContentModify (hub)", stall, || {
        let _ = black_box(certify(
            &sym,
            &dep,
            black_box(&ChangeKind::ContentModify),
            black_box(&delta),
            IMPACT_BUDGET,
            MAX_REVERSE_IMPACT_DEPTH,
        ));
    });

    if failed {
        eprintln!("hot-read latency gate FAILED (see FAIL lines above)");
        std::process::exit(1);
    }
}

/// Measure `SAMPLES` warm iterations of `op`, report p50/p95/p99, and gate on
/// the p95 budget. Returns `true` on a breach.
fn measure_gate(label: &str, stall: Duration, mut op: impl FnMut()) -> bool {
    for _ in 0..WARMUP {
        op();
    }
    let mut samples = Vec::with_capacity(SAMPLES);
    for _ in 0..SAMPLES {
        let started = Instant::now();
        op();
        if !stall.is_zero() {
            std::thread::sleep(stall);
        }
        samples.push(started.elapsed());
    }
    // Sort here so the p95 the gate reads is not coupled to a side effect of
    // `report`; `report` receives an already-sorted slice.
    samples.sort_unstable();
    report_dimensions(label);
    report(label, &samples);
    gate(
        label,
        samples[percentile_index(samples.len(), 95)],
        HOT_READ_P95_BUDGET,
    )
}

// ---------------------------------------------------------------------------
// Corpus: `hot-read-corpus-v1`
// ---------------------------------------------------------------------------

/// The fan-in hub whose reverse-impact closure the depth benches walk.
const HUB_FILE: &str = "hub.ts";
/// A symbol id resident in the corpus: `build_corpus` adds the hub file's
/// symbols first, so id 0 is the hub's public `api` symbol — used by the
/// `symbol_owner`/`boundary_membership` O(1) reads.
const HUB_API_SYMBOL_ID: u64 = 0;
/// A file that directly imports the hub — i.e. a known edge `(importer → hub)`
/// resident in the dependency graph, used by the `known_edge` read.
const KNOWN_IMPORTER: &str = "l1_0.ts";
/// Total files in the corpus (hub + direct importers + second-hop importers).
const CORPUS_FILES: usize = 1 + HUB_DIRECT_IMPORTERS + HUB_DIRECT_IMPORTERS * SECOND_HOP_FANOUT;

/// Import-chain depth [`build_corpus`] lays down (hub ← L1 ← L2). The
/// "depth=cap" bench is only a real ceiling test if the corpus is at least as
/// deep as the hard cap, so this is pinned to the cap by a compile-time assert:
/// if [`MAX_REVERSE_IMPACT_DEPTH`] is ever raised (e.g. the GV2-026 runtime
/// lever), this build breaks until `build_corpus` grows another layer — it can
/// never silently fall back to under-exercising the new cap.
const CORPUS_HOP_DEPTH: u32 = 2;
const _: () = assert!(
    CORPUS_HOP_DEPTH >= MAX_REVERSE_IMPACT_DEPTH,
    "hot-read corpus is shallower than MAX_REVERSE_IMPACT_DEPTH; add an import \
     layer to build_corpus so the depth=cap bench exercises the full cap",
);

/// Build a representative resident `(SymbolGraph, DependencyGraph)`: a hub file
/// imported by [`HUB_DIRECT_IMPORTERS`] direct importers, each of which is in
/// turn imported by [`SECOND_HOP_FANOUT`] second-hop files. Every file carries a
/// small symbol set so per-file lookups read a realistic node, not an empty one.
fn build_corpus() -> (SymbolGraph, DependencyGraph) {
    let mut sym = SymbolGraph::new();
    let mut dep = DependencyGraph::new();
    let mut next_id: u64 = 0;

    add_file_symbols(&mut sym, &mut next_id, HUB_FILE);

    for i in 0..HUB_DIRECT_IMPORTERS {
        let l1 = format!("l1_{i}.ts");
        add_file_symbols(&mut sym, &mut next_id, &l1);
        // `l1` imports the hub, so `dependents_of(hub)` yields the L1 layer.
        dep.add_dependency(l1.clone(), HUB_FILE.to_string());
        for j in 0..SECOND_HOP_FANOUT {
            let l2 = format!("l2_{i}_{j}.ts");
            add_file_symbols(&mut sym, &mut next_id, &l2);
            dep.add_dependency(l2, l1.clone());
        }
    }

    (sym, dep)
}

/// Add a small, realistic symbol set (one public boundary symbol + two internal
/// ones) for `file`, advancing `next_id`.
fn add_file_symbols(sym: &mut SymbolGraph, next_id: &mut u64, file: &str) {
    let specs = [
        ("api", Visibility::Public, TrustLevel::Boundary),
        ("helper_a", Visibility::Internal, TrustLevel::Internal),
        ("helper_b", Visibility::Internal, TrustLevel::Internal),
    ];
    for (name, vis, trust) in specs {
        sym.add_symbol(SymbolNode {
            id: *next_id,
            kind: SymbolKind::Function,
            name: name.to_string(),
            visibility: vis,
            file: file.to_string(),
            trust_level: trust,
            span: None,
        })
        .expect("corpus symbol id is unique");
        *next_id += 1;
    }
}

/// A `ContentModify` delta whose `previously_public` set no longer matches the
/// current public surface of `file` (it names a since-removed public symbol), so
/// `export_surface_diff` is non-empty and `certify` walks the impact closure.
fn surface_change_delta(file: &str) -> GraphDelta {
    GraphDelta {
        file: file.to_string(),
        previously_public: [SymbolIdentity {
            file: file.to_string(),
            kind: SymbolKind::Function,
            name: "removed_public_api".to_string(),
            ordinal: 0,
        }]
        .into_iter()
        .collect(),
        ..Default::default()
    }
}

// ---------------------------------------------------------------------------
// Reporting + gate (shape-aligned with `ipc_roundtrip` for cross-gate parity)
// ---------------------------------------------------------------------------

/// ADR-031 §"Corpus and harness requirements" dimensions line, so a logged
/// measurement is self-describing. `boundary` is the canonical ADR-031
/// `validation.service` timing boundary (so this joins the sibling
/// `ipc_roundtrip` save-time measurements); the op identity rides on `surface`.
fn report_dimensions(op: &str) {
    println!(
        "dimensions: mode=save boundary=validation.service surface=hot-read:{op} \
         contentSource=memory ruleSet=none fixtureCorpus=hot-read-corpus-v1 contentSize=0 \
         platform={} daemonState=warm driverProtocol=in-process debounceMs=0",
        std::env::consts::OS,
    );
}

/// Print p50/p95/p99 for an **already-sorted** sample slice (the caller sorts
/// once in `measure_gate`, so the gate's p95 does not depend on a side effect
/// here).
fn report(name: &str, samples: &[Duration]) {
    if samples.is_empty() {
        println!("{name}: samples=0 (no measurements)");
        return;
    }
    println!(
        "{name}: samples={} p50={} p95={} p99={}",
        samples.len(),
        fmt(samples[percentile_index(samples.len(), 50)]),
        fmt(samples[percentile_index(samples.len(), 95)]),
        fmt(samples[percentile_index(samples.len(), 99)]),
    );
}

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

/// Nearest-rank percentile index (same convention as `ipc_roundtrip`).
fn percentile_index(len: usize, percentile: usize) -> usize {
    (len.saturating_sub(1) * percentile) / 100
}

fn fmt(duration: Duration) -> String {
    format!("{:.3}ms", duration.as_secs_f64() * 1_000.0)
}
