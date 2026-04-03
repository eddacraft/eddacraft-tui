# Intercept Rules

| ID | Owner | Status |
|----|-------|--------|
| INTR | @aneki | Draft |

## Purpose

The intercept rules module defines the rule evaluation contract and the initial
set of deterministic, cheap rules that run on the daemon hot path. It bridges
existing anvil-checks (secret detection, antipattern scanning) into the intercept
pipeline via a thin trait, and adds new path-deny and regex-content rules specific
to the intercept use case. All rules must execute in microseconds to hundreds of
microseconds -- no graph recomputation, no network calls, no expensive AST
analysis.

## In Scope

- InterceptRule trait definition (input: change batch + file content; output:
  allow | interrupt with reason)
- Wrapper rules for existing anvil-checks secret detection
- Wrapper rules for existing anvil-checks antipattern scanning
- PathDenyList rule (configurable list of forbidden path patterns)
- RegexContent rule (configurable regex patterns matched against changed file
  content)
- Rule registry (ordered evaluation, short-circuit on first interrupt)
- Rule configuration loading from `.anvil.yaml` enforcement block
- Observe-only mode (rules evaluate but decisions are logged, not enforced)

## Out of Scope

- Graph-assisted checks (boundary membership, symbol ownership)
- OPA policy evaluation on the hot path
- Per-rule enforcement granularity (all rules share the project enforcement mode)
- Warn or block decisions (v1 is binary: allow | interrupt)
- Rule authoring UI or wizard
- Custom rule plugin loading
- Performance benchmarking infrastructure (covered by BENCH module)

## Interfaces

- **Depends on:** anvil-checks (secret detection, antipattern scanning crates)
- **Exposes:** InterceptRule trait and rule registry for consumption by
  intercept-daemon (INTD) enforcement pipeline

## Tasks

### INTR-001: InterceptRule Trait

- **Intent:** Define the contract that all hot-path rules implement, ensuring
  consistent input/output and composability
- **Expected Outcome:** A `crates/anvil-intercept-rules/` crate added to root
  workspace; a trait accepting a change batch reference and optional file
  content, returning an allow or interrupt decision with reason metadata; trait
  is object-safe for dynamic dispatch in the rule registry
- **Validation:** `cargo test -p anvil-intercept-rules --lib trait`
- **Status:** Draft

### INTR-002: Secret Detection Wrapper

- **Intent:** Expose existing anvil-checks secret detection as an InterceptRule
  without duplicating the detection logic
- **Expected Outcome:** A thin adapter that calls anvil-checks secret scanning
  on changed file content and maps findings to interrupt decisions
- **Validation:** `cargo test -p anvil-intercept-rules --lib secret`
- **Status:** Draft

### INTR-003: Antipattern Scanning Wrapper

- **Intent:** Expose existing anvil-checks antipattern scanning as an
  InterceptRule
- **Expected Outcome:** A thin adapter that calls anvil-checks antipattern
  detection on changed file content and maps findings to interrupt decisions
- **Validation:** `cargo test -p anvil-intercept-rules --lib antipattern`
- **Status:** Draft

### INTR-004: Path Deny List Rule

- **Intent:** Allow projects to declare file paths or glob patterns that should
  never be written by agent sessions
- **Expected Outcome:** A rule that evaluates changed file paths against a
  configurable deny list; matches produce an interrupt decision with the
  matching pattern and path
- **Validation:** `cargo test -p anvil-intercept-rules --lib path_deny`
- **Status:** Draft

### INTR-005: Regex Content Rule

- **Intent:** Allow projects to declare content patterns that should trigger
  interruption when written by agent sessions
- **Expected Outcome:** A rule that applies compiled regex patterns against
  changed file content; matches produce an interrupt decision with the matching
  pattern and line context
- **Validation:** `cargo test -p anvil-intercept-rules --lib regex_content`
- **Status:** Draft

### INTR-006: Rule Registry

- **Intent:** Compose multiple rules into an ordered evaluation pipeline with
  short-circuit semantics
- **Expected Outcome:** A registry that holds registered InterceptRule
  implementations, evaluates them in order, and returns the first interrupt
  decision (or allow if all pass); supports observe-only mode where interrupt
  decisions are logged but not enforced
- **Validation:** `cargo test -p anvil-intercept-rules --lib registry`
- **Status:** Draft

### INTR-007: Rule Configuration

- **Intent:** Load rule parameters (deny lists, regex patterns, enabled checks)
  from the `.anvil.yaml` enforcement block
- **Expected Outcome:** Configuration parsed from the enforcement section of
  `.anvil.yaml`; rule instances constructed from configuration; missing config
  falls back to sensible defaults (secret detection enabled, no custom deny
  lists); regex patterns compiled once at startup and cached for the lifetime
  of the rule instance
- **Validation:** `cargo test -p anvil-intercept-rules --lib config`
- **Status:** Draft
