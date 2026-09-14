# ADR-142: Graph Answers Attest Their Own Bounds

## Status

Accepted 2026-09-07 (owner)

## Date

2026-09-07

## Context

Anvil's graph-context surface answers six questions an assistant asks before it
edits code — which symbols exist, who calls them, what depends on them, what a
change hits, which tests reach it, and what one symbol looks like. The product
claim is that this output can be trusted without reading Anvil's source.

An answer can be wrong about itself in three distinct ways. Two are already
disclosed:

- **Privacy** — what egress removed. `RedactionSummary`
  (`crates/anvil-gctx-types/src/lib.rs`) carries `omitted_sensitive_paths`,
  `fields_suppressed`, `snippets_truncated`, `fully_suppressed_symbols`, all
  counts-only per PV-9 CE-11.
- **Assurance** — whether the graph was warm. `workspace_assurance.state` with a
  `reason`, and the named degradation outcomes (`NotReady`, `Unavailable`,
  `StaleGraph`) from ADR-084 / CE-7.

The third is not: **what the answer itself could not resolve, or could not
fit.** It is disclosed well on one tool and thinly or not at all on the rest.

- `anvil_search_symbols` reports `matched` (the pre-cap total), `returned` and
  `truncated`. This is the good case.
- `ImpactSummary` and `AffectedTestsSummary` count only what was *returned* and
  carry a single `truncated: bool` whose own doc comment says the cap was
  "**either** the affected-symbol set or the dependent-closure walk". A consumer
  cannot tell which budget fired, and has no pre-cap figure to compare against.
- `crates/anvil-graph-cache/src/call_graph.rs` computes ambiguity **per call
  edge** and then OR-s it across all of a caller's edges into one caller-level
  `heuristic` flag. A caller with one unresolvable call and three exact ones is
  reported as wholly heuristic. The per-edge decision is computed and discarded.
- The versioned token estimator (`estimate_gctx_tokens`,
  `GCTX_TOKEN_ESTIMATOR_VERSION`, `crates/anvil-graph-cache/src/tokens.rs`,
  GCTX-020) drives the snippet budget but is never reported to the caller, so an
  assistant cannot budget an answer before spending context on it.

The result: `anvil_impact_of_change` returns the answer with the highest stakes
in the product — the blast radius a reviewer is asked to trust — and it is the
answer a reviewer can audit least. "These twelve symbols are affected" does not
say that three of those edges were guesses, that the walk stopped at a node
budget, or which of two budgets stopped it.

**Why now.** The GCTX delivery contract is frozen and its Phase 1–3 items are
all Merged; the surface is shipped and in front of assistants today. The
[ripwire borrow assessment](../brainstorms/2026-09-07-ripwire-borrow-assessment.md)
(2026-09-07) examined an external tool built around exactly this disclosure
discipline and recommended one primitive out of it, declining code adoption and
dependency. This ADR is that primitive. It needs an ADR rather than a bare spec
amendment for three reasons: it binds all six existing tools **and every future
one**, it is a convention that is expensive to retrofit once clients parse the
envelope, and it extends the surface that PV-9 conditioned.

**One thing this decision must not do.** ripwire's headline disclosure is a
ranking confidence with a score margin. `SearchSymbolsQuery` is conjunctive
filters over a deterministic keyset walk — there is **no ranker**, so there is no
ranking confidence, and inventing a field for one would be the exact defect this
ADR exists to prevent: an estimate presented with the confidence of a
measurement.

## Decision

Every GCTX projection carries a single, shared **`Attestation`** block stating
what bound the answer and what it costs. The block is additive, counts-and-flags
only, and identical in shape across every tool and the `graph://` resources.

### 1. A shared type, not per-tool fields

`Attestation` is one struct in `crates/anvil-gctx-types`, held by every
projection DTO. It is **not** a set of per-tool ad-hoc fields, and it is **not**
folded into `RedactionSummary`.

Two separate structs, because they answer two different questions and are
audited by different people: `RedactionSummary` answers *what did the privacy
boundary remove* (CE-11, a security artefact); `Attestation` answers *what did
the query fail to resolve or fit* (an epistemic artefact). Merging them would
make the CE-11 counts harder to review in isolation, which is the one property
that made them acceptable to PV-9.

Because the type is required by every projection DTO, a new tool cannot ship
without disclosure. That is the enforcement mechanism, and it is the point:
partial adoption across six tools is worse than none, since a caller then cannot
distinguish a tool with no bounds from one that does not report them.

### 2. Bounds are named and separately counted — never one bool

For every section an answer can cap, `Attestation` carries one entry naming:

- the **section** bounded,
- the **budget** that bound it (a closed enum — page limit, node budget,
  traversal depth, byte ceiling, session ceiling), one entry per budget that
  actually fired,
- **`returned`**, and
- **`total`**.

A section that was not bounded contributes no entry. `truncated: bool` is
retained on existing DTOs for one release and becomes *derived* (`true` iff at
least one bound entry exists), documented as a compatibility shim, not removed
in this change.

### 3. `total` is a tagged count, never a bare number

`total` is `Exact(n)` or `AtLeast(n)`. It is never a bare integer.

This is load-bearing rather than pedantic. On a filtered scan (`search_symbols`)
the pre-cap total is cheap and honest — `Exact`. On a **bounded traversal**
(`impact_of_change`'s dependent closure, `affected_tests`' reverse walk) the true
total is unknowable without completing the walk the budget exists to prevent:
reporting one would require defeating our own node budget on every call. `AtLeast`
states the floor and says it is a floor, in the type, so no consumer can read it
as a total.

A number that reads like a total but is a floor is the defect this whole ADR is
about. Encoding it in the type is the only way it cannot recur by omission.

### 4. Resolution fidelity moves to the edge

Call-resolution ambiguity is reported per call edge. Any caller-level
`heuristic` flag is retained as a **derived summary of its edge set** and
documented as such.

Marking the symbol when one of its edges is ambiguous is a false statement about
its other edges, and blast-radius answers are read edge by edge.

### 5. The answer reports its own cost

`Attestation` carries `est_tokens` and the `estimator_version` that produced it,
from the existing `estimate_gctx_tokens`. No second estimator is introduced. The
version travels with the number so it stays comparable across releases and reads
as an estimate rather than a measurement.

This reports cost; it does not enforce it. The snippet budget keeps its current
behaviour.

### 6. Egress posture: counts and closed enums only, and `total` is post-filter

Every field is a count, a closed enum variant, or a version string. No name, no
path, no span, no content. The block therefore travels in the identity-only
default (CE-1) and inherits the counts-only posture PV-9 accepted for
`RedactionSummary` (CE-11); the `gctx.egress` opt-in is untouched.

One constraint is **binding** rather than incidental: every count is taken
**after** the CE-3 sensitive-path deny-list is applied. Counting before it would
turn `total` into an oracle for the existence of denied paths — a query whose
`total` exceeds its post-filter result set would confirm that matching
`.env`/`id_rsa`/`.aws` content exists. Denied paths stay counted only in
`RedactionSummary.omitted_sensitive_paths`, which is where PV-9 put them.

### 7. Gates are written before the fields they guard

No attestation field merges before a test that forces its condition and proves
the disclosure observable — a cap that actually fires, an ambiguous edge that
actually exists, an estimate actually attached. Each such gate must be shown to
fail when its subject is removed.

A gate that cannot observe what it asserts is worse than no gate, because it
reports confidence. This is adopted as a rule for this contract specifically, not
as a repo-wide process change.

### 8. Both surfaces, one shape

The block is defined on the projection, above the transport, so the MCP tools and
the ADR-095 CLI secondary render the same attestation from the same
`GctxProjector` output. Neither surface may carry a field the other lacks.

## Rationale

The alternative framings all fail on one of two things: they either leave the
highest-stakes answer unauditable, or they add a field that sounds informative
and is not.

Anvil's differentiation is not retrieval quality — that is a crowded race this
decision deliberately does not enter (ADR-075 scope guard; the GV2 framing rule
that GCTX is a projection, never a driver of substrate). The differentiation is
that Anvil's answers can be *checked*. Disclosure is therefore on-mission in a
way that ranking is not, and it is cheap: the ambiguity is already computed, the
estimator already exists, the counts are already partly there.

### Alternatives Considered

| Option | Pros | Cons |
| --- | --- | --- |
| **Chosen: shared `Attestation`, tagged counts, per-edge fidelity** | One shape across six tools and every future one; a new tool cannot ship undisclosed; floors cannot masquerade as totals; no substrate change | Widens every projection DTO; one release carrying a derived-but-retained `truncated` |
| Spec amendment, no ADR | Lighter; no decision-log churn | Binds six tools plus a rule for future ones and extends a PV-9-conditioned surface — that is a convention, and conventions belong in an ADR where they can be found and superseded |
| Fold the fields into `RedactionSummary` | One struct; no new type | Conflates "what privacy removed" with "what the query could not resolve"; degrades the CE-11 audit story that made the counts acceptable to PV-9 |
| Per-tool ad-hoc fields | Minimal diff per tool; each tool discloses exactly what it has | The sixth tool forgets. Sibling-completeness is the dominant defect class here — a fix that lands on one member of a family and not its siblings |
| Bare-integer `total` everywhere | Simplest type; familiar | Either lies on bounded traversals or forces the walk past the budget on every call. Both are worse than the current bool |
| Import ripwire's ranking confidence + margin | Strong buyer language; proven in their product | There is no ranker to attest to. The field would be decoration at best and a false precision signal at worst |
| Do nothing | Zero cost; surface is shipped and working | Leaves `impact_of_change` — the answer with the highest stakes — the least auditable thing Anvil returns |

## Consequences

- **Positive:** the blast-radius answer becomes reviewable evidence rather than
  an opaque assertion — a reviewer can see which edges were inferred and where
  the walk stopped. An assistant can route on the disclosure: page when bounded,
  treat an inferred edge as unconfirmed, budget before spending. Exported
  receipts state the map's limits instead of implying completeness. A future
  tool inherits disclosure by construction.
- **Negative:** every projection DTO grows, and every consumer that
  exhaustively matches on them must be updated. `truncated` is carried
  redundantly for one release. A per-edge marker is more data on the wire than a
  per-caller one for callers with wide fan-out.
- **Risks:**
  - *Disclosure that never fires.* A bound list that is always empty, or an
    `est_tokens` that is always attached but never right, launders a guess into
    apparent rigour — the failure mode this ADR is meant to prevent, reappearing
    inside its own solution.
  - *`total` as an inference channel.* Aggregate counts are a small but real
    signal about a workspace.
  - *Half-adoption.* Landing the block on three of six tools produces a surface
    where absence of bounds is ambiguous.
  - *Value gated upstream.* Honest disclosure over a graph that timed out during
    a full scan (CIB-341/342) tells the user precisely that the answer is bad.
    That is better than lying, and it is not a product.
- **Mitigations:**
  - Decision §7 — gates written first, each proven to fail when its subject is
    removed.
  - Decision §6 — counts taken post-CE-3-deny-list, so `total` cannot probe
    denied paths; the posture otherwise extends the `matched` count PV-9 already
    accepted rather than opening a new class.
  - Decision §1 — the shared type makes half-adoption a compile error rather
    than a review catch.
  - Sequencing: CIB-341/342 land before this work is scheduled, not after.

## References

- Related ADRs: ADR-084 (GCTX graph-handle access, degradation outcomes),
  ADR-086 (symbol call-graph substrate, the CALL-1 heuristic marker),
  ADR-095 (CLI as co-equal secondary surface), ADR-069 (GV2 persistence posture),
  ADR-075 (v0.8.0 graph product scope), ADR-063 (hot-path boundary)
- APS module: GATT (`plans/modules/graph-answer-attestation.aps.md`), GATT-001..006
- Spec: [`docs/architecture/graph-context-delivery-spec.md`](../../docs/architecture/graph-context-delivery-spec.md)
  (CE-1, CE-3, CE-5, CE-6, CE-7, CE-11)
- Review: [context-egress privacy review (PV-9)](../archive/reviews/2026-06-15-gctx-context-egress-privacy-review-verdict.md)
- Origin: [ripwire borrow assessment](../brainstorms/2026-09-07-ripwire-borrow-assessment.md)
- Upstream sequencing: CIB-341 / CIB-342 (full-scan timeout, cold serve) —
  [GCTX dogfood failure points](../../docs/archive/reviews/2026-08-16-gctx-dogfood-failure-points.md)
