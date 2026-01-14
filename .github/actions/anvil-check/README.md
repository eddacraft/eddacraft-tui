# Anvil Check Action

Run Anvil quality gates on your codebase as part of your GitHub Actions
workflow.

## Quick Start

```yaml
- name: Run Anvil Check
  uses: ./.github/actions/anvil-check
  # Node version auto-detected from .node-version, .nvmrc, or package.json
```

## Inputs

| Input               | Description                                                                | Required | Default        |
| ------------------- | -------------------------------------------------------------------------- | -------- | -------------- |
| `node-version`      | Explicit Node.js version (e.g., `20`, `22`). If empty, auto-detects.       | No       | `''`           |
| `node-version-file` | File to read Node version from                                             | No       | `''`           |
| `working-directory` | Directory to run Anvil in (for monorepos)                                  | No       | `.`            |
| `fail-on-warnings`  | Fail the check if warnings are found                                       | No       | `false`        |
| `check-type`        | Type of check: `gate` (full) or `check` (quick)                            | No       | `check`        |
| `files`             | Specific files to check (space-separated)                                  | No       | `''`           |
| `auto-detect-files` | Auto-detect changed files in PR/push                                       | No       | `true`         |
| `github_token`      | GitHub token for API access. Use for custom tokens (e.g., `GH_AUTH_TOKEN`) | No       | `github.token` |

## Outputs

| Output           | Description                                         |
| ---------------- | --------------------------------------------------- |
| `warnings-count` | Total number of warnings found                      |
| `errors-count`   | Total number of errors found                        |
| `result-json`    | Full JSON result from Anvil                         |
| `exit-code`      | Exit code from Anvil (0=pass, 1=warnings, 2=errors) |
| `files-checked`  | List of files that were checked                     |

## Features

### PR Comments

On pull requests, Anvil automatically posts a comment summarising the results:

- ✅ **Passed** — No issues found
- ⚠️ **Warnings** — Issues found (non-blocking by default)
- ❌ **Errors** — Issues that must be fixed

The comment is updated on subsequent runs, not duplicated.

### Commit Status

Anvil sets a commit status (`Anvil Check`) that appears in the PR:

| Scenario                      | Status     | Description                      |
| ----------------------------- | ---------- | -------------------------------- |
| No issues                     | ✅ success | All checks passed                |
| Warnings only                 | ✅ success | Non-blocking (informational)     |
| Warnings + `fail-on-warnings` | ❌ failure | Blocking mode enabled            |
| Errors                        | ❌ failure | Errors must be fixed             |
| Skipped (no files)            | ✅ success | No analysable files in changeset |

### Inline Annotations

When warnings are found, Anvil creates a Check Run with inline annotations that
appear directly in the PR's "Files changed" tab. This makes it easy to see
exactly where issues occur without leaving the PR review.

> **Note**: GitHub limits annotations to 50 per check run. If more issues exist,
> see the full output in the Actions tab.

### Changed Files Detection

By default (`auto-detect-files: true`), the action only checks files that
changed in the PR or push. This speeds up CI and focuses feedback on relevant
changes.

Supported file types: `.ts`, `.tsx`, `.js`, `.jsx`, `.mjs`, `.cjs`

To check all files instead:

```yaml
- uses: ./.github/actions/anvil-check
  with:
    auto-detect-files: 'false'
```

## Node Version Auto-Detection

The action automatically detects your project's Node.js version in this order:

1. **Explicit `node-version` input** — Use this to override everything
2. **Explicit `node-version-file` input** — Specify a custom file
3. **`.node-version`** — Auto-detected if present
4. **`.nvmrc`** — Auto-detected if present
5. **`package.json` engines.node** — Auto-detected if present
6. **Fallback to Node 20** — Anvil's minimum supported version

## Required Permissions

Add these permissions to your workflow:

```yaml
permissions:
  contents: read
  pull-requests: write # For PR comments
  statuses: write # For commit status
  checks: write # For inline annotations
```

## Examples

### Basic Usage (Auto-Detect Everything)

```yaml
name: Anvil Check

on:
  pull_request:
    types: [opened, synchronize, reopened]

permissions:
  contents: read
  pull-requests: write
  statuses: write
  checks: write

jobs:
  anvil:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: ./.github/actions/anvil-check
```

### Explicit Node Version

```yaml
- uses: ./.github/actions/anvil-check
  with:
    node-version: '22'
```

### Monorepo with Matrix

```yaml
jobs:
  anvil:
    runs-on: ubuntu-latest
    strategy:
      matrix:
        package: [core, cli, adapters]
    steps:
      - uses: actions/checkout@v4
      - uses: ./.github/actions/anvil-check
        with:
          working-directory: packages/${{ matrix.package }}
```

### Blocking Mode (Fail on Warnings)

```yaml
- uses: ./.github/actions/anvil-check
  with:
    fail-on-warnings: 'true'
```

### Using Outputs

```yaml
- uses: ./.github/actions/anvil-check
  id: anvil

- name: Check results
  run: |
    echo "Warnings: ${{ steps.anvil.outputs.warnings-count }}"
    echo "Errors: ${{ steps.anvil.outputs.errors-count }}"
```

### Using a Custom GitHub Token

If the default `github.token` doesn't have sufficient permissions (e.g., "User
does not have write access"), provide a custom token:

```yaml
- uses: ./.github/actions/anvil-check
  with:
    github_token: ${{ secrets.GH_AUTH_TOKEN }}
```

## Troubleshooting

### "Command not found: anvil"

The action installs Anvil globally via npm. Ensure your workflow has network
access to npm registry.

### "No Node version file found"

If you don't have `.node-version`, `.nvmrc`, or `package.json` with
`engines.node`, the action falls back to Node 20. Add a version file or use the
`node-version` input explicitly.

### "User does not have write access on this repository"

This error occurs when the default `github.token` lacks sufficient permissions.
To fix this:

1. **Option 1: Create a Personal Access Token (PAT)** or **GitHub App token**
   with appropriate permissions
2. **Option 2: Add the token as a repository secret** (e.g., `GH_AUTH_TOKEN`)
3. **Option 3: Use the custom token in your workflow:**

```yaml
- uses: ./.github/actions/anvil-check
  with:
    github_token: ${{ secrets.GH_AUTH_TOKEN }}
```

### Warnings not blocking PR

By default, warnings are informational (non-blocking). To enforce blocking:

```yaml
- uses: ./.github/actions/anvil-check
  with:
    fail-on-warnings: 'true'
```

### No PR comment appearing

Ensure your workflow has `pull-requests: write` permission:

```yaml
permissions:
  pull-requests: write
```

### No commit status appearing

Ensure your workflow has `statuses: write` permission:

```yaml
permissions:
  statuses: write
```

### No inline annotations appearing

Ensure your workflow has `checks: write` permission and there are warnings to
annotate:

```yaml
permissions:
  checks: write
```

### "Resource not accessible by integration" error

This usually means missing permissions. Add all required permissions to your
workflow (see Required Permissions section above).
