# Graph Context Delivery

| ID   | Owner | Status |
| ---- | ----- | ------ |
| GCTX | —     | Draft  |

**Last reviewed:** 2026-04-26

> **Audit note (2026-04-26):** Module premise is sound post-migration — it
> builds on the Rust `anvil-kernel` graph and proposes a new `anvil-graph-store`
> crate. References to `packages/mcp-server/src/tools/` remain valid (the TS
> MCP server package still exists). GCTX-000 dual-config references
> (`packages/edda-stack/src/config.ts` and `packages/anvil/core/src/config/`)
> are now superseded by the unified config direction in the UCFG module —
> when scheduling, align with `crates/anvil-config/` (per UCFG) instead of
> the dual TS schemas.

## Purpose

Expose Anvil's kernel symbol graph to AI coding assistants via MCP, so they
consume precise, blast-radius-scoped context instead of re-reading whole files
on every task. Adds persistent graph storage, transitive impact analysis,
multi-language parsers, and graph-aware MCP tools on top of the existing
`anvil-kernel` graph pipeline.

**Why:** An external project,
[tirth8205/code-review-graph](https://github.com/tirth8205/code-review-graph),
demonstrates that a Tree-sitter-backed code graph served to AI assistants via
MCP can deliver an 8.2x average token reduction on real repos (up to 49x on
monorepos). Anvil already has the hard parts of that pipeline — Tree-sitter
parsing, an incremental `SymbolGraph`, a file watcher, and an MCP server — but
they are currently wired only for architecture enforcement, not context
delivery. The graph is in-memory, queries are single-hop, only TS/JS parsers are
registered, and no MCP tool surfaces the graph. This module closes that gap so
Anvil's kernel doubles as an AI-context engine, without duplicating the
tirth8205 project or losing any existing enforcement capability.

**Non-goal:** Replace code-review-graph. We are leveraging Anvil's existing
kernel and keeping the architecture-enforcement surface intact.

## In Scope

- **User toggle** — `graph.context` boolean in `.anvilrc` / `.anvil/config.json`
  (Zod schema, default `true`); controls only the **new context delivery
  layer** (persistence, impact analysis, MCP graph tools, context slicing).
  The core kernel symbol graph used for architecture enforcement is
  **always on** and unaffected by this toggle. Granular sub-toggles for
  persistence (`graph.persist: true`) and MCP exposure (`graph.mcp: true`)
  so users can run the context layer in-memory-only or keep it private
  from assistants. When `graph.context: false`, persistence is skipped,
  impact API is inactive, and MCP graph tools return
  `{ enabled: false }` gracefully instead of erroring.
- SQLite persistence layer for `SymbolGraph` and `DependencyGraph` with
  incremental writes on every `GraphDelta`
- Transitive impact analysis API on the existing `petgraph` structure
  (transitive callers, transitive dependents, blast-radius sets)
- Test-coverage edges derived from test-file import analysis
- Multi-language parser registration (Python, Go, Rust) via the existing
  Tree-sitter parser abstraction; coordinated with the `lang-*` draft modules
- New graph-exposure MCP tools: symbol search, caller/dependent traversal,
  impact-of-change, symbol context slicing, affected-tests lookup
- Context slicer that produces minimal code snippets for a change set within a
  token budget, with deterministic output ordering
- Token-reduction benchmarks against a naive "read all changed files"
  baseline, to validate the approach end-to-end
- User guide for wiring the MCP server into Claude Code, Cursor, Continue

## Out of Scope

- Community detection / Leiden clustering (code-review-graph has it — defer
  until a concrete Anvil use case emerges)
- Semantic (embedding-based) search — defer until after lexical search lands
- Language support beyond Python, Go, Rust in this module (more languages go
  through the existing `lang-*` modules)
- Replacing the existing architecture-enforcement MCP tools
- Multi-repo registry / cross-repo queries
- Visualisation surfaces (dashboard views, D3 graphs) — separate `DASH*`
  modules own that

## Interfaces

**Depends on:**

- `anvil-kernel` — `SymbolGraph`, `DependencyGraph`, parser, watcher, embedded
  mode
- `anvil-kernel-types` — `SymbolNode`, `SymbolEdge`, `GraphDelta`
- `packages/mcp-server` — existing tool and resource registration surface
- `rkyv` (zero-copy serialisation) — primary persistence backend
- `rusqlite` (optional, behind `features = ["sqlite"]`) — alternative backend
- Tree-sitter language grammars: `tree-sitter-python`, `tree-sitter-go`,
  `tree-sitter-rust`
- `tiktoken-rs` or equivalent for token estimation
- APS modules: `lang-python`, `lang-go`, `lang-rust` (parser alignment only —
  this module does not duplicate their anti-pattern or suppression work)

**Exposes:**

- `anvil-graph-store` crate — SQLite-backed persistence for the kernel graph
- `anvil-kernel::graph::impact` module — transitive traversal API
- New MCP tools on the existing server:
  - `anvil_find_callers`
  - `anvil_find_dependents`
  - `anvil_impact_of_change`
  - `anvil_search_symbols`
  - `anvil_symbol_context`
  - `anvil_affected_tests`
- New MCP resource: `graph://symbols`, `graph://edges`, `graph://stats`
- CLI: `anvil graph export|query|stats` for offline inspection

## Constraints

- UK English spelling in all plan text and user-facing docs
- Persistent store must survive kernel restarts and converge to the same
  state as a full rebuild (property-based test)
- Incremental write path must not regress kernel incremental-update latency
  beyond the 100ms budget set in the kernel spec
- Context-slicing output must be deterministic given the same graph state and
  query inputs (reproducibility for caching and evaluation)
- MCP graph tools must degrade gracefully when the graph is still warming up
  (return partial results with a `stale: true` flag rather than blocking)
- No breaking changes to the existing MCP tool schema — additions only
- Token-reduction benchmark harness must be reproducible and checked into the
  repo so claims can be re-validated

## Prerequisites

- KERN-020..023 (symbol graph, dependency graph, trust, incremental updates) —
  all complete
- BENCH harness available for regression measurement (BENCH module)
- `lang-python` draft at minimum Ready (parser surface alignment)

## Ready Checklist

Change status to **Ready** when:

- [x] Storage backend decided (rkyv default, SQLite opt-in via feature flag)
- [x] Graph file location decided (per-repo `.anvil/graph.bin`)
- [x] MCP server approach decided (reuse existing TS server)
- [x] Test-coverage strategy decided (import heuristic v1, lcov deferred)
- [x] Default toggle decided (`graph.context: true`)
- [x] Backend selection mechanism decided (feature flag, not config)
- [ ] Persistence ADR drafted and reviewed (GCTX-001)
- [ ] Impact-analysis algorithm choice decided (BFS vs bidirectional, cycle
      handling)
- [ ] Token-budget strategy agreed with MCP server owner
- [ ] Tree-sitter grammar licensing reviewed for Python/Go/Rust
- [ ] User-guide outline agreed with docs team

---

## Phase 0 — Configuration & Feature Gate

> Land the toggle before anything else so every subsequent phase respects it
> from day one.

### GCTX-000: `graph` config section with `context`, `persist`, `mcp` toggles

- **Status:** Draft
- **Intent:** Let users toggle the context delivery layer independently
  from core architecture enforcement. The kernel's symbol graph used for
  boundary checks, import rules, and cycle detection is **always on** —
  it is not affected by these toggles. `graph.context` controls only the
  new features in this module: persistence, transitive impact analysis,
  MCP graph tools, and context slicing. Sub-toggles `graph.persist` and
  `graph.mcp` give finer control. Default: all on.
- **Expected Outcome:** Zod schema in `packages/edda-stack/src/config.ts`
  (or `packages/anvil/core/src/config/`); Rust mirror in
  `crates/anvil-kernel/src/config.rs`; `.anvilrc` example:
  ```yaml
  graph:
    context: true    # false = disables context delivery layer only
    persist: true    # false = in-memory only, rebuilt each run
    mcp: true        # false = graph tools hidden from MCP clients
  ```
  When `graph.context: false`, the kernel still builds its graph for
  enforcement but skips persistence, impact API, and MCP graph tools.
  MCP tools return `{ enabled: false }` gracefully when toggled off
  rather than erroring.
- **Validation:** Unit test: parse config with each combination of
  true/false; integration test: kernel startup with `context: false`
  still runs architecture enforcement, produces no persistence file,
  and does not register MCP graph tools
- **Files:** `packages/anvil/core/src/config/graph.ts`,
  `packages/anvil/runtime/src/gate/gate-config.ts`,
  `crates/anvil-kernel/src/config.rs`,
  `packages/mcp-server/src/tools/index.ts`
- **Confidence:** high
- **Priority:** Critical
- **Dependencies:** None

---

## Phase 1 — Persistent Graph Store

> Move the `SymbolGraph` and `DependencyGraph` from process-memory-only to a
> persistent store that survives restarts and supports incremental writes.
> Default storage backend is `petgraph` + `rkyv` serialisation (zero-copy,
> pure Rust, no C dependencies). SQLite is a supported alternative behind a
> build feature flag for users who want SQL-queryable graph data.

### GCTX-001: ADR — graph persistence strategy

- **Status:** Draft
- **Intent:** Record the storage decision and rationale. Primary backend:
  `petgraph` + `rkyv` zero-copy serialisation with atomic file rename for
  crash safety. The graph is a derivable cache (re-indexable from source),
  so ACID transactions are not required. Alternative SQLite backend behind
  `--features sqlite` for users who need SQL-queryable graph data or
  multi-process readers.
  **Candidates evaluated:** SQLite (rusqlite), RocksDB, sled, redb,
  SurrealDB embedded, Cozo, petgraph+rkyv, DuckDB, LMDB.
  `petgraph+rkyv` chosen because: (a) graph is already `petgraph` in
  memory so serialisation is structural identity, (b) zero-copy
  deserialisation means cold-start loads a 50k-node graph in <1ms,
  (c) pure Rust with no C dependencies, (d) traversal stays in-memory at
  microsecond latency vs millisecond DB queries.
- **Expected Outcome:** ADR committed in `plans/decisions/`; comparison
  table; benchmark targets
- **Validation:** ADR reviewed by council-reviewer and kernel-maintainer
- **Files:** `plans/decisions/NNN-graph-persistence.md`
- **Confidence:** high
- **Priority:** Critical
- **Dependencies:** None

---

### GCTX-002: `anvil-graph-store` crate scaffold

- **Status:** Draft
- **Intent:** Introduce a new crate that owns persistence, decoupled from
  `anvil-kernel` so the kernel's in-memory fast path stays lean. Crate
  exposes a `GraphStore` trait with two backends: `RkyvStore` (default,
  pure Rust) and `SqliteStore` (behind `features = ["sqlite"]`)
- **Expected Outcome:** `crates/anvil-graph-store/` crate builds, exposes
  `GraphStore::open(path)`, `save(&SymbolGraph)`, `load() -> SymbolGraph`,
  with an `InMemoryStore` for tests; `RkyvStore` uses atomic rename for
  crash safety
- **Validation:** `cargo test -p eddacraft-anvil-graph-store`
- **Files:** `crates/anvil-graph-store/src/lib.rs`,
  `crates/anvil-graph-store/src/rkyv_store.rs`,
  `crates/anvil-graph-store/Cargo.toml`
- **Confidence:** high
- **Priority:** Critical
- **Dependencies:** GCTX-001

---

### GCTX-003: `RkyvStore` full-graph serialisation

- **Status:** Draft
- **Intent:** Persist a complete `SymbolGraph` + `DependencyGraph` to disk
  using `rkyv` zero-copy serialisation so cold-start can skip re-parsing on
  unchanged repos. Atomic rename (`write tmp → rename`) for crash safety.
- **Expected Outcome:** Round-trip a 5k-file fixture graph; loaded graph is
  observationally identical to the source via `graph_eq()` helper; cold
  load of a 50k-node graph in <1ms
- **Validation:** Property-based test `snapshot_roundtrip` on generated
  graphs up to 10k nodes; Criterion bench for load time at 50k nodes
- **Files:** `crates/anvil-graph-store/src/rkyv_store.rs`
- **Confidence:** high
- **Priority:** Critical
- **Dependencies:** GCTX-002

---

### GCTX-004: Incremental persistence on `GraphDelta`

- **Status:** Draft
- **Intent:** After each `GraphDelta`, persist the updated graph. For
  `RkyvStore` this is a full re-serialise + atomic rename (the whole graph
  at 50k nodes is ~5MB, serialises in <10ms with `rkyv`). For `SqliteStore`
  this is a transaction applying the delta. The kernel's watch loop fans
  deltas into the store, gated by `graph.persist` config toggle.
- **Expected Outcome:** `GraphStore::apply_delta(&GraphDelta)` completes
  within 10ms on a 10k-node graph; debounced to avoid thrashing under
  rapid saves (reuse the watcher's existing 50ms debounce window)
- **Validation:** Criterion benchmark `persist_delta_10k` in
  `crates/anvil-graph-store/benches/`
- **Files:** `crates/anvil-graph-store/src/rkyv_store.rs`,
  `crates/anvil-graph-store/src/sqlite_store.rs`,
  `crates/anvil-kernel/src/watch.rs`
- **Confidence:** high (rkyv path is straightforward; SQLite path is medium)
- **Priority:** Critical
- **Dependencies:** GCTX-003

---

### GCTX-005: Cold-start from snapshot with hash-based validation

- **Status:** Draft
- **Intent:** On kernel startup, if a snapshot exists and its per-file
  content hashes match the filesystem, reuse it instead of full re-parse;
  otherwise re-parse and overwrite
- **Expected Outcome:** Cold start on an unchanged 5k-file repo drops from
  the current full-rebuild cost to under 300ms (snapshot load + hash check
  only)
- **Validation:** BENCH scenario comparing warm vs cold startup, target hit
- **Files:** `crates/anvil-kernel/src/embedded.rs`,
  `crates/anvil-graph-store/src/cold_start.rs`
- **Confidence:** medium
- **Priority:** High
- **Dependencies:** GCTX-003, GCTX-004

---

## Phase 2 — Transitive Impact Analysis

> Build the blast-radius query layer on top of `petgraph`. The existing
> `SymbolGraph` only exposes single-hop `incoming_edges`/`outgoing_edges`;
> this phase adds transitive traversal, test-coverage edges, and a public
> impact API.

### GCTX-010: Transitive caller traversal

- **Status:** Draft
- **Intent:** Given a symbol ID, return the set of all transitive callers
  (direct + indirect) bounded by a configurable depth, with cycle detection
- **Expected Outcome:** `SymbolGraph::transitive_callers(id, max_depth) ->
  Vec<SymbolId>` runs in under 5ms on a 10k-node graph at depth 8; cycles
  (mutual recursion, trait impls) are handled without infinite loops
- **Validation:** Unit tests on hand-crafted fixtures (chain, diamond,
  cycle, disjoint); Criterion bench at 10k nodes
- **Files:** `crates/anvil-kernel/src/graph/impact.rs`
- **Confidence:** high
- **Priority:** Critical
- **Dependencies:** None

---

### GCTX-011: Transitive dependent-file traversal

- **Status:** Draft
- **Intent:** Given a file path, return the set of transitively dependent
  files via import edges in `DependencyGraph`, with cycle handling
- **Expected Outcome:** `DependencyGraph::transitive_dependents(file,
  max_depth) -> Vec<PathBuf>`; covers the "I changed utils.ts, what else
  needs re-checking?" case
- **Validation:** Unit tests plus regression against a fixture with known
  expected reach sets
- **Files:** `crates/anvil-kernel/src/graph/impact.rs`
- **Confidence:** high
- **Priority:** Critical
- **Dependencies:** None

---

### GCTX-012: Test-coverage edge derivation

- **Status:** Draft
- **Intent:** Identify test files via path heuristics (`*.test.ts`,
  `*_test.go`, `test_*.py`, `tests/`), parse their imports, and emit
  `SymbolEdge::TestedBy` edges from target symbols back to test file
  symbols. Heuristic-first; no runtime trace dependency.
- **Expected Outcome:** New `SymbolEdge::TestedBy` variant; fixture with
  `src/foo.ts` + `src/foo.test.ts` produces expected edges; false-positive
  rate on a TS fixture tracked and documented
- **Validation:** Unit tests on fixtures; integration test against the
  `anvil-cli` repo showing expected edge density
- **Files:** `crates/anvil-kernel-types/src/graph.rs`,
  `crates/anvil-kernel/src/graph/tests.rs`
- **Confidence:** medium (heuristic accuracy is the risk)
- **Priority:** High
- **Dependencies:** None

---

### GCTX-013: Unified `BlastRadius` query API

- **Status:** Draft
- **Intent:** Combine symbol-level and file-level traversal plus test-edge
  lookup into a single public entry point: given a set of changed files,
  return affected symbols, affected dependent files, and affected tests
- **Expected Outcome:** `BlastRadius::for_changes(&[PathBuf]) ->
  BlastRadiusReport` returns a structured result inside 20ms on a 10k-node
  graph
- **Validation:** Criterion bench `blast_radius_10k`; integration test with
  a 3-file change on the anvil-cli fixture
- **Files:** `crates/anvil-kernel/src/graph/impact.rs`
- **Confidence:** high
- **Priority:** Critical
- **Dependencies:** GCTX-010, GCTX-011, GCTX-012

---

### GCTX-014: Impact analysis Criterion benchmarks

- **Status:** Draft
- **Intent:** Add Criterion benches for the impact API at graph sizes
  matching the BENCH module's existing tiers (1k, 5k, 10k nodes) so
  regressions are caught in CI
- **Expected Outcome:** New Criterion group `impact_analysis` in the
  existing kernel bench file; results published to the README bench table
- **Validation:** `cargo bench --bench kernel -- impact_analysis` completes
- **Files:** `crates/anvil-kernel/benches/kernel.rs`, `README.md`
- **Confidence:** high
- **Priority:** High
- **Dependencies:** GCTX-013

---

## Phase 3 — Multi-Language Parser Registration

> Today's parser hard-codes TS/JS/TSX/JSX in
> `crates/anvil-kernel/src/parser/languages.rs`. This phase generalises
> registration and brings Python, Go, and Rust online. Anti-pattern and
> suppression support for each language remains with the existing `lang-*`
> modules; this phase touches parsing only.

### GCTX-020: Extract parser registry trait

- **Status:** Draft
- **Intent:** Replace the hard-coded language list with a
  `LanguageParser` trait + registry keyed by file extension, so new
  languages can be added without touching existing code paths
- **Expected Outcome:** `LanguageParser` trait with `parse`, `extract`,
  `language_name`; existing TS/JS parsers refactored behind the trait;
  no behavioural change (existing tests pass unchanged)
- **Validation:** Full kernel test suite green; snapshot tests for
  existing TS fixtures unchanged
- **Files:** `crates/anvil-kernel/src/parser/registry.rs`,
  `crates/anvil-kernel/src/parser/languages.rs`,
  `crates/anvil-kernel/src/parser/extract.rs`
- **Confidence:** high
- **Priority:** Critical
- **Dependencies:** None

---

### GCTX-021: Python parser (symbols + imports)

- **Status:** Draft
- **Intent:** Register `tree-sitter-python` and extract functions, classes,
  methods, and imports into the existing `SymbolGraph`/`DependencyGraph`
- **Expected Outcome:** Python fixture repo produces expected symbols and
  import edges; coordinates with `lang-python` module to avoid duplicating
  work
- **Validation:** Snapshot tests on Python fixtures; integration with
  existing `tests/architecture_parity.rs` style test
- **Files:** `crates/anvil-kernel/src/parser/python.rs`,
  `crates/anvil-kernel/Cargo.toml`
- **Confidence:** medium (relative imports + dynamic `importlib` are edge
  cases)
- **Priority:** High
- **Dependencies:** GCTX-020

---

### GCTX-022: Go parser (symbols + imports)

- **Status:** Draft
- **Intent:** Register `tree-sitter-go` and extract functions, methods,
  types, and imports
- **Expected Outcome:** Go fixture produces expected symbols and edges;
  exported vs unexported visibility determined by identifier case
- **Validation:** Snapshot tests on Go fixtures
- **Files:** `crates/anvil-kernel/src/parser/go.rs`
- **Confidence:** high (Go grammar is simple and stable)
- **Priority:** High
- **Dependencies:** GCTX-020

---

### GCTX-023: Rust parser (symbols + imports)

- **Status:** Draft
- **Intent:** Register `tree-sitter-rust` and extract functions, methods,
  structs, enums, traits, and `use` imports. Handles `mod` declarations
  and `pub(crate)` visibility correctly.
- **Expected Outcome:** Rust fixture produces expected symbols; `use`
  paths resolved to dependency edges; macro-expanded items deferred
- **Validation:** Snapshot tests on Rust fixtures; dog-food against the
  `anvil-kernel` crate itself
- **Files:** `crates/anvil-kernel/src/parser/rust.rs`
- **Confidence:** medium (macros and `cfg` attributes are edge cases)
- **Priority:** High
- **Dependencies:** GCTX-020

---

### GCTX-024: Cross-language fixture + parity tests

- **Status:** Draft
- **Intent:** Build a polyglot fixture repo (TS + Python + Go + Rust in one
  tree) and assert that the graph contains the expected symbols and edges
  for each language without interference
- **Expected Outcome:** New `tests/polyglot_graph.rs` integration test
  covering all four languages
- **Validation:** `cargo test -p eddacraft-anvil-kernel --test polyglot_graph`
- **Files:** `crates/anvil-kernel/tests/polyglot_graph.rs`,
  `crates/anvil-kernel/tests/fixtures/polyglot/`
- **Confidence:** high
- **Priority:** High
- **Dependencies:** GCTX-021, GCTX-022, GCTX-023

---

## Phase 4 — MCP Graph Tools

> Expose the graph on the existing MCP server. Additive only — no existing
> tool signatures change. Each new tool has a clear, single purpose so
> assistants can compose them.

### GCTX-030: `anvil_search_symbols` MCP tool

- **Status:** Draft
- **Intent:** Lexical symbol search by name pattern, kind filter, and file
  glob; returns paginated results with rank ordering
- **Expected Outcome:** MCP tool registered in
  `packages/mcp-server/src/tools/`; integration test queries the tool
  against a fixture and asserts expected results
- **Validation:** MCP server integration test
- **Files:**
  `packages/mcp-server/src/tools/search-symbols.tool.ts`,
  `packages/mcp-server/src/tools/index.ts`
- **Confidence:** high
- **Priority:** Critical
- **Dependencies:** GCTX-013

---

### GCTX-031: `anvil_find_callers` MCP tool

- **Status:** Draft
- **Intent:** Return transitive callers of a named symbol, bounded by a
  `max_depth` argument, with each caller's file and line location
- **Expected Outcome:** Tool calls `BlastRadius` API and returns structured
  JSON; default depth 3, max 10
- **Validation:** MCP integration test on a fixture with a known call
  chain
- **Files:** `packages/mcp-server/src/tools/find-callers.tool.ts`
- **Confidence:** high
- **Priority:** Critical
- **Dependencies:** GCTX-013, GCTX-030

---

### GCTX-032: `anvil_find_dependents` MCP tool

- **Status:** Draft
- **Intent:** File-level counterpart to `find_callers` — returns transitive
  dependent files for a given path
- **Expected Outcome:** MCP tool returning `{file, distance}` pairs sorted
  by distance
- **Validation:** MCP integration test
- **Files:** `packages/mcp-server/src/tools/find-dependents.tool.ts`
- **Confidence:** high
- **Priority:** High
- **Dependencies:** GCTX-013

---

### GCTX-033: `anvil_impact_of_change` MCP tool

- **Status:** Draft
- **Intent:** The headline blast-radius tool. Given a list of changed
  files (staged or arbitrary), return affected symbols, affected
  dependent files, and affected tests in a single structured response.
- **Expected Outcome:** MCP tool accepting `{files: string[], max_depth?:
  number}`, returning a `BlastRadiusReport` JSON; integrated with git
  staging status so assistants can say "what's affected by my current
  diff?"
- **Validation:** MCP integration test simulating a 3-file change
- **Files:** `packages/mcp-server/src/tools/impact-of-change.tool.ts`
- **Confidence:** high
- **Priority:** Critical
- **Dependencies:** GCTX-013

---

### GCTX-034: `anvil_affected_tests` MCP tool

- **Status:** Draft
- **Intent:** Given a set of changed files or symbols, return the tests
  that transitively exercise them via the test-coverage edges built in
  GCTX-012
- **Expected Outcome:** MCP tool returning `{test_file, tested_symbols}`
  records; distinct from `impact_of_change` which reports all affected
  code, this focuses specifically on the test layer
- **Validation:** MCP integration test
- **Files:** `packages/mcp-server/src/tools/affected-tests.tool.ts`
- **Confidence:** medium (quality depends on GCTX-012 heuristic recall)
- **Priority:** High
- **Dependencies:** GCTX-012, GCTX-013

---

### GCTX-035: `graph://` MCP resources

- **Status:** Draft
- **Intent:** Expose the graph as read-only MCP resources so assistants
  can browse symbols, edges, and stats without needing to call tools for
  every lookup
- **Expected Outcome:** Three new resources registered in
  `packages/mcp-server/src/resources/`: `graph://symbols`, `graph://edges`,
  `graph://stats`
- **Validation:** MCP resource listing test
- **Files:** `packages/mcp-server/src/resources/graph.resource.ts`
- **Confidence:** high
- **Priority:** Medium
- **Dependencies:** GCTX-030

---

## Phase 5 — Context Slicing & Token Budget

> Turn graph query results into the minimal code snippets an AI assistant
> actually needs, bounded by a token budget. This is the piece that
> converts "here's a list of 50 symbols" into "here's 4k tokens of
> relevant code to read first."

### GCTX-040: Token-count estimator

- **Status:** Draft
- **Intent:** Provide a fast, deterministic token-count estimator for
  arbitrary source snippets, compatible with Claude and GPT tokenisers
- **Expected Outcome:** `estimate_tokens(&str, Model) -> usize` with a
  documented accuracy envelope (±5% vs the reference tokeniser); used
  downstream for budget enforcement
- **Validation:** Unit tests against a corpus of known snippets with
  reference token counts
- **Files:** `crates/anvil-graph-store/src/tokens.rs`
- **Confidence:** medium (approximations drift between model families)
- **Priority:** High
- **Dependencies:** None

---

### GCTX-041: Symbol snippet extractor

- **Status:** Draft
- **Intent:** Given a symbol ID, return its source span (function body,
  class definition) as a string, with optional N lines of surrounding
  context
- **Expected Outcome:** `extract_snippet(symbol_id, context_lines) ->
  Snippet { text, file, start_line, end_line }`
- **Validation:** Unit tests on TS, Python, Go, Rust fixtures
- **Files:** `crates/anvil-kernel/src/graph/snippet.rs`
- **Confidence:** high
- **Priority:** Critical
- **Dependencies:** None

---

### GCTX-042: Budget-bounded context slicer

- **Status:** Draft
- **Intent:** Given a `BlastRadiusReport` and a token budget, return the
  highest-priority snippets that fit, with deterministic ordering
  (caller distance, then file path) so results are cacheable
- **Expected Outcome:** `slice_context(report, budget) -> ContextSlice`
  with total tokens under budget, and a `truncated: bool` flag when not
  everything fits
- **Validation:** Unit tests on fixtures; property test that total
  tokens never exceed the budget
- **Files:** `crates/anvil-kernel/src/graph/slice.rs`
- **Confidence:** high
- **Priority:** Critical
- **Dependencies:** GCTX-040, GCTX-041, GCTX-013

---

### GCTX-043: `anvil_symbol_context` MCP tool

- **Status:** Draft
- **Intent:** Combine slicer + blast radius into a single MCP entry point:
  "I'm about to work on symbol X, give me the minimal code context I
  need to understand it"
- **Expected Outcome:** MCP tool `anvil_symbol_context({symbol, budget})`
  returning a `ContextSlice`; works equally well for files via an
  overload
- **Validation:** MCP integration test; manual smoke test with Claude
  Code against a fixture
- **Files:** `packages/mcp-server/src/tools/symbol-context.tool.ts`
- **Confidence:** high
- **Priority:** Critical
- **Dependencies:** GCTX-042

---

### GCTX-044: Token-reduction benchmark harness

- **Status:** Draft
- **Intent:** Measure token reduction vs a naive "read all changed files
  plus their imports" baseline across a fixed set of fixture repos;
  publish results so we can validate whether Anvil matches or beats the
  8.2x baseline reported by code-review-graph
- **Expected Outcome:** New scenario in `anvil-bench` that runs
  representative change sets against fixture repos and reports
  token-reduction ratios in JSON plus a README table
- **Validation:** Reproducible results for at least three fixture repos
  (small TS monorepo, Python library, Go service)
- **Files:** `crates/anvil-bench/src/scenarios/token_reduction.rs`,
  `README.md`
- **Confidence:** medium (fair baselines are the hard part)
- **Priority:** High
- **Dependencies:** GCTX-043

---

## Phase 6 — Docs, CLI, Integration

### GCTX-050: `anvil graph` CLI subcommands

- **Status:** Draft
- **Intent:** Expose graph inspection from the CLI for offline debugging
  and scripting: `anvil graph stats`, `anvil graph export`,
  `anvil graph query --symbol foo`, `anvil graph impact <files>`
- **Expected Outcome:** New `graph` subcommand in `anvil-cli` wired to
  `anvil-graph-store`; JSON and table output modes
- **Validation:** E2E test in `apps/e2e/`
- **Files:** `crates/anvil-cli/src/commands/graph.rs`,
  `crates/anvil-cli/src/commands/mod.rs`
- **Confidence:** high
- **Priority:** Medium
- **Dependencies:** GCTX-013, GCTX-042

---

### GCTX-051: User guide — connecting AI assistants via MCP

- **Status:** Draft
- **Intent:** Document how to wire the Anvil MCP server (with the new
  graph tools) into Claude Code, Cursor, Continue, Zed, Windsurf, with a
  worked "refactor this function" walkthrough showing the token savings
- **Expected Outcome:** `docs/guides/ai-context-delivery.md` published;
  linked from main docs index
- **Validation:** Manual walkthrough against each supported client
- **Files:** `docs/guides/ai-context-delivery.md`
- **Confidence:** high
- **Priority:** High
- **Dependencies:** GCTX-043

---

### GCTX-052: Architecture spec

- **Status:** Draft
- **Intent:** Document the end-to-end graph context delivery pipeline as
  a reference spec — data flow, schema, query algorithms, slicing
  strategy, performance targets
- **Expected Outcome:** `docs/architecture/graph-context-delivery-spec.md`
  reviewed and merged
- **Validation:** Council review pass
- **Files:** `docs/architecture/graph-context-delivery-spec.md`
- **Confidence:** high
- **Priority:** High
- **Dependencies:** All prior phases

---

## Risks

| Risk | Likelihood | Impact | Mitigation |
| ---- | ---------- | ------ | ---------- |
| rkyv format changes break cold-start on kernel upgrades | Medium | Medium | Version header in the serialised file; if version mismatch, discard and rebuild from source — it's a cache, not a source of truth |
| SQLite backend write contention under high change rates (if opted in) | Low | Medium | WAL mode, batch deltas; this is the alternative backend, not the default |
| Test-coverage heuristic has poor recall on some ecosystems (e.g. Jest with remapped modules) | Medium | Medium | Document the heuristic, measure recall on representative repos, allow user overrides via config before graduating |
| Multi-language parser grammar licensing conflicts | Low | High | Review each grammar licence in the Ready checklist before landing |
| Context slicer picks the wrong snippets and wastes budget | Medium | Medium | Deterministic ordering + benchmark harness (GCTX-044) to measure reduction; iterate on prioritisation rules |
| MCP tool surface grows too large and assistants get confused | Low | Medium | Stop at 6 new tools in this module; defer semantic search / clustering to a follow-up |
| Duplicates effort with the existing `lang-*` modules | Medium | Low | This module touches parser *registration* only; anti-pattern + suppression work remains with `lang-*`. Coordinated via shared interfaces in GCTX-020 |
| Token-reduction claims don't hold up vs code-review-graph | Medium | Low | GCTX-044 makes the benchmark reproducible; even a 3-4x reduction is valuable and we don't need to beat the headline 8.2x to justify this module |

## Decisions (formerly Open Questions)

1. **Per-repo** — `.anvil/graph.bin`. One graph file per repo, no cross-repo
   contamination. `.anvil/` is already gitignored. Multi-repo queries are
   out of scope; if needed later, a registry layer can index multiple
   per-repo files.
2. **Reuse TS MCP server** — the existing `packages/mcp-server` already
   handles tool registration, transports, and resources. The graph data
   crosses the boundary as JSON-RPC regardless of server language. A
   Rust-native MCP server can follow if latency becomes a bottleneck.
3. **Heuristic only for v1** — import-based test-coverage edges (test file
   imports target → TestedBy edge). Conservative over-estimation (high
   recall, lower precision) is the safe direction. Coverage-report ingestion
   (`lcov.info`, `coverage.xml`) deferred to a follow-up.
4. ~~Partial parse failures~~ — resolved by rkyv + atomic rename. On-disk
   file is always the last complete good state.
5. **Default `true`** — `graph.context` defaults to `true`. Overhead when
   unqueried is negligible (debounced persistence write, <10ms). MCP tools
   only activate when an assistant connects. First-run log line for
   visibility.
6. **Feature flag only** — SQLite backend via
   `cargo install anvil --features sqlite`. Keeps the default binary lean
   (no `libsqlite3`). Promotion to a config option later if demand warrants
   it.

## Stats

| Phase | Items | Status |
| ----- | ----- | ------ |
| 0 — Configuration & Feature Gate | 1 | 0/1 Draft |
| 1 — Persistent Graph Store | 5 | 0/5 Draft |
| 2 — Transitive Impact Analysis | 5 | 0/5 Draft |
| 3 — Multi-Language Parser Registration | 5 | 0/5 Draft |
| 4 — MCP Graph Tools | 6 | 0/6 Draft |
| 5 — Context Slicing & Token Budget | 5 | 0/5 Draft |
| 6 — Docs, CLI, Integration | 3 | 0/3 Draft |
| **Total** | **30** | **0/30 Draft** |
