# OPAG Execution Steps

## Scope

Execute OPAG-001 through OPAG-007 with checkpoint validation and incremental rollout.

### 1. Establish orchestration contract
- **Purpose:** Lock schema before implementation spread.
- **Produces:** Contract definitions and compatibility tests.
- **Checkpoint:** Contract fixtures validate and version field is present.
- **Validate:** `pnpm nx test core --testNamePattern="orchestration contract"`

### 2. Implement checkpoint runner
- **Purpose:** Make policy timing deterministic across triggers.
- **Produces:** Save/staged/CI checkpoint runner.
- **Checkpoint:** Same input yields same outcome across surfaces.
- **Validate:** `pnpm nx test core --testNamePattern="checkpoint runner"`

### 3. Normalize remediation guidance
- **Purpose:** Improve clarity and actionability of failures.
- **Produces:** Guidance serializer and tests.
- **Checkpoint:** Violation outputs include rationale and next action.
- **Validate:** `pnpm nx test core --testNamePattern="policy guidance"`

### 4. Add exception lifecycle + audit events
- **Purpose:** Prevent ad-hoc bypasses and preserve accountability.
- **Produces:** Workflow states and immutable event records.
- **Checkpoint:** Exception transitions are validated and recorded.
- **Validate:** `pnpm nx test core --testNamePattern="exception workflow|policy audit events"`

### 5. Wire CLI/IDE/MCP/CI adapters
- **Purpose:** Keep user experience consistent by surface.
- **Produces:** Unified adapter outputs.
- **Checkpoint:** Surface adapters render same status semantics.
- **Validate:** `pnpm nx test cli && pnpm nx test mcp-server`

### 6. Enable guarded rollout
- **Purpose:** Deploy safely with measurable reliability.
- **Produces:** Feature flags, latency budgets, rollout docs.
- **Checkpoint:** Rollout can be toggled and observed by environment.
- **Validate:** `pnpm nx test core --testNamePattern="orchestration performance"`
