---
id: team-flow
title: Team Flow
description: Integrating anvil into team workflows with CI/CD and code review.
sidebar_position: 2
---

# Team Flow

This guide covers anvil workflows for teams, including CI integration, PR
checks, and governance.

## Overview

Team workflow adds layers to the solo flow:

```
Developer → Local anvil → Push → CI anvil → PR Review → Merge
              (catch)            (enforce)   (verify)
```

**Local anvil** catches issues early. **CI anvil** enforces standards.
**Review** verifies intent.

## CI Integration

### GitHub Actions

Add anvil to your CI workflow:

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

      - name: Install anvil
        run: curl -fsSL https://install.eddacraft.ai | sh

      - name: Run anvil
        env:
          ANVIL_LICENSE: ${{ secrets.ANVIL_LICENSE }}
        run: anvil gate --profile ci
```

### CI Mode Behaviour

The `ci` profile adjusts behaviour for pipeline environments:

| Aspect    | Interactive             | CI (`--profile ci`)     |
| --------- | ----------------------- | ----------------------- |
| Output    | Terminal UI             | Plain text              |
| Colours   | Yes                     | No (TTY detection)      |
| Exit code | 0 (pass) / 1 / 2 (fail) | 0 (pass) / 1 / 2 (fail) |
| Checks    | Profile-dependent       | No profile-level skips  |

### Exit Codes

| Code | Meaning          | Action            |
| ---- | ---------------- | ----------------- |
| 0    | All gates passed | Continue          |
| 1    | General error    | Investigate       |
| 2    | Gate failure     | Block merge       |
| 3    | Auth required    | Check credentials |
| 4    | Config error     | Fix configuration |

Configure which checks run by using `.anvilrc#checks` as the persistent default
filter, or `--only-checks` / `--skip-checks` for a specific command invocation:

```bash
anvil gate --profile ci --skip-checks coverage,dependency
```

## PR Comments

anvil can post results as PR comments:

```yaml
- name: Run anvil
  run: anvil --json gate --profile ci > anvil-results.json

- name: Comment on PR
  if: github.event_name == 'pull_request'
  uses: actions/github-script@v7
  with:
    script: |
      const results = require('./anvil-results.json');
      // Post formatted comment
```

For automated PR comments, use the `actions/github-script` approach above or a
custom workflow step that parses `anvil-results.json` and posts via the GitHub
API.

## Branch Protection

Require anvil to pass before merge:

1. Go to **Settings → Branches → Branch protection rules**
2. Add rule for `main`
3. Check **Require status checks to pass**
4. Select **anvil** from the list

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

Watch behaviour is controlled with CLI flags per session rather than repo
config. For example:

```bash
anvil watch --source --debounce 500
anvil watch --source --file src/payments/
anvil watch --source --exclude "dist/**,node_modules/**"
```

If your team still uses local config overrides for non-watch settings, keep them
out of git:

```
.anvilrc.local
```

Use `--file` and `--debounce` as the primary day-to-day tuning knobs. Use glob
patterns such as `dist/**` for `--exclude`; bare names only match that exact
path.

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
- name: Create Review Capsule
  env:
    BASE_SHA: ${{ github.event.pull_request.base.sha }}
    HEAD_SHA: ${{ github.sha }}
  run: anvil capsule create --range "$BASE_SHA..$HEAD_SHA" --out review-capsule
- name: Upload Evidence
  uses: actions/upload-artifact@v4
  with:
    name: anvil-review-capsule
    path: review-capsule/
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
      - name: Install anvil
        run: curl -fsSL https://install.eddacraft.ai | sh

      - name: Audit witness coverage
        run: anvil audit-chain --json > audit-chain.json
      - uses: actions/upload-artifact@v4
        with:
          name: weekly-audit
          path: audit-chain.json
```

## Rollout Strategy

### Phase 1: Shadow Mode

Run anvil in CI without blocking:

```yaml
- name: Run anvil (Shadow)
  env:
    ANVIL_LICENSE: ${{ secrets.ANVIL_LICENSE }}
  run: anvil gate --profile ci || true
  continue-on-error: true
```

Collect data on what would fail.

### Phase 2: Focused Gate

Start by enforcing the checks your team is ready to act on:

```bash
anvil gate --profile ci --only-checks import-boundaries,antipattern-scan,secret-detection
```

### Phase 3: Strict Mode

Expand the gate to the full CI profile:

```bash
anvil gate --profile ci
```

### Phase 4: Full Governance

Add evidence, auditing, and approval workflows.

---

**Next:** [Agent harness patterns →](/anvil/guides/agent-harness)
