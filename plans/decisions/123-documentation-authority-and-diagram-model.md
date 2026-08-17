# ADR-123: Documentation authority and diagram model

## Status

Accepted 2026-08-17 (operator). Executes DOCRB-001. Complements ADR-119 and
ADR-122; does not supersede them.

## Date

2026-08-17

## Context

anvil's documentation grew as a central corpus around a fast-moving monorepo.
Readers must cross architecture guides, dated as-built documents, package
READMEs, public pages, ADRs, and APS modules without a consistent rule for
where a concern lives. The result is predictable:

- component detail is remote from the code that invalidates it;
- several documents can appear authoritative for the same concern;
- reviews have no small, consistent diagram-impact trigger;
- Mermaid exists, but is concentrated in a few central files;
- Draw.io sources exist without committed accessible SVG exports;
- public documentation is organised as prose first, with little visual support;
- freshness checks (ADR-119) detect some movement but do not decide authority
  or diagram placement.

The operator-approved design is
[`plans/specs/2026-08-16-docs-rebaseline.md`](../specs/2026-08-16-docs-rebaseline.md).
This ADR is the durable contract. DOCRB-002 and later items execute the
inventory, pilots, migration, public pipeline, and enforcement.

A second, live gap sits in production hosting. `infra/src/vercel.ts` deploys
`docs-shell` at `docs.eddacraft.ai` and proxies `anvil-docs-private` and
`docs-public`. `apps/docs-site` is retained only for rollback. The DSITE module
still describes `apps/docs-site` as the shared host. That module remains the
owner of its recorded legacy work items; this ADR records the topology and the
ownership gap without changing DSITE status.

## Decision

Adopt a hybrid documentation model with one authority per concern.

### 1. Authority homes

| Concern | Authoritative home | Expected form | Diagram format |
| ------- | ------------------ | ------------- | -------------- |
| Component purpose, entry points, ownership, local development | Component-root `README.md` | Short orientation with links to deeper authority | Mermaid only when a small map materially helps |
| Component internals, invariants, local data/control flow | Component-root `ARCHITECTURE.md` | Source-linked as-built explanation | Mermaid source inline |
| Cross-component context, containers, dependencies, trust or deployment boundaries | `docs/architecture/**` | C4-like views and cross-system explanation | Mermaid for engineering views; Draw.io plus SVG where public-facing |
| Durable architectural decision | `plans/decisions/**` plus decision-log entry | ADR | Mermaid only when the decision needs a flow or state model |
| Planned work and execution state | `plans/modules/**` and `plans/specs/**` | APS module/work item and approved design source | Mermaid only when sequence or dependency structure needs it |
| Operator procedure | `docs/runbooks/**` | Executable runbook | Mermaid for non-trivial operational sequence/state |
| Public learning and use | `docs/public/**` | Diátaxis tutorial, how-to, reference, or explanation | Curated Draw.io source plus accessible SVG export |

Authority is about a concern, not a file type. A document may link to another
authority. It must not restate that authority as a second source of truth.

Existing central `*-as-built.md` files remain derived implementation maps until
DOCRB-005 migrates component-internal truth beside code. After migration, a
central as-built file may remain only when it owns a genuine cross-system
concern.

### 2. Internal Mermaid

Use Mermaid beside the prose and code it explains for:

- module or crate boundaries;
- request, event, data, and control flow;
- state machines and lifecycle transitions;
- dependency direction and extension points;
- trust boundaries and failure/fallback paths;
- sequences that are difficult to verify from prose alone.

Every internal diagram must name the concern it owns, link important nodes to
source paths in adjacent prose, avoid duplicating a central view, and remain
small enough to review as text. Syntax must render in the repository's
supported Mermaid toolchain.

### 3. Public Draw.io and SVG

Use Draw.io for public diagrams where layout, typography, visual grouping, or
brand consistency materially improves comprehension. Each governed public
diagram commits:

- the editable `.drawio` source;
- a deterministic `.svg` export with the matching base name;
- alt text or an adjacent textual explanation of the same meaning;
- ownership and upstream metadata through the containing document;
- a parity check that fails when source and export no longer match the
  approved export process.

SVG is the default public export. Raster export is an explicit exception for a
consumer that cannot use accessible SVG. Silent raster-only or stale-export
drift is not allowed.

### 4. One diagram per concern

Each concern has one authoritative diagram. A second audience receives a
linked or deliberately simplified view, not a copied diagram that can silently
drift. DOCRB-002 records the relationship when both an engineering view and a
public view are justified.

### 5. Change-coupled maintenance, phased enforcement

A code or contract change reviews its documentation and diagram impact in the
same change when it:

- adds, removes, renames, splits, or merges a component;
- changes a dependency direction, public contract, trust boundary, deployment
  boundary, state transition, or material data/control flow;
- changes a user workflow or system behaviour shown in a public diagram;
- changes the authority or lifecycle state of a depicted surface.

It is normally not required for internal refactors inside an unchanged
boundary, tests that only add coverage, or prose-only repairs that do not
alter the depicted concern.

Enforcement is phased:

1. **Advisory baseline.** This ADR plus DOCRB-002 establish authority and
   ownership. DOCRB-003 adds the thin `AGENTS.md` trigger.
2. **Representative pilots.** DOCRB-004 proves the co-located model.
3. **Migration and public pipeline.** DOCRB-005..008 move authorities, remove
   duplicates, and establish Draw.io-to-SVG parity.
4. **Mandatory review.** DOCRB-009 activates enforceable, low-noise checks
   only after the canonical map and tooling exist.
5. **Independent verification.** DOCRB-010 tests navigation, accuracy,
   accessibility, and maintainability from a clean checkout.

Until DOCRB-009, the diagram-impact rule is advisory. Do not fail CI for
missing diagram updates under this ADR.

### 6. Module boundaries

| Surface | Owns | Does not own |
| ------- | ---- | ------------ |
| **DOCRB** | Authority model, corpus disposition, diagram topology, co-location migration, public diagram source/export pipeline, activation of the new review rule | Sibling-module work-item status; release claims |
| **DOCFRESH** | Declared-upstream freshness (ADR-119) and release-boundary freshness | Authority or diagram placement |
| **DOCSYNC** | Substantive public content and release-aligned public refreshes | Public information architecture and diagram conventions |
| **DSITE** | Recorded legacy `apps/docs-site` host and section-wiring work items | Live production topology (see below) |
| Archived **DOCGOV** | Historical evidence for current governance | Living rules; do not rewrite or reopen it |

Coordination means linking and sequencing. It does not absorb, close, or
alter sibling-module status.

### 7. Production docs topology and ownership gap

Live production, as deployed by `infra/src/vercel.ts`:

- `apps/docs-shell` is the public entrypoint at `docs.eddacraft.ai`. It
  authenticates `/anvil/*` and proxies to the two upstream Docusaurus apps.
- `apps/anvil-docs-private` serves gated Anvil/beta docs.
- `apps/docs-public` serves public APS, Kindling, edda-stack, and blog docs.
- `apps/docs-site` is retained only for rollback. Its domain has moved to
  `docs-shell`. It is not the live public renderer.

Ownership gap: DSITE still describes `apps/docs-site` as the shared host and
remains the owner of those recorded work items. No successor DSITE item is
opened here. Until DSITE or a later bookkeeping change adopts the live
topology, **implementation truth** for what is deployed is
`infra/src/vercel.ts`, and **documentation authority** for the topology is
this ADR plus the production-host records in
`docs/architecture/README.md` and
`docs/guides/documentation-governance.md`.

### 8. Release posture

DOCRB is a high-priority engineering-effectiveness programme. It is not a
release claim, readiness gate, or release-cut dependency. A release may
consume completed DOCRB improvements. Release records must not imply the
re-baseline is complete until DOCRB-010 is terminal.

## Rationale

The hybrid model follows established practice without treating one framework
as a complete answer: C4 for diagram abstraction levels, arc42 for
maintainable architecture-document structure, Diátaxis for public information
architecture, Mermaid for reviewable internal diagrams, and Draw.io plus SVG
for polished public views.

Keeping component truth beside code is the only arrangement that makes
change-coupled review cheap. Centralising every as-built in
`docs/architecture/**` is what produced the current drift. Keeping
cross-system views central prevents every crate README from reinventing the
system map.

Mermaid-only would make public diagrams harder to brand and lay out.
Draw.io-only would make internal diagrams expensive to review in pull
requests. Pairing them by audience is the cheaper split.

Phased enforcement avoids a high-noise gate on a corpus that has not yet
been inventoried or migrated.

### Alternatives Considered

| Option | Pros | Cons |
| ------ | ---- | ---- |
| **Chosen: hybrid authority with phased enforcement** | Matches how the repo actually changes; one authority per concern; low-noise until tooling exists | Migration cost; temporary dual homes during DOCRB-005 |
| Keep expanding central as-builts | No file moves | Component truth stays remote; reviews stay expensive |
| Mermaid for every audience | One toolchain; easy PR review | Weak public layout/brand; no committed polished export |
| Draw.io for every audience | Precise layout | Poor PR review for internal changes; source/export drift already exists |
| Mandatory diagram CI now | Immediate pressure | High false-positive noise before inventory and exemptions exist |
| Treat DSITE as already owning `docs-shell` | One owner on paper | Rewrites sibling-module status by implication |

## Consequences

- **Positive:** Contributors have one rule for where truth lives. Diagram
  format follows audience. Freshness (ADR-119) and copied-section attestation
  (ADR-122) keep their jobs. Production hosting is stated honestly.
- **Negative:** Until DOCRB-005, some component truth remains in central
  as-builts. Readers must follow links. The DSITE ownership gap stays visible
  rather than silently closed.
- **Risks:** Agents may treat this ADR as a licence to rewrite the corpus in
  one change, or as a mandatory CI gate before DOCRB-009.
- **Mitigations:** Expected outcomes for later DOCRB items name the migration
  batches. This ADR forbids mandatory enforcement and release gating. Feature
  PRs do not edit DSITE status.

## References

- Related ADRs: ADR-119, ADR-122, ADR-042, ADR-117
- APS modules: DOCRB, DOCFRESH, DOCSYNC, DSITE; archived DOCGOV
- Design source: [`plans/specs/2026-08-16-docs-rebaseline.md`](../specs/2026-08-16-docs-rebaseline.md)
- Production deploy: `infra/src/vercel.ts`
- Living guides: `docs/guides/documentation-governance.md`,
  `docs/guides/architecture-diagrams.md`
