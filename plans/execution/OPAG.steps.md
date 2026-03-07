# Steps: OPAG

| Field  | Value                                                                                   |
| ------ | --------------------------------------------------------------------------------------- |
| Source | [../modules/opa-agent-orchestration.aps.md](../modules/opa-agent-orchestration.aps.md) |
| Task   | OPAG — Full module execution                                                            |
| Status | Draft                                                                                   |

## Prerequisites

- [ ] OPA agent orchestration scope and priorities agreed
- [ ] Core test infrastructure available (`pnpm` and `nx` commands runnable)

## Steps

### 1. Establish orchestration contract

- **Checkpoint:** Contract fixtures validate and version field is present.
- **Validate:** `pnpm nx test core --testNamePattern="orchestration contract"`

### 2. Implement checkpoint runner

- **Checkpoint:** Same input yields same outcome across surfaces.
- **Validate:** `pnpm nx test core --testNamePattern="checkpoint runner"`

### 3. Normalise remediation guidance

- **Checkpoint:** Violation outputs include rationale and next action.
- **Validate:** `pnpm nx test core --testNamePattern="policy guidance"`

### 4. Add exception lifecycle + audit events

- **Checkpoint:** Exception transitions are validated and recorded.
- **Validate:** `pnpm nx test core --testNamePattern="exception workflow|policy audit events"`

### 5. Wire CLI/IDE/MCP/CI adapters

- **Checkpoint:** Surface adapters render same status semantics.
- **Validate:** `pnpm nx test cli && pnpm nx test mcp-server`

### 6. Enable guarded rollout

- **Checkpoint:** Rollout can be toggled and observed by environment.
- **Validate:** `pnpm nx test core --testNamePattern="orchestration performance"`

## Completion

- [ ] All checkpoints validated
- [ ] Task marked complete in source module
