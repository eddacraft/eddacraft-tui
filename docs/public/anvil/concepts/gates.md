---
id: gates
title: Checks, findings, and gates
description: Understand the three result layers used by anvil.
---

# Checks, findings, and gates

A **check** evaluates one concern. A **finding** is the result of a check. A
**gate** combines checks into a workflow decision.

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

## Exit codes and automation

Do not parse human-readable wording. Use `--format json` or `--format sarif`
where a command offers it, and use the process exit code for control flow.

## Next step

Try [the first finding tutorial](../first-gate.md) or add
[continuous integration](../integrations/github.md).
