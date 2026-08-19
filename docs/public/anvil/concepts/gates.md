---
id: gates
title: Checks, findings, and gates
description: Understand the three result layers used by anvil.
owner: DOCSYNC
upstream:
  - crates/anvil-cli/src/commands/gate.rs
  - crates/anvil-cli/src/commands/check.rs
verified_against: 0.9.0-beta
---

# Checks, findings, and gates

A **check** evaluates one concern. A **finding** is the result of a check. A
**gate** combines checks into a workflow decision.

The full model — check versus scan, the planless `anvil check` subset, profiles,
and when anvil runs — is [How anvil evaluates a project](evaluation-model.md).

## Checks

Use `anvil check` for a quick, file-focused scan:

```text
anvil check src/example.ts --format plain
```

Use `anvil check --changed` for changed files or `anvil check --staged` for
staged files.

## Findings

A useful finding tells you:

- which rule ran;
- which file and location matched;
- why the pattern matters; and
- what a safe correction looks like.

A finding is evidence, not a command to apply a suggested edit blindly. Review
the surrounding code and fix the cause.

## Gates

Use `anvil gate` when the workflow needs several checks and a single verdict:

```text
anvil gate --profile dev --format plain
```

Profiles choose checks and thresholds for a context such as local development or
continuous integration. List installed profiles with:

```text
anvil gate --list-profiles
```

## Warnings and blocking findings

A finding's severity is not the same as the gate's exit decision.

- **Warning-severity** anti-pattern findings, and findings from the four
  warn-only surfaces (`dockerfile`, `shell-scripts`, `sql-migrations`,
  `github-actions`), are reported but do **not** fail `anvil gate` by default.
  Opt in to the stricter posture with `--fail-on-warnings` or
  `ANVIL_FAIL_ON_WARNINGS`.
- **Error-severity** findings fail the gate on their own merit. Broken ciphers /
  ECB and JWT configured with the `none` algorithm stay in that set so they
  block without the opt-in.
- Other gate engines (secrets, architecture, policy, and similar) keep their own
  thresholds.

Use a warning as evidence to review, not as a silent pass.

## Exit codes and automation

Do not parse human-readable wording. Use `--format json` or `--format sarif`
where a command offers it, and use the process exit code for control flow.

## Next step

Try [the first finding tutorial](../first-gate.md) or add
[continuous integration](../integrations/github.md).
