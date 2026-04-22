# Command Safety Surfaces

| ID     | Owner | Status   | Progress |
|--------|-------|----------|----------|
| CMDSH  | —     | Complete | 4/4      |

## Purpose

Define how command safety should operate as a shared capability across Anvil's
preflight and runtime enforcement surfaces. The Rust command-safety engine
already exists in `crates/anvil-checks/`, but it is currently shaped around
script-plan analysis and is not yet integrated into the current `anvil gate`
contract or the future intercept daemon. Without a deliberate shared-surface
design, Anvil risks creating one command-safety path for plan review and a
separate one for live agent command interception.

This module establishes the architectural shape for one canonical command-
safety capability reused by multiple surfaces, with clear rules for when it
applies, what it analyses, how it reports findings, and how workflow judgement
differ from preflight gate evaluation and live interruption.

## In Scope

- Inventory the current Rust command-safety engine, result model, and tests
- Define where command safety participates in `check`, `gate`, and future
  intercept/daemon flows
- Define the input contracts for command-safety evaluation:
  script plans, executable plan sections, live issued commands, and future agent
  command streams
- Define shared finding, explanation, and suppression semantics for command
  safety
- Define how command-safety results roll up into preflight gate judgement versus
  live allow/warn/block/interrupt decisions
- Identify follow-on implementation slices for gate integration and intercept
  reuse

## Out of Scope

- Implementing the full intercept daemon or transport
- Rewriting the command-safety rule engine itself
- Reworking shell parser internals unless a design constraint requires it
- Web dashboard UX for command-safety events
- Non-Anvil sandboxing or container execution policy

## Interfaces

**Depends on:**

- `crates/anvil-checks/src/command_safety/` — existing Rust capability
- `check-language-and-onboarding` (CLAR) — checks/findings/gates language model
- `intercept-daemon` (INTD) — future runtime interruption architecture
- `notification-framework` (NOTIFY) — delivery/escalation semantics for warn /
  block / interrupt outputs

**Exposes:**

- Shared architectural contract for command safety across gate and intercept
- Input-model decision for plan/script preflight and live agent command checks
- Follow-on implementation tasks for runtime surfaces

## Acceptance Criteria

- [x] A discovery/design document exists describing command safety as one shared
      capability with multiple surface adapters
- [x] The document defines which current and future surfaces should invoke
      command safety and under what inputs
- [x] The difference between preflight gate judgement and live enforcement is
      explicit
- [x] Follow-on execution tasks exist for gate integration and intercept reuse

## Constraints

- Do not create separate rule sets for plan review and live agent execution
- Reuse the existing Rust capability wherever possible
- Keep command-safety findings aligned with the broader checks -> findings ->
  gates model
- Live enforcement may have stronger actions than preflight review, but both
  must derive from the same underlying capability and taxonomy

## Tasks

### CMDSH-001: Inventory the current command-safety capability

- **Intent:** Document the current Rust implementation, result model, and input
  assumptions for command safety
- **Expected Outcome:** A discovery note summarises the existing engine,
  supported rules, `CommandSafetyCheckContext`, result types, and current test
  coverage
- **Files:** `crates/anvil-checks/src/command_safety/`,
  `crates/anvil-checks/tests/command_safety_validation.rs`
- **Validation:** `plans/specs/YYYY-MM-DD-command-safety-surfaces-design.md`
  exists with current-state inventory
- **Confidence:** high
- **Status:** Complete

### CMDSH-002: Define shared surface architecture

- **Intent:** Decide how command safety participates in preflight gate review,
  plan validation, and future live agent/intercept enforcement
- **Expected Outcome:** The design note defines which surfaces invoke command
  safety, what inputs each surface provides, and how the same capability is
  adapted without duplicating rules
- **Files:** `plans/specs/`, `docs/architecture/`, `plans/modules/intercept-daemon.aps.md`
- **Dependencies:** CMDSH-001
- **Validation:** Design note includes a surface matrix and input contract table
- **Confidence:** medium
- **Status:** Complete

### CMDSH-003: Define result and judgement model

- **Intent:** Define how command-safety findings map onto preflight pass/fail,
  warning, block, and interrupt semantics
- **Expected Outcome:** The design note defines shared finding semantics and the
  distinction between preflight gate judgement and live enforcement decisions
- **Files:** `plans/specs/`, `docs/architecture/quality-model.md`,
  `plans/modules/notification-framework.aps.md`
- **Dependencies:** CMDSH-002
- **Validation:** Design note includes finding/judgement mapping and escalation
  rules
- **Confidence:** medium
- **Status:** Complete

### CMDSH-004: Define follow-on execution slices

- **Intent:** Convert the design into bounded implementation work
- **Expected Outcome:** Follow-on tasks or modules are proposed for gate
  integration, CLI help/docs updates, and intercept reuse
- **Files:** `plans/modules/`, `plans/specs/`, `plans/index.aps.md`
- **Dependencies:** CMDSH-003
- **Validation:** Follow-on work items listed with scope and validation
- **Confidence:** medium
- **Status:** Complete
