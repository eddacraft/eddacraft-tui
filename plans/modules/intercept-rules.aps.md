# Intercept Rules

| ID   | Owner  | Status | Progress |
| ---- | ------ | ------ | -------- |
| INTR | @aneki | Draft  | 0/8      |

**Last reviewed:** 2026-04-28

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
- Minimum launch reasoning-pattern rule wrapper for A1 / RMCP pre-write
  validation
- PathDenyList rule (configurable list of forbidden path patterns)
- RegexContent rule (configurable regex patterns matched against changed file
  content)
- Rule registry (ordered evaluation, short-circuit on first interrupt)
- Rule configuration loading from `.anvil.yaml` enforcement block
- Observe-only mode (rules evaluate but decisions are logged, not enforced)

## Out of Scope

- Graph-assisted checks in this module's current scope. Future boundary
  membership, symbol ownership, known-edge, or architectural-index rules must
  wait for GV2's hot-read API and stay within ADR-031 latency budgets.
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

## Graph v2 Coordination

INTR intentionally remains cheap and deterministic. Graph v2 does not change the
current INTR scope: no graph recompute, transitive traversal, context slicing,
or explanation work belongs in hot-path rule evaluation.

After GV2-022 exposes bounded hot reads, a later INTR slice may add graph-backed
rules for boundary membership, symbol ownership, known-edge existence, or
precomputed architectural-index checks. Those rules must consume GV2 hot indexes
only; they must not query the general graph registry or perform traversal on the
daemon hot path.

## Tasks

### INTR-001: InterceptRule Trait

- **Intent:** Define the contract that all hot-path rules implement, ensuring
  consistent input/output and composability
- **Expected Outcome:** A `crates/anvil-intercept-rules/` crate added to root
  workspace; a trait accepting a change batch reference and optional file
  content, returning an allow or interrupt decision with reason metadata; trait
  is object-safe for dynamic dispatch in the rule registry
- **Validation:** `cargo test -p eddacraft-anvil-intercept-rules --lib trait`
- **Status:** Draft

### INTR-002: Secret Detection Wrapper

- **Intent:** Expose existing anvil-checks secret detection as an InterceptRule
  without duplicating the detection logic
- **Expected Outcome:** A thin adapter that calls anvil-checks secret scanning
  on changed file content and maps findings to interrupt decisions
- **Validation:** `cargo test -p eddacraft-anvil-intercept-rules --lib secret`
- **Status:** Draft

### INTR-003: Antipattern Scanning Wrapper

- **Intent:** Expose existing anvil-checks antipattern scanning as an
  InterceptRule
- **Expected Outcome:** A thin adapter that calls anvil-checks antipattern
  detection on changed file content and maps findings to interrupt decisions
- **Validation:** `cargo test -p eddacraft-anvil-intercept-rules --lib antipattern`
- **Status:** Draft

### INTR-004: Path Deny List Rule

- **Intent:** Allow projects to declare file paths or glob patterns that should
  never be written by agent sessions
- **Expected Outcome:** A rule that evaluates changed file paths against a
  configurable deny list; matches produce an interrupt decision with the
  matching pattern and path
- **Validation:** `cargo test -p eddacraft-anvil-intercept-rules --lib path_deny`
- **Status:** Draft

### INTR-005: Regex Content Rule

- **Intent:** Allow projects to declare content patterns that should trigger
  interruption when written by agent sessions
- **Expected Outcome:** A rule that applies compiled regex patterns against
  changed file content; matches produce an interrupt decision with the matching
  pattern and line context
- **Validation:** `cargo test -p eddacraft-anvil-intercept-rules --lib regex_content`
- **Status:** Draft

### INTR-006: Rule Registry

- **Intent:** Compose multiple rules into an ordered evaluation pipeline with
  short-circuit semantics
- **Expected Outcome:** A registry that holds registered InterceptRule
  implementations, evaluates them in order, and returns the first interrupt
  decision (or allow if all pass); supports observe-only mode where interrupt
  decisions are logged but not enforced
- **Validation:** `cargo test -p eddacraft-anvil-intercept-rules --lib registry`
- **Status:** Draft

### INTR-007: Rule Configuration

- **Intent:** Load rule parameters (deny lists, regex patterns, enabled checks)
  from the `.anvil.yaml` enforcement block
- **Expected Outcome:** Configuration parsed from the enforcement section of
  `.anvil.yaml`; rule instances constructed from configuration; missing config
  falls back to sensible defaults (secret detection enabled, no custom deny
  lists); regex patterns compiled once at startup and cached for the lifetime
  of the rule instance
- **Validation:** `cargo test -p eddacraft-anvil-intercept-rules --lib config`
- **Status:** Draft

### INTR-008: Launch reasoning-pattern rule wrapper

- **Intent:** Give the A1 Rust MCP pre-write path at least one AI-output
  reasoning-pattern rule beyond secret detection, without expanding INTR into a
  full reasoning engine.
- **Expected Outcome:** A minimum-viable rule from `anvil-checks` reasoning
  patterns, such as AI-001 appeal-to-authority or unjustified precision, is
  exposed as an `InterceptRule`. The rule evaluates proposed content supplied by
  RMCP/RTAI the same way it evaluates file-backed content from INTD. Findings map
  to the canonical diagnostic envelope and obey the project enforcement mode.
- **Non-scope:** Full AI-001..AI-007 catalogue, false-positive tuning beyond the
  launch fixture, or LLM-based classification.
- **Validation:** Unit test triggers the rule on a fixture planning/code comment
  payload and asserts the RMCP/RTAI response contains a canonical diagnostic with
  no dependency on Node.js or `packages/mcp-server`
- **Status:** Draft
