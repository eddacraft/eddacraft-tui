# Actions: PATT

| Field  | Value                                                                                                 |
| ------ | ----------------------------------------------------------------------------------------------------- |
| Source | [../modules/prompt-attack-regression-packs.aps.md](../modules/prompt-attack-regression-packs.aps.md) |
| Task   | PATT-001..004 — attack scenario schema, pack runner, fail-policy gate, live observer                  |
| Status | In Progress (001..003 Done; 004 Proposed)                                                             |

## Actions

### 1. PATT-001 attack scenario schema (`crates/anvil-kernel-types/src/attack_scenario.rs`)

- `AttackScenario` (id, category, payload, objective, expected safe behaviour,
  version, optional severity) as pure serde wire types; closed
  `AttackCategory`/`SafeBehaviour` enums, forward-compatible (`#[serde(other)]
  Unknown` per the wire lesson); severity reuses `io_risk::RiskSeverity`.
- **Validate:** `cargo test -p eddacraft-anvil-kernel-types -- attack_scenario_schema`

### 2. PATT-002 attack pack runner (`crates/anvil-policy/src/attack/runner.rs`)

- Versioned `AttackPack` manifest (`deny_unknown_fields`, fail-closed loader
  mirroring the policy-pack manifest taxonomy) + `run_pack` over an injected
  `DefenceObserver` seam; normalised `ScenarioOutcome`s (fail-closed pass rule,
  bounded `Confidence`, deterministic manifest order). Baseline
  `ConformanceObserver` ships until a live defence-under-test is wired.
- **Validate:** `cargo test -p eddacraft-anvil-policy -- attack_pack_runner`

### 3. PATT-003 fail policy + gate (`crates/anvil-cli/src/commands/policy/attack_regression.rs`)

- Configurable `FailPolicy` (severity threshold; warnings-first default per
  ADR-002) maps a `PackRunReport` to a `GateDecision` (pass/warn/fail); CLI
  `anvil policy attack-regression` surface, report-only by default,
  `--fail-above <severity>` opts into blocking. Fail-closed on unknown/missing
  severity. No new blocking CI step — a real gate promotion is a later gated
  decision (mirrors EVALCI report-only phase).
- **Validate:** `cargo test -p eddacraft-anvil -- attack_regression_gate`

### 4. PATT-004 live defence observer (after EVALCI-009)

- Bind the existing `DefenceObserver` seam to the product-selected live defence
  after EVALCI-009 moves the CLI-only runner out of `crates/anvil-policy`.
- Keep `ConformanceObserver` for hermetic tests and do not introduce a second
  policy evaluator or interception boundary.
- **Validate:** `cargo test -p eddacraft-anvil -- attack_pack_runner`
- **Status:** Proposed; also needs the product decision on which defence surface
  to bind first.

## Completion

- [x] PATT-001 Done — schema green
- [x] PATT-002 Done — runner green
- [x] PATT-003 Done — gate green
- [ ] PATT-004 — live defence selected and wired on the post-EVALCI-009 home
