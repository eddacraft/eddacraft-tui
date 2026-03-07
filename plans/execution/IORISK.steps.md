# IORISK Execution Steps

### 1. Define IO risk taxonomy
- **Checkpoint:** Categories and severity map are stable.
- **Validate:** `pnpm nx test contracts --testNamePattern="io risk taxonomy"`

### 2. Implement scanner pipeline
- **Checkpoint:** Input/output checks run in pluggable sequence.
- **Validate:** `pnpm nx test core --testNamePattern="io scanner pipeline"`

### 3. Integrate policy guidance
- **Checkpoint:** Findings appear in unified policy outputs.
- **Validate:** `pnpm nx test core --testNamePattern="io risk guidance"`
