# anvil Roadmap

**Last updated:** 2026-08-03 (`v0.9.1-beta` published; active cut is
`v0.9.2-beta` MCP 2.0 reconnect patch in `RELEASE-PLAN.md`. Next minor after
that remains unscoped. Module status lives in the SOT files below; this roadmap
stays thematic.)

> Companion: [RELEASE-PLAN.md](./RELEASE-PLAN.md) — pickable menu of release-
> slice candidates with waves, dependencies, and parallelisation. Source of
> truth for module status: [`plans/index.aps.md`](./plans/index.aps.md).

## Mission

anvil makes AI-generated code safe to merge by catching architecture-boundary
violations and AI escape-hatch anti-patterns at file-save time. Developers get
actionable warnings before code leaves the file, with human-owned exceptions for
intentional deviations.

The product thesis is simple: **trust in AI-generated code, so more of it
reaches production faster, while architecture drift slows or reverses over
time.**

## Posture

- **Planless-first.** anvil delivers value without requiring config or APS
  plans.
- **Warnings over blocks.** Inform; let CI enforce if desired. Exit 0 by
  default.
- **New edges only.** Baseline existing state; warn on new violations.
- **Defense in depth.** Each surface contributes the strongest layer it can;
  layers compensate for one another's failure modes.
- **Honest claim only.** anvil never says "Protected" when a layer is
  unverified. False confidence is the worst failure mode.
- **Air-gapped by default.** Core operation requires no internet; cloud services
  are opt-in amplifiers, never foundations.
- **First-touch wow.** Onboarding is the conversion moment — the first minute
  matters more than the next ten.

## Horizons

Horizons are **ordered by sequence, not by date.** Each unlocks the next.
Detailed work-item lists live in `plans/index.aps.md`; this roadmap names
**capabilities and themes**.

### Horizon 0 — Shipped to date

| Tag           | Theme                                    | Headline capability                                                                                                                                                                                                                                   |
| ------------- | ---------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `v0.5.0-beta` | AI Guardrails & Mid-Edit Validation      | Real-time AI validation fires before save through the MCP launch shim. Validation backend was embedded fallback, not yet daemon-backed.                                                                                                               |
| `v0.5.1-beta` | Scanner Signal & TUI Hotfixes            | Patch — secret/antipattern false-positive fixes, TUI zoom, audit env-template filtering, kernel import bug fixes.                                                                                                                                     |
| `v0.6.0-beta` | Wow-Start Activation + Daemon-Backed RTV | `install → cd repo → anvil start` is the canonical first minute. MCP `tools/call` runs through the daemon when owner-only IPC is available, embedded path remains correctness-equivalent fallback. Driver framework + editor-driver protocol shipped. |

### Horizon 1 — Daemon Working End-to-End — Shipped `v0.7.0-beta` (2026-05-21)

**Theme:** Flip the daemon from "available when invoked" to "always-on, in-tree,
defensible."

`v0.6.0-beta` ships the daemon + driver substrate. This horizon turns that
substrate into a coherent end-to-end protection loop the user can rely on
without thinking about it.

**Capabilities delivered:**

- **Witness chain.** Per-commit, hash-chained, in-tree proof of which layers
  fired. Travels via git, survives `git worktree add`, tamper-detectable.
- **Hook surface.** Pre-commit, pre-push, post-commit / -merge / -rewrite.
  Self-contained binary, framework-agnostic (husky, lefthook, pre-commit-
  framework, plain). Silent on success, terse on failure.
- **L4 policy framework.** Per-branch rules with server-side fallback for
  unwitnessed commits; legacy acceptance via `cutoff_commit`.
- **Baseline adoption.** Existing repos adopt anvil without a wash of warnings.
  Hard-pinned classes (`secrets`, `command-safety`) cannot be config-disabled.
- **Multi-agent coordination.** Per-task fence isolation so one bad sub-agent
  doesn't cascade-fence a whole worktree. `AgentTag` composite session key.
- **Wrapped-launch ingress (`anvil-run`).** Agent processes register sessions
  before spawn; PGIDs / Job Objects let the daemon target interrupts; drop-guard
  cleanup on exit.
- **L5 audit.** Periodic re-scan of mainline for drift detection — catches what
  bypassed L0–L4 (admin overrides, force-pushes).
- **Air-gapped operation guarantee.** Every core command tested under a
  network-blocked sandbox.

**Hard release gate.** A protection-claim contract test suite pins the closed
set of states the user can be in:
`unprotected | warming | pre-write-embedded | pre-write-daemon | save-time-only | full | degraded-protection | cross-boundary-mixed | multi-daemon-detected | path-uncertain`.
No item ships until every state is reachable in fixtures and rendered claims
match.

**Shipped:** `v0.7.0-beta` (2026-05-21), followed by patches `v0.7.1-beta`
(2026-05-22), `v0.7.2-beta` (2026-05-25), `v0.7.3-beta` (2026-05-31), and
`v0.7.4-beta` (2026-06-01). Detailed sequencing, waves, and parallelisation in
the [v0.7.0 release record](./plans/releases/v0.7.0-beta.md).

### Between Horizon 1 and Horizon 2 — Shipped minors

These minors land capability between the always-on daemon loop and the team-lead
browser surface. Immutable records live under
[`plans/releases/`](./plans/releases/).

| Tag           | Theme                                  | Headline capability                                                                                                                                                       |
| ------------- | -------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `v0.8.0-beta` | The Save-Time Daemon                   | Persistent intercept daemon validates save-time deltas (ADR-061); save-time governance no longer cold-spawns per save. Shipped 2026-06-11.                                |
| `v0.8.1-beta` | Headless GitHub Login                  | Headless auth/login path patch on the v0.8 line.                                                                                                                          |
| `v0.9.0-beta` | First-Run Wins and the Assistant Graph | Assistant-facing graph over MCP (GCTX), warm-start persistence, MCP-optional useful daemon, USAGE analytics, JOURNEY-conducted first-run cut. Shipped 2026-07-12.         |
| `v0.9.1-beta` | Daily Path Polish and MCP 2.0 support  | Bare daily ensure, default activation TUI, twelve-client MCP install, dual-era protocol support, managed skills, and warning-over-block gate honesty. Shipped 2026-08-02. |

**Latest shipped headline:** `v0.9.1-beta` (2026-08-02). Release record:
[`plans/releases/v0.9.1-beta.md`](./plans/releases/v0.9.1-beta.md).

### Horizon 2 — Daily path + MCP reach — shipped `v0.9.1-beta`

**Delivered theme:** Daily path polish and MCP 2.0 support — bare `anvil`
ensure, default activation TUI, multi-client MCP install + dual-era protocol,
and managed skills on the v0.9 line.

**Longer-term Horizon 2 (not this claim):** Team-lead browser glance remains the
next credible surface after this cut. DASH/DASHCORE foundation is already on
`main` behind `dashboard.web` (default-off) for testing only — not a customer
claim until default-on or a later named window owns it.

**Active cut:** `v0.9.2-beta` — patch for the MCP 2.0 reconnect regression
(MCP26-013). Not a new minor theme. After closeout, the next minor window is
unscoped until the operator names it. Current state:
[`RELEASE-PLAN.md`](./RELEASE-PLAN.md). Module status:
[`plans/index.aps.md`](./plans/index.aps.md).

### Horizon 3 — Enterprise Readiness

**Theme:** "How does this deploy in front of N repos for an org-tier customer?"

A coherent constellation that delivers as a sequenced foundation, not as
isolated features:

- **Gateway control plane** — deployment topology, enforcement contract,
  observability event model.
- **Policy federation** — multi-repo publish/subscribe over OPAE bundle
  primitives.
- **Org policy hierarchy** — multi-level inheritance.
- **Policy lifecycle** — canary, grace periods, changelog generation.
- **Compliance reporting** — SOC 2 / ISO 27001 / NIST framework mapping.
- **Compliance evidence workspace** — auditor-facing surface.
- **Trust centre automation** — public trust-artifact publishing pipeline.

Promotion to active work is **demand-pulled** — first enterprise prospect or
design-partner request lights this horizon up.

### Horizon 4 — Coverage Breadth

**Theme:** More languages, more surfaces, more packs.

Phased rollout of the language-and-coverage design:

- **Phase 1** — TypeScript substrate + SQL migrations + the smallest viable pack
  set (Pulumi, LLM-provider).
- **Phase 2** — Rust + Python anchors; Drizzle / Next.js / Hono / Tokio packs;
  markdown governance slice.
- **Phase 3** — Surfaces: GitHub Actions, Dockerfile, shell. Tail-wave language
  coverage.

Demand-pulled. Most stays parked until a customer or dogfood signal asks for it.

### Horizon 5 — Long Bets

Real concepts, no current consumer. Held in the catalogue so they don't get lost
or duplicated:

- **Agent infrastructure** — provider-agnostic agent runtime + harness with
  zero-copy semantic graph access.
- **Graph v2 substrate** — joined semantic / dependency / trust / control /
  provenance graph. anvil-first foundation; assistant context delivery becomes a
  projection over it.
- **Symbol-graph-driven effect prediction** — predict the effect of a change
  against captured intent. anvil's _original_ use case, sharpened by the graph
  substrate.
- **Lineage & authorship confidence** — line-level human / AI / mixed
  attribution.
- **Adjacent surfaces** — gateway integrations, open-spec ingestion, unified
  config formats. Surface when their primary horizon lands.

## Big bets

| Bet                          | Why it matters                                                                                          | Where it lives                              |
| ---------------------------- | ------------------------------------------------------------------------------------------------------- | ------------------------------------------- |
| **Witness chain + hooks**    | The load-bearing primitive that makes "anvil protects this project" a defensible claim across machines. | Horizon 1                                   |
| **Daemon + drivers**         | Mechanical validation at the surface where AI tools propose writes, without a Node sidecar.             | Horizon 0 (shipped) → Horizon 1 (always-on) |
| **Real-time AI validation**  | Refuse the bad write before disk. The launch demo.                                                      | Horizon 0 (shipped)                         |
| **Wrapped-launch ingress**   | Daemon-coordinated session lifecycle for shell-launched AI agents (Claude Code in tmux, etc.).          | Horizon 1                                   |
| **Team-lead dashboard**      | The persona that funds the tool. Buyer surface, not developer surface.                                  | Horizon 2                                   |
| **Enterprise constellation** | The org-tier deployment story — gateway, federation, hierarchy, lifecycle, compliance.                  | Horizon 3                                   |
| **Graph v2 substrate**       | Joined structural model for enforcement, trust, control, provenance, and later agent context.           | Horizon 5                                   |
| **Effect prediction**        | What intent-ledger governance becomes — predict effect of a change against captured intent.             | Horizon 5                                   |

## Doctrine pinned to architecture

These are non-negotiable architectural commitments. Every horizon must trace
back to one or more.

1. **Deterministic, pre-commit.** anvil catches violations before they land in
   shared history.
2. **Defense in depth.** L0 (mid-edit MCP) is best-effort; L2 / L3 / L4
   (save-time, pre-commit, pre-push) are mandatory deterministic gates; L5
   (audit) catches what slipped through.
3. **Failure reduces noise, not increases it.** Silent on success; one terse
   line on warning; repeat-suppressed.
4. **Honest claim only.** Closed-set status states; never "Protected" when a
   layer is unverified.
5. **Planless-first.** Works without config; `anvil start` writes minimal
   anvil-managed files; nothing else required.
6. **New edges only.** Existing state grandfathered at adoption time;
   security-class rules exempt.
7. **anvil cloud is opt-in.** Hosted services are amplifiers, never foundations.
8. **Air-gapped by default.** Core operation requires no internet.

## What this roadmap is NOT

- **Not a schedule.** No quarter or month commitments. Sequence over date.
- **Not a backlog dump.** Capabilities and themes, not work items.
- **Not the source of truth for module status.** That lives in
  [`plans/index.aps.md`](./plans/index.aps.md).
- **Not the release menu.** That lives in
  [`RELEASE-PLAN.md`](./RELEASE-PLAN.md).
