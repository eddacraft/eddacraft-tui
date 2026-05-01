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

> **Last refreshed:** 2026-05-01 (`v0.5.0-beta` shipped — H1 hype-builder
> release is now in market with the locked A1 + A2 + A3 + A4 slate;
> horizons rebased so H1 becomes post-release follow-up + daemon-backed
> RTV, H2 becomes second-surface driver reach, H3 stays GA; cherry-pick
> output re-derived against the new H1 window).
>
> **Purpose:** Hold the strategic context that does not survive a fresh
> chat. When a new session opens and asks "where are we, what is next,
> why are we doing it this way", read this first. The
> [`index.aps.md`](./index.aps.md) is the source of truth for module
> status; this file is the source of truth for **sequencing intent**.

---

## Where we are right now

- **Branch:** `chore/post-release-clean` off `dev`. Tag `v0.5.0-beta` cut
  from `release/v0.5.0-beta`; release branch merged back to `dev` via
  PR #1215.
- **What just shipped (`v0.5.0-beta`, 2026-05-01):** the locked
  A1 + A2 + A3 + A4 slate per
  [`RELEASE-PLAN.md`](../RELEASE-PLAN.md):
  - **A1 — RTAI Spike Slice** (24 items, INTD/INTR/RMCP/RTAI). Real-
    time AI validation fires before save through `anvil mcp serve --stdio`.
    Validation backend recorded as **embedded-fallback-backed, not daemon-
    backed**: RMCP-005's `DaemonValidationClient` defaults to `Unavailable`
    so MCP `tools/call` runs through embedded `anvil-checks`. Three GUI-
    dry-run gaps tracked outside the contract (#1194 missing `--command`
    override, #1195 Claude Code config-path mismatch, #1197 clients ignore
    `anvil_validate_write` without prompt instruction).
  - **A2 — AIGUARD** (4 items): `anvil gate --profile ai` + canonical
    `anvil.diagnostic.v1` envelope shared with RTAI / INTD / RMCP / DRVR.
  - **A3 — Release Engineering smallest-viable cut** (7 items): GHOOK-001,
    ATTRIB-001/-002/-003, SCAN-001/-002/-003. ATTRIB-004..-011 and
    SCAN-004/-005 stay queued.
  - **A4 — Language Credibility Floor** (9 items): LANGTS-001/-003,
    OPSUP-001 check-ID registry slice, SURFENV-001..-006.
  - **TRACE-001** — cross-cutting tracing baseline (anvil-observability
    crate, `traceparent` envelope round-trip, INTD-014 conformance assert).
    TRACE-002 (TS mirror) and TRACE-003 (redaction hardening) are post-
    launch.
- **Open follow-ups (foreground for the next release window):**
  - **Daemon-backed RMCP path** — replace RMCP-005's `Unavailable` stub
    with a live JSON-RPC client; graduate `tools/call` from embedded
    fallback to daemon-backed pipeline. Daemon side (`scan_buffer`,
    INTD-002 listener) already in place.
  - **V050F** — 5/16 done, 11 outstanding (per-operator audit attribution,
    cascade revoke, `/admin/approve` flag-gate, regex compile cache, eager
    rayon pool init, CI-class bench baseline, `release/*` push filter,
    custom-pattern compile errors, svix>uuid override removal,
    `WAITLIST_PAUSED` runbook).
  - **#1191** — wire RTAI mid-edit baseline-comparison gating into CI
    against the recorded 7-case ADR-031 corpus; until then, manual
    `cargo bench -p eddacraft-anvil-intercept --bench midedit_roundtrip`
    is the only safety net.
  - **TSRET-005 execution** — archive `crates/anvil-checks/tests/scanner_parity.rs`
    + the `packages/anvil/core/src/antipattern/` and
    `packages/anvil/core/src/suppression/parser.ts` trees + `tests/scanner-parity/`
    to `archive/anvil-ts-scanner/` per ADR-033. Unblocked but not executed
    pre-release.
- **Last few commits:**
  - `9e623aba2` fix(plans): correct broken Contents anchor for
    Infrastructure as Code
  - `4e727b5be` chore(plans): reconcile APS index against code
  - `92cd0967f` chore: merge release v0.5.0-beta back to dev (#1215)
  - `5e8040854` fix(intercept): avoid codeql macro false positive
  - `9ded49664` fix(release): address security review blockers

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

The funding context: the hype-builder release shipped on 2026-05-01 as
`v0.5.0-beta`, the **primary funding mechanism**. Influencers and
industry mates are lined up against this tag to drive waitlist signal;
the resulting numbers are what investors are looking at. The hype phase
is the highest-leverage horizon — the next release window has to convert
that signal into a credible "headline capability proven on the daemon
path" demo, not a warm-up for some later "real" launch.

---

## Three horizons

> No dates. Sequence and dependency only. Releases get planned after
> each release ships.

### H1 — Hype-builder release ✅ SHIPPED as `v0.5.0-beta` (2026-05-01)

**What it was.** First-touch surface recommendable to the audience the
launch is aimed at: demo-grade *but not embarrassing*. The product
story shipped as "save-time trust today, in-flight AI oversight
imminent" — except "imminent" turned out to mean *in the same release*:
the embedded fallback behind the MCP launch shim already validates AI
output before disk, just not yet through the daemon-backed path.

**What shipped.** The locked A1 + A2 + A3 + A4 slate (44 items).
LAUNCH-001 (glob filter), LAUNCH-004 (post-init auto-analysis), and
LAUNCH-005 (doctor remediation depth) all shipped Complete; LAUNCH-007
(unified interactive fix handling) closed in the same window.
LAUNCH-002, LAUNCH-003 and LAUNCH-006 remain Todo and roll into the
next-release window. DOCSYNC and EATEST shipped what fitted.

**Embedded-fallback caveat.** The launch demo runs through the embedded
`anvil-checks` pipeline behind RMCP, not the live daemon. This is the
single load-bearing caveat for the headline framing: RTV-before-disk
fires today; daemon-backed RTV-before-disk is the H2 deliverable.
Three GUI-dry-run gaps tracked outside the release contract (#1194 /
#1195 / #1197) and **do not retroactively un-ship A1**.

**Tag convention.** Settled as `-beta`. The X5 contradiction (Open
Decision 1) effectively resolved as **Option A by default** — `-beta`
shipped on the hype-builder cut and the daemon-backed product release
still has a tag-rename option open, but the rename is not blocking.

### H2 — Daemon-backed RTV + driver reach (next release window, slate not yet locked)

**What it is.** The product release. RTV — validating AI output mid-
edit through the **live daemon**, not the embedded fallback — is the
headline upgrade over `v0.5.0-beta`. A second surface (editor or second
MCP target) attaches via DRVR-001/-002. The "daemon + drivers"
architecture is no longer aspirational; it is the actual data path for
the next demo.

**Already shipped via A1 (counts toward H2 in the original framing).**
INTD-001/-002/-003/-005/-007/-013/-014, INTR-001/-002/-006/-008,
RMCP-001..-008, RTAI-001/-002/-003/-006/-008. The launch shim and the
mid-edit RPC are real; the daemon-backed wiring is the missing edge.

**What is left for H2.**

- **Daemon-backed RMCP** — replace RMCP-005's `Unavailable` stub with
  a live JSON-RPC client; verify the daemon-backed `tools/call`
  matches the embedded fallback envelope (RTAI contract test +
  AIGUARD-002 envelope shape).
- **DRVR-001 / DRVR-002** — shared driver client + editor-driver
  protocol.
- **RMCPF (Rust MCP Full Port)** — graduate the launch shim to feature
  parity with the archived TS MCP server.
- **RTAI-004 / -005 / -007 / -009** — driver-side debouncer, editor
  mid-edit path, telemetry mirror, architecture doc + supersession
  links.
- **Remaining INTD items** — -004 (watcher), -006 (process-group
  interrupt), -008..-012 (config / embedded / unregistered-change /
  status / Windows CI matrix), -015 (telemetry subscription scoping),
  -016 (DoS protection budgets).
- **#1191** — wire RTAI mid-edit baseline-comparison gating into CI
  against the recorded ADR-031 corpus.
- **TSRET-005 execution** — archive the TS scanner / suppression
  parser / parity harness per ADR-033. Unblocked but not executed
  pre-`v0.5.0-beta`.
- The post-release follow-ups in **V050F** that should ride this
  window (per-operator audit attribution, family-theft cascade,
  `/admin/approve` flag-gate, regex compile cache, eager rayon pool
  init, CI-class bench baseline, custom-pattern compile errors).

**Gate to call it ready.** Mid-edit diagnostics from a real AI tool
session reach the user inside the latency budget through the **daemon-
backed** path on at least one surface; the embedded fallback remains
correctness-equivalent. Architecture docs reflect shipped reality
(RTAI-009). RMCP-008 dry-run repeated against the daemon backend.

**Deferred.** Multi-driver parity (DRVR-008 capability negotiation
matters, but second-editor reach is H3). Reasoning-pattern catalogue
itself — the AI-001..AI-007 detectors live in `anvil-checks`, not in
RTAI; their own roadmap is downstream.

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

## The next release (daemon-backed RTV) in detail

This is the section the team executes against. Everything else in
this document is context. Slate **not yet locked**; the entries below
are the cherry-pick verdicts that survived the post-`v0.5.0-beta`
sweep.

### In scope (working slate, pending lock)

| ID | Module | Why it ships next |
|----|--------|-------------------|
| Daemon-backed RMCP | RMCP / RMCPF | Replace RMCP-005's `Unavailable` stub with a live JSON-RPC client; graduate `tools/call` from embedded fallback to daemon-backed pipeline. The single load-bearing caveat that gates the headline framing. |
| `DRVR-001` / `DRVR-002` | DRVR | Shared driver client + editor-driver protocol. Lets a second surface attach. |
| `RTAI-004` / `-005` / `-007` / `-009` | RTAI | Driver-side debouncer, editor mid-edit path, telemetry mirror, architecture doc + supersession links. Closes the H2 RTAI checklist. |
| Remaining `INTD-004/-006/-008..-012/-015/-016` | INTD | Watcher integration, process-group interrupt, config / embedded / unregistered-change / status / Windows CI matrix, telemetry subscription scoping, DoS protection budgets. |
| `INTR-003` / `-004` / `-005` / `-007` | INTR | The remaining rule traits + configuration; prereq for second-rule expansion behind the daemon. |
| `#1191` | (RTAI ops) | Wire ADR-031 mid-edit baseline-comparison gating into CI against the recorded 7-case corpus. Until landed, manual `cargo bench` is the only safety net. |
| `TSRET-005` | TSRET | Archive the TS scanner / suppression parser / parity harness to `archive/anvil-ts-scanner/` per ADR-033. Unblocked, not yet executed. |
| `LAUNCH-002` / `-003` / `-006` | LAUNCH | Watch polish that did not make `v0.5.0-beta`. Should land in this window so the watch flow is solid by the time the daemon-backed demo ships against it. -003 still watches for TUIDASH supersession. |
| V050F outstanding (11 items) | V050F | Per-operator audit attribution, family-theft cascade, `/admin/approve` flag-gate, regex compile cache, eager rayon pool init, CI-class bench baseline, `release/*` push filter, custom-pattern compile errors, svix>uuid override removal, `WAITLIST_PAUSED` runbook. |

Each ships as **a standalone PR**, not bundled. No single "next
release" branch. The slate is a **sequencing label**, not a release
label.

### Not in scope, deliberately

- All Web Dashboard waves. DASH/DASHCORE/DASHARCH/DASHOPS/DASHAI is
  the team-lead-surface horizon (H3); pulling any of it forward
  fragments the daemon-backed demo.
- All Policy Governance constellation work. OPAE/ORGHIER/POLLC/etc.
  remain Draft and stay there until H3.
- All Language & Coverage tail work beyond the smallest-viable A4 cut
  that already shipped.
- WEAVE / agent-infrastructure import. Schedule after the intercept-
  loop thesis is proven on the daemon path.
- Any new Open-Spec / Pocketflow / Graph v2 work — see REVISIT.

### The leverage move that is not in H2 but is gated by H2

**Plan the H3 dashboard scoping conversation now.** Once the daemon-
backed RTV path is live, the Dashboard MVP "Team-Lead Glance" cut
(Council B's ~12-of-39 80/20 slice) is the next coherent product
release after H2. The `anvil export` CLI work item is the load-bearing
glue between CLI artefacts and the dashboard read path. Neither is in
scope for H2; both should have a Ready Checklist drafted before H2
tags so the H3 cut does not start cold.

---

## Cherry-pick output

Verdicts assigned against the H1-shipped / H2-daemon-RTV / H3-GA frame
after `v0.5.0-beta`. Sorted by verdict, then by module ID. Active and
Draft modules only; already-archived modules are not re-listed.

### 🔥 SHIPPED in `v0.5.0-beta` — H1 is closed

| Module | ID | Final state at `v0.5.0-beta` ship | Notes |
|--------|----|-----------------------------------|-------|
| [launch-flow-readiness](./modules/launch-flow-readiness.aps.md) | LAUNCH | In Progress 5/7 | LAUNCH-001/-004/-005/-007 Complete; -002/-003/-006 roll into H2 watch polish. |
| [intercept-daemon](./modules/intercept-daemon.aps.md) | INTD | In Progress 7/16 | A1 slice closed: -001/-002/-003/-005/-007/-013/-014. -004/-006/-008..-012/-015/-016 roll into H2. |
| [intercept-rules](./modules/intercept-rules.aps.md) | INTR | In Progress 4/8 | A1 slice closed: -001/-002/-006/-008. -003/-004/-005/-007 roll into H2. |
| [rust-mcp-launch-shim](./modules/rust-mcp-launch-shim.aps.md) | RMCP | Complete 8/8 | Embedded-fallback-backed; daemon-backed graduation rolls into H2 RMCPF and RMCP follow-up. |
| [realtime-ai-validation](./modules/realtime-ai-validation.aps.md) | RTAI | In Progress 5/9 | A1 slice closed: -001/-002/-003/-006/-008. -004/-005/-007/-009 roll into H2. |
| [ai-guardrail-profile](./modules/ai-guardrail-profile.aps.md) | AIGUARD | Complete 4/4 | Diagnostic envelope shared with RTAI / RMCP / DRVR. |
| [git-config-hooks](./archive/modules/git-config-hooks.aps.md) | GHOOK | Complete 6/6 | A3 hygiene cut. |
| [attribution-pipeline-v3](./modules/attribution-pipeline-v3.aps.md) | ATTRIB | In Progress 3/11 | A3 smallest-viable cut: ATTRIB-001/-002/-003. -004..-011 roll into H2. |
| [scan-performance](./modules/scan-performance.aps.md) | SCAN | In Progress 3/5 | A3 smallest-viable cut: SCAN-001/-002/-003. -004/-005 stay queued. |
| [lang-ts-audit](./modules/lang-ts-audit.aps.md) | LANGTS | Ready 2/5 | A4 floor: -001/-003 shipped. -002/-004/-005 stay queued. |
| [operational-supplement](./modules/operational-supplement.aps.md) | OPSUP | In Progress 1/7 | A4 check-ID registry slice. -002..-007 stay queued. |
| [surface-env-files](./modules/surface-env-files.aps.md) | SURFENV | Complete 6/6 | A4 `.env` secret scan. |
| [tracing-foundation](./modules/tracing-foundation.aps.md) | TRACE | In Progress 1/3 | TRACE-001 (anvil-observability + traceparent) shipped; -002/-003 are post-launch. |

### ➡️ NEXT — needed for H2 daemon-backed RTV

The headline-capability work and everything that gates it.

| Module | ID | Status | Verdict rationale |
|--------|----|--------|-------------------|
| Daemon-backed RMCP / RMCPF | RMCP / RMCPF | RMCP Complete 8/8; RMCPF Draft 0/9 | Replace RMCP-005's `Unavailable` stub with a live JSON-RPC client; graduate `tools/call` from embedded fallback to daemon-backed. RMCPF brings full TS-MCP parity. The single load-bearing caveat from `v0.5.0-beta`. |
| [intercept-daemon](./modules/intercept-daemon.aps.md) | INTD | In Progress 7/16 | Remaining INTD-004/-006/-008..-012/-015/-016. Watcher integration, process-group interrupt, config / embedded / unregistered-change / status / Windows CI matrix, telemetry subscription scoping, DoS protection budgets. Critical-path tail. |
| [intercept-launcher](./modules/intercept-launcher.aps.md) | INTL | Draft 0/9 | Session ingress for shell-launched agents. Required for the daemon's session-attribution story to hold once non-editor agents (Claude Code in a tmux pane) become a demo target. |
| [intercept-rules](./modules/intercept-rules.aps.md) | INTR | In Progress 4/8 | Remaining INTR-003/-004/-005/-007. Rule trait extension + configuration; prereq for second-rule expansion behind the daemon. |
| [surface-drivers](./modules/surface-drivers.aps.md) | DRVR | Draft 0/4 active | DRVR-001/-002 (shared driver client + editor-driver protocol) is the H2 minimum. DRVR-003 (VSCode editor driver) deferred per ADR-033 until a new extension package is created on the daemon-driver path. DRVR-004 superseded by RMCP/RMCPF. |
| [realtime-ai-validation](./modules/realtime-ai-validation.aps.md) | RTAI | In Progress 5/9 | Remaining RTAI-004/-005/-007/-009. Driver-side debouncer, editor mid-edit path, telemetry mirror, architecture doc + supersession links. |
| [anvil-ts-scanner-retirement](./modules/anvil-ts-scanner-retirement.aps.md) | TSRET | In Progress 2/5 active | TSRET-005 unblocked under ADR-033 but execution is post-`v0.5.0-beta`. Module reaches terminal state once -005 archives the TS scanner / TS suppression parser / parity harness to `archive/anvil-ts-scanner/`. |
| `LAUNCH-002` / `-003` / `-006` (within LAUNCH) | LAUNCH | In Progress 5/7 | Watch polish that did not make `v0.5.0-beta`. Should land in this window so the watch flow is solid by the time the daemon-backed demo ships against it. LAUNCH-003 watches for TUIDASH supersession. |
| `#1191` (RTAI ops) | n/a | Open | Wire ADR-031 mid-edit baseline-comparison gating into CI against the recorded 7-case corpus. Until then, manual `cargo bench` is the only safety net. |
| [v050-release-followups](./modules/v050-release-followups.aps.md) | V050F | In Progress 5/16 | 11 outstanding hardening items deferred from `v0.5.0-beta` review rounds. Per-operator audit attribution, family-theft cascade, `/admin/approve` flag-gate, regex compile cache, eager rayon pool init, CI-class bench baseline, `release/*` push filter, custom-pattern compile errors, svix>uuid override removal, `WAITLIST_PAUSED` runbook. |

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
| [bmad-v4-backward-compat](./modules/bmad-v4-backward-compat.aps.md) | BMAD4 | Proposed | BMAD v4 backward compat. Demand for v4 is unproven post-v6.0.3. Defer or archive based on a real user signal. |

### ✅ DONE — already complete or in-flight under another module

| Module | ID | Status | Notes |
|--------|----|--------|-------|
| (Most of the index's "Complete" entries) | — | — | See [index.aps.md](./index.aps.md) Release Plan tables and [completed-index.aps.md](./completed-index.aps.md). |
| [nx-rust-plugin](./archive/modules/nx-rust-plugin.aps.md) | NXRUST | Complete (8/8) | Archived to `plans/archive/modules/`; sweep complete. |

---

## Open decisions

The decisions on this list change downstream sequencing. Each needs a
human call.

### 1. The X5 contradiction in ADR-030 — effectively resolved by Option B-ish

**Where it lived.** [`plans/decisions/030-surface-drivers-supersede-napi-cutover.md`](./decisions/030-surface-drivers-supersede-napi-cutover.md)
"Sequencing decision (2026-04-24, X5 closed)" section.

**How `v0.5.0-beta` resolved it.** Reality picked **(B)-ish**: INTD
work was *not* deferred behind the hype-builder cut. The A1 INTD slice
(INTD-001/-002/-003/-005/-007/-013/-014) shipped *as part of*
`v0.5.0-beta`, alongside RMCP and the RTAI A1 items, with a tag of
`-beta`. The launch shim is real but embedded-fallback-backed, so the
"beta is still aspirational on RTV" reading holds — but ADR-030's
"after the v0.4.0-beta release" language is now historical.

**Outstanding sub-question — tag rename for H2.** Whether the next
release (daemon-backed RTV) tags as `-beta` again, `-rc`, or graduates
out of `-beta` is still open and downstream of audience-reading
considerations. Not blocking; pick at H2 lock.

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

`v0.5.0-beta` shipped under `-beta` so the in-session call held. The
question only resurfaces if H2 (daemon-backed RTV) wants a different
tag (`-rc`, drop `-beta`, or stay). Decide at H2 lock; not blocking.

### 5. ~~Archive sweep owed~~ (resolved)

NXRUST module file is now at `plans/archive/modules/nx-rust-plugin.aps.md`
as expected. Sweep complete; the active-module list and the index
agree.

### 6. ~~The pre-flight scope of LAUNCH-005~~ (resolved)

LAUNCH-005 (doctor remediation depth) shipped Complete in
`v0.5.0-beta`. Decision retired.

---

## What recently changed

> Append-only. Newest entry first. A future session reads this to
> see what has moved since the last refresh.

### 2026-05-01 (`v0.5.0-beta` shipped — H1 closed, horizons rebased)

- `v0.5.0-beta` tagged from `release/v0.5.0-beta`, public release
  artefacts published, release branch merged back to `dev` via
  PR #1215.
- The locked A1 + A2 + A3 + A4 slate (44 items) shipped as a single
  cut. A1 RTAI Spike Slice closed at 24/24 with RMCP-008 Cursor /
  Claude Code GUI dry-run recorded in
  `plans/specs/2026-04-26-rtai-demo-runbook.md` §8. Backend recorded
  as **embedded-fallback-backed, not daemon-backed** — RMCP-005's
  `DaemonValidationClient` defaults to `Unavailable` and `tools/call`
  runs through the embedded `anvil-checks` pipeline. Three GUI-dry-
  run gaps tracked outside the contract (#1194 missing `--command`,
  #1195 Claude Code config-path mismatch, #1197 clients ignore
  `anvil_validate_write` without prompt instruction).
- TRACE-001 (anvil-observability crate, `traceparent` envelope,
  INTD-014 conformance assert) shipped as a cross-cutting baseline
  per ADR-034 / ADR-035; TRACE-002 (TS mirror) and TRACE-003
  (redaction hardening) remain post-launch.
- V050F advanced 5/16 during the release window (cargo-dist installer
  pin, scoop PAT scope, winget `gh` arg regression, migration runner,
  private-release Latest promotion). 11 items outstanding as
  post-release follow-ups.
- ADR-033 surface archives executed:
  `packages/vscode-extension/` → `archive/anvil-vscode-extension/`,
  `packages/mcp-server/` → `archive/anvil-mcp-server/`. TSRET-005
  (engine archive of TS scanner / TS suppression parser / parity
  harness to `archive/anvil-ts-scanner/`) **remains unblocked but
  not yet executed** — pending post-release window.
- X5 contradiction effectively resolved by reality: INTD work *did*
  ship inside the `-beta` cut. Tag-rename question for H2 is still
  open but no longer load-bearing.
- Strategic frame this snapshot records: H1 hype-builder is closed;
  H2 daemon-backed RTV + driver reach is the next-release window
  (slate not yet locked); H3 GA is unchanged.

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
ADR-030, ADR-033). Apply the H1 / H2 / H3 lens; assign one of
🔥 SHIPPED / ➡️ NEXT / 🌱 LATER / ❓ REVISIT / ✅ DONE per module; surface
contradictions explicitly; do not paper over the open decisions.

The "What recently changed" log above is the only section that
should be appended to rather than rewritten — it is the audit trail
between snapshots.
