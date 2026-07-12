# IO Risk Controls

| ID | Owner | Status |
|----|-------|--------|
| IORISK | @aneki | Complete |

**Last reviewed:** 2026-07-11 (post-POLRESET downstream coherence review —
`plans/reviews/2026-07-11-polreset-downstream-coherence.md`: all three items
were delivered via POLRESET-004 / PR #3139, so the module advances to Done.
Concrete heavyweight scanners remain later intake — file them as new work
items, e.g. under ACTAX risk-score fusion, when prioritised).

2026-07-13: all Merged items confirmed in the v0.9.0-beta tag (record:
plans/releases/v0.9.0-beta.md) and advanced to Released/Shipped; module
ready to archive per the archive cascade.

> **Retarget (POLRESET-004 / ADR-098, 2026-07-04):** taxonomy contracts stay
> in `crates/anvil-kernel-types` (pure serde types); the scanner-contract
> pipeline and guidance integration live in `crates/anvil-policy-engine`
> (`src/io_risk/`) — not `crates/anvil-policy`, which ADR-098 AD-2 slates for
> eventual deletion once the exceptions extraction (EXCEPT-012) completes.
> First slice ships the provider-agnostic contracts and chain
> executor; concrete heavyweight scanners are later intake.
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
- **Status:** Done
- **Intent:** Standardize categories, severity, and confidence for IO risk findings.
- **Expected Outcome:** A consistent taxonomy is used across scanners and policy outputs.
- **Validation:** `cargo test -p eddacraft-anvil-kernel-types -- io_risk_taxonomy` — 8 passed (`crates/anvil-kernel-types/src/io_risk.rs`; pure serde, `#[serde(other)]` forward-compat fallbacks per ADR-096, remediation-first `RiskFinding`, no new deps).

### IORISK-002: Implement scanner pipeline
- **Status:** Done
- **Intent:** Add scanner execution pipeline for pre/post model checks.
- **Expected Outcome:** Input/output streams are evaluated through pluggable scanner chain.
- **Validation:** `cargo test -p eddacraft-anvil-policy-engine -- io_scanner_pipeline` — 7 passed (`crates/anvil-policy-engine/src/io_risk/pipeline.rs`; `Scanner` trait + deterministic `ScannerChain`, registration-order aggregation, no short-circuit, panic isolated to a separate `scanner_errors` channel).
- **Dependencies:** IORISK-001

### IORISK-003: Integrate risk findings with policy outputs
- **Status:** Done
- **Intent:** Map IO findings to policy outcomes and remediation actions.
- **Expected Outcome:** Findings appear in unified guidance and CI summaries.
- **Validation:** `cargo test -p eddacraft-anvil-policy-engine -- io_risk_guidance` — 8 passed (`crates/anvil-policy-engine/src/io_risk/guidance.rs`; posture-driven `decision_under`/`blocks_under` over a shared `EnforcementPosture`, warnings-first default per ADR-002, blocking never stored on the guidance). Folds in the CPOL `context/guidance.rs` correction: dropped the band-derived stored `blocking` field for the same posture-parameterised shape.
- **Dependencies:** IORISK-002

## Execution

Action plan: [../../execution/IORISK.actions.md](../../execution/IORISK.actions.md)
