---
id: github
title: GitHub Integration
description: Using Anvil with GitHub Actions and PR checks.
sidebar_position: 1
---

# GitHub Integration

Anvil integrates with GitHub for CI/CD validation and PR feedback.

## GitHub Action

The official Anvil GitHub Action:

```yaml
- uses: eddacraft/anvil-action@v1
  with:
    github_token: ${{ secrets.GITHUB_TOKEN }}
```

### Full Example

```yaml
name: Anvil CI

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

      - uses: pnpm/action-setup@v2
        with:
          version: 10

      - uses: actions/setup-node@v4
        with:
          node-version: 20
          cache: pnpm

      - run: pnpm install

      - uses: eddacraft/anvil-action@v1
        with:
          github_token: ${{ secrets.GITHUB_TOKEN }}
          comment: true
          fail_on_warnings: false
```

### Action Inputs

| Input              | Description                | Default             |
| ------------------ | -------------------------- | ------------------- |
| `github_token`     | Token for PR comments      | Required            |
| `comment`          | Post results as PR comment | `true`              |
| `fail_on_warnings` | Fail workflow on warnings  | `false`             |
| `config`           | Path to config file        | `anvil.config.json` |

### Action Outputs

| Output        | Description               |
| ------------- | ------------------------- |
| `status`      | `pass`, `warn`, or `fail` |
| `errors`      | Number of errors          |
| `warnings`    | Number of warnings        |
| `evidence_id` | ID of generated evidence  |

## PR Comments

When enabled, Anvil posts a summary comment:

```markdown
## Anvil Results

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
## Anvil Results

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

Require Anvil before merge:

1. **Repository Settings** → **Branches**
2. **Add branch protection rule** for `main`
3. Enable **Require status checks to pass**
4. Search and select **Anvil CI**
5. Save changes

Now PRs cannot merge until Anvil passes.

## Check Runs

Anvil creates GitHub Check Runs for detailed inline feedback:

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
        name: 'Anvil',
        head_sha: context.sha,
        status: 'completed',
        conclusion: results.status === 'pass' ? 'success' : 'failure',
        output: {
          title: 'Anvil Results',
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

For monorepos, run Anvil per-package:

```yaml
jobs:
  anvil:
    runs-on: ubuntu-latest
    strategy:
      matrix:
        package: [core, cli, api]
    steps:
      - uses: actions/checkout@v4

      - uses: eddacraft/anvil-action@v1
        with:
          github_token: ${{ secrets.GITHUB_TOKEN }}
          working_directory: packages/${{ matrix.package }}
```

## Caching

Speed up CI with caching:

```yaml
- uses: actions/cache@v4
  with:
    path: .anvil/cache
    key: anvil-${{ runner.os }}-${{ hashFiles('**/anvil.config.json') }}
```

---

**Next:** [VS Code integration →](/anvil/integrations/vscode)
