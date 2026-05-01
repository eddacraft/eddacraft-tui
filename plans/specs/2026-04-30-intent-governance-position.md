# Intent Governance Position

**Date:** 2026-04-30 **Status:** Draft position for RCLI3 realignment **Applies
to:** Anvil plan/spec support, RCLI3 workflow commands, gate semantics,
agent-facing validation, and future spec-driven development integrations

## Purpose

Define Anvil's future relationship to plans, specifications, prompts, and
spec-driven development before RCLI3 ports any historical `anvil plan ...`
commands.

This document deliberately does **not** redesign APS. APS remains one useful
planning format. The larger product question is how Anvil should use any
intent-bearing artefact to make software evolution safer, more governable, and
more traceable.

## Position

Anvil is not a planning system.

Anvil is an **intent-governance layer for software evolution**.

It ingests plans, specs, prompts, ADRs, PRDs, issue descriptions, PR summaries,
and other accepted intent artefacts into a deterministic intent model, then
validates whether code changes remain legitimate relative to that intent and the
project constitution.

Plans and specs are not rigid execution scripts. They are intent sources.

Anvil's job is not to force the initial plan to remain true. Its job is to make
intent explicit, bind changes to that intent, detect drift, and preserve
provenance when reality forces the intent to evolve.

## Why This Changed

Anvil started as a plan validation and execution tool. ADR-001 moved the product
to a planless-first posture because the original model over-indexed on plans and
created adoption friction.

The old model was too rigid:

```text
write plan -> validate plan -> write code -> gate code against original plan
```

That does not match real software work, especially AI-assisted work.

In practice:

- requirements change during implementation
- tests reveal missing cases
- implementation discovers constraints the plan did not know
- refactors preserve behaviour but change structure
- agents take alternate paths to satisfy the same intent
- prompts, specs, and code drift unless reconciled

The useful invariant is not strict adherence to the first plan. The useful
invariant is whether each change remains legitimate under the current declared,
approved, or validated intent.

Inferred intent can exist, but it is not authoritative by itself. Anvil may
infer candidate intent from context, specs, PRDs, or agent sessions; that
inference only becomes enforceable after a user or trusted workflow approves it
into a durable intent artefact.

## Related Ideas

Structured Prompt-Driven Development (SPDD) argues that prompts and structured
specifications should become first-class delivery artefacts: versioned,
reviewable, reusable, and synchronised with code.

Reference: <https://martinfowler.com/articles/structured-prompt-driven/>

OpenSPDD implements that workflow around the REASONS Canvas:

- Requirements
- Entities
- Approach
- Structure
- Operations
- Norms
- Safeguards

Reference: <https://github.com/gszhangwei/open-spdd>

Anvil should not copy OpenSPDD's workflow role. OpenSPDD is primarily an
authoring and command-template system. Anvil's role is different: deterministic
ingestion, enforcement, provenance, drift detection, and legitimacy judgement.

The useful overlap is that SPDD treats prompts/specs as durable intent artefacts
rather than disposable chat. That maps directly onto Anvil's constitutional
engineering direction.

## Product Framing

Recommended framing:

> Anvil is the deterministic control layer that binds software changes to
> declared, approved, or validated intent, checks whether the evolution is
> legitimate, and records provenance when reality forces the intent to change.

This keeps ADR-001 intact:

- Without plans/specs, Anvil remains useful through architecture, policy,
  antipattern, command-safety, and secret checks.
- With plans/specs/prompts, Anvil gains intent, scope, safeguards, required
  evidence, and provenance anchors.

Planless-first becomes the adoption baseline. Intent-aware becomes the
high-leverage mode.

## Non-Goals

Anvil should not become:

- a task management platform
- a product planning tool
- an autonomous planning engine
- a generic prompt authoring tool
- a replacement for OpenSPDD, SpecKit, BMAD, or APS
- a live LLM judge for enforcement decisions

Anvil may help normalise or validate artefacts from those systems, but the
authoring workflow remains outside Anvil unless a future decision explicitly
changes that boundary.

## Core Concepts

### Intent Artefact

An intent artefact is any durable source that explains why a change exists and
what constraints govern it.

Examples:

- APS module or task
- SPDD REASONS Canvas
- SpecKit feature spec
- BMAD PRD or architecture document
- ADR
- GitHub issue
- PR description
- commit message
- agent-session summary
- human override record

### Intent Model

Anvil should normalise supported artefacts into a small canonical model.

For RCLI3 realignment, the minimum model should stay small:

```text
IntentArtifactV1
  id
  source_format
  source_path_or_url
  title
  summary
  intent_claims[]
  scope[]
  non_scope[]
  safeguards[]
  validation_evidence_required[]
  approval_status
  accepted_by
  accepted_at
  source_hash
  adapter_version
```

Only artefacts with an accepted approval state are authoritative enforcement
input. Inferred or extracted artefacts remain candidate context until accepted.

Future versions may extend the model with richer graph and provenance fields.

Sketch:

```text
IntentArtifact
  id
  source_format
  source_path_or_url
  title
  summary
  intent_claims[]
  requirements[]
  scope[]
  non_scope[]
  safeguards[]
  authorised_changes[]
  affected_paths[]
  affected_symbols[]
  validation_evidence_required[]
  dependencies[]
  open_questions[]
  decisions[]
  confidence
  extraction_evidence[]
```

This broader sketch is not the first implementation target. It is the direction
for the future intent graph. The first RCLI3 slice should not require symbol
binding, trust graph integration, dependency graph materialisation, or generic
cross-format adapter infrastructure.

This model is not a new planning language. It is an enforcement substrate.

`confidence` is extraction provenance, not authority. A low-confidence or
heuristically extracted artefact may help a human review intent, but it must not
become a gate input unless accepted into the deterministic model.

### Intent Graph

Multiple intent artefacts combine into an intent graph.

This is future direction, not a prerequisite for RCLI3.

The intent graph links:

- artefacts to claims
- claims to safeguards
- safeguards to checks/policies
- scope to files/symbols/modules
- changes to commits/PRs/agent sessions
- validations to evidence
- exceptions to human approvals

The aspirational graph becomes:

```text
Repository
  -> AST Graph
  -> Symbol Graph
  -> Dependency Graph
  -> Trust Graph
  -> Intent Graph
```

This generalises the older "Plan Graph" idea without making APS the centre of
the product.

## Determinism Boundary

Anvil can use probabilistic tools to help with interpretation, but enforcement
must be deterministic.

The boundary is:

```text
interpretation time = LLMs may propose candidate intent
acceptance time = user/trusted workflow approves intent into a durable artefact
enforcement time = deterministic checks only
```

Anvil must not gate a change on a fresh, unreviewed LLM interpretation of a PRD.

If a messy PRD requires LLM extraction, the output becomes a candidate intent
artefact. Once approved by a user or trusted workflow and committed or otherwise
made durable, Anvil can enforce against the stable extracted model.

## Adapter Strategy

Anvil should support spec-driven development by ingesting common formats through
adapters. Adapter output should feed the canonical intent model.

### Tier 1: Native Structured Adapters

For formats with stable, recognisable structure.

Examples:

- APS
- OpenSPDD / REASONS Canvas
- SpecKit
- BMAD
- ADRs
- OpenAPI where endpoint/API intent is relevant
- structured GitHub issue or PR templates

These should be deterministic parsers.

For the pre-RCLI3 slice, APS is the only assumed native adapter. Other examples
need separate prioritisation before implementation.

### Tier 2: Heuristic Markdown Adapter

For ordinary PRDs, design docs, and project notes.

Extract deterministic signals from headings and field conventions:

- Requirements
- Acceptance Criteria
- Scope
- Out of Scope
- Constraints
- Risks
- Validation
- Dependencies
- Non-functional Requirements
- Security
- Data Handling

This tier is lower confidence but still deterministic.

Deterministic does not automatically mean authoritative. Heuristic Markdown
outputs are candidate intent until reviewed and accepted.

### Tier 3: LLM-Assisted Extraction

For unstructured or inconsistent artefacts.

The LLM may propose a normalised intent model, but Anvil treats it as derived
intent until accepted. The enforcement layer consumes only the accepted result.

## Drift Classes

Anvil should distinguish kinds of divergence instead of treating every mismatch
as a plan violation.

| Drift class             | Meaning                                    | Correct response                            |
| ----------------------- | ------------------------------------------ | ------------------------------------------- |
| Intent change           | The desired behaviour changed              | Update the intent artefact first, then code |
| Implementation mismatch | Code does not satisfy declared intent      | Change code or tests to match intent        |
| Safe refactor           | Behaviour preserved, structure changed     | Sync the artefact after code review         |
| Discovery               | Implementation uncovered a new constraint  | Record the decision and update intent       |
| Scope creep             | Code expands beyond declared authority     | Require approval, new intent, or rollback   |
| Constitutional breach   | Change violates a non-negotiable invariant | Block or require explicit exception         |

This is the SPDD loop expressed in Anvil terms.

The table describes reconciliation order, not default enforcement action. Anvil
still follows warnings-over-blocks by default; blocking requires explicit
configuration or a later intercept/control decision.

## Relationship To Gates

`anvil gate` should evolve from "run checks, optionally file-scope from a plan"
to "judge whether the change passes required checks under accepted active
intent".

Plan/spec-aware gates should be able to ask:

- Is this change bound to an intent artefact?
- Are touched files/symbols inside declared scope?
- Did the change introduce unauthorised architecture edges?
- Did it expand public API, trust surface, or external surface without declared
  intent?
- Were required validations run?
- Are safeguards satisfied?
- Were exceptions recorded with provenance?

This keeps gates aligned with the quality model: a gate is workflow judgement,
not task management.

Intent binding should be opt-in or policy-driven. Planless-first remains the
default: when no accepted intent artefact is supplied or configured, Anvil runs
normal checks and gates rather than requiring a plan/spec.

## Relationship To Watch, MCP, And Intercept

Intent governance is most valuable at change creation time.

For watch, MCP, and intercept surfaces, Anvil should eventually detect:

- edits outside active intent scope
- pre-write changes that conflict with safeguards
- agent tool calls without an intent anchor
- repeated attempts to cross a constitutional boundary
- code generation that changes behaviour when only a refactor was authorised

This turns spec-driven development from retrospective review into live control.

These surfaces must fail open for missing or unavailable intent unless an
explicit repository policy chooses stricter behaviour.

## Relationship To Provenance

Every governed or intent-bound significant change should eventually answer:

```text
What intent authorised this?
What files and symbols changed?
What constitutional safeguards applied?
What evidence proved it?
Who or what performed it?
What changed when reality diverged from the original intent?
```

This is the bridge between intent governance and Anvil's longer-term code
provenance engine.

## Outstanding Design Decision: Memory And Agent Artefacts

Intent governance introduces artefacts that were not in scope when the Edda
Stack and Kindling integration were originally framed:

- chat transcripts
- agent-session summaries
- tool-call traces
- prompt/spec evolution records
- model-generated rationales
- human review notes on AI output

These artefacts may contain intent, evidence, and provenance, but they also
carry different trust and privacy properties from normal repository events. They
should not be casually forced into Kindling, assigned to a new fourth memory
layer, or stored through an ad hoc side channel that bypasses the Edda Stack.

Interim guardrails:

- Raw chat transcripts are not stored by default.
- Model chain-of-thought is out of scope: Anvil must not request, persist,
  normalise, or enforce against chain-of-thought.
- Observable reasoning artefacts are allowed: prompts, tool calls, diffs,
  validation output, user decisions, summaries, approvals, and evidence records.
- Agent artefacts default to structured, redacted event summaries with source,
  trust level, retention class, redaction status, and deletion policy.
- Raw transcript or raw tool-payload storage requires an explicit future ADR
  with consent, retention, redaction, and access-control requirements.
- Raw transcripts, raw tool payloads, and model rationales are never gate
  evidence by themselves.

The design question is open:

> Where should conversational and agent-execution artefacts live in Anvil's
> memory model, and what portions of them are legitimate enforcement evidence?

This decision must address:

- whether raw transcripts are ever stored, or only structured summaries/events
- whether model chain-of-thought is explicitly out of scope and replaced by
  observable reasoning artefacts, decisions, and evidence
- how consent, redaction, retention, and trust boundaries apply to AI-session
  data
- whether Kindling should ingest these as observations, whether Ember should
  interpret them into proposals, or whether a separate boundary is required
- how intent binding links to existing provenance, notification, and future
  graph records
- what deterministic subset can be consumed by gates, watch, MCP, and intercept

Until this is resolved, intent governance must treat chat/transcript-derived
data as candidate context only. It must not become authoritative enforcement
input unless converted into an accepted, deterministic artefact with provenance
and retention rules.

For RCLI3, the minimum decision is narrower: chat, transcript, and tool-trace
artefacts are not supported enforcement inputs unless they have already been
converted into an accepted deterministic artefact. The broader Kindling / Ember
/ Edda placement decision can be deferred until Anvil deliberately supports
memory-derived evidence.

## RCLI3 Implications

RCLI3 should not blindly port the old Node.js `anvil plan ...` command set.

Current RCLI3 items that mention plan workflow need review before
implementation:

- `RCLI3-008`: `anvil plan validate`
- `RCLI3-009`: `anvil plan load` and `anvil plan status`
- `RCLI3-010`: `anvil plan lock` and `anvil plan unlock`

Open questions before implementation:

- Should the surface still be named `plan`, or should it become `intent`?
- Should `validate` validate a document format, a canonical intent model, or
  both?
- Should `status` mean task workflow state, evidence state, or intent drift
  state?
- Should locking exist in Anvil, or is binding/evidence a better primitive?
- Should RCLI3 support APS first, or a generic intent adapter interface first?
- How do memory artefacts from chats, agents, and prompt evolution relate to the
  Edda Stack, and which parts are safe to use as deterministic intent evidence?

Minimum pre-RCLI3 answer: do not consume chat, transcript, or tool-trace memory
as enforcement evidence. Only accepted deterministic artefacts are in scope.

The likely answer is that RCLI3 needs a smaller, sharper surface than the old
plan workflow.

Possible future commands, names not final and not approved for RCLI3:

```bash
anvil intent validate <artifact>
anvil intent inspect <artifact>
anvil intent bind <artifact>
anvil intent drift
anvil gate --intent <artifact>
```

This command shape is intentionally provisional. The decision to rename from
`plan` to `intent` requires separate product review.

Provisional RCLI3 recommendation: keep user-facing `anvil plan validate` only if
it is explicitly defined as deterministic intent artefact validation, not task
management. Reserve a top-level `anvil intent ...` surface for a later product
rename decision.

## First Practical Slice

The smallest useful slice is not full intent governance. It is a decision and a
minimal deterministic validation target for RCLI3.

Recommended first slice:

1. Decide the RCLI3 surface: keep `anvil plan validate` for compatibility or
   defer plan commands until an `intent` surface is approved.
2. Define `IntentArtifactV1` as the minimum deterministic model above.
3. Support APS or the current repository plan format only as the first accepted
   source, or explicitly document the supported executable subset.
4. Validate artefacts into canonical JSON with source hash, adapter version,
   approval state, and stable diagnostics.
5. Update `anvil gate <plan>` documentation so current behaviour is not
   overstated.
6. Add golden fixtures proving deterministic extraction does not change without
   an intentional adapter/schema update.

This slice informs `RCLI3-008`. It does not implement `RCLI3-009` or
`RCLI3-010`. `plan load/status/lock/unlock` remain deferred until the
intent-governance surface decision is complete.

## Risks

| Risk                                   | Mitigation                                                                 |
| -------------------------------------- | -------------------------------------------------------------------------- |
| Rebuilding a planning app              | Keep authoring out of scope; ingest only                                   |
| LLM-based enforcement                  | Enforce only accepted deterministic artefacts                              |
| Format explosion                       | Use adapter tiers and a small canonical model                              |
| Over-rigid plan policing               | Classify drift and support legitimate intent evolution                     |
| Naming confusion                       | Separate check, gate, validate, status, and intent language                |
| APS special casing                     | Treat APS as one adapter, not the product centre                           |
| Memory boundary confusion              | Decide transcript/agent artefact handling before treating them as evidence |
| Secret leakage from traces/transcripts | Do not store raw transcripts by default; redact before persistence         |
| Chain-of-thought capture               | Treat model chain-of-thought as out of scope                               |
| Stale or ambiguous extracted intent    | Require acceptance state, source hash, adapter version, and warnings       |

## Open Questions

1. Is `intent` the right product word, or should the CLI keep `plan` for user
   familiarity?
2. Should accepted LLM-derived intent be stored as JSON, markdown front matter,
   sidecar files, or committed generated specs?
3. What is the minimum provenance event required to bind a change to intent?
4. Which safeguards are enforceable in the first slice without Graph v2?
5. Should SPDD/REASONS Canvas be a first-class adapter before or after APS
   realignment?
6. How should intent binding interact with ADRs and constitutional invariants?
7. Which RCLI3 tasks should be rewritten, superseded, or split?
8. Where do chat transcripts, agent-session artefacts, and prompt evolution
   records belong relative to Kindling, Ember, Edda, and future graph storage?

## Decision Needed Before RCLI3

Before implementing `RCLI3-008`, `RCLI3-009`, or `RCLI3-010`, decide:

- whether the RCLI3 workflow surface is plan-centric or intent-centric
- which artefact formats count as supported intent sources
- what deterministic model gates consume
- whether `RCLI3-008` validates APS specifically or an accepted intent artefact
  model

Before implementing `RCLI3-009` or `RCLI3-010`, additionally decide:

- whether task workflow status remains a product concept
- whether task locking remains a product concept
- how plan/spec drift is surfaced without becoming task management

Before consuming conversational or agent-execution memory as evidence, decide:

- where those artefacts live relative to Kindling, Ember, Edda, and future graph
  storage
- how they are redacted, retained, accepted, and audited
- whether they can ever become deterministic evidence, and under what explicit
  approval path

Until those decisions are made, RCLI3 plan workflow work should remain Proposed.
