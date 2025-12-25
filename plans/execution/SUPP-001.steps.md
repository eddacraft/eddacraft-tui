# Steps: SUPP-001

| Field      | Value                                                            |
| ---------- | ---------------------------------------------------------------- |
| Source     | [../modules/suppressions.aps.md](../modules/suppressions.aps.md) |
| Task(s)    | SUPP-001 — Suppression parser                                    |
| Created by | AI                                                               |
| Status     | Completed                                                        |

## Prerequisites

- [x] ANTI-004 complete (anti-pattern check integration)
- [x] ARCH-003 complete (architecture check integration)
- [x] ADR-004 accepted (suppression syntax defined)

## Context

Parse inline suppression comments from source files. Per ADR-004:

- `// @anvil-ignore <WARNING-ID>: <reason>`
- `// @anvil-ignore-until <DATE> <WARNING-ID>: <reason>`

Existing types in `core/src/antipattern/types.ts`:

- `SuppressionSchema` - basic suppression metadata
- `SuppressionRecordSchema` - for provenance tracking

## Steps

### 1. Create suppression parser module

- **Checkpoint:** `core/src/suppression/parser.ts` exists
- **Files:** `core/src/suppression/parser.ts`

### 2. Implement regex patterns for comment parsing

Patterns to detect:

- `@anvil-ignore <ID>: <reason>` (permanent)
- `@anvil-ignore-until <DATE> <ID>: <reason>` (time-boxed)

Support comment styles:

- `//` single-line comments
- `/* */` block comments
- `/** */` JSDoc comments

### 3. Implement parseSuppressions function

```typescript
interface ParsedSuppression {
  warningId: string; // e.g., "AP-001", "ARCH-001"
  reason: string; // Human-provided reason
  expiresAt?: Date; // For @anvil-ignore-until
  line: number; // Line number of the comment
  scope: 'line' | 'statement' | 'file';
}

function parseSuppressions(
  content: string,
  filePath: string
): ParsedSuppression[];
```

### 4. Implement scope detection

| Pattern Location       | Scope     | Effect                         |
| ---------------------- | --------- | ------------------------------ |
| Above statement        | statement | Suppresses next statement only |
| End of line            | line      | Suppresses that line only      |
| Top of file (line 1-5) | file      | Suppresses entire file         |

### 5. Add validation

- Reject empty reasons
- Validate warning ID format (AP-XXX, ARCH-XXX, BOUND-XXX)
- Validate date format for `@anvil-ignore-until` (ISO 8601)

### 6. Add tests

- **Checkpoint:** `core/src/suppression/parser.test.ts` exists with
  comprehensive tests
- **Files:** `core/src/suppression/parser.test.ts`

## Acceptance Criteria

- [x] Parser extracts warning ID, reason, expiry from comments
- [x] Parser rejects empty reasons
- [x] Parser handles all comment styles (// /_ /\*\* _/)
- [x] Scope detection works correctly
- [x] All tests pass
