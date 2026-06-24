# Contextual Policy Assertions

| ID   | Owner  | Status |
| ---- | ------ | ------ |
| CPOL | @aneki | Ready  |

**Last reviewed:** 2026-05-25 (APSCAN-010 canonical-heading migration)

> NOTE(post-rust): Validation commands updated from `pnpm nx test core` to
> the Rust workspace equivalents in `crates/anvil-policy`.
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

- **Status:** Ready
- **Intent:** Create a schema for contextual policy assertions.
- **Expected Outcome:** Assertions support scoped conditions and outcomes.
- **Validation:** `cargo test -p eddacraft-anvil-policy -- assertion_schema`

### CPOL-002: Implement context adapters

- **Status:** Ready
- **Intent:** Populate assertions with workflow and runtime context.
- **Expected Outcome:** Assertions evaluate with deterministic context payloads.
- **Validation:** `cargo test -p eddacraft-anvil-policy -- assertion_context`
- **Dependencies:** CPOL-001

### CPOL-003: Add assertion guidance outputs

- **Status:** Ready
- **Intent:** Provide actionable failure explanations and fix guidance.
- **Expected Outcome:** Assertion failures map to remediation-first outputs.
- **Validation:** `cargo test -p eddacraft-anvil-policy -- assertion_guidance`
- **Dependencies:** CPOL-002

## Execution

Action plan: [../execution/CPOL.actions.md](../execution/CPOL.actions.md)
