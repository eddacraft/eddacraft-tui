# CI Pipeline Improvements Implementation Plan

**Goal:** Fix 9 CI issues: extract shared setup, cache CodeQL Rust builds, fix lint-staged order, simplify matrix, enable Nx cache, improve observability, and document fallback behaviour.
**Architecture:** All changes are config-level (YAML, JSON). A new composite action absorbs repeated setup steps from `ci.yml`. Remaining fixes are single-file edits.
**Tech Stack:** GitHub Actions, Nx, lint-staged, Cargo

---

## File Map

| File | Responsibility |
|------|---------------|
| Create: `.github/actions/setup-workspace/action.yml` | Composite action encapsulating checkout + pnpm + Node + deps + Azure login + Nx SHAs |
| Modify: `.github/workflows/ci.yml` | Replace 6 jobs' setup blocks with composite action; remove single-element matrix; add Azure login comments |
| Modify: `.github/workflows/codeql.yml` | Add Cargo target cache to analyze-rust job |
| Modify: `.github/workflows/security.yml` | Add TODO comment about broken Semgrep |
| Modify: `.github/workflows/bench.yml` | Increase retention-days from 30 to 90 |
| Modify: `.github/actions/detect-changes/action.yml` | Add `::warning::` on fallback |
| Modify: `nx.json` | Add `"cache": true` to typecheck target |
| Modify: `.lintstagedrc.json` | Reorder: format before lint |

---

### Task 1: Create setup-workspace composite action

**Files:**
- Create: `.github/actions/setup-workspace/action.yml`

- [ ] Create composite action with inputs: `node-version` (default 22), `fetch-depth` (default 1), `azure-login` (default false), `nx-shas` (default false)
- [ ] Verify: `yamllint .github/actions/setup-workspace/action.yml` or manual YAML review
- [ ] Commit: `git commit -m "chore(ci): create setup-workspace composite action"`

**Content:**
```yaml
name: Setup Workspace
description: Checkout, install pnpm + Node.js + deps, optionally Azure login + Nx SHAs

inputs:
  node-version:
    description: 'Node.js version'
    required: false
    default: '22'
  fetch-depth:
    description: 'Git fetch depth (0 for full history)'
    required: false
    default: '1'
  azure-login:
    description: 'Enable Azure login for Nx remote cache'
    required: false
    default: 'false'
  nx-shas:
    description: 'Run nrwl/nx-set-shas for affected commands'
    required: false
    default: 'false'

runs:
  using: composite
  steps:
    - name: Checkout repository
      uses: actions/checkout@de0fac2e4500dabe0009e67214ff5f5447ce83dd # v6.0.2
      with:
        fetch-depth: ${{ inputs.fetch-depth }}

    - name: Install pnpm
      uses: pnpm/action-setup@fc06bc1257f339d1d5d8b3a19a8cae5388b55320 # v5.0.0

    - name: Setup Node.js ${{ inputs.node-version }}
      uses: actions/setup-node@53b83947a5a98c8d113130e565377fae1a50d02f # v6.3.0
      with:
        node-version: ${{ inputs.node-version }}
        cache: 'pnpm'

    - name: Install dependencies
      shell: bash
      run: pnpm install --frozen-lockfile

    # Azure login enables Nx remote cache (Azure Blob Storage).
    # continue-on-error: fork PRs don't have secrets, so login fails
    # silently and Nx falls back to local cache. This is intentional.
    - name: Azure Login
      if: inputs.azure-login == 'true'
      uses: azure/login@532459ea530d8321f2fb9bb10d1e0bcf23869a43 # v3.0.0
      continue-on-error: true
      with:
        creds: >-
          {"clientId":"${{ env.ARM_CLIENT_ID }}","clientSecret":"${{ env.ARM_CLIENT_SECRET }}","subscriptionId":"${{ env.ARM_SUBSCRIPTION_ID }}","tenantId":"${{ env.ARM_TENANT_ID }}"}

    - name: Set Nx SHAs
      if: inputs.nx-shas == 'true'
      uses: nrwl/nx-set-shas@afb73a62d26e41464e9254689e1fd6122ee683c1 # v5.0.1
```

**Note:** The Azure login step needs the secrets passed as env vars from the calling workflow since composite actions can't access `secrets.*` directly. Each job will set:
```yaml
env:
  ARM_CLIENT_ID: ${{ secrets.ARM_CLIENT_ID }}
  ARM_CLIENT_SECRET: ${{ secrets.ARM_CLIENT_SECRET }}
  ARM_SUBSCRIPTION_ID: ${{ secrets.ARM_SUBSCRIPTION_ID }}
  ARM_TENANT_ID: ${{ secrets.ARM_TENANT_ID }}
```

---

### Task 2: Replace ci.yml setup blocks with composite action + remove single-element matrix + document Azure

**Files:**
- Modify: `.github/workflows/ci.yml`

- [ ] Replace docs-lint job setup (lines 50-63) with composite action call (no azure, no nx-shas)
- [ ] Replace lint job setup (lines 81-108) with composite action call (fetch-depth 0, azure, nx-shas) + add env block for ARM secrets
- [ ] Replace typecheck job setup (lines 132-159) with composite action call (fetch-depth 0, azure, nx-shas) + add env block
- [ ] Replace test job setup (lines 211-238): remove matrix, hardcode node-version 22.x, use composite action (fetch-depth 0, azure, nx-shas) + add env block. Update job name from `Node ${{ matrix.node-version }}` to `Node 22.x`. Update coverage step condition from `matrix.node-version == '22.x'` to `always()`. Update artifact name from `coverage-report-${{ matrix.node-version }}` to `coverage-report-22.x`.
- [ ] Replace test-release-gate job setup (lines 281-303) with composite action call (azure only) + add env block
- [ ] Replace build job setup (lines 319-341): remove matrix, hardcode 22.x, use composite action (azure only) + add env block. Update job name.
- [ ] Update test-skip job name from `Unit Tests (Node 22.x, ubuntu-latest)` — keep matching the real test job name
- [ ] Verify: review the full file for consistency
- [ ] Commit: `git commit -m "chore(ci): use setup-workspace action and simplify matrix"`

**Example replacement for lint job:**
```yaml
  lint:
    name: Lint & Format
    runs-on: ubuntu-latest
    needs: detect-changes
    if: needs.detect-changes.outputs.code-changed == 'true'
    env:
      ARM_CLIENT_ID: ${{ secrets.ARM_CLIENT_ID }}
      ARM_CLIENT_SECRET: ${{ secrets.ARM_CLIENT_SECRET }}
      ARM_SUBSCRIPTION_ID: ${{ secrets.ARM_SUBSCRIPTION_ID }}
      ARM_TENANT_ID: ${{ secrets.ARM_TENANT_ID }}

    steps:
      - name: Setup workspace
        uses: ./.github/actions/setup-workspace
        with:
          fetch-depth: 0
          azure-login: true
          nx-shas: true

      - name: Build ESLint plugin (required by lint rules)
        run: pnpm exec nx run eslint-plugin-anvil:build

      # ... rest of job-specific steps unchanged
```

---

### Task 3: Cache Rust build in CodeQL

**Files:**
- Modify: `.github/workflows/codeql.yml`

- [ ] Add `actions/cache` step before `cargo build --workspace` in analyze-rust job (after line 86, before line 94)
- [ ] Verify: review the diff
- [ ] Commit: `git commit -m "chore(ci): cache Rust target dir in CodeQL workflow"`

**Add between setup-rust-toolchain and codeql/init:**
```yaml
      - name: Cache Cargo target
        uses: actions/cache@1bd1e32a3bdc45362d1e726936510720a7c30a57 # v4
        with:
          path: target
          key: codeql-rust-${{ hashFiles('Cargo.lock') }}
          restore-keys: codeql-rust-
```

---

### Task 4: Add TODO for broken Semgrep

**Files:**
- Modify: `.github/workflows/security.yml`

- [ ] Add comment above the semgrep job explaining the issues
- [ ] Create GitHub issue tracking the Semgrep fix
- [ ] Commit: `git commit -m "chore(ci): document broken Semgrep config, create tracking issue"`

**Replace lines 44-50 comment/header with:**
```yaml
  # ── SAST: Semgrep ─────────────────────────────────────────────
  # TODO(ci): Semgrep is currently non-functional:
  #   1. .semgrep.yml has invalid format (mixes registry packs with rules: key)
  #   2. returntocorp/semgrep-action@v1 is deprecated (always exits 0)
  #   3. SARIF upload fails due to permissions
  # Tracked in: <issue URL inserted after creation>
  # Fix: upgrade to semgrep/semgrep-action, fix .semgrep.yml, add
  #      actions: read permission for SARIF upload.
```

**GitHub issue body:**
```
## Broken Semgrep CI

The Semgrep SAST job in `security.yml` is non-functional:

1. **Invalid config**: `.semgrep.yml` uses `rules:` with registry pack names (`p/owasp-top-ten`) — this is not valid Semgrep config format
2. **Deprecated action**: `returntocorp/semgrep-action@v1` always exits 0 regardless of findings
3. **SARIF upload fails**: "Resource not accessible by integration" — missing `actions: read` permission

### Fix needed
- Upgrade to `semgrep/semgrep-action` (modern action, respects exit codes)
- Fix `.semgrep.yml`: remove `rules:` key, use only `paths:` for excludes; pass registry packs via `SEMGREP_RULES` env (already done)
- Add `actions: read` to the job permissions for SARIF upload
- Once fixed, consider removing `continue-on-error: true` to make blocking
```

---

### Task 5: Fix lint-staged order

**Files:**
- Modify: `.lintstagedrc.json`

- [ ] Reorder: format first (`oxfmt --write`), then lint (`oxlint --fix`, `eslint --fix`)
- [ ] Verify: `cat .lintstagedrc.json | python3 -m json.tool`
- [ ] Commit: `git commit -m "chore(dx): run formatter before linter in lint-staged"`

**New content:**
```json
{
  "*.{js,jsx,ts,tsx}": ["oxfmt --write", "oxlint --fix", "eslint --fix"],
  "*.json": ["oxfmt --write", "eslint --fix"],
  "!(pnpm-lock|temper).{yml,yaml}": ["yamllint", "oxfmt --write"],
  "temper.{yml,yaml}": ["oxfmt --write"],
  "*.md": ["markdownlint --fix"]
}
```

---

### Task 6: Enable Nx typecheck cache

**Files:**
- Modify: `nx.json`

- [ ] Add `"cache": true` to the `typecheck` target default
- [ ] Verify: `cat nx.json | python3 -m json.tool`
- [ ] Commit: `git commit -m "chore(ci): enable Nx cache for typecheck target"`

**Change line 48 from:**
```json
"typecheck": { "dependsOn": ["^build"] },
```
**To:**
```json
"typecheck": { "dependsOn": ["^build"], "cache": true },
```

---

### Task 7: Log warning on change detection fallback

**Files:**
- Modify: `.github/actions/detect-changes/action.yml`

- [ ] Add `echo "::warning::..."` when git diff fails and fallback is used
- [ ] Verify: review the diff
- [ ] Commit: `git commit -m "chore(ci): log warning when change detection falls back to API"`

**Change lines 45-46 from:**
```bash
        if [ $? -ne 0 ] || [ -z "$CHANGED_FILES" ]; then
          echo "git diff failed — falling back to GitHub API"
```
**To:**
```bash
        if [ $? -ne 0 ] || [ -z "$CHANGED_FILES" ]; then
          echo "::warning::git diff failed (shallow clone?) — falling back to GitHub API. Results may be incomplete."
```

---

### Task 8: Increase benchmark retention

**Files:**
- Modify: `.github/workflows/bench.yml`

- [ ] Change `retention-days: 30` to `retention-days: 90` (line 66)
- [ ] Commit: `git commit -m "chore(ci): increase benchmark artifact retention to 90 days"`

---

### Task 9: Final verification

- [ ] Review all changed files for consistency
- [ ] Ensure no broken YAML (indentation, quoting)
- [ ] Commit the spec + plan: `git commit -m "docs: add CI improvements spec and plan"`
