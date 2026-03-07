# CEWS Execution Steps

### 1. Define control-evidence model
- **Checkpoint:** Model supports control, evidence, owner, status.
- **Validate:** `pnpm nx test contracts --testNamePattern="control evidence model"`

### 2. Implement evidence linking
- **Checkpoint:** Policy/eval outcomes attach to evidence records.
- **Validate:** `pnpm nx test core --testNamePattern="evidence linking"`

### 3. Add workspace views/contracts
- **Checkpoint:** Workspace surfaces gaps, ownership, and readiness.
- **Validate:** `pnpm nx test cli --testNamePattern="evidence workspace"`

### 4. Generate export packs
- **Checkpoint:** Export includes auditable control-evidence trace.
- **Validate:** `pnpm nx test core --testNamePattern="compliance export"`
