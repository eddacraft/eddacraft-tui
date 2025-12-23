<!-- APS: See https://github.com/EddaCraft/anvil-plan-spec for format reference -->
<!-- Executable only if tasks exist and status is Ready. -->

# CI Integration

| Scope | Owner | Priority | Status |
| ----- | ----- | -------- | ------ |
| CI    | —     | low      | Draft  |

## Purpose

Mirror save-time warnings in CI/CD pipelines as a fail-safe. Catches issues that
slip through local development (web commits, skipped hooks).

## In Scope

- GitHub Action for Anvil checks
- PR status checks (informational by default — neutral state, not failing)
- PR comment summaries with warning counts
- Changed-files-only analysis

## Out of Scope

- GitLab CI (separate module)
- Merge blocking (configurable, not default)
- Override commands (v2)

## Interfaces

**Depends on:**

- `save-time-trust` — analysis runner

**Exposes:**

- `.github/actions/anvil-check/action.yml`
- PR comment bot

## Acceptance Criteria

- [ ] GitHub Action runs on PR
- [ ] Status check posts neutral (informational) by default, not failing
- [ ] Optional `fail-on-warnings: true` input to enable blocking mode
- [ ] PR comment shows warning summary with counts
- [ ] Only changed files analysed

## Tasks

_Tasks to be defined when module status is Ready._
