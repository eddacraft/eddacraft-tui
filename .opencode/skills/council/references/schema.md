---
name: council-schemas
description: Data model reference for Council review sessions, findings, evidence, planning sessions, and judge decisions.
---

# Council Schemas

The authoritative persistence schema remains `references/schema.json`. Existing
session, finding, evidence, waiver, event, planning-session, and publication
shapes are unchanged by ADR-0025.

## Persisted session model

`session.mode` accepts `streaming`, `batch`, `design`, and `assurance`.
`session.reviewerPack` accepts `quick`, `standard`, `full`, `full:codex`, and
`risk-selected`. `session.judgeResult` holds the recorded decision
(`decision`, `gate`, `rationale`, `candidateHead`, `repairPassesUsed`,
`decidedAt`) and is written by `council-session.sh decide`. A decision bound to a
`candidateHead` other than the current head is stale.

Review sessions keep their existing `streaming|batch` mode, pinned target
identity, reviewer pack, findings, evidence, waivers, events, and timestamps.
Findings retain stable fingerprints and these contract dispositions:

- `in_contract`
- `later_item`
- `out_of_scope`
- `no_contract`

Finding lifecycle remains `open → fixed|deferred|waived|dismissed`, subject to
existing severity/waiver policy.

The existing `planningSession` definition in `schema.json` remains the persisted
shape for multi-persona planning/architecture work. `council design` may use that
shape; ADR-0025 does not create a second planning-session format.

## Reviewer output

Reviewer agents return one JSON object containing `findings` and `summary`.
Council supervision/deduplication ingests canonical findings into session state.

## Judge output

ADR-0025 extends, rather than breaks, the existing judge wire contract.
`gate` remains for scripts, PR publication and existing consumers; `decision`
adds loop-routing semantics.

```jsonc
{
  "gate": "BLOCK" | "WARN" | "PASS",
  "decision": "PASS" | "REPAIR" | "REWORK" | "REPLAN" | "BLOCK" | "DEBATE_REQUIRED",
  "summary": "One sentence evidence-backed verdict",
  "must_fix": [],
  "should_fix": [],
  "consider": [],
  "later_items": [],
  "root_causes": [],
  "debates_resolved": [],
  "reviewers": []
}
```

### Gate mapping

- `decision: PASS` with no meaningful advisories → `gate: PASS`
- `decision: PASS` with non-blocking `should_fix`/`consider` → `gate: WARN`
- `REPAIR | REWORK | REPLAN | BLOCK | DEBATE_REQUIRED` → `gate: BLOCK`

### Decision meaning

- `PASS` — no unresolved current-contract blockers.
- `REPAIR` — bounded defects; incremental repair remains economical.
- `REWORK` — defect density/coherence shows the implementation attempt itself
  should be replaced/escalated rather than patched finding by finding.
- `REPLAN` — design, decomposition, acceptance, or intent is materially wrong or incomplete.
- `BLOCK` — critical risk, authority, dependency, or environment prevents progress.
- `DEBATE_REQUIRED` — contradictory reviewer positions need adjudication first.

`must_fix` contains only current `in_contract` requirements. Valid later work
belongs in `later_items`; `out_of_scope` never blocks the current target.

## Council convergence

One judge decision ends one Council invocation. After a bounded `REPAIR`, fresh
deterministic evidence plus `verify-loop` check the change. Another full Council
requires material redesign/scope change, a critical redesign finding, a new
material verifier concern, explicit policy, or explicit user request.

## Debate output

The existing debate contract remains unchanged: one verdict (`A|B|split`),
evidence/proportionality/context scores, rationale, and action.

## Publication

Existing `council-publish.sh` formats remain supported. Consumers that only know
`gate` continue to work; consumers aware of ADR-0025 should use `decision` for
repair/rework/replan routing.
