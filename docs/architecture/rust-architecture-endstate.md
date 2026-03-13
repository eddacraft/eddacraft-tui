# Anvil Rust Architecture — End State Specification

> **Date:** 2026-03-13 **Status:** Reference — synthesised from KERN, RENG,
> RATS, PORT, RSTLAN modules and architecture specifications
>
> Legend: **[DONE]** = shipped, **[DRAFT]** = planned, **[DEFERRED]** = post-H1

---

## 1. Executive Summary

Anvil's Rust layer replaces the TypeScript engine with a persistent,
incremental, deterministic semantic runtime. The migration delivers 10-40x
performance improvements on individual checks and a 14x reduction in watch-cycle
latency (2.9s → 200ms).

The Rust stack is organised into five APS modules totalling ~58 work items:

| Module     | Name                  | Items | Status       | Purpose                                        |
| ---------- | --------------------- | ----- | ------------ | ---------------------------------------------- |
| **KERN**   | Rust Kernel           | 25    | Phase 0 Done | Watcher, parser, semantic graph, policy engine |
| **RENG**   | Engine Ports          | 6     | 4 Done       | Port existing TS checks to Rust                |
| **RATS**   | Ratatui TUI           | 7     | 1 Done       | New TUI surfaces consuming kernel events       |
| **PORT**   | Ink-to-Ratatui Port   | 15    | 2 Done       | 1:1 port of existing Ink surfaces              |
| **RSTLAN** | Rust Language Support | ~5    | Placeholder  | Extend analysis to Rust codebases              |

---

## 2. Crate Layout

### Workspace Configuration (End-State Target)

> **Note:** The current workspace uses an explicit `members` list and additional
> clippy lint settings (e.g. `pedantic` at `warn`). The snippet below shows the
> target configuration once all crates are migrated to the `crates/` directory.

```toml
# Cargo.toml (workspace root) — TARGET
[workspace]
resolver = "2"
members = ["crates/*"]

[workspace.package]
edition = "2024"

[workspace.lints.rust]
unsafe_code = "forbid"

[workspace.lints.clippy]
all = "deny"
pedantic = { level = "warn", priority = -1 }
```

### Crate Map

```
crates/
│
│  ── DONE ──────────────────────────────────────────────────
│
├── spike/                          [DONE] KERN Phase 0
│   ├── src/treesitter.rs           tree-sitter validation (<1ms/file)
│   ├── src/notify.rs               notify-rs validation (<20ms p99)
│   └── src/petgraph.rs             petgraph validation (<500MB)
│
├── anvil-kernel-types/             [DONE] Shared type contracts
│   ├── src/events.rs               EngineEvent, EventType, EventPayload
│   ├── src/graph.rs                SymbolNode, SymbolEdge, SymbolKind
│   ├── src/trust.rs                TrustLevel enum
│   └── src/lib.rs                  EngineId (Rust | Legacy)
│
├── eddacraft-tui/                  [DONE] RATS-001, PORT-001, PORT-002
│   ├── src/keyboard/               KeyHandler, Action types
│   ├── src/theme/                  EddaCraft dark theme
│   └── src/widgets/                15+ shared widgets
│       ├── select.rs               Select list
│       ├── text_input.rs           Text input field
│       ├── progress_bar.rs         Progress bar
│       ├── status_bar.rs           Status bar
│       ├── spinner.rs              Animated spinner
│       ├── status_badge.rs         Pass/fail/warn badges
│       ├── header.rs               Section headers
│       ├── container.rs            Layout container (variants)
│       ├── divider.rs              Horizontal divider
│       ├── confirm.rs              Yes/no confirmation
│       ├── log_panel.rs            Scrollable log output
│       ├── parallel_progress.rs    Multi-check progress
│       ├── quick_wins_panel.rs     Quick-fix suggestions
│       └── results_dashboard.rs    Results summary view
│
├── anvil-checks/                   [DONE] RENG-001..003, RENG-005
│   ├── src/secret/                 Secret detection
│   │   ├── patterns.rs             Pattern definitions
│   │   ├── scanner.rs              Pattern-matching scanner
│   │   ├── entropy.rs              Shannon entropy calculator
│   │   ├── git_scanner.rs          Git-aware scanning
│   │   ├── check.rs                Check interface impl
│   │   └── types.rs                Result types
│   ├── src/antipattern/            Anti-pattern detection
│   │   ├── patterns.rs             13 patterns (AP-001..AP-013)
│   │   ├── scanner.rs              Pattern scanner
│   │   ├── check.rs                Check interface impl
│   │   └── types.rs                Result types
│   ├── src/command_safety/         Command safety validation
│   │   ├── parser.rs               Shell command parser
│   │   ├── matcher.rs              Rule matcher
│   │   ├── rules/                  36 rules (17 git + 19 filesystem)
│   │   ├── check.rs                Check interface impl
│   │   └── types.rs                Result types
│   └── benches/                    Criterion benchmarks
│
│  ── PLANNED ───────────────────────────────────────────────
│
├── anvil-kernel/                   [DRAFT] KERN Phases 1-4
│   ├── src/watcher/                File system watching (notify-rs)
│   │   ├── watcher.rs              Recursive directory watcher
│   │   ├── debounce.rs             50-100ms debounce window
│   │   ├── backpressure.rs         Memory-bounded queue
│   │   └── ignore.rs               .gitignore + custom patterns
│   ├── src/parser/                 Incremental parsing (tree-sitter)
│   │   ├── parser.rs               AST cache keyed by file hash
│   │   ├── symbols.rs              Symbol extraction (fn, class, mod, export)
│   │   └── adapters/               Per-language symbol extractors
│   │       ├── typescript.rs       TS/JS adapter
│   │       └── rust.rs             Rust adapter (RSTLAN)
│   ├── src/graph/                  Semantic graphs (petgraph)
│   │   ├── symbol_graph.rs         SymbolNode + SymbolEdge graph
│   │   ├── dependency_graph.rs     Module-level dependency derivation
│   │   ├── trust.rs                Trust metadata on nodes
│   │   └── incremental.rs          Delta-based graph updates
│   ├── src/policy/                 Policy engine
│   │   ├── config.rs               .anvil/architecture.yaml loader
│   │   ├── framework.rs            GraphDelta → Violation evaluation
│   │   └── invariants/             H1 invariant implementations
│   │       ├── cross_layer.rs      Cross-layer boundary violation
│   │       ├── new_dependency.rs   New external dependency
│   │       ├── api_surface.rs      Public API surface expansion
│   │       └── privilege.rs        Privilege expansion heuristic
│   ├── src/protocol/               Event emission
│   │   ├── events.rs               Progress, Snapshot, Violation, Error
│   │   └── transport.rs            NDJSON framing, channel-based
│   └── src/modes/                  Execution modes
│       ├── embedded.rs             One-shot library API
│       ├── watch.rs                Long-lived foreground mode
│       └── daemon.rs               [DEFERRED] Unix socket server
│
├── anvil-tui/                      [DRAFT] RATS Phase 2-3, PORT Phase 2-4
│   └── src/surfaces/
│       ├── welcome.rs              Welcome screen
│       ├── doctor.rs               Diagnostics
│       ├── status.rs               Status dashboard
│       ├── init/                   Init wizard (multi-step)
│       ├── audit.rs                Audit results
│       ├── template.rs             Template browser
│       ├── gate.rs                 Gate explorer
│       ├── watch.rs                Watch dashboard
│       └── tutorial/               Tutorial paths
│           ├── picker.rs           Tutorial selection
│           ├── policy.rs           Policy tutorial (6 steps)
│           ├── architecture.rs     Architecture tutorial (6 steps)
│           ├── drift.rs            Drift tutorial (5 steps)
│           └── ci.rs               CI tutorial (6 steps)
│
├── eddacraft-kindling/             [DRAFT] (no current RENG ID — future work)
│   └── src/query.rs                Kindling query integration
│
└── bench/                          [DRAFT] RENG-005 (benchmarking)
    └── Cross-crate performance benchmarks
```

---

## 3. Kernel Architecture (KERN)

### Pipeline

```
 ┌──────────────────┐
 │    File System    │
 └────────┬─────────┘
          │  inotify / FSEvents / ReadDirectoryChanges
          ▼
 ┌──────────────────┐     Ignore patterns (.gitignore, .anvilignore)
 │     Watcher      │     Git-aware filtering (optional)
 │   (notify-rs)    │     Recursive directory watching
 └────────┬─────────┘
          │  Raw file events
          ▼
 ┌──────────────────┐     50-100ms debounce window
 │  Debounce/Merge  │     Coalesce multiple events per file
 │     Queue        │     Bounded memory, drop redundant events
 │                  │     Prevent re-entrant recompute loops
 └────────┬─────────┘
          │  Batched change set
          ▼
 ┌──────────────────┐     AST cache keyed by file content hash
 │   Incremental    │     Reparse only changed files
 │     Parser       │     Replace AST subtree, recompute symbols
 │  (tree-sitter)   │     Language-pluggable via grammar crates
 └────────┬─────────┘
          │  Updated symbol tables
          ▼
 ┌──────────────────┐     Persistent in-memory graph (petgraph)
 │  Symbol Graph    │─────Nodes: Function, Class, Module, Export
 │                  │     Edges: Contains, References, Calls, Imports
 └────────┬─────────┘     Each node: id, type, visibility, file, trust_level
          │
          ▼
 ┌──────────────────┐     Derived from symbol graph import edges
 │ Dependency Graph │     Nodes: modules/files
 │                  │     Edges: import/require relationships
 └────────┬─────────┘
          │
          ▼
 ┌──────────────────┐     Trust annotations on graph nodes
 │   Trust Graph    │     Inferred from parser + overridden by config
 │                  │     TrustLevel: Unknown → Internal → Boundary
 └────────┬─────────┘                   → External → Privileged
          │
          │  GraphDelta (added/removed/modified nodes and edges)
          ▼
 ┌──────────────────┐     Invariants are Rust functions
 │  Policy Engine   │     Input: GraphDelta
 │                  │     Output: zero or more Violation events
 │  4 H1 Invariants │     Dedup by (policy_id, file, symbol)
 └────────┬─────────┘     Layer definitions from .anvil/architecture.yaml
          │
          │  EngineEvent stream
          ▼
 ┌──────────────────┐     JSON-RPC 2.0 envelope
 │  Event Emission  │     NDJSON framing for streams
 │                  │     Unix domain socket (daemon mode)
 │  Engine Protocol │     Tokio channels (embedded mode)
 └──────────────────┘
```

### Phases and Work Items

#### Phase 0 — Spike [DONE]

| ID       | Description                   | Validated Target            |
| -------- | ----------------------------- | --------------------------- |
| KERN-001 | tree-sitter parse speed       | <1ms per file               |
| KERN-002 | notify-rs detection latency   | <20ms p99                   |
| KERN-003 | petgraph memory budget        | <500MB for 2000 nodes       |
| KERN-004 | Cargo workspace configuration | Edition 2024, lints         |
| KERN-005 | Rust CI pipeline              | Draft (pending monorepo CI) |

#### Phase 1 — Watcher + Parser [DRAFT]

| ID       | Description                                    | Dependencies |
| -------- | ---------------------------------------------- | ------------ |
| KERN-010 | notify-rs watcher with debounce/backpressure   | —            |
| KERN-011 | tree-sitter incremental parsing with AST cache | —            |
| KERN-012 | Symbol extraction (fn, class, module, export)  | KERN-011     |
| KERN-013 | Ignore patterns + git-aware filtering          | KERN-010     |

#### Phase 2 — Semantic Graph [DRAFT]

| ID       | Description                                          | Dependencies       |
| -------- | ---------------------------------------------------- | ------------------ |
| KERN-020 | Symbol graph (petgraph, SymbolNode + SymbolEdge)     | KERN-012           |
| KERN-021 | Dependency graph derived from import edges           | KERN-020           |
| KERN-022 | Trust metadata on nodes (TrustLevel enum)            | KERN-020           |
| KERN-023 | Incremental graph update (reparse → update subgraph) | KERN-020, KERN-011 |

#### Phase 3 — Policy Engine + Events [DRAFT]

| ID       | Description                                                 | Dependencies                 |
| -------- | ----------------------------------------------------------- | ---------------------------- |
| KERN-030 | Architecture config loader (.anvil/architecture.yaml)       | KERN-020                     |
| KERN-031 | Invariant evaluation framework (GraphDelta → Violations)    | KERN-023                     |
| KERN-032 | H1 invariants (cross-layer, new dep, public API, privilege) | KERN-031, KERN-030, KERN-022 |
| KERN-033 | Event emission (Progress, Snapshot, Violation, Error)       | KERN-031                     |

#### Phase 4 — Integration & Validation [DRAFT]

| ID       | Description                                      | Dependencies                 |
| -------- | ------------------------------------------------ | ---------------------------- |
| KERN-040 | Embedded mode (library API for one-shot checks)  | KERN-033                     |
| KERN-041 | Foreground watch mode (long-lived event stream)  | KERN-010, KERN-023, KERN-033 |
| KERN-042 | Dual-run harness (compare with legacy TS engine) | KERN-040                     |
| KERN-043 | Performance benchmarks against spec targets      | KERN-041                     |
| KERN-044 | Cross-compilation for Linux, macOS, Windows      | KERN-040                     |

#### Phase 5 — Daemon Mode [DEFERRED]

| ID       | Description                              | Dependencies |
| -------- | ---------------------------------------- | ------------ |
| KERN-050 | Unix socket transport (JSON-RPC 2.0)     | KERN-041     |
| KERN-051 | Session management + client multiplexing | KERN-050     |
| KERN-052 | Graceful shutdown + state persistence    | KERN-051     |

---

## 4. Engine Event Protocol

The Engine Event Protocol is the stable contract between the Rust kernel and all
surface consumers. It is the "thin waist" of the architecture — surfaces and
engines are decoupled through this protocol.

### Event Envelope

```rust
struct EngineEvent {
    event_type: EventType,    // progress | snapshot | violation | error
    seq: u64,                 // monotonic sequence number per session
    timestamp: String,        // ISO 8601 with ms precision
    engine: EngineId,         // rust | legacy
    payload: EventPayload,    // variant per event_type
}

enum EngineId { Rust, Legacy }
```

### Event Types

```rust
// Progress — emitted during parsing/evaluation phases
struct ProgressPayload {
    stage: String,            // "parsing", "graph_update", "policy_eval"
    message: String,
    percent: Option<f32>,     // 0.0..1.0
    detail: Option<String>,
}

// Snapshot — emitted after graph recomputation completes
struct SnapshotPayload {
    graph_hash: String,
    files_indexed: u32,
    symbols_indexed: u32,
    duration_ms: u64,
}

// Violation — emitted when a policy invariant is violated
// Matches crates/anvil-kernel-types/src/events.rs EventPayload::Violation
struct ViolationPayload {
    policy_id: String,        // e.g. "cross-layer-boundary"
    file: String,
    symbol: String,           // symbol where violation occurred
    message: String,          // human-readable explanation
}

// Error — emitted on recoverable/non-recoverable errors
// Matches crates/anvil-kernel-types/src/events.rs ErrorPayload
struct ErrorPayload {
    code: ErrorCode,          // ParseError | ConfigError | Internal
    file: Option<String>,
    message: String,
    recoverable: bool,
}
```

### Transport

| Mode               | Transport          | Framing                           |
| ------------------ | ------------------ | --------------------------------- |
| Embedded           | Tokio mpsc channel | Direct Rust types                 |
| Watch (foreground) | stdout             | NDJSON (one JSON object per line) |
| Daemon             | Unix domain socket | JSON-RPC 2.0 + NDJSON             |

---

## 5. Engine Ports (RENG)

Ported checks run as standalone Rust functions, independent of the kernel's
semantic graph. They operate on file content directly.

### Completed Ports

| ID       | Check          | TS Latency | Rust Latency | Speedup | Patterns                  |
| -------- | -------------- | ---------- | ------------ | ------- | ------------------------- |
| RENG-001 | Secret scan    | 200-800ms  | 5-20ms       | **40x** | Entropy + regex patterns  |
| RENG-002 | Anti-pattern   | 500-2000ms | 20-100ms     | **25x** | 13 patterns (AP-001..013) |
| RENG-003 | Command safety | 100-500ms  | 5-20ms       | **25x** | 36 rules (17 git + 19 fs) |
| RENG-005 | Benchmarks     | —          | —            | —       | Criterion harness         |

### Remaining Items

| ID       | Description                                               | Dependencies       | Status      |
| -------- | --------------------------------------------------------- | ------------------ | ----------- |
| RENG-004 | Validate architecture check parity with kernel invariants | KERN-032           | **[DRAFT]** |
| RENG-006 | Feature flag + dual-run for ported checks                 | RENG-005, KERN-042 | **[DRAFT]** |

---

## 6. TUI Architecture (RATS + PORT)

### Component Hierarchy

```
┌─────────────────────────────────────────────────────┐
│                  anvil-tui crate                    │
│              (Anvil-specific surfaces)              │
│                                                     │
│  ┌────────────┐ ┌────────────┐ ┌────────────────┐   │
│  │  Welcome   │ │   Doctor   │ │    Status      │   │
│  │  Surface   │ │  Surface   │ │   Dashboard    │   │
│  └────────────┘ └────────────┘ └────────────────┘   │
│  ┌────────────┐ ┌────────────┐ ┌────────────────┐   │
│  │    Init    │ │   Audit    │ │   Template     │   │
│  │   Wizard   │ │  Results   │ │   Browser      │   │
│  └────────────┘ └────────────┘ └────────────────┘   │
│  ┌────────────┐ ┌────────────┐ ┌────────────────┐   │
│  │    Gate    │ │   Watch    │ │   Tutorial     │   │
│  │  Explorer  │ │ Dashboard  │ │  Orchestrator  │   │
│  └────────────┘ └────────────┘ └────────────────┘   │
│                       │                             │
│              consumes kernel events                 │
│                  via protocol                       │
└───────────────────────┼─────────────────────────────┘
                        │
                   depends on
                        │
┌───────────────────────▼─────────────────────────────┐
│              eddacraft-tui crate                    │
│          (Shared component library)                 │
│                                                     │
│  Theme │ Keyboard │ Select │ TextInput │ Progress   │
│  Header │ Container │ Divider │ Spinner │ Badge     │
│  LogPanel │ ParallelProgress │ QuickWins │ Results  │
│  Confirm │ StatusBar                                │
└─────────────────────────────────────────────────────┘
                        │
                   depends on
                        │
┌───────────────────────▼─────────────────────────────┐
│              ratatui + crossterm                    │
│           (Terminal rendering layer)                │
└─────────────────────────────────────────────────────┘
```

### RATS Work Items

| ID       | Description                               | Dependencies                 | Status      |
| -------- | ----------------------------------------- | ---------------------------- | ----------- |
| RATS-001 | eddacraft-tui shared crate                | —                            | **[DONE]**  |
| RATS-002 | Watch dashboard (live gate results)       | RATS-001, PORT-030, KERN-033 | **[DRAFT]** |
| RATS-003 | Gate result viewer (interactive)          | RATS-001, PORT-023, KERN-040 | **[DRAFT]** |
| RATS-004 | APS onboarding wizard                     | RATS-001                     | **[DRAFT]** |
| RATS-005 | Ink-to-Ratatui migration path             | RATS-002, PORT-023, PORT-030 | **[DRAFT]** |
| RATS-006 | Terminal platform compatibility           | RATS-001, RATS-002           | **[DRAFT]** |
| RATS-007 | `anvil watch` TUI integration entry point | RATS-002, KERN-041           | **[DRAFT]** |

### PORT Work Items

| ID       | Description                        | Ink Source                            | Status      |
| -------- | ---------------------------------- | ------------------------------------- | ----------- |
| PORT-001 | Shared layout + display components | Header, Container, Divider, etc.      | **[DONE]**  |
| PORT-002 | Composite display components       | LogPanel, ParallelProgress, etc.      | **[DONE]**  |
| PORT-010 | Welcome surface                    | `Welcome.tsx`                         | **[DRAFT]** |
| PORT-011 | Doctor surface                     | `Diagnostics.tsx`                     | **[DRAFT]** |
| PORT-012 | Status dashboard                   | `StatusDashboard.tsx` + 3 panels      | **[DRAFT]** |
| PORT-020 | Init wizard                        | `InitWizard.tsx` + 5 steps            | **[DRAFT]** |
| PORT-021 | Audit results                      | `AuditResults.tsx`                    | **[DRAFT]** |
| PORT-022 | Template browser                   | `TemplateBrowser.tsx`                 | **[DRAFT]** |
| PORT-023 | Gate explorer                      | `GateExplorer.tsx` + 3 panels         | **[DRAFT]** |
| PORT-030 | Watch dashboard                    | `WatchDashboard.tsx` + 4 panels       | **[DRAFT]** |
| PORT-040 | Tutorial orchestrator + picker     | `Tutorial.tsx` + `TutorialPicker.tsx` | **[DRAFT]** |
| PORT-041 | Policy tutorial path               | 6 step components                     | **[DRAFT]** |
| PORT-042 | Architecture tutorial path         | 6 step components                     | **[DRAFT]** |
| PORT-043 | Drift tutorial path                | 5 step components                     | **[DRAFT]** |
| PORT-044 | CI tutorial path                   | 6 step components                     | **[DRAFT]** |

---

## 7. Rust Language Support (RSTLAN)

**Status: Placeholder** — extends Anvil to analyse Rust codebases (dogfooding).

### Scope

| Capability        | Description                                    |
| ----------------- | ---------------------------------------------- |
| File detection    | `.rs` files in `src/`, `crates/`, `tests/`     |
| Import extraction | `use`, `mod`, `pub use`, `extern crate`        |
| Module boundary   | `mod.rs` / directory modules, `pub` visibility |
| Anti-patterns     | 6 Rust-specific patterns (see below)           |

### Planned Anti-Patterns

| Pattern                      | What It Detects                   |
| ---------------------------- | --------------------------------- |
| `unsafe` blocks              | Direct unsafe code usage          |
| `#[allow(...)]`              | Suppressed compiler warnings      |
| `unwrap()`/`expect()`        | Panic-prone error handling        |
| `todo!()`/`unimplemented!()` | Incomplete implementations        |
| `as` casts                   | Potentially lossy type casts      |
| `#[cfg(test)]` misuse        | Test code leaking into production |

### Out of Scope (H1)

Cargo dependency resolution, proc-macro expansion, lifetime analysis, trait
resolution, borrow checker analysis.

---

## 8. Dependency Graph Between Modules

```
                         ┌───────────────────┐
                         │       KERN        │
                         │   Rust Kernel     │
                         │  (25 work items)  │
                         └─┬───────────┬─────┘
                           │           │
            ┌──────────────┘           └──────────────┐
            │                                         │
            ▼                                         ▼
  ┌─────────────────┐                      ┌──────────────────┐
  │      RENG       │                      │      RATS        │
  │  Engine Ports   │                      │   Ratatui TUI    │
  │ (6 work items)  │                      │  (7 work items)  │
  │                 │                      │                  │
  │ Depends on:     │                      │ Depends on:      │
  │  KERN Phase 1-2 │                      │  KERN Phase 3    │
  │  (parser/graph) │                      │  (event emission)│
  └─────────────────┘                      └────────┬─────────┘
                                                    │
                                           depends on RATS-001
                                                    │
                                           ┌────────▼─────────┐
                                           │      PORT        │
                                           │ Ink→Ratatui Port │
                                           │ (15 work items)  │
                                           │                  │
                                           │ Depends on:      │
                                           │  RATS-001 (done) │
                                           └──────────────────┘

                                           ┌──────────────────┐
                                           │     RSTLAN       │
                                           │ Rust Lang Support│
                                           │  (~5 items)      │
                                           │                  │
                                           │ Depends on:      │
                                           │  KERN Phase 1    │
                                           │  (parser infra)  │
                                           └──────────────────┘
```

### Critical Path

```
KERN Phase 1 (Watcher + Parser)
    └──► KERN Phase 2 (Semantic Graph)
           └──► KERN Phase 3 (Policy Engine + Events)
                  ├──► KERN Phase 4 (Integration + Validation)
                  │      └──► RENG-004 (Architecture parity)
                  │      └──► RENG-006 (Dual-run feature flag)
                  └──► RATS-002..007 (Kernel-native TUI surfaces)
                         └──► RATS-005 (Migration path)
```

### Parallelisable Work

These can proceed independently of the kernel critical path:

- **PORT-010..044** — Ink-to-Ratatui surface ports (external dependency is
  RATS-001, which is done; internally PORT-010..030 depend on PORT-001/002, and
  PORT-041..044 depend on PORT-040)
- **RSTLAN** — Rust language support (depends on KERN Phase 1 parser, but the
  grammar/adapter work can start in parallel)
- **RATS-004** — APS onboarding wizard (depends only on RATS-001)

---

## 9. Performance Targets

| Metric                       | Current (TS) | Target (Rust) | Improvement    |
| ---------------------------- | ------------ | ------------- | -------------- |
| Watch cycle                  | 2.9s         | 200ms         | **14x**        |
| Secret scan                  | 200-800ms    | 5-20ms        | **40x**        |
| Anti-pattern scan            | 500-2000ms   | 20-100ms      | **25x**        |
| Command safety               | 100-500ms    | 5-20ms        | **25x**        |
| Cold graph build (100k LOC)  | N/A          | <3s           | New capability |
| Incremental update (1 file)  | N/A          | <100ms        | New capability |
| File detection latency (p99) | N/A          | <20ms         | New capability |
| tree-sitter parse (1 file)   | N/A          | <1ms          | New capability |
| Event emission overhead      | N/A          | <10ms         | New capability |
| Memory footprint (100k LOC)  | N/A          | <500MB        | New capability |

---

## 10. Key Dependencies (Cargo)

| Category      | Crate                    | Version  | Used By                  |
| ------------- | ------------------------ | -------- | ------------------------ |
| Parsing       | `tree-sitter`            | 0.26     | anvil-kernel             |
| Parsing       | `tree-sitter-typescript` | 0.23     | anvil-kernel             |
| Parsing       | `tree-sitter-javascript` | 0.25     | anvil-kernel             |
| File watching | `notify`                 | 8        | anvil-kernel             |
| Graph         | `petgraph`               | 0.8      | anvil-kernel             |
| Serialisation | `serde`                  | 1        | all crates               |
| Serialisation | `serde_json`             | 1        | all crates               |
| TUI           | `ratatui`                | 0.30     | eddacraft-tui, anvil-tui |
| TUI           | `crossterm`              | 0.29     | eddacraft-tui, anvil-tui |
| Async         | `tokio`                  | 1 (full) | anvil-kernel             |
| Testing       | `insta`                  | 1 (yaml) | all crates               |
| Benchmarks    | `criterion`              | 0.5      | anvil-checks, bench      |
| Regex         | `regex`                  | 1        | anvil-checks             |

---

## 11. Migration Strategy

### Phase 1 — Coexistence

- Rust binary ships alongside Node.js CLI
- `--engine rust` flag selects kernel (hidden/opt-in)
- `--tui=ratatui` flag selects TUI
- Ink TUI and TS engine remain defaults
- Dual-run mode validates output parity

### Phase 2 — Validation

- All ported checks validated via dual-run diffing
- Ratatui surfaces feature-complete and tested
- Performance benchmarks confirm targets met
- Beta users opt in to Rust engine

### Phase 3 — Cutover

- Rust becomes default engine
- Ratatui becomes default TUI
- `--engine legacy` and `--tui=ink` remain as fallbacks
- Watch mode uses persistent graph by default

### Phase 4 — Standalone

- Single Rust binary, no Node.js required
- npm thin wrapper (optional) for npm-based install
- Legacy TS engine removed
- Ink TUI removed

---

## 12. Workspace Policies

- **`unsafe` code forbidden** — workspace-level `forbid` lint
- **Clippy deny-all** — with select pedantic exceptions
- **Edition 2024** — latest stable Rust edition
- **Resolver 2** — Cargo feature resolver v2
- **Release profile** — thin LTO + symbol stripping enabled
- **Snapshot testing** — insta for all crate tests
- **No panics across boundaries** — all kernel errors are structured events
- **Deterministic output** — no AI in enforcement path; same inputs → same
  outputs
