# Steps: ANTI-003

| Field      | Value                                                                          |
| ---------- | ------------------------------------------------------------------------------ |
| Source     | [../modules/antipattern-library.aps.md](../modules/antipattern-library.aps.md) |
| Task(s)    | ANTI-003 — Type escape detection                                               |
| Created by | AI                                                                             |
| Status     | Draft                                                                          |

## Prerequisites

- [ ] ANTI-001 complete (pattern catalogue)

## Context

Detect type safety escapes: `any`, `@ts-ignore`, `@ts-expect-error`. These are
high-confidence anti-patterns in TypeScript codebases.

## Steps

### 1. Create type-escape-detector.ts

- **Checkpoint:** `core/src/antipattern/type-escape-detector.ts` exists
- **Files:** `core/src/antipattern/type-escape-detector.ts`

### 2. Implement @ts-ignore detection

- **Checkpoint:** Finds `// @ts-ignore` and `/* @ts-ignore */` comments
- **Validate:** `nx test core --testNamePattern="type-escape"`
- **Files:** `core/src/antipattern/type-escape-detector.ts`

### 3. Implement @ts-expect-error detection

- **Checkpoint:** Finds `@ts-expect-error` comments
- **Files:** `core/src/antipattern/type-escape-detector.ts`

### 4. Implement `any` type detection

- **Checkpoint:** Finds `as any`, `<any>`, explicit `any` return types
- **Pattern:** Regex — skip `: any` in catch clauses (legitimate pre-TS 4.4)
- **Files:** `core/src/antipattern/type-escape-detector.ts`

### 5. Exclude legitimate uses

- **Checkpoint:** Skips: type definitions (.d.ts), test files (configurable)
- **Files:** `core/src/antipattern/type-escape-detector.ts`

### 6. Create Warning from detection

- **Checkpoint:** `detectTypeEscapes(content, filePath): Warning[]` returns
  warnings
- **Files:** `core/src/antipattern/type-escape-detector.ts`

### 7. Add tests

- **Checkpoint:** Tests cover @ts-ignore, @ts-expect-error, :any, as any
- **Validate:** `nx test core`
- **Files:** `core/src/antipattern/type-escape-detector.test.ts`

## Completion

- [ ] All checkpoints validated
- [ ] Task marked complete in source module

**Completed by:** —
