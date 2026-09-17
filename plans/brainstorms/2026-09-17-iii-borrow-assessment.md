# iii — Borrow Assessment

**Date:** 2026-09-17
**Status:** Brainstorm — Anvil opportunity assessment of `iii-hq/iii`, nominated
by the operator on the observation that "this is very cool, is it something we
could use". **Outcome: cool is confirmed; use is not.** iii is a backend
composition runtime with an agent harness layered on top — the layer the scope
guard names out of scope twice over (agent orchestration platform, observability
platform). The single point of contact is its optional `approval-gate` worker,
which holds an agent's function call pending a human. That gate is a policy
surface with **no evidence surface** — it keeps no resolved-approval history by
design — so it confirms Anvil's differentiation by contrast rather than adding
to it. **Decline code adoption** (Elastic License 2.0 engine with a
patent-pending notice; 128K lines of Rust that would sit beside Anvil's own
daemon). **Decline dependency.** Take two ideas: the **held-call gate shape**
(`continue` / `deny` / `hold`, fail-closed evaluation order, resumable
identity via a human-only `resolve`) and the **strip-at-entry rule** for
mutable hook points. **Disposition: Track** — catalogue entry only, no APS
module, no spec. One cheap experiment noted: an `anvil gate` iii worker bound to
the harness `pre_trigger` hook, in a scratch repo, to validate a distribution
channel and the mapping onto `ControlDecision`.
**Source:** https://github.com/iii-hq/iii (engine Elastic-2.0 + patent notice;
SDKs, CLI, console, docs Apache-2.0; v0.23.2-rc.2; last commit 2026-09-17;
vendor: Motia LLC — the April 2026 successor to the Motia framework). Star count
(~18.7k) read from the GitHub page, not the API; recorded as approximate.

---

## What this document is

A borrow assessment of an external repository, in the format set by
[`2026-09-07-ripwire-borrow-assessment.md`](./2026-09-07-ripwire-borrow-assessment.md):
same header block, same place in `plans/brainstorms/`, body structured as an
Anvil Opportunity Assessment per the `anvil-opportunity-assessment` skill. Facts
were read from a fresh shallow clone of the public repository at commit
`67ca962` (2026-09-17) — `README.md`, `engine/src/protocol.rs`,
`engine/src/invocation/auth.rs`, `engine/src/config/mod.rs`, `engine/LICENSE`,
`engine/PATENTS`, `LICENSE.spdx`, and the tech specs under `tech-specs/` (the
2026-06-08 agentic set, the 2026-06-22 RBAC proxy worker, and
`engine-register-trigger-metadata.md`) — and cross-checked against
[`docs/vision/anvil-scope-guard.md`](../../docs/vision/anvil-scope-guard.md),
[ADR-098](../decisions/098-policy-enforcement-reset-gate.md) AD-3/AD-4,
[ADR-024](../decisions/024-internal-agent-harness.md), and the
[2026-08-18 factory gap assessment](./2026-08-18-autonomous-factory-gap-assessment.md).

No shared module file is edited. No APS module is filed. The catalogue entry is
in
[`docs/strategy/borrow-adopt-candidates.md`](../../docs/strategy/borrow-adopt-candidates.md).

---

## What iii is

iii (pronounced "three eye") reduces a backend to three primitives:

- **Workers** — processes that connect to the engine over WebSocket and
  register what they offer. Any language with an SDK (TypeScript, Python, Rust,
  Go) becomes a worker.
- **Functions** — units of work with a stable id such as `orders::validate`,
  callable from any worker.
- **Triggers** — declarative bindings that fire a function: HTTP route, cron,
  queue subscription, state change, stream event, or a custom trigger type
  another worker defines.

The engine routes, serialises, and delivers; every invocation carries W3C
`traceparent` so the console shows one trace across all workers. Workers can be
added at runtime, so an agent lacking a capability installs a worker, discovers
its functions via `engine::functions::list`, and calls them. That is the agent
pitch. Engine: ~128K lines of Rust. SDKs and console: ~63K lines.

The `tech-specs/` directory is the strongest part of the repository: each spec
carries an explicit design-evolution section recording rejected iterations and
where the implementation diverged from the ticket.

---

## Anvil Opportunity Assessment

### Executive Summary

iii is a runtime substrate; Anvil is a deterministic control layer at the point
of change creation. They touch only at iii's optional approval gate, which is a
policy surface without an evidence surface. Decline the codebase and the
dependency; record two ideas and one experiment.

### Roadmap Disposition

**Track.** The durable value is one small primitive already half-present in
Anvil's enforcement vocabulary, plus a distribution channel with no customer
behind it. A catalogue entry is proportionate; an APS module is not.

### Recommended Next Step

Catalogue entry in `docs/strategy/borrow-adopt-candidates.md` (filed alongside
this document). Revisit only if (a) a prospect runs agents on iii, or (b) drain
mode grows into long-lived unattended processes and the ADR-024 harness
decision is reopened.

### Candidate Primitive

- **Name:** Held call with fail-closed resolution.
- **Description:** A `pre_trigger` hook answers `continue`, `deny`, or `hold`. A
  hold parks the agent turn without blocking the process, writes an ephemeral
  pending record, and a separate human-only `resolve` releases or denies it.
  Evaluation order is fixed: human-only targets deny → mode (`manual` / `auto`
  / `full`) → remembered human grants (`approved_always`, honoured in every
  mode) → curated allow list (`always_allow`, auto mode only) → YAML policy;
  any policy error or timeout degrades to **deny**, never to allow or to an
  unattended hold.
- **Why it matters to Anvil:** the shipped `ControlDecision` vocabulary is warn
  / fence / interrupt. There is no first-class "parked pending a human" outcome
  with a resumable identity. If Anvil ever wants an agent-facing approval
  surface, this is a clean, field-tested shape. ADR-098 AD-4 forbids a new
  tool-call interception layer without its own ADR, so it stays an idea.

### Scout Hypothesis Assessment

- **Status:** Not Provided (operator nomination: "cool; could we use it").
- **Explanation:** Cool confirmed. Use rejected on layer and licence grounds.

### Better Primitive Identified

The **metadata sidecar**. `engine-register-trigger-metadata.md` threads
registration-time context through the whole invocation path as a distinct
argument, never folded into the payload, so a shared handler can tell which
registration fired without corrupting the event. Same problem
`anvil-attribution` solves for "which agent, which session, which policy".
Anvil already has the equivalent; worth a read when attribution plumbing is next
touched.

### Customer Surface Test

Weak. No Anvil buyer gets better evidence, provenance, or a stronger gate from
anything in iii. The approval gate makes an agent safer to *run* — a runtime
concern, not a creation-time one.

### Criteria Scorecard

| Criterion                   | Score      |
| --------------------------- | ---------- |
| Direct anvil Fit            | 3          |
| Borrowable Primitive        | 6          |
| Developer-Native Usefulness | 7          |
| Evidence Before Enforcement | 3          |
| Deterministic Governance    | 5          |
| Audit and Export Value      | 2          |
| Narrow Beta Wedge           | 5          |
| Strategic Differentiation   | 4          |
| Clean-Room Feasibility      | 8          |
| Buyer Language Strength     | 5          |
| **Overall**                 | **48/100** |

### Key Ideas Worth Exploring

- **Hold as a third gate outcome.** Resumable identity for a blocked agent
  action. Action: note against POLCAP; gated on the ADR-098 AD-4 ADR.
- **Anvil as an iii worker.** The harness exposes `harness::hook::pre_trigger`;
  a small worker binding it to `anvil gate` puts Anvil in front of every agent
  on iii. Action: track; revisit on customer pull. Links only the Apache-2.0
  SDK.
- **Discovery filtered to the caller's boundary.** The RBAC proxy rewrites
  `engine::*` discovery results so a caller never sees what it cannot call.
  Anvil's GCTX privacy-gated projection already does this — confirmation, not
  addition.

### Patterns and Processes Worth Replicating

- **Strip at entry, not after the fact.** `post_generate` cannot mutate because
  the message already streamed; secrets are stripped in `post_trigger` before
  persistence. Same rule for Anvil's ledger: redact where evidence enters.
- **At-least-once delivery, at-most-once side effects.** On restart a call
  found `triggered` but not `done` is not re-run; a synthetic error result is
  written and the model decides. Worth borrowing as phrasing for witness and
  capsule recovery paths.
- **Honest design-evolution sections** in every tech spec.

### Licensing Assessment

- **Licence:** Engine Elastic License 2.0 with a patent-pending notice (Motia
  LLC). SDKs, CLI, console, docs Apache-2.0. Inbound contributions Apache-2.0
  only.
- **Risk Level:** High (engine), Low (SDKs).
- **Dependency Suitability:** Poor. ELv2 is source-available, not open source:
  no managed-service offering, no licence-key circumvention, patent notice on
  top.
- **Vendoring Suitability:** None for the engine.
- **Clean-Room Preference:** Yes — the ideas are small enough to reimplement
  from the public specs without reading engine code.
- **Notes:** ELv2 permits internal use, so dogfooding iii in Anvil's own
  pipeline would be legal; it is declined on other grounds (below).

### Acquisition Strategy

- **Selected Strategy:** Inspiration Only.
- **Reasoning:** Architectural, small ideas. The daemon dependency boundary and
  local-first posture rule out the engine as a dependency; ELv2 plus patents
  rule out vendoring or adaptation.

### Anvil Integration Surface

If the distribution idea is pursued: an adapter beside the MCP server — an iii
worker binding `harness::hook::pre_trigger` to the shipped gate. Nothing in the
kernel, checks, or policy engine changes.

### Development-side use (asked separately)

What iii would replace for Anvil's own engineering: the file-based pull-only
`agent-messaging` bus, council fan-out via the harness Agent tool, dev-loop and
drain-mode skills with a committed journal, and GitHub Actions + Nx. Declined
today because:

- Our agents are not processes — reviewers live for one council run inside one
  harness; there is nothing for an engine to hold a connection to.
- A second resident daemon on every developer machine and CI runner undercuts
  the daemon dependency boundary and local-first posture.
- ADR-024 already proposes weave as the internal harness; adopting iii's would
  be a competing decision without an ADR.
- The pull bus is a limitation only because council runs are short.
- Release-candidate churn; protocol messages defined but not yet emitted
  (`RegistrationRejected`).

It becomes a real decision if drain mode grows into unattended, long-lived
processes needing durable resume and a crash-surviving parked-pending-approval
state — the scenario the factory gap assessment describes. Note against ADR-024
rather than act now.

### Risks and Concerns

- **Layer mismatch** — same wrong-layer verdict as Proxilion.
- **Maturity churn** — rebranded five months ago, ships RCs, moving wire
  format.
- **Audit posture is opposite to Anvil's** — the gate deliberately keeps no
  resolved-approval history; Anvil's differentiator is the durable record.
- **Licence** — as above.

### Final Verdict

If I were making the decision today, I would track iii and take nothing but the
hold-and-resolve gate shape and the strip-at-entry rule, because everything else
it does well is a runtime concern Anvil has chosen not to own, and its engine
licence makes any deeper reuse a liability.

---

## Suggested follow-ups

- Filed: catalogue entry (`docs/strategy/borrow-adopt-candidates.md`,
  2026-09-17 section + deep-dive index).
- Not filed, deliberately: APS module; ADR; the scratch-repo `anvil gate`
  worker experiment (one afternoon, no roadmap commitment — run only if the
  distribution question becomes live).
