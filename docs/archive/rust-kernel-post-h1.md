# Rust Kernel — Post-H1 Capabilities Reference

**Status:** Future — not in H1 scope

This document preserves technical detail for capabilities planned after H1. For
the H1 spec, see [Rust Kernel Specification](rust-kernel-spec.md). For the
phased rollout, see [Architecture Evolution](anvil-architecture-evolution.md).

---

## Additional Language Support

- **Go** — common in infrastructure codebases
- **Python**, **Java**, **C#** — as demand warrants

---

## Policy DSL (Post-H1)

H1 uses hardcoded invariant checks. Post-H1, policies will be defined
declaratively. Each policy maps to one of the three invariant categories from
[Constitutional Engineering](../vision/constitutional-engineering.md).

### Structural Law

Hard invariants — non-negotiable boundary rules:

```yaml
policy: external-traffic-restricted
category: structural
match:
  call: fetch
  trust_level: HIGH
require:
  route: approved-gateway

policy: no-cross-layer-imports
category: structural
match:
  import_from: infrastructure/*
  import_in: domain/*
deny: true
message: "Domain layer must not import from infrastructure"
```

### Evolution Law

Rules about the rate and direction of change:

```yaml
policy: public-api-growth-gate
category: evolution
match:
  visibility: public
  change_type: added
require:
  plan_alignment: true
message: 'New public exports require an active plan reference'
```

### Procedural Law

Rules about how change is introduced:

```yaml
policy: boundary-shift-declaration
category: procedural
match:
  trust_level_changed: true
require:
  annotation: 'anvil:trust-change'
message: 'Trust level changes must be explicitly declared'
```

---

## APS Integration

Watcher can optionally load an active APS plan.

When enabled:

- Detect code changes outside declared plan scope
- Detect plan drift
- Validate declared structural change against actual diff

```
PlanDrift {
  file
  symbol
  expected_module
  actual_location
}
```

---

## Behavioural Diff Engine

Instead of text diff, compute:

- Call graph delta
- Public API surface delta
- Async path introduction
- New side-effect surface

```
BehaviouralDelta {
  new_external_calls: 1
  new_public_methods: 2
  privilege_scope_expansion: true
}
```

---

## Structural Drift Modelling

Maintain:

- Expected architecture boundaries
- Layer enforcement rules
- Drift tolerance thresholds

Track cumulative drift over time. Warn before architecture collapses.

---

## Provenance Tracking

For each symbol:

- Introduced in commit
- Introduced under plan
- Modified by whom
- Policy state history

---

## Semantic Risk Scoring

Compute:

- Risk index per module
- Architectural entropy score
- Privilege concentration index

Trend over time.

---

## Data Flow Classification

Advanced static analysis:

- Track data origin
- Track data sinks
- Enforce classification boundaries

```
PII → logging → violation
```

---

## Real-Time Agent Feedback Channel

Agents connected via:

- IPC socket
- JSON stream

Agents receive structured semantic deltas instead of raw git diff.

---

## Cross-Repo Structural Awareness

Enterprise mode:

- Multiple repos
- Shared contracts
- Service boundary validation

---

## Architecture Snapshots

Persist graph snapshots. Enable:

- Time-travel architecture diff
- Architectural regression detection

---

## Live Architecture Visualisation API

Expose:

- GraphQL
- JSON export
- Real-time graph streaming

Feed into TUI via
[Diagram Rendering](../research/diagram-rendering-for-ratatui.md).

---

## Distributed Watcher Mesh

Compile to native binary and WASM. Run in CI, agent VMs, dev machines,
pre-commit hooks. Aggregate telemetry into Edda.
