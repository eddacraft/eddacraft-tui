<!-- APS: See https://github.com/EddaCraft/anvil-plan-spec for format reference -->
<!-- Executable only if tasks exist and status is Ready. -->

# Drift Reporting

| Scope | Owner | Priority | Status |
| ----- | ----- | -------- | ------ |
| DRIFT | —     | medium   | Draft  |

## Purpose

Provide visibility into how the codebase architecture evolves over time. Show
trends in boundary violations, suppressions, and anti-pattern introductions so
tech leads can spot systemic issues.

## In Scope

- Snapshot current architecture state
- Compare snapshots over time
- Report new vs existing violations
- Suppression trend reporting

## Out of Scope

- Real-time dashboards (v2)
- Team-level attribution
- Integration with external analytics

## Interfaces

**Depends on:**

- `architecture-safety` — baseline and edge data
- `antipattern-library` — anti-pattern counts for trending
- `suppressions` — suppression tracking for trend reporting

**Exposes:**

- `anvil drift snapshot` — capture current state
- `anvil drift compare` — compare two snapshots
- `anvil drift report` — generate drift report

## Acceptance Criteria

- [ ] `anvil drift snapshot` creates timestamped snapshot
- [ ] `anvil drift compare` shows added/removed edges
- [ ] Report distinguishes new violations from existing

## Tasks

_Tasks to be defined when module status is Ready._
