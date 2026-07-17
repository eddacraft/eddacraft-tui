# Actions: CEWS

| Field  | Value                                                                                       |
| ------ | ------------------------------------------------------------------------------------------- |
| Source | [../modules/compliance-evidence-workspace.aps.md](../modules/compliance-evidence-workspace.aps.md) |
| Task   | CEWS — Full module execution                                                                |
| Status | Draft                                                                                       |

## Prerequisites

- [ ] Control-evidence model requirements agreed
- [ ] COMPLY-001..004 and the required POLLC lifecycle contracts are complete
- [x] Eval output contract is stable (EVAL Complete)
- [ ] Implementation homes are confirmed against ADR-098 AD-2; do not add new
      work to `crates/anvil-policy`

## Actions

### 1. Define control-evidence model

- **Checkpoint:** Model supports control, evidence, owner, status.
- **Validate:** `cargo test -p eddacraft-anvil-kernel-types -- control_evidence_model`

### 2. Implement evidence linking

- **Checkpoint:** Policy/eval outcomes attach to evidence records.
- **Validate:** `cargo test -p eddacraft-anvil-policy-engine -- evidence_linking`

### 3. Add workspace views/contracts

- **Checkpoint:** Workspace surfaces gaps, ownership, and readiness.
- **Validate:** `cargo test -p eddacraft-anvil -- evidence_workspace`

### 4. Generate export packs

- **Checkpoint:** Export includes auditable control-evidence trace.
- **Validate:** `cargo test -p eddacraft-anvil -- compliance_export`

## Completion

- [ ] All checkpoints validated
- [ ] Task marked complete in source module
