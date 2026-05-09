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

> **Last refreshed:** 2026-05-09 (`v0.6.0-beta` substrate locked + ready to
> tag; daemon-working slate proposed as the next window per
> [`RELEASE-PLAN.md` → NEXT RELEASE WINDOW](../RELEASE-PLAN.md#next-release-window-proposed--post-v060-beta-daemon-working-slate);
> horizons rebased: H1 + H2 closed-as-shipped, H3 daemon-working becomes the
> next-release window, H4 GA pushed back).
>
> **Purpose:** Hold the strategic context that does not survive a fresh
> chat. When a new session opens and asks "where are we, what is next,
> why are we doing it this way", read this first. The
> [`index.aps.md`](./index.aps.md) is the source of truth for module
> status; this file is the source of truth for **sequencing intent**.

---

## Where we are right now

- **Branch:** `dev`. Tag candidate for `v0.6.0-beta` ready; A1 (Wow-Start
  Activation) and A2 (Daemon-Backed RMCP + Driver Reach Waves 1–3) both
  fully shipped on `dev`. `v0.5.1-beta` was the last public tag (2026-05-03).
- **What just shipped (`v0.6.0-beta` substrate, locked and ready to tag):**
  - **A1 — Wow-Start Activation** (LAUNCH 18/18). `install → cd repo →
    anvil start` is the canonical first minute. Cursor and Claude Code MCP
    paths activate honestly; watch mode is the save-time fallback when MCP
    can't attach.
  - **A2 — Daemon-Backed RMCP + Driver Reach** (INTD 16/16, DRVR 5/5
    active, RTAI 6/9, RMCP 8/8). MCP `tools/call` runs through the daemon
    when owner-only IPC is available; embedded path remains correctness-
    equivalent fallback. Editor-driver protocol + capability negotiation
    + shared TS driver client all shipped. A2 Wave 4 (RTAI-005/-007/-009,
    DRVR-003) deferred per ADR-033 — VS Code extension archived.
  - **Release artefacts:** `docs/runbooks/v0.6.0-beta-security-note.md`
    (4 HIGH trade-offs documented) and
    `docs/runbooks/v0.6.0-beta-release-runbook.md` (5 ops items).
- **What was already shipped before this window:**
  - **`v0.5.0-beta` (2026-05-01):** AI guardrails + mid-edit validation.
    Validation backend was embedded fallback, not yet daemon-backed.
  - **`v0.5.1-beta` (2026-05-03):** Scanner signal + TUI hotfixes.
- **Open follow-ups carried by `v0.6.0-beta`:**
  - **V050F** — 14/16 done; 2 non-blocking carry-overs (CI-class bench
    baseline `V050F-008`, `svix → uuid` override removal `V050F-015`).
  - **V060F** — 2/25 done; nominations + as-built sweep follow-ups filed
    2026-05-07 (CLI gaps, macOS interrupt branch, JSON-RPC method
    surface drift, kernel spec divergences, etc.).
  - **Tracking issue [#1233](https://github.com/eddacraft/anvil-001/issues/1233)**
    — durable release log; close when no further entry needed.
- **What's been proposed for the next window (slate not yet locked):**
  - **Daemon-working slate** — MLP (17 items) + INTL (9 items) + 6 carry-
    forward gates + documentation runbooks. Target tag candidate
    `v0.7.0-beta`. **MLP-009** (protection-claim contract test suite) is
    the hard release gate. Spec:
    [`plans/specs/2026-05-07-anvil-multilayer-protection-architecture.md`](./specs/2026-05-07-anvil-multilayer-protection-architecture.md).

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
the enforcement ladder. Editor surfaces, MCP surfaces, and future
surfaces (tmux, web sessions, remote shells) attach as **drivers** over
JSON-RPC 2.0. ADR-030 commits to this; `v0.6.0-beta` made the data
path real.

The next architectural mile — **the daemon-working slate** — turns the
daemon from "available when invoked" into "always-on, in-tree,
defensible." The witness chain is the load-bearing primitive: every
commit carries a hash-chained record of which layers fired, in-tree,
travelling with git. Pre-commit / pre-push hooks are deterministic
gates that can't be bypassed silently. `anvil baseline` lets existing
repos adopt without a wash of warnings. `anvil-run` wraps shell-
launched agents so the session-attribution story holds outside the
editor.

The funding context: hype-builder shipped, daemon substrate shipped.
The next-release window has to convert that proven substrate into a
defensible **claim** — "Anvil protects this project" must be testable,
not a slogan. The protection-claim contract test suite (MLP-009) is
the hard gate that prevents the claim from drifting.

---

## Four horizons

> No dates. Sequence and dependency only. Releases get planned after
> each release ships.

### H1 — Hype-builder release ✅ SHIPPED as `v0.5.0-beta` (2026-05-01)

**What shipped.** A1 (RTAI Spike Slice) + A2 (AIGUARD) + A3 (Release
Engineering smallest-viable cut) + A4 (Language Credibility Floor) —
44 items as a single tagged release. Real-time AI validation fires
before save through the MCP launch shim. The `anvil.diagnostic.v1`
envelope is shared across RTAI / RMCP / DRVR / INTD.

**Caveat (closed by H2).** Validation backend was embedded-fallback-
backed, not daemon-backed.

**Patch follow-up.** `v0.5.1-beta` (2026-05-03) — scanner false-positive
fixes, TUI zoom controls, audit env-template filtering, kernel import
bug fixes.

### H2 — Daemon-backed RTV + driver reach ✅ SHIPPED as `v0.6.0-beta` (substrate locked 2026-05-08)

**What shipped.** Wow-start activation (A1: LAUNCH 18/18) +
daemon-backed RMCP graduation (A2: RMCP 8/8, INTD 16/16, DRVR 5/5
active, RTAI 6/9). MCP `tools/call` runs through the live daemon when
owner-only IPC is available; embedded path remains correctness-
equivalent fallback. Editor-driver protocol + capability negotiation +
shared TS driver client all shipped. The "daemon + drivers"
architecture is no longer aspirational.

**Caveat (addressed by H3).** Daemon is "available when invoked" —
not always-on. Hooks don't fire. Witness chain doesn't exist.
Baseline adoption story is missing. `anvil-run` is still draft.

**Deferred.** A2 Wave 4 (RTAI-005/-007/-009, DRVR-003) per ADR-033 —
VS Code extension archived; replacement editor surface decision is a
separate horizon.

### H3 — Daemon working end-to-end (next release window, slate proposed)

**What it is.** The release that flips the daemon from "available when
invoked" to "always-on, in-tree, defensible." `anvil start` lands a
real testable protection claim. Hooks fire deterministically.
`anvil/witnessed.ndjson` records every commit with a hash chain.
Existing repos can adopt via `anvil baseline` without a wash of
warnings. Sub-agent waves get per-task fence isolation.
`anvil-run` wraps shell-launched agents.

**Source of truth for sequencing.**
[`RELEASE-PLAN.md` → NEXT RELEASE WINDOW](../RELEASE-PLAN.md#next-release-window-proposed--post-v060-beta-daemon-working-slate)
holds the full wave plan, parallelisation map, and dependency graph.

**Capabilities delivered.**

- **Witness chain.** `anvil/witnessed.ndjson` (active) + manifest +
  archive with rollover; `flock`-protected hash chain; DAG-aware
  verification; `merge=union -text` via `.gitattributes`.
- **Hook surface.** Pre-commit / pre-push / post-commit / post-merge /
  post-rewrite. Self-contained binary, framework-agnostic. Silent on
  success, terse on failure, repeat-suppressed.
- **L4 policy framework.** `anvil/policy.yml` per-branch rules;
  `validate_at_l4` server-side fallback; `cutoff_commit` legacy
  acceptance.
- **Baseline adoption.** `anvil baseline` scans + grandfathers per
  rule class; `secrets` and `command-safety` are hard-pinned and
  cannot be config-disabled.
- **Multi-agent coordination.** Per-task fence isolation,
  `(WorktreeKey, AgentTag)` composite session key,
  cascade-fence detection.
- **Wrapped-launch ingress.** `anvil-run` + shell integration; PGIDs /
  Job Objects let the daemon target interrupts; drop-guard cleanup.
- **L5 audit.** `anvil audit` on-demand re-scan + nightly cron template.
- **Air-gapped guarantee.** Core operation tested under network-blocked
  sandbox.

**Hard release gate.** **MLP-009** — protection-claim contract test
suite. Pinned states (`unprotected | warming | pre-write-only |
save-time-only | full | degraded | cross-boundary-mixed | path-uncertain`)
must all be reachable in fixtures and rendered claims must match.
**No MLP item ships Complete in `index.aps.md` until that suite is
green.**

**Tag candidate.** `v0.7.0-beta`. Narrower fall-back shape
(`v0.6.x-beta`) is the "Daemon-backbone" combo from RELEASE-PLAN.md
— witness backbone + hooks + baseline + config without pre-push,
multi-session, audit, GH Action.

**Deferred from H3.** Multi-driver parity (DRVR-008 capability
negotiation already shipped, but second-editor-surface reach stays in
H4). Reasoning-pattern catalogue itself (AI-001..AI-007 detectors live
in `anvil-checks`, not in MLP). GitHub App / GitLab native
integrations. Anvil cloud sidecar.

### H4 — GA

**What it is.** Multi-driver, multi-language, dashboard, compliance
packs, enterprise constellation, the long tail of governance work that
exists mostly as Draft today.

**What is in it.**

- **Team-lead dashboard surface.** DASH foundation + warnings list +
  detail panel; `anvil export` CLI bridge.
- **Enterprise readiness constellation.** Gateway / federation /
  hierarchy / lifecycle / compliance / evidence workspace / trust
  centre.
- **Coverage breadth.** Phase 1 → Phase 3 of the language-and-coverage
  design.
- **Long bets.** Agent infrastructure (WEAVE), Graph v2 substrate,
  effect prediction (ILGOV rescope), lineage / authorship confidence.

**Gate to call it ready.** Out of scope for this document. Scope
when H3 ships.

---

## The next release (daemon-working slate) in detail

This is the section the team executes against. Everything else in
this document is context. Slate **proposed**; lock occurs when the
ADRs (036/037/038/039) are promoted Proposed → Accepted.

### In scope (working slate, pending lock)

| Pick | What | Why it ships next |
|------|------|-------------------|
| **N1 — MLP** (17 items) | Witness chain + hooks + L4 policy + baseline + multi-agent coordination + rule distribution | The architectural mile that turns the daemon into always-on protection. Hard gate: MLP-009. |
| **N2 — INTL** (9 items) | `anvil-run` wrapped-launch ingress | Closes the session-attribution gap for shell-launched agents (Claude Code in tmux, etc.). Coordinates `AgentTag` proto with MLP-014. |
| **N3 — Carry-forward gates** (6 items) | ADR promotion, project-id, noise-discipline audit, AIGUARD re-run, INTR-004 promotion, DRVR fwd-compat | Pre-positioning A7 defined for the current release. None can land alongside Proposed ADRs. |
| **N4 — Documentation** (6 docs) | Adoption / air-gap / witness-chain / hooks-integration runbooks; migration note; INTL manpage | Air-gapped operation, baseline adoption, and witness-chain ergonomics need operator-facing docs at tag time. |

Each item ships as **a standalone PR**, not bundled. No single
"next release" branch. The slate is a **sequencing label**, not a
release label.

### Wave summary (cross-lane parallelisation)

| Wave | Active in parallel | Gate to next |
|------|--------------------|--------------|
| 0 — Gates | ADRs Accepted, noise-discipline audit, DRVR fwd-compat, doc skeletons | All ADRs Accepted |
| 1 — Foundations | Witness backbone, config + rules_sha, air-gap harness, INTL scaffold | MLP-002 contract pinned |
| 2 — Adoption + connectivity | Hooks (pre-commit / post-* / bootstrap), baseline, L4 policy, INTL daemon path, INTR-004 | Hooks deterministic on green path |
| 3 — Coordination + extensions | Pre-push, multi-session, audit, L1→Kindling, GH Action, INTL spawn + cleanup, hook side-channel, runbooks | All consumer-facing items code-complete |
| 4 — Hard gate | MLP-009 contract suite, INTL cross-platform tests, runbooks final, migration note | Suite green; tag |

### Hard sequencing rules

1. **No `anvil-witness` consumer ships before MLP-002 contract pin.**
   Witness line schema cannot drift after first hook merges.
2. **Hard-pinned class enforcement (MLP-013) merges with the config
   parser (MLP-011).** A loophole window for security-class rules is
   unacceptable.
3. **MLP-009 last.** Protection-claim states must be reachable before
   the contract suite pins them.
4. **`AgentTag` proto change is one shared PR** between MLP-014 and
   INTL-003. Do not let either ship a session shape the other has to
   break.

### Not in scope, deliberately

- All Web Dashboard waves. The team-lead surface is H4; pulling any
  of it forward fragments the daemon-working window.
- All Policy Governance constellation work beyond the L4 framework
  in MLP-006. OPAE / ORGHIER / POLLC / etc. stay parked.
- All Language & Coverage tail work. LANGTS-002/-004/-005,
  OPSUP-002..-007, SURFSQL Phase 1 — H4.
- WEAVE / agent-infrastructure import. Schedule after the daemon-
  working thesis is proven.
- Any new Open-Spec / Pocketflow / Graph v2 work — see REVISIT.

### The leverage move that is not in H3 but is gated by H3

**Plan the H4 dashboard scoping conversation now.** Once the daemon-
working slate is live, the Dashboard MVP "Team-Lead Glance" cut is
the next coherent product release. The `anvil export` CLI work item is
the load-bearing glue between CLI artefacts and the dashboard read
path. Neither is in scope for H3; both should have a Ready Checklist
drafted before H3 tags so the H4 cut does not start cold.

---

## Cherry-pick output

Verdicts assigned against the H1-shipped / H2-shipped / H3-daemon-
working / H4-GA frame after the `v0.6.0-beta` substrate lock. Sorted
by verdict, then by module ID. Active and Draft modules only;
already-archived modules are not re-listed.

### 🔥 SHIPPED in `v0.5.0-beta` / `v0.5.1-beta` / `v0.6.0-beta` substrate

| Module | ID | Final state | Notes |
|--------|----|-------------|-------|
| [launch-flow-readiness](./modules/launch-flow-readiness.aps.md) | LAUNCH | Complete 18/18 | Wow-Start Activation A1 — full slate. |
| [intercept-daemon](./modules/intercept-daemon.aps.md) | INTD | Complete 16/16 | A1 slice + A2 Waves 1–3. Daemon binary, IPC, watcher, fence, enforcement, telemetry, status, DoS budgets. |
| [intercept-rules](./modules/intercept-rules.aps.md) | INTR | In Progress 4/8 | A1 slice closed: -001/-002/-006/-008. -003/-005/-007 stay queued; **-004 (path-deny) promoted to N3 G5**. |
| [rust-mcp-launch-shim](./modules/rust-mcp-launch-shim.aps.md) | RMCP | Complete 8/8 | Daemon-backed `tools/call` + embedded fallback both shipping; A2 graduated the daemon-vs-embedded path. |
| [realtime-ai-validation](./modules/realtime-ai-validation.aps.md) | RTAI | In Progress 6/9 | A1 + A2 Waves 1–3 closed. -005/-007/-009 deferred per ADR-033. |
| [ai-guardrail-profile](./modules/ai-guardrail-profile.aps.md) | AIGUARD | Complete 4/4 | Diagnostic envelope shared across RTAI / RMCP / DRVR / INTD. |
| [surface-drivers](./modules/surface-drivers.aps.md) | DRVR | Complete 5/5 active | Editor-driver protocol + capability negotiation + shared TS driver client. -003 deferred per ADR-033; -004 superseded by RMCP/RMCPF. |
| [git-config-hooks](./archive/modules/git-config-hooks.aps.md) | GHOOK | Complete 6/6 | A3 hygiene cut. |
| [attribution-pipeline-v3](./modules/attribution-pipeline-v3.aps.md) | ATTRIB | In Progress 3/11 | A3 smallest-viable cut. -004..-011 stay queued. |
| [scan-performance](./modules/scan-performance.aps.md) | SCAN | In Progress 3/5 | A3 smallest-viable cut. -004/-005 stay queued. |
| [lang-ts-audit](./modules/lang-ts-audit.aps.md) | LANGTS | In Progress 2/5 | A4 floor: -001/-003 shipped. -002/-004/-005 stay queued. |
| [operational-supplement](./modules/operational-supplement.aps.md) | OPSUP | In Progress 1/7 | A4 check-ID registry slice. -002..-007 stay queued. |
| [surface-env-files](./modules/surface-env-files.aps.md) | SURFENV | Complete 6/6 | A4 `.env` secret scan. |
| [tracing-foundation](./modules/tracing-foundation.aps.md) | TRACE | In Progress 1/3 | TRACE-001 shipped. -002/-003 post-launch. |
| [v050-release-followups](./modules/v050-release-followups.aps.md) | V050F | In Progress 14/16 | Two non-blocking carry-overs: V050F-008, V050F-015. |
| [anvil-ts-scanner-retirement](./modules/anvil-ts-scanner-retirement.aps.md) | TSRET | Complete (terminal) | TSRET-005 archive cascade executed. Module reaches terminal state. |

### ➡️ NEXT — needed for H3 daemon-working slate

The architectural mile and everything that gates it.

| Module | ID | Status | Verdict rationale |
|--------|----|--------|-------------------|
| [multilayer-protection](./modules/multilayer-protection.aps.md) | MLP | Proposed 0/17 | The witness chain + hooks + L4 + baseline + multi-agent coordination + rule distribution backbone. **Hard gate: MLP-009** (protection-claim contract suite). New crates: `anvil-witness`, `anvil-hook`, `anvil-l4`, `anvil-config`, `anvil-baseline`, `anvil-attribution`. |
| [intercept-launcher](./modules/intercept-launcher.aps.md) | INTL | Draft 0/9 | `anvil-run` wrapped-launch ingress for shell-launched agents. New crate: `anvil-run`. **Coordinates `AgentTag` proto with MLP-014.** |
| ADRs to promote | n/a | Proposed | ADR-036 (rewritten), ADR-037, ADR-038, ADR-039 — all must be Accepted before any MLP code merges. **N3 G1.** |
| [intercept-rules](./modules/intercept-rules.aps.md) | INTR-004 only | Draft | Path-deny rule promoted from B1 → **N3 G5**. Rule registration metadata feeds MLP-013 hard-pinned class. |
| [v060-release-candidates](./modules/v060-release-candidates.aps.md) | V060F | In Progress 2/25 | As-built sweep follow-ups filed 2026-05-07. Triage and slot opportunistically across H3 waves. |
| #1233 closeout | n/a | Open | Tracking issue for the v0.5.x release log; close when no further log entry needed. |

### 🌱 LATER — needed for H4 GA, parked until H3 ships

These have real product value at GA. Letting them consume cycles
before H3 is the most likely failure mode for the project.

#### Web dashboard (the team-lead surface)

| Module | ID | Status | Verdict rationale |
|--------|----|--------|-------------------|
| [dashboard-foundation](./modules/dashboard-foundation.aps.md) | DASH | Ready | Buyer surface; not the developer-trust story. |
| [dashboard-core-views](./modules/dashboard-core-views.aps.md) | DASHCORE | Ready | The smallest credible demo: warnings list + detail panel. Pin to today's CLI `--json` shapes; ship-now over governance. |
| [dashboard-architecture-views](./modules/dashboard-architecture-views.aps.md) | DASHARCH | Demoted Ready → Draft | Pending real schema source from `crates/anvil-architecture` + drift snapshot format. |
| [dashboard-ops-views](./modules/dashboard-ops-views.aps.md) | DASHOPS | Ready (subset) | Plan/role/AI-tool views are spec-orphan. Config viewer + diagnostics ride H4 dashboard cut. |
| [dashboard-ai-builder](./modules/dashboard-ai-builder.aps.md) | DASHAI | Draft | Wave 4 of dashboard. |
| [tui-dashboard-render](./modules/tui-dashboard-render.aps.md) | TUIDASH | Demoted Ready → Draft | Pending DASHAI catalogue resolution and schema source pin. |
| `anvil export` CLI work item | n/a | Not yet filed | The load-bearing glue between CLI and dashboard. File before H4 cut. |

#### Enterprise readiness constellation

The org-tier deployment story. **Promotion-gated** — first enterprise
prospect or design-partner request lights this horizon up.

| Module | ID | Status | Verdict rationale |
|--------|----|--------|-------------------|
| [gateway-control-plane-patterns](./modules/gateway-control-plane-patterns.aps.md) | GATE | Draft | Foundation: deployment topology + enforcement contract. |
| [policy-federation](./modules/policy-federation.aps.md) | POLFED | Draft | Multi-repo publish/subscribe over OPAE bundle primitives. |
| [org-policy-hierarchy](./modules/org-policy-hierarchy.aps.md) | ORGHIER | Draft | Multi-level inheritance. |
| [policy-lifecycle](./modules/policy-lifecycle.aps.md) | POLLC | Draft | Canary, grace periods, changelog generation. |
| [compliance-reporting](./modules/compliance-reporting.aps.md) | COMPLY | Draft | SOC 2 / ISO 27001 / NIST mapping. |
| [compliance-evidence-workspace](./modules/compliance-evidence-workspace.aps.md) | CEWS | Draft | Auditor surfaces; depends on COMPLY-001..004. |
| [trust-center-automation](./modules/trust-center-automation.aps.md) | TRUST | Draft | Public trust-artifact publishing pipeline. |

Sequence: GATE + POLFED + ORGHIER + POLLC first; COMPLY + CEWS +
TRUST second.

#### Policy governance support modules

| Module | ID | Status | Verdict rationale |
|--------|----|--------|-------------------|
| [opa-enhancements](./modules/opa-enhancements.aps.md) | OPAE | Draft | 36 tasks. Only policy-library + bundle inheritance pieces are launch-relevant; defer until a "policy library beats gate" slice. |
| [policy-pack-validation](./modules/policy-pack-validation.aps.md) | POLVAL | Draft | Necessary precondition for any pack work. |
| [compliance-policy-packs](./modules/compliance-policy-packs.aps.md) | CPACKS | Draft | Off-the-shelf packs; ships as ecosystem content after OPAE library + POLVAL. |
| [opa-agent-orchestration](./modules/opa-agent-orchestration.aps.md) | OPAG | Ready | Orchestration on a policy stack that does not exist yet. |
| [agent-governance-patterns](./modules/agent-governance-patterns.aps.md) | AGOV | Draft | Signal-producer for CPACKS / MDGOV. |
| [contextual-policy-assertions](./modules/contextual-policy-assertions.aps.md) | CPOL | Ready | Isolated, complements OPAE; small scope. |
| [io-risk-controls](./modules/io-risk-controls.aps.md) | IORISK | Ready | Closest to RTAI's I/O validation theme. |
| [adversarial-testing-catalog](./modules/adversarial-testing-catalog.aps.md) | ATC | Ready | Pair with PATT as v0.7 safety pack. |
| [prompt-attack-regression-packs](./modules/prompt-attack-regression-packs.aps.md) | PATT | Ready | Pair with ATC. |
| [eval-harness-integration](./modules/eval-harness-integration.aps.md) | EVAL | Ready | Adapter contract small; useful for RTAI regression once H3 ships. |

#### Coverage breadth (Language & Coverage)

The five-track plan. Excellent design work; entirely H4. Phased
rollout per `2026-04-08-language-and-coverage-design.md`.

| Module | ID | Status | Verdict rationale |
|--------|----|--------|-------------------|
| [lang-rust](./modules/lang-rust.aps.md) | RSTLAN | Draft | Anchor T3. Self-dogfood compelling, not launch-blocking. |
| [lang-python](./modules/lang-python.aps.md) | PYLAN | Draft | Anchor T3. |
| [lang-tail-wave](./modules/lang-tail-wave.aps.md) | LANGTAIL | Draft | Tail T1 batched sprint. |
| Surface modules (SURFSQL, SURFGHA, SURFDOCK, SURFSH) | various | Draft | Phase 1–3 surfaces. |
| Pack modules (PACKPUL, PACKLLM, PACKDRZ, PACKNXT, PACKHON, PACKTOK) | various | Draft | Phase 1–2 packs. **PACKLLM PII heuristics are a false-positive minefield** — keep out unless explicitly green-lit. |
| [markdown-governance](./modules/markdown-governance.aps.md) | MDGOV | Draft | Track 5. M1 wellformedness as internal compounding value. |

#### Long bets

| Module | ID | Status | Verdict rationale |
|--------|----|--------|-------------------|
| [weave](./modules/weave.aps.md) | WEAVE / AHARNESS | Draft | Greenfield import + harness build. Schedule after intercept-loop thesis is proven. |
| [graph-v2-foundation](./modules/graph-v2-foundation.aps.md) | GV2 | Draft | Joined semantic / dependency / trust / control / provenance graph. Anvil-first foundation. |
| [graph-context-delivery](./modules/graph-context-delivery.aps.md) | GCTX | Draft | Projection over GV2; assistant context delivery. |
| [intent-ledger-governance](./modules/intent-ledger-governance.aps.md) | ILGOV | Draft (rescope) | Becomes "predict effect of a change against captured intent" via the symbol/architecture graph. |
| [lineage-authorship-confidence](./modules/lineage-authorship-confidence.aps.md) | LAC | Ready | Line-level human / AI / mixed attribution. |

#### Other H4 modules

| Module | ID | Status | Verdict rationale |
|--------|----|--------|-------------------|
| [observability-foundation](./modules/observability-foundation.aps.md) | OBS | Draft | Park; rescope post-launch against `apps/anvil-api`. |
| [feature-flag-catalogue](./modules/feature-flag-catalogue.aps.md) | FLAGCAT | Draft | Manifest unification across surfaces. |
| [api-governance](./modules/api-governance.aps.md) | APGOV | Proposed | API contract governance. |
| [security](./modules/security.aps.md) | SEC | Proposed | Cargo audit / pnpm audit cadence. Hygiene. |
| [test-coverage-uplift](./modules/test-coverage-uplift.aps.md) | TCOV | In Progress | 14/25; Phase 4 needs scope refresh. Background hygiene. |
| [test-integration-surface](./modules/test-integration-surface.aps.md) | TINT | Draft | Promote Draft → Ready — TFIX/RCLI/KERN deps now archived-Complete. |
| [test-external-services](./modules/test-external-services.aps.md) | TEXT | Draft | External service contract tests. Not on launch critical path. |
| [early-access-tests](./modules/early-access-tests.aps.md) | EATEST | Ready | Rust-aligned. 38 items. |
| [early-access-migration](./modules/early-access-migration.aps.md) | EAMIG | Ready | Rust-aligned. 50 items. |
| [documentation-sync](./modules/documentation-sync.aps.md) | DOCSYNC | In Progress | Rolling. |
| [schema-contracts](./modules/schema-contracts.aps.md) | SCHEMA | Proposed | TS↔Rust contract parity; activate when the parity surface starts churning. |
| [config-intelligence](./modules/config-intelligence.aps.md) | CFGINT | Draft | Cross-language dependency graph. Feeds the architecture-edge detector. |
| [rust-cli-tier2](./modules/rust-cli-tier2.aps.md) | RCLI2 | Proposed | Tier 2 commands. Re-audit before commit. |
| [rust-cli-tier3](./modules/rust-cli-tier3.aps.md) | RCLI3 | Proposed | Tier 3 commands. Pure historical-contract work. |
| [skills-discovery-observability](./modules/skobs.aps.md) | SKOBS | Draft | Newly registered 2026-05-04. Promote when scope locks. |

### ❓ REVISIT — premise should be re-examined

Modules whose framing predates either the daemon + drivers
architecture (ADR-030), the multi-layer protection architecture
(ADRs 036–039 Proposed), or the RTV-is-the-product framing.

| Module | ID | Status | What to revisit |
|--------|----|--------|-----------------|
| [unified-config-format](./modules/unified-config-format.aps.md) | UCFG | Proposed | The driver-framework already implies `.anvil.yaml` as the config root; MLP-011 adds JSON/TOML support. Does UCFG still buy anything beyond what MLP-011 + INTD-008 already specify? Re-read against the multi-layer stack before scheduling. |
| [open-spec-adapter](./modules/open-spec-adapter.aps.md) | OPENSPEC | Draft | Anvil's own planning is APS, not open-spec. Was this targeting cross-tool plan ingestion? Close it or downgrade to "discovery-only". |
| [pocketflow-gateway](./modules/pocketflow-gateway.aps.md) | PFGW | Draft | Pocketflow integration predates the driver-framework. Does PFGW still have a distinct role, or is it absorbed by DRVR + MCP driver? |
| [council-gate-bridge](./modules/council-gate-bridge.aps.md) | CGBDG | Proposed | Bridges Claude-Code council reviews into Anvil attestations. Verify the bridge target hasn't shifted under the witness-chain work — MLP-002's witness lines may **be** the attestation format. |
| [intent-ledger-governance](./modules/intent-ledger-governance.aps.md) | ILGOV | Draft (rescope) | Per Big Bets, becomes "effect prediction" via the symbol/architecture graph. Confirm scope before any code lands. |
| [lineage-authorship-confidence](./modules/lineage-authorship-confidence.aps.md) | LAC | Ready | Does it plug into INTD-013's notification envelope cleanly, or want a separate emission path? Re-verify against the witness chain (MLP-002). |
| [bmad-v4-backward-compat](./modules/bmad-v4-backward-compat.aps.md) | BMAD4 | Proposed | Demand for v4 is unproven. Defer or archive based on a real user signal. |

### ✅ DONE — already complete or in-flight under another module

| Module | ID | Status | Notes |
|--------|----|--------|-------|
| (Most of the index's "Complete" entries) | — | — | See [index.aps.md](./index.aps.md) Release Plan tables and [completed-index.aps.md](./completed-index.aps.md). |

---

## Open decisions

The decisions on this list change downstream sequencing. Each needs a
human call.

### 1. Tag rename for `v0.7.0-beta`

The hype-builder shipped under `-beta`; the daemon-backed RTV cut
shipped under `-beta`. The next-window cut (daemon-working slate) is
the candidate that genuinely earns dropping `-beta` — it's the first
release where "Anvil protects this project" is testable as a closed
contract via MLP-009. Open: keep `-beta`, move to `-rc`, or graduate
out of `-beta`. Decide at H3 lock; not blocking implementation.

### 2. ADR promotion gate (Proposed → Accepted)

ADR-036 (rewritten), ADR-037, ADR-038, ADR-039 are all **Proposed**.
N3 G1 requires them Accepted before any MLP code merges. **Open:**
single council session for all four, or separate sessions per ADR?
Recommend single session — they form one architectural commit
(daemon-scope + witness chain + hooks + baseline). Decide before
Wave 0 starts.

### 3. RTAI-005/-007/-009 + DRVR-003 disposition

A2 Wave 4 was deferred per ADR-033. The mid-edit editor path,
telemetry mirror, and architecture supersession links sit waiting on
the next editor-surface decision. **Open:** wait for a new extension
package on the daemon-driver path (could come from RMCPF), or
re-pause indefinitely? No urgency; stays parked until either MLP-016
(L1 driver → Kindling) or a new editor surface forces the call.

### 4. RTAI's three open questions (still relevant for H3 telemetry)

- **4a. Mid-edit blocking semantics.** Does a mid-edit diagnostic
  ever escalate to `block` / `interrupt`, or is it always advisory?
  MCP pre-write *can* refuse a tool call. LSP `didChange` *cannot*
  prevent the editor from showing the user their own keystrokes.
  Asymmetric capability needs to be in the protocol from the start
  if it is going to land at all — bolting on later is the bad shape.
- **4b. Where do reasoning-pattern rules live?** `anvil-checks`
  antipattern crate, or a new `anvil-checks-reasoning` crate?
  Catalogue authors need a target.
- **4c. Mid-edit + suppression UX.** Save-time has a suppression
  model. Mid-edit diagnostics have no on-disk anchor yet. The
  protocol must not preclude it.

### 5. The `eddacraft/anvil-action` publishing repo

MLP-010 (GitHub Marketplace action) needs a separate publishing repo
at `github.com/eddacraft/anvil-action`. **Open:** create the skeleton
repo in Wave 0 of H3, or stand it up on first publish? Recommend
Wave 0 — gives MLP-010 somewhere to publish in Wave 3 without a
sequencing scramble.

---

## What recently changed

> Append-only. Newest entry first. A future session reads this to
> see what has moved since the last refresh.

### 2026-05-09 (daemon-working slate proposed; horizons rebased)

- **`v0.6.0-beta` substrate locked** on `dev`. A1 (Wow-Start
  Activation, LAUNCH 18/18) and A2 (Daemon-Backed RMCP + Driver
  Reach Waves 1–3, INTD 16/16, DRVR 5/5 active, RTAI 6/9) both
  fully shipped. Daemon-backed `tools/call` runs through live
  daemon when owner-only IPC is available; embedded path remains
  correctness-equivalent fallback. Tag candidate ready; release
  artefacts at `docs/runbooks/v0.6.0-beta-{security-note,release-runbook}.md`.
- **MLP architecture spec landed** (commit `c93d0b9b`, 2026-05-07):
  [`plans/specs/2026-05-07-anvil-multilayer-protection-architecture.md`](./specs/2026-05-07-anvil-multilayer-protection-architecture.md)
  — supersedes the earlier daemon-lifecycle spec; consolidates the
  full multi-layer protection architecture (witness chain, hooks,
  L4 policy, baseline, multi-agent coordination, rule
  distribution).
- **MLP module created** with 17 work items, status Proposed.
  Hard release gate **MLP-009** (protection-claim contract suite)
  pinned. New crates: `anvil-witness`, `anvil-hook`, `anvil-l4`,
  `anvil-config`, `anvil-baseline`, `anvil-attribution`.
- **Four ADRs filed Proposed:** ADR-036 (rewritten — daemon scope
  + discovery + OS-boundary), ADR-037 (witness chain + L4 policy),
  ADR-038 (hook surface + noise discipline / Serena rule), ADR-039
  (baseline policy + hard-pinned classes). All gate H3 entry.
- **RELEASE-PLAN.md updated** with the next-release window section
  defining N1 (MLP) + N2 (INTL) + N3 (carry-forward gates) + N4
  (documentation runbooks). Full wave plan with parallelisation
  map, dependency graph, anti-parallelism rules, hard sequencing.
- **B1 (Intercept Loop v0) absorbed** into N2 (INTL) + N3 G5
  (INTR-004). Back-pointer preserved.
- **Index updated** with next-window subsection and MLP module
  registry row in the Intercept Loop section.
- **Horizons rebased.** H1 (hype-builder) and H2 (daemon-backed
  RTV + driver reach) both closed-as-shipped. H3 becomes daemon-
  working end-to-end (the MLP + INTL slate). H4 absorbs former H3
  GA scope plus the now-pushed-back horizons.
- **Strategic frame this snapshot records:** the funding-phase
  releases shipped on schedule; the daemon path is real; the next
  release window converts the substrate into a defensible claim
  via the witness chain + hooks. MLP-009 is the gate that prevents
  the claim from drifting.

### 2026-05-07 (MLP architecture spec + ADRs Proposed)

- Multi-layer protection brainstorm → spec → 4 ADRs landed in a
  single planning push. Spec consolidates daemon-lifecycle (DLIFE)
  and extends it to the full L0–L5 model. DLIFE-008 promoted to
  MLP-014 (per-task fence isolation in v1).
- ADR-036 rewritten from "Proposed (single per-user daemon)" to
  "Proposed (per-execution-scope daemons by design, multi-daemon)
  with `info.json` runtime sidecar + hardened `os_locality_token`.
- ADR-037 / -038 / -039 added to the architecture decisions list
  in the index.

### 2026-05-03 (Wow-Start Activation council ratified)

- Five independent agent brainstorms (Claude / Codex / Copilot /
  Gemini / Opencode) converged on the same gap; planning council
  ratified `anvil start` as the activation entrypoint.
- LAUNCH module promoted from "in-flight" to "headline next-tag
  investment"; A1 cut sequenced as 6 PRs (later 7 with LAUNCH-009.6).
- Cursor + Claude Code MCP paths scoped as v1; Windsurf, VS Code,
  Copilot CLI, Codex CLI, process auto-attach explicitly deferred
  until RMCP / DRVR verifies them.

### 2026-05-01 (`v0.5.0-beta` shipped; TSRET-005 archive cascade)

- `v0.5.0-beta` tagged from `release/v0.5.0-beta`; release branch
  merged back to `dev` via PR #1215. Locked A1 + A2 + A3 + A4
  slate (44 items) shipped as a single cut. Validation backend
  recorded as embedded-fallback-backed, not yet daemon-backed.
- TSRET-005 engine archive cascade executed onto
  `chore/TSRET-005-v2`. TS scanner / suppression / drift / gate
  runner / explainer / parity harness moved under
  `archive/anvil-ts-scanner/`. Module reaches terminal state.
- Strategic frame: H1 hype-builder closed; H2 daemon-backed RTV +
  driver reach is the next-release window.

### 2026-04-29 (ADR-033 — archive IDE/MCP, retire TS scanner)

- ADR-033 authored. VSCode extension and TS MCP server archived
  under `archive/`; TS scanner / suppression parser / parity
  harness retire to `archive/anvil-ts-scanner/`. CI for archived
  packages switches off via existing `'!archive/**'` workspace
  exclusion.
- DRVR-003 (VSCode editor driver) deferred until a new extension
  package is created on the daemon-driver path. RMCPF starts from
  "TS MCP server is archived" rather than "active migration source".

### 2026-04-26 (`v0.4.0-beta` release prep)

- Three rounds of council ran against the release branch, plus an
  external Codex CLI review each round. ~25 findings surfaced;
  18 fixed in-flight. 10 hardening items consciously deferred to
  V050F.

### 2026-04-24 (RTAI module + cross-cutting convention)

- LAUNCH module created, trialing the cross-cutting module
  convention.
- RTAI module created (9 items) on the daemon + drivers
  architecture per ADR-030. Reuses cross-cutting convention from
  LAUNCH — second use is the trigger to consider promoting to a
  first-class APS primitive (see Open Decisions item rotated
  out of this snapshot — promotion deferred until MLP lands).
- ADR-030 sequencing decision (Option A) recorded.

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
  modules against the new H_n).

**How to re-run.** Read `plans/aps-rules.md`, `plans/index.aps.md`,
every active module file (skim for purpose + status), the critical-
path modules in full (MLP / INTL / INTD / DRVR / RMCP / RTAI), and
the load-bearing ADRs (currently ADR-030, ADR-033, ADR-036, ADR-037,
ADR-038, ADR-039). Apply the H1 / H2 / H3 / H4 lens; assign one of
🔥 SHIPPED / ➡️ NEXT / 🌱 LATER / ❓ REVISIT / ✅ DONE per module; surface
contradictions explicitly; do not paper over the open decisions.

The "What recently changed" log above is the only section that
should be appended to rather than rewritten — it is the audit trail
between snapshots.
