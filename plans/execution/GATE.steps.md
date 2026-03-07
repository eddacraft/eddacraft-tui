# GATE Execution Steps

### 1. Define reference topologies
- **Checkpoint:** Topology docs include trust boundaries and routing paths.
- **Validate:** `pnpm nx test docs --testNamePattern="gateway topology"`

### 2. Define enforcement contract
- **Checkpoint:** Gateway policy decision schema is stable.
- **Validate:** `pnpm nx test contracts --testNamePattern="gateway enforcement"`

### 3. Define observability event model
- **Checkpoint:** Events support auditable routing and denial traces.
- **Validate:** `pnpm nx test core --testNamePattern="gateway events"`
