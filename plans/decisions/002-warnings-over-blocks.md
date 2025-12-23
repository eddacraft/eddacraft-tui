# ADR-002: Warnings Over Blocks

## Status

Accepted

## Context

Traditional linting tools fail builds on violations. This creates pressure to
disable checks rather than fix issues, especially for legacy code or time
pressure.

## Decision

Anvil warnings do **not** block by default. Exit code 0 for warnings, non-zero
only for errors (schema failures, crashes).

CI integration offers opt-in `fail-on-warnings: true` for teams that want
enforcement.

## Consequences

- Developers are informed, not blocked — reduces resistance
- Adoption is easier (no "fix everything first" barrier)
- Teams that want enforcement can enable it explicitly
- Risk: warnings may be ignored if not visible enough
- Mitigation: IDE integration makes warnings hard to miss
