# Steps: ANTI-002

| Field      | Value                                                                          |
| ---------- | ------------------------------------------------------------------------------ |
| Source     | [../modules/antipattern-library.aps.md](../modules/antipattern-library.aps.md) |
| Task(s)    | ANTI-002 — ESLint disable detection                                            |
| Created by | AI                                                                             |
| Status     | Draft                                                                          |

## Prerequisites

- [ ] ANTI-001 complete (pattern catalogue)

## Context

Need to detect eslint-disable comments in TypeScript/JavaScript files. Focus on
NEW disables vs baseline.

## Steps

### 1. Create eslint-disable-detector.ts

- **Checkpoint:** `core/src/antipattern/eslint-disable-detector.ts` exists
- **Files:** `core/src/antipattern/eslint-disable-detector.ts`

### 2. Implement comment extraction

- **Checkpoint:** `extractEslintDisables(content): EslintDisable[]` works
- **Pattern:** Regex for `eslint-disable`, `eslint-disable-next-line`,
  `eslint-disable-line`
- **Validate:** `nx test core --testNamePattern="eslint-disable"`
- **Files:** `core/src/antipattern/eslint-disable-detector.ts`

### 3. Add scope detection

- **Checkpoint:** Detects scope: file-level, next-line, inline
- **Files:** `core/src/antipattern/eslint-disable-detector.ts`

### 4. Add rule extraction

- **Checkpoint:** Extracts which rules are disabled (or "all" for broad disable)
- **Files:** `core/src/antipattern/eslint-disable-detector.ts`

### 5. Create Warning from detection

- **Checkpoint:** `toWarning(disable, pattern): Warning` function exists
- **Pattern:** Uses Warning schema from types.ts
- **Files:** `core/src/antipattern/eslint-disable-detector.ts`

### 6. Add tests

- **Checkpoint:** Tests cover file-level, next-line, inline, rule-specific cases
- **Validate:** `nx test core`
- **Files:** `core/src/antipattern/eslint-disable-detector.test.ts`

## Completion

- [ ] All checkpoints validated
- [ ] Task marked complete in source module

**Completed by:** —
