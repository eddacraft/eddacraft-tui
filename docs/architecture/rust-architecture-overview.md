# Rust Architecture — Full Overview

| Type  | Authority | Owner | Status | Freshness                                        |
| ----- | --------- | ----- | ------ | ------------------------------------------------ |
| Guide | Derived   | KERN  | Live   | Metadata backfilled 2026-05-24 during DOCGOV-009 |

| Upstream                                        | Downstream                                 |
| ----------------------------------------------- | ------------------------------------------ |
| KERN, RENG, RATS, PORT, RSTLAN, and TUI modules | Rust architecture docs and onboarding docs |

> Compiled from APS modules KERN, RENG, RATS, PORT, RSTLAN, TUI (superseded),
> and supporting architecture documents. This is a reference document — not a
> plan itself.

## Executive Summary

The Rust architecture has replaced Anvil's Node.js-based analysis engine with a
standalone Rust binary that provides 10-40x performance improvements. The
`anvil` binary watches files, builds a semantic graph, evaluates policies, and
renders a terminal UI — all in one process with zero IPC overhead. It is
distributed as a single static binary via cargo-dist for all six platform
targets.

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

### Workspace Crates

```
Cargo.toml                          # Workspace root (edition 2024, unsafe_code = "forbid")
crates/
  anvil-cli/                        # CLI binary — primary entry point (clap + Ratatui)
  anvil-kernel/                     # KERN — watcher, parser, semantic graph, policy engine
    src/
      watcher/                      # notify-rs integration (KERN-010, KERN-013)
      parser/                       # tree-sitter integration (KERN-011, KERN-012)
      graph/                        # Semantic graph (KERN-020..023)
        symbol_graph.rs
        dependency.rs
        trust.rs
        incremental.rs
      policy/                       # Policy engine (KERN-030..032)
        config.rs
        engine.rs
        invariants/
      protocol/                     # Event emission (KERN-033)
        emitter.rs
      embedded.rs                   # One-shot library API (KERN-040)
      watch.rs                      # Foreground watch mode (KERN-041)
    tests/
      dual_run.rs                   # Parity harness vs TS engine (KERN-042)
    benches/
      kernel.rs                     # criterion.rs benchmarks (KERN-043)
  anvil-kernel-types/               # Shared types: events, graph nodes, trust levels
  anvil-tui/                        # RATS + PORT — all TUI surfaces (complete)
    src/
      surfaces/
        welcome/                    # PORT-010
        doctor/                     # PORT-011
        status/                     # PORT-012
        init/                       # PORT-020
        audit/                      # PORT-021
        browser/                    # PORT-022
        gate/                       # PORT-023 + RATS-003
        watch/                      # PORT-030 + RATS-002
        wizard/                     # RATS-004
        tutorial/                   # PORT-040–044
  anvil-checks/                     # RENG — ported gate checks (secret, antipattern, AI-001, SURFENV-001, command safety)
  anvil-checks-napi/                # Node bindings build canary for anvil-checks (ADR-033)
  anvil-intercept/                  # INTD — mid-edit intercept daemon (RTAI launch path)
  anvil-intercept-proto/            # Wire-protocol types shared with the intercept daemon
  anvil-intercept-rules/            # Rule set evaluated by the intercept daemon
  anvil-intercept-win32/            # Windows-specific intercept transport bits
  anvil-observability/              # TRACE — tracing baseline, traceparent envelope, redaction
  anvil-policy/                     # OPA policy evaluation engine
  anvil-architecture/               # Architecture enforcement (boundaries, drift)
  anvil-bench/                      # Stress-test harness and benchmarks
  spike/                            # Phase 0 validation spikes (done)
  workspace-hack/                   # Hakari-managed feature unifier (build-time only)
```

### External Dependencies

`eddacraft-tui` (shared Ratatui component library — theme, keyboard, widgets) is
an external git dependency, not part of the workspace.

### Workspace Dependencies

| Dependency             | Purpose                        | Used By                          |
| ---------------------- | ------------------------------ | -------------------------------- |
| tree-sitter            | Incremental parsing            | anvil-kernel                     |
| tree-sitter-typescript | TS/JS grammar                  | anvil-kernel                     |
| tree-sitter-javascript | JS grammar                     | anvil-kernel                     |
| notify                 | File system watching           | anvil-kernel                     |
| petgraph               | In-memory semantic graph       | anvil-kernel                     |
| rayon                  | Parallel parse (cold start)    | anvil-kernel                     |
| num_cpus               | Core count for thread pool cap | anvil-kernel                     |
| serde, serde_json      | Serialisation                  | anvil-kernel-types, anvil-kernel |
| ratatui                | Terminal UI framework          | eddacraft-tui, anvil-tui         |
| crossterm              | Terminal backend               | eddacraft-tui, anvil-tui         |
| tokio                  | Async runtime                  | anvil-kernel                     |
| insta                  | Snapshot testing               | all crates                       |

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

| Phase | Name                   | Items | Status   | Key Deliverables                                                                                  |
| ----- | ---------------------- | ----- | -------- | ------------------------------------------------------------------------------------------------- |
| 0     | Spike                  | 5     | **Done** | tree-sitter <1ms, notify <20ms, petgraph <500MB, Cargo+pnpm coexistence                           |
| 1     | Watcher + Parser       | 4     | **Done** | File watching with debounce/backpressure, incremental parsing, symbol extraction, ignore patterns |
| 2     | Semantic Graph         | 4     | **Done** | Symbol graph (petgraph), dependency graph, trust metadata, incremental update with GraphDelta     |
| 3     | Policy Engine + Events | 4     | **Done** | Architecture config loader, invariant framework, 4 H1 invariants, event emission                  |
| 4     | Integration            | 5     | **Done** | Embedded mode, watch mode, dual-run harness, benchmarks, cross-compilation                        |
| 5     | Daemon Mode            | 3     | Deferred | Unix socket transport, JSON-RPC protocol, session management                                      |

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

_Targets updated 2026-04-03 to reflect rayon parallel parse implementation (PR
#746)._

| Metric                           | Target     | Actual (benchmarked)      |
| -------------------------------- | ---------- | ------------------------- |
| Cold graph build (100 files)     | <3 seconds | **14.5 ms** (rayon)       |
| Cold graph build (1,000 files)   | <3 seconds | **~565 ms** (estimated)   |
| Incremental update (single file) | <100ms     | **10 µs**                 |
| Policy evaluation (all H1)       | <10ms      | **799 ns**                |
| Event emission (1,000 events)    | <10ms      | **408 µs**                |
| Memory footprint (medium repo)   | <500MB     | Not yet measured at scale |
| File detection latency (p99)     | <20ms      | Unchanged (notify-rs)     |
| tree-sitter parse (single file)  | <1ms       | **< 1ms** ✓               |
| Concurrent burst (10 files)      | —          | **693 µs** (rayon)        |
| Concurrent burst (50 files)      | —          | **3.5 ms** (rayon)        |

#### Parallelism Architecture

The parse phase of both `run_embedded` (CLI one-shot) and `initial_scan` (watch
mode) runs in parallel using rayon. The thread pool is capped at
`max(1, cpus/2)` to avoid saturating VS Code extension host and CI runners.
Graph updates remain sequential (`SymbolGraph` requires `&mut`).

Key implementation notes:

- `extract_symbols()` assigns 0-based sequential IDs per file; IDs are rebased
  to be globally unique in the sequential apply phase
- Parse errors surface as `EngineEvent::Error` events — not silently dropped
- Cancellation is supported via
  `run_embedded_cancellable(stop: Arc<AtomicBool>)`
- Stop flag checked inside each rayon closure for responsive shutdown

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

| ID       | Title                              | Status   | Dependencies       |
| -------- | ---------------------------------- | -------- | ------------------ |
| RENG-001 | Port secret scan                   | **Done** | None               |
| RENG-002 | Port anti-pattern detection        | **Done** | KERN-011           |
| RENG-003 | Port command safety check          | **Done** | None               |
| RENG-004 | Validate architecture check parity | **Done** | KERN-032           |
| RENG-005 | Benchmark all ported checks vs JS  | **Done** | RENG-001–003       |
| RENG-006 | Feature flag + dual-run mode       | **Done** | RENG-005, KERN-042 |

All ported checks are shipped in the Rust binary and are the only engine.
Legacy/Dual modes were dropped when the TypeScript engine was retired.

---

## RATS — Ratatui TUI (7 work items)

New TUI surfaces built on Ratatui, consuming kernel events in-process.

### Phases

| Phase | Name              | Items | Status   |
| ----- | ----------------- | ----- | -------- |
| 1     | Shared Components | 1     | **Done** |
| 2     | Core Surfaces     | 3     | **Done** |
| 3     | Integration       | 3     | **Done** |

### Key Surfaces

| ID       | Surface                        | Description                                                          | Status   |
| -------- | ------------------------------ | -------------------------------------------------------------------- | -------- |
| RATS-001 | eddacraft-tui shared crate     | Theme, keyboard, widgets (Select, TextInput, ProgressBar, StatusBar) | **Done** |
| RATS-002 | Watch dashboard                | Live gate results, file status, violations — 4-panel layout          | **Done** |
| RATS-003 | Gate result viewer             | Interactive violation browser with detail panes                      | **Done** |
| RATS-004 | APS onboarding wizard          | Multi-step project init wizard                                       | **Done** |
| RATS-005 | Ink-to-Ratatui migration path  | Ratatui is now the only TUI — Ink removed                            | **Done** |
| RATS-006 | Terminal compatibility testing | Cross-terminal validation (iTerm2, WezTerm, GNOME, Windows Terminal) | **Done** |
| RATS-007 | `anvil watch` TUI entry point  | Wire Ratatui dashboard into `anvil` binary                           | **Done** |

### Design Constraints

- Dark-only theme, eddacraft 5-colour palette
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

## Migration Status

The migration has reached Phase 3 (Cutover, complete) and is now in standalone
mode. The Rust binary is the primary distribution — Node.js is no longer
required to run the CLI.

### Phase 1: Coexistence [DONE]

- Rust binary shipped alongside Node.js CLI
- `--engine` flag selected which engine ran checks
- Dual-run mode validated parity

### Phase 2: Validation [DONE]

- All ported checks validated via dual-run diffing
- Ratatui surfaces completed (RATS 7/7, PORT 15/15)
- Performance benchmarks confirmed speedup targets

### Phase 3: Cutover [DONE]

- Rust is the default (and only) engine
- Ratatui is the default (and only) TUI
- Node.js CLI deprecated; single Rust binary distributed via cargo-dist

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

- CI action versions are unpinned in workflow files
- Spike crate has zero tests (acceptable for spikes, not carried forward)
- MermaidDiagram port is likely infeasible in terminal — ASCII fallback needed

---

## 0.5.0-beta Surface Deliveries

The following surfaces landed during the 0.5.0-beta cycle and are now part of
the shipped Rust architecture:

- **AI-001 reasoning rule** in `anvil-checks` — flags appeal-to-authority
  comments via the registry-backed pattern catalogue, runs only inside comment
  regions, honours `// @anvil-ignore AI-001`, and emits at info severity through
  the shared `Notification` envelope.
- **`anvil-checks` rule registry** — every shipped rule (anti-pattern, secret,
  AI-001, SURFENV-001, command safety) now flows through the compiled `.anvil`
  registry, with rule provenance attached to every finding.
- **Parallel scan rollout** — `gate`, `audit`, `check`, `drift`, policy,
  architecture validation, and the watcher all share the gitignore-aware
  discovery walk plus the rayon scan pattern; first-run scans cap their pool via
  `ANVIL_SCAN_THREADS` (default `min(num_cpus, 4)`).
- **`anvil mcp-config` Rust CLI command** — generates Claude Code, Cursor,
  Windsurf, and VS Code configurations with stdio/http transports, `--write`,
  `--verify`, workspace overrides, path-safety prompts, and atomic writes. Lives
  in `anvil-cli` and reuses the rest of the workspace for path resolution and
  config generation.
- **`validate_write` MCP tool** — exposes save-time and mid-edit validation
  through the MCP server, backed by the canonical `anvil.diagnostic.v1` envelope
  owned by `anvil-kernel-types`.
- **Tracing baseline (`anvil-observability`)** — TRACE-001 lands the
  `anvil-observability` crate with a traceparent envelope, redaction helpers,
  and the cross-cutting namespace registry consumed by `anvil-intercept` and
  `anvil-cli`. Subsequent TRACE work items wire it through the daemon and the TS
  surfaces; the foundation is now part of the shipped Rust architecture.

## Total Work Item Count

| Module                         | Items   | Done   | Status       |
| ------------------------------ | ------- | ------ | ------------ |
| KERN (Rust Kernel)             | 25      | 22     | In Progress  |
| RENG (Engine Ports)            | 6       | 6      | **Complete** |
| RATS (Ratatui TUI)             | 7       | 7      | **Complete** |
| PORT (Ink-to-Ratatui Port)     | 15      | 15     | **Complete** |
| RSTLAN (Rust Language Support) | ~5      | 0      | Placeholder  |
| **Total**                      | **~58** | **50** | —            |
