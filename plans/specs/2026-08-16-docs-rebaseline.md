# Documentation re-baseline

| Type | Authority | Owner | Status | Freshness |
| ---- | --------- | ----- | ------ | --------- |
| Spec | Authoritative for DOCRB design | [DOCRB](../modules/docs-rebaseline.aps.md) | Accepted | 2026-08-16 — operator-approved hybrid documentation model |

| Upstream | Downstream |
| -------- | ---------- |
| Repository component and documentation inventory at `c4fd624ce`; [documentation governance](../../docs/guides/documentation-governance.md); [architecture diagram maintenance](../../docs/guides/architecture-diagrams.md); [ADR-119](../decisions/119-documentation-freshness-from-declared-upstream.md) | [DOCRB](../modules/docs-rebaseline.aps.md); future documentation-authority ADR; component-local documentation; central architecture docs; public documentation assets |

**Execution authority** is the DOCRB work-item set. This specification records
the approved direction and initial assessment; it does not authorise product
code changes or make any release claim.

## 1. Problem

anvil's documentation has grown as a central corpus around a fast-moving
monorepo. It contains valuable material, but readers must cross between broad
architecture guides, dated as-built documents, package READMEs, public guides,
ADRs, and APS modules without a consistent rule for where component truth
lives. The result is predictable:

- component detail is remote from the code that invalidates it;
- several documents can appear authoritative for the same concern;
- code reviews do not have a small, consistent diagram-impact trigger;
- Mermaid is easy to maintain but concentrated in central documents;
- Draw.io sources exist, but public-friendly exports and parity rules do not;
- public documentation is organised as prose first, with little visual support;
- freshness checks detect some movement but do not establish the right
  authority or diagram topology.

This is an information-architecture problem, not a request for more prose.

## 2. Approved outcome

Adopt a hybrid documentation model:

1. **Component truth beside code.** Each material component owns a concise
   `README.md` and, where its internal design warrants it, an
   `ARCHITECTURE.md`. Internal diagrams use Mermaid beside the prose and code
   they explain.
2. **Central docs for cross-system concerns.** `docs/architecture/**` owns only
   system context, cross-component boundaries, trust boundaries, deployment
   topology, and flows that genuinely span several components.
3. **Public docs by reader need.** Public material follows the Diátaxis split:
   tutorials, how-to guides, reference, and explanation. Public diagrams are
   curated Draw.io views with committed accessible SVG exports.
4. **One authority per concern.** Other documents link to the authority instead
   of copying its explanation or diagram.
5. **Change-coupled maintenance.** A code or contract change reviews its
   documentation and diagram impact in the same change. Enforcement becomes
   mandatory only after ownership, migration, and render checks are in place.

This direction follows established documentation practices without treating
one framework as a complete answer: the
[C4 model](https://c4model.com/) supplies diagram abstraction levels,
[arc42](https://arc42.org/overview) supplies maintainable architecture-document
structure, [Diátaxis](https://diataxis.fr/) supplies public information
architecture, and [Mermaid](https://mermaid.js.org/) plus
[Draw.io](https://www.drawio.com/doc/faq/export-to-svg) provide source-controlled
internal and polished public diagram formats.

## 3. Current-state assessment

The initial repository assessment is pinned to `c4fd624ce` on 2026-08-16.
DOCRB-002 re-runs it as the authoritative corpus inventory before migration.

| Observation | Evidence | Consequence |
| ----------- | -------- | ----------- |
| Component surface | 37 crate roots, 12 package/group roots, and 9 app roots tracked at the pinned commit: 58 total | The documentation unit must be the component/domain, not one ever-growing central guide |
| Co-located orientation | 27 of 58 tracked roots have a root `README.md` or `ARCHITECTURE.md`; 31 do not | Readers cannot reliably start beside the code, and ownership is uneven |
| Co-located architecture | No root-level component `ARCHITECTURE.md` was found | Internal architecture has no consistent beside-code home |
| Central architecture corpus | 36 files under `docs/architecture/**`, including many component as-built documents | Component truth is centralised and expensive to keep aligned with local changes |
| Mermaid | Active architecture Mermaid is concentrated in five blocks in `overview.md` and one in `quality-model.md` | Text-source diagrams exist, but not where most component changes happen |
| Draw.io | Two `.drawio` sources exist; neither has a sibling SVG export | The editable source is preserved, but reviewable/public output and source-export parity are absent |
| Public corpus | 91 files under `docs/public/**` and no committed Draw.io/SVG/PNG diagram assets in that tree | Public journeys lack a governed visual layer |
| Production docs topology | `infra/src/vercel.ts` deploys `docs-shell` in front of `anvil-docs-private` and `docs-public`; `docs-site` is retained only for rollback, while DSITE still describes it as the shared host | DOCRB-001 must reconcile current production ownership and routing before migration relies on DSITE's legacy contract |
| Existing governance | ADR-119 and DOCFRESH check declared upstream freshness | Freshness machinery can support the new model, but does not decide authority or diagram placement |

The figures are a baseline, not a target that every directory must receive a
diagram. Generated, vendored, fixture, spike, and grouping-only roots may be
explicitly exempted by DOCRB-002.

## 4. Documentation authority model

| Concern | Authoritative home | Expected form | Diagram format |
| ------- | ------------------ | ------------- | -------------- |
| Component purpose, entry points, ownership, local development | Component-root `README.md` | Short orientation with links to deeper authority | Mermaid only when a small map materially helps |
| Component internals, invariants, local data/control flow | Component-root `ARCHITECTURE.md` | Source-linked as-built explanation | Mermaid source inline |
| Cross-component context, containers, dependencies, trust or deployment boundaries | `docs/architecture/**` | C4-like views and cross-system explanation | Mermaid for engineering views; Draw.io plus SVG where public-facing |
| Durable architectural decision | `plans/decisions/**` plus decision-log entry | ADR | Mermaid only when the decision needs a flow or state model |
| Planned work and execution state | `plans/modules/**` and `plans/specs/**` | APS module/work item and approved design source | Mermaid only when sequence or dependency structure needs it |
| Operator procedure | `docs/runbooks/**` | Executable runbook | Mermaid for non-trivial operational sequence/state |
| Public learning and use | `docs/public/**` | Diátaxis-organised tutorial, how-to, reference, or explanation | Curated Draw.io source plus accessible SVG export |

Authority is about a concern, not a file type. A component-local document may
link to an ADR or public guide, but must not restate it as a second source of
truth.

## 5. Diagram policy

### 5.1 Internal Mermaid

Use Mermaid beside code for diagrams that help maintainers reason about:

- module or crate boundaries;
- request, event, data, and control flow;
- state machines and lifecycle transitions;
- dependency direction and extension points;
- trust boundaries and failure/fallback paths;
- sequences that are difficult to verify from prose alone.

Every internal diagram must name the concern it owns, link its important nodes
to source paths in adjacent prose, avoid duplicating a central view, and remain
small enough to review as text. Diagram syntax must render in the repository's
supported Mermaid toolchain.

### 5.2 Public Draw.io and SVG

Use Draw.io for public diagrams where layout, typography, visual grouping, or
brand consistency materially improves comprehension. Each governed public
diagram commits:

- the editable `.drawio` source;
- a deterministic `.svg` export with matching base name;
- alt text or an adjacent textual explanation of the same meaning;
- ownership and upstream metadata through the containing document;
- a parity check that fails when source and export no longer match the approved
  export process.

SVG is the default public export. Raster export is an explicit exception for a
consumer that cannot use accessible SVG.

### 5.3 One diagram per concern

Each concern has one authoritative diagram. A second audience should receive a
linked or deliberately simplified view, not a copied diagram that can silently
drift. The inventory records the relationship between engineering and public
views when both are justified.

## 6. Initial component and diagram map

DOCRB-002 must disposition every component root, central document, and public
page. The following domain map is the initial prioritised assessment.

| Domain | Components in scope | Co-located documentation need | Cross-system/public diagram need |
| ------ | ------------------- | ----------------------------- | -------------------------------- |
| CLI and runtime orchestration | `anvil-cli`, `anvil-run`, `anvil-config`, `anvil-hook`, `anvil-baseline`, `anvil-capsule`, `anvil-sarif` | Command dispatch, runtime ownership, evidence and failure boundaries | CLI-to-engine request flow and major user workflows |
| Kernel and semantic graph | `anvil-kernel`, `anvil-kernel-types`, `anvil-graph-cache`, `anvil-gctx-types`, `anvil-gctx-egress`, `anvil-architecture`, `anvil-plan-read-model`, `anvil-grammar-wat` | Parse/graph/cache boundaries, hot/cold reads, wire contracts, ownership | Source-to-graph-to-finding flow; GCTX projection and trust boundary |
| Checks, policy, evidence, and observability | `anvil-checks`, `anvil-checks-ast`, `anvil-checks-napi`, `anvil-rules`, `anvil-policy`, `anvil-policy-engine`, `anvil-l4`, `anvil-witness`, `anvil-attribution`, `anvil-observability` | Registry/evaluation boundaries, suppression, enforcement projection, evidence and tracing flow | Check/policy pipeline and decision-to-witness lifecycle |
| Save interception and platform adapters | `anvil-intercept`, `anvil-intercept-proto`, `anvil-intercept-rules`, `anvil-intercept-macos`, `anvil-intercept-win32`, `anvil-rayon-init` | Daemon, IPC, validation, platform adapter, and fallback invariants | Save-to-validation sequence with trust/failure boundaries |
| Terminal and local dashboard surfaces | `anvil-tui`, `eddacraft-tui`, `anvil-dashboard-server`, `apps/dashboard`, `anvil-driver-client` | Shared widget boundary, surface state, local API/client contract | Surface/container view and terminal/dashboard data flow |
| Hosted API | `apps/anvil-api` | API composition, auth/admin boundary, persistence and failure modes | Deployment, identity, data, and trust-boundary view |
| Documentation and web delivery | `apps/docs-shell`, `apps/anvil-docs-private`, `apps/docs-public`, `apps/docs-public-astro`, legacy `apps/docs-site`, `apps/website`, `packages/docs-meta` | Current shell/private/public routing, legacy-host disposition, source/render/deploy ownership, metadata/indexing, entitlement boundary | Public-doc build/publish flow and public/private content boundary |
| TypeScript compatibility and tooling | `packages/anvil/**`, `packages/adapters`, `packages/aps`, `packages/eslint-plugin-anvil`, `packages/libs`, `packages/shared`, `packages/tooling`, `packages/transactional` | Retained versus retiring surfaces, package/group boundaries, generated contracts | Rust-first/compatibility map and release-surface lifecycle |
| Memory and sibling integration | `packages/edda-stack`, `packages/kindling-integration` | Edda/Ember responsibilities, capture boundary, external-product distinction | anvil-to-kindling/edda-stack context and privacy flow |
| Validation, benchmarking, and workspace infrastructure | `anvil-bench`, `apps/e2e`, `workspace-hack`, `spike`, root Cargo/pnpm/Nx configuration | Test harness, benchmark method, build-only or exempt status | CI/build/release flow only where it aids maintainers or contributors |

The inventory must classify every root as one of: `component-doc required`,
`README only`, `central cross-system authority`, `generated/vendor`,
`grouping-only`, `historical`, or `explicit exemption`. It must also classify
every existing diagram as `retain`, `move`, `redraw`, `merge`, or `retire`.

## 7. Change trigger and enforcement phases

The durable rule belongs in documentation governance, with only a thin trigger
in root `AGENTS.md`.

Diagram review is relevant when a change:

- adds, removes, renames, splits, or merges a component;
- changes a dependency direction, public contract, trust boundary, deployment
  boundary, state transition, or material data/control flow;
- changes a user workflow or system behaviour shown in a public diagram;
- changes the authority or lifecycle state of a depicted surface.

It is normally not relevant for internal refactors inside an unchanged
boundary, tests that only add coverage, or prose-only repairs that do not alter
the depicted concern.

Enforcement is phased:

1. **Advisory baseline.** DOCRB-001 and DOCRB-002 establish authority and
   ownership. DOCRB-003 adds the thin `AGENTS.md` trigger and review guidance.
2. **Representative pilots.** DOCRB-004 proves the co-located model on Rust,
   MCP/save-interception, dashboard/API, and documentation-delivery surfaces.
3. **Migration and public pipeline.** DOCRB-005..008 move authorities, remove
   duplicates, establish Draw.io-to-SVG parity, and rebuild public information
   architecture.
4. **Mandatory review.** DOCRB-009 activates enforceable diagram review and
   render/freshness checks only after the canonical map and tooling exist.
5. **Independent verification.** DOCRB-010 tests navigation, accuracy,
   accessibility, and maintainability from a clean checkout.

## 8. Module boundaries

- **DOCFRESH** continues to own declared-upstream freshness mechanics and
  release-boundary freshness behaviour.
- **DOCSYNC** continues to own substantive public documentation content and
  release-aligned public refreshes.
- **DSITE** remains the owner of its recorded legacy `apps/docs-site` host and
  section-wiring work. The live production topology has moved to `docs-shell`
  proxying `anvil-docs-private` and `docs-public`; DOCRB-001 must reconcile the
  current owner and contract without changing DSITE work-item status by
  implication.
- **DOCRB** owns the authority model, corpus disposition, diagram topology,
  co-location migration, public diagram source/export pipeline, and activation
  of the new review rule.
- The archived **DOCGOV** module remains historical evidence. DOCRB may replace
  its living guidance, but must not rewrite or reopen the archived module.

Coordination means linking and sequencing with those modules, not absorbing or
closing their remaining work.

## 9. Release posture

DOCRB is a high-priority engineering-effectiveness programme. It is not part of
a release claim set, does not gate release readiness or a release cut, and may
progress independently of release work. A release may consume completed DOCRB
improvements, but release records must not imply the re-baseline is complete
until DOCRB-010 is terminal.

## 10. Success criteria

- Every material component has one discoverable documentation disposition and
  owner.
- Component-internal truth is beside code; central architecture contains only
  cross-system authority.
- Every retained diagram has one authoritative source, declared upstreams, and
  a tested render path.
- Every public diagram has committed Draw.io source, accessible SVG export,
  adjacent text, and parity evidence.
- Public documentation is navigable by tutorial, how-to, reference, and
  explanation intent without duplicating authority.
- Architecture-relevant changes receive diagram-impact review in the same
  change, with low-noise exemptions for unaffected changes.
- A clean-room reviewer can locate a component, understand its boundaries, and
  trace a representative system flow without relying on tribal knowledge.

## 11. Non-goals

- Rewriting every document at once.
- Requiring a diagram for every package, crate, page, or code change.
- Making Draw.io the source for internal component diagrams.
- Making Mermaid the presentation format for polished public diagrams.
- Replacing ADRs, APS, source comments, API schemas, or generated references.
- Gating a release on DOCRB progress.
- Reopening DOCFRESH, DOCSYNC, DSITE, or archived DOCGOV ownership.
