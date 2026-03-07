# CPOL Execution Steps

### 1. Define assertion schema
- **Checkpoint:** Assertion schema validates scoped condition rules.
- **Validate:** `pnpm nx test core --testNamePattern="assertion schema"`

### 2. Add context adapters
- **Checkpoint:** Assertions evaluate with deterministic context payloads.
- **Validate:** `pnpm nx test core --testNamePattern="assertion context"`

### 3. Add guidance outputs
- **Checkpoint:** Failed assertions include remediation guidance.
- **Validate:** `pnpm nx test core --testNamePattern="assertion guidance"`
