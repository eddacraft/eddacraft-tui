//! GCALL-006: save-time call-lift latency gate (ADR-086 / ADR-031).
//!
//! Times `update_file` call-lift and `re_resolve_calls` over a dense corpus;
//! exits non-zero if p95 exceeds the budget. `ANVIL_BENCH_CALLLIFT_STALL_MS`
//! forces a trip for CI self-test. Not a Criterion report — a pass/fail gate.

use std::hint::black_box;
use std::time::{Duration, Instant};

use anvil_graph_cache::{SymbolGraph, re_resolve_calls, update_file};
use anvil_kernel_types::{
    CallSite, CalleeRef, FileSymbols, ImportEdge, LocalSymbolRef, ReexportEdge, SymbolKind,
    SymbolNode, TrustLevel, Visibility,
};

/// Save-time applies measured per operation. Matches the sibling latency gates'
/// sample budget.
const SAMPLES: usize = 200;

/// Warm-up applies discarded before measurement, so first-touch page-in and
/// allocator warm-up never land in the percentiles.
const WARMUP: usize = 20;

/// ADR-031 interactive save-time `validation.service` p95 budget (80 ms). The
/// call lift is a sub-component of that service path, so it must clear the
/// ceiling with ample headroom (the 50k-node corpus measures ~13× under); the
/// gate fires on a gross regression in the lift's cost.
const CALL_LIFT_P95_BUDGET: Duration = Duration::from_millis(80);

/// Functions in the hot caller file. Sized to a large-but-real module so the
/// per-save lift walks a representative call count, not a toy one.
const HOT_FUNCS: usize = 100;
/// Same-file callees each hot function fans out to (resolve `via_import: None`).
const SAME_FILE_FANOUT: usize = 2;
/// Resident callee modules the hot file imports and calls into (cross-file lift).
const CALLEE_MODULES: usize = 100;
/// Filler symbols padding the resident graph to a mid-large-workspace node count
/// (see the module-level "Why the corpus is padded …" note). 3 symbols/file, so
/// ~17k filler files plus the callee modules and the hot file.
const RESIDENT_FILLER_SYMBOLS: usize = 50_000;

/// The hot caller file whose call sites the gate lifts each save.
const HOT_FILE: &str = "src/hot.ts";

/// Id base for the hot file's symbols, above the corpus's entire id range
/// (callee modules + filler) so the file's globally-unique ids never collide with
/// a resident symbol — `update_file` inserts ids verbatim (the kernel assigns
/// them globally unique via `id_offset`; it does not rebase).
const HOT_ID_BASE: u64 = (CALLEE_MODULES as u64) * 3 + RESIDENT_FILLER_SYMBOLS as u64 + 1;

fn main() {
    let stall = Duration::from_millis(
        std::env::var("ANVIL_BENCH_CALLLIFT_STALL_MS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0),
    );
    if !stall.is_zero() {
        println!(
            "note: ANVIL_BENCH_CALLLIFT_STALL_MS={} ms injected (synthetic-regression mode)",
            stall.as_millis()
        );
    }
    let call_sites = HOT_FUNCS * (SAME_FILE_FANOUT + 1);
    let resident_nodes = CALLEE_MODULES * 3 + RESIDENT_FILLER_SYMBOLS;
    println!(
        "note: call-lift corpus=call-lift-corpus-v1 hot-funcs={HOT_FUNCS} \
         callee-modules={CALLEE_MODULES} call-sites={call_sites} \
         resident-nodes={resident_nodes} (run on a quiet/CI box)",
    );

    let hot = hot_file_symbols();

    // Prove the lift does real work before timing it: a fresh apply must lift
    // *every* call site to a resident `Calls` edge. Asserting the full count
    // (not just "> 0") fails if EITHER path silently breaks — same-file
    // (`via_import: None`, 2/func) contributes `2 * HOT_FUNCS`, cross-file
    // (resolved `via_import`, 1/func) contributes `HOT_FUNCS`; a regression in
    // either drops the total below `call_sites` and trips this. Without it the
    // gate could measure a no-op and pass vacuously.
    let mut probe = build_corpus();
    let lifted = update_file(&mut probe, hot.clone());
    let call_edges = lifted
        .added_edges
        .iter()
        .filter(|(_, _, ty)| *ty == anvil_kernel_types::EdgeType::Calls)
        .count();
    assert!(
        call_edges >= call_sites,
        "call-lift bench corpus is degenerate: only {call_edges} Calls edges lifted \
         (expected {call_sites} — same-file and/or cross-file resolution broke)",
    );
    println!("note: warm-apply lifted {call_edges} resident Calls edges");

    let mut failed = false;

    // Op 1 — the full per-save apply of the hot file (remove + re-add + lift
    // import edges + lift call edges). Re-applying identical content is stable:
    // `remove_file` clears the file's nodes/edges, the re-add re-inserts the same
    // ids, and callees resolve to the untouched resident modules.
    {
        let mut graph = build_corpus();
        // Apply once so steady-state (edges already present, dedup active) is what
        // the warm samples measure — the realistic repeated-save hot path.
        let _ = update_file(&mut graph, hot.clone());
        failed |= measure_gate("update_file lift (hot.ts, ~300 calls)", stall, || {
            let _ = black_box(update_file(black_box(&mut graph), hot.clone()));
        });
    }

    // Op 2 — the daemon forward-reference re-resolution pass over the file's call
    // accumulator (the path that resolves callees not yet resident at first save).
    {
        let mut graph = build_corpus();
        let _ = update_file(&mut graph, hot.clone());
        let accumulator: Vec<(String, CallSite)> = hot
            .calls
            .iter()
            .map(|c| (HOT_FILE.to_string(), c.clone()))
            .collect();
        failed |= measure_gate("re_resolve_calls (accumulator, ~300 calls)", stall, || {
            let _ = black_box(re_resolve_calls(black_box(&mut graph), &accumulator));
        });
    }

    // Op 3 — the worst-case lift the ADR-086 §3 cap admits: a single file at the
    // `MAX_CALL_SITES` ceiling. Proves a maximally call-dense file's save stays in
    // budget, so the cap is a real bound, not a number (council OPS-1).
    {
        let cap_file = cap_ceiling_file_symbols();
        let mut graph = build_corpus();
        let _ = update_file(&mut graph, cap_file.clone());
        let label = "update_file lift (cap-ceiling file, MAX_CALL_SITES calls)";
        failed |= measure_gate(label, stall, || {
            let _ = black_box(update_file(black_box(&mut graph), cap_file.clone()));
        });
    }

    if failed {
        eprintln!("call-lift latency gate FAILED (see FAIL lines above)");
        std::process::exit(1);
    }
}

/// A single file carrying exactly [`anvil_kernel_types::MAX_CALL_SITES`]
/// same-file call sites (all resolved, `via_import: None`) over [`HOT_FUNCS`]
/// functions — the densest call file the ADR-086 §3 cap admits onto the lift.
fn cap_ceiling_file_symbols() -> FileSymbols {
    const FILE: &str = "src/dense.ts";
    let cap = anvil_kernel_types::MAX_CALL_SITES;
    let mut symbols = Vec::with_capacity(HOT_FUNCS);
    for i in 0..HOT_FUNCS {
        symbols.push(SymbolNode {
            id: HOT_ID_BASE + i as u64,
            kind: SymbolKind::Function,
            name: format!("fn_{i}"),
            visibility: Visibility::Internal,
            file: FILE.to_string(),
            trust_level: TrustLevel::Internal,
            span: None,
        });
    }
    let mut calls = Vec::with_capacity(cap);
    for n in 0..cap {
        let caller = n % HOT_FUNCS;
        let target = (n + 1) % HOT_FUNCS;
        calls.push(CallSite {
            from: LocalSymbolRef {
                kind: SymbolKind::Function,
                name: format!("fn_{caller}"),
                ordinal: 0,
                module_scope: false,
            },
            callee: CalleeRef {
                name: format!("fn_{target}"),
                via_import: None,
            },
            line: 1,
        });
    }
    FileSymbols {
        file: FILE.to_string(),
        symbols,
        imports: Vec::new(),
        reexports: Vec::<ReexportEdge>::new(),
        calls,
        // Models the post-truncation state of an over-cap file: cap_call_sites
        // truncates a >MAX_CALL_SITES file down to exactly MAX_CALL_SITES and sets
        // calls_partial=true, so a genuinely-capped file carries this flag with
        // exactly this call count. (The lift ignores the flag; it is set here only
        // so the fixture is a faithful capped-file shape, not for the bench logic.)
        calls_partial: true,
        has_unresolved_dynamic_import: false,
        content_hash: None,
    }
}

/// Measure `SAMPLES` warm iterations of `op`, report p50/p95/p99, and gate on the
/// p95 budget. Returns `true` on a breach.
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
    samples.sort_unstable();
    report_dimensions(label);
    report(label, &samples);
    gate(
        label,
        samples[percentile_index(samples.len(), 95)],
        CALL_LIFT_P95_BUDGET,
    )
}

// ---------------------------------------------------------------------------
// Corpus: `call-lift-corpus-v1`
// ---------------------------------------------------------------------------

/// Build the resident corpus: [`CALLEE_MODULES`] modules each named to match the
/// specifier the hot file calls (so cross-file callees resolve to a real resident
/// node, never a synthetic external), each carrying a public `handler` symbol the
/// hot file calls plus two internal helpers; then [`RESIDENT_FILLER_SYMBOLS`]
/// filler symbols so the graph's node count is production-representative (see the
/// const's note on `resolve_import`'s `O(nodes)` scan).
fn build_corpus() -> SymbolGraph {
    let mut graph = SymbolGraph::new();
    let mut next_id: u64 = 0;
    for i in 0..CALLEE_MODULES {
        let file = callee_module(i);
        add_symbol(
            &mut graph,
            &mut next_id,
            &file,
            "handler",
            Visibility::Public,
        );
        add_symbol(
            &mut graph,
            &mut next_id,
            &file,
            "helper_a",
            Visibility::Internal,
        );
        add_symbol(
            &mut graph,
            &mut next_id,
            &file,
            "helper_b",
            Visibility::Internal,
        );
    }
    // Pad the resident graph to a representative node count (3 symbols/file).
    for i in 0..RESIDENT_FILLER_SYMBOLS {
        let file = format!("filler/f_{}.ts", i / 3);
        add_symbol(&mut graph, &mut next_id, &file, "sym", Visibility::Internal);
    }
    graph
}

/// The non-relative specifier / file path of callee module `i`. The call's
/// `via_import` uses the same string, so `resolve_import` matches it against the
/// resident file path (its `s.file == specifier` branch) with no synthetic node.
fn callee_module(i: usize) -> String {
    format!("mod_{i}")
}

fn add_symbol(graph: &mut SymbolGraph, next_id: &mut u64, file: &str, name: &str, vis: Visibility) {
    graph
        .add_symbol(SymbolNode {
            id: *next_id,
            kind: SymbolKind::Function,
            name: name.to_string(),
            visibility: vis,
            file: file.to_string(),
            trust_level: TrustLevel::Internal,
            span: None,
        })
        .expect("corpus symbol id is unique");
    *next_id += 1;
}

/// The hot file's [`FileSymbols`]: [`HOT_FUNCS`] functions, each calling
/// [`SAME_FILE_FANOUT`] sibling functions (same-file, `via_import: None`) and one
/// resident callee module's `handler` (cross-file, `via_import: Some(mod_i)`),
/// with the matching import edges. Symbol ids are offset by [`HOT_ID_BASE`] so
/// they stay globally unique against the resident corpus (`update_file` inserts
/// ids verbatim — no rebase).
fn hot_file_symbols() -> FileSymbols {
    let mut symbols = Vec::with_capacity(HOT_FUNCS);
    let mut calls = Vec::with_capacity(HOT_FUNCS * (SAME_FILE_FANOUT + 1));
    let mut imports = Vec::with_capacity(HOT_FUNCS);

    for i in 0..HOT_FUNCS {
        symbols.push(SymbolNode {
            id: HOT_ID_BASE + i as u64,
            kind: SymbolKind::Function,
            name: format!("fn_{i}"),
            visibility: Visibility::Internal,
            file: HOT_FILE.to_string(),
            trust_level: TrustLevel::Internal,
            span: None,
        });

        let caller = LocalSymbolRef {
            kind: SymbolKind::Function,
            name: format!("fn_{i}"),
            ordinal: 0,
            module_scope: false,
        };

        // Same-file fan-out: fn_i calls fn_(i+1), fn_(i+2), … (wrap around).
        for k in 1..=SAME_FILE_FANOUT {
            let target = (i + k) % HOT_FUNCS;
            calls.push(CallSite {
                from: caller.clone(),
                callee: CalleeRef {
                    name: format!("fn_{target}"),
                    via_import: None,
                },
                line: u32::try_from(i + 1).expect("hot-file line number fits u32"),
            });
        }

        // Cross-file: fn_i imports and calls mod_(i % CALLEE_MODULES)::handler.
        let module = callee_module(i % CALLEE_MODULES);
        imports.push(ImportEdge {
            from_file: HOT_FILE.to_string(),
            to_source: module.clone(),
            line: u32::try_from(i + 1).expect("hot-file line number fits u32"),
        });
        calls.push(CallSite {
            from: caller,
            callee: CalleeRef {
                name: "handler".to_string(),
                via_import: Some(module),
            },
            line: u32::try_from(i + 1).expect("hot-file line number fits u32"),
        });
    }

    FileSymbols {
        file: HOT_FILE.to_string(),
        symbols,
        imports,
        reexports: Vec::<ReexportEdge>::new(),
        calls,
        calls_partial: false,
        has_unresolved_dynamic_import: false,
        content_hash: None,
    }
}

// ---------------------------------------------------------------------------
// Reporting + gate (shape-aligned with `hot_read` for cross-gate parity)
// ---------------------------------------------------------------------------

/// ADR-031 dimensions line so a logged measurement is self-describing.
fn report_dimensions(op: &str) {
    println!(
        "dimensions: mode=save boundary=validation.service surface=call-lift:{op} \
         contentSource=memory ruleSet=none fixtureCorpus=call-lift-corpus-v1 contentSize=0 \
         platform={} daemonState=warm driverProtocol=in-process debounceMs=0",
        std::env::consts::OS,
    );
}

/// Print p50/p95/p99 for an **already-sorted** sample slice.
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

/// Nearest-rank percentile index (same convention as the sibling gates).
fn percentile_index(len: usize, percentile: usize) -> usize {
    (len.saturating_sub(1) * percentile) / 100
}

fn fmt(duration: Duration) -> String {
    format!("{:.3}ms", duration.as_secs_f64() * 1_000.0)
}
