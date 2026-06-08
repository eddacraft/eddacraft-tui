<!-- APS: Completed Work Archive — read-only record of all shipped work -->
<!-- This document is non-executable. It archives completed tasks, milestones, and modules. -->

# Anvil — Completed Work

## Overview

Anvil makes AI-generated code safe to merge by catching architecture boundary
violations and AI escape-hatch anti-patterns at file-save time. Developers get
actionable warnings before code leaves the file, with human-owned exceptions for
intentional deviations.

**Why this matters:** AI coding tools are accelerating development, but they
don't understand your architecture. They produce code that compiles and passes
tests, yet drifts from intended patterns. By the time drift is noticed in
review, it's already merged or too expensive to fix. Anvil catches it at the
moment of creation — when fixing is cheap.

**Product thesis:** Anvil improves trust in AI-generated code so more of it
reaches production faster, while architecture drift slows or reverses over time.

**Primary beneficiary:** Individual developers — they get to use AI safely at
the pace leadership expects.

## Release Plan

### 0.1.0 — Beta (Complete)

**Philosophy:** A powerful engine is worthless if no one uses it. The initial
release must deliver both the core value AND a friction-free first experience.

#### Core Engine

| Feature             | Description                                    | Status   |
| ------------------- | ---------------------------------------------- | -------- |
| Analysis Engine     | `anvil check <files>` with caching + parallel  | Complete |
| Architecture Safety | Baseline inference, new-edge detection          | Complete |
| Anti-patterns       | 7 high-confidence patterns                     | Complete |
| Suppressions        | Time-boxed with mandatory explanations          | Complete |
| Git Integration     | `--changed`, `--staged`, `--since <ref>`       | Complete |
| Watch Mode          | `anvil watch --source` for real-time feedback   | Complete |
| CI/CD               | GitHub Action with PR comments + status checks  | Complete |

#### Onboarding Experience

| Feature           | Description                                     | Status   |
| ----------------- | ----------------------------------------------- | -------- |
| TUI Foundation    | Ink setup + base components (TUI-001)           | Complete |
| Init Wizard       | Visual `anvil init` with guided flow (TUI-002)  | Complete |
| Status Dashboard  | Quick health check: `anvil status` (TUI-003)    | Complete |
| Doctor Command    | Diagnose setup issues: `anvil doctor` (TUI-004) | Complete |
| First-run Welcome | Show value immediately on first run (TUI-005)   | Complete |

#### Documentation & Polish

| Feature           | Description                     | Status   |
| ----------------- | ------------------------------- | -------- |
| Quick Start Guide | 5-minute path to first value    | Complete |
| User Guide        | Complete command reference       | Complete |
| Demo/Tutorial     | Show Anvil catching real issues  | Complete |
| Error Messages    | Actionable, not cryptic          | Complete |

#### Drift Visibility & Developer Trust

| Feature                | Description                                    | Status   |
| ---------------------- | ---------------------------------------------- | -------- |
| Explain Command        | `anvil explain <id>` — deep-dive into warnings | Complete |
| Drift Snapshots        | `anvil drift snapshot` — capture current state  | Complete |
| Drift Compare          | `anvil drift compare` — show changes over time  | Complete |
| Drift Reports          | `anvil drift report` — visualise trends         | Complete |
| OPA Architecture       | DC-OPA bridge, YAML-first architecture          | Complete |
| Architecture Templates | Layered, Hexagonal, Clean, DDD presets          | Complete |
| Remote Policy Bundles  | Centralised policy distribution                 | Complete |
| Monorepo Migration     | Restructure to apps/packages layered layout     | Complete |

#### AI Tool Integration

| Feature         | Description                                | Status   |
| --------------- | ------------------------------------------ | -------- |
| llms.txt Export | Export constraints for AI tool consumption | Complete |
| Command Safety  | Validate AI tool commands (CMDSAF)         | Complete |
| MCP Server      | Real-time validation during AI generation  | Complete |

#### HTML/CSS, Tutorial & First Run

| Feature                   | Description                                         | Status   |
| ------------------------- | --------------------------------------------------- | -------- |
| Configurable Extensions   | Make analysable file extensions configurable         | Complete |
| HTML Anti-patterns        | Inline styles, scripts, event handlers, deprecated  | Complete |
| CSS Anti-patterns         | `!important` abuse, CSS `@import` performance       | Complete |
| Tutorial Overhaul         | Scan-watch-fix flow, feature tutorials, docs        | Complete |
| Intelligent First Run     | Post-init analysis, smart defaults, quick wins      | Complete |

### 0.1.x — Completed Work

| Feature                    | Description                                              | Status   | Progress |
| -------------------------- | -------------------------------------------------------- | -------- | -------- |
| Forge Hook & Agent         | Pre-commit hook + reviewer agent with codex delegation   | Complete | —        |
| Forge Negotiation          | Structured finding/response protocol, round cap          | Complete | —        |
| Deferred Finding Filing    | Auto-file deferred findings as GH issues or APS items    | Complete | —        |
| Temper Workflow             | GitHub Actions self-healing loop with 2-cycle cap        | Complete | —        |
| Configuration & Docs       | Env vars, settings.json, CLAUDE.md, toggle matrix        | Complete | —        |
| CLI Hardening              | Error handling, edge cases, robustness                   | Complete | —        |
| Coaching Nudges            | Context-aware suggestions for pattern improvement        | Complete | —        |
| Nx Task Migration          | Migrate root scripts to Nx-orchestrated per-project      | Complete | 6/6      |
| CLI esbuild Bundling       | Self-contained npm package via esbuild                   | Complete | 3/3      |
| MCP Server Hardening       | Production-readiness for MCP server                      | Complete | —        |
| Security CI Pipeline       | Automated security scanning on every PR                  | Complete | —        |
| Tutorial Path Continuation | Continue with another tutorial from completion screen     | Complete | —        |
| Post-Beta Launch Uplift    | Address 57 findings from v0.1.2-beta post-release review | Complete | 57/57    |
| Code Review Backlog        | 29 architectural recommendations from code review        | Complete | 29/29    |
| Security Review Backlog    | Cross-package security findings from adversarial review  | Complete | 8/8      |

**Design doc (Forge & Temper — archived):** [docs/archive/2026-02-24-forge-temper-review-pipeline.md](../docs/archive/2026-02-24-forge-temper-review-pipeline.md)

### 0.4.0 — Edda Stack (Memory System)

| Feature                | Description                                    | Status   |
| ---------------------- | ---------------------------------------------- | -------- |
| Kindling Integration   | Observation layer — session and gate hooks      | Complete |
| Ember                  | Interpretive layer — candidate memory proposals | Complete |
| Edda                   | Canonical memory — git-backed, provenance-tracked | Complete |
| Edda Stack Integration | Shared schemas, event bus, layer ports          | Complete |
| Edda-Ember Review      | Non-critical improvements from consolidated review | Complete |

### 0.5.0-beta – 0.7.4-beta — Beta Release Windows (Shipped)

Migrated from [`index.aps.md`](./index.aps.md) on 2026-06-08 to keep the active
index focused on current work. These windows are fully shipped; module tables
below read "Complete / Locked". For active release sequencing see
[`RELEASE-PLAN.md`](../RELEASE-PLAN.md); for thematic context see
[`ROADMAP.md`](../ROADMAP.md). The headings below were `###` in the source
index and are demoted one level here to sit under this window banner.

#### Shipped operational window — `v0.6.2-beta` / `v0.6.3-beta` patch

The OPMODEL-012 main-first cutover landed on 2026-05-11, RELORCH completed the
deterministic release command surface, and CICD closed the targeting/drift
readiness work on 2026-05-12. The operational release `v0.6.2-beta` is tagged;
the `v0.6.3-beta` patch (2026-05-15, release record at
[`plans/releases/v0.6.3-beta.md`](./releases/v0.6.3-beta.md)) rolled WATCHUX
8/8 and ADOPT-005 `anvil uninstall` on top. The daemon-working product slate
has since shipped as `v0.7.0-beta` (2026-05-21) plus the `v0.7.1-beta`
(2026-05-23), `v0.7.2-beta` (2026-05-25), `v0.7.3-beta` (2026-05-31), and
`v0.7.4-beta` (2026-06-01) patches; the current planning window is now the
`v0.8.0-beta` candidate.

| Area | Status | Progress | Notes |
| ---- | ------ | -------- | ----- |
| Shipped baseline | Shipped | `v0.6.3-beta` tag (2026-05-15, hotfix on top of `v0.6.2-beta`) | Wow-start activation, daemon-backed validation, the executable release operating model, and the beta watch UX / uninstall hotfix are behind us; current work should not reopen operational substrate scope. v0.6.3-beta released WATCHUX-001..-004 + ADOPT-005 (record at [`plans/releases/v0.6.3-beta.md`](./releases/v0.6.3-beta.md)). |
| Main-first cutover | Complete | OPMODEL 12/12 — archived 2026-05-11 | Cutover SHA `b6f236e9`; `main` ruleset id 16217152 enforces 7 required checks + PR + non-FF + no-delete; `dev` retired as `dev-retired-2026-05-11` tag (deletion follow-up #1419). Module archived. |
| CI/CD release readiness | Shipped | OPMODEL-005 spec + CICD-009 implementation complete | `.github/workflows/release-readiness.yml` validates an exact `main` SHA with no publishing credentials; candidate metadata + retention live. CICD-012 added cutover-aware gates and self-defending fork-reject. |
| Release orchestration | Complete | RELORCH 12/12 | Completed command-surface slice after OPMODEL-012 unblocked main-targeted work: assess, preflight, prepare, promote, tag, monitor, verify, closeout, command harness, release-record yank/discard schema closure, and skill/runbook wire-up with legacy runner removal. Live CI readiness authority remains tracked under CICD. |
| CI targeting + drift | Complete | CICD 12/12 (closed 2026-05-12) | All twelve items shipped: cost reporting (-001), classifier (-002), local validation (-003), fast PR validation (-004), integration SHA split (-005), coverage cost controls (-006), security/dependency targeting (-007), platform-matrix targeting (-008), release-readiness reconciliation (-009), workflow contract map + authority audit (-010), APS/repo/release drift checks in CI with PR-metadata extension (-011), and cutover readiness (-012). Council follow-ups closed via PR #1442 (issue #1438). |
| Daemon-working product slate | Shipped | MLP 18/18 Complete (Done 2026-05-13/-14); MLP2 71/87 (In Progress — **MLP2-047 Merged 2026-05-25 via PR [#1941](https://github.com/eddacraft/anvil-001/pull/1941) — two Linux-gated subprocess smoke tests for `anvil hook pre-push` (no-policy + version-floor branches); proves exit-code/stderr/witness contract end-to-end. Done-count advances 68 → 69.** **MLP2-051g Merged 2026-05-25 via PR [#1909](https://github.com/eddacraft/anvil-001/pull/1909) (`03e6a73f`) — `anvil start --verify --why` + `anvil status --verify --why` print per-tier activation evidence to stderr, closing acceptance criterion #3 of GH [#1831](https://github.com/eddacraft/anvil-001/issues/1831). Copilot-review hardening shipped in the same merge: clap `requires = "verify"` on `StatusArgs::why` (no silent no-op), drop the nonexistent `anvil intercept recover` from copy, ensure all `anvil intercept start` hints include `--foreground`, dispatch `why_summary` on `protection_state()` instead of solely `daemon_attestation`. 10 pinned tests in `crates/anvil-cli/src/activation/render.rs::tests`. Done-count advances 67 → 68.** **MLP2-070 reconciled to `Released/Shipped via v0.7.0-beta` 2026-05-24 — daemon IPC handler re-derives the lineage anchor from authenticated peer credentials (`SO_PEERCRED` + `/proc/<peer_pid>/stat` on Linux; client-supplied `pid_starttime` forwarded as advisory on non-Linux). Closes DeepSec [#1674](https://github.com/eddacraft/anvil-001/issues/1674); shipped via PR [#1805](https://github.com/eddacraft/anvil-001/pull/1805) merged 2026-05-21 at `c8193511` (+ non-Linux follow-up at `fefb6e8c`). APS status was previously stuck at `In Progress` despite both commits being in `v0.7.0-beta` and `v0.7.1-beta` tags. Group P advances 0/2 → 1/2 (Phase 1 of MLP2-071 already Merged; Phase 2 still pending). Done-count advances 66 → 67.** **MLP2-074 Merged 2026-05-24 via PR #1895 (`5bb10f3a`) — daemon-side `session.report_process` IPC handler narrows the lineage anchor from the launcher to the spawned child; PR-review hardening added cross-session anchor-collision rejection + Linux server-side `pid_starttime` re-derivation mirroring MLP2-070's trust-boundary defence; Group R closes 1/1.** **MLP2-025 umbrella closed 2026-05-18: Phase 1 primitives merged via PR #1597 (2026-05-15); Phase 2 (-025b PR #1603) + Phase 3 (-025c PR #1608) shipped 2026-05-16. End-to-end spoof cross-check live in production; counter sweep +1 (58 → 59).** **MLP2-051 re-specced 2026-05-17 — split into umbrella + 051a..051e sub-tasks after audit showed only `anvil status` renders the typed claim today; doctor/MCP/TS-driver/GH-Action surfaces emit no claim yet, so the work is additive rather than migrative; net +5 task IDs.** **MLP2-068..-069 filed 2026-05-17 (MLP2-068 Merged; MLP2-069 Done 2026-05-22) as Group O — MLP2-016 audit follow-ons (Council-deferred kernel/ops items): `git cat-file --batch` for per-commit blob fetch perf + dedicated `EngineUnavailableReason::IoError` variant; neither gates `v0.7.0-beta`. Companion infra item filed as GitHub issue #1630 (ship `patterns/compiled/registry.json` with installed binary).** **MLP2-067 filed 2026-05-16 (Draft) as Group N — daemon-hosted graph cache with narrow evaluator RPC, the middle-ground bridge to GV2; does not gate v0.7.0-beta.** **Group K closed 4/4 via PR `d96ab458` (MLP2-053..-056 audit-chain workflow + Kindling emission + rule rescan + time-budget cap); Group L closed 4/4 via PR `7a39e5f9` (MLP2-059 per-worktree invalidation rate limit); Group M closed 6/6 on 2026-05-16 via PRs #1602 (MLP2-061..-063) + #1604 (MLP2-064..-066)**; wave 1A (PR #1522): MLP2-001 + MLP2-002; wave 1B: MLP2-023 (composite session key); MLP2-003 (composite-identity check primitive); MLP2-024 + MLP2-009 (per-worktree session cap + rate_window primitive); MLP2-029 (TS `AgentTag` mirror); MLP2-030 + MLP2-060 shipped 2026-05-14 together — TS mid-edit Kindling observation mirror (closes Group F 2/2) + YAML resource-bounds hardening (alias-reject + size-cap + depth-cap, closes Council #C-023b); wave 1C shipped 2026-05-14 on branch `feat/mlp2-wave-016-048-057-052` — MLP2-052 (additive-optional-fields forward-compat pin), MLP2-057 (bounded LRU rule_cache + SessionRegistry unregister hook, closes Council #C-007/-018/-024), MLP2-048 (`anvil status --json` emits nested ProtectionClaim, closes MLP-009 HARD-GATE render surface), MLP2-016 (typed `validate_at_l4` engine + on_warn-aware pre-push pipeline, closes Council #C-016A); wave 1D shipped 2026-05-14 via PR #1563 at `fc19b58b` — MLP2-058 (tracing + DaemonStatus observability on rule_cache + in_flight, closes Council #C-008/-009/-012/-013/-014/-015/-025), MLP2-012 (witness manifest event stream at `anvil/witness/manifest/chain.ndjson` for rollover consumers), MLP2-046 (dedicated `anvil l4-validate` CLI subcommand), MLP2-049 (per-state golden ProtectionClaim fixtures at `crates/anvil-cli/tests/fixtures/status_v1/`); wave 1E shipped 2026-05-14 via PR #1566 at `9ec726dd` — MLP2-020 (hook-side `required_anvil_version` floor check with split routing: `BelowFloor` → `ErrorClass::VersionFloor` "upgrade anvil", `InvalidFloor` → `ErrorClass::EmbeddedFailed`), MLP2-021 (`cutoff_commit` baseline-ancestry acceptance via `git rev-list --first-parent --max-count=100000` per ref + hex-shape validation on `Policy::cutoff_commit` + O(1) per-commit lookup via hoisted `HashMap<sha, index>`), MLP2-022 (`PRE_PUSH_BUDGET = 2s` wall-clock cap with `ErrorClass::TimedOut` distinct render + `tracing::warn!` partial-state event; `ValidationPending` suppressed when budget fires); wave 1F shipped 2026-05-14 via PR #1567 at `96ad5d2d` — MLP2-018 (daemon-side `evaluate_version_floor(policy_floor, witness_anvil_version)` returning typed `VersionFloorOutcome` server-side mirror of MLP2-020; uses `semver::Version` directly), MLP2-019 (`crates/anvil-l4/src/recognised_rules.rs` new module — `RecognisedRulesRegistry` HashMap O(1) lookup + `RuleSetMetadata` + `evaluate_rules_sha` routing through `OnNoWitness` vocabulary, closes the v1 L4 recognition surface), MLP2-031 (`pin_cutoff_commit(path, cutoff)` in `crates/anvil-l4/src/policy.rs` — atomic temp+rename writer with symlink refusal + hex-shape pre-flight + multi-format round-trip across yaml/yml/json/toml + non-map-baseline refusal, producer side of MLP2-021); +24 new unit pins (82 anvil-l4 tests total, was 67 baseline); Council quick reviewed (3 MAJOR fixed: NotAnObject ambiguity → BaselineNotAMap split, atomic_replace Windows comment, dead_code cleanup); +20 new unit pins across anvil-cli/anvil-hook/anvil-l4; integration follow-ups split out from MLP-018 catalogue + 4 Council-filed hardening tasks in Group L); INTL 9/9 Done (Merged via PR #1528 at `5d38e546`, awaiting `v0.7.0-beta` release evidence to advance to Released/Shipped → Complete); carry-forward gates 6/6 confirmed (Wave 0 closed 2026-05-13) | Shipped via `v0.7.0-beta` (2026-05-21) + `v0.7.1-beta` (2026-05-23) + `v0.7.2-beta` (2026-05-25) + `v0.7.3-beta` (2026-05-31) + `v0.7.4-beta` (2026-06-01) patches; next candidate `v0.8.0-beta`. MLP v1 surface area shipped + INTL launcher ingress shipped. Integration debt tracked module-locally in MLP2 — each of the 60 sub-items (Groups A–K from the MLP-018 catalogue + Group L production hardening from Council session `council-e2fdfc0c`) is now a first-class APS task. |

#### Shipped — _daemon-working slate_ (`v0.7.0-beta` + `v0.7.1-beta` / `v0.7.2-beta` patches)

OPMODEL, RELORCH, and CICD are closed. This slate **shipped** as `v0.7.0-beta`
(2026-05-21), with the `v0.7.1-beta` (Activation Diagnostic Honesty, 2026-05-23)
and `v0.7.2-beta` (Save-Time Scanning & Tooling Honesty, 2026-05-25) Boring Week
patches on top. **Theme:** _Daemon working end-to-end_ — `anvil start` lands a
real testable protection claim, hooks fire deterministically, the witness chain
records every commit, baseline adoption works, and `anvil-run` wraps agent
processes. `v0.7.3-beta` ("Surfacing the Signal", 2026-05-31) and
`v0.7.4-beta` ("Side-by-Side Installs", 2026-06-01) have since shipped; the
active planning window is now the **`v0.8.0-beta`** candidate (save-time
daemon arc, ADR-061; scope assembling on `main`).

Source of truth for current parallelisation and release dependencies:
[`RELEASE-PLAN.md`](../RELEASE-PLAN.md).

**MLP2 audit note (final sweep 2026-05-19):** the module stood at 60/76 as of
that sweep. Subsequent additions land in the N1b row below — see that row for
current counts (Group P added 2026-05-20 took the total to 78; Group Q added
2026-05-21 took it to 80).
MLP2-068 was reconciled to Merged after implementation commit `d54a5f86`,
advancing Group O to 1/2; MLP2-069 was promoted and completed on 2026-05-22
as a small observability-hygiene follow-up ahead of the next hotfix, advancing
MLP2 to 65/83 **(historical snapshot as of 2026-05-22; the current
total advanced to /86 when MLP2-051g/i/j were filed Draft later the
same day, and the done-count has since advanced past 65 — see the
N1b row + module-table row below for live counts)**. It does not
gate `v0.7.0-beta`. Earlier 2026-05-17/18 reconciliation closed MLP2-025 umbrella,
split MLP2-051 into umbrella + 051a..051e, filed Group O (MLP2-068..069), and
closed Group K, Group L, and Group M. MLP2-016 and MLP2-048 are re-closed after
the full MLP/MLP2 Council audit reopened them.
Earlier wave prose that says MLP2-016 / MLP2-048 closed hard
gates is historical PR evidence; the current release cut-line lives in the
MLP2 module and `RELEASE-PLAN.md`.

| Pick | Status | Progress | Notes |
| ---- | ------ | -------- | ----- |
| N1 — Multi-Layer Protection v1 (MLP) | Complete | 18/18 | Witness chain + hooks + L4 policy + baseline + multi-agent coord + rule distribution. Crates: `anvil-witness`, `anvil-config`, `anvil-rules`, `anvil-baseline`, `anvil-hook`, `anvil-l4`, `anvil-attribution`, `anvil-kernel-types::protection_claim`, plus `anvil-intercept::kindling_observation` module. **Hard gate: MLP-009 — Done 2026-05-13.** Promoted from Proposed during Wave 0 readiness review (2026-05-13). Wave 1 / Wave 2 / Wave 3 all shipped 2026-05-13. MLP-018 (v1-deferrals catalogue) closed 2026-05-14 — the 56 sub-items split out into the new MLP2 module ([`plans/modules/multilayer-protection-v2.aps.md`](./modules/multilayer-protection-v2.aps.md)) so each integration item is plannable in isolation. |
| N1b — Multi-Layer Protection v2 (MLP2) | In Progress | 71/87 | **MLP2-051g Merged 2026-05-25 via PR [#1909](https://github.com/eddacraft/anvil-001/pull/1909) at `03e6a73f` — `--verify --why` ships per-tier activation evidence to stderr, closes acceptance criterion #3 of GH [#1831](https://github.com/eddacraft/anvil-001/issues/1831). Copilot-review fixes in the same merge: clap-enforced `--verify` requirement on `StatusArgs::why`, drop nonexistent `anvil intercept recover`, ensure all `anvil intercept start` hints include `--foreground`, dispatch `why_summary` on `protection_state()` not solely `daemon_attestation`. Done-count advances 67 → 68.** **MLP2-070 reconciled to `Released/Shipped via v0.7.0-beta` 2026-05-24 — daemon IPC handler re-derives the lineage anchor from authenticated peer credentials, closing DeepSec [#1674](https://github.com/eddacraft/anvil-001/issues/1674); shipped via PR [#1805](https://github.com/eddacraft/anvil-001/pull/1805) at `c8193511` (+ non-Linux advisory follow-up `fefb6e8c`). APS status was stuck at `In Progress` even though both commits are in `v0.7.0-beta` + `v0.7.1-beta`. Group P advances 0/2 → 1/2. Done-count advances 66 → 67.** **MLP2-074 Merged 2026-05-24 via PR [#1895](https://github.com/eddacraft/anvil-001/pull/1895) (rebase-merge at `5bb10f3a`) — daemon-side `session.report_process` IPC handler narrows the registry's lineage anchor from the launcher's `(pid, pid_starttime)` to the spawned agent child's, closing v0.7.0-beta pre-tag release council action A2 (`council-a1e2648f`). PR-review hardening (`5bb10f3a`) adds cross-session anchor-collision rejection (typed `RegistryError::LineageAnchorCollision`) + Linux server-side `pid_starttime` re-derivation from `/proc/<child_pid>/stat` mirroring MLP2-070's `verify_lineage_claim` trust-boundary defence. Group R closes 1/1. Done-count advances 65 → 66.** **`v0.7.1-beta` released 2026-05-23T04:00:06Z — post-publish cleanup-agent sweep advanced MLP2-051f, MLP2-051h, and MLP2-069 to `Released/Shipped via v0.7.1-beta`. Tag SHA `c3e55d6068134dadbfebacb90d3fd215a412ac4f`; release record `plans/releases/v0.7.1-beta.md` advanced `candidate` → `published`; tracking issue [#1867](https://github.com/eddacraft/anvil-001/issues/1867). Done-count unchanged at 65.** **MLP2-069 Done 2026-05-22 — `EngineUnavailableReason::IoError` now distinguishes git/tempdir/materialisation I/O outages from missing tooling; Done-count advances 64 → 65.** **Group J extended 2026-05-22 — MLP2-051g, -051i, -051j filed `Draft` as MLP2-051f hardening follow-ups. MLP2-051g adds the `anvil start --verify --why` verbose tier-evidence flag (closes acceptance criterion #3 of GH [#1831](https://github.com/eddacraft/anvil-001/issues/1831)); MLP2-051i tightens the MCP `query_protection_claim` IPC timeout to match the 500 ms activation budget; MLP2-051j adds client-side peer-owner SID validation on Windows named-pipe connections, mirroring the Unix `SO_PEERCRED` UID check. Filed against the full Council review on the MLP2-051f/g/h + MLP2-075 work-set (2026-05-22); none of the three blocks `v0.7.0-beta`. Total advances 83 → 86; done-count unchanged at 64.** **MLP2-051h advanced `In Progress` → `Merged` 2026-05-22 via PR [#1837](https://github.com/eddacraft/anvil-001/pull/1837) rebase-merged at `4ec9c5a4` — `DaemonStatusV1::generated_at_unix` wire-add shipped with parity tests, live-stamp test, and sentinel-equality test pinning the `== 0` no-anchor contract that MLP2-051f (PR [#1840](https://github.com/eddacraft/anvil-001/pull/1840)) consumes. Subsequently shipped via `v0.7.1-beta` (published 2026-05-23T04:00:06Z) — see the lead paragraph above for the cleanup-agent sweep. Done-count advances 63 → 64.** **MLP2-051f Merged 2026-05-22 via PR [#1840](https://github.com/eddacraft/anvil-001/pull/1840) at `e1cc066a` — activation diagnostic now consumes the daemon `ProtectionClaim` snapshot through `promote_to_live_validation_when_daemon_attests`; `anvil start --verify` and `anvil status --verify` reach `protecting` when the intercept daemon attests the canonical worktree. Closes GH [#1831](https://github.com/eddacraft/anvil-001/issues/1831). Done-count advances 62 → 63.** **Earlier 2026-05-22: MLP2-051f filed under Group J (denominator advanced 82 → 83). Hard-gate precursors merged: MLP2-075 (Windows IPC parity, PR #1836) + MLP2-051h (`generated_at_unix` wire-add, `main` at `4ec9c5a4`). Implements `plans/specs/2026-05-21-activation-daemon-evidence-wireup.md` per council session `plan-f4668683` (4 COUNTER / 1 CONSENSUS).** **MLP2-051h filed 2026-05-22 — `DaemonStatusV1::generated_at_unix` wire-add precursor to the MLP2-051f activation diagnostic. Additive `u64` field with `#[serde(default)]`, parity test pins pre-MLP2-051h shapes deserialise to `0`. Filed ahead of MLP2-051f per the activation-daemon-evidence wire-up spec §"APS placement" so the field exists on the wire before the first consumer arrives; does not block `v0.7.0-beta`. Total advances 81 → 82.** **Group R (MLP2-074) added 2026-05-21 — v0.7.0-beta pre-tag release council action A2; daemon-side `session.report_process` IPC handler unimplemented (launcher absorbs gracefully, ships as Known Gap). Tracked at GH [#1827](https://github.com/eddacraft/anvil-001/issues/1827); does not block `v0.7.0-beta`.** **Group Q closed 2/2 on 2026-05-21 — MLP2-072 PR #1819 `18c899bb`; MLP2-073 PR #1821 `15a397bd`. Group Q (MLP2-072..-073) added 2026-05-21 — new-user journey audit follow-ups: MCP `anvil_validate_write` auth-gate shape (MLP2-072, GH #1796) + pre-write `summary.total` dedupe (MLP2-073, GH #1799). Filed from `plans/audits/2026-05-21-new-user-journey-audit.md`; neither blocks `v0.7.0-beta`. Total advanced 78 → 80 when Group Q was filed; closure on 2026-05-21 advanced done-count 60 → 62.** **MLP2-071 advanced `Blocked` → `Ready` on 2026-05-21 — cross-session-attribution design pass artefact landed at `plans/specs/2026-05-21-intd-015-cross-session-attribution-design-pass.md`; module entry now carries the implementation slice contract + validation matrix. Done-count unchanged (`Ready` is a planning status, not a done-count advance).** **Group P (MLP2-070..-071) added 2026-05-20 — v0.7.0-beta release-council follow-ups: lineage anchor daemon-derivation hardening (MLP2-070, In Progress, ship-with-doc verdict on #1674) + INTD-015 cross-session policy follow-up (MLP2-071, Ready after 2026-05-21 design pass, defer-with-issue verdict on #1722). Filed in MLP2 rather than `intercept-daemon.aps.md` because INTD is archived at 16/16 Complete; MLP2 is the active home for daemon integration debt. Neither blocks `v0.7.0-beta`; both have operator-facing release-note coverage in `docs/runbooks/v0.7.0-beta-security-note.md` §M1 and CHANGELOG "Known gaps".** **MLP2-051 re-specced 2026-05-17 — split into umbrella + 051a..051e sub-tasks after audit showed only `anvil status` renders the typed claim today; net +5 IDs, done unchanged.** **Group O (MLP2-068..-069) added 2026-05-17 — MLP2-016 audit follow-ons: `git cat-file --batch` perf + `EngineUnavailableReason::IoError` variant (MLP2-068 Merged; MLP2-069 Draft; companion infra item is GH issue #1630).** **Group N (MLP2-067) added 2026-05-16 — daemon-hosted graph cache + narrow evaluator RPC, GV2 groundwork (Draft).** Follow-up module collecting the 56 integration items from MLP-018's catalogue, 4 Council-filed production-hardening items (Group L), and 6 full-audit corrective items (Group M). **Group K (MLP2-053..-056) and Group L (MLP2-057..-060) both closed 4/4 after the post-merge reconciliation sweep on 2026-05-16 advanced MLP2-053..-056 and MLP2-059 from `In Progress` to `Merged`.** **MLP2-025 split into MLP2-025 + MLP2-025b + MLP2-025c during the daemon-control wiring; Group D and the Stats footer now count each sub-task separately (total 68 vs the original 66).** **Group M closed 6/6 on 2026-05-16 via PRs #1602 (MLP2-061..-063 rollover/L4/policy hardening) + #1604 (MLP2-064..-066 rule-cache + baseline drift + ADR-046 YAML deferral).** **Wave 1I shipped 2026-05-15 via PR #1589 at `2ba61ca1`:** MLP2-011 (DAG-aware `verify_chain_dag` walking the merge-join graph; legacy `verify_chain` becomes a `#[deprecated]` linear-only wrapper, four production call sites migrated to the DAG verifier), MLP2-013 (`anvil-baseline::save_with_genesis` emits `GENESIS-BASELINED` or `GENESIS-FRESH` as the chain's first witness line; schema change adds `cutoff_commit: Option<String>` to `WitnessLine`), MLP2-014 (pre-commit hook reads `.anvil.<ext>` via `anvil_config::discover`, computes `rules_sha` via `anvil_rules::rules_sha`, and threads the digest onto every witness line — empty rule-id list for now until the future rule-engine wiring lands), MLP2-015 (80-writer stress test promoted out of `#[ignore]` after 10/10 ~10ms flake budget). Council session `council-8c8842cf` quick-converged with 1 MAJOR + 2 NIT + 1 MINOR fixed pre-push (`save_with_genesis` migrated to `verify_chain_dag`, all-None merge contract pin test, `#[deprecated]` on the linear wrapper, io.rs idempotency-comment refresh). **Group B closed 5/5** (MLP2-012 already shipped via wave 1D). **Wave 1G shipped 2026-05-15 via PR #1576 at `33659b6c`:** MLP2-037 (`anvil hook bootstrap --witness-recent` walks `git rev-list --reverse @{u}..HEAD` and writes retroactive lines with `validation_at: "bootstrap-recovery"`), MLP2-038 (end-to-end union-merge proof: real `git init` + `git merge` integration test on the orchestrator's `.gitattributes` writer), MLP2-039 (`anvil start --format yaml|yml|json|toml` pre-writes `.anvil.<ext>` with the embedded `format` field matching the chosen extension; `activation::diagnostic::probe_config_status` now recognises `.anvil.<ext>` via MLP-011's `discover`), MLP2-040 (`gate.rs::read_anvilrc_checks` prefers `.anvil.<ext>` via `anvil_config::discover`, new `anvil migrate` command for the legacy `.anvilrc` → `.anvil.<ext>` bridge), MLP2-041 (typed `GateConfigView` / `InitConfigView` / `PolicyConfigView` foundation with `from_value(&serde_json::Value)` boundary-validated constructors; `#![allow(dead_code)]` until consumers adopt incrementally per spec). **Group H closed 5/5.** Council session `council-e8633cef` quick-converged with 2 MAJOR + 2 MINOR fixed pre-push (`default_anvil_config_value` format-derivation fix; `--reverse` added to `list_unwitnessed_range` so the recovery walk writes oldest-first; watch-skip copy now names both `.anvilrc` and `.anvil.<ext>` adoption paths). **Wave 1D shipped 2026-05-14 via PR #1563 at `fc19b58b`:** MLP2-058 (`tracing::` instrumentation + `DaemonStatus` surface for `rule_cache` + `in_flight` counter, closes Council #C-008/-009/-012/-013/-014/-015/-025), MLP2-012 (manifest event stream at `anvil/witness/manifest/chain.ndjson` for rollover events from `WitnessWriter::append`), MLP2-046 (dedicated `anvil l4-validate` CLI subcommand replacing `anvil hook pre-push` reuse), MLP2-049 (per-state golden JSON fixtures at `crates/anvil-cli/tests/fixtures/status_v1/`). Closes the gap between every v1 primitive and the full surfaces it targets. 12 groups: A–K cover the MLP-018 catalogue (daemon enforcement integration, witness-chain extensions, L4 policy execution, multi-session + fence isolation, cross-platform attribution, TS driver-client mirrors, baseline + identity wiring, hook + config completion, GH Action publishing, protection-claim render conformance, Kindling activation orchestrator). Group L (MLP2-057..-060, added 2026-05-14) covers production hardening on MLP2's own surface flagged during the PR #1522 Council review. Every task carries an explicit `Source:` line — Groups A–K cite their originating MLP task / footnote / PR; Group L cites Council session `council-e2fdfc0c` finding IDs. **Wave 1C shipped 2026-05-14 on branch `feat/mlp2-wave-016-048-057-052`:** MLP2-052 (additive-optional-fields forward-compat pin, 5 new unit + 3 new contract tests), MLP2-057 (bounded LRU rule_cache + unregister hook on SessionRegistry, +14 anvil-intercept tests; Group L Council #C-007/-018/-024 closed), MLP2-048 (`anvil status --json` emits nested `ProtectionClaim`, new `build_protection_claim` daemon-side helper, HARD-GATE rendering surface closed; schema file extended; +8 tests), MLP2-016 (typed `ValidationEngine` trait + `validate_at_l4` pipeline in `anvil-l4`, pre-push hook swaps inline `InternalError { TimedOut }` for trait dispatch with on_warn-aware verdict routing; +11 tests). Council #C-016A `on_warn` consultation fix folded into MLP2-016 in the same wave. |
| N2 — Intercept Launcher v1 (INTL) | Complete | 9/9 | `anvil-run` wrapped-launch ingress. Crate `crates/anvil-run/` shipped via PR #1528 (merged 2026-05-14 at `5d38e546`) with INTL-001..-009 covered by 49 unit + 3 shell-integration tests. Schema status moved **In Progress → Done → Released/Shipped → Complete**: all nine items shipped in `v0.7.0-beta` (2026-05-21), so the module is now **Complete** and archived to `plans/archive/modules/`. Two QoL follow-ups deferred to #1529 (foreground TTY passing + blocked-launch shell quoting). |
| N3 — Carry-forward gates | 6/6 confirmed | 6/6 | G1 ADR-036/-037/-038/-039 **Accepted** (2026-05-13), `DECISION-LOG.md` updated, `pnpm adr:check` green; G2 `anvil/project-id` schema reaffirmed (MLP-001 + ADR-036 §D-2); G3 noise-discipline **policy** confirmed (ADR-038), behavioural audit deferred to Wave 2; G4 AIGUARD envelope re-run via `cargo test -p eddacraft-anvil-kernel-types` (`diagnostic_schema_version_constant_matches_spec` pins `anvil.diagnostic.v1`); G5 INTR-004 promoted **Draft → Ready** (2026-05-13); G6 DRVR forward-compat: new `session.rs` co-existed with existing `protocol.rs` types under the full proto suite (28 passed). |
| N4 — Documentation lanes | Owned, scoped | 6/6 | **Closed 2026-05-18.** All six lanes live: air-gap (`docs/runbooks/anvil-air-gapped.md`), hooks-integration (`docs/runbooks/anvil-hook-coexistence.md`), witness-chain operator (`docs/runbooks/anvil-witness-chain.md`), adoption (`docs/runbooks/anvil-adoption.md`), `v0.6.x → v0.7.0-beta` migration (`docs/archive/runbooks/v0.6.x-to-v0.7.0-beta-migration.md`), and INTL / `anvil-run` manpage (`docs/runbooks/anvil-run.md`). Wave 0 (2026-05-13) confirmed ownership: all six lanes assigned to @aneki. |
| N5 — Adoption Trust Surface (ADTRUST) | Complete | 6/6 | All six tasks shipped 2026-05-14: -001 legibility (PR #1531), -005 `--json` schema pin (PR #1532), -006 first-run recipe (PR #1533), -002 banner primitive (PR #1534), -003 doctor states + runbook (PR #1536), -004 start idempotency pin (PR #1537). Cross-crate wire-ups for -002 (watch TUI + hook bridge) and -004 (anvil-hook + kernel embedded fallback) tracked under MLP2 group J. Module archived. |
| N6 — Adoption Friction Removal (ADOPT) | Complete | 6/6 | Remove first-week adoption friction. **Hook coexistence (-001 Done 2026-05-15**, runbook at `docs/runbooks/anvil-hook-coexistence.md`), **CI-enforced resource budget (-002 Done 2026-05-16)**, **AI tool auto-detect (-003 Merged 2026-05-18 via PR #1700** — primitive in PR #1543), **complete ignore policy (-004 Merged 2026-05-16 via PR #1658)**, **clean uninstall (-005) shipped 2026-05-14 via PR #1521**, **editor coexistence matrix (-006 Merged 2026-05-17 via PR #1682)**. All six items Released/Shipped (ADOPT-005 via `v0.6.3-beta`; -001/-002/-003/-004/-006 via `v0.7.0-beta` on 2026-05-21); module **Complete** and archived to `plans/archive/modules/`. Wave 3A of `RELEASE-PLAN.md`. |
| N7 — Distribution & Self-Update (DISTRIB) | Complete | 6/6 | Harden the update/distribution loop so hotfix iteration actually reaches users. Signature verification + resolution-chain robustness (**-001 Merged via PR #1562**), **`anvil version --check` advisory surface (-002 Merged via PR #1569)**, **Homebrew formula automation (-003 Merged via PR #1652)**, release cadence + EOL policy doc (**-004 Done 2026-05-16**, `docs/policies/release-cadence.md`), `anvil migrate` (-005 Released/Shipped via v0.7.3-beta). **DISTRIB-006 Released/Shipped via v0.7.4-beta** (PR #2185 at `c5ee305b` confirmed in tag; `ANVIL_HOME` / `--anvil-home` install-root override) — module advanced to **Complete** 2026-06-08 per the v0.7.4-beta release-record post-tag note. Promoted **Proposed → Ready** 2026-05-14. ADR-044 §9 makes -001 and -002 load-bearing for the MCP-backend swap discovery gap. Landed in Wave 3A. |
| N8 — Usage Insights (INSIGHTS) | In Progress | 3/4 | Periodic value signal during the silent middle. `anvil insights` weekly summary (-001 Done 2026-05-17), suppression health view (-002 Merged), drift trend sparkline (-003 Merged 2026-05-29 via PR #2111), first-week adoption hint (-004). Local-only, no telemetry. Promoted **Proposed → Ready** 2026-05-14; INSIGHTS-001 picked up and completed 2026-05-17. Lands in Wave 4. |
| N9 — Boring Week validation gate | Post-tag graduation | — | Three or more internal users run `v0.7.0-beta` on real work for one calendar week under fresh-user config. Any disabled check, unresolved suppression, or hook bypass blocks graduation of the sit-on claim and triggers a patch/yank decision per `RELEASE-PLAN.md` Wave 5. Participants TBD by @aneki before tag. |

**Window risk:** MLP-002 (witness chain primitive) is the single point of
failure — every downstream lane reads/writes against it. Spike-first as a
standalone PR (flock + DAG verification + 80-parallel-hook test) before any
hook lane starts. Keep the recovery shape in the active release plan when MLP is
promoted back into the current release window.

#### Last release — `v0.5.0-beta` (shipped 2026-05-01)

The slate below shipped as `v0.5.0-beta` on 2026-05-01. Tables are retained
for historical record; counts read "Complete / Locked" rather than "Complete
/ In Progress". For active release sequencing see
[`ROADMAP.md`](../ROADMAP.md) (strategic narrative) and the module status
table earlier in this file (work-state authority); the next-release menu
lives in [`RELEASE-PLAN.md`](../RELEASE-PLAN.md).

#### A1 — RTAI Spike Slice (launch-blocker, ~24 items, shipped)

The A1 cut was a **virtual slice** cherry-picked across four modules
(INTD, INTR, RMCP, RTAI). Status was reconciled on 2026-04-30 after the
RMCP-008 Cursor / Claude Code GUI dry-run completed against
`target/release/anvil` and was recorded in the RTAI demo runbook validation
log (`plans/specs/2026-04-26-rtai-demo-runbook.md` §8). The shipped release
state and dependency order are mirrored in
[`RELEASE-PLAN.md`](../RELEASE-PLAN.md).

| Source module | A1 items | Complete | Committed | In Progress | Ready / unblocked | Blocked |
| ------------- | -------- | -------- | --------- | ----------- | ----------------- | ------- |
| INTD | -001, -002, -003, -005, -007, -013, -014 | -001, -002, -003, -005, -007, -013, -014 | — | — | — | — |
| INTR | -001 (trait), -002 (secret), -006 (registry), -008 (reasoning) | -001, -002, -006, -008 | — | — | — | — |
| RMCP | -001..-008 | -001..-008 | — | — | — | — |
| RTAI | -001 (spike), -002, -003, -006, -008 | -001, -002, -003, -006, -008 | — | — | — | — |
| **Total** | **24** | **24** | **0** | **0** | **0** | **0** |

**A1 — Shipped in `v0.5.0-beta`.** All 24 items shipped and validated. The
next slice for RMCP/RMCPF is captured here so it does not get lost between
release cuts. `v0.5.0-beta` was explicitly validated as
**embedded-fallback-backed, not daemon-backed**; the daemon wiring is the
headline post-release follow-up:

1. **Wire the daemon validation client:** RMCP-005's live daemon-backed
   `DaemonValidationClient` is committed in PR #1277. The client now calls the
   daemon `scan_buffer` RPC when available and keeps the embedded path as the
   correctness-equivalent fallback for genuinely unavailable daemon paths.

**Daemon-path note:** RMCP-005's `DaemonValidationClient` now has a live
JSON-RPC implementation committed in PR #1277. MCP `tools/call` uses the
daemon-backed pipeline when the owner-only IPC endpoint is available; the
embedded path remains the correctness-equivalent fallback when the daemon is not
running.

**A1 ambiguities resolved by ship:**

- INTR slice item: launch slice listed "INTR-006 config" — INTR-006 is the
  rule **Registry** and INTR-007 is rule **Configuration**. The registry
  was load-bearing for the daemon-backed path and shipped under -006;
  INTR-007 (Configuration) remains Draft for the next release window.
- The X5 ADR-030 sequencing question is effectively resolved: INTD work
  *did* ship inside the `-beta` cut. The tag-rename option for the next
  release (daemon-backed RTV) is still open but no longer blocks A1's
  status.

#### A2-A4 — Shipped Source Modules

The remaining `v0.5.0-beta` slices were smaller than A1 but still spanned
multiple APS modules. This table names the exact module subsets that formed
each slice; full module state remains in the detailed module tables below.
Items listed under "Remaining state" did **not** ship in `v0.5.0-beta` and
remain candidates for the next-release slate.

| Slice | Source module | Locked items | Complete | Remaining state |
| ----- | ------------- | ------------ | -------- | --------------- |
| A2 | AIGUARD | AIGUARD-001..-004 | AIGUARD-001..-004 | — |
| A3 | GHOOK | GHOOK-001 | GHOOK-001 | — |
| A3 | ATTRIB | ATTRIB-001, ATTRIB-002, ATTRIB-003 | ATTRIB-001..-003 | ATTRIB-004..-011 remain outside this release cut |
| A3 | SCAN | SCAN-001, SCAN-002, SCAN-003 | SCAN-001..-003 | SCAN-004/-005 remain outside this release cut |
| A4 | LANGTS | LANGTS-001, LANGTS-003 | LANGTS-001, LANGTS-003 | LANGTS-002/-004/-005 remain outside the locked floor unless re-scoped |
| A4 | OPSUP | OPSUP-001 (check-ID registry slice) | OPSUP-001 | OPSUP-002..-007 remain outside this release cut |
| A4 | SURFENV | SURFENV-001..-006 | SURFENV-001..-006 | — |


### Future — Ratatui TUI (RATS, Done)

7/7 tasks complete. Full task table in [completed-index.aps.md](./completed-index.aps.md).
**Module:** [RATS — Ratatui TUI](./archive/modules/ratatui-tui.aps.md)

### Future — Ink-to-Ratatui Port (PORT, Done)

15/15 tasks complete. Full task table in [completed-index.aps.md](./completed-index.aps.md).
**Module:** [PORT — Ink-to-Ratatui Port](./archive/modules/ink-to-ratatui-port.aps.md)

## Milestones

### M1: Core Analysis Engine

- **Status:** Complete
- **Includes:** save-time-trust, architecture-safety
- **Delivered:** `anvil check <file>` returns warnings with explanations

### M2: Anti-pattern Detection

- **Status:** Complete
- **Includes:** antipattern-library
- **Delivered:** ESLint-disable, `any`, `@ts-ignore` detected in new code

### M3: Developer Ergonomics

- **Status:** Complete
- **Includes:** suppressions, drift-reporting
- **Delivered:** Developers can suppress with accountability; drift snapshots and reports

### M4: Integration Points

- **Status:** Complete
- **Includes:** ci-integration, ide-integration
- **Delivered:** PRs show warning summaries via GitHub Action; VS Code extension v0.1.0

## Modules

### Completed (0.1.0)

Task-level detail for all completed work is archived in
[completed.aps.md](./completed.aps.md).

| Module | Scope | Release |
| ------ | ----- | ------- |
| [save-time-trust](./archive/modules/save-time-trust.aps.md) | CORE | 0.1.0 |
| [architecture-safety](./archive/modules/architecture-safety.aps.md) | ARCH | 0.1.0 |
| [antipattern-library](./archive/modules/antipattern-library.aps.md) | ANTI | 0.1.0 |
| [suppressions](./archive/modules/suppressions.aps.md) | SUPP | 0.1.0 |
| [ci-integration](./archive/modules/ci-integration.aps.md) | CI | 0.1.0 |
| [tui](./archive/modules/tui.aps.md) | TUI | 0.1.0 |
| [documentation-polish](./archive/modules/documentation-polish.aps.md) | DOCS | 0.1.0 |
| [explain-command](./archive/modules/explain-command.aps.md) | EXPLAIN | 0.1.0 |
| [drift-reporting](./archive/modules/drift-reporting.aps.md) | DRIFT | 0.1.0 |
| [opa-architecture-integration](./archive/modules/opa-architecture-integration.aps.md) | OPA | 0.1.0 |
| [ide-integration](./archive/modules/ide-integration.aps.md) | IDE | 0.1.0 |
| [llms-txt-export](./archive/modules/llms-txt-export.aps.md) | LLMS | 0.1.0 |
| [command-safety-validation](./archive/modules/command-safety-validation.aps.md) | CMDSAF | 0.1.0 |
| [mcp-server](./archive/modules/mcp-server.aps.md) | MCP | 0.1.0 |
| [aps-markdown-adapter](./archive/modules/aps-markdown-adapter.aps.md) | APSMD | 0.1.0 |
| [adapter-upstream-updates](./archive/modules/adapter-upstream-updates.aps.md) | ADAPTUP | 0.1.0 |
| [onboarding-feedback-resolution](./archive/modules/onboarding-feedback-resolution.aps.md) | ONFBK | 0.1.0 |
| [html-css-support](./archive/modules/html-css-support.aps.md) | HTMLCSS | 0.1.0 |
| [intelligent-first-run](./archive/modules/intelligent-first-run.aps.md) | IFR | 0.1.0 |
| [tutorial-overhaul](./archive/modules/tutorial-overhaul.aps.md) | TUT | 0.1.0 |
| [tutorial-path-continuation](./archive/modules/tutorial-path-continuation.aps.md) | Tutorial | 0.1.x |
| [website-migration](./archive/modules/website-migration.aps.md) | WEB | 0.1.0 |
| [monorepo-migration](./archive/modules/monorepo-migration.aps.md) | MONO | 0.1.0 |
| [test-quality](./archive/modules/test-quality.aps.md) | TEST | — |
| [beta-launch-checklist](./archive/modules/beta-launch-checklist.aps.md) | — | 0.1.2-beta |
| [beta-testing-improvements](./archive/modules/beta-testing-improvements.aps.md) | — | 0.1.2-beta |
| [post-beta-launch-uplift](./archive/modules/post-beta-launch-uplift.aps.md) | PBLU | 0.1.x |
| [migrate-unosend-to-resend](./archive/modules/migrate-unosend-to-resend.md) | — | 0.1.x |

### Completed (0.1.x)

| Module | Scope | Status | Progress |
| ------ | ----- | ------ | -------- |
| [cli-hardening](./archive/modules/cli-hardening.aps.md) | CLIH | Complete | — |
| [coaching-nudges](./archive/modules/coaching-nudges.aps.md) | NUDGE | Complete | — |
| [mcp-server-hardening](./archive/modules/mcp-server-hardening.aps.md) | MCPH | Complete | — |
| [nx-task-migration](./archive/modules/nx-task-migration.aps.md) | NXTASK | Complete | 6/6 |
| [security-ci-pipeline](./archive/modules/security-ci-pipeline.aps.md) | SEC | Complete | — |
| [cli-esbuild-bundling](./archive/modules/cli-esbuild-bundling.aps.md) | BUNDLE | Complete | 3/3 |
| [01-forge-hook-agent](./archive/modules/01-forge-hook-agent.aps.md) | FORGE | Complete | 5/5 |
| [02-forge-negotiation](./archive/modules/02-forge-negotiation.aps.md) | FNEG | Complete | 5/5 |
| [03-deferred-finding-filing](./archive/modules/03-deferred-finding-filing.aps.md) | DEFER | Complete | 5/5 |
| [04-temper-workflow](./archive/modules/04-temper-workflow.aps.md) | TEMPER | Complete | 6/6 |
| [05-forge-temper-config](./archive/modules/05-forge-temper-config.aps.md) | FTCFG | Complete | 6/6 |
| [code-review-backlog](./archive/modules/code-review-backlog.aps.md) | CRB | Complete | 29/29 |

### Completed (0.4.0 — Edda Stack)

| Module | Scope | Status | Progress | Dependencies |
| ------ | ----- | ------ | -------- | ------------ |
| [kindling-integration](./archive/modules/kindling-integration.aps.md) | KINDLING | Complete | 19/19 | save-time-trust, drift-reporting |
| [ember](./archive/modules/ember.aps.md) | EMBER | Complete | 14/14 | kindling-integration |
| [edda](./archive/modules/edda.aps.md) | EDDA | Complete | 19/19 | ember |
| [edda-stack-integration](./archive/modules/edda-stack-integration.aps.md) | STACK | Complete | 19/19 | kindling-integration, ember, edda |
| [edda-ember-review](./archive/modules/edda-ember-review.aps.md) | EERB | Complete | 16/16 | ember, edda |

### Completed (Distribution)

| Module | Scope | Status | Progress | Notes |
| ------ | ----- | ------ | -------- | ----- |
| [distribution-pipeline](./archive/modules/distribution-pipeline.aps.md) | DIST | Complete | 8/10 | DIST-008 (crates.io) deferred per ADR-018; DIST-011 (scoop) optional-deferred. Install path via install.sh / install.ps1 / Homebrew tap / WinGet all shipping on every tagged release. |

### Retired / Superseded

| Module | Scope | Status | Superseded By | Notes |
| ------ | ----- | ------ | ------------- | ----- |
| [interactive-tutorial](./archive/modules/interactive-tutorial.aps.md) | TUTOR | Retired | [restore-welcome-screen](./archive/modules/restore-welcome-screen.aps.md) (WELCOME) | All 13 TUTOR items absorbed into WELCOME's 18 items across 6 phases. |

### Task Status — 0.1.0 (Core Engine)

| Task     | Module          | Description                      | Status   |
| -------- | --------------- | -------------------------------- | -------- |
| CORE-001 | save-time-trust | Warning schema definition        | Complete |
| CORE-002 | save-time-trust | Check runner refactor            | Complete |
| CORE-003 | save-time-trust | CLI check command                | Complete |
| CORE-004 | save-time-trust | Git-aware changed file detection | Complete |
| CORE-005 | save-time-trust | Source file watch mode           | Complete |
| ARCH-001 | architecture    | Baseline inference               | Complete |
| ARCH-002 | architecture    | Edge detection                   | Complete |
| ARCH-003 | architecture    | Architecture check integration   | Complete |
| ARCH-004 | architecture    | CLI architecture service         | Complete |
| ANTI-001 | antipattern     | Pattern catalogue definition     | Complete |
| ANTI-002 | antipattern     | Scanner implementation           | Complete |
| ANTI-003 | antipattern     | Antipattern check integration    | Complete |
| ANTI-004 | antipattern     | Allowlist and opt-in support     | Complete |
| SUPP-001 | suppressions    | Suppression parser               | Complete |
| SUPP-002 | suppressions    | Suppression store                | Complete |
| SUPP-003 | suppressions    | Gate runner integration          | Complete |
| CI-001   | ci-integration  | GitHub Action composite          | Complete |
| CI-002   | ci-integration  | Changed files detection          | Complete |
| CI-003   | ci-integration  | PR comments and status checks    | Complete |
| CI-004   | ci-integration  | Documentation and configuration  | Complete |

### Task Status — 0.1.0 (Onboarding TUI)

| Task    | Module | Description                   | Status   | Priority |
| ------- | ------ | ----------------------------- | -------- | -------- |
| TUI-001 | tui    | Ink foundation and components | Complete | high     |
| TUI-002 | tui    | `anvil init` wizard           | Complete | high     |
| TUI-003 | tui    | `anvil status` dashboard      | Complete | high     |
| TUI-004 | tui    | `anvil doctor` diagnostics    | Complete | high     |
| TUI-005 | tui    | First-run welcome experience  | Complete | high     |
| TUI-008 | tui    | Testing infrastructure        | Complete | medium   |

### Task Status — 0.1.0 (Documentation)

| Task     | Module | Description            | Status   | Priority |
| -------- | ------ | ---------------------- | -------- | -------- |
| DOCS-001 | docs   | Quick Start Guide      | Complete | high     |
| DOCS-002 | docs   | User Guide command ref | Complete | high     |
| DOCS-003 | docs   | Demo material creation | Complete | high     |
| DOCS-004 | docs   | Error message audit    | Complete | medium   |
| DOCS-005 | docs   | Troubleshooting guide  | Complete | medium   |
| DOCS-006 | docs   | README refresh         | Complete | high     |

### Task Status — 0.1.0 (Explain Command)

| Task       | Module  | Description               | Status   | Priority |
| ---------- | ------- | ------------------------- | -------- | -------- |
| EXPLAIN-001 | explain | Warning ID system         | Complete | high     |
| EXPLAIN-002 | explain | Explanation templates     | Complete | high     |
| EXPLAIN-003 | explain | Architecture explanations | Complete | high     |
| EXPLAIN-004 | explain | Anti-pattern explanations | Complete | high     |
| EXPLAIN-005 | explain | ExplainService            | Complete | high     |
| EXPLAIN-006 | explain | CLI explain command       | Complete | high     |

### Task Status — 0.1.0 (Drift Reporting)

| Task     | Module | Description               | Status   | Priority |
| -------- | ------ | ------------------------- | -------- | -------- |
| DRIFT-001 | drift  | Snapshot schema & storage | Complete | high     |
| DRIFT-002 | drift  | Snapshot capture          | Complete | high     |
| DRIFT-003 | drift  | Snapshot comparison       | Complete | high     |
| DRIFT-004 | drift  | Report generator          | Complete | medium   |
| DRIFT-005 | drift  | CLI drift commands        | Complete | high     |

### Task Status — 0.1.0 (Onboarding Feedback Resolution)

| Task     | Module | Description                                 | Status   | Priority |
| -------- | ------ | ------------------------------------------- | -------- | -------- |
| ONFBK-001 | onfbk  | Fix --no-tui flag handling                  | Complete | high     |
| ONFBK-002 | onfbk  | Fix TUI wizard early exit                   | Complete | high     |
| ONFBK-003 | onfbk  | Improve layer detection for project variety | Complete | high     |
| ONFBK-004 | onfbk  | Improve entry points presentation           | Complete | medium   |
| ONFBK-005 | onfbk  | Add architecture explanation                | Complete | medium   |

### Task Status — 0.1.0 (OPA & Architecture Integration)

| Task    | Module | Description                         | Status      | Priority |
| ------- | ------ | ----------------------------------- | ----------- | -------- |
| OPA-001 | opa    | Architecture YAML schema (Zod)      | Complete    | high     |
| OPA-002 | opa    | YAML parser with template expansion | Complete    | high     |
| OPA-003 | opa    | DC config generator from YAML       | Complete    | high     |
| OPA-004 | opa    | `anvil architecture init` command   | Complete    | high     |
| OPA-005 | opa    | Architecture context extraction     | Complete    | high     |
| OPA-006 | opa    | OPA input schema enhancement        | Complete    | high     |
| OPA-007 | opa    | Gate runner integration             | Complete    | high     |
| OPA-008 | opa    | Rego generator from architecture    | Complete    | high     |
| OPA-009 | opa    | Generated policy marker             | Complete    | medium   |
| OPA-010 | opa    | Auto-regeneration on YAML change    | Complete    | medium   |
| OPA-011 | opa    | Layered architecture template       | Complete    | medium   |
| OPA-012 | opa    | Hexagonal architecture template     | Complete    | medium   |
| OPA-013 | opa    | Clean Architecture template         | Complete    | medium   |
| OPA-014 | opa    | DDD template with bounded contexts  | Complete    | medium   |
| OPA-015 | opa    | Template loader and validator       | Complete    | medium   |
| OPA-016 | opa    | TypeScript analyser foundation      | Deferred    | low      |
| OPA-017 | opa    | Path alias resolver                 | Deferred    | low      |
| OPA-018 | opa    | Analyser feature flag               | Deferred    | low      |
| OPA-019 | opa    | Bundle download and caching         | Complete    | medium   |
| OPA-020 | opa    | Signature verification              | Complete    | medium   |
| OPA-021 | opa    | Basic auth and CLI commands         | Complete    | medium   |

> **Note:** OPA-016 through OPA-018 were deferred when the OPA module was marked
> Complete at OPA-015. OPA-019 through OPA-021 (remote policy bundles) were
> subsequently implemented. The remaining tasks may be revisited in the OPA
> Enhancements module (OPAE) or a future release.

### Task Status — 0.1.0 (Monorepo Migration)

| Task     | Module | Description                          | Status   | Priority |
| -------- | ------ | ------------------------------------ | -------- | -------- |
| MONO-001 | mono   | Nx generators for package scaffolding | Complete | high     |
| MONO-002 | mono   | Import path codemod                  | Complete | high     |
| MONO-003 | mono   | Shared tooling packages              | Complete | medium   |
| MONO-004 | mono   | Extract contracts from core          | Complete | high     |
| MONO-005 | mono   | Extract ports from core              | Complete | high     |
| MONO-006 | mono   | Extract pure domain to core          | Complete | high     |
| MONO-007 | mono   | Extract runtime package              | Complete | high     |
| MONO-008 | mono   | Extract policy package               | Complete | high     |
| MONO-009 | mono   | Extract config package               | Complete | medium   |
| MONO-010 | mono   | Extract storage package              | Complete | medium   |
| MONO-011 | mono   | Extract crypto package               | Complete | medium   |
| MONO-012 | mono   | Split adapters per-integration       | Complete | medium   |
| MONO-013 | mono   | Move CLI to apps/                    | Complete | high     |
| MONO-014 | mono   | Reorganise E2E tests                 | Complete | medium   |
| MONO-015 | mono   | Move scripts to tools/               | Complete | low      |
| MONO-016 | mono   | Full test suite validation           | Complete | high     |
| MONO-017 | mono   | Dependency graph validation          | Complete | high     |
| MONO-018 | mono   | Documentation update                 | Complete | medium   |

### Task Status — 0.1.0 (APS Markdown Adapter)

| Task     | Module | Description                          | Status   | Priority |
| -------- | ------ | ------------------------------------ | -------- | -------- |
| APSMD-001 | apsmd  | APSMarkdownAdapter with detection    | Complete | high     |
| APSMD-002 | apsmd  | Confidence scoring system            | Complete | high     |
| APSMD-003 | apsmd  | Parse method implementation          | Complete | high     |
| APSMD-004 | apsmd  | Task-to-Change conversion            | Complete | high     |
| APSMD-005 | apsmd  | Registry integration                 | Complete | high     |
| APSMD-006 | apsmd  | CLI PlanLoader integration           | Complete | high     |

### Task Status — 0.1.0 (Advanced Experience)

#### IDE Integration (VS Code Extension)

| Task    | Module | Description                                     | Status   | Priority |
| ------- | ------ | ----------------------------------------------- | -------- | -------- |
| IDE-001 | ide    | Embed @eddacraft/anvil-core for fast-path operations      | Complete | high     |
| IDE-002 | ide    | Anti-pattern detection on save with diagnostics | Complete | high     |
| IDE-003 | ide    | Improve source location mapping from CLI output | Complete | medium   |
| IDE-004 | ide    | Architecture gate display in tree view          | Complete | high     |
| IDE-005 | ide    | OPA policy failure display with remediation     | Complete | high     |
| IDE-006 | ide    | Click-to-navigate for all violation types       | Complete | medium   |
| IDE-007 | ide    | APS and Rego syntax highlighting                | Complete | medium   |
| IDE-008 | ide    | Analysis caching and Marketplace preparation    | Complete | medium   |

#### TUI Operational (CLI)

| Task    | Module | Description                       | Status  | Priority |
| ------- | ------ | --------------------------------- | ------- | -------- |
| TUI-009 | tui    | `anvil watch` real-time dashboard | Complete | medium   |
| TUI-013 | tui    | `<MermaidDiagram />` component + `layersToMermaid()` helper | Complete | high |
| TUI-014 | tui    | Replace existing ASCII diagrams with mermaid rendering | Complete | high |
| TUI-015 | tui    | `anvil architecture visualise` command (ascii/svg/mermaid formats) | Complete | high |

### Task Status — 0.1.0 (HTML/CSS Support)

| Task        | Module  | Description                                 | Status   | Priority |
| ----------- | ------- | ------------------------------------------- | -------- | -------- |
| HTMLCSS-001 | htmlcss | Make analysable extensions configurable      | Complete | high     |
| HTMLCSS-002 | htmlcss | HTML anti-pattern detectors (AP-008–011)     | Complete | high     |
| HTMLCSS-003 | htmlcss | CSS anti-pattern detectors (AP-012–013)      | Complete | high     |
| HTMLCSS-004 | htmlcss | HTML/CSS edge detection                      | Complete | high     |
| HTMLCSS-005 | htmlcss | HTML suppression comment syntax              | Complete | high     |
| HTMLCSS-006 | htmlcss | VS Code extension HTML/CSS trigger           | Complete | medium   |
| HTMLCSS-007 | htmlcss | Documentation and tests                      | Complete | medium   |

### Task Status — 0.1.0 (Tutorial Overhaul)

| Task    | Module | Description                                          | Status   | Priority |
| ------- | ------ | ---------------------------------------------------- | -------- | -------- |
| TUT-001 | tut    | Rewrite tutorial step types for scan-watch-fix flow  | Complete | high     |
| TUT-002 | tut    | Create ScanStep TUI component                        | Complete | high     |
| TUT-003 | tut    | Create WatchStep TUI component                       | Complete | high     |
| TUT-004 | tut    | Create FixStep TUI component                         | Complete | high     |
| TUT-005 | tut    | Create NextStepsStep and wire up Tutorial.tsx         | Complete | high     |
| TUT-006 | tut    | Interactive policy creation tutorial                  | Complete | medium   |
| TUT-007 | tut    | Interactive architecture boundaries tutorial          | Complete | medium   |
| TUT-008 | tut    | Interactive drift tracking tutorial                   | Complete | medium   |
| TUT-009 | tut    | Interactive CI integration tutorial                   | Complete | high     |
| TUT-010 | tut    | Docs-site tutorials section                           | Complete | high     |
| TUT-011 | tut    | Rewrite quickstart.md and update navigation           | Complete | high     |
| TUT-012 | tut    | Tutorial --list flag and e2e test                     | Complete | high     |

### Task Status — 0.1.0 (Intelligent First Run)

| Task    | Module | Description                                   | Status   | Priority |
| ------- | ------ | --------------------------------------------- | -------- | -------- |
| IFR-001 | ifr    | Add project context detection service         | Complete | high     |
| IFR-002 | ifr    | Create smart defaults generator               | Complete | high     |
| IFR-003 | ifr    | Add post-init automatic analysis              | Complete | high     |
| IFR-004 | ifr    | Create quick wins identifier                  | Complete | high     |
| IFR-005 | ifr    | Create interactive results dashboard TUI      | Complete | high     |
| IFR-006 | ifr    | Add historical analysis feature               | Complete | medium   |
| IFR-007 | ifr    | Integrate all components in init flow         | Complete | high     |
| IFR-008 | ifr    | Update documentation                          | Complete | medium   |

### Task Status — 0.1.0 (Adapter Upstream Updates)

| Task        | Module  | Description                                 | Status   | Priority |
| ----------- | ------- | ------------------------------------------- | -------- | -------- |
| ADAPTUP-001 | adaptup | Update BMAD folder structure detection       | Complete | high     |
| ADAPTUP-002 | adaptup | Update BMAD config path handling             | Complete | high     |
| ADAPTUP-003 | adaptup | Update BMAD variable syntax                  | Complete | medium   |
| ADAPTUP-004 | adaptup | Add BMAD hasSidecar field support             | Complete | medium   |
| ADAPTUP-005 | adaptup | Update SpecKit command namespace detection   | Complete | high     |
| ADAPTUP-006 | adaptup | Add SpecKit AGENTS.md support                | Complete | medium   |
| ADAPTUP-007 | adaptup | Update adapter test fixtures                 | Complete | high     |
| ADAPTUP-008 | adaptup | Update adapter documentation                 | Complete | medium   |

### Task Status — 0.1.0 (AI Tool Integration)

| Task       | Module         | Description                       | Status  | Priority |
| ---------- | -------------- | --------------------------------- | ------- | -------- |
| LLMS-001   | llms-txt       | Constraint collector              | Complete | high     |
| LLMS-002   | llms-txt       | llms.txt formatter                | Complete | high     |
| LLMS-003   | llms-txt       | MCP resource formatter            | Complete | medium   |
| LLMS-004   | llms-txt       | Prompt fragment formatter         | Complete | medium   |
| LLMS-005   | llms-txt       | CLI export command                | Complete | high     |
| CMDSAF-001 | command-safety | Rule system and types             | Complete | high     |
| CMDSAF-002 | command-safety | Command parser with unwrapping    | Complete | high     |
| CMDSAF-003 | command-safety | Rule matcher with specificity     | Complete | high     |
| CMDSAF-004 | command-safety | Default git operation rules       | Complete | medium   |
| CMDSAF-005 | command-safety | Default filesystem rules          | Complete | medium   |
| CMDSAF-006 | command-safety | CommandSafetyCheck implementation | Complete | high     |
| CMDSAF-007 | command-safety | Configuration system              | Complete | medium   |
| CMDSAF-008 | command-safety | Message formatting                | Complete | low      |
| CMDSAF-009 | command-safety | CLI integration and documentation | Complete | high     |
| MCP-001    | mcp-server     | Package scaffold and basic server | Complete | high     |
| MCP-002    | mcp-server     | anvil_check tool implementation   | Complete | high     |
| MCP-003    | mcp-server     | anvil_gate and anvil_status tools | Complete | high     |
| MCP-004    | mcp-server     | anvil_fix and anvil_suppress tools| Complete | high     |
| MCP-005    | mcp-server     | anvil_query_boundary tool         | Complete | high     |
| MCP-006    | mcp-server     | Resources with subscriptions      | Complete | medium   |
| MCP-007    | mcp-server     | Prompt templates                  | Complete | medium   |
| MCP-008    | mcp-server     | Streamable HTTP transport         | Complete | medium   |
| MCP-009    | mcp-server     | Config generators and CLI         | Complete | high     |
| MCP-010    | mcp-server     | Error handling and JSON-RPC       | Complete | high     |

### Task Status — 0.4.0 (Edda Stack — Memory System)

The Edda Stack provides a three-layer architecture for memory: Kindling (observation),
Ember (interpretation), and Edda (canonical memory).

#### Kindling Integration (Observation Layer)

| Task         | Module   | Description                         | Status   | Priority |
| ------------ | -------- | ----------------------------------- | -------- | -------- |
| KINDLING-001 | kindling | Kindling service wrapper            | Complete | high     |
| KINDLING-002 | kindling | Configuration schema and loading    | Complete | high     |
| KINDLING-003 | kindling | Session observation hooks           | Complete | high     |
| KINDLING-004 | kindling | Gate evaluation observations        | Complete | high     |
| KINDLING-005 | kindling | Action execution observations       | Complete | medium   |
| KINDLING-006 | kindling | Plan lifecycle observations         | Complete | medium   |
| KINDLING-007 | kindling | Human input and constraint obs      | Complete | medium   |
| KINDLING-008 | kindling | Error observations                  | Complete | high     |
| KINDLING-009 | kindling | Query service with scope enforcement| Complete | high     |
| KINDLING-010 | kindling | Query limits and throttling         | Complete | high     |
| KINDLING-011 | kindling | Malicious AI test suite             | Complete | high     |
| KINDLING-012 | kindling | Session query command (run show)    | Complete | high     |
| KINDLING-013 | kindling | Plan, gate, action query commands   | Complete | high     |
| KINDLING-014 | kindling | Status integration                  | Complete | medium   |
| KINDLING-015 | kindling | Sensitive data validation           | Complete | high     |
| KINDLING-016 | kindling | Retention and pruning               | Complete | medium   |
| KINDLING-017 | kindling | Performance benchmarking            | Complete | medium   |
| KINDLING-018 | kindling | Documentation and examples          | Complete | medium   |
| KINDLING-019 | kindling | OpenAPI spec generation             | Complete | medium   |

#### Ember (Interpretive Layer — Candidate Memory)

| Task      | Module | Description                       | Status   | Priority |
| --------- | ------ | --------------------------------- | -------- | -------- |
| EMBER-001 | ember  | Candidate Memory Proposal schema  | Complete | high     |
| EMBER-002 | ember  | Proposal type definitions         | Complete | high     |
| EMBER-003 | ember  | Ember configuration schema        | Complete | high     |
| EMBER-004 | ember  | ProposalStore implementation      | Complete | high     |
| EMBER-005 | ember  | DecayService implementation       | Complete | high     |
| EMBER-006 | ember  | AggregatorService foundation      | Complete | medium   |
| EMBER-007 | ember  | Evaluation rules engine           | Complete | medium   |
| EMBER-008 | ember  | Built-in evaluation rules         | Complete | medium   |
| EMBER-009 | ember  | CandidateService (high-level API) | Complete | high     |
| EMBER-010 | ember  | Kindling observation hooks        | Complete | medium   |
| EMBER-011 | ember  | CLI ember commands                | Complete | high     |
| EMBER-012 | ember  | Query API implementation          | Complete | high     |
| EMBER-013 | ember  | Status integration                | Complete | medium   |
| EMBER-014 | ember  | Documentation and examples        | Complete | medium   |

#### Edda (Canonical Memory Layer)

| Task      | Module | Description                       | Status   | Priority |
| --------- | ------ | --------------------------------- | -------- | -------- |
| EDDA-001  | edda   | Memory Object schema              | Complete | high     |
| EDDA-002  | edda   | Memory type definitions           | Complete | high     |
| EDDA-003  | edda   | Provenance schema                 | Complete | high     |
| EDDA-004  | edda   | Evolution graph schema            | Complete | high     |
| EDDA-005  | edda   | Edda configuration schema         | Complete | high     |
| EDDA-006  | edda   | Git-backed MemoryStore            | Complete | high     |
| EDDA-007  | edda   | YAML serialisation                | Complete | high     |
| EDDA-008  | edda   | Version tracking                  | Complete | medium   |
| EDDA-009  | edda   | PromotionService                  | Complete | high     |
| EDDA-010  | edda   | ProvenanceService                 | Complete | medium   |
| EDDA-011  | edda   | EvolutionService                  | Complete | high     |
| EDDA-012  | edda   | MemoryService (high-level API)    | Complete | high     |
| EDDA-013  | edda   | CLI list and show commands        | Complete | high     |
| EDDA-014  | edda   | CLI promote command               | Complete | high     |
| EDDA-015  | edda   | CLI retire and trace commands     | Complete | high     |
| EDDA-016  | edda   | Human-in-the-loop enforcement     | Complete | high     |
| EDDA-017  | edda   | Status integration                | Complete | medium   |
| EDDA-018  | edda   | Schema migration tooling          | Complete | medium   |
| EDDA-019  | edda   | Documentation                     | Complete | medium   |

#### Edda Stack Integration

| Task      | Module | Description                       | Status   | Priority |
| --------- | ------ | --------------------------------- | -------- | -------- |
| STACK-001 | stack  | Common identifier schemas         | Complete | high     |
| STACK-002 | stack  | Timestamp and temporal schemas    | Complete | high     |
| STACK-003 | stack  | Confidence scale definitions      | Complete | high     |
| STACK-004 | stack  | Provenance link schema            | Complete | high     |
| STACK-005 | stack  | Proposal → Memory type mapping    | Complete | high     |
| STACK-006 | stack  | Observation → Proposal mapping    | Complete | medium   |
| STACK-007 | stack  | Layer port definitions            | Complete | high     |
| STACK-008 | stack  | Event bus for layer communication | Complete | medium   |
| STACK-009 | stack  | Layer mock factories              | Complete | high     |
| STACK-010 | stack  | Integration test fixtures         | Complete | high     |
| STACK-011 | stack  | Provenance chain validator        | Complete | high     |
| STACK-012 | stack  | Stack configuration schema        | Complete | high     |
| STACK-013 | stack  | CLI stack status command          | Complete | high     |
| STACK-014 | stack  | CLI stack validate command        | Complete | high     |
| STACK-015 | stack  | Stack architecture documentation  | Complete    | medium   |
| STACK-016 | stack  | Migration guide                   | Complete    | medium   |
| STACK-017 | stack  | Path drift cleanup in APS plans   | Complete | medium   |
| STACK-018 | stack  | Retroactive evidence capture      | Complete    | medium   |
| STACK-019 | stack  | Missing deliverable audit         | Complete    | medium   |

#### Edda-Ember Review Backlog

Non-critical improvements from the 2026-03-05 consolidated code review of the
Edda + Ember feature branches. All 10 critical issues resolved; these track
remaining major and minor improvements.

| Task     | Module | Description                                        | Status   | Priority |
| -------- | ------ | -------------------------------------------------- | -------- | -------- |
| EERB-001 | eerb   | Race condition in processSession candidate limit   | Complete | Low      |
| EERB-002 | eerb   | EscalationRule assumes array order equals temporal  | Complete | Medium   |
| EERB-003 | eerb   | Prune threshold duplicated with different values   | Complete | Medium   |
| EERB-004 | eerb   | Fallback synthesises fake UUIDs for provenance     | Complete | Medium   |
| EERB-005 | eerb   | Duplicated queryProposals call in ember list       | Complete | Low      |
| EERB-006 | eerb   | Dismissed count missing from anvil status Ember    | Complete | Low      |
| EERB-007 | eerb   | colourStatus/colourConfidence duplicated in ember  | Complete | Low      |
| EERB-008 | eerb   | Hardcoded method: 'cli_command' in attribution     | Complete | Low      |
| EERB-009 | eerb   | Double search filtering is redundant               | Complete | Medium   |
| EERB-010 | eerb   | Hardcoded limit: 100 silently truncates methods    | Complete | Low      |
| EERB-011 | eerb   | groupByKind uses O(n²) array spread in loop        | Complete | Low      |
| EERB-012 | eerb   | getExpiringsSoon double-s typo                     | Complete | Low      |
| EERB-013 | eerb   | SurpriseRule references unknown observation kinds   | Complete | Low      |
| EERB-014 | eerb   | validateEvolutionGraph uses .parse() not .safeParse | Complete | Low      |
| EERB-015 | eerb   | serialisation.ts has manual MemoryIndexEntry type  | Complete | Low      |
| EERB-016 | eerb   | migrateV0ToV1 status preservation path untested    | Complete | Low      |

### Task Status — 0.1.0 (Pulumi Infrastructure as Code)

| Task    | Module | Description                              | Status   | Priority |
| ------- | ------ | ---------------------------------------- | -------- | -------- |
| IAC-001 | iac    | Scaffold Pulumi project in monorepo      | Complete | high     |
| IAC-002 | iac    | Configure Pulumi state backend           | Complete | high     |
| IAC-003 | iac    | Manage website Vercel project config     | Complete | high     |
| IAC-004 | iac    | Manage docs-site Vercel project config   | Complete | high     |
| IAC-005 | iac    | Create VercelApp ComponentResource       | Complete | medium   |
| IAC-007 | iac    | Manage Azure DNS zones and records       | Complete | high     |
| IAC-008 | iac    | Add Pulumi CI/CD pipeline integration    | Complete | high     |
| IAC-009 | iac    | Write unit tests for infrastructure code | Complete | medium   |
| IAC-010 | iac    | Import existing Vercel resources         | Complete | high     |
| IAC-011 | iac    | Document IaC setup and contributor guide | Complete | medium   |
| IAC-012 | iac    | Document rollback procedures             | Complete | medium   |

### Task Status — 0.1.x (Code Review Backlog)

Architectural recommendations from the 2026-02-16 code review.

| Task    | Module | Description                                         | Status   | Priority |
| ------- | ------ | --------------------------------------------------- | -------- | -------- |
| CRB-001 | crb    | Standardise stderr/stdout policy across CLI         | Complete | Medium   |
| CRB-002 | crb    | Consolidate hook scripts to single source           | Complete | Medium   |
| CRB-003 | crb    | Add Zod validation to runtime YAML parsers          | Complete | Medium   |
| CRB-004 | crb    | OPA binary manager safer PATH + shared logger       | Complete | Low      |
| CRB-005 | crb    | Dependency audit — surface errors deterministically | Complete | Medium   |
| CRB-006 | crb    | Monorepo-wide vitest config strategy                | Complete | Low      |
| CRB-007 | crb    | Move process.exit from library code to CLI layer    | Complete | High     |
| CRB-008 | crb    | Consistent workspace root containment for output    | Complete | High     |
| CRB-009 | crb    | OPA checksum table contains placeholder hashes      | Complete | High     |
| CRB-010 | crb    | APS task locking is not atomic (race condition)     | Complete | Medium   |
| CRB-011 | crb    | APS loader maxDepth parameter ignored               | Complete | Low      |
| CRB-012 | crb    | Config loader placeholder vs Complete status drift  | Complete | Low      |
| CRB-013 | crb    | MCP server tests not in vitest include globs        | Complete | Medium   |
| CRB-014 | crb    | Add tests for git command composition safety        | Complete | Medium   |
| CRB-015 | crb    | Add symlink escape tests to file-storage            | Complete | Medium   |
| CRB-016 | crb    | Add Windows separator tests to MCP path guards      | Complete | Low      |
| CRB-017 | crb    | Add tests for platform/core config loaders          | Complete | Low      |
| CRB-018 | crb    | Standardise works-from-repo-root workflow           | Complete | Medium   |
| CRB-019 | crb    | Consistent logging/output conventions               | Complete | Medium   |
| CRB-020 | crb    | Option parsing/validation inconsistency             | Complete | Low      |
| CRB-021 | crb    | Duplicated implementations and naming drift         | Complete | Low      |
| CRB-023 | crb    | Silent fallbacks without visibility                 | Complete | Medium   |
| CRB-024 | crb    | Subprocess calls without timeouts in CI             | Complete | Medium   |
| CRB-025 | crb    | Docs and scripts drifting from reality              | Complete | Low      |
| CRB-026 | crb    | Fix spinner leak on TUI fallback path in audit      | Complete | Medium   |
| CRB-027 | crb    | Add workspace path containment to policy validate   | Complete | High     |
| CRB-028 | crb    | Annotate mcp-config symlink guard as fixed          | Complete | Low      |
| CRB-029 | crb    | Expand test coverage for untested CLI commands      | Complete | Medium   |

### Task Status — 0.1.x (Codebase Maintenance)

| Task      | Module | Description                                         | Status   | Priority |
| --------- | ------ | --------------------------------------------------- | -------- | -------- |
| MAINT-001 | maint  | CLI option coercion utility (from CRB-020 discovery) | Complete | High     |
| MAINT-002 | maint  | Error formatting consistency                        | Complete | Medium   |
| MAINT-003 | maint  | Workspace root resolution patterns                  | Complete | Low      |
| MAINT-004 | maint  | Git operation wrappers                              | Complete | Medium   |
| MAINT-005 | maint  | JSON output formatting                              | Complete | Low      |
| MAINT-006 | maint  | Nx generator for CLI commands                       | Complete | Low      |
| MAINT-007 | maint  | Nx generator for gate checks                        | Complete | Low      |
| MAINT-008 | maint  | Spinner/progress patterns                           | Complete | Low      |

<!--
  Task Status — 0.1.x (Forge & Temper) was the parked internal pre-commit /
  post-push code-review tooling. It was superseded by the Council review system
  and is not a product feature. Per-task tables removed 2026-04-30 to stop the
  concept being surfaced as a feature; full task-level history lives in:
    - plans/archive/modules/01-forge-hook-agent.aps.md
    - plans/archive/modules/02-forge-negotiation.aps.md
    - plans/archive/modules/03-deferred-finding-filing.aps.md
    - plans/archive/modules/04-temper-workflow.aps.md
    - plans/archive/modules/05-forge-temper-config.aps.md
  Design doc archived at docs/archive/2026-02-24-forge-temper-review-pipeline.md.
-->

## Decisions

- **D-001:** Planless-first posture — deliver value without requiring APS plans
  ([ADR](./decisions/001-planless-first.md))
- **D-002:** Warnings over blocks — inform, don't prevent; let CI enforce if
  desired ([ADR](./decisions/002-warnings-over-blocks.md))
- **D-003:** New edges only — baseline existing architecture; warn only on new
  violations ([ADR](./decisions/003-new-edges-only.md))
- **D-004:** Suppression syntax — `@anvil-ignore <ID>: <reason>` with mandatory
  explanation ([ADR](./decisions/004-suppression-syntax.md))
- **D-005:** Ink over OpenTUI — Node.js compatibility over native performance
  ([ADR](./decisions/005-ink-over-opentui.md))
- **D-006:** Hybrid DC + OPA — DC for analysis, OPA for policies, bridge between
  ([ADR](./decisions/006-hybrid-dc-opa.md))
- **D-007:** Pulumi for IaC — open-source Pulumi with TypeScript over Terraform
  for consistency with the monorepo's TypeScript-first toolchain
  ([ADR](./decisions/007-pulumi-iac.md))
- **D-011:** OPA Agent Orchestration — orchestration layer for checkpointed policy
  evaluation, remediation guidance, and auditable exception workflows
  ([ADR](./decisions/011-opa-agent-orchestration.md))
- **D-012:** Eval Harness Adoption — adopt external eval framework behind Anvil
  adapter contracts for CI-native trust regression testing
  ([ADR](./decisions/012-eval-harness-adoption.md))
