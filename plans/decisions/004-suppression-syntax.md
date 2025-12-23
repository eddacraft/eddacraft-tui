# ADR-004: Suppression Syntax

## Status

Accepted

## Context

Developers need to intentionally bypass warnings for legitimate reasons (legacy
integration, known trade-offs, temporary workarounds). Suppressions must be:

- Targeted (specific warning, not all warnings)
- Explained (require human reasoning)
- Traceable (who, when, why)

## Decision

Suppression syntax:

```typescript
// @anvil-ignore ARCH-001: Legacy auth integration, see TECH-123
// @anvil-ignore-until 2025-06-01 AP-002: Temp workaround for migration
```

Format: `@anvil-ignore[-until <DATE>] <WARNING-ID>: <reason>`

- `WARNING-ID` is required — no blanket suppressions
- `reason` is required — parser rejects empty reasons
- `until` variant enables time-boxed suppressions that auto-expire

## Consequences

- Every suppression is intentional and documented
- Drift reports can show suppression trends
- Expired suppressions resurface warnings automatically
- Slightly verbose, but accountability is the point
