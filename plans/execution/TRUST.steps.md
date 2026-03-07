# TRUST Execution Steps

### 1. Define trust artifact schema
- **Checkpoint:** Artifact schema covers policy/eval/compliance sources.
- **Validate:** `pnpm nx test contracts --testNamePattern="trust artifact model"`

### 2. Implement publishing pipeline
- **Checkpoint:** Trust summaries are generated with traceable metadata.
- **Validate:** `pnpm nx test core --testNamePattern="trust publishing"`

### 3. Add freshness + ownership controls
- **Checkpoint:** Stale artifacts are flagged and routed.
- **Validate:** `pnpm nx test core --testNamePattern="trust freshness"`
