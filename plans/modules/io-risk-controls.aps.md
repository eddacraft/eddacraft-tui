# IO Risk Controls

| ID | Owner | Status |
|----|-------|--------|
| IORISK | @aneki | Ready |

**Last reviewed:** 2026-04-26

> NOTE(post-rust): Validation commands retargeted to the Rust workspace
> (`crates/anvil-kernel-types` for taxonomy contracts, `crates/anvil-policy`
> for scanner pipeline and findings integration).
>
> **Policy-solution validation (2026-06-24):** IORISK remains Ready as a
> producer/normaliser of findings that feed POLENG result semantics. Rego policy
> integration should run through the regorus facade; OPAG is a later
> orchestration consumer, not a prerequisite for the taxonomy or scanner
> pipeline.

## Purpose

Introduce provider-agnostic input/output risk controls for prompt injection, sensitive data leakage, and unsafe response patterns.

## In Scope

- Input and output scanner contracts
- Risk taxonomy and severity model
- Policy integration for enforce/warn modes

## Work Items

### IORISK-001: Define IO risk taxonomy
- **Intent:** Standardize categories, severity, and confidence for IO risk findings.
- **Expected Outcome:** A consistent taxonomy is used across scanners and policy outputs.
- **Validation:** `cargo test -p eddacraft-anvil-kernel-types -- io_risk_taxonomy`

### IORISK-002: Implement scanner pipeline
- **Intent:** Add scanner execution pipeline for pre/post model checks.
- **Expected Outcome:** Input/output streams are evaluated through pluggable scanner chain.
- **Validation:** `cargo test -p eddacraft-anvil-policy -- io_scanner_pipeline`
- **Dependencies:** IORISK-001

### IORISK-003: Integrate risk findings with policy outputs
- **Intent:** Map IO findings to policy outcomes and remediation actions.
- **Expected Outcome:** Findings appear in unified guidance and CI summaries.
- **Validation:** `cargo test -p eddacraft-anvil-policy -- io_risk_guidance`
- **Dependencies:** IORISK-002

## Execution

Steps: [../execution/IORISK.steps.md](../execution/IORISK.steps.md)
