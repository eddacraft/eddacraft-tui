# RuleHub — Borrow Assessment

**Date:** 2026-06-03
**Status:** Brainstorm — assessment of RuleHub as a borrow candidate (nominated
by Morgan). **Outcome: decline code/runtime adoption and decline dependency
(Python + Rego, MIT, 5★ / 71 commits / no release / single-org, multi-repo with
a Helm chart + Backstage plugin — wrong stack and wrong surfaces). Validate
Morgan's *primitive* — control-framework mapping that stays tied to enforcement
evidence — but challenge its novelty and its unit. It is not a new prompt: it is
the fourth in-repo sighting of a primitive Anvil has tracked since 2026-03-08
(`verifywise` → CEWS/COMPLY/TRUST), with `asqav-sdk` ("Asqav") and
`agent-audit-trail-mcp` already capturing the EU-AI-Act-citation answer
(2026-05-13). Anvil already owns the crosswalk across three modules — CPACKS
`controlMappings`, COMPLY `ComplianceMapper` + `anvil compliance map`, CEWS
`ControlEvidenceMap` — so the borrow is not a feature but a *seam*: the
control-mapping field must travel pack → decision → evidence record → export,
recorded once and rendered many. The one genuinely missing thing RuleHub
surfaces is that no decision names which of those three modules owns the
canonical field — a source-of-truth gap, the same shape DocGraph surfaced before
ADR-062. No APS module filed; suggested CIB filings (next-available from
CIB-048) in §7.3. ISO 42001 is the one net-new framework — track only. Cite
RuleHub as parallel evolution + market corroboration; no dependency.**
**Source:** https://github.com/rulehub/rulehub (MIT, Python 53.0% / Open Policy
Agent 40.0% / Makefile 4.6% / Shell 2.1%, 5★ / 1 fork / 0 issues / 71 commits /
no published release; multi-repo `rulehub` org: core engine + Helm charts +
Backstage plugin. Facts read from the public repo landing 2026-06-03.)

---

## 0. What this document is

A borrow assessment of an external repo, in the format of
[`2026-06-03-meho-borrow-assessment.md`](./2026-06-03-meho-borrow-assessment.md),
[`2026-05-31-docgraph-borrow-assessment.md`](./2026-05-31-docgraph-borrow-assessment.md),
[`2026-05-24-drako-borrow-assessment.md`](./2026-05-24-drako-borrow-assessment.md),
and [`2026-05-22-proxilion-pic-borrow-assessment.md`](./2026-05-22-proxilion-pic-borrow-assessment.md).
The goal is **not** to adopt RuleHub but to mine it for reusable governance
ideas, scope-guard each one, map it onto exact APS modules, and name the gaps.
Facts were read from the public repo landing on 2026-06-03 and cross-checked
against `plans/modules/*`, `docs/vision/anvil-scope-guard.md`,
`docs/strategy/borrow-adopt-candidates.md`, and the prior assessments above.

---

## 1. Nomination summary

Morgan nominated RuleHub on the framing that the sharp Anvil borrow is
**compliance mapping that stays tied to enforcement evidence**: a policy pack
should be able to say which external control, risk, or framework requirement a
decision supports, so Anvil output becomes *audit evidence* rather than only
developer feedback. He flags RuleHub's OPA/Kyverno engines, signed bundles,
evidence trails, and mappings to EU AI Act / NIST AI RMF / ISO 42001; notes the
overlap with Microsoft AGT and Asqav; calls RuleHub "the cleanest prompt for
framework crosswalks"; and asks two open questions: **which framework should
Anvil map first, and should mapping appear in CLI output, audit exports, or
both?**

| Project | What it is | Stack | Maturity |
| ------- | ---------- | ----- | -------- |
| RuleHub | A "developer-first Policy-as-Code framework unifying safety, security, and compliance for AI systems." Encodes requirements as reusable **OPA (Rego) + Kyverno** policies; each policy carries YAML control-mapping metadata (`id: CM-001`, `title:`, `controls: ['CIS-1.1']`); maps to **EU AI Act, NIST AI RMF, ISO 42001**; ships **SBOM/AIBOM generation, cosign signatures, provenance**; emits **Prometheus / OpenTelemetry metrics and evidence trails**; `make`-driven dev workflow. | Python 53% + OPA/Rego 40% + Makefile/Shell | MIT · **5★** / 1 fork / 0 issues · 71 commits · **no published release** · multi-repo `rulehub` org (core engine + Helm charts + Backstage plugin) · single-org, low external adoption |

**Operational footprint:** RuleHub is a policy *library + framework* with CI
hooks, a Helm chart for deployment, and a Backstage plugin for a developer
portal. The compliance value is the **control-mapping metadata on each policy**
plus the **provenance/evidence export**, not the deployment surfaces.

---

## 2. Scope-guard test

Per [`docs/vision/anvil-scope-guard.md`](../../docs/vision/anvil-scope-guard.md),
Anvil operates at the **moment of change creation**, enforces deterministic
policy against artefacts, and captures **provenance for policy decisions**. The
four borderline questions: (1) increases prevention, (2) operates before/at
execution time, (3) strengthens deterministic control, (4) enforces rather than
only informs.

The central move of this assessment is to be honest about **which pillar the
borrow lands on**. Control-framework mapping does *not* add prevention,
execution-time control, or determinism to a verdict — the verdict already has
those. It adds **provenance and traceability** (In-Scope capability #5: "linking
actions to policies and rules"). That distinction is the whole ballgame:

| RuleHub capability | Decisive question | Scope-guard read |
| ------------------ | ----------------- | ---------------- |
| Control-mapping **as metadata on an enforcement decision** (`controls: [...]` rides a Rego verdict Anvil already produces, exported as evidence) | #4 — does it enforce, or inform tied to enforcement? | **In (the borrow).** It is provenance attached to a decision that *itself* enforces. This is exactly what "stays tied to enforcement evidence" means, and it is In-Scope pillar #5. |
| Control-mapping **as a standalone compliance coverage report / posture dashboard** detached from any decision | #4 — only informs? | **Out.** A retrieval/marketing surface, not a trust layer. This is the precise line the Drako assessment drew ("compliance-mapping is a marketing surface, not enforcement … a separate docs layer, not the engine", 2026-05-24 §5). |
| OPA/Rego as the policy engine | #3 — already Anvil's engine | **In, but already owned.** ADR-006 (hybrid-dc-opa), ADR-022 (opa-agent-orchestration), ADR-040 (regorus). Parallel evolution. |
| Kyverno (Kubernetes-admission policy) | #2/#3 — at change creation, deterministic? | **Out (today).** Anvil has no k8s/Helm surface; the MEHO assessment (2026-06-03 §7.2) already filed that as a gap. RuleHub reinforces the gap; it does not change the verdict. |
| Signed bundles (cosign / SBOM / AIBOM / provenance) on the *policy bundle* | #5 — provenance of the enforcer | **In, but already owned.** SCA (Proposed) owns release SBOM+SLSA; the witness chain already threads `rules_sha` per line (MLP2-014); MEHO Borrow-A names `policy_bundle_digest`. Parallel evolution. |
| Prometheus / OpenTelemetry metrics + evidence trails (runtime telemetry) | #1/#4 — feeds enforcement? | **Out as a borrow.** Observability that does not feed creation-time enforcement (scope-guard exclusion #5). The OTLP angle is already tracked additive/off-by-default via the `langfuse` candidate (2026-05-13). |
| Helm chart + Backstage developer-portal plugin | #2 — Anvil's surface? | **Out.** Deployment topology + portal UI = GATE/`infra/` territory; same disposition as MEHO's chart (2026-06-03 §5). |

**Verdict:** the borrow is **In** only as *provenance metadata on an enforcement
decision* — and only for as long as it stays bound to that decision. The moment
the crosswalk detaches into a standalone "are we compliant?" report, it becomes
the Drako marketing-surface reject. Morgan's phrase "stays tied to enforcement
evidence" is not decoration — it is the exact scope-guard boundary that lets
this borrow in where Drako's standalone article-mapping stayed out. His instinct
is right; the framing needs this boundary stated as a hard rule so the crosswalk
never grows into a compliance-dashboard product (§6).

---

## 3. Overlap with existing Anvil work

This is the crux of the challenge to Morgan. The "framework crosswalk" is not a
gap — Anvil has already scoped it across **three** modules, and has tracked the
identical external primitive **three** prior times.

### 3.1 The crosswalk is already a three-module design

| RuleHub capability | Anvil equivalent (status) |
| ------------------ | ------------------------- |
| `controls: [...]` mapping metadata authored **on the policy** | **CPACKS** (compliance-policy-packs) · **Draft (high)** — CPACKS-002 already specs a `controlMappings` array linking policy IDs → framework control IDs/titles/categories, and ships packs for OWASP, SOC 2, ISO 27001, GDPR, **NIST AI RMF, and EU AI Act** (the exact AI frameworks Morgan names). |
| Policy-to-control **resolution + coverage** | **COMPLY** (compliance-reporting) · **Draft (medium)** — already exposes `ComplianceFrameworkRegistry`, `ComplianceMapper`, `EvidenceCollector`, built-in SOC 2 + ISO 27001, custom-framework YAML, and **`anvil compliance map` / `status` / `report`** CLI verbs. This *is* the crosswalk module. |
| Control-to-**evidence** linkage + audit export | **CEWS** (compliance-evidence-workspace) · **Draft** (demoted Ready→Draft 2026-04-26, blocked behind COMPLY-004) — `ControlEvidenceMap`, `EvidenceRecord`, `ComplianceWorkspaceReport`; CEWS-002 links policy/eval outcomes → evidence + controls; CEWS-004 export packs. |
| Buyer-facing trust/evidence publishing | **TRUST** (trust-center-automation) · **Ready** — assembles publishable artifacts from "policy, eval, and compliance sources" with freshness/ownership. |
| Signed policy-bundle provenance feeding a verdict | **MLP2** witness `rules_sha` (MLP2-014) + **SCA** (Proposed) SBOM/SLSA + MEHO Borrow-A `policy_bundle_digest`. |
| Findings → standard machine-readable export | **SARIFOUT** · **Complete (6/6)** — `--format sarif` shipped on `anvil check`/`gate`/`audit` (promoted from Drako CIB-014). A SARIF `result.properties` bag is the obvious, already-shipped carrier for `controls: [...]`. |
| AI-pack signals (trust score, capability, audit chain) the AI crosswalks depend on | **AGOV** (agent-governance-patterns) · **Draft** — CPACKS-051/061/062/063 already declare AGOV-001/006/007 dependencies. |

Anvil's lane: **deterministic, evidence-producing governance of change at
save/commit/push time, exported as audit evidence.** RuleHub's compliance slice
sits squarely in it — *and is already three modules deep in the plan.* What is
**not** yet decided is the seam between those three modules (§7.2 gap 1).

### 3.2 RuleHub is the fourth sighting, not a new prompt

`docs/strategy/borrow-adopt-candidates.md` already tracks the same primitive
from three other repos, two of them the very projects Morgan names as overlaps:

- **`bluewave-labs/verifywise`** (2026-03-08): "compliance crosswalk +
  evidence-linked reporting UX" → copy-ux, **High** impact, aps link
  **CEWS / COMPLY / TRUST**. This is Morgan's exact primitive, tracked ~3 months
  ago against the exact three modules.
- **`jagmarques/asqav-sdk`** (2026-05-13) — the **"Asqav"** in Morgan's blurb:
  "EU AI Act article-mapping baked into report output (named article citations
  in compliance exports)." Architecture note already recorded: "EU AI Act
  article mapping is a metadata annotation layer, not a schema redesign."
- **`AiAgentKarl/agent-audit-trail-mcp`** (2026-05-13): "EU AI Act Article 12
  compliance framing as a named output target … naming specific Act articles in
  compliance exports is a procurement checklist item." Architecture note already
  records the answer to Morgan's Q2: **"Add EU AI Act article citation as a
  metadata annotation on finding exports — do not fork the audit trail store.
  Reuse existing hash-chain schema; define article citations as an optional
  structured field so it degrades gracefully for non-EU buyers."**

The **"Microsoft AGT"** Morgan references appears in the same tracker as the
agent-identity model Anvil plans SPIFFE/SVID compatibility against (Symbiont
entry, 2026-05-13) — adjacent identity lane, not the crosswalk.

So RuleHub is **corroborating market signal** (four independent projects
converging on "control mapping as exported evidence" is strong demand evidence)
but **not a new capability prompt**. Its unique contribution is being the
*cleanest reference schema* for the mapping field (`id` + `title` + `controls:
[...]`), not a new idea.

---

## 4. The borrows worth taking

Verdicts: **Use directly** (field shape / answer is a fact; clean-room) ·
**Adapt** · **Inspiration only**.

### Borrow A — the evidence-bound control-mapping field, recorded once and rendered many (framing · the actual primitive)

The most valuable primitive is **not** the crosswalk catalogue (COMPLY already
specs it) and **not** the engine (OPA, already owned). It is the **seam**: a
single control-mapping field that travels

> **policy pack (CPACKS authors `controls`) → decision (mapping rides the Rego
> verdict at eval time) → evidence record (CEWS `ControlEvidenceMap`/
> `EvidenceRecord` records it) → export (rendered many ways)**

with **one source of truth and many renderers**. This is the same discipline as
DocGraph Borrow-A (source-quality fields → `EvidenceRecord`) and MEHO Borrow-A
(provenance block → `EvidenceRecord`), applied to *control provenance*. RuleHub
makes the field concrete — author it on the policy (CPACKS-002), resolve it in
COMPLY's `ComplianceMapper`, record it on CEWS, render it in CLI/SARIF/audit
export. The borrow is the *contract*, not the table.

### Borrow B — the "where does it appear?" answer: record-on-evidence always; CLI advisory; export structured-optional; never a gate (framing · answers Morgan's Q2)

Morgan's Q2 ("CLI, audit exports, or both?") is answered by composing the
scope-guard with the already-recorded `agent-audit-trail-mcp` note:

1. **Recorded on the evidence record always** (CEWS) — it is provenance; it is
   not conditional on a surface.
2. **CLI: advisory context only.** Show "this finding supports SOC 2 CC8.1" as
   information beside the verdict — **never** a "✅ compliant" status. A green
   compliance verdict is exactly the *false-assurance* risk both CPACKS ("False
   sense of compliance from partial coverage") and COMPLY ("Mapping inaccuracy
   creates false assurance") already flag, and it would be the Drako
   marketing-surface reject wearing a CLI badge. Warnings-over-blocks (ADR-002):
   compliance mapping must never gate.
3. **Audit export / SARIF: a first-class but optional structured field** that
   degrades gracefully for non-regulated buyers (the agent-audit-trail-mcp
   contract). Now that **SARIFOUT is Complete**, `result.properties.controls`
   is a shipped, deterministic carrier — no new export surface required.

So the answer is **both**, but emitted from one source (CEWS), advisory in the
CLI, structured-optional in exports, and **never a pass/fail gate**.

### Borrow C — the YAML mapping-field shape, as a clean-room reference (concrete · Use directly as spec input)

RuleHub's `id: CM-001 / title / controls: ['CIS-1.1']` is a clean, minimal
shape. Use it as a **reference** when CPACKS-002 / COMPLY-001 finalise the
`controlMappings` schema (field names, `framework` + `controlId` + `controlTitle`
+ `category`, version pinning). Clean-room; cite RuleHub as parallel evolution.
Do **not** import RuleHub's Rego or YAML files.

### Borrow D — ISO 42001 as a candidate framework (concrete · Track only)

ISO 42001 (AI Management System) is the **one framework RuleHub names that
Anvil's CPACKS does not cover** (CPACKS ships OWASP/SOC2/ISO27001/GDPR/NIST AI
RMF/EU AI Act). It is the only net-new content here. Per CPACKS D-CPACKS-001
("prove the pattern first") and its "Future Packs" discipline, this is a
**tracked candidate**, demand-gated like the other future packs — not a planned
work item today.

### Borrow E — four-project convergence as demand evidence (Inspiration only)

verifywise + asqav + agent-audit-trail-mcp + RuleHub all pointing at
"control mapping as exported evidence" is the strongest market signal yet for
the CEWS/COMPLY/TRUST constellation. Cite it as the **demand trigger** in those
modules' notes, alongside the GATE "enterprise prospect" gate — corroboration,
not new scope.

---

## 5. What NOT to borrow

| Item | Reason |
| ---- | ------ |
| RuleHub's Python/Rego codebase | Wrong stack (Anvil = Rust workspace + TS). MIT is vendor-friendly but clean-room is preferable; no port/vendor. 5★ / 71 commits / no release / single-org = a dependency would be a de-facto fork. |
| RuleHub's Rego policy library + control-mapping YAML files | CPACKS already owns a language-agnostic Rego pack surface mapped to the same frameworks. Importing RuleHub's rules forks the threat model and the maintenance burden. Borrow the *field shape* (Borrow C), not the files. |
| Kyverno integration | Anvil has no k8s/Helm surface; this is the MEHO §7.2 `surface-kubernetes` gap, demand-gated. Do not add a second policy engine to chase Kyverno. |
| Helm chart + Backstage portal plugin | Deployment topology + developer-portal UI. Anvil is a local-first CLI + daemon; portals/charts are GATE/`infra/` and governed separately (same disposition as MEHO's chart, 2026-06-03 §5). The multi-repo split (engine / Helm / Backstage) is a **negative** example — do not fork the crosswalk into a portal surface. |
| Prometheus / OpenTelemetry evidence metrics as a borrow | Observability that does not feed creation-time enforcement (scope exclusion #5). OTLP is already tracked additive/off-by-default via `langfuse`; `observability-export`/`observability-foundation` own Anvil's own telemetry. |
| cosign / SBOM / AIBOM signing as a new initiative | Already owned by SCA (Proposed) + witness `rules_sha` (MLP2-014) + MEHO Borrow-A. AIBOM overlaps the `anvil bom` triage (Drako CIB-015). Parallel evolution; do not expand. |
| A compliance **posture score** / "% compliant" headline | COMPLY-005 sketches a posture score, but surfacing it as a top-line "compliant" number is the false-assurance trap (§6). If kept, it stays an internal coverage metric with prominent gap labelling — never a buyer-facing pass/fail. |
| Standalone "compliance coverage report" detached from decisions | The scope-guard Out case (§2) and the Drako marketing-surface reject. The crosswalk only earns its place bound to enforcement evidence. |

---

## 6. Risks of the proposed framing

- **Crosswalk drifts into a compliance-dashboard product (highest).** "Anvil
  output becomes audit evidence" is one sentence away from "Anvil tells you if
  you're compliant." The first is provenance (In); the second is a GRC product
  (Out, and a legal-liability surface — CPACKS already excludes "legal advice").
  Hard-state Borrow-B's "advisory in CLI, never a gate, no '✅ compliant'
  verdict" in any spec, or the next reviewer ships a compliance badge.
- **Three modules, no source of truth.** CPACKS (`controlMappings`), COMPLY
  (`ComplianceMapper`/`ComplianceFrameworkRegistry`), and CEWS
  (`ControlEvidenceMap`) each define a piece of "control mapping" with no
  decision saying who is canonical and how the field travels. The
  borrow-candidates doc's own cross-cutting rule already warns: *"single source
  of truth per concern: Evidence truth = compliance evidence workspace"* and
  *"keep OPA policy IDs and detector IDs separate; bridge them via an explicit
  mapping table."* Without a seam decision this falls between modules — the same
  failure mode DocGraph surfaced before ADR-062 (§7.2 gap 1).
- **Leading with the AI frameworks for marketing pull.** EU AI Act / NIST AI RMF
  are the headline frameworks, but their CPACKS packs are **low/medium
  confidence** and depend on AGOV-001/006/007 signals that are **not built**
  (AGOV is Draft). A first crosswalk on EU AI Act would be mapping to controls
  Anvil cannot yet deterministically evidence — aspirational coverage, exactly
  the false-assurance risk. Lead instead with controls Anvil **already enforces**
  (§8).
- **Field-name collisions.** `control`, `mapping`, `evidence`, `framework`
  recur across CPACKS/COMPLY/CEWS/TRUST. Namespace per the DocGraph §6 /
  ADR-058 discipline (`control_mapping_ref`, not `mapping`; bridge policy IDs ↔
  control IDs via an explicit table, not by overloading either).
- **Maturity / dependency temptation.** RuleHub is more polished than Drako/PIC
  on the OPA front, which makes "just use their packs" tempting — but no
  release, 5★, single-org, and a different stack mean a dependency is a fork.
  Engineering polish ≠ adoption maturity (the MEHO §6 caution).
- **Re-litigating Drako.** The Drako assessment parked "EU AI Act mapping =
  marketing surface." This assessment does **not** overturn that — it draws the
  boundary Drako didn't have language for (mapping bound to a decision = In;
  detached = Out). State the reconciliation explicitly so the next reviewer
  doesn't read CPACKS/CEWS as contradicting Drako.

---

## 7. Recommendation

**Decline RuleHub code/runtime adoption and dependency. Validate Morgan's
primitive but reframe it: the borrow is the evidence-bound control-mapping
*seam* (pack → decision → evidence → export, record-once/render-many), which
Anvil already owns across CPACKS/COMPLY/CEWS — not a new feature. The one
genuinely missing thing is a source-of-truth decision for that field across the
three modules; that is worth a documented seam contract (candidate ADR). Adopt
Borrow-B's "advisory CLI, structured-optional export, never a gate" as the
standing answer to Q2. Track ISO 42001 as a future framework and RuleHub as the
fourth corroborating market signal. Cite as parallel evolution; clean-room; no
APS module filed in this pass.**

Decision-ladder placement (per the brief: ignore / track / document / specify /
plan / prototype / depend):

| Slice | Placement | Why |
| ----- | --------- | --- |
| RuleHub product (engine + Helm + Backstage) | **Ignore / decline** | Wrong stack + wrong surfaces; single-org, no release. |
| Evidence-bound control-mapping primitive (Borrow A) | **Already Planned → Document & converge** | Lives across CPACKS-002 / COMPLY-001..003 / CEWS-001..002. No new module; record the seam + converge the three. |
| Source-of-truth + field-travel seam across the 3 modules | **Specify** (candidate ADR) | The one real gap (§7.2.1). Mirrors DocGraph → ADR-062. |
| "Where it appears" answer (Borrow B) | **Document now** (ratify in CEWS/COMPLY/CLI/SARIF notes) | ~80% already recorded via agent-audit-trail-mcp; ratify + add the never-a-gate rule. |
| Mapping-field YAML shape (Borrow C) | **Document** (reference input to CPACKS-002 / COMPLY-001) | Clean-room reference; cite as parallel evolution. |
| ISO 42001 pack (Borrow D) | **Track** | Net-new framework; demand-gated like CPACKS Future Packs. |
| OPA/Kyverno, signed bundles, OTel metrics, Helm/Backstage | **Ignore / already-owned / parallel evolution** | OPA owned; SCA/witness own signing; langfuse owns OTLP; charts/portals are GATE/infra. |
| Dependency on RuleHub | **No** | De-facto fork at a layer + stack Anvil does not share. |

### 7.1 APS modules to update (exact list)

| Module | ID · Status | Update |
| ------ | ----------- | ------ |
| compliance-policy-packs | **CPACKS** · Draft (high) | Note RuleHub's `controls: [...]` shape as reference input to the CPACKS-002 `controlMappings` schema; cite as parallel evolution. No status change. |
| compliance-reporting | **COMPLY** · Draft | Confirm `ComplianceMapper` + `ComplianceFrameworkRegistry` + `anvil compliance map` are the canonical resolver/registry/CLI for the crosswalk; record the source-of-truth seam (CPACKS authors → COMPLY resolves → CEWS records). Carry the never-a-gate / suggested-mapping discipline (already in COMPLY risks). |
| compliance-evidence-workspace | **CEWS** · Draft (blocked behind COMPLY-004) | `ControlEvidenceMap`/`EvidenceRecord` is the canonical *record* of the resolved mapping (Borrow A); CEWS-004 export packs serialize it. Note it is downstream of COMPLY — same staging caveat as the DocGraph borrow. |
| trust-center-automation | **TRUST** · Ready | TRUST artifacts already assemble from "policy, eval, and compliance sources" — confirm the control-mapping field flows through to buyer-facing trust output as evidence, not as a compliance score. |
| sarif-output | **SARIFOUT** · Complete | Record that `result.properties.controls` is the shipped export carrier for the mapping field (no new surface needed); future additive-only. |
| agent-governance-patterns | **AGOV** · Draft | Note that the AI-framework crosswalks (CPACKS-051/061/062/063) depend on AGOV-001/006/007 signals — so the *first* crosswalk should not be an AI framework (§8). |
| supply-chain-attestation | **SCA** · Proposed | Cross-ref: RuleHub's cosign/SBOM/AIBOM is parallel evolution; signed-bundle-digest-as-evidence already covered by SCA + witness `rules_sha` (MLP2-014) + MEHO Borrow-A. No promotion. |

### 7.2 Gaps identified

1. **No source-of-truth decision for the control-mapping field.** CPACKS,
   COMPLY, and CEWS each define a piece; nothing names the canonical owner or
   the field's path between them. This is the real gap RuleHub surfaces — and
   the one thing worth specifying (candidate ADR: *control mapping authored in
   CPACKS, resolved by COMPLY's mapper, recorded on CEWS, rendered many; policy
   IDs ↔ control IDs bridged by an explicit table, never overloaded*).
2. **CEWS is blocked behind COMPLY-004** and both carry post-Rust path debt
   (TS-tree scopes). The borrow is downstream work; staged, not immediately
   executable — same caveat as DocGraph/MEHO.
3. **No ISO 42001 pack** (CPACKS stops at EU AI Act / NIST AI RMF for AI). The
   one net-new framework; track only.
4. **The "never a compliance gate / no ✅-compliant verdict" rule is not written
   down** as a hard constraint anywhere — only implied by COMPLY/CPACKS risk
   rows. Borrow B should make it explicit before any CLI surface ships.
5. **No `surface-kubernetes` / Kyverno surface** (cross-ref MEHO §7.2). RuleHub
   reinforces the gap; demand-gated, no action.

### 7.3 Suggested CIB filings (next-available IDs — allocate at filing time)

Next-available is **CIB-048** (CIB header reads 29/47; max id CIB-047). Not
hard-coded here to avoid a numbering race — allocate when filed under
[`continuous-improvement-backlog`](../modules/continuous-improvement-backlog.aps.md):

- **CIB-(next, docs/ADR):** Write the control-mapping **seam contract** —
  CPACKS authors `controls` on the policy → COMPLY `ComplianceMapper` resolves
  coverage → CEWS `ControlEvidenceMap` records it → CLI (advisory) + SARIF
  (`properties.controls`) + audit export render it. Names the source of truth
  and the policy-ID ↔ control-ID bridge table. Candidate ADR (§7.2.1).
- **CIB-(next+1, docs):** Ratify Borrow B as the standing answer to "where does
  mapping appear" — recorded-on-evidence always, advisory in CLI, optional
  structured field in exports, **never a gate / no compliant verdict** — into
  the CEWS/COMPLY/SARIFOUT spec notes. Cross-ref `agent-audit-trail-mcp` +
  `asqav-sdk` notes.
- **CIB-(next+2, track):** Triage ISO 42001 as a candidate CPACKS framework
  (Borrow D) — track-only; demand-gated like the other Future Packs.

---

## 8. Open questions (defer to follow-up specs)

- **Which framework first?** Challenge the marketing instinct to lead with EU AI
  Act. The first crosswalk should ride controls Anvil **already deterministically
  enforces** so the evidence is real, not aspirational: **SOC 2 CC8**
  (change-management — PR review, coverage gates; CPACKS-022) and **OWASP A02/A03**
  (crypto/injection; CPACKS-011/013) are backed by signals Anvil produces today.
  The AI frameworks (NIST AI RMF / EU AI Act) depend on unbuilt AGOV signals
  (§7.1) and should follow, not lead.
- Does COMPLY's `ComplianceMapper`/`ComplianceFrameworkRegistry` or CEWS's
  `ControlEvidenceMap` hold the *canonical* mapping, with CPACKS authoring it on
  the policy? (The §7.2.1 source-of-truth question — pin it in an ADR, don't let
  each module invent its own.)
- Does the witness chain's `rules_sha` (MLP2-014) already give a signed
  policy-bundle reference, so the evidence record *cites* it rather than
  re-deriving a digest? (Same shrink-the-borrow question DocGraph/MEHO raised.)
- Should control-mapping coverage be **baselined** (new-edges-only, ADR-003) so
  a pre-existing unmapped policy doesn't flip every evidence record to
  "uncovered" on first run? Likely baseline coverage; hard-pin nothing
  (mapping is advisory, never an error class).
- Is there an enterprise-prospect trigger that promotes the CEWS/COMPLY/TRUST
  (+ ISO 42001) constellation together — the same demand-gate as GATE and the
  MEHO "Enterprise Readiness constellation"? Four converging external projects
  (§3.2) is the market evidence; a real prospect is still the promotion gate.

---

## 9. One-line summary

> Decline the RuleHub codebase, its Kyverno/Helm/Backstage surfaces, and a
> dependency (wrong stack, no release, single-org). Validate Morgan's primitive
> but reframe it: the borrow is the **evidence-bound control-mapping seam**
> (pack → decision → evidence → export, recorded once and rendered many) —
> which Anvil already owns across CPACKS/COMPLY/CEWS and has tracked since
> 2026-03-08 (`verifywise`), with Asqav and `agent-audit-trail-mcp` already
> answering "where does it appear." The only real gap is a source-of-truth
> decision across the three modules (candidate ADR); the only net-new content is
> ISO 42001 (track). Mapping is advisory in the CLI, structured-optional in
> exports, and **never a compliance gate**. No dependency; clean-room; cite as
> parallel evolution + the fourth corroborating market signal.
