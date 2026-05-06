---
id: github
title: GitHub Integration
description: Using anvil with GitHub Actions and PR checks.
sidebar_position: 1
---

# GitHub Integration

anvil integrates with GitHub for CI/CD validation and PR feedback.

:::info Current integration shape

Use the CLI directly in GitHub Actions today. A packaged first-party GitHub
Action and automatic hosted evidence links are not part of the current public
surface.

:::

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
        env:
          ANVIL_LICENSE: ${{ secrets.ANVIL_LICENSE }}
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
| `4`  | Config error     | Fix configuration |

## PR Comments

Anvil does not post PR comments by itself. If you want a comment, run with JSON
output and add a workflow step that formats the result.

```markdown
<!-- anvil-check-results -->

## Anvil Gate Results

✓ All gates passed

| Check         | Status | Duration |
| ------------- | ------ | -------- |
| Import boundaries | ✓ Pass | 23ms     |
| Anti-pattern scan | ✓ Pass | 15ms     |
| Secret detection  | ✓ Pass | 8ms      |

Generated from `anvil --json gate --profile ci`.
```

### Comment on Failure

```markdown
<!-- anvil-check-results -->

## Anvil Gate Results

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
4. Search for and select the workflow job name, usually **anvil**
5. Save changes

Now PRs cannot merge until anvil passes.

## Check Runs

Anvil does not create GitHub Check Runs by itself. You can create one from the
JSON output if you want detailed inline feedback:

- Annotations appear on specific lines in the PR diff
- Expandable details for each issue
- Links to documentation

### Manual Check Run Creation

The snippet below is a starting point. Adapt the field mapping to the JSON shape
you get from `anvil --json gate --profile ci` in your version.

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
        env:
          ANVIL_LICENSE: ${{ secrets.ANVIL_LICENSE }}
        working-directory: packages/${{ matrix.package }}
        run: anvil gate --profile ci
```

## Caching

If your workflow persists Anvil caches, use a normal GitHub Actions cache step:

```yaml
- uses: actions/cache@v4
  with:
    path: .anvil/cache
    key: anvil-${{ runner.os }}-${{ hashFiles('**/.anvilrc') }}
```

---

**Next:** [VS Code integration →](/anvil/integrations/vscode)
