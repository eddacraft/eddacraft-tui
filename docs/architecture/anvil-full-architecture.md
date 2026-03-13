# Anvil — Full Architecture (Current vs Proposed End State)

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
│  │  (TS + Ink)  │  │ (Next.js)│  │   (TS)    │  │  Ext  │  │
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

| Package          | Purpose                                                   | Status        |
| ---------------- | --------------------------------------------------------- | ------------- |
| `apps/anvil-cli` | CLI + Ink TUI (30+ commands)                              | **[CURRENT]** |
| `apps/website`   | Next.js marketing site (dashboard is [PROPOSED] — see §8) | **[CURRENT]** |
| `apps/anvil-api` | API server                                                | **[CURRENT]** |
| `apps/docs-site` | Documentation site                                        | **[CURRENT]** |
| `apps/e2e`       | End-to-end test suite                                     | **[CURRENT]** |

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

### Platform

| Package                     | Purpose                            | Status        |
| --------------------------- | ---------------------------------- | ------------- |
| `packages/platform/config`  | Config file loading and resolution | **[CURRENT]** |
| `packages/platform/storage` | File-system storage abstraction    | **[CURRENT]** |
| `packages/platform/crypto`  | Cryptographic utilities            | **[CURRENT]** |

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
| `crates/spike`              | Phase 0 validation spikes (tree-sitter, notify, petgraph) | **[CURRENT]**  |
| `crates/anvil-kernel-types` | Shared event/graph/trust type contracts                   | **[CURRENT]**  |
| `crates/eddacraft-tui`      | Shared Ratatui component library                          | **[CURRENT]**  |
| `crates/anvil-checks`       | Ported checks: secret, antipattern, command safety        | **[CURRENT]**  |
| `crates/anvil-kernel`       | Core kernel: watcher, parser, graph, policy engine        | **[PROPOSED]** |
| `crates/anvil-tui`          | Anvil-specific TUI surfaces                               | **[PROPOSED]** |
| `crates/anvil-napi`         | N-API bridge for Node.js CLI integration                  | **[PROPOSED]** |
| `crates/eddacraft-kindling` | Kindling Rust integration                                 | **[PROPOSED]** |
| `crates/bench`              | Cross-crate performance benchmarks                        | **[PROPOSED]** |

> **Note:** Watcher, gate, and engine responsibilities are consolidated into
> `anvil-kernel` as internal modules — see `rust-architecture-endstate.md` for
> the detailed crate map. Earlier planning documents referenced separate
> `anvil-gate`, `anvil-watcher`, and `anvil-engine` crates; those have been
> folded into the kernel to reduce cross-crate complexity.

---

## 3. Gate / Check System

The gate is Anvil's core enforcement mechanism — it runs a configurable set of
checks against repository state and produces pass/fail/warn results.

### Current Gate Checks (TypeScript)

| Check                | What It Does                                | External Deps      | Rust Port                 |
| -------------------- | ------------------------------------------- | ------------------ | ------------------------- |
| `SecretCheck`        | Entropy + pattern-based secret detection    | None               | **Done** (RENG-001)       |
| `AntipatternCheck`   | Detects code anti-patterns (13 patterns)    | None               | **Done** (RENG-002)       |
| `CommandSafetyCheck` | Validates shell commands (36 rules)         | None               | **Done** (RENG-003)       |
| `ArchitectureCheck`  | Layer violations via dependency analysis    | dependency-cruiser | **[PROPOSED]** (RENG-004) |
| `DependencyCheck`    | Vulnerability audit via npm/yarn/pnpm audit | npm/yarn/pnpm      | Stays TS                  |
| `PolicyCheck`        | OPA Rego policy evaluation                  | OPA binary         | **[PROPOSED]**            |
| `ESLintCheck`        | ESLint rule violations                      | ESLint             | Stays TS                  |
| `CoverageCheck`      | Test coverage thresholds                    | Jest/Vitest        | Stays TS                  |

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

### Proposed Gate Pipeline (End State)

```
                              [PROPOSED]
                                 │
  .anvil/gate.yaml ──► GateConfigManager ──► Engine Selector
                                                │
                              ┌─────────────────┴──────────────┐
                              ▼                                ▼
                      Legacy TS Engine                  Rust Kernel Engine
                      (--engine legacy)                 (--engine rust)
                              │                                │
                              ▼                                ▼
                       GateRunner (TS)              ┌──────────────────┐
                       (as today)                   │ Watcher          │
                                                    │ → Parser         │
                                                    │ → Semantic Graph │
                                                    │ → Policy Engine  │
                                                    │ → Event Emission │
                                                    └──────────────────┘
                              │                                │
                              ▼                                ▼
                    ┌────────────────────────────────────────────┐
                    │         Engine Event Protocol              │
                    │  Progress | Snapshot | Violation | Error   │
                    └────────────────────────────────────────────┘
                                       │
                              ┌────────┼────────┐
                              ▼        ▼        ▼
                           CLI      Website   VS Code
                           TUI     Dashboard  Extension
```

---

## 4. Watch System

### Current (TypeScript)

```
File System (chokidar) ──► FileWatcher ──► Debouncer ──► WatchOrchestrator
                                                               │
                                                    ┌──────────┼──────────┐
                                                    ▼          ▼          ▼
                                              GateRunner  GitStatus   Ink TUI
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

### Current Ink TUI (React-based) [CURRENT]

Located in `apps/anvil-cli/src/tui/`:

**Shared Components:** Header, Container, Divider, Spinner, StatusBadge,
Confirm, Select, TextInput, ProgressBar, LogPanel, ParallelProgress,
QuickWinsPanel, ResultsDashboard, MermaidDiagram, ErrorBoundary

**Command Surfaces:**

| Surface  | Components                                   | Complexity |
| -------- | -------------------------------------------- | ---------- |
| Welcome  | `Welcome.tsx`                                | Simple     |
| Doctor   | `Diagnostics.tsx`                            | Simple     |
| Status   | `StatusDashboard.tsx` + 3 panels             | Medium     |
| Init     | `InitWizard.tsx` + 5 step components         | Medium     |
| Audit    | `AuditResults.tsx`                           | Medium     |
| New      | `TemplateBrowser.tsx`                        | Medium     |
| Gate     | `GateExplorer.tsx` + 3 panels                | Complex    |
| Watch    | `WatchDashboard.tsx` + 4 panels              | Complex    |
| Tutorial | `Tutorial.tsx` + Picker + 4 paths (23 steps) | Complex    |

### Proposed Ratatui TUI (Rust) [PROPOSED]

Located in `crates/eddacraft-tui/` (shared) + `crates/anvil-tui/` (planned):

**Shared Components (Done):** Header, Container, Divider, Spinner, StatusBadge,
Confirm, Select, TextInput, ProgressBar, StatusBar, LogPanel, ParallelProgress,
QuickWinsPanel, ResultsDashboard

**Planned Surfaces:** 1:1 port of all Ink surfaces above (PORT module, 15
items), plus new kernel-native surfaces (RATS module, 7 items)

### Migration Path

```
Phase 1 (Coexistence):   Ink TUI (default)  │  Ratatui (--tui=ratatui)
Phase 2 (Validation):    Ink TUI (default)  │  Ratatui (validated)
Phase 3 (Cutover):       Ratatui (default)  │  Ink (--tui=ink fallback)
Phase 4 (Removal):       Ratatui only       │  Node.js dependency removed
```

---

## 7. CLI Command Surface

**Status: [CURRENT]** — 30+ commands in `apps/anvil-cli/src/commands/`

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
| Forge Pipeline       | Pre-commit cross-model review                    | **[CURRENT]** |
| Temper Pipeline      | Post-push CI auto-healing                        | **[CURRENT]** |

---

## 10. Execution Modes (End State)

| Mode             | Engine            | Surface     | Use Case                               |
| ---------------- | ----------------- | ----------- | -------------------------------------- |
| One-shot check   | Rust (embedded)   | CLI output  | `anvil check` in CI/terminal           |
| Interactive gate | Rust (embedded)   | Ratatui TUI | `anvil gate` interactive explorer      |
| Watch mode       | Rust (foreground) | Ratatui TUI | `anvil watch` live dashboard           |
| Daemon mode      | Rust (background) | Any client  | VS Code, MCP, dashboard consume events |
| Legacy mode      | TypeScript        | Ink TUI     | `--engine legacy` fallback             |
| Dual-run mode    | Both              | CLI diff    | `--engine dual` parity validation      |

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
