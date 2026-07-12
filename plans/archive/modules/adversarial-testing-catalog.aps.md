# Adversarial Testing Catalog

| ID  | Owner  | Status      |
| --- | ------ | ----------- |
| ATC | @aneki | Complete |

**Last reviewed:** 2026-07-11 (post-POLRESET downstream coherence review —
module closed 2026-07-05 alongside POLRESET-009: ATC-001..004 Merged
2026-07-05 via PR #3181; archive after release-tag inclusion, with PATT)

2026-07-13: all Merged items confirmed in the v0.9.0-beta tag (record:
plans/releases/v0.9.0-beta.md) and advanced to Released/Shipped; module
ready to archive per the archive cascade.

## Purpose

Build an Anvil-native catalog of adversarial test probes to continuously
validate prompt safety, data handling, and model behaviour regressions.

## In Scope

- Probe taxonomy and metadata model
- Reusable probe packs by risk category
- Probe execution hooks via eval harness integration
- Regression trend reporting for adversarial findings

## Work Items

<!-- Audit 2026-04-26: Validation commands updated for Rust crates per ADR-026. Categorise UK English: standardise/categorise. EVAL-002 dependency satisfied — EVAL Done, items Merged 2026-06-30 via PR #3013. -->

### ATC-001: Define adversarial probe taxonomy

- **Status:** Released/Shipped via v0.9.0-beta (2026-07-12). Merged 2026-07-05 via PR #3181
- **Intent:** Standardise categories, payload classes, and expected outcomes.
- **Expected Outcome:** Probe catalog supports traceable and versioned test assets.
- **Validation:** `cargo test -p eddacraft-anvil-kernel-types -- adversarial_taxonomy`
  (green — 8 tests). Added `crates/anvil-kernel-types/src/adversarial.rs`:
  `ProbeCategory`, `PayloadClass`, `ExpectedOutcome` (each with a `#[serde(other)]
  Unknown` fallback, kebab-case wire form) and the versioned `Probe` record.
  Serde-only, additive to the wire crate.

### ATC-002: Implement probe pack registry

- **Status:** Released/Shipped via v0.9.0-beta (2026-07-12). Merged 2026-07-05 via PR #3181
- **Intent:** Add loadable probe packs with versioned manifests.
- **Expected Outcome:** Probe sets can be selected by risk profile and context.
- **Validation:** `cargo test -p eddacraft-anvil-policy -- probe_registry` (green —
  21 tests). Added `crates/anvil-policy/src/adversarial/{mod,registry}.rs`:
  `ProbePack` versioned manifest (`deny_unknown_fields` root, forward-compatible
  probe entries), `load_probe_pack`, containment-safe `discover_probe_packs`
  (canonicalise + contain, per-entry reject), and `ProbeRegistry` with
  `RiskProfile` selection and fail-closed `load` admission. Mirrors the
  policy-engine pack module. New internal dep `anvil-kernel-types` (serde-only,
  hakari-neutral).
- **Dependencies:** ATC-001

### ATC-003: Integrate probe execution into eval harness

- **Status:** Released/Shipped via v0.9.0-beta (2026-07-12). Merged 2026-07-05 via PR #3181
- **Intent:** Execute adversarial probes in CI and local eval runs.
- **Expected Outcome:** Probe outcomes appear in eval regression summaries.
- **Validation:** `cargo test -p eddacraft-anvil-policy -- adversarial_eval_integration`
  (green — 7 tests). Added `crates/anvil-policy/src/adversarial/execution.rs`:
  a `ProbeExecutor` injection point, `run_probe_pack`, and
  `ProbeRunReport::to_eval_summaries` which projects a probe run onto the
  **unchanged** `EvalRunSummary`/`EvalFinding` types — one synthetic
  `probe:<category>` suite each — so probes flow through the existing
  regression diff and store. Frozen `eval --json` v1 / EVALCI baseline shape
  untouched: no eval type gains or loses a field; the category rides in the
  existing `suite` string.
- **Dependencies:** ATC-002, EVAL-002

### ATC-004: Add adversarial trend reporting

- **Status:** Released/Shipped via v0.9.0-beta (2026-07-12). Merged 2026-07-05 via PR #3181
- **Intent:** Surface probe pass/fail trends by category over time.
- **Expected Outcome:** Teams can spot recurring weak points and regressions.
- **Validation:** `cargo test -p eddacraft-anvil -- adversarial_trends` (green —
  5 tests). Added `crates/anvil-cli/src/commands/policy/adversarial_trends.rs`:
  the pure `category_trends` reporting function reads probe (`probe:<category>`)
  runs from the eval store history, groups by category, and reports each
  category's chronological pass/fail series and current health, surfaced via a
  thin `anvil policy probe-trends` command. CLIC-010 help lints stay green.
- **Dependencies:** ATC-003

## Execution

Action plan: [../../execution/ATC.actions.md](../../execution/ATC.actions.md)
