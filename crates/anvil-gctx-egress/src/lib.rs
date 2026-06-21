//! Daemon-side GCTX egress projector (ADR-084).
//!
//! The single [`GctxProjector`] choke point (CE-5) that turns the daemon's warm
//! [`SymbolGraph`] into sealed, identity-only [`anvil_gctx_types`] DTOs. It
//! depends on `anvil-graph-cache` (to read the graph) and `anvil-gctx-types`
//! (the sealed DTOs), and is **daemon-only**: the MCP consumer never links it,
//! so no graph internal can reach the wire through this path.
//!
//! # Concurrency split (ADR-084 C2)
//!
//! GCTX reads must not block save-time mutation. The projection is split so the
//! caller does the cheap match-and-collect **under** the cache lock
//! ([`GctxProjector::collect_candidates`]) and the sort/paginate/seal
//! **outside** it ([`GctxProjector::project`]). Holding the inner `Mutex` across
//! the whole projection is prohibited (ADR-031 80ms p95). `collect_candidates`
//! returns already-sealed [`SymbolSummary`] values (identity + visibility only),
//! so nothing borrowed from the graph escapes the lock.

use std::path::Path;

use anvil_gctx_types::{
    AffectedTestsReport, AffectedTestsSummary, CallerSummary, DependentSummary, EdgeSummary,
    FindCallersProjection, FindCallersQuery, FindDependentsProjection, FindDependentsQuery,
    GraphEdgesProjection, GraphEdgesQuery, GraphStatsProjection, ImpactReport, ImpactSummary,
    OpaqueCursor, RedactionSummary, SearchSymbolsProjection, SearchSymbolsQuery, SymbolSummary,
    TestEvidence,
};
use anvil_graph_cache::{DependencyGraph, SymbolGraph};
use anvil_kernel_types::{EdgeType, SymbolIdentity, SymbolKind, SymbolNode, Visibility};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

/// The single CE-5 egress choke point: builds sealed identity-only DTOs from the
/// daemon's warm [`SymbolGraph`].
pub struct GctxProjector;

impl GctxProjector {
    /// Match `query` against the warm graph and collect identity-only
    /// candidates.
    ///
    /// **Call this under the cache lock** (it borrows `graph`). It does only
    /// cheap matching plus identity cloning — no sorting, no pagination — so the
    /// lock is held for the minimum (ADR-084 C2). The returned
    /// [`SymbolSummary`] values own their data, so the caller releases the lock
    /// before calling [`GctxProjector::project`].
    ///
    /// Ordinals are computed per file in parse order via
    /// [`SymbolIdentity::for_file_symbols`], so files are visited in a
    /// deterministic (sorted) order.
    #[must_use]
    pub fn collect_candidates(
        graph: &SymbolGraph,
        query: &SearchSymbolsQuery,
    ) -> (Vec<SymbolSummary>, usize) {
        // Lower-case the filters ONCE, not per node, so the lock-held loop does
        // no avoidable *filter* allocation (ADR-084 C2). The deny-list check
        // (`is_sensitive_egress_path`) still allocates a lowercased String per
        // path segment per file under the lock — a deliberate, security-necessary
        // per-file cost (CE-3), not avoidable here without a case-folding rewrite.
        let name_lc = query.name.as_deref().map(str::to_lowercase);
        let file_lc = query.file.as_deref().map(str::to_lowercase);

        let mut out = Vec::new();
        // CIB-091a (CE-3): count files dropped by the sensitive-path deny-list so
        // the projection's `omitted_sensitive_paths` is honest.
        let mut omitted_sensitive = 0usize;
        // Iterate the file index (O(files)) rather than every node (O(symbols))
        // to rediscover the file set — the final order is imposed by `project`,
        // so file-visit order is irrelevant here.
        for file in graph.file_names() {
            // CE-5 defence in depth: the egress projector emits only
            // workspace-root-relative paths. The parser feed supplies relative
            // paths, so an absolute path should never be resident — but if one
            // is, drop it rather than leak an absolute filesystem location.
            if is_absolute_path_like(file) {
                continue;
            }
            // CIB-091a (CE-3): the substrate scans with `standard_filters(false)`,
            // so secret/dotfile paths are resident — drop them before they egress
            // as identity-only paths, and count the drop.
            if is_sensitive_egress_path(file) {
                omitted_sensitive += 1;
                continue;
            }
            if let Some(filter) = file_lc.as_deref()
                && !file.to_lowercase().contains(filter)
            {
                continue;
            }
            if let Some(lang) = query.language.as_deref()
                && !language_matches(file, lang)
            {
                continue;
            }
            let symbols = graph.symbols_in_file(file);
            let identities = SymbolIdentity::for_file_symbols(&symbols);
            for (node, identity) in symbols.iter().zip(identities) {
                if symbol_matches(node, name_lc.as_deref(), query) {
                    out.push(SymbolSummary {
                        identity,
                        visibility: node.visibility,
                    });
                }
            }
        }
        (out, omitted_sensitive)
    }

    /// Sort, paginate, and seal collected candidates into the egress projection.
    ///
    /// **Call this after releasing the cache lock** (ADR-084 C2). Ordering is a
    /// deterministic total order on [`SymbolIdentity`] (`file`, `kind`, `name`,
    /// `ordinal`).
    ///
    /// # Pagination (CE-6)
    ///
    /// Keyset (seek) pagination: when more matches remain than fit in one page,
    /// the projection carries a **server-minted opaque** `next_cursor` encoding
    /// the last returned identity plus a fingerprint of the query filters. A
    /// follow-up call echoes that cursor back in [`SearchSymbolsQuery::cursor`]
    /// and resumes strictly after that identity. Keyset (not offset) so the walk
    /// stays deterministic and robust if the graph mutates between pages, and the
    /// cursor is never a client-supplied offset.
    ///
    /// # Errors
    ///
    /// Returns the rejection reason (for an `InvalidQuery` outcome) when the
    /// supplied `cursor` is malformed or was minted for a different query — a
    /// cursor is only valid for the filter set it was issued against.
    pub fn project(
        mut candidates: Vec<SymbolSummary>,
        query: &SearchSymbolsQuery,
        omitted_sensitive: usize,
    ) -> Result<SearchSymbolsProjection, String> {
        candidates.sort_by(|a, b| a.identity.cmp(&b.identity));
        let matched = candidates.len();
        let fingerprint = query_fingerprint(query);

        // Resolve the seek start from an echoed cursor (keyset, not offset).
        let start = match &query.cursor {
            None => 0,
            Some(cursor) => {
                if cursor.as_str().len() > MAX_CURSOR_BYTES {
                    return Err("pagination cursor is too long".to_string());
                }
                let payload = decode_cursor::<CursorPayload>(cursor)
                    .ok_or_else(|| "malformed pagination cursor".to_string())?;
                if payload.fingerprint != fingerprint {
                    return Err("pagination cursor does not match this query's filters".to_string());
                }
                // First index strictly after the cursor's last identity. Robust
                // if that identity was removed between pages.
                candidates.partition_point(|s| s.identity <= payload.last)
            }
        };

        let limit = query.effective_limit();
        let mut page = if start >= candidates.len() {
            Vec::new()
        } else {
            candidates.split_off(start)
        };
        let has_more = page.len() > limit;
        page.truncate(limit);

        let next_cursor = has_more.then(|| {
            // `page` is non-empty here: it held `> limit` (`>= 1`) rows before
            // the truncate, so `last()` is always `Some`.
            let last = page.last().expect("a page with more rows is non-empty");
            encode_cursor(&CursorPayload {
                fingerprint,
                last: last.identity.clone(),
            })
        });

        let returned = page.len();
        Ok(SearchSymbolsProjection {
            // `truncated` is the authoritative "more pages follow" signal — it is
            // `false` on the final page of a multi-page walk (where `matched`
            // still exceeds this page's `returned`).
            redaction_summary: RedactionSummary {
                matched,
                returned,
                truncated: next_cursor.is_some(),
                omitted_sensitive_paths: omitted_sensitive,
            },
            symbols: page,
            next_cursor,
        })
    }

    /// Walk the reverse-impact (dependents) set of `file` and collect identity-only
    /// candidates (GCTX-011).
    ///
    /// **Call this under the cache lock** (it borrows `dep`). Breadth-first over
    /// [`DependencyGraph::dependents_of`] up to `depth` hops, recording each
    /// importer at the **first** (smallest) distance it is reached — so a file
    /// reachable by both a 1-hop and a 2-hop path is reported at distance 1. The
    /// caller MUST clamp `depth` to the GV2-026
    /// [`anvil_graph_cache::clamp_reverse_impact_depth`] lever before calling; a
    /// `debug_assert` enforces the ADR-063 ceiling so a future caller that lifts
    /// the cap on this read path trips under test rather than silently
    /// reintroducing an unbounded walk.
    ///
    /// `file` is never reported as its own dependent. On a cyclic import graph the
    /// `seen` set terminates the walk (a file is visited at most once). The
    /// returned [`DependentSummary`] values own their data, so the caller releases
    /// the lock before calling [`GctxProjector::project_dependents`].
    ///
    /// The collected set is bounded at [`MAX_DEPENDENTS_WALK`] nodes so a
    /// pathologically-imported file (a barrel imported by tens of thousands of
    /// files) cannot make this lock-held pass allocate without bound (ADR-031 hot
    /// budget). Because each `dependents_of` frontier is walked in **sorted**
    /// order, the bound truncates deterministically (the same prefix on every
    /// call), so keyset pagination over the result stays stable.
    #[must_use]
    pub fn collect_dependents(
        dep: &DependencyGraph,
        file: &str,
        depth: u32,
    ) -> (Vec<DependentSummary>, bool, usize) {
        Self::collect_dependents_with_budget(dep, file, depth, MAX_DEPENDENTS_WALK)
    }

    fn collect_dependents_with_budget(
        dep: &DependencyGraph,
        file: &str,
        depth: u32,
        max_walk: usize,
    ) -> (Vec<DependentSummary>, bool, usize) {
        debug_assert!(
            depth <= anvil_graph_cache::MAX_REVERSE_IMPACT_DEPTH,
            "dependents walk depth {depth} exceeds the ADR-063 cap {} (caller must clamp)",
            anvil_graph_cache::MAX_REVERSE_IMPACT_DEPTH,
        );
        let mut out: Vec<DependentSummary> = Vec::new();
        let mut walk_truncated = false;
        let mut omitted_sensitive = 0usize;
        let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
        // The origin is excluded from its own dependent set and seeded so a cycle
        // back through `file` does not re-add it.
        seen.insert(file.to_string());
        let mut frontier: Vec<String> = vec![file.to_string()];

        'walk: for hop in 1..=depth {
            let mut next: Vec<String> = Vec::new();
            for current in &frontier {
                // Sort each frontier so an over-budget truncation keeps a stable,
                // deterministic prefix (the dependency index is a `HashSet`, whose
                // iteration order is otherwise unspecified).
                let mut importers = dep.dependents_of(current);
                importers.sort_unstable();
                for importer in importers {
                    // CE-5 defence in depth: never emit an absolute path, even if
                    // one were resident in the dependency index.
                    if is_absolute_path_like(importer) {
                        continue;
                    }
                    // CIB-091a (CE-3): drop a sensitive importer before it egresses
                    // and count it; seed `seen` so it is not revisited.
                    if is_sensitive_egress_path(importer) {
                        if seen.insert(importer.to_string()) {
                            omitted_sensitive += 1;
                        }
                        continue;
                    }
                    // First (smallest) distance wins; a re-seen file is skipped.
                    if seen.insert(importer.to_string()) {
                        let importer = importer.to_string();
                        out.push(DependentSummary {
                            file: importer.clone(),
                            distance: hop,
                        });
                        next.push(importer);
                        if out.len() >= max_walk {
                            walk_truncated = true;
                            break 'walk;
                        }
                    }
                }
            }
            if next.is_empty() {
                break;
            }
            // Sort the next frontier so a depth-2 over-budget truncation keeps a
            // path-ordered prefix, not an arbitrary insertion-ordered one.
            next.sort_unstable();
            frontier = next;
        }
        (out, walk_truncated, omitted_sensitive)
    }

    /// Sort, paginate, and seal collected dependents into the egress projection.
    ///
    /// **Call this after releasing the cache lock** (ADR-084 C2). Ordering is a
    /// deterministic total order on the dependent `file` path (unique within the
    /// set — each importer appears once, at its minimum distance). Pagination is
    /// the same keyset (seek) scheme as [`GctxProjector::project`]: the
    /// server-minted opaque `next_cursor` encodes the last returned `file` plus a
    /// fingerprint of the query's traversal filters (`file` + `max_depth`).
    ///
    /// # Errors
    ///
    /// Returns the rejection reason when the supplied `cursor` is malformed,
    /// oversized, or was minted for a different query.
    pub fn project_dependents(
        mut candidates: Vec<DependentSummary>,
        query: &FindDependentsQuery,
        depth: u32,
        walk_truncated: bool,
        omitted_sensitive: usize,
    ) -> Result<FindDependentsProjection, String> {
        candidates.sort_by(|a, b| a.file.cmp(&b.file));
        let matched = candidates.len();
        let fingerprint = dependents_fingerprint(query, depth);

        let start = match &query.cursor {
            None => 0,
            Some(cursor) => {
                if cursor.as_str().len() > MAX_CURSOR_BYTES {
                    return Err("pagination cursor is too long".to_string());
                }
                let payload = decode_cursor::<DependentsCursorPayload>(cursor)
                    .ok_or_else(|| "malformed pagination cursor".to_string())?;
                if payload.fingerprint != fingerprint {
                    return Err("pagination cursor does not match this query's filters".to_string());
                }
                candidates.partition_point(|d| d.file <= payload.last)
            }
        };

        let limit = query.effective_limit();
        let mut page = if start >= candidates.len() {
            Vec::new()
        } else {
            candidates.split_off(start)
        };
        let has_more = page.len() > limit;
        page.truncate(limit);

        let next_cursor = has_more.then(|| {
            let last = page.last().expect("a page with more rows is non-empty");
            encode_cursor(&DependentsCursorPayload {
                fingerprint,
                last: last.file.clone(),
            })
        });

        let returned = page.len();
        Ok(FindDependentsProjection {
            redaction_summary: RedactionSummary {
                matched,
                returned,
                truncated: next_cursor.is_some(),
                omitted_sensitive_paths: omitted_sensitive,
            },
            dependents: page,
            next_cursor,
            partial: walk_truncated,
        })
    }

    /// Collect the identity-only callers of `target` from the warm symbol graph
    /// (GCTX-014), plus whether the bounded walk was budget-truncated.
    ///
    /// **Call this under the cache lock** (it borrows `graph`). It delegates to
    /// [`anvil_graph_cache::callers_of`] (the bounded reverse-`Calls` BFS) and
    /// seals each result into an identity-only [`CallerSummary`] carrying the
    /// caller identity, hop distance, and the GCALL-007 CALL-1 `heuristic`
    /// (fan-out) marker. The returned summaries own their data, so the caller
    /// releases the lock before calling [`GctxProjector::project_callers`]. The
    /// `bool` is the walk's `truncated` flag (node budget hit) — folded into the
    /// projection's `partial` marker.
    ///
    /// `depth` MUST be caller-clamped to the GV2-026 ceiling.
    #[must_use]
    pub fn collect_callers(
        graph: &SymbolGraph,
        target: &SymbolIdentity,
        depth: u32,
    ) -> (Vec<CallerSummary>, bool, usize) {
        let report = anvil_graph_cache::callers_of(graph, target, depth);
        let mut omitted_sensitive = 0usize;
        let callers = report
            .callers
            .into_iter()
            // CE-5 defence in depth: symbol identities carry workspace-relative
            // paths, so an absolute path should never be resident — but if one is,
            // drop the caller rather than egress an absolute filesystem location
            // via `CallerSummary.caller.file` (mirrors `collect_dependents`).
            .filter(|c| !is_absolute_path_like(&c.caller.file))
            // CIB-091a (CE-3): drop a sensitive-path caller before it egresses and
            // count it.
            .filter(|c| {
                if is_sensitive_egress_path(&c.caller.file) {
                    omitted_sensitive += 1;
                    false
                } else {
                    true
                }
            })
            .map(|c| CallerSummary {
                caller: c.caller,
                distance: c.distance,
                heuristic: c.heuristic,
            })
            .collect();
        (callers, report.truncated, omitted_sensitive)
    }

    /// Sort, paginate, and seal collected callers into the egress projection
    /// (GCTX-014). **Call this after releasing the cache lock** (ADR-084 C2).
    ///
    /// Ordering is a deterministic total order on the caller [`SymbolIdentity`]
    /// (each caller appears once, at its minimum distance) — a stable **page**
    /// order for keyset pagination, which is a separate concern from the
    /// nearest-first order [`collect_callers`] uses to decide which callers to
    /// *keep* under the node budget. So the page is identity-ordered, not
    /// distance-ordered; a consumer that wants direct-before-transitive reads each
    /// row's `distance` field (carried on every [`CallerSummary`]) rather than
    /// relying on page position. Pagination is the same keyset (seek) scheme as
    /// [`GctxProjector::project_dependents`]: the server-minted opaque
    /// `next_cursor` encodes the last returned identity plus a fingerprint of the
    /// query's traversal filters (`target` + `max_depth`).
    /// `walk_truncated` (the node-budget bound from [`collect_callers`]) and
    /// `callers_incomplete` (the caller set may be missing entries — a non-`Clean`
    /// graph **or** an unresolved call site naming the target in the daemon
    /// accumulator, folded by the egress) together set the projection's `partial`
    /// marker (CALL-1).
    ///
    /// # Errors
    ///
    /// Returns the rejection reason when the supplied `cursor` is malformed,
    /// oversized, or was minted for a different query.
    pub fn project_callers(
        mut candidates: Vec<CallerSummary>,
        query: &FindCallersQuery,
        depth: u32,
        walk_truncated: bool,
        callers_incomplete: bool,
        omitted_sensitive: usize,
    ) -> Result<FindCallersProjection, String> {
        candidates.sort_by(|a, b| a.caller.cmp(&b.caller));
        let matched = candidates.len();
        let fingerprint = callers_fingerprint(query, depth);

        let start = match &query.cursor {
            None => 0,
            Some(cursor) => {
                if cursor.as_str().len() > MAX_CURSOR_BYTES {
                    return Err("pagination cursor is too long".to_string());
                }
                let payload = decode_cursor::<CallersCursorPayload>(cursor)
                    .ok_or_else(|| "malformed pagination cursor".to_string())?;
                if payload.fingerprint != fingerprint {
                    return Err("pagination cursor does not match this query's filters".to_string());
                }
                candidates.partition_point(|c| c.caller <= payload.last)
            }
        };

        let limit = query.effective_limit();
        let mut page = if start >= candidates.len() {
            Vec::new()
        } else {
            candidates.split_off(start)
        };
        let has_more = page.len() > limit;
        page.truncate(limit);

        let next_cursor = has_more.then(|| {
            let last = page.last().expect("a page with more rows is non-empty");
            encode_cursor(&CallersCursorPayload {
                fingerprint,
                last: last.caller.clone(),
            })
        });

        let returned = page.len();
        Ok(FindCallersProjection {
            redaction_summary: RedactionSummary {
                matched,
                returned,
                truncated: next_cursor.is_some(),
                omitted_sensitive_paths: omitted_sensitive,
            },
            callers: page,
            next_cursor,
            partial: walk_truncated || callers_incomplete,
        })
    }

    /// Collect the raw, identity-only pieces of an impact-of-change report
    /// (GCTX-012): the symbols defined in the changed files, and the
    /// depth-bounded reverse-impact (dependent) closure of the whole change set.
    ///
    /// **Call this under the cache lock** (it borrows both graphs). The dependent
    /// closure is a **multi-source** breadth-first walk seeded with *all* changed
    /// files at once (with a single shared `seen` set and a single aggregate node
    /// budget), so a 200-file input cannot fan out to `200 ×` the per-file bound.
    /// The changed files seed `seen`, so they are excluded from their own
    /// dependent set. Both the affected-symbol set ([`MAX_AFFECTED_SYMBOLS`]) and
    /// the dependent closure ([`MAX_DEPENDENTS_WALK`]) are bounded so the
    /// lock-held pass cannot allocate without bound on a pathological graph
    /// (ADR-031). Every frontier (seeds and each `next` hop) is walked in **sorted
    /// path order**, so an over-budget truncation keeps a deterministic,
    /// path-ordered prefix.
    ///
    /// Returns the owned, already-sealed pieces, the count of **distinct,
    /// non-absolute** changed files actually seeded (what the report is computed
    /// over), and whether a budget bound the walk. The caller releases the lock
    /// before calling [`GctxProjector::project_impact`].
    ///
    /// `depth` MUST be caller-clamped to the GV2-026 ceiling (a `debug_assert`
    /// enforces it).
    #[must_use]
    pub fn collect_impact(
        sym: &SymbolGraph,
        dep: &DependencyGraph,
        changed_files: &[String],
        depth: u32,
    ) -> CollectedImpact {
        Self::collect_impact_with_budget(sym, dep, changed_files, depth, MAX_AFFECTED_SYMBOLS)
    }

    fn collect_impact_with_budget(
        sym: &SymbolGraph,
        dep: &DependencyGraph,
        changed_files: &[String],
        depth: u32,
        max_affected: usize,
    ) -> CollectedImpact {
        debug_assert!(
            depth <= anvil_graph_cache::MAX_REVERSE_IMPACT_DEPTH,
            "impact walk depth {depth} exceeds the ADR-063 cap {} (caller must clamp)",
            anvil_graph_cache::MAX_REVERSE_IMPACT_DEPTH,
        );

        // Seed every distinct non-absolute, non-sensitive changed file before
        // collecting symbols or walking dependents — the affected-symbol cap must
        // not skip seeds. CIB-091a (CE-3): a sensitive changed-file seed is dropped
        // (no affected symbols, no dependent walk) and counted.
        //
        // The non-sensitive seeds are captured directly here (in `seed_files`),
        // and the sensitive ones still fence the BFS by joining `seen`, so the
        // deny-list predicate runs exactly once per changed file — no second
        // under-lock re-filter pass over `seen` (CIB-091 LOW perf follow-up).
        let mut truncated = false;
        let mut omitted_sensitive = 0usize;
        let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
        let mut seed_files: Vec<String> = Vec::new();
        for file in changed_files {
            if is_absolute_path_like(file) {
                continue;
            }
            if !seen.insert(file.clone()) {
                // A duplicate changed-file entry: already classified.
                continue;
            }
            if is_sensitive_egress_path(file) {
                // Counted once and kept in `seen` to fence the BFS, but excluded
                // from the change surface the report is computed over.
                omitted_sensitive += 1;
                continue;
            }
            seed_files.push(file.clone());
        }
        // Distinct, non-sensitive seeds — the change surface the report covers.
        let changed_count = seed_files.len();
        seed_files.sort_unstable();

        // CIB-091c: collect affected symbols into a bounded max-heap keyed by
        // identity (O(log n) push) so the lock-held pass does work bounded by the
        // cap — no O(n) `Vec::insert` shift of up to `MAX_AFFECTED_SYMBOLS` per
        // symbol under the cache lock. The heap keeps the identity-SMALLEST
        // `max_affected` summaries (a max-heap pops the largest once over cap),
        // preserving the existing "keep the lowest-identity-ordered prefix"
        // truncation semantics. The final sort happens in `project_impact` after
        // the lock releases.
        let mut affected_heap: std::collections::BinaryHeap<HeapSummary> =
            std::collections::BinaryHeap::new();
        for file in &seed_files {
            let symbols = sym.symbols_in_file(file);
            let identities = SymbolIdentity::for_file_symbols(&symbols);
            for (node, identity) in symbols.iter().zip(identities) {
                let summary = SymbolSummary {
                    identity,
                    visibility: node.visibility,
                };

                if max_affected == 0 {
                    truncated = true;
                    continue;
                }

                if affected_heap.len() < max_affected {
                    affected_heap.push(HeapSummary(summary));
                    continue;
                }

                // At cap: keep the new summary only if it is smaller than the
                // current largest (the heap root), evicting that root.
                if affected_heap
                    .peek()
                    .is_some_and(|largest| summary.identity < largest.0.identity)
                {
                    affected_heap.pop();
                    affected_heap.push(HeapSummary(summary));
                }
                truncated = true;
            }
        }
        // Unwrap the heap into an unsorted Vec; `project_impact` sorts it.
        let affected: Vec<SymbolSummary> = affected_heap.into_iter().map(|h| h.0).collect();

        // Dependent closure: one multi-source BFS over all seeds.
        let mut dependents: Vec<DependentSummary> = Vec::new();
        let mut frontier = seed_files;
        'walk: for hop in 1..=depth {
            let mut next: Vec<String> = Vec::new();
            for current in &frontier {
                let mut importers = dep.dependents_of(current);
                importers.sort_unstable();
                for importer in importers {
                    if is_absolute_path_like(importer) {
                        continue;
                    }
                    // CIB-091a (CE-3): drop a sensitive importer before it egresses
                    // and count it; seed `seen` so it is not revisited.
                    if is_sensitive_egress_path(importer) {
                        if seen.insert(importer.to_string()) {
                            omitted_sensitive += 1;
                        }
                        continue;
                    }
                    if seen.insert(importer.to_string()) {
                        let importer = importer.to_string();
                        dependents.push(DependentSummary {
                            file: importer.clone(),
                            distance: hop,
                        });
                        next.push(importer);
                        if dependents.len() >= MAX_DEPENDENTS_WALK {
                            truncated = true;
                            break 'walk;
                        }
                    }
                }
            }
            if next.is_empty() {
                break;
            }
            // Sort the next frontier so a depth-2 over-budget truncation keeps a
            // path-ordered prefix (not an arbitrary insertion-ordered one).
            next.sort_unstable();
            frontier = next;
        }

        CollectedImpact {
            affected,
            dependents,
            changed_count,
            truncated,
            omitted_sensitive,
        }
    }

    /// Sort and seal collected impact pieces into the report (GCTX-012).
    /// **Call this after releasing the cache lock.**
    ///
    /// `affected_symbols` is ordered by [`SymbolIdentity`]; `dependent_files` by
    /// path (already distinct from the shared-`seen` walk); `known_tests` is the
    /// subset of dependent paths matching the best-effort test-file heuristic.
    /// Deterministic for an identical change set and graph state. The
    /// `changed_files` / `truncated` summary fields are taken from
    /// [`collect_impact`] so they always reflect what was actually walked.
    #[must_use]
    pub fn project_impact(collected: CollectedImpact) -> ImpactReport {
        let CollectedImpact {
            mut affected,
            mut dependents,
            changed_count,
            truncated,
            omitted_sensitive,
        } = collected;
        // Identities are already distinct (each changed file is seeded once and
        // `for_file_symbols` assigns distinct ordinals), so a sort suffices — no
        // dedup needed.
        affected.sort_by(|a, b| a.identity.cmp(&b.identity));
        dependents.sort_by(|a, b| a.file.cmp(&b.file));

        let known_tests: Vec<String> = dependents
            .iter()
            .filter(|d| is_test_file(&d.file))
            .map(|d| d.file.clone())
            .collect();

        let summary = ImpactSummary {
            changed_files: changed_count,
            affected_symbols: affected.len(),
            dependent_files: dependents.len(),
            known_tests: known_tests.len(),
            truncated,
            omitted_sensitive_paths: omitted_sensitive,
        };
        ImpactReport {
            affected_symbols: affected,
            dependent_files: dependents,
            known_tests,
            summary,
        }
    }

    /// Collect the raw, identity-only pieces of an affected-tests report
    /// (GCTX-013): the **test files** that import the change set within the depth
    /// bound — each with its evidence edges (the changed files it directly
    /// imports) and traversal distance — and the changed **non-test** files with
    /// no resident test importer (coverage gaps).
    ///
    /// **Call this under the cache lock** (it borrows the dependency graph). It
    /// needs only the [`DependencyGraph`]'s reverse edges (to find importers) and
    /// forward edges (`dependencies_of`, for the evidence link and transitive
    /// coverage), not the symbol graph.
    ///
    /// Two bounded passes run under the lock:
    /// 1. a **reverse** multi-source breadth-first walk seeded with all changed
    ///    files (shared `seen`, single aggregate node budget) discovers every
    ///    dependent within `depth`; the test files among them become the report's
    ///    `tests`, each tagged with `dependencies_of(test) ∩ changed_set` and its
    ///    hop distance;
    /// 2. a **forward** multi-source walk seeded with the discovered test files
    ///    determines which changed files a test transitively reaches within
    ///    `depth` — the *covered* set; a changed non-test file outside it is a
    ///    coverage gap.
    ///
    /// The two passes **share one aggregate** [`MAX_DEPENDENTS_WALK`] node-visit
    /// budget (the reverse pass's count carries into the forward pass), so a
    /// single call's lock-held cost matches the single-walk sibling verbs and
    /// cannot allocate without bound on a pathological graph (ADR-031); every
    /// frontier is walked in **sorted path order**, so an over-budget truncation
    /// keeps a deterministic, path-ordered prefix. The changed files seed the
    /// reverse `seen`, so a changed file is never reported as importing itself.
    ///
    /// `depth` MUST be caller-clamped to the GV2-026 ceiling (a `debug_assert`
    /// enforces it). The caller releases the lock before calling
    /// [`GctxProjector::project_affected_tests`].
    #[must_use]
    pub fn collect_affected_tests(
        dep: &DependencyGraph,
        changed_files: &[String],
        depth: u32,
    ) -> CollectedAffectedTests {
        debug_assert!(
            depth <= anvil_graph_cache::MAX_REVERSE_IMPACT_DEPTH,
            "affected-tests walk depth {depth} exceeds the ADR-063 cap {} (caller must clamp)",
            anvil_graph_cache::MAX_REVERSE_IMPACT_DEPTH,
        );

        // Distinct, non-absolute changed seeds. This set doubles as change-set
        // membership for evidence-edge and coverage tests. CIB-091a (CE-3): a
        // sensitive changed file is excluded from the change set (so it can never
        // surface as an evidence edge or coverage gap) and counted.
        let mut omitted_sensitive = 0usize;
        let mut changed_set: std::collections::HashSet<String> = std::collections::HashSet::new();
        for file in changed_files {
            if is_absolute_path_like(file) {
                continue;
            }
            if is_sensitive_egress_path(file) {
                omitted_sensitive += 1;
                continue;
            }
            changed_set.insert(file.clone());
        }
        let changed_count = changed_set.len();

        let mut truncated = false;

        // --- Pass 1: reverse walk → dependent tests within the depth bound. ---
        let mut tests: Vec<TestEvidence> = Vec::new();
        let mut seen: std::collections::HashSet<String> = changed_set.clone();
        let mut frontier: Vec<String> = {
            let mut f: Vec<String> = changed_set.iter().cloned().collect();
            f.sort_unstable();
            f
        };
        let mut walked = 0usize;
        'reverse: for hop in 1..=depth {
            let mut next: Vec<String> = Vec::new();
            for current in &frontier {
                let mut importers = dep.dependents_of(current);
                importers.sort_unstable();
                for importer in importers {
                    if is_absolute_path_like(importer) {
                        continue;
                    }
                    // CIB-091a (CE-3): drop a sensitive importer before it can
                    // egress (as a test path or evidence edge) and count it.
                    if is_sensitive_egress_path(importer) {
                        if seen.insert(importer.to_string()) {
                            omitted_sensitive += 1;
                        }
                        continue;
                    }
                    if seen.insert(importer.to_string()) {
                        walked += 1;
                        if is_test_file(importer) {
                            // Evidence edge: the changed files this test directly
                            // imports. Empty when it reaches the change only
                            // transitively (the `distance` still records the hop).
                            let mut evidence: Vec<String> = dep
                                .dependencies_of(importer)
                                .into_iter()
                                .filter(|dep_file| changed_set.contains(*dep_file))
                                .map(ToString::to_string)
                                .collect();
                            evidence.sort_unstable();
                            tests.push(TestEvidence {
                                file: importer.to_string(),
                                changed_dependencies: evidence,
                                distance: hop,
                            });
                        }
                        next.push(importer.to_string());
                        if walked >= MAX_DEPENDENTS_WALK {
                            truncated = true;
                            break 'reverse;
                        }
                    }
                }
            }
            if next.is_empty() {
                break;
            }
            next.sort_unstable();
            frontier = next;
        }

        // --- Pass 2: forward walk from the tests → covered changed files. ---
        // Seed with the discovered (external) tests AND any changed file that is
        // itself a test: a changed test importing another changed file still
        // covers it, even though it is excluded from the dependent `tests` output
        // as part of the change set.
        let mut coverage_seeds: Vec<String> = tests.iter().map(|t| t.file.clone()).collect();
        for file in &changed_set {
            if is_test_file(file) {
                coverage_seeds.push(file.clone());
            }
        }
        // Share the node budget across both passes (`walked` carries over) so a
        // single call stays bounded at one [`MAX_DEPENDENTS_WALK`] aggregate, not
        // one per pass — the lock-held cost matches the single-walk sibling verbs
        // (ADR-031).
        let covered = covered_changed_files(
            dep,
            &coverage_seeds,
            &changed_set,
            depth,
            &mut walked,
            &mut truncated,
            &mut omitted_sensitive,
        );

        // Coverage gaps: changed non-test files no test reaches within the bound.
        let coverage_gaps: Vec<String> = changed_set
            .iter()
            .filter(|file| !is_test_file(file) && !covered.contains(*file))
            .cloned()
            .collect();

        CollectedAffectedTests {
            tests,
            coverage_gaps,
            changed_count,
            truncated,
            omitted_sensitive,
        }
    }

    /// Sort and seal collected affected-tests pieces into the report (GCTX-013).
    /// **Call this after releasing the cache lock.**
    ///
    /// `tests` is ordered by path (already distinct from the shared-`seen`
    /// reverse walk); `coverage_gaps` by path. Deterministic for an identical
    /// change set and graph state. The `changed_files` / `truncated` summary
    /// fields are taken from [`collect_affected_tests`] so they always reflect
    /// what was actually walked. `heuristic` is always `true`.
    #[must_use]
    pub fn project_affected_tests(collected: CollectedAffectedTests) -> AffectedTestsReport {
        let CollectedAffectedTests {
            mut tests,
            mut coverage_gaps,
            changed_count,
            truncated,
            omitted_sensitive,
        } = collected;
        tests.sort_by(|a, b| a.file.cmp(&b.file));
        coverage_gaps.sort_unstable();

        let evidence_edges = tests.iter().map(|t| t.changed_dependencies.len()).sum();
        let summary = AffectedTestsSummary {
            changed_files: changed_count,
            tests: tests.len(),
            evidence_edges,
            coverage_gaps: coverage_gaps.len(),
            truncated,
            omitted_sensitive_paths: omitted_sensitive,
        };
        AffectedTestsReport {
            tests,
            coverage_gaps,
            heuristic: true,
            summary,
        }
    }

    /// Build the counts-only `graph://stats` projection (GCTX-030). Pure
    /// construction from aggregate counts the caller reads under the lock — there
    /// is nothing to seal (no names, paths, or content), so this is trivially
    /// CE-5-safe and needs no lock itself.
    #[must_use]
    pub fn project_stats(
        symbol_count: usize,
        symbol_edge_count: usize,
        file_count: usize,
        dependency_edge_count: usize,
    ) -> GraphStatsProjection {
        GraphStatsProjection {
            symbol_count,
            symbol_edge_count,
            file_count,
            dependency_edge_count,
        }
    }

    /// Collect identity-only `(from, to, edge_type)` edge summaries from the
    /// resident symbol graph (GCTX-030 `graph://edges`). Returns the summaries and
    /// a `bounded` flag (`true` when the [`MAX_EDGES_WALK`] enumeration bound was
    /// hit, so some edges were not collected and the matched count is a lower
    /// bound).
    ///
    /// **Call this under the cache lock** (it borrows `graph`). When
    /// `file_filter` is `Some`, only edges whose **source** symbol is in that
    /// workspace-root-relative file are collected; the target may live in any
    /// file. Both endpoints are resolved to stable [`SymbolIdentity`] via a
    /// `node id → identity` map built once over **all** files (so a cross-file
    /// `to` endpoint always resolves), so an edge is emitted only when **both**
    /// endpoints resolve to a non-absolute-path resident symbol (CE-5 defence in
    /// depth — a synthetic node with an absolute file is dropped). An external
    /// module node (file == a bare import specifier like `node:fs` or a scoped
    /// package) is *not* absolute-path-like, so an `imports`/`reexports` edge to it
    /// IS surfaced — the same identity surface GCTX-010 `search_symbols` already
    /// egresses (PV-9 identity-only); it exposes the dependency relationship, not
    /// source content.
    ///
    /// **Determinism (council ADV-1/ADV-2/KM-1):** `file_names()` is a `HashMap`
    /// iterator (unordered) and `outgoing_edges()` is petgraph insertion order, so
    /// this **sorts** the file list and each node's resolved outgoing edges before
    /// applying the bound. The collected prefix under [`MAX_EDGES_WALK`] is
    /// therefore the same set on every call, so keyset pagination over it stays
    /// stable. The returned summaries own their data, so the caller releases the
    /// lock before calling [`GctxProjector::project_edges`].
    #[must_use]
    pub fn collect_all_edges(
        graph: &SymbolGraph,
        file_filter: Option<&str>,
    ) -> (Vec<EdgeSummary>, bool, usize) {
        // Pass 1: one `symbols_in_file` per file (cached for pass 2), building the
        // global node id → identity map. Files held in a Vec so pass 2 can visit
        // them in a deterministic sorted order. CIB-091a (CE-3): a sensitive file
        // is excluded from the identity map, so neither its symbols (as `from`)
        // nor any edge targeting them (as `to`) can resolve — the edge is dropped.
        let mut by_file: Vec<(&str, Vec<&SymbolNode>)> = Vec::new();
        let mut identities: std::collections::HashMap<u64, SymbolIdentity> =
            std::collections::HashMap::new();
        let mut omitted_sensitive = 0usize;
        for file in graph.file_names() {
            if is_absolute_path_like(file) {
                continue;
            }
            if is_sensitive_egress_path(file) {
                omitted_sensitive += 1;
                continue;
            }
            let symbols = graph.symbols_in_file(file);
            let resolved = SymbolIdentity::for_file_symbols(&symbols);
            for (node, identity) in symbols.iter().zip(resolved) {
                identities.insert(node.id, identity);
            }
            by_file.push((file, symbols));
        }
        by_file.sort_by(|a, b| a.0.cmp(b.0));

        // Pass 2: walk sorted files, and within each node its outgoing edges in
        // sorted `(to, edge_type)` order, so the bounded prefix is deterministic.
        let mut out = Vec::new();
        let mut bounded = false;
        'walk: for (file, symbols) in &by_file {
            if let Some(filter) = file_filter
                && *file != filter
            {
                continue;
            }
            for node in symbols {
                let Some(from) = identities.get(&node.id) else {
                    continue;
                };
                let mut resolved_edges: Vec<(SymbolIdentity, EdgeType)> = graph
                    .outgoing_edges(node.id)
                    .into_iter()
                    .filter_map(|edge| {
                        identities
                            .get(&edge.to)
                            .map(|to| (to.clone(), edge.edge_type))
                    })
                    .collect();
                resolved_edges.sort();
                for (to, edge_type) in resolved_edges {
                    out.push(EdgeSummary {
                        from: from.clone(),
                        to,
                        edge_type,
                    });
                    if out.len() >= MAX_EDGES_WALK {
                        bounded = true;
                        break 'walk;
                    }
                }
            }
        }
        (out, bounded, omitted_sensitive)
    }

    /// Sort, paginate, and seal collected edges into the `graph://edges`
    /// projection (GCTX-030). **Call this after releasing the cache lock.**
    ///
    /// Ordering is the deterministic total order on [`EdgeSummary`]
    /// (`from`, `to`, `edge_type`). Pagination is the same CE-6 keyset scheme as
    /// the other GCTX surfaces: the server-minted opaque `next_cursor` encodes the
    /// last returned edge plus a fingerprint of the query's `file` filter.
    ///
    /// # Errors
    ///
    /// Returns the rejection reason when the supplied `cursor` is malformed,
    /// oversized, or was minted for a different `file` filter.
    pub fn project_edges(
        mut candidates: Vec<EdgeSummary>,
        query: &GraphEdgesQuery,
        bounded: bool,
        omitted_sensitive: usize,
    ) -> Result<GraphEdgesProjection, String> {
        candidates.sort();
        candidates.dedup();
        let matched = candidates.len();
        let fingerprint = edges_fingerprint(query);

        let start = match &query.cursor {
            None => 0,
            Some(cursor) => {
                if cursor.as_str().len() > MAX_CURSOR_BYTES {
                    return Err("pagination cursor is too long".to_string());
                }
                let payload = decode_cursor::<EdgesCursorPayload>(cursor)
                    .ok_or_else(|| "malformed pagination cursor".to_string())?;
                if payload.fingerprint != fingerprint {
                    return Err("pagination cursor does not match this query's filters".to_string());
                }
                candidates.partition_point(|e| *e <= payload.last)
            }
        };

        let limit = query.effective_limit();
        let mut page = if start >= candidates.len() {
            Vec::new()
        } else {
            candidates.split_off(start)
        };
        let has_more = page.len() > limit;
        page.truncate(limit);

        let next_cursor = has_more.then(|| {
            let last = page.last().expect("a page with more rows is non-empty");
            encode_cursor(&EdgesCursorPayload {
                fingerprint,
                last: last.clone(),
            })
        });

        let returned = page.len();
        Ok(GraphEdgesProjection {
            redaction_summary: RedactionSummary {
                matched,
                returned,
                truncated: next_cursor.is_some(),
                omitted_sensitive_paths: omitted_sensitive,
            },
            edges: page,
            next_cursor,
            bounded,
        })
    }
}

/// The owned, identity-only pieces [`GctxProjector::collect_affected_tests`]
/// gathers under the cache lock, handed to
/// [`GctxProjector::project_affected_tests`] after the lock releases (ADR-084
/// C2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CollectedAffectedTests {
    /// The test files attributed to the change set, with evidence edges +
    /// distance (unsorted; sealed on `project_affected_tests`).
    pub tests: Vec<TestEvidence>,
    /// Changed non-test files with no resident test importer within the bound.
    pub coverage_gaps: Vec<String>,
    /// Distinct, non-absolute changed files actually seeded (the count the report
    /// is computed over).
    pub changed_count: usize,
    /// Whether the shared [`MAX_DEPENDENTS_WALK`] budget bound either walk.
    pub truncated: bool,
    /// CIB-091a (CE-3): identity-only paths dropped by the sensitive-path egress
    /// deny-list across the reverse/forward walks and the change set.
    pub omitted_sensitive: usize,
}

/// Forward multi-source breadth-first walk from `test_files`, returning the
/// changed files a test transitively imports within `depth` hops (the *covered*
/// set for the GCTX-013 coverage-gap check). `walked` is the **shared, aggregate**
/// node-visit counter carried over from the reverse pass, so the whole call stays
/// bounded at one [`MAX_DEPENDENTS_WALK`] (not one budget per pass) — an
/// over-budget walk sets `truncated` (coverage may then be under-reported, i.e. a
/// real gap conservatively shown). Absolute-path dependency nodes are dropped
/// (CE-5 defence in depth; they can never be in the change set, so they only burn
/// budget) before they consume the budget. Each frontier is walked in sorted path
/// order for a deterministic prefix.
fn covered_changed_files(
    dep: &DependencyGraph,
    test_files: &[String],
    changed_set: &std::collections::HashSet<String>,
    depth: u32,
    walked: &mut usize,
    truncated: &mut bool,
    omitted_sensitive: &mut usize,
) -> std::collections::HashSet<String> {
    let mut covered: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut frontier: Vec<String> = test_files.to_vec();
    frontier.sort_unstable();
    frontier.dedup();
    for test in &frontier {
        seen.insert(test.clone());
    }
    // The reverse pass may already have spent the whole budget.
    if *walked >= MAX_DEPENDENTS_WALK {
        *truncated = true;
        return covered;
    }
    'forward: for _hop in 1..=depth {
        let mut next: Vec<String> = Vec::new();
        for current in &frontier {
            let mut deps = dep.dependencies_of(current);
            deps.sort_unstable();
            for dep_file in deps {
                // CE-5 defence in depth: an absolute dependency node can never be
                // in `changed_set`, so it contributes no coverage — drop it rather
                // than let it burn budget and bloat the lock-held frontier.
                if is_absolute_path_like(dep_file) {
                    continue;
                }
                // CIB-091a (CE-3): a sensitive dependency node is likewise never
                // in the (sensitive-free) `changed_set`; drop it before it burns
                // budget and count it.
                if is_sensitive_egress_path(dep_file) {
                    if seen.insert(dep_file.to_string()) {
                        *omitted_sensitive += 1;
                    }
                    continue;
                }
                // Mark coverage regardless of whether `dep_file` is re-expanded —
                // a changed file reached by two tests is covered all the same.
                if changed_set.contains(dep_file) {
                    covered.insert(dep_file.to_string());
                }
                if seen.insert(dep_file.to_string()) {
                    *walked += 1;
                    next.push(dep_file.to_string());
                    if *walked >= MAX_DEPENDENTS_WALK {
                        *truncated = true;
                        break 'forward;
                    }
                }
            }
        }
        if next.is_empty() {
            break;
        }
        next.sort_unstable();
        frontier = next;
    }
    covered
}

/// A [`SymbolSummary`] wrapper ordered by [`SymbolIdentity`] only, so a
/// [`std::collections::BinaryHeap`] can keep the identity-smallest `max_affected`
/// summaries with O(log n) pushes under the cache lock (CIB-091c) — replacing the
/// O(n) sorted `Vec::insert` that shifted up to [`MAX_AFFECTED_SYMBOLS`] per
/// symbol while the lock was held. A `BinaryHeap` is a max-heap, so the root is
/// the LARGEST identity; over cap we evict it, retaining the smallest prefix.
struct HeapSummary(SymbolSummary);

impl PartialEq for HeapSummary {
    fn eq(&self, other: &Self) -> bool {
        self.0.identity == other.0.identity
    }
}
impl Eq for HeapSummary {}
impl PartialOrd for HeapSummary {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for HeapSummary {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.identity.cmp(&other.0.identity)
    }
}

/// The owned, identity-only pieces [`GctxProjector::collect_impact`] gathers
/// under the cache lock, handed to [`GctxProjector::project_impact`] after the
/// lock releases (ADR-084 C2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CollectedImpact {
    /// Sealed identity summaries of the symbols defined in the changed files.
    pub affected: Vec<SymbolSummary>,
    /// The reverse-impact (dependent) closure, file-keyed with hop distance.
    pub dependents: Vec<DependentSummary>,
    /// Distinct, non-absolute changed files actually seeded (the count the report
    /// is computed over).
    pub changed_count: usize,
    /// Whether a budget ([`MAX_AFFECTED_SYMBOLS`] or [`MAX_DEPENDENTS_WALK`])
    /// bound the walk.
    pub truncated: bool,
    /// CIB-091a (CE-3): identity-only paths dropped by the sensitive-path egress
    /// deny-list (sensitive changed-file seeds + sensitive importers).
    pub omitted_sensitive: usize,
}

/// Best-effort, heuristic test-file recognition over a workspace-relative path
/// (GCTX-012). Deliberately conservative and convention-based — it never claims
/// authoritative coverage (GCTX-013 owns the evidence-edge treatment). Matches
/// the common TS/JS/Rust conventions: a `.test.` / `.spec.` infix, a `_test`/
/// `_spec` stem suffix, or a `tests` / `__tests__` path component.
fn is_test_file(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    if lower.contains(".test.") || lower.contains(".spec.") {
        return true;
    }
    // Path-component check: `tests/…`, `…/tests/…`, `__tests__/…`.
    if lower
        .split(['/', '\\'])
        .any(|seg| seg == "tests" || seg == "__tests__")
    {
        return true;
    }
    // Stem suffix `_test` / `_spec` (e.g. `foo_test.rs`).
    if let Some(stem) = Path::new(&lower).file_stem().and_then(|s| s.to_str())
        && (stem.ends_with("_test") || stem.ends_with("_spec"))
    {
        return true;
    }
    false
}

/// Cap on an echoed cursor's encoded length.
///
/// The cursor is **separately bounded** from the CE-6 "≤ 512 bytes/param" cap on
/// user-typed *filter* params (name/file/language): it is not user query text but
/// a server-minted opaque token the client echoes back, and it must hold a
/// hex-encoded [`SymbolIdentity`] whose `file` path can approach `PATH_MAX`
/// (~4 KiB) — hex doubling that already exceeds 512 bytes for a legitimate deep
/// path. 16 KiB comfortably covers the largest legitimate payload — the
/// GCTX-030 edges cursor encodes **two** `SymbolIdentity` values (`from` + `to`),
/// so two `PATH_MAX` paths hex-doubled can approach 16 KiB on pathologically deep
/// monorepo paths (council ADV-4); the single-identity cursors stay far under it.
/// Still bounds hex-decode work on a hostile oversized token (the IPC frame cap
/// is the outer limit). A real cursor is a few hundred bytes.
const MAX_CURSOR_BYTES: usize = 16 * 1024;

/// Hard cap on the number of dependents [`GctxProjector::collect_dependents`]
/// materialises in a single lock-held pass. Two depth hops over a barrel file can
/// reach an arbitrarily large importer set; this bounds the lock-held allocation
/// (ADR-031) well above any honest page (`MAX_PAGE_LIMIT` is 200) while capping
/// the pathological case. Truncation is deterministic (sorted-frontier walk), so
/// keyset pagination stays stable across the bound.
const MAX_DEPENDENTS_WALK: usize = 10_000;

/// Hard cap on the affected-symbol set [`GctxProjector::collect_impact`]
/// materialises under the lock. A 200-file change set where each file defines
/// thousands of symbols would otherwise allocate millions of summaries inside the
/// cache `Mutex` (ADR-031). The cap is well above any honest change set; hitting
/// it sets the report's `truncated` flag.
const MAX_AFFECTED_SYMBOLS: usize = 20_000;

/// The decoded contents of an [`OpaqueCursor`]: the keyset seek position plus a
/// fingerprint binding it to the query filters it was minted for.
#[derive(Serialize, Deserialize)]
struct CursorPayload {
    /// Fingerprint of the query *filters* (not the page size) — see
    /// [`query_fingerprint`].
    #[serde(rename = "q")]
    fingerprint: u64,
    /// The last [`SymbolIdentity`] returned on the previous page; the next page
    /// resumes strictly after it.
    #[serde(rename = "k")]
    last: SymbolIdentity,
}

/// Encode a keyset cursor payload (the search [`CursorPayload`] or the
/// dependents [`DependentsCursorPayload`]) as a hex-wrapped opaque token. Generic
/// so both GCTX surfaces mint cursors through the same JSON-then-hex path.
fn encode_cursor<T: Serialize>(payload: &T) -> OpaqueCursor {
    let bytes = serde_json::to_vec(payload).expect("cursor payload serialises");
    OpaqueCursor::new(hex::encode(bytes))
}

/// Decode an opaque cursor back into its payload, or `None` if the token is not
/// valid hex / does not deserialise into `T` (a malformed or wrong-surface
/// cursor).
fn decode_cursor<T: DeserializeOwned>(cursor: &OpaqueCursor) -> Option<T> {
    let bytes = hex::decode(cursor.as_str()).ok()?;
    serde_json::from_slice(&bytes).ok()
}

/// A deterministic fingerprint of the query's **filter** fields (name, kind,
/// file, language, visibility) — deliberately **not** the page size or cursor,
/// so changing `limit` mid-walk is allowed but changing a filter invalidates a
/// cursor. Uses FNV-1a over the canonical serialised filters rather than
/// `std::hash` (whose default hasher is randomly seeded and not reproducible —
/// privacy verdict PV-2).
fn query_fingerprint(query: &SearchSymbolsQuery) -> u64 {
    // Normalise to the same case-insensitive semantics the match uses, so a
    // cursor stays valid when only the *case* of a filter changes between pages.
    // `None` serialises as `null` (not skipped) — intentional: this is the
    // fingerprint's own internal encoding, never the query's wire format.
    #[derive(Serialize)]
    struct Filters {
        name: Option<String>,
        kind: Option<SymbolKind>,
        file: Option<String>,
        language: Option<String>,
        visibility: Option<Visibility>,
    }
    let filters = Filters {
        name: query.name.as_deref().map(str::to_lowercase),
        kind: query.kind,
        file: query.file.as_deref().map(str::to_lowercase),
        language: query.language.as_deref().map(str::to_ascii_lowercase),
        visibility: query.visibility,
    };
    let bytes = serde_json::to_vec(&filters).expect("query filters serialise");
    fnv1a(&bytes)
}

/// The decoded contents of a dependents [`OpaqueCursor`]: the keyset seek
/// position (last returned file path) plus a fingerprint binding it to the
/// traversal filters it was minted for.
#[derive(Serialize, Deserialize)]
struct DependentsCursorPayload {
    /// Fingerprint of the traversal filters (`file` + `max_depth`) — see
    /// [`dependents_fingerprint`].
    #[serde(rename = "q")]
    fingerprint: u64,
    /// The last dependent `file` returned on the previous page; the next page
    /// resumes strictly after it.
    #[serde(rename = "k")]
    last: String,
}

/// A deterministic fingerprint of a dependents query's traversal filters: the
/// **resolved** depth (after the daemon clamps it) and the **exact** queried
/// `file`. Changing `limit` mid-walk is allowed (not part of the fingerprint);
/// changing the target file or the resolved depth invalidates a cursor.
///
/// Unlike [`query_fingerprint`], the `file` is **not** case-normalised: a
/// dependents walk does an *exact* case-sensitive lookup into the dependency
/// graph (`DependencyGraph::dependents_of`, whose keys are stored as-is), not a
/// case-insensitive substring filter. Lower-casing here would let a cursor minted
/// for `src/a.ts` match a `SRC/A.TS` query that resolves to a different (likely
/// empty) result set, producing pagination overlap/gap. FNV-1a over the canonical
/// serialised filters (a reproducible, non-randomly-seeded hash — PV-2).
fn dependents_fingerprint(query: &FindDependentsQuery, depth: u32) -> u64 {
    #[derive(Serialize)]
    struct Filters<'a> {
        // A constant surface tag domain-separates this fingerprint from the
        // search surface (and any future GCTX traversal cursor), so a cursor
        // minted for one surface can never fingerprint-match another even if the
        // non-cryptographic FNV hash collided on the rest of the payload.
        surface: &'static str,
        file: Option<&'a str>,
        depth: u32,
    }
    let filters = Filters {
        surface: "find_dependents",
        file: query.file.as_deref(),
        depth,
    };
    let bytes = serde_json::to_vec(&filters).expect("dependents filters serialise");
    fnv1a(&bytes)
}

/// The decoded contents of a callers [`OpaqueCursor`]: the keyset seek position
/// (last returned caller identity) plus a fingerprint binding it to the traversal
/// filters it was minted for.
#[derive(Serialize, Deserialize)]
struct CallersCursorPayload {
    /// Fingerprint of the traversal filters (`target` + `max_depth`).
    #[serde(rename = "q")]
    fingerprint: u64,
    /// The last caller identity returned on the previous page; the next page
    /// resumes strictly after it.
    #[serde(rename = "k")]
    last: SymbolIdentity,
}

/// A deterministic fingerprint of a callers query's traversal filters: the
/// **resolved** depth (after the daemon clamps it) and the **exact** target
/// [`SymbolIdentity`]. Changing `limit` mid-walk is allowed (not part of the
/// fingerprint); changing the target or the resolved depth invalidates a cursor.
/// A constant surface tag domain-separates it from the other GCTX cursors.
fn callers_fingerprint(query: &FindCallersQuery, depth: u32) -> u64 {
    #[derive(Serialize)]
    struct Filters<'a> {
        surface: &'static str,
        target: Option<&'a SymbolIdentity>,
        depth: u32,
    }
    let filters = Filters {
        surface: "find_callers",
        target: query.target.as_ref(),
        depth,
    };
    let bytes = serde_json::to_vec(&filters).expect("callers filters serialise");
    fnv1a(&bytes)
}

/// Hard cap on the edges [`GctxProjector::collect_all_edges`] materialises in one
/// lock-held pass (GCTX-030). A dense monorepo graph has O(edges) edges; this
/// bounds the lock-held allocation well above any honest page
/// (`MAX_PAGE_LIMIT` = 200) while capping the pathological case. The bound is
/// applied over the sorted-file / outgoing-edge walk, so truncation is
/// deterministic and keyset pagination stays stable across it.
const MAX_EDGES_WALK: usize = 50_000;

/// The decoded contents of an edges [`OpaqueCursor`]: the keyset seek position
/// (last returned edge) plus a fingerprint binding it to the query's `file`
/// filter.
#[derive(Serialize, Deserialize)]
struct EdgesCursorPayload {
    /// Fingerprint of the `file` filter — see [`edges_fingerprint`].
    #[serde(rename = "q")]
    fingerprint: u64,
    /// The last [`EdgeSummary`] returned on the previous page; the next page
    /// resumes strictly after it in `(from, to, edge_type)` order.
    #[serde(rename = "k")]
    last: EdgeSummary,
}

/// A deterministic fingerprint of a `graph://edges` query's `file` filter
/// (domain-separated by a surface tag so it can never match another GCTX
/// cursor). The `file` is **not** case-normalised — `collect_all_edges` does an
/// exact case-sensitive path match, mirroring [`dependents_fingerprint`]. Page
/// size is excluded so `limit` may change mid-walk. FNV-1a over the canonical
/// serialised filter (reproducible, non-randomly-seeded — PV-2).
fn edges_fingerprint(query: &GraphEdgesQuery) -> u64 {
    #[derive(Serialize)]
    struct Filters<'a> {
        surface: &'static str,
        file: Option<&'a str>,
    }
    let filters = Filters {
        surface: "graph_edges",
        file: query.file.as_deref(),
    };
    let bytes = serde_json::to_vec(&filters).expect("edges filters serialise");
    fnv1a(&bytes)
}

/// FNV-1a 64-bit over `bytes`. Reproducible across restarts (unlike the randomly
/// seeded `std` hasher) so a minted cursor stays valid — PV-2.
fn fnv1a(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for &byte in bytes {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

/// `name_lc` is the already-lower-cased name filter (lowered once by the
/// caller, not per node).
fn symbol_matches(node: &SymbolNode, name_lc: Option<&str>, query: &SearchSymbolsQuery) -> bool {
    if let Some(name) = name_lc
        && !node.name.to_lowercase().contains(name)
    {
        return false;
    }
    if let Some(kind) = query.kind
        && node.kind != kind
    {
        return false;
    }
    if let Some(vis) = query.visibility
        && node.visibility != vis
    {
        return false;
    }
    true
}

fn language_matches(file: &str, lang: &str) -> bool {
    language_of(file).is_some_and(|known| known.eq_ignore_ascii_case(lang))
}

fn is_absolute_path_like(file: &str) -> bool {
    if Path::new(file).is_absolute() {
        return true;
    }

    let bytes = file.as_bytes();
    if bytes.first().is_some_and(|b| *b == b'/' || *b == b'\\') {
        return true;
    }

    bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':'
}

/// CE-3 sensitive-path egress deny-list (CIB-091a).
///
/// The graph substrate scans with `standard_filters(false)`, so secret and
/// dotfile paths (`.env`, private keys, `.git/`, `secrets/`, …) are graph-
/// resident and would otherwise leak as identity-only paths in every projection
/// (`graph://symbols` file fields, `graph://edges` endpoints, `find_dependents`,
/// `find_callers`, `impact_of_change`, `affected_tests`). This predicate returns
/// `true` for any path the projector must drop **before** sealing the DTO.
///
/// A path is sensitive when **any** segment (split on both `/` and `\`) matches
/// the deny-list:
/// - a segment exactly (case-insensitively) one of `.git`, `secrets`, `.aws`,
///   `.ssh`, `.gnupg` (the common secret-bearing directories);
/// - a basename starting with `.env` (covers `.env`, `.env.production`,
///   `.env.local`);
/// - a basename starting with one of the SSH private-key conventions
///   `id_rsa`, `id_dsa`, `id_ecdsa`, `id_ed25519` (covers the key material
///   and its `.pub` half — both stay private);
/// - a basename whose extension is `pem`, `key`, `p12`, `pfx`, or `p8`.
///
/// Matching is case-insensitive on the segment. The extension check looks at the
/// final `.`-delimited component, so `keys.ts` (extension `ts`) is **not**
/// matched while `private.key` (extension `key`) is.
fn is_sensitive_egress_path(file: &str) -> bool {
    for segment in file.split(['/', '\\']) {
        if segment.is_empty() {
            continue;
        }
        let lower = segment.to_ascii_lowercase();

        // Exact secret-directory segments.
        if matches!(
            lower.as_str(),
            ".git" | "secrets" | ".aws" | ".ssh" | ".gnupg"
        ) {
            return true;
        }

        // Basename-prefix matches (segments are path components, so every
        // segment is a candidate basename; the deny-list is checked per segment
        // rather than only on the final one so a `secrets/.env.production`-style
        // path matches on the directory segment too).
        if lower.starts_with(".env")
            || lower.starts_with("id_rsa")
            || lower.starts_with("id_dsa")
            || lower.starts_with("id_ecdsa")
            || lower.starts_with("id_ed25519")
        {
            return true;
        }

        // Extension matches: the final `.`-delimited component of the segment.
        if let Some(ext) = Path::new(&lower).extension().and_then(|e| e.to_str())
            && matches!(ext, "pem" | "key" | "p12" | "pfx" | "p8")
        {
            return true;
        }
    }
    false
}

/// Map a file extension to a coarse language token. `None` for unknown
/// extensions (which then never match a `language` filter).
fn language_of(file: &str) -> Option<&'static str> {
    let ext = Path::new(file).extension()?.to_str()?;
    Some(match ext.to_ascii_lowercase().as_str() {
        "ts" | "tsx" | "mts" | "cts" => "typescript",
        "js" | "jsx" | "mjs" | "cjs" => "javascript",
        "rs" => "rust",
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use anvil_kernel_types::{SymbolKind, TrustLevel, Visibility};

    fn node(id: u64, name: &str, file: &str, kind: SymbolKind, vis: Visibility) -> SymbolNode {
        SymbolNode {
            id,
            kind,
            name: name.into(),
            visibility: vis,
            file: file.into(),
            trust_level: TrustLevel::Unknown,
        }
    }

    fn graph_of(nodes: Vec<SymbolNode>) -> SymbolGraph {
        let mut g = SymbolGraph::new();
        for n in nodes {
            g.add_symbol(n).unwrap();
        }
        g
    }

    fn run(graph: &SymbolGraph, query: &SearchSymbolsQuery) -> SearchSymbolsProjection {
        // Mirror the daemon call sequence: collect under (a notional) lock, then
        // project after release.
        let (candidates, omitted) = GctxProjector::collect_candidates(graph, query);
        GctxProjector::project(candidates, query, omitted).expect("valid query")
    }

    #[test]
    fn returns_all_symbols_with_empty_query() {
        let g = graph_of(vec![
            node(
                1,
                "alpha",
                "src/a.ts",
                SymbolKind::Function,
                Visibility::Public,
            ),
            node(
                2,
                "beta",
                "src/b.ts",
                SymbolKind::Class,
                Visibility::Internal,
            ),
        ]);
        let p = run(&g, &SearchSymbolsQuery::default());
        assert_eq!(p.symbols.len(), 2);
        assert_eq!(p.redaction_summary.matched, 2);
        assert!(!p.redaction_summary.truncated);
    }

    #[test]
    fn ordering_is_deterministic_by_identity() {
        // Insertion order deliberately not sorted; output must be.
        let g = graph_of(vec![
            node(
                3,
                "zeta",
                "src/z.ts",
                SymbolKind::Function,
                Visibility::Public,
            ),
            node(
                1,
                "alpha",
                "src/a.ts",
                SymbolKind::Function,
                Visibility::Public,
            ),
            node(
                2,
                "mid",
                "src/m.ts",
                SymbolKind::Function,
                Visibility::Public,
            ),
        ]);
        let p = run(&g, &SearchSymbolsQuery::default());
        let files: Vec<&str> = p.symbols.iter().map(|s| s.identity.file.as_str()).collect();
        assert_eq!(files, ["src/a.ts", "src/m.ts", "src/z.ts"]);
    }

    #[test]
    fn name_filter_is_case_insensitive_substring() {
        let g = graph_of(vec![
            node(
                1,
                "handleRequest",
                "src/a.ts",
                SymbolKind::Function,
                Visibility::Public,
            ),
            node(
                2,
                "render",
                "src/b.ts",
                SymbolKind::Function,
                Visibility::Public,
            ),
        ]);
        let p = run(
            &g,
            &SearchSymbolsQuery {
                name: Some("REQUEST".into()),
                ..Default::default()
            },
        );
        assert_eq!(p.symbols.len(), 1);
        assert_eq!(p.symbols[0].identity.name, "handleRequest");
    }

    #[test]
    fn kind_and_visibility_filters_are_exact() {
        let g = graph_of(vec![
            node(1, "A", "src/a.ts", SymbolKind::Class, Visibility::Public),
            node(2, "B", "src/a.ts", SymbolKind::Function, Visibility::Public),
            node(3, "C", "src/a.ts", SymbolKind::Class, Visibility::Internal),
        ]);
        let p = run(
            &g,
            &SearchSymbolsQuery {
                kind: Some(SymbolKind::Class),
                visibility: Some(Visibility::Public),
                ..Default::default()
            },
        );
        assert_eq!(p.symbols.len(), 1);
        assert_eq!(p.symbols[0].identity.name, "A");
    }

    #[test]
    fn file_and_language_filters() {
        let g = graph_of(vec![
            node(1, "A", "src/a.ts", SymbolKind::Function, Visibility::Public),
            node(2, "B", "src/b.rs", SymbolKind::Function, Visibility::Public),
            node(3, "C", "lib/c.ts", SymbolKind::Function, Visibility::Public),
        ]);
        let by_file = run(
            &g,
            &SearchSymbolsQuery {
                file: Some("src/".into()),
                ..Default::default()
            },
        );
        assert_eq!(by_file.symbols.len(), 2);

        let by_lang = run(
            &g,
            &SearchSymbolsQuery {
                language: Some("rust".into()),
                ..Default::default()
            },
        );
        assert_eq!(by_lang.symbols.len(), 1);
        assert_eq!(by_lang.symbols[0].identity.file, "src/b.rs");
    }

    #[test]
    fn overloads_get_distinct_ordinals() {
        let g = graph_of(vec![
            node(
                1,
                "foo",
                "src/a.ts",
                SymbolKind::Function,
                Visibility::Public,
            ),
            node(
                2,
                "foo",
                "src/a.ts",
                SymbolKind::Function,
                Visibility::Public,
            ),
        ]);
        let p = run(&g, &SearchSymbolsQuery::default());
        assert_eq!(p.symbols.len(), 2);
        assert_eq!(p.symbols[0].identity.ordinal, 0);
        assert_eq!(p.symbols[1].identity.ordinal, 1);
    }

    #[test]
    fn limit_truncates_and_records_redaction() {
        let g = graph_of(vec![
            node(1, "a", "src/a.ts", SymbolKind::Function, Visibility::Public),
            node(2, "b", "src/b.ts", SymbolKind::Function, Visibility::Public),
            node(3, "c", "src/c.ts", SymbolKind::Function, Visibility::Public),
        ]);
        let p = run(
            &g,
            &SearchSymbolsQuery {
                limit: Some(2),
                ..Default::default()
            },
        );
        assert_eq!(p.symbols.len(), 2);
        assert_eq!(p.redaction_summary.matched, 3);
        assert_eq!(p.redaction_summary.returned, 2);
        assert!(p.redaction_summary.truncated);
        // Truncation keeps the deterministic head of the order.
        assert_eq!(p.symbols[0].identity.file, "src/a.ts");
        assert_eq!(p.symbols[1].identity.file, "src/b.ts");
    }

    #[test]
    fn empty_graph_yields_empty_projection() {
        let g = SymbolGraph::new();
        let p = run(&g, &SearchSymbolsQuery::default());
        assert!(p.symbols.is_empty());
        assert_eq!(p.redaction_summary.matched, 0);
        assert!(!p.redaction_summary.truncated);
    }

    #[test]
    fn absolute_path_symbol_is_dropped_not_leaked() {
        // CE-5 defence in depth: an absolute path should never be resident, but
        // if one is, the projector must not emit it.
        let g = graph_of(vec![
            node(
                1,
                "ok",
                "src/a.ts",
                SymbolKind::Function,
                Visibility::Public,
            ),
            node(
                2,
                "leaked_unix",
                "/etc/passwd",
                SymbolKind::Function,
                Visibility::Public,
            ),
            node(
                3,
                "leaked_windows",
                "C:\\Users\\runneradmin\\secret.ts",
                SymbolKind::Function,
                Visibility::Public,
            ),
            node(
                4,
                "leaked_unc",
                "\\\\server\\share\\secret.ts",
                SymbolKind::Function,
                Visibility::Public,
            ),
        ]);
        let p = run(&g, &SearchSymbolsQuery::default());
        assert_eq!(p.symbols.len(), 1);
        assert_eq!(p.symbols[0].identity.file, "src/a.ts");
        assert!(
            p.symbols
                .iter()
                .all(|s| !is_absolute_path_like(&s.identity.file)),
            "no absolute path may reach the projection"
        );
    }

    // --- CE-6 opaque pagination cursors ---

    fn five_symbol_graph() -> SymbolGraph {
        graph_of(vec![
            node(1, "a", "src/a.ts", SymbolKind::Function, Visibility::Public),
            node(2, "b", "src/b.ts", SymbolKind::Function, Visibility::Public),
            node(3, "c", "src/c.ts", SymbolKind::Function, Visibility::Public),
            node(4, "d", "src/d.ts", SymbolKind::Function, Visibility::Public),
            node(5, "e", "src/e.ts", SymbolKind::Function, Visibility::Public),
        ])
    }

    #[test]
    fn pagination_walks_all_pages_without_overlap_or_gap() {
        let g = five_symbol_graph();
        let mut seen: Vec<String> = Vec::new();
        let mut cursor = None;
        let mut pages = 0;
        loop {
            let query = SearchSymbolsQuery {
                limit: Some(2),
                cursor: cursor.clone(),
                ..Default::default()
            };
            let p = run(&g, &query);
            assert!(p.symbols.len() <= 2);
            assert_eq!(p.redaction_summary.matched, 5);
            // `truncated` tracks `next_cursor` exactly — true on pages 1-2,
            // false on the final page 3 (never a "stuck truncated" last page).
            assert_eq!(p.redaction_summary.truncated, p.next_cursor.is_some());
            seen.extend(p.symbols.iter().map(|s| s.identity.file.clone()));
            pages += 1;
            assert!(pages <= 5, "pagination must terminate");
            match p.next_cursor {
                Some(c) => cursor = Some(c),
                None => break,
            }
        }
        assert_eq!(pages, 3, "5 items at page size 2 → 3 pages");
        assert_eq!(
            seen,
            ["src/a.ts", "src/b.ts", "src/c.ts", "src/d.ts", "src/e.ts"],
            "every item exactly once, in identity order"
        );
    }

    #[test]
    fn cursor_from_a_different_query_is_rejected() {
        let g = five_symbol_graph();
        let cursor = run(
            &g,
            &SearchSymbolsQuery {
                limit: Some(2),
                ..Default::default()
            },
        )
        .next_cursor
        .expect("more pages remain");

        // Echo the default-query cursor back with a *different* filter set.
        let mismatched = SearchSymbolsQuery {
            name: Some("a".into()),
            limit: Some(2),
            cursor: Some(cursor),
            ..Default::default()
        };
        let (candidates, _omitted) = GctxProjector::collect_candidates(&g, &mismatched);
        let result = GctxProjector::project(candidates, &mismatched, 0);
        assert!(result.is_err(), "a cursor is only valid for its own query");
    }

    #[test]
    fn malformed_cursor_is_rejected() {
        let g = five_symbol_graph();
        let (candidates, _omitted) =
            GctxProjector::collect_candidates(&g, &SearchSymbolsQuery::default());
        let result = GctxProjector::project(
            candidates,
            &SearchSymbolsQuery {
                cursor: Some(OpaqueCursor::new("not-hex-zzzz".into())),
                ..Default::default()
            },
            0,
        );
        assert!(result.is_err(), "a malformed cursor must be rejected");
    }

    #[test]
    fn pagination_is_robust_to_deletion_between_pages() {
        let mut g = five_symbol_graph();
        let page1 = run(
            &g,
            &SearchSymbolsQuery {
                limit: Some(2),
                ..Default::default()
            },
        );
        assert_eq!(page1.symbols[1].identity.file, "src/b.ts");
        let cursor = page1.next_cursor.expect("more pages remain");

        // Remove the cursor's own last item (b.ts) between pages. Keyset resume
        // must still continue strictly after b — no gap, no re-yield.
        g.remove_file("src/b.ts");
        let page2 = run(
            &g,
            &SearchSymbolsQuery {
                limit: Some(2),
                cursor: Some(cursor),
                ..Default::default()
            },
        );
        let files: Vec<&str> = page2
            .symbols
            .iter()
            .map(|s| s.identity.file.as_str())
            .collect();
        assert_eq!(
            files,
            ["src/c.ts", "src/d.ts"],
            "resumes after b, not before"
        );
    }

    #[test]
    fn cursor_survives_a_filter_case_change_between_pages() {
        // Two symbols both match a case-insensitive name filter "x".
        let g = graph_of(vec![
            node(
                1,
                "Xavier",
                "src/a.ts",
                SymbolKind::Function,
                Visibility::Public,
            ),
            node(
                2,
                "xenon",
                "src/b.ts",
                SymbolKind::Function,
                Visibility::Public,
            ),
        ]);
        let cursor = run(
            &g,
            &SearchSymbolsQuery {
                name: Some("X".into()),
                limit: Some(1),
                ..Default::default()
            },
        )
        .next_cursor
        .expect("a second page remains");

        // Resume with the *lower-case* filter — same results, so the cursor must
        // still be accepted (fingerprint normalises case).
        let resumed = SearchSymbolsQuery {
            name: Some("x".into()),
            limit: Some(1),
            cursor: Some(cursor),
            ..Default::default()
        };
        let (candidates, _omitted) = GctxProjector::collect_candidates(&g, &resumed);
        let page2 = GctxProjector::project(candidates, &resumed, 0)
            .expect("a case-only filter change keeps the cursor valid");
        assert_eq!(page2.symbols.len(), 1);
        assert_eq!(page2.symbols[0].identity.file, "src/b.ts");
    }

    #[test]
    fn oversized_cursor_is_rejected() {
        let g = five_symbol_graph();
        let (candidates, _omitted) =
            GctxProjector::collect_candidates(&g, &SearchSymbolsQuery::default());
        let result = GctxProjector::project(
            candidates,
            &SearchSymbolsQuery {
                cursor: Some(OpaqueCursor::new("a".repeat(MAX_CURSOR_BYTES + 1))),
                ..Default::default()
            },
            0,
        );
        assert!(result.is_err(), "an oversized cursor must be rejected");
    }

    // --- GCTX-011 find_dependents traversal ---

    /// Build a `DependencyGraph` from `importer → imported` edges.
    fn dep_graph(edges: &[(&str, &str)]) -> DependencyGraph {
        let mut g = DependencyGraph::new();
        for (from, to) in edges {
            g.add_dependency((*from).to_string(), (*to).to_string());
        }
        g
    }

    fn run_dependents(
        dep: &DependencyGraph,
        file: &str,
        depth: u32,
        query: &FindDependentsQuery,
    ) -> FindDependentsProjection {
        let (candidates, walk_truncated, omitted) =
            GctxProjector::collect_dependents(dep, file, depth);
        GctxProjector::project_dependents(candidates, query, depth, walk_truncated, omitted)
            .expect("valid query")
    }

    fn files_and_distances(p: &FindDependentsProjection) -> Vec<(String, u32)> {
        p.dependents
            .iter()
            .map(|d| (d.file.clone(), d.distance))
            .collect()
    }

    #[test]
    fn dependents_chain_reports_distance_per_hop() {
        // c imports b imports a. Dependents of a: b at 1, c at 2 (within 2 hops).
        let g = dep_graph(&[("src/b.ts", "src/a.ts"), ("src/c.ts", "src/b.ts")]);
        let p = run_dependents(&g, "src/a.ts", 2, &FindDependentsQuery::default());
        assert_eq!(
            files_and_distances(&p),
            vec![("src/b.ts".into(), 1), ("src/c.ts".into(), 2)],
        );
        assert_eq!(p.redaction_summary.matched, 2);
        assert!(!p.redaction_summary.truncated);
    }

    #[test]
    fn dependents_depth_one_stops_at_direct_importers() {
        let g = dep_graph(&[("src/b.ts", "src/a.ts"), ("src/c.ts", "src/b.ts")]);
        let p = run_dependents(&g, "src/a.ts", 1, &FindDependentsQuery::default());
        // Only the direct importer at 1 hop; c.ts (2 hops) is excluded.
        assert_eq!(files_and_distances(&p), vec![("src/b.ts".into(), 1)]);
    }

    #[test]
    fn dependents_diamond_reports_each_importer_at_min_distance() {
        // b and c both import a; d imports both b and c. Dependents of a:
        // b@1, c@1, d@2 — d reached via two 2-hop paths but reported once.
        let g = dep_graph(&[
            ("src/b.ts", "src/a.ts"),
            ("src/c.ts", "src/a.ts"),
            ("src/d.ts", "src/b.ts"),
            ("src/d.ts", "src/c.ts"),
        ]);
        let p = run_dependents(&g, "src/a.ts", 2, &FindDependentsQuery::default());
        assert_eq!(
            files_and_distances(&p),
            vec![
                ("src/b.ts".into(), 1),
                ("src/c.ts".into(), 1),
                ("src/d.ts".into(), 2),
            ],
        );
    }

    #[test]
    fn dependents_cycle_terminates_and_excludes_origin() {
        // a → b → a is a cycle (b imports a, a imports b). Dependents of a: just
        // b at 1; the walk terminates and a is never reported as its own
        // dependent even though it re-enters the frontier.
        let g = dep_graph(&[("src/b.ts", "src/a.ts"), ("src/a.ts", "src/b.ts")]);
        let p = run_dependents(&g, "src/a.ts", 2, &FindDependentsQuery::default());
        assert_eq!(files_and_distances(&p), vec![("src/b.ts".into(), 1)]);
        assert!(
            p.dependents.iter().all(|d| d.file != "src/a.ts"),
            "origin must never appear as its own dependent",
        );
    }

    #[test]
    fn dependents_max_depth_truncates_the_walk() {
        // A 3-deep chain queried at depth 2 must omit the 3-hop importer — the
        // caller-clamped depth is the only walk bound here.
        let g = dep_graph(&[
            ("src/b.ts", "src/a.ts"),
            ("src/c.ts", "src/b.ts"),
            ("src/d.ts", "src/c.ts"),
        ]);
        let p = run_dependents(&g, "src/a.ts", 2, &FindDependentsQuery::default());
        let files: Vec<String> = p.dependents.iter().map(|d| d.file.clone()).collect();
        assert_eq!(files, vec!["src/b.ts".to_string(), "src/c.ts".to_string()]);
        assert!(
            !files.contains(&"src/d.ts".to_string()),
            "the 3-hop importer is beyond the depth-2 walk",
        );
    }

    #[test]
    fn dependents_with_no_importers_is_empty_not_truncated() {
        let g = dep_graph(&[("src/b.ts", "src/a.ts")]);
        // b.ts has no importers.
        let p = run_dependents(&g, "src/b.ts", 2, &FindDependentsQuery::default());
        assert!(p.dependents.is_empty());
        assert_eq!(p.redaction_summary.matched, 0);
        assert!(!p.redaction_summary.truncated);
    }

    #[test]
    fn dependents_pagination_walks_all_pages_without_overlap_or_gap() {
        // Five direct importers of a.ts, paged 2 at a time.
        let g = dep_graph(&[
            ("src/b.ts", "src/a.ts"),
            ("src/c.ts", "src/a.ts"),
            ("src/d.ts", "src/a.ts"),
            ("src/e.ts", "src/a.ts"),
            ("src/f.ts", "src/a.ts"),
        ]);
        let mut seen: Vec<String> = Vec::new();
        let mut cursor = None;
        let mut pages = 0;
        loop {
            let query = FindDependentsQuery {
                file: Some("src/a.ts".into()),
                limit: Some(2),
                cursor: cursor.clone(),
                ..Default::default()
            };
            let p = run_dependents(&g, "src/a.ts", 1, &query);
            assert!(p.dependents.len() <= 2);
            assert_eq!(p.redaction_summary.matched, 5);
            assert_eq!(p.redaction_summary.truncated, p.next_cursor.is_some());
            seen.extend(p.dependents.iter().map(|d| d.file.clone()));
            pages += 1;
            assert!(pages <= 5, "pagination must terminate");
            match p.next_cursor {
                Some(c) => cursor = Some(c),
                None => break,
            }
        }
        assert_eq!(pages, 3, "5 items at page size 2 → 3 pages");
        assert_eq!(
            seen,
            ["src/b.ts", "src/c.ts", "src/d.ts", "src/e.ts", "src/f.ts"],
            "every dependent exactly once, in file order",
        );
    }

    #[test]
    fn dependents_cursor_from_a_different_depth_is_rejected() {
        let g = dep_graph(&[
            ("src/b.ts", "src/a.ts"),
            ("src/c.ts", "src/a.ts"),
            ("src/d.ts", "src/a.ts"),
        ]);
        let query = FindDependentsQuery {
            file: Some("src/a.ts".into()),
            limit: Some(1),
            ..Default::default()
        };
        let cursor = run_dependents(&g, "src/a.ts", 1, &query)
            .next_cursor
            .expect("more pages remain");

        // Echo the depth-1 cursor against a depth-2 walk: the fingerprint differs.
        let mismatched = FindDependentsQuery {
            file: Some("src/a.ts".into()),
            limit: Some(1),
            cursor: Some(cursor),
            ..Default::default()
        };
        let (candidates, walk_truncated, _omitted) =
            GctxProjector::collect_dependents(&g, "src/a.ts", 2);
        let result =
            GctxProjector::project_dependents(candidates, &mismatched, 2, walk_truncated, 0);
        assert!(
            result.is_err(),
            "a cursor is only valid for the depth it was minted at",
        );
    }

    #[test]
    fn dependents_cursor_is_bound_to_the_exact_case_of_the_target_file() {
        // The dependency lookup is case-sensitive; a cursor minted for `src/a.ts`
        // must NOT be accepted for a `SRC/A.TS` query (which resolves to a
        // different/empty set), or pagination would overlap/gap.
        let g = dep_graph(&[("src/b.ts", "src/a.ts"), ("src/c.ts", "src/a.ts")]);
        let cursor = run_dependents(
            &g,
            "src/a.ts",
            1,
            &FindDependentsQuery {
                file: Some("src/a.ts".into()),
                limit: Some(1),
                ..Default::default()
            },
        )
        .next_cursor
        .expect("more pages remain");

        let cross_case = FindDependentsQuery {
            file: Some("SRC/A.TS".into()),
            limit: Some(1),
            cursor: Some(cursor),
            ..Default::default()
        };
        // Same depth, different-case file → different fingerprint → rejected.
        let (candidates, walk_truncated, _omitted) =
            GctxProjector::collect_dependents(&g, "SRC/A.TS", 1);
        let result =
            GctxProjector::project_dependents(candidates, &cross_case, 1, walk_truncated, 0);
        assert!(
            result.is_err(),
            "a cursor is bound to the exact case of its target file",
        );
    }

    #[test]
    fn dependents_malformed_and_oversized_cursors_are_rejected() {
        let g = dep_graph(&[("src/b.ts", "src/a.ts")]);
        let (candidates, walk_truncated, _omitted) =
            GctxProjector::collect_dependents(&g, "src/a.ts", 1);
        let malformed = GctxProjector::project_dependents(
            candidates.clone(),
            &FindDependentsQuery {
                cursor: Some(OpaqueCursor::new("not-hex-zzzz".into())),
                ..Default::default()
            },
            1,
            walk_truncated,
            0,
        );
        assert!(malformed.is_err(), "a malformed cursor must be rejected");

        let oversized = GctxProjector::project_dependents(
            candidates,
            &FindDependentsQuery {
                cursor: Some(OpaqueCursor::new("a".repeat(MAX_CURSOR_BYTES + 1))),
                ..Default::default()
            },
            1,
            walk_truncated,
            0,
        );
        assert!(oversized.is_err(), "an oversized cursor must be rejected");
    }

    #[test]
    fn dependents_walk_is_deterministic_regardless_of_insertion_order() {
        // The dependency index is a HashSet; the sorted-frontier walk must yield a
        // stable order so the budget truncation and keyset cursor are reproducible.
        let edges = [
            ("src/z.ts", "src/a.ts"),
            ("src/m.ts", "src/a.ts"),
            ("src/b.ts", "src/a.ts"),
        ];
        let first = run_dependents(
            &dep_graph(&edges),
            "src/a.ts",
            1,
            &FindDependentsQuery::default(),
        );
        let mut reversed = edges;
        reversed.reverse();
        let second = run_dependents(
            &dep_graph(&reversed),
            "src/a.ts",
            1,
            &FindDependentsQuery::default(),
        );
        assert_eq!(files_and_distances(&first), files_and_distances(&second));
        assert_eq!(
            first
                .dependents
                .iter()
                .map(|d| d.file.clone())
                .collect::<Vec<_>>(),
            ["src/b.ts", "src/m.ts", "src/z.ts"],
        );
    }

    #[test]
    fn dependents_rejects_a_search_surface_cursor() {
        // A search cursor must never be accepted by find_dependents. Its payload
        // carries a `SymbolIdentity` object for `k` (vs a string here), so decode
        // fails outright — the domain-separated fingerprint is a second guard.
        let g = five_symbol_graph();
        let search_cursor = GctxProjector::project(
            GctxProjector::collect_candidates(
                &g,
                &SearchSymbolsQuery {
                    limit: Some(1),
                    ..Default::default()
                },
            )
            .0,
            &SearchSymbolsQuery {
                limit: Some(1),
                ..Default::default()
            },
            0,
        )
        .expect("more pages")
        .next_cursor
        .expect("a search cursor");

        let dep = dep_graph(&[("src/b.ts", "src/a.ts")]);
        let (candidates, walk_truncated, _omitted) =
            GctxProjector::collect_dependents(&dep, "src/a.ts", 1);
        let result = GctxProjector::project_dependents(
            candidates,
            &FindDependentsQuery {
                file: Some("src/a.ts".into()),
                cursor: Some(search_cursor),
                ..Default::default()
            },
            1,
            walk_truncated,
            0,
        );
        assert!(
            result.is_err(),
            "a search cursor must not seek a dependents page"
        );
    }

    #[test]
    fn dependents_absolute_path_importer_is_dropped() {
        // CE-5 defence in depth: an absolute importer path must not be emitted.
        let g = dep_graph(&[("src/b.ts", "src/a.ts"), ("/etc/evil.ts", "src/a.ts")]);
        let p = run_dependents(&g, "src/a.ts", 1, &FindDependentsQuery::default());
        assert_eq!(files_and_distances(&p), vec![("src/b.ts".into(), 1)]);
        assert!(
            p.dependents.iter().all(|d| !is_absolute_path_like(&d.file)),
            "no absolute path may reach the projection",
        );
    }

    // --- GCTX-014 find_callers pagination + cursor binding ---

    fn target_identity(name: &str) -> SymbolIdentity {
        SymbolIdentity {
            file: "src/target.ts".into(),
            kind: SymbolKind::Function,
            name: name.into(),
            ordinal: 0,
        }
    }

    /// Build caller candidates directly (the unit under test for pagination is
    /// `project_callers`, which takes the collected `Vec<CallerSummary>`).
    fn caller_candidates(files: &[&str]) -> Vec<CallerSummary> {
        files
            .iter()
            .map(|f| CallerSummary {
                caller: SymbolIdentity {
                    file: (*f).into(),
                    kind: SymbolKind::Function,
                    name: "caller".into(),
                    ordinal: 0,
                },
                distance: 1,
                heuristic: false,
            })
            .collect()
    }

    #[test]
    fn callers_pagination_walks_all_pages_without_overlap_or_gap() {
        // Five callers, paged 2 at a time: every caller exactly once, in identity
        // order, and `truncated` tracks `next_cursor` (no stuck last page).
        let files = ["src/b.ts", "src/c.ts", "src/d.ts", "src/e.ts", "src/f.ts"];
        let mut seen: Vec<String> = Vec::new();
        let mut cursor = None;
        let mut pages = 0;
        loop {
            let query = FindCallersQuery {
                target: Some(target_identity("hot")),
                limit: Some(2),
                cursor: cursor.clone(),
                ..Default::default()
            };
            let p = GctxProjector::project_callers(
                caller_candidates(&files),
                &query,
                1,
                false,
                false,
                0,
            )
            .expect("valid query");
            assert!(p.callers.len() <= 2);
            assert_eq!(p.redaction_summary.matched, 5);
            assert_eq!(p.redaction_summary.truncated, p.next_cursor.is_some());
            seen.extend(p.callers.iter().map(|c| c.caller.file.clone()));
            pages += 1;
            assert!(pages <= 5, "pagination must terminate");
            match p.next_cursor {
                Some(c) => cursor = Some(c),
                None => break,
            }
        }
        assert_eq!(pages, 3, "5 items at page size 2 → 3 pages");
        assert_eq!(
            seen,
            ["src/b.ts", "src/c.ts", "src/d.ts", "src/e.ts", "src/f.ts"],
            "every caller exactly once, in identity order",
        );
    }

    #[test]
    fn callers_cursor_from_a_different_depth_is_rejected() {
        let files = ["src/b.ts", "src/c.ts", "src/d.ts"];
        let cursor = GctxProjector::project_callers(
            caller_candidates(&files),
            &FindCallersQuery {
                target: Some(target_identity("hot")),
                limit: Some(1),
                ..Default::default()
            },
            1,
            false,
            false,
            0,
        )
        .expect("valid query")
        .next_cursor
        .expect("more pages remain");

        // Echo the depth-1 cursor against a depth-2 walk: the fingerprint differs.
        let mismatched = FindCallersQuery {
            target: Some(target_identity("hot")),
            limit: Some(1),
            cursor: Some(cursor),
            ..Default::default()
        };
        let result = GctxProjector::project_callers(
            caller_candidates(&files),
            &mismatched,
            2,
            false,
            false,
            0,
        );
        assert!(
            result.is_err(),
            "a cursor is only valid for the depth it was minted at",
        );
    }

    #[test]
    fn callers_cursor_from_a_different_target_is_rejected() {
        // A cursor minted for `hot`'s callers must not seek `cold`'s page — the
        // result set differs, so accepting it would overlap/gap pagination.
        let files = ["src/b.ts", "src/c.ts", "src/d.ts"];
        let cursor = GctxProjector::project_callers(
            caller_candidates(&files),
            &FindCallersQuery {
                target: Some(target_identity("hot")),
                limit: Some(1),
                ..Default::default()
            },
            1,
            false,
            false,
            0,
        )
        .expect("valid query")
        .next_cursor
        .expect("more pages remain");

        let mismatched = FindCallersQuery {
            target: Some(target_identity("cold")),
            limit: Some(1),
            cursor: Some(cursor),
            ..Default::default()
        };
        let result = GctxProjector::project_callers(
            caller_candidates(&files),
            &mismatched,
            1,
            false,
            false,
            0,
        );
        assert!(
            result.is_err(),
            "a cursor is bound to the exact target symbol it was minted for",
        );
    }

    #[test]
    fn callers_malformed_and_oversized_cursors_are_rejected() {
        let files = ["src/b.ts", "src/c.ts"];
        let malformed = GctxProjector::project_callers(
            caller_candidates(&files),
            &FindCallersQuery {
                target: Some(target_identity("hot")),
                cursor: Some(OpaqueCursor::new("not-hex-zzzz".into())),
                ..Default::default()
            },
            1,
            false,
            false,
            0,
        );
        assert!(malformed.is_err(), "a malformed cursor must be rejected");

        let oversized = GctxProjector::project_callers(
            caller_candidates(&files),
            &FindCallersQuery {
                target: Some(target_identity("hot")),
                cursor: Some(OpaqueCursor::new("a".repeat(MAX_CURSOR_BYTES + 1))),
                ..Default::default()
            },
            1,
            false,
            false,
            0,
        );
        assert!(oversized.is_err(), "an oversized cursor must be rejected");
    }

    #[test]
    fn callers_reject_a_dependents_surface_cursor() {
        // A dependents cursor must never seek a callers page: its payload carries a
        // `file` string (vs a `SymbolIdentity` here) so decode fails, and the
        // domain-separated surface tag is a second guard.
        let dep = dep_graph(&[("src/b.ts", "src/a.ts"), ("src/c.ts", "src/a.ts")]);
        let (candidates, walk_truncated, _omitted) =
            GctxProjector::collect_dependents(&dep, "src/a.ts", 1);
        let dependents_cursor = GctxProjector::project_dependents(
            candidates,
            &FindDependentsQuery {
                file: Some("src/a.ts".into()),
                limit: Some(1),
                ..Default::default()
            },
            1,
            walk_truncated,
            0,
        )
        .expect("valid query")
        .next_cursor
        .expect("two importers at page size 1 leaves a next page");

        let result = GctxProjector::project_callers(
            caller_candidates(&["src/b.ts"]),
            &FindCallersQuery {
                target: Some(target_identity("hot")),
                cursor: Some(dependents_cursor),
                ..Default::default()
            },
            1,
            false,
            false,
            0,
        );
        assert!(
            result.is_err(),
            "a dependents cursor must not seek a callers page",
        );
    }

    #[test]
    fn callers_partial_marks_truncated_or_unclean_graph() {
        let q = FindCallersQuery {
            target: Some(target_identity("hot")),
            ..Default::default()
        };
        let clean = GctxProjector::project_callers(
            caller_candidates(&["src/b.ts"]),
            &q,
            1,
            false,
            false,
            0,
        )
        .expect("valid");
        assert!(
            !clean.partial,
            "a complete walk on a clean graph is not partial"
        );
        let walk_bound =
            GctxProjector::project_callers(caller_candidates(&["src/b.ts"]), &q, 1, true, false, 0)
                .expect("valid");
        assert!(walk_bound.partial, "a node-budget-bound walk is partial");
        let unclean =
            GctxProjector::project_callers(caller_candidates(&["src/b.ts"]), &q, 1, false, true, 0)
                .expect("valid");
        assert!(unclean.partial, "a non-Clean graph is partial");
    }

    #[test]
    fn callers_absolute_path_caller_is_dropped() {
        // CE-5 defence in depth: a caller resident with an absolute path must not
        // egress via `CallerSummary.caller.file`.
        use anvil_kernel_types::{EdgeType, SymbolEdge};
        let mut g = graph_of(vec![
            node(
                1,
                "callee",
                "src/callee.ts",
                SymbolKind::Function,
                Visibility::Public,
            ),
            node(
                2,
                "rel",
                "src/rel.ts",
                SymbolKind::Function,
                Visibility::Public,
            ),
            node(
                3,
                "abs",
                "/etc/evil.ts",
                SymbolKind::Function,
                Visibility::Public,
            ),
        ]);
        g.add_edge(SymbolEdge {
            from: 2,
            to: 1,
            edge_type: EdgeType::Calls,
        })
        .unwrap();
        g.add_edge(SymbolEdge {
            from: 3,
            to: 1,
            edge_type: EdgeType::Calls,
        })
        .unwrap();

        let callee = SymbolIdentity {
            file: "src/callee.ts".into(),
            kind: SymbolKind::Function,
            name: "callee".into(),
            ordinal: 0,
        };
        let (callers, _truncated, _omitted) = GctxProjector::collect_callers(&g, &callee, 1);
        let files: Vec<&str> = callers.iter().map(|c| c.caller.file.as_str()).collect();
        assert_eq!(files, ["src/rel.ts"], "the absolute-path caller is dropped");
        assert!(
            callers
                .iter()
                .all(|c| !is_absolute_path_like(&c.caller.file)),
            "no absolute path may reach the caller projection",
        );
    }

    // --- GCTX-012 impact_of_change ---

    fn run_impact(
        sym: &SymbolGraph,
        dep: &DependencyGraph,
        changed: &[&str],
        depth: u32,
    ) -> anvil_gctx_types::ImpactReport {
        let changed: Vec<String> = changed.iter().map(ToString::to_string).collect();
        let collected = GctxProjector::collect_impact(sym, dep, &changed, depth);
        GctxProjector::project_impact(collected)
    }

    #[test]
    fn impact_reports_affected_symbols_dependents_and_tests() {
        // a.ts defines `alpha`; b.ts imports a; a.test.ts imports a (a test).
        let sym = graph_of(vec![
            node(1, "alpha", "a.ts", SymbolKind::Function, Visibility::Public),
            node(2, "beta", "b.ts", SymbolKind::Function, Visibility::Public),
        ]);
        let dep = dep_graph(&[("b.ts", "a.ts"), ("a.test.ts", "a.ts")]);

        let report = run_impact(&sym, &dep, &["a.ts"], 1);

        // Affected = symbols defined in the changed file.
        assert_eq!(report.affected_symbols.len(), 1);
        assert_eq!(report.affected_symbols[0].identity.name, "alpha");
        // Dependents = importers of a.ts (b.ts + a.test.ts), file-ordered.
        let deps: Vec<&str> = report
            .dependent_files
            .iter()
            .map(|d| d.file.as_str())
            .collect();
        assert_eq!(deps, ["a.test.ts", "b.ts"]);
        // known_tests = the heuristic test subset of the dependents.
        assert_eq!(report.known_tests, ["a.test.ts"]);
        assert_eq!(report.summary.changed_files, 1);
        assert_eq!(report.summary.affected_symbols, 1);
        assert_eq!(report.summary.dependent_files, 2);
        assert_eq!(report.summary.known_tests, 1);
        assert!(!report.summary.truncated);
    }

    #[test]
    fn impact_three_file_change_unions_and_dedups_dependents() {
        // Changed: a.ts, b.ts, c.ts. d.ts imports a AND b (one dependent, once).
        let sym = graph_of(vec![
            node(1, "a", "a.ts", SymbolKind::Function, Visibility::Public),
            node(2, "b", "b.ts", SymbolKind::Function, Visibility::Public),
            node(3, "c", "c.ts", SymbolKind::Function, Visibility::Public),
        ]);
        let dep = dep_graph(&[("d.ts", "a.ts"), ("d.ts", "b.ts"), ("e.ts", "c.ts")]);

        let report = run_impact(&sym, &dep, &["a.ts", "b.ts", "c.ts"], 1);

        assert_eq!(report.summary.affected_symbols, 3);
        // d.ts (imports a+b, deduped) and e.ts (imports c), file-ordered.
        let deps: Vec<&str> = report
            .dependent_files
            .iter()
            .map(|d| d.file.as_str())
            .collect();
        assert_eq!(deps, ["d.ts", "e.ts"]);
    }

    #[test]
    fn impact_excludes_changed_files_from_their_own_dependents() {
        // a.ts and b.ts both changed; b imports a. b must NOT appear as a
        // dependent (it is part of the change set), and the closure is bounded.
        let sym = graph_of(vec![
            node(1, "a", "a.ts", SymbolKind::Function, Visibility::Public),
            node(2, "b", "b.ts", SymbolKind::Function, Visibility::Public),
        ]);
        let dep = dep_graph(&[("b.ts", "a.ts"), ("c.ts", "b.ts")]);

        let report = run_impact(&sym, &dep, &["a.ts", "b.ts"], 1);
        let deps: Vec<&str> = report
            .dependent_files
            .iter()
            .map(|d| d.file.as_str())
            .collect();
        // c.ts imports b.ts (changed) → distance 1; b.ts excluded as a seed.
        assert_eq!(deps, ["c.ts"]);
        assert!(report.dependent_files.iter().all(|d| d.file != "b.ts"));
    }

    #[test]
    fn impact_is_deterministic_regardless_of_input_order() {
        let sym = graph_of(vec![
            node(1, "a", "a.ts", SymbolKind::Function, Visibility::Public),
            node(2, "b", "b.ts", SymbolKind::Function, Visibility::Public),
        ]);
        let dep = dep_graph(&[("z.ts", "a.ts"), ("m.ts", "b.ts"), ("q.ts", "a.ts")]);

        let first = run_impact(&sym, &dep, &["a.ts", "b.ts"], 1);
        let second = run_impact(&sym, &dep, &["b.ts", "a.ts"], 1);
        assert_eq!(first, second);
        let deps: Vec<&str> = first
            .dependent_files
            .iter()
            .map(|d| d.file.as_str())
            .collect();
        assert_eq!(deps, ["m.ts", "q.ts", "z.ts"]);
    }

    #[test]
    fn impact_empty_change_set_is_an_empty_report() {
        let sym = graph_of(vec![node(
            1,
            "a",
            "a.ts",
            SymbolKind::Function,
            Visibility::Public,
        )]);
        let dep = dep_graph(&[("b.ts", "a.ts")]);
        let report = run_impact(&sym, &dep, &[], 1);
        assert!(report.affected_symbols.is_empty());
        assert!(report.dependent_files.is_empty());
        assert!(report.known_tests.is_empty());
        assert_eq!(report.summary.changed_files, 0);
    }

    #[test]
    fn impact_drops_absolute_changed_path() {
        // CE-5: an absolute changed path is not used as a seed nor surfaced.
        let sym = graph_of(vec![node(
            1,
            "a",
            "a.ts",
            SymbolKind::Function,
            Visibility::Public,
        )]);
        let dep = dep_graph(&[("b.ts", "a.ts")]);
        let report = run_impact(&sym, &dep, &["a.ts", "/etc/passwd"], 1);
        // Only a.ts seeds; the absolute path is dropped from BOTH the surface and
        // the `changed_files` count (the summary never over-reports the input).
        assert_eq!(report.affected_symbols.len(), 1);
        assert_eq!(
            report
                .dependent_files
                .iter()
                .map(|d| d.file.as_str())
                .collect::<Vec<_>>(),
            ["b.ts"]
        );
        assert_eq!(report.summary.changed_files, 1);
    }

    #[test]
    fn impact_affected_cap_still_seeds_later_files_for_dependents() {
        // big.ts exceeds MAX_AFFECTED_SYMBOLS; small.ts (later in the input) must
        // still be seeded so its dependent is found and the count stays correct —
        // the affected cap bounds symbol *collection*, never seed coverage.
        let mut nodes = Vec::new();
        for i in 0..=(MAX_AFFECTED_SYMBOLS as u64) {
            nodes.push(node(
                i,
                &format!("s{i}"),
                "big.ts",
                SymbolKind::Function,
                Visibility::Public,
            ));
        }
        nodes.push(node(
            9_999_999,
            "small",
            "small.ts",
            SymbolKind::Function,
            Visibility::Public,
        ));
        let sym = graph_of(nodes);
        let dep = dep_graph(&[("dep_of_small.ts", "small.ts")]);

        let report = run_impact(&sym, &dep, &["big.ts", "small.ts"], 1);
        assert!(
            report.summary.truncated,
            "the affected-symbol cap must mark the report truncated"
        );
        assert_eq!(report.affected_symbols.len(), MAX_AFFECTED_SYMBOLS);
        assert!(
            report
                .dependent_files
                .iter()
                .any(|d| d.file == "dep_of_small.ts"),
            "a later changed file must still seed the dependent walk despite the cap",
        );
        assert_eq!(report.summary.changed_files, 2, "both files are counted");
    }

    #[test]
    fn impact_affected_cap_is_independent_of_input_file_order() {
        let mut nodes = Vec::new();
        for i in 0..4 {
            nodes.push(node(
                i,
                &format!("z{i}"),
                "a.ts",
                SymbolKind::Function,
                Visibility::Public,
            ));
        }
        for i in 4..8 {
            nodes.push(node(
                i,
                &format!("y{i}"),
                "b.ts",
                SymbolKind::Function,
                Visibility::Public,
            ));
        }
        let sym = graph_of(nodes);
        let dep = dep_graph(&[]);
        let order_ab: Vec<String> = vec!["a.ts".into(), "b.ts".into()];
        let order_ba: Vec<String> = vec!["b.ts".into(), "a.ts".into()];
        let first = GctxProjector::project_impact(GctxProjector::collect_impact_with_budget(
            &sym, &dep, &order_ab, 1, 3,
        ));
        let second = GctxProjector::project_impact(GctxProjector::collect_impact_with_budget(
            &sym, &dep, &order_ba, 1, 3,
        ));
        assert_eq!(first, second);
        assert!(first.summary.truncated);
        assert_eq!(first.affected_symbols.len(), 3);
    }

    /// CIB-091c: the bounded max-heap that replaced the O(n) sorted `Vec::insert`
    /// under the cache lock must preserve the existing semantics — when over the
    /// affected-symbol cap, keep the identity-SMALLEST prefix (not insertion or
    /// input order). Symbols are inserted in a deliberately non-sorted graph order;
    /// with a cap of 3 the report must hold the three smallest identities.
    #[test]
    fn impact_affected_cap_keeps_identity_smallest_prefix() {
        // One file `m.ts` defines five functions whose names sort `n0..n4`. The
        // identity order is by (file, kind, name, ordinal); same file + kind, so
        // the tie-break is name then ordinal. Insert them out of name order.
        let sym = graph_of(vec![
            node(3, "n3", "m.ts", SymbolKind::Function, Visibility::Public),
            node(1, "n1", "m.ts", SymbolKind::Function, Visibility::Public),
            node(4, "n4", "m.ts", SymbolKind::Function, Visibility::Public),
            node(0, "n0", "m.ts", SymbolKind::Function, Visibility::Public),
            node(2, "n2", "m.ts", SymbolKind::Function, Visibility::Public),
        ]);
        let dep = dep_graph(&[]);
        let changed = vec!["m.ts".to_string()];
        // Cap at 3 → keep the three identity-smallest. `for_file_symbols` assigns
        // ordinals in the file's parse order, so the surviving prefix is the three
        // smallest by the full identity order.
        let report = GctxProjector::project_impact(GctxProjector::collect_impact_with_budget(
            &sym, &dep, &changed, 1, 3,
        ));
        assert!(
            report.summary.truncated,
            "the cap must mark the report truncated"
        );
        assert_eq!(report.affected_symbols.len(), 3);
        // The result is identity-sorted by `project_impact`; assert it is exactly
        // the three smallest of the five identities present.
        let all = GctxProjector::project_impact(GctxProjector::collect_impact_with_budget(
            &sym,
            &dep,
            &changed,
            1,
            MAX_AFFECTED_SYMBOLS,
        ));
        assert_eq!(all.affected_symbols.len(), 5);
        let expected_prefix: Vec<_> = all
            .affected_symbols
            .iter()
            .take(3)
            .map(|s| s.identity.clone())
            .collect();
        let got: Vec<_> = report
            .affected_symbols
            .iter()
            .map(|s| s.identity.clone())
            .collect();
        assert_eq!(
            got, expected_prefix,
            "cap must keep the identity-smallest prefix"
        );
    }

    #[test]
    fn dependents_walk_budget_sets_partial_on_final_page() {
        let edges: Vec<(String, String)> = (0..5)
            .map(|i| (format!("src/imp{i}.ts"), "src/a.ts".to_string()))
            .collect();
        let edge_refs: Vec<(&str, &str)> = edges
            .iter()
            .map(|(from, to)| (from.as_str(), to.as_str()))
            .collect();
        let g = dep_graph(&edge_refs);
        let (candidates, walk_truncated, _omitted) =
            GctxProjector::collect_dependents_with_budget(&g, "src/a.ts", 1, 3);
        assert_eq!(candidates.len(), 3);
        assert!(walk_truncated);
        let projection = GctxProjector::project_dependents(
            candidates,
            &FindDependentsQuery {
                file: Some("src/a.ts".into()),
                ..Default::default()
            },
            1,
            walk_truncated,
            0,
        )
        .expect("valid query");
        assert!(projection.partial);
        assert!(!projection.redaction_summary.truncated);
        assert!(projection.next_cursor.is_none());
    }

    #[test]
    fn impact_changed_count_is_distinct_non_absolute_seeds() {
        let sym = graph_of(vec![
            node(1, "a", "a.ts", SymbolKind::Function, Visibility::Public),
            node(2, "b", "b.ts", SymbolKind::Function, Visibility::Public),
        ]);
        let dep = dep_graph(&[("c.ts", "a.ts")]);
        // Duplicate `a.ts` + an absolute path → 2 distinct usable seeds (a, b).
        let report = run_impact(&sym, &dep, &["a.ts", "a.ts", "b.ts", "/abs.ts"], 1);
        assert_eq!(report.summary.changed_files, 2);
    }

    #[test]
    fn is_test_file_recognises_common_conventions() {
        for p in [
            "src/a.test.ts",
            "src/a.spec.js",
            "tests/a.rs",
            "crate/tests/it.rs",
            "src/__tests__/a.ts",
            "src/a_test.rs",
            "src/a_spec.rb",
        ] {
            assert!(is_test_file(p), "{p} should be a test file");
        }
        for p in [
            "src/a.ts",
            "src/latest.ts",
            "src/contest.ts",
            "lib/spectrum.ts",
        ] {
            assert!(!is_test_file(p), "{p} should NOT be a test file");
        }
    }

    // --- GCTX-013 affected_tests ---

    fn run_affected_tests(
        dep: &DependencyGraph,
        changed: &[&str],
        depth: u32,
    ) -> anvil_gctx_types::AffectedTestsReport {
        let changed: Vec<String> = changed.iter().map(ToString::to_string).collect();
        let collected = GctxProjector::collect_affected_tests(dep, &changed, depth);
        GctxProjector::project_affected_tests(collected)
    }

    /// The GCTX-013 fixture: a changed source `s.ts` with a test `s.test.ts`
    /// importing it, and a second changed source `u.ts` with no test → the test
    /// appears with an evidence edge to `s.ts`, `u.ts` is a coverage gap, and the
    /// heuristic marker is set.
    #[test]
    fn affected_tests_attributes_test_and_flags_coverage_gap() {
        let dep = dep_graph(&[("s.test.ts", "s.ts")]);
        let report = run_affected_tests(&dep, &["s.ts", "u.ts"], 1);

        assert!(report.heuristic);
        assert_eq!(report.tests.len(), 1);
        assert_eq!(report.tests[0].file, "s.test.ts");
        assert_eq!(report.tests[0].changed_dependencies, ["s.ts"]);
        assert_eq!(report.tests[0].distance, 1);
        // s.ts is covered by s.test.ts; only u.ts is a gap.
        assert_eq!(report.coverage_gaps, ["u.ts"]);
        assert_eq!(report.summary.changed_files, 2);
        assert_eq!(report.summary.tests, 1);
        assert_eq!(report.summary.evidence_edges, 1);
        assert_eq!(report.summary.coverage_gaps, 1);
        assert!(!report.summary.truncated);
    }

    /// A test importing two changed files carries both as evidence edges (sorted),
    /// and both are covered (no gaps).
    #[test]
    fn affected_tests_evidence_unions_multiple_changed_imports() {
        // one.test.ts imports both changed sources a.ts and b.ts.
        let dep = dep_graph(&[("one.test.ts", "a.ts"), ("one.test.ts", "b.ts")]);
        let report = run_affected_tests(&dep, &["b.ts", "a.ts"], 1);

        assert_eq!(report.tests.len(), 1);
        assert_eq!(report.tests[0].changed_dependencies, ["a.ts", "b.ts"]);
        assert!(report.coverage_gaps.is_empty());
        assert_eq!(report.summary.evidence_edges, 2);
    }

    /// Transitive coverage: a test that reaches a changed file only through a
    /// non-test intermediate is found at distance 2 (with no direct evidence
    /// edge), and the changed file is covered — so it is NOT a gap at depth 2,
    /// but IS a gap at depth 1 (the test is out of reach).
    #[test]
    fn affected_tests_transitive_coverage_respects_depth() {
        // t.test.ts → m.ts → x.ts. x.ts is the changed source; m.ts a non-test
        // intermediate; t.test.ts the test two hops up.
        let dep = dep_graph(&[("m.ts", "x.ts"), ("t.test.ts", "m.ts")]);

        let depth2 = run_affected_tests(&dep, &["x.ts"], 2);
        assert_eq!(depth2.tests.len(), 1);
        assert_eq!(depth2.tests[0].file, "t.test.ts");
        assert_eq!(depth2.tests[0].distance, 2);
        // The test reaches x.ts only transitively → no direct evidence edge…
        assert!(depth2.tests[0].changed_dependencies.is_empty());
        // …but x.ts is still covered within the bound, so it is not a gap.
        assert!(depth2.coverage_gaps.is_empty());

        let depth1 = run_affected_tests(&dep, &["x.ts"], 1);
        // At depth 1 the test is out of reach: no tests, x.ts is a coverage gap.
        assert!(depth1.tests.is_empty());
        assert_eq!(depth1.coverage_gaps, ["x.ts"]);
    }

    /// Determinism: the report is identical regardless of changed-input order,
    /// with both `tests` and `coverage_gaps` in path order.
    #[test]
    fn affected_tests_is_deterministic_regardless_of_input_order() {
        let dep = dep_graph(&[
            ("z.test.ts", "a.ts"),
            ("m.test.ts", "b.ts"),
            ("q.test.ts", "a.ts"),
        ]);
        let first = run_affected_tests(&dep, &["a.ts", "b.ts", "c.ts"], 1);
        let second = run_affected_tests(&dep, &["c.ts", "b.ts", "a.ts"], 1);
        assert_eq!(first, second);
        let tests: Vec<&str> = first.tests.iter().map(|t| t.file.as_str()).collect();
        assert_eq!(tests, ["m.test.ts", "q.test.ts", "z.test.ts"]);
        // c.ts has no importer at all → the lone coverage gap.
        assert_eq!(first.coverage_gaps, ["c.ts"]);
    }

    /// CE-5: an absolute changed path is neither seeded nor counted, and a changed
    /// test file is never reported as a coverage gap (it is filtered as a test).
    #[test]
    fn affected_tests_drops_absolute_and_never_gaps_a_changed_test() {
        // a.ts changed (covered by a.test.ts which is ALSO changed); the absolute
        // path is dropped from the seed set and the count.
        let dep = dep_graph(&[("a.test.ts", "a.ts")]);
        let report = run_affected_tests(&dep, &["a.ts", "a.test.ts", "/etc/passwd"], 1);

        // Two usable seeds (a.ts, a.test.ts); the absolute path is dropped.
        assert_eq!(report.summary.changed_files, 2);
        // a.test.ts is a changed test → it is in `tests`, never a coverage gap.
        assert!(report.coverage_gaps.is_empty());
        // No emitted path is absolute.
        for test in &report.tests {
            assert!(!is_absolute_path_like(&test.file));
            assert!(
                test.changed_dependencies
                    .iter()
                    .all(|d| !is_absolute_path_like(d))
            );
        }
    }

    /// CE-5 / bounds: an absolute-path dependency node reached by the forward
    /// coverage walk is dropped — it can never be in the change set, so it neither
    /// covers anything nor burns the shared node budget. The real changed source
    /// reached past it stays covered (not a false gap).
    #[test]
    fn affected_tests_forward_walk_drops_absolute_dependency_nodes() {
        // t.test.ts imports an absolute node_modules path AND the changed x.ts.
        let dep = dep_graph(&[
            ("t.test.ts", "/abs/node_modules/pkg/index.js"),
            ("t.test.ts", "x.ts"),
        ]);
        let report = run_affected_tests(&dep, &["x.ts"], 1);

        assert_eq!(report.tests.len(), 1);
        assert_eq!(report.tests[0].file, "t.test.ts");
        // The absolute import is dropped from the evidence edge…
        assert_eq!(report.tests[0].changed_dependencies, ["x.ts"]);
        // …and x.ts is covered, so there is no coverage gap.
        assert!(report.coverage_gaps.is_empty());
    }

    /// An empty change set yields an empty report.
    #[test]
    fn affected_tests_empty_change_set_is_an_empty_report() {
        let dep = dep_graph(&[("b.test.ts", "a.ts")]);
        let report = run_affected_tests(&dep, &[], 1);
        assert!(report.tests.is_empty());
        assert!(report.coverage_gaps.is_empty());
        assert_eq!(report.summary.changed_files, 0);
        assert!(report.heuristic);
    }

    // --- GCTX-030 graph:// resources ---

    fn edge(
        from: u64,
        to: u64,
        edge_type: anvil_kernel_types::EdgeType,
    ) -> anvil_kernel_types::SymbolEdge {
        anvil_kernel_types::SymbolEdge {
            from,
            to,
            edge_type,
        }
    }

    fn edges_graph() -> SymbolGraph {
        use anvil_kernel_types::EdgeType;
        let mut g = graph_of(vec![
            node(1, "a", "src/a.ts", SymbolKind::Function, Visibility::Public),
            node(2, "b", "src/b.ts", SymbolKind::Function, Visibility::Public),
            node(
                3,
                "c",
                "src/a.ts",
                SymbolKind::Function,
                Visibility::Internal,
            ),
        ]);
        g.add_edge(edge(1, 2, EdgeType::Calls)).unwrap();
        g.add_edge(edge(3, 2, EdgeType::Imports)).unwrap();
        g
    }

    #[test]
    fn project_stats_constructs_counts() {
        let p = GctxProjector::project_stats(12, 30, 4, 7);
        assert_eq!(p.symbol_count, 12);
        assert_eq!(p.symbol_edge_count, 30);
        assert_eq!(p.file_count, 4);
        assert_eq!(p.dependency_edge_count, 7);
    }

    #[test]
    fn collect_all_edges_resolves_both_endpoints_to_identity() {
        let g = edges_graph();
        let (edges, bounded, _omitted) = GctxProjector::collect_all_edges(&g, None);
        // Both edges resolve (every endpoint is a resident, relative-path symbol).
        assert_eq!(edges.len(), 2);
        assert!(!bounded);
        // Every endpoint is identity-only with a relative path.
        for e in &edges {
            assert!(!e.from.file.starts_with('/'));
            assert!(!e.to.file.starts_with('/'));
        }
    }

    #[test]
    fn collect_all_edges_is_deterministic_across_calls() {
        // Determinism guard (council ADV-1/ADV-2): the sorted file + sorted edge
        // walk must yield byte-identical results on repeat calls, even though the
        // underlying file_names()/outgoing_edges() iteration order is unspecified.
        let g = edges_graph();
        let (first, _, _) = GctxProjector::collect_all_edges(&g, None);
        for _ in 0..8 {
            let (again, _, _) = GctxProjector::collect_all_edges(&g, None);
            assert_eq!(first, again, "collect_all_edges must be deterministic");
        }
    }

    #[test]
    fn project_edges_orders_and_paginates_deterministically() {
        let g = edges_graph();
        let (candidates, bounded, _omitted) = GctxProjector::collect_all_edges(&g, None);
        let page1 = GctxProjector::project_edges(
            candidates.clone(),
            &GraphEdgesQuery {
                limit: Some(1),
                ..Default::default()
            },
            bounded,
            0,
        )
        .unwrap();
        assert_eq!(page1.edges.len(), 1);
        assert_eq!(page1.redaction_summary.matched, 2);
        assert!(page1.redaction_summary.truncated);
        assert!(!page1.bounded);
        let cursor = page1.next_cursor.clone().expect("more pages");

        let page2 = GctxProjector::project_edges(
            candidates,
            &GraphEdgesQuery {
                limit: Some(1),
                cursor: Some(cursor),
                ..Default::default()
            },
            bounded,
            0,
        )
        .unwrap();
        assert_eq!(page2.edges.len(), 1);
        assert!(!page2.redaction_summary.truncated);
        // The two pages are disjoint and in ascending (from, to, edge_type) order.
        assert!(page1.edges[0] < page2.edges[0]);
    }

    #[test]
    fn project_edges_propagates_bounded_flag() {
        let g = edges_graph();
        let (candidates, _, _) = GctxProjector::collect_all_edges(&g, None);
        let p =
            GctxProjector::project_edges(candidates, &GraphEdgesQuery::default(), true, 0).unwrap();
        assert!(
            p.bounded,
            "the collection-bound signal must reach the projection"
        );
    }

    #[test]
    fn project_edges_rejects_cursor_from_a_different_filter() {
        let g = edges_graph();
        let (candidates, bounded, _omitted) = GctxProjector::collect_all_edges(&g, None);
        let page1 = GctxProjector::project_edges(
            candidates.clone(),
            &GraphEdgesQuery {
                limit: Some(1),
                ..Default::default()
            },
            bounded,
            0,
        )
        .unwrap();
        let cursor = page1.next_cursor.unwrap();
        // Same cursor, but now with a `file` filter → fingerprint mismatch.
        let err = GctxProjector::project_edges(
            candidates,
            &GraphEdgesQuery {
                file: Some("src/a.ts".into()),
                cursor: Some(cursor),
                ..Default::default()
            },
            bounded,
            0,
        )
        .unwrap_err();
        assert!(err.contains("does not match"), "{err}");
    }

    #[test]
    fn collect_all_edges_file_filter_scopes_to_source_file() {
        let g = edges_graph();
        // Only edges whose source symbol is in src/a.ts: a→b (id 1) and c→b (id 3).
        let (edges, _, _) = GctxProjector::collect_all_edges(&g, Some("src/a.ts"));
        assert_eq!(edges.len(), 2);
        assert!(edges.iter().all(|e| e.from.file == "src/a.ts"));
        // A filter matching no source file yields nothing.
        assert!(
            GctxProjector::collect_all_edges(&g, Some("src/missing.ts"))
                .0
                .is_empty()
        );
    }

    // --- CIB-091a: CE-3 sensitive-path egress deny-list ---

    #[test]
    fn is_sensitive_egress_path_matches_deny_list() {
        // Secret-directory segments (case-insensitive), anywhere in the path.
        for p in [
            ".git/config",
            "src/.git/HEAD",
            "secrets/token.txt",
            "config/secrets/db",
            ".aws/credentials",
            "home/.ssh/known_hosts",
            ".gnupg/pubring.kbx",
            "SECRETS/api",
            ".GIT/config",
        ] {
            assert!(is_sensitive_egress_path(p), "expected sensitive: {p}");
        }

        // `.env*` basenames (including dotted suffixes) anywhere in the path.
        for p in [
            ".env",
            ".env.production",
            "app/.env.local",
            "secrets/.env.production",
        ] {
            assert!(is_sensitive_egress_path(p), "expected sensitive: {p}");
        }

        // SSH private-key basenames — private key and its public half both stay
        // private, across the modern key-type conventions.
        for p in [
            ".ssh/id_rsa",
            "keys/id_rsa",
            "id_rsa.pub",
            "deploy/id_ecdsa",
            "ci/id_ed25519",
        ] {
            assert!(is_sensitive_egress_path(p), "expected sensitive: {p}");
        }

        // pem / key / p12 / pfx / p8 extensions.
        for p in [
            "lib/private.pem",
            "keys/app.key",
            "certs/bundle.p12",
            "deep/dir/x.PEM",
            "cert.pfx",
            "key.p8",
        ] {
            assert!(is_sensitive_egress_path(p), "expected sensitive: {p}");
        }
    }

    #[test]
    fn is_sensitive_egress_path_allows_ordinary_paths() {
        // Negatives: ordinary source files, including ones whose *stem* looks
        // secret-ish but whose extension/segment does not match.
        for p in [
            "config/app.ts",
            "src/handler.ts",
            "keys.ts",            // ext is `ts`, not `key`
            "src/keystore.ts",    // segment is not exactly `secrets`/`.ssh`/...
            "src/environment.ts", // basename does not start with `.env`
            "src/git/repo.ts",    // `git` segment, not `.git`
            "lib/awscli.ts",      // not the `.aws` segment
            "README.md",
        ] {
            assert!(!is_sensitive_egress_path(p), "expected NOT sensitive: {p}");
        }
    }

    /// CIB-091a structural no-leak: build symbol + dependency graphs containing
    /// sensitive files alongside ordinary ones, run every projection, and assert
    /// no sensitive path appears in ANY emitted field — and that the
    /// `omitted_sensitive_paths` counter is non-zero where drops occurred.
    #[test]
    #[allow(clippy::too_many_lines)] // one end-to-end gate across all six projections
    fn no_projection_egresses_a_sensitive_path() {
        use anvil_kernel_types::{EdgeType, SymbolEdge};

        const SENSITIVE: &[&str] = &[
            "secrets/.env.production",
            "lib/private.pem",
            ".ssh/id_rsa",
            "keys/app.key",
        ];
        let is_sensitive = |s: &str| SENSITIVE.iter().any(|p| s.contains(p));

        // Symbol graph: a normal callee, a normal caller, and one caller per
        // sensitive file all calling the callee.
        let mut sym = graph_of(vec![
            node(
                1,
                "callee",
                "src/callee.ts",
                SymbolKind::Function,
                Visibility::Public,
            ),
            node(
                2,
                "relCaller",
                "src/caller.ts",
                SymbolKind::Function,
                Visibility::Public,
            ),
            node(
                3,
                "secretEnv",
                "secrets/.env.production",
                SymbolKind::Function,
                Visibility::Public,
            ),
            node(
                4,
                "secretPem",
                "lib/private.pem",
                SymbolKind::Function,
                Visibility::Public,
            ),
            node(
                5,
                "secretKeyf",
                ".ssh/id_rsa",
                SymbolKind::Function,
                Visibility::Public,
            ),
            node(
                6,
                "secretKey",
                "keys/app.key",
                SymbolKind::Function,
                Visibility::Public,
            ),
        ]);
        for from in [2u64, 3, 4, 5, 6] {
            sym.add_edge(SymbolEdge {
                from,
                to: 1,
                edge_type: EdgeType::Calls,
            })
            .unwrap();
        }

        // search_symbols: no sensitive file in the symbol set.
        let search = run(&sym, &SearchSymbolsQuery::default());
        assert!(
            search
                .symbols
                .iter()
                .all(|s| !is_sensitive(&s.identity.file)),
            "search_symbols leaked a sensitive path",
        );
        assert!(
            search.redaction_summary.omitted_sensitive_paths >= SENSITIVE.len(),
            "search must count the dropped sensitive symbols",
        );

        // find_callers: only the relative caller survives; the count records the
        // four sensitive callers dropped.
        let callee = SymbolIdentity {
            file: "src/callee.ts".into(),
            kind: SymbolKind::Function,
            name: "callee".into(),
            ordinal: 0,
        };
        let (caller_candidates, walk_truncated, callers_omitted) =
            GctxProjector::collect_callers(&sym, &callee, 1);
        assert_eq!(
            callers_omitted,
            SENSITIVE.len(),
            "all four sensitive callers dropped"
        );
        let callers = GctxProjector::project_callers(
            caller_candidates,
            &FindCallersQuery::default(),
            1,
            walk_truncated,
            false,
            callers_omitted,
        )
        .expect("valid");
        assert!(
            callers
                .callers
                .iter()
                .all(|c| !is_sensitive(&c.caller.file)),
            "find_callers leaked a sensitive path",
        );
        assert!(callers.redaction_summary.omitted_sensitive_paths >= SENSITIVE.len());

        // graph://edges: neither endpoint of any edge is a sensitive file.
        let (edge_candidates, bounded, edges_omitted) =
            GctxProjector::collect_all_edges(&sym, None);
        assert!(
            edges_omitted >= SENSITIVE.len(),
            "edges must count sensitive drops"
        );
        let edges = GctxProjector::project_edges(
            edge_candidates,
            &GraphEdgesQuery::default(),
            bounded,
            edges_omitted,
        )
        .expect("valid");
        assert!(
            edges
                .edges
                .iter()
                .all(|e| { !is_sensitive(&e.from.file) && !is_sensitive(&e.to.file) }),
            "graph://edges leaked a sensitive path",
        );
        assert!(edges.redaction_summary.omitted_sensitive_paths >= SENSITIVE.len());

        // Dependency graph: each sensitive file imports `app.ts`, plus one normal
        // importer; and `app.ts` imports a sensitive file (forward edge for the
        // affected-tests coverage walk).
        let dep = dep_graph(&[
            ("src/importer.ts", "src/app.ts"),
            ("secrets/.env.production", "src/app.ts"),
            ("lib/private.pem", "src/app.ts"),
            (".ssh/id_rsa", "src/app.ts"),
            ("keys/app.key", "src/app.ts"),
        ]);

        // find_dependents: only the relative importer survives.
        let (dep_candidates, dep_truncated, dep_omitted) =
            GctxProjector::collect_dependents(&dep, "src/app.ts", 1);
        assert_eq!(
            dep_omitted,
            SENSITIVE.len(),
            "all four sensitive importers dropped"
        );
        let dependents = GctxProjector::project_dependents(
            dep_candidates,
            &FindDependentsQuery::default(),
            1,
            dep_truncated,
            dep_omitted,
        )
        .expect("valid");
        assert!(
            dependents.dependents.iter().all(|d| !is_sensitive(&d.file)),
            "find_dependents leaked a sensitive path",
        );
        assert!(dependents.redaction_summary.omitted_sensitive_paths >= SENSITIVE.len());

        // impact_of_change: changed sensitive seeds contribute no affected symbol
        // and no dependent; the count records them.
        let changed: Vec<String> = SENSITIVE.iter().map(ToString::to_string).collect();
        let impact =
            GctxProjector::project_impact(GctxProjector::collect_impact(&sym, &dep, &changed, 1));
        assert!(
            impact
                .affected_symbols
                .iter()
                .all(|s| !is_sensitive(&s.identity.file)),
            "impact affected_symbols leaked a sensitive path",
        );
        assert!(
            impact
                .dependent_files
                .iter()
                .all(|d| !is_sensitive(&d.file)),
            "impact dependent_files leaked a sensitive path",
        );
        assert!(
            impact.summary.omitted_sensitive_paths >= SENSITIVE.len(),
            "impact must count the dropped sensitive seeds",
        );

        // affected_tests: a sensitive changed file yields no test/gap leakage.
        let tests = GctxProjector::project_affected_tests(GctxProjector::collect_affected_tests(
            &dep, &changed, 1,
        ));
        assert!(
            tests.tests.iter().all(|t| {
                !is_sensitive(&t.file) && t.changed_dependencies.iter().all(|d| !is_sensitive(d))
            }),
            "affected_tests leaked a sensitive path",
        );
        assert!(
            tests.coverage_gaps.iter().all(|g| !is_sensitive(g)),
            "affected_tests coverage_gaps leaked a sensitive path",
        );
        assert!(tests.summary.omitted_sensitive_paths >= SENSITIVE.len());
    }
}
