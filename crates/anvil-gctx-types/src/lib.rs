//! Sealed, graph-free egress value types for Graph Context Delivery (GCTX).
//!
//! ADR-084 crate split: this leaf crate holds the **sealed egress DTOs** the
//! GCTX projection produces, plus the structural no-leak test that is the CE-5
//! hard gate. It depends on [`anvil_kernel_types`] (for the stable
//! [`SymbolIdentity`]) and `serde` **only** — never on `anvil-graph-cache`. The
//! wire crate (`anvil-intercept-proto`) and the MCP consumer (`anvil-cli`) link
//! *this* crate, so they are **structurally incapable** of naming a graph
//! internal (`SymbolNode`, `GraphDelta`, the session-local `SymbolNode.id`): the
//! no-leak guarantee is enforced by the Cargo dependency graph, not by
//! convention. The serialised-shape tests below are defence in depth.
//!
//! Phase 1 (GCTX-010) is **identity-only**: a [`SymbolSummary`] carries the
//! stable identity (workspace-root-relative path, structural kind/name, overload
//! ordinal) plus visibility, and nothing else — no source text, no byte span, no
//! trust posture. Source-text egress (snippets) is a Phase-2 concern gated behind
//! the `gctx.egress` flag and is intentionally unrepresentable here.

use anvil_kernel_types::{SymbolIdentity, SymbolKind, Visibility};
use serde::{Deserialize, Serialize};

/// Default page size when a query omits `limit`.
pub const DEFAULT_PAGE_LIMIT: u32 = 50;

/// Hard upper bound on a single page (CE-6 volume bound). A client-supplied
/// `limit` above this is clamped, never honoured.
pub const MAX_PAGE_LIMIT: u32 = 200;

/// A single identity-only symbol summary — the CE-4 field allowlist.
///
/// Carries the stable [`SymbolIdentity`] (workspace-root-relative `file`,
/// structural `kind` + `name`, overload `ordinal`) and `visibility`. It carries
/// **no source text, no [`anvil_kernel_types::ByteRange`] span, no trust level,
/// and no session-local id** — the extra fields a `SymbolNode` would add are
/// structurally absent. `identity` is a nested object (no `serde(flatten)`, per
/// CE-5).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SymbolSummary {
    /// Stable cross-restart identity (GV2-002).
    pub identity: SymbolIdentity,
    /// `Public` or `Internal`.
    pub visibility: Visibility,
}

/// Conjunctive filters for `anvil_search_symbols`. An absent filter matches
/// everything. Identity-only: there is deliberately no source-text predicate.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchSymbolsQuery {
    /// Case-insensitive substring match on the symbol name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Exact structural kind.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<SymbolKind>,
    /// Case-insensitive substring match on the workspace-root-relative path.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    /// Language token derived from the file extension (`typescript`,
    /// `javascript`, `rust`, …), matched case-insensitively.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    /// Exact visibility.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub visibility: Option<Visibility>,
    /// Maximum summaries to return in this page. Clamped to [`MAX_PAGE_LIMIT`];
    /// absent uses [`DEFAULT_PAGE_LIMIT`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
    /// Opaque server-minted pagination cursor (CE-6). **Reserved**: Phase-1 is
    /// single-page and always returns `next_cursor: None`, so this is currently
    /// ignored. Carried now so Phase-2 pagination needs no wire change.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cursor: Option<OpaqueCursor>,
}

impl SearchSymbolsQuery {
    /// The clamped page size to apply: the client `limit` capped at
    /// [`MAX_PAGE_LIMIT`], or [`DEFAULT_PAGE_LIMIT`] when absent.
    #[must_use]
    pub fn effective_limit(&self) -> usize {
        self.limit.unwrap_or(DEFAULT_PAGE_LIMIT).min(MAX_PAGE_LIMIT) as usize
    }
}

/// An opaque, server-minted pagination cursor (CE-6).
///
/// The client MUST treat it as a meaningless token and echo it back verbatim.
/// **Reserved**: Phase-1 never mints one; cursor pagination is Phase-2.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct OpaqueCursor(String);

impl OpaqueCursor {
    /// Wrap a server-minted token.
    #[must_use]
    pub fn new(token: String) -> Self {
        Self(token)
    }

    /// The wrapped token.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Counts-only summary of what the projection elided (CE-11).
///
/// Carries **no** names, paths, or content — only totals — so it is itself safe
/// to egress.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RedactionSummary {
    /// Symbols that matched the query before the page limit was applied.
    pub matched: usize,
    /// Summaries actually returned in this page.
    pub returned: usize,
    /// Whether the page was truncated (more matched than returned).
    pub truncated: bool,
}

/// The identity-only projection returned when the graph is readable.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchSymbolsProjection {
    /// Identity summaries, ordered deterministically by [`SymbolIdentity`].
    pub symbols: Vec<SymbolSummary>,
    /// Opaque next-page cursor, or `None` when the page is complete. Phase-1 is
    /// single-page, so always `None`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<OpaqueCursor>,
    /// Counts-only elision summary (CE-11).
    pub redaction_summary: RedactionSummary,
}

/// The status-tagged outcome of a search.
///
/// The non-`Ready` variants are the named degradation surface (ADR-084 CE-7):
/// a warming or cold graph yields [`NotReady`](Self::NotReady) with a recovery
/// hint, an absent daemon/graph yields [`Unavailable`](Self::Unavailable), and a
/// rejected query yields [`InvalidQuery`](Self::InvalidQuery) — **never** a
/// source-file fallback. The [`crate::SearchSymbolsProjection`] travels only in
/// the `Ready` arm.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum SearchSymbolsOutcome {
    /// Graph readable (assurance `Clean` or `Stale`): identity results, possibly
    /// empty.
    Ready(SearchSymbolsProjection),
    /// Graph not yet readable (warming, or cold and not yet save-populated).
    /// The hint tells the assistant how to make progress.
    NotReady {
        /// Human-readable, enum-stable recovery guidance.
        recovery_hint: String,
    },
    /// Daemon or graph unavailable; no fallback was attempted (CE-7).
    Unavailable,
    /// The query was rejected before any read (CE-6 validation).
    InvalidQuery {
        /// Why the query was rejected.
        reason: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn sample_identity() -> SymbolIdentity {
        SymbolIdentity {
            file: "src/handler.ts".into(),
            kind: SymbolKind::Function,
            name: "handleRequest".into(),
            ordinal: 0,
        }
    }

    fn sample_summary() -> SymbolSummary {
        SymbolSummary {
            identity: sample_identity(),
            visibility: Visibility::Public,
        }
    }

    fn sample_projection() -> SearchSymbolsProjection {
        SearchSymbolsProjection {
            symbols: vec![sample_summary()],
            next_cursor: None,
            redaction_summary: RedactionSummary {
                matched: 1,
                returned: 1,
                truncated: false,
            },
        }
    }

    // --- CE-5 structural no-leak (the hard gate) ---

    /// A serialised [`SymbolSummary`] exposes ONLY the identity-allowlisted
    /// fields. If a field is ever added that widens egress (a span, a snippet, a
    /// trust level, the session-local id), this fails and the build with it.
    #[test]
    fn symbol_summary_serialised_keys_are_identity_only() {
        let v = serde_json::to_value(sample_summary()).unwrap();
        let obj = v.as_object().expect("summary serialises to an object");
        let mut keys: Vec<&str> = obj.keys().map(String::as_str).collect();
        keys.sort_unstable();
        assert_eq!(keys, ["identity", "visibility"]);

        let id = obj["identity"].as_object().expect("identity is an object");
        let mut id_keys: Vec<&str> = id.keys().map(String::as_str).collect();
        id_keys.sort_unstable();
        assert_eq!(id_keys, ["file", "kind", "name", "ordinal"]);
    }

    /// Defence in depth: the serialised form names none of the forbidden egress
    /// concepts. `id` would be the session-local `SymbolNode` counter;
    /// `span`/`byte`/`text`/`body`/`snippet`/`content` would be code content;
    /// `trust` would be privilege posture.
    #[test]
    fn symbol_summary_names_no_forbidden_concepts() {
        let s = serde_json::to_string(&sample_summary()).unwrap();
        for forbidden in [
            "span", "byte", "\"text\"", "\"body\"", "snippet", "trust", "content", "\"id\"",
        ] {
            assert!(!s.contains(forbidden), "leaked `{forbidden}` in `{s}`");
        }
    }

    /// Walk a serialised value and assert no string *value* is an absolute path
    /// (Unix `/…` or a Windows `C:\…` drive). The exact-key tests above are the
    /// primary CE-5 gate (they break on any new field); this is a value-level
    /// backstop against an identity that ever carried an absolute path.
    fn assert_no_absolute_path_values(value: &Value) {
        match value {
            Value::String(s) => {
                assert!(!s.starts_with('/'), "absolute path value leaked: {s}");
                let bytes = s.as_bytes();
                let windows_drive = bytes.len() >= 3
                    && bytes[0].is_ascii_alphabetic()
                    && bytes[1] == b':'
                    && (bytes[2] == b'\\' || bytes[2] == b'/');
                assert!(!windows_drive, "absolute Windows path value leaked: {s}");
            }
            Value::Array(items) => items.iter().for_each(assert_no_absolute_path_values),
            Value::Object(map) => map.values().for_each(assert_no_absolute_path_values),
            _ => {}
        }
    }

    #[test]
    fn projection_carries_no_absolute_path_values() {
        assert_no_absolute_path_values(&serde_json::to_value(sample_projection()).unwrap());
    }

    /// The full projection wire shape is also closed to the forbidden concepts.
    #[test]
    fn projection_names_no_forbidden_concepts() {
        let s = serde_json::to_string(&sample_projection()).unwrap();
        for forbidden in [
            "span",
            "byte",
            "\"text\"",
            "\"body\"",
            "snippet",
            "trust",
            "\"content\"",
            "\"id\"",
        ] {
            assert!(!s.contains(forbidden), "leaked `{forbidden}` in `{s}`");
        }
    }

    // --- round-trips ---

    #[test]
    fn summary_round_trips() {
        let s = sample_summary();
        let back: SymbolSummary =
            serde_json::from_str(&serde_json::to_string(&s).unwrap()).unwrap();
        assert_eq!(s, back);
    }

    #[test]
    fn query_round_trips_and_skips_absent_filters() {
        let q = SearchSymbolsQuery {
            name: Some("handle".into()),
            kind: Some(SymbolKind::Function),
            ..Default::default()
        };
        let json = serde_json::to_string(&q).unwrap();
        // Absent filters are omitted, not serialised as null.
        assert!(!json.contains("visibility"));
        assert!(!json.contains("cursor"));
        let back: SearchSymbolsQuery = serde_json::from_str(&json).unwrap();
        assert_eq!(q, back);
    }

    #[test]
    fn effective_limit_clamps_and_defaults() {
        assert_eq!(SearchSymbolsQuery::default().effective_limit(), 50);
        assert_eq!(
            SearchSymbolsQuery {
                limit: Some(10),
                ..Default::default()
            }
            .effective_limit(),
            10
        );
        assert_eq!(
            SearchSymbolsQuery {
                limit: Some(100_000),
                ..Default::default()
            }
            .effective_limit(),
            MAX_PAGE_LIMIT as usize
        );
    }

    #[test]
    fn outcome_ready_is_status_tagged() {
        let v = serde_json::to_value(SearchSymbolsOutcome::Ready(sample_projection())).unwrap();
        assert_eq!(v["status"], Value::String("ready".into()));
        // Internally tagged: the projection fields sit beside the tag.
        assert!(v.get("symbols").is_some());
        assert!(v.get("redaction_summary").is_some());
    }

    #[test]
    fn outcome_not_ready_carries_hint() {
        let v = serde_json::to_value(SearchSymbolsOutcome::NotReady {
            recovery_hint: "warming".into(),
        })
        .unwrap();
        assert_eq!(v["status"], Value::String("not_ready".into()));
        assert_eq!(v["recovery_hint"], Value::String("warming".into()));
    }

    #[test]
    fn outcome_unavailable_is_unit_tagged() {
        let v = serde_json::to_value(SearchSymbolsOutcome::Unavailable).unwrap();
        assert_eq!(v["status"], Value::String("unavailable".into()));
    }

    #[test]
    fn outcome_round_trips() {
        for outcome in [
            SearchSymbolsOutcome::Ready(sample_projection()),
            SearchSymbolsOutcome::NotReady {
                recovery_hint: "save a file".into(),
            },
            SearchSymbolsOutcome::Unavailable,
            SearchSymbolsOutcome::InvalidQuery {
                reason: "bad file filter".into(),
            },
        ] {
            let json = serde_json::to_string(&outcome).unwrap();
            let back: SearchSymbolsOutcome = serde_json::from_str(&json).unwrap();
            assert_eq!(outcome, back);
        }
    }

    #[test]
    fn opaque_cursor_is_transparent() {
        let c = OpaqueCursor::new("page-2".into());
        assert_eq!(serde_json::to_string(&c).unwrap(), "\"page-2\"");
        assert_eq!(c.as_str(), "page-2");
    }
}
