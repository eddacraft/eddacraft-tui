# Steps: ANTI-001

| Field      | Value                                                                          |
| ---------- | ------------------------------------------------------------------------------ |
| Source     | [../modules/antipattern-library.aps.md](../modules/antipattern-library.aps.md) |
| Task(s)    | ANTI-001 — Pattern catalogue definition                                        |
| Created by | AI                                                                             |
| Status     | Draft                                                                          |

## Prerequisites

- [x] Warning schema exists in `core/src/antipattern/types.ts`
- [x] AntiPatternSchema defined with detection config

## Context

`AntiPatternSchema` already defines pattern structure with:

- `id`, `name`, `category`, `severity`, `confidence`
- `detection` (regex or AST-based)
- `title`, `explanation`, `suggestion`
- `allowlist`, `threshold`, `enabled`, `optIn`

Need to create the actual pattern catalogue.

## Steps

### 1. Create patterns.ts with catalogue structure

- **Checkpoint:** `core/src/antipattern/patterns.ts` exists with `PATTERNS`
  array
- **Files:** `core/src/antipattern/patterns.ts`

### 2. Add eslint-disable patterns

- **Checkpoint:** AP-001 (broad eslint-disable), AP-002 (rule-specific disable)
  defined
- **Pattern:** File-level: `eslint-disable(?!-next-line|-line)`, Line-level:
  `eslint-disable-(next-)?line`
- **Files:** `core/src/antipattern/patterns.ts`

### 3. Add type escape patterns

- **Checkpoint:** AP-003 (any), AP-004 (@ts-ignore), AP-005 (@ts-expect-error)
  defined
- **Files:** `core/src/antipattern/patterns.ts`

### 4. Add error handling patterns

- **Checkpoint:** AP-006 (empty catch), AP-007 (console in prod) defined
- **Files:** `core/src/antipattern/patterns.ts`

### 5. Add pattern lookup function

- **Checkpoint:** `getPattern(id)`, `getPatternsByCategory()` functions exist
- **Validate:** `nx test core --testNamePattern="patterns"`
- **Files:** `core/src/antipattern/patterns.ts`

### 6. Export from index

- **Checkpoint:** Patterns exported from `core/src/antipattern/index.ts`
- **Validate:** `pnpm typecheck`

## Completion

- [ ] All checkpoints validated
- [ ] Task marked complete in source module

**Completed by:** —
