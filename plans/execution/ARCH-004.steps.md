# Steps: ARCH-004

| Field      | Value                                                                          |
| ---------- | ------------------------------------------------------------------------------ |
| Source     | [../modules/architecture-safety.aps.md](../modules/architecture-safety.aps.md) |
| Task(s)    | ARCH-004 — Init command enhancement                                            |
| Created by | AI                                                                             |
| Status     | Draft                                                                          |

## Prerequisites

- [ ] ARCH-001 complete (baseline inference)

## Context

`anvil init` exists at `cli/src/commands/init.ts`. Need to enhance it to:

1. Run architecture analysis
2. Show inferred boundaries
3. Allow user confirmation
4. Save baseline to `.anvil/architecture.json`

## Steps

### 1. Add architecture inference to init flow

- **Checkpoint:** `anvil init` calls `inferBaseline()` during setup
- **Files:** `cli/src/commands/init.ts`

### 2. Display inferred layers interactively

- **Checkpoint:** User sees detected layers with file counts
- **Pattern:** Use existing inquirer prompts in init.ts
- **Files:** `cli/src/commands/init.ts`

### 3. Allow boundary confirmation/editing

- **Checkpoint:** User can accept, skip, or modify boundaries
- **Files:** `cli/src/commands/init.ts`

### 4. Save baseline on confirmation

- **Checkpoint:** `.anvil/architecture.json` created with inferred baseline
- **Validate:** Manual test `anvil init` in test project
- **Files:** `cli/src/commands/init.ts`

### 5. Add --skip-architecture flag

- **Checkpoint:** `anvil init --skip-architecture` skips this step
- **Files:** `cli/src/commands/init.ts`

## Completion

- [ ] All checkpoints validated
- [ ] Task marked complete in source module

**Completed by:** —
