<!--
APS Module: Rust Core Engine
==============================
Introduce Rust as the core engine layer for performance-critical subsystems:
policy engine, file watching, check execution, observation storage, and TUI.
Based on ADR-011.

Scopes: RENG (main)
-->

# Rust Core Engine

| ID   | Owner | Status   |
| ---- | ----- | -------- |
| RENG | —     | Proposed |

## Purpose

Introduce Rust as the core engine layer for Anvil and the EddaCraft product
family. The TypeScript CLI (30+ commands) stays. Rust handles the
performance-critical subsystems: policy engine, file watching, check execution,
observation storage, and interactive TUI.

**Why:** Watch mode gate execution takes 3-15 seconds in Node.js. Rust
parallelism and zero-GC execution bring this to ~200ms — crossing the threshold
from "background task" to "live feedback." This also unlocks 50x more policy
headroom and near-instant pre-commit hooks via cache lookups.

**Decision:** [ADR-011](../decisions/011-rust-core-engine.md)

## In Scope

- Shared Rust crates for EddaCraft product family (TUI, engine, storage)
- Anvil core engine (policy checks, lint, secret scan, architecture)
- Anvil watcher (notify-rs, adaptive debounce, git2)
- Kindling observation storage (rusqlite)
- Ratatui TUI (watch dashboard, gate viewer, init wizard)
- N-API bindings for TypeScript CLI integration
- Feature-flagged rollout (`ANVIL_RUST_ENGINE=1`)

## Out of Scope

- Rewriting the TypeScript CLI commands (they stay TypeScript)
- Test execution (Vitest stays JS — user code is JS)
- Coverage instrumentation (stays JS)
- E2E/Playwright tests
- Full ESLint replacement (oxlint supplements, doesn't replace)

## Interfaces

**Depends on:**

- `apps/anvil-cli/` — TypeScript CLI, N-API consumer
- `packages/anvil/core/` — current check implementations (being ported)
- `packages/anvil/runtime/` — watcher, gate runner (being replaced)
- `@kindling/store-sqlite` — observation store (being replaced)

**Exposes:**

- `anvil-core.node` — N-API native module for TypeScript CLI
- `anvil-tui` — standalone Ratatui binary for watch mode
- `eddacraft-tui` — shared crate for product family TUI components
- `eddacraft-engine` — shared crate for policy evaluation
- `eddacraft-kindling` — shared crate for observation storage

## Constraints

- Each phase delivers independently; TypeScript CLI works throughout
- Rust components are opt-in behind `ANVIL_RUST_ENGINE` feature flag
- Phase 0 spike must validate assumptions before committing to later phases
- If spike fails targets, fall back to JS-only optimisations (Alternative A)
- Must maintain parity with JS check results during transition
- Dual-run mode (Rust + JS in parallel) for validation during rollout

## Ready Checklist

Change status to **Ready** when:

- [ ] ADR-011 status changed from Proposed to Accepted
- [ ] Phase 0 spike validates tree-sitter parsing (<1ms per file)
- [ ] Phase 0 spike validates N-API round-trip overhead
- [ ] Phase 0 spike validates rusqlite write speed (<1ms per observation)
- [ ] Phase 0 spike validates Ratatui component library sufficiency
- [ ] Team confirms Rust proficiency for engine scope
- [ ] Cargo workspace structure agreed

---

## Phase 0 — Spike (Validation)

### RENG-001: Validate tree-sitter TypeScript parsing speed

- **Status:** Draft
- **Intent:** Confirm tree-sitter parses TypeScript files in <1ms per file,
  making AST-based checks viable at watch-mode speed
- **Expected Outcome:** Benchmark showing parse time for representative files
  (100-1000 LOC) is consistently <1ms
- **Validation:** Benchmark harness with 50+ real project files, p99 < 1ms
- **Files:** New Rust crate `crates/spike/`
- **Confidence:** high (tree-sitter benchmarks support this)
- **Priority:** Critical
- **Dependencies:** None

---

### RENG-002: Validate N-API binding round-trip overhead

- **Status:** Draft
- **Intent:** Confirm N-API calls from TypeScript to Rust add <1ms overhead
  per invocation, making the hybrid architecture viable
- **Expected Outcome:** Round-trip benchmark showing call overhead is
  negligible compared to check execution time
- **Validation:** 1000-call benchmark, median overhead < 0.5ms
- **Files:** `crates/spike/`, `apps/anvil-cli/src/native/`
- **Confidence:** high (napi-rs benchmarks support this)
- **Priority:** Critical
- **Dependencies:** None

---

### RENG-003: Validate rusqlite write speed for observations

- **Status:** Draft
- **Intent:** Confirm observation emission to SQLite takes <1ms, allowing
  the watcher to persist results without impacting cycle time
- **Expected Outcome:** Write benchmark showing single-row inserts consistently
  <1ms with WAL mode
- **Validation:** 10,000 sequential writes, p99 < 1ms
- **Files:** `crates/spike/`
- **Confidence:** high
- **Priority:** Critical
- **Dependencies:** None

---

### RENG-004: Validate Ratatui component library for wizard flows

- **Status:** Draft
- **Intent:** Confirm Ratatui Select, MultiSelect, and TextInput components
  support the onboarding wizard UX patterns used by Ink today
- **Expected Outcome:** Prototype wizard flow (3-4 screens) demonstrating
  equivalent UX to current Ink init wizard
- **Validation:** Visual comparison with current `anvil init` flow
- **Files:** `crates/spike/`
- **Confidence:** medium
- **Priority:** High
- **Dependencies:** None

---

### RENG-005: Validate notify-rs file detection latency

- **Status:** Draft
- **Intent:** Confirm notify-rs detects file changes with <10ms latency,
  replacing Chokidar's ~75ms
- **Expected Outcome:** Latency benchmark showing detection time from write
  to callback
- **Validation:** 100 file-write events, p99 detection < 20ms
- **Files:** `crates/spike/`
- **Confidence:** high
- **Priority:** High
- **Dependencies:** None

---

## Phase 1 — Secret Scanner

### RENG-006: Port secret scan patterns to Rust

- **Status:** Draft
- **Intent:** Port all secret detection regex patterns and entropy calculation
  to Rust as the first real check migration
- **Expected Outcome:** Rust secret scanner produces identical results to JS
  implementation on the full test fixture set
- **Validation:** Run both implementations on test fixtures, diff results
- **Files:** `crates/anvil-engine/src/secret/`
- **Confidence:** high
- **Priority:** High
- **Dependencies:** RENG-001 (spike validates approach)

---

### RENG-007: N-API binding for scanSecrets

- **Status:** Draft
- **Intent:** Expose Rust secret scanner to TypeScript CLI via N-API so the
  CLI can use it transparently
- **Expected Outcome:** `engine.scanSecrets(files)` callable from TypeScript,
  returns same result type as JS implementation
- **Validation:** Existing secret scan tests pass when routed through N-API
- **Files:** `crates/anvil-napi/`, `apps/anvil-cli/src/native/`
- **Confidence:** high
- **Priority:** High
- **Dependencies:** RENG-002, RENG-006

---

### RENG-008: Benchmark secret scanner (Rust vs JS)

- **Status:** Draft
- **Intent:** Validate the 40x speedup estimate with real project files
- **Expected Outcome:** Benchmark report showing actual speedup factor
- **Validation:** Side-by-side benchmark on 100+ files
- **Files:** `crates/bench/`
- **Confidence:** high
- **Priority:** Medium
- **Dependencies:** RENG-006, RENG-007

---

### RENG-009: Feature flag for Rust engine opt-in

- **Status:** Draft
- **Intent:** Add `ANVIL_RUST_ENGINE=1` feature flag so Rust checks are
  opt-in during rollout, with JS fallback as default
- **Expected Outcome:** When flag is set, secret scan routes through Rust;
  otherwise uses existing JS implementation
- **Validation:** Both paths produce identical results; flag toggles cleanly
- **Files:** `apps/anvil-cli/src/services/gate-runner.ts`
- **Confidence:** high
- **Priority:** High
- **Dependencies:** RENG-007

---

## Phase 2 — Architecture + Anti-Pattern Checks

### RENG-010: tree-sitter TypeScript/JavaScript parsing

- **Status:** Draft
- **Intent:** Integrate tree-sitter for fast AST parsing, replacing the JS
  parser used by architecture and anti-pattern checks
- **Expected Outcome:** Parse API that returns typed AST nodes for dependency
  extraction and pattern matching
- **Validation:** Parse 100+ project files, extract same imports as JS parser
- **Files:** `crates/anvil-engine/src/parse/`
- **Confidence:** high
- **Priority:** High
- **Dependencies:** RENG-001

---

### RENG-011: Dependency graph construction in Rust

- **Status:** Draft
- **Intent:** Build the architecture dependency graph from tree-sitter AST
  output, replacing the JS graph builder
- **Expected Outcome:** Same dependency edges detected as current JS
  implementation
- **Validation:** Diff graph output against JS implementation on project
- **Files:** `crates/anvil-engine/src/architecture/`
- **Confidence:** high
- **Priority:** High
- **Dependencies:** RENG-010

---

### RENG-012: Anti-pattern rule evaluation in Rust

- **Status:** Draft
- **Intent:** Port anti-pattern pattern matching to Rust, operating on
  tree-sitter AST nodes
- **Expected Outcome:** Same warnings produced as JS anti-pattern scanner
- **Validation:** Run both implementations on test fixtures, diff results
- **Files:** `crates/anvil-engine/src/antipattern/`
- **Confidence:** high
- **Priority:** High
- **Dependencies:** RENG-010

---

### RENG-013: N-API bindings for architecture + anti-pattern checks

- **Status:** Draft
- **Intent:** Expose Rust architecture and anti-pattern checks to TypeScript
  CLI via N-API
- **Expected Outcome:** `engine.checkArchitecture(files)` and
  `engine.checkAntiPatterns(files)` callable from TypeScript
- **Validation:** Existing check tests pass when routed through N-API
- **Files:** `crates/anvil-napi/`
- **Confidence:** high
- **Priority:** High
- **Dependencies:** RENG-011, RENG-012

---

## Phase 3 — Watcher

### RENG-014: notify-rs file watching with adaptive debounce

- **Status:** Draft
- **Intent:** Replace Chokidar with notify-rs for 10ms file detection and
  adaptive debouncing (50ms for single file, longer for batch saves)
- **Expected Outcome:** Watcher detects file changes and debounces correctly,
  same behaviour as current Chokidar watcher
- **Validation:** Watch mode detects saves and triggers checks
- **Files:** `crates/anvil-watcher/`
- **Confidence:** high
- **Priority:** High
- **Dependencies:** RENG-005

---

### RENG-015: git2 integration for status filtering

- **Status:** Draft
- **Intent:** Replace `git diff` subprocess calls with libgit2 via the git2
  crate for faster status filtering in the watcher
- **Expected Outcome:** Git status filtering in ~20ms vs ~125ms (subprocess)
- **Validation:** Same files filtered as current `git diff --name-only`
- **Files:** `crates/anvil-watcher/src/git.rs`
- **Confidence:** high
- **Priority:** Medium
- **Dependencies:** None

---

### RENG-016: Parallel gate runner orchestration

- **Status:** Draft
- **Intent:** Run all Rust checks in parallel within a single gate cycle,
  streaming results as they complete
- **Expected Outcome:** Dev profile gate completes in ~120ms (longest check
  wins, not sum)
- **Validation:** Benchmark showing parallel execution time ≈ max(individual)
- **Files:** `crates/anvil-gate/`
- **Confidence:** high
- **Priority:** High
- **Dependencies:** RENG-006, RENG-011, RENG-012

---

### RENG-017: Pre-commit cache layer

- **Status:** Draft
- **Intent:** Cache last-known-good check results so pre-commit hooks can
  do sub-millisecond lookups instead of re-running checks
- **Expected Outcome:** Pre-commit hook queries cache, returns instantly if
  all staged files have passing results
- **Validation:** Pre-commit with warm cache completes in <10ms
- **Files:** `crates/anvil-watcher/src/cache.rs`
- **Confidence:** medium
- **Priority:** Medium
- **Dependencies:** RENG-016

---

## Phase 4 — Kindling Storage

### RENG-018: rusqlite observation store

- **Status:** Draft
- **Intent:** Replace `@kindling/store-sqlite` with a Rust implementation
  using rusqlite for faster write/query performance
- **Expected Outcome:** Same observation schema, faster writes (<1ms),
  retention pruning in Rust
- **Validation:** Existing Kindling tests pass against Rust store
- **Files:** `crates/eddacraft-kindling/`
- **Confidence:** high
- **Priority:** Medium
- **Dependencies:** RENG-003

---

### RENG-019: Kindling query API with scope enforcement

- **Status:** Draft
- **Intent:** Expose observation queries via N-API with scope enforcement
  (session, gate, file)
- **Expected Outcome:** `engine.queryKindling({ scope, session_id })` returns
  observations, same API contract as current TypeScript implementation
- **Validation:** Query results match existing implementation
- **Files:** `crates/eddacraft-kindling/src/query.rs`, `crates/anvil-napi/`
- **Confidence:** high
- **Priority:** Medium
- **Dependencies:** RENG-018

---

## Phase 5 — TUI

### RENG-020: eddacraft-tui shared crate

- **Status:** Draft
- **Intent:** Create shared Ratatui component library with EddaCraft theme,
  keyboard conventions, and reusable widgets
- **Expected Outcome:** Themed Select, MultiSelect, TextInput, ProgressBar,
  StatusBar components
- **Validation:** Visual parity with current Ink components
- **Files:** `crates/eddacraft-tui/`
- **Confidence:** medium
- **Priority:** Medium
- **Dependencies:** RENG-004

---

### RENG-021: Watch mode dashboard

- **Status:** Draft
- **Intent:** Ratatui watch dashboard showing live gate results, file status,
  and warning list
- **Expected Outcome:** Equivalent to current Ink watch dashboard, rendered
  at ~2ms per frame
- **Validation:** Visual comparison with current watch mode
- **Files:** `crates/anvil-tui/src/dashboard/`
- **Confidence:** medium
- **Priority:** Medium
- **Dependencies:** RENG-016, RENG-020

---

### RENG-022: APS onboarding wizard

- **Status:** Draft
- **Intent:** Ratatui wizard for APS project initialisation, shared across
  Anvil and APS products
- **Expected Outcome:** Multi-step wizard flow with template selection,
  configuration, and scaffold generation
- **Validation:** Functional parity with current `anvil init` wizard
- **Files:** `crates/anvil-tui/src/wizard/`
- **Confidence:** medium
- **Priority:** Low
- **Dependencies:** RENG-020

---

## Phase 6 — Lint Integration

### RENG-023: oxlint integration

- **Status:** Draft
- **Intent:** Integrate oxlint for Rust-speed linting, supplementing ESLint
  for rules with oxlint equivalents
- **Expected Outcome:** oxlint runs configured rules at ~120ms (vs 1200ms
  ESLint), results mapped to Anvil warning format
- **Validation:** Rule coverage comparison, dual-run validation
- **Files:** `crates/anvil-engine/src/lint/`
- **Confidence:** medium
- **Priority:** Low
- **Dependencies:** RENG-016

---

### RENG-024: Pre-commit hook optimisation

- **Status:** Draft
- **Intent:** Replace lint-staged re-execution with watcher cache lookups
  for near-instant pre-commit checks
- **Expected Outcome:** Pre-commit hook completes in <10ms when watcher cache
  is warm, falls back to incremental check (~50-200ms) when stale
- **Validation:** Commit time benchmark: current (3-8s) vs optimised (<200ms)
- **Files:** `.husky/pre-commit`, `crates/anvil-watcher/`
- **Confidence:** medium
- **Priority:** Low
- **Dependencies:** RENG-017

---

## Performance Targets

| Metric | Current (Node.js) | Target (Rust) | Speedup |
| ------ | ----------------- | ------------- | ------- |
| Dev gate cycle | 2400ms | 120ms | 20x |
| Full gate cycle | 12900ms | 5000ms | 2.6x |
| Watch total cycle | 2907ms | 202ms | 14x |
| Secret scan | 400ms | 10ms | 40x |
| Architecture check | 800ms | 40ms | 20x |
| Anti-pattern check | 800ms | 40ms | 20x |
| Pre-commit (warm cache) | 3-8s | <10ms | 300x+ |
| Watcher memory | 50-100MB | 10-20MB | 5x |

## Risks

| Risk | Likelihood | Impact | Mitigation |
| ---- | ---------- | ------ | ---------- |
| Rust performance estimates optimistic | Low | Medium | Phase 0 spike with real benchmarks |
| N-API binding complexity | Medium | Medium | Spike validates; subprocess fallback |
| tree-sitter TS edge cases | Medium | Low | JS parser fallback during transition |
| Team Rust proficiency | Medium | High | Evaluate after Phase 1 before committing |
| oxlint rule coverage gaps | Medium | Medium | Dual-run mode; custom rules fill gaps |
| Build complexity (Cargo + pnpm) | Medium | Low | Pre-built native modules in CI |

## Stats

| Phase | Items | Estimated |
| ----- | ----- | --------- |
| 0 — Spike | 5 | 1 week |
| 1 — Secret Scanner | 4 | 2-3 weeks |
| 2 — Architecture + Anti-Pattern | 4 | 3-4 weeks |
| 3 — Watcher | 4 | 2-3 weeks |
| 4 — Kindling Storage | 2 | 2-3 weeks |
| 5 — TUI | 3 | 3-4 weeks |
| 6 — Lint Integration | 2 | 2-3 weeks |
| **Total** | **24** | **15-20 weeks** |
