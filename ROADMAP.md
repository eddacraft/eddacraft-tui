# Anvil Roadmap

**Last updated:** 2026-05-01 (post `v0.5.0-beta` ship — Horizons 0–2 rebased)

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

### Horizon 0 — Just Shipped (`v0.5.0-beta`, 2026-05-01)

`v0.5.0-beta` shipped the locked A1 + A2 + A3 + A4 slate as a single tagged
release. The `release/v0.5.0-beta` branch has been merged back to `dev`; the
current branch (`chore/post-release-clean`) carries the post-release planning
sweep.

What landed:

- **A1 — RTAI Spike Slice** (24 items across INTD, INTR, RMCP, RTAI). Real-time
  AI validation fires before save through `anvil mcp serve --stdio`. The release
  was validated **embedded-fallback-backed, not daemon-backed**: RMCP-005's
  `DaemonValidationClient` defaults to `Unavailable` and MCP `tools/call` runs
  through the embedded `anvil-checks` pipeline. Three GUI-dry-run gaps tracked
  outside the contract: #1194 (`mcp install --command` override), #1195 (Claude
  Code path mismatch), #1197 (clients ignore `anvil_validate_write` without
  explicit prompt instruction).
- **A2 — AIGUARD** (4 items). `anvil gate --profile ai` with the stable
  `anvil.diagnostic.v1` envelope shared with RTAI / INTD / DRVR / RMCP.
- **A3 — Release Engineering smallest-viable cut** (7 items): GHOOK-001,
  ATTRIB-001/-002/-003, SCAN-001/-002/-003. ATTRIB-004..-011 and SCAN-004/-005
  remain outside this release.
- **A4 — Language Credibility Floor** (9 items): LANGTS-001/-003, OPSUP-001
  check-ID registry slice, SURFENV-001..-006 (`.env` secret scanning).
- **TRACE-001** — cross-cutting tracing baseline (anvil-observability crate, W3C
  `traceparent` propagation, redaction layer, INTD-014 fixture). TRACE-002 (TS
  mirror) and TRACE-003 (redaction hardening) are post-launch.

Headline post-release follow-ups (foreground for the next release window):

- **Daemon-backed RMCP path** — replace RMCP-005's `Unavailable` stub with a
  live JSON-RPC client; graduate MCP `tools/call` from the embedded fallback to
  the daemon-backed pipeline. The daemon side (`scan_buffer` RPC, INTD-002
  listener) is already in place.
- **V050F** (`v050-release-followups`) — 6/16 done; 10 hardening items
  outstanding (per-operator audit attribution, family-theft cascade,
  `/admin/approve` flag-gate, allowlist regex compile cache, eager rayon pool
  init, CI-class bench baseline, `release/*` push filter, etc.).
- **Latency CI gating** (#1191) — RTAI mid-edit baseline-comparison gating
  against the recorded 7-case ADR-031 corpus. Until #1191 lands, regressions are
  caught only by manual `cargo bench` runs.
- **TSRET-005 execution** — archive the TS scanner / TS suppression parser /
  parity harness to `archive/anvil-ts-scanner/` per ADR-033. Unblocked but not
  executed pre-release.

### Horizon 1 — Daemon-Backed RTV + Driver Reach (next release, slate not yet locked)

**Theme: graduate real-time AI validation from embedded fallback to the
daemon-backed pipeline, and bring at least one editor / second-MCP surface
online.**

`v0.5.0-beta` proved the demo with the embedded `anvil-checks` pipeline behind
the MCP launch shim. The next release closes the daemon path end-to-end and
starts the driver-reach story:

- **Daemon-backed RMCP** — wire the live JSON-RPC client; verify the
  daemon-backed `tools/call` matches the embedded fallback envelope (RTAI
  contract test, AIGUARD-002 envelope shape).
- **DRVR-001 / DRVR-002** — shared driver client + editor-driver protocol; the
  framework that lets a second surface attach.
- **RMCPF (Rust MCP Full Port)** — graduate the launch shim to feature parity
  with the archived TS MCP server; reuse the AIGUARD envelope.
- **RTAI-004 / -005 / -007 / -009** — driver-side debouncer, editor mid-edit
  path, telemetry mirror, architecture doc + supersession links.
- **Remaining INTD items** (-004, -006, -008..-012, -015, -016) — watcher
  integration, process-group interrupt, configuration loading, embedded mode,
  unregistered-change handling, status / diagnostics, Windows CI matrix,
  telemetry subscription scoping, DoS protection budgets.
- **ADR-031 latency CI gating** (#1191).

Slate **not yet locked**. Cherry-pick verdict against this horizon lives in
[`plans/next-steps.md`](./plans/next-steps.md).

### Horizon 2 — Credibility & Hygiene (carry-over from `v0.5.0-beta`)

A3 and A4 shipped their **smallest-viable cuts** in `v0.5.0-beta`. The broader
hygiene tracks remain queued, eligible to ride the next-release window without
contending for RTAI engineering bandwidth:

- **Release Engineering tail** — ATTRIB-004..-011 (full attribution pipeline
  v3), SCAN-004..-005 (parallel-scan rollout to remaining call-sites).
- **Language Credibility tail** — LANGTS-002/-004/-005, OPSUP-002..-007 (drift
  schema versioning, per-track flags, file-presence guards, FP reporting),
  SURFSQL Phase 1.

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
- **Graph v2 Foundation (GV2)** — joined semantic/dependency/trust/control/
  provenance graph substrate. Anvil-first foundation for enforcement and
  provenance; assistant context delivery is a projection over it.
- **Rust MCP Full Port (RMCPF)** — next-release full parity port of the existing
  TypeScript MCP server after RMCP proves the launch path.
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

| Bet                                         | Why it matters                                                                                | Where it lives                                          |
| ------------------------------------------- | --------------------------------------------------------------------------------------------- | ------------------------------------------------------- |
| **Intercept Daemon + Rust MCP launch shim** | Mechanical validation at the surface where AI tools propose writes, without a Node sidecar.   | INTD, INTR, RMCP, RTAI (Horizon 1)                      |
| **Real-time AI validation**                 | The launch demo. Refuse the bad write before disk.                                            | RTAI (Horizon 1)                                        |
| **AI guardrail diagnostic envelope**        | Stable JSON shape AI tools consume. The integration surface.                                  | AIGUARD (Horizon 1)                                     |
| **Dashboard for non-developer personas**    | Team-lead/platform-engineer/compliance — the buyer that funds the tool.                       | DASH/DASHCORE (Horizon 3)                               |
| **Enterprise readiness constellation**      | The org-tier deployment story. Becoming important soon.                                       | GATE/POLFED/ORGHIER/POLLC/COMPLY/CEWS/TRUST (Horizon 4) |
| **Graph v2 substrate**                      | Joined structural model for enforcement, trust, control, provenance, and later agent context. | GV2/GCTX (Horizon 6+)                                   |
| **Symbol-graph-driven effect prediction**   | What ILGOV becomes — predict effect of a change against captured intent.                      | ILGOV rescope (Horizon 6)                               |

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
- [ADR-030: Surface Drivers Supersede napi Cutover](./plans/decisions/030-surface-drivers-supersede-napi-cutover.md)
- [ADR-031: Validation Latency Rubric](./plans/decisions/031-validation-latency-rubric.md)
- [ADR-033: Park IDE/MCP, retire TS scanner](./plans/decisions/033-park-ide-mcp-retire-ts-scanner.md)

## What this roadmap is NOT

- **Not a schedule.** No quarter or month commitments. Sequence over date.
- **Not a feature list.** Themes and bets, not a backlog dump.
- **Not the source of truth for module status.** That lives in
  [`plans/index.aps.md`](./plans/index.aps.md). This roadmap is the narrative.
- **Not the release menu.** That lives in
  [`RELEASE-PLAN.md`](./RELEASE-PLAN.md).
