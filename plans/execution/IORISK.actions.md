# Actions: IORISK

| Field  | Value                                                                   |
| ------ | ----------------------------------------------------------------------- |
| Source | [../modules/io-risk-controls.aps.md](../modules/io-risk-controls.aps.md) |
| Task   | IORISK-001..003 — taxonomy, scanner contracts, guidance integration      |
| Status | Done                                                                    |

## Actions

### 1. IORISK-001 taxonomy (`crates/anvil-kernel-types/src/io_risk.rs`)

- Categories, severity, confidence for IO risk findings; serde types only;
  forward-compatible enums (`#[serde(other)]` fallbacks per the wire lesson).
- **Validate:** `cargo test -p eddacraft-anvil-kernel-types -- io_risk_taxonomy`

### 2. IORISK-002 scanner contracts (`crates/anvil-policy-engine/src/io_risk/pipeline.rs`)

- Provider-agnostic `Scanner` trait + deterministic chain executor over
  input/output payloads; contracts only, no concrete heavyweight scanners.
- **Validate:** `cargo test -p eddacraft-anvil-policy-engine -- io_scanner_pipeline`

### 3. IORISK-003 guidance integration (`crates/anvil-policy-engine/src/io_risk/guidance.rs`)

- Map findings to policy outcomes + remediation-first guidance consumable by
  packs and CI summaries.
- **Validate:** `cargo test -p eddacraft-anvil-policy-engine -- io_risk_guidance`
