# MEHO — Borrow Assessment

**Date:** 2026-06-03
**Status:** Brainstorm — assessment of MEHO as a borrow candidate (nominated by
Morgan). **Outcome: decline code/runtime adoption and decline dependency (MEHO
is a self-hosted runtime control plane — the Proxilion/PIC + GATE lane Anvil has
already deferred). Validate the narrow borrow Morgan actually points at — a
*provenance field set* on Anvil's decision/export evidence models — but split it
from the runtime half of his framing, which fails the scope guard. Take the
deployment-posture half as a candidate *deterministic scan surface / policy
pack* (Anvil enforces it on others' charts; it is not how Anvil deploys
itself). The "own vs record" open question is already answered by the GATE
demotion + the Proxilion/PIC precedent: Anvil owns artifact-time provenance and
*records opaque references* to runtime credential provenance it does not own.
No APS module filed; suggested CIB filings (next-available IDs from CIB-048) in
§7.3. Cite MEHO as parallel evolution; no dependency.**
**Source:** https://github.com/evoila/meho (Apache-2.0, Python 80.8% / Go 16.8%,
v0.10.0, 673 commits, 3★, vendor: evoila GmbH)

---

## 0. What this document is

A borrow assessment of an external repo, in the format of
[`2026-05-31-docgraph-borrow-assessment.md`](./2026-05-31-docgraph-borrow-assessment.md),
[`2026-05-24-drako-borrow-assessment.md`](./2026-05-24-drako-borrow-assessment.md),
and [`2026-05-22-proxilion-pic-borrow-assessment.md`](./2026-05-22-proxilion-pic-borrow-assessment.md).
The goal is **not** to adopt MEHO but to mine it for reusable governance ideas,
scope-guard each one, map it onto exact APS modules, and name the gaps. Facts
were read from the public repo landing on 2026-06-03 and cross-checked against
`plans/modules/*`, `docs/vision/anvil-scope-guard.md`, and the prior assessments
above.

---

## 1. Nomination summary

Morgan nominated MEHO on the framing that the sharp Anvil borrow is **credential
provenance plus deployment discipline in the same evidence story as policy** —
that a policy verdict on an infrastructure action is weak unless the audit
record also knows operator identity, tenant, short-lived credential exchange,
RBAC dependency, policy bundle digest, enforcing artifact, deployment reference,
image/chart pinning, readiness state, and signed artifact provenance. He also
flags MEHO's Helm/deployment posture (typed `values.schema.json`, migrations,
default-deny NetworkPolicy, restricted security context, explicit tag pinning,
no `:latest`) and asks the open question: **how much should Anvil own versus
record from an external control plane?**

| Project | What it is | Stack | Maturity |
| ------- | ---------- | ----- | -------- |
| MEHO | A "policy-gated, audit-grade, MCP-native" **governance backplane** that sits *at runtime* between AI agents (Claude Code, Cursor, Cline) and infrastructure (VMware/VCF, NSX, Kubernetes, cloud). Keycloak OIDC federates the caller; short-lived OIDC tokens are exchanged with **Vault** for backend credentials (agents never hold secrets); every operation passes one authorization seam; immutable PostgreSQL audit rows are attributed to the calling principal; activity broadcasts over Valkey. Ships a hardened Helm chart (values.schema.json, NetworkPolicy, securityContext, Cosign keyless-signed multi-arch images, no `:latest`). | Python 80.8% + Go 16.8% | Apache-2.0 · v0.10.0 · 673 commits · **3★** · single org (evoila GmbH, a real cloud consultancy — more engineering maturity than Drako/DocGraph/Proxilion, still low external adoption) |

**Operational footprint:** self-hosted; operators must provision **Vault,
Keycloak, PostgreSQL, Valkey, and an ingress controller**. This is a deployed
control plane, not a CLI.

---

## 2. Scope-guard test

Per [`docs/vision/anvil-scope-guard.md`](../../docs/vision/anvil-scope-guard.md),
Anvil operates at the **moment of change creation**, enforces deterministic
policy against artefacts, and captures provenance for **policy decisions**. The
four-question borderline framework: (1) increases prevention, (2) operates
before/at execution time, (3) strengthens deterministic control, (4) enforces
rather than only informs.

Morgan's framing bundles two things that the scope guard separates cleanly. The
assessment's central move is to **un-bundle them**:

| MEHO capability | Question that decides it | Scope-guard read |
| --------------- | ------------------------ | ---------------- |
| Runtime authorization seam over live infra actions (host evac, node drain, credential rotation) | #2 — before/at change creation? | **Out.** Runtime mediation of executed actions. Identical to Proxilion/PIC (2026-05-22) and to the runtime rung 6 declined for Drako (2026-05-24). |
| Keycloak→Vault short-lived credential **exchange**; operator-identity / tenant / RBAC resolution | #2, #3 — Anvil's deterministic control of *artefacts*? | **Out as a capability; In as a recorded reference.** Anvil neither issues nor exchanges credentials. It may record an *opaque reference / digest* of the exchange as evidence — never the value, never the exchange itself. |
| Immutable PostgreSQL audit rows + Valkey activity broadcast | Observability platform? | **Out.** Runtime audit store + live broadcast = the "observability that does not feed *creation-time* enforcement" exclusion. Anvil's own audit lane is the witness chain. |
| Provenance **field set** on the decision record (policy bundle digest, enforcing artifact, deployment ref, image/chart pinning, signed provenance) | #4 — does it strengthen the evidence an Anvil decision exports? | **In (the borrow).** These are artifact-time facts that can enrich Anvil's `EvidenceRecord` / `ProtectionClaim` and make an exported verdict independently auditable. This is the half worth taking. |
| Helm/chart hardening posture (no `:latest`, tag pinning, default-deny NetworkPolicy, restricted securityContext, typed values, migration discipline) | #1, #3, #4 — deterministic, preventive, at change creation? | **In as *policy Anvil enforces on others' charts*** (deterministic, save-time, evidence-producing). **Out as a model for Anvil's own deployment topology** (that belongs to `infra/` / GATE, not the engine). |
| Cosign keyless signing + GH Actions cert verification of MEHO's *own* release artifacts | Release-time attestation? | **In, but already owned** by SCA (supply-chain-attestation) — SLSA provenance + signed SBOM at release. Parallel evolution, not a new borrow. |

**Verdict:** the runtime backplane fails decision-rule #2 (operates after change
creation) on the same precedent that declined Proxilion/PIC and Drako rung 6.
The **provenance-field** half and the **deployment-posture-as-policy** half are
in scope and are the borrows. Morgan's instinct is right; his framing needs the
un-bundling above so the runtime half does not smuggle the rest of MEHO's
topology into Anvil.

---

## 3. Overlap with existing Anvil work

Anvil has already adjudicated MEHO's lane in two places, and already owns most
of the provenance scaffolding the borrow would land on.

| MEHO capability | Anvil equivalent (status) |
| --------------- | ------------------------- |
| Deployed governance backplane / central enforcement point | **GATE** (gateway-control-plane-patterns) · **Draft 0/3** — *demoted Ready→Draft on 2026-04-26 pending an enterprise consumer asking for gateway deployment guidance.* MEHO is a working reference of exactly GATE's "deployable control-plane patterns." It does not change GATE's promotion gate (needs a prospect). |
| Runtime agent→infra call mediation | **Declined** — Proxilion/PIC (2026-05-22) and Drako rung 6 (2026-05-24). Same operational-topology objection (proxy/seam + Postgres audit store). |
| Policy checkpoint orchestration + append-only `PolicyAuditEvents` | **OPAG** (opa-agent-orchestration) · **Ready** — already owns checkpoint evaluation + audit events; *explicitly out of scope: external identity/approval providers (SSO, HRIS)* — i.e. the Keycloak half is already declined. |
| Operator identity / tenant / RBAC resolution feeding a verdict | **ACTAX** (`<domain>.<verb>` taxonomy + RiskScore) · **IORISK** (io-risk-controls) · **ORGHIER** (org-policy-hierarchy) · **POLFED** (policy-federation) — Anvil classifies the *action* and routes risk; it does not resolve the caller's cloud identity. |
| Provenance fields on the decision/export record | **CEWS** (compliance-evidence-workspace) · **Draft** — `EvidenceRecord` / `ControlEvidenceMap` is the natural home for `policy_bundle_digest`, `enforcing_artifact_ref`, `deployment_ref`, `image_digest`, `chart_digest`, `signed_provenance_ref`. Mirrors the DocGraph Borrow-A pattern (source-quality fields on `EvidenceRecord`). |
| Hash-chained attribution of a decision to its actor | **AGOV** witness chain (ADR-037) + **MLP2** `ProtectionClaim` — already carries actor/agent attribution; can carry an *opaque* runtime-credential reference. |
| Cosign-signed release artifacts + SLSA-style provenance | **SCA** (supply-chain-attestation) · **Proposed** — release-time SBOM + SLSA provenance + signed attestation already scoped here. |
| Helm chart hardening (no `:latest`, NetworkPolicy, securityContext, typed values) | **Gap.** Surfaces today are `surface-dockerfile`, `surface-github-actions`, `surface-shell`, `surface-sql-migrations`. **No `surface-kubernetes` / `surface-helm`.** `pack-pulumi` exists but has no k8s/Helm posture rules. This is the one genuinely net-new deterministic check MEHO suggests. |
| Migration discipline | **surface-sql-migrations** already exists — partial overlap; MEHO adds chart-level migration ordering, which is k8s-surface territory. |
| "Enterprise Readiness" constellation (POLFED, ORGHIER, POLLC, COMPLY, CEWS, TRUST) | All already exist as modules, sequenced together "when the first enterprise prospect surfaces" (GATE audit note). MEHO is corroborating market evidence for that constellation, not new scope. |

Anvil's lane: **deterministic, evidence-producing governance of change at
save/commit/push time, language-agnostic, local-first CLI + daemon.** MEHO's
lane: **a self-hosted runtime authorization seam for agent→infra actions.** They
are adjacent (both gate agent actions with policy + audit), not overlapping in
the layer each uniquely owns — the same finding as Proxilion/PIC.

---

## 4. The borrows worth taking

Verdicts: **Use directly** (field names / posture rules are facts; clean-room
reimplement) · **Adapt** · **Inspiration only**.

### Borrow A — provenance field set on the decision/export evidence model (concrete · Adapt → CEWS / AGOV)

The strongest, most scope-aligned borrow, and the one Morgan is actually
pointing at. Add to CEWS's planned `EvidenceRecord` (and, where attribution
already lives, to the witness `ProtectionClaim`) a typed provenance block so an
exported infra-policy verdict is independently auditable:

- `policy_bundle_digest` — content hash of the policy/rule bundle that produced
  the verdict (Anvil already threads `rules_sha` onto witness lines, MLP2-014 —
  this *names* and *exports* it on the evidence record).
- `enforcing_artifact_ref` — which built artifact/binary did the enforcing.
- `deployment_ref` + `image_digest` + `chart_digest` — pinned references to the
  thing being changed/deployed (digests, never `:latest`).
- `signed_provenance_ref` — pointer to the SLSA/Cosign attestation (owned by
  SCA; CEWS records the reference).
- `credential_provenance_ref` — **opaque reference only** to the external
  credential-exchange record (e.g. "Vault lease id / Keycloak token jti
  digest"), plus `operator_identity_ref` / `tenant_ref` / `rbac_grant_ref`.
  **Never the credential, never the token, never the secret value** — Anvil
  records the *fact and pointer*, the external control plane owns the exchange.

This is the DocGraph Borrow-A move (source-quality fields → `EvidenceRecord`)
applied to *infrastructure-action provenance*. Same discipline: authored/derived
facts on the record, advisory where unverifiable, never blocks.

### Borrow B — credential/enforcer provenance as *recorded reference*, not owned capability (framing · the answer to Morgan's open question)

Morgan's open question — "how much should Anvil own versus record?" — is
answered by composing two existing decisions:

1. **GATE demotion + Proxilion/PIC precedent:** Anvil does **not own** the
   runtime credential exchange, the OIDC federation, or the authorization seam.
   Wrong layer, wrong topology.
2. **Witness/evidence model:** Anvil **records opaque references** to that
   provenance so a creation-time decision can *cite* it without *holding* it.

So: **own artifact-time provenance (policy bundle, enforcer, signed artifact,
pinning); record-by-reference runtime provenance (operator/tenant/credential
exchange/RBAC) emitted by an external control plane such as MEHO.** This is a
zero-code framing borrow that should be written into the SCA/CEWS specs and any
future GATE work so the next reviewer does not re-open "should Anvil issue
short-lived creds?" (answer: no — same as OPAG's "external identity providers =
out of scope").

### Borrow C — deployment-posture-as-policy: a k8s/Helm deterministic scan surface (concrete · Adapt → new surface / CPACKS)

MEHO's chart hardening is a **checklist of deterministic, save-time, preventive
properties** — exactly Anvil's lane *when pointed at the developer's manifests*,
not at Anvil's own deploy:

- image references pinned by digest or `:vX.Y.Z`; **no `:latest`** (warn/error).
- `values.schema.json` present + typed (untyped chart values = finding).
- default-deny `NetworkPolicy` present.
- restricted `securityContext` (non-root, drop-caps, read-only-rootfs).
- migration ordering/hooks declared (extends surface-sql-migrations thinking to
  chart-level Helm hooks).

This is a candidate **`surface-kubernetes` / `surface-helm`** module (gap in
§3), or a **CPACKS** policy pack, deterministic and new-edges-only. It is **not**
a statement about how Anvil itself deploys — that conflation is the trap in §6.

### Borrow D — single-governed-seam + immutable-attribution discipline (Inspiration only)

MEHO's "every operation passes one governed authorization seam; every row is
immutable and attributed to the calling principal" is good narrative
reinforcement of Anvil's witness-chain invariants (append-only, attributed,
hash-chained). Cite as parallel evolution; build nothing.

---

## 5. What NOT to borrow

| Item | Reason |
| ---- | ------ |
| MEHO's Python/Go codebase | Wrong stack (Anvil = Rust workspace + TS). Apache-2.0 is vendor-friendly but clean-room is preferable; no port/vendor. |
| The runtime authorization seam / MCP-native action gateway | Out of scope (decision-rule #2). Same precedent as Proxilion/PIC (2026-05-22) and Drako rung 6 (2026-05-24). Changes Anvil's operational topology. |
| Keycloak OIDC federation + Vault credential exchange | Anvil neither issues nor brokers credentials. OPAG already declares "external identity/approval providers (SSO, HRIS)" out of scope. Record references; do not broker. |
| PostgreSQL audit store + Valkey activity broadcast | Runtime audit/observability substrate; Anvil's audit lane is the local-first witness chain. Adopting these changes the footprint. |
| VMware/VCF / NSX / k8s **action connectors** (host evac, DRS, node drain) | These *execute* infra actions — the exact thing Anvil deliberately does not do. (The *manifest-linting* posture in Borrow C is the opposite: it inspects artefacts, executes nothing.) |
| MEHO's own Helm chart as a template for Anvil's deployment | Anvil ships as a CLI + daemon; its own infra is `infra/` + GATE, governed separately. Borrow the *posture as enforceable policy* (Borrow C), not the chart. |
| Cosign/SLSA signing of MEHO artifacts as a new initiative | Already owned by SCA (Proposed). Parallel evolution, not net-new. |
| Multi-tenant `tenant_id`/`tenant_role` runtime model | Anvil is local-first per-repo; tenancy lives in ORGHIER/POLFED as *policy hierarchy*, not as a runtime tenant context. |

---

## 6. Risks of the proposed framing

- **Bundling runtime credential provenance with artifact provenance (highest).**
  Morgan's one-line story — "credential provenance plus deployment discipline in
  the same evidence story as policy" — reads as a single borrow. It is two, and
  one of them (runtime credential exchange) fails the scope guard. If specced as
  one field block without the **opaque-reference-only** boundary, the witness
  chain starts wanting to *hold* credential/token material. Hard-state the
  boundary (Borrow B) in any spec.
- **"Deployment discipline" → Anvil's own deploy.** Easy misread of Borrow C as
  "harden Anvil's Helm chart." The borrow is *Anvil enforces this posture on the
  developer's charts*. Anvil's own deployment is GATE/`infra/` and out of this
  brainstorm.
- **GATE re-promotion pressure.** MEHO is exactly the "working reference of a
  deployable control plane" that could tempt re-promoting GATE to Ready. The
  2026-04-26 audit note is explicit: GATE promotes only on a real enterprise
  prospect. A nominated competitor repo is not a prospect. Do not move GATE.
- **Field-name collisions.** `provenance`, `attestation`, `evidence`, `digest`
  recur across SCA/CEWS/AGOV/witness. Namespace the new fields
  (`credential_provenance_ref`, not `provenance`; `policy_bundle_digest`, not
  `digest`) per the DocGraph §6 naming-collision lesson and ADR-058's
  "no unified cross-command finding model" boundary.
- **Maturity asymmetry.** MEHO is more engineered than prior candidates (673
  commits, real vendor) which makes "just depend on it" more tempting — but 3★
  and single-org adoption means a dependency would still be a de-facto fork at a
  layer Anvil does not operate at. Engineering maturity ≠ adoption maturity.

---

## 7. Recommendation

**Decline MEHO code/runtime adoption and dependency. Validate Morgan's borrow
after un-bundling it: take the provenance field set onto Anvil's decision/export
evidence models (CEWS/AGOV), take the deployment posture as a candidate
deterministic scan surface (new `surface-kubernetes`/CPACKS), and adopt the
own-vs-record framing as the standing answer to the open question. Cite MEHO as
parallel evolution; no APS module filed in this pass.**

Decision-ladder placement (per the assessment brief: ignore / track / document /
specify / plan / prototype / depend):

| Slice | Placement | Why |
| ----- | --------- | --- |
| MEHO runtime backplane (the product) | **Ignore / decline** | Out of scope; Proxilion/PIC + GATE precedent. |
| Provenance field set on evidence/export models (Borrow A) | **Document now → Specify when CEWS/AGOV advance** | In scope; lands on existing Draft modules; file CIB. |
| Own-vs-record framing (Borrow B) | **Document now** (this brainstorm + SCA/CEWS/GATE spec notes) | Zero-code; closes Morgan's open question. |
| k8s/Helm deployment-posture surface (Borrow C) | **Track → Specify/Plan only on an enterprise consumer** | Net-new, in scope, but same demand-gate as GATE. |
| Cosign/SLSA signing | **Already planned** under SCA | Parallel evolution, no new work. |
| Dependency on MEHO | **No** | Single-org, low adoption, wrong layer/topology. |

### 7.1 APS modules to update (exact list)

| Module | ID · Status | Update |
| ------ | ----------- | ------ |
| compliance-evidence-workspace | **CEWS** · Draft | Extend the planned `EvidenceRecord` with the Borrow-A provenance block (`policy_bundle_digest`, `enforcing_artifact_ref`, `deployment_ref`, `image_digest`, `chart_digest`, `signed_provenance_ref`) + the opaque `credential_provenance_ref` / `operator_identity_ref` / `tenant_ref` / `rbac_grant_ref`. Mirrors DocGraph Borrow-A. |
| supply-chain-attestation | **SCA** · Proposed | Confirm SLSA/Cosign release attestation already covers `signed_provenance_ref`; donate the digest/pinning conventions; record MEHO as parallel evolution. No promotion. |
| agent-governance-patterns | **AGOV** · Draft | Note that `ProtectionClaim`/witness can carry the opaque `credential_provenance_ref` (reference, never value); record the single-seam/immutable-attribution parallel (Borrow D). |
| gateway-control-plane-patterns | **GATE** · Draft | Add MEHO to GATE-001 reference topologies as a *parallel-evolution* example of a deployable control plane. **Do not change status** — promotion still gated on an enterprise prospect (2026-04-26 audit note). |
| opa-agent-orchestration | **OPAG** · Ready | Cross-reference: MEHO's Keycloak/Vault identity layer is the "external identity providers" OPAG already declares out of scope — reinforces, adds nothing. |
| compliance-policy-packs | **CPACKS** · Draft (high) | Candidate home for the Borrow-C k8s/Helm posture rules if a new surface module is not created. |
| io-risk-controls / policy-action-taxonomy | **IORISK / ACTAX** | Note-only: infra-action risk classification (drain/rotate/evac) is ACTAX `<domain>.<verb>` + IORISK dimensions if Anvil ever *classifies* (not executes) such actions. |

### 7.2 Gaps identified

1. **No `surface-kubernetes` / `surface-helm` module.** Surfaces today stop at
   dockerfile / github-actions / shell / sql-migrations. The Borrow-C posture
   checks (no `:latest`, NetworkPolicy, securityContext, typed values) have no
   home — gap to fill *if/when* a consumer wants chart governance.
2. **`credential_provenance_ref` has no defined shape.** The opaque-reference
   contract (what digest/pointer, never value) is net-new and needs the Borrow-B
   boundary written down before any field lands, or the witness chain will drift
   toward holding token material.
3. **CEWS/SCA are both pre-Ready** (Draft/Proposed). The Borrow-A fields are
   downstream work — document now, implement when those modules advance.
4. **"Provenance/attestation/digest" namespace is crowded** across
   SCA/CEWS/AGOV/witness — needs the §6 naming discipline before fields land.

### 7.3 Suggested CIB filings (next-available IDs — allocate at filing time)

Next-available is **CIB-048** (CIB header reads 29/47; max id CIB-047). Not
hard-coded here, to avoid a numbering race — allocate when filed under
[`continuous-improvement-backlog`](../modules/continuous-improvement-backlog.aps.md):

- **CIB-(next):** Add the Borrow-A infra-action provenance block +
  opaque `credential_provenance_ref` to CEWS `EvidenceRecord` (coordinate with
  SCA SLSA refs + AGOV witness). Carry the Borrow-B boundary as an acceptance
  note: references only, never values.
- **CIB-(next+1):** Triage a `surface-kubernetes`/`surface-helm` deterministic
  posture surface (Borrow C) — brainstorm follow-up that scope-guards each rule
  and decides surface-module vs CPACKS pack. Triage-only; no module yet.
- **CIB-(next+2, docs):** Write the own-vs-record provenance framing (Borrow B)
  into the SCA/CEWS/GATE spec notes so the open question stays closed.

---

## 8. Open questions (defer to follow-up specs)

- Does the witness chain's `rules_sha` (MLP2-014) already satisfy
  `policy_bundle_digest`, so CEWS records a *reference* to the witness line
  rather than re-hashing? (Same question DocGraph raised about deriving
  `policy_source_digest` from the chain.) Likely yes — shrink the borrow.
- For `credential_provenance_ref`: what is the minimal opaque shape that proves
  "a short-lived credential was exchanged for this action" without Anvil ever
  seeing the credential? A digest of the external lease/token id + issuer URI,
  probably — but pin it in an ADR, do not invent a schema ad hoc.
- Should infra-action provenance be **baselined** (new-edges-only, ADR-003) so
  pre-existing unattributed deploys do not flip every evidence record to weak on
  first run, with `policy.conflicting`-style classes hard-pinned (ADR-039)?
- Is there ever an enterprise-prospect trigger that promotes GATE *and* the
  Borrow-C surface together (the "Enterprise Readiness constellation")? If so,
  MEHO becomes a reference topology in that wave — but only then.

---

## 9. One-line summary

> Decline the MEHO codebase, its runtime authorization seam, and its
> Keycloak/Vault/Postgres topology (scope guard: Out; Proxilion/PIC + GATE
> precedent). Validate Morgan's borrow *after un-bundling it*: take the
> **provenance field set** onto CEWS/AGOV evidence models, take the **Helm/k8s
> hardening as enforceable policy** (candidate new surface), and adopt
> **own-the-artifact-provenance / record-the-runtime-provenance-by-reference**
> as the standing answer to "own vs record." No dependency; clean-room; cite as
> parallel evolution.
