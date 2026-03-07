# EVAL Execution Steps

### 1. Define eval harness contract
- **Checkpoint:** Port types compile with no framework imports.
- **Validate:** `pnpm nx test core --testNamePattern="eval harness port"`

### 2. Implement adapter and fixture runs
- **Checkpoint:** Adapter executes suites and returns normalized results.
- **Validate:** `pnpm nx test core --testNamePattern="eval harness adapter"`

### 3. Add CI regression command
- **Checkpoint:** CI command emits regression summary and exit code policy.
- **Validate:** `pnpm nx test cli --testNamePattern="eval regression command"`

### 4. Persist normalized results
- **Checkpoint:** Historical eval runs are queryable in Anvil schema.
- **Validate:** `pnpm nx test storage --testNamePattern="eval result persistence"`

### 5. Map failures to remediation guidance
- **Checkpoint:** Eval failures include policy-linked next actions.
- **Validate:** `pnpm nx test core --testNamePattern="eval policy guidance"`
