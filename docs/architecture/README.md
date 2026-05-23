# Architecture Documentation

This directory holds Anvil's living architecture references, as-built component
docs, and pre-implementation specs. It is the engineering-side index — the
public-facing docs site is rendered from `apps/docs-site/`. If a doc here is
stale, that is recorded inline rather than removed; see the classifications
below.

## As-built docs

Component-level, dated, source-pinned descriptions of what is actually shipping.
The shape is set by the [as-built template](_as-built-template.md) — copy it
when adding a new one.

- [Auth System](auth-as-built.md) — beta auth API, token lifecycle, JWT licence,
  device-code / OTP flows, gaps register (current)
- [Intercept daemon](intercept-as-built.md) — IPC surface (UDS + named pipe),
  peer-cred trust boundary, AD-7 fence-on-failure, fence persistence, interrupt
  ladder, registry, win32 listener (current, against `v0.6.0-beta`)
- [Activation orchestrator](activation-as-built.md) — `anvil start` flow,
  six-state protection vocabulary, language profile (LAUNCH-015/-016), MCP
  install (LAUNCH-009), watch-fallback decision (LAUNCH-011) (current, against
  `v0.6.0-beta`)
- [MCP shim](mcp-shim-as-built.md) — Rust MCP server, `anvil_validate_write`
  tool, daemon-backed vs embedded-fallback validation, correlation envelope,
  §4.4 redaction filter (current, against `v0.6.0-beta`)
- [Checks pipeline](checks-as-built.md) — `anvil-checks` registry, AP / AI / GS
  / DD / RL families, suppressions, language-profile gating, baseline, the four
  CLI surfaces (current, against `v0.6.0-beta`)
- [Kernel](kernel-as-built.md) — watcher (notify + glob filter), tree-sitter
  parser, semantic graph (KERN-020..023), policy engine, embedded API, watch
  loop. Supersedes `rust-kernel-spec.md` for "what shipped" (current, against
  `v0.6.0-beta`)
- [TUI surfaces](tui-as-built.md) — Ratatui surfaces (audit / browser / doctor /
  gate / init / onboarding / status / tutorial / watch / welcome / wizard),
  shared widget vocabulary, snapshot infrastructure, watch dashboard event
  adapter (current, against `v0.6.0-beta`)
- [Driver framework + intercept-proto](driver-framework-as-built.md) — JSON-RPC
  wire protocol, driver registration + capability negotiation, TS / Rust driver
  clients, Win32 named-pipe primitives, intercept-rules hot-path library
  (current, against `v0.6.0-beta`; spec→code drift documented in §12)
- [anvil-api service](api-as-built.md) — Hono on Vercel, non-auth admin
  surfaces, license / migration runner, middleware stack, Neon DB layer,
  apps/admin-cli retirement path (current, against `v0.6.0-beta`; auth flows
  live in `auth-as-built.md`)
- [anvil-observability](observability-as-built.md) — namespace registry, tracing
  subscriber, traceparent helper, sensitive-fields advisory list (current,
  against `v0.6.0-beta`)
- [Tutorial subsystem](tutorial-as-built.md) —
  `anvil-tui/src/surfaces/tutorial/*` multi-file engine (mod.rs 1845 + discovery
  913 + discovery_render 683 + executor + fix + render + showcase + verify +
  watch_demo + 10 snapshot pins). LAUNCH-014 ProtectionLoop default + two
  test-pinned copy invariants (current, against `v0.6.0-beta`)
- [Widget catalogue](widgets-as-built.md) — `anvil-tui/widgets/` (anvil-
  specific composites) + upstream `eddacraft-tui` v0.1.0 (published on
  crates.io: 13 widgets — confirm / container / divider / editor / header /
  log_panel / parallel_progress / progress_bar / select / spinner / status_badge
  / status_bar / text_input). Theme contract, keyboard handler, snapshot pinning
  (current, against `v0.6.0-beta`)
- [CLI TUI runner](cli-tui-runner-as-built.md) — `crates/anvil-cli/src/tui.rs`
  (495 lines) — terminal session lifecycle, `run_surface_in` shared-terminal
  pattern, animation tick, watch_loop dirty-paint gate, panic-safety gap
  documented (current, against `v0.6.0-beta`)
- [Adapter packages](adapter-packages-as-built.md) — `packages/adapters/`
  (SpecKit + BMAD + Generic + APS-Markdown shipping; OpenSpec + BMAD-v4 in
  progress), `packages/aps/` (validator + templates + examples + schemas),
  `packages/kindling-integration/` (capture session bridge, observation
  contract, benchmarks). APS schema drift to public docs flagged (current,
  against `v0.6.0-beta`)

Planned next set:

- Workspace / build (planned) — Cargo workspace shape, `nx` / `pnpm` layering,
  `cargo-dist` release pipeline, project-level `nx.json` / `Cargo.toml`
  workspace conventions

## Living references

Cross-component references and conceptual maps. Each entry is flagged as
`(current)` if it tracks today's code or `(stale, last reviewed YYYY-MM-DD)` if
it predates the Rust kernel cutover (`v0.4.0-beta`, when the native scanner
became authoritative). Stale docs are kept because the framing is still useful,
but trust the source first.

- [overview.md](overview.md) — top-level architecture overview, package
  layering, quality model, surface architecture, and live Rust-first component
  diagrams (current, last reviewed 2026-05-23)
- [anvil-full-architecture.md](anvil-full-architecture.md) — current vs proposed
  end-state synthesis with `[CURRENT]` / `[PROPOSED]` / `[PARTIAL]` markers
  (stale, last reviewed 2026-03-13 — pre-cutover; the current-vs-proposed
  framing is still useful but specifics have moved)
- [anvil-architecture-evolution.md](anvil-architecture-evolution.md) — Current →
  H1 → H2 phased rollout plan; supersedes ADR-011 (current — framing still
  applies, though phase status has advanced)
- [rust-architecture-overview.md](rust-architecture-overview.md) — crate layout
  for the Rust workspace, module map (KERN / RENG / RATS / PORT / RSTLAN)
  (current)
- [rust-architecture-endstate.md](rust-architecture-endstate.md) — Rust
  end-state spec; tracks aspirational shape, not strictly what's shipping (last
  reviewed 2026-04-03 — flag as aspiration; trust as direction, not as-built)
- [system-spec.md](system-spec.md) — Edda Stack components (PocketFlow /
  Kindling / Ember / Edda / Anvil), component topology and hard limits (current)
- [edda-stack.md](edda-stack.md) — three-layer memory architecture (Kindling /
  Ember / Edda), separation of observation / interpretation / memory (current)
- [quality-model.md](quality-model.md) — conceptual model for `check`, `gate`,
  `watch`, `audit`, `doctor`, `architecture`, `policy` surfaces (current)
- [monorepo-structure.md](monorepo-structure.md) — historical migration plan and
  archived target shape (stale by design — for live layout, use the root
  `README.md`, `apps/README.md`, and `plans/index.aps.md`)
- [oss-surface.md](oss-surface.md) — eddacraft's three open-source repos
  (`eddacraft-tui`, `anvil-plan-spec`, `kindling`) and their relationship to the
  closed product (current)

## Kernel proposals and benchmarking

Specs that describe the kernel's intended shape — not as-builts, not living
references, but proposal/spec docs that the kernel implementation tracks
against.

- [rust-kernel-spec.md](rust-kernel-spec.md) — Rust Watcher Kernel
  specification, H1 implementation target; refines ADR-011a, governed by the
  architecture-evolution rollout
- [kernel-benchmarking-spec.md](kernel-benchmarking-spec.md) — benchmarking
  strategy: Criterion regression detection plus `anvil-bench` capacity
  discovery; tracks performance targets from the kernel spec

## Active architecture specs

Specs that are active authority for planned implementation slices.

- [rust-mcp-server-spec.md](rust-mcp-server-spec.md) — RMCPF-002 Rust MCP parity
  architecture: command layout, protocol support, DRVR-006 tool classification,
  resources, prompts, transports, and TypeScript MCP retirement gates (Ready,
  2026-05-14)

## Specs (`docs/specs/`)

Older design drafts and feature design specs. These describe intent at the time
of writing and predate the as-built docs.

- [`2026-03-15-beta-auth-streamline-design.md`](../specs/2026-03-15-beta-auth-streamline-design.md)
  — design that produced the device-code + OTP flows now documented in
  `auth-as-built.md`
- [`2026-03-27-rust-cli-cutover-design.md`](../specs/2026-03-27-rust-cli-cutover-design.md)
  — RCLI module cutover, archival, and distribution design
- [`command-safety-validation.md`](../specs/command-safety-validation.md) —
  command safety validation specification (older — 2025-12-28)
- [`edda-api-contracts.md`](../specs/edda-api-contracts.md) — Edda API contracts
  and integration points
- [`edda-authority-trust.md`](../specs/edda-authority-trust.md) — Edda authority
  and trust model
- [`edda-enforcement-hooks.md`](../specs/edda-enforcement-hooks.md) — Edda
  enforcement and guidance hooks

## Internal (`docs/internal/`)

Smaller engineering-internal references that don't fit the as-built or spec
shape.

- [`realtime-feed-contract.md`](../internal/realtime-feed-contract.md) — minimum
  event-feed contract for dashboard operations views
- [`weave-feature-brief.md`](../internal/weave-feature-brief.md) — internal
  brief for the `weave` agent harness crates

## Runbooks (`docs/runbooks/`)

Operational procedures live in [`docs/runbooks/`](../runbooks/) and have their
own structure. The current release runbook is
[`v0.6.0-beta-release-runbook.md`](../runbooks/v0.6.0-beta-release-runbook.md);
release-time security context is captured in
[`v0.6.0-beta-security-note.md`](../runbooks/v0.6.0-beta-security-note.md). For
day-to-day ops (admin CLI, branch reconciliation, DB migrations, observability
triage, post-deploy smoke checks, waitlist email operations), see the directory
listing.

## Adjacent indexes

- [`docs/guides/`](../guides/) — how-to guides for developers working on Anvil
  (release runbooks, ADR process, testing, branching strategy, feature flags,
  command safety, …)
- [`docs/vision/`](../vision/) — north-star docs (vision, scope guard,
  aspirational ultimate feature, constitutional engineering)
- [`plans/decisions/DECISION-LOG.md`](../../plans/decisions/DECISION-LOG.md) —
  condensed ADR index; all architecture decisions land here
- [`plans/index.aps.md`](../../plans/index.aps.md) — single source of truth for
  module status and progress counts
