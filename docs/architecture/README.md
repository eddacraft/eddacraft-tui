# Architecture Documentation

| Type   | Authority | Owner | Status | Freshness                                                                                                                            |
| ------ | --------- | ----- | ------ | ------------------------------------------------------------------------------------------------------------------------------------ |
| README | Advisory  | DOCRB | Live   | Last reviewed 2026-08-19 against ADR-123, `infra/src/vercel.ts`, `docs/architecture/`, and `docs/guides/documentation-governance.md` |

| Upstream                                                                  | Downstream                      |
| ------------------------------------------------------------------------- | ------------------------------- |
| ADR-123, `docs/guides/documentation-governance.md`, `infra/src/vercel.ts` | Architecture document discovery |

This directory holds Anvil's living **cross-system** architecture references:
source-pinned as-built maps, conceptual guides, and frozen or active design
specs. Component-internal truth moves to component-root `ARCHITECTURE.md` files
under DOCRB (ADR-123). Until that migration, existing `*-as-built.md` files
remain the derived maps they are today.

**Production docs host (ADR-123):** `docs.eddacraft.ai` is `apps/docs-shell`,
which proxies `apps/anvil-docs-private` (gated Anvil/beta) and
`apps/docs-public` (APS, Kindling, edda-stack, blog). `apps/docs-site` is
retained only for rollback. DSITE still owns its recorded legacy host work
items; this README records the live topology and the ownership gap without
changing DSITE status. Implementation truth is `infra/src/vercel.ts`.

**Authority order:** code, schemas, and tests beat prose. Prefer a
`*-as-built.md` for "what ships today". Prefer a Spec for frozen design
contracts still tracked by implementation. Prefer ADRs in `plans/decisions/` for
"why we chose this".

## As-built docs

Component-level, dated, source-pinned descriptions of what is actually shipping.
The shape is set by the [as-built template](_as-built-template.md) — copy it
when adding a new one.

> Freshness note: treat each doc's own metadata header as authoritative. Entries
> dated `2026-07-02` against main `d1fded280` were re-verified in the source-pin
> drift sweep on that date; later delta reviews (for example activation / MCP
> shim on 2026-07-29) supersede that pin where present.

- [Auth System](auth-as-built.md) — beta auth API, token lifecycle, GitHub
  OAuth + OTP/device-code flows, JWT licence, local admin key lifecycle
- [Intercept daemon](intercept-as-built.md) — IPC surface (UDS + named pipe),
  peer-cred trust boundary, fence-on-failure, save-time `validate_paths`
  pipeline
- [Activation orchestrator](activation-as-built.md) — `anvil start` flow,
  six-state protection vocabulary, MCP install, watch-fallback, gate posture
- [MCP shim](mcp-shim-as-built.md) — Rust MCP server, tool registry including
  GCTX tools, `anvil_validate_write`, daemon-backed vs embedded validation
- [Checks pipeline](checks-as-built.md) — `anvil-checks` registry, rule
  families, suppressions, language-profile gating, baseline
- [Kernel](kernel-as-built.md) — watcher, tree-sitter parser, semantic graph,
  policy engine, embedded API, watch loop, GV2 hot-read surface. Supersedes
  `rust-kernel-spec.md` for "what shipped"
- [TUI surfaces](tui-as-built.md) — Ratatui surfaces, shared widgets, dashboard
  surface family
- [Driver framework + intercept-proto](driver-framework-as-built.md) — JSON-RPC
  wire protocol, driver clients, intercept-rules hot path
- [anvil-api service](api-as-built.md) — Hono on Vercel, admin surfaces, Neon
  DB; auth flows live in `auth-as-built.md`
- [anvil-observability](observability-as-built.md) — namespace registry, tracing
  subscriber, redaction
- [Tutorial subsystem](tutorial-as-built.md) — TUI tutorial engine and
  ProtectionLoop default path
- [Widget catalogue](widgets-as-built.md) — `anvil-tui` composites +
  `eddacraft-tui` shared widgets
- [CLI TUI runner](cli-tui-runner-as-built.md) — terminal session lifecycle in
  `crates/anvil-cli/src/tui.rs`
- [Adapter packages](adapter-packages-as-built.md) — SpecKit / BMAD / Generic /
  APS-Markdown adapters, APS validator, Kindling capture bridge
- [Review capsules](capsule-as-built.md) — `anvil capsule`
  create/verify/explain/prune
- [JS/TS release surfaces](jsts-release-surfaces.md) — inventory of JS/TS
  surfaces still on the release path vs canary/archive

Planned next set:

- Workspace / build (planned) — Cargo workspace shape, `nx` / `pnpm` layering,
  `cargo-dist` release pipeline

## Living references

Cross-component references and conceptual maps that should stay current.

- [overview.md](overview.md) — top-level architecture overview, package
  layering, quality model, surface architecture, and live Rust-first component
  diagrams (start here)
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
  as-builts for present state. Status: Historical
- [rust-kernel-spec.md](rust-kernel-spec.md) — H1 kernel design intent.
  `kernel-as-built.md` owns "what shipped"
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

- Mermaid diagrams live inline in [overview.md](overview.md) and, after
  DOCRB-004, in component `ARCHITECTURE.md` files
- Draw.io sources:
  [anvil-system-components.drawio](anvil-system-components.drawio),
  [pptx-workflow.drawio](pptx-workflow.drawio) — neither yet has a sibling SVG;
  see
  [`docs/guides/architecture-diagrams.md`](../guides/architecture-diagrams.md)
  and ADR-123

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
