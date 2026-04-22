<!--
APS Module: Anvil Scanner Parity Gaps
=====================================
Closes the correctness defects surfaced by the council review of
RSCAN-008 (commit f17a074e). Follows ADR-026. See: plans/aps-rules.md
-->

# Anvil Scanner Parity Gaps

| ID  | Owner | Status   |
| --- | ----- | -------- |
| SPG | —     | Proposed |

## Purpose

Close the correctness defects in the authoritative Rust scanner that the
`anvil-rust-scanner` (RSCAN) module left unresolved and that the
`/council-full` review of commit `f17a074e` (RSCAN-008) surfaced:

1. Several default rules silently emit zero matches in the Rust scanner
   because their registry patterns use PCRE lookaround that the `regex`
   crate cannot compile. Users running `anvil check` on code that trips
   these rules see no warning; the TS scanner (VSCode, MCP) does fire
   them. This is a behaviour asymmetry that breaks ADR-026's authoritative
   framing.
2. The `flags: "i"` case-insensitive flag on DD-004 and RL-001..006 is
   silently dropped by `compiled_to_antipattern` in
   `crates/anvil-checks/src/antipattern/registry_loader.rs`. The Rust
   scanner matches case-sensitively on patterns the schema and TS engine
   treat as case-insensitive.
3. Pattern compilation failures are swallowed (`Regex::new(...).ok()`).
   When a rule can't compile under the Rust engine, the scanner records
   `primary_regex: None` and emits no diagnostic.
4. The RSCAN-007 parity fixture set covers 6 of 18 rules. The remaining
   12 were skipped because fixtures would expose (1)–(3) above. Parity
   is therefore asserted on the subset both engines happen to agree on.
5. RSCAN-005's validation criterion named
   `cargo bench -p anvil-bench --bench antipattern_scan`; that benchmark
   does not exist. The rayon parallelism is real, but the performance
   claim has no benchmark artifact.

Authoritative ADR: [ADR-026](../decisions/026-rust-scanner-authoritative.md).

## Background

RSCAN landed in 2026-04-21 and made the Rust scanner authoritative over the
compiled registry. The scanner works correctly on the rules it can run,
and the parity harness (RSCAN-007) proves agreement on those rules. But
the adversarial council reviewer surfaced that a non-trivial slice of the
default catalogue is silently dead in Rust: six default rules (DD-001,
DD-002, DD-003, GS-001, RL-001, RL-005) never fire because their regex
uses PCRE features the `regex` crate rejects, and seven rules that do
compile (DD-004, RL-001..006) are semantically incorrect under
case-varied input because the loader drops `flags: "i"`.

This module closes the gap. After it lands, every enabled registry rule
either fires correctly in the Rust scanner or is explicitly marked as
TS-only in the registry itself (and the scanner warns when it's run).

## Scope

**In scope:**

- Rewrites or alternate-pattern fallbacks for the 6 lookaround-affected
  default rules
- Honouring `flags: "i"` in `registry_loader.rs::compiled_to_antipattern`
- Surfacing compile failures as structured warnings instead of
  silent drops
- Fixture coverage for every registry rule that both engines can run
- A real `antipattern_scan` benchmark in `anvil-bench` backing the
  parallel-scan throughput claim
- Updates to `tests/scanner-parity/README.md` to reflect closed gaps

**Out of scope:**

- Switching Rust to a PCRE-style regex engine (would require replacing
  `regex` with `fancy-regex` or similar; cross-cutting)
- Retiring the TS scanner (separate module: `anvil-ts-scanner-retirement`)
- Changes to the `.anvil` schema (frozen per ANVFMT)

## Interfaces

**Depends on:**

- `crates/anvil-checks/src/antipattern/` — Rust scanner and loader
- `patterns/` — `.anvil` source rules that may need rewrites
- `patterns/compiled/registry.json` — compiled artifact
- `tests/scanner-parity/` — parity harness from RSCAN-007
- `crates/anvil-bench/` — benchmark harness

**Exposes:**

- A parity-complete authoritative Rust scanner
- A named benchmark artifact for CI performance regression guards

## Tasks

### SPG-001: Honour `flags: "i"` in the Rust registry loader

- **Intent:** The `flags` field on registry `detection.regex` entries is
  load-bearing; dropping it changes semantics. Rust must honour at least
  the `i` flag to match `.anvil` spec behaviour.
- **Expected Outcome:** `compiled_to_antipattern` in
  `crates/anvil-checks/src/antipattern/registry_loader.rs` reads the
  `flags` field from `Detection::Regex` and constructs regexes via
  `RegexBuilder::new(pattern).case_insensitive(flags.contains('i'))`.
  A unit test pins `flags: "i"` behaviour on a case-varied input and
  confirms the Rust scanner matches what the TS scanner does.
- **Scope:** `crates/anvil-checks/src/antipattern/registry_loader.rs`,
  `crates/anvil-checks/src/antipattern/scanner.rs` (prepare step)
- **Dependencies:** —
- **Validation:** `cargo test -p eddacraft-anvil-checks registry_loader`;
  new fixture in `tests/scanner-parity/fixtures.json` with case-varied
  content for RL-002 passes on both engines
- **Confidence:** high
- **Priority:** High
- **Status:** Proposed
- **Origin:** adversarial-reviewer F3; security-analyst verified
  `Detection::Regex { pattern, .. }` discards flags at
  `registry_loader.rs:308`

---

### SPG-002: Surface registry pattern compile failures as warnings

- **Intent:** When a registry rule's regex fails to compile in the Rust
  engine, today the scanner stores `primary_regex: None` and emits
  nothing. Operators cannot tell the difference between "rule ran, no
  matches" and "rule never ran". Fix the observability.
- **Expected Outcome:** `prepare_pattern` emits a structured diagnostic
  (log + optional scan-result warning) when a registry rule cannot
  compile, listing the rule ID, the compile error, and a pointer to
  `tests/scanner-parity/README.md`. The diagnostic is gated by a config
  flag (off by default in CI output to avoid noise, on by default in
  `anvil doctor`).
- **Scope:** `crates/anvil-checks/src/antipattern/scanner.rs`,
  `crates/anvil-cli/src/commands/doctor.rs` (surface the diagnostic)
- **Dependencies:** —
- **Validation:** `cargo test -p eddacraft-anvil-checks scanner`;
  manual run with a deliberately broken pattern shows the diagnostic in
  `anvil doctor`
- **Confidence:** high
- **Priority:** High
- **Status:** Proposed
- **Origin:** adversarial-reviewer F3; general-reviewer (correction of
  "skipped" language in docs)

---

### SPG-003: Rewrite or gate the 6 lookaround-affected default rules

- **Intent:** DD-001, DD-002, DD-003, GS-001, RL-001, and RL-005 use
  PCRE lookaround their authors needed to avoid false positives. Today
  they silently do not fire in the authoritative Rust scanner.
  Either rewrite each pattern with a lookaround-free equivalent and add
  a post-match filter in the scanner, or mark them `ts_only: true` in
  the registry so the Rust engine skips them with an explicit signal.
- **Expected Outcome:** For each of the six rules, pick one of:
  (a) replace the lookaround with a character-class / alternation /
  post-filter that compiles under the `regex` crate and matches the
  same intended set on the existing TS fixtures, **or**
  (b) add a `ts_only: true` flag to the `.anvil` source and update
  `compiled_to_antipattern` to respect it (skipping the rule in Rust
  with a named reason rather than a silent drop).
  Fixture coverage added for every rule that now runs in Rust.
- **Scope:** `patterns/deferred-debt/DD-001.anvil`,
  `patterns/deferred-debt/DD-002.anvil`,
  `patterns/deferred-debt/DD-003.anvil`,
  `patterns/guardrail-suppression/GS-001.anvil`,
  `patterns/responsibility-laundering/RL-001.anvil`,
  `patterns/responsibility-laundering/RL-005.anvil`;
  possibly `crates/anvil-checks/src/antipattern/registry_loader.rs`
  (schema change); `tests/scanner-parity/fixtures.json`
- **Dependencies:** SPG-002 (so the fallback path is observable)
- **Validation:** `pnpm test:scanner-parity` covers every rule with at
  least one positive and one negative fixture; no silent Rust drops
- **Confidence:** medium
- **Priority:** High
- **Status:** Proposed
- **Origin:** adversarial-reviewer F3

---

### SPG-004: Fixture coverage for every registry rule

- **Intent:** RSCAN-007 landed with 6 fixtures covering 6 rules; 12
  rules have no fixture. Close the coverage gap.
- **Expected Outcome:** `tests/scanner-parity/fixtures.json` contains at
  least one positive and one negative fixture per registry rule for
  every rule both engines can run after SPG-001 + SPG-003. The README
  "Known divergence" list is either empty or explicitly enumerates only
  rules that are `ts_only` by design.
- **Scope:** `tests/scanner-parity/fixtures.json`,
  `tests/scanner-parity/README.md`
- **Dependencies:** SPG-001, SPG-003
- **Validation:** `pnpm test:scanner-parity` passes; count of fixtures
  ≥ 2 × count of Rust-runnable rules
- **Confidence:** high
- **Priority:** Medium
- **Status:** Proposed
- **Origin:** adversarial-reviewer F3; pragmatic-lead (parity README
  forward reference)

---

### SPG-005: Named `antipattern_scan` benchmark in anvil-bench

- **Intent:** RSCAN-005's validation criterion named
  `cargo bench -p anvil-bench --bench antipattern_scan`. That
  benchmark does not exist. The parallel-scan claim has no CI guard.
- **Expected Outcome:** A new `benches/antipattern_scan.rs` under
  `crates/anvil-bench/` (or an antipattern scenario added to the
  existing `stress.rs`) that exercises the rayon scan loop across a
  synthetic corpus of ≥ 200 artifacts, measuring wall time vs thread
  count. Documented baseline recorded in the module README or
  `docs/architecture/kernel-benchmarking-spec.md`.
- **Scope:** `crates/anvil-bench/benches/`, `crates/anvil-bench/src/`
  (scenario code if needed)
- **Dependencies:** —
- **Validation:** `cargo bench -p anvil-bench --bench antipattern_scan`
  runs and reports throughput across thread counts; regression guard
  note added to `docs/guides/release-runbook.md` preflight
- **Confidence:** medium
- **Priority:** Medium
- **Status:** Proposed
- **Origin:** adversarial-reviewer F5

---

### SPG-006: Registry integrity / trust-boundary documentation

- **Intent:** `ANVIL_REGISTRY_PATH` is a trust boundary — the loader will
  read a registry from any path the env var points to. The docs do not
  name it as a trust boundary and there's no integrity check on the
  registry payload. A poisoned registry could silently disable rules.
- **Expected Outcome:** `docs/guides/anvil-rule-authoring.md` gains a
  "Registry integrity" subsection naming `ANVIL_REGISTRY_PATH` as a
  trust boundary, recommending that CI use the in-tree registry
  compiled at the checked-out SHA. Consider (but do not require here)
  a `--expect-registry-hash` CLI flag; the design decision goes in an
  ADR rather than straight code.
- **Scope:** `docs/guides/anvil-rule-authoring.md`;
  optionally `plans/decisions/` (new ADR if the hash-flag is pursued)
- **Dependencies:** —
- **Validation:** grep finds the "Registry integrity" heading; ADR
  lands (or a closed-no-change note is recorded)
- **Confidence:** high
- **Priority:** Medium
- **Status:** Proposed
- **Origin:** security-analyst NIT #1

## Risks

- **PCRE-free rewrites may change the matched set.** SPG-003 rewrites
  are allowed to shrink or grow the matched set; the validation is that
  every existing TS fixture result is preserved. If a rewrite admits a
  false positive the fixture set will flag it; if it admits a false
  negative, the TS scanner will still catch it until TS retires.
- **Schema churn if `ts_only` is added.** Adding a registry field is a
  schema change. Coordinate with ANVFMT; bump `schema_version` if so.

## Milestones

- **M1 (SPG-001, SPG-002):** Rust scanner is honest — flags honoured,
  failures observable. No behaviour change on compiling rules.
- **M2 (SPG-003, SPG-004):** All registry rules either fire correctly
  in Rust or are explicitly `ts_only`; fixtures cover every runnable
  rule.
- **M3 (SPG-005, SPG-006):** Benchmark artifact lands; trust-boundary
  doc lands.

## Progress Log

- **2026-04-21 — module proposed.** Created from council review of
  RSCAN-008 commit `f17a074e`. Surfaced by adversarial-reviewer (F3,
  F5), general-reviewer (scanner.rs silent-drop finding), and
  security-analyst (registry integrity NIT).
