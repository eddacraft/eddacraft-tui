---
id: team-flow
title: Team Flow
description: Integrating Anvil into team workflows with CI/CD and code review.
sidebar_position: 2
---

# Team Flow

This guide covers Anvil workflows for teams, including CI integration, PR
checks, and governance.

## Overview

Team workflow adds layers to the solo flow:

```
Developer → Local Anvil → Push → CI Anvil → PR Review → Merge
              (catch)            (enforce)   (verify)
```

**Local Anvil** catches issues early. **CI Anvil** enforces standards.
**Review** verifies intent.

## CI Integration

### GitHub Actions

Add Anvil to your CI workflow:

```yaml
# .github/workflows/ci.yml
name: CI

on:
  pull_request:
    branches: [main]
  push:
    branches: [main]

jobs:
  anvil:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install Anvil
        run: curl -fsSL https://install.eddacraft.ai | sh

      - name: Run Anvil
        run: anvil gate --profile ci
```

### CI Mode Behaviour

The `ci` profile adjusts behaviour for pipeline environments:

| Aspect    | Interactive             | CI (`--profile ci`)     |
| --------- | ----------------------- | ----------------------- |
| Output    | Terminal UI             | Plain text              |
| Colours   | Yes                     | No (TTY detection)      |
| Exit code | 0 (pass) / 1 / 2 (fail) | 0 (pass) / 1 / 2 (fail) |
| Checks    | Profile-dependent       | All checks enabled      |

### Exit Codes

| Code | Meaning          | Action       |
| ---- | ---------------- | ------------ |
| 0    | All gates passed | Continue     |
| 1    | General error    | Investigate  |
| 2    | Gate failure     | Block merge  |

Configure warning behaviour:

```json
{
  "ci": {
    "fail_on_warnings": false
  }
}
```

## PR Comments

Anvil can post results as PR comments:

```yaml
- name: Run Anvil
  run: anvil gate --profile ci --json > anvil-results.json

- name: Comment on PR
  if: github.event_name == 'pull_request'
  uses: actions/github-script@v7
  with:
    script: |
      const results = require('./anvil-results.json');
      // Post formatted comment
```

Or use the Anvil GitHub Action:

```yaml
- uses: eddacraft/anvil-action@v1
  with:
    github_token: ${{ secrets.GITHUB_TOKEN }}
    comment: true
```

## Branch Protection

Require Anvil to pass before merge:

1. Go to **Settings → Branches → Branch protection rules**
2. Add rule for `main`
3. Check **Require status checks to pass**
4. Select **Anvil** from the list

## Team Configuration

### Shared Config

Store configuration in the repo root:

```
project/
├── .anvilrc      # Shared team config
├── .anvilrc.local       # Personal overrides (gitignored)
└── ...
```

### Local Overrides

Developers can override for their environment:

```json
// .anvilrc.local
{
  "extends": "./.anvilrc",
  "watch": {
    "debounce_ms": 500
  }
}
```

Add to `.gitignore`:

```
.anvilrc.local
```

### Team-Wide Suppressions

Suppress known issues team-wide:

```json
{
  "suppressions": [
    {
      "pattern": "src/legacy/**",
      "checks": ["AP-003"],
      "reason": "Legacy code migration in progress (JIRA-123)"
    }
  ]
}
```

## Governance Workflow

For teams needing approval workflows:

### 1. Suppression Approval

Require PR review for new suppressions:

```yaml
# .github/CODEOWNERS
.anvilrc @team/architecture **/anvil-ignore* @team/leads
```

### 2. Evidence Review

Attach evidence to PRs:

```yaml
- name: Upload Evidence
  uses: actions/upload-artifact@v4
  with:
    name: anvil-evidence
    path: .anvil/evidence/
```

### 3. Audit Export

Regular export for compliance:

```yaml
# .github/workflows/audit.yml
on:
  schedule:
    - cron: '0 0 * * 0' # Weekly

jobs:
  audit:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Install Anvil
        run: curl -fsSL https://install.eddacraft.ai | sh

      # Evidence export is planned for a future release.
      # For now, copy the evidence directory directly.
      - run: cp -r .anvil/evidence/ audit/
      - uses: actions/upload-artifact@v4
        with:
          name: weekly-audit
          path: audit/
```

## Rollout Strategy

### Phase 1: Shadow Mode

Run Anvil in CI without blocking:

```yaml
- name: Run Anvil (Shadow)
  run: anvil gate --profile ci || true
  continue-on-error: true
```

Collect data on what would fail.

### Phase 2: Warn Mode

Fail on errors, warn on anti-patterns:

```json
{
  "ci": {
    "fail_on_warnings": false
  }
}
```

### Phase 3: Strict Mode

All issues block:

```json
{
  "ci": {
    "fail_on_warnings": true
  }
}
```

### Phase 4: Full Governance

Add evidence, auditing, and approval workflows.

---

**Next:** [Agent harness patterns →](/anvil/guides/agent-harness)
