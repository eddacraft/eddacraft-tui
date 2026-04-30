<!--
plans/next-steps.md
===================
Canonical session-continuity artefact for the Anvil project.

Read this first when picking up cold. Every other plan file describes
*what* a slice of work is; this file describes *why we are doing it
next*, against the strategic frame the team has actually committed to.

Snapshot, not a contract. Re-run the cherry-pick sweep after every
merge that changes strategic shape.
-->

# Anvil — Next Steps

> **Last refreshed:** 2026-04-29 (ADR-033 lands — IDE/MCP surfaces
> archived under `archive/anvil-vscode-extension/` and
> `archive/anvil-mcp-server/`; TSRET-005 (engine archive) is
> **unblocked** by ADR-033 but execution is **out of scope for
> this PR** and lands separately on `chore/TSRET-005`; TSRET-006
> superseded; DRVR-003 deferred until a new extension package is
> created on the daemon-driver path; module/index docs reconciled).
>
> **Purpose:** Hold the strategic context that does not survive a fresh
> chat. When a new session opens and asks "where are we, what is next,
> why are we doing it this way", read this first. The
> [`index.aps.md`](./index.aps.md) is the source of truth for module
> status; this file is the source of truth for **sequencing intent**.

---

## Where we are right now

- **Branch:** `dev`, clean against `origin/dev`.
- **Working tree at the moment this was written:** RTAI module
  ([`plans/modules/realtime-ai-validation.aps.md`](./modules/realtime-ai-validation.aps.md))
  is uncommitted; RTVF
  ([`plans/archive/modules/real-time-validation-full.aps.md`](./archive/modules/real-time-validation-full.aps.md))
  is moved into the archive and the original `plans/modules/`
  copy is deleted; `plans/index.aps.md` is updated to record both
  supersessions. RTVS
  ([`plans/archive/modules/real-time-validation-simplified.aps.md`](./archive/modules/real-time-validation-simplified.aps.md))
  was already archived in the previous commit. Orchestrator will
  commit these shortly.
- **Last few commits:**
  - `57be8fc1` docs(aps): mark TSRET-002 Complete under the
    ADR-030-reduced scope
  - `b65a9180` docs(launch): clarify LAUNCH-001 updates `--exclude`
    to glob semantics
  - `0383e00c` plan(launch): add LAUNCH cross-cutting module +
    supersede RTVS
  - `64ef65f0` docs(adr): record Option A for ADR-030 sequencing —
    INTD post-release
- **What just changed strategically (this session):**
  1. RTVS archived; its watch-flow intent moved into LAUNCH; its
     validation-engine intent moved into RTAI.
  2. RTAI authored from scratch on the daemon + drivers
     architecture, replacing RTVF entirely.
  3. Real-time AI-output validation (RTV) recognised as **the**
     Anvil headline capability — not a save-time linter feature.
  4. Cross-cutting module convention proven on its second use
     (LAUNCH → RTAI). Promotion to a first-class APS primitive is
     now an open decision, not a hypothetical.

---

## The pitch

Anvil is being built to be **the trust layer for AI-assisted
development**: it watches what an AI coding tool is producing — in
flight, while the agent is still typing — and tells the user (and the
agent itself, where the surface allows) when the in-flight change is
structurally or semantically off. Boundary violations, suppression
bypass, appeals-to-authority in planning docs, unjustified precision,
secret leaks, anti-patterns — caught **before** the file lands, not
after, not in PR review, not as a save-time post-hoc warning.

The architecture is daemon + drivers. A long-running Rust daemon
(`anvil-intercept`) owns the rule engine, the session registry, and
the enforcement ladder. Editor surfaces (VSCode, then any LSP-shaped
client), MCP surfaces, future surfaces (tmux, web sessions, remote
shells) attach as **drivers** over JSON-RPC 2.0. This is what
ADR-030 commits to and what the next architectural mile of work
exists to deliver.

The funding context: a hype-builder release is being prepared as the
**primary funding mechanism**. Influencers and industry mates are
lined up to drive a waitlist; the resulting signal is what investors
are looking at. The hype phase is the highest-leverage horizon, not
a warm-up for the "real" launch.

---

## Three horizons

> No dates. Sequence and dependency only. Releases get planned after
> each release ships.

### H1 — Hype-builder release

**What it is.** The next ship. Re-establishes Anvil's first-touch
surface so it is recommendable to the people the launch is aimed at:
demo-grade *but not embarrassing*. The product story is "save-time
trust today, in-flight AI oversight imminent" — proof of the headline
capability is staged for H2. H1's job is to convert hype into
waitlist signal without making the inevitable poke-around a
disappointment.

**What is in it.**

- LAUNCH pre-flight items: `LAUNCH-001` (glob filter actually works),
  `LAUNCH-004` (post-init auto-analysis restored), and probably
  `LAUNCH-005` (doctor remediation depth). Each ships as a standalone
  PR, not bundled.
- Hygiene items already in flight that happen to land before the
  cut (DOCSYNC, EATEST as time permits).

**Gate to call it ready.** The two pre-flight items are merged on
`dev`; `anvil init` lands on a useful first signal; `anvil watch
--patterns` actually filters; the existing welcome → init → watch
chain has no first-touch papercuts that would make a wait-listed
viewer close the tab.

**Deferred.** Polish that is not visible in the first ten minutes.
Anything that requires the daemon to exist. Anything that requires
DRVR to be in flight. The H1 ship is on the in-process Rust surfaces
that already exist.

**Tag convention.** Stays `-beta`. Renaming to `-preview` / `-RC` is
a live option (see Open Decisions) but is not blocking.

### H2 — Beta launch with real-time AI-output validation

**What it is.** The product release. RTV — validating AI output
mid-edit, not at save time — is live behind at least one driver. The
"daemon + drivers" architecture is no longer aspirational; it is the
actual data path for the headline demo.

**What is in it.**

- `INTD-001`/`-002` and the rest of INTD reaching a stable IPC
  surface.
- DRVR-001 (shared driver client), DRVR-002 (editor-driver protocol),
  and **either** DRVR-003 (VSCode editor driver) **or** DRVR-004
  (MCP driver) — pick the one that demos best, ship the other in
  H2 patch.
- `RTAI-001` spike → `RTAI-002`/`-003`/`-004` (mid-edit RPC,
  latency benchmark, driver-side debouncer) → either `RTAI-005`
  (editor mid-edit) or `RTAI-006` (MCP pre-write). Same demo-driven
  pick as DRVR-003 vs DRVR-004 — they pair.
- RTAI-007 (telemetry mirror), RTAI-008 (errors-as-first-class
  contract test), RTAI-009 (architecture doc + supersession links)
  before declaring H2 done.
- TSRET-005 (delete TS scanner) — comes for free once a driver
  ships, per ADR-030.

**Gate to call it ready.** Mid-edit diagnostics from a real AI tool
session reach the user inside the latency budget on at least one
surface. The other surface has a stub or known follow-up. Architecture
docs reflect shipped reality (RTAI-009).

**Deferred.** Multi-driver parity (DRVR-008 capability negotiation
matters, but second-editor reach is H3). Reasoning-pattern catalogue
itself — the AI-001..AI-007 detectors live in `anvil-checks`, not
in RTAI; their own roadmap is downstream.

### H3 — GA

**What it is.** Multi-driver, multi-language, compliance packs,
dashboard, the long tail of governance work that exists mostly as
Draft today.

**What is in it.** Track 1/2/3/4/5 language work; the Policy
Governance constellation (OPAE, ORGHIER, POLLC, COMPLY, POLFED,
POLVAL, ARCHCFG, AIGUARD, OPAG, EVAL, CEWS, CPOL, IORISK, GATE,
ATC, PATT, TRUST, AGOV, CPACKS); the Web Dashboard waves
(DASH, DASHCORE, DASHARCH, DASHOPS, DASHAI); WEAVE; CFGINT; OBS;
CGBDG; SCAN remainder; FLAGCAT; BMAD4; UCFG; the long-tail Future
modules.

**Gate to call it ready.** Out of scope for this document. Scope it
when H2 ships.

**Deferred from explicit H3 list.** Items flagged REVISIT below — a
strategic decision is owed before they consume effort.

---

## The next release (hype-builder) in detail

This is the section the team executes against. Everything else in
this document is context.

### In scope

| ID | Module | Why it ships in H1 |
|----|--------|--------------------|
| `LAUNCH-001` | LAUNCH | `--patterns` and `--exclude` actually filter. The current code declares the fields but never consumes them; first-time visitors will try this and notice. |
| `LAUNCH-004` | LAUNCH | Post-init auto-analysis. `anvil init` currently ends on "now run `anvil doctor`" — the hype-phase user clicks away there. IFR-003 shipped this in TS; the Rust port regressed it. |
| `LAUNCH-005` (probable) | LAUNCH | Doctor remediation depth. Bare "see README" references on `anvil doctor` failures kill the demo. Confidence is medium; ship if it slots in, defer if it grows. |

Each ships as **a standalone PR**, not bundled. No "LAUNCH release"
branch. The pre-flight bundle is a **sequencing label**, not a
release label.

### Not in scope, deliberately

- `LAUNCH-002` (allow `--action` with TUI mode). Worth doing, not a
  first-touch papercut. NEXT.
- `LAUNCH-003` (real watch TUI stats rollup). Coordinates with
  TUIDASH; the bespoke surface may go away. NEXT, with eye on
  Superseded-by TUIDASH-009.
- `LAUNCH-006` (welcome `--skip-to watch` shortcut). Returning-user
  optimisation. NEXT.
- All RTAI work. RTAI is the H2 headline; staging part of it into
  H1 to look impressive will not finish in time and will fragment
  the demo.
- All DRVR work. Same reasoning.
- All INTD work *except as preparation* — see immediately below.

### The leverage move that is not in H1 but is gated by H1

**Staff INTD-001 / INTD-002 immediately.** Per ADR-030's Sequencing
Decision (Option A), INTD picks up "straight after the v0.4.0-beta
release". The H1 ship is what unblocks that; the team should be
ready to point at INTD on the day H1 cuts. RTAI-001 (the spike) is
deliberately designed to start against a partial INTD, so the
critical-path delay between H1 and H2 is dominated by INTD reaching
"stable enough to spike against", not by INTD reaching done.

This is also where the **X5 contradiction** lives — see Open
Decisions.

---

## Cherry-pick output

Verdicts assigned against the H1-funding / H2-RTV / H3-GA frame.
Sorted by verdict, then by module ID. Active and Draft modules only;
already-archived modules are not re-listed.

### 🔥 HYPE — needed for the next release

| Module | ID | Status | Verdict rationale |
|--------|----|--------|-------------------|
| [launch-flow-readiness](./modules/launch-flow-readiness.aps.md) | LAUNCH | Draft | Owns the pre-flight items. Module itself is not "complete in H1" — only LAUNCH-001 / -004 / (-005) ship. |

That is the entire HYPE list. The funding-driven release rides on
**three work items** out of one module, against the existing
in-process Rust surfaces. Adding anything else to HYPE is what burns
the release window.

### ➡️ NEXT — needed for H2 beta with RTV

The headline-capability work and everything that gates it.

| Module | ID | Status | Verdict rationale |
|--------|----|--------|-------------------|
| [intercept-daemon](./modules/intercept-daemon.aps.md) | INTD | Draft | The daemon. RTAI cannot exist without it; DRVR cannot function without it. Pick up immediately after H1 cut per ADR-030 Option A. Critical-path root. |
| [intercept-launcher](./modules/intercept-launcher.aps.md) | INTL | Draft | Session ingress for shell-launched agents. Required for the daemon's session-attribution story to hold once non-editor agents (Claude Code in a tmux pane) become a demo target. |
| [intercept-rules](./modules/intercept-rules.aps.md) | INTR | Draft | Rule trait + initial rule set. RTAI evaluates whatever INTR registers. Cannot ship the headline demo without at least the secret-detection and antipattern wrappers running on the hot path. |
| [surface-drivers](./modules/surface-drivers.aps.md) | DRVR | Draft | The driver framework. DRVR-001/-002 plus one of DRVR-003/-004 is the H2 minimum. |
| [realtime-ai-validation](./modules/realtime-ai-validation.aps.md) | RTAI | Proposed | The headline. Spike (RTAI-001) starts against partial INTD; the rest blocks on INTD + DRVR reaching the pinned deliverables. |
| [anvil-ts-scanner-retirement](./modules/anvil-ts-scanner-retirement.aps.md) | TSRET | In Progress | TSRET-005 unblocked under ADR-033 — IDE/MCP surfaces archived under `archive/anvil-vscode-extension/` and `archive/anvil-mcp-server/`; TS scanner + TS suppression parser + parity harness move to `archive/anvil-ts-scanner/` (no longer blocking on DRVR). TSRET-006 superseded — transition window collapses. Module reaches terminal state once -005 lands. |
| [notification-framework cross-link] (telemetry stream contract) | n/a | Complete | Already merged; called out so future readers do not re-open it. RTAI-007 rides on INTD-013 which rides on this contract. |
| `LAUNCH-002` / `-003` / `-006` (within LAUNCH) | LAUNCH | Draft | Watch polish that does not block H1 but should land in the H1→H2 window so the watch flow is solid by the time the headline demo ships against it. LAUNCH-003 watches for TUIDASH supersession. |

### 🌱 LATER — needed for H3 GA, parked until H2 ships

These have real product value at GA. They do not move the needle for
either the funding-phase demo or the headline-capability launch.
Letting them consume cycles before H2 is the most likely failure mode
for the project.

#### Web dashboard

| Module | ID | Status | Verdict rationale |
|--------|----|--------|-------------------|
| [dashboard-foundation](./modules/dashboard-foundation.aps.md) | DASH | Ready | Useful for team-leads / compliance roles; not the developer-trust story. |
| [dashboard-core-views](./modules/dashboard-core-views.aps.md) | DASHCORE | Ready | Same. |
| [dashboard-architecture-views](./modules/dashboard-architecture-views.aps.md) | DASHARCH | Ready | Same. |
| [dashboard-ops-views](./modules/dashboard-ops-views.aps.md) | DASHOPS | Ready | Same. |
| [dashboard-ai-builder](./modules/dashboard-ai-builder.aps.md) | DASHAI | Draft | json-render builder; a wow demo, but not the wow demo. |
| [tui-dashboard-render](./modules/tui-dashboard-render.aps.md) | TUIDASH | Ready | TUI side of json-render. May supersede LAUNCH-003 — important to *track*, not to *ship* in H1/H2. |

#### Policy governance constellation

All the OPA-derived governance work. Right premise, wrong horizon.
None of this exists in code; all are Draft.

| Module | ID | Status | Verdict rationale |
|--------|----|--------|-------------------|
| [opa-enhancements](./modules/opa-enhancements.aps.md) | OPAE | Draft | 36 tasks. Battle-tested core engine prerequisite. |
| [org-policy-hierarchy](./modules/org-policy-hierarchy.aps.md) | ORGHIER | Draft | Multi-repo / fleet aggregation. |
| [policy-lifecycle](./modules/policy-lifecycle.aps.md) | POLLC | Draft | Versioning and rollout for policies. |
| [compliance-reporting](./modules/compliance-reporting.aps.md) | COMPLY | Draft | SOC 2 / ISO 27001 mapping. |
| [policy-federation](./modules/policy-federation.aps.md) | POLFED | Draft | Cross-org policy sharing. |
| [policy-pack-validation](./modules/policy-pack-validation.aps.md) | POLVAL | Draft | Policy-pack QA. |
| [architecture-config-validation](./modules/architecture-config-validation.aps.md) | ARCHCFG | Draft | Architecture config QA. |
| [ai-guardrail-profile](./modules/ai-guardrail-profile.aps.md) | AIGUARD | Draft | Guardrail-style governance profile. RTAI is the *capability*; AIGUARD is the *enterprise packaging*. Sequence in that order. |
| [opa-agent-orchestration](./modules/opa-agent-orchestration.aps.md) | OPAG | Ready | Orchestration of OPA evaluation. |
| [eval-harness-integration](./modules/eval-harness-integration.aps.md) | EVAL | Ready | External eval framework adapter. |
| [compliance-evidence-workspace](./modules/compliance-evidence-workspace.aps.md) | CEWS | Ready | Evidence collection for compliance. |
| [contextual-policy-assertions](./modules/contextual-policy-assertions.aps.md) | CPOL | Ready | Per-context policy assertions. |
| [io-risk-controls](./modules/io-risk-controls.aps.md) | IORISK | Ready | IO-risk taxonomy and scanner. |
| [gateway-control-plane-patterns](./modules/gateway-control-plane-patterns.aps.md) | GATE | Ready | Reference topologies for control-plane gateways. |
| [adversarial-testing-catalog](./modules/adversarial-testing-catalog.aps.md) | ATC | Ready | Adversarial probe catalogue. |
| [prompt-attack-regression-packs](./modules/prompt-attack-regression-packs.aps.md) | PATT | Ready | Prompt-attack regression. |
| [trust-center-automation](./modules/trust-center-automation.aps.md) | TRUST | Ready | Automated trust-centre publishing. |
| [agent-governance-patterns](./modules/agent-governance-patterns.aps.md) | AGOV | Draft | Governance patterns for agent-driven flows. |
| [compliance-policy-packs](./modules/compliance-policy-packs.aps.md) | CPACKS | Draft | Off-the-shelf compliance policy packs. |

> The "Ready" status on many of these is **specification-ready**, not
> "ready to execute against the funding window". Do not pull from
> this list to fill H1/H2 cycles.

#### Language and coverage

The five-track plan. Excellent design work; entirely H3.

| Module | ID | Status | Verdict rationale |
|--------|----|--------|-------------------|
| [lang-ts-audit](./modules/lang-ts-audit.aps.md) | LANGTS | Draft | Anchor item zero. |
| [lang-rust](./modules/lang-rust.aps.md) | RSTLAN | Draft | Anchor T3. |
| [lang-python](./modules/lang-python.aps.md) | PYLAN | Draft | Anchor T3. |
| [lang-tail-wave](./modules/lang-tail-wave.aps.md) | LANGTAIL | Draft | Tail T1 batched sprint. |
| [surface-sql-migrations](./modules/surface-sql-migrations.aps.md) | SURFSQL | Draft | Phase 1 surface. |
| [surface-github-actions](./modules/surface-github-actions.aps.md) | SURFGHA | Draft | Phase 2 surface. |
| [surface-dockerfile](./modules/surface-dockerfile.aps.md) | SURFDOCK | Draft | Phase 3 surface. |
| [surface-shell](./modules/surface-shell.aps.md) | SURFSH | Draft | Phase 3 surface. |
| [surface-env-files](./modules/surface-env-files.aps.md) | SURFENV | Draft | Phase 3 surface. |
| [pack-pulumi](./modules/pack-pulumi.aps.md) | PACKPUL | Draft | Phase 1 pack. |
| [pack-llm-provider](./modules/pack-llm-provider.aps.md) | PACKLLM | Draft | Phase 1+2 pack. Tactically relevant to the AI-trust pitch — promote to NEXT *if* a pack-style demo would land better than the editor demo, not by default. |
| [pack-drizzle](./modules/pack-drizzle.aps.md) | PACKDRZ | Draft | Phase 2 pack. |
| [pack-nextjs](./modules/pack-nextjs.aps.md) | PACKNXT | Draft | Phase 2 pack. |
| [pack-hono](./modules/pack-hono.aps.md) | PACKHON | Draft | Phase 2 pack. |
| [pack-tokio](./modules/pack-tokio.aps.md) | PACKTOK | Draft | Phase 2 pack. |
| [markdown-governance](./modules/markdown-governance.aps.md) | MDGOV | Draft | Track 5. |
| [operational-supplement](./modules/operational-supplement.aps.md) | OPSUP | Draft | Cross-track infrastructure for the language tracks. |

#### Other H3 modules

| Module | ID | Status | Verdict rationale |
|--------|----|--------|-------------------|
| [observability-foundation](./modules/observability-foundation.aps.md) | OBS | Draft | Telemetry contract / Neon health / runbooks. Real, but not what the launch is being judged on. |
| [feature-flag-catalogue](./modules/feature-flag-catalogue.aps.md) | FLAGCAT | Draft | Manifest unification across surfaces. Nice-to-have. |
| [scan-performance](./modules/scan-performance.aps.md) | SCAN | Proposed | Roll out the parallel-scan pattern to remaining call-sites. Low-risk wins; opportunistic. |
| [bmad-v4-backward-compat](./modules/bmad-v4-backward-compat.aps.md) | BMAD4 | Proposed | Compat for BMAD v4 documents. Demand-pulled. |
| [council-gate-bridge](./modules/council-gate-bridge.aps.md) | CGBDG | Proposed | LLM council → deterministic attestation bridge. Intriguing premise, no demo lift for H1/H2. |
| [api-governance](./modules/api-governance.aps.md) | APGOV | Proposed | API contract governance. |
| [security](./modules/security.aps.md) | SEC | Proposed | Cargo audit / pnpm audit cadence. Hygiene. |
| [testing-strategy](./modules/testing-strategy.aps.md) | TEST | Proposed | Strategy doc; the executable test work is TCOV/TINT/TEXT below. |
| [test-coverage-uplift](./modules/test-coverage-uplift.aps.md) | TCOV | In Progress | 14/25; Phase 4 needs scope refresh. Background hygiene. |
| [test-integration-surface](./modules/test-integration-surface.aps.md) | TINT | Draft | Integration boundary tests. |
| [test-external-services](./modules/test-external-services.aps.md) | TEXT | Draft | External service contract tests. |
| [early-access-tests](./modules/early-access-tests.aps.md) | EATEST | Ready | Tests-that-would-have-caught-real-bugs. Slot opportunistically. |
| [early-access-migration](./modules/early-access-migration.aps.md) | EAMIG | Ready | Deferred council findings. Slot opportunistically. |
| [documentation-sync](./modules/documentation-sync.aps.md) | DOCSYNC | In Progress | Keep docs current with H1 ship; rolling. |
| [schema-contracts](./modules/schema-contracts.aps.md) | SCHEMA | Proposed | Schema-contract enforcement. |
| [config-intelligence](./modules/config-intelligence.aps.md) | CFGINT | Draft | Cross-language dependency graph from config files. Premise survives RTV pivot — feeds the architecture-edge detector. H3. |
| [rust-cli-tier2](./modules/rust-cli-tier2.aps.md) | RCLI2 | Proposed | Tier 2 commands (`check`, `pr-comment`, `policy-debug`, etc.). Useful, not blocking. |
| [rust-cli-tier3](./modules/rust-cli-tier3.aps.md) | RCLI3 | Proposed | Tier 3 commands (Edda, APS, Agent). Required to fully archive the Node CLI; not blocking. |
| [weave](./modules/weave.aps.md) | WEAVE / AHARNESS | Draft | Agent runtime (Apache-2.0, in `eddacraft/weave-rs`) plus harness with zero-copy graph access. Greenfield — schedule after the intercept-loop thesis is proven, per index. |
| [intent-ledger-governance](./modules/intent-ledger-governance.aps.md) | ILGOV | Ready | Intent-ledger governance. |
| [lineage-authorship-confidence](./modules/lineage-authorship-confidence.aps.md) | LAC | Ready | Lineage / authorship confidence. |
| [graph-context-delivery](./modules/graph-context-delivery.aps.md) | GCTX | Draft | Graph-context delivery for policy evaluation. |
| [pocketflow-gateway](./modules/pocketflow-gateway.aps.md) | PFGW | Draft | Gateway integration. |
| [open-spec-adapter](./modules/open-spec-adapter.aps.md) | OPENSPEC | Draft | Parse open-spec format as a planning source. |
| [unified-config-format](./modules/unified-config-format.aps.md) | UCFG | Proposed | Unified config-format ADR (ADR-016 Proposed). H3 unless a surface forces it sooner. |
| [nx-rust-plugin](./modules/nx-rust-plugin.aps.md) | NXRUST | Complete (8/8) | Listed Complete in index but file remains in active modules; archive sweep owed. See REVISIT. |

### ❓ REVISIT — premise should be re-examined

Modules whose framing predates either the daemon + drivers
architecture (ADR-030) or the RTV-is-the-product framing of this
session. Each is a decision waiting to be made.

| Module | ID | Status | What to revisit |
|--------|----|--------|-----------------|
| [unified-config-format](./modules/unified-config-format.aps.md) | UCFG | Proposed | The driver-framework already implies `.anvil.yaml` as the config root. Does UCFG still buy anything beyond what driver-config + INTD-008 already specify? Re-read against the ADR-030 stack before scheduling. |
| [open-spec-adapter](./modules/open-spec-adapter.aps.md) | OPENSPEC | Draft | Anvil's own planning is APS, not open-spec. Was this targeting cross-tool plan ingestion? If yes, low priority; if it was speculative, close it. |
| [pocketflow-gateway](./modules/pocketflow-gateway.aps.md) | PFGW | Draft | Pocketflow integration predates the driver-framework. The driver-framework ADR positions MCP as a driver; does PFGW still have a distinct role, or is it absorbed by DRVR + MCP driver? |
| [council-gate-bridge](./modules/council-gate-bridge.aps.md) | CGBDG | Proposed | Bridges Claude-Code council reviews into Anvil attestations. Discovery-first by design. Worth keeping, but verify the bridge target (attestation format) hasn't shifted under the daemon work. |
| [graph-context-delivery](./modules/graph-context-delivery.aps.md) | GCTX | Draft | Graph context for policy evaluation. INTR explicitly *forbids* graph recomputation on the hot path. GCTX may belong to the cold-path policy evaluation story; the framing should be re-pointed. |
| [intent-ledger-governance](./modules/intent-ledger-governance.aps.md) | ILGOV | Ready | "Ready" but the executable consumer is unclear post-RTAI. Confirm scope. |
| [lineage-authorship-confidence](./modules/lineage-authorship-confidence.aps.md) | LAC | Ready | Same — promising, but does it *plug into* INTD-013's notification envelope cleanly, or does it want a separate emission path? |
| [nx-rust-plugin](./modules/nx-rust-plugin.aps.md) | NXRUST | Complete (8/8) | Listed Complete in index Hardening table but the module file is still in `plans/modules/`. Archive sweep owed (`git mv` to `plans/archive/modules/`). |
| [bmad-v4-backward-compat](./modules/bmad-v4-backward-compat.aps.md) | BMAD4 | Proposed | BMAD v4 backward compat. Demand for v4 is unproven post-v6.0.3. Defer or archive based on a real user signal. |

### ✅ DONE — already complete or in-flight under another module

| Module | ID | Status | Notes |
|--------|----|--------|-------|
| (Most of the index's "Complete" entries) | — | — | See [index.aps.md](./index.aps.md) Release Plan tables and [completed-index.aps.md](./completed-index.aps.md). |
| `nx-rust-plugin` | NXRUST | Complete (8/8) | But also flagged REVISIT for archive sweep. |

---

## Open decisions

The decisions on this list change downstream sequencing. Each needs a
human call.

### 1. The X5 contradiction in ADR-030

**Where it lives.** [`plans/decisions/030-surface-drivers-supersede-napi-cutover.md`](./decisions/030-surface-drivers-supersede-napi-cutover.md)
"Sequencing decision (2026-04-24, X5 closed)" section.

**The contradiction.** ADR-030's Option A says "INTD-001 / INTD-002
are picked up straight after the v0.4.0-beta release". This session
re-framed RTV-on-drivers as the **launch-blocker** for the actual
product (H2 beta). RTV requires INTD + DRVR + RTAI. So either:

- INTD work *is* part of the beta (and ADR-030's "after the beta"
  language is wrong), or
- The beta cut now has two stages and "beta" needs renaming, or
- "Beta" stays the name for the H1 hype-builder ship and the H2
  product launch needs a different label.

**Three options surfaced this session.**

- **(A) Rename current "beta" to preview / RC.** Reserve "beta" for
  the H2 product launch with RTV. Smallest sequencing change; a
  marketing/tag-rename cost only. Preserves ADR-030 Option A's
  intent (INTD picks up straight after the *current* cut, which is
  now called preview). This is the cleanest reading.
- **(B) Rewrite X5 to start INTD now, in parallel with H1.**
  Eliminates the gap between H1 and INTD start. Costs: INTD work
  inevitably bleeds into H1 review bandwidth; ADR-030 needs editing;
  the team has to defend "two parallel mainlines".
- **(C) Two-stage beta (beta-1, beta-2).** Honest but loud. The
  funding-phase audience does not necessarily know to read the suffix.

**Recommendation deferred — this is the user's call, not the
orchestrator's.** All three are coherent; the choice depends on what
the funding-phase audience reads as "beta" and how loud a rename
costs.

### 2. Promote the cross-cutting module convention to a first-class APS primitive?

**Where it lives.** Trialled in
[LAUNCH](./modules/launch-flow-readiness.aps.md) ("Cross-cutting
convention" section). Reused in
[RTAI](./modules/realtime-ai-validation.aps.md) ("Cross-cutting
convention" section, with explicit acknowledgement of the second-use
trigger).

**The rule LAUNCH set.** "Do not copy this convention to a second
module before it has been tried in anger here. If the pattern proves
useful across at least one further cross-cutting bundle, promote it
to a first-class module type in `aps-rules.md` — ideally with a
machine-readable callout syntax (e.g. YAML frontmatter) so a lint can
verify references."

**The trigger has fired.** RTAI is the second use. The pattern works:
both modules have legible cross-references that survive review.
**Open question:** promote now (write the rule into
[`plans/aps-rules.md`](./aps-rules.md), add a typed callout shape,
add a lint), or one more cycle of trial?

**Risk of waiting.** A third author copies the prose convention into
a third module; cross-references silently rot at the next module
rename or archive (the same risk LAUNCH's own warning called out).
The rot is real but slow; promoting now adds a small build cost (the
lint).

### 3. RTAI's three open questions (from RTAI's own "Open questions" section)

- **3a. Mid-edit blocking semantics.** Does a mid-edit diagnostic
  ever escalate to `block` / `interrupt`, or is it always advisory?
  The MCP pre-write path *can* refuse a tool call. The LSP
  `didChange` path *cannot* prevent the editor from showing the user
  their own keystrokes. Asymmetric capability needs to be in the
  protocol from RTAI-002 if it is going to land at all — bolting on
  later is the bad shape.
- **3b. Where do reasoning-pattern rules live?** Add to existing
  `anvil-checks` antipattern crate, or carve out a new
  `anvil-checks-reasoning` crate? Decide before RTAI-003 lands. RTAI
  is intentionally agnostic here — it consumes whatever INTR
  registers — but the catalogue authors need a target.
- **3c. Mid-edit + suppression UX.** Save-time has a suppression
  model. Mid-edit diagnostics have no on-disk anchor yet. Out of
  scope for v1, but the protocol must not preclude it.

### 4. Tag rename bandwidth

Independent of decision 1, the team agreed in-session that "tag
convention stays `-beta`" because rename effort is not worth it
right now. Worth re-checking once X5 is closed — if X5 lands on
Option A (rename to preview/RC), the tag rename folds in.

### 5. Archive sweep owed

**NXRUST** module file lives in `plans/modules/nx-rust-plugin.aps.md`
but the index lists it Complete (8/8) under the Hardening table. Run
`git mv` to `plans/archive/modules/` next time the modules directory
is touched. Same sweep should re-check anything else the index
records as Complete that hasn't been archived.

### 6. The pre-flight scope of LAUNCH-005

LAUNCH-005 (doctor remediation depth) is a candidate for H1 but its
own confidence is medium. Decision: ship if it slots between
LAUNCH-001 and LAUNCH-004 review, defer to NEXT if it grows. Does
not need to be made in advance.

---

## What recently changed

> Append-only. Newest entry first. A future session reads this to
> see what has moved since the last refresh.

### 2026-04-29 (ADR-033 — archive IDE/MCP, retire TS scanner now)

- ADR-033 authored
  ([`plans/decisions/033-park-ide-mcp-retire-ts-scanner.md`](./decisions/033-park-ide-mcp-retire-ts-scanner.md)),
  Status Proposed. Archives the VSCode extension and TS MCP
  server under the project's `archive/<name>/` convention
  (precedent: `archive/anvil-cli-node/` ADR-012,
  `archive/anvil-tui-ink/` ADR-011a); retires TS scanner / TS
  suppression parser / parity harness under TSRET-005 to
  `archive/anvil-ts-scanner/` rather than waiting on
  DRVR-003/-004; CI for archived packages switches off via the
  existing `'!archive/**'` workspace exclusion; napi crate stays
  as a build canary; surfaces return as **new** active packages
  via DRVR / RMCPF / a future return-path ADR.
- ADR-026, 028, 029, 030 carry append-only Status notes pointing
  at ADR-033. Decisions in those ADRs are unchanged; only the
  carve-outs and sequencing referencing TS-stays-alive are
  amended/moot.
- DECISION-LOG updated: new ADR-033 row under Rust Migration;
  amendment markers on 026/028/029/030 rows.
- TSRET module re-pointed: TSRET-005 unblocked (no longer waits
  on DRVR; rewritten as "Archive the TS scanner" with explicit
  `archive/anvil-ts-scanner/` destination); TSRET-006 superseded
  (transition window collapses); module reaches terminal state
  once -005 lands.
- DRVR module re-pointed: DRVR-003 (VSCode editor driver)
  deferred until a new extension package is created on the
  daemon-driver path; DRVR-001/-002/-005 continue against
  existing INTD dependencies.
- RMCPF module re-pointed: starts from "TS MCP server is
  archived" rather than "active migration source"; Decision #4
  amended; RMCPF-031 partially executed by ADR-033 (package
  already in `archive/`).
- Index updated for TSRET / DRVR / RMCPF rows; ADR-033 added to
  the Intercept and Drivers section's Architecture Decisions.
- *Open Decisions §1 (the X5 contradiction) is unaffected — ADR-033
  is about TS engine code, not about INTD sequencing relative to
  the H1 cut.*
- **Surface packages physically archived:**
  `packages/vscode-extension/` → `archive/anvil-vscode-extension/`;
  `packages/mcp-server/` → `archive/anvil-mcp-server/` (both via
  `git mv`). READMEs rewritten to the Archived banner shape
  matching `archive/anvil-cli-node/`. Workspace glob
  `'!archive/**'` already excludes them from build/test/publish.
  `pnpm-lock.yaml` regenerated to drop the stale workspace
  references (Vercel deployments use `--frozen-lockfile` and
  failed on the original commit).
- **Root configs reconciled:**
  `pnpm-workspace.yaml` — `packages/mcp-server` and
  `packages/vscode-extension` lines removed (replaced by an
  archive-pointer comment).
  `tsconfig.json` — `./packages/mcp-server` project reference
  dropped.
  `tsconfig.base.json` — `@eddacraft/anvil-mcp-server` path
  mapping dropped.
  `vitest.config.ts` — `@eddacraft/anvil-mcp-server` and
  `vscode` mock aliases dropped (with archive-pointer comments).
- All active-plan cross-refs (`plans/index.aps.md`,
  `plans/modules/{anvil-ts-scanner-retirement,surface-drivers,rust-mcp-launch-shim,rust-mcp-full-port}.aps.md`,
  `plans/specs/{rust-mcp-launch-shim,anvil-driver-framework/editor-and-mcp-driver-design}.md`)
  updated from `packages/{vscode-extension,mcp-server}/` to the
  `archive/anvil-{vscode-extension,mcp-server}/` paths.
- **Still untouched:** CI workflow disabling for the parity
  harness job; TSRET-005 execution proper (move
  `packages/anvil/core/src/antipattern/`,
  `packages/anvil/core/src/suppression/parser.ts`,
  `tests/scanner-parity/` to `archive/anvil-ts-scanner/` and rip
  out inbound imports across `packages/`); deletion of the
  Rust-side `crates/anvil-checks/tests/scanner_parity.rs`. These
  are real refactors that need build-green verification —
  pending approval of ADR-033 before execution.

### 2026-04-26 (v0.4.0-beta release prep)

- Branch `release/v0.4.0-beta` cut from `dev` for the H1 hype-builder
  tag. Stabilisation strategy. CHANGELOG and ENGINEERING-HISTORY
  rewritten in the 0.3.3-beta user-centric style covering the full
  390-commit window (LAUNCH, NOTIFY, RSCAN/ANVFMT/SPG, RUSTNX,
  ADMINCLIH, FLAGM, TSRET, DIST-011, plus a 7-issue GH sweep).
- Workspace + 14 bundled `package.json` manifests + `crates/anvil-checks-napi/package.json`
  bumped to `0.4.0-beta`. Cargo.lock regenerated. `cargo-hakari` clean.
- Three rounds of council ran against the release branch, plus an
  external Codex CLI review each round. ~25 findings surfaced
  in total; 18 fixed in-flight across rounds 1–3 (commits
  `eae47b3d`, `f9961b28`, `6f16b059`, `907af5f2` plus the bench
  refresh and CI-unblock commits). 10 hardening items consciously
  deferred to V050F (see new module).
- V050F module created
  ([`plans/modules/v050-release-followups.aps.md`](./modules/v050-release-followups.aps.md))
  with 10 work items, status Ready. Captures the 10 deferred items
  so the deferral does not silently rot:
  cargo-dist installer pin (V050F-001), per-operator audit attribution
  (V050F-002), family-theft cascade revoke (V050F-003), `/admin/approve`
  flag-gate (V050F-004), graded-scope regression tests (V050F-005),
  allowlist regex compile cache (V050F-006), eager rayon pool init
  (V050F-007), CI-class bench baseline (V050F-008), `release/*` push
  filter (V050F-009), `WAITLIST_PAUSED` runbook (V050F-010).
- Bench baselines refreshed on `release/v0.4.0-beta`:
  `antipattern_scan` ≈ 11.2 ms (~28.6 K artefacts/s), 23% faster than
  the 2026-04-22 pre-RUSTNX-008 baseline. Kernel hot-path rewrite
  (monotonic `next_id` counter + `HashSet<String>` of tracked files)
  validated by the new measurement.

### 2026-04-24 (this session)

- LAUNCH module created
  ([`plans/modules/launch-flow-readiness.aps.md`](./modules/launch-flow-readiness.aps.md))
  with 6 work items, status Draft. Trials the cross-cutting module
  convention.
- RTVS superseded and archived
  ([`plans/archive/modules/real-time-validation-simplified.aps.md`](./archive/modules/real-time-validation-simplified.aps.md)).
  Watch intent → LAUNCH; validation core intent → RTAI.
- RTAI module created
  ([`plans/modules/realtime-ai-validation.aps.md`](./modules/realtime-ai-validation.aps.md))
  with 9 work items, status Proposed. Real-time AI-output validation
  on the daemon + drivers architecture per ADR-030. Reuses the
  cross-cutting convention from LAUNCH.
- RTVF superseded and archived
  ([`plans/archive/modules/real-time-validation-full.aps.md`](./archive/modules/real-time-validation-full.aps.md))
  in the same change. Its "unified validation server" framing
  pre-dated ADR-030; the work is replaced by RTAI on the daemon.
- ADR-030 sequencing decision (Option A) recorded —
  [`plans/decisions/030-surface-drivers-supersede-napi-cutover.md`](./decisions/030-surface-drivers-supersede-napi-cutover.md)
  Sequencing section. Council review item X5 closed in that doc, but
  re-opened *strategically* by this session (see Open Decision 1).
- Council session council-881a8ca8 ran on the RTAI / LAUNCH change
  set: 10 findings, 8 fixed, 2 deferred, verdict PASS.
- Strategic frame this document records was set in this session: RTV
  is the product, hype phase funds the product, ship-now-with-pre-
  flight, no time estimates, cherry-pick lens for sequencing, second
  use of cross-cutting convention is the trigger to consider
  promotion.
- TSRET-002 marked Complete under the ADR-030-reduced scope (commit
  `57be8fc1`).

---

## How to refresh this doc

This doc is a **snapshot**, not a contract. It rots whenever:

- A module supersedes another, gets archived, or changes status.
- An ADR lands that changes the strategic shape (architecture,
  sequencing, or product framing).
- A release ships and the horizons re-base.

**When to re-run.**

- After every merge that touches `plans/modules/` materially.
- After every ADR landing.
- Always at the start of a new release window (re-cherry-pick all
  modules against the new H1).

**How to re-run.** Invoke the cherry-pick agent with the same brief
that produced this version: read `plans/aps-rules.md`,
`plans/index.aps.md`, every active module file (skim for
purpose + status), the critical-path modules in full
(LAUNCH / RTAI / INTD / DRVR), and the load-bearing ADRs (currently
ADR-030). Apply the H1 / H2 / H3 lens; assign one of
🔥 HYPE / ➡️ NEXT / 🌱 LATER / ❓ REVISIT / ✅ DONE per module; surface
contradictions explicitly; do not paper over the open decisions.

The "What recently changed" log above is the only section that
should be appended to rather than rewritten — it is the audit trail
between snapshots.
