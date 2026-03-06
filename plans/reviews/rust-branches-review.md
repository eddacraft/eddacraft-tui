# Consolidated Code Review: Rust Feature Branches

**Date:** 2026-03-05
**Reviewer:** Claude (automated, 3 parallel review agents)
**Branches reviewed:**

| Branch | Content | Size |
|--------|---------|------|
| `docs/rust-core-engine-decision-space` | ADR-011 decision document | 602 lines |
| `docs/rust-core-engine-plan` | RENG APS module plan | 525 lines |
| `feat/rust-workspace-spike-ci` | Workspace, spike crates, TUI library, CI | ~4.7k lines, 27 files |

These branches are independent (not stacked).

---

## Overall Assessment: NEEDS_CHANGES

The Rust workspace foundation is solid — idiomatic Cargo configuration, correct
unicode handling in the TUI library, safe tree-sitter FFI, and good CI
change-detection. However, the planning documents are stale relative to `main`
(which has already evolved to a KERN/RENG/RATS three-module split), and the spike
code has correctness and portability gaps that should be fixed before the pattern
gets replicated into production crates.

---

## Critical Issues (5 — must fix)

### 1. APS plan is superseded — branch is behind `main`

**Branch:** docs/rust-core-engine-plan

ADR-011 is already marked `Status: Superseded` on `main`. The `main` branch has
evolved to a three-module architecture (KERN, RENG, RATS), but this branch's
`rust-core-engine.aps.md` collapses all three into a monolithic 24-item RENG
plan. Merging would:

- Replace the correctly-scoped 6-item RENG on `main` with a 24-item plan that
  absorbs KERN and RATS scope
- Create duplicate "Future — Rust Core Engine" sections in `index.aps.md`
- Reassign all RENG IDs, breaking cross-references in the APS file map

**Action:** Close this branch as superseded, or rebase and update to describe
only what is new relative to `main`'s current state.

### 2. ADR N-API integration path contradicts KERN module

**Branch:** docs/rust-core-engine-decision-space
**File:** `plans/decisions/011-rust-core-engine.md:393-445`

The ADR proposes N-API bindings as the integration mechanism. KERN module
(already on `main`) explicitly states "N-API bindings (superseded by standalone
binary approach)." The `AnvilEngine` TypeScript import example is stale. Update
to reflect the actual embedded library API approach.

### 3. ADR crate naming contradicts existing APS modules

**Branch:** docs/rust-core-engine-decision-space
**File:** `plans/decisions/011-rust-core-engine.md:253-278`

Three different crate layouts exist across: (a) this ADR, (b) the RENG module,
and (c) the KERN module. The ADR must acknowledge this divergence, reference the
KERN/RENG modules as the implementation-level resolution, or explicitly supersede
them.

### 4. Potential panic in benchmark p99 index calculation

**Branch:** feat/rust-workspace-spike-ci
**File:** `crates/spike/src/treesitter.rs:77,141`

```rust
let p99 = durations[(iterations as f64 * 0.99) as usize];
```

No `.min(count - 1)` guard. The `notify.rs` spike correctly uses this pattern —
apply it consistently. Currently safe with `iterations = 1000` but a latent panic
if iterations changes.

### 5. `/proc/self/statm` is Linux-only with no platform guard

**Branch:** feat/rust-workspace-spike-ci
**File:** `crates/spike/src/petgraph.rs:140-147`

`get_rss_bytes()` reads `/proc/self/statm` and uses `rustix::param::page_size()`
— both Linux-only. No `#[cfg(target_os = "linux")]` guard exists. The `rustix`
dependency in `Cargo.toml` has no platform restriction and will fail to compile
on Windows. Either add a `cfg` guard or restrict the dependency to
`[target.'cfg(unix)'.dependencies]`.

---

## Major Suggestions (11 — should fix)

### ADR-011 (docs/rust-core-engine-decision-space)

| # | Location | Issue |
|---|----------|-------|
| 1 | Line 491 | "14x faster watch cycle" contradicts the document's own 8.5x calculation at line 209 |
| 2 | Lines 223-252 | CI provenance reduction defers the security-critical integrity model with no concrete design — either scope it out or provide a model |
| 3 | Lines 74-89 | oxlint treated as ESLint replacement in performance table, but RENG module explicitly says "ESLint stays, oxlint is a separate concern" — reconcile or adjust the 10x claim |
| 4 | Lines 473-481 | Phase 6 (Lint Integration) conflicts with RENG's out-of-scope declaration for ESLint replacement |
| 5 | Throughout | No cross-reference to KERN, RENG, or RATS APS modules — readers cannot find implementation detail |

### Rust Workspace + Spike + CI (feat/rust-workspace-spike-ci)

| # | Location | Issue |
|---|----------|-------|
| 6 | CI workflow | `dtolnay/rust-toolchain@master` is unpinned — pin to SHA or tag for reproducible CI |
| 7 | CI workflow | Missing `timeout-minutes` on Rust job — default is 6 hours |
| 8 | `text_input.rs` | `TextInput::render` never renders a visible cursor — widget is functionally incomplete |
| 9 | `handler.rs:42` | `'q'` mapped to `Action::Quit` unconditionally — conflicts with `TextInput` when typing. Same issue with vim bindings (`j`, `k`, `h`, `l`). Needs input-mode concept |
| 10 | `crates/spike/src/` | Zero `#[cfg(test)]` blocks — `cargo test` passes vacuously. Add smoke assertions for symbol extraction, RSS measurement, and query captures |
| 11 | `eddacraft-tui/Cargo.toml` | `insta` declared as dev-dependency but no snapshot tests exist — either add them or remove the dependency |

---

## Minor Notes (13 — nice to fix)

### ADR-011

| # | Summary |
|---|---------|
| 1 | Debounce baseline: 50ms target applies only to single-file case — burst debounce is still 300ms, affects watch cycle calculation |
| 2 | "50x more policy headroom" in Consequences has no back-reference to the Checks Per Window analysis |
| 3 | Binary size comparison uses Bun (~106MB) but current TUI uses Ink/Node.js (~30MB per ADR-008) — Bun was rejected in ADR-005 |
| 4 | Missing cross-reference to ADR-006 in the supersedes list — verify ADR numbering |
| 5 | "can't be Rusted" is informal — use "cannot be ported to Rust" |
| 6 | Reference URL to external private repo may not be accessible to all readers |

### APS Plan

| # | Summary |
|---|---------|
| 7 | RENG-005 validation criterion (p99 < 20ms) contradicts intent (< 10ms); RENG-002 has inverse discrepancy |
| 8 | RENG-017 cache invalidation strategy (stale fallback) mentioned in ADR but absent from acceptance criteria |
| 9 | RENG-019 has duplicate `Files:` entry for `query.rs` |

### Rust Workspace + Spike + CI

| # | Summary |
|---|---------|
| 10 | CI should use `cargo build --workspace --all-targets` explicitly rather than relying on workspace root inference |
| 11 | `rustix` is a heavy dependency for a single `page_size()` call — replace with `libc::sysconf` or hardcode 4096 for spike |
| 12 | `StatusBar` right section is not right-aligned — uses `Percentage(50)` layout but no `Alignment::Right` |
| 13 | `EngineId` inlined in `lib.rs` while all other types have dedicated modules |

---

## Positive Observations

### ADR-011

- The reframing from "does Ratatui render faster?" to "how many checks can we
  run per save window?" is the conceptual core and is well-argued.
- Full-profile analysis honestly shows ~1x speedup because Vitest/Coverage remain
  JS-bound — acknowledging where the recommendation does not help builds trust.
- "When to Make This Decision" section appropriately gates on spike outcomes
  rather than performance estimates.
- Alternative D (Go) is included with concrete rationale (GC latency in tight
  loops) rather than being dismissed.
- Watch mode data flow diagram showing "Single process. Zero IPC. Zero
  serialization" is the clearest articulation of the architectural benefit.

### APS Plan

- Phase ordering is well-considered: spike-first, lowest-risk check first
  (secret scan), parser infrastructure before dependent checks, watcher after
  checks are ready.
- Performance Targets table maps directly to ADR-011's numerical analysis and
  is specific enough to be testable.
- Feature-flag rollout correctly stated as a module-level constraint rather than
  per-item.
- Out of Scope section is precise — explicitly excludes ESLint replacement, test
  execution, and coverage instrumentation.

### Rust Workspace + Spike + CI

- **Workspace configuration is exemplary.** `[workspace.lints]`,
  `[workspace.package]`, and `[workspace.dependencies]` all used correctly.
  Shared dependencies properly hoisted.
- **`unsafe_code = "forbid"` at workspace level** — correct default for a
  codebase that should never need unsafe.
- **Edition 2024** adoption is correct (stabilised in Rust 1.85.0, toolchain
  pinned to 1.88.0).
- **`TextInputState` unicode handling** uses `char_indices()` and `len_utf8()`
  for multi-byte character navigation — one of the most common TUI bugs,
  handled correctly here.
- **`benchmark_parse` warm-up loop** (100 iterations) correctly accounts for
  tree-sitter grammar initialisation overhead.
- **CI change-detection** integrates cleanly with existing pattern. `RUST_CHANGED`
  initialised to `false` before loop — avoids "unset variable is truthy"
  footgun.
- **`SelectState::previous` uses `checked_sub`** — prevents underflow to
  `usize::MAX`.
- **`ProgressBarState::fraction` clamps to `[0.0, 1.0]`** — prevents overdraw.
- **Theme colour distinctness test** catches accidental copy-paste assignment.

---

## Recommended Fix Priority

### First pass — Critical fixes

1. Close or rebase `docs/rust-core-engine-plan` against `main`'s KERN/RENG/RATS
   split
2. Update ADR-011 to reflect standalone binary approach (not N-API) and correct
   crate naming
3. Add platform guards for Linux-only code in spike crate
4. Add `.min(count - 1)` guard for p99 index calculation

### Second pass — Major improvements

5. Pin CI action, add timeout, add spike tests
6. Reconcile ADR performance claims (14x vs 8.5x, oxlint scope)
7. Implement cursor rendering in `TextInput` or document limitation
8. Add input-mode concept to `KeyHandler`

### Follow-up items

- Snapshot tests for TUI widget rendering (justifies `insta` dependency)
- Cross-reference ADR-011 to KERN/RENG/RATS modules
- CI provenance integrity model (separate ADR or scoped-out note)
