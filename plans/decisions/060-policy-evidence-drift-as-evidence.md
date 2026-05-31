# ADR-060: Policy & evidence source-quality drift as first-class evidence

## Status

Proposed

Emerged from the DocGraph borrow assessment
([`plans/brainstorms/2026-05-31-docgraph-borrow-assessment.md`](../brainstorms/2026-05-31-docgraph-borrow-assessment.md)).
Ready for council/PR review; not yet operator-ratified.

## Date

2026-05-31

## Context

Anvil exports governance evidence (the CEWS `EvidenceRecord` /
`ComplianceWorkspaceReport`, and the finding-emitting commands) that may **cite
policy, ADR, research, and documentation artefacts** as support for a decision.
Today the *trustworthiness of the cited source itself* is implicit: a decision
can cite a policy that is overdue for review, superseded, conflicting with
another approved policy, or resting on unverified research — and nothing in the
evidence record reflects that the support is weak. A stale or conflicted policy
should not be exported as **strong** compliance evidence.

The scope guard ([`docs/vision/anvil-scope-guard.md`](../../docs/vision/anvil-scope-guard.md))
has already adjudicated this surface in its Borderline Cases table: **"Policy
drift detection (stale/superseded/conflicting policies) → In"** ("Detection +
evidence; deterministic; strengthens trust in cited governance artefacts");
research-claim provenance is **"In if it feeds evidence"**; agent metadata
enrichment is **"advisory only"**; and knowledge-graph doc search is **Out**. So
the *what* is decided; this ADR records the *how*, drawing the deterministic
finding pattern from the [DocGraph](https://github.com/Detective-XH/DocGraph)
reference design (MIT, Go) without taking a code or runtime dependency.

Two existing decisions constrain the design:

- [ADR-058](058-sarif-shared-emitter-no-finding-model.md) **rejected a unified
  cross-command in-process finding model**; per-command shapes map *into SARIF*
  via adapters. Anvil already has many finding structs — `DriftFinding`
  (`crates/anvil-checks/src/surface/env/drift.rs:49`), `Finding`
  (`crates/anvil-policy-engine/src/result.rs:49`), `BaselineFinding`
  (`crates/anvil-baseline/src/finding.rs:13`), plus Secret/Env/Gitignore/etc. —
  and deliberately did not unify them. New findings must respect that boundary.
- [ADR-052](052-automated-drift-snapshots.md) already owns "drift" for
  **architecture edge drift** (`anvil drift snapshot|report`,
  `crates/anvil-cli/src/commands/drift.rs`); `surface/env/drift.rs` owns **env
  drift**. A new bare `drift_findings` / `DriftFinding` would collide.

A decision is needed now because CEWS-001 is about to define the
`control_evidence_model`; the source-quality fields are cheapest to add before
that model and its export packs (CEWS-004) solidify.

## Decision

Adopt **policy/evidence source-quality drift as first-class, advisory evidence**,
reimplemented in Rust from the DocGraph reference design (no code or runtime
dependency). Specifically:

1. **Evidence record fields.** Extend CEWS's `EvidenceRecord` (CEWS-001) with:
   `policy_source_ref`, `policy_source_digest`, `policy_canonical`,
   `policy_review_due`, `policy_superseded_by`, `source_conflict_status`,
   `evidence_verification_status`, `code_anchor_status`, and
   `evidence_drift_findings: Vec<EvidenceDriftFinding>`. The finding record
   follows the existing finding idiom (`{ code, severity (error|warning|info),
   subject_ref, related_ref, message, evidence }`) but is **namespaced**
   `EvidenceDriftFinding` because `DriftFinding` and `Finding` are already taken
   (see Context).

2. **`evidence_strength` + deterministic downgrade.** Introduce
   `evidence_strength` (`strong | moderate | weak`). The worst **unresolved**
   finding on a cited source caps strength: any `error` (e.g.
   `policy.conflicting`) → `weak`; any `warning` (e.g. `policy.stale_review`,
   `policy.superseded_referenced`) → `moderate`; clean → `strong`. Computation is
   deterministic against an **injected `AsOf`** (no live clock). The decision
   **still exports** — strength drops, reasons attach. Warnings-over-blocks
   ([ADR-002](002-warnings-over-blocks.md)), not a gate.

3. **Computation home = MDGOV; record home = CEWS.** The drift *computation*
   (the `policy.*` and `research.*` checks) lands in the markdown-governance
   crate (`crates/anvil-markdown-governance/`,
   [ADR-028](028-markdown-governance-crate.md)) as its **M2 "claim hygiene"**
   tier; CEWS consumes the findings onto the evidence record. No new engine.

4. **Authored-wins + content-hash anchoring.** Any agent-inferred evidence
   metadata is advisory (`source = agent_inferred`), never overrides authored/
   extracted metadata (source-priority projection), and is rejected if the cited
   source's content digest changed since inference. This realises the scope
   guard's "advisory only" verdict and ties into the witness chain
   ([ADR-037](037-witness-chain-and-l4-policy.md)) and LAC.

5. **Scope boundary (defers to ADR-058).** These are **domain findings on the
   evidence record**, *not* a unified cross-command finding model. When exported
   to SARIF they map via a per-command adapter exactly like `check`/`gate`/
   `audit`. We do **not** refactor the engine crates onto a shared finding model.

6. **Naming (avoids ADR-052 / env-drift collision).** Use `evidence_*`-scoped
   names ("policy-evidence source drift"), distinct from architecture-edge drift
   ([ADR-052](052-automated-drift-snapshots.md)) and env drift.
   `policy_source_ref` (not `policy_source`, which is already a local in
   `crates/anvil-cli/src/commands/policy/eval.rs`).

7. **Baseline / hard-pin behaviour.** `evidence_drift_findings` follow
   new-edges-only baseline semantics ([ADR-003](003-new-edges-only.md)) so a
   first run over an established corpus does not flip every decision to `weak`;
   `policy.conflicting` is a candidate **hard-pinned** class that is never
   baselined ([ADR-039](039-baseline-policy-and-hard-pinned-classes.md)).

8. **Phasing.** Ship the `policy.*` subset first (freshness, supersession,
   conflict — all computable from authored frontmatter). `research.*` follows.
   `code_anchor_status` and `code.*` are **advisory/beta-later**, consumed from
   `graph-v2-foundation` when a real symbol graph exists — DocGraph's shallow
   doc-comment scraping is not a model to copy. CEWS itself is **downstream** of
   COMPLY (both Draft), so this is staged work, not immediately executable.

## Rationale

A stale/conflicted/superseded source is a fact about the *evidence*, and the
right place to record a fact about evidence is the evidence record — surfaced as
reduced strength, not a build failure. Determinism (injected `AsOf`) makes it
testable and CI-stable. Putting computation in MDGOV (which already exists to
govern prose artefacts) and the record in CEWS (which already exists to hold
evidence) avoids inventing a new engine or a new shared finding model that
ADR-058 explicitly didn't want. The scope guard already classed policy drift as
In, so this is execution of an accepted direction, not a new scope claim.

### Alternatives Considered

| Option | Pros | Cons |
|--------|------|------|
| **Source-quality findings on `EvidenceRecord`, computed in MDGOV, advisory (chosen)** | Aligns with scope guard + ADR-001/002/003; deterministic; reuses three existing modules; honours ADR-058 boundary | Adds fields to a Draft model (CEWS); downstream of COMPLY; needs MDGOV M2 work |
| **A. Unified in-process finding model first** | One mapping into SARIF | Rejected by [ADR-058](058-sarif-shared-emitter-no-finding-model.md); large, hard-to-reverse engine refactor; out of proportion |
| **B. Runtime dependency / DocGraph sidecar** | Working code today | 5-day-old, single-author, 0-adoption project; Go vs Rust+TS; couples governance to an unproven binary; doc-search lane is scope-guard Out |
| **C. Gate/block on stale or conflicted policy** | Strong enforcement | Violates [ADR-002](002-warnings-over-blocks.md); brittle; punishes baseline state |
| **D. Do nothing (treat source risk as implicit)** | No work | The hidden-retrieval-risk problem this ADR exists to fix remains; contradicts the scope guard's "In" verdict |

## Consequences

- **Positive:** governance exports become auditable about *why* evidence is
  strong or weak; deterministic and CI-testable; no new engine, no new shared
  finding model; reuses CEWS + MDGOV + DOCGOV; clean-room-by-language so no
  licence entanglement.
- **Negative:** more fields on the (still Draft) `EvidenceRecord`; real
  computation work in MDGOV M2; the most useful end-to-end slice is blocked
  behind COMPLY.
- **Risks:** "drift"/"Finding" vocabulary overload; scope creep toward a unified
  finding model; over-reach into code-anchoring before a symbol graph exists.
- **Mitigations:** `evidence_*`-scoped names (Decision §6); explicit ADR-058
  boundary (§5); `code.*` deferred to `graph-v2-foundation` (§8); ship the
  `policy.*` subset first; baseline + hard-pin behaviour pinned (§7).

## References

- Borrow assessment: [`../brainstorms/2026-05-31-docgraph-borrow-assessment.md`](../brainstorms/2026-05-31-docgraph-borrow-assessment.md)
- Scope guard: [`../../docs/vision/anvil-scope-guard.md`](../../docs/vision/anvil-scope-guard.md) (Borderline Cases: policy drift = In)
- External: [`Detective-XH/DocGraph`](https://github.com/Detective-XH/DocGraph) (MIT, Go, v0.2.3)
- Related ADRs: [ADR-001](001-planless-first.md), [ADR-002](002-warnings-over-blocks.md), [ADR-003](003-new-edges-only.md), [ADR-028](028-markdown-governance-crate.md), [ADR-037](037-witness-chain-and-l4-policy.md), [ADR-039](039-baseline-policy-and-hard-pinned-classes.md), [ADR-040](040-rust-policy-engine-regorus.md), [ADR-052](052-automated-drift-snapshots.md), [ADR-058](058-sarif-shared-emitter-no-finding-model.md)
- APS modules: CEWS-001/004 (compliance-evidence-workspace), MDGOV M2 (markdown-governance), DOCGOV, LAC, AGOV, COMPLY, CPACKS, EVAL
- Code: `crates/anvil-checks/src/surface/env/drift.rs:49` (`DriftFinding`), `crates/anvil-policy-engine/src/result.rs:49` (`Finding`), `crates/anvil-markdown-governance/` (computation home), `crates/anvil-cli/src/commands/policy/eval.rs` (`policy_source` collision)
