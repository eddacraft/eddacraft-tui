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
    DependentSummary, FindDependentsProjection, FindDependentsQuery, OpaqueCursor,
    RedactionSummary, SearchSymbolsProjection, SearchSymbolsQuery, SymbolSummary,
};
use anvil_graph_cache::{DependencyGraph, SymbolGraph};
use anvil_kernel_types::{SymbolIdentity, SymbolKind, SymbolNode, Visibility};
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
    ) -> Vec<SymbolSummary> {
        // Lower-case the filters ONCE, not per node, so the lock-held loop does
        // no avoidable allocation (ADR-084 C2).
        let name_lc = query.name.as_deref().map(str::to_lowercase);
        let file_lc = query.file.as_deref().map(str::to_lowercase);

        let mut out = Vec::new();
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
        out
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
    ) -> Vec<DependentSummary> {
        debug_assert!(
            depth <= anvil_graph_cache::MAX_REVERSE_IMPACT_DEPTH,
            "dependents walk depth {depth} exceeds the ADR-063 cap {} (caller must clamp)",
            anvil_graph_cache::MAX_REVERSE_IMPACT_DEPTH,
        );
        let mut out: Vec<DependentSummary> = Vec::new();
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
                    // First (smallest) distance wins; a re-seen file is skipped.
                    if seen.insert(importer.to_string()) {
                        let importer = importer.to_string();
                        out.push(DependentSummary {
                            file: importer.clone(),
                            distance: hop,
                        });
                        next.push(importer);
                        if out.len() >= MAX_DEPENDENTS_WALK {
                            break 'walk;
                        }
                    }
                }
            }
            if next.is_empty() {
                break;
            }
            frontier = next;
        }
        out
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
            },
            dependents: page,
            next_cursor,
        })
    }
}

/// Cap on an echoed cursor's encoded length.
///
/// The cursor is **separately bounded** from the CE-6 "≤ 512 bytes/param" cap on
/// user-typed *filter* params (name/file/language): it is not user query text but
/// a server-minted opaque token the client echoes back, and it must hold a
/// hex-encoded [`SymbolIdentity`] whose `file` path can approach `PATH_MAX`
/// (~4 KiB) — hex doubling that already exceeds 512 bytes for a legitimate deep
/// path. 8 KiB comfortably covers a `PATH_MAX` identity while still bounding
/// hex-decode work on a hostile oversized token (the IPC frame cap is the outer
/// limit). A real cursor is a few hundred bytes.
const MAX_CURSOR_BYTES: usize = 8 * 1024;

/// Hard cap on the number of dependents [`GctxProjector::collect_dependents`]
/// materialises in a single lock-held pass. Two depth hops over a barrel file can
/// reach an arbitrarily large importer set; this bounds the lock-held allocation
/// (ADR-031) well above any honest page (`MAX_PAGE_LIMIT` is 200) while capping
/// the pathological case. Truncation is deterministic (sorted-frontier walk), so
/// keyset pagination stays stable across the bound.
const MAX_DEPENDENTS_WALK: usize = 10_000;

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
/// **resolved** depth (after the daemon clamps it) and the queried `file`
/// (case-normalised to match the search surface's convention). Changing `limit`
/// mid-walk is allowed (not part of the fingerprint); changing the target file or
/// the resolved depth invalidates a cursor. FNV-1a over the canonical serialised
/// filters (a reproducible, non-randomly-seeded hash — PV-2).
fn dependents_fingerprint(query: &FindDependentsQuery, depth: u32) -> u64 {
    #[derive(Serialize)]
    struct Filters {
        // A constant surface tag domain-separates this fingerprint from the
        // search surface (and any future GCTX traversal cursor), so a cursor
        // minted for one surface can never fingerprint-match another even if the
        // non-cryptographic FNV hash collided on the rest of the payload.
        surface: &'static str,
        file: Option<String>,
        depth: u32,
    }
    let filters = Filters {
        surface: "find_dependents",
        file: query.file.as_deref().map(str::to_lowercase),
        depth,
    };
    let bytes = serde_json::to_vec(&filters).expect("dependents filters serialise");
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
        let candidates = GctxProjector::collect_candidates(graph, query);
        GctxProjector::project(candidates, query).expect("valid query")
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
        let candidates = GctxProjector::collect_candidates(&g, &mismatched);
        let result = GctxProjector::project(candidates, &mismatched);
        assert!(result.is_err(), "a cursor is only valid for its own query");
    }

    #[test]
    fn malformed_cursor_is_rejected() {
        let g = five_symbol_graph();
        let candidates = GctxProjector::collect_candidates(&g, &SearchSymbolsQuery::default());
        let result = GctxProjector::project(
            candidates,
            &SearchSymbolsQuery {
                cursor: Some(OpaqueCursor::new("not-hex-zzzz".into())),
                ..Default::default()
            },
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
        let candidates = GctxProjector::collect_candidates(&g, &resumed);
        let page2 = GctxProjector::project(candidates, &resumed)
            .expect("a case-only filter change keeps the cursor valid");
        assert_eq!(page2.symbols.len(), 1);
        assert_eq!(page2.symbols[0].identity.file, "src/b.ts");
    }

    #[test]
    fn oversized_cursor_is_rejected() {
        let g = five_symbol_graph();
        let candidates = GctxProjector::collect_candidates(&g, &SearchSymbolsQuery::default());
        let result = GctxProjector::project(
            candidates,
            &SearchSymbolsQuery {
                cursor: Some(OpaqueCursor::new("a".repeat(MAX_CURSOR_BYTES + 1))),
                ..Default::default()
            },
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
        let candidates = GctxProjector::collect_dependents(dep, file, depth);
        GctxProjector::project_dependents(candidates, query, depth).expect("valid query")
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
        let candidates = GctxProjector::collect_dependents(&g, "src/a.ts", 2);
        let result = GctxProjector::project_dependents(candidates, &mismatched, 2);
        assert!(
            result.is_err(),
            "a cursor is only valid for the depth it was minted at",
        );
    }

    #[test]
    fn dependents_malformed_and_oversized_cursors_are_rejected() {
        let g = dep_graph(&[("src/b.ts", "src/a.ts")]);
        let candidates = GctxProjector::collect_dependents(&g, "src/a.ts", 1);
        let malformed = GctxProjector::project_dependents(
            candidates.clone(),
            &FindDependentsQuery {
                cursor: Some(OpaqueCursor::new("not-hex-zzzz".into())),
                ..Default::default()
            },
            1,
        );
        assert!(malformed.is_err(), "a malformed cursor must be rejected");

        let oversized = GctxProjector::project_dependents(
            candidates,
            &FindDependentsQuery {
                cursor: Some(OpaqueCursor::new("a".repeat(MAX_CURSOR_BYTES + 1))),
                ..Default::default()
            },
            1,
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
            ),
            &SearchSymbolsQuery {
                limit: Some(1),
                ..Default::default()
            },
        )
        .expect("more pages")
        .next_cursor
        .expect("a search cursor");

        let dep = dep_graph(&[("src/b.ts", "src/a.ts")]);
        let candidates = GctxProjector::collect_dependents(&dep, "src/a.ts", 1);
        let result = GctxProjector::project_dependents(
            candidates,
            &FindDependentsQuery {
                file: Some("src/a.ts".into()),
                cursor: Some(search_cursor),
                ..Default::default()
            },
            1,
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
}
