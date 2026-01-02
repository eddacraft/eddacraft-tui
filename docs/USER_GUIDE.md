# Anvil User Guide

Complete guide to using Anvil for catching architecture violations and
anti-patterns in your code.

## Table of Contents

- [Introduction](#introduction)
- [Installation](#installation)
- [Core Commands](#core-commands)
  - [anvil check](#anvil-check)
  - [anvil watch](#anvil-watch)
  - [anvil status](#anvil-status)
  - [anvil doctor](#anvil-doctor)
  - [anvil hooks](#anvil-hooks)
  - [anvil init](#anvil-init)
- [Plan Commands](#plan-commands)
  - [anvil validate](#anvil-validate)
  - [anvil gate](#anvil-gate)
  - [anvil export](#anvil-export)
- [Configuration](#configuration)
- [Anti-Pattern Detection](#anti-pattern-detection)
- [Architecture Safety](#architecture-safety)
- [Suppressions](#suppressions)
- [CI/CD Integration](#cicd-integration)
- [Best Practices](#best-practices)

## Introduction

### What is Anvil?

Anvil catches architecture boundary violations and AI-generated anti-patterns at
file save time — when fixing is cheap. It provides:

- **Save-time Feedback** — Warnings appear as you code, not in review
- **Architecture Safety** — Detect dependencies crossing boundaries
- **Anti-pattern Detection** — Catch `any`, `@ts-ignore`, empty catches
- **Accountability** — Suppress with explanations, track in reports
- **CI Integration** — Block or warn on PRs with issues

### Key Concepts

**Warnings**: Anvil produces warnings, not errors. Your code still runs and
compiles. You decide whether to fix, suppress, or ignore.

**Baseline**: Existing violations are tracked but not warned about. Only **new**
violations trigger warnings.

**Suppressions**: Use `@anvil-ignore <ID>: <reason>` to acknowledge and bypass a
warning with accountability.

## Installation

### Prerequisites

- **Node.js**: Version 20.x or 22.x (minimum: 20.0.0)
- **pnpm**: Version 10.17.1 or higher
- **Git**: Required for changed-file detection

### From Source (Pre-release)

```bash
# Clone and build
git clone https://github.com/EddaCraft/anvil-001.git
cd anvil-001
pnpm install
pnpm build

# Link CLI globally
pnpm link:cli

# Verify
anvil --version

# To unlink later
pnpm unlink:cli
```

### Future Installation (npm)

```bash
npm install -g @anvil/cli
```

## Core Commands

### `anvil check`

Analyse files for architecture violations and anti-patterns.

**Syntax**:

```bash
anvil check [files...] [options]
```

**Options**:

| Option          | Description                                 |
| --------------- | ------------------------------------------- |
| `--changed`     | Analyse git-changed files only              |
| `--staged`      | With `--changed`, analyse only staged files |
| `--since <ref>` | With `--changed`, compare against git ref   |
| `-v, --verbose` | Show explanations and fix suggestions       |
| `--json`        | Output results as JSON                      |
| `--no-cache`    | Disable caching                             |

**Examples**:

```bash
# Check specific files
anvil check src/api/handler.ts src/utils/parser.ts

# Check all changed files (staged + unstaged)
anvil check --changed

# Check only staged files (pre-commit)
anvil check --changed --staged

# Check against main branch
anvil check --changed --since main

# Verbose output with fix suggestions
anvil check --changed --verbose

# JSON for CI/CD
anvil check --changed --json
```

**Output (Human)**:

```
Checked 3 changed file(s)

Warnings:

Errors:
  ✗ [AP-001] Broad eslint-disable found
    src/legacy/handler.ts:1
    Disabling all ESLint rules in this file
    Why: Broad ESLint disables hide all linting issues
    Fix: Use specific rule disables: /* eslint-disable specific-rule */

Warnings:
  ⚠ [AP-003] Explicit any type detected
    src/api/handler.ts:42
    Using 'any' defeats type safety

Summary:
  Total: 2
  Errors: 1
  Warnings: 1
  Time: 45ms
```

**Output (JSON)**:

```json
{
  "version": "1.0.0",
  "timestamp": "2025-01-02T12:00:00Z",
  "files": ["src/api/handler.ts"],
  "hasBlockingWarnings": false,
  "executionTimeMs": 45,
  "checksRun": ["architecture", "antipattern"],
  "warnings": [
    {
      "id": "AP-003",
      "category": "antipattern",
      "severity": "warning",
      "title": "Explicit any type detected",
      "message": "Using 'any' defeats type safety",
      "file": "src/api/handler.ts",
      "line": 42,
      "suggestion": "Define a proper type or use 'unknown'"
    }
  ],
  "summary": {
    "total": 1,
    "errors": 0,
    "warnings": 1,
    "info": 0,
    "suppressed": 0
  }
}
```

---

### `anvil watch`

Watch files for changes and run checks in real-time.

**Syntax**:

```bash
anvil watch [file] [options]
```

**Options**:

| Option                | Description                                      |
| --------------------- | ------------------------------------------------ |
| `--source`            | Watch source files and run checks                |
| `-a, --action`        | Action to run: `validate`, `gate`, or `check`    |
| `--patterns <glob>`   | Glob patterns to watch (comma-separated)         |
| `--exclude <glob>`    | Patterns to exclude (comma-separated)            |
| `--debounce <ms>`     | Debounce interval in milliseconds (default: 300) |
| `--include-untracked` | Include untracked git files                      |
| `--no-git-filter`     | Watch all changes, not just git-tracked          |
| `-p, --profile`       | Gate profile: `dev`, `ci`, `production`          |
| `-v, --verbose`       | Verbose output                                   |

**Examples**:

```bash
# Watch source files and run checks (recommended)
anvil watch --source

# Watch specific patterns
anvil watch --patterns "src/**/*.ts,lib/**/*.ts"

# Watch with dev profile (skip slow checks)
anvil watch --source --profile dev

# Watch a specific file
anvil watch src/api/handler.ts

# Watch plans and run gate checks
anvil watch docs/plans/*.md --action gate
```

**Output**:

```
ANVIL WATCH

  Mode: source files → check
  Patterns: src/**/*.ts, **/*.tsx
  Git filter: unstaged changes only

  ◉ Watching for changes... (Ctrl+C to stop)

  [14:32:05] Change detected: src/api/handler.ts
  [14:32:05] ✓ 0 warnings (23ms)

  [14:35:12] Change detected: src/utils/parser.ts
  [14:35:12] ⚠ 1 warning (31ms)
             AP-003: Explicit any type detected
```

---

### `anvil status`

Show Anvil workspace status at a glance.

**Syntax**:

```bash
anvil status [options]
```

**Options**:

| Option     | Description             |
| ---------- | ----------------------- |
| `--json`   | Output as JSON          |
| `--tui`    | Force TUI dashboard     |
| `--no-tui` | Force plain text output |

**Example**:

```bash
anvil status
```

**Output**:

```
ANVIL STATUS

Project: my-project

• HOOKS
  ✓ pre-commit: active
  ✓ pre-push: active

• CONFIGURATION
  Plans: docs/plans
  Format: speckit
  Coverage: 80%
  Checks: lint, test, coverage, secrets

• RECENT RESULTS
  ✓ docs/plans/spec.md — 4/4 checks
  ✓ docs/plans/feature.md — 4/4 checks
```

---

### `anvil doctor`

Run diagnostic checks and fix common issues.

**Syntax**:

```bash
anvil doctor [options]
```

**Options**:

| Option      | Description                 |
| ----------- | --------------------------- |
| `--fix`     | Auto-fix all fixable issues |
| `--json`    | Output as JSON              |
| `--verbose` | Show detailed diagnostics   |
| `--no-tui`  | Force plain text mode       |

**What it checks**:

1. **System Requirements** — Node.js version, git availability
2. **Configuration** — `.anvilrc` exists and is valid
3. **Hooks** — Git hooks installed and executable
4. **Permissions** — Directories are writable/readable

**Example**:

```bash
anvil doctor

# Output:
[ok] Node.js Version: v22.0.0
[ok] Git Available: git 2.43.0
[ok] Configuration File: .anvilrc found
[ok] Configuration Valid: Valid JSON
[!] Pre-commit Hook: not executable
   -> Run: chmod +x .husky/pre-commit

9 passed • 1 warning

1 issue(s) can be auto-fixed with: anvil doctor --fix
```

**Auto-fix**:

```bash
anvil doctor --fix
```

---

### `anvil hooks`

Manage git hooks integration.

**Syntax**:

```bash
anvil hooks <command> [options]
```

**Commands**:

| Command     | Description              |
| ----------- | ------------------------ |
| `install`   | Install Anvil git hooks  |
| `uninstall` | Remove Anvil git hooks   |
| `status`    | Show current hook status |

**Options for install**:

| Option    | Description                         |
| --------- | ----------------------------------- |
| `--husky` | Integrate with existing Husky setup |
| `--force` | Overwrite existing hooks            |

**Examples**:

```bash
# Install pre-commit and pre-push hooks
anvil hooks install

# Integrate with existing Husky
anvil hooks install --husky

# Check hook status
anvil hooks status

# Remove hooks
anvil hooks uninstall
```

**Installed hooks**:

- **pre-commit**: Runs `anvil check --changed --staged`
- **pre-push**: Runs `anvil gate` (if configured)

**Skip hooks** (emergency bypass):

```bash
ANVIL_SKIP_HOOKS=1 git commit -m "WIP"
```

---

### `anvil init`

Initialise Anvil in the current project.

**Syntax**:

```bash
anvil init [options]
```

**Options**:

| Option              | Description                      |
| ------------------- | -------------------------------- |
| `--force`           | Overwrite existing configuration |
| `--non-interactive` | Use defaults without prompting   |

**What it does**:

1. Detects your environment (TypeScript, ESLint, testing framework)
2. Creates `.anvilrc` with recommended settings
3. Sets up `.anvil/` directory
4. Optionally creates example documents
5. Updates `.gitignore`

**Example**:

```bash
anvil init

# Interactive prompts:
? Where should planning documents be stored? docs/plans
? Which planning format do you use? SpecKit
? Create example document? Yes
? Enable pre-commit hook? Yes
? Coverage threshold (0-100): 80
```

---

## Plan Commands

These commands work with planning documents (SpecKit, BMAD, or generic
markdown).

### `anvil validate`

Validate a planning document for schema correctness.

**Syntax**:

```bash
anvil validate <plan-file> [options]
```

**Options**:

| Option               | Description               |
| -------------------- | ------------------------- |
| `-v, --verbose`      | Detailed output           |
| `--format <format>`  | Override format detection |
| `--native`           | Treat as native APS       |
| `--no-validate-hash` | Skip hash validation      |

**Examples**:

```bash
anvil validate spec.md
anvil validate prd.md --verbose
anvil validate plan.json --native
```

---

### `anvil gate`

Run quality gates on a planning document.

**Syntax**:

```bash
anvil gate <plan-file> [options]
```

**Options**:

| Option                 | Description                            |
| ---------------------- | -------------------------------------- |
| `-c, --config <path>`  | Custom gate configuration              |
| `-v, --verbose`        | Verbose output                         |
| `--format <format>`    | Override format detection              |
| `--only-checks <list>` | Run only specified checks              |
| `--skip-checks <list>` | Skip specified checks                  |
| `--fail-fast`          | Stop on first failure                  |
| `-p, --profile <name>` | Use gate profile (dev, ci, production) |

**Examples**:

```bash
anvil gate spec.md
anvil gate spec.md --only-checks lint,test
anvil gate spec.md --skip-checks coverage
anvil gate spec.md --profile dev
```

**Output**:

```
Gate Results:
┌──────────┬────────┬─────────┬─────────────────────────────┐
│ Check    │ Status │ Score   │ Message                     │
├──────────┼────────┼─────────┼─────────────────────────────┤
│ lint     │ ✓ PASS │ 100/100 │ No linting errors found     │
│ test     │ ✓ PASS │ 100/100 │ All tests passing           │
│ coverage │ ✓ PASS │  85/100 │ Coverage: 85% (≥80%)        │
│ secrets  │ ✓ PASS │ 100/100 │ No secrets detected         │
└──────────┴────────┴─────────┴─────────────────────────────┘

Overall: ✓ PASSED (4/4 checks passed)
```

---

### `anvil export`

Convert plans between formats.

**Syntax**:

```bash
anvil export <source> --to <format> [options]
```

**Options**:

| Option            | Description                             |
| ----------------- | --------------------------------------- |
| `--to <format>`   | Target format: aps, json, yaml, speckit |
| `--output <path>` | Output file or directory                |
| `--from <format>` | Source format (auto-detected)           |
| `--compact`       | Compact JSON output                     |

**Examples**:

```bash
anvil export spec.md --to aps
anvil export spec.md --to yaml --output plan.yaml
anvil export plan.json --to speckit --output ./output/
```

---

## Configuration

### `.anvilrc`

Main configuration file in your project root:

```json
{
  "checks": {
    "antipattern": {
      "enabled": true,
      "patterns": ["AP-001", "AP-003", "AP-004", "AP-006"],
      "allowlist": ["AP-007"]
    },
    "architecture": {
      "enabled": true,
      "baseline": ".anvil/baseline.json"
    },
    "lint": {
      "enabled": true,
      "command": "pnpm lint"
    },
    "test": {
      "enabled": true,
      "command": "pnpm test",
      "timeout": 60000
    },
    "coverage": {
      "enabled": true,
      "threshold": 80
    },
    "secrets": {
      "enabled": true,
      "patterns": ["password", "api_key", "secret", "token"]
    }
  },
  "watch": {
    "patterns": ["src/**/*.ts", "src/**/*.tsx"],
    "exclude": ["**/*.test.ts", "**/__tests__/**"],
    "debounceMs": 300,
    "git": {
      "unstagedOnly": true,
      "includeUntracked": false
    }
  },
  "profiles": {
    "dev": {
      "skipChecks": ["coverage", "dependency"]
    },
    "ci": {
      "failOnWarnings": true
    }
  }
}
```

### Gate Profiles

```bash
# Dev: Skip slow checks for fast iteration
anvil check --changed --profile dev

# CI: Full checks, fail on warnings
anvil check --changed --profile ci

# Production: Strictest checks
anvil check --changed --profile production
```

### Environment Variables

| Variable             | Description                   |
| -------------------- | ----------------------------- |
| `ANVIL_SKIP_HOOKS`   | Skip git hooks (set to 1)     |
| `ANVIL_SKIP_GATES`   | Comma-separated gates to skip |
| `ANVIL_NO_CACHE`     | Disable caching               |
| `ANVIL_SKIP_WELCOME` | Skip first-run welcome screen |

---

## Anti-Pattern Detection

### Built-in Patterns

| ID     | Pattern                      | Severity | Default |
| ------ | ---------------------------- | -------- | ------- |
| AP-001 | Broad `/* eslint-disable */` | warning  | enabled |
| AP-003 | Explicit `any` type          | warning  | enabled |
| AP-004 | `@ts-ignore` directive       | warning  | enabled |
| AP-006 | Empty catch block            | warning  | enabled |
| AP-007 | Console in production        | info     | opt-in  |

### Enabling/Disabling Patterns

```json
{
  "checks": {
    "antipattern": {
      "patterns": ["AP-001", "AP-003", "AP-004", "AP-006"],
      "allowlist": ["AP-007"]
    }
  }
}
```

---

## Architecture Safety

### How It Works

Anvil tracks dependencies between files and detects when code crosses
architectural boundaries:

1. **Baseline**: Existing dependencies are recorded (not warned)
2. **Detection**: New dependencies are checked against rules
3. **Warning**: Violations produce warnings with explanations

### Architecture Configuration

Define layers in `.dependency-cruiser.js`:

```javascript
module.exports = {
  forbidden: [
    {
      name: 'api-to-database',
      from: { path: '^src/api' },
      to: { path: '^src/database' },
      severity: 'warn',
    },
  ],
};
```

### Example Warning

```
⚠ [ARCH-001] New cross-boundary dependency
  src/api/handler.ts → src/database/queries.ts
  API layer should not directly access database layer
  Consider: Use the service layer as intermediary
```

---

## Suppressions

### Syntax

```typescript
// @anvil-ignore <ID>: <reason>
const handler = sdk.createHandler(callback as any);
```

### Examples

```typescript
// @anvil-ignore AP-003: SDK requires any for legacy callback
const legacyCallback: any = processData;

// @anvil-ignore AP-004: Type definition missing from @types/library
// @ts-ignore
import { undefinedExport } from 'untyped-library';

// @anvil-ignore AP-006: Intentionally swallowing expected error
try {
  optional.cleanup();
} catch {}
```

### Tracking Suppressions

```bash
anvil status  # Shows suppression count
```

---

## CI/CD Integration

### GitHub Action

```yaml
name: Anvil Check

on:
  pull_request:
    types: [opened, synchronize, reopened]

permissions:
  contents: read
  pull-requests: write
  checks: write

jobs:
  anvil:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: ./.github/actions/anvil-check
        with:
          fail-on-warnings: 'false' # Set to 'true' to block PRs
```

### GitLab CI

```yaml
anvil-check:
  stage: test
  image: node:22
  script:
    - npx @anvil/cli check --changed --json > anvil-report.json
  artifacts:
    reports:
      codequality: anvil-report.json
```

### Pre-commit Hook

```bash
anvil hooks install
```

Or manually in `.husky/pre-commit`:

```bash
#!/bin/sh
anvil check --changed --staged || exit 1
```

---

## Best Practices

### 1. Start with Anti-Patterns Only

```json
{
  "checks": {
    "antipattern": { "enabled": true },
    "architecture": { "enabled": false }
  }
}
```

Enable architecture checks after establishing baseline.

### 2. Use Watch Mode During Development

```bash
anvil watch --source
```

Get feedback immediately, fix issues before commit.

### 3. Configure CI to Warn, Not Block

```yaml
- uses: ./.github/actions/anvil-check
  with:
    fail-on-warnings: 'false'
```

Let developers see issues without blocking velocity.

### 4. Review Suppressions in Code Review

Suppressions should be intentional. Review the reasons.

### 5. Gradually Increase Thresholds

Start with warnings, then enable blocking:

```json
{
  "profiles": {
    "dev": { "skipChecks": ["coverage"] },
    "ci": { "failOnWarnings": false },
    "production": { "failOnWarnings": true }
  }
}
```

---

## Next Steps

- **[Examples](./EXAMPLES.md)** — Real-world workflows
- **[Troubleshooting](./TROUBLESHOOTING.md)** — Common issues
- **[CLI Reference](../cli/README.md)** — All commands
- **[Architecture](./ARCHITECTURE.md)** — System design

---

**Version**: 1.0.0 | **Last Updated**: January 2026
