# Proposed Rust Architecture — Full Overview

> Compiled from APS modules KERN, RENG, RATS, PORT, RSTLAN, TUI (superseded),
> and supporting architecture documents. This is a reference document — not a
> plan itself.

## Executive Summary

The Rust architecture replaces Anvil's Node.js-based analysis engine with a
standalone Rust binary that provides 10-40x performance improvements. The
migration is incremental: existing TypeScript checks continue to work via a
dual-run mode while Rust equivalents are validated for parity. The end state is
a single `anvil` binary that watches files, builds a semantic graph, evaluates
policies, and renders a terminal UI — all in one process with zero IPC overhead.

## Module Map

Six APS modules cover the Rust work. Three are structural (KERN, RENG, PORT) and
three are surface/integration (RATS, RSTLAN, TUI — superseded).

```
KERN (Rust Kernel)          — The foundation. Watcher, parser, graph, policy engine.
  |
  +-- RENG (Engine Ports)   — Port existing TS checks to Rust using KERN's infrastructure.
  |
  +-- RATS (Ratatui TUI)    — New TUI surfaces consuming KERN events.
  |     |
  |     +-- PORT (Ink Port) — 1:1 port of existing Ink surfaces to Ratatui.
  |
  +-- RSTLAN (Rust Lang)    — Extend Anvil to analyse Rust codebases (placeholder).

TUI (superseded)            — Original OpenTUI/Ink approach, replaced by RATS.
```

## Crate Layout

### Current State (in monorepo)

```
Cargo.toml                          # Workspace root
crates/
  anvil-kernel-types/               # Shared types: events, graph nodes, trust levels
    src/
      lib.rs
      events.rs                     # EngineEvent envelope contract
      graph.rs                      # SymbolNode, SymbolEdge types
      trust.rs                      # TrustLevel enum
  eddacraft-tui/                    # Shared Ratatui component library (RATS-001, done)
    src/
      lib.rs
      keyboard/                     # Key handler, conventions (j/k, space, esc, q)
      theme/                        # EddaCraft dark theme, trait system
      widgets/                      # Select, TextInput, ProgressBar, StatusBar
  spike/                            # Phase 0 validation spikes (done)
    src/
      treesitter.rs                 # Parse speed validation (<1ms/file)
      notify.rs                     # File detection latency (<20ms p99)
      petgraph.rs                   # Memory validation (<500MB for 2000 nodes)
```

### Planned Crates (from KERN, RENG, RATS, PORT modules)

```
crates/
  anvil-kernel/                     # KERN — core engine
    src/
      lib.rs                        # Embedded mode API (KERN-040)
      watch.rs                      # Foreground watch mode (KERN-041)
      watcher/                      # notify-rs integration (KERN-010)
        filter.rs                   # .gitignore + ignore patterns (KERN-013)
      parser/                       # tree-sitter integration (KERN-011)
        queries/                    # .scm query files for symbol extraction
        extract.rs                  # Symbol extraction (KERN-012)
      graph/                        # Semantic graph (KERN-020)
        dependency.rs               # Module-level dependency graph (KERN-021)
        trust.rs                    # TrustLevel annotation (KERN-022)
        incremental.rs              # Incremental subgraph update (KERN-023)
      policy/                       # Policy engine (KERN-031)
        config.rs                   # Architecture YAML loader (KERN-030)
        engine.rs                   # Invariant evaluation framework (KERN-031)
        invariants/                 # H1 invariants (KERN-032)
      protocol/                     # Event emission (KERN-033)
      transport/                    # Daemon mode (KERN-050–052, deferred)
    tests/
      dual_run/                     # Parity harness vs TS engine (KERN-042)
    benches/
      checks.rs                     # criterion.rs benchmarks (KERN-043, RENG-005)

  anvil-kernel-types/               # Already exists — shared type contract

  anvil-tui/                        # RATS + PORT — Anvil-specific TUI surfaces
    src/
      surfaces/
        welcome/                    # PORT-010
        doctor/                     # PORT-011
        status/                     # PORT-012
        init/                       # PORT-020
        audit/                      # PORT-021
        new/                        # PORT-022
        gate/                       # PORT-023 + RATS-003
        watch/                      # PORT-030 + RATS-002
        tutorial/                   # PORT-040–044
          policy/
          architecture/
          drift/
          ci/

  eddacraft-tui/                    # Already exists — shared widget library
```

### Workspace Dependencies

| Dependency             | Purpose                  | Used By                          |
| ---------------------- | ------------------------ | -------------------------------- |
| tree-sitter            | Incremental parsing      | anvil-kernel                     |
| tree-sitter-typescript | TS/JS grammar            | anvil-kernel                     |
| tree-sitter-javascript | JS grammar               | anvil-kernel                     |
| notify                 | File system watching     | anvil-kernel                     |
| petgraph               | In-memory semantic graph | anvil-kernel                     |
| serde, serde_json      | Serialisation            | anvil-kernel-types, anvil-kernel |
| ratatui                | Terminal UI framework    | eddacraft-tui, anvil-tui         |
| crossterm              | Terminal backend         | eddacraft-tui, anvil-tui         |
| tokio                  | Async runtime            | anvil-kernel                     |
| insta                  | Snapshot testing         | all crates                       |

### Workspace Policies

- `unsafe` code is **forbidden** (workspace-level lint)
- Clippy set to **deny all** with select pedantic exceptions
- Edition: 2024
- Resolver: 2

---

## KERN — Rust Kernel (25 work items)

The kernel is the centrepiece. It is a persistent, incremental, deterministic
semantic runtime that continuously enforces structural invariants and streams
governance events.

### What It Does

1. **Watches** the file system (notify-rs, <20ms detection latency)
2. **Parses** changed files incrementally (tree-sitter, <1ms per file)
3. **Extracts** symbols (functions, classes, modules, exports)
4. **Builds** a persistent semantic graph (petgraph — SymbolNode + SymbolEdge)
5. **Derives** a dependency graph from import edges
6. **Annotates** nodes with trust levels (Unknown, Internal, Boundary, External,
   Privileged)
7. **Evaluates** structural invariants against graph deltas
8. **Emits** typed events (Progress, Snapshot, Violation, Error) via EngineEvent
   envelope

### Phases

| Phase | Name                   | Items | Status           | Key Deliverables                                                                                  |
| ----- | ---------------------- | ----- | ---------------- | ------------------------------------------------------------------------------------------------- |
| 0     | Spike                  | 5     | Done             | tree-sitter <1ms, notify <20ms, petgraph <500MB, Cargo+pnpm coexistence                           |
| 1     | Watcher + Parser       | 4     | Draft            | File watching with debounce/backpressure, incremental parsing, symbol extraction, ignore patterns |
| 2     | Semantic Graph         | 4     | Draft            | Symbol graph (petgraph), dependency graph, trust metadata, incremental update with GraphDelta     |
| 3     | Policy Engine + Events | 4     | Draft            | Architecture config loader, invariant framework, 4 H1 invariants, event emission                  |
| 4     | Integration            | 5     | Draft            | Embedded mode, watch mode, dual-run harness, benchmarks, cross-compilation                        |
| 5     | Daemon Mode            | 3     | Draft (deferred) | Unix socket transport, JSON-RPC protocol, session management                                      |

### H1 Invariants (KERN-032)

Four structural invariants ship in the first release:

1. **Cross-layer boundary violation** — Module in layer A imports from layer B
   where the dependency is not allowed by the architecture definition
2. **New external dependency introduction** — A new `import` or `require` of a
   package not previously in the dependency graph
3. **Public API surface expansion** — A symbol's visibility increases (e.g.
   internal function becomes exported)
4. **Privilege expansion heuristic** — Code gains access to sensitive APIs
   (network, filesystem, process) that it didn't previously use

### Execution Modes

| Mode              | Description                                 | Entry Point   |
| ----------------- | ------------------------------------------- | ------------- |
| Embedded          | One-shot check, library API, no IPC         | `anvil check` |
| Foreground Watch  | Long-lived process, continuous event stream | `anvil watch` |
| Daemon (deferred) | Multi-client, Unix socket, JSON-RPC         | `anvild`      |

### Performance Targets

| Metric                           | Target     |
| -------------------------------- | ---------- |
| Cold graph build (100k LOC)      | <3 seconds |
| Incremental update (single file) | <100ms     |
| Event emission overhead          | <10ms      |
| Memory footprint (medium repo)   | <500MB     |
| File detection latency (p99)     | <20ms      |
| tree-sitter parse (single file)  | <1ms       |

---

## RENG — Rust Engine Ports (6 work items)

Ports existing TypeScript checks to Rust for speed. These checks already work in
JS — the goal is identical results at 10-40x the speed.

### Checks Being Ported

| Check                  | Current (Node.js) | Target (Rust) | Speedup | AST Needed           | Depends On                               |
| ---------------------- | ----------------- | ------------- | ------- | -------------------- | ---------------------------------------- |
| Secret scan            | 200-800ms         | 5-20ms        | 40x     | No (regex + entropy) | Nothing (self-contained)                 |
| Anti-pattern detection | 500-2000ms        | 20-100ms      | 25x     | Yes (tree-sitter)    | KERN-011                                 |
| Command safety         | 100-500ms         | 5-20ms        | 25x     | No (string analysis) | Nothing (self-contained)                 |
| Architecture check     | 500-2000ms        | 20-100ms      | 25x     | Yes (graph)          | KERN-032 (merged into kernel invariants) |

### Work Items

| ID       | Title                              | Status | Dependencies       |
| -------- | ---------------------------------- | ------ | ------------------ |
| RENG-001 | Port secret scan                   | Draft  | None               |
| RENG-002 | Port anti-pattern detection        | Draft  | KERN-011           |
| RENG-003 | Port command safety check          | Draft  | None               |
| RENG-004 | Validate architecture check parity | Draft  | KERN-032           |
| RENG-005 | Benchmark all ported checks vs JS  | Draft  | RENG-001–003       |
| RENG-006 | Feature flag + dual-run mode       | Draft  | RENG-005, KERN-042 |

### Rollout Strategy

The `--engine` CLI flag controls which engine runs:

- `--engine legacy` — JS only (current behaviour)
- `--engine rust` — Rust only
- `--engine dual` — Both engines run, results are diffed for parity validation

Each check is independently feature-flagged. Secret scan and command safety can
be ported immediately (no AST dependency). Anti-pattern detection waits for KERN
Phase 1 (tree-sitter). Architecture check merges into the kernel's invariant
framework rather than being a separate port.

---

## RATS — Ratatui TUI (7 work items)

New TUI surfaces built on Ratatui, consuming kernel events in-process.

### Phases

| Phase | Name              | Items | Status          |
| ----- | ----------------- | ----- | --------------- |
| 1     | Shared Components | 1     | Done (RATS-001) |
| 2     | Core Surfaces     | 3     | Draft           |
| 3     | Integration       | 3     | Draft           |

### Key Surfaces

| ID       | Surface                        | Description                                                          | Key Dependencies             |
| -------- | ------------------------------ | -------------------------------------------------------------------- | ---------------------------- |
| RATS-001 | eddacraft-tui shared crate     | Theme, keyboard, widgets (Select, TextInput, ProgressBar, StatusBar) | None — **Done**              |
| RATS-002 | Watch dashboard                | Live gate results, file status, violations — 4-panel layout          | PORT-030, KERN-033           |
| RATS-003 | Gate result viewer             | Interactive violation browser with detail panes                      | PORT-023, KERN-040           |
| RATS-004 | APS onboarding wizard          | Multi-step project init wizard                                       | RATS-001                     |
| RATS-005 | Ink-to-Ratatui migration path  | `--tui=ink` / `--tui=ratatui` flag, feature flags per surface        | RATS-002, PORT-023, PORT-030 |
| RATS-006 | Terminal compatibility testing | Cross-terminal validation (iTerm2, WezTerm, GNOME, Windows Terminal) | RATS-001, RATS-002           |
| RATS-007 | `anvil watch` TUI entry point  | Wire Ratatui dashboard into `anvil` binary                           | RATS-002, KERN-041           |

### Design Constraints

- Dark-only theme, EddaCraft 5-colour palette
- Keyboard: j/k navigate, space/enter select, esc back, q quit
- Minimum terminal size: 80x24
- TUI render must not block kernel event processing

---

## PORT — Ink-to-Ratatui Port (15 work items)

Systematic 1:1 port of every existing Ink (React) TUI surface to Ratatui. PORT
produces surfaces with mock/static data; RATS wires them to live kernel events.

### Phases

| Phase | Name              | Items | Surfaces                                                                                                                |
| ----- | ----------------- | ----- | ----------------------------------------------------------------------------------------------------------------------- |
| 1     | Shared Components | 2     | Header, Container, Divider, Spinner, StatusBadge, Confirm, LogPanel, ParallelProgress, QuickWinsPanel, ResultsDashboard |
| 2     | Simple Surfaces   | 3     | Welcome, Doctor, Status Dashboard                                                                                       |
| 3     | Medium Surfaces   | 4     | Init Wizard, Audit Results, Template Browser, Gate Explorer                                                             |
| 4     | Complex Surfaces  | 6     | Watch Dashboard, Tutorial Orchestrator, 4 Tutorial Paths (Policy, Architecture, Drift, CI)                              |

### Inventory of Ink Surfaces Being Ported

| Surface            | Complexity  | Service Dependencies    | PORT ID      |
| ------------------ | ----------- | ----------------------- | ------------ |
| Welcome            | Low         | None (static)           | PORT-010     |
| Doctor             | Low-Medium  | Props only              | PORT-011     |
| Status             | Medium      | Props only              | PORT-012     |
| Init Wizard        | Medium      | 5-step wizard           | PORT-020     |
| Audit Results      | Medium      | repo-scanner service    | PORT-021     |
| Template Browser   | Medium      | template-loader service | PORT-022     |
| Gate Explorer      | Medium-High | Props only              | PORT-023     |
| Watch Dashboard    | High        | Imperative handle API   | PORT-030     |
| Tutorial (4 paths) | High        | 23 step components      | PORT-040–044 |

### RATS/PORT Coordination

| PORT Task        | RATS Task | PORT Scope                     | RATS Scope              |
| ---------------- | --------- | ------------------------------ | ----------------------- |
| PORT-023 (Gate)  | RATS-003  | Port Ink layout with mock data | Wire live kernel events |
| PORT-030 (Watch) | RATS-002  | Port Ink layout with mock data | Wire live kernel events |

PORT-020 (init wizard port) and RATS-004 (APS onboarding wizard) are
**independent** — different surfaces for different flows.

---

## RSTLAN — Rust Language Support (Placeholder)

Extends Anvil's analysis to Rust codebases. Currently a placeholder — tasks will
be defined when it moves to Ready.

### Planned Capabilities

- Import extraction: `use`, `mod`, `pub use`, `extern crate`
- Crate-relative path resolution (`crate::`, `super::`, `self::`)
- Entry point detection: `fn main()`, `Cargo.toml` bin targets
- Module boundary detection via `mod` declarations and `pub` visibility

### Rust-Specific Anti-Patterns (6 planned)

| Pattern                                      | Concern                  |
| -------------------------------------------- | ------------------------ |
| `unsafe` blocks                              | Deliberate safety bypass |
| `#[allow(...)]` directives                   | Linter suppression       |
| `unwrap()` / `expect()` in non-test code     | Panic risk               |
| `todo!()` / `unimplemented!()` in production | Incomplete code          |
| `as` type casts                              | Potential data loss      |
| `#[cfg(test)]` outside test modules          | Test leakage             |

---

## Dependency Graph Between Modules

```
                    KERN (Kernel)
                   /      |      \
                  /       |       \
           KERN Ph1    KERN Ph2    KERN Ph3
           (parse)     (graph)     (events)
              |           |           |
              v           v           v
          RENG-002    RENG-004    RATS-002
          RENG-001*   (merged)    RATS-003
          RENG-003*               RATS-007
              |                      |
              v                      v
          RENG-005               PORT-023
          RENG-006               PORT-030
                                     |
                                     v
                                 RATS-005
                              (migration path)

  * = self-contained, no KERN dependency
```

### Critical Path

The longest dependency chain is:

```
KERN-011 (parser) -> KERN-012 (symbols) -> KERN-020 (graph) -> KERN-023 (incremental)
-> KERN-031 (policy engine) -> KERN-032 (invariants) -> KERN-033 (events)
-> KERN-041 (watch mode) -> RATS-007 (TUI entry point)
```

### Parallelisable Work

These can start immediately without waiting for KERN:

| Work Item                        | Why                                    |
| -------------------------------- | -------------------------------------- |
| RENG-001 (secret scan)           | Self-contained regex + entropy, no AST |
| RENG-003 (command safety)        | Self-contained string analysis         |
| PORT-001–002 (shared components) | Uses eddacraft-tui, mock data          |
| PORT-010–012 (simple surfaces)   | Static/props-only data                 |
| RATS-004 (APS wizard)            | Only needs eddacraft-tui widgets       |

---

## Migration Strategy

### Phase 1: Coexistence

- Rust binary ships alongside Node.js CLI
- `--engine` flag selects which engine runs checks
- Dual-run mode validates parity
- Ink TUI remains default

### Phase 2: Validation

- All ported checks validated via dual-run diffing
- Ratatui surfaces available via `--tui=ratatui` flag
- Performance benchmarks confirm speedup targets

### Phase 3: Cutover

- Rust becomes default engine
- Ratatui becomes default TUI
- `--engine legacy` and `--tui=ink` remain as fallbacks
- Node.js dependency eventually removed (long-term)

---

## Known Issues and Open Questions

These items were originally captured in an internal "rust-branches-review" note
dated 2026-03-05 (not included in this repository):

### Resolved by Plan Evolution

- ADR-011's N-API approach is **superseded** by KERN's standalone binary
  approach
- Crate naming has been consolidated in the workspace Cargo.toml
- The old single-module RENG plan is superseded by the KERN/RENG/RATS split

### Still Open

- Watch cycle speedup claimed as 14x in ADR but calculated as 8.5x — needs
  reconciliation
- CI action versions are unpinned in workflow files
- Spike crate has zero tests (acceptable for spikes, but should not carry
  forward)
- `insta` is a workspace dependency but unused in any test currently
- MermaidDiagram port is likely infeasible in terminal — ASCII fallback needed

---

## Total Work Item Count

| Module                         | Items   | Status                   |
| ------------------------------ | ------- | ------------------------ |
| KERN (Rust Kernel)             | 25      | Phase 0 Done, rest Draft |
| RENG (Engine Ports)            | 6       | Draft                    |
| RATS (Ratatui TUI)             | 7       | 1 Done, rest Draft       |
| PORT (Ink-to-Ratatui Port)     | 15      | Draft                    |
| RSTLAN (Rust Language Support) | ~5      | Placeholder              |
| **Total**                      | **~58** | —                        |
