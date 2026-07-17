# Actions: TRUST

| Field  | Value                                                                                     |
| ------ | ----------------------------------------------------------------------------------------- |
| Source | [../modules/trust-center-automation.aps.md](../modules/trust-center-automation.aps.md)   |
| Task   | TRUST — Full module execution                                                             |
| Status | Blocked                                                                                   |

## Prerequisites

- [ ] Trust artifact requirements and ownership model agreed
- [ ] COMPLY and CEWS outputs are implemented, or TRUST is explicitly
      re-scoped to a policy+eval-only summary
- [x] Policy/eval output contracts are stable
- [ ] Implementation homes are confirmed against ADR-098 AD-2; do not add new
      work to `crates/anvil-policy`

## Actions

### 1. Define trust artifact schema

- **Checkpoint:** Artifact schema covers policy/eval/compliance sources.
- **Validate:** `cargo test -p eddacraft-anvil-kernel-types -- trust_artifact_model`

### 2. Implement publishing pipeline

- **Checkpoint:** Trust summaries are generated with traceable metadata.
- **Validate:** `cargo test -p eddacraft-anvil -- trust_publishing`

### 3. Add freshness + ownership controls

- **Checkpoint:** Stale artifacts are flagged and routed.
- **Validate:** `cargo test -p eddacraft-anvil -- trust_freshness`

## Completion

- [ ] All checkpoints validated
- [ ] Task marked complete in source module
