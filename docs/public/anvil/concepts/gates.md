---
id: gates
title: Gates
description:
  Quality gates that validate code changes against deterministic rules.
sidebar_position: 1
---

# Gates

Gates are quality checks that code must pass before proceeding. They're
deterministic, composable, and run automatically.

## What is a Gate?

A gate is a validation checkpoint. Code enters, gets checked, and either passes
or fails:

```
┌─────────┐     ┌─────────┐     ┌─────────┐
│  Code   │ ──▶ │  Gate   │ ──▶ │  Pass   │
│ Changes │     │ Checks  │     │  Fail   │
└─────────┘     └─────────┘     └─────────┘
```

Gates are:

- **Deterministic** — same input, same result
- **Composable** — multiple checks per gate
- **Configurable** — enable/disable per project
- **Fast** — run in milliseconds

## Built-in Gate Checks

### Architecture Check

Validates that imports respect defined boundaries.

```json
{
  "architecture": {
    "enabled": true,
    "boundaries": [
      {
        "name": "api-layer",
        "pattern": "src/api/**",
        "deny": ["src/repositories/**"]
      }
    ]
  }
}
```

**Catches:**

- Layer violations (UI importing data layer directly)
- Domain violations (one domain importing another's internals)
- Circular dependencies

### Anti-Pattern Check

Detects known problematic patterns:

Rules are grouped into five **families**: guardrail-suppression,
type-system-evasion, error-visibility, responsibility-laundering, and
deferred-debt. When a warning fires, the family provenance points the
reviewer at the shared meta-issue instead of a single rule in isolation.

**Default patterns** (enabled out of the box):

| ID     | Pattern                                  | Why it matters                 |
| ------ | ---------------------------------------- | ------------------------------ |
| AP-001 | Broad `eslint-disable`                   | Hides multiple issues          |
| AP-003 | Explicit `any`                           | Defeats type safety            |
| AP-004 | `@ts-ignore`                             | Masks type errors              |
| AP-006 | Empty catch block                        | Swallows errors                |
| GS-001 | Non-null assertion (`!`)                 | Overrides nullability guardrail |
| RL-001 | Unverified "pre-existing" claim          | Shifts blame to the baseline   |
| RL-002 | Phantom follow-up                        | Work that never lands          |
| RL-003 | Blanket unrelated dismissal              | Silent scope expansion         |
| RL-004 | Unverified "not touched" claim           | Untestable denial              |
| RL-005 | Deferred without artifact                | Forgotten commitment           |
| RL-006 | Reply disguised as fix                   | Closes review without changing code |
| DD-001 | TODO/FIXME without tracking reference    | Debt with no ticket            |
| DD-002 | HACK without tracking reference          | Workaround with no follow-up   |
| DD-003 | Temporary code without expiry            | Permanently temporary          |
| DD-004 | Completion claim with outstanding TODOs  | Misrepresenting status         |

**Opt-in patterns** (enable via `.anvilrc` or `--include-opt-in`):

| ID     | Pattern                         | Why it matters              |
| ------ | ------------------------------- | --------------------------- |
| AP-002 | Rule-specific `eslint-disable`  | Granular but still hides    |
| AP-005 | `@ts-expect-error`              | Masks type errors           |
| AP-007 | Console in production           | Debug code leaked           |

### Secret Detection

Finds potential secrets in code:

- API keys (pattern matching)
- Passwords (entropy analysis)
- Credentials in git history

### ESLint Check

Runs ESLint as a gate check, failing on errors:

```json
{
  "eslint": {
    "enabled": true,
    "failOn": "error"
  }
}
```

### Coverage Check

Validates code coverage thresholds:

```json
{
  "coverage": {
    "enabled": true,
    "threshold": {
      "lines": 80,
      "branches": 75
    }
  }
}
```

### Policy Check

Custom rules via OPA/Rego:

```rego
package anvil.policy

deny[msg] {
  input.file.path == "src/index.ts"
  count(input.file.lines) > 500
  msg := "src/index.ts exceeds 500 lines"
}
```

## Gate Results

Each gate check produces a result:

```typescript
interface GateResult {
  status: 'pass' | 'fail' | 'warn' | 'skip';
  check: string;
  message: string;
  details?: {
    file: string;
    line: number;
    column: number;
    suggestion?: string;
  }[];
}
```

### Status Levels

| Status | Meaning                   | Default behaviour      |
| ------ | ------------------------- | ---------------------- |
| `pass` | Check passed              | Continue               |
| `warn` | Issue found, not blocking | Continue (log warning) |
| `fail` | Blocking issue            | Stop execution         |
| `skip` | Check not applicable      | Continue               |

### Configuring Severity

Override default severity:

```json
{
  "antiPatterns": {
    "patterns": {
      "AP-003": { "severity": "error" },
      "AP-007": { "severity": "off" }
    }
  }
}
```

## Gate Execution

### Run Order

Gates run in dependency order:

1. **Lint** — fast syntax checks
2. **Architecture** — import analysis
3. **Anti-patterns** — pattern matching
4. **Secrets** — security scan
5. **Tests** — if configured
6. **Coverage** — if configured

### Parallel Execution

Independent checks run in parallel. Dependent checks wait:

```
lint ────────┐
             ├──▶ architecture ──▶ anti-patterns
secrets ─────┘
```

### Caching

Gate results are cached by file hash. Unchanged files skip re-validation:

```
File: src/utils.ts
Hash: abc123
Cached result: PASS (from 2 minutes ago)
Skipping re-check.
```

## Suppressions

When a gate check should be bypassed, use suppressions:

```typescript
// @anvil-ignore AP-003 Using any for legacy API compatibility
const legacyData: any = fetchLegacyApi();
```

**Rules:**

- Suppressions require explanations
- Suppressions are tracked in evidence
- Unexplained suppressions trigger warnings

### File-Level Suppression

```typescript
// @anvil-ignore-file AP-007
// This file legitimately uses console for CLI output
```

### Configuration Suppression

```json
{
  "suppressions": [
    {
      "pattern": "scripts/**",
      "checks": ["AP-007"],
      "reason": "Scripts may use console"
    }
  ]
}
```

---

**Next:** [Sessions and runs →](/anvil/concepts/sessions)
