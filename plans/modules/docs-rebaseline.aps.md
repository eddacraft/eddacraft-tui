<!-- APS: See https://github.com/eddacraft/anvil-plan-spec for format reference -->
<!-- Executable only if work items exist and status is Ready. -->

# Documentation Re-baseline

| ID | Owner | Priority | Status | Progress |
| -- | ----- | -------- | ------ | -------- |
| DOCRB | — | high | In Progress | 10/11 |

**Last reviewed:** 2026-08-23 against DOCRB-009 rebase-merge receipt
`93f543e671b0ddcf4ec0481eaf1539a3006a9f82`, DOCRB-008 rebase-merge
receipt `182d77b6329f460f4635e3e946b329ff9af84445`, the repository
component/documentation inventory at `0a0f00c20`, current
DOCFRESH/DOCSYNC/DSITE/DOCDEF ownership, the operator-approved
[documentation re-baseline design](../specs/2026-08-16-docs-rebaseline.md),
and the
[definition-layer design](../specs/2026-08-19-anvil-docs-definition-layer.md)
(DOCRB-011 live-nav split).

> **Exclusive module.** DOCRB owns the documentation-authority and diagram
> re-baseline. Feature PRs update only their own item status and evidence;
> stored progress counts are reconciled separately under ADR-053.

## Purpose

Replace anvil's difficult-to-navigate, centralised, and inconsistently
diagrammed documentation model with one maintainable authority system:

- internal component truth beside code, with Mermaid diagrams where useful;
- central architecture documentation limited to cross-system views;
- public documentation organised by reader need, with polished Draw.io diagrams
  and accessible SVG exports;
- one authoritative document and diagram per concern;
- diagram-impact review coupled to relevant code and contract changes.

This module owns the re-baseline programme, not every future documentation edit.

## Release Posture

DOCRB is a **high-priority engineering-effectiveness programme**. It is not part
of a release claim set, does not gate release readiness or a release cut, and
may progress independently of release work. `RELEASE-PLAN.md` remains unchanged.

## Approved Model

The authoritative design source is
[`2026-08-16-docs-rebaseline.md`](../specs/2026-08-16-docs-rebaseline.md).
Its governing decisions are:

1. component `README.md`/`ARCHITECTURE.md` files own local truth;
2. internal diagrams use inline Mermaid beside code;
3. `docs/architecture/**` owns only cross-component concerns;
4. public docs use Diátaxis information types and curated Draw.io views;
5. public diagram source and accessible SVG export are both committed and
   parity-checked;
6. every concern has one authoritative diagram;
7. the root `AGENTS.md` rule starts advisory and becomes mandatory only after
   DOCRB-009 establishes enforceable, low-noise checks.

## In Scope

- A complete inventory and disposition of component docs, central docs, public
  docs, diagrams, generated references, historical records, and exemptions.
- A durable authority contract, recorded through the ADR process.
- Component-local documentation standards and representative pilots.
- Migration of component truth out of central docs and retirement of duplicate
  authorities.
- Cross-system architecture and operational diagram re-baselining.
- Draw.io-to-accessible-SVG export, parity, render, and freshness checks.
- Public information-architecture and diagram refresh in coordination with
  DOCSYNC and DSITE.
- A thin root `AGENTS.md` diagram-impact trigger backed by the authoritative
  documentation-governance guide.
- Clean-room verification of navigation, accuracy, accessibility, and
  maintenance workflow.

## Out of Scope

- Product code changes unrelated to documentation enablement.
- A diagram for every component or change regardless of explanatory value.
- Replacing generated API/CLI references with hand-maintained prose.
- Restating ADR, APS, schema, test, or source authority in documentation.
- Reopening archived DOCGOV work or taking over DOCFRESH, DOCSYNC, or DSITE
  work-item status.
- Making DOCRB a release claim, readiness gate, or release-cut dependency.

## Interfaces and Coordination

| Module/surface | Boundary |
| -------------- | -------- |
| DOCFRESH | Owns declared-upstream freshness and release-boundary freshness checks; DOCRB supplies the new authority/diagram topology and coordinates any checker extension |
| DOCSYNC | Owns substantive public content and release-aligned refreshes; DOCRB owns public information architecture and diagram conventions/pipeline |
| DOCDEF | Owns public Anvil definition content and the public-reference generator; does not own the live sidebar |
| DSITE and production docs apps | DSITE records legacy `apps/docs-site` host/section wiring, but production is now `docs-shell` proxying `anvil-docs-private` and `docs-public`; DOCRB-001 must reconcile the live owner and contract without changing DSITE status by implication |
| DOCGOV (archived) | Historical source for current governance; DOCRB replaces living rules through new decisions and docs without editing archived status |
| Component owners | Own the accuracy of local README/architecture material after migration |
| `AGENTS.md` | Carries only the thin diagram-impact trigger and link; detailed procedure stays in documentation governance |

Coordination does not absorb, close, or alter the status of sibling-module work.

## Readiness and Sequencing

| Phase | Work items | Gate |
| ----- | ---------- | ---- |
| Authority | DOCRB-001 | Documentation placement, authority, formats, accessibility, phasing, and module boundaries are accepted |
| Baseline | DOCRB-002 | Every current component/document/diagram has a disposition and owner or explicit exemption |
| Advisory adoption | DOCRB-003, DOCRB-004 | Thin agent rule and representative co-located pilots are usable without a mandatory gate |
| Live public nav | DOCRB-011 | Live sidebar exposes already-written definition pages; nav check targets the live host |
| Migration and public pipeline | DOCRB-005..008 | Duplicate authorities are retired, cross-system views rebuilt, and public source/export parity works |
| Enforcement and verification | DOCRB-009, DOCRB-010 | Mandatory checks are low-noise and an independent clean-room review passes |

DOCRB-001, DOCRB-002, DOCRB-003, DOCRB-004, DOCRB-005, DOCRB-006, DOCRB-007,
DOCRB-008, DOCRB-009, and DOCRB-011 are Merged. DOCRB-010 is In Progress under
the operator-approved four-path clean-room verification plan. The stored 10/11
progress remains unchanged until post-merge reconciliation.

## Success Criteria

- [ ] Every material component root has a documented disposition and owner.
- [ ] Component-internal truth is beside code and central architecture is
      cross-system only.
- [ ] Retained diagrams have one authority, declared upstreams, and a tested
      render path.
- [ ] Public diagrams commit Draw.io source, accessible SVG, adjacent textual
      meaning, and parity evidence.
- [ ] Public documentation follows a consistent reader-needs structure without
      duplicating internal authority.
- [ ] Relevant code/contract changes review diagram impact in the same change;
      unaffected changes are not burdened with false-positive work.
- [ ] Clean-room navigation, accuracy, accessibility, and maintenance checks
      pass.

## Work Items

### DOCRB-001: Ratify the documentation architecture and authority contract

- **Status:** Merged 2026-08-17 via PR #3975
- **Intent:** Establish the durable contract for documentation placement,
  authority, diagram formats, accessibility, maintenance triggers, phased
  enforcement, module coordination, and non-release posture.
- **Expected Outcome:** An accepted ADR records the hybrid model; the decision
  log, documentation-governance guide, and architecture-diagram guide agree on
  one authority per concern, component-local Mermaid, central cross-system
  views, public Draw.io plus SVG, the advisory-to-mandatory transition, and the
  DOCFRESH/DOCSYNC/DSITE boundaries, and the production `docs-shell` →
  `anvil-docs-private`/`docs-public` topology plus its current ownership gap.
- **Files:** `plans/decisions/123-documentation-authority-and-diagram-model.md`,
  `plans/decisions/DECISION-LOG.md`,
  `docs/guides/documentation-governance.md`,
  `docs/guides/architecture-diagrams.md`, `docs/architecture/README.md`
- **Scope:** `plans/decisions/**`, `plans/decisions/DECISION-LOG.md`,
  `docs/guides/documentation-governance.md`,
  `docs/guides/architecture-diagrams.md`, production docs-host authority
  records
- **Non-scope:** Corpus migration, component doc edits, public reorganisation,
  mandatory enforcement, release planning
- **Dependencies:** —
- **Confidence:** high
- **Validation:** `pnpm format:check && pnpm aps:active-lint && pnpm docs:check`

### DOCRB-002: Inventory and disposition the documentation and diagram corpus

- **Status:** Merged 2026-08-17 via PR #3976
- **Intent:** Produce a complete, source-pinned ownership and disposition map
  before moving or deleting documentation.
- **Expected Outcome:** Every crate, package/group, app, central architecture
  document, public page family, generated reference, and diagram is classified
  with an owner, authoritative concern, target location, required diagram view,
  and `retain`, `move`, `redraw`, `merge`, `retire`, or explicit-exemption
  disposition; duplicate-authority pairs and broken discovery paths are listed.
- **Files:** `plans/specs/2026-08-17-docrb-corpus-disposition.md`,
  `plans/modules/docs-rebaseline.aps.md`
- **Scope:** Repository documentation/component inventory and a governed DOCRB
  assessment artefact
- **Non-scope:** Executing migrations or changing sibling-module status
- **Dependencies:** DOCRB-001
- **Confidence:** high
- **Validation:** `pnpm docs:index:check && pnpm docs:check && pnpm aps:drift --json`

### DOCRB-003: Add the advisory diagram trigger and co-located documentation standard

- **Status:** Merged 2026-08-20 via PR #4027
- **Intent:** Give contributors and agents a small, consistent rule for
  reviewing documentation and diagram impact while the corpus is being
  re-baselined.
- **Expected Outcome:** Root `AGENTS.md` contains a thin advisory trigger and
  link; documentation governance defines when a code/contract change requires
  a documentation or diagram update, when it does not, and the required shape
  of component `README.md`/`ARCHITECTURE.md` files without duplicating
  procedure in agent adapters.
- **Files:** `AGENTS.md`,
  `docs/guides/documentation-governance.md`,
  `docs/guides/architecture-diagrams.md`,
  `docs/architecture/_as-built-template.md`,
  `docs/architecture/README.md`,
  `docs/governance/tags-catalogue.md`,
  `docs/guides/README.md`,
  `docs/guides/adapters/README.md`,
  `docs/guides/testing.md`,
  `docs/indexes/by-authority.md`,
  `docs/indexes/by-owner.md`,
  `docs/indexes/by-status.md`,
  `docs/indexes/by-type.md`,
  `docs/reviews/README.md`,
  `plans/modules/docs-rebaseline.aps.md`,
  `plans/index.aps.md`
- **Evidence:** Policy and template implementation, including Council repairs,
  is carried by the DOCRB-003 commit range containing this work-item update.
  Publication receipt: https://github.com/eddacraft/anvil-001/pull/4027.
  The template's corrected metadata is mechanically reflected in the four
  generated discovery indexes named above. Lifecycle bookkeeping is separately
  owned:
  `plans/index.aps.md` records DOCRB-003 as Merged via PR #4027 and is
  validated against the merged tree with APS index and drift checks. The hosted
  freshness repair manually reviewed the seven binding downstream documents
  named in Files against their changed file-level upstreams on 2026-08-20; no
  substantive contradiction was found, so their metadata records that review.
  The CI-equivalent `check-docs-owed --since` gate validates the repaired
  edges against the live pull-request base.
- **Scope:** `AGENTS.md`, `docs/guides/documentation-governance.md`,
  `docs/guides/architecture-diagrams.md`, the component-documentation
  template and architecture discovery link, generated documentation discovery
  indexes, the seven binding downstream freshness reviews named in Files, the
  DOCRB-003 module record, and its lifecycle row in `plans/index.aps.md`
- **Non-scope:** Mandatory CI failure or mass component migration
- **Dependencies:** DOCRB-001, DOCRB-002
- **Confidence:** high
- **Validation:** `pnpm format:check && pnpm docs:check && pnpm aps:active-lint && pnpm aps:index:check && pnpm aps:drift --json && git diff --check`

### DOCRB-004: Pilot co-located Mermaid on representative components

- **Status:** Merged 2026-08-20 via PR #4031 (rebase-merge commit
  `0a0f00c20cacd59fc33971771387a3a3f4cb8bbc`)
- **Intent:** Prove the beside-code model across materially different anvil
  surfaces before applying it repository-wide.
- **Expected Outcome:** The six component roots selected by DOCRB-002 satisfy
  these acceptance behaviours:
  1. each root has a concise `README.md` covering purpose, owner, supported
     entry points, local validation, and links to deeper authorities;
  2. each root has a source-linked `ARCHITECTURE.md` covering boundaries,
     dependencies, invariants, material flow, trust, failure, and fallback;
  3. Mermaid diagrams cover kernel source-to-parse-to-graph-to-finding,
     intercept save-to-validate-to-fence, dashboard UI-to-generated-client-to-
     loopback-server plus the server capability/auth boundary, hosted API
     request-to-middleware-to-route-to-persistence/trust, and docs-shell
     auth/routing-to-private/public-renderer flows;
  4. ownership is explicit: KERN, INTD, DASH, and APGOV own their component
     concerns; APGOV owns the hosted API component; BAUTH remains authoritative
     for auth;
  5. docs-shell records the live `docs-shell` topology, the rollback-only
     `docs-site`, and the unresolved DOCRB/DSITE ownership gap without
     changing sibling status;
  6. local pilot docs link retained central authorities without superseding or
     migrating them; and
  7. only docs-shell gains a thin local `AGENTS.md` spoke, `CONTEXT.md`
     discovers it, and one findings report records the source revision,
     navigation trace, Mermaid render method/result, source-link, ownership,
     duplication checks, and DOCRB-005/-009 recommendations.
- **Files:** `crates/anvil-kernel/README.md`,
  `crates/anvil-kernel/ARCHITECTURE.md`,
  `crates/anvil-intercept/README.md`,
  `crates/anvil-intercept/ARCHITECTURE.md`,
  `apps/dashboard/README.md`, `apps/dashboard/ARCHITECTURE.md`,
  `crates/anvil-dashboard-server/README.md`,
  `crates/anvil-dashboard-server/ARCHITECTURE.md`,
  `apps/anvil-api/README.md`, `apps/anvil-api/ARCHITECTURE.md`,
  `apps/docs-shell/README.md`, `apps/docs-shell/ARCHITECTURE.md`,
  `apps/docs-shell/AGENTS.md`, `CONTEXT.md`,
  `plans/reviews/2026-08-20-docrb-004-pilot-findings.md`,
  `plans/modules/docs-rebaseline.aps.md`, `plans/index.aps.md`
- **Evidence:** PR #4031 merged after fresh hosted CI with zero unresolved
  review threads. Its final head `1840551a74238b832e1478e8d62bae5539ae31fc`
  and rebase-merge commit share tree
  `e2df831181db1004968e66ea9f0f88078f345817`. The single
  `plans/reviews/2026-08-20-docrb-004-pilot-findings.md` report records
  source-pinned navigation, manual Mermaid render/trace, ownership,
  source-link, and duplication evidence plus follow-on recommendations.
- **Scope:** The exact DOCRB-002 pilot roots `crates/anvil-kernel`,
  `crates/anvil-intercept`, `apps/dashboard`,
  `crates/anvil-dashboard-server`, `apps/anvil-api`, and
  `apps/docs-shell`; the thin docs-shell agent spoke; root context discovery;
  and the single pilot findings report
- **Non-scope:** `packages/**`; product or configuration code; central
  as-built migration or authority changes; root `AGENTS.md` or governance
  changes; public diagrams; automated Mermaid tooling or enforcement; sibling
  work-item status
- **Dependencies:** DOCRB-001, DOCRB-002
- **Confidence:** medium
- **Validation:** `cargo test -p eddacraft-anvil-kernel && cargo test -p eddacraft-anvil-intercept && cargo test -p eddacraft-anvil-dashboard-server && pnpm --filter @eddacraft/anvil-dashboard test && pnpm --filter @eddacraft/anvil-api test -- --run && pnpm --filter @eddacraft/docs-shell test && pnpm format:check && pnpm docs:check && pnpm aps:active-lint && pnpm aps:index:check && pnpm aps:drift --json && git diff --check`

### DOCRB-005: Migrate component truth and remove duplicate authorities

- **Status:** Merged 2026-08-20 via PR #4055 (rebase-merge receipt
  `5db473d504ea702b9e1e9fe69878d780d45cf71a`)
- **Intent:** Move maintainable component truth beside code and leave central
  documents as links or genuine cross-system authorities.
- **Expected Outcome:** All inventory entries assigned `move`, `merge`, or
  `retire` are resolved in dependency-aware batches; component docs carry
  owners/upstreams; central as-built material is reduced or reshaped without
  losing decision history; redirects and links preserve discovery; no concern
  retains two apparent authorities.
- **Files:** 46 exact paths: `plans/execution/DOCRB-005.actions.md`,
  `plans/index.aps.md`, the
  fourteen central move/merge as-builts, eighteen bounded component-local
  authority/discovery paths, documentation authority/discovery records, four
  generated documentation indexes, two binding file-level docs-owed repairs,
  `plans/reviews/2026-08-20-docrb-005-component-truth-migration.md`, and this
  item record
- **Scope:** Component `README.md`/`ARCHITECTURE.md`,
  `docs/architecture/**`, generated documentation indexes, and only the two
  source-proved file-level freshness edges reported by `docs:owed`; this
  merge closeout reconciles only DOCRB-005's Merged truth and the aggregate
  count, while every other item status remains unchanged
- **Non-scope:** Substantive public content refresh owned by DOCSYNC
- **Dependencies:** DOCRB-003, DOCRB-004
- **Confidence:** medium
- **Validation:** `pnpm format:check && pnpm docs:check && pnpm aps:drift --json`
- **Evidence:** PR #4055 merged from final head
  `a3897465eddccb56c849c1cbd0c91e1c771cd02c` after all 40 hosted checks
  completed acceptably and all review threads were resolved. The
  rebase-merge receipt `5db473d504ea702b9e1e9fe69878d780d45cf71a` is an
  ancestor of `origin/main`, and the remote feature branch is deleted.

### DOCRB-006: Rebuild central cross-system and operational diagrams

- **Status:** Merged 2026-08-20 via PR #4040 (squash-merge commit
  `a6885875948f3ddfc10279cc05d7b8fe7da36421`)
- **Intent:** Replace stale or over-broad central diagrams with a small set of
  authoritative cross-system views.
- **Expected Outcome:** The five required DOCRB-006 views are authoritative and
  source-traceable: system context; container/component relationships;
  trust/deployment boundaries; save-to-validation; and docs delivery. Every
  retained view declares audience, concern, upstreams, owner, lifecycle state,
  local-authority relationship, and adjacent textual meaning. Quality, BAUTH,
  EDDA, Rust, adapter, and component-local details remain in their owning
  documents; obsolete central duplicates are retired.
- **Files:** `plans/modules/docs-rebaseline.aps.md`, `plans/index.aps.md`,
  `plans/execution/DOCRB-006.actions.md`,
  `plans/specs/2026-08-17-docrb-corpus-disposition.md`, `CONTEXT.md`,
  `docs/README.md`,
  `docs/guides/documentation-governance.md`,
  `docs/reviews/shipped-codebase-review-checklist.md`,
  `docs/architecture/README.md`, `docs/architecture/overview.md`,
  `docs/architecture/quality-model.md`,
  `docs/architecture/auth-as-built.md`,
  `docs/architecture/edda-stack.md`,
  `docs/architecture/trust-and-deployment-boundaries.md`,
  `docs/architecture/save-to-validation.md`,
  `docs/architecture/docs-delivery.md`,
  `docs/runbooks/save-time-background-driver.md`,
  `docs/architecture/anvil-system-components.drawio`,
  `docs/architecture/pptx-workflow.drawio`,
  `docs/indexes/by-authority.md`, `docs/indexes/by-owner.md`,
  `docs/indexes/by-status.md`, `docs/indexes/by-type.md`, and
  `plans/reviews/2026-08-20-docrb-006-central-views.md`
- **Evidence:** The action plan fixes the execution order and authority
  boundaries. The final review records per-view source-edge traces, temporary
  pinned Mermaid 11.16.0 render outputs, link resolution, duplicate-authority
  checks, Draw.io retirement references, exact range, and repository gates.
  For publication, the six implementation patches rebased range-diff-equivalent
  from original base `d9b30b23daef0da05f74a7d44dfa3accd0e03fe7` onto
  `8bf8622e755324452304bd9226830bdf507fcac3`; the rebased implementation
  head before evidence reconciliation is
  `ce83aa1fcdfbe64ebcb1fb04783757110e46a49b`. All ten upstream
  config-catalogue paths remain blob-identical, and the DOCRB range remains 24
  changed paths.
- **Scope:** The five required DOCRB-006 views; retained supporting central
  authorities; diagram disposition and discovery; context/checklist navigation;
  governance and directly owed downstream freshness review; the save-time
  runbook cross-link; two obsolete Draw.io retirements; generated documentation
  indexes; one evidence report
- **Non-scope:** Component-internal migration (DOCRB-005); public Draw.io/SVG
  assets (DOCRB-007/-008); automated Mermaid tooling or mandatory
  affected-change enforcement (DOCRB-009); product/runtime behaviour; public
  styling; sibling-module lifecycle
- **Dependencies:** DOCRB-001, DOCRB-002
- **Confidence:** high
- **Validation:** manual pinned Mermaid 11.16.0 render/source-edge/link trace,
  then `pnpm format:check && pnpm docs:index && pnpm docs:index:check && pnpm docs:check && pnpm docs:owed --since 8bf8622e7 && pnpm aps:active-lint && pnpm aps:index:check && pnpm aps:drift --json && git diff --check`; independent verify-loop and Council

### DOCRB-007: Establish the Draw.io-to-accessible-SVG public asset pipeline

- **Status:** Merged 2026-08-20 via PR #4051 (rebase-merge receipt
  `7804355bb23b61018da2f18c7cb94275daf06ff0`)
- **Intent:** Make polished public diagrams source-controlled, reviewable,
  accessible, and reproducible.
- **Expected Outcome:** A pinned Draw.io Desktop export path governs the five
  public families' explicit `assets/diagrams` directories, commits lower-kebab
  paired `.drawio`/`.svg` files with canonically equal embedded source and
  deterministic provenance. Fail-closed checks cover structural
  sibling/embedded XML, SVG activity, canonical non-symlink confinement, atomic
  output, exact exporter version/output/arguments, source/export freshness,
  accessibility, auditable real Markdown/MDX references, and directory-scoped
  ADR-123 raster exceptions. The validator is a first-class `docs:check`
  surface, adversarial fixture tests exercise failure modes, and both production
  Docusaurus renderers build to prove mount integration at the system boundary.
- **Files:** `plans/modules/docs-rebaseline.aps.md`, `plans/index.aps.md`,
  `plans/execution/DOCRB-007.actions.md`,
  `plans/reviews/2026-08-20-docrb-007-public-svg-pipeline.md`,
  `package.json`, `scripts/docs/docs-check.mjs`,
  `scripts/docs/docs-check.test.sh`,
  `scripts/docs/check-public-diagrams.mjs`,
  `scripts/docs/export-public-diagram.mjs`,
  `scripts/docs/lib/public-diagrams.mjs`,
  `scripts/docs/public-diagrams.json`,
  `scripts/docs/check-public-diagrams.test.mjs`,
  `scripts/docs/fixtures/public-diagrams/**`,
  `docs/guides/architecture-diagrams.md`, and generated documentation indexes
  if changed
- **Evidence:** PR #4051 merged from final reviewed head
  `e33926db65a9c968b55e60b12af00bffa5c3b356` after all hosted required
  checks passed and all review threads were resolved. Rebase-merge receipt
  `7804355bb23b61018da2f18c7cb94275daf06ff0` is an ancestor of
  `origin/main`, and the remote
  `docs/docrb-007-public-svg-pipeline` head no longer exists.
- **Scope:** Documentation tooling, the mounted `anvil`, `beta`, `aps`,
  `kindling`, and `edda-stack` public family roots, export/accessibility
  guidance, fixture tests, and both production Docusaurus renderer builds
- **Non-scope:** Production diagram authoring or content prioritisation;
  unmounted `docs/public/start-here`; retired or legacy
  `docs/architecture/**.drawio`; rollback-only `apps/docs-site`; and
  DOCRB-009 component-Mermaid or affected-change enforcement
- **Dependencies:** DOCRB-001, DOCRB-002
- **Confidence:** high
- **Validation:** `node --test scripts/docs/check-public-diagrams.test.mjs && pnpm test:docs-check && pnpm docs:check && pnpm --filter @eddacraft/anvil-docs-private build && pnpm --filter @eddacraft/docs-public build && pnpm format:check && pnpm aps:active-lint && pnpm aps:index:check && pnpm aps:drift --json`

### DOCRB-008: Re-baseline public information architecture and curated diagrams

- **Status:** Merged 2026-08-21 via PR #4068 (rebase-merge receipt
  `182d77b6329f460f4635e3e946b329ff9af84445`)
- **Intent:** Make public documentation easier to follow by organising it around
  reader needs and adding a restrained set of polished visual explanations.
- **Expected Outcome:** In coordination with DOCSYNC, DSITE, and the owners
  ratified for the production docs apps, public content has explicit
  tutorial/how-to/reference/explanation placement, navigation and cross-links
  avoid duplicate authority, high-value user journeys use paired Draw.io/SVG
  diagrams, current shell/private/public routing builds, and release/version
  provenance remains intact.
- **Files:** `apps/anvil-docs-private/sidebars/anvil.ts`,
  `apps/anvil-docs-private/sidebars/beta.ts`,
  `apps/docs-public/sidebars/kindling.ts`,
  `apps/docs-public/sidebars/edda-stack.ts`,
  `docs/public/anvil/first-gate.md`,
  `docs/public/anvil/assets/diagrams/detect-fix-verify.{drawio,svg}`,
  `docs/public/aps/getting-started.md`,
  `docs/public/aps/assets/diagrams/work-item-lifecycle.{drawio,svg}`,
  `scripts/docs/lib/public-diagrams.mjs`,
  `scripts/docs/check-public-diagrams.test.mjs`,
  `docs/guides/architecture-diagrams.md`,
  `docs/guides/documentation-governance.md`,
  `docs/guides/README.md`,
  `docs/architecture/README.md`,
  `docs/governance/tags-catalogue.md`,
  `docs/guides/adapters/README.md`,
  `docs/reviews/README.md`,
  `docs/README.md`,
  `plans/index.aps.md`,
  `plans/specs/2026-08-17-docrb-corpus-disposition.md`,
  `plans/execution/DOCRB-008.actions.md`, and
  `plans/reviews/2026-08-21-docrb-008-public-ia.md`
- **Evidence:** PR #4068 merged from final reviewed head
  `294a93234ae511de4d7f6f91aebd7a4fffa63d23` after all required hosted
  checks passed and unresolved review threads were zero. Rebase-merge receipt
  `182d77b6329f460f4635e3e946b329ff9af84445` is an ancestor of
  `origin/main`, and the remote `docs/docrb-008-public-ia` branch no longer
  exists.
- **Scope:** Label/grouping-only public sidebar changes; exactly two curated
  public Draw.io/SVG journey pairs and their mounted owning-page explanations;
  the one public-diagram inventory note; evidence; and the exact two-file
  prerequisite repair that removes only Draw.io Desktop 31.1.8's anchored
  official SVG prolog before annotation/hashing while retaining fail-closed
  checker coverage; plus the seven gating and one baselined direct file-level
  freshness closeouts required by those approved changes; and the current-item
  lifecycle reconciliation and bounded Council finding closeout.
- **Approved prerequisite:** On 2026-08-21 the operator directly approved the
  exact `public-diagrams.mjs` normaliser and
  `check-public-diagrams.test.mjs` regression coverage after authentic Draw.io
  output proved that the shipped exporter preserved a prolog the shipped
  checker rejects. This does not authorise broader tooling changes.
- **Non-scope:** Content moves or rewrites, APS sidebar or workflow-page
  changes, generated references, extra diagrams, dependencies or lockfiles,
  checker weakening, closing DOCSYNC or DSITE items, marketing-site redesign,
  publishing a release, PR #4050 absorption, or DOCRB-009/-010 work.
- **Dependencies:** DOCRB-006, DOCRB-007, DOCRB-011
- **Confidence:** medium
- **Validation:** `node --test scripts/docs/check-public-diagrams.test.mjs && pnpm docs:public:check && pnpm docs:public:diagrams && pnpm docs:check && pnpm --filter @eddacraft/anvil-docs-private build && pnpm --filter @eddacraft/docs-public build && pnpm --filter @eddacraft/docs-shell build`

### DOCRB-011: Unhide the live public definition layer

- **Status:** Merged 2026-08-20 via PR #4009
- **Intent:** Make already-written Anvil definition pages findable on the
  live host without waiting for public diagram work.
- **Expected Outcome:** `apps/anvil-docs-private/sidebars/anvil.ts` has a
  first-class Reference category and unhides existing pages using front-matter
  IDs (`cli-reference`, `rule-reference`, `support-reference`, `agent-skills`);
  overview has two doors that point at those pages; `scripts/docs/check-public-docs.mjs`
  validates the live private sidebar (rollback sidebar remains a second
  check); no new evaluation-model or catalogue prose.
- **Files:** `apps/anvil-docs-private/sidebars/anvil.ts`,
  `scripts/docs/check-public-docs.mjs`,
  `docs/public/anvil/overview.md`
- **Scope:** Live sidebar, overview doors, live-nav check
- **Non-scope:** Evaluation-model prose (DOCDEF-001), generated catalogues,
  public Draw.io diagrams (DOCRB-008), dashboard as generally available
- **Dependencies:** DOCRB-001, DOCRB-002
- **Confidence:** high
- **Validation:** `pnpm docs:public:check && pnpm docs:check && pnpm --filter @eddacraft/anvil-docs-private build`

### DOCRB-009: Activate mandatory diagram review and render/freshness enforcement

- **Status:** Merged 2026-08-23 via PR #4099 (rebase-merge receipt
  `93f543e671b0ddcf4ec0481eaf1539a3006a9f82`)
- **Intent:** Convert the proven advisory rule into enforceable, change-scoped
  maintenance checks without burdening unrelated changes.
- **Expected Outcome:** The root agent contract and contributor workflow require
  diagram-impact disposition for defined architecture/public-flow triggers;
  Mermaid rendering, Draw.io/SVG parity, declared-upstream freshness, and
  affected-change classification are tested; fixtures prove relevant changes
  fail when diagrams drift and irrelevant changes pass without waiver noise.
- **Files:** `AGENTS.md`, `CONTRIBUTING.md`,
  `docs/guides/documentation-governance.md`,
  `docs/guides/architecture-diagrams.md`, `docs/README.md`,
  `docs/architecture/README.md`, `docs/architecture/docs-delivery.md`,
  `docs/architecture/overview.md`,
  `docs/architecture/trust-and-deployment-boundaries.md`,
  `docs/governance/tags-catalogue.md`, `docs/guides/README.md`,
  `docs/guides/adapters/README.md`, `docs/guides/testing.md`,
  `docs/reviews/README.md`,
  `docs/reviews/shipped-codebase-review-checklist.md`,
  `apps/docs-shell/ARCHITECTURE.md`,
  `plans/specs/2026-08-19-anvil-docs-definition-layer.md`, `package.json`,
  `pnpm-lock.yaml`, `pnpm-workspace.yaml`,
  `scripts/docs/check-diagram-impact.mjs`,
  `scripts/docs/check-diagram-impact.test.mjs`,
  `scripts/docs/docs-check.mjs`, `scripts/docs/docs-check.test.sh`,
  `scripts/ci/classify-changes.sh`,
  `scripts/ci/classify-changes.test.sh`,
  `scripts/ci/integration-validation.test.sh`,
  `scripts/validate/local.sh`, `scripts/validate/local.test.sh`,
  `.github/actions/detect-changes/action.yml`, `.github/workflows/ci.yml`,
  `plans/modules/docs-rebaseline.aps.md`, `plans/index.aps.md`,
  `plans/execution/DOCRB-009.actions.md`, and
  `plans/reviews/2026-08-21-docrb-009-diagram-enforcement.md`
- **Evidence:** PR #4099 merged from final reviewed head
  `cf145163f2f5c26624bf7f972bad3c376c0bb5ec` after all required hosted
  checks passed and unresolved review threads were zero. Rebase-merge receipt
  `93f543e671b0ddcf4ec0481eaf1539a3006a9f82` is an ancestor of current
  `origin/main`, and the remote `docs/docrb-009-diagram-enforcement` branch
  no longer exists.
- **Scope:** Mandatory agent/contributor disposition contract; one pinned
  Mermaid CLI 11.16.0 root development dependency and only its required
  Puppeteer build allowance; one composed diagram-impact docs-check surface;
  one routing signal through the existing trusted classifier, local validator,
  and Docs Lint workers; fixture tests; current-item lifecycle; action plan;
  thirteen triggered freshness/review metadata closeouts; evidence. The
  approved ceiling is 35 paths.
- **Non-scope:** Release gating beyond existing documentation checks or
  administrator/policy bypasses; DOCRB-010; new or rewritten diagrams; public
  IA, content, or navigation; Draw.io exporter/checker changes; DOCFRESH
  mechanics or baseline changes; PR-body parsing; a new CI job or required
  status; generated indexes without changed output; product runtime code
- **Dependencies:** DOCRB-003, DOCRB-004, DOCRB-005, DOCRB-006, DOCRB-007,
  DOCRB-008
- **Confidence:** medium
- **Validation:** `node --test scripts/docs/check-diagram-impact.test.mjs &&
  pnpm test:docs-check && pnpm docs:check && pnpm docs:public:diagrams &&
  pnpm docs:owed --since <exact-base> --fail-on-owed &&
  pnpm test:ci-classify && pnpm test:validate-local &&
  pnpm test:ci-integration && pnpm format:check && pnpm lint:check &&
  pnpm aps:active-lint && pnpm aps:index:check && pnpm aps:drift --json &&
  git diff --check`

### DOCRB-010: Independently verify the documentation re-baseline

- **Status:** In Progress
- **Intent:** Prove the new system is navigable, accurate, accessible, and
  maintainable from a clean checkout before calling the programme complete.
- **Expected Outcome:** A clean-room report tests representative maintainer,
  contributor, operator, and public-reader journeys; traces documented nodes
  and arrows to current source/contracts; checks Mermaid and SVG rendering and
  accessibility; exercises both relevant-change failure and unaffected-change
  pass paths; records residual gaps as new APS or GitHub work rather than
  silently accepting them.
- **Files:** `plans/modules/docs-rebaseline.aps.md`, `plans/index.aps.md`,
  `plans/execution/DOCRB-010.actions.md`, and
  `plans/reviews/2026-08-23-docrb-010-clean-room-verification.md`
- **Scope:** Read-only clean-room verification of the whole documentation
  system and representative code/contract upstreams, recorded in exactly four
  repository paths
- **Non-scope:** Repairing discovered gaps; product, documentation, diagram,
  checker, build, or workflow changes; a release claim; implicit closure of
  sibling-module work; automatic tracker writes; administrator/policy bypass
- **Dependencies:** DOCRB-009
- **Confidence:** medium
- **Validation:** `pnpm install --frozen-lockfile && pnpm exec mmdc --version &&
  node --test scripts/docs/check-diagram-impact.test.mjs &&
  node scripts/docs/check-diagram-impact.mjs --json &&
  pnpm test:docs-check && pnpm docs:check && pnpm docs:public:check &&
  pnpm docs:public:diagrams &&
  pnpm docs:owed --since <exact-base> --fail-on-owed &&
  pnpm docs:index:check && pnpm test:ci-classify &&
  pnpm test:validate-local && pnpm test:ci-integration &&
  pnpm exec nx test docs-shell &&
  pnpm --filter @eddacraft/anvil-docs-private build &&
  pnpm --filter @eddacraft/docs-public build &&
  pnpm --filter @eddacraft/docs-shell build &&
  pnpm validate:changed && pnpm format:check && pnpm lint:check &&
  pnpm aps:active-lint && pnpm aps:index:check &&
  pnpm aps:drift --json && git diff --check`
