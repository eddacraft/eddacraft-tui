# Governance Module Family — Map

> **Draft companion** to [`2026-06-08-git-native-governance-review.md`](./2026-06-08-git-native-governance-review.md)
> (finding F-1). Disambiguates the ~7 APS modules that carry "governance" in
> their name, states how they join, and flags the overlaps and naming hazards.
> Candidate to graduate into `docs/architecture/` (with governance metadata +
> `docs:index`) once agreed.

## Why this exists

Seven active modules use the word **governance**, and it means at least **three
different things** across them. Without a stating-doc, the family drifts and
"what does governance mean this week" confusion is guaranteed (the same taxonomy
failure that left the GV2 module stale-grounded). This map names the axes, places
each module, and marks the seams.

## The three axes

**The word "governance" is overloaded across three orthogonal axes.** A module
belongs to exactly one.

- **Axis A — AI-change evidence governance.** "Anvil governs AI-assisted change
  and makes the evidence durable." The core product thesis. Substrate + the
  evidence types that ride it.
- **Axis B — Knowledge/docs governance.** "Govern the project's own docs and
  plans." Meta-governance of the knowledge system, not of code changes.
- **Axis C — Product-surface governance.** "Govern a product surface as a
  first-class architectural concern." Orthogonal to AI-change governance
  entirely.

## Module placement

| Module | Axis | Role | Status | One-line |
| --- | --- | --- | --- | --- |
| **GITGOV** (git-native-governance) | A | **Substrate (umbrella)** | Proposed | Git as durable trust substrate; Review Capsules wedge. The home all other Axis-A evidence reports into. |
| **EXCEPT** (git-native-exceptions) | A | Evidence type / consumer | In Progress | First-class, durable, reviewable exceptions under `anvil/`; appear in capsules. File-based sibling of `@anvil-ignore` (ADR-004). |
| **AGOV** (agent-governance-patterns) | A | Signal **producer** | Draft (Tier C) | Trust scoring, destructive-pattern detection, change-volume thresholds, metadata-secret scan, hash-chained audit, capability declaration. Upstream signals consumed by CPACKS + MDGOV. |
| **ILGOV** (intent-ledger-governance) | A | Provenance layer | Draft (0/6) | Intent-vs-effect provenance — the *original* Anvil thesis ("prove the plan was followed"), now graph-derived effect prediction vs captured intent. |
| **DOCGOV** (documentation-governance) | B | Knowledge system | In Progress (9/12) | Authority model + lifecycle metadata + validation (`docs:check`/`docs:index`) for APS/ADRs/as-built/runbooks. |
| **MDGOV** (markdown-governance, Track 5) | B | Validator | Draft | Markdown-as-governance-artefact well-formedness (APS schema, cross-ref integrity). Standalone crate `crates/anvil-markdown-governance` (ADR-028). |
| **APGOV** (api-governance) | C | Product-surface | Ready | Versioning/deprecation/CORS/OpenAPI/error-contract for the `anvil-api` REST surface. Unrelated to AI-change evidence — see naming hazard below. |

## The Axis-A spine (where the real joins are)

```text
                         ┌──────────────────────────────┐
   producers             │   GITGOV — Git substrate      │   ADR-072 substrate
   ┌──────────┐          │   (Review Capsule wedge)      │   ADR-073 state boundary
   │  AGOV    │ signals  │                               │   ADR-074 capsule v0
   │ trust/   ├─────────▶│  capsule = { policy, rules,   │
   │ audit/   │          │    baseline, witness,         │
   │ capability│         │    diagnostics, exceptions,   │◀── EXCEPT (exceptions
   └──────────┘          │    sealed-edda-ref }          │      as durable evidence)
                         └───────────────┬───────────────┘
   ┌──────────┐ intent-vs-effect         │ reuses (verbatim, no re-model)
   │  ILGOV   ├──────────────────────────┤
   │ provenance│   shares provenance     ▼
   └────┬─────┘   contract with     anvil-witness · anvil-baseline ·
        │         GV2-014/EDDA-SEAL  anvil-rules · anvil-policy(exceptions) · SARIF
        │
        ▼ (graph-derived effect prediction)
   GV2 semantic/dependency graph  ·  Edda provenance (TS today; Rust = open seam)
```

- **GITGOV is the umbrella** for Axis A: the substrate decision + the capsule
  artefact every other Axis-A evidence type flows into.
- **EXCEPT** is both a fix (move exceptions to tracked `anvil/`) and an evidence
  type (exceptions appear in capsules) — a consumer of the substrate.
- **AGOV** is a set of signal *producers* (it is not consumed by GITGOV directly
  today; it feeds CPACKS and MDGOV). Its inclusion in "Axis A" is by subject
  matter (agent/change governance), not by a wired dependency on GITGOV.
- **ILGOV** is the provenance/intent layer and the one with the **deepest GV2
  coupling** — it uses the symbol/architecture graph to predict effect.

## Seams and overlaps to watch

| Edge | Nature | Action |
| --- | --- | --- |
| EXCEPT → GITGOV | exceptions are capsule evidence; recording exception-use must not fork the witness schema | resolve before GITGOV-009 (review finding F-4) |
| ILGOV ↔ GV2-014 ↔ EDDA-SEAL | three items independently touch durable provenance under `anvil/edda/` (Rust↔TS boundary) | **one** shared provenance contract, not three (review finding §5) |
| AGOV-002 ↔ CPACKS | only AGOV-002 (HIPAA/PCI pack stubs) overlaps CPACKS; AGOV-001/003-007 are distinct | migrate AGOV-002 → CPACKS per AGOV audit note; keep the rest |
| MDGOV ↔ DOCGOV | both validate docs; MDGOV at markdown-parse level (own crate), DOCGOV at authority/metadata level | keep distinct (MDGOV = structural well-formedness; DOCGOV = authority/freshness/links) |
| APGOV ↔ family | Axis C; shares only the word "governance" | consider renaming out of the family (e.g. `api-surface-policy`) — see hazard |

## Naming hazard (the F-1 root cause)

"Governance" is doing too much work:

- **Axis A** is *change-evidence governance* (the product).
- **Axis B** is *knowledge governance* (internal docs hygiene).
- **Axis C** (APGOV) is *product-surface governance* (the REST API) — it would
  read identically if it were called `api-surface-policy`, and it has no
  relationship to the other six beyond the shared word.

Recommendation: keep this map as the canonical disambiguator; when modules are
next touched, consider an axis-signalling convention (e.g. GITGOV/EXCEPT/AGOV/
ILGOV stay as the "change-evidence" family, DOCGOV/MDGOV are explicitly the
"docs" pair, and APGOV is renamed/re-homed out of the governance family). Do not
mass-rename now — the map is the cheap fix; renames are churn.

## Open questions

1. Is **GITGOV** intended as the formal umbrella that AGOV/ILGOV/EXCEPT report
   into, or are they peers under the Axis-A thesis? (Affects whether AGOV/ILGOV
   should declare GITGOV as upstream.)
2. Should the **provenance contract** (ILGOV ∩ GV2-014 ∩ EDDA-SEAL) be its own
   ADR before any of the three freeze a schema?
3. Should **APGOV** leave the governance family by name?

## Related

- Review: [`2026-06-08-git-native-governance-review.md`](./2026-06-08-git-native-governance-review.md)
- ADRs: [072](../decisions/072-git-native-governance-substrate.md),
  [073](../decisions/073-durable-vs-local-anvil-state.md),
  [074](../decisions/074-review-capsule-v0-format.md),
  [028](../decisions/028-markdown-governance-crate.md)
- Modules: [GITGOV](../archive/modules/git-native-governance.aps.md),
  [EXCEPT](../modules/git-native-exceptions.aps.md),
  [AGOV](../modules/agent-governance-patterns.aps.md),
  [ILGOV](../modules/intent-ledger-governance.aps.md),
  [DOCGOV](../archive/modules/documentation-governance.aps.md),
  [MDGOV](../modules/markdown-governance.aps.md),
  [APGOV](../modules/api-governance.aps.md)
