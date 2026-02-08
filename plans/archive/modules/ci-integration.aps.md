<!-- APS: See https://github.com/EddaCraft/anvil-plan-spec for format reference -->
<!-- Executable only if tasks exist and status is Ready. -->

# CI Integration

| Scope | Owner | Priority | Status   |
| ----- | ----- | -------- | -------- |
| CI    | —     | high     | Complete |

## Purpose

Mirror save-time warnings in CI/CD pipelines as the enforcement point. This is
where governance becomes mandatory — developers can bypass local hooks, but CI
catches everything before merge.

## In Scope

- GitHub Action for Anvil checks (composite action)
- PR status checks (informational by default — neutral state, not failing)
- PR comment summaries with warning counts and details
- Changed-files-only analysis for performance
- Inline annotations in PR files view
- Configurable blocking mode

## Out of Scope

- GitLab CI (separate module, post-MVP)
- Override commands via PR comments (v2)
- Caching between workflow runs (v2)
- Branch protection rule automation (v2)

## Interfaces

**Depends on:**

- `save-time-trust` — analysis runner (`anvil check`)
- CLI JSON output mode (`--output json`)

**Exposes:**

- `.github/actions/anvil-check/action.yml` — composite action
- `.github/workflows/anvil.yml.example` — example workflow
- Documentation in `docs/USER_GUIDE.md`

## Tasks

| ID     | Task                                    | Status   | Depends on |
| ------ | --------------------------------------- | -------- | ---------- |
| CI-001 | GitHub Action scaffold                  | Complete | —          |
| CI-002 | Changed files detection                 | Complete | CI-001     |
| CI-003 | PR comment and status check integration | Complete | CI-002     |
| CI-004 | Configuration and documentation         | Complete | CI-003     |

## Acceptance Criteria

- [x] GitHub Action runs on PR open/sync events
- [x] Status check posts neutral (informational) by default
- [x] Optional `fail-on-warnings: true` input enables blocking mode
- [x] PR comment shows warning summary with counts by category
- [x] Only changed files analysed (performance)
- [x] Inline annotations appear in PR files view
- [x] Works with matrix builds (monorepo support)
- [x] Clear documentation with copy-paste workflow example

## Technical Notes

### Composite Action vs JavaScript Action

Using composite action (shell scripts) for:

- Transparency — users can see exactly what runs
- Simplicity — no build step, no node_modules in action
- Flexibility — easy to modify without rebuilding

### Permissions Required

```yaml
permissions:
  contents: read
  pull-requests: write
  statuses: write
  checks: write
```

### Exit Codes

- `0` — No issues found
- `1` — Warnings found (non-blocking by default)
- `2` — Errors found (always fails)
