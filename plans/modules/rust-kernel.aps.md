<!--
APS Module: Rust Kernel
========================
Standalone semantic runtime: watcher, parser, graph, policy engine, event
emission. The foundation for structural analysis in Anvil.

Scopes: KERN (main)
-->

# Rust Kernel

| ID   | Owner | Status |
| ---- | ----- | ------ |
| KERN | —     | In Progress |

## Purpose

Build the Rust Watcher Kernel — a persistent, incremental, deterministic semantic
runtime that continuously enforces structural invariants and streams governance
events.

**Why:** The kernel is the source of truth for structural analysis. It owns the
persistent semantic graph (symbol + dependency + trust metadata) and evaluates
architectural invariants on every file change. This is a new capability that does
not exist in the current TypeScript engine — the TS engine runs batch checks, but
does not maintain a live graph.

**Spec:** [Rust Kernel Specification](../../docs/architecture/rust-kernel-spec.md)
**Evolution:** [Architecture Evolution](../../docs/architecture/anvil-architecture-evolution.md)

## In Scope

- File system watching (notify-rs) with debounce and backpressure
- Incremental parsing (tree-sitter) with AST cache
- Symbol extraction (functions, classes, modules, exports)
- Persistent semantic graph (petgraph — SymbolNode + SymbolEdge)
- Dependency graph derived from import edges
- Trust metadata on graph nodes (TrustLevel enum)
- Policy engine evaluating structural invariants against graph deltas
- Event emission (Progress, Snapshot, Violation, Error) via EngineEvent envelope
- Embedded mode (library API for one-shot checks)
- Foreground watch mode (long-lived event stream)
- Cargo workspace layout (`anvil-kernel`, `anvil-kernel-types`)

## Out of Scope

- Porting existing checks to Rust (see RENG module)
- TUI rendering (see RATS module)
- CLI argument parsing (future `anvil-cli` crate)
- Daemon mode implementation (architecture-ready but deferred)
- N-API bindings (superseded by standalone binary approach)
- Kindling observation storage
- VS Code extension integration

## Interfaces

**Depends on:**

- `.anvil/architecture.yaml` — layer definitions for policy evaluation
- tree-sitter grammar crates (`tree-sitter-typescript`, `tree-sitter-rust`)

**Exposes:**

- `anvil-kernel` — main crate (watcher, parser, graph, policy, protocol)
- `anvil-kernel-types` — shared types (events, graph nodes, config). This is the
  **canonical EngineEvent envelope contract** consumed by RENG and RATS. The
  event schema is defined in KERN-033; the types crate is the shared dependency
  boundary.
- Library API for embedded mode (called by `anvil` binary)
- Event stream for watch mode (consumed by surfaces)

## Constraints

- H1 targets TypeScript/JavaScript and Rust language support
- Cold graph build <3 seconds for 100k LOC repo
- Incremental update <100ms for single-file change in medium repo (~2000 files)
- Event emission overhead <10ms
- Memory footprint <500MB for medium repo
- Must support dual-run comparison with legacy TS engine
- Architecture must not prevent future daemon mode (keep public API clean)

## Ready Checklist

Change status to **Ready** when:

- [x] Phase 0 spike validates tree-sitter parsing (<1ms per file)
- [x] Phase 0 spike validates notify-rs detection latency (<20ms p99)
- [x] Phase 0 spike validates petgraph memory for 2000-node graph (<500MB)
- [x] Phase 0 spike validates Cargo workspace builds alongside pnpm _(validated
      in external Rust workspace)_
- [x] Cargo workspace structure agreed _(documented in spec; pending monorepo
      integration)_

---

## Phase 0 — Spike (Validation)

> **Note:** Phase 0 spike work was validated in the standalone
> [eddacraft-rust-kernel](https://github.com/EddaCraft/eddacraft-rust-kernel)
> workspace. The `crates/spike/` path references that repo. Spike artefacts will
> be vendored into this monorepo when Phase 1 begins.

### KERN-001: Validate tree-sitter TS/JS parsing speed

- **Status:** Done
- **Intent:** Confirm tree-sitter parses TypeScript files in <1ms per file
  (100-1000 LOC), making AST-based checks viable at watch-mode speed
- **Expected Outcome:** Benchmark showing parse time for representative files is
  consistently <1ms
- **Validation:** Benchmark harness with 50+ real project files, p99 < 1ms
- **Files:** `crates/spike/`
- **Confidence:** high (tree-sitter benchmarks support this)
- **Priority:** Critical
- **Dependencies:** None

---

### KERN-002: Validate notify-rs file detection latency

- **Status:** Done
- **Intent:** Confirm notify-rs detects file changes with <20ms p99 latency,
  replacing Chokidar's ~75ms
- **Expected Outcome:** Latency benchmark showing detection time from write to
  callback
- **Validation:** 100 file-write events, p99 detection < 20ms
- **Files:** `crates/spike/`
- **Confidence:** high
- **Priority:** Critical
- **Dependencies:** None

---

### KERN-003: Validate petgraph memory for 2000-node graph

- **Status:** Done
- **Intent:** Confirm petgraph memory usage is within the <500MB budget for a
  medium repo (~2000 files, ~100k LOC)
- **Expected Outcome:** Memory measurement showing graph + AST cache fits within
  budget
- **Validation:** Build graph for synthetic repo (2000 files, ~15k symbol nodes),
  measure RSS
- **Files:** `crates/spike/`
- **Confidence:** high (petgraph is arena-allocated, expected <50MB for graph
  alone)
- **Priority:** High
- **Dependencies:** None

---

### KERN-004: Validate Cargo workspace builds alongside pnpm

- **Status:** Done
- **Intent:** Confirm Cargo workspace and pnpm monorepo coexist without CI
  conflicts
- **Expected Outcome:** Both `cargo build` and `pnpm build` succeed in CI
- **Validation:** CI pipeline runs both build systems, no conflicts
- **Files:** `Cargo.toml`, `.github/workflows/`
- **Confidence:** high
- **Priority:** High
- **Dependencies:** None

---

### KERN-005: Rust CI pipeline (cargo test, clippy, fmt)

- **Status:** Done
- **Intent:** Establish a Rust CI pipeline in GitHub Actions that runs `cargo
  test`, `cargo clippy`, and `cargo fmt --check` on every PR alongside the
  existing pnpm pipeline
- **Expected Outcome:** PRs touching Rust code are gated by Rust quality checks;
  failures block merge
- **Validation:** CI rejects a PR with a clippy warning or fmt violation
- **Files:** `.github/workflows/`, `Cargo.toml` _(validated in external Rust
  workspace; CI steps will be added to this repo's workflow when Rust source
  lands)_
- **Confidence:** high
- **Priority:** High
- **Dependencies:** KERN-004

---

## Phase 1 — Watcher + Parser

### KERN-010: notify-rs watcher with debounce + backpressure

- **Status:** Done
- **Intent:** Implement file watching with platform-native notifications,
  coalescing rapid changes within a debounce window (50-100ms), and bounding
  memory growth under burst conditions
- **Expected Outcome:** Watcher detects file changes, coalesces bursts, and
  feeds batched change sets to the parser
- **Validation:** Stress test with 100 rapid file writes, watcher produces
  correct batches without memory growth
- **Files:** `crates/anvil-kernel/src/watcher/`
- **Confidence:** high
- **Priority:** Critical
- **Dependencies:** KERN-002

---

### KERN-011: tree-sitter incremental parsing with AST cache

- **Status:** Done
- **Intent:** Integrate tree-sitter for incremental parsing with an AST cache
  keyed by file content hash. On file change, reparse only the changed file and
  replace its AST subtree.
- **Expected Outcome:** Parser produces typed ASTs for TS/JS and Rust files,
  cached by content hash, with incremental reparse on change
- **Validation:** Parse 100+ project files, verify cache hits on unchanged files,
  verify correct reparse on changed files
- **Files:** `crates/anvil-kernel/src/parser/`
- **Confidence:** high
- **Priority:** Critical
- **Dependencies:** KERN-001

---

### KERN-012: Symbol extraction (functions, classes, modules, exports)

- **Status:** Done
- **Intent:** Extract symbol nodes from tree-sitter ASTs using `.scm` query
  files. Produce SymbolNode entries for functions, classes, modules, and exports.
- **Expected Outcome:** Symbol table populated from parsed ASTs, covering H1
  node types
- **Validation:** Extract symbols from 50+ representative files, compare against
  manually verified expected output
- **Files:** `crates/anvil-kernel/src/parser/queries/`, `crates/anvil-kernel/src/parser/extract.rs`
- **Confidence:** high
- **Priority:** Critical
- **Dependencies:** KERN-011

---

### KERN-013: Ignore patterns + git-aware filtering

- **Status:** Done
- **Intent:** Support ignore patterns (node_modules, build outputs, .gitignore)
  and optional git-aware filtering to skip untracked/ignored files
- **Expected Outcome:** Watcher and parser skip files matching ignore patterns
- **Validation:** Watcher ignores node_modules, build/, and .gitignore'd paths
- **Files:** `crates/anvil-kernel/src/watcher/filter.rs`
- **Confidence:** high
- **Priority:** High
- **Dependencies:** KERN-010

---

## Phase 2 — Semantic Graph

### KERN-020: Symbol graph (petgraph, SymbolNode + SymbolEdge)

- **Status:** Done
- **Intent:** Build the persistent in-memory symbol graph using petgraph. Nodes
  are SymbolNode (functions, classes, modules, exports). Edges are SymbolEdge
  (contains, references, calls, imports).
- **Expected Outcome:** Graph populated from symbol extraction output, queryable
  for node/edge lookups
- **Validation:** Build graph for test fixture repo, verify node/edge counts and
  relationships match expected structure
- **Files:** `crates/anvil-kernel/src/graph/`
- **Confidence:** high
- **Priority:** Critical
- **Dependencies:** KERN-012

---

### KERN-021: Dependency graph derived from import edges

- **Status:** Done
- **Intent:** Derive a module-level dependency graph from the symbol graph's
  import edges. Nodes are modules/files, edges are import/require relationships.
- **Expected Outcome:** Dependency graph correctly reflects import structure,
  usable for cross-layer violation detection
- **Validation:** Compare dependency edges against known project import structure
- **Files:** `crates/anvil-kernel/src/graph/dependency.rs`
- **Confidence:** high
- **Priority:** Critical
- **Dependencies:** KERN-020

---

### KERN-022: Trust metadata on nodes (TrustLevel enum)

- **Status:** Done
- **Intent:** Annotate graph nodes with TrustLevel (Unknown, Internal, Boundary,
  External, Privileged) based on symbol extraction heuristics and configuration
- **Expected Outcome:** Nodes have correct trust levels based on their
  characteristics (exports → Boundary, fetch calls → External, etc.)
- **Validation:** Verify trust levels for representative symbols against expected
  values
- **Files:** `crates/anvil-kernel/src/graph/trust.rs`, `crates/anvil-kernel-types/src/trust.rs`
- **Confidence:** medium (heuristics need tuning)
- **Priority:** High
- **Dependencies:** KERN-020

---

### KERN-023: Incremental graph update (reparse → update subgraph)

- **Status:** Done
- **Intent:** On file change, reparse the affected file, diff the old and new
  symbol sets, and update only the affected subgraph (add/remove/modify nodes
  and edges). Produce a GraphDelta for the policy engine.
- **Expected Outcome:** Graph updates correctly on file change without full
  rebuild, GraphDelta accurately reflects changes
- **Validation:** Modify test fixture files, verify graph delta matches expected
  changes, verify graph state is correct after incremental update
- **Files:** `crates/anvil-kernel/src/graph/incremental.rs`
- **Confidence:** medium (incremental correctness is the hardest part)
- **Priority:** Critical
- **Dependencies:** KERN-020, KERN-011

---

## Phase 3 — Policy Engine + Events

### KERN-030: Architecture config loader (`.anvil/architecture.yaml`)

- **Status:** Done
- **Intent:** Load layer definitions from `.anvil/architecture.yaml` (existing
  format: clean.yaml, layered.yaml, etc.) and annotate graph nodes with their
  architectural layer
- **Expected Outcome:** Config loader parses YAML, maps file paths to layers,
  nodes annotated with layer metadata
- **Validation:** Load existing architecture configs from test fixtures, verify
  layer assignment matches expected output
- **Files:** `crates/anvil-kernel/src/policy/config.rs`
- **Confidence:** high
- **Priority:** High
- **Dependencies:** KERN-020

---

### KERN-031: Invariant evaluation framework (GraphDelta → violations)

- **Status:** Done
- **Intent:** Build the framework for evaluating invariants against GraphDeltas.
  Invariants are Rust functions that receive a delta and return violations.
  Violations are fingerprinted by (policy_id, file, symbol) for deduplication.
- **Expected Outcome:** Framework accepts registered invariant functions, runs
  them against deltas, deduplicates violations, and produces Violation events
- **Validation:** Register test invariants, feed synthetic deltas, verify correct
  violations produced and deduplicated
- **Files:** `crates/anvil-kernel/src/policy/engine.rs`
- **Confidence:** high
- **Priority:** Critical
- **Dependencies:** KERN-023

---

### KERN-032: H1 invariants (cross-layer, new dep, public API, privilege)

- **Status:** Done
- **Intent:** Implement the four H1 invariants as Rust functions:
  1. Cross-layer boundary violation
  2. New external dependency introduction
  3. Public API surface expansion
  4. Privilege expansion heuristic
- **Expected Outcome:** Each invariant correctly detects its violation type in
  test fixtures
- **Validation:** Test fixtures with known violations, verify detection. Snapshot
  tests via `insta` crate for regression.
- **Files:** `crates/anvil-kernel/src/policy/invariants/`
- **Confidence:** medium (heuristics need tuning, especially privilege expansion)
- **Priority:** Critical
- **Dependencies:** KERN-031, KERN-030, KERN-022

---

### KERN-033: Event emission (Progress, Snapshot, Violation, Error)

- **Status:** Done
- **Intent:** Implement the EngineEvent envelope and all H1 event types. Events
  are emitted in-process via a channel (tokio::sync or crossbeam) for consumption
  by surfaces.
- **Expected Outcome:** Kernel emits correctly structured events during parsing,
  graph building, and policy evaluation
- **Validation:** Capture event stream during full cycle, verify event ordering,
  envelope fields, and payload correctness
- **Files:** `crates/anvil-kernel/src/protocol/`, `crates/anvil-kernel-types/src/events.rs`
- **Confidence:** high
- **Priority:** Critical
- **Dependencies:** KERN-031

---

## Phase 4 — Integration & Validation

### KERN-040: Embedded mode (library API for one-shot checks)

- **Status:** Done
- **Intent:** Expose a library API for one-shot checks. The `anvil` binary calls
  kernel functions directly (no IPC, no serialization). Runs engine, emits
  events, exits.
- **Expected Outcome:** `anvil check` runs the kernel in embedded mode and
  produces the same event stream as watch mode for a given set of files
- **Validation:** Run embedded mode on test fixture, compare event output with
  expected results
- **Files:** `crates/anvil-kernel/src/lib.rs`
- **Confidence:** high
- **Priority:** Critical
- **Dependencies:** KERN-033

---

### KERN-041: Foreground watch mode (long-lived event stream)

- **Status:** Done
- **Intent:** Implement `anvil watch` as a long-lived process that runs the full
  watcher → parser → graph → policy → event pipeline continuously
- **Expected Outcome:** Watch mode detects file changes, incrementally updates
  the graph, evaluates policies, and streams events
- **Validation:** Start watch mode on test repo, modify files, verify events
  stream correctly with incremental updates
- **Files:** `crates/anvil-kernel/src/watch.rs`
- **Confidence:** high
- **Priority:** Critical
- **Dependencies:** KERN-010, KERN-023, KERN-033

---

### KERN-042: Dual-run harness (compare with legacy TS engine)

- **Status:** Done
- **Intent:** Build a test harness that runs both the Rust kernel and the legacy
  TS engine against the same change stream, normalises their event output, and
  diffs the results
- **Expected Outcome:** Harness identifies discrepancies between engines,
  enabling parity validation before cutover
- **Validation:** Run on test fixture repo, verify harness catches intentionally
  introduced discrepancies
- **Files:** `crates/anvil-kernel/tests/dual_run.rs`
- **Confidence:** medium (normalisation logic may be complex)
- **Priority:** High
- **Dependencies:** KERN-040

---

### KERN-043: Performance benchmarks against spec targets

- **Status:** Done
- **Completed:** 2026-03-16
- **Updated:** 2026-04-03 (rayon parallel parse, PR #746)
- **Intent:** Benchmark the kernel against all performance targets from the spec:
  cold build <3s, incremental <100ms, event overhead <10ms, memory <500MB
- **Expected Outcome:** Benchmark report confirming all targets are met (or
  identifying which need optimisation)
- **Validation:** criterion.rs benchmarks on representative repos
- **Files:** `crates/anvil-kernel/benches/`
- **Confidence:** high
- **Priority:** High
- **Dependencies:** KERN-041

#### Benchmark Results (2026-04-03, rayon parallel parse)

| Metric | Before | After | Notes |
|---|---|---|---|
| Cold build, 10 files | 1.51 ms | 978 µs | 1.5x via rayon |
| Cold build, 50 files | 9.70 ms | 5.56 ms | 1.7x via rayon |
| Cold build, 100 files | 24.5 ms | 14.5 ms | 1.7x via rayon |
| Incremental save | 10 µs | 10 µs | Unchanged |
| Policy evaluation (H1) | 799 ns | 799 ns | Unchanged |
| Secret scan (small file) | — | 4.07 ms | |
| Command safety (simple) | — | 507 ns | |
| Burst, 10 files | 1,924 µs | 693 µs | 2.8x via rayon |
| Burst, 50 files | 10,449 µs | 3,460 µs | 3.0x via rayon |

All spec targets met. Cold build well under 3s at any realistic codebase size.
Incrementalupdate at 10µs is 10,000x under the 100ms target.

---

### KERN-044: Cross-compilation for Linux, macOS, Windows

- **Status:** Done _(6/6 platforms — Linux x86_64/aarch64, macOS x86_64/aarch64, Windows x86_64/aarch64)_
- **Intent:** Ensure the `anvil` binary cross-compiles for all target platforms
  (Linux x86_64/aarch64, macOS x86_64/aarch64, Windows x86_64/aarch64) and passes
  platform-specific smoke tests
- **Expected Outcome:** CI produces release binaries for all targets; smoke tests
  confirm binary runs on each platform
- **Validation:** CI matrix builds for all targets succeed, smoke test verifies
  binary startup and basic check on each
- **Files:** `.github/workflows/rust.yml`, `crates/anvil-kernel/src/embedded.rs`,
  `crates/anvil-cli/src/commands/tutorial.rs`
- **Confidence:** high
- **Priority:** High
- **Dependencies:** KERN-040

---

## Phase 5 — Daemon Mode (Deferred, architecture-ready)

### KERN-050: Unix domain socket transport

- **Status:** Draft
- **Intent:** Add Unix domain socket transport for daemon mode, enabling
  multi-client connections to a long-lived kernel process
- **Expected Outcome:** Kernel accepts client connections over Unix socket
- **Validation:** Multiple clients connect simultaneously, receive events
- **Files:** `crates/anvil-kernel/src/transport/`
- **Confidence:** medium
- **Priority:** Low (deferred to post-H1)
- **Dependencies:** KERN-041

---

### KERN-051: JSON-RPC request/response + notification protocol

- **Status:** Draft
- **Intent:** Wrap the kernel's public API with JSON-RPC 2.0 for daemon mode.
  Request/response for queries (kernel/status, kernel/check, kernel/graph/query).
  Notifications for streaming (kernel/violation, kernel/progress, kernel/snapshot).
- **Expected Outcome:** Full JSON-RPC protocol implementation over Unix socket
- **Validation:** JSON-RPC conformance tests, round-trip latency benchmarks
- **Files:** `crates/anvil-kernel/src/transport/jsonrpc.rs`
- **Confidence:** medium
- **Priority:** Low (deferred to post-H1)
- **Dependencies:** KERN-050

---

### KERN-052: Client session management + event fan-out

- **Status:** Draft
- **Intent:** Manage multiple client sessions with per-session event filtering
  and fan-out. Each client subscribes to event types and receives only relevant
  events.
- **Expected Outcome:** Multiple clients with different subscriptions receive
  correct event subsets
- **Validation:** Multi-client test with different subscription filters
- **Files:** `crates/anvil-kernel/src/transport/session.rs`
- **Confidence:** medium
- **Priority:** Low (deferred to post-H1)
- **Dependencies:** KERN-051

---

## Performance Targets

| Metric | Target |
| ------ | ------ |
| Cold graph build (100k LOC) | <3 seconds |
| Incremental update (single file) | <100ms |
| Event emission overhead | <10ms |
| Memory footprint (medium repo) | <500MB |
| File detection latency (p99) | <20ms |
| tree-sitter parse (single file) | <1ms |

## Risks

| Risk | Likelihood | Impact | Mitigation |
| ---- | ---------- | ------ | ---------- |
| Incremental graph correctness | Medium | High | Extensive snapshot testing with insta |
| tree-sitter TS grammar edge cases | Medium | Low | Pin grammar versions, test on update |
| Trust level heuristic accuracy | Medium | Medium | Conservative defaults (Unknown), allow overrides |
| Cargo + pnpm CI coexistence | Low | Medium | Spike validates (KERN-004) |
| Memory budget exceeded | Low | Medium | Profile early, arena allocation |

## Stats

| Phase | Items | Status |
| ----- | ----- | ------ |
| 0 — Spike | 5 | Done |
| 1 — Watcher + Parser | 4 | Done |
| 2 — Semantic Graph | 4 | Done |
| 3 — Policy Engine + Events | 4 | Done |
| 4 — Integration & Validation | 5 | Done |
| 5 — Daemon Mode (Deferred) | 3 | Draft |
| **Total** | **25** | **22/25 done** |
