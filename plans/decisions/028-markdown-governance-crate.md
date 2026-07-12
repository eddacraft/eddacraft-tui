# ADR-028: Markdown Governance Crate Location

## Status

Accepted (2026-04-26)

> Accepted at minimum bar per Council D recommendation; refinements to land on first implementation.
>
> **Rationale strengthened (2026-04-29) by [ADR-033](./033-park-ide-mcp-retire-ts-scanner.md).**
> This ADR's Rationale referred to the TS scanner as
> "soon-to-be-retired". Under ADR-033 the TS scanner is retired
> outright (the in-process surfaces that justified it are archived
> under `archive/`). The decision below is unchanged; the "do not
> anchor new analysis in retiring TS code" argument is reinforced,
> not revised.

## Date

2026-04-22

## Context

The [2026-04-08 Language and Coverage Design](../specs/2026-04-08-language-and-coverage-design.md)
introduces Track 5 — Markdown governance — with the [`markdown-governance`](../modules/markdown-governance.aps.md)
module. The council review (§16.5 #11, finding C-017) explicitly flagged
that this work **does not belong in the Rust kernel**: the kernel is for
parsing programming languages into symbol graphs, and markdown does not
fit that abstraction. Either it forces a markdown fast-path inside the
parser, or it adds `tree-sitter-markdown` as another grammar dependency.
Neither is acceptable.

The decision needs to be made before MDGOV moves past Draft — its Ready
Checklist names this ADR as the gating prerequisite.

The two viable options are a standalone Rust crate or the existing TS
layer. The decision is mostly forced: ADR-026 already establishes that the
Rust scanner is the authoritative analysis surface and that the TS scanner
exists only for in-process IDE/MCP surfaces until a napi-rs migration
retires it. New analysis surfaces should not anchor in TS that is being
retired.

## Decision

Markdown governance lives in a **standalone Rust crate**:
`crates/anvil-markdown-governance/`.

- The crate consumes the existing scanner infrastructure (suppression
  parser, drift baseline, check registry from
  [OPSUP](../archive/modules/operational-supplement.aps.md)) but does **not** live
  inside `crates/anvil-kernel/`.
- The crate uses an existing markdown parser dependency (`pulldown-cmark`
  is the current Rust ecosystem default) — **not** `tree-sitter-markdown`.
  The kernel's tree-sitter parser pool stays focused on programming
  languages.
- M1 capabilities (APS wellformedness, cross-reference integrity,
  decision-record hygiene) live in this crate. M2/M3 (stale-claim
  detection, capability-aware) will extend the same crate when promoted.
- The crate is registered as a check source through OPSUP's check
  registry, the same way Track 3 surface checks are registered. From the
  gate runner's perspective, markdown governance is just another check
  source — it does not get a special integration path.

## Rationale

The "not the Rust kernel" constraint (council C-017) is non-negotiable —
the spec accepts this and the design's intent is clear. Of the two viable
hosts:

- A **standalone Rust crate** keeps the analysis on the authoritative
  side of ADR-026, reuses scanner/suppression/drift infrastructure
  without forcing a TS round-trip, and avoids re-anchoring new work in
  the soon-to-be-retired TS scanner.
- The **TS layer** would re-anchor new analysis work in code ADR-026
  is actively retiring. Choosing TS now would create a migration debt the
  moment napi-rs lands.

`pulldown-cmark` is preferred over `tree-sitter-markdown` because the
kernel's tree-sitter parser pool is sized and tuned for programming
languages (council C-026 thread-safety concerns, council C-005 grammar
maturity audits). Putting markdown into that pool is exactly the
"markdown fast-path inside the parser" failure mode council C-017 calls
out. `pulldown-cmark` is a streaming parser used in Rust documentation
tooling (`mdBook`, `rustdoc`) — proven, dependency-light, and disjoint
from the tree-sitter pool.

### Alternatives Considered

| Option | Pros | Cons |
|--------|------|------|
| **Standalone Rust crate `crates/anvil-markdown-governance/`** *(chosen)* | On the authoritative analysis side of ADR-026; reuses existing Rust scanner infra; disjoint from kernel parser pool | New crate to maintain; markdown analysis is genuinely different from code analysis (no symbol graph) |
| TS layer in `packages/anvil/core/src/markdown/` | Closer to existing APS-shaped TS code; some markdown reasoning already exists in the `aps-planning` skill | Re-anchors new analysis work in the TS scanner that ADR-026 is retiring; creates a migration debt the moment napi-rs lands |
| Inside `crates/anvil-kernel/` with `tree-sitter-markdown` | Reuses kernel infrastructure | Council C-017 explicitly rejects this; pollutes parser pool with a non-code grammar; council C-005 grammar maturity concern applies |
| Inside `crates/anvil-kernel/` with a markdown fast-path | Avoids tree-sitter pollution | Council C-017 explicitly rejects this; fast-paths in the kernel are the "by accident" anti-pattern the council called out |

## Consequences

- **Positive:**
  - Markdown analysis stays disjoint from the programming-language parser
    pool — no thread-safety or grammar-maturity entanglement.
  - On the authoritative analysis side of ADR-026.
  - Reuses the same scanner/suppression/drift infrastructure other
    Rust-side checks use.
  - M2/M3 extensions land in the same crate without architectural rework.

- **Negative:**
  - One more crate in the workspace.
  - The `aps-planning` skill (which currently has markdown logic in TS)
    eventually wants to read the same wellformedness rules — that means
    either a future port to the new crate or a duplication-with-a-reason
    (skill is dev-tooling, governance is product). Accept duplication for
    now; revisit if it bites.

- **Risks:**
  - `pulldown-cmark` may not handle every flavour of GFM / MDX —
    acceptable for M1 (APS files use a narrow subset of markdown).
  - The crate becomes a magnet for unrelated markdown work over time
    (council C-016 "markdown linter" risk applies at the architecture
    level too). Mitigation: M1 acceptance bar wording stays strict.

- **Mitigations:**
  - M1 scope is fixed by the MDGOV module — no scope-creep into prose
    quality, rendering correctness, or NLP.
  - The crate exposes its check rules through the OPSUP registry;
    additions go through the same governance as other check sources.

## References

- Related ADRs: ADR-026 (Rust scanner authoritative — load-bearing for
  this decision), ADR-027 (Pack architecture — establishes the per-domain
  crate pattern this ADR follows)
- Spec: [2026-04-08 Language and Coverage Design](../specs/2026-04-08-language-and-coverage-design.md)
  §8.5, §16.5 #11, council finding C-017
- APS modules: [markdown-governance](../modules/markdown-governance.aps.md)
  (this ADR is its Ready Checklist gating prerequisite),
  [operational-supplement](../archive/modules/operational-supplement.aps.md)
  (check registry that the new crate registers against)
- External: [pulldown-cmark](https://github.com/raphlinus/pulldown-cmark) —
  the markdown parser this crate adopts
