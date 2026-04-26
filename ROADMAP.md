# Anvil Roadmap

**Last updated:** 2026-04-26 (after APS audit + 5 mini planning councils)

> Companion: [RELEASE-PLAN.md](./RELEASE-PLAN.md) — pickable menu of
> release-slice candidates organised by readiness tier.

## Mission

Anvil makes AI-generated code safe to merge by catching architecture-boundary
violations and AI escape-hatch anti-patterns at file-save time. Developers get
actionable warnings before code leaves the file, with human-owned exceptions for
intentional deviations.

The product thesis is simple: **trust in AI-generated code, so more of it
reaches production faster, while architecture drift slows or reverses over
time.**

## Posture

- **Planless-first.** Anvil delivers value without requiring config or APS
  plans.
- **Warnings over blocks.** Inform; let CI enforce if desired. Exit 0 by
  default.
- **New edges only.** Baseline existing state; warn on new violations.
- **Ship-now-with-pre-flight.** The hype phase funds the build. Sequence work
  via cherry-pick; no time estimates on horizons.
- **First-touch wow.** Onboarding/init/tutorial is the conversion moment — speed
  - watcher + real-time validation differentiate.

## Horizons

The horizons are **ordered by sequence, not by date**. Each unlocks the next.

### Horizon 0 — Now Shipping (in flight)

The current branch (`0.4.x`) carries hardening work that is mid-flight and not
yet released:

- **Branch reconciliation** — main/dev integrated; `dev` is canonical.
- **Anvil TS scanner retirement** (TSRET) — Rust scanner is authoritative per
  ADR-026; Surface Drivers (DRVR) supersede the napi cutover per ADR-030.
- **Launch flow readiness** (LAUNCH) — init/welcome/doctor + watcher polish to
  ship-quality.
- **Test coverage uplift** (TCOV) — phase 3 in flight.
- **Documentation sync** (DOCSYNC) — Rust-migration phase 9/10.

These are foreground work; they ride out on the current branch.

### Horizon 1 — Launch (the differentiator) 🔒 LOCKED 2026-04-26

**Theme: real-time AI-output validation that fires before save.**

The thesis Anvil sells against is "AI tools produce code that compiles and
passes tests yet drifts from intended patterns." The launch demo answers that
literally: open Cursor, ask Claude Code to make a confident-but-wrong rewrite,
watch Anvil refuse the write **before it hits disk** and surface the reason to
the agent.

Big bets in this horizon:

- **Intercept Daemon (INTD)** — host-local enforcement daemon, JSON-RPC over
  stdio, fence-on-fail.
- **Surface Drivers (DRVR)** — editor + MCP drivers feeding the daemon.
- **Real-time AI Validation (RTAI)** — mid-edit and pre-write validation paths.
- **One reasoning-pattern rule** — minimum-viable AI-pattern check landing in
  `crates/anvil-checks` (gap surfaced by Council A).
- **AI Guardrail Profile (AIGUARD)** — `anvil gate --profile ai` with stable
  JSON diagnostic envelope; the shape AI tools consume.

These ship as a coherent **RTAI Spike Slice** plus the AIGUARD envelope work.
The slice is deliberately small (≈20 items) so it does not compete with itself
for attention. Everything below this line waits.

### Horizon 2 — Credibility & Hygiene (parallel with Horizon 1) 🔒 LOCKED 2026-04-26

Two narrow tracks that ship without contending for RTAI engineering bandwidth:

- **Release Engineering** — git-config-hooks (GHOOK), attribution-pipeline-v3
  (ATTRIB), scan-performance (SCAN). Launch hygiene without product-surface
  bloat. Council E's "smallest viable cut" is 7 items.
- **Language Credibility Floor** — lang-ts-audit (LANGTS) artefact + OPSUP slice
  1 (check-ID registry) + SURFENV (`.env` secret scan). 3 items. Pure
  governance/operational floor.

### Horizon 3 — Team-Lead Surface (post-launch)

Once RTAI ships, the second persona — team leads, platform engineers, compliance
roles — gets a credible browser surface.

- **Dashboard MVP "Team-Lead Glance"** — DASH foundation + warnings list +
  detail panel + optional config/diagnostics views. Council B's slice is ~12 of
  39 dashboard tasks (the 80/20 cut).
- **`anvil export` CLI work item** (NEW) — bridges CLI output to canonical
  `.anvil/*.json` artefacts the dashboard reads. Critical missing glue surfaced
  by Council B.

### Horizon 4 — Enterprise Readiness (near-term, becoming important soon)

A coherent constellation of seven modules that together answer "how does this
deploy in front of N repos for an org-tier customer?" Promoted from parking lot
to **Tier B (queued)** as enterprise prospects surface.

- **Gateway Control Plane (GATE)** — deployment topology + enforcement
  contract + observability event model.
- **Policy Federation (POLFED)** — multi-repo publish/subscribe workflow over
  OPAE bundle primitives.
- **Org Policy Hierarchy (ORGHIER)** — multi-level inheritance.
- **Policy Lifecycle (POLLC)** — canary, grace periods, changelog generation.
- **Compliance Reporting (COMPLY)** — SOC 2 / ISO 27001 / NIST framework
  support; policy-to-control mapper; posture scoring.
- **Compliance Evidence Workspace (CEWS)** — auditor surfaces (after COMPLY
  prerequisites land).
- **Trust Center Automation (TRUST)** — publishing pipeline for public trust
  artefacts.

This horizon is where the policy-governance theme delivers — not on the current
release, but as a coherent enterprise pitch. Sequence: GATE + POLFED + ORGHIER +
POLLC first (the foundation); COMPLY + CEWS + TRUST second (the auditor
surface).

### Horizon 5 — Coverage Breadth (post-launch, demand-pulled)

The 5-track Language & Coverage design (per
[`plans/specs/2026-04-08-language-and-coverage-design.md`](./plans/specs/2026-04-08-language-and-coverage-design.md))
delivers in waves:

- **Phase 1 spec-faithful slice** — LANGTS + SURFSQL + PACKPUL + PACKLLM (TS,
  warn-only). Council D's larger candidate; ships post-launch only.
- **Phase 2 expansion** — RSTLAN + PYLAN anchors, MDGOV slice 1, remaining Track
  4 packs (PACKDRZ, PACKNXT, PACKHON, PACKTOK).
- **Phase 3 / open-ended** — Track 3 surfaces (SURFGHA, SURFDOCK, SURFSH), Track
  2 tail wave (LANGTAIL).

Demand-pulled. Most of this stays parked until a customer or dogfood signal asks
for it.

### Horizon 6 — Long Bets (parking lot)

Real concepts, no current consumer:

- **Agent Infrastructure (WEAVE)** — provider-agnostic agent runtime in upstream
  `eddacraft/weave-rs`, anvil-weave harness with zero-copy semantic graph
  access. Greenfield import + harness build.
- **PocketFlow Orchestration Gateway (PFGW)** — agent-task orchestration layer
  (capsule lifecycle, memory I/O routing). Complementary to INTD/DRVR, not a
  substitute. PocketFlow-RS upstream as substrate option.
- **Intent Ledger Governance (ILGOV)** — Anvil's _original_ use case: prove the
  plan was followed via captured intent → code diff → gate decision. The current
  version is more powerful — uses the symbol/architecture graph to predict
  effect of a change against captured intent.
- **Lineage & Authorship Confidence (LAC)** — line-level human/AI/mixed
  attribution.
- **Pocketflow Gateway**, **Open-Spec Adapter (OPENSPEC)**, **Graph Context
  Delivery (GCTX)**, **Unified Config Format (UCFG)** — supporting concepts that
  surface when their primary horizons land.

## Big bets, named

| Bet                                       | Why it matters                                                                                        | Where it lives                                          |
| ----------------------------------------- | ----------------------------------------------------------------------------------------------------- | ------------------------------------------------------- |
| **Intercept Daemon + Surface Drivers**    | Mechanical enforcement at the surface where AI tools actually write code. The thing nothing else has. | INTD, DRVR, INTR (Horizon 1)                            |
| **Real-time AI validation**               | The launch demo. Refuse the bad write before disk.                                                    | RTAI (Horizon 1)                                        |
| **AI guardrail diagnostic envelope**      | Stable JSON shape AI tools consume. The integration surface.                                          | AIGUARD (Horizon 1)                                     |
| **Dashboard for non-developer personas**  | Team-lead/platform-engineer/compliance — the buyer that funds the tool.                               | DASH/DASHCORE (Horizon 3)                               |
| **Enterprise readiness constellation**    | The org-tier deployment story. Becoming important soon.                                               | GATE/POLFED/ORGHIER/POLLC/COMPLY/CEWS/TRUST (Horizon 4) |
| **Symbol-graph-driven effect prediction** | What ILGOV becomes — predict effect of a change against captured intent.                              | ILGOV rescope (Horizon 6)                               |

## Cuts and parks

After the audit + 5 councils, **no modules are recommended for archive.** The
audit and councils were systematically too aggressive on archive recommendations
— they conflated _archived planning modules_ (work-item lists that completed)
with _archived components_ (code that no longer exists). Components are live in
`packages/edda-stack/` and `packages/kindling-integration/`.

What changed instead:

- **7 modules rescoped** with corrected banners (PFGW, ILGOV, CFGINT, AGOV,
  AIGUARD, POLFED, GATE) — see followup tasks #17–22.
- **Status corrections** — false-Ready signals demoted to Draft on GATE, ILGOV,
  TUIDASH, CEWS.
- **AGOV-002 → CPACKS migration** — the only intra-module consolidation.
- **Enterprise Readiness reclassification** — 7 modules promoted Tier C → B
  (followup task #23).

## Decisions referenced by this roadmap

- [ADR-011: Ratatui replaces Ink](./plans/decisions/011-ink-vs-ratatui-watch-mode-performance.md)
- [ADR-015: Intercept Loop Enforcement](./plans/decisions/015-intercept-loop-enforcement.md)
- [ADR-026: Rust scanner is authoritative](./plans/decisions/026-rust-scanner-authoritative.md)
- [ADR-027: Pack Architecture](./plans/decisions/027-pack-architecture.md)
- [ADR-028: Markdown Governance Crate](./plans/decisions/028-markdown-governance-crate.md)
- [ADR-029: Suppression Parser Authority](./plans/decisions/029-suppression-parser-authority.md)
- [ADR-031: Validation Latency Rubric](./plans/decisions/031-validation-latency-rubric.md)
- [ADR-030: Surface Drivers Supersede napi Cutover](./plans/decisions/030-surface-drivers-supersede-napi-cutover.md)

## What this roadmap is NOT

- **Not a schedule.** No quarter or month commitments. Sequence over date.
- **Not a feature list.** Themes and bets, not a backlog dump.
- **Not the source of truth for module status.** That lives in
  [`plans/index.aps.md`](./plans/index.aps.md). This roadmap is the narrative.
- **Not the release menu.** That lives in
  [`RELEASE-PLAN.md`](./RELEASE-PLAN.md).
