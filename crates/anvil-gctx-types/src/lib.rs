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
    /// Opaque server-minted pagination cursor (CE-6). Echo a previous response's
    /// `next_cursor` here to fetch the next page; the projector resumes the
    /// keyset walk strictly after the cursor's position. A cursor is valid only
    /// for the filter set it was minted against (a mismatch is rejected).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cursor: Option<OpaqueCursor>,
}

impl SearchSymbolsQuery {
    /// The page size to apply: the client `limit` clamped to
    /// `1..=`[`MAX_PAGE_LIMIT`], or [`DEFAULT_PAGE_LIMIT`] when absent. The lower
    /// bound floors a `limit: 0` to 1 (it is clamped, not rejected), which would
    /// otherwise yield a contradictory empty-but-more-remain page.
    #[must_use]
    pub fn effective_limit(&self) -> usize {
        self.limit
            .unwrap_or(DEFAULT_PAGE_LIMIT)
            .clamp(1, MAX_PAGE_LIMIT) as usize
    }
}

/// An opaque, server-minted pagination cursor (CE-6).
///
/// The client MUST treat it as a meaningless token and echo it back verbatim —
/// it is never a client-supplied offset. The daemon mints it (the projector
/// encodes the keyset seek position + a fingerprint of the query filters) and is
/// the only party that interprets it.
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
    /// Whether more pages follow this one (equivalently,
    /// [`SearchSymbolsProjection::next_cursor`] is `Some`). `false` on the final
    /// page of a multi-page walk, even though `matched` still exceeds `returned`.
    pub truncated: bool,
}

/// The identity-only projection returned when the graph is readable.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchSymbolsProjection {
    /// Identity summaries, ordered deterministically by [`SymbolIdentity`].
    pub symbols: Vec<SymbolSummary>,
    /// Opaque next-page cursor when more matches remain, or `None` when this is
    /// the final page. Echo it back in [`SearchSymbolsQuery::cursor`] to continue.
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
    /// The egress surface is switched off by the operator
    /// (`ANVIL_GCTX_EGRESS=0`, re-read per call — CE-11 kill-switch). Distinct
    /// from `Unavailable` (which is an absent daemon/graph, not a deliberate
    /// disable).
    Disabled,
    /// The query was rejected before any read (CE-6 validation).
    InvalidQuery {
        /// Why the query was rejected.
        reason: String,
    },
}

impl SearchSymbolsOutcome {
    /// Classify this outcome into the PII-free [`GctxOutcome`] telemetry enum
    /// (CE-10). The match over `SearchSymbolsOutcome` is exhaustive, so a new
    /// response variant forces an explicit classification here. A readable result
    /// splits into `Hit` (≥1 symbol) and `Miss` (empty) by the page contents —
    /// the only place response counts influence the label.
    #[must_use]
    pub fn telemetry_outcome(&self) -> GctxOutcome {
        match self {
            Self::Ready(projection) if projection.symbols.is_empty() => GctxOutcome::Miss,
            Self::Ready(_) => GctxOutcome::Hit,
            Self::NotReady { .. } => GctxOutcome::Warming,
            Self::Unavailable => GctxOutcome::Unavailable,
            Self::Disabled => GctxOutcome::GraphDisabled,
            Self::InvalidQuery { .. } => GctxOutcome::InvalidQuery,
        }
    }
}

/// The PII-free classification of a GCTX response (CE-10).
///
/// Every counter, span attribute, tracing event, and notification binds to
/// **this enum** — and nothing else: no symbol names, paths, query text, or
/// per-symbol token counts may appear in a telemetry label. Only
/// response-aggregate counts (e.g. `matched`/`returned`) may accompany it on the
/// tracing pipe, as event fields within the dispatch span. `Hit`/`Miss` split a
/// readable result by whether it returned any symbol. The labels match the
/// GCTX-001 spec's outcome vocabulary (`hit`/`miss`/`warming`/`graph_disabled`/…).
/// `#[non_exhaustive]` because Phase-2 adds `redacted`/`budget_exceeded` as
/// snippet/credit surfaces land.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum GctxOutcome {
    /// Readable, ≥1 symbol returned.
    Hit,
    /// Readable, zero matches.
    Miss,
    /// Graph warming / not yet populated.
    Warming,
    /// Daemon or graph absent.
    Unavailable,
    /// Operator kill-switch engaged (`ANVIL_GCTX_EGRESS=0`). Emits the spec label
    /// `graph_disabled`.
    GraphDisabled,
    /// Query/cursor rejected before any read.
    InvalidQuery,
}

impl GctxOutcome {
    /// Stable `snake_case` label for telemetry emission (the GCTX-001 spec
    /// outcome vocabulary).
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Hit => "hit",
            Self::Miss => "miss",
            Self::Warming => "warming",
            Self::Unavailable => "unavailable",
            Self::GraphDisabled => "graph_disabled",
            Self::InvalidQuery => "invalid_query",
        }
    }
}

// ============================================================================
// GCTX-011 — `anvil_find_dependents` dependency traversal (ADR-084)
// ============================================================================

/// Conjunctive query for `anvil_find_dependents`: which file's blast radius to
/// walk, how deep, and how to page. Identity-only and graph-free — like
/// [`SearchSymbolsQuery`], it names no graph internal.
///
/// Dependents resolve at **file** granularity over the warm dependency graph
/// (`importer → imported`): the result is the set of files that import `file`,
/// transitively up to `max_depth` hops. Symbol-granular caller edges are out of
/// scope (split to GCTX-014).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FindDependentsQuery {
    /// Workspace-root-relative path whose importers to find. Required in
    /// practice; `Option` so a bare `{}` deserialises (rejected daemon-side with
    /// a structured `InvalidQuery`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    /// Traversal depth in hops: `1` is direct importers, `2` adds their
    /// importers. Clamped daemon-side to the GV2-026
    /// `MAX_REVERSE_IMPACT_DEPTH` ceiling (an over-cap value is clamped, not
    /// honoured); absent defaults to a 1-hop walk.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_depth: Option<u32>,
    /// Maximum dependents to return in this page. Clamped to [`MAX_PAGE_LIMIT`];
    /// absent uses [`DEFAULT_PAGE_LIMIT`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
    /// Opaque server-minted pagination cursor (CE-6), valid only for the filter
    /// set (`file` + `max_depth`) it was minted against.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cursor: Option<OpaqueCursor>,
}

impl FindDependentsQuery {
    /// The page size to apply: the client `limit` clamped to
    /// `1..=`[`MAX_PAGE_LIMIT`], or [`DEFAULT_PAGE_LIMIT`] when absent. Mirrors
    /// [`SearchSymbolsQuery::effective_limit`].
    #[must_use]
    pub fn effective_limit(&self) -> usize {
        self.limit
            .unwrap_or(DEFAULT_PAGE_LIMIT)
            .clamp(1, MAX_PAGE_LIMIT) as usize
    }
}

/// A single identity-only dependent summary: an importing file and how many hops
/// away it sits in the reverse-impact walk.
///
/// Carries **only** the workspace-root-relative `file` path and the traversal
/// `distance` — no [`anvil_kernel_types::ByteRange`] span, no source text, no
/// session-local id. The file path is the dependent's identity at the file-keyed
/// granularity this tool projects.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DependentSummary {
    /// The importing file, workspace-root-relative.
    pub file: String,
    /// Hop distance from the queried file: `1` for a direct importer, `2` for an
    /// importer-of-an-importer, etc. (bounded by the depth cap).
    pub distance: u32,
}

/// The identity-only projection returned when the dependency graph is readable.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FindDependentsProjection {
    /// Dependent summaries, ordered deterministically by `file`.
    pub dependents: Vec<DependentSummary>,
    /// Opaque next-page cursor when more dependents remain, or `None` on the
    /// final page. Echo it back in [`FindDependentsQuery::cursor`] to continue.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<OpaqueCursor>,
    /// Counts-only elision summary (CE-11).
    pub redaction_summary: RedactionSummary,
}

/// The status-tagged outcome of a dependents traversal.
///
/// The non-`Ready` variants are the same named degradation surface as
/// [`SearchSymbolsOutcome`] (ADR-084 CE-7): a warming/cold graph yields
/// [`NotReady`](Self::NotReady), an absent daemon/graph yields
/// [`Unavailable`](Self::Unavailable), an operator kill-switch yields
/// [`Disabled`](Self::Disabled), and a rejected query yields
/// [`InvalidQuery`](Self::InvalidQuery) — **never** a source-file fallback.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum FindDependentsOutcome {
    /// Graph readable: dependent results, possibly empty (a file with no
    /// importers is a `Ready` empty page, not a `NotReady`).
    Ready(FindDependentsProjection),
    /// Graph not yet readable (warming, or cold and not yet save-populated).
    NotReady {
        /// Human-readable, enum-stable recovery guidance.
        recovery_hint: String,
    },
    /// Daemon or graph unavailable; no fallback was attempted (CE-7).
    Unavailable,
    /// The egress surface is switched off by the operator
    /// (`ANVIL_GCTX_EGRESS=0`, re-read per call — CE-11 kill-switch).
    Disabled,
    /// The query was rejected before any read (CE-6 validation).
    InvalidQuery {
        /// Why the query was rejected.
        reason: String,
    },
}

impl FindDependentsOutcome {
    /// Classify into the shared PII-free [`GctxOutcome`] telemetry enum (CE-10).
    /// Exhaustive, so a new response variant forces an explicit classification.
    /// A readable result splits into `Hit` (≥1 dependent) and `Miss` (empty).
    #[must_use]
    pub fn telemetry_outcome(&self) -> GctxOutcome {
        match self {
            Self::Ready(projection) if projection.dependents.is_empty() => GctxOutcome::Miss,
            Self::Ready(_) => GctxOutcome::Hit,
            Self::NotReady { .. } => GctxOutcome::Warming,
            Self::Unavailable => GctxOutcome::Unavailable,
            Self::Disabled => GctxOutcome::GraphDisabled,
            Self::InvalidQuery { .. } => GctxOutcome::InvalidQuery,
        }
    }
}

// ============================================================================
// GCTX-014 — `anvil_find_callers` symbol caller traversal (ADR-084 / GCALL-007)
// ============================================================================

/// Query for `anvil_find_callers`: the symbol whose callers to find, plus the
/// traversal depth and pagination. Identity-only and graph-free.
///
/// Callers resolve at **symbol** granularity over the warm call graph
/// (`caller → callee`): the result is the set of symbols that call `target`,
/// transitively up to `max_depth` hops. The file-level dependency variant is
/// `anvil_find_dependents` (GCTX-011).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FindCallersQuery {
    /// The symbol identity whose callers to find. Required in practice; `Option`
    /// so a bare `{}` deserialises (rejected daemon-side with a structured
    /// `InvalidQuery`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<SymbolIdentity>,
    /// Traversal depth in hops: `1` is direct callers, `2` adds their callers.
    /// Clamped daemon-side to the GV2-026 `MAX_REVERSE_IMPACT_DEPTH` ceiling;
    /// absent defaults to a 1-hop walk.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_depth: Option<u32>,
    /// Maximum callers to return in this page. Clamped to [`MAX_PAGE_LIMIT`];
    /// absent uses [`DEFAULT_PAGE_LIMIT`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
    /// Opaque server-minted pagination cursor (CE-6), valid only for the filter
    /// set (`target` + `max_depth`) it was minted against.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cursor: Option<OpaqueCursor>,
}

impl FindCallersQuery {
    /// The page size to apply: the client `limit` clamped to
    /// `1..=`[`MAX_PAGE_LIMIT`], or [`DEFAULT_PAGE_LIMIT`] when absent.
    #[must_use]
    pub fn effective_limit(&self) -> usize {
        self.limit
            .unwrap_or(DEFAULT_PAGE_LIMIT)
            .clamp(1, MAX_PAGE_LIMIT) as usize
    }
}

/// A single identity-only caller summary: a calling symbol, its hop distance, and
/// whether the call reaching it is an overload fan-out (GCALL-007 CALL-1).
///
/// Carries **only** the caller's [`SymbolIdentity`], the traversal `distance`,
/// and the `heuristic` marker — no source text, no call-site arguments, no byte
/// span, no session-local id.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CallerSummary {
    /// The calling symbol (identity-only).
    pub caller: SymbolIdentity,
    /// Hop distance from the queried symbol: `1` for a direct caller, `2` for a
    /// caller-of-a-caller (bounded by the depth cap).
    pub distance: u32,
    /// `true` when the call reaching this caller is an **overload fan-out** — the
    /// static resolver could not pick one overload and attached the call to all,
    /// so this caller may be over-included (GCALL-007 CALL-1, ADR-086 §1). A
    /// consumer must not treat a `heuristic` caller as an exact call.
    pub heuristic: bool,
}

/// The identity-only projection returned when the call graph is readable.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FindCallersProjection {
    /// Caller summaries, ordered deterministically by [`SymbolIdentity`].
    pub callers: Vec<CallerSummary>,
    /// Opaque next-page cursor when more callers remain, or `None` on the final
    /// page. Echo it back in [`FindCallersQuery::cursor`] to continue.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<OpaqueCursor>,
    /// Counts-only elision summary (CE-11).
    pub redaction_summary: RedactionSummary,
    /// `true` when the caller set may be **incomplete** (GCALL-007 CALL-1): the
    /// node-budget bound the walk, or the graph is not fully resolved
    /// (`Stale`/`Bounded`). Static-analysis under-inclusion (dynamic dispatch,
    /// unresolved/default-import callees) is conveyed by the tool description, not
    /// this flag — an unresolved call leaves no edge to count.
    #[serde(default)]
    pub partial: bool,
}

/// The status-tagged outcome of a caller traversal. Same named degradation
/// surface as [`FindDependentsOutcome`] (ADR-084 CE-7).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum FindCallersOutcome {
    /// Graph readable: caller results, possibly empty (a symbol with no callers
    /// is a `Ready` empty page, not a `NotReady`).
    Ready(FindCallersProjection),
    /// Graph not yet readable (warming, or cold and not yet save-populated).
    NotReady {
        /// Human-readable, enum-stable recovery guidance.
        recovery_hint: String,
    },
    /// Daemon or graph unavailable; no fallback was attempted (CE-7).
    Unavailable,
    /// The egress surface is switched off by the operator
    /// (`ANVIL_GCTX_EGRESS=0`, re-read per call — CE-11 kill-switch).
    Disabled,
    /// The query was rejected before any read (CE-6 validation).
    InvalidQuery {
        /// Why the query was rejected.
        reason: String,
    },
}

impl FindCallersOutcome {
    /// Classify into the shared PII-free [`GctxOutcome`] telemetry enum (CE-10).
    /// A readable result splits into `Hit` (≥1 caller) and `Miss` (empty).
    #[must_use]
    pub fn telemetry_outcome(&self) -> GctxOutcome {
        match self {
            Self::Ready(projection) if projection.callers.is_empty() => GctxOutcome::Miss,
            Self::Ready(_) => GctxOutcome::Hit,
            Self::NotReady { .. } => GctxOutcome::Warming,
            Self::Unavailable => GctxOutcome::Unavailable,
            Self::Disabled => GctxOutcome::GraphDisabled,
            Self::InvalidQuery { .. } => GctxOutcome::InvalidQuery,
        }
    }
}

// ============================================================================
// GCTX-012 — `anvil_impact_of_change` blast-radius report (ADR-084)
// ============================================================================

/// Hard cap on the number of changed files accepted in one
/// `anvil_impact_of_change` call (CE-6 input bound, GCTX-001 spec "≈ ≤ 200").
/// An input above this is **rejected** with a structured `InvalidQuery` before
/// any graph read — not silently truncated — so the report is never built from a
/// partial input the caller is unaware of.
pub const MAX_CHANGED_FILES: usize = 200;

/// Query for `anvil_impact_of_change`: the set of changed file paths whose blast
/// radius to report. Identity-only and graph-free.
///
/// **Paths only.** This carries changed file *paths*, never diff content — the
/// MCP tool may derive the paths client-side from a git diff, but no diff text
/// ever reaches this type or the daemon (CE-6, GCTX-001 spec "Diff is
/// paths-only").
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImpactQuery {
    /// Workspace-root-relative changed file paths (deduplicated daemon-side).
    /// Capped at [`MAX_CHANGED_FILES`]; an over-cap input is rejected.
    pub changed_files: Vec<String>,
    /// Reverse-impact traversal depth in hops for the dependent closure. Clamped
    /// daemon-side to the GV2-026 `MAX_REVERSE_IMPACT_DEPTH` ceiling; absent
    /// defaults to a 1-hop walk.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_depth: Option<u32>,
}

/// Counts-only summary of an impact report (CE-5 safe — totals, no names/paths).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImpactSummary {
    /// Distinct changed input files the report was computed over (post-validation
    /// dedup).
    pub changed_files: usize,
    /// `affected_symbols` returned.
    pub affected_symbols: usize,
    /// `dependent_files` returned.
    pub dependent_files: usize,
    /// `known_tests` returned (a subset of `dependent_files`).
    pub known_tests: usize,
    /// Whether a result cap bound the report — **either** the affected-symbol
    /// set or the dependent-closure walk hit its node budget. The returned sets
    /// are then a deterministic, path-ordered prefix, never a silent full cutoff.
    pub truncated: bool,
}

/// The identity-only blast-radius report for a change set.
///
/// All three sections are identity-only: symbol identities and
/// workspace-relative file paths — **no source text, no byte spans, no
/// session-local ids**. Deterministic for an identical change set and graph
/// state (every section is sorted).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImpactReport {
    /// Identity summaries of the symbols **defined in** the changed files (the
    /// change surface), ordered by [`SymbolIdentity`].
    pub affected_symbols: Vec<SymbolSummary>,
    /// The depth-bounded reverse-impact closure of the changed set: the files
    /// that import them (transitively, within the depth cap), file-keyed with
    /// traversal distance, ordered by path. Excludes the changed files
    /// themselves.
    pub dependent_files: Vec<DependentSummary>,
    /// The subset of `dependent_files` whose paths match a **best-effort,
    /// heuristic** test-file convention. Marked heuristic: an assistant must not
    /// treat it as authoritative coverage — `anvil_affected_tests` (GCTX-013)
    /// owns the richer evidence-edge + coverage-gap treatment.
    pub known_tests: Vec<String>,
    /// Counts-only totals + truncation marker.
    pub summary: ImpactSummary,
}

/// The status-tagged outcome of an impact-of-change report.
///
/// Same named degradation surface as the other GCTX tools (ADR-084 CE-7); the
/// report is identity-only, so it inherits the warming/stale carve-out.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum ImpactOutcome {
    /// Graph readable: the blast-radius report (any section may be empty).
    Ready(ImpactReport),
    /// Graph not yet readable (warming, or cold and not yet save-populated).
    NotReady {
        /// Human-readable, enum-stable recovery guidance.
        recovery_hint: String,
    },
    /// Daemon or graph unavailable; no fallback was attempted (CE-7).
    Unavailable,
    /// The egress surface is switched off by the operator
    /// (`ANVIL_GCTX_EGRESS=0`, re-read per call — CE-11 kill-switch).
    Disabled,
    /// The query was rejected before any read (CE-6 validation — e.g. an empty,
    /// over-cap, or malformed `changed_files`).
    InvalidQuery {
        /// Why the query was rejected.
        reason: String,
    },
}

impl ImpactOutcome {
    /// Classify into the shared PII-free [`GctxOutcome`] telemetry enum (CE-10).
    /// Exhaustive. A readable report splits into `Hit` (any affected symbol or
    /// dependent file) and `Miss` (the change set has no resident surface and no
    /// dependents) by the report contents.
    #[must_use]
    pub fn telemetry_outcome(&self) -> GctxOutcome {
        match self {
            Self::Ready(report)
                if report.affected_symbols.is_empty() && report.dependent_files.is_empty() =>
            {
                GctxOutcome::Miss
            }
            Self::Ready(_) => GctxOutcome::Hit,
            Self::NotReady { .. } => GctxOutcome::Warming,
            Self::Unavailable => GctxOutcome::Unavailable,
            Self::Disabled => GctxOutcome::GraphDisabled,
            Self::InvalidQuery { .. } => GctxOutcome::InvalidQuery,
        }
    }
}

// ============================================================================
// GCTX-013 — `anvil_affected_tests` test-attribution report (ADR-084)
// ============================================================================

/// Query for `anvil_affected_tests`: the changed file paths whose likely tests
/// and coverage gaps to report. Identity-only and graph-free.
///
/// **Paths only.** Like [`ImpactQuery`], this carries changed file *paths*, never
/// diff content — the MCP tool may derive the paths client-side from a git diff,
/// but no diff text ever reaches this type or the daemon (CE-6).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AffectedTestsQuery {
    /// Workspace-root-relative changed file paths (deduplicated daemon-side).
    /// Capped at [`MAX_CHANGED_FILES`]; an over-cap input is rejected.
    pub changed_files: Vec<String>,
    /// Reverse-impact traversal depth in hops for test discovery and the
    /// coverage-gap check. Clamped daemon-side to the GV2-026
    /// `MAX_REVERSE_IMPACT_DEPTH` ceiling; absent defaults to a 1-hop walk.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_depth: Option<u32>,
}

/// One test file attributed to the change set, with the *why* that links it.
///
/// Identity-only: a workspace-relative test path, the workspace-relative changed
/// source files it directly imports, and the traversal distance — **no source
/// text, no byte spans, no session-local ids**.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TestEvidence {
    /// The test file (workspace-relative), recognised by the best-effort
    /// test-file heuristic.
    pub file: String,
    /// The changed source files this test **directly** depends on
    /// (`dependencies_of(test) ∩ changed_set`), ordered by path — the evidence
    /// edge connecting the test to the change. May be empty when the test reaches
    /// the change only transitively (distance > 1 through a non-changed
    /// intermediate); the `distance` still records the hop.
    pub changed_dependencies: Vec<String>,
    /// Reverse-impact traversal distance (hops) from the change set to this test.
    pub distance: u32,
}

/// Counts-only summary of an affected-tests report (CE-5 safe — totals, no
/// names/paths).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AffectedTestsSummary {
    /// Distinct changed input files the report was computed over (post-validation
    /// dedup).
    pub changed_files: usize,
    /// `tests` returned.
    pub tests: usize,
    /// Total evidence edges across all `tests` (summed
    /// `changed_dependencies.len()`).
    pub evidence_edges: usize,
    /// `coverage_gaps` returned.
    pub coverage_gaps: usize,
    /// Whether a result cap bound the report (either traversal hit the node
    /// budget); the returned sets are then a deterministic prefix, and a
    /// coverage gap may be over-reported (a test that would have covered it was
    /// beyond the bound) — never a silent full cutoff.
    pub truncated: bool,
}

/// The identity-only test-attribution report for a change set.
///
/// Both sections are identity-only (workspace-relative file paths, no source
/// text). Deterministic for an identical change set and graph state (every
/// section is sorted). Relevance is **import-derived, not execution-verified**
/// (`heuristic`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AffectedTestsReport {
    /// The test files that import a changed file within the depth bound, each
    /// with its evidence edges and traversal distance, ordered by path.
    pub tests: Vec<TestEvidence>,
    /// Changed **non-test** files with **no** test importer within the depth
    /// bound — the "you changed X, nothing tests it" warning. Ordered by path.
    pub coverage_gaps: Vec<String>,
    /// Always `true`: relevance is an import heuristic (file-keyed, not
    /// execution-verified, not symbol-level), so an assistant must not treat the
    /// report as authoritative coverage.
    pub heuristic: bool,
    /// Counts-only totals + truncation marker.
    pub summary: AffectedTestsSummary,
}

/// The status-tagged outcome of an affected-tests report.
///
/// Same named degradation surface as the other GCTX tools (ADR-084 CE-7); the
/// report is identity-only, so it inherits the warming/stale carve-out.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum AffectedTestsOutcome {
    /// Graph readable: the test-attribution report (any section may be empty).
    Ready(AffectedTestsReport),
    /// Graph not yet readable (warming, or cold and not yet save-populated).
    NotReady {
        /// Human-readable, enum-stable recovery guidance.
        recovery_hint: String,
    },
    /// Daemon or graph unavailable; no fallback was attempted (CE-7).
    Unavailable,
    /// The egress surface is switched off by the operator
    /// (`ANVIL_GCTX_EGRESS=0`, re-read per call — CE-11 kill-switch).
    Disabled,
    /// The query was rejected before any read (CE-6 validation — e.g. an empty,
    /// over-cap, or malformed `changed_files`).
    InvalidQuery {
        /// Why the query was rejected.
        reason: String,
    },
}

impl AffectedTestsOutcome {
    /// Classify into the shared PII-free [`GctxOutcome`] telemetry enum (CE-10).
    /// Exhaustive. A readable report splits into `Hit` (any attributed test or
    /// coverage gap) and `Miss` (the change set yields neither) by the report
    /// contents.
    #[must_use]
    pub fn telemetry_outcome(&self) -> GctxOutcome {
        match self {
            Self::Ready(report) if report.tests.is_empty() && report.coverage_gaps.is_empty() => {
                GctxOutcome::Miss
            }
            Self::Ready(_) => GctxOutcome::Hit,
            Self::NotReady { .. } => GctxOutcome::Warming,
            Self::Unavailable => GctxOutcome::Unavailable,
            Self::Disabled => GctxOutcome::GraphDisabled,
            Self::InvalidQuery { .. } => GctxOutcome::InvalidQuery,
        }
    }
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
        assert_no_forbidden_keys(
            &serde_json::to_value(sample_summary()).unwrap(),
            &[
                "span", "byte", "text", "body", "snippet", "trust", "content", "id",
            ],
        );
    }

    fn assert_no_forbidden_keys(value: &Value, forbidden: &[&str]) {
        match value {
            Value::Object(map) => {
                for key in map.keys() {
                    assert!(
                        !forbidden.contains(&key.as_str()),
                        "leaked forbidden key `{key}` in `{value}`"
                    );
                }
                map.values()
                    .for_each(|nested| assert_no_forbidden_keys(nested, forbidden));
            }
            Value::Array(items) => items
                .iter()
                .for_each(|nested| assert_no_forbidden_keys(nested, forbidden)),
            _ => {}
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
        assert_no_forbidden_keys(
            &serde_json::to_value(sample_projection()).unwrap(),
            &[
                "span", "byte", "text", "body", "snippet", "trust", "content", "id",
            ],
        );
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
        // `limit: 0` floors to 1 (never a contradictory empty page).
        assert_eq!(
            SearchSymbolsQuery {
                limit: Some(0),
                ..Default::default()
            }
            .effective_limit(),
            1
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
    fn outcome_disabled_is_unit_tagged() {
        let v = serde_json::to_value(SearchSymbolsOutcome::Disabled).unwrap();
        assert_eq!(v["status"], Value::String("disabled".into()));
    }

    #[test]
    fn telemetry_outcome_classifies_every_variant() {
        let cases = [
            (
                SearchSymbolsOutcome::Ready(sample_projection()),
                GctxOutcome::Hit,
                "hit",
            ),
            (
                SearchSymbolsOutcome::Ready(SearchSymbolsProjection {
                    symbols: Vec::new(),
                    next_cursor: None,
                    redaction_summary: RedactionSummary::default(),
                }),
                GctxOutcome::Miss,
                "miss",
            ),
            (
                SearchSymbolsOutcome::NotReady {
                    recovery_hint: "warming".into(),
                },
                GctxOutcome::Warming,
                "warming",
            ),
            (
                SearchSymbolsOutcome::Unavailable,
                GctxOutcome::Unavailable,
                "unavailable",
            ),
            (
                SearchSymbolsOutcome::Disabled,
                GctxOutcome::GraphDisabled,
                "graph_disabled",
            ),
            (
                SearchSymbolsOutcome::InvalidQuery { reason: "x".into() },
                GctxOutcome::InvalidQuery,
                "invalid_query",
            ),
        ];
        for (outcome, expected, label) in cases {
            assert_eq!(outcome.telemetry_outcome(), expected);
            assert_eq!(expected.as_str(), label);
        }
    }

    #[test]
    fn outcome_round_trips() {
        for outcome in [
            SearchSymbolsOutcome::Ready(sample_projection()),
            SearchSymbolsOutcome::NotReady {
                recovery_hint: "save a file".into(),
            },
            SearchSymbolsOutcome::Unavailable,
            SearchSymbolsOutcome::Disabled,
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

    // --- GCTX-011 find_dependents: CE-5 structural no-leak + shape ---

    fn sample_dependent() -> DependentSummary {
        DependentSummary {
            file: "src/importer.ts".into(),
            distance: 1,
        }
    }

    fn sample_dependents_projection() -> FindDependentsProjection {
        FindDependentsProjection {
            dependents: vec![sample_dependent()],
            next_cursor: None,
            redaction_summary: RedactionSummary {
                matched: 1,
                returned: 1,
                truncated: false,
            },
        }
    }

    /// A serialised [`DependentSummary`] exposes ONLY the file-keyed identity
    /// allowlist (`file`, `distance`). A new field that widens egress (a span, a
    /// snippet, a symbol id) fails this — the CE-5 hard gate for the new type.
    #[test]
    fn dependent_summary_serialised_keys_are_identity_only() {
        let v = serde_json::to_value(sample_dependent()).unwrap();
        let obj = v.as_object().expect("summary serialises to an object");
        let mut keys: Vec<&str> = obj.keys().map(String::as_str).collect();
        keys.sort_unstable();
        assert_eq!(keys, ["distance", "file"]);
    }

    #[test]
    fn dependent_summary_names_no_forbidden_concepts() {
        assert_no_forbidden_keys(
            &serde_json::to_value(sample_dependent()).unwrap(),
            &[
                "span", "byte", "text", "body", "snippet", "trust", "content", "id",
            ],
        );
    }

    #[test]
    fn dependents_projection_carries_no_absolute_path_values() {
        assert_no_absolute_path_values(
            &serde_json::to_value(sample_dependents_projection()).unwrap(),
        );
    }

    #[test]
    fn dependents_projection_names_no_forbidden_concepts() {
        assert_no_forbidden_keys(
            &serde_json::to_value(sample_dependents_projection()).unwrap(),
            &[
                "span", "byte", "text", "body", "snippet", "trust", "content", "id",
            ],
        );
    }

    // --- GCTX-014 find_callers: CE-5 structural no-leak (CALL-2 hard gate) ---

    fn sample_caller() -> CallerSummary {
        CallerSummary {
            caller: sample_identity(),
            distance: 1,
            heuristic: true,
        }
    }

    fn sample_callers_projection() -> FindCallersProjection {
        FindCallersProjection {
            callers: vec![sample_caller()],
            next_cursor: None,
            redaction_summary: RedactionSummary {
                matched: 1,
                returned: 1,
                truncated: false,
            },
            partial: true,
        }
    }

    /// A serialised [`CallerSummary`] exposes ONLY the identity allowlist
    /// (`caller`, `distance`, `heuristic`), and its nested `caller` only the
    /// `SymbolIdentity` keys. A field that widens egress fails this — the CE-5 /
    /// CALL-2 hard gate for the new caller type.
    #[test]
    fn caller_summary_serialised_keys_are_identity_only() {
        let v = serde_json::to_value(sample_caller()).unwrap();
        let obj = v.as_object().expect("summary serialises to an object");
        let mut keys: Vec<&str> = obj.keys().map(String::as_str).collect();
        keys.sort_unstable();
        assert_eq!(keys, ["caller", "distance", "heuristic"]);

        let id = obj["caller"].as_object().expect("caller is an object");
        let mut id_keys: Vec<&str> = id.keys().map(String::as_str).collect();
        id_keys.sort_unstable();
        assert_eq!(id_keys, ["file", "kind", "name", "ordinal"]);
    }

    #[test]
    fn callers_projection_names_no_forbidden_concepts() {
        assert_no_forbidden_keys(
            &serde_json::to_value(sample_callers_projection()).unwrap(),
            &[
                "span", "byte", "text", "body", "snippet", "trust", "content", "id",
            ],
        );
    }

    #[test]
    fn callers_projection_carries_no_absolute_path_values() {
        assert_no_absolute_path_values(&serde_json::to_value(sample_callers_projection()).unwrap());
    }

    #[test]
    fn callers_outcome_round_trips_and_classifies_telemetry() {
        // A readable non-empty result is a hit; empty is a miss.
        assert_eq!(
            FindCallersOutcome::Ready(sample_callers_projection())
                .telemetry_outcome()
                .as_str(),
            "hit"
        );
        let empty = FindCallersOutcome::Ready(FindCallersProjection {
            callers: Vec::new(),
            next_cursor: None,
            redaction_summary: RedactionSummary::default(),
            partial: false,
        });
        assert_eq!(empty.telemetry_outcome().as_str(), "miss");

        for outcome in [
            FindCallersOutcome::Ready(sample_callers_projection()),
            FindCallersOutcome::NotReady {
                recovery_hint: "warming".into(),
            },
            FindCallersOutcome::Unavailable,
            FindCallersOutcome::Disabled,
            FindCallersOutcome::InvalidQuery {
                reason: "target is required".into(),
            },
        ] {
            let json = serde_json::to_string(&outcome).unwrap();
            let back: FindCallersOutcome = serde_json::from_str(&json).unwrap();
            assert_eq!(outcome, back);
        }
    }

    #[test]
    fn callers_effective_limit_clamps_and_defaults() {
        assert_eq!(FindCallersQuery::default().effective_limit(), 50);
        assert_eq!(
            FindCallersQuery {
                limit: Some(0),
                ..Default::default()
            }
            .effective_limit(),
            1
        );
        assert_eq!(
            FindCallersQuery {
                limit: Some(100_000),
                ..Default::default()
            }
            .effective_limit(),
            MAX_PAGE_LIMIT as usize
        );
    }

    #[test]
    fn dependents_effective_limit_clamps_and_defaults() {
        assert_eq!(FindDependentsQuery::default().effective_limit(), 50);
        assert_eq!(
            FindDependentsQuery {
                limit: Some(0),
                ..Default::default()
            }
            .effective_limit(),
            1
        );
        assert_eq!(
            FindDependentsQuery {
                limit: Some(100_000),
                ..Default::default()
            }
            .effective_limit(),
            MAX_PAGE_LIMIT as usize
        );
    }

    #[test]
    fn dependents_query_round_trips_and_skips_absent_fields() {
        let q = FindDependentsQuery {
            file: Some("src/a.ts".into()),
            max_depth: Some(2),
            ..Default::default()
        };
        let json = serde_json::to_string(&q).unwrap();
        assert!(!json.contains("limit"));
        assert!(!json.contains("cursor"));
        let back: FindDependentsQuery = serde_json::from_str(&json).unwrap();
        assert_eq!(q, back);
    }

    #[test]
    fn dependents_outcome_is_status_tagged_and_round_trips() {
        let v = serde_json::to_value(FindDependentsOutcome::Ready(sample_dependents_projection()))
            .unwrap();
        assert_eq!(v["status"], Value::String("ready".into()));
        assert!(v.get("dependents").is_some());

        for outcome in [
            FindDependentsOutcome::Ready(sample_dependents_projection()),
            FindDependentsOutcome::NotReady {
                recovery_hint: "warming".into(),
            },
            FindDependentsOutcome::Unavailable,
            FindDependentsOutcome::Disabled,
            FindDependentsOutcome::InvalidQuery {
                reason: "file is required".into(),
            },
        ] {
            let json = serde_json::to_string(&outcome).unwrap();
            let back: FindDependentsOutcome = serde_json::from_str(&json).unwrap();
            assert_eq!(outcome, back);
        }
    }

    #[test]
    fn dependents_telemetry_outcome_classifies_every_variant() {
        let cases = [
            (
                FindDependentsOutcome::Ready(sample_dependents_projection()),
                "hit",
            ),
            (
                FindDependentsOutcome::Ready(FindDependentsProjection {
                    dependents: Vec::new(),
                    next_cursor: None,
                    redaction_summary: RedactionSummary::default(),
                }),
                "miss",
            ),
            (
                FindDependentsOutcome::NotReady {
                    recovery_hint: "warming".into(),
                },
                "warming",
            ),
            (FindDependentsOutcome::Unavailable, "unavailable"),
            (FindDependentsOutcome::Disabled, "graph_disabled"),
            (
                FindDependentsOutcome::InvalidQuery { reason: "x".into() },
                "invalid_query",
            ),
        ];
        for (outcome, label) in cases {
            assert_eq!(outcome.telemetry_outcome().as_str(), label);
        }
    }

    // --- GCTX-012 impact_of_change: CE-5 structural no-leak + shape ---

    fn sample_impact_report() -> ImpactReport {
        ImpactReport {
            affected_symbols: vec![sample_summary()],
            dependent_files: vec![DependentSummary {
                file: "src/importer.ts".into(),
                distance: 1,
            }],
            known_tests: vec!["src/importer.test.ts".into()],
            summary: ImpactSummary {
                changed_files: 1,
                affected_symbols: 1,
                dependent_files: 1,
                known_tests: 1,
                truncated: false,
            },
        }
    }

    /// CE-5 hard gate: a serialised [`ImpactReport`] exposes ONLY the
    /// identity-allowlisted section keys, and its nested summaries only their
    /// own allowlists. A field that widens egress fails this and the build.
    #[test]
    fn impact_report_serialised_keys_are_identity_only() {
        let v = serde_json::to_value(sample_impact_report()).unwrap();
        let obj = v.as_object().expect("report serialises to an object");
        let mut keys: Vec<&str> = obj.keys().map(String::as_str).collect();
        keys.sort_unstable();
        assert_eq!(
            keys,
            [
                "affected_symbols",
                "dependent_files",
                "known_tests",
                "summary"
            ]
        );

        let summary = obj["summary"].as_object().expect("summary is an object");
        let mut sk: Vec<&str> = summary.keys().map(String::as_str).collect();
        sk.sort_unstable();
        assert_eq!(
            sk,
            [
                "affected_symbols",
                "changed_files",
                "dependent_files",
                "known_tests",
                "truncated"
            ]
        );
    }

    #[test]
    fn impact_report_carries_no_absolute_paths_or_forbidden_concepts() {
        let v = serde_json::to_value(sample_impact_report()).unwrap();
        assert_no_absolute_path_values(&v);
        assert_no_forbidden_keys(
            &v,
            &[
                "span", "byte", "text", "body", "snippet", "trust", "content", "id",
            ],
        );
    }

    #[test]
    fn impact_query_is_paths_only_and_round_trips() {
        let q = ImpactQuery {
            changed_files: vec!["src/a.ts".into(), "src/b.ts".into()],
            max_depth: Some(2),
        };
        let json = serde_json::to_string(&q).unwrap();
        // No diff-content field exists on the type — paths only (CE-6).
        assert!(!json.contains("diff"));
        assert!(!json.contains("content"));
        let back: ImpactQuery = serde_json::from_str(&json).unwrap();
        assert_eq!(q, back);
        // `max_depth` is omitted when absent.
        let bare = ImpactQuery {
            changed_files: vec!["src/a.ts".into()],
            ..Default::default()
        };
        assert!(!serde_json::to_string(&bare).unwrap().contains("max_depth"));
    }

    #[test]
    fn impact_outcome_is_status_tagged_and_round_trips() {
        let v = serde_json::to_value(ImpactOutcome::Ready(sample_impact_report())).unwrap();
        assert_eq!(v["status"], Value::String("ready".into()));
        assert!(v.get("affected_symbols").is_some());

        for outcome in [
            ImpactOutcome::Ready(sample_impact_report()),
            ImpactOutcome::NotReady {
                recovery_hint: "warming".into(),
            },
            ImpactOutcome::Unavailable,
            ImpactOutcome::Disabled,
            ImpactOutcome::InvalidQuery {
                reason: "too many files".into(),
            },
        ] {
            let json = serde_json::to_string(&outcome).unwrap();
            let back: ImpactOutcome = serde_json::from_str(&json).unwrap();
            assert_eq!(outcome, back);
        }
    }

    #[test]
    fn impact_telemetry_outcome_classifies_every_variant() {
        // A report with content → hit.
        assert_eq!(
            ImpactOutcome::Ready(sample_impact_report())
                .telemetry_outcome()
                .as_str(),
            "hit"
        );
        // An empty report (no surface, no dependents) → miss.
        let empty = ImpactOutcome::Ready(ImpactReport {
            affected_symbols: Vec::new(),
            dependent_files: Vec::new(),
            known_tests: Vec::new(),
            summary: ImpactSummary::default(),
        });
        assert_eq!(empty.telemetry_outcome().as_str(), "miss");
        for (outcome, label) in [
            (
                ImpactOutcome::NotReady {
                    recovery_hint: "x".into(),
                },
                "warming",
            ),
            (ImpactOutcome::Unavailable, "unavailable"),
            (ImpactOutcome::Disabled, "graph_disabled"),
            (
                ImpactOutcome::InvalidQuery { reason: "x".into() },
                "invalid_query",
            ),
        ] {
            assert_eq!(outcome.telemetry_outcome().as_str(), label);
        }
    }

    // --- GCTX-013 affected_tests: CE-5 structural no-leak + shape ---

    fn sample_affected_tests_report() -> AffectedTestsReport {
        AffectedTestsReport {
            tests: vec![TestEvidence {
                file: "src/handler.test.ts".into(),
                changed_dependencies: vec!["src/handler.ts".into()],
                distance: 1,
            }],
            coverage_gaps: vec!["src/untested.ts".into()],
            heuristic: true,
            summary: AffectedTestsSummary {
                changed_files: 2,
                tests: 1,
                evidence_edges: 1,
                coverage_gaps: 1,
                truncated: false,
            },
        }
    }

    /// CE-5 hard gate: a serialised [`AffectedTestsReport`] exposes ONLY the
    /// identity-allowlisted section keys, and its nested summary / evidence only
    /// their own allowlists. A field that widens egress fails this and the build.
    #[test]
    fn affected_tests_report_serialised_keys_are_identity_only() {
        let v = serde_json::to_value(sample_affected_tests_report()).unwrap();
        let obj = v.as_object().expect("report serialises to an object");
        let mut keys: Vec<&str> = obj.keys().map(String::as_str).collect();
        keys.sort_unstable();
        assert_eq!(keys, ["coverage_gaps", "heuristic", "summary", "tests"]);

        let test = obj["tests"][0].as_object().expect("test is an object");
        let mut tk: Vec<&str> = test.keys().map(String::as_str).collect();
        tk.sort_unstable();
        assert_eq!(tk, ["changed_dependencies", "distance", "file"]);

        let summary = obj["summary"].as_object().expect("summary is an object");
        let mut sk: Vec<&str> = summary.keys().map(String::as_str).collect();
        sk.sort_unstable();
        assert_eq!(
            sk,
            [
                "changed_files",
                "coverage_gaps",
                "evidence_edges",
                "tests",
                "truncated"
            ]
        );
    }

    #[test]
    fn affected_tests_report_carries_no_absolute_paths_or_forbidden_concepts() {
        let v = serde_json::to_value(sample_affected_tests_report()).unwrap();
        assert_no_absolute_path_values(&v);
        assert_no_forbidden_keys(
            &v,
            &[
                "span", "byte", "text", "body", "snippet", "trust", "content", "id",
            ],
        );
    }

    #[test]
    fn affected_tests_query_is_paths_only_and_round_trips() {
        let q = AffectedTestsQuery {
            changed_files: vec!["src/a.ts".into(), "src/b.ts".into()],
            max_depth: Some(2),
        };
        let json = serde_json::to_string(&q).unwrap();
        // No diff-content field exists on the type — paths only (CE-6).
        assert!(!json.contains("diff"));
        assert!(!json.contains("content"));
        let back: AffectedTestsQuery = serde_json::from_str(&json).unwrap();
        assert_eq!(q, back);
        // `max_depth` is omitted when absent.
        let bare = AffectedTestsQuery {
            changed_files: vec!["src/a.ts".into()],
            ..Default::default()
        };
        assert!(!serde_json::to_string(&bare).unwrap().contains("max_depth"));
    }

    #[test]
    fn affected_tests_outcome_is_status_tagged_and_round_trips() {
        let v = serde_json::to_value(AffectedTestsOutcome::Ready(sample_affected_tests_report()))
            .unwrap();
        assert_eq!(v["status"], Value::String("ready".into()));
        assert!(v.get("tests").is_some());

        for outcome in [
            AffectedTestsOutcome::Ready(sample_affected_tests_report()),
            AffectedTestsOutcome::NotReady {
                recovery_hint: "warming".into(),
            },
            AffectedTestsOutcome::Unavailable,
            AffectedTestsOutcome::Disabled,
            AffectedTestsOutcome::InvalidQuery {
                reason: "too many files".into(),
            },
        ] {
            let json = serde_json::to_string(&outcome).unwrap();
            let back: AffectedTestsOutcome = serde_json::from_str(&json).unwrap();
            assert_eq!(outcome, back);
        }
    }

    #[test]
    fn affected_tests_telemetry_outcome_classifies_every_variant() {
        // A report with content → hit.
        assert_eq!(
            AffectedTestsOutcome::Ready(sample_affected_tests_report())
                .telemetry_outcome()
                .as_str(),
            "hit"
        );
        // An empty report (no tests, no gaps) → miss.
        let empty = AffectedTestsOutcome::Ready(AffectedTestsReport {
            tests: Vec::new(),
            coverage_gaps: Vec::new(),
            heuristic: true,
            summary: AffectedTestsSummary::default(),
        });
        assert_eq!(empty.telemetry_outcome().as_str(), "miss");
        for (outcome, label) in [
            (
                AffectedTestsOutcome::NotReady {
                    recovery_hint: "x".into(),
                },
                "warming",
            ),
            (AffectedTestsOutcome::Unavailable, "unavailable"),
            (AffectedTestsOutcome::Disabled, "graph_disabled"),
            (
                AffectedTestsOutcome::InvalidQuery { reason: "x".into() },
                "invalid_query",
            ),
        ] {
            assert_eq!(outcome.telemetry_outcome().as_str(), label);
        }
    }
}
