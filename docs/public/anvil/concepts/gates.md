---
id: gates
title: Gates
description:
  How checks, findings, and gates fit together in anvil's quality model.
sidebar_position: 1
---

# Gates

In anvil, a **gate** is not the same thing as a **check**.

- A **check** evaluates one concern
- A **finding** is a result emitted by a check
- A **gate** is the workflow judgement over one or more checks

That distinction is the core of the product model.

## The Short Version

anvil analyses your project, runs checks, collects findings, and then decides
whether the overall work passes the required quality bar.

```text
project -> checks -> findings -> gate decision
```

## Checks vs Gates

### Checks

Checks are the smallest user-facing unit of evaluation.

Examples using the public names you should pass to `--only-checks`,
`--skip-checks`, and `.anvilrc#checks`:

- `secret-detection`
- `import-boundaries`
- `antipattern-scan`
- `policy`
- `lint`
- `test`
- `coverage`
- `dependency`
- `command-safety`

Older aliases such as `secret` and `architecture` still work in some places, but
the public docs use the canonical names above.

Unknown check names are handled deliberately by surface: `--only-checks` and
`--skip-checks` fail fast because they describe one explicit invocation, while
`.anvilrc#checks` warns and continues with the known subset so a stale shared
config file does not block every local or CI gate. If every configured name is
unknown, the gate fails because there is no safe subset to run.

Each check answers one question about the codebase.

### Findings

Findings are the results produced by checks.

Examples:

- a boundary violation
- an explicit `any` anti-pattern
- a leaked secret
- a policy warning

Findings can have different severities such as warning, error, or info.

### Gates

Gates answer the workflow question:

- can this advance?
- can this merge?
- does this pass the required quality bar?

That is why `anvil gate` exists separately from `anvil check`.

## When to Use Each Surface

### `anvil check`

Use `anvil check` when you want targeted or exploratory analysis.

- inspect files
- surface findings
- understand what anvil sees
- run planless local analysis

### `anvil gate`

Use `anvil gate` when you want workflow judgement.

- aggregate multiple checks
- see pass/fail across the selected check set
- use in CI or pre-merge workflows
- use in watch mode when you care about whether the current state is acceptable

### `anvil watch`

Use `anvil watch` when you want continuous checks and gate updates as files
change. Watch is the **save-time fallback** for the AI guardrail when the MCP
pre-write path cannot attach — it never replaces pre-write interception, but it
does give you the next-best signal.

### `anvil start`

Use `anvil start` once per repo to wire MCP entries (Cursor / Claude Code),
baseline the repo, and end in one literal protection state. See the
[Quickstart](/anvil/quickstart) for the install-to-protection flow.

### `anvil doctor`

Use `anvil doctor` for setup and environment health. It is not a gate.

### `anvil audit`

Use `anvil audit` for broad repository review. It is a wider reporting surface,
not the primary gate flow.

## Built-in Gate Checks

These checks can participate in gate evaluation:

- **Import boundaries** — catches dependency edges that violate declared
  boundaries
- **Anti-pattern scan** — catches known harmful coding patterns such as broad
  `eslint-disable`, explicit `any`, empty catch blocks, and deferred-debt
  markers
- **Secret detection** — finds likely secrets and credentials in code
- **Policy** — evaluates custom OPA/Rego rules
- **Lint** — runs project linting checks
- **Test** — runs the project test suite
- **Coverage** — enforces coverage thresholds when configured
- **Dependency** — checks dependency risk and blocked packages
- **Command safety** — detects dangerous shell commands in plan-described
  scripts

## Gate Results

Each check contributes a result, and the gate summarises them.

Typical result states are:

| Status | Meaning                                           |
| ------ | ------------------------------------------------- |
| `pass` | The check passed                                  |
| `fail` | The check found blocking problems                 |
| `skip` | The check did not apply or had nothing to analyse |

In practice, the important point is this:

- findings explain **what** is wrong
- the gate tells you **whether you can proceed**

Every `anvil gate` run also records its result to `.anvil/gates.json` (pass
rate, per-check status, and the checks needing attention). The **Gate Summary**
dashboard renders that snapshot — run `anvil dashboard` and pick **Gate
Summary** to read the last run without re-running it. See
[Dashboards](../guides/dashboard.md#gate-summary).

## Example

```text
Checking architecture... done
Checking policy...
  AP-003 explicit any type detected
    src/utils/parser.ts:42

Checking secret... done

Quality gates failed (2/3 passed)
```

The anti-pattern finding tells you what to fix. The gate failure tells you the
current state is not good enough to advance.

## Why This Distinction Matters

Without the distinction, every surface turns into a vague “validation tool”.
With it, the model stays clear:

- checks inspect
- findings explain
- gates decide

That makes watch mode, CI, tutorials, and docs all easier to understand.

---

**Next:** [Quickstart](/anvil/quickstart) or
[Your First Gate Moment](/anvil/first-gate)
