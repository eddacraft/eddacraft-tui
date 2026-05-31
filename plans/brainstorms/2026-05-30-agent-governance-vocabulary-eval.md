# Agent Governance Vocabulary — Borrow Assessment

**Date:** 2026-05-30
**Status:** Brainstorm — borrow assessment. **Decision: borrow the _method_
(match-strength taxonomy, honest-gap discipline, warn-not-block validator,
deterministic matrix), reframe it as an _evidence importer/normalizer_ that
feeds enforcement — NOT as a published "Anvil governance vocabulary." Do not
take a build/runtime dependency on the upstream repo; clean-room reimplement
the catalogue/prose, reuse the enum tokens as facts. One provenance question
(below) must be resolved before any public framing.**
**Source:** External repo
[`aeoess/agent-governance-vocabulary`](https://github.com/aeoess/agent-governance-vocabulary)
(Apache-2.0, v0.1 draft), surfaced via Morgan research note
_Agent Governance Vocabulary (2026-05-29)_.
**Scope-guard:** [`../../docs/vision/anvil-scope-guard.md`](../../docs/vision/anvil-scope-guard.md)
**Related modules:** AGOV
([`../modules/agent-governance-patterns.aps.md`](../modules/agent-governance-patterns.aps.md)),
ACTAX
([`../modules/policy-action-taxonomy.aps.md`](../modules/policy-action-taxonomy.aps.md)),
OPENSPEC
([`../modules/open-spec-adapter.aps.md`](../modules/open-spec-adapter.aps.md)),
CEWS
([`../modules/compliance-evidence-workspace.aps.md`](../modules/compliance-evidence-workspace.aps.md))

---

## 1. Decision frame

Per the External Tool Integration Strategy
([`./2026-03-07-external-tool-integration-strategy.md`](./2026-03-07-external-tool-integration-strategy.md)),
each candidate is either **Borrow pattern** (do not name the vendor in
product-facing design; own the information architecture) or **Adopt directly**
(name it, take a dependency, file an ADR + APS module).

**Headline: Borrow pattern.** The upstream is a single-maintainer v0.1 draft
(4 stars). Its _ideas_ are strong; its _artifact_ is not load-bearing. Borrow
the discipline, reimplement in Anvil's stack, do not depend on the repo.

The scope-guard test (per the `anvil bom` triage,
[`./2026-05-26-anvil-bom-surface.md`](./2026-05-26-anvil-bom-surface.md)) is the
gate: a capability earns its place **only if it feeds enforcement**. A
governance _vocabulary_ is advisory by nature, and the scope-guard explicitly
rejects advisory features ("If it only informs → Reject"). So the naïve read
("Anvil publishes/joins a governance vocabulary") **fails** the scope-guard.
The surviving shape is an **evidence importer** (§3).

## 2. What the upstream actually is

A "naming layer _over_ existing specs" modelled on the IANA JWT claim registry
and JSON-LD `@context`. Three layers, deliberately separated:

1. **`vocabulary.yaml`** — canonical primitive names (signal types + descriptor
   dimensions + match-type definitions). Top-level keys: `version`, `status`,
   `signal_types`, `descriptor_dimensions`, `system_attributes`,
   `context_dimensions`, `crosswalk_match_types`, `decision_trajectory`,
   `constraints`, `out_of_scope`.
2. **External multi-attestation spec** — envelope `type` values (algorithm
   ids, proof types). **Explicitly out of scope here** — owned by the
   originating standards; not renamed by this vocabulary.
3. **`crosswalk/<system>.yaml`** — per-system mappings from local terms to
   canonical names, with a declared match strength and honest `no_mapping`
   gaps.

**Tooling:** zero production deps, one dev dep (`js-yaml`), two custom Node
scripts — `validate-crosswalks.js` and `generate-crosswalk-matrix.js` — wired
to npm `validate` / `test` / `generate:matrix`. No JSON Schema for the YAML
(validation is imperative JS). A `scripts/legacy-descriptor-overrides.yaml`
whitelists stale descriptor combos to **warn, not block**, contributor CI.

**Coverage:** 25 crosswalk files exist; 15 render into a 15×13 matrix
(`docs/generated/crosswalk-matrix.md`). The delta (proposed/alt-format/reverse/
fixture files) is consistent with the upstream's own "two independent
implementations before a term is canonical" rule — single-impl terms sit at
`status: proposed` and are excluded from the published matrix.

> Facts above were retrieved from the repo on 2026-05-30 (see §10). The
> `vocabulary.yaml` fetch was partially summarised by the fetch tool; enum
> values in §6 are quoted from a targeted re-fetch and should be spot-checked
> against source before they are encoded anywhere load-bearing.

## 3. The reframe — importer, not publisher

Anvil is already a **signal _producer_**, not a consumer of someone else's
vocabulary:

- **AGOV** — trust score (0–1000), capability manifest (AGOV-007),
  hash-chained audit / witness chain (AGOV-006, ADR-037), destructive-pattern
  and change-volume gate checks.
- **ACTAX** — a canonical, append-only `<domain>.<verb>` action taxonomy and a
  thin `RiskScore` (IORISK dimensions: destruction, outbound, sensitivity,
  irreversibility, scope) fused into `warn / fence / interrupt` routing.
- **CEWS** — `EvidenceRecord` / `ControlEvidenceMap` linking policy outcomes to
  audit artifacts.

So the in-scope shape is **an importer/normalizer that maps third-party
attestations onto Anvil's _existing deterministic signals_ so external evidence
can feed enforcement.** Concretely:

- The crosswalk file → an **internal adapter manifest** for an external
  attestation issuer (fits the `packages/adapters/` `detect/parse/serialize/
  validate` pattern that OPENSPEC/BMAD/SpecKit already use).
- The match-strength taxonomy → the **admissibility gate** on imported
  evidence.
- The matrix generator → an **evidence-coverage report** (which issuers Anvil
  will accept and act on, with declared gaps) — in-scope only because it
  documents what feeds enforcement, not as a standalone dashboard.

This keeps Anvil's internal signal/verdict names frozen (mirroring the
upstream's "don't rename live envelope values" rule); only the _import edge_
gets canonical aliases.

## 4. Reuse trichotomy

| Artifact | Verdict | Notes |
| --- | --- | --- |
| 5 `crosswalk_match_types` + one-line defs | **Use directly** | Small, well-calibrated, factual. Most reusable thing. Reimplement clean-room (tokens are facts). |
| Validator contract (errors→exit 1 block CI; warnings→exit 0; legacy→warn) | **Use directly (as pattern)** | This _is_ Anvil's "warnings over blocks / exit 0 by default." Reimplement in Rust/TS; do not vendor the JS. |
| Descriptor-dimension enums (`enforcement_class`, `governed_action_class`, `replay_class`, `validity_temporal`, `refusal_authority`, `invariant_survival`, `measurement_point`) | **Adapt** | Strong starter ontology for classifying imported evidence; prune to Anvil's reality; align `governed_action_class` with ACTAX. |
| Crosswalk YAML schema (`canonical`/`match`/`notes` + namespaced `<sys>_term/_field/_symbols` + signed-shape `issuer_uri`/`jwks`/`algorithm`/`kid_prefix`/`signed_shape`) | **Adapt** | Becomes the attestation-importer adapter manifest; fits `packages/adapters/`. |
| Deterministic matrix generator (alpha file sort, locale system sort, count-desc+alpha coverage) | **Adapt** | Reframe as evidence-coverage matrix tied to enforcement/provenance. |
| Two-independent-implementations promotion rule + `domain_incubation` 90-day sunset / max-3 cap | **Adapt** | Discipline for promoting advisory→binding and expiring unproven mappings; mirrors Anvil's "the shape that earns its place." |
| Signal-type catalogue (`wallet_state`, `settlement_witness`, `entity_continuity`, …) | **Inspiration only** | Crypto/agent-economy flavoured (peers: a2a, agent-DID, soulboundrobots, sovereign-atom, moltrust). Only `governance_attestation`/`trust_verification`/`compliance_risk`/`peer_review`/`completion_ratio` are even adjacent. |
| IANA-JWT / JSON-LD `@context` framing; `llms.txt` | **Inspiration only** | Good narrative + cheap AI-discoverability touch; not things to build. |
| Generator/validator JS; "Working Group" framing | **Inspiration only** | Single-maintainer, unversioned, no schema; README overstates governance (CONTRIBUTING admits no formal WG). |

## 5. Patterns / processes worth replicating

- **Don't rename live envelope values.** Freeze Anvil's internal signal/verdict
  names (`warn/fence/interrupt`, `allow/interrupt`, trust 0–1000, witness rows);
  only the import edge gets canonical aliases.
- **Errors block, warnings inform, legacy is whitelisted-to-warn.** Identical to
  `architecture.md` ("warnings over blocks"; "new edges only — baseline
  existing state, warn on new violations"). Imported evidence below a fidelity
  threshold may only warn, never block.
- **Honest-gap-by-construction.** `no_mapping` is a required, reviewed
  declaration with a rationale — not a silent omission.
- **Determinism on the import verdict, not just the doc.** Same external
  attestation → same admissibility verdict, reproducibly, in the policy path.
- **Earn-your-place promotion.** Two independent implementations +
  `verified_at`/sunset gate when an imported field becomes load-bearing.

## 6. Two-axis model (the key technical decision)

The upstream's 5 match types are a **mapping-fidelity** axis. Enforcement weight
is a **separate** axis (their `enforcement_class` descriptor). Anvil must keep
them orthogonal — collapsing "advisory-only" into the fidelity axis (as the
open question phrasing risks) throws away the false-friend warning that
`non_equivalent_similar_label` exists to carry.

**Axis 1 — fidelity** (quoted defs):

| Match type | Definition |
| --- | --- |
| `exact` | Identical primitive, same signature semantics. |
| `structural` | Same primitive shape, different field names. |
| `partial` | Overlapping but not equivalent; specify dimensions of divergence. |
| `non_equivalent_similar_label` | Looks similar lexically, governance semantics differ. |
| `no_mapping` | No analog in target system. Honest gap. |

**Axis 2 — admissibility** (reuse `enforcement_class`): `advisory | binding |
refusal_authority`.

**Descriptor dimension enums (quoted, for reference):**

- `enforcement_class`: advisory, binding, refusal_authority
- `validity_temporal`: at_issuance, at_acceptance, at_processing, continuously,
  sequence, windowed, epoch
- `refusal_authority`: issuer, verifier, consumer_policy, shared
- `invariant_survival`: pre_action, during_action, post_action, permanent
- `replay_class`: full_replay, decision_replay, fingerprint_only, no_replay
- `governed_action_class`: read, write, transfer, delegate, publish, compose
- `measurement_point`: session_boundary, mutation_boundary

## 7. Open questions (answered)

**Q1 — Which fields need canonical names first?** Order by enforcement
load-bearing-ness, and exclude internal verdicts from renaming:

1. **Action taxonomy (ACTAX)** — already canonical + append-only; the natural
   join key; highest leverage.
2. **Imported third-party attestations + evidence strength** — the import
   admissibility surface (maps to CEWS `EvidenceRecord`, SUPPLY).
3. **Approval lifecycle** — witness/provenance states (ADR-037).
4. **Policy verdicts** — canonical aliases _for import comparison only_; do
   **not** rename internal `warn/fence/interrupt` / `allow/interrupt`.

**Q2 — Publish a formal vocabulary early, or importer crosswalks first?**
**Importer crosswalks first, internal.** Scope-guard (advisory), maturity/
bus-factor risk, and the upstream's own two-impl rule all say publishing early
is premature. Externalise a vocabulary later only as a deliberate standards
play — never as a beta feature.

**Q3 — Which match strengths for beta?** Keep the **5 fidelity grades** as-is +
a **separate admissibility flag** (`advisory | binding`). Beta default: anything
below `structural` fidelity is `advisory` (warn-only), never blocks;
`no_mapping` is a required first-class declaration.

## 8. Risks, licensing, provenance

1. **Scope-guard drift (highest).** Adopting this as a vocabulary _product
   surface_ fails "if it only informs → reject." Mitigation: importer-feeds-
   enforcement framing only, gated by ADR.
2. **Provenance / naming collision (must resolve).** The repo ships
   `crosswalk/aeoess-aps.yaml` whose system short-name is `aps`, an _exact_
   match for `passport_grade`/`governance_attestation`, with `jwks`/`issuer_uri`/
   `algorithm`/`kid_prefix`/`signed_shape`. That could be **Anvil Plan Spec
   lineage** (Anvil's APS ref is `eddacraft/anvil-plan-spec`; repo owner
   `aeoess`) **or** an unrelated "Agent Passport System" colliding on "APS."
   **Could not confirm from the repo. Do not claim "Anvil is already in the
   matrix" until authorship is verified** — it flips the licensing calculus and
   is a public-comms footgun.
3. **Maturity / bus factor.** v0.1, 4 stars, single maintainer, README
   overstates a non-existent Working Group, imperative JS validation, no schema.
   **No runtime/build dependency.**
4. **Licensing (clean, manageable — Apache-2.0).** Same pattern as permit0
   (already vendored Apache-2.0 via ATTRIB + NOTICE + ACKNOWLEDGEMENTS):
   - Match-type tokens, dimension _names_, enum values = facts/short phrases →
     not protectable; reimplement clean-room, no attribution strictly required.
   - Prose definitions, the signal catalogue, any copied YAML/JS → preserve
     `LICENSE`/`NOTICE`, mark modified files, add an ACKNOWLEDGEMENTS entry.
   - Apache-2.0 patent grant is fine; avoid implying endorsement / treating the
     draft as ratified.
   - **Clean-room is _preferable_ for the catalogue + prose** anyway — frees
     Anvil from the draft's churn and the crypto-economy framing.

## 9. Recommended next actions

1. **Confirm provenance** (cheapest, unblocks framing): verify whether `aeoess`
   ↔ `eddacraft`/Anvil and what `aps` denotes in the crosswalk.
2. **ADR** (per `docs/guides/adr-process.md`, log in `DECISION-LOG.md`):
   _"External evidence import & admissibility."_ Establish importer-not-
   publisher posture, the fidelity × admissibility two-axis model, warn-only
   default below `structural`, and the scope-guard justification.
3. **File an APS work item** under `packages/adapters/` (OPENSPEC precedent) or
   a new IMPORT module: an attestation-importer adapter using the adapted
   crosswalk schema; reuse the 5 match types + warn/error contract directly.
4. **Pilot one round-trip** before generalising: one real external source →
   one Anvil signal (e.g. SLSA/in-toto supply-chain attestation → SUPPLY/
   AGOV-006 witness; or external compliance attestation → CEWS `EvidenceRecord`)
   with a deterministic admissibility verdict + a generated coverage matrix.
5. **Freeze internal envelope values**; only the import edge gets aliases.
6. **Do not depend** on the upstream repo; treat it as a reference draft and
   reimplement generator/validator in Rust + the TS adapters layer.

## 10. Method & sources

Read-only external research on 2026-05-30; no upstream contribution made. Files
fetched from `github.com/aeoess/agent-governance-vocabulary` (repo landing +
`raw.githubusercontent.com/.../main/`): `README.md`, `vocabulary.yaml`
(structure + targeted enum re-fetch), `crosswalk/aeoess-aps.yaml` (schema),
`package.json`, `docs/generated/crosswalk-matrix.md`, `CONTRIBUTING.md`,
`scripts/validate-crosswalks.js`, `scripts/generate-crosswalk-matrix.js`.
Anvil grounding: `docs/vision/anvil-vision.md`, `anvil-scope-guard.md`,
`.claude/rules/architecture.md`, and the AGOV / ACTAX / OPENSPEC / CEWS
modules. The one `vocabulary.yaml` verbatim fetch was declined by the fetch
tool on length grounds; enum values were recovered via a narrower factual
re-fetch and are flagged for spot-check in §2.
