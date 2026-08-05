# Rust Architecture — Full Overview

| Type  | Authority | Owner | Status | Freshness                                                                                                                                                                                                                                                               |
| ----- | --------- | ----- | ------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Guide | Derived   | KERN  | Live   | Module-map status refresh 2026-08-05 (KERN/RENG/RATS/PORT/RSTLAN all Complete/archived). Crate-layout + dependency tables regenerated 2026-07-02 against main `d1fded280` (35 workspace crates, 15-language registry). Metadata backfilled 2026-05-27 during DOCGOV-011 |

| Upstream                                                                     | Downstream                                 |
| ---------------------------------------------------------------------------- | ------------------------------------------ |
| Archived KERN, RENG, RATS, PORT, RSTLAN modules; live crates under `crates/` | Rust architecture docs and onboarding docs |

> Compiled from APS modules KERN, RENG, RATS, PORT, RSTLAN (all Complete and
> archived under `plans/archive/modules/`), the superseded TUI module, and
> supporting architecture documents. This is a crate-layout reference — not a
> plan. Shipping component detail lives in the matching `*-as-built.md` docs.

## Executive Summary

The Rust architecture has replaced Anvil's Node.js-based analysis engine with a
standalone Rust binary that provides 10-40x performance improvements. The
`anvil` binary watches files, builds a semantic graph, evaluates policies, and
renders a terminal UI — all in one process with zero IPC overhead. It is
distributed as a single static binary via cargo-dist for all six platform
targets.

## Module Map

Six APS modules covered the original Rust cutover. All six are **Complete** and
archived; the map below is provenance plus the live crate graph.

```
KERN (Rust Kernel)          — Complete. Watcher, parser, graph, policy engine.
  |
  +-- RENG (Engine Ports)   — Complete. Port existing TS checks to Rust.
  |
  +-- RATS (Ratatui TUI)    — Complete. New TUI surfaces consuming KERN events.
  |     |
  |     +-- PORT (Ink Port) — Complete. 1:1 port of existing Ink surfaces to Ratatui.
  |
  +-- RSTLAN (Rust Lang)    — Complete (Released/Shipped via v0.8.0-beta). Rust language support.

TUI (superseded)            — Original OpenTUI/Ink approach, replaced by RATS.
```

## Crate Layout

### Workspace Crates

The workspace root `Cargo.toml` uses an explicit `members` list (35 crates as of
2026-07-02, edition 2024, `unsafe_code = "forbid"`). Grouped by role:

```
crates/
  # ── Kernel substrate (KERN) ──────────────────────────────
  anvil-kernel/                     # KERN — watcher, parser, policy engine, event protocol
    src/
      watcher/                      # notify-rs integration (KERN-010, KERN-013)
      parser/                       # tree-sitter integration, 15-language registry (KERN-011/012)
        languages.rs                #   Language enum: TS/TSX/JS/JSX/Rust/Python/Dart/Go/Java/
                                    #   Kotlin/C#/C/C++/Zig/Wat + grammar_version cache key
        extract/                    #   LanguageExtractor dispatch + per-language extractors
      policy/                       # Policy engine + 4 H1 invariants (KERN-030..032)
      protocol/emitter.rs           # Event emission (KERN-033)
      embedded.rs                   # One-shot library API (KERN-040)
      watch.rs                      # Foreground watch mode (KERN-041)
    benches/kernel.rs               # criterion benchmarks (KERN-043)
  anvil-kernel-types/               # Shared wire types: events, graph nodes, trust, diagnostics
  anvil-graph-cache/                # Parser-free semantic graph + save-time cache (ADR-064):
                                    #   SymbolGraph/DependencyGraph, incremental, trust, certify,
                                    #   hot_index, call_graph, registry, snapshot, tokens
  anvil-grammar-wat/                # Vendored WebAssembly-text tree-sitter grammar (LTW2-002, ADR-093)
  anvil-rayon-init/                 # Shared rayon global-pool cap (half cores, VS Code coexistence)

  # ── CLI + surfaces ───────────────────────────────────────
  anvil-cli/                        # CLI binary — primary entry point (clap + Ratatui)
  anvil-tui/                        # RATS + PORT — all TUI surfaces (welcome/doctor/status/init/
                                    #   audit/browser/gate/watch/wizard/tutorial)
  eddacraft-tui/                    # Shared Ratatui component library (theme, keyboard, widgets; ADR-047)

  # ── Checks + policy ──────────────────────────────────────
  anvil-checks/                     # RENG — ported gate checks (secret, antipattern, AI-001, command safety)
  anvil-checks-ast/                 # AST-aware anti-pattern detection, gate-time only (ADR-071)
  anvil-checks-napi/                # Node bindings build canary for anvil-checks (ADR-033)
  anvil-policy/                     # Policy engine — evaluation, library loading, lifecycle
  anvil-policy-engine/              # Policy engine facade over regorus (ADR-040)
  anvil-l4/                         # L4 policy framework — anvil/policy.yml, per-branch matching (MLP-006)
  anvil-rules/                      # Rule-set hashing + version-floor primitives (MLP-012)
  anvil-architecture/               # Architecture enforcement (boundaries, import rules, drift)

  # ── Intercept daemon (INTD / RTAI) ───────────────────────
  anvil-intercept/                  # INTD — mid-edit / save-time intercept daemon
  anvil-intercept-proto/            # Wire-protocol types shared with the daemon
  anvil-intercept-rules/            # Rule set evaluated by the daemon
  anvil-intercept-macos/            # macOS-only intercept helpers
  anvil-intercept-win32/            # Windows-specific intercept transport bits
  anvil-run/                        # Wrapped-launch ingress for the Anvil Intercept Loop (INTL)

  # ── GCTX egress ──────────────────────────────────────────
  anvil-gctx-types/                 # Sealed graph-free GCTX egress value types (ADR-084)
  anvil-gctx-egress/                # Daemon-side GCTX projector — single CE-5 choke point (ADR-084)

  # ── Governance + supporting primitives ───────────────────
  anvil-baseline/                   # Baseline store — anvil/baseline.json + move-resistant fingerprint (MLP-007)
  anvil-witness/                    # Hash-chained ndjson witness chain, flock-serialised (MLP-002)
  anvil-capsule/                    # Review Capsule v0 — anvil.capsule.v1 manifest + schema (ADR-074)
  anvil-sarif/                      # Shared SARIF 2.1.0 emitter (ADR-058)
  anvil-config/                     # Multi-format config loader: yaml/json/toml (MLP-011)
  anvil-hook/                       # Hook surface primitives — framework detection, templates (MLP-003)
  anvil-attribution/                # Agent-attribution: env propagation + process-tree walk (MLP-014)
  anvil-observability/              # TRACE — tracing baseline, traceparent envelope, redaction
  anvil-bench/                      # Stress-test harness and benchmarks
  spike/                            # Phase 0 validation spikes (done)
  workspace-hack/                   # Hakari-managed feature unifier (build-time only)
```

### Workspace membership

`eddacraft-tui` is a **workspace member** at `crates/eddacraft-tui`
(`Cargo.toml` `members` + `eddacraft-tui = { path = "crates/eddacraft-tui" }`),
consumed by path per ADR-047 — not an external git dependency. It ships from
this repo alongside `anvil-tui`.

### Workspace Dependencies

| Dependency                                                         | Purpose                           | Used By                          |
| ------------------------------------------------------------------ | --------------------------------- | -------------------------------- |
| tree-sitter                                                        | Incremental parsing               | anvil-kernel                     |
| tree-sitter-typescript                                             | TS / TSX grammar                  | anvil-kernel                     |
| tree-sitter-javascript                                             | JS / JSX grammar                  | anvil-kernel                     |
| tree-sitter-{rust,python,dart,go,java,kotlin-ng,c-sharp,c,cpp,zig} | 11 tail-language grammars         | anvil-kernel                     |
| anvil-grammar-wat                                                  | Vendored WebAssembly-text grammar | anvil-kernel                     |
| notify                                                             | File system watching              | anvil-kernel                     |
| petgraph                                                           | In-memory semantic graph          | anvil-graph-cache                |
| rayon                                                              | Parallel parse (cold start)       | anvil-kernel, anvil-checks       |
| anvil-rayon-init                                                   | Shared half-cores pool cap        | anvil-kernel, anvil-checks       |
| serde, serde_json                                                  | Serialisation                     | anvil-kernel-types, anvil-kernel |
| ratatui                                                            | Terminal UI framework             | eddacraft-tui, anvil-tui         |
| crossterm                                                          | Terminal backend                  | eddacraft-tui, anvil-tui         |
| regorus                                                            | Rego policy evaluation            | anvil-policy-engine (ADR-040)    |
| insta                                                              | Snapshot testing                  | all crates                       |

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

| Module                         | Items   | Done   | Status                                                                |
| ------------------------------ | ------- | ------ | --------------------------------------------------------------------- |
| KERN (Rust Kernel)             | 25      | 22     | **Complete** (archived; KERN-050..052 superseded by INTD per ADR-030) |
| RENG (Engine Ports)            | 6       | 6      | **Complete**                                                          |
| RATS (Ratatui TUI)             | 7       | 7      | **Complete**                                                          |
| PORT (Ink-to-Ratatui Port)     | 15      | 15     | **Complete**                                                          |
| RSTLAN (Rust Language Support) | ~5      | 0      | Placeholder                                                           |
| **Total**                      | **~58** | **50** | —                                                                     |
