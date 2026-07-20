---
id: determinism
title: Validation, audit, and CI safety
description:
  Understand what APS checks and when plan validation may execute commands.
sidebar_position: 3
---

# Validation, audit, and CI safety

APS has two different verification layers:

- `aps lint` parses plan files and reports structural errors or drift warnings.
- `aps audit` compares plan claims with project state and can execute validation
  commands.

That distinction is a security boundary.

## Lint a plan

```bash
aps lint
```

Errors produce a non-zero exit. Warnings do not, unless a separate project
policy promotes them. Use JSON for automation:

```bash
aps lint plans --json
```

### Error codes

| Code          | Meaning                                                                |
| ------------- | ---------------------------------------------------------------------- |
| `E001`        | A module has no `Purpose` section.                                     |
| `E002`        | A module has no `Work Items` section.                                  |
| `E003`        | A module has no ID or status metadata table.                           |
| `E004`        | An index has no `Modules` section.                                     |
| `E005`        | An active work item lacks intent, expected outcome, or validation.     |
| `E010`        | An issues file has no `Issues` section.                                |
| `E011`        | An issues file has no `Questions` section.                             |
| `R001`–`R004` | A release file has an invalid name or lacks required release sections. |

### Warning codes

| Code          | Meaning                                                               |
| ------------- | --------------------------------------------------------------------- |
| `W001`        | A work-item ID does not use `PREFIX-NNN`.                             |
| `W002`        | A conductor references a work item that cannot be found.              |
| `W003`        | A dependency cannot be resolved.                                      |
| `W004`        | A required planning section is empty.                                 |
| `W005`        | A ready module has no work items.                                     |
| `W006`        | A crosscutting index entry does not identify a conductor module.      |
| `W010`–`W013` | An issue or question has missing or malformed metadata.               |
| `W017`        | An active module has no recent review date.                           |
| `W018`        | A completed item cannot be audited because validation is missing.     |
| `W019`        | An index links to a module file that does not exist.                  |
| `W020`–`W021` | Federated child plans contain ambiguous work-item or module IDs.      |
| `W022`        | A package tag does not resolve to a package or application directory. |

## Enforce the CLI pin

By default APS warns when `.aps/config.yml` expects another CLI version. Use
strict mode in CI when drift must fail:

```bash
aps --strict lint
```

## Audit plan claims

The safe pull-request form is:

```bash
aps audit --no-run
```

Without `--no-run`, `aps audit` executes validation commands stored in completed
work items with shell semantics. Only do that for plan content you trust. Never
run untrusted pull-request validation fields with execution enabled.

Audit findings cover:

- completed work whose validation fails;
- draft work whose named files already exist;
- ready work in stale modules; and
- broken module links.

## Minimal CI job

```yaml
- name: Validate APS plans
  run: aps --strict lint
- name: Check plan drift without executing plan commands
  run: aps audit --no-run
```

Use the [CLI reference](../tooling/validation.md) for command options.
