# Quality Model

| Type  | Authority     | Owner | Status | Freshness                                                                                                                                                                                              |
| ----- | ------------- | ----- | ------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Guide | Authoritative | KERN  | Live   | Last reviewed 2026-08-20 at `d9b30b23d` against `crates/anvil-cli/src/commands/check_catalog.rs`, `crates/anvil-kernel-types/src/diagnostics.rs`, and `crates/anvil-kernel-types/src/notifications.rs` |

| Upstream                                                                                                                                               | Downstream                                                                                        |
| ------------------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------- |
| `crates/anvil-cli/src/commands/check_catalog.rs`, `crates/anvil-kernel-types/src/diagnostics.rs`, and `crates/anvil-kernel-types/src/notifications.rs` | Architecture docs, CLI/TUI copy, check/gate/audit/watch terminology, public product documentation |

This document defines the conceptual architecture of Anvil's quality system. It
is the internal reference for how to talk about checks, findings, gates, watch
mode, audit, doctor, and related surfaces.

This KERN-owned view is the detailed authority for the quality concern; the
central [overview](overview.md) links it rather than restating it.

## Purpose

Anvil has several adjacent surfaces that can easily blur together:

- `check`
- `gate`
- `watch`
- `audit`
- `doctor`
- `architecture`
- `policy`

This document describes the intended relationship between them so new features,
docs, and UI copy teach one coherent model.

## Core Hierarchy

The quality model has five layers.

1. **Graph / structure** Anvil builds a structural understanding of the project:
   files, imports, boundaries, layers, dependencies, and related context.
2. **Checks** A check evaluates one concern against the project or its
   structure.
3. **Findings** Checks emit findings. A finding is the generic noun for a
   detected problem, risk, or observation.
4. **Gate** A gate is the workflow judgement over one or more checks.
5. **Surfaces / modes** Commands and UIs expose the model for different
   purposes: setup, targeted analysis, continuous feedback, and workflow
   advancement.

## Canonical Terms

### Check

The smallest user-facing unit of evaluation.

Examples:

- secret detection
- import boundaries
- policy evaluation
- lint
- test
- coverage
- dependency scan
- anti-pattern scan

Use `check` when talking about one evaluative concern.

### Canonical Check Names

When a document, example, `.anvilrc#checks`, `anvil gate --only-checks`, or
`anvil gate --skip-checks` names a check, use the runtime canonical names from
`crates/anvil-cli/src/commands/check_catalog.rs`. Stable IDs are diagnostic
identifiers; the canonical name is what users should type.

| Stable ID           | Canonical name      | Accepted aliases | Gate-supported | Gate-config supported |
| ------------------- | ------------------- | ---------------- | -------------- | --------------------- |
| `ANV-CORE-001`      | `secret-detection`  | `secret`         | Yes            | Yes                   |
| `ANV-CORE-002`      | `import-boundaries` | `architecture`   | Yes            | Yes                   |
| `ANV-CORE-003`      | `antipattern-scan`  | —                | Yes            | Yes                   |
| `ANV-CORE-004`      | `policy`            | —                | Yes            | Yes                   |
| `ANV-CORE-005`      | `lint`              | —                | Yes            | Yes                   |
| `ANV-CORE-006`      | `test`              | —                | Yes            | Yes                   |
| `ANV-CORE-007`      | `coverage`          | —                | Yes            | Yes                   |
| `ANV-CORE-008`      | `dependency`        | —                | Yes            | Yes                   |
| `ANV-CORE-009`      | `command-safety`    | —                | Yes            | Yes                   |
| `ANV-SURF-SQL-001`  | `sql-migrations`    | `sql`            | Yes            | No                    |
| `ANV-SURF-GHA-001`  | `github-actions`    | `gha`            | Yes            | No                    |
| `ANV-SURF-DOCK-001` | `dockerfile`        | `dock`           | Yes            | No                    |
| `ANV-SURF-SH-001`   | `shell-scripts`     | `sh`, `shell`    | Yes            | No                    |

Legacy aliases stay accepted where the CLI advertises them, but user-facing docs
should prefer `secret-detection` over `secret` and `import-boundaries` over
`architecture`.

### Finding

The generic result emitted by a check.

Use `finding` across the product when the result could be any of:

- warning
- violation
- error
- informational result

Subtypes still matter:

- `violation` for a breached rule or boundary
- `warning` for severity or non-blocking state
- `issue` only where a dedicated surface intentionally groups mixed concerns

### Gate

The workflow decision over one or more checks.

`gate` is deliberately stronger than “list of results”. It answers:

- can this advance?
- can this merge?
- does this pass the required quality bar?

That is why `gate` should be reserved for workflow judgement, not used as a
generic synonym for control, config, or validation.

### Notification

A delivery artefact that carries information to a human or machine consumer.

Notifications are not the same thing as findings:

- a **finding** is a domain result emitted by a check
- a **notification** is how that result is delivered, prioritised, grouped, or
  escalated for a particular surface

Examples:

- inline terminal warning text
- a TUI status banner
- a watch-mode update row
- a future block or interrupt event from the intercept daemon

This distinction matters because Anvil already has multiple output streams
today, and future daemon-era interrupts will add more. Those streams should all
share one notification framework rather than re-defining findings, checks, or
gates.

### Scan

A discovery or evidence-gathering action.

Examples:

- discovery scan
- secret scan
- anti-pattern scan

`scan` describes how evidence is gathered. It should not replace `check` or
`gate` as the top-level product model.

### Boundary

A declared structural constraint about what parts of the system may depend on
one another.

This is the preferred user-facing term when the subject is dependency
constraints, even if the implementation lives under “architecture”.

### Graph

Anvil's structural understanding of the project.

`graph` is a useful explanatory term and a real differentiator, but it is a
second-step teaching concept. Users should understand checks and gates before
being expected to reason in graph terms.

## Surface Model

### `anvil check`

Targeted or exploratory analysis.

- Use when the goal is to inspect files and surface findings.
- `check` is the best fit for planless, local, developer-driven analysis.
- This is the flagship analysis surface for scanners such as anti-patterns.

### `anvil gate`

Workflow judgement.

- Use when the question is whether work passes the required set of checks.
- A gate aggregates selected checks into one decision surface.
- Gates are the right conceptual layer for CI, merge readiness, and continuous
  watch judgements.

### `anvil watch`

Continuous mode over checks and gates.

- Watch mode observes the workspace continuously: the kernel watcher emits
  change events, and the default action is `check`. Use `--action gate` to run a
  gate on each change, or `--action none` for an architecture/dependency-only
  watch with no code-quality scan.
- When a save-time daemon is already available, watch routes changed-path
  validation through the daemon. Headless, `--json`, CI, hook, and piped runs
  stay deterministic and do not prompt to start a daemon.
- It should be understood as continuous quality feedback, not as a separate
  parallel quality system. See [`checks-as-built.md`](./checks-as-built.md) for
  the live event-emission + deferred-dispatch path.

### `anvil doctor`

Setup and environment health.

- Doctor uses checks, but they are setup checks.
- It should not be described as a gate.

### `anvil audit`

Broad repository review.

- Audit is a broader exploratory surface.
- It may continue to use `issue` for its own UX, but it should still be framed
  as a reporting surface over findings rather than a separate conceptual model.

### `anvil architecture`

Configuration and structure-definition surface.

- `architecture validate` validates the model/configuration.
- Boundary-enforcement results should be discussed as checks or findings within
  the quality model, not as a second unrelated system.

### `anvil policy`

Policy authoring and inspection.

- Policy is a specialised subsystem.
- Policy checks are one family of checks, not the only check type.

## Current Runtime Shape

The desired architecture is straightforward:

- checks are first-class
- findings are the shared result noun
- gates aggregate checks into workflow judgement

The broader architectural shape is best understood as four layers:

1. **Coverage / substrate layer** — what artefacts Anvil can inspect at all
2. **Capability layer** — check engines, graph/context builders, explanations
3. **Decision layer** — finding aggregation, gate judgement, enforcement mapping
4. **Surface layer** — CLI, TUI, watch, intercept, dashboard, PR/reporting

This matters because some capabilities, such as command safety, are naturally
shared across multiple surfaces but do not belong to only one of them.

The implementation is still converging on that shape. Some checks are fully
represented across init, config, gate, and docs. Others exist in analysis
surfaces before they are fully wired into gate orchestration.

When documenting or extending the system:

- prefer the conceptual model first
- call out implementation gaps explicitly
- do not teach parallel dialects just because two runtime paths have not yet
  converged

## Design Rules

When adding or revising Anvil surfaces:

1. Use `check` for the smallest evaluative unit.
2. Use `finding` as the generic result noun.
3. Use `gate` only for workflow judgement.
4. Use `scan` for evidence-gathering actions.
5. Use `boundary` when the subject is dependency constraints.
6. Explain `graph` as the structural reason checks work, not as default jargon.
7. Keep findings and notifications separate: findings are domain results,
   notifications are delivery and escalation artefacts.

## Relationship Diagram

```mermaid
flowchart TD
    projectGraph["Project graph / structure"]
    checks["Checks\nsecret / boundaries / policy / anti-patterns / lint / test / coverage"]
    findings["Findings\nwarning / violation / info"]
    gate["Gate\nworkflow judgement"]
    surfaces["Surfaces\ncheck / gate / watch / audit / doctor / tutorial"]

    projectGraph --> checks
    checks --> findings
    checks --> gate
    findings --> gate
    gate --> surfaces
    findings --> surfaces
```
