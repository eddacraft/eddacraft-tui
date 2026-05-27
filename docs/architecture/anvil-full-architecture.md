# Anvil — Full Architecture (Current vs Proposed End State)

| Type | Authority | Owner  | Status | Freshness                                        |
| ---- | --------- | ------ | ------ | ------------------------------------------------ |
| Spec | Derived   | DOCGOV | Live   | Metadata backfilled 2026-05-27 during DOCGOV-011 |

| Upstream                                          | Downstream                                                 |
| ------------------------------------------------- | ---------------------------------------------------------- |
| APS modules, architecture specs, vision documents | Architecture reference docs, public architecture tutorials |

> **Date:** 2026-03-13 **Status:** Reference — synthesised from APS modules,
> architecture specs, and vision documents
>
> Legend: **[CURRENT]** = shipped today, **[PROPOSED]** = planned but not yet
> built, **[PARTIAL]** = foundation exists but incomplete

---

## 1. System-Level View

### Current Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                        SURFACES                             │
│                                                             │
│  ┌──────────────┐  ┌──────────┐  ┌───────────┐  ┌───────┐  │
│  │  anvil-cli   │  │ Website  │  │ MCP Server│  │VS Code│  │
│  │(Rust+Ratatui)│  │ (Next.js)│  │   (TS)    │  │  Ext  │  │
│  └──────┬───────┘  └────┬─────┘  └─────┬─────┘  └───┬───┘  │
└─────────┼───────────────┼──────────────┼────────────┼───────┘
          │               │              │            │
          ▼               ▼              ▼            ▼
┌─────────────────────────────────────────────────────────────┐
│                    RUNTIME / ENGINE (TS)                     │
│                                                             │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌────────────┐  │
│  │   Gate    │  │  Watch   │  │  Cache   │  │Concurrency │  │
│  │  Runner   │  │Orchestr. │  │ Provider │  │  Manager   │  │
│  └──────────┘  └──────────┘  └──────────┘  └────────────┘  │
│  ┌──────────────────────────────────────────────────────┐   │
│  │                    Gate Checks                       │   │
│  │  Secret │ Anti-pattern │ Architecture │ Dependency   │   │
│  │  ESLint │ Coverage     │ Command Safety│ Policy(OPA) │   │
│  └──────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
          │               │              │
          ▼               ▼              ▼
┌─────────────────────────────────────────────────────────────┐
│                     CORE LIBRARIES (TS)                      │
│                                                             │
│  Config │ Architecture │ Antipattern │ Drift │ Provenance   │
│  Contracts │ Suppression │ Warnings │ Crypto │ Validation   │
└─────────────────────────────────────────────────────────────┘
          │               │              │
          ▼               ▼              ▼
┌─────────────────────────────────────────────────────────────┐
│                     PLATFORM LAYER (TS)                      │
│                                                             │
│  Config Loader │ Storage │ Crypto                            │
└─────────────────────────────────────────────────────────────┘
          │
          ▼
┌─────────────────────────────────────────────────────────────┐
│                EXTERNAL TOOLS & FILE SYSTEM                  │
│                                                             │
│  dependency-cruiser │ OPA │ ESLint │ Git │ Repository FS    │
└─────────────────────────────────────────────────────────────┘
```

### Proposed End State Architecture (H2)

```
┌─────────────────────────────────────────────────────────────┐
│                    SURFACES [PROPOSED]                       │
│                                                             │
│  ┌──────────────┐  ┌──────────┐  ┌───────────┐  ┌───────┐  │
│  │  Rust CLI +  │  │ Website  │  │ MCP Server│  │VS Code│  │
│  │ Ratatui TUI  │  │ (Next.js)│  │   (TS)    │  │  Ext  │  │
│  │  [PROPOSED]  │  │[CURRENT] │  │ [CURRENT] │  │[CURR] │  │
│  └──────┬───────┘  └────┬─────┘  └─────┬─────┘  └───┬───┘  │
└─────────┼───────────────┼──────────────┼────────────┼───────┘
          │               │              │            │
          ▼               ▼              ▼            ▼
┌─────────────────────────────────────────────────────────────┐
│              ENGINE EVENT PROTOCOL [PROPOSED]                │
│                                                             │
│  EngineEvent { event_type, seq, timestamp, engine, payload }  │
│  Transport: JSON-RPC 2.0 / NDJSON / Unix domain socket      │
│  Events: Progress | Snapshot | Violation | Error             │
└──────────┬──────────────────────────────────┬───────────────┘
           │                                  │
     ┌─────▼──────┐                   ┌───────▼──────┐
     │ Legacy TS   │                   │  Rust Kernel │
     │  Engine     │                   │   Engine     │
     │ [CURRENT]   │                   │  [PROPOSED]  │
     │             │                   │              │
     │ gate-runner │                   │  Watcher     │
     │ watch orch. │                   │  Parser      │
     │ dep-cruiser │                   │  Sem. Graph  │
     │ OPA/ESLint  │                   │  Policy Eng. │
     └─────┬───────┘                   └──────┬───────┘
           │                                  │
           ▼                                  ▼
┌─────────────────────────────────────────────────────────────┐
│                   REPOSITORY / GIT / FS                      │
└─────────────────────────────────────────────────────────────┘
```

---

## 2. Monorepo Package Map

### Applications

| Package            | Purpose                                | Status        |
| ------------------ | -------------------------------------- | ------------- |
| `crates/anvil-cli` | CLI + Ratatui TUI (Rust, 20+ commands) | **[CURRENT]** |
| `apps/website`     | Next.js dashboard + marketing          | **[CURRENT]** |
| `apps/anvil-api`   | API server                             | **[CURRENT]** |
| `apps/docs-site`   | Documentation site                     | **[CURRENT]** |
| `apps/e2e`         | End-to-end test suite                  | **[CURRENT]** |

### Core Libraries

| Package                    | Purpose                                                                                                      | Status        |
| -------------------------- | ------------------------------------------------------------------------------------------------------------ | ------------- |
| `packages/anvil/core`      | Core domain: config, architecture, antipattern, drift, provenance, suppression, warnings, crypto, validation | **[CURRENT]** |
| `packages/anvil/runtime`   | Execution engine: gate runner, checks, watch orchestrator, cache, concurrency, storage, export               | **[CURRENT]** |
| `packages/anvil/contracts` | Shared TypeScript contracts/interfaces                                                                       | **[CURRENT]** |
| `packages/anvil/ports`     | Port interfaces (hexagonal architecture)                                                                     | **[CURRENT]** |
| `packages/anvil/policy`    | Policy evaluation abstractions                                                                               | **[CURRENT]** |

### Edda Stack (Observation → Memory Pipeline)

| Package                         | Purpose                                     | Status        |
| ------------------------------- | ------------------------------------------- | ------------- |
| `packages/edda-stack`           | Three-layer memory: Kindling → Ember → Edda | **[CURRENT]** |
| `packages/kindling-integration` | Kindling observation capture integration    | **[CURRENT]** |

### Shared and Support Packages

| Package                            | Purpose                            | Status        |
| ---------------------------------- | ---------------------------------- | ------------- |
| `packages/shared/`                 | Shared cross-cutting utilities     | **[CURRENT]** |
| `packages/shared/storage/`         | Shared storage helpers             | **[CURRENT]** |
| `packages/shared/admin-contracts/` | Shared admin API schemas and types | **[CURRENT]** |
| `packages/libs/render/`            | Shared render-layer utilities      | **[CURRENT]** |

### Tooling & Integration

| Package                          | Purpose                                        | Status        |
| -------------------------------- | ---------------------------------------------- | ------------- |
| `packages/aps`                   | APS (Anvil Plan Spec) loader and state machine | **[CURRENT]** |
| `packages/mcp-server`            | Model Context Protocol server for AI agents    | **[CURRENT]** |
| `packages/eslint-plugin-anvil`   | ESLint plugin with Anvil-specific rules        | **[CURRENT]** |
| `packages/vscode-extension`      | VS Code extension for Anvil                    | **[CURRENT]** |
| `packages/adapters`              | External tool adapters                         | **[CURRENT]** |
| `packages/tooling/eslint-config` | Shared ESLint config                           | **[CURRENT]** |
| `packages/tooling/tsconfig`      | Shared TypeScript config                       | **[CURRENT]** |

### Rust Crates

| Crate                       | Purpose                                                   | Status         |
| --------------------------- | --------------------------------------------------------- | -------------- |
| `crates/anvil-cli`          | CLI binary (clap + Ratatui) — primary entry point         | **[CURRENT]**  |
| `crates/anvil-kernel`       | Core kernel: watcher, parser, graph, policy engine        | **[CURRENT]**  |
| `crates/anvil-kernel-types` | Shared event/graph/trust type contracts                   | **[CURRENT]**  |
| `crates/anvil-tui`          | Anvil-specific TUI surfaces (all ported)                  | **[CURRENT]**  |
| `crates/anvil-checks`       | Ported checks: secret, antipattern, command safety        | **[CURRENT]**  |
| `crates/anvil-policy`       | OPA policy evaluation engine                              | **[CURRENT]**  |
| `crates/anvil-architecture` | Architecture enforcement (boundaries, drift)              | **[CURRENT]**  |
| `crates/anvil-bench`        | Stress-test harness and benchmarks                        | **[CURRENT]**  |
| `crates/spike`              | Phase 0 validation spikes (tree-sitter, notify, petgraph) | **[CURRENT]**  |
| `eddacraft-tui` (external)  | Shared Ratatui component library (git dependency)         | **[EXTERNAL]** |
| `crates/eddacraft-kindling` | Kindling Rust integration                                 | **[PROPOSED]** |

> **Note:** `eddacraft-tui` is an external git dependency, not part of the Cargo
> workspace. `anvil-napi` (N-API bridge) was superseded by the standalone Rust
> binary approach — the CLI is distributed directly via cargo-dist. Watcher,
> gate, and engine responsibilities are consolidated into `anvil-kernel` as
> internal modules.

---

## 3. Gate / Check System

The gate is Anvil's core enforcement mechanism — it runs a configurable set of
checks against repository state and produces pass/fail/warn results.

### Gate Checks

| Check                | What It Does                                      | Engine     | Status                         |
| -------------------- | ------------------------------------------------- | ---------- | ------------------------------ |
| `SecretCheck`        | Entropy + pattern-based secret detection          | Rust       | **Done** (RENG-001)            |
| `AntipatternCheck`   | Anti-patterns (18 registry rules, rayon-parallel) | Rust       | **Done** (RENG-002, RSCAN-008) |
| `CommandSafetyCheck` | Validates shell commands (36 rules)               | Rust       | **Done** (RENG-003)            |
| `ArchitectureCheck`  | Layer violations via dependency analysis          | Rust       | **Done** (RENG-004)            |
| `PolicyCheck`        | OPA Rego policy evaluation                        | Rust       | **Done** (KERN-031)            |
| `DependencyCheck`    | New/changed dependency detection                  | TypeScript | Current                        |
| `ESLintCheck`        | ESLint rule violations                            | TypeScript | Stays TS                       |
| `CoverageCheck`      | Test coverage thresholds                          | TypeScript | Stays TS                       |

### Gate Pipeline Flow

```
                              [CURRENT]
                                 │
  .anvil/gate.yaml ──► GateConfigManager ──► GateRunner
                                                │
                          ┌─────────────────────┼─────────────────────┐
                          ▼                     ▼                     ▼
                    SecretCheck          ArchitectureCheck       PolicyCheck
                    AntipatternCheck     DependencyCheck         ESLintCheck
                    CommandSafetyCheck   CoverageCheck
                          │                     │                     │
                          ▼                     ▼                     ▼
                    SuppressionService ──► Merge Results ──► GateRunResult
                                                │
                              ┌─────────────────┼──────────────┐
                              ▼                 ▼              ▼
                         Provenance         Formatters      Cache
                          Record            (CLI/MCP)      Provider
```

### Current Gate Pipeline

```
                                 │
  .anvil/gate.yaml ──► GateConfigManager ──► Rust Engine / Kernel
                                                │
                                                ▼
                                       ┌──────────────────┐
                                       │ Watcher          │
                                       │ → Parser         │
                                       │ → Semantic Graph │
                                       │ → Policy Engine  │
                                       │ → Event Emission │
                                       └──────────────────┘
                                                │
                                                ▼
                    ┌────────────────────────────────────────────┐
                    │         Engine Event Protocol              │
                    │  Progress | Snapshot | Violation | Error   │
                    └────────────────────────────────────────────┘
                                       │
                              ┌────────┼─────────────┐
                              ▼        ▼             ▼
                           CLI/TUI  Website     Driver clients
                                                 (editor / MCP,
                                                 planned via daemon)
```

---

## 4. Watch System

### Current (TypeScript)

```
File System (chokidar) ──► FileWatcher ──► Debouncer ──► WatchOrchestrator
                                                               │
                                                    ┌──────────┼──────────┐
                                                    ▼          ▼          ▼
                                              GateRunner  GitStatus   Ratatui
                                              (full run)  Tracker    Dashboard
```

- **Latency:** ~2.9s per watch cycle
- **Parser:** None (works on file paths, not ASTs)
- **Graph:** None (dependency-cruiser rebuilds on each run)

### Proposed End State (Rust Kernel)

```
File System (notify-rs) ──► Watcher ──► Debounce/Merge Queue
                                              │
                                    ┌─────────▼──────────┐
                                    │ Incremental Parser  │
                                    │   (tree-sitter)     │
                                    │  AST cache by hash  │
                                    └─────────┬──────────┘
                                              │
                                    ┌─────────▼──────────┐
                                    │  Persistent Graph   │
                                    │  Symbol → Dep →     │
                                    │  Trust (petgraph)   │
                                    │  Incremental update │
                                    └─────────┬──────────┘
                                              │
                                    ┌─────────▼──────────┐
                                    │   Policy Engine     │
                                    │  GraphDelta → check │
                                    │  4 H1 invariants    │
                                    └─────────┬──────────┘
                                              │
                                    ┌─────────▼──────────┐
                                    │  Streaming Events   │
                                    │  via Engine Protocol│
                                    └─────────────────────┘

  Latency: ~200ms per watch cycle (14x faster)
  Parser: tree-sitter (incremental, <1ms per file)
  Graph: persistent petgraph (<500MB, incremental updates)
```

---

## 5. Edda Stack (Memory Architecture)

**Status: [CURRENT]** — all three layers implemented in `packages/edda-stack`

```
┌──────────────────────────────────────────────────────────┐
│                                                          │
│  ┌────────────┐    ┌────────────┐    ┌────────────┐      │
│  │  Kindling   │ ──►│   Ember    │ ──►│   Edda     │      │
│  │  (Camera)   │    │  (Curator) │    │  (Ledger)  │      │
│  │             │    │            │    │            │      │
│  │ Observations│    │ Proposals  │    │ Canonical  │      │
│  │ 11 kinds    │    │ 6 types    │    │ Memories   │      │
│  │ No judgment │    │ TTL decay  │    │ Git-backed │      │
│  │ Structured  │    │ Ephemeral  │    │ Versioned  │      │
│  └────────────┘    └────────────┘    └────────────┘      │
│                                                          │
│  Ports: IKindlingPort │ IEmberPort │ IEddaPort            │
│  Testing: mocks, fixtures, validators                    │
│                                                          │
│  Services:                                               │
│  ├── MemoryStore, MemoryService (EDDA)                   │
│  ├── ProposalStore, CandidateService, DecayService (EMBER)│
│  ├── EvolutionService, PromotionService (lifecycle)      │
│  ├── ProvenanceService, VersionTracker (audit)           │
│  └── ObservationHook, AggregatorService (KINDLING→EMBER) │
│                                                          │
│  Rules: Convergence │ Escalation │ Repetition │          │
│         Resolution │ Surprise                            │
└──────────────────────────────────────────────────────────┘
```

---

## 6. TUI Surfaces

### Ratatui TUI (Rust) [CURRENT]

The shared Ratatui component library (`eddacraft-tui`) is an external git
dependency. Anvil-specific TUI surfaces live in `crates/anvil-tui/`:

**Shared Components:** Header, Container, Divider, Spinner, StatusBadge,
Confirm, Select, TextInput, ProgressBar, StatusBar, LogPanel, ParallelProgress,
QuickWinsPanel, ResultsDashboard

**Command Surfaces (all ported from Ink — PORT and RATS modules complete):**

| Surface  | Location                                  | Complexity |
| -------- | ----------------------------------------- | ---------- |
| Welcome  | `crates/anvil-tui/src/surfaces/welcome/`  | Simple     |
| Doctor   | `crates/anvil-tui/src/surfaces/doctor/`   | Simple     |
| Status   | `crates/anvil-tui/src/surfaces/status/`   | Medium     |
| Init     | `crates/anvil-tui/src/surfaces/init/`     | Medium     |
| Audit    | `crates/anvil-tui/src/surfaces/audit/`    | Medium     |
| Browser  | `crates/anvil-tui/src/surfaces/browser/`  | Medium     |
| Gate     | `crates/anvil-tui/src/surfaces/gate/`     | Complex    |
| Watch    | `crates/anvil-tui/src/surfaces/watch/`    | Complex    |
| Tutorial | `crates/anvil-tui/src/surfaces/tutorial/` | Complex    |
| Wizard   | `crates/anvil-tui/src/surfaces/wizard/`   | Medium     |

### Legacy Ink TUI (React-based) [REMOVED]

The original Ink TUI (`apps/anvil-cli/src/tui/`) has been fully replaced by
Ratatui. The Node.js CLI package (`@eddacraft/anvil-cli`) is deprecated.

---

## 7. CLI Command Surface

**Status: [CURRENT]** — 30+ commands in `crates/anvil-cli/src/commands/`

| Domain           | Commands                                                                                                       |
| ---------------- | -------------------------------------------------------------------------------------------------------------- |
| **Core**         | `check`, `gate`, `gate-config`, `watch`, `status`, `init`, `doctor`, `welcome`                                 |
| **Architecture** | `architecture`, `drift`, `validate`                                                                            |
| **Policy**       | `policy` (bundle, diff, doc, explain, init, list, scaffold, toggle, validate, why)                             |
| **Memory**       | `edda` (list, show, promote, retire, trace), `ember` (list, show, promote), `stack` (config, status, validate) |
| **Planning**     | `plan` (load, lock, status, unlock, validate)                                                                  |
| **Agent**        | `agent` (cleanup, info, list, status)                                                                          |
| **Export**       | `export`, `audit`, `explain`, `authorship`                                                                     |
| **Setup**        | `hooks`, `new`, `tutorial`, `beta`, `release`                                                                  |
| **Auth**         | `login`, `logout`, `whoami`                                                                                    |
| **Integration**  | `mcp-config`                                                                                                   |

---

## 8. Dashboard (Web)

**Status: [PROPOSED]** — route structure defined, not yet implemented

| Module       | Routes                                                                     | APS           |
| ------------ | -------------------------------------------------------------------------- | ------------- |
| Shell        | Layout, sidebar, top bar                                                   | DASH-001..002 |
| Components   | Metric cards, tables, badges, charts                                       | DASH-003..004 |
| API Layer    | `/api/anvil/*` routes (status, gates, warnings, drift, config, provenance) | DASH-005..006 |
| Core Views   | Overview, gates, gate detail, warnings, breakdown, patterns                | DASHCORE      |
| Architecture | Violations, graph, drift, drift detail, compare, suppressions              | DASHARCH      |
| Operations   | Audit log, user activity, AI tool usage, plans, config, diagnostics        | DASHOPS       |
| AI Builder   | JSON-render engine, component catalogue, templates, saved dashboards       | DASHAI        |

---

## 9. Integration Points & External Dependencies

### Current External Dependencies

| Tool                   | Used By                            | Purpose                | End State                                  |
| ---------------------- | ---------------------------------- | ---------------------- | ------------------------------------------ |
| **dependency-cruiser** | ArchitectureCheck, DependencyCheck | Import graph analysis  | **Replaced** by Rust kernel semantic graph |
| **OPA**                | PolicyCheck                        | Rego policy evaluation | **Replaced** by Rust policy engine         |
| **ESLint**             | ESLintCheck                        | Lint rule evaluation   | **Stays** (complementary)                  |
| **Jest/Vitest**        | CoverageCheck                      | Test coverage          | **Stays** (complementary)                  |
| **Git**                | Watch, Drift, Provenance           | VCS operations         | **Stays**                                  |
| **chokidar**           | FileWatcher                        | FS notifications       | **Replaced** by notify-rs                  |
| **tree-sitter**        | (not yet)                          | Parsing                | **[PROPOSED]** in Rust kernel              |

### AI/Agent Integration Points

| Integration          | Purpose                                          | Status        |
| -------------------- | ------------------------------------------------ | ------------- |
| MCP Server           | Expose Anvil to AI agents (fix, suppress, query) | **[CURRENT]** |
| VS Code Extension    | IDE integration                                  | **[CURRENT]** |
| Kindling Integration | Capture AI tool observations                     | **[CURRENT]** |
| Claude Code Hooks    | Pre/post tool-use hooks for Claude Code          | **[CURRENT]** |

---

## 10. Execution Modes (End State)

| Mode             | Engine            | Surface     | Use Case                               |
| ---------------- | ----------------- | ----------- | -------------------------------------- |
| One-shot check   | Rust (embedded)   | CLI output  | `anvil check` in CI/terminal           |
| Interactive gate | Rust (embedded)   | Ratatui TUI | `anvil gate` interactive explorer      |
| Watch mode       | Rust (foreground) | Ratatui TUI | `anvil watch` live dashboard           |
| Daemon mode      | Rust (background) | Any client  | VS Code, MCP, dashboard consume events |
| Legacy mode      | TypeScript        | _Removed_   | _Deprecated — Node.js CLI retired_     |

---

## 11. Vision: Constitutional Engineering

**Status: [PROPOSED]** — philosophy documented, not yet implemented

Beyond the H1 invariants, the end-state vision includes:

- **Structural Law:** Hard invariants (boundary violations, privilege expansion)
- **Evolution Law:** Rules about how structure may change over time
- **Procedural Law:** How changes are introduced and approved
- **Anticipatory Metrics:**
  - Boundary Stress Index — pressure on architectural boundaries
  - Privilege Drift Index — creeping access to sensitive resources
  - API Surface Growth — rate of public API expansion
  - External Surface Creep — growing external dependency surface
  - Entropy Gradient Detection — structural disorder trends
- **Behavioural Diff:** Summarise "what changed in behaviour" not just "what
  lines changed"
- **Plan-Aware Watching:** Validate code evolution against declared APS plans
- **Code Provenance Engine:** Track who introduced what, under which plan, with
  what guards

---

## 12. Deployment & Distribution

### Current

- **npm package** — `@eddacraft/anvil-cli` via npm
- **Node.js required** — runtime dependency
- **External binaries** — OPA, dependency-cruiser installed separately

### Proposed End State

- **Standalone Rust binary** — single binary, no Node.js required
- **npm wrapper** (optional) — thin npm package calling Rust binary
- **Cross-compiled** — Linux, macOS, Windows (KERN-044)
- **Optional daemon** — background service for persistent graph (KERN-050..052)
