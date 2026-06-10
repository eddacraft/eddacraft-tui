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

To cover all three platforms in one job, use a matrix:

```yaml
jobs:
  anvil:
    strategy:
      matrix:
        include:
          - os: ubuntu-latest
            install: curl -fsSL https://install.eddacraft.ai | sh
          - os: windows-latest
            install: irm https://install.eddacraft.ai/windows | iex
            shell: pwsh
          - os: macos-latest
            install: curl -fsSL https://install.eddacraft.ai | sh
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4

      - name: Install anvil
        shell: ${{ matrix.shell || 'bash' }}
        run: ${{ matrix.install }}

      - name: Run anvil
        env:
          ANVIL_LICENSE: ${{ secrets.ANVIL_LICENSE }}
        run: anvil gate --profile ci
```

:::tip CI authentication

If your gate checks require beta access, set `ANVIL_LICENSE` from your CI secret
store before running anvil. Locally you use `anvil auth login`; CI should use a
secret value, not an interactive login.

:::

The `ci` profile runs all check categories (no skips). Output mode and
interactivity are controlled separately by TTY detection, `--json`, and
`--progress` flags — the profile selects which checks to run and which
thresholds to apply.

### Where CI fits

CI is the outermost of anvil's protection layers — the safety net behind
save-time validation (`anvil watch` / MCP pre-write, see the
[save-time guide](../guides/save-time-validation.md)) and commit-time git hooks
(see [Git hook setup](../operations/git-hooks.md)). Issues caught at inner
layers never reach CI; the goal is to catch everything at save-time.

### Other CI systems

The same install-and-gate pattern works on any runner with the exit codes below.
GitLab CI, for example:

```yaml
# .gitlab-ci.yml
anvil:
  stage: test
  variables:
    ANVIL_LICENSE: $ANVIL_LICENSE
  before_script:
    - curl -fsSL https://install.eddacraft.ai | sh
  script:
    - anvil gate --profile ci
  rules:
    - if: $CI_PIPELINE_SOURCE == "merge_request_event"
    - if: $CI_COMMIT_BRANCH == $CI_DEFAULT_BRANCH
```

### Exit Codes

| Code | Meaning          | Action            |
| ---- | ---------------- | ----------------- |
| `0`  | All gates passed | Continue          |
| `1`  | General error    | Investigate       |
| `2`  | Gate failure     | Block merge       |
| `3`  | Auth required    | Check credentials |
| `4`  | Config error     | Fix configuration |

## Code Scanning (SARIF)

`anvil check`, `anvil gate`, and `anvil audit` can emit
[SARIF 2.1.0](https://docs.oasis-open.org/sarif/sarif/v2.1.0/sarif-v2.1.0.html)
with `--format sarif`, so their findings can be uploaded to GitHub Code Scanning
and shown in the **Security** tab.

```yaml
- name: anvil check (SARIF)
  run: anvil check --all --format sarif > anvil.sarif

- name: Upload to Code Scanning
  uses: github/codeql-action/upload-sarif@v3
  with:
    sarif_file: anvil.sarif
```

`--format` is the canonical output selector
(`auto | tui | plain | json | sarif`) on these three commands. `--json` keeps
working as an alias for `--format json`; `sarif` must be requested explicitly
(it is never auto-selected) and is rejected on commands that do not emit
findings. SARIF emission is **exit-code-neutral** — a failing `gate` still exits
`2` — so capture stdout to a file rather than gating the upload on the exit
code.

### What is in the SARIF

Anvil emits the GitHub Code Scanning ingest subset of SARIF 2.1.0:
`runs[].tool.driver` (named `anvil`) with the encountered `rules[]`, and
`results[]` with `ruleId`, `level`, `message`, `locations[]`, and a stable
`partialFingerprints` entry so Code Scanning dedupes findings across runs.

- **`anvil check`** — one result per warning (anti-pattern or secret); `ruleId`
  is the pattern id or a `SECRET-*` id; `locations[]` carry file + line.
  `@anvil-ignore`-suppressed findings render under `suppressions[]`.
- **`anvil audit`** — one result per audit issue; `ruleId` is the issue
  category; `locations[]` carry file + line (omitted for whole-file findings).
- **`anvil gate`** — one result per failed or config-needed check; `ruleId` is
  the check name; results are repo-level with no location (gate findings are
  per-check aggregates). Config-needed checks are reported at `note` level so
  they do not inflate the failure set.

Gate results have no file location, so some Code Scanning views may not surface
them. Full SARIF 2.1.0 conformance (code flows, taxonomies, fixes, multi-run) is
out of scope.

## PR Comments

Anvil does not post PR comments by itself. If you want a comment, run with JSON
output and add a workflow step that formats the result.

```markdown
<!-- anvil-check-results -->

## Anvil Gate Results

✓ All gates passed

| Check             | Status | Duration |
| ----------------- | ------ | -------- |
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
