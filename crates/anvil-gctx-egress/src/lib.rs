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
    RedactionSummary, SearchSymbolsProjection, SearchSymbolsQuery, SymbolSummary,
};
use anvil_graph_cache::SymbolGraph;
use anvil_kernel_types::{SymbolIdentity, SymbolNode};

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
            if Path::new(file).is_absolute() {
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
    /// `ordinal`). Phase-1 is single-page: `next_cursor` is always `None` and
    /// over-limit matches are truncated (recorded in the counts-only
    /// [`RedactionSummary`]).
    #[must_use]
    pub fn project(
        mut candidates: Vec<SymbolSummary>,
        query: &SearchSymbolsQuery,
    ) -> SearchSymbolsProjection {
        candidates.sort_by(|a, b| a.identity.cmp(&b.identity));
        let matched = candidates.len();
        candidates.truncate(query.effective_limit());
        let returned = candidates.len();
        SearchSymbolsProjection {
            symbols: candidates,
            next_cursor: None,
            redaction_summary: RedactionSummary {
                matched,
                returned,
                truncated: matched > returned,
            },
        }
    }
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
        GctxProjector::project(candidates, query)
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
                "leaked",
                "/etc/passwd",
                SymbolKind::Function,
                Visibility::Public,
            ),
        ]);
        let p = run(&g, &SearchSymbolsQuery::default());
        assert_eq!(p.symbols.len(), 1);
        assert_eq!(p.symbols[0].identity.file, "src/a.ts");
        assert!(
            p.symbols.iter().all(|s| !s.identity.file.starts_with('/')),
            "no absolute path may reach the projection"
        );
    }
}
