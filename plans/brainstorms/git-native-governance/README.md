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

