# Actions: CPOL

| Field  | Value                                                                                             |
| ------ | ------------------------------------------------------------------------------------------------- |
| Source | [../archive/modules/contextual-policy-assertions.aps.md](../archive/modules/contextual-policy-assertions.aps.md) |
| Task   | CPOL-001..003 — assertion schema, context adapters, guidance                                      |
| Status | Done                                                                                              |

## Actions

### 1. CPOL-001 assertion schema (`crates/anvil-policy-engine/src/context/assertion.rs`)

- Scoped conditions + outcomes over `PolicyInput`; serde round-trip; fail-closed
  on unknown fields.
- **Validate:** `cargo test -p eddacraft-anvil-policy-engine -- assertion_schema`

### 2. CPOL-002 context adapters (`crates/anvil-policy-engine/src/context/adapters.rs`)

- Deterministic changed-code / workflow / config context payloads for
  assertions; no clock, no filesystem reach-out at eval time (ADR-040 D-2).
- **Validate:** `cargo test -p eddacraft-anvil-policy-engine -- assertion_context`

### 3. CPOL-003 guidance outputs (`crates/anvil-policy-engine/src/context/guidance.rs`)

- Remediation-first assertion failure explanations; align issue shape with
  `pack::ValidationIssue` conventions.
- **Validate:** `cargo test -p eddacraft-anvil-policy-engine -- assertion_guidance`
