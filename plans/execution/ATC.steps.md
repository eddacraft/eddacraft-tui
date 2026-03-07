# ATC Execution Steps

### 1. Define adversarial taxonomy
- **Checkpoint:** Probe categories and expectations are validated.
- **Validate:** `pnpm nx test contracts --testNamePattern="adversarial taxonomy"`

### 2. Build probe registry
- **Checkpoint:** Versioned probe packs are loadable.
- **Validate:** `pnpm nx test core --testNamePattern="probe registry"`

### 3. Integrate eval execution
- **Checkpoint:** Probe runs appear in eval summaries.
- **Validate:** `pnpm nx test core --testNamePattern="adversarial eval integration"`

### 4. Add trend reporting
- **Checkpoint:** Historical probe outcomes are visible by category.
- **Validate:** `pnpm nx test cli --testNamePattern="adversarial trends"`
