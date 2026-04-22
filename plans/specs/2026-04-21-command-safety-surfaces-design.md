# Command Safety Surfaces Design

## Purpose

Define how command safety should operate as a shared capability across Anvil's
preflight and runtime enforcement surfaces.

The Rust command-safety engine already exists, but today it is shaped around
script-plan analysis and is not integrated into the current `anvil gate`
contract or the future intercept daemon. This document defines one shared
capability with multiple surface adapters rather than separate rule systems for
plan review and live agent command interception.

## Summary

Command safety should be modelled as:

- one **shared capability** for analysing commands against deterministic safety
  rules
- multiple **surface adapters** that feed different inputs into that capability
- distinct **judgement policies** depending on surface
- one **shared finding taxonomy** and explanation model

This lets Anvil reuse the same command-safety rules for:

- preflight gate review of executable plans
- future live agent command validation in the intercept loop
- later CLI and dashboard explanation/reporting surfaces

## Current Capability Inventory

### Existing Rust engine

Current implementation lives in:

- `crates/anvil-checks/src/command_safety/check.rs`
- `crates/anvil-checks/src/command_safety/parser.rs`
- `crates/anvil-checks/src/command_safety/matcher.rs`
- `crates/anvil-checks/src/command_safety/rules/`
- `crates/anvil-checks/src/command_safety/types.rs`
- `crates/anvil-checks/tests/command_safety_validation.rs`

### What it does today

- extracts commands from `ScriptPlan` proposed changes
- parses shell commands and compound commands
- matches them against deterministic git/filesystem safety rules
- produces blocked and warning findings
- formats separate blocked and warning messages
- exposes score, pass/fail, summary, and skip semantics

### Current input contract

The orchestration entrypoint is `run_command_safety_check` and it expects a
`CommandSafetyCheckContext`:

- `plan: Option<ScriptPlan>`
- `check_config: Option<CommandSafetyConfig>`
- `workspace_root: Option<String>`

This means the current capability is naturally aligned with executable plan
review rather than general repository file analysis.

### Current result contract

The engine emits `CommandSafetyCheckResult` with:

- `passed`
- `score`
- `message`
- `blocked: Vec<CommandSafetyFinding>`
- `warnings: Vec<CommandSafetyFinding>`
- `summary`
- `details`
- formatted blocked/warning message strings
- `skipped`

The findings already carry the important shared information:

- command text
- rule id
- category
- action
- severity
- reason
- suggestion
- references
- source

## Shared Architecture

### Four-layer model

Command safety fits best into a four-layer shared architecture:

1. **Coverage / substrate layer**
   Determines what inputs Anvil can inspect at all. This follows the broader
   coverage design in `2026-04-08-language-and-coverage-design.md`: command
   safety is not a language anchor, but a governance capability over executable
   command artefacts.
2. **Capability layer**
   The command-safety engine itself: parsing, rule matching, finding creation,
   scoring, explanation.
3. **Decision layer**
   Maps findings onto pass/fail/warn/block/interrupt semantics depending on the
   caller.
4. **Surface layer**
   `gate`, future intercept, explain/reporting surfaces, machine-readable event
   output.

This is the right shape because the capability should be shared while the
delivery and enforcement policy differ by surface.

## Surface Matrix

| Surface | Role | Should Use Command Safety? | Input Shape | Output Shape |
| --- | --- | --- | --- | --- |
| `anvil gate <plan>` | Preflight workflow judgement | Yes | Executable plan / script changes | Gate result + findings |
| `anvil check` | Repo/content analysis | Not by default | File/content scan | N/A unless a future command artefact mode is defined |
| `anvil validate` | APS/spec validation | No direct rule execution | Document structure | Structural issues only |
| Intercept daemon | Live runtime enforcement | Yes | Issued command stream from agents | allow / warn / block / interrupt + findings |
| Explain/reporting | Inspection and diagnostics | Yes | Existing findings/results | Human-readable explanation |
| Dashboard / audit trail | Observability and review | Later | Stored findings/events | Aggregated history |

## Input Contracts By Surface

### Preflight gate input

For `anvil gate <plan>`, command safety should run only when the plan or
execution artefact contains executable command proposals.

Recommended input contract:

- read executable command proposals from the supplied plan or execution file
- adapt them into `ScriptPlan` / `ScriptChange` values
- if no executable commands are present: `skip`

This keeps command safety meaningful without forcing it into repo-only gates.

### Live intercept input

For intercept, command safety should accept live issued commands from agents
before or during execution.

Recommended live input contract:

- command string
- working directory / worktree context
- session / agent metadata
- optional plan/task provenance when available

The intercept adapter should reuse the same parser, matcher, rules, and finding
types as preflight review.

### Future command-artefact input

If `anvil check` or another CLI surface later gains a “check these commands”
mode, it should feed command artefacts into the same capability, not create a
new evaluator.

## Result and Judgement Model

### Shared finding model

Command-safety findings should remain part of the broader checks -> findings ->
gates model.

- command safety is a **check family**
- blocked/warned command matches are **findings**
- preflight `gate` rolls those findings into workflow judgement
- intercept maps the same findings into live enforcement actions

### Preflight gate judgement

Preflight semantics should be workflow-oriented but non-interruptive:

- blocked findings -> gate fail
- warning findings -> gate warn or fail depending on configured threshold
- no findings -> pass
- no executable commands -> skip

### Live enforcement judgement

Live semantics should be stronger and notification-aware:

- allow
- warn
- block
- interrupt

These are not new rule results; they are decision-layer mappings over the same
underlying command-safety findings.

### Relationship to notifications

Command-safety findings are domain outputs.
Notifications are delivery artefacts.

Examples:

- preflight gate output in terminal/TUI
- watch or review banner
- daemon interrupt event
- machine-readable NDJSON/IPC event

This design depends on `NOTIFY` for the shared delivery/escalation model.

## Recommended Integration Shape

### Near-term

1. Integrate command safety into `anvil gate <plan>` when the plan contains
   executable commands
2. Expose it in the gate catalogue and gate-config only for plan/execution
   contexts where it applies
3. Document skip semantics when no executable commands exist

### Medium-term

1. Adapt intercept command events into the same capability
2. Reuse finding and explanation output for live enforcement
3. Route warn/block/interrupt notifications through the shared notification
   framework

## Things We Should Not Do

- Do not create a separate rule set for intercept-only command validation
- Do not model command safety as a generic repo file scan just to fit it into
  existing gate plumbing
- Do not let preflight and live enforcement invent separate severities or rule
  identifiers

## Follow-On Execution Slices

### Slice 1: Gate integration for executable plans

- adapt plan/execution artefacts into `ScriptPlan`
- run command safety in `anvil gate <plan>`
- map blocked/warning findings into gate output
- document skip semantics

### Slice 2: CLI/runtime contract updates

- update check catalogue and gate-config behaviour for command safety
- add user-facing help and docs
- ensure canonical naming aligns with CLAR

### Slice 3: Intercept reuse

- define intercept adapter input contract
- map findings to allow/warn/block/interrupt decisions
- emit notifications through the shared framework

### Slice 4: Explanation and observability

- add explain/reporting support for command-safety findings
- make events visible to audit/dashboard surfaces later

## Recommendation

Treat command safety as a shared capability with two primary adapters:

- **preflight gate adapter** for plans/execution proposals
- **live intercept adapter** for issued agent commands

That gives Anvil one command-safety brain and multiple surface-specific action
policies.
