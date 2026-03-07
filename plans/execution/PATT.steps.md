# PATT Execution Steps

### 1. Define attack scenario schema
- **Checkpoint:** Scenarios validate payload/objective/expected behavior.
- **Validate:** `pnpm nx test contracts --testNamePattern="attack scenario schema"`

### 2. Implement pack runner
- **Checkpoint:** Attack packs execute deterministically.
- **Validate:** `pnpm nx test core --testNamePattern="attack pack runner"`

### 3. Wire CI threshold policy
- **Checkpoint:** CI enforces severity thresholds for regressions.
- **Validate:** `pnpm nx test cli --testNamePattern="attack regression gate"`
