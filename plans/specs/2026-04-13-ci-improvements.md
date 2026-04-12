# CI Pipeline Improvements

**Date:** 2026-04-13
**Branch:** chore/ci-improvements
**Scope:** GitHub Actions workflows, Nx config, lint-staged config

## Problem

The CI pipeline has accumulated several inefficiencies and issues:

- Setup steps (checkout, pnpm, Node, Azure login, deps) are copy-pasted
  across 6 jobs in `ci.yml` — hard to maintain, easy to drift
- Rust CodeQL build runs from scratch every time (no caching)
- Semgrep is broken (invalid config, deprecated action) but CI reports
  success — filed as separate issue
- lint-staged runs formatters after linters, causing unnecessary re-commits
- Single-element matrix adds job overhead for no benefit
- Nx typecheck target misses cache, re-running on every PR
- Change detection fallback is silent, hiding shallow-clone issues
- Benchmark artifacts expire after 30 days (too short for trend analysis)
- Azure login fallback behaviour is undocumented

## Fixes

### Fix 1: Extract CI setup composite action

Create `.github/actions/setup-workspace/action.yml` that encapsulates:
- `actions/checkout` (with optional `fetch-depth: 0`)
- `pnpm/action-setup`
- `actions/setup-node` (with pnpm cache)
- `pnpm install --frozen-lockfile`
- Azure login (optional, with `continue-on-error`)
- `nrwl/nx-set-shas` (optional)

**Inputs:**
- `node-version` (default: `22.x`)
- `fetch-depth` (default: `1`, set to `0` for affected commands)
- `azure-login` (default: `false`)
- `nx-shas` (default: `false`)

Replace all 6 jobs' setup blocks with a single step:
```yaml
- uses: ./.github/actions/setup-workspace
  with:
    fetch-depth: 0
    azure-login: true
    nx-shas: true
```

**Files:** `.github/actions/setup-workspace/action.yml`, `.github/workflows/ci.yml`

### Fix 2: Cache Rust build in CodeQL

Add `actions/cache` for `target/` before the `cargo build --workspace`
step in the `analyze-rust` job. Key on `Cargo.lock` hash.

```yaml
- uses: actions/cache@v4
  with:
    path: target
    key: codeql-rust-${{ hashFiles('Cargo.lock') }}
    restore-keys: codeql-rust-
```

**File:** `.github/workflows/codeql.yml`

### Fix 3: Document broken Semgrep + create tracking issue

Add a `# TODO` comment in `security.yml` noting Semgrep is non-functional
(invalid `.semgrep.yml`, deprecated action). Create a GitHub issue to
track the proper fix (upgrade action, fix config, fix SARIF permissions).

**File:** `.github/workflows/security.yml`

### Fix 4: Fix lint-staged order

Change `.lintstagedrc` to run formatters before linters:

**Before:** `["oxlint --fix", "eslint --fix", "oxfmt --write"]`
**After:** `["oxfmt --write", "oxlint --fix", "eslint --fix"]`

Same for JSON files. This ensures code is formatted before linting, so
linters see the final form and don't flag formatting issues.

**File:** `.lintstagedrc`

### Fix 5: Remove single-element matrix

In `ci.yml`, the test job uses `matrix: { node-version: [22.x] }` — a
single-element matrix. Replace with a direct `22.x` reference. The
multi-version testing is handled by the nightly workflow.

**File:** `.github/workflows/ci.yml`

### Fix 6: Enable Nx typecheck cache

Add `"cache": true` to the `typecheck` target in `nx.json`. TypeScript
checks are deterministic and safe to cache.

**File:** `nx.json`

### Fix 7: Log warning on change detection fallback

When `git diff` fails and the action falls back to the GitHub API, log a
`::warning::` annotation so it's visible in the Actions UI. Currently
the fallback is silent.

**File:** `.github/actions/detect-changes/action.yml`

### Fix 8: Increase benchmark artifact retention

Change `retention-days` from 30 to 90 for benchmark results. 30 days is
too short for meaningful trend analysis across release cycles.

**File:** `.github/workflows/bench.yml`

### Fix 9: Document Azure login fallback

Add a comment above each Azure login step explaining that
`continue-on-error: true` means Nx cache silently falls back to local
when credentials aren't available (e.g. fork PRs). This is intentional
but undocumented.

**File:** `.github/workflows/ci.yml`

## Out of Scope

- Semgrep fix (separate PR — config rewrite + action upgrade)
- Workflow restructuring beyond setup extraction
- New CI jobs or checks
- Runner selection changes

## Success Criteria

1. `ci.yml` setup blocks replaced with composite action — same behaviour
2. All existing CI checks still pass
3. CodeQL Rust job uses cached `target/` on subsequent runs
4. lint-staged formats before linting
5. Nx typecheck is cached
6. Change detection fallback is visible in Actions UI
