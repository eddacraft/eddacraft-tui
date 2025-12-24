# Steps: ANTI-004

| Field      | Value                                                                          |
| ---------- | ------------------------------------------------------------------------------ |
| Source     | [../modules/antipattern-library.aps.md](../modules/antipattern-library.aps.md) |
| Task(s)    | ANTI-004 — Anti-pattern check integration                                      |
| Created by | AI                                                                             |
| Status     | Draft                                                                          |

## Prerequisites

- [x] ANTI-002 complete (ESLint disable detection)
- [x] ANTI-003 complete (type escape detection)
- [x] CORE-002 complete (analyzeFiles method)

## Context

Wire anti-pattern detectors into the GateRunner check system. Must return
`WarningResult` for integration with `analyzeFiles()`.

## Steps

### 1. Create antipattern.check.ts

- **Checkpoint:** `core/src/gate/checks/antipattern.check.ts` exists
- **Files:** `core/src/gate/checks/antipattern.check.ts`

### 2. Implement BaseCheck interface

- **Checkpoint:** `AntipatternCheck extends BaseCheck` with `run(context)`
- **Pattern:** See `architecture.check.ts` for reference
- **Files:** `core/src/gate/checks/antipattern.check.ts`

### 3. Wire ESLint disable detector

- **Checkpoint:** Check runs eslint-disable detection on target files
- **Files:** `core/src/gate/checks/antipattern.check.ts`

### 4. Wire type escape detector

- **Checkpoint:** Check runs type-escape detection on target files
- **Files:** `core/src/gate/checks/antipattern.check.ts`

### 5. Aggregate warnings into WarningResult

- **Checkpoint:** `run()` returns `GateResult` with
  `details.warnings: WarningResult`
- **Pattern:** Use `createWarningResult()` from antipattern/types.ts
- **Files:** `core/src/gate/checks/antipattern.check.ts`

### 6. Register in GateRunner

- **Checkpoint:** `AntipatternCheck` registered in `registerDefaultChecks()`
- **Files:** `core/src/gate/gate-runner.ts`

### 7. Add to analyzeFiles default checks

- **Checkpoint:** `analyzeFiles()` runs `antipattern` check by default
- **Files:** `core/src/gate/gate-runner.ts`

### 8. Add tests

- **Checkpoint:** Tests verify warning generation and WarningResult structure
- **Validate:** `nx test core`
- **Files:** `core/src/gate/checks/antipattern.check.test.ts`

## Completion

- [ ] All checkpoints validated
- [ ] Task marked complete in source module

**Completed by:** —
