//! GCTX-031: Token-reduction benchmark harness.
//!
//! Measures whether identity-only graph-context delivery reduces the assistant
//! context size needed to answer a change-impact question, versus naive
//! file-reading baselines. For a fixed set of deterministic fixtures it answers
//! the same question — *"what is impacted by changing symbol X?"* — three ways
//! and counts the tokens each delivery strategy costs, using the GCTX-020
//! estimator (`estimate_gctx_tokens`) so the figures are measured with GCTX's
//! own planning budget rather than an external tokenizer:
//!
//! 1. **Naive whole-repo** — a tool-less assistant reads every source file in
//!    full. The pathological upper bound: no real assistant reads a whole repo
//!    for a one-symbol question, so this is reported only as a ceiling.
//! 2. **Naive neighbourhood** — a graph-less but savvy reader opens, in full,
//!    every file in the impacted set (the changed file plus the files holding
//!    its reverse-dependency closure). This is the meaningful baseline.
//! 3. **Graph context** — the assistant receives the identity-only JSON payload
//!    modelled on the `anvil_impact_of_change` response shape (affected-symbol
//!    identities + dependent files + a summary), with no source text.
//!
//! The impacted set is the **2-hop reverse-dependency closure** of the target,
//! matching the production cap (`MAX_REVERSE_IMPACT_DEPTH = 2` in
//! `anvil-graph-cache`). Both the neighbourhood baseline and the graph payload
//! cover that same set, so the comparison is apples-to-apples: the reduction
//! reflects only delivering identities instead of whole files.
//!
//! Honesty caveats (see the README for the full disclosure): the GCTX estimator
//! counts punctuation-dense source code via its `lexical_units` branch and
//! sparse identity text via its `bytes/4` branch, so it leans toward
//! over-counting the source baselines relative to the identity payload — real
//! BPE ratios are likely a few points lower. The figures bound the reduction for
//! identity-style impact queries on synthetic fixtures, not every assistant
//! task; snippet-bearing modes (GCTX-021..023) trade higher graph cost for
//! richer context and would narrow the ratios.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::time::Instant;

use anvil_gctx_types::{
    DependentSummary, ImpactOutcome, ImpactReport, ImpactSummary, SymbolSummary,
};
use anvil_graph_cache::estimate_gctx_tokens;
use anvil_kernel_types::{
    EdgeType, SymbolEdge, SymbolIdentity, SymbolKind, SymbolNode, TrustLevel, Visibility,
};

use crate::measure::MemoryGuard;
use crate::report::ScenarioResult;

/// Reverse-impact depth modelled, matching `MAX_REVERSE_IMPACT_DEPTH = 2` in
/// `anvil-graph-cache`'s hot index — the cap the production
/// `anvil_impact_of_change` tool applies.
const MAX_IMPACT_DEPTH: u32 = 2;

/// Prime stride for deterministic, cross-file dependency selection.
const DEP_STRIDE: usize = 7;

/// A fixed, named fixture: a deterministic synthetic project plus the change
/// set (the most-depended-on symbol) whose impact we deliver three ways.
#[derive(Debug, Clone)]
pub struct FixtureSpec {
    /// Stable fixture name, used as the metric prefix. Must be unique within a
    /// config.
    pub name: &'static str,
    /// Number of source files in the synthetic project.
    pub files: usize,
    /// Functions (symbols) per file.
    pub symbols_per_file: usize,
    /// Outgoing dependency edges per symbol (call/import fan-out). Must be > 0.
    pub fanout: usize,
}

/// Configuration: the fixed fixture set the harness measures.
#[derive(Debug, Clone)]
pub struct TokenReductionConfig {
    pub fixtures: Vec<FixtureSpec>,
}

impl Default for TokenReductionConfig {
    fn default() -> Self {
        Self {
            fixtures: vec![
                // A small library: change a leaf used by a handful of callers.
                FixtureSpec {
                    name: "small_lib",
                    files: 8,
                    symbols_per_file: 4,
                    fanout: 2,
                },
                // A layered application: moderate fan-out, mid-layer change.
                FixtureSpec {
                    name: "layered_app",
                    files: 20,
                    symbols_per_file: 5,
                    fanout: 3,
                },
                // A wide-fan-out core: one heavily depended-on symbol changes.
                FixtureSpec {
                    name: "wide_fanout",
                    files: 12,
                    symbols_per_file: 6,
                    fanout: 4,
                },
            ],
        }
    }
}

/// A generated fixture: the graph substrate plus the source files rendered from
/// it, and the change-set target (the highest in-degree symbol).
#[derive(Debug, Clone)]
struct Fixture {
    nodes: Vec<SymbolNode>,
    edges: Vec<SymbolEdge>,
    /// Path -> full source text, rendered from `nodes` + `edges`.
    files: BTreeMap<String, String>,
    /// Symbol id of the change-set target.
    target: u64,
}

/// Symbol ids index `nodes` directly; they are small synthetic counters, so a
/// checked conversion is exact. Centralised so the casts stay lint-clean.
fn node_idx(id: u64) -> usize {
    usize::try_from(id).expect("symbol id fits in usize")
}

/// Build a fixture deterministically from a spec. The symbol graph is the single
/// source of truth; source files are rendered from it.
fn build_fixture(spec: &FixtureSpec) -> Fixture {
    let total = spec.files * spec.symbols_per_file;
    assert!(total > 1, "fixture must have at least two symbols");
    assert!(
        spec.fanout > 0,
        "FixtureSpec.fanout must be > 0 to guarantee a non-empty impact set"
    );

    let mut nodes = Vec::with_capacity(total);
    for i in 0..total {
        nodes.push(SymbolNode {
            id: i as u64,
            kind: SymbolKind::Function,
            name: format!("sym_{i}"),
            visibility: if i % 3 == 0 {
                Visibility::Public
            } else {
                Visibility::Internal
            },
            file: format!("src/mod_{}.rs", i / spec.symbols_per_file),
            trust_level: TrustLevel::Internal,
            span: None,
        });
    }
    // node_idx() assumes id == position; assert the invariant at construction.
    debug_assert!(
        nodes.iter().enumerate().all(|(i, n)| n.id == i as u64),
        "node ids must equal their vec indices — invariant required by node_idx()"
    );

    // Deterministic cross-file dependency edges: symbol i depends on
    // `fanout` later symbols chosen by a prime stride, skipping self.
    let mut edges = Vec::new();
    for i in 0..total {
        for k in 1..=spec.fanout {
            let target = (i + k * DEP_STRIDE) % total;
            if target == i {
                continue;
            }
            edges.push(SymbolEdge {
                from: i as u64,
                to: target as u64,
                edge_type: if k % 2 == 0 {
                    EdgeType::Imports
                } else {
                    EdgeType::Calls
                },
            });
        }
    }

    // Change-set target: the most depended-on symbol (highest in-degree). This
    // represents the strongest, most realistic question — "what depends on this
    // core symbol?" — and guarantees a non-empty impact set.
    let mut in_degree = vec![0usize; total];
    for e in &edges {
        in_degree[node_idx(e.to)] += 1;
    }
    let target = in_degree
        .iter()
        .enumerate()
        .max_by_key(|(idx, deg)| (**deg, std::cmp::Reverse(*idx)))
        .map_or(0, |(idx, _)| idx as u64);

    let files = render_files(&nodes, &edges, spec.symbols_per_file);

    Fixture {
        nodes,
        edges,
        files,
        target,
    }
}

/// Render full source text per file from the graph. Each node becomes a function
/// whose body references its outgoing-edge targets, so file byte sizes track the
/// graph structure realistically. Imports are de-duplicated per file so two
/// symbols sharing a dependency do not double-count.
fn render_files(
    nodes: &[SymbolNode],
    edges: &[SymbolEdge],
    symbols_per_file: usize,
) -> BTreeMap<String, String> {
    // Collect imports per file first (de-duplicated), then render bodies.
    let mut imports: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for node in nodes {
        let set = imports.entry(node.file.clone()).or_default();
        for e in edges.iter().filter(|e| e.from == node.id) {
            let dep = &nodes[node_idx(e.to)];
            if dep.file != node.file {
                set.insert(format!(
                    "use crate::{}::{};",
                    module_stem(&dep.file),
                    dep.name
                ));
            }
        }
    }

    let mut by_file: BTreeMap<String, String> = BTreeMap::new();
    for (i, node) in nodes.iter().enumerate() {
        let entry = by_file.entry(node.file.clone()).or_default();
        if entry.is_empty() {
            let _ = writeln!(entry, "// module {}", node.file);
            if let Some(uses) = imports.get(&node.file) {
                for u in uses {
                    let _ = writeln!(entry, "{u}");
                }
            }
        }

        let deps: Vec<&SymbolNode> = edges
            .iter()
            .filter(|e| e.from == node.id)
            .map(|e| &nodes[node_idx(e.to)])
            .collect();

        let vis = visibility_kw(node.visibility);
        let _ = writeln!(entry, "{vis}fn {}(input: u64) -> u64 {{", node.name);
        let mut acc = format!("input.wrapping_add({i})");
        for (n, dep) in deps.iter().enumerate() {
            let _ = writeln!(entry, "    let v{n} = {}({acc});", dep.name);
            acc = format!("v{n}");
        }
        let _ = writeln!(entry, "    {acc}.wrapping_mul({symbols_per_file})");
        let _ = writeln!(entry, "}}");
        let _ = writeln!(entry);
    }

    by_file
}

fn module_stem(file: &str) -> String {
    file.trim_start_matches("src/")
        .trim_end_matches(".rs")
        .to_string()
}

fn visibility_kw(v: Visibility) -> &'static str {
    match v {
        Visibility::Public => "pub ",
        Visibility::Internal => "",
    }
}

/// The result of answering the change-impact question three ways for one fixture.
#[derive(Debug, Clone)]
struct FixtureAnalysis {
    files: usize,
    symbols: usize,
    /// Symbols defined in the changed file (`ImpactReport.affected_symbols`).
    change_surface: usize,
    /// Files in the reverse-impact closure (`ImpactReport.dependent_files`).
    dependent_files: usize,
    neighbourhood_files: usize,
    whole_repo_tokens: usize,
    neighbourhood_tokens: usize,
    graph_tokens: usize,
}

impl FixtureAnalysis {
    fn reduction_pct(&self, baseline: usize) -> f64 {
        if baseline == 0 {
            return 0.0;
        }
        (1.0 - (self.graph_tokens as f64 / baseline as f64)) * 100.0
    }
}

/// Token estimate using the GCTX-020 estimator. Inputs here are well under the
/// estimator's 64 KiB input cap; exceeding it is a fixture-sizing error, not a
/// condition to silently paper over with a different formula.
fn tokens(text: &str) -> usize {
    estimate_gctx_tokens(text, None)
        .expect("fixture input exceeds the GCTX estimator cap — reduce fixture size")
        .tokens
}

/// The production "change surface": identity summaries of the symbols **defined
/// in** the changed file (`ImpactReport.affected_symbols`), in file/parse order.
fn change_surface(fx: &Fixture) -> Vec<SymbolSummary> {
    let target_file = &fx.nodes[node_idx(fx.target)].file;
    fx.nodes
        .iter()
        .filter(|n| &n.file == target_file)
        .map(|n| SymbolSummary {
            identity: SymbolIdentity {
                file: n.file.clone(),
                kind: n.kind,
                name: n.name.clone(),
                ordinal: 0,
            },
            visibility: n.visibility,
        })
        .collect()
}

/// The file-keyed reverse-impact closure of the changed file with hop distance
/// (`ImpactReport.dependent_files`): files that import the changed file
/// transitively, bounded by [`MAX_IMPACT_DEPTH`], excluding the changed file,
/// ordered by path.
fn dependent_files(fx: &Fixture) -> Vec<DependentSummary> {
    let target_file = fx.nodes[node_idx(fx.target)].file.clone();

    // importers_of[f] = files importing f (a symbol in the importer depends on a
    // symbol in f), excluding same-file edges.
    let mut importers_of: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    for e in &fx.edges {
        let from_file = fx.nodes[node_idx(e.from)].file.as_str();
        let to_file = fx.nodes[node_idx(e.to)].file.as_str();
        if from_file != to_file {
            importers_of.entry(to_file).or_default().insert(from_file);
        }
    }

    let mut dist: BTreeMap<String, u32> = BTreeMap::new();
    let mut frontier: BTreeSet<String> = BTreeSet::from([target_file.clone()]);
    for depth in 1..=MAX_IMPACT_DEPTH {
        let mut next: BTreeSet<String> = BTreeSet::new();
        for f in &frontier {
            for &imp in importers_of.get(f.as_str()).into_iter().flatten() {
                if imp != target_file && !dist.contains_key(imp) {
                    dist.insert(imp.to_string(), depth);
                    next.insert(imp.to_string());
                }
            }
        }
        if next.is_empty() {
            break;
        }
        frontier = next;
    }

    // BTreeMap iterates by key, so this is path-ordered.
    dist.into_iter()
        .map(|(file, distance)| DependentSummary { file, distance })
        .collect()
}

/// Build the identity-only graph-context payload: the **real** production
/// `ImpactOutcome::Ready(ImpactReport)` serialised compact, so the token count
/// reflects the exact wire shape `anvil_impact_of_change` emits (`snake_case`,
/// status-tagged, file-keyed `dependent_files` with distance — not a lookalike).
/// The change-invariant response envelope (`workspace_assurance` +
/// `workspace_root`) is excluded; it does not vary across fixtures.
fn impact_payload(fx: &Fixture) -> String {
    let affected_symbols = change_surface(fx);
    let deps = dependent_files(fx);

    let summary = ImpactSummary {
        changed_files: 1,
        affected_symbols: affected_symbols.len(),
        dependent_files: deps.len(),
        known_tests: 0,
        truncated: false,
        omitted_sensitive_paths: 0,
    };
    let report = ImpactReport {
        affected_symbols,
        dependent_files: deps,
        known_tests: Vec::new(),
        summary,
    };

    serde_json::to_string(&ImpactOutcome::Ready(report)).expect("impact outcome serialises")
}

fn analyze(fx: &Fixture) -> FixtureAnalysis {
    let target_file = fx.nodes[node_idx(fx.target)].file.clone();
    let surface = change_surface(fx);
    let deps = dependent_files(fx);

    // Whole-repo baseline: every file read in full (per-file estimates summed,
    // mirroring an assistant loading each file as a separate context chunk).
    let whole_repo_tokens: usize = fx.files.values().map(|src| tokens(src)).sum();

    // Neighbourhood baseline: the changed file plus every dependent file, read in
    // full — the same impacted set the graph payload covers.
    let mut neighbourhood: BTreeSet<&str> = BTreeSet::from([target_file.as_str()]);
    for d in &deps {
        neighbourhood.insert(d.file.as_str());
    }
    let neighbourhood_tokens: usize = neighbourhood
        .iter()
        .filter_map(|f| fx.files.get(*f))
        .map(|src| tokens(src))
        .sum();

    // Graph context: the real identity-only impact payload.
    let graph_tokens = tokens(&impact_payload(fx));

    FixtureAnalysis {
        files: fx.files.len(),
        symbols: fx.nodes.len(),
        change_surface: surface.len(),
        dependent_files: deps.len(),
        neighbourhood_files: neighbourhood.len(),
        whole_repo_tokens,
        neighbourhood_tokens,
        graph_tokens,
    }
}

/// Run the token-reduction scenario over the configured fixtures.
pub fn run(config: &TokenReductionConfig) -> ScenarioResult {
    let unique: BTreeSet<&str> = config.fixtures.iter().map(|f| f.name).collect();
    assert_eq!(
        unique.len(),
        config.fixtures.len(),
        "fixture names must be unique — they are used as metric prefixes"
    );

    let mem = MemoryGuard::start();
    let start = Instant::now();

    let mut result = ScenarioResult::new("token_reduction");

    let mut sum_reduction_whole = 0.0;
    let mut sum_reduction_neighbourhood = 0.0;
    let mut counted = 0u32;

    for spec in &config.fixtures {
        let fx = build_fixture(spec);
        let a = analyze(&fx);

        let p = spec.name;
        result.add_metric(&format!("{p}_files"), a.files as f64, "count");
        result.add_metric(&format!("{p}_symbols"), a.symbols as f64, "count");
        result.add_metric(
            &format!("{p}_affected_symbols"),
            a.change_surface as f64,
            "count",
        );
        result.add_metric(
            &format!("{p}_dependent_files"),
            a.dependent_files as f64,
            "count",
        );
        result.add_metric(
            &format!("{p}_neighbourhood_files"),
            a.neighbourhood_files as f64,
            "count",
        );
        result.add_metric(
            &format!("{p}_baseline_whole_repo_tokens"),
            a.whole_repo_tokens as f64,
            "tokens",
        );
        result.add_metric(
            &format!("{p}_baseline_neighbourhood_tokens"),
            a.neighbourhood_tokens as f64,
            "tokens",
        );
        result.add_metric(
            &format!("{p}_graph_context_tokens"),
            a.graph_tokens as f64,
            "tokens",
        );

        let r_whole = a.reduction_pct(a.whole_repo_tokens);
        let r_neighbourhood = a.reduction_pct(a.neighbourhood_tokens);
        result.add_metric(&format!("{p}_reduction_vs_whole_repo_pct"), r_whole, "pct");
        result.add_metric(
            &format!("{p}_reduction_vs_neighbourhood_pct"),
            r_neighbourhood,
            "pct",
        );

        sum_reduction_whole += r_whole;
        sum_reduction_neighbourhood += r_neighbourhood;
        counted += 1;
    }

    if counted > 0 {
        result.add_metric(
            "mean_reduction_vs_whole_repo_pct",
            sum_reduction_whole / f64::from(counted),
            "pct",
        );
        result.add_metric(
            "mean_reduction_vs_neighbourhood_pct",
            sum_reduction_neighbourhood / f64::from(counted),
            "pct",
        );
    }

    let mem_delta = mem.finish();
    result.set_duration(start.elapsed());
    result.add_memory("token_reduction", &mem_delta);

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_spec() -> FixtureSpec {
        FixtureSpec {
            name: "sample",
            files: 6,
            symbols_per_file: 4,
            fanout: 3,
        }
    }

    fn metric(result: &ScenarioResult, name: &str) -> f64 {
        result
            .metrics
            .iter()
            .find(|m| m.name == name)
            .unwrap_or_else(|| panic!("metric {name} missing"))
            .value
    }

    #[test]
    fn fixture_generation_is_deterministic() {
        let spec = sample_spec();
        let a = build_fixture(&spec);
        let b = build_fixture(&spec);

        assert_eq!(a.target, b.target);
        assert_eq!(a.files, b.files);
        assert_eq!(a.nodes.len(), b.nodes.len());
        assert_eq!(a.edges.len(), b.edges.len());
    }

    #[test]
    fn impact_sets_are_non_empty_and_well_formed() {
        let fx = build_fixture(&sample_spec());
        let target_file = fx.nodes[node_idx(fx.target)].file.clone();

        let surface = change_surface(&fx);
        assert!(
            !surface.is_empty(),
            "the changed file must define at least one symbol"
        );
        assert!(surface.iter().all(|s| s.identity.file == target_file));

        let deps = dependent_files(&fx);
        assert!(
            !deps.is_empty(),
            "the change-set target must have a non-empty reverse-impact closure"
        );
        // Distances are within the cap and the changed file is excluded.
        assert!(
            deps.iter()
                .all(|d| d.distance >= 1 && d.distance <= MAX_IMPACT_DEPTH)
        );
        assert!(deps.iter().all(|d| d.file != target_file));
        // dependent_files is path-ordered (BTreeMap-sourced).
        let mut sorted = deps.clone();
        sorted.sort_by(|a, b| a.file.cmp(&b.file));
        assert_eq!(
            deps.iter().map(|d| &d.file).collect::<Vec<_>>(),
            sorted.iter().map(|d| &d.file).collect::<Vec<_>>()
        );
    }

    #[test]
    fn payload_matches_production_outcome_shape() {
        // The payload must deserialise back into the real ImpactOutcome — proof
        // it is the production wire shape, not a lookalike.
        let fx = build_fixture(&sample_spec());
        let payload = impact_payload(&fx);
        let parsed: ImpactOutcome =
            serde_json::from_str(&payload).expect("payload is a real ImpactOutcome");
        match parsed {
            ImpactOutcome::Ready(report) => {
                assert_eq!(
                    report.summary.affected_symbols,
                    report.affected_symbols.len()
                );
                assert_eq!(report.summary.dependent_files, report.dependent_files.len());
            }
            other => panic!("expected Ready, got {other:?}"),
        }
        // Snake_case, status-tagged wire form (not camelCase).
        assert!(payload.contains("\"status\":\"ready\""));
        assert!(payload.contains("\"dependent_files\""));
        assert!(payload.contains("\"affected_symbols\""));
    }

    #[test]
    fn graph_context_beats_both_baselines() {
        let fx = build_fixture(&sample_spec());
        let a = analyze(&fx);

        assert!(
            a.graph_tokens < a.neighbourhood_tokens,
            "graph context ({}) must cost fewer tokens than the neighbourhood baseline ({})",
            a.graph_tokens,
            a.neighbourhood_tokens
        );
        assert!(
            a.neighbourhood_tokens <= a.whole_repo_tokens,
            "neighbourhood ({}) must not exceed whole-repo ({})",
            a.neighbourhood_tokens,
            a.whole_repo_tokens
        );
        assert!(a.graph_tokens < a.whole_repo_tokens);
    }

    #[test]
    fn reductions_clear_defensible_floors() {
        // Floors derived from the recorded default-fixture results; they guard
        // against a regression silently eroding the headline claim (the GCTX-031
        // risk). They are deliberately well below the observed values.
        let result = run(&TokenReductionConfig::default());
        assert!(
            metric(&result, "mean_reduction_vs_neighbourhood_pct") > 70.0,
            "mean neighbourhood reduction fell below the 70% floor"
        );
        assert!(
            metric(&result, "mean_reduction_vs_whole_repo_pct") > 85.0,
            "mean whole-repo reduction fell below the 85% floor"
        );
    }

    #[test]
    fn estimator_inputs_stay_within_cap() {
        let fx = build_fixture(&sample_spec());
        for src in fx.files.values() {
            assert!(
                estimate_gctx_tokens(src, None).is_ok(),
                "per-file source should fit the estimator input cap"
            );
        }
        let payload = impact_payload(&fx);
        assert!(estimate_gctx_tokens(&payload, None).is_ok());
    }

    #[test]
    fn run_rejects_duplicate_fixture_names() {
        let config = TokenReductionConfig {
            fixtures: vec![
                FixtureSpec {
                    name: "dup",
                    files: 4,
                    symbols_per_file: 3,
                    fanout: 2,
                },
                FixtureSpec {
                    name: "dup",
                    files: 5,
                    symbols_per_file: 3,
                    fanout: 2,
                },
            ],
        };
        assert!(std::panic::catch_unwind(|| run(&config)).is_err());
    }

    #[test]
    fn run_emits_reduction_and_mean_metrics() {
        let config = TokenReductionConfig {
            fixtures: vec![
                FixtureSpec {
                    name: "fx_a",
                    files: 5,
                    symbols_per_file: 3,
                    fanout: 2,
                },
                FixtureSpec {
                    name: "fx_b",
                    files: 8,
                    symbols_per_file: 4,
                    fanout: 3,
                },
            ],
        };

        let result = run(&config);
        assert_eq!(result.scenario, "token_reduction");

        let has = |name: &str| result.metrics.iter().any(|m| m.name == name);
        assert!(has("fx_a_reduction_vs_whole_repo_pct"));
        assert!(has("fx_a_reduction_vs_neighbourhood_pct"));
        assert!(has("fx_b_graph_context_tokens"));
        assert!(has("mean_reduction_vs_whole_repo_pct"));
        assert!(has("mean_reduction_vs_neighbourhood_pct"));
    }

    #[test]
    fn default_fixture_token_counts_are_stable() {
        // Golden values: any change to the fixtures, rendering, estimator, or
        // payload shape must update both these constants AND the README table in
        // the same commit. This ties the published numbers to the code.
        let result = run(&TokenReductionConfig::default());

        // (whole_repo, neighbourhood, graph) per fixture.
        let expected = [
            ("small_lib", GOLDEN_SMALL_LIB),
            ("layered_app", GOLDEN_LAYERED_APP),
            ("wide_fanout", GOLDEN_WIDE_FANOUT),
        ];
        // Token counts are whole numbers stored as f64; compare within half a
        // unit (exact for integer counts, clippy-clean vs a float `==`).
        let eq = |got: f64, want: usize, label: &str| {
            assert!(
                (got - want as f64).abs() < 0.5,
                "{label} drifted: got {got}, want {want} — update the README table too"
            );
        };
        for (name, (whole, neighbourhood, graph)) in expected {
            eq(
                metric(&result, &format!("{name}_baseline_whole_repo_tokens")),
                whole,
                &format!("{name} whole_repo tokens"),
            );
            eq(
                metric(&result, &format!("{name}_baseline_neighbourhood_tokens")),
                neighbourhood,
                &format!("{name} neighbourhood tokens"),
            );
            eq(
                metric(&result, &format!("{name}_graph_context_tokens")),
                graph,
                &format!("{name} graph tokens"),
            );
        }
    }

    // (whole_repo, neighbourhood, graph) — see README token_reduction table.
    const GOLDEN_SMALL_LIB: (usize, usize, usize) = (2163, 2163, 410);
    const GOLDEN_LAYERED_APP: (usize, usize, usize) = (8614, 4738, 520);
    const GOLDEN_WIDE_FANOUT: (usize, usize, usize) = (7548, 6919, 570);
}
