<!--
APS Module: Rust Engine Ports
==============================
Port existing Anvil checks to Rust for speed: secret scan, anti-patterns,
architecture, command safety. Uses kernel's tree-sitter/graph infrastructure.

Scopes: RENG (main)
-->

# Rust Engine Ports

| ID   | Owner | Status      |
| ---- | ----- | ----------- |
| RENG | —     | In Progress |

## Purpose

Port existing Anvil checks from TypeScript to Rust for performance. These checks
already work — the goal is speed, not new functionality.

**Why:** The current checks run in Node.js at 200-2000ms each. Ported to Rust,
they run at 5-100ms — a 10-40x speedup that makes real-time check execution
viable. Some checks merge into the kernel (architecture check uses the dependency
graph natively), while others run alongside as independent Rust functions (secret
scan, command safety).

**Kernel dependency:** RENG uses the kernel's tree-sitter parser and graph
infrastructure (KERN module). Checks that need ASTs or the dependency graph call
into kernel APIs rather than reimplementing parsing.

## In Scope

- Port secret scan to Rust (regex + entropy, self-contained)
- Port anti-pattern detection to Rust (uses kernel's tree-sitter ASTs)
- Port command safety check to Rust (string analysis, self-contained)
- Merge architecture check into kernel's dependency graph (not a separate port)
- Benchmark all ported checks vs JS originals
- Feature flag + dual-run mode for validation during rollout

## Out of Scope

- Kernel implementation (see KERN module)
- TUI rendering (see RATS module)
- ESLint replacement (ESLint stays, oxlint is a separate concern)
- Test execution (Vitest stays JS — user code is JS)
- Coverage instrumentation (stays JS)
- Kindling observation storage

## Interfaces

**Depends on:**

- KERN Phase 1 — tree-sitter parser infrastructure for AST-based checks
- KERN Phase 2 — semantic graph for architecture check merge
- `anvil-kernel-types` — shared types
- Current TS check implementations — reference for parity validation

**Exposes:**

- Rust check functions callable from the `anvil` binary
- `--engine rust/legacy/dual` flag for engine selection during rollout
- Benchmark results for speedup validation

## Constraints

- Must maintain exact parity with JS check results during transition
- Dual-run mode (Rust + JS in parallel) for validation before cutover
- Each ported check is independently feature-flagged
- Checks that don't need the graph (secret scan, command safety) can be ported
  before KERN Phase 2

## Ready Checklist

Change status to **Ready** when:

- [x] KERN Phase 1 (parser infrastructure) is complete
- [x] Cargo workspace structure agreed
- [x] Current TS check test fixtures identified for parity validation

---

### RENG-001: Port secret scan to Rust

- **Status:** Done
- **Intent:** Port all secret detection regex patterns and Shannon entropy
  calculation to Rust. This is self-contained — no AST parsing needed.
- **Expected Outcome:** Rust secret scanner produces identical results to JS
  implementation on the full test fixture set
- **Validation:** Run both implementations on test fixtures, diff results.
  ~40x speedup expected.
- **Files:** `crates/anvil-checks/src/secret/`
- **Confidence:** high
- **Priority:** High
- **Dependencies:** None (self-contained, can start before KERN)

---

### RENG-002: Port anti-pattern detection to Rust

- **Status:** Done
- **Intent:** Port anti-pattern pattern matching to Rust, operating on
  tree-sitter ASTs from the kernel's parser. Covers all 7+ high-confidence
  patterns plus HTML/CSS anti-patterns.
- **Expected Outcome:** Same warnings produced as JS anti-pattern scanner
- **Validation:** Run both implementations on test fixtures, diff results.
  ~25x speedup expected.
- **Files:** `crates/anvil-checks/src/antipattern/`
- **Confidence:** high
- **Priority:** High
- **Dependencies:** KERN-011 (tree-sitter parsing)

---

### RENG-003: Port command safety check to Rust

- **Status:** Done
- **Intent:** Port AI command safety validation to Rust. String analysis and
  pattern matching — self-contained, no AST needed.
- **Expected Outcome:** Same safety verdicts as JS implementation
- **Validation:** Run both implementations on test fixtures, diff results.
  ~25x speedup expected.
- **Files:** `crates/anvil-checks/src/command_safety/`
- **Confidence:** high
- **Priority:** Medium
- **Dependencies:** None (self-contained, can start before KERN)

---

### RENG-004: Validate architecture check parity with kernel invariants

- **Status:** In Progress
- **Intent:** Validate that the kernel's policy engine (KERN-032 invariants)
  produces equivalent architecture violation results to the current JS
  architecture check. RENG-004 owns the parity validation and gap analysis —
  the invariant implementation itself lives in KERN-032.
- **Expected Outcome:** Parity report showing kernel invariant output matches
  current JS architecture check output on all test fixture repos, with any
  gaps documented and tracked
- **Validation:** Dual-run comparison on test fixture repos with known
  architecture violations, zero unexplained discrepancies
- **Files:** Part of KERN-032 (kernel invariants), not a separate crate
- **Confidence:** high
- **Priority:** High
- **Dependencies:** KERN-032 (H1 invariants)

---

### RENG-005: Benchmark all ported checks vs JS

- **Status:** Done
- **Intent:** Run criterion.rs benchmarks for all ported checks alongside their
  JS originals to validate speedup estimates
- **Expected Outcome:** Benchmark report showing actual speedup factors for each
  check type
- **Validation:** Side-by-side benchmarks on 100+ representative files
- **Files:** `crates/anvil-checks/benches/checks.rs`
- **Confidence:** high
- **Priority:** Medium
- **Dependencies:** RENG-001, RENG-002, RENG-003

---

### RENG-006: Feature flag + dual-run for ported checks

- **Status:** Draft
- **Intent:** Add `--engine rust/legacy/dual` flag so ported checks can be
  validated against JS originals before cutover. In dual mode, both engines run
  and results are diffed.
- **Expected Outcome:** Clean engine selection via CLI flag, dual mode diffs
  output and reports discrepancies
- **Validation:** All three modes work correctly, dual mode catches intentionally
  introduced discrepancies
- **Files:** `apps/anvil-cli/src/` (TS CLI integration for `--engine` flag), integration with KERN-042
- **Confidence:** high
- **Priority:** High
- **Dependencies:** RENG-005, KERN-042

---

## Performance Targets

| Check | Current (Node.js) | Target (Rust) | Expected Speedup |
| ----- | ----------------- | ------------- | ---------------- |
| Secret scan | 200-800ms | 5-20ms | 40x |
| Anti-pattern check | 500-2000ms | 20-100ms | 25x |
| Command safety | 100-500ms | 5-20ms | 25x |
| Architecture check | 500-2000ms | 20-100ms | 25x (merged into kernel) |

## Risks

| Risk | Likelihood | Impact | Mitigation |
| ---- | ---------- | ------ | ---------- |
| Parity gaps with JS checks | Medium | High | Extensive fixture-based comparison |
| Regex behaviour differences | Low | Medium | Use same pattern syntax, test edge cases |
| Anti-pattern AST differences | Medium | Medium | Snapshot testing with insta |
| Architecture check merge complexity | Medium | Medium | Leverage existing kernel invariant framework |

## Stats

| Item | Status |
| ---- | ------ |
| RENG-001 Secret scan | Done |
| RENG-002 Anti-pattern detection | Done |
| RENG-003 Command safety | Done |
| RENG-004 Architecture check merge | In Progress (invariants done, parity validation pending) |
| RENG-005 Benchmarks | Done |
| RENG-006 Feature flag + dual-run | Draft (KERN-042 unblocked) |
| **Total** | **6 items (4/6 done)** |
