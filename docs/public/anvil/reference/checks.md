---
id: checks
title: Check catalogue
description:
  Look up every shipped anvil check, including the planless pair and flag-driven
  surfaces.
owner: DOCDEF
upstream:
  - crates/anvil-cli/src/commands/check_catalog.rs
  - crates/anvil-cli/src/commands/check.rs
  - flags/manifest.json
  - scripts/docs/generate-anvil-public-reference.mjs
verified_against: 0.9.7-beta
---

<!-- Generated from shipped product sources. Do not edit by hand. -->

# Check catalogue

This catalogue is generated from the shipped check definitions. It is the
complete engine list; [what anvil can do](what-anvil-can-do.md) stays a 12-row
index.

- **Planless `anvil check` pair:** `secret-detection` and `antipattern-scan`.
  Every other engine is ignored by `anvil check`, even if it appears in
  `checks:`.
- **Init-default checks:** `secret-detection`, `import-boundaries`,
  `antipattern-scan`.
- **Surface checks** (`sql-migrations`, `github-actions`, `dockerfile`,
  `shell-scripts`) are shipped-with-flag-status: default-on in `anvil gate`, not
  list-editable via `checks:`, and warn-only unless `--fail-on-warnings`.

Read [how anvil evaluates a project](../concepts/evaluation-model.md) for check
versus scan versus gate.

## `secret-detection`

Detect leaked secrets and credentials.

| Field                  | Value                   |
| ---------------------- | ----------------------- |
| Stable ID              | `ANV-CORE-001`          |
| Canonical name         | `secret-detection`      |
| Aliases                | `secret`                |
| Init enabled / visible | enabled / visible       |
| Gate / gate-config     | yes / yes               |
| Selection              | `.anvil` `checks:` list |
| `anvil check`          | runs                    |

### What it evaluates

Detect leaked secrets and credentials.

### Findings / warn-only

Follows the engine severity and gate thresholds.

### Configure

Select with top-level `checks:`, `--only-checks`, or `--skip-checks`.

### Related

- Model: [How anvil evaluates a project](../concepts/evaluation-model.md)

## `import-boundaries`

Enforce module import boundaries.

| Field                  | Value                            |
| ---------------------- | -------------------------------- |
| Stable ID              | `ANV-CORE-002`                   |
| Canonical name         | `import-boundaries`              |
| Aliases                | `architecture`                   |
| Init enabled / visible | enabled / visible                |
| Gate / gate-config     | yes / yes                        |
| Selection              | `.anvil` `checks:` list          |
| `anvil check`          | **ignored** (planless pair only) |

### What it evaluates

Enforce module import boundaries.

### Findings / warn-only

Follows the engine severity and gate thresholds.

### Configure

Select with top-level `checks:`, `--only-checks`, or `--skip-checks`.

### Related

- Model: [How anvil evaluates a project](../concepts/evaluation-model.md)
- Boundaries: [Architecture boundaries](../concepts/boundaries.md)

## `antipattern-scan`

Detect patterns covered by anvil's built-in rule catalogue.

| Field                  | Value                   |
| ---------------------- | ----------------------- |
| Stable ID              | `ANV-CORE-003`          |
| Canonical name         | `antipattern-scan`      |
| Aliases                | none                    |
| Init enabled / visible | enabled / visible       |
| Gate / gate-config     | yes / yes               |
| Selection              | `.anvil` `checks:` list |
| `anvil check`          | runs                    |

### What it evaluates

Detect patterns covered by anvil's built-in rule catalogue.

### Findings / warn-only

Follows the engine severity and gate thresholds.

### Configure

Select with top-level `checks:`, `--only-checks`, or `--skip-checks`.

### Related

- Model: [How anvil evaluates a project](../concepts/evaluation-model.md)
- Rules body: [Compiled pattern catalogue](rules.md)

## `policy`

Evaluate OPA policy rules.

| Field                  | Value                            |
| ---------------------- | -------------------------------- |
| Stable ID              | `ANV-CORE-004`                   |
| Canonical name         | `policy`                         |
| Aliases                | none                             |
| Init enabled / visible | not enabled / visible            |
| Gate / gate-config     | yes / yes                        |
| Selection              | `.anvil` `checks:` list          |
| `anvil check`          | **ignored** (planless pair only) |

### What it evaluates

Evaluate OPA policy rules.

### Findings / warn-only

Follows the engine severity and gate thresholds.

### Configure

Select with top-level `checks:`, `--only-checks`, or `--skip-checks`.

### Related

- Model: [How anvil evaluates a project](../concepts/evaluation-model.md)
- Packs: [Policy model](../concepts/policy-model.md)
- Commands: [Policy command reference](policy.md)

## `lint`

Code quality and style checks.

| Field                  | Value                            |
| ---------------------- | -------------------------------- |
| Stable ID              | `ANV-CORE-005`                   |
| Canonical name         | `lint`                           |
| Aliases                | none                             |
| Init enabled / visible | not enabled / hidden             |
| Gate / gate-config     | yes / yes                        |
| Selection              | `.anvil` `checks:` list          |
| `anvil check`          | **ignored** (planless pair only) |

### What it evaluates

Code quality and style checks.

### Findings / warn-only

Follows the engine severity and gate thresholds.

### Configure

Select with top-level `checks:`, `--only-checks`, or `--skip-checks`.

### Related

- Model: [How anvil evaluates a project](../concepts/evaluation-model.md)

## `test`

Test suite execution.

| Field                  | Value                            |
| ---------------------- | -------------------------------- |
| Stable ID              | `ANV-CORE-006`                   |
| Canonical name         | `test`                           |
| Aliases                | none                             |
| Init enabled / visible | not enabled / hidden             |
| Gate / gate-config     | yes / yes                        |
| Selection              | `.anvil` `checks:` list          |
| `anvil check`          | **ignored** (planless pair only) |

### What it evaluates

Test suite execution.

### Findings / warn-only

Follows the engine severity and gate thresholds.

### Configure

Select with top-level `checks:`, `--only-checks`, or `--skip-checks`.

### Related

- Model: [How anvil evaluates a project](../concepts/evaluation-model.md)

## `coverage`

Code coverage thresholds.

| Field                  | Value                            |
| ---------------------- | -------------------------------- |
| Stable ID              | `ANV-CORE-007`                   |
| Canonical name         | `coverage`                       |
| Aliases                | none                             |
| Init enabled / visible | not enabled / hidden             |
| Gate / gate-config     | yes / yes                        |
| Selection              | `.anvil` `checks:` list          |
| `anvil check`          | **ignored** (planless pair only) |

### What it evaluates

Code coverage thresholds.

### Findings / warn-only

Follows the engine severity and gate thresholds.

### Configure

Select with top-level `checks:`, `--only-checks`, or `--skip-checks`.

### Related

- Model: [How anvil evaluates a project](../concepts/evaluation-model.md)

## `dependency`

Dependency vulnerability scanning.

| Field                  | Value                            |
| ---------------------- | -------------------------------- |
| Stable ID              | `ANV-CORE-008`                   |
| Canonical name         | `dependency`                     |
| Aliases                | none                             |
| Init enabled / visible | not enabled / hidden             |
| Gate / gate-config     | yes / yes                        |
| Selection              | `.anvil` `checks:` list          |
| `anvil check`          | **ignored** (planless pair only) |

### What it evaluates

Dependency vulnerability scanning.

### Findings / warn-only

Follows the engine severity and gate thresholds.

### Configure

Select with top-level `checks:`, `--only-checks`, or `--skip-checks`.

### Related

- Model: [How anvil evaluates a project](../concepts/evaluation-model.md)

## `command-safety`

Detect dangerous shell commands in plan-described scripts.

| Field                  | Value                            |
| ---------------------- | -------------------------------- |
| Stable ID              | `ANV-CORE-009`                   |
| Canonical name         | `command-safety`                 |
| Aliases                | none                             |
| Init enabled / visible | not enabled / hidden             |
| Gate / gate-config     | yes / yes                        |
| Selection              | `.anvil` `checks:` list          |
| `anvil check`          | **ignored** (planless pair only) |

### What it evaluates

Detect dangerous shell commands in plan-described scripts.

### Findings / warn-only

Follows the engine severity and gate thresholds.

### Configure

Select with top-level `checks:`, `--only-checks`, or `--skip-checks`.

### Related

- Model: [How anvil evaluates a project](../concepts/evaluation-model.md)

## `sql-migrations`

Flag destructive/irreversible operations in SQL migrations.

| Field                  | Value                                                                          |
| ---------------------- | ------------------------------------------------------------------------------ |
| Stable ID              | `ANV-SURF-SQL-001`                                                             |
| Canonical name         | `sql-migrations`                                                               |
| Aliases                | `sql`                                                                          |
| Init enabled / visible | not enabled / hidden                                                           |
| Gate / gate-config     | yes / no                                                                       |
| Selection              | feature flag `track.surface.sql` (session opt-out `ANVIL_TRACK_SURFACE_SQL=0`) |
| `anvil check`          | **ignored** (planless pair only)                                               |

### What it evaluates

Flag destructive/irreversible operations in SQL migrations.

### Findings / warn-only

Warn-only in `anvil gate` unless `--fail-on-warnings` or
`ANVIL_FAIL_ON_WARNINGS`. Session opt-out: `ANVIL_TRACK_SURFACE_SQL=0`.

### Configure

Surface checks cannot be enabled or disabled through the `checks:` list.

### Related

- Model: [How anvil evaluates a project](../concepts/evaluation-model.md)

## `github-actions`

Flag supply-chain risks in GitHub Actions workflows.

| Field                  | Value                                                                          |
| ---------------------- | ------------------------------------------------------------------------------ |
| Stable ID              | `ANV-SURF-GHA-001`                                                             |
| Canonical name         | `github-actions`                                                               |
| Aliases                | `gha`                                                                          |
| Init enabled / visible | not enabled / hidden                                                           |
| Gate / gate-config     | yes / no                                                                       |
| Selection              | feature flag `track.surface.gha` (session opt-out `ANVIL_TRACK_SURFACE_GHA=0`) |
| `anvil check`          | **ignored** (planless pair only)                                               |

### What it evaluates

Flag supply-chain risks in GitHub Actions workflows.

### Findings / warn-only

Warn-only in `anvil gate` unless `--fail-on-warnings` or
`ANVIL_FAIL_ON_WARNINGS`. Session opt-out: `ANVIL_TRACK_SURFACE_GHA=0`.

### Configure

Surface checks cannot be enabled or disabled through the `checks:` list.

### Related

- Model: [How anvil evaluates a project](../concepts/evaluation-model.md)

## `dockerfile`

Flag build-hygiene / supply-chain risks in Dockerfiles.

| Field                  | Value                                                                            |
| ---------------------- | -------------------------------------------------------------------------------- |
| Stable ID              | `ANV-SURF-DOCK-001`                                                              |
| Canonical name         | `dockerfile`                                                                     |
| Aliases                | `dock`                                                                           |
| Init enabled / visible | not enabled / hidden                                                             |
| Gate / gate-config     | yes / no                                                                         |
| Selection              | feature flag `track.surface.dock` (session opt-out `ANVIL_TRACK_SURFACE_DOCK=0`) |
| `anvil check`          | **ignored** (planless pair only)                                                 |

### What it evaluates

Flag build-hygiene / supply-chain risks in Dockerfiles.

### Findings / warn-only

Warn-only in `anvil gate` unless `--fail-on-warnings` or
`ANVIL_FAIL_ON_WARNINGS`. Session opt-out: `ANVIL_TRACK_SURFACE_DOCK=0`.

### Configure

Surface checks cannot be enabled or disabled through the `checks:` list.

### Related

- Model: [How anvil evaluates a project](../concepts/evaluation-model.md)

## `shell-scripts`

Flag dangerous commands in checked-in shell scripts.

| Field                  | Value                                                                        |
| ---------------------- | ---------------------------------------------------------------------------- |
| Stable ID              | `ANV-SURF-SH-001`                                                            |
| Canonical name         | `shell-scripts`                                                              |
| Aliases                | `sh`, `shell`                                                                |
| Init enabled / visible | not enabled / hidden                                                         |
| Gate / gate-config     | yes / no                                                                     |
| Selection              | feature flag `track.surface.sh` (session opt-out `ANVIL_TRACK_SURFACE_SH=0`) |
| `anvil check`          | **ignored** (planless pair only)                                             |

### What it evaluates

Flag dangerous commands in checked-in shell scripts.

### Findings / warn-only

Warn-only in `anvil gate` unless `--fail-on-warnings` or
`ANVIL_FAIL_ON_WARNINGS`. Session opt-out: `ANVIL_TRACK_SURFACE_SH=0`.

### Configure

Surface checks cannot be enabled or disabled through the `checks:` list.

### Related

- Model: [How anvil evaluates a project](../concepts/evaluation-model.md)
