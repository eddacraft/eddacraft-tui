# Contextual Policy Assertions

| ID   | Owner  | Status |
| ---- | ------ | ------ |
| CPOL | @aneki | Complete |

**Last reviewed:** 2026-07-11 (post-POLRESET downstream coherence review —
`plans/reviews/2026-07-11-polreset-downstream-coherence.md`: all three items
were delivered via POLRESET-004 / PR #3139, so the module advances to Done).

2026-07-13: all Merged items confirmed in the v0.9.0-beta tag (record:
plans/releases/v0.9.0-beta.md) and advanced to Released/Shipped; module
ready to archive per the archive cascade.

> **Retarget (POLRESET-004 / ADR-098, 2026-07-04):** assertion schema,
> context adapters, and guidance live in the product-path crate,
> `crates/anvil-policy-engine` (`src/context/`), alongside `PolicyInput` —
> not in `crates/anvil-policy`, which ADR-098 AD-2 slates for eventual
> deletion once the exceptions extraction (EXCEPT-012) completes.
> Validation targets updated accordingly.
>
> **Policy-solution validation (2026-06-24):** CPOL is still Ready when scoped
> as deterministic context adapters and assertion guidance over
> `PolicyInput`/regorus. It should not wait on OPAG unless a work item needs the
> agent orchestration UX; the engine dependency is POLENG.

## Purpose

Add contextual assertion rules that evaluate agent and workflow actions with
richer runtime context while preserving Anvil policy-pack semantics.

## In Scope

- Assertion rule schema and execution model
- Context adapters for policy inputs
- Assertion violation explanations and remediation

## Work Items

### CPOL-001: Define assertion schema

- **Status:** Done
- **Intent:** Create a schema for contextual policy assertions.
- **Expected Outcome:** Assertions support scoped conditions and outcomes.
- **Validation:** `cargo test -p eddacraft-anvil-policy-engine -- assertion_schema` — 18 passed (schema in `crates/anvil-policy-engine/src/context/assertion.rs`; fail-closed serde, per-field validation, pack severity reuse).

### CPOL-002: Implement context adapters

- **Status:** Done
- **Intent:** Populate assertions with workflow and runtime context.
- **Expected Outcome:** Assertions evaluate with deterministic context payloads.
- **Validation:** `cargo test -p eddacraft-anvil-policy-engine -- assertion_context` — 12 passed (`crates/anvil-policy-engine/src/context/adapters.rs`; ADR-040 D-2 pure transforms, normalised ordering, scope + condition evaluation reporting the first unmet condition).
- **Dependencies:** CPOL-001

### CPOL-003: Add assertion guidance outputs

- **Status:** Done
- **Intent:** Provide actionable failure explanations and fix guidance.
- **Expected Outcome:** Assertion failures map to remediation-first outputs.
- **Validation:** `cargo test -p eddacraft-anvil-policy-engine -- assertion_guidance` — 6 passed (`crates/anvil-policy-engine/src/context/guidance.rs`; reuses pack `IssueSeverity`/`PolicySeverity`, stable kebab `GuidanceCode`, skip-serialised optionals, ADR-002 blocking-axis derivation).
- **Dependencies:** CPOL-002

## Execution

Action plan: [../../execution/CPOL.actions.md](../../execution/CPOL.actions.md)
