# Architecture Documentation

| Type   | Authority | Owner | Status | Freshness                                                                                                                                       |
| ------ | --------- | ----- | ------ | ----------------------------------------------------------------------------------------------------------------------------------------------- |
| README | Advisory  | DOCRB | Live   | Last reviewed 2026-08-21 against ADR-123, DOCRB-003, `infra/src/vercel.ts`, `docs/architecture/`, and `docs/guides/documentation-governance.md` |

| Upstream                                                                  | Downstream                      |
| ------------------------------------------------------------------------- | ------------------------------- |
| ADR-123, `docs/guides/documentation-governance.md`, `infra/src/vercel.ts` | Architecture document discovery |

This directory holds anvil's living **cross-system** architecture references:
source-pinned as-built maps, conceptual guides, and frozen or active design
specs. Component-internal truth lives in component-root `README.md` and
`ARCHITECTURE.md` files under ADR-123. Deprecated central `*-as-built.md`
records preserve stable discovery, retained cross-system links, and a route to
Git history; they are not duplicate component authority.

**Production docs host (ADR-123):** `docs.eddacraft.ai` is `apps/docs-shell`,
which proxies `apps/anvil-docs-private` (gated Anvil/beta) and
`apps/docs-public` (APS, Kindling, edda-stack, blog). `apps/docs-site`, the
rollback-only host, was retired 2026-07-08 and deleted once the window closed.
DSITE's remaining work items are recorded history, not a live surface; this
README records the live topology without changing DSITE status. Implementation
truth is `infra/src/vercel.ts`.

**Authority order:** code, schemas, and tests beat prose. For current component
internals, prefer the component-root `ARCHITECTURE.md`. A live central
`*-as-built.md` owns only its stated cross-system concern; a Deprecated one is a
migration/history route. Prefer a Spec for frozen design contracts still tracked
by implementation. Prefer ADRs in `plans/decisions/` for "why we chose this".

## Cross-system as-built docs and compatibility records

These are dated, source-pinned descriptions of retained cross-system concerns
plus deprecated compatibility records for component maps migrated by DOCRB-005.
Do not add new component-internal authority here. For a new or migrated
component, use the [component-documentation template](_as-built-template.md) to
create a component-root `README.md` and, when its internals warrant it,
`ARCHITECTURE.md`.

> Freshness note: treat each doc's own metadata header as authoritative. Entries
> dated `2026-07-02` against main `d1fded280` were re-verified in the source-pin
> drift sweep on that date; later delta reviews (for example activation / MCP
> shim on 2026-07-29) supersede that pin where present.

- [Auth System](auth-as-built.md) — beta auth API, token lifecycle, GitHub
  OAuth + OTP/device-code flows, JWT licence, local admin key lifecycle
- [Intercept daemon compatibility record](intercept-as-built.md) — local
  internals moved to `crates/anvil-intercept/ARCHITECTURE.md`; retained
  save/trust views remain linked
- [Activation compatibility record](activation-as-built.md) and
  [MCP shim compatibility record](mcp-shim-as-built.md) — local authority moved
  to `crates/anvil-cli/ARCHITECTURE.md`
- [Checks pipeline](checks-as-built.md) — `anvil-checks` registry, rule
  families, suppressions, language-profile gating, baseline
- [Kernel compatibility record](kernel-as-built.md) — shipped internals moved to
  `crates/anvil-kernel/ARCHITECTURE.md`; historical design intent remains in
  `rust-kernel-spec.md`
- [TUI compatibility records](tui-as-built.md), [widgets](widgets-as-built.md),
  [tutorial](tutorial-as-built.md), and [CLI runner](cli-tui-runner-as-built.md)
  — local authority lives in `crates/anvil-tui/ARCHITECTURE.md`,
  `crates/anvil-cli/ARCHITECTURE.md`, and the shared
  `crates/eddacraft-tui/README.md`
- [Driver framework + intercept-proto](driver-framework-as-built.md) — JSON-RPC
  wire protocol, driver clients, intercept-rules hot path
- [anvil-api compatibility record](api-as-built.md) — service internals moved to
  `apps/anvil-api/ARCHITECTURE.md`; auth remains cross-system
- [anvil-observability compatibility record](observability-as-built.md) and
  [review-capsule compatibility record](capsule-as-built.md) — narrow leaf
  authority moved to the respective component-root READMEs
- [Adapter compatibility record](adapter-packages-as-built.md) — split local
  authority links for adapters, APS tooling, and the kindling bridge
- [JS/TS release surfaces](jsts-release-surfaces.md) — inventory of JS/TS
  surfaces still on the release path vs canary/archive

Planned next set:

- Workspace / build (planned) — Cargo workspace shape, `nx` / `pnpm` layering,
  `cargo-dist` release pipeline

## Living references

Cross-component references and conceptual maps that should stay current.

- [overview.md](overview.md) — authoritative system-context and
  container/component views (start here)
- [trust-and-deployment-boundaries.md](trust-and-deployment-boundaries.md) —
  macro local-daemon, hosted API, and documentation trust boundaries
- [save-to-validation.md](save-to-validation.md) — cross-owner caller-buffer and
  post-save validation sequence
- [docs-delivery.md](docs-delivery.md) — documentation source, build,
  deployment, shell, and renderer flow
- [rust-architecture-overview.md](rust-architecture-overview.md) — crate layout
  for the Rust workspace and module map (KERN / RENG / RATS / PORT / RSTLAN)
- [edda-stack.md](edda-stack.md) — three-layer memory architecture (Kindling /
  Ember / Edda); design contract while the TS `edda-stack` package retires
- [quality-model.md](quality-model.md) — conceptual model for `check`, `gate`,
  `watch`, `audit`, `doctor`, `architecture`, `policy`
- [oss-surface.md](oss-surface.md) — eddacraft's three open-source repos and
  their relationship to the closed product

## Historical migration records

Kept only for provenance; not live architecture authority.

- [anvil-architecture-evolution.md](anvil-architecture-evolution.md) — Current →
  H1 → H2 rollout plan that superseded ADR-011. H1 largely shipped; read
  component-local architecture and retained live cross-system maps for present
  state. Status: Historical
- [rust-kernel-spec.md](rust-kernel-spec.md) — H1 kernel design intent.
  `crates/anvil-kernel/ARCHITECTURE.md` owns current shipped internals;
  `kernel-as-built.md` preserves migration discovery and history
- _Archived 2026-05-23 (DOCGOV-008):_
  [`monorepo-structure.md`](../archive/architecture/monorepo-structure.md)
- _Archived 2026-08-05:_
  [`anvil-full-architecture.md`](../archive/architecture/anvil-full-architecture.md)
  — pre-cutover CURRENT/PROPOSED synthesis (2026-03-13); superseded by
  `overview.md` + as-builts
- _Archived 2026-08-05:_
  [`rust-architecture-endstate.md`](../archive/architecture/rust-architecture-endstate.md)
  — aspirational end-state (2026-04-03); superseded by
  `rust-architecture-overview.md` + as-builts

## Active and frozen architecture specs

Specs that remain design authority for planned work or completed modules with
frozen contracts.

- [rust-mcp-server-spec.md](rust-mcp-server-spec.md) — RMCPF / MCP26 Rust MCP
  parity and dual-era protocol (active)
- [kernel-benchmarking-spec.md](kernel-benchmarking-spec.md) — Criterion +
  `anvil-bench` methodology (BENCH Complete 16/16; Status: Live)
- [graph-v2-foundation-spec.md](graph-v2-foundation-spec.md) — frozen Graph v2
  spine (joined graphs, hot-read boundary); GV2 module Complete
- [graph-context-delivery-spec.md](graph-context-delivery-spec.md) — frozen GCTX
  delivery / egress contract; GCTX module Complete
- [dev-acceleration-benchmark-spec.md](dev-acceleration-benchmark-spec.md) —
  assistant-facing acceleration benchmarks (DEVACC)
- [system-spec.md](system-spec.md) — aspirational Edda Stack five-component
  topology (PocketFlow unbuilt; PFGW Draft). Target-state only

## References (`references/`)

Advisory external or pre-implementation notes.

- [entire-branch-sidecar.md](references/entire-branch-sidecar.md) — Entire's git
  branch-as-sidecar pattern (session storage ideas)
- [pocketflow-capabilities.md](references/pocketflow-capabilities.md) /
  [pocketflow-vendoring.md](references/pocketflow-vendoring.md) — PocketFlow
  notes for the Draft PFGW module (library not vendored in-tree today)

## Diagrams

The five required DOCRB-006 views are system context and container/component
relationships in [overview.md](overview.md), plus the
[trust/deployment](trust-and-deployment-boundaries.md),
[save-to-validation](save-to-validation.md), and
[documentation-delivery](docs-delivery.md) views. They sit alongside retained
supporting central authorities such as KERN's [quality model](quality-model.md),
BAUTH's [auth as-built](auth-as-built.md), and EDDA's
[stack view](edda-stack.md). Component internals remain in component-root
`ARCHITECTURE.md` files.

DOCRB-006 retired the former live Draw.io sources
`docs/architecture/anvil-system-components.drawio` and
`docs/architecture/pptx-workflow.drawio` after replacement and disposition
review. Historical archive references remain provenance, not live links. Public
Draw.io/SVG work remains DOCRB-007/-008; see
[`docs/guides/architecture-diagrams.md`](../guides/architecture-diagrams.md) and
ADR-123.

## Adjacent indexes

- [`docs/specs/`](../specs/) — non-architecture design contracts (for example
  watch-output)
- [`docs/internal/`](../internal/) — engineering-internal briefs
- [`docs/runbooks/`](../runbooks/) — operational procedures (current release
  runbook:
  [`v0.7.0-beta-release-runbook.md`](../runbooks/v0.7.0-beta-release-runbook.md))
- [`docs/guides/`](../guides/) — developer practice
- [`docs/vision/`](../vision/) — north-star and scope guard
- [`docs/archive/architecture/`](../archive/architecture/) — retired
  architecture docs
- [`plans/decisions/DECISION-LOG.md`](../../plans/decisions/DECISION-LOG.md) —
  ADR index
- [`plans/index.aps.md`](../../plans/index.aps.md) — module status
