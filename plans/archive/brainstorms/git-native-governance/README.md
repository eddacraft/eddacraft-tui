# Anvil Git-Native Governance Pack

This pack captures the working product and architecture direction discussed in the conversation about using Git as more than source control inside Anvil.

The core thesis:

> Anvil should become the Git-native governance engine for AI-assisted software. Git becomes the durable trust substrate. Kindling observes work. Ember proposes meaning. Edda preserves institutional memory. Anvil witnesses and capsules prove that changes were governed.

## Documents

| File | Purpose |
| --- | --- |
| [`context.md`](./context.md) | Background, problem framing, current Anvil/Edda Stack context, key insights from the conversation, and the strategic product thesis. |
| [`architecture.md`](./architecture.md) | End-state architecture, component boundaries, Git object model, data ownership, flows, and how this plugs into Anvil today. |
| [`solution.md`](./solution.md) | PRD, technical design, command surface, data schemas, verification model, acceptance criteria, and MVP design. |
| [`roadmap.md`](./roadmap.md) | Execution plan with proposed modules, phases, work items, validation commands, and sequencing. |
| [`agent-handoff.md`](./agent-handoff.md) | Practical handoff for development/planning agents: what to inspect, what to decide first, open questions, and planning prompts. |

## Capture status — read this first

These documents are a **verbatim capture** of a design conversation, kept
intentionally unedited. Where they conflict with shipped code or the ADRs,
**the ADRs win**:

- `solution.md` §5.3 `WitnessExtract` (the `L0..L4` sub-objects,
  `agent.task_id`/`step_id`, `sha256:`-prefixed hashes) is **design fiction**
  — the shipped type is `anvil-witness::WitnessLine`
  (`crates/anvil-witness/src/line.rs`), embedded verbatim per ADR-074. Do not
  implement `WitnessExtract`.
- `architecture.md` §2.3 / `solution.md` §5.6 per-exception files under
  `anvil/exceptions/active|revoked/` describe a possible future layout; v0
  shipped a flat `anvil/exceptions/store.json`, and the layout decision
  belongs to EXCEPT-003.
- Authoritative decisions:
  [ADR-072](../../decisions/072-git-native-governance-substrate.md),
  [ADR-073](../../decisions/073-durable-vs-local-anvil-state.md),
  [ADR-074](../../decisions/074-review-capsule-v0-format.md).

## Recommended first implementation slice

Build **Anvil Review Capsules** first.

```sh
anvil capsule create --range main..HEAD --out review.anvil-capsule
anvil capsule verify review.anvil-capsule
anvil capsule explain review.anvil-capsule
```

The first capsule does not need to be a perfect Git bundle or use advanced refs/notes. It should prove the product loop:

> A developer can package a branch’s governance evidence, and another person can verify locally that the change was governed without trusting Anvil Cloud.

## Important architectural boundary

Resolve this early:

```text
.anvil/ = local runtime, cache, SQLite, logs, daemon state
anvil/  = tracked durable governance, evidence, memory, policy, witness state
```

This is foundational because the current Edda docs describe `.anvil/edda/` as tracked, while the newer multi-layer protection architecture treats `.anvil/` as local-only and `anvil/` as tracked project metadata. The proposed direction is:

```text
Kindling -> .anvil/kindling.db
Ember    -> .anvil/ember.db
Edda     -> anvil/edda/
Witness  -> anvil/witness/
Evidence -> anvil/evidence/ and/or refs/notes/anvil-*
```

## Product line

Use this as the internal north star:

> Anvil turns Git from source control into a tamper-evident governance substrate for AI-assisted engineering. It records what changed, what governed it, what evidence supported it, what exceptions were used, what humans approved, and what the organisation learned.

