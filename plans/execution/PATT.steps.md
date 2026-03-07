# Steps: PATT

| Field  | Value                                                                                           |
| ------ | ----------------------------------------------------------------------------------------------- |
| Source | [../modules/prompt-attack-regression-packs.aps.md](../modules/prompt-attack-regression-packs.aps.md) |
| Task   | PATT — Full module execution                                                                    |
| Status | Draft                                                                                           |

## Prerequisites

- [ ] Attack scenario categories and severity definitions agreed
- [ ] CI gate infrastructure available

## Steps

### 1. Define attack scenario schema

- **Checkpoint:** Scenarios validate payload/objective/expected behaviour.
- **Validate:** `pnpm nx test contracts --testNamePattern="attack scenario schema"`

### 2. Implement pack runner

- **Checkpoint:** Attack packs execute deterministically.
- **Validate:** `pnpm nx test core --testNamePattern="attack pack runner"`

### 3. Wire CI threshold policy

- **Checkpoint:** CI enforces severity thresholds for regressions.
- **Validate:** `pnpm nx test cli --testNamePattern="attack regression gate"`

## Completion

- [ ] All checkpoints validated
- [ ] Task marked complete in source module
