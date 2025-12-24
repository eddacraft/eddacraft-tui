# Steps: ANTI-002

| Field      | Value                                                                          |
| ---------- | ------------------------------------------------------------------------------ |
| Source     | [../modules/antipattern-library.aps.md](../modules/antipattern-library.aps.md) |
| Task(s)    | ANTI-002 — ESLint disable detection                                            |
| Created by | AI                                                                             |
| Status     | Completed                                                                      |

## Prerequisites

- [x] ANTI-001 complete (pattern catalogue)

## Context

Need to detect eslint-disable comments in TypeScript/JavaScript files. Focus on
NEW disables vs baseline.

## Implementation Note

**Implemented via unified scanner approach** rather than separate detector file.
`scanner.ts` uses patterns AP-001 (broad eslint-disable) and AP-002
(rule-specific disable) from the pattern catalogue to detect ESLint disables.

## Steps

### 1. Create detection via scanner.ts

- **Checkpoint:** `core/src/antipattern/scanner.ts` exists and handles
  eslint-disable patterns ✅
- **Files:** `core/src/antipattern/scanner.ts`

### 2. Implement comment extraction via regex patterns

- **Checkpoint:** AP-001 pattern:
  `/\*\s*eslint-disable\s*\*/|//\s*eslint-disable(?!-next-line|-line)\s*$` ✅
- **Checkpoint:** AP-002 pattern:
  `eslint-disable(?:-next-line|-line)?\s+[\w@/-]+` ✅
- **Pattern:** Regex for `eslint-disable`, `eslint-disable-next-line`,
  `eslint-disable-line`
- **Validate:** `pnpm test` — 33 scanner tests pass
- **Files:** `core/src/antipattern/patterns.ts`,
  `core/src/antipattern/scanner.ts`

### 3. Scope detection via pattern differentiation

- **Checkpoint:** AP-001 detects file-level/block disables, AP-002 detects
  line-level with rules ✅
- **Files:** `core/src/antipattern/patterns.ts`

### 4. Rule extraction via regex groups

- **Checkpoint:** AP-002 pattern extracts rule names from disable comments ✅
- **Files:** `core/src/antipattern/patterns.ts`

### 5. Create Warning from detection

- **Checkpoint:** `scanFile()` returns `Warning[]` using Warning schema ✅
- **Pattern:** Uses Warning schema from types.ts
- **Files:** `core/src/antipattern/scanner.ts`

### 6. Tests complete

- **Checkpoint:** Tests cover file-level, next-line, inline, rule-specific cases
  ✅
- **Validate:** `pnpm test` — patterns.test.ts (40 tests), scanner.test.ts (33
  tests)
- **Files:** `core/src/antipattern/patterns.test.ts`,
  `core/src/antipattern/scanner.test.ts`

## Completion

- [x] All checkpoints validated
- [x] Task marked complete in source module

**Completed by:** AI (2025-12-24)
