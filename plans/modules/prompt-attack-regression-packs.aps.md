# Prompt Attack Regression Packs

| ID   | Owner  | Status      |
| ---- | ------ | ----------- |
| PATT | @aneki | Done        |

**Last reviewed:** 2026-07-11 (post-POLRESET downstream coherence review:
PATT-001..003 all Done, merged 2026-07-04 via PR #3175, module closed
alongside POLRESET-009; PATT-004 filed as a Proposed follow-up so the
live-`DefenceObserver` wiring — previously tracked nowhere — does not
evaporate; archive with ATC after release-tag inclusion)

## Purpose

Create reusable prompt-attack regression packs to validate resilience against
injection, exfiltration, and instruction-hijack scenarios before release.

## In Scope

- Attack scenario schema and fixture format
- Pack runner for local and CI use
- Severity scoring and fail-policy integration

## Work Items

<!-- Audit 2026-04-26: Validation commands updated for Rust crates per ADR-026. UK spelling applied. -->

### PATT-001: Define attack scenario schema

- **Status:** Done
- **Intent:** Standardise prompt attack case representation.
- **Expected Outcome:** Scenarios encode payload, objective, and expected safe behaviour.
- **Validation:** `cargo test -p eddacraft-anvil-kernel-types -- attack_scenario_schema`
- **Validated:** `crates/anvil-kernel-types/src/attack_scenario.rs` — `AttackScenario`
  + `AttackCategory`/`SafeBehaviour` forward-compatible enums (serde-only, kebab-case,
  `#[serde(other)] Unknown`); severity reuses `RiskSeverity`. 8 filtered tests green,
  full crate + clippy `-D warnings` + fmt clean.

### PATT-002: Build attack pack runner

- **Status:** Done
- **Intent:** Execute scenario packs deterministically across environments.
- **Expected Outcome:** Runner emits normalised outcomes and confidence metadata.
- **Validation:** `cargo test -p eddacraft-anvil-policy -- attack_pack_runner`
- **Dependencies:** PATT-001
- **Validated:** `crates/anvil-policy/src/attack/runner.rs` — `AttackPack`
  (`deny_unknown_fields`, fail-closed loader mirroring the policy-pack manifest
  taxonomy) + `run_pack` over an injected `DefenceObserver` seam; normalised
  `ScenarioOutcome`s (fail-closed pass rule, bounded `Confidence`, manifest
  order). Baseline `ConformanceObserver` shipped until a live defence is wired.
  12 filtered tests green, full crate + clippy `-D warnings` + fmt clean.

### PATT-003: Connect fail policy and CI gates

- **Status:** Done
- **Intent:** Enforce configurable pass/fail thresholds by severity.
- **Expected Outcome:** CI can block or warn based on attack regression policy.
- **Validation:** `cargo test -p eddacraft-anvil -- attack_regression_gate`
- **Dependencies:** PATT-002
- **Validated:** `crates/anvil-cli/src/commands/policy/attack_regression.rs` —
  `FailPolicy` (severity threshold, warnings-first default per ADR-002) maps a
  `PackRunReport` to a `GateDecision` (pass/warn/fail); `anvil policy
  attack-regression` CLI surface, report-only by default, `--fail-above
  <severity>` opts into blocking; fail-closed on unknown/missing severity. The
  mechanism ships **report-only** — a new required, blocking CI step is a later
  gated decision (mirrors EVALCI's report-only phase), deliberately not wired
  here. 12 filtered tests green, clippy `-D warnings` + fmt clean.

### PATT-004: Wire a live DefenceObserver (follow-up)

- **Status:** Proposed (filed 2026-07-11 — the post-POLRESET coherence review
  found this deliberate open end from PATT-002 tracked nowhere)
- **Intent:** Replace the baseline `ConformanceObserver` with a live defence
  wired into the PATT-002 `DefenceObserver` seam, so attack packs exercise a
  real defence surface instead of conformance-only baselines.
- **Expected Outcome:** `run_pack` evaluates scenarios against a live defence;
  the baseline observer remains available for hermetic tests.
- **Validation:** `cargo test -p eddacraft-anvil-policy -- attack_pack_runner`
- **Dependencies:** PATT-002; a product decision on which defence surface to
  bind first

## Execution

Action plan: [../execution/PATT.actions.md](../execution/PATT.actions.md)
