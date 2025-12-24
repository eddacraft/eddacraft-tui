# Steps: ARCH-003

| Field      | Value                                                                          |
| ---------- | ------------------------------------------------------------------------------ |
| Source     | [../modules/architecture-safety.aps.md](../modules/architecture-safety.aps.md) |
| Task(s)    | ARCH-003 — Architecture check integration                                      |
| Created by | AI                                                                             |
| Status     | Draft                                                                          |

## Prerequisites

- [x] ARCH-002 complete (new edge detection)
- [x] CORE-002 complete (analyzeFiles method)

## Context

`ArchitectureCheck` exists at `core/src/gate/checks/architecture.check.ts` but
uses dependency-cruiser directly. Need to integrate with the baseline-aware
analyzer and return `WarningResult` for planless mode.

## Steps

### 1. Add WarningResult output to ArchitectureCheck

- **Checkpoint:** `run()` populates `details.warnings: WarningResult`
- **Validate:** `nx test core --testNamePattern="architecture.check"`
- **Files:** `core/src/gate/checks/architecture.check.ts`

### 2. Wire baseline loading into check

- **Checkpoint:** Check loads `.anvil/architecture.json` if exists
- **Files:** `core/src/gate/checks/architecture.check.ts`

### 3. Convert violations to Warning format

- **Checkpoint:** Each violation maps to `Warning` with id `ARCH-###`
- **Pattern:** See `core/src/antipattern/types.ts` for Warning schema
- **Files:** `core/src/gate/checks/architecture.check.ts`

### 4. Add new-only mode

- **Checkpoint:** When baseline exists, only NEW violations generate warnings
- **Files:** `core/src/gate/checks/architecture.check.ts`

### 5. Update tests

- **Checkpoint:** Tests cover baseline-aware behaviour
- **Validate:** `nx test core`
- **Files:** `core/src/gate/checks/architecture.check.test.ts`

## Completion

- [ ] All checkpoints validated
- [ ] Task marked complete in source module

**Completed by:** —
