# Steps: ANTI-003

| Field      | Value                                                                          |
| ---------- | ------------------------------------------------------------------------------ |
| Source     | [../modules/antipattern-library.aps.md](../modules/antipattern-library.aps.md) |
| Task(s)    | ANTI-003 — Type escape detection                                               |
| Created by | AI                                                                             |
| Status     | Completed                                                                      |

## Prerequisites

- [x] ANTI-001 complete (pattern catalogue)

## Context

Detect type safety escapes: `any`, `@ts-ignore`, `@ts-expect-error`. These are
high-confidence anti-patterns in TypeScript codebases.

## Implementation Note

**Implemented via unified scanner approach** rather than separate detector file.
`scanner.ts` uses patterns AP-003 (`any`), AP-004 (`@ts-ignore`), and AP-005
(`@ts-expect-error`) from the pattern catalogue.

## Steps

### 1. Detection via scanner.ts with patterns

- **Checkpoint:** `core/src/antipattern/scanner.ts` handles type escape patterns
  ✅
- **Files:** `core/src/antipattern/scanner.ts`,
  `core/src/antipattern/patterns.ts`

### 2. @ts-ignore detection via AP-004

- **Checkpoint:** AP-004 pattern: `@ts-ignore` ✅
- **Validate:** `pnpm test` — pattern regex tests pass
- **Files:** `core/src/antipattern/patterns.ts`

### 3. @ts-expect-error detection via AP-005

- **Checkpoint:** AP-005 pattern: `@ts-expect-error` ✅
- **Files:** `core/src/antipattern/patterns.ts`

### 4. `any` type detection via AP-003

- **Checkpoint:** AP-003 pattern: `:\s*any\b|as\s+any\b|<any>` ✅
- **Pattern:** Matches `: any`, `as any`, `<any>` while avoiding false positives
  like "company"
- **Files:** `core/src/antipattern/patterns.ts`

### 5. Exclude legitimate uses via allowlist

- **Checkpoint:** AP-003 allowlist:
  `['*.d.ts', '**/__mocks__/**', '**/test/**/*.ts']` ✅
- **Checkpoint:** AP-005 allowlist:
  `['**/*.test.ts', '**/*.spec.ts', '**/__tests__/**']` ✅
- **Files:** `core/src/antipattern/patterns.ts`

### 6. Create Warning from detection

- **Checkpoint:** `scanFile()` returns `Warning[]` for type escapes ✅
- **Files:** `core/src/antipattern/scanner.ts`

### 7. Tests complete

- **Checkpoint:** Tests cover @ts-ignore, @ts-expect-error, :any, as any, <any>
  ✅
- **Validate:** `pnpm test` — patterns.test.ts has dedicated regex tests for
  AP-003, AP-004, AP-005
- **Files:** `core/src/antipattern/patterns.test.ts`,
  `core/src/antipattern/scanner.test.ts`

## Completion

- [x] All checkpoints validated
- [x] Task marked complete in source module

**Completed by:** AI (2025-12-24)
