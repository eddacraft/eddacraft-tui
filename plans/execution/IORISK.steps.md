# Steps: IORISK

| Field  | Value                                                                     |
| ------ | ------------------------------------------------------------------------- |
| Source | [../modules/io-risk-controls.aps.md](../modules/io-risk-controls.aps.md) |
| Task   | IORISK — Full module execution                                            |
| Status | Draft                                                                     |

## Prerequisites

- [ ] IO risk categories and severity definitions agreed
- [ ] Core policy output pipeline is available

## Steps

### 1. Define IO risk taxonomy

- **Checkpoint:** Categories and severity map are stable.
- **Validate:** `pnpm nx test contracts --testNamePattern="io risk taxonomy"`

### 2. Implement scanner pipeline

- **Checkpoint:** Input/output checks run in pluggable sequence.
- **Validate:** `pnpm nx test core --testNamePattern="io scanner pipeline"`

### 3. Integrate policy guidance

- **Checkpoint:** Findings appear in unified policy outputs.
- **Validate:** `pnpm nx test core --testNamePattern="io risk guidance"`

## Completion

- [ ] All checkpoints validated
- [ ] Task marked complete in source module
