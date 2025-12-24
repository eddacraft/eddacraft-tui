# Steps: ARCH-004

| Field      | Value                                                                          |
| ---------- | ------------------------------------------------------------------------------ |
| Source     | [../modules/architecture-safety.aps.md](../modules/architecture-safety.aps.md) |
| Task(s)    | ARCH-004 — Init command enhancement                                            |
| Created by | AI                                                                             |
| Status     | Completed                                                                      |

## Prerequisites

- [x] ARCH-001 complete (baseline inference)

## Context

`anvil init` exists at `cli/src/commands/init.ts`. Need to enhance it to:

1. Run architecture analysis
2. Show inferred boundaries
3. Allow user confirmation
4. Save baseline to `.anvil/architecture.json`

## Steps

### 1. Add architecture inference to init flow

- **Checkpoint:** `anvil init` calls `analyseProjectArchitecture()` during setup
  ✅
- **Checkpoint:** `architecture-service.ts` wraps core architecture module ✅
- **Files:** `cli/src/commands/init.ts`,
  `cli/src/services/architecture-service.ts`

### 2. Display inferred layers interactively

- **Checkpoint:** User sees detected layers with file counts via
  `formatLayerDiagram()` ✅
- **Checkpoint:** User sees entry points via `formatEntryPoints()` ✅
- **Pattern:** ASCII box diagram with layer names and file counts
- **Files:** `cli/src/commands/init.ts`,
  `cli/src/services/architecture-service.ts`

### 3. Allow boundary confirmation/editing

- **Checkpoint:** User can accept, skip, or update via inquirer prompt ✅
- **Checkpoint:** Existing baseline detected and offered keep/update/skip
  options ✅
- **Files:** `cli/src/commands/init.ts`

### 4. Save baseline on confirmation

- **Checkpoint:** `saveArchitectureBaseline()` creates
  `.anvil/architecture.json` ✅
- **Validate:** Manual test `anvil init` in test project — works
- **Files:** `cli/src/commands/init.ts`,
  `cli/src/services/architecture-service.ts`

### 5. Non-interactive mode supported

- **Checkpoint:** `--non-interactive` flag auto-creates baseline if none exists
  ✅
- **Note:** `--skip-architecture` not implemented (not strictly needed — user
  can skip via prompt)
- **Files:** `cli/src/commands/init.ts`

## Completion

- [x] All checkpoints validated
- [x] Task marked complete in source module

**Completed by:** AI (2025-12-24)
