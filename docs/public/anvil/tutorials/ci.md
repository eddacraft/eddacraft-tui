---
id: ci
title: CI Integration
sidebar_position: 5
---

# CI Integration

Anvil provides three layers of protection. This tutorial covers all three:
save-time, commit-time, and CI-time.

## Protection Layers

```
Save              Commit             CI
 │                  │                 │
 ▼                  ▼                 ▼
anvil watch    git pre-commit    pipeline step
 (instant)      (seconds)         (minutes)
```

Each layer catches issues that slipped past the previous one. Together they form
a defence in depth.

## Layer 1: Save-Time (Watch Mode)

```bash
anvil watch --source
```

Runs in the background, validates on every file save. Fastest feedback loop.

## Layer 2: Commit-Time (Git Hooks)

Add a pre-commit hook that checks staged files:

```bash
# .husky/pre-commit (or .git/hooks/pre-commit)
npx anvil check --staged
```

This blocks commits that introduce violations. Only staged files are checked, so
it stays fast.

## Layer 3: CI-Time (Pipeline)

The final gate. Runs a full check on every push or pull request.

### GitHub Actions

```yaml
# .github/workflows/anvil.yml
name: Anvil

on:
  pull_request:
  push:
    branches: [main]

jobs:
  anvil:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - uses: actions/setup-node@v4
        with:
          node-version: 20

      - name: Install dependencies
        run: pnpm install --frozen-lockfile

      - name: Run Anvil
        run: pnpm anvil check --all --ci
```

### GitLab CI

```yaml
# .gitlab-ci.yml
anvil:
  image: node:20
  stage: test
  script:
    - pnpm install --frozen-lockfile
    - pnpm anvil check --all --ci
  rules:
    - if: $CI_PIPELINE_SOURCE == "merge_request_event"
    - if: $CI_COMMIT_BRANCH == $CI_DEFAULT_BRANCH
```

## Exit Codes

| Code | Meaning                                             |
| ---- | --------------------------------------------------- |
| `0`  | All gates passed                                    |
| `1`  | One or more warnings or failures                    |
| `2`  | Configuration error (missing config, invalid rules) |

CI runners use these exit codes to pass or fail the pipeline step.

## CI-Specific Options

The `--ci` flag adjusts behaviour for pipeline environments:

- Outputs machine-readable JSON alongside human-readable text
- Disables watch mode and interactive prompts
- Writes evidence files to `.anvil/evidence/`

Override via environment variables:

```bash
ANVIL_CI=true ANVIL_FAIL_ON_WARNINGS=true anvil check --all
```

## Layered Protection Diagram

```
┌──────────────────────────────────────────────────┐
│                    CI Pipeline                    │
│  Full check, all gates, evidence trail           │
│  ┌──────────────────────────────────────────┐    │
│  │            Git Pre-Commit                │    │
│  │  Staged files only, fast                 │    │
│  │  ┌──────────────────────────────────┐    │    │
│  │  │          Watch Mode              │    │    │
│  │  │  Instant, per-file               │    │    │
│  │  └──────────────────────────────────┘    │    │
│  └──────────────────────────────────────────┘    │
└──────────────────────────────────────────────────┘
```

Issues caught at inner layers never reach outer ones. The goal is to catch
everything at save-time -- CI exists as the safety net.

:::info The `--staged` flag pairs well with lint-staged or Husky. See
[Configuration](/anvil/operations/config) for hook setup options. :::

---

**Next:** [Suppressions](/anvil/tutorials/suppressions)
