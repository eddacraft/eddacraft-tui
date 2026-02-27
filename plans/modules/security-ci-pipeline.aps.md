<!-- APS: See https://github.com/EddaCraft/anvil-plan-spec for format reference -->
<!-- Executable only if tasks exist and status is Ready. -->

# Security CI Pipeline

| Scope | Owner | Priority | Status |
| ----- | ----- | -------- | ------ |
| SEC   | —     | high     | Ready  |

## Purpose

Add automated security scanning to the CI pipeline. Today the CI runs lint,
typecheck, tests, and build — but zero automated security scanning. The
adversarial code review (2026-02-06) was manual and one-off. This module adds
continuous, automated security gates that run on every PR and push.

## What We're Adding

| Tool          | What It Catches                                  | Speed  |
| ------------- | ------------------------------------------------ | ------ |
| Semgrep       | SAST — injection, XSS, path traversal, OWASP    | ~60s   |
| pnpm audit    | Known CVEs in dependencies                       | ~10s   |
| license check | GPL/AGPL/unlicensed deps in production           | ~5s    |
| secret scan   | Leaked keys/tokens in committed code             | ~15s   |
| OSSF Scorecard| Supply chain health of the repo itself           | ~30s   |

## Why These Tools

- **Semgrep** over CodeQL: faster, easier custom rules, free for open-source,
  supports TypeScript natively, no compilation step needed. CodeQL requires a
  build step and is slower for interpreted languages. Semgrep also has a large
  community rule registry covering OWASP Top 10.
- **npm audit** over Snyk: already installed, zero config, covers CVEs in the
  dependency tree. Dependabot covers upgrade PRs; npm audit covers the gate.
- **TruffleHog** over Secretlint/Gitleaks: aligns with SEC-004 and the existing
  CI workflow, supports both high-entropy and pattern-based secret detection, and
  is already battle-tested in our environment. Alternatively, GitHub's built-in
  secret scanning can be enabled at the repo level.
- **OSSF Scorecard**: free GitHub Action, measures supply chain hygiene (branch
  protection, signed commits, dependency update policy, etc.)

## Out of Scope

- DAST (no deployed application to scan from this repo)
- Container scanning (no Dockerfiles in this repo currently)
- IaC scanning (no K8s manifests yet — add when infra grows)
- CodeQL (overlaps with Semgrep; revisit if GitHub Advanced Security is enabled)
- Runtime protection (covered by the Aegis brainstorm separately)

## Interfaces

**Depends on:**

- `.github/workflows/ci.yml` — existing CI pipeline to integrate with
- `.github/dependabot.yml` — existing dependency update automation
- `.anvilrc` — existing check configuration (secret check already defined)

**Exposes:**

- `.github/workflows/security.yml` — new security scanning workflow
- `.semgrep/` — custom Semgrep rules directory
- `scripts/license-check.sh` — license compliance script
- PR status check: `Security / SAST (Semgrep)`
- PR status check: `Security / Dependency Audit`
- PR status check: `Security / Secret Scan`
- PR status check: `Security / License Check`

## Acceptance Criteria

- [ ] Semgrep runs on every PR and push to main/develop, reports inline findings
- [ ] npm audit runs and fails on critical/high vulnerabilities
- [ ] License check flags copyleft licenses in production dependencies
- [ ] Secret scanning catches patterns beyond Anvil's built-in checks
- [ ] SARIF output uploads to GitHub Security tab (if Advanced Security enabled)
- [ ] Security jobs run in parallel with existing CI (not sequential)
- [ ] False positive suppression via `.semgrep/ignore` and inline comments
- [ ] Total added CI time < 2 minutes (all security jobs run in parallel)
- [ ] Workflow uses `security-events: write` for SARIF upload, minimal other perms
- [ ] OSSF Scorecard runs weekly on main, publishes results

## Risks & Mitigations

| Risk                                   | Mitigation                                |
| -------------------------------------- | ----------------------------------------- |
| False positives block PRs              | Start with warn-only, promote to blocking |
| Semgrep rules too noisy                | Start with `p/owasp-top-ten` + `p/typescript` rulesets only |
| npm audit blocks on dev-only vulns     | Only audit production deps (`--omit=dev`) |
| License check flags legitimate deps    | Allowlist for reviewed exceptions          |
| CI time increase                       | All security jobs parallel, use caching   |
| Secrets in test fixtures trigger scans | `.secretlintignore` for test directories  |

## Tasks

### SEC-001: Create security scanning workflow

- **Intent:** Add `.github/workflows/security.yml` with parallel security jobs
- **Expected Outcome:** Workflow runs on PR and push to main/develop. Contains
  jobs for Semgrep, dependency audit, secret scanning, and license check, all
  running in parallel. Respects the existing `detect-changes` pattern — only
  triggers on code changes (not docs-only).
- **Scope:** `.github/workflows/security.yml`
- **Non-scope:** Modifying existing `ci.yml`
- **Validation:** Workflow passes `actionlint` and YAML syntax check
- **Confidence:** high

### SEC-002: Semgrep SAST integration

- **Intent:** Run Semgrep with TypeScript/JavaScript rulesets on every PR
- **Expected Outcome:** Semgrep job uses `returntocorp/semgrep-action` with
  `p/owasp-top-ten`, `p/typescript`, and `p/nodejs` rulesets. Outputs SARIF for
  GitHub Security tab integration. Initially non-blocking (continue-on-error)
  until baseline established.
- **Scope:** Semgrep job in `security.yml`, `.semgrep.yml` config
- **Non-scope:** Custom Semgrep rules (Phase 2)
- **Dependencies:** SEC-001
- **Validation:** Semgrep runs against codebase without errors
- **Confidence:** high

### SEC-003: Dependency vulnerability audit

- **Intent:** Fail CI on critical/high CVEs in production dependencies
- **Expected Outcome:** Job runs `pnpm audit --prod --audit-level=high`. Fails
  on high+ severity findings. Outputs structured JSON for reporting. Allowfile
  for known/accepted vulnerabilities (`.pnpm-audit-allowlist.json`).
- **Scope:** Audit job in `security.yml`
- **Non-scope:** Dev dependency audit (too noisy, Dependabot handles upgrades)
- **Dependencies:** SEC-001
- **Validation:** `pnpm audit` runs cleanly or exits with expected code
- **Confidence:** high

### SEC-004: Secret scanning in CI

- **Intent:** Catch leaked secrets that Anvil's built-in patterns might miss
- **Expected Outcome:** Job uses `trufflesecurity/trufflehog` GitHub Action to
  scan committed code for secrets. Scans only the diff (not full history) on PRs
  for speed. Supports `.trufflehog-ignore` for test fixture exceptions.
  Alternatively, enable GitHub's native secret scanning at the repo settings
  level and skip the CI job.
- **Scope:** Secret scan job in `security.yml`
- **Non-scope:** Git history scanning (separate one-time audit)
- **Dependencies:** SEC-001
- **Validation:** No false positives on current codebase
- **Confidence:** high

### SEC-005: License compliance check

- **Intent:** Prevent copyleft/unlicensed dependencies from entering production
- **Expected Outcome:** Job runs `license-checker` or `pnpm licenses list` to
  enumerate production dependency licenses. Fails if GPL, AGPL, or unlicensed
  packages detected. Allowlist for reviewed exceptions. Outputs report as CI
  artifact.
- **Scope:** License job in `security.yml`, `scripts/license-check.sh`
- **Non-scope:** Dev dependency licenses
- **Dependencies:** SEC-001
- **Validation:** Current production deps pass the check
- **Confidence:** medium

### SEC-006: Custom Semgrep rules for Anvil patterns

- **Intent:** Add project-specific Semgrep rules for patterns found in the
  adversarial review
- **Expected Outcome:** `.semgrep/` directory with custom rules covering:
  - `execSync` with string args (prefer `execFileSync`)
  - `path.join` without traversal validation
  - `z.unknown()` in contract schemas
  - Missing `shell: false` on `spawn`/`exec`
  - Unvalidated path construction from user input
- **Scope:** `.semgrep/anvil-rules.yml`
- **Non-scope:** Upstream Semgrep rules
- **Dependencies:** SEC-002
- **Validation:** Rules match known patterns in codebase
- **Confidence:** medium

### SEC-007: OSSF Scorecard integration

- **Intent:** Measure and track supply chain security posture of the repo
- **Expected Outcome:** Weekly Scorecard run on main branch via
  `ossf/scorecard-action`. Results published to GitHub Security tab. Tracks
  branch protection, dependency update, signed commits, CI/CD hardening,
  contributor trust, etc.
- **Scope:** Scorecard job in `security.yml` (or separate workflow)
- **Non-scope:** Acting on Scorecard recommendations (tracked separately)
- **Dependencies:** SEC-001
- **Validation:** Scorecard completes and publishes results
- **Confidence:** high

### SEC-008: Security scan result reporting

- **Intent:** Aggregate security findings into a single PR summary
- **Expected Outcome:** Final job that waits for all security scans and posts a
  summary comment on the PR. Includes: Semgrep finding count by severity,
  dependency audit status, secret scan pass/fail, license check pass/fail.
  Uses `actions/github-script` to post the comment.
- **Scope:** Summary job in `security.yml`
- **Non-scope:** Slack/email notifications
- **Dependencies:** SEC-002, SEC-003, SEC-004, SEC-005
- **Validation:** Summary comment appears on test PR
- **Confidence:** medium
