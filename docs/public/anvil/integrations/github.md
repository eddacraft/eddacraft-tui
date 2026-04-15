---
id: github
title: GitHub Integration
description: Using anvil with GitHub Actions and PR checks.
sidebar_position: 1
---

# GitHub Integration

anvil integrates with GitHub for CI/CD validation and PR feedback.

## CI Setup

Install the anvil binary and run the gate check in your workflow:

```yaml
name: anvil CI

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
        run: anvil gate --profile ci
```

For Windows runners, use the PowerShell installer:

```yaml
- name: Install anvil
  shell: pwsh
  run: irm https://install.eddacraft.ai/windows | iex
```

### Exit Codes

| Code | Meaning          | Action            |
| ---- | ---------------- | ----------------- |
| `0`  | All gates passed | Continue          |
| `1`  | General error    | Investigate       |
| `2`  | Gate failure     | Block merge       |
| `3`  | Auth required    | Check credentials |

## PR Comments

When enabled, anvil posts a summary comment:

```markdown
<!-- anvil-check-results -->
## 🔨 Anvil Check Results

✓ All gates passed

| Check         | Status | Duration |
| ------------- | ------ | -------- |
| Architecture  | ✓ Pass | 23ms     |
| Anti-patterns | ✓ Pass | 15ms     |
| Secrets       | ✓ Pass | 8ms      |

[View full evidence](link-to-evidence)
```

### Comment on Failure

```markdown
<!-- anvil-check-results -->
## 🔨 Anvil Check Results

✗ 2 issues found

### Errors

| File               | Line | Issue                       |
| ------------------ | ---- | --------------------------- |
| `src/api/users.ts` | 42   | AP-003: Explicit 'any' type |

### Warnings

| File               | Line | Issue                         |
| ------------------ | ---- | ----------------------------- |
| `src/utils/log.ts` | 15   | AP-007: Console in production |

<details>
<summary>How to fix</summary>

**AP-003**: Replace `any` with a specific type or generic...

</details>
```

## Branch Protection

Require anvil before merge:

1. **Repository Settings** → **Branches**
2. **Add branch protection rule** for `main`
3. Enable **Require status checks to pass**
4. Search for and select **Anvil Check**
5. Save changes

Now PRs cannot merge until anvil passes.

## Check Runs

anvil creates GitHub Check Runs for detailed inline feedback:

- Annotations appear on specific lines in the PR diff
- Expandable details for each issue
- Links to documentation

### Manual Check Run Creation

```yaml
- name: Create Check Run
  uses: actions/github-script@v7
  with:
    script: |
      const results = require('./anvil-results.json');

      await github.rest.checks.create({
        owner: context.repo.owner,
        repo: context.repo.repo,
        name: 'Anvil Check',
        head_sha: context.sha,
        status: 'completed',
        conclusion: results.status === 'pass' ? 'success' : 'failure',
        output: {
          title: 'Anvil Check Results',
          summary: results.summary,
          annotations: results.issues.map(i => ({
            path: i.file,
            start_line: i.line,
            end_line: i.line,
            annotation_level: i.severity === 'error' ? 'failure' : 'warning',
            message: i.message
          }))
        }
      });
```

## Monorepo Support

For monorepos, run anvil per-package using a matrix:

```yaml
jobs:
  anvil:
    runs-on: ubuntu-latest
    strategy:
      matrix:
        package: [core, cli, api]
    steps:
      - uses: actions/checkout@v4

      - name: Install anvil
        run: curl -fsSL https://install.eddacraft.ai | sh

      - name: Run anvil
        working-directory: packages/${{ matrix.package }}
        run: anvil gate --profile ci
```

## Caching

Speed up CI with caching:

```yaml
- uses: actions/cache@v4
  with:
    path: .anvil/cache
    key: anvil-${{ runner.os }}-${{ hashFiles('**/.anvilrc') }}
```

---

**Next:** [VS Code integration →](/anvil/integrations/vscode)
