//! GCALL-003 — bounded caller-traversal read API over resident `Calls` edges
//! (ADR-086).
//!
//! "Who calls this symbol", at symbol granularity, over the `EdgeType::Calls`
//! edges that [`crate::incremental`] lifts into the [`SymbolGraph`]. The shape
//! mirrors the GCTX-011 reverse-impact walk one level finer: a breadth-first
//! walk over **incoming** `Calls` edges, clamped by the GV2-026
//! [`clamp_reverse_impact_depth`](crate::clamp_reverse_impact_depth) lever and a
//! node budget, with a `seen` set (so recursion/cycles terminate) and
//! identity-sorted frontiers. The walk is **distance-first** (BFS by hop) and
//! identity-ordered within a hop, so an over-budget truncation keeps a
//! deterministic **nearest-callers-first** prefix in `(distance, identity)`
//! order — the right callers to keep for "who calls this" (direct before
//! transitive). Output is **identity-only** — calling-symbol [`SymbolIdentity`]
//! plus traversal distance — the substrate GCTX-014 (`anvil_find_callers`)
//! projects.

use anvil_kernel_types::{EdgeType, SymbolIdentity};

use crate::hot_index::MAX_REVERSE_IMPACT_DEPTH;
use crate::symbol_graph::SymbolGraph;

/// Node-visit budget for one caller traversal — the lock-held walk cannot grow
/// without bound on a pathologically-called symbol (ADR-031). Matches the
/// GCTX-011/013 reverse-impact bound.
pub const MAX_CALLERS_WALK: usize = 10_000;

/// One caller of the queried symbol: its identity, the traversal distance
/// (hops) from the target, and whether the edge reaching it is an overload
/// fan-out (GCALL-007 CALL-1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CallerResult {
    /// The calling symbol (identity-only).
    pub caller: SymbolIdentity,
    /// Distance in `Calls`-edge hops from the queried target (1 = direct caller).
    pub distance: u32,
    /// True when the edge this caller was reached through is an **overload
    /// fan-out** — the caller has `Calls` edges to two or more symbols sharing
    /// the called symbol's `(file, kind, name)`, so the static resolver could not
    /// pick one overload and attached the call to all (ADR-086 §1). A consumer
    /// must not treat a `heuristic` caller as an exact call (GCALL-007 CALL-1).
    /// Conservative: a caller that genuinely calls two distinct overloads is also
    /// flagged — the marker never *under*-reports fan-out, the safe direction.
    pub heuristic: bool,
}

/// The bounded result of a caller traversal.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CallersReport {
    /// Callers in deterministic **nearest-first** order: ascending traversal
    /// `(distance, caller identity)`, each caller appearing once at its minimum
    /// distance. `callers[0]` is therefore always a closest (smallest-distance)
    /// caller — the order a `find_callers` consumer relies on to read direct
    /// callers before transitive ones.
    pub callers: Vec<CallerResult>,
    /// Whether the node budget bound the walk. The returned set is then a
    /// deterministic **nearest-first** prefix — all callers at smaller distances,
    /// identity-ordered within each distance — never a silent cutoff. (It is not a
    /// global identity-ordered prefix: a distance-2 caller sorting earlier
    /// alphabetically is still dropped before a kept distance-1 caller.)
    pub truncated: bool,
}

/// Walk the resident `Calls` edges to find the callers of `target` up to `depth`
/// hops.
///
/// `depth` MUST be caller-clamped to the GV2-026 ceiling
/// ([`clamp_reverse_impact_depth`](crate::clamp_reverse_impact_depth)); a
/// `debug_assert` enforces it. Recursion and cycles terminate via the `seen`
/// set; the walk is distance-first and each hop is expanded in identity order, so
/// an over-budget truncation keeps a deterministic nearest-first prefix (all
/// nearer callers, identity-ordered within a distance). A `target` that is not
/// resident yields an empty report.
#[must_use]
pub fn callers_of(graph: &SymbolGraph, target: &SymbolIdentity, depth: u32) -> CallersReport {
    // `depth` is contractually caller-clamped to the GV2-026 ceiling; the
    // `debug_assert` catches a caller that forgot in debug/test builds, and the
    // `.min` makes a release build self-correcting (a stray over-depth caller
    // walks the clamped budget, never an unbounded BFS) rather than silently
    // honouring an out-of-contract depth.
    debug_assert!(
        depth <= MAX_REVERSE_IMPACT_DEPTH,
        "callers_of depth {depth} exceeds the GV2-026 cap {MAX_REVERSE_IMPACT_DEPTH} (caller must clamp)",
    );
    let depth = depth.min(MAX_REVERSE_IMPACT_DEPTH);

    let mut identities = IdentityCache::new(graph);
    let Some(target_id) = node_for_identity(graph, target) else {
        return CallersReport::default();
    };

    let mut callers: Vec<CallerResult> = Vec::new();
    let mut truncated = false;
    // `seen` excludes the target from its own caller set and terminates cycles
    // (a recursive `a → a` re-reaching `a` is skipped).
    let mut seen: std::collections::HashSet<u64> = std::collections::HashSet::new();
    seen.insert(target_id);
    let mut frontier: Vec<u64> = vec![target_id];

    'walk: for hop in 1..=depth {
        // Collect this hop's new callers, deduplicated by caller node id. A caller
        // reachable via several frontier nodes in one hop is recorded once, with
        // `heuristic` **OR-ed** across all its edges — so a caller is heuristic if
        // *any* of its calls into the frontier is an overload fan-out (a
        // deterministic, conservative result independent of frontier-visit order).
        let mut hop_callers: std::collections::HashMap<u64, (SymbolIdentity, bool)> =
            std::collections::HashMap::new();
        for &current in &frontier {
            for edge in graph.incoming_edges(current) {
                if edge.edge_type != EdgeType::Calls || seen.contains(&edge.from) {
                    continue;
                }
                // Identity is resolved through a per-file-memoised cache, not
                // rebuilt from scratch per edge — a hot symbol with thousands of
                // callers across few files pays the file-symbol scan once per file,
                // not once per caller node, on this lock-held read path.
                if let Some(identity) = identities.get(edge.from) {
                    let heuristic = is_fan_out_call(graph, edge.from, current);
                    hop_callers
                        .entry(edge.from)
                        .and_modify(|entry| entry.1 |= heuristic)
                        .or_insert((identity, heuristic));
                }
            }
        }
        // Expand in identity order so an over-budget truncation keeps a
        // deterministic prefix.
        let mut next_ids: Vec<(u64, SymbolIdentity, bool)> = hop_callers
            .into_iter()
            .map(|(id, (identity, heuristic))| (id, identity, heuristic))
            .collect();
        next_ids.sort_by(|a, b| a.1.cmp(&b.1));

        let mut next_frontier: Vec<u64> = Vec::new();
        for (id, identity, heuristic) in next_ids {
            // `hop_callers` keys are unique and already excluded prior-hop callers;
            // `seen.insert` records this caller for the next hop's cycle guard.
            if !seen.insert(id) {
                continue;
            }
            callers.push(CallerResult {
                caller: identity,
                distance: hop,
                heuristic,
            });
            next_frontier.push(id);
            if callers.len() >= MAX_CALLERS_WALK {
                truncated = true;
                break 'walk;
            }
        }
        if next_frontier.is_empty() {
            break;
        }
        frontier = next_frontier;
    }

    // Order nearest-first: ascending `(distance, identity)`. The BFS already
    // appends in hop order, but a final sort makes the `(distance, identity)`
    // total order explicit and independent of frontier-visit order, so
    // `callers[0]` is always a closest caller (matching the rustdoc contract a
    // consumer reads).
    callers.sort_by(|a, b| {
        a.distance
            .cmp(&b.distance)
            .then_with(|| a.caller.cmp(&b.caller))
    });

    if truncated {
        // Counts only (no identities/paths) — PV-10 telemetry posture. Lets an
        // operator see that a caller result was budget-bound without a profiler.
        tracing::debug!(
            target: "anvil_graph_cache::call_graph",
            returned = callers.len(),
            budget = MAX_CALLERS_WALK,
            depth,
            "callers_of truncated by node budget"
        );
    }

    CallersReport { callers, truncated }
}

/// Resolve the innermost symbol whose defining-node span contains
/// `byte_offset` in `file`, or `None` if no resident symbol covers it.
///
/// Mirrors the "innermost span containing an offset" algorithm the Rust
/// extractor already uses internally to attribute call sites to their
/// enclosing caller (`caller_at` in
/// `crates/anvil-kernel/src/parser/extract/rust.rs`), but as a general,
/// reusable query over the **resident** [`SymbolGraph`] (via
/// [`SymbolGraph::symbols_in_file`]) rather than a transient per-file
/// extraction-time vector — the shape GCTX-provider `textDocument/references`
/// needs (position → [`SymbolIdentity`] → [`callers_of`]).
///
/// Spans are `[start, end)` half-open byte offsets; a symbol with no recorded
/// span (`span: None` — synthetic/imported/reconstructed nodes) is never a
/// candidate. Ties (nested symbols of equal width, which does not
/// happen for well-formed source but is not assumed) resolve to whichever the
/// file's symbol list visits first, deterministically.
#[must_use]
pub fn symbol_at_offset(
    graph: &SymbolGraph,
    file: &str,
    byte_offset: u32,
) -> Option<SymbolIdentity> {
    let symbols = graph.symbols_in_file(file);
    let identities = SymbolIdentity::for_file_symbols(&symbols);
    identities
        .into_iter()
        .zip(symbols.iter())
        .filter_map(|(identity, node)| {
            let span = node.span?;
            (span.start <= byte_offset && byte_offset < span.end)
                .then_some((identity, span.end - span.start))
        })
        .min_by_key(|(_, width)| *width)
        .map(|(identity, _)| identity)
}

/// Resolve a [`SymbolIdentity`] to its resident node id, or `None` if absent.
fn node_for_identity(graph: &SymbolGraph, target: &SymbolIdentity) -> Option<u64> {
    let symbols = graph.symbols_in_file(&target.file);
    let identities = SymbolIdentity::for_file_symbols(&symbols);
    identities
        .iter()
        .zip(symbols.iter())
        .find(|(id, _)| *id == target)
        .map(|(_, node)| node.id)
}

/// A per-traversal node-id → [`SymbolIdentity`] cache over one [`SymbolGraph`].
///
/// There is no resident node→identity index (identities are assigned over a
/// file's full parse-ordered symbol list), so recovering one node's identity
/// costs an O(file-symbols) scan. In a caller walk a hot symbol's callers cluster
/// into relatively few files, so memoising **per file** — the first lookup for a
/// file materialises every symbol in it — turns O(callers × file-symbols) into
/// O(distinct-caller-files × file-symbols + callers), the win
/// [`callers_of`] needs on the lock-held read path (council CR-3/PL-3/OPS-4).
struct IdentityCache<'g> {
    graph: &'g SymbolGraph,
    by_id: std::collections::HashMap<u64, SymbolIdentity>,
    files_loaded: std::collections::HashSet<String>,
}

impl<'g> IdentityCache<'g> {
    fn new(graph: &'g SymbolGraph) -> Self {
        Self {
            graph,
            by_id: std::collections::HashMap::new(),
            files_loaded: std::collections::HashSet::new(),
        }
    }

    /// The identity of resident node `id`, or `None` if absent. On the first
    /// lookup into a file, every symbol in that file is identity-mapped at once.
    fn get(&mut self, id: u64) -> Option<SymbolIdentity> {
        if let Some(identity) = self.by_id.get(&id) {
            return Some(identity.clone());
        }
        let file = self.graph.get_symbol(id)?.file.clone();
        if self.files_loaded.insert(file.clone()) {
            let symbols = self.graph.symbols_in_file(&file);
            let identities = SymbolIdentity::for_file_symbols(&symbols);
            for (node, identity) in symbols.iter().zip(identities) {
                self.by_id.insert(node.id, identity);
            }
        }
        self.by_id.get(&id).cloned()
    }
}

/// Whether `caller`'s `Calls` edge to `callee` is an **overload fan-out** — i.e.
/// `caller` has two or more `Calls` edges to symbols sharing `callee`'s
/// `(file, kind, name)` (GCALL-007 CALL-1). The resident edge carries no
/// provenance, so this is a conservative read-time signal: a caller that
/// genuinely calls two distinct overloads is also flagged, but a real fan-out is
/// never missed (the safe direction for an honesty marker).
fn is_fan_out_call(graph: &SymbolGraph, from_id: u64, to_id: u64) -> bool {
    let Some(called) = graph.get_symbol(to_id) else {
        return false;
    };
    let siblings = graph
        .outgoing_edges(from_id)
        .into_iter()
        .filter(|edge| edge.edge_type == EdgeType::Calls)
        .filter(|edge| {
            graph.get_symbol(edge.to).is_some_and(|n| {
                n.file == called.file && n.kind == called.kind && n.name == called.name
            })
        })
        .count();
    siblings > 1
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::incremental::update_file;
    use anvil_kernel_types::{
        CallSite, CalleeRef, FileSymbols, LocalSymbolRef, SymbolKind, SymbolNode, TrustLevel,
        Visibility,
    };

    fn func(id: u64, name: &str, file: &str) -> SymbolNode {
        SymbolNode {
            id,
            kind: SymbolKind::Function,
            name: name.to_string(),
            visibility: Visibility::Internal,
            file: file.to_string(),
            trust_level: TrustLevel::Unknown,
            span: None,
        }
    }

    fn caller_ref(name: &str) -> LocalSymbolRef {
        LocalSymbolRef {
            kind: SymbolKind::Function,
            name: name.to_string(),
            ordinal: 0,
            module_scope: false,
        }
    }

    /// `b` and `c` call `a` (same file); `d` calls `b`. Direct callers of `a` at
    /// depth 1 are `b`, `c`; at depth 2 also `d`.
    fn fixture() -> SymbolGraph {
        let mut g = SymbolGraph::new();
        let file = "a.ts";
        let calls = vec![
            CallSite {
                from: caller_ref("b"),
                callee: CalleeRef {
                    name: "a".into(),
                    via_import: None,
                },
                line: 1,
            },
            CallSite {
                from: caller_ref("c"),
                callee: CalleeRef {
                    name: "a".into(),
                    via_import: None,
                },
                line: 2,
            },
            CallSite {
                from: caller_ref("d"),
                callee: CalleeRef {
                    name: "b".into(),
                    via_import: None,
                },
                line: 3,
            },
        ];
        update_file(
            &mut g,
            FileSymbols {
                file: file.to_string(),
                symbols: vec![
                    func(0, "a", file),
                    func(1, "b", file),
                    func(2, "c", file),
                    func(3, "d", file),
                ],
                imports: Vec::new(),
                reexports: Vec::new(),
                calls,
                calls_partial: false,
                has_unresolved_dynamic_import: false,
                content_hash: None,
            },
        );
        g
    }

    fn identity(name: &str) -> SymbolIdentity {
        SymbolIdentity {
            file: "a.ts".into(),
            kind: SymbolKind::Function,
            name: name.into(),
            ordinal: 0,
        }
    }

    #[test]
    fn direct_callers_at_depth_1() {
        let g = fixture();
        let report = callers_of(&g, &identity("a"), 1);
        let names: Vec<&str> = report
            .callers
            .iter()
            .map(|c| c.caller.name.as_str())
            .collect();
        assert_eq!(names, ["b", "c"]);
        assert!(report.callers.iter().all(|c| c.distance == 1));
        assert!(!report.truncated);
        // Unambiguous direct calls are not heuristic.
        assert!(report.callers.iter().all(|c| !c.heuristic));
    }

    #[test]
    fn fan_out_caller_is_marked_heuristic() {
        // `caller` calls `t`, which fans out to two overloads (t#0, t#1). Each
        // overload's caller result is flagged heuristic; a non-overloaded callee
        // (`u`, one definition) called by the same caller is not.
        let mut g = SymbolGraph::new();
        let file = "o.ts";
        update_file(
            &mut g,
            FileSymbols {
                file: file.to_string(),
                symbols: vec![
                    func(0, "t", file),
                    func(1, "t", file),
                    func(2, "u", file),
                    func(3, "caller", file),
                ],
                imports: Vec::new(),
                reexports: Vec::new(),
                calls: vec![
                    CallSite {
                        from: caller_ref("caller"),
                        callee: CalleeRef {
                            name: "t".into(),
                            via_import: None,
                        },
                        line: 1,
                    },
                    CallSite {
                        from: caller_ref("caller"),
                        callee: CalleeRef {
                            name: "u".into(),
                            via_import: None,
                        },
                        line: 2,
                    },
                ],
                calls_partial: false,
                has_unresolved_dynamic_import: false,
                content_hash: None,
            },
        );
        // callers_of(t#0): the caller is heuristic (fan-out to t#0 + t#1).
        let t0 = SymbolIdentity {
            file: file.into(),
            kind: SymbolKind::Function,
            name: "t".into(),
            ordinal: 0,
        };
        let t_report = callers_of(&g, &t0, 1);
        assert_eq!(t_report.callers.len(), 1);
        assert!(
            t_report.callers[0].heuristic,
            "a fan-out caller must be flagged heuristic"
        );

        // callers_of(u): the same caller, but `u` has one definition → not fan-out.
        let u = SymbolIdentity {
            file: file.into(),
            kind: SymbolKind::Function,
            name: "u".into(),
            ordinal: 0,
        };
        let u_report = callers_of(&g, &u, 1);
        assert_eq!(u_report.callers.len(), 1);
        assert!(
            !u_report.callers[0].heuristic,
            "an unambiguous callee's caller is not heuristic"
        );
    }

    #[test]
    fn transitive_callers_at_depth_2() {
        let g = fixture();
        let report = callers_of(&g, &identity("a"), 2);
        let pairs: Vec<(&str, u32)> = report
            .callers
            .iter()
            .map(|c| (c.caller.name.as_str(), c.distance))
            .collect();
        // b, c direct (1); d transitive via b (2).
        assert_eq!(pairs, [("b", 1), ("c", 1), ("d", 2)]);
    }

    #[test]
    fn direct_recursion_terminates_with_target_excluded() {
        // a calls itself: the walk terminates (seen-set) and the target is
        // excluded from its own caller set — the same origin-exclusion
        // collect_dependents uses. So a purely self-recursive symbol reports no
        // callers; the point of the test is that it terminates rather than loops.
        let mut g = SymbolGraph::new();
        let file = "r.ts";
        update_file(
            &mut g,
            FileSymbols {
                file: file.to_string(),
                symbols: vec![func(0, "a", file)],
                imports: Vec::new(),
                reexports: Vec::new(),
                calls: vec![CallSite {
                    from: caller_ref("a"),
                    callee: CalleeRef {
                        name: "a".into(),
                        via_import: None,
                    },
                    line: 1,
                }],
                calls_partial: false,
                has_unresolved_dynamic_import: false,
                content_hash: None,
            },
        );
        let report = callers_of(&g, &identity_in("r.ts", "a"), 2);
        assert!(report.callers.is_empty());
        assert!(!report.truncated);
    }

    #[test]
    fn mutual_recursion_terminates() {
        // a → b → a (mutual). callers_of(a): b is a direct caller; the cycle back
        // through a is stopped by the seen-set (a is the excluded target).
        let mut g = SymbolGraph::new();
        let file = "m.ts";
        update_file(
            &mut g,
            FileSymbols {
                file: file.to_string(),
                symbols: vec![func(0, "a", file), func(1, "b", file)],
                imports: Vec::new(),
                reexports: Vec::new(),
                calls: vec![
                    CallSite {
                        from: caller_ref("b"),
                        callee: CalleeRef {
                            name: "a".into(),
                            via_import: None,
                        },
                        line: 1,
                    },
                    CallSite {
                        from: caller_ref("a"),
                        callee: CalleeRef {
                            name: "b".into(),
                            via_import: None,
                        },
                        line: 2,
                    },
                ],
                calls_partial: false,
                has_unresolved_dynamic_import: false,
                content_hash: None,
            },
        );
        let report = callers_of(&g, &identity_in("m.ts", "a"), 2);
        let names: Vec<&str> = report
            .callers
            .iter()
            .map(|c| c.caller.name.as_str())
            .collect();
        assert_eq!(names, ["b"]);
    }

    fn identity_in(file: &str, name: &str) -> SymbolIdentity {
        SymbolIdentity {
            file: file.into(),
            kind: SymbolKind::Function,
            name: name.into(),
            ordinal: 0,
        }
    }

    #[test]
    fn absent_target_yields_empty_report() {
        let g = fixture();
        let report = callers_of(&g, &identity("does_not_exist"), 2);
        assert!(report.callers.is_empty());
        assert!(!report.truncated);
    }

    #[test]
    fn deterministic_regardless_of_call() {
        let g = fixture();
        assert_eq!(
            callers_of(&g, &identity("a"), 2),
            callers_of(&g, &identity("a"), 2)
        );
    }

    /// Regression for the council ADV-2 finding: the report is nearest-first
    /// `(distance, identity)`, not identity-only. `m`'s direct caller is `z`
    /// (distance 1) and `z`'s caller is `a` (distance 2); alphabetically `a < z`,
    /// so an identity-only sort would wrongly put the distance-2 caller first.
    #[test]
    fn nearest_first_beats_alphabetical_order() {
        let mut g = SymbolGraph::new();
        let file = "n.ts";
        update_file(
            &mut g,
            FileSymbols {
                file: file.to_string(),
                symbols: vec![func(0, "m", file), func(1, "z", file), func(2, "a", file)],
                imports: Vec::new(),
                reexports: Vec::new(),
                calls: vec![
                    CallSite {
                        from: caller_ref("z"),
                        callee: CalleeRef {
                            name: "m".into(),
                            via_import: None,
                        },
                        line: 1,
                    },
                    CallSite {
                        from: caller_ref("a"),
                        callee: CalleeRef {
                            name: "z".into(),
                            via_import: None,
                        },
                        line: 2,
                    },
                ],
                calls_partial: false,
                has_unresolved_dynamic_import: false,
                content_hash: None,
            },
        );
        let report = callers_of(&g, &identity_in("n.ts", "m"), 2);
        let pairs: Vec<(&str, u32)> = report
            .callers
            .iter()
            .map(|c| (c.caller.name.as_str(), c.distance))
            .collect();
        assert_eq!(
            pairs,
            [("z", 1), ("a", 2)],
            "nearest-first: the distance-1 caller `z` precedes the distance-2 caller `a` despite a < z"
        );
        assert_eq!(
            report.callers[0].distance, 1,
            "callers[0] must be a closest caller"
        );
    }

    /// `depth == 0` is in-contract (the GV2-026 clamp floor) and yields no callers:
    /// the `1..=0` hop range is empty. Guards against an off-by-one that would
    /// either panic or walk one hop.
    #[test]
    fn depth_zero_yields_no_callers() {
        let g = fixture();
        let report = callers_of(&g, &identity("a"), 0);
        assert!(report.callers.is_empty());
        assert!(!report.truncated);
    }

    fn func_spanned(id: u64, name: &str, file: &str, start: u32, end: u32) -> SymbolNode {
        SymbolNode {
            span: Some(anvil_kernel_types::ByteRange { start, end }),
            ..func(id, name, file)
        }
    }

    /// Two adjacent, non-overlapping symbols: an offset inside one resolves to
    /// exactly that symbol, never its neighbour.
    #[test]
    fn symbol_at_offset_resolves_the_containing_symbol() {
        let mut g = SymbolGraph::new();
        let file = "a.ts";
        update_file(
            &mut g,
            FileSymbols {
                file: file.to_string(),
                symbols: vec![
                    func_spanned(0, "a", file, 0, 10),
                    func_spanned(1, "b", file, 10, 20),
                ],
                imports: Vec::new(),
                reexports: Vec::new(),
                calls: Vec::new(),
                calls_partial: false,
                has_unresolved_dynamic_import: false,
                content_hash: Some(0),
            },
        );

        assert_eq!(symbol_at_offset(&g, file, 5), Some(identity_in(file, "a")));
        assert_eq!(symbol_at_offset(&g, file, 15), Some(identity_in(file, "b")));
    }

    /// The end offset is exclusive: it belongs to whatever starts there (or
    /// nothing), never the symbol whose span it closes.
    #[test]
    fn symbol_at_offset_end_is_exclusive() {
        let mut g = SymbolGraph::new();
        let file = "a.ts";
        update_file(
            &mut g,
            FileSymbols {
                file: file.to_string(),
                symbols: vec![func_spanned(0, "a", file, 0, 10)],
                imports: Vec::new(),
                reexports: Vec::new(),
                calls: Vec::new(),
                calls_partial: false,
                has_unresolved_dynamic_import: false,
                content_hash: Some(0),
            },
        );

        assert_eq!(symbol_at_offset(&g, file, 9), Some(identity_in(file, "a")));
        assert_eq!(symbol_at_offset(&g, file, 10), None);
    }

    /// A nested symbol (e.g. a method inside a class) resolves to the
    /// **innermost** enclosing span, not the outer one — the whole point of
    /// the min-width tie-break.
    #[test]
    fn symbol_at_offset_prefers_the_innermost_span() {
        let mut g = SymbolGraph::new();
        let file = "a.ts";
        update_file(
            &mut g,
            FileSymbols {
                file: file.to_string(),
                symbols: vec![
                    func_spanned(0, "Outer", file, 0, 100),
                    func_spanned(1, "inner", file, 20, 40),
                ],
                imports: Vec::new(),
                reexports: Vec::new(),
                calls: Vec::new(),
                calls_partial: false,
                has_unresolved_dynamic_import: false,
                content_hash: Some(0),
            },
        );

        assert_eq!(
            symbol_at_offset(&g, file, 30),
            Some(identity_in(file, "inner")),
            "an offset inside both spans must resolve to the narrower (inner) one"
        );
        assert_eq!(
            symbol_at_offset(&g, file, 60),
            Some(identity_in(file, "Outer")),
            "an offset inside only the outer span still resolves to it"
        );
    }

    /// A symbol with no recorded span (`span: None`) is never a candidate,
    /// and an offset outside every span resolves to nothing.
    #[test]
    fn symbol_at_offset_skips_unspanned_symbols_and_misses_cleanly() {
        let mut g = SymbolGraph::new();
        let file = "a.ts";
        update_file(
            &mut g,
            FileSymbols {
                file: file.to_string(),
                symbols: vec![
                    func(0, "synthetic", file),
                    func_spanned(1, "a", file, 50, 60),
                ],
                imports: Vec::new(),
                reexports: Vec::new(),
                calls: Vec::new(),
                calls_partial: false,
                has_unresolved_dynamic_import: false,
                content_hash: Some(0),
            },
        );

        assert_eq!(
            symbol_at_offset(&g, file, 5),
            None,
            "no spanned symbol covers offset 5"
        );
        assert_eq!(symbol_at_offset(&g, file, 55), Some(identity_in(file, "a")));
    }

    /// An unknown file yields `None` rather than panicking.
    #[test]
    fn symbol_at_offset_on_absent_file_is_none() {
        let g = fixture();
        assert_eq!(symbol_at_offset(&g, "missing.ts", 0), None);
    }
}
