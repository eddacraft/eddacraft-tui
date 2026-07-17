# Actions: GATE

| Field  | Value                                                                                                         |
| ------ | ------------------------------------------------------------------------------------------------------------- |
| Source | [../modules/gateway-control-plane-patterns.aps.md](../modules/gateway-control-plane-patterns.aps.md)         |
| Task   | GATE — Full module execution                                                                                  |
| Status | Draft                                                                                                         |

## Prerequisites

- [ ] Gateway component boundaries identified
- [ ] Core services and external dependencies that interact with the gateway are known
- [ ] First enterprise consumer or approved internal reference topology exists
- [ ] Any new interception boundary has the separate ADR required by ADR-098 AD-4

## Actions

### 1. Define reference topologies

- **Checkpoint:** Topology docs include trust boundaries and routing paths.
- **Validate:** `pnpm docs:check`

### 2. Define enforcement contract

- **Checkpoint:** Gateway policy decision schema reuses `ControlDecision` and
  shared `EnforcementMode` without introducing parallel vocabularies.
- **Validate:** `cargo test -p eddacraft-anvil-kernel-types -- gateway_enforcement`

### 3. Define observability event model

- **Checkpoint:** Events support auditable routing and denial traces.
- **Validate:** `cargo test -p eddacraft-anvil-observability -- gateway_events`

## Completion

- [ ] All checkpoints validated
- [ ] Task marked complete in source module
