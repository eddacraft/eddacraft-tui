# Documentation Governance

| Type  | Authority     | Owner | Status | Freshness                                                                                                                                                    |
| ----- | ------------- | ----- | ------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Guide | Authoritative | DOCRB | Live   | Manually reviewed 2026-08-20 at `97899b00a` against ADR-123, `infra/src/vercel.ts`, and the 2026-08-17 DOCRB corpus disposition; no semantic change required |

| Upstream                                                                                                                                                                       | Downstream                                                                                     |
| ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ---------------------------------------------------------------------------------------------- |
| ADR-123, ADR-119, ADR-122, `plans/specs/2026-08-16-docs-rebaseline.md`, `plans/specs/2026-08-17-docrb-corpus-disposition.md`, `plans/project-context.md`, `plans/aps-rules.md` | `docs/README.md`, `docs/guides/README.md`, `docs/guides/architecture-diagrams.md`, `AGENTS.md` |

Documentation is operational knowledge for humans and agents. It exists to make
engineering behaviour deterministic: what to read, what to trust, what to
update, and what must be verified before work is closed.

This guide was seeded by the archived `DOCGOV` APS module. **DOCRB** now owns
the living authority and diagram contract (ADR-123). AICON still owns
agent-contract routing for `AGENTS.md`. Future executable improvements stay in
APS rather than being copied into agent adapters.

## Authority Model

Authority is about a **concern**, not a file type. Each concern has one home.
Other documents link to that home instead of restating it.

| Question                                | Authoritative source                                                                |
| --------------------------------------- | ----------------------------------------------------------------------------------- |
| What work is authorised?                | APS work item in `plans/modules/*.aps.md`                                           |
| What work is active or planned?         | `plans/index.aps.md`                                                                |
| Why was an architectural choice made?   | ADR in `plans/decisions/`                                                           |
| What is actually implemented?           | Code, schemas, tests, generated artefacts                                           |
| How does this component work locally?   | Component-root `README.md` (orientation) and `ARCHITECTURE.md` (internals, ADR-123) |
| How do several components fit together? | Cross-system views in `docs/architecture/**`                                        |
| How is a system operated?               | Runbook in `docs/runbooks/`                                                         |
| How should developers work?             | Guide in `docs/guides/`                                                             |
| What do users see?                      | Public docs in `docs/public/**`, organised by Diátaxis intent                       |
| What happened historically?             | Archive or release evidence                                                         |
| What is deployed for docs hosting?      | `infra/src/vercel.ts`, summarised below                                             |

No document should duplicate another document's authority. Link to the upstream
source instead.

Until DOCRB-005 migrates component-internal truth beside code, existing central
`*-as-built.md` files remain the derived implementation maps they are today.
After migration, a central as-built remains only when it owns a genuine
cross-system concern.

## Document Types

| Type              | Purpose                                                 | Location                                                                                                                                                                                           |
| ----------------- | ------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| APS index         | Module discovery and active state                       | `plans/index.aps.md`                                                                                                                                                                               |
| APS module        | Execution authority                                     | `plans/modules/*.aps.md`                                                                                                                                                                           |
| Release plan      | Current release-slate summary                           | `RELEASE-PLAN.md`                                                                                                                                                                                  |
| ADR               | Durable decision rationale                              | `plans/decisions/*.md`                                                                                                                                                                             |
| Spec              | Intended design before or during work                   | `plans/specs/`, `docs/specs/`                                                                                                                                                                      |
| As-built          | Current cross-system implementation map                 | `docs/architecture/*-as-built.md` (component internals move to `ARCHITECTURE.md` under DOCRB-005)                                                                                                  |
| Architecture      | Component-internal as-built                             | Component-root `ARCHITECTURE.md` (ADR-123; pilots in DOCRB-004)                                                                                                                                    |
| Runbook           | Operational procedure                                   | `docs/runbooks/*.md`                                                                                                                                                                               |
| Guide             | Developer practice and operational policy               | `docs/guides/*.md`, `docs/policies/*.md`                                                                                                                                                           |
| README            | Local orientation                                       | nearest package, crate, app, or directory                                                                                                                                                          |
| Contributor guide | Contribution workflow and expectations                  | `CONTRIBUTING.md`                                                                                                                                                                                  |
| Public docs       | User-facing tutorial, how-to, reference, or explanation | `docs/public/**/*.md` (published at `docs.eddacraft.ai` via `docs-shell`; internal docs MUST NOT link into the public surface for navigation, and the public surface owns its own discovery layer) |
| Archive           | Historical reference                                    | `docs/archive/`, `plans/archive/`                                                                                                                                                                  |

## Canonical Folder Layout

Live documentation is organised by **Type**: a document's directory follows the
`Location` column above, so placement is mechanical once the Type is known.
Authority, owner, and status are _not_ encoded in the folder tree — they are
declared in each document's metadata table and surfaced through the generated
indexes under `docs/indexes/` (`pnpm docs:index`). A new document lands in the
directory that matches its Type; `pnpm docs:check` validates the metadata and
the generated indexes keep discovery current without a manual index step.

| Directory                                                                 | Holds (Type)                                                                                                                             |
| ------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------- |
| `docs/architecture/`                                                      | Cross-system as-built maps (`*-as-built.md`) and architecture references. Component internals belong in component-root `ARCHITECTURE.md` |
| `docs/runbooks/`                                                          | Runbooks — operational procedure                                                                                                         |
| `docs/guides/`, `docs/policies/`                                          | Guides — developer practice and operational policy                                                                                       |
| `docs/specs/`                                                             | Specs — design intent retained in-repo                                                                                                   |
| `docs/public/`                                                            | Public docs — published to docs.eddacraft.ai; own discovery layer                                                                        |
| `docs/indexes/`                                                           | Generated discovery indexes (do not hand-edit)                                                                                           |
| `docs/governance/`                                                        | Governance surfaces: the authoritative hand-maintained tags catalogue (`tags-catalogue.md`) and the generated `docs-check.baseline.json` |
| `docs/vision/`, `docs/strategy/`                                          | Aspirational / scope guidance (Advisory unless declared otherwise)                                                                       |
| `docs/observability/`, `docs/testing/`, `docs/internal/`, `docs/reviews/` | Role-specific operational records                                                                                                        |
| `docs/archive/`                                                           | Historical reference (not a live path)                                                                                                   |

`docs/README.md` is the only Markdown file at the `docs/` root — it is the human
entrypoint into the tree. The one deliberate exception to type-by-directory
placement is `docs/guides/runbook-template.md`, which carries `Type: Runbook`
because it is the authoring template for runbooks but lives under `guides/`
alongside the other authoring templates.

Component `README.md` and `ARCHITECTURE.md` files live beside their crate,
package, or app — they are not filed under `docs/`.

## Component documentation standard

Use the source-pinned disposition in
`plans/specs/2026-08-17-docrb-corpus-disposition.md` to decide whether a
component needs both files, a README only, or an explicit exemption. The
inventory is not an allowlist: adding, removing, renaming, splitting, or merging
any component requires a fresh classification.

A new or newly discovered component absent from the pinned inventory defaults to
`component-doc required`. Classify it as `README only` only when orientation
fully explains a narrow leaf surface. `grouping-only`, `generated/vendor`,
`historical`, and `explicit exemption` require evidence in the change-impact
disposition; absence from the inventory is never an exemption. Do not create an
empty `ARCHITECTURE.md` merely to satisfy a filename convention.

| File                             | Role                                                                                          | Required shape                                                                                                                                                                             | Diagram posture                                                                                   |
| -------------------------------- | --------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------- |
| Component-root `README.md`       | Orient a contributor before they read internals                                               | Purpose and scope; owner; supported entry points; local development and validation; links to deeper authorities, including `ARCHITECTURE.md` when present                                  | Add a small Mermaid map only when it materially improves orientation                              |
| Component-root `ARCHITECTURE.md` | Explain source-linked, as-built internals when the inventory requires component documentation | Boundaries and dependencies; invariants; material data/control flow; trust, failure, and fallback behaviour; links to source paths and governing ADRs; links to central cross-system views | Keep maintainable Mermaid source inline when a diagram makes those relationships easier to verify |

Both files follow the [metadata convention](#metadata-convention), cite current
source or contracts as upstreams, and link rather than copy another authority.
Keep cross-component context, deployment topology, and shared trust boundaries
in `docs/architecture/**`. A README-only or exempt disposition is valid when
there are no component internals that warrant a separate as-built explanation.

## Production docs topology

Live production, deployed by `infra/src/vercel.ts`:

| App                       | Role                                                                                                             |
| ------------------------- | ---------------------------------------------------------------------------------------------------------------- |
| `apps/docs-shell`         | Public entrypoint at `docs.eddacraft.ai`. Authenticates `/anvil/*` and proxies the two upstream Docusaurus apps. |
| `apps/anvil-docs-private` | Gated Anvil and beta docs.                                                                                       |
| `apps/docs-public`        | Public APS, Kindling, edda-stack, and blog docs.                                                                 |
| `apps/docs-site`          | Legacy Docusaurus host retained for rollback only. Domain moved to `docs-shell`.                                 |

**Ownership gap:** the DSITE module still describes `apps/docs-site` as the
shared host and remains the owner of those recorded work items. DOCRB records
the live topology here and in ADR-123. It does **not** change DSITE status or
open a successor DSITE item. Until DSITE or a bookkeeping change adopts the live
topology, implementation truth is `infra/src/vercel.ts`.

## Change-impact review

A code or contract change reviews documentation and diagram impact in the same
change when it matches a trigger below. A triggered review does not
automatically require a documentation edit: the disposition is either an update
to the authoritative document or diagram, or a brief explanation that the
authoritative concern is unaffected.

| Change                                                                                                                            | Documentation review                                                        | Diagram review                                                                             |
| --------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------ |
| Add, remove, rename, split, or merge any component or documentation unit, including one absent from the pinned inventory          | Classify it, then review its orientation, authority, owner, and cross-links | Review the authoritative component or cross-system view when it depicts that topology      |
| Change dependency direction, a public contract, a trust or deployment boundary, a state transition, or material data/control flow | Review the authoritative component or cross-system explanation              | Review the authoritative diagram when its nodes, edges, boundaries, states, or flow change |
| Change a user workflow or system behaviour shown publicly                                                                         | Review the authoritative public tutorial, how-to, reference, or explanation | Review the public diagram that depicts the workflow or behaviour                           |
| Change the authority or lifecycle state of a depicted surface                                                                     | Review metadata, discovery, and links to the authority                      | Review the diagram's labels or disposition                                                 |

Change type alone is never an exemption. Review is normally not required for an
internal refactor, coverage-only test, generated or formatting-only change, or
prose repair only when it leaves every documented or depicted concern unchanged.
That includes observable behaviour, public contracts, security invariants,
trust, failure and fallback behaviour, state or lifecycle, and material
data/control flow.

A bug fix that changes any of those concerns receives the same review as any
other behaviour change. For generated output, review the upstream contract and
rendered result; for formatting-only changes, confirm that semantics and diagram
meaning are unchanged.

This review is **advisory** until DOCRB-009. Do not add a mandatory CI gate or
fail CI for a missing diagram update under ADR-123 before that item. Reviewers
may still request an update when a trigger exposes inaccurate authoritative
documentation.

## Diagram policy

See [`architecture-diagrams.md`](architecture-diagrams.md) for inventory and
diagram-specific update procedure. The contract (ADR-123) is:

- **Internal / component:** Mermaid beside the prose and code it explains.
- **Public:** curated Draw.io source plus committed accessible SVG export, with
  alt text or adjacent equivalent prose, and source-export parity.
- **One diagram per concern.** A second audience gets a linked or simplified
  view, not a silent copy.

## Module boundaries

| Surface         | Owns                                                                                                                                                                     | Does not own                                                           |
| --------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ---------------------------------------------------------------------- |
| DOCRB           | Authority model, corpus disposition, diagram topology, co-location migration, public diagram pipeline, live public IA/nav (DOCRB-011), activation of the new review rule | Sibling-module work-item status; release claims; definition-page prose |
| DOCFRESH        | Declared-upstream freshness (ADR-119) and release-boundary freshness                                                                                                     | Authority or diagram placement                                         |
| DOCSYNC         | Substantive public content and release-aligned public refreshes                                                                                                          | Public information architecture and diagram conventions                |
| DOCDEF          | Public Anvil definition content and the public-reference generator (evaluation model, check/config/CLI catalogues, policy model)                                         | Live sidebar; public Draw.io diagrams; DOCSYNC journeys                |
| DSITE           | Recorded legacy `apps/docs-site` host and section-wiring work                                                                                                            | Live production topology (above)                                       |
| Archived DOCGOV | Historical evidence                                                                                                                                                      | Living rules — do not rewrite or reopen it                             |

DOCRB is not a release claim, readiness gate, or release-cut dependency.

## Metadata Convention

New documents and materially touched non-APS documentation must declare their
governance metadata immediately after the H1 title. Legacy live documents may
remain on the validation baseline until DOCGOV-009 backfills metadata, but do
not add new missing-metadata debt to live docs.

APS modules, APS indexes, and ADRs keep their native metadata formats unless
their own schema or process explicitly adopts this table. Do not add this table
to APS files just because their plan state changed.

Use this compact table immediately after the title:

```markdown
| Type  | Authority     | Owner                       | Status | Freshness                                            |
| ----- | ------------- | --------------------------- | ------ | ---------------------------------------------------- |
| Guide | Authoritative | APS module, team, or handle | Live   | Last reviewed YYYY-MM-DD against tag/SHA/source path |

| Upstream                                         | Downstream                                          |
| ------------------------------------------------ | --------------------------------------------------- |
| Canonical source(s) this doc must not contradict | Docs, tooling, or workflows that depend on this doc |
```

Keep values short and link to canonical sources when useful. For active non-APS
docs, fill every field in the table; do not replace the table with prose
elsewhere in the document. Archive-only documents may omit `Downstream` when no
live document depends on them.

### Field Meanings

| Field      | Meaning                                                                         |
| ---------- | ------------------------------------------------------------------------------- |
| Type       | The document type from the table above                                          |
| Authority  | Whether the document is source-of-truth, derived, advisory, or historical       |
| Owner      | Who keeps the document correct; prefer APS module IDs for active work           |
| Status     | Current lifecycle state of the document itself                                  |
| Freshness  | Review date plus tag, SHA, source path, release, or other check anchor          |
| Upstream   | Documents, code, schemas, tests, release records, or ADRs this doc derives from |
| Downstream | Documents, generated indexes, runbooks, agents, or workflows that read this doc |

### Status Values

| Value      | Use when                                                  |
| ---------- | --------------------------------------------------------- |
| Draft      | Content is being shaped and is not yet safe as guidance   |
| Proposed   | Direction is reviewed but not yet operational authority   |
| Ready      | Content is approved for use but not yet live practice     |
| Live       | Content is current operational guidance or discovery      |
| Deprecated | Content is stale or superseded but remains in active path |
| Archived   | Content is historical reference only                      |

### Authority Values

| Value         | Use when                                                                     |
| ------------- | ---------------------------------------------------------------------------- |
| Authoritative | The document owns the answer for its declared scope                          |
| Derived       | The document summarises or maps implementation truth from upstream sources   |
| Advisory      | The document offers practice guidance but must defer to stronger sources     |
| Historical    | The document is preserved for context and should not be edited as live truth |

### Freshness Rules

- **As-built docs:** cite a tag or SHA and source paths reviewed.
- **Runbooks:** cite the last successful dry-run, release, incident, or command
  review, plus executable source paths where the procedure depends on scripts or
  command surfaces.
- **Guides:** cite the upstream rule, APS item, ADR, or source path reviewed.
- **Public docs:** cite the release or product version they describe.
- **Archives:** cite the superseding document or archive date.

When freshness cannot be established, mark the document `Status: Deprecated` or
track the gap in APS instead of leaving it ambiguous.

### Source-Reference Validation

`pnpm docs:check` validates source references for governed `As-built` and
`Runbook` documents through the `asbuilt-paths` surface. The validator reads the
metadata table, requires a `YYYY-MM-DD` freshness date, extracts
backtick-wrapped repository paths from freshness/upstream/downstream/body
references, and checks that each path resolves in the repository. Markdown
anchors are allowed and are resolved to the owning file; placeholder paths using
angle brackets are treated as examples and ignored.

Use `docs/architecture/_as-built-template.md` when authoring component-root
`README.md` and `ARCHITECTURE.md` files, and `docs/guides/runbook-template.md`
for operational procedures.

## Docs Workflow Skill Shape

A future `docs-workflow` skill should be a router, not a bureaucracy layer. It
should classify the request, load the right rules, and require closeout.

| Intent                     | Route                                                    |
| -------------------------- | -------------------------------------------------------- |
| Planning or execution docs | APS rules and module state                               |
| Architecture change        | Decision log, ADR need, as-built impact                  |
| ADR work                   | Numbering, status, supersession, decision-log entry      |
| As-built update            | Source references, gaps, APS/ADR/runbook links           |
| Runbook update             | Owner, trigger, commands, success/failure, rollback      |
| Guide update               | Audience, lifecycle, authority, upstream source          |
| Public docs                | Release/version alignment and user-facing behaviour      |
| Release docs               | Evergreen guide versus version-specific runbook evidence |
| Archive or retirement      | Supersession, stale markers, index updates               |
| Validation                 | Links, APS, ADRs, metadata, source references            |

The skill must answer three questions before editing:

1. What type of document is this?
2. What authority does it have?
3. What upstream source must it not duplicate or contradict?

## Closeout Protocol

Closeout is mandatory for documentation-affecting work. It prevents agents from
doing the visible edit and skipping hygiene.

Before final response, check:

- **Classification:** each changed document has an understood type and
  authority.
- **APS alignment:** active work is tracked in APS, and `plans/index.aps.md` is
  updated when module status or progress changes.
- **ADR alignment:** architecture decisions update ADRs and
  `plans/decisions/DECISION-LOG.md` when needed.
- **As-built alignment:** implementation claims cite code, schema, config, test,
  release, or generated artefacts.
- **Runbook alignment:** operational docs include executable commands, success
  output, failure modes, and rollback or safety notes.
- **Index alignment:** local README indexes and documentation maps still point
  to the authoritative entrypoints.
- **Stale-state handling:** stale or superseded information is marked inline,
  archived, fixed, or tracked in APS.
- **Validation:** relevant checks are run, or skipped with a reason.

Final responses for documentation changes should include a short closeout note:

```markdown
Docs Closeout:

- Authority checked: yes
- Indexes updated: yes/not needed
- Cross-links checked: yes
- Validation: <command or reason skipped>
- Residual drift risk: <none or short note>
```

## Minimal Validation Baseline

Until dedicated tooling exists, use the smallest relevant check set:

| Change         | Minimum validation                                             |
| -------------- | -------------------------------------------------------------- |
| Markdown only  | `pnpm format:check` plus manual link/source check              |
| APS files      | `pnpm format:check` plus manual status/progress reconciliation |
| ADR files      | `pnpm format:check` plus decision-log and numbering check      |
| Public docs    | `pnpm format:check` plus release/version source check          |
| Package README | `pnpm format:check` plus link/source path check                |
| Runbook        | `pnpm format:check` plus command review                        |

DOCGOV is replacing these manual checks with explicit validators over time.

## Automated Indexing Requirement

Documentation indexes must be generated, not manually maintained. The only
manual indexing input should be document-local metadata and, when needed, adding
a new approved tag to the tag catalogue.

Current generated-index flow:

```text
document metadata -> docs:index -> docs/indexes/ -> docs:check freshness check
```

The generator:

- scans governed documentation sources
- parses document-local metadata
- infers safe fields such as title and path
- generates `docs/indexes/` by type, authority, owner, status, and tag
- rejects unknown tags unless they exist in the approved tag catalogue
- relies on `pnpm docs:check` metadata validation for required metadata
- fails CI when generated indexes are stale

Required commands:

```bash
pnpm docs:index        # regenerate generated indexes
pnpm docs:index:check  # fail if generated indexes are stale
pnpm docs:check        # metadata, tags, links, and index freshness
```

Generated indexes must be marked as generated and must not contain hand-written
authority prose. They are discovery surfaces over canonical documents, not a new
source of truth.

## Drift Rule

Known documentation drift must not be left as tribal knowledge. Resolve it in
the same change, mark it stale, or create/link an APS work item.
