---
id: evaluation-model
title: How anvil evaluates a project
description: Checks, findings, gates, scans, profiles, and when anvil runs.
owner: DOCDEF
upstream:
  - docs/architecture/quality-model.md
  - crates/anvil-cli/src/commands/check.rs
  - crates/anvil-cli/src/commands/gate.rs
  - crates/anvil-cli/src/commands/check_catalog.rs
verified_against: 0.9.6-beta
---

# How anvil evaluates a project

**For:** readers who want the product model, not a first-run walkthrough

**Time:** 8 minutes

**Outcome:** know what a check, finding, gate, and scan are, what `anvil check`
actually runs, and when anvil evaluates your project

## What you are looking at

This page defines how anvil evaluates a project. It is not a tutorial. For a
first run, use [install and get first value](../quickstart.md) or the
[ten-minute protection tutorial](../first-gate.md).

anvil evaluates **checks**. A **gate** is the workflow judgement over those
checks. A **scan** is how evidence is gathered for a check, not a third kind of
result.

## Where most confusion occurs

### Check

A **check** is the smallest thing anvil evaluates: one concern, one name you can
put in `checks:` or `--only-checks`.

"Check is the smallest unit of evaluation" does not mean `anvil check` runs
every check.

### Finding

A **finding** is the generic result a check emits (warning, violation, error, or
informational). It is evidence, not a command to apply a suggested edit.

### Gate

A **gate** is the workflow judgement over one or more checks: can this change
advance or merge? `anvil gate` is the only surface that answers that question.

### Scan

These ten sentences are the model for check, scan, and gate. **Planless** means
`anvil check` does not need a plan file or a full project setup.

1. A **check** is the smallest thing anvil evaluates: one concern, one name you
   can put in `checks:` or `--only-checks`.
2. A **scan** is how evidence is gathered for a check, not a second product
   object and not a command you choose instead of `check` or `gate`.
3. The command `anvil check` is a planless command that happens to say "scan" in
   its `--help` text; that wording names the method, not a type of result.
4. `antipattern-scan` is a **check name** — the engine that runs the compiled
   rule catalogue — even though the word "scan" appears in the name.
5. `anvil check` runs only the planless-eligible pair `secret-detection` and
   `antipattern-scan`.
6. Other engines listed in `.anvil.yaml` (`import-boundaries`, `policy`,
   `command-safety`, `lint`, `test`, `coverage`, `dependency`, and the surface
   checks) are ignored by `anvil check` and run under `anvil gate`.
7. A **gate** is the workflow judgement over one or more checks: can this change
   advance or merge?
8. `welcome` has its own discovery pass that honours `.gitignore`; that pass is
   not a check, and other commands do not follow that gitignore rule.
9. When CLI help says "scan files", read it as the check command gathering
   evidence, not as a third noun beside check and gate.
10. Findings from the four warn-only surfaces (`dockerfile`, `shell-scripts`,
    `sql-migrations`, `github-actions`) and warning-severity anti-pattern
    findings do not fail `anvil gate` unless you pass `--fail-on-warnings` or
    set `ANVIL_FAIL_ON_WARNINGS`.

## Commands versus the model

### `anvil check` is planless and narrow

`anvil check` is planless: it operates on the supplied file list and needs no
profile, policy bundle, or project-level config beyond the source itself.

| Command       | Engines it will run                                                                                                                        | What it ignores                                                                                                                   |
| ------------- | ------------------------------------------------------------------------------------------------------------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------- |
| `anvil check` | Only `secret-detection` and `antipattern-scan`                                                                                             | Every other catalogue engine, **even if** it appears in top-level `checks:`. Unknown / non-planless entries are silently ignored. |
| `anvil gate`  | The gate set: init-default checks, other `checks:` entries that are gate-supported, and default-on surface checks behind `track.surface.*` | Engines the profile / `--only-checks` / `--skip-checks` exclude                                                                   |

`import-boundaries` (alias `architecture`), `policy`, `command-safety`, `lint`,
`test`, `coverage`, and `dependency` require config, a toolchain, or a profile,
and live under `anvil gate`.

### `anvil gate` is the merge decision

`anvil gate` is the workflow judgement. It is the only surface that answers "may
this change advance?" Select a profile with `--profile`, restrict engines with
`--only-checks` / `--skip-checks`, and list profiles with
`anvil gate --list-profiles`.

### Watch, doctor, audit, architecture, policy, baseline

| Surface              | Role                                                                                                                | Default posture                                     |
| -------------------- | ------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------- |
| `anvil watch`        | Continuous mode. Default action is `check` (therefore the planless pair). `--action gate` or `--action none` exist. | Not itself a check.                                 |
| `anvil doctor`       | Setup / environment health.                                                                                         | Not a gate.                                         |
| `anvil audit`        | Broader exploratory reporting over findings.                                                                        | Not a merge decision.                               |
| `anvil architecture` | Structure definition.                                                                                               | Enforcement is still a check (`import-boundaries`). |
| `anvil policy`       | Pack install, show, validate, test, and gate.                                                                       | Policy is one family of **gate** checks (`policy`). |
| `anvil baseline`     | Record of findings accepted when anvil was introduced.                                                              | Not a check.                                        |

A **rule** is one compiled anti-pattern pattern. Rules belong to the
`antipattern-scan` check; they are not interchangeable with checks or with a
policy pack.

A **boundary** is a declared structural dependency constraint. Prefer that word
over "architecture" in ordinary use. It is not interchangeable with the
`architecture` CLI or the `import-boundaries` alias.

## Gate profiles

A **gate profile** is a named bundle of checks and thresholds for a context.
Shipped names:

| Profile      | Role                                                                              |
| ------------ | --------------------------------------------------------------------------------- |
| `dev`        | Local development. Skips coverage and dependency.                                 |
| `ci`         | Continuous integration. Runs the full gate set.                                   |
| `production` | Strict thresholds over the full gate set.                                         |
| `ai`         | Curated checks for AI-generated code. Skips lint, test, coverage, and dependency. |

A profile is not a check and not a synonym for `.anvil.yaml`.

## Warn-only surfaces and `--fail-on-warnings`

The four **surface checks** (`dockerfile`, `shell-scripts`, `sql-migrations`,
`github-actions`) are default-on in `anvil gate`, selected by feature flags
rather than the `checks:` list, and warn-only unless you pass
`--fail-on-warnings` or set `ANVIL_FAIL_ON_WARNINGS`. Warning-severity
anti-pattern findings follow the same default.

Error-severity findings still fail the gate on their own merit.

## When anvil runs (pre-write, save-time, daemon, witness)

| Moment                   | What it is                             | Typical path                                                                           |
| ------------------------ | -------------------------------------- | -------------------------------------------------------------------------------------- |
| **Pre-write validation** | Evaluates a write **before** it lands. | Local daemon / intercept, and MCP `anvil_validate_write` / `anvil_apply_patch`.        |
| **Save-time validation** | Evaluates after a save.                | `anvil watch` and optional Git hooks. A fallback, not the same guarantee as pre-write. |
| **On demand**            | You run a command.                     | `anvil check` (planless pair) or `anvil gate` (merge judgement).                       |

The **daemon** is the local process that keeps protection on (`anvil start`,
bare `anvil` daily ensure, intercept). **Protection state** is whether that
process is armed for the project. A **witness** is evidence that a protected
action ran (audit trail, review capsule, or hook). None of those is a check.

`anvil welcome` is the only discovery pass that honours `.gitignore`. Other
commands do not follow that gitignore rule.

## Honesty rules and known gaps

- This page names only the public checks. If a check is not listed here, do not
  treat it as a public engine.
- Surface checks are shipped with flag status: they are default-on in gate
  today, not list-editable via `checks:`.

## Try it

- [Install and get first value](../quickstart.md)
- [Ten-minute protection tutorial](../first-gate.md)

## Related definitions

- [What anvil can do](../reference/what-anvil-can-do.md)
- [Glossary](glossary.md)
- [Checks, findings, and gates](gates.md)
- [Policy packs](policy-model.md)
- [Architecture boundaries](boundaries.md)
- [Introduction baseline](baseline.md)
- [CLI command reference](../reference/cli.md)
- [Compiled pattern catalogue](../reference/rules.md)
