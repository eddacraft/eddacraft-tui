---
id: ci
title: CI Integration
sidebar_position: 5
---

# CI Integration

anvil provides three layers of protection. This tutorial covers all three:
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
anvil check --changed --staged
```

This blocks commits that introduce violations. The `--changed --staged` flags
restrict analysis to staged files only, so it stays fast.

On Git 2.54 or newer you can also use `anvil hooks install --config` to install
Anvil's managed pre-commit gate hook through native `[hook.<name>]` config
blocks instead of a file. That installs `ANVIL_HOOK=1 anvil gate --progress`,
which is separate from the manual `anvil check --changed --staged` example above
— keep that one if you want staged-only checks. See
[Git hook setup](/anvil/operations/git-hooks) for both modes, coexistence rules,
and which to pick — file mode remains the default.

## Layer 3: CI-Time (Pipeline)

The final gate. Runs a full check on every push or pull request.

:::tip CI authentication

If your gate checks require beta access, set `ANVIL_LICENSE` from your CI secret
store before running Anvil. Locally you use `anvil auth login`; CI should use a
secret value, not an interactive login.

:::

### GitHub Actions (Linux)

```yaml
# .github/workflows/anvil.yml
name: anvil

on:
  pull_request:
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

### GitHub Actions (Windows)

```yaml
# .github/workflows/anvil.yml
name: anvil

on:
  pull_request:
  push:
    branches: [main]

jobs:
  anvil:
    runs-on: windows-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install anvil
        shell: pwsh
        run: irm https://install.eddacraft.ai/windows | iex

      - name: Run anvil
        env:
          ANVIL_LICENSE: ${{ secrets.ANVIL_LICENSE }}
        run: anvil gate --profile ci
```

### GitHub Actions (Cross-Platform Matrix)

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

### GitLab CI

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

## Exit Codes

| Code | Meaning                   |
| ---- | ------------------------- |
| `0`  | All gates passed          |
| `1`  | General error             |
| `2`  | One or more gate failures |
| `3`  | Authentication required   |
| `4`  | Configuration error       |

CI runners use these exit codes to pass or fail the pipeline step.

## CI-Specific Options

The `ci` profile runs all check categories (no skips). Output mode and
interactivity are controlled separately by TTY detection, `--json`, and
`--progress` flags — the profile itself selects which checks to run and which
thresholds to apply.

```bash
anvil gate --profile ci
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

:::info The `--staged` flag pairs well with lint-staged or Husky. On Git 2.54 or
newer, `anvil hooks install --config` is an alternative install mode that
manages native `[hook.<name>]` config blocks and installs the pre-commit command
`anvil gate --progress`. See [Git hook setup](/anvil/operations/git-hooks) for
both modes and coexistence rules. :::

---

**Next:** [Suppressions](/anvil/tutorials/suppressions)
