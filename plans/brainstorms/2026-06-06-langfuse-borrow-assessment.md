# Langfuse — Borrow Assessment

**Date:** 2026-06-06
**Status:** Brainstorm — assessment of Langfuse as a borrow candidate (nominated
by Morgan; deep-dive promotion 2026-06-05 of the `langfuse` tracker entry filed
2026-05-13, itself from the 2026-05-02 radar). **Outcome: decline code/runtime
adoption and decline dependency (Langfuse is an LLM-engineering *platform* —
observability, evals, prompt management, datasets, playground — a product-breadth
surface Anvil does not occupy). Validate Morgan's strategic read (governance
evidence should be correlatable inside the buyer's own observability stack), but
this is NOT net-new roadmap space and two specifics of the proposed fixture
collide with decisions Anvil has already ratified. The genuine, durable primitive
is the *governance-decision correlation span* — a refs-only OpenTelemetry
breadcrumb that carries the cross-pipe correlation key plus pointers, while the
durable evidence stays on Kindling / the witness chain. (1) Morgan's fixture
stuffs durable governance evidence (`verdict`, `evidence_refs`, `policy_digest`,
`risk_signal_refs`) into a span — directly against [ADR-035](../decisions/035-three-pipe-observability-rule.md)
(three-pipe rule: the tracing/OTEL pipe is ephemeral, never source-of-truth).
(2) The "publish into Langfuse / Datadog / Grafana / Honeycomb" multi-destination
posture (`exporter_ref` / `destination_ref` as pluggable sinks) cuts against
[ADR-059](../decisions/059-production-tracing-sink.md) (single Azure Monitor sink,
operator-hosted only, CLI/daemon local-first and never auto-exporting). The work
already lives in TRACE (In Progress 2/4) + EXPORT (Draft); the true increment is
**S** (semconv alignment + a refs-only correlation-span contract + one corrected
fixture), not the **M/greenfield** the nomination implies. Disposition: **Track →
fold into EXPORT**; no new APS module filed; suggested CIB filings in §7.3. Cite
Langfuse as market validation of the OTel-GenAI wedge; no dependency; clean-room
against the open OpenTelemetry GenAI semantic conventions, not Langfuse code.**
**Source:** https://github.com/langfuse/langfuse — **MIT** for the core, with
`ee/` (Enterprise Edition) folders under a separate proprietary/commercial
licence. High-traction, actively maintained; integrates with OpenTelemetry,
LangChain, OpenAI SDK, LiteLLM, and more. (Facts read from the public repo
landing / README on 2026-06-05–06.)

---

## 0. What this document is

A borrow assessment of an external repo, in the format of
[`2026-06-06-node9-borrow-assessment.md`](./2026-06-06-node9-borrow-assessment.md),
[`2026-06-03-rulehub-borrow-assessment.md`](./2026-06-03-rulehub-borrow-assessment.md),
and [`2026-06-03-meho-borrow-assessment.md`](./2026-06-03-meho-borrow-assessment.md).
The goal is **not** to adopt Langfuse but to mine it for a reusable primitive,
scope-guard it, map it onto the exact APS modules that already own this territory,
and name the gaps. Facts were cross-checked against `plans/modules/*`
(in particular [tracing-foundation](../modules/tracing-foundation.aps.md) and
[observability-export](../modules/observability-export.aps.md)),
`docs/vision/anvil-scope-guard.md`,
[ADR-035](../decisions/035-three-pipe-observability-rule.md),
[ADR-059](../decisions/059-production-tracing-sink.md),
`crates/anvil-observability/`, and the
[borrow-adopt-candidates](../../docs/strategy/borrow-adopt-candidates.md) tracker
(the `langfuse` entry, 2026-05-13).

**Maturity note up front:** Langfuse is by far the most-adopted candidate
assessed (a high-traction, well-maintained platform vs Node9 200★, Drako/MEHO/PIC
single-digit ★). That makes the "just integrate it / emit to it" pull strong and
the "the OTel-GenAI ecosystem is real" signal loud — but it does not move the
scope-guard line, and it does not make this net-new: Anvil already has a tracing
crate, a W3C `traceparent` implementation, the three-pipe rule, and a ratified
production sink. Adoption maturity ≠ unfilled roadmap.

---

## 1. Nomination summary

Morgan nominated Langfuse on a single, sharp framing: **trace interoperability**.
Anvil governance decisions should emit OpenTelemetry-compatible evidence so teams
can inspect agent actions, policy gates, and audit events in the observability
stack they already run (Langfuse, Datadog, Grafana, Honeycomb) instead of
adopting yet another dashboard. The point is explicitly *not* product breadth —
"don't become Langfuse" — it is correlation: emit policy decisions, normalized
actions, evidence refs, verdicts, risk signals, and latency into
OTel-compatible traces so governance proof sits beside the LLM/app traces.

The proposed next action is a `GovernanceTraceExport` / `AnvilOtelSpan` fixture
with: `trace_id`, `span_id`, `parent_span_id`, `agent_session_ref`,
`workflow_ref`, `anvil_decision_ref`, `policy_ref`, `policy_digest`,
`normalized_action_ref`, `tool_call_ref`, `verdict`, `evidence_refs`,
`risk_signal_refs`, `latency_ms`, `exporter_ref`, `otel_semconv_version`,
`destination_ref`, `correlation_status` — and one fixture where an Anvil block
correlates to an upstream LLM/tool span **without exporting raw prompt or secret
material**.

Open questions Morgan raises: (1) define Anvil's own span names first, or align
with emerging OpenTelemetry GenAI semantic conventions from day one? (2) which
decision fields should be span attributes vs linked evidence artifacts? (3) how
much sensitive prompt/tool data must be omitted or redacted before export?

| Project | What it is | Stack | Maturity |
| ------- | ---------- | ----- | -------- |
| Langfuse | Open-source LLM engineering platform: tracing/observability over LLM calls + app logic (retrieval, embeddings, agent actions), plus prompt management, evaluations, datasets, a playground, and API/SDK integration. Self-hostable. Integrates with OpenTelemetry, LangChain, OpenAI SDK, LiteLLM, etc. | TypeScript-heavy web app + SDKs | **MIT** (core) + proprietary `ee/`; high-traction, actively maintained |

**Operational footprint:** Langfuse is a deployed platform (web app + datastore +
SDKs). The Anvil-relevant surface is *none of the product* — it is the single
idea that governance evidence should be **correlatable via OpenTelemetry** with
the traces a buyer already collects.

---

## 2. Scope-guard test

Per [`docs/vision/anvil-scope-guard.md`](../../docs/vision/anvil-scope-guard.md),
Anvil operates at the **moment of change creation**, enforces deterministic policy
against artefacts, and **captures provenance for policy decisions**. The four
borderline questions: (1) increases prevention, (2) operates before/at execution
time, (3) strengthens deterministic control, (4) enforces rather than only
informs. Observability is an explicit **out-of-scope** example *except where it
directly supports enforcement or provenance*.

Morgan's nomination bundles one in-scope primitive with two posture choices that
fail the guard. Un-bundle them:

| Langfuse-adjacent capability | Decisive question | Scope-guard read |
| ---------------------------- | ----------------- | ---------------- |
| **Governance-decision correlation span** — emit a span at an Anvil decision carrying the correlation key + pointers so the decision is *joinable* to the buyer's LLM/tool trace | #4 — provenance tied to enforcement? | **In (the borrow).** This is In-Scope pillar #5 ("provenance & traceability; linking actions to policies and rules"). The span makes an enforcement decision Anvil *already produces* findable next to the work it governed. It is provenance attached to a decision that itself enforces. |
| **Durable governance evidence carried *inside* the span** (`verdict`, `evidence_refs`, `policy_digest`, `risk_signal_refs` as inline span payload treated as the record) | source-of-truth? | **Out — violates ADR-035.** The tracing/OTEL pipe is ephemeral debugging context, *never* source-of-truth. Governance facts live on Kindling; user-visible state on the notification envelope. Evidence-in-span manufactures a second, lossy, sampled, retention-bounded source of truth. The span carries **refs**, not the evidence. |
| **Multi-destination "publish anywhere" export** (`exporter_ref` / `destination_ref` modelling Langfuse *and* Datadog *and* Grafana *and* Honeycomb as pluggable sinks) | Anvil's decided architecture? | **Out (today) — collides with ADR-059.** ADR-059 ratified a **single** sink (Azure Monitor + Application Insights), operator-hosted only, with the Rust CLI/daemon staying **local-first and never auto-exporting**. A pluggable destination matrix is a different posture that needs its own ADR, not a fixture field. (OTLP being vendor-neutral *at the wire* is fine; "Anvil ships exporters to four backends" is the scope change.) |
| **Standalone observability dashboard / "inspect agent actions" product surface** detached from a decision | #4 — only informs? | **Out.** Scope-guard exclusion #5 (observability platform). Anvil must not require — or build — a dashboard for its decisions to be trusted; the whole point of the borrow is to *avoid* that by emitting into the buyer's stack. |
| **OTel as the wire format / W3C `traceparent` propagation** | #3 — already owned | **In, but already owned.** `anvil-observability` ships `TraceContext` (W3C `traceparent` v00), `init_tracing`, JSON formatter, redaction deny-list (TRACE-001/004, shipped). Parallel evolution. |
| **Prompt management / evals / datasets / playground** | #1–#4 | **Out — product breadth.** None of it is governance, provenance, or deterministic control. This is the "don't become Langfuse" line Morgan correctly draws. |

**Verdict:** the borrow is **In** only as a *refs-only correlation span that sells
and locates enforcement evidence* — and only while it stays a breadcrumb pointing
back to durable Kindling/witness evidence. The instant durable evidence moves into
the span (ADR-035 reject) or Anvil starts shipping exporters to a matrix of
backends (ADR-059 reject), the borrow leaves scope. Morgan's strategic instinct —
"don't make buyers live in an Anvil dashboard; emit into theirs" — is exactly
right; his *fixture* needs the un-bundling above so it doesn't quietly demote the
witness chain to a sampled span.

---

## 3. Overlap with existing Anvil work

This is the crux of the challenge: the OTel-export capability is **not a gap** —
it is already a two-module roadmap with a ratified sink and a governing ADR, and
the `langfuse` candidate has been on the tracker since **2026-05-13**.

| Langfuse-adjacent capability | Anvil equivalent (status) |
| ---------------------------- | ------------------------- |
| OTel/OTLP as trace transport for AI events | **EXPORT** (observability-export) · **Draft 0/1** — EXPORT-001 wires the production exporter. Sink ratified: **Azure Monitor + App Insights** ([ADR-059](../decisions/059-production-tracing-sink.md), Accepted 2026-05-30), OTLP-neutral, off by default, operator-hosted only. The original candidate list literally included Honeycomb / Grafana Cloud / OTLP. |
| Tracing baseline + W3C `traceparent` propagation | **TRACE** (tracing-foundation) · **In Progress 2/4** — `anvil-observability` crate shipped (TRACE-001/004): `TraceContext`, `init_tracing(BinaryKind)`, JSON formatter, `bind_traceparent_to_current_span`, local `ANVIL_TRACE_SINK=file=<path>` dev sink. TS mirror (TRACE-002) and redaction hardening (TRACE-003) partially landed, then Blocked. |
| "Spans never source-of-truth; correlate to durable facts" | **[ADR-035](../decisions/035-three-pipe-observability-rule.md) — three-pipe rule** (Accepted). Kindling = governance facts (durable); notification envelope = user-visible state (durable); tracing/OTEL = ephemeral debugging. **`traceparent` is the named cross-pipe correlation key.** This ADR *is* the answer to "where does the evidence live vs the span." |
| Durable governance evidence / audit record | **Witness chain** (MLP2, `anvil/witness/manifest/chain.ndjson`, hash-chained, local-first) + Kindling. `rules_sha` (MLP2-014) already threads a signed policy-bundle reference per line — i.e. Morgan's `policy_digest` already exists as durable evidence. |
| Redaction before any span leaves the process | **TRACE-003** + `anvil-observability` redaction deny-list (`SENSITIVE_FIELDS`); EXPORT-001 **V2** asserts the exporter cannot bypass redaction. Morgan's OQ3 is already a shipped test contract. |
| Decision verdict / normalized action / risk signal | **ACTAX** (policy-action-taxonomy) `RiskScore` + normalized action families; **gate**/**check** verdicts. These are *produced* today; the borrow is emitting *refs* to them, not re-deriving them in a span. |
| Findings → standard machine-readable export | **SARIFOUT** · **Complete (6/6)** — `--format sarif` already a deterministic export carrier; precedent that Anvil exports evidence in *open standard envelopes*. |
| Multi-backend "publish anywhere" exporter | **No — and deliberately so.** ADR-059 chose one sink and a local-first boundary. No module owns a pluggable destination matrix; introducing one is a new decision (§6). |
| Prompt mgmt / evals / datasets / playground | **No equivalent and out of scope** — product breadth Anvil does not pursue. |

Anvil's lane: **deterministic, evidence-producing governance of change at
save/commit/push time, with durable provenance on the witness chain/Kindling and
an *ephemeral* tracing pipe that correlates back to it.** Langfuse's lane: **an
LLM-engineering platform you deploy and inspect.** The only point of contact is
the wire (OTel) and the join key (`traceparent`) — both of which Anvil already
implements. The borrow is one *contract refinement* on a roadmap that exists, not
a new capability.

---

## 4. The borrows worth taking

Verdicts: **Use directly** (facts/shapes; clean-room) · **Adapt** · **Inspiration
only**.

### Borrow A — the governance-decision correlation span as a refs-only breadcrumb (the primitive · Adapt → an EXPORT contract refinement)

**This is the most valuable primitive, and the one Morgan is right about — once
reframed.** Emit an OTel-compatible span at the point of an Anvil decision that
makes the decision *joinable* to the buyer's existing LLM/tool trace, while the
evidence of record stays durable. Morgan's field list survives almost intact once
each field is classified by the three-pipe rule:

- **Correlation key (span identity / join) — keep as span identity:** `trace_id`,
  `span_id`, `parent_span_id`. This is the breadcrumb; `traceparent` is already
  ADR-035's named cross-pipe key.
- **Pointers (links to durable evidence, *not* inline values) — keep as refs:**
  `agent_session_ref`, `workflow_ref`, `anvil_decision_ref`, `policy_ref`,
  `normalized_action_ref`, `tool_call_ref`, `evidence_refs`, `risk_signal_refs`.
  These resolve back to Kindling / the witness chain — the span says *where* the
  proof is, never *is* the proof.
- **`policy_digest` — keep (already a hash, non-sensitive):** ties to
  `rules_sha` (MLP2-014); a digest, not a payload, so it rides safely.
- **Low-sensitivity routing attributes — keep as span attributes:** `verdict`,
  `latency_ms`, `correlation_status`, `otel_semconv_version`. Cheap, non-secret,
  the part a buyer actually filters on in their stack.
- **`exporter_ref` / `destination_ref` — keep, but constrained:** describe the
  *one decided* exporter/sink (ADR-059), **not** a pluggable matrix. Modelling
  arbitrary destinations is the ADR-059 collision (§6, Borrow-not-taken).

So the fixture Morgan asks for is *worth building* — an Anvil block correlating to
an upstream LLM/tool span with **no raw prompt/secret/tool material** — with the
field semantics corrected to **refs-not-payloads** and the exporter bound to the
ratified sink. It lands as an **EXPORT contract** (and a test fixture under the
EXPORT/`anvil-observability` boundary), not a new module.

### Borrow B — align attribute names with OpenTelemetry GenAI semantic conventions, namespaced `anvil.*` for governance-specific fields (framing · answers Morgan's OQ1)

Morgan's OQ1 ("own names first, or GenAI semconv from day one?") has a clean
answer in Anvil's existing convention: **align to OTel GenAI semconv where stable;
fall back to `anvil.<domain>.*` only for governance-specific fields.** This is
exactly the ADR-019 `anvil.flags.*` precedent and the
[namespace registry](../../docs/observability/namespace-registry.md) +
founder-PR-review gate (TRACE-001), and it directly mitigates TRACE R2
(`anvil.<domain>.*` fragmentation). Pin governance fields under a single
`anvil.governance.*` namespace; map `verdict`/`latency`/agent/tool fields to
GenAI semconv keys where they exist. Cite the standard, version it
(`otel_semconv_version`), and don't invent parallel names for things semconv
already names.

### Borrow C — redaction-before-export as a hard, tested invariant (concrete · Use directly — it's already a contract)

Morgan's OQ3 ("how much sensitive data to omit?") is **already decided and
testable**: the correlation span MUST pass through `anvil-observability`'s
redaction deny-list (TRACE-003), carry **zero** prompt/secret/tool payload, and
the exporter MUST NOT be able to bypass it — EXPORT-001 **V2** is exactly this
assertion. Make "refs-only, redaction-enforced, no raw prompt/tool material" a
test on the new fixture, not a guideline. The refs-only design (Borrow A) makes
this nearly free: there is no raw payload in the span to leak.

### Borrow D — the answer to "attributes vs linked artifacts" is the three-pipe rule (framing · answers Morgan's OQ2)

Morgan's OQ2 is the crux, and ADR-035 already answers it: **durable evidence is
linked (refs to Kindling/witness chain); only low-sensitivity routing fields ride
as span attributes.** This is the same record-once/render-many discipline the
RuleHub and DocGraph assessments landed for control-mapping and source-quality
fields — applied here to decision provenance. The witness chain stays the source
of truth; the span is a renderer/locator. Write this into the EXPORT contract so
the next reviewer cannot "just add the verdict body to the span to make it
self-contained."

### Borrow E — Langfuse as market validation of the OTel-GenAI wedge (Inspiration only)

A high-traction platform building its observability on OpenTelemetry GenAI
conventions is the strongest signal yet that "emit governance evidence the buyer's
stack can already read" has pull. Cite it as the **demand trigger** in EXPORT's
notes (alongside ADR-059's "first paying customer / production incident" gate) —
corroboration that lands the wedge with the buyer's tools, not new scope.

---

## 5. What NOT to borrow

| Item | Reason |
| ---- | ------ |
| Langfuse's codebase (web app + SDKs) | Wrong shape and wrong stack for a governance engine (Anvil core = Rust workspace + TS packages). MIT core is vendor-friendly, but the `ee/` folders are proprietary — and clean-room against the **open OTel GenAI semconv** is the right source anyway. No port, no vendor, no dependency. |
| Langfuse as a deployed platform / dependency | It is a deploy-and-inspect product at a layer Anvil does not operate; depending on it would be adopting an observability platform (scope exclusion #5) and a second source of truth. |
| Durable governance evidence inside the span | ADR-035 reject. Spans are ephemeral, sampled, retention-bounded; evidence lives on the witness chain / Kindling. The span carries refs (Borrow A/D). |
| `exporter_ref` / `destination_ref` as a **pluggable multi-backend matrix** | ADR-059 collision. The ratified architecture is a single Azure Monitor sink, operator-hosted, local-first, off by default. "Publish to Langfuse + Datadog + Grafana + Honeycomb" is a different posture requiring its own ADR — do not smuggle it in as a fixture field. (OTLP-neutral wire is fine; an exporter fleet is not.) |
| Prompt management / evals / datasets / playground | Product breadth; zero governance/provenance content. The "don't become Langfuse" line. |
| A standalone Anvil "inspect agent actions" dashboard | Scope exclusion #5, and self-defeating — the borrow exists to land evidence in the buyer's stack, not to build another dashboard. |
| Re-deriving `verdict` / `RiskScore` / normalized actions in the span | Already produced by ACTAX/gate; the span emits *refs*, not recomputed values. |

---

## 6. Risks of the proposed framing

- **Source-of-truth demotion (highest).** Implementing Morgan's fixture verbatim
  puts durable governance evidence in the ephemeral OTel pipe, creating a second,
  lossy, sampled "record" that contradicts the witness chain. Hard-state in any
  spec: **refs-only**, evidence stays on Kindling/witness chain, and cite ADR-035
  inline so the next reviewer cannot re-open it. Enforce with a fixture test
  (no payload fields; refs resolve).
- **Architecture drift past ADR-059.** `exporter_ref`/`destination_ref` invite a
  "publish anywhere" exporter fleet that the ratified sink decision explicitly
  declined. If multi-destination is genuinely wanted, that is a **new ADR**, not a
  beta fixture. Bound the beta to the decided single sink + the local file sink.
- **Dashboard creep.** "Inspect agent actions, policy gates, audit events" reads
  like an observability product (scope exclusion #5). It survives *only* as
  evidence emitted into the buyer's existing stack. Every span field must point at
  a real enforcement decision; if a field has no decision behind it, cut it.
- **Semconv churn.** OTel GenAI semantic conventions are still maturing; aligning
  early risks a rename. Mitigate by namespacing governance-specific fields under
  `anvil.governance.*`, mapping only stable semconv keys, and pinning
  `otel_semconv_version` (Borrow B; TRACE R2 namespace-registry gate).
- **Sizing inflation.** The nomination's **M / High** reads as greenfield. The
  capability is on the roadmap (TRACE shipped the plumbing; EXPORT owns the sink);
  the real increment — semconv-aligned, refs-only correlation-span contract + one
  corrected fixture — is **S**, and its impact is high *as a refinement of EXPORT*,
  not as a standalone initiative. Treating it as M risks claiming a wave slot it
  doesn't merit.
- **EXPORT is Draft on a timing gate, not a design gate.** ADR-059 settled the
  design; EXPORT stays Draft until a paying customer / production incident. The
  correlation-span contract can be *specified* now (cheap, durable) without
  pulling EXPORT execution forward.

---

## 7. Recommendation

**Decline Langfuse code/runtime adoption and dependency. Validate Morgan's
strategic read but reframe the fixture: the borrow is the *governance-decision
correlation span* as a refs-only OpenTelemetry breadcrumb (Borrow A), aligned to
OTel GenAI semantic conventions with `anvil.governance.*` for governance-specific
fields (Borrow B), redaction-enforced with zero raw prompt/tool material (Borrow
C, already EXPORT-001 V2), with durable evidence linked not inlined per the
three-pipe rule (Borrow D). This is a contract refinement of the existing EXPORT
module, governed by ADR-035 and ADR-059 — not a new capability and not a
dependency. Reject the evidence-in-span and multi-backend-export parts. Cite
Langfuse as market validation of the OTel-GenAI wedge. No APS module filed in this
pass; suggested CIB filings in §7.3.**

### Most valuable primitive

The **governance-decision correlation span (refs-only)** — Anvil's verdict made
*joinable* to the buyer's LLM/tool trace via the `traceparent` cross-pipe key,
while the witness chain stays the source of truth. It is high-leverage precisely
because it is built almost entirely from parts Anvil already ships (the
`anvil-observability` crate, W3C `traceparent`, the redaction layer, the witness
chain, the namespace registry) — repackaged so governance proof is correlatable
*where the buyer already debugs and audits*.

### Customer impact

**High on adoption/audit confidence, low on net-new engineering.** A platform team
on Datadog/Grafana/Honeycomb can pivot from an LLM/agent span to "what did Anvil
decide here, and where is the proof" without adopting an Anvil dashboard. It turns
Anvil from a separate gate into evidence that lives where incidents are debugged
and compliance is demonstrated. Because it reuses existing collectors and the
decided sink, the cost is a span contract + semconv mapping + a fixture, not a new
exporter fleet — and it explicitly avoids forcing buyers into yet another tool.

### Acquisition strategy

**Inspiration only / clean-room, no dependency.** Reimplement the correlation-span
contract in the Rust workspace over the existing `anvil-observability` crate and
the witness chain; align attribute names to the **open** OpenTelemetry GenAI
semantic conventions (the real standard to clone), not to Langfuse code. Cite
Langfuse as parallel evolution and market validation of the wedge.

### Decision-ladder placement

Per the assessment brief (ignore / track / document / specify / plan / prototype /
depend):

| Slice | Placement | Why |
| ----- | --------- | --- |
| Langfuse the platform (observability/evals/prompt mgmt/datasets) | **Ignore / decline** | Product breadth; scope exclusion #5; no governance content. |
| Governance-decision correlation span, refs-only (Borrow A) | **Specify** (EXPORT contract + corrected fixture) | In scope as provenance; highest value, lowest net-new; builds on TRACE + ADR-035 + ADR-059. |
| OTel GenAI semconv alignment (Borrow B) | **Document → Specify** (EXPORT + namespace registry) | Answers OQ1; pins names before they proliferate (TRACE R2). |
| Redaction-before-export invariant (Borrow C) | **Document now** (ratify; already EXPORT-001 V2) | Answers OQ3; make it a fixture test. |
| Attributes-vs-links answer (Borrow D) | **Document now** (ratify ADR-035 as the answer to OQ2) | Zero-code; prevents evidence-in-span drift. |
| Multi-backend "publish anywhere" export | **Track behind an ADR gate** | ADR-059 collision; needs its own decision, not a fixture field. |
| Dependency on Langfuse | **No** | Platform at a layer Anvil does not occupy; second source of truth. |

### 7.1 APS modules to update (exact list)

| Module | ID · Status | Update |
| ------ | ----------- | ------ |
| observability-export | **EXPORT** · Draft 0/1 | Record the governance-decision correlation span as an EXPORT contract refinement (refs-only; bound to the ADR-059 sink). Add an Open Question: align to OTel GenAI semconv, namespaced `anvil.governance.*` (Borrow B). Note Langfuse as the demand-trigger corroboration alongside the existing "first paying customer / incident" gate. No status change (Draft is a timing gate). |
| tracing-foundation | **TRACE** · In Progress 2/4 | Cross-reference: the correlation span consumes `bind_traceparent_to_current_span` (TRACE-004) and the redaction layer (TRACE-003); any new `anvil.governance.*` attributes register in the [namespace registry](../../docs/observability/namespace-registry.md) per TRACE R2. |
| (ADR) three-pipe rule | **ADR-035** · Accepted | Cite as the binding answer to OQ2 (attributes vs links) and the prohibition on evidence-in-span. No change; reference. |
| (ADR) production tracing sink | **ADR-059** · Accepted | Cite as the binding constraint on `exporter_ref`/`destination_ref`; any multi-backend export needs a *new* ADR. No change; reference. |
| witness-chain / MLP2 | (owned) | `policy_digest` rides as a ref to `rules_sha` (MLP2-014); the span links to witness/Kindling records, does not duplicate them. |

### 7.2 Gaps identified

1. **No governance-decision correlation-span contract.** TRACE ships
   `traceparent` binding and a local file sink; EXPORT owns the production sink;
   nothing yet defines the *governance* span shape (refs-only) that joins an Anvil
   decision to an upstream LLM/tool span. This is the work — a contract + fixture,
   not a module.
2. **OTel GenAI semconv mapping is unspecified.** The namespace registry covers
   `anvil.flags.*` / `kindling.*` / `anvil.rtai.*`; there is no
   `anvil.governance.*` block nor a GenAI-semconv crosswalk. Borrow B fills it.
3. **Multi-destination export has no decision.** ADR-059 chose one sink;
   "publish to the buyer's arbitrary backend" is undecided. If demand appears, it
   needs its own ADR (do not let a fixture field pre-empt it).

### 7.3 Suggested CIB filings (next-available IDs — allocate at filing time)

Next-available is **CIB-048** (CIB header reads 29/47; max id CIB-047). Not
hard-coded here to avoid a numbering race — allocate when filed under
[`continuous-improvement-backlog`](../modules/continuous-improvement-backlog.aps.md):

- **CIB-(next, spec):** Specify the **governance-decision correlation span**
  (working name `AnvilGovernanceSpan`) as an EXPORT contract — refs-only fields
  resolving to Kindling/witness records, low-sensitivity routing attributes only,
  bound to the ADR-059 sink. Acceptance: a fixture where an Anvil **block**
  correlates to an upstream LLM/tool span via `traceparent` with **no raw
  prompt/secret/tool material**; a test asserting refs-not-payloads and redaction
  (mirrors EXPORT-001 V2). Cite ADR-035 inline. Builds on TRACE-004.
- **CIB-(next+1, docs):** Add an `anvil.governance.*` block + an **OTel GenAI
  semconv crosswalk** to the namespace registry (Borrow B); pin
  `otel_semconv_version`; route new names through the founder-PR-review gate
  (TRACE R2).
- **CIB-(next+2, docs):** Ratify the **attributes-vs-links** answer (Borrow D, =
  ADR-035) and the **redaction-before-export, refs-only** invariant (Borrow C) in
  the EXPORT spec notes, with a one-line "multi-backend export requires its own
  ADR" guard (the ADR-059 boundary).

---

## 8. Open questions (defer to follow-up specs)

- **Own span names vs OTel GenAI semconv (Morgan OQ1):** lean **align to semconv
  where stable, `anvil.governance.*` otherwise** — but the semconv is still
  evolving, so pin `otel_semconv_version` and treat the mapping as additive. Which
  exact GenAI semconv keys map to `verdict` / agent / tool fields needs a crosswalk
  pass when the contract is specified.
- **Attributes vs linked artifacts (Morgan OQ2):** answered by ADR-035 (links for
  durable evidence; attributes only for low-sensitivity routing). Confirm each of
  Morgan's fields lands on the right side in the contract (the §4 Borrow-A
  classification is the starting allocation).
- **Sensitive data omission (Morgan OQ3):** answered by refs-only + TRACE-003
  redaction + EXPORT-001 V2. Open: do `tool_call_ref` / `normalized_action_ref`
  ever need a redacted *summary* attribute for usability, or is a pure ref enough?
  Default to pure ref; add a redacted summary only if a buyer needs in-stack
  filtering without a round-trip.
- **Does any of this pull EXPORT off its timing gate?** No — the contract +
  fixture + semconv crosswalk are specifiable now without wiring the production
  exporter (ADR-059's "first paying customer / incident" gate still governs
  execution). Confirm the spec lands as design, not execution.
- **Is there ever a multi-backend demand trigger** (a buyer who runs Honeycomb,
  not Azure) that promotes a pluggable destination ADR? If so, that is the moment
  to revisit `destination_ref` as more than a description of the single sink —
  and it gets its own scope-guard + ADR pass.

---

## 9. One-line summary

> Decline the Langfuse platform and a dependency (product breadth at a layer Anvil
> doesn't occupy; scope exclusion #5). Validate Morgan's strategic read — emit
> governance evidence into the buyer's own observability stack — but reframe the
> fixture: the borrow is a **refs-only governance-decision correlation span**
> (`traceparent` join key + pointers to Kindling/witness evidence, low-sensitivity
> routing attributes only), aligned to **OTel GenAI semconv** (`anvil.governance.*`
> for the rest), redaction-enforced, with evidence **linked not inlined**. It is a
> contract refinement of the existing **EXPORT** module under **ADR-035**
> (three-pipe rule) and **ADR-059** (single Azure sink) — not net-new, sized **S**
> not M. Reject evidence-in-span and multi-backend export; the latter needs its own
> ADR. No dependency; clean-room against the open OTel GenAI conventions; cite
> Langfuse as market validation of the wedge.
