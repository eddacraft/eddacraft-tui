# ADR-011: Rust Core Engine for Anvil Product Family

## Status

Proposed (Decision Space)

## Date

2026-02-28

## Context

Three converging pressures have exposed limitations in the current
all-TypeScript architecture:

1. **Watch mode performance** — The watcher runs checks sequentially in
   Node.js. Gate execution takes 3-15 seconds per save. ADR-009 correctly
   identified check execution as the bottleneck (99% of latency) but
   concluded "performance is not a differentiator" by comparing only TUI
   render times (5ms vs 1ms). The actual question — whether faster check
   execution lets us run *more checks* in the same time window — was never
   evaluated.

2. **TUI framework** — ADR-005 chose Ink over OpenTUI (Bun dependency).
   ADR-008 rejected Ratatui (technology mismatch). Both decisions assumed
   the codebase would remain pure TypeScript. If Rust enters the stack for
   other reasons, the "two languages" argument against Ratatui dissolves.

3. **Product family growth** — APS needs a TUI for its onboarding wizard.
   Kindling needs fast write/query performance. Building shared Rust
   components (TUI, storage, engine) serves all three products without
   duplicating work across languages.

### What ADR-008 and ADR-009 Got Right

- TUI rendering is not the bottleneck (correct, still true)
- Rust ↔ TypeScript integration adds complexity (correct, mitigated if
  Rust is already in the stack)
- Ink is sufficient for current TUI needs (correct for current scope)

### What ADR-008 and ADR-009 Missed

- The question "can we run more checks in the same time window?" was
  never evaluated — only "does the TUI render faster?"
- The policy engine, secret scanner, architecture checker, and
  anti-pattern detector are CPU-bound work where Rust is 10-100x faster
- If Rust enters the stack for the engine, the cost of also using it
  for TUI drops to near zero
- Hook-based and CI-based checks could move into the watcher if the
  watcher were fast enough to maintain real-time status

## Recommendation

**Introduce Rust as the core engine layer for Anvil, with shared crates
for the product family.**

This is NOT a full rewrite. The TypeScript CLI (30+ commands) stays. Rust
handles the performance-critical subsystems: policy engine, file watching,
check execution, observation storage, and interactive TUI.

## Performance Analysis: Actual Anvil Workloads

### Current Check Execution Times (from ADR-009 and gate-runner.ts)

| Check | Current (Node.js) | Estimated (Rust) | Speedup | Basis |
|-------|-------------------|-------------------|---------|-------|
| ESLint | 500-2000ms | 50-200ms | 10x | oxlint benchmarks vs ESLint |
| Vitest | 1000-5000ms | N/A (stays JS) | 1x | Test execution is JS by nature |
| Coverage | 2000-10000ms | N/A (stays JS) | 1x | Coverage instrumentation is JS |
| Secret scan | 200-800ms | 5-20ms | 40x | Regex + entropy calc, pure CPU |
| Dependency audit | 500-2000ms | 100-400ms | 5x | JSON parse + graph traversal |
| Architecture check | 500-2000ms | 20-100ms | 25x | AST parse + dependency graph |
| Anti-pattern check | 500-2000ms | 20-100ms | 25x | AST traversal + pattern match |
| Policy (OPA/Rego) | 200-1000ms | 50-200ms | 5x | Rego evaluation |
| Command safety | 100-500ms | 5-20ms | 25x | String analysis + pattern match |

### Basis for Estimates

**ESLint → oxlint (10x):**
oxlint (Rust-based linter) benchmarks at 50-100x faster than ESLint for
rule checking. Conservative 10x accounts for startup, config loading, and
the subset of rules applicable. Source: oxlint benchmarks.

**Secret scan (40x):**
Current implementation uses JavaScript regex + Shannon entropy calculation.
Rust's regex crate is ~40x faster than JS RegExp for batch matching.
Entropy calculation is pure arithmetic — no GC, no boxing. This is the
most straightforward Rust win.

**Architecture/Anti-pattern checks (25x):**
Currently parse TypeScript ASTs with JS parsers, traverse, and match
patterns. Rust with tree-sitter parses TypeScript files in ~0.1-1ms
(vs ~5-20ms in JS). Pattern matching on AST nodes is pure CPU work
with zero allocation pressure in Rust.

**Dependency audit (5x):**
JSON parsing + graph traversal. serde_json parses ~5x faster than
JSON.parse for large dependency trees. Graph algorithms benefit from
Rust's cache-friendly data structures.

**Vitest/Coverage (1x — stays JS):**
Test execution runs user JavaScript code. This cannot move to Rust.
Tests stay as-is.

### Gate Execution: Current vs Projected

#### Dev Profile (ESLint + Secret + Architecture)

```
CURRENT (sequential, Node.js):
  ESLint:       1200ms
  Secret:        400ms
  Architecture:  800ms
  ─────────────────────
  Total:        2400ms

RUST ENGINE (parallel):
  ESLint (oxlint): 120ms ┐
  Secret:            10ms ├─ parallel
  Architecture:      40ms ┘
  ─────────────────────────
  Total:            120ms  (longest check wins)
```

**Speedup: 20x (2400ms → 120ms)**

#### Full Profile (All Checks)

```
CURRENT (sequential, Node.js):
  ESLint:         1200ms
  Vitest:         3000ms
  Coverage:       5000ms
  Secret:          400ms
  Dependency:     1000ms
  Architecture:    800ms
  Anti-pattern:    800ms
  Policy:          500ms
  Command safety:  200ms
  ─────────────────────────
  Total:         12900ms

RUST ENGINE (parallel, JS tests separate):
  Rust checks (parallel):
    ESLint (oxlint): 120ms ┐
    Secret:            10ms │
    Dependency:       200ms ├─ all parallel = 200ms
    Architecture:      40ms │
    Anti-pattern:      40ms │
    Policy:           100ms │
    Command safety:    10ms ┘

  JS checks (parallel):
    Vitest:         3000ms ┐
    Coverage:       5000ms ├─ parallel = 5000ms
                           ┘

  Total: max(200ms, 5000ms) = 5000ms
```

**Speedup: 2.6x (12900ms → 5000ms)**

The Rust checks finish in 200ms — they're essentially free while
waiting for JS tests. The bottleneck shifts entirely to test execution,
which is user JS code and can't be Rusted.

### Watch Mode: What Changes

#### Current Watch Cycle

```
File save
  → Chokidar detect:    ~75ms
  → Debounce:           ~300ms
  → Git filter:         ~125ms
  → Gate (dev):         ~2400ms
  → TUI render:         ~7ms
  ─────────────────────────────
  Total:                ~2907ms  (≈3 seconds)
```

#### Rust Watch Cycle

```
File save
  → notify-rs detect:    ~10ms   (replaces Chokidar)
  → Debounce:            ~50ms   (adaptive: single file = 50ms)
  → Git filter:          ~20ms   (libgit2 via git2 crate)
  → Gate (dev, parallel): ~120ms
  → TUI render:           ~2ms   (Ratatui)
  ─────────────────────────────
  Total:                 ~202ms  (≈0.2 seconds)
```

**Speedup: 14x (2907ms → 202ms)**

At 200ms response time, the watcher feels *instantaneous*. This crosses
the threshold from "background task that eventually reports" to "live
feedback as you type."

### What 200ms Unlocks: Moving Hooks and CI Into the Watcher

If the watcher maintains real-time check status (results always < 1 second
old), several checks can shift from hooks and CI into the watcher:

#### Pre-Commit Hooks (Currently via lint-staged)

| Check | Current Hook Time | Watcher Status |
|-------|-------------------|----------------|
| ESLint --fix | 1-3s per file | Already checked, result cached |
| Prettier --write | 0.5-1s per file | Can integrate as formatter |
| markdownlint --fix | 0.5-1s per file | Already checked if .md |

**With a Rust watcher:** The pre-commit hook becomes a *status check*
rather than a *re-execution*. Instead of running ESLint again at commit
time, the hook asks the watcher "is this file clean?" — a sub-millisecond
lookup. If the watcher hasn't caught up (race condition), it runs an
incremental check on just the staged files.

```
CURRENT pre-commit:
  lint-staged → ESLint → Prettier → markdownlint
  Time: 3-8 seconds (re-runs everything)

RUST WATCHER pre-commit:
  Query watcher cache for staged files
  If all clean: instant pass (<10ms)
  If stale: incremental check on changed files only (~50-200ms)
```

**Developer impact: commits go from 3-8 seconds to near-instant.**

#### CI Checks (Currently via GitHub Actions)

| CI Job | Current CI Time | With Watcher |
|--------|----------------|--------------|
| ESLint (affected) | 30-120s (with setup) | Redundant if watcher ran |
| Format check | 15-30s | Redundant if watcher ran |
| Typecheck | 30-60s | Could move to watcher |
| Secret scan | 10-30s | Redundant if watcher ran |

**With a Rust watcher:** Gate results can be persisted as provenance
records (via Kindling). CI can verify the provenance trail ("was every
file checked?") rather than re-running checks. This shifts CI from
"run all checks" to "verify all checks ran" — seconds instead of minutes.

Checks that remain in CI:
- **Vitest/Coverage** — must run in clean environment
- **Cross-platform tests** — OS-specific issues
- **E2E/Playwright** — browser tests
- **Build** — compilation verification

Checks that move to watcher + provenance verification:
- **Lint** — watcher runs continuously
- **Format** — watcher runs continuously
- **Secret scan** — watcher runs continuously
- **Architecture** — watcher runs continuously
- **Typecheck** — watcher can run tsc incrementally

### Checks Per Window: The Real Question

ADR-009 asked: "Does Ratatui render faster?" (wrong question)

The right question: "How many policy rules can we evaluate between saves?"

**Assumptions:**
- Developer saves every 30 seconds on average
- Watcher should complete before next save
- Current: 2.9 seconds per cycle = ~10 cycles per save window
- Rust: 0.2 seconds per cycle = ~150 cycles per save window

But more importantly, within a *single* cycle:

| | Current (Node.js) | Rust Engine |
|---|---|---|
| **ESLint rules** | ~300 rules in 1200ms | ~300 rules in 120ms |
| **Secret patterns** | ~50 patterns in 400ms | ~50 patterns in 10ms |
| **Architecture rules** | ~10 rules in 800ms | ~10 rules in 40ms |
| **Custom policies** | ~5 policies in 500ms | ~5 policies in 100ms |
| **Headroom** | None — already at 2.4s | 29.8s of unused budget |

With Rust, you could run **50x more policy rules** and still finish in
under 3 seconds. Or run the same rules and have the result ready in 200ms.

This means:
- Team-specific policies (coding standards, naming conventions)
- Security policies beyond secret scanning (OWASP patterns, injection risks)
- Architectural invariants (no circular deps, layer violations)
- Custom project rules (no direct DB access from handlers, etc.)

All running continuously, all with sub-second feedback.

## Architecture

### Shared Rust Crates (EddaCraft Product Family)

```
eddacraft-tui           Ratatui components, EddaCraft theme,
                        keyboard conventions (j/k, space, enter, esc)

eddacraft-engine        Policy engine, tree-sitter AST parsing,
                        rule evaluation, pattern matching

eddacraft-kindling      Observation store (rusqlite), query API,
                        retention pruning, sensitive data validation
```

### Anvil Architecture

```
anvil-core (Rust)
├── engine/              Policy checks, lint, secret scan, architecture
├── watcher/             notify-rs file watching, adaptive debounce, git2
├── kindling/            Observation emission + query (uses eddacraft-kindling)
├── gate/                Gate runner (parallel, streaming results)
└── napi/                N-API bindings for TypeScript CLI

anvil-tui (Rust)
├── dashboard/           Watch mode dashboard (uses eddacraft-tui)
├── gate-view/           Interactive gate results
└── init-wizard/         Onboarding wizard

anvil-cli (TypeScript — unchanged)
├── commands/            30+ CLI commands
├── services/            Business logic
└── tui/ (Ink)           Existing TUI (deprecated incrementally)
```

### APS Architecture

```
aps-init (Rust)
├── wizard/              Onboarding wizard (uses eddacraft-tui)
└── scaffold/            Template selection + download

aps-cli (Bash — unchanged)
├── bin/aps              Lint, update
└── lib/                 Lint rules, scaffold logic

scaffold/install (Bash — unchanged)
└── curl | bash fallback
```

### Data Flow: Watcher With Rust Engine

```
File save
  │
  ▼
notify-rs (Rust)              ─── 10ms
  │
  ▼
Adaptive debouncer            ─── 50ms (single file)
  │
  ▼
git2 status filter            ─── 20ms
  │
  ▼
Gate runner (parallel)        ─── 120ms
  ├── oxlint check ─────────── 120ms ─┐
  ├── secret scan ───────────── 10ms  │
  ├── architecture check ────── 40ms  ├─ parallel
  ├── anti-pattern check ────── 40ms  │
  ├── policy eval ───────────── 100ms │
  └── command safety ────────── 10ms ─┘
  │
  ├──▶ Kindling emit          ─── <1ms (same process)
  ├──▶ Ratatui TUI update     ─── 2ms (same process)
  └──▶ Cache update           ─── <1ms (for pre-commit lookups)
```

Total: ~202ms. Single process. Zero IPC. Zero serialization between
engine, storage, and TUI.

### Integration: TypeScript CLI ↔ Rust Core

The TypeScript CLI calls into Rust via N-API for heavy operations:

```typescript
// TypeScript CLI (unchanged command structure)
import { AnvilEngine } from './native/anvil-core.node';

// Gate command — calls Rust engine
const engine = new AnvilEngine(config);
const results = await engine.runGate(files, {
  profile: 'dev',
  onProgress: (event) => {
    // Stream results to CLI output
    output.handleEvent(event);
  }
});

// Watch command — starts Rust watcher
const watcher = engine.startWatch({
  patterns: ['**/*.ts', '**/*.md'],
  onResult: (result) => output.handleEvent(result),
});

// Query Kindling — calls Rust storage
const observations = await engine.queryKindling({
  scope: 'session',
  session_id: runId,
});
```

The TypeScript CLI remains the user-facing layer. Rust is the engine
underneath. Users see no difference in command names, flags, or output
format.

## Migration Strategy

### Phase 0: Spike (1 week)

Validate the critical assumptions:

- [ ] tree-sitter TypeScript parsing speed in Rust (target: <1ms per file)
- [ ] rusqlite write speed for observation emission (target: <1ms)
- [ ] N-API binding from TypeScript to Rust (round-trip overhead)
- [ ] Ratatui Select/MultiSelect component for wizard flows
- [ ] notify-rs vs Chokidar file detection latency

### Phase 1: Secret Scanner (2-3 weeks)

Lowest-risk, highest-impact first check to port:

- Port secret scan patterns to Rust (regex crate)
- Entropy calculation in Rust
- N-API binding for `engine.scanSecrets(files)`
- Benchmark against current JS implementation
- Feature-flag: `ANVIL_RUST_ENGINE=1` to opt in

**Why first:** Self-contained, no AST parsing needed, pure regex + math.
Easy to validate correctness. 40x expected speedup is immediately
measurable.

### Phase 2: Architecture + Anti-Pattern Checks (3-4 weeks)

- tree-sitter integration for TypeScript/JavaScript parsing
- Dependency graph construction in Rust
- Anti-pattern rule evaluation
- N-API bindings for architecture checks

### Phase 3: Watcher (2-3 weeks)

- notify-rs file watching
- Adaptive debouncer
- git2 integration for status filtering
- Gate runner orchestration (parallel check execution)
- Cache layer for pre-commit lookups

### Phase 4: Kindling Storage (2-3 weeks)

- rusqlite observation store
- Query API with scope enforcement
- Retention pruning
- Migrate from @kindling/store-sqlite

### Phase 5: TUI (3-4 weeks)

- eddacraft-tui shared crate (Ratatui components, theme)
- Watch dashboard
- Gate result viewer
- APS onboarding wizard

### Phase 6: Lint Integration (2-3 weeks)

- oxlint integration or custom lint rules
- Pre-commit hook optimization (cache lookup)
- CI provenance verification

**Total estimated timeline: 15-20 weeks (incremental, feature-flagged)**

Each phase delivers independently. The TypeScript CLI works throughout.
Rust components are opt-in behind feature flags until validated.

## Consequences

### Positive

- **14x faster watch cycle** (2.9s → 0.2s) — live feedback as you code
- **50x more policy headroom** — run hundreds of rules in the time
  currently spent on a handful
- **Near-instant commits** — pre-commit checks become cache lookups
- **Faster CI** — lint/format/secret checks become provenance verification
- **Smaller binaries** — Rust TUI + engine: ~15MB vs Bun TUI: ~106MB
- **Shared product family components** — TUI, engine, storage across
  Anvil, APS, and Kindling
- **Lower memory** — watcher at ~10-20MB RSS vs ~50-100MB (Node.js)

### Negative

- **Rust learning curve** — team needs Rust proficiency
- **Two languages** — TypeScript CLI + Rust engine (mitigated: clear
  boundary, N-API bridge)
- **Build complexity** — Cargo + pnpm (mitigated: Rust components
  pre-built as native modules)
- **15-20 week investment** — significant effort, though incremental
- **N-API maintenance** — binding layer between TypeScript and Rust
  needs upkeep

### Neutral

- **Supersedes ADR-005, ADR-008, ADR-009** — those decisions assumed
  an all-TypeScript stack. If Rust enters for the engine, the rationale
  for Ink over Ratatui changes fundamentally.
- **Existing TypeScript code is not thrown away** — 30+ CLI commands,
  services, and business logic remain TypeScript. Rust handles the
  computation layer underneath.

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| Rust performance estimates are optimistic | Low | Medium | Phase 0 spike validates with real benchmarks |
| N-API binding complexity higher than expected | Medium | Medium | Spike includes N-API round-trip test; fallback to subprocess |
| tree-sitter TypeScript parsing has edge cases | Medium | Low | Incremental rollout, JS parser as fallback |
| Team struggles with Rust | Medium | High | Phase 1 is self-contained; evaluate team velocity before committing to later phases |
| oxlint rule coverage doesn't match ESLint | Medium | Medium | Run both in parallel during transition; custom rules fill gaps |

## Alternatives Considered

### A: Stay All-TypeScript (Status Quo)

Keep current architecture. Accept 3-15 second gate cycles.
Optimise with parallelisation and caching (ADR-009 recommendations).

**Why not:** Parallelising JS checks saves ~40% on full gate but doesn't
change the fundamental ceiling. Hooks and CI checks cannot move to the
watcher because the watcher is too slow. The watch cycle floor with JS
is ~1-2 seconds even with all optimisations.

### B: OpenTUI + Bun for TUI Only

Use OpenTUI for TUI components, keep engine in TypeScript.

**Why not:** 106MB binary for a TUI. Doesn't address the real bottleneck
(check execution speed). Adds Bun as a runtime dependency. Doesn't serve
Kindling or the engine.

### C: Full Rust Rewrite

Rewrite everything — CLI, TUI, engine, storage — in Rust.

**Why not:** 30+ CLI commands don't benefit from Rust. The command layer
is I/O-bound (spawn processes, read files, print output). Rewriting it
adds months of work for no performance gain. The hybrid approach gets
95% of the benefit at 30% of the cost.

### D: Go Instead of Rust

Use Go for the engine. Bubbletea for TUI.

**Why not:** Go's GC introduces latency spikes in tight loops (policy
evaluation, AST traversal). Rust's zero-cost abstractions and
predictable performance are better suited for a real-time watcher.
Go would be ~5-10x faster than JS (vs Rust's 10-100x).

## When to Make This Decision

This decision should be made after the Phase 0 spike validates:

1. tree-sitter parsing speed meets targets
2. N-API binding overhead is acceptable
3. Ratatui component library is sufficient for TUI needs
4. Team is comfortable with Rust for the engine scope

If the spike fails to meet targets, fall back to Alternative A
(status quo with JS optimisations).

## References

- ADR-005: Ink over OpenTUI (superseded if Rust enters stack)
- ADR-008: Ink vs Ratatui assessment (superseded if Rust enters stack)
- ADR-009: Watch mode performance analysis (correct analysis, wrong question)
- [oxlint benchmarks](https://oxc.rs/docs/guide/usage/linter.html)
- [tree-sitter](https://tree-sitter.github.io/tree-sitter/)
- [Ratatui](https://ratatui.rs/)
- [notify-rs](https://github.com/notify-rs/notify)
- APS onboarding design: `anvil-plan-spec/docs/plans/2026-02-27-onboarding-design.md`
