---
id: determinism
title: Validation Rules
description: How APS lint enforces structure, references, and consistency.
sidebar_position: 3
---

# Validation Rules

| Type        | Authority | Owner   | Status | Freshness                                               |
| ----------- | --------- | ------- | ------ | ------------------------------------------------------- |
| Public docs | Derived   | DOCSYNC | Live   | Last reviewed 2026-06-22 against anvil-plan-spec v0.4.0 |

| Upstream                                                                  | Downstream            |
| ------------------------------------------------------------------------- | --------------------- |
| [anvil-plan-spec](https://github.com/EddaCraft/anvil-plan-spec) `docs/**` | APS docs-site section |

APS enforces consistent, machine-checkable plan structure through `aps lint` and
`aps audit`. This page describes what the tooling validates and why.

## What lint checks

`aps lint` validates APS documents for:

- Required sections and metadata tables
- Work item field completeness
- ID format conformance
- Cross-reference resolution
- Release narrative structure

```bash
aps lint plans/                       # lint entire plan tree
aps lint plans/modules/auth.aps.md    # lint one file
aps lint . --json                     # machine-readable output
```

Errors cause a non-zero exit code. Warnings are informational unless `--strict`
is set.

## Error codes

| Code | Scope     | Description                                                            |
| ---- | --------- | ---------------------------------------------------------------------- |
| E001 | Module    | Missing `## Purpose` section                                           |
| E002 | Module    | Missing `## Work Items` section                                        |
| E003 | Module    | Missing ID/Status metadata table                                       |
| E004 | Index     | Missing `## Modules` section                                           |
| E005 | Work Item | Missing required field (`Intent`, `Expected Outcome`, or `Validation`) |
| E010 | Issues    | Missing `## Issues` section                                            |
| E011 | Issues    | Missing `## Questions` section                                         |
| R001 | Release   | Release file not named `v<version>.md`                                 |
| R002 | Release   | Missing release header table with `Target` and `Status`                |
| R003 | Release   | Missing `## Release Theme` section                                     |
| R004 | Release   | Missing `## What Ships` section                                        |

## Warning codes

| Code | Scope          | Description                                                                    |
| ---- | -------------- | ------------------------------------------------------------------------------ |
| W001 | Work Item      | ID does not match `PREFIX-NNN` pattern                                         |
| W002 | Module         | Conductor references a work-item ID that resolves nowhere                      |
| W003 | Work Item      | Dependency references an ID not found in the plan tree                         |
| W004 | Module / Index | Section exists but is empty                                                    |
| W005 | Module         | Status is `Ready` but no work items are defined                                |
| W006 | Index          | Module listed under Conductor subsection but file is not `Type: Conductor`     |
| W010 | Issues         | Issue entry missing `Status`, `Discovered`, or `Severity`                      |
| W011 | Issues         | Question entry missing `Status`, `Discovered`, or `Priority`                   |
| W012 | Issues         | Issue ID does not match `ISS-NNN` format                                       |
| W013 | Issues         | Question ID does not match `Q-NNN` format                                      |
| W017 | Module         | Active module missing or stale `**Last reviewed:**` field (threshold: 60 days) |
| W018 | Work Item      | Complete item missing `Validation` in an active module                         |
| W019 | Index          | Module link points to a non-existent file                                      |

## Work item ID format

Pattern: `{PREFIX}-{NNN}`

```text
✓ AUTH-001   ✓ PAY-123   ✓ CORE-001
✗ auth-001   ✗ AUTH-1    ✗ AUTH_001
```

- PREFIX: 1–10 uppercase alphanumeric characters
- NNN: 3-digit zero-padded number

## Required work item fields

```markdown
### AUTH-001: Title

- **Intent:** Required — what outcome this achieves
- **Expected Outcome:** Required — testable result
- **Validation:** Required — command to verify completion
```

## Status consistency

The orchestration CLI enforces a state machine:

```text
Draft ──→ Ready ──→ In Progress ──→ Complete
```

| Command        | Transition enforced                                 |
| -------------- | --------------------------------------------------- |
| `aps next`     | None — read-only                                    |
| `aps start`    | Ready → In Progress (dependencies must be Complete) |
| `aps complete` | In Progress → Complete                              |

Invalid transitions are rejected with a clear error.

## Toolchain pinning

Projects pin their expected CLI version in `.aps/config.yml`:

```yaml
cli_version: 0.4.0
```

`aps` compares the pin to the running binary and warns on mismatch. Add
`--strict` (or `APS_STRICT=1`) to fail CI on drift:

```bash
aps lint --strict
```

## Audit: plan vs reality

`aps audit` checks whether Complete items actually pass their validation
commands and whether Draft items have files that already exist:

| Code | Meaning                                                         |
| ---- | --------------------------------------------------------------- |
| A001 | Overstated — Complete item whose Validation command fails       |
| A002 | Understated — Draft item whose Files already exist with content |
| A003 | Stale — Ready module with no recent `**Last reviewed:**`        |
| A004 | Broken link — index module link points to a non-existent file   |

> **CI safety:** Use `aps audit --no-run` in pull-request jobs. Running with
> execution enabled executes Validation commands from plan files with full shell
> semantics — only run on trusted branches.

## CI integration

```yaml
- name: Lint APS documents
  run: aps lint plans/ --strict
```

See the canonical workflow at
[anvil-plan-spec `docs/ci-lint-example.yml`](https://github.com/EddaCraft/anvil-plan-spec/blob/main/docs/ci-lint-example.yml).

---

**Next:** [Document structure →](../schemas/json-schema.md)
