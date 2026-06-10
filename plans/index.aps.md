<!-- APS: See https://github.com/eddacraft/anvil-plan-spec for format reference -->
<!-- This document is non-executable. -->

# Anvil — Save-time Trust

> **Latest release tag: `v0.7.4-beta`** (shipped 2026-06-01) — Side-by-Side
> Installs patch (`ANVIL_HOME` / `--anvil-home` install-root override, RLB-007
> watch-CPU stopgap, Windows named-pipe hardening; record at
> [`plans/releases/v0.7.4-beta.md`](./releases/v0.7.4-beta.md)) on top of
> `v0.7.3-beta` (2026-05-31, "Surfacing the Signal" — native TUI dashboards,
> SARIF export, new `anvil insights` views; record at
> [`plans/releases/v0.7.3-beta.md`](./releases/v0.7.3-beta.md)). Those two
> patches close the `v0.7.x` Boring Week window that shipped `v0.7.0-beta`
> (2026-05-21, daemon-working product slate: MLP v1 18/18, `anvil-run`
> launcher INTL 9/9, MLP2 integration surface) plus the `v0.7.1-beta` /
> `v0.7.2-beta` honesty patches. The next active window is **`v0.8.0-beta`**
> ("The Graph-Backed Save-Time Daemon" — the interim-cache slice is Merged;
> [ADR-075](./decisions/075-v080-graph-product-scope.md) (Accepted via council)
> scopes the window to the GV2 **A′ slice** + the A→A′ swap + default-on daemon
> routing. The assistant graph product — GCTX + multi-graph registry — and
> persistence are deferred to v0.9). See [`RELEASE-PLAN.md`](../RELEASE-PLAN.md)
> for the cut detail and [`ROADMAP.md`](../ROADMAP.md) for thematic context.

## Contents

- [Next Best Items](#next-best-items)
- [Release Plan](#release-plan)
- [Graph Substrate](#graph-substrate)
- [Hardening & Maintenance](#hardening--maintenance)
- [Intercept Loop](#intercept-loop)
- [Continuous Improvement](#continuous-improvement)
- [Adoption and Sustained Use](#adoption-and-sustained-use)
- [Rust Engine](#rust-engine)
- [Auth & Access](#auth--access)
- [Tracing Foundation](#tracing-foundation)
- [Usage Analytics](#usage-analytics)
- [Infrastructure as Code](#infrastructure-as-code)
- [Web Dashboard](#web-dashboard)
- [Policy Governance](#policy-governance)
- [Engineering Platform](#engineering-platform)
- [Test Quality](#test-quality)
- [Language & Coverage](#language--coverage)
- [Rust MCP Launch Path](#rust-mcp-launch-path)
- [Future](#future)
- [Dormant: Not Yet Scheduled](#dormant-not-yet-scheduled)

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

## Problem & Success Criteria

**Problem:** The most damaging recurring failure is second-wave feature work
drifting from intended patterns because engineers:

- don't know which patterns apply
- don't read ADRs or architecture diagrams
- don't recognise when their change crosses a boundary

The most reliable early signal: a **new dependency edge** where a function or
class reaches across architectural contexts.

**Success Criteria:**

- [ ] 50%+ of developers run Anvil on every save (adoption) — post-release
- [ ] Time-to-merge for AI-assisted PRs does not increase (throughput) —
      post-release
- [ ] New cross-boundary edges per sprint decreases by 30% within 8 weeks
      (drift) — post-release
- [x] Save-time feedback latency < 2 seconds cached, < 5 seconds cold (speed)
- [ ] < 10% of warnings are suppressed without resolution (signal quality) —
      post-release

## Next Best Items

**Next Best Item (NBI)** is the running, index-owned selector for the best work
to pick up or schedule next. It does not replace APS module truth: every row
must point at an APS module, work item, release-plan gate, or documented
operational follow-up. Keep the list short, ranked, and current when an item
starts, completes, blocks, or a release priority changes.

Selection rules:

- Prefer `Ready` and unblocked work that advances the current release claim,
  adoption, trust, signal quality, or recurring delivery friction.
- Include `Schedule` rows only when the work is not execution-ready but should be
  shaped next because it is likely to outrank ordinary ready work.
- Do not duplicate module tables here. Link the source of truth and state only
  the next action needed to move the item.
- If this list is stale, derive the next pick from the highest-value `Ready`
  item in the active module tables and refresh this section in the same change.

| Rank | NBI | Mode | Source | Why now | Next action |
| ---- | --- | ---- | ------ | ------- | ----------- |
| 1 | `v0.8.0-beta` release cut — verify cut criteria | Schedule | [`RELEASE-PLAN.md`](../RELEASE-PLAN.md) + [ADR-075](./decisions/075-v080-graph-product-scope.md) | The ADR-075 v0.8.0 implementation slice is **complete**: the GV2 A′ hot-path hardening (GV2-024 #2470 + GV2-025 #2459, on the #2442/#2446 swap wave) **and** default-on save-time daemon routing (**DSV-021 #2473** — unset routes through a live daemon, `=0` opts out, presence-guarded so daemon-absent installs are unaffected). What remains is the cut itself. | Audit the §8 base correctness bar (full `Cross` matrix incl. Windows green, `release-readiness.yml` on the source SHA, fresh `ACKNOWLEDGEMENTS`) and confirm the remaining ADR-075 rollout controls (named revert signal + staged beta→GA), then cut `v0.8.0-beta`. |
| 2 | GITGOV-013 — capsule retention/prune decision | Schedule | [`git-native-governance`](./modules/git-native-governance.aps.md) | GITGOV-003..012 and now **GITGOV-014 (state-boundary enforcement, Merged 2026-06-10 via PR #2479)** are all Merged — the capsule wedge plus the ADR-073 boundary enforcement are complete. The last open GITGOV item is GITGOV-013 (retention/prune), design-gated on an ADR-074 retention amendment. (EXCEPT-003 is Done via PR #2401; CIB-053 tracks the dogfood tracked-`.anvil/` disposition the new doctor check surfaced.) | Shape the GITGOV-013 retention decision when an owner slot opens. |

NBI review note (2026-06-10, sixth pass): the entire ADR-075 v0.8.0
implementation slice is now **complete**. The GV2 A′ hot-path hardening landed —
**GV2-024** (hot-read type split + seal + ADR-077 depth cap, #2470) and
**GV2-025** (Criterion p95 / ADR-031 CI gate, #2459), on the #2442/#2446 wave
(GV2-022/027/028/029) — and **DSV-021** (#2473) flipped `ANVIL_WATCH_DAEMON`
default-on with the ADR-075 rollout controls (live-daemon presence guard, explicit
opt-out/force). `validate_paths` certifies through the resident GV2 hot-read index
with verdict parity, the hot path is sealed, the latency gate is green, and the
save-time fix now reaches every user on a live daemon. The remaining v0.8.0 work
is the **release cut** itself (rank 1): the §8 base correctness bar + confirming
the named revert signal / staged beta→GA rollout controls.
Governance frontier advanced — GITGOV-003..010 all Merged 2026-06-08 (capsule
complete through PR #2427; -008 diagnostics via PR #2405), GITGOV-011 (#2460)
and GITGOV-012 (#2465) Merged 2026-06-09, then GITGOV-014 Merged 2026-06-10
via PR #2479 (ADR-073 state-boundary enforcement); EXCEPT-003 Done (#2401).
Last open GITGOV item: GITGOV-013 (Proposed, design-gated on the ADR-074
retention amendment).
The broad Ready pool (USAGE, EDGE, DASH*, OPAG, EVAL, CPOL, IORISK, GATE, ATC,
PATT, TRUST, ILGOV, LAC) remains available but does not outrank the v0.8.0
payload.

## Release Plan

Releases are themed by what they deliver, not sequenced by version number.
Individual packages still use semantic versioning for npm/cargo publishes.

**Shipped release windows** — `v0.5.0-beta` (2026-05-01) through `v0.7.4-beta`
(2026-06-01), including the daemon-working `v0.7.0-beta` slate (MLP / INTL /
N1–N9 picks) and the `v0.6.x` operating-model windows — are fully shipped.
Their per-window tables and slice records have moved to
[`completed-index.aps.md`](./completed-index.aps.md#release-plan) to keep this
index focused on current work. The active planning window is the **`v0.8.0-beta`**
candidate — see the header above, [`RELEASE-PLAN.md`](../RELEASE-PLAN.md), and the
active module tables below.

**Active work below leads with the current `v0.8.0-beta` window** — Graph
Substrate (GV2 A′ slice, the now-landed v0.8.0 payload; the NBI rank-1 item is
verifying the release-cut criteria), Hardening & Maintenance (the
A→A′ daemon save-time swap, DSV), and Intercept Loop (MLP2 enforcement
substrate) — then the rest of the active modules, then the
[Dormant](#dormant-not-yet-scheduled) band.

### Graph Substrate

Persistent joined graph substrate for deterministic enforcement, provenance,
trust, control/session joins, and optional assistant context projection. Graph
v2 is Anvil-first; agent context delivery consumes projections over that same
trusted model.

| Module | Scope | Status | Progress | Dependencies |
| ------ | ----- | ------ | -------- | ------------ |
| [graph-v2-foundation](./modules/graph-v2-foundation.aps.md) | GV2 | In Progress | 13/20 | KERN, anvil-graph-cache, ADR-061/063/064/067/069, ADR-031, INTD, GCTX, EDDA |
| [graph-context-delivery](./modules/graph-context-delivery.aps.md) | GCTX | Draft | 0/13 | GV2 |

### Hardening & Maintenance

Codebase cleanup, .anvil file format, and BMAD v4 compatibility.

| Module                                                                          | Scope  | Status      | Progress                                                                                                                                                                                                                                                                                                                                                                                    |
| ------------------------------------------------------------------------------- | ------ | ----------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| [codebase-maintenance](./archive/modules/codebase-maintenance.aps.md)           | MAINT  | Complete    | 11/11 (1 deferred)                                                                                                                                                                                                                                                                                                                                                                          |
| [anvil-file-format](./archive/modules/anvil-file-format.aps.md)                 | ANVFMT | Complete    | 15/16 (1 reparented to RSCAN-006 under ADR-026)                                                                                                                                                                                                                                                                                                                                             |
| [anvil-rust-scanner](./archive/modules/anvil-rust-scanner.aps.md)               | RSCAN  | Complete    | 8/8 (RSCAN-008 landed — docs now describe the authoritative Rust scanner and the scanner-parity story per ADR-026)                                                                                                                                                                                                                                                                          |
| [nx-task-migration](./archive/modules/nx-task-migration.aps.md)                 | NXTASK | Complete    | 6/6                                                                                                                                                                                                                                                                                                                                                                                         |
| [anvil-scanner-parity-gaps](./archive/modules/anvil-scanner-parity-gaps.aps.md) | SPG    | Complete    | 6/6 (`flags:"i"` honoured, lookaround rules handled via post-filters, doctor surfaces compile failures, fixtures cover every rule, `antipattern_scan` bench + trust-boundary docs landed)                                                                                                                                                                                                   |
| [anvil-ts-scanner-retirement](./archive/modules/anvil-ts-scanner-retirement.aps.md) | TSRET  | **Complete** | 3/3 active (3 superseded) — TSRET-001/-002/-005 Complete; TSRET-003/-004 superseded by DRVR; TSRET-006 superseded by ADR-033. Terminal state on `chore/TSRET-005` (2026-04-29): TS scanner + suppression + drift + gate runner + constraint collector all archived to `archive/anvil-ts-scanner/`; minimal `Warning` type extracted to `core/src/warnings/types.ts`; Rust-side parity test deleted; root `test:scanner-parity` script removed.                                                                 |
| [scanner-adjacent-ts-retirement](./archive/modules/scanner-adjacent-ts-retirement.aps.md) | TSGAP  | Complete    | 9/9 (Remediation complete 2026-05-12: core exports cleaned; compiler moved to active `anvil-format`; drift/export/suppression ownership settled on Rust CLI/local readers; AP-* explanations explicitly retired until Rust explain lands; RMCPF now maps MCP resources to Rust-owned sources; final audit passed) |
| [bmad-v4-backward-compat](./modules/bmad-v4-backward-compat.aps.md)             | BMAD4  | Proposed    | 0/8                                                                                                                                                                                                                                                                                                                                                                                         |
| [dev-environment-hardening](./modules/dev-environment-hardening.aps.md)         | DEVENV | In Progress | 5/8 (ADR-057 worktree/dev-env hardening; DEVENV-001..-006 Merged — debug line-tables, per-worktree CARGO_TARGET_DIR, target eviction, Node 24 standardise, wt.toml bootstrap; DEVENV-003 Blocked on upstream nxrust cache; -007 (wt/CI classifier parity) + -008 (reproducible-base spike) Ready; per-item detail in the module file) |
| [scan-performance](./modules/scan-performance.aps.md)                           | SCAN   | Complete    | 6/6 (SCAN-001/-002/-003 landed as one slice — parallel-scan rollout, ReDoS line-length guard, first-run rayon pool cap; SCAN-004 Merged 2026-05-27 via PR #2021 — welcome `files_skipped_by_ignore` provenance; SCAN-005 Merged 2026-05-28 via PR #2034 — `WalkParallel` benchmark spike (4.5–6.3× walk speedup, ~10–17% end-to-end); SCAN-006 Merged 2026-05-28 via PR #2041 — parallelised the uncapped Phase 1a discovery walk; module all-merged, Released/Shipped in v0.7.3-beta (tag 8bfd48c4d, 2026-05-31) — Complete)                                                                                                                                                                                                         |
| [resource-load-benchmarking](./modules/resource-load-benchmarking.aps.md)       | RLB    | In Progress | 7/8 (filed 2026-05-30 from the beta-tester high-CPU report, GH #2156. RLB-001 + RLB-007 Released/Shipped via v0.7.4-beta — PR #2184 at `72f2de98` confirmed in tag; the load-ramp harness + per-save `anvil check` scoped to the changed file (1 agent 6.55 → 0.08 cores). RLB-002/-003/-004/-005/-008 Merged 2026-06-02 via PR #2228 — process-tree sampler + per-process CPU/RSS budgets (watch churn, intercept daemon, MCP server) + concurrent aggregate + SLO docs/CI. RLB-006 (cross-platform) Proposed.)                                                                                                |
| [daemon-save-time-validation](./modules/daemon-save-time-validation.aps.md)     | DSV    | In Progress | 18/19 (v0.8.0-beta daemon save-time arc; Sub-phase A 9/9 + A-W 2/2 Merged, deferred follow-ups Done, A′ 2/2 Done — GV2 backing swap reconciled from #2446 and DSV-021 #2473 flipped `ANVIL_WATCH_DAEMON` default-on with live-daemon guard + opt-out/force; Sub-phase B warm-start persistence remains Blocked; sub-phase + PR detail in the module file) |
| [nx-rust-plugin](./archive/modules/nx-rust-plugin.aps.md)                       | NXRUST | Complete    | 8/8 (plugin now consumed from npm as `@eddacraft/nxrust`; NXRUST-005/-006 superseded by `cargo metadata` inference — zero per-crate `project.json` needed)                                                                                                                                                                                                                                  |
| [rust-nx-migration](./archive/modules/rust-nx-migration.aps.md)                 | RUSTNX | Complete    | 9/9                                                                                                                                                                                                                                                                                                                                                                                         |
| [v050-release-followups](./modules/v050-release-followups.aps.md)               | V050F  | In Progress | 15/16 (16 hardening items deferred from `v0.5.0-beta` release work: 10 from the council rounds, 1 from the copilot PR #1081 review, 3 from the v0.4.0-beta tag run + post-tag deploy — scoop PAT scope, winget gh arg regression, missing migration runner — 1 from the copilot PR #1090 review tracking the svix>uuid override exception, and 1 private-release Latest promotion fix; 15 done; 1 outstanding — V050F-008 (bench baselines on CI hardware). V050F-015 (svix>uuid override removal) closed 2026-05-31 when `resend@6.12.4` dropped svix. V050F-006 + V050F-011 closed via `fix/v050f-scanner-hotpath` (#1323); V050F-007 closed via `fix/v050f-rayon-init` (#1330).) |
| [v060-release-candidates](./modules/v060-release-candidates.aps.md)             | V060F  | In Progress | 4/25 (V060F-001 complete via RCLI2-009 admin command parity; V060F-025 complete — OPA runtime pin bumped to 1.16.1; V060F-020 complete 2026-05-12 — `TerminalGuard` + idempotent panic hook; V060F-021 complete 2026-05-12 — refreshed tutorial legacy paths; V060F-002..V060F-011 filed 2026-05-07 batch 1; V060F-012..V060F-019 filed 2026-05-07 batch 2; V060F-022..V060F-024 remain open from batch 3) |
| [release-orchestration](./archive/modules/release-orchestration.aps.md)                 | RELORCH | Complete | 12/12 (Completed 2026-05-11 after OPMODEL-012 unblocked main-targeted command work. RELORCH-001 design spec; RELORCH-002 reusable command harness and CI workflow; RELORCH-003 assess; RELORCH-004 preflight; RELORCH-005 prepare with tracking issue create/resume, idempotent release-time edits, preparation commit flow, and metadata comments; RELORCH-006 promote with PR create/resume, conflict/review/merge-state reporting, and readiness workflow request/resume; RELORCH-007 tag with guarded pre/post-push recovery semantics; RELORCH-008 monitor with workflow result surfacing; RELORCH-009 verify with structured release/publisher checks; RELORCH-010 closeout with verification gating and issue closeout semantics; RELORCH-011 skill/runbook wire-up and legacy runner deletion; RELORCH-012 release-record `discarded`/`yanked` lifecycle states and closed `policyDecisions` entries. Successor to archived RELMGMT; supersedes parts of `2026-04-20-relmgmt-agent-driven-release-design.md` while inheriting its no-persistent-manifest tradeoff as a hard constraint.) |

**Design doc (Forge & Temper — archived):**
[docs/archive/2026-02-24-forge-temper-review-pipeline.md](../docs/archive/2026-02-24-forge-temper-review-pipeline.md)

### Intercept Loop

Host-local enforcement daemon that detects policy violations from AI agent file
changes and interrupts the correct session via process-group control.
Shell-first, single-host initially, proving the core enforcement thesis. See
[design spec](./specs/anvil-driver-framework/) for the broader driver framework
vision.

**Implementation state (2026-04-30):** The A1 INTD slice is merged and green:
INTD-001 (daemon scaffold), INTD-002 (full cross-platform IPC), INTD-003
(session registry), INTD-005 (enforcement pipeline), INTD-007 (fence
persistence), INTD-013 (telemetry mirror), and INTD-014 (JSON-RPC conformance +
latency harness). The current release now pulls the completed A1 subset from
INTD and INTR to support RMCP/RTAI pre-write validation; the remaining
INTD/INTR/INTL/DRVR work is queued after the launch shim.

<!--
  INTD count history:
  - Pre-NOTIFY-009: index claimed 0/11, module already had 12 tasks (001–012) — off-by-one.
  - NOTIFY-009 added INTD-013 to mirror control decisions onto telemetry.
  - 2026-04-24 council review M1/M5/M9 filed INTD-014 (JSON-RPC 2.0
    conformance + latency benchmark), INTD-015 (daemon-enforced
    telemetry subscription scoping), INTD-016 (DoS protection budgets).
  - Net: module now has 16 tasks; denominator reconciled to /16 (0 done
    at the time of this note; INTD has since completed 16/16, Complete).

  Note: this comment lives ABOVE the table because an HTML comment between
  table rows terminates the markdown table semantically; oxfmt then sees the
  post-comment rows as orphaned prose and rewraps them. Keeping the comment
  here ensures the four module rows below form one contiguous, valid table.
-->

| Module | Scope | Status | Progress | Dependencies |
| ------ | ----- | ------ | -------- | ------------ |
| [intercept-daemon](./archive/modules/intercept-daemon.aps.md) | INTD | Complete | 16/16 (A1 slice: INTD-001/-002/-003/-005/-007/-013/-014; A2 Wave 1: INTD-008/-012/-015 (PRs #1305/#1306); A2 Wave 2: INTD-004/-006/-009/-010/-016 (PR #1308); A2 Wave 3: INTD-011 (PR #1309)) | anvil-checks, anvil-kernel (watcher), INTR, INTL, NOTIFY |
| [intercept-launcher](./archive/modules/intercept-launcher.aps.md) | INTL | Complete | 9/9 | INTD; coordinates `AgentTag` proto with MLP-014; shipped via PR #1528 (merged 2026-05-14 at `5d38e546`) with `crates/anvil-run/` + 49 unit + 3 shell-integration tests green. All nine items Released/Shipped via `v0.7.0-beta` (2026-05-21); module **Complete**; archived |
| [intercept-rules](./modules/intercept-rules.aps.md) | INTR | In Progress | 5/8 (INTR-004 path-deny rule Complete 2026-05-13; INTR-003 antipattern wrapper / INTR-005 regex-content / INTR-007 rule-config fleshed to **Ready** 2026-05-28 — scope/files grounded in `crates/anvil-intercept-rules/` + existing `anvil-checks` / `anvil-config` APIs; INTR-007 depends on INTR-003 + INTR-005) | anvil-checks, GV2 later for hot-read rules only |
| [multilayer-protection](./archive/modules/multilayer-protection.aps.md) | MLP | Complete | 18/18 (Done 2026-05-13/-14: MLP-001..-018; MLP-018 closed 2026-05-14 via split into MLP2) | INTD / DRVR / RMCP / RTAI / LAUNCH + anvil-checks; ADRs 036–039 Accepted. MLP-009 was the v0.7.0-beta hard gate; MLP-018 split into MLP2. Per-item detail in the archived module. |
| [multilayer-protection-v2](./modules/multilayer-protection-v2.aps.md) | MLP2 | In Progress | 72/87 (daemon-integration debt from the MLP-018 catalogue, Groups A–R; per-item PR/wave history in the module file) | All MLP v1 primitives; INTD enforcement pipeline; DRVR driver framework; RMCP/RMCPF MCP shim; RTAI mid-edit telemetry; LAUNCH activation orchestrator; kindling-integration. ADRs 036–039 already Accepted under MLP. |
| [ssh-remote-host-daemon](./modules/ssh-remote-host-daemon.aps.md) | SSHREMOTE | Proposed | 0/8 (created 2026-05-14 from ADR-043 / SSH remote-host daemon design; remote host owns daemon, hooks, launcher, and witnesses; local side is display/control only) | INTD, INTL, MLP, DRVR, RMCP/RMCPF; ADRs [036](./decisions/036-daemon-scope-discovery-and-boundaries.md), [037](./decisions/037-witness-chain-and-l4-policy.md), [038](./decisions/038-hook-surface-and-noise-discipline.md), [043](./decisions/043-ssh-remote-host-daemon.md). Not in the v0.7 MLP release gate until promoted. |
| [watch-ux-advisory-rules](./archive/modules/watch-ux-advisory-rules.aps.md) | WATCHUX | Complete | 8/8 (**WATCHUX-001..004 Released/Shipped via [`v0.6.3-beta`](./releases/v0.6.3-beta.md) on 2026-05-15**; WATCHUX-005..007 merged via PR #1524; WATCHUX-008 implemented on `feat/watchux-008-config-cache`) | anvil-cli audit/start/watch/status/config, anvil-kernel watch/watcher, anvil-tui watch surface, MLP config/baseline |
| [watch-output-contract](./modules/watch-output-contract.aps.md) | WOUT | Done | 6/6 (created 2026-05-14 from consumer-piping question; hardens `anvil --json watch` from best-effort JSON lines into a versioned NDJSON contract — `anvil.watch.event.v1`. WOUT-001..006 implemented 2026-05-14 with typed wire envelope, stdout/stderr discipline, integration harness, golden fixtures and consumer docs. PR #1554 merged; narratively **Merged** in lifecycle; advances to Released/Shipped on v0.7.0-beta release evidence) | anvil-cli watch JSON mode, anvil-kernel watch events, anvil-kernel-types, WATCHUX stdout/stderr fallback semantics |
| [surface-drivers](./archive/modules/surface-drivers.aps.md) | DRVR | Complete | 5/5 active (2 superseded, 1 deferred under ADR-033) — DRVR-007 Complete (PR #1304: auth.rs trust boundary v1); DRVR-006 Complete (PR #1304: option-(b) Distinguish recorded); DRVR-001 Complete (PR #1307: shared TS driver client); DRVR-002 Complete (PR #1310: editor-driver protocol design + capability negotiation); DRVR-008 Complete (PR #1310: capability negotiation + manifest method advertisement) | INTD-002/-003/-005/-013/-015, ADR-030, ADR-033 (IDE/MCP archived — DRVR-003 deferred until a new extension package is created on the daemon-driver path), RMCP/RMCPF sequencing, GV2 control/session graph later — supersedes TSRET-003/-004 (KERN-050/-051/-052 superseded-into-INTD per ADR-030); DRVR-004 superseded by RMCP/RMCPF; DRVR-003 deferred per ADR-033; DRVR-005 (architecture cross-links) remains Draft pending DRVR-003 un-pause |

**Architecture Decisions:**
[D-015: Intercept Loop Enforcement](./decisions/015-intercept-loop-enforcement.md),
[D-030: Surface Drivers Supersede napi Cutover](./decisions/030-surface-drivers-supersede-napi-cutover.md),
[D-033: Park IDE/MCP Surfaces; Retire TS Scanner Now](./decisions/033-park-ide-mcp-retire-ts-scanner.md)

### Continuous Improvement

Continuous-improvement-backlog is the standing intake for concrete improvement
items identified anywhere in the project. It intentionally remains active while
the project is active; append executable `CIB-NNN` items as they are found.
Codebase-maintenance and code-review-backlog are retained for history.

| Module                                                                      | Scope | Status      | Progress           |
| --------------------------------------------------------------------------- | ----- | ----------- | ------------------ |
| [continuous-improvement-backlog](./modules/continuous-improvement-backlog.aps.md) | CIB   | In Progress | 32/53 (standing continuous-improvement intake; recent CIB-041..-046 release-reliability + flag-gating items Merged/Ready; per-item status in the module file) |
| [clawpatch-pre-tag-v0.7.0-beta](./archive/modules/clawpatch-pre-tag-v0.7.0-beta.aps.md) | CLAWP | Archived | 53/65 (archived 2026-06-03 via CIB-039 — 53 Merged / 11 Ship / 1 Deferred-tracked; CLAWP-001 PR #1732, CLAWP-008 PR #1765, CLAWP-011 PR #1791, CLAWP-012 PR #1772, CLAWP-013 PR #1788, CLAWP-014 PR #1786, CLAWP-015 PR #1783, CLAWP-021 PR #1764, CLAWP-022 PR #1770, CLAWP-028 PR #1763, CLAWP-029 PR #1789, CLAWP-030 commit `9253d9f3` in PR #1732, CLAWP-019 PR #2065, CLAWP-033 PR #2136, CLAWP-009 PR #2135, CLAWP-004 PR #2137, CLAWP-007 PR #2144, CLAWP-027 PR #2145, CLAWP-031 PR #2143, CLAWP-038 PR #2142, CLAWP-017 PR #2058, CLAWP-024 PR #2061, CLAWP-025 PR #2160, CLAWP-026 PR #2159, CLAWP-065 PR #2211; 2026-06-03 reconcile of fixes shipped untracked, verified vs `origin/main`: CLAWP-034 PR #1186, CLAWP-043 PR #1114, CLAWP-044 PR #1163, CLAWP-051 PR #1653; 2026-06-03 #1740 test-hardening batch (24 items) Merged via PRs #2261 / #2265 / #2267) |
| [aps-dashboard-starter](./modules/aps-dashboard-starter.aps.md)             | APSDASH | In Progress | 2/4 (APSDASH-001 Done — ADR-055 filed (OSS carve-out for read-only APS-format consumers — viewer + `scripts/aps/*` tooling; legal-gated). APSDASH-002 Done — seed kit staged under `tools/starters/aps-dashboard/` and build-verified (30/30 tests vs crates.io `eddacraft-tui`). **APSDASH-003 Blocked on ADR-055** — a 2026-05-27 Council review blocked publication: the kit copies proprietary `anvil-cli`/`anvil-tui` source (ADR-018) and relicenses it Apache-2.0 for the public APS repo; must scrub + get legal sign-off first. APSDASH-004 Proposed — downstream re-development in `anvil-plan-spec`.) |
| [code-review-backlog](./archive/modules/code-review-backlog.aps.md)         | CRB   | Complete    | 29/29              |

> ~~continuous-improvement~~ (CI) — retired 2026-04-18; was a meta-module
> without executable tasks. It remains archived. New concrete cross-project
> improvement intake now goes through
> [continuous-improvement-backlog](./modules/continuous-improvement-backlog.aps.md).

### Adoption and Sustained Use

The "release we sit on" cohort. These four modules cover what turns
`v0.7.0-beta` from "feature complete" into "ready for senior engineers to
use daily for a month without uninstalling." They were promoted from
proposal to live planning on 2026-05-14 alongside acceptance of
[`plans/specs/2026-05-14-release-plan-v0.7.0-sit-on.md`](./specs/2026-05-14-release-plan-v0.7.0-sit-on.md);
the live release sequencing is in
[`RELEASE-PLAN.md`](../RELEASE-PLAN.md) (Waves 3A / 3B / 5).

| Module                                                                  | Scope    | Status | Progress | Notes                                                                                                                                                                                              |
| ----------------------------------------------------------------------- | -------- | ------ | -------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| [adoption-trust-surface](./archive/modules/adoption-trust-surface.aps.md) | ADTRUST  | Complete    | 6/6      | All six shipped 2026-05-14 (PRs #1531, #1532, #1533, #1534, #1536, #1537). Cross-crate wire-ups for -002 + -004 tracked under MLP2 group J. Archived.                                                                                                                                                  |
| [adoption-friction](./archive/modules/adoption-friction.aps.md)                 | ADOPT    | Complete | 6/6 | First-week friction removal. **ADOPT-005 `anvil uninstall` merged 2026-05-14 (PR #1521), Released/Shipped via [`v0.6.3-beta`](./releases/v0.6.3-beta.md) on 2026-05-15; ADOPT-001 hook coexistence Done 2026-05-15** (runbook at `docs/runbooks/anvil-hook-coexistence.md`); **resource budget (-002 Done 2026-05-16)**, **shared ignore policy (-004 Merged 2026-05-16 via PR #1658)**, **editor coexistence (-006 Merged 2026-05-17 via PR #1682)**, **AI auto-detect (-003 Merged 2026-05-18 via PR #1700** — primitive in PR #1543). All six items Released/Shipped (ADOPT-005 via `v0.6.3-beta`; the rest via `v0.7.0-beta` on 2026-05-21); module **Complete**; archived. Wave 3A. |
| [distribution-and-update](./archive/modules/distribution-and-update.aps.md)     | DISTRIB  | Complete | 6/6      | Harden `anvil update` + Homebrew + cadence policy so hotfix iteration reaches users. **DISTRIB-001 Merged via PR #1562** (minisign verification + ADR-045). **DISTRIB-002 Merged via PR #1569** (`anvil version --check` advisory surface + watch/status hint). **DISTRIB-003 Merged via PR #1652** (Homebrew formula auto-bump extracted into tested script + workflow + runbook + macOS smoke matrix). **DISTRIB-004 Done 2026-05-16** (`docs/policies/release-cadence.md`). **DISTRIB-005 Released/Shipped via v0.7.3-beta** (PR #1984 at `8ae65b10` confirmed in tag; `anvil migrate schema`). **DISTRIB-006 Released/Shipped via v0.7.4-beta** (PR #2185 at `c5ee305b` confirmed in tag) — `ANVIL_HOME` / `--anvil-home` install-root override for side-by-side candidate installs, ADR-060 gate Accepted 2026-05-31. Module advanced to **Complete** 2026-06-08 per the v0.7.4-beta release-record post-tag note. ADR-044 §9 makes DISTRIB-001 / -002 load-bearing for the MCP-backend swap discovery gap. Wave 3A. |
| [usage-insights](./modules/usage-insights.aps.md)                       | INSIGHTS | In Progress | 3/4      | Local-only periodic value signal (`anvil insights`); INSIGHTS-001 Done 2026-05-17; -002 (#1996) + -003 (#2111) Released/Shipped via v0.7.3-beta 2026-05-31; -004 implemented + PR #2226 (first-week nudge surfaces in `status` + watch; `anvil insights` run suppresses; tests pass; path recon applied). No telemetry.                                            |

### Rust Engine

Rust kernel for structural graph analysis (KERN), performance-critical check
ports (RENG). RATS (Ratatui TUI) and PORT (Ink-to-Ratatui port) are complete.
TUIDASH adds a Rust-native json-render spec interpreter for Ratatui dashboard
rendering; TDASH ships hand-written native Ratatui dashboards for state already
persisted under `.anvil/` (no spec interpreter, no AI), following the `anvil plan
dashboard` precedent. KERN is complete (3 daemon-mode items deferred post-H1),
RENG is complete, RCLI is complete.

| Module                                                                    | Scope   | Status      | Progress                                                                                                          | Dependencies                                                                                                                                                                                                                                                                       |
| ------------------------------------------------------------------------- | ------- | ----------- | ----------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| [rust-kernel](./archive/modules/rust-kernel.aps.md)                       | KERN    | Complete    | 22/25 (3 superseded by INTD per ADR-030 — KERN-050 → INTD-002, KERN-051 → INTD-002+INTD-013, KERN-052 → INTD-003) | —                                                                                                                                                                                                                                                                                  |
| [rust-core-engine](./archive/modules/rust-core-engine.aps.md)             | RENG    | Complete    | 6/6                                                                                                               | KERN Phase 1, KERN Phase 2                                                                                                                                                                                                                                                         |
| [ratatui-tui](./archive/modules/ratatui-tui.aps.md)                       | RATS    | Complete    | 7/7                                                                                                               | KERN Phase 3                                                                                                                                                                                                                                                                       |
| [ink-to-ratatui-port](./archive/modules/ink-to-ratatui-port.aps.md)       | PORT    | Complete    | 15/15                                                                                                             | RATS-001 (complete)                                                                                                                                                                                                                                                                |
| [rust-cli](./archive/modules/rust-cli.aps.md)                             | RCLI    | Complete    | 64/64                                                                                                             | KERN, RATS, PORT                                                                                                                                                                                                                                                                   |
| [kernel-benchmarking](./archive/modules/kernel-benchmarking.aps.md)       | BENCH   | Complete    | 16/16                                                                                                             | KERN Phases 1-2                                                                                                                                                                                                                                                                    |
| [tui-dashboard-render](./modules/tui-dashboard-render.aps.md)             | TUIDASH | In Progress | 13/13 (TUIDASH-001/-002 Released/Shipped via v0.7.3-beta — PRs #2068/#2097 confirmed in tag; TUIDASH-003..-012 engine/components/charts/binding/surface+CLI/parity/responsive/previews Merged 2026-06-02 via PR #2229; TUIDASH-013 ship example gate-summary spec + gate-result persistence Merged 2026-06-02 via PR #2246 — GH #2237/#2242; -003..-013 post-date the v0.7.4-beta tag and ride `v0.8.0-beta`) | eddacraft-tui (engine, feature-gated) + anvil-tui (catalogue/surface) per ADR-054; spec contract `@eddacraft/render` (`packages/libs/render/`); extends TDASH `anvil dashboard`. DASHAI parallel, not blocking                                                                      |
| [native-tui-dashboards](./modules/native-tui-dashboards.aps.md)           | TDASH   | Complete    | 4/4                                                                                                               | anvil-tui (`plan_dashboard` precedent), eddacraft-tui, RCLI; reads persisted `.anvil/` state. Parallel to TUIDASH (json-render); neither blocks the other. Gate-summary/watch-session deferred until their data persists.                                                          |
| [launch-flow-readiness](./archive/modules/launch-flow-readiness.aps.md)   | LAUNCH  | Complete    | 18/18                                                                                                             | RCLI, KERN; coordinates with TUIDASH, DRVR, RMCP, RTAI, INTD; supersedes RTVS in part; adds upgrade/version UX, tutorial polish, repo language profile + filter                                                                                                                    |
| [realtime-ai-validation](./modules/realtime-ai-validation.aps.md)         | RTAI    | In Progress | 8/9                                                                                                               | A1 launch slice complete: RTAI-001 (spike), -002 (PR #1186), -003 (PR #1189), -006 (PR #1190), -008 (PR #1188) merged 2026-04-29/30. A2 Wave 3: RTAI-004 (PR #1311) merged 2026-05-06. RTAI-007 (mid-edit telemetry mirror — `mirror.path = "midEdit"` discriminator) + RTAI-009 (architecture docs + RTVF supersession) **Merged 2026-06-02 via PR #2227**. Only RTAI-005 remains — reframed 2026-06-02 from a VS Code extension to a generic **LSP-server surface** (`anvil lsp`), still **parked under ADR-033**.                                                              |
| [rust-cli-tier2](./modules/rust-cli-tier2.aps.md)                         | RCLI2   | In Progress | 5/9                                                                                                               | RCLI; RCLI2-001..-004 shipped per 2026-04-26 freshness audit (commits 1e44ef2d / c5679432 / a2297dca / 06d764d4); -005..-008 still Proposed (gated on OPAE); -009 complete (admin command parity — list/show/revoke/audit/send-migration/email-update)                           |
| [rust-cli-tier3](./modules/rust-cli-tier3.aps.md)                         | RCLI3   | In Progress | 5/20 (7 Ready)                                                                                                    | RCLI; RCLI3-001 merged 2026-05-17 (PR #1664, `anvil edda list` Rust port). RCLI3-002 completed 2026-05-26 (`anvil edda show <id>` over the existing YAML store). Readiness audit 2026-05-17 promoted RCLI3-005/-008/-012/-014/-015/-017/-018 to Ready. Earlier 2026-05-17: RCLI3-017b merged (PR #1657); RCLI3-016b reconciled (RMCP-007 79da411d) |
| [tui-polish](./archive/modules/tui-polish.aps.md)                         | POLISH  | Complete    | 8/8                                                                                                               | RCLI, RATS                                                                                                                                                                                                                                                                         |
| [restore-welcome-screen](./archive/modules/restore-welcome-screen.aps.md) | WELCOME | Complete    | 18/18                                                                                                             | RCLI, RATS                                                                                                                                                                                                                                                                         |
| [distribution-pipeline](./archive/modules/distribution-pipeline.aps.md)   | DIST    | Complete    | 8/10 (1 deferred, 1 optional-deferred)                                                                            | RCLI                                                                                                                                                                                                                                                                               |

The TypeScript CLI is archived — the Rust kernel adds structural graph analysis
as a new capability (KERN), existing checks port to Rust for speed (RENG), TUI
surfaces use Ratatui (RATS), and existing Ink surfaces are ported systematically
(PORT). See
[Architecture Evolution](../docs/architecture/anvil-architecture-evolution.md)
for the phased rollout plan.

### Auth & Access

Streamline beta access: device code + email OTP activation flows, JWT session
model with rotating refresh tokens, admin CLI approval, Resend audience
management. Docs auth gating adds GitHub OAuth as a third activation mechanism
and gates `/anvil` docs behind it via Vercel Edge.

| Module                                                                | Scope     | Status   | Progress | Dependencies |
| --------------------------------------------------------------------- | --------- | -------- | -------- | ------------ |
| [beta-auth-streamline](./archive/modules/beta-auth-streamline.aps.md) | BAUTH     | Complete | 20/20    | —            |
| [docs-auth-gating](./archive/modules/docs-auth-gating.aps.md)         | DOCSAUTH  | Complete | 7/7      | BAUTH, IAC   |
| [admin-cli](./archive/modules/admin-cli.aps.md)                       | ADMINCLI  | Complete | 13/13    | BAUTH        |
| [admin-cli-hardening](./archive/modules/admin-cli-hardening.aps.md)   | ADMINCLIH | Complete | 4/4      | ADMINCLI     |
| [email-broadcast](./modules/email-broadcast.aps.md)                   | EMAIL     | In Progress | 10/10    | ADMINCLIH    |
| [github-cli-auth](./modules/github-cli-auth.aps.md)                   | GHCLIAUTH | In Progress | 2/11     | BAUTH, DOCSAUTH |

**Design specs:**

- `docs/archive/specs/2026-03-15-beta-auth-streamline-design.md` (archived 2026-05-23, DOCGOV-008)
- `plans/specs/2026-04-03-docs-auth-gating-design.md`
- `plans/specs/2026-04-16-admin-cli-design.md`

### Tracing Foundation

Cross-cutting runtime tracing baseline across `anvil-intercept` (Rust
daemon), `anvil-cli` (Rust), `anvil-api` (TS), and the dashboard ops
surface. Second trial of the cross-cutting module convention promoted to
APS under [ADR-034](./decisions/034-cross-cutting-modules-as-aps-primitive.md).
Pre-launch scope is **TRACE-001 + TRACE-004**: subscriber init, W3C
`traceparent` propagation, namespace registry stub, INTD-014 fixture update,
call-path instrumentation for the daemon / CLI paths shipped so far, and a
local hardened file sink. TRACE-002 is partially implemented as of 2026-05-25
(TS mirror package + `anvil-api` ingress) and blocked on a concrete dashboard
live-feed consumer for the joined-view smoke test. TRACE-003 has a partial Rust
tracing-formatter redaction slice and is blocked on INTD-015 / EXPORT-owned
policy parity and sampled-exporter behaviour. Kernel-surface breadth and production sink choice
remain post-launch / EXPORT follow-up scope.

| Module                                                          | Scope  | Status | Progress | Dependencies                                                                                                                                                                                                                  |
| --------------------------------------------------------------- | ------ | ------ | -------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| [tracing-foundation](./modules/tracing-foundation.aps.md)       | TRACE  | In Progress | 2/4      | INTD-014 (Committed); coordinates with RTAI, INTD-013, INTD-015, dashboard-ops-views, USAGE; cites ADR-019, ADR-034, ADR-035; TRACE-001 Complete 2026-04-30 (anvil-observability crate, init_tracing in both binaries, traceparent envelope round-trip, INTD-014 conformance assertion); TRACE-004 Complete 2026-05-11 via PR #1435 — call-path instrumentation + `traceparent` correlation fields + local hardened file sink; TRACE-002 partial 2026-05-25 (TS mirror package + `anvil-api` ingress) blocked on concrete dashboard live-feed consumer; TRACE-003 partial 2026-05-25 (Rust tracing-formatter redaction) blocked on INTD-015 / EXPORT-owned policy parity and sampled-exporter behaviour; OTLP/exporter-backed parent propagation and walkthrough deferred to EXPORT |
| [observability-export](./modules/observability-export.aps.md)   | EXPORT | Draft  | 0/1      | Blocks on TRACE-001/-002/-003; OQ1 (production sink choice — Tempo / Honeycomb / Grafana Cloud / self-hosted Jaeger / OTLP-to-Vercel-OTel) deferred until first paying customer or first production incident                  |

> **Precondition resolved 2026-04-30:** LAUNCH-003's open
> `Coordinates with: TUIDASH-009` callout was swept per ADR-034 rule 3.
> LAUNCH-003 shipped first; the conditional "Superseded by" branch did not
> fire. The named `WatchStats` contract is the inheritance TUIDASH-009 will
> consume when the dashboard surface lands. TRACE is now **In Progress** (TRACE-001 Complete 2026-04-30).

### Usage Analytics

Cross-cutting durable usage observations on Kindling — command invocations,
inline flag-context snapshots, dev-investment query views. Third trial of the
cross-cutting module convention promoted under
[ADR-034](./decisions/034-cross-cutting-modules-as-aps-primitive.md). Founder
request 2026-05-10 — answers "who is using what" durably so dev-investment
decisions are evidence-based. Per
[ADR-035](./decisions/035-three-pipe-observability-rule.md), usage facts are
governance-shaped (durable, queryable, source-of-truth) and live on Kindling,
not on the tracing pipe. USAGE-001 is the launch-blocker candidate (founder
lean 2026-05-10 → new `command.invoked` Kindling kind, with FLAGS
cross-clarification resolved by ADR-041); USAGE-002 (flag-context correlation)
and USAGE-003 (canned dev-investment query views) follow once invocations land.

| Module                                              | Scope | Status | Progress | Dependencies                                                                                                                                                                                                                |
| --------------------------------------------------- | ----- | ------ | -------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| [usage-analytics](./modules/usage-analytics.aps.md) | USAGE | Ready  | 0/3      | Kindling, TRACE-001 (consumes `TraceContext`); coordinates with TRACE-004 (incoming `traceparent` binding), FLAGCAT-007 / ADR-041 (resolved: inline `flag_set`, manifest `key` join, ADR-019 unchanged), TRACE-003 (shared `SENSITIVE_FIELDS` deny-list), OBS-001 (post-launch). Privacy contract + OQ2 anonymisation (hash + per-deployment salt) confirmed 2026-05-11. USAGE-001 promoted Ready 2026-05-30 (operator review; OQ1 observation-kind decision folded into the task); USAGE-002/003 stay Draft. |

### Infrastructure as Code

Pulumi-managed infrastructure: Vercel projects, Azure DNS, backend migration to
Azure Blob Storage + KeyVault. EDGE module (Azure Front Door multi-origin edge
layer) in flight per ADR-032.

| Module                                                                    | Scope | Status   | Progress | Dependencies                                                                                                                                       |
| ------------------------------------------------------------------------- | ----- | -------- | -------- | -------------------------------------------------------------------------------------------------------------------------------------------------- |
| [pulumi-iac](./archive/modules/pulumi-iac.aps.md)                         | IAC   | Complete | 20/20    | —                                                                                                                                                  |
| [database-consolidation](./archive/modules/database-consolidation.aps.md) | DBCON | Complete | 4/4      | IAC                                                                                                                                                |
| [edge](./modules/edge.aps.md)                                             | EDGE  | Ready    | 0/24     | IAC; coordinates with OBS (Log Analytics workspace), Vercel origins, and 8-week Azure-hosted origin commit. AFD Standard, Australia East. ADR-032. |

### Web Dashboard

Browser-based interface for exploring Anvil data. Built into `apps/website/`
(Next.js 16 + shadcn/ui + Recharts). Four execution waves; 39 tasks total.

| Module                                                                        | Scope    | Status | Progress | Wave | Dependencies                                                             |
| ----------------------------------------------------------------------------- | -------- | ------ | -------- | ---- | ------------------------------------------------------------------------ |
| [dashboard-foundation](./modules/dashboard-foundation.aps.md)                 | DASH     | Ready  | 1/9      | 1    | apps/website, contracts                                                  |
| [dashboard-core-views](./modules/dashboard-core-views.aps.md)                 | DASHCORE | Ready  | 0/9      | 2    | dashboard-foundation                                                     |
| [dashboard-architecture-views](./modules/dashboard-architecture-views.aps.md) | DASHARCH | Ready  | 0/8      | 2    | dashboard-foundation, architecture-safety, drift-reporting, suppressions |
| [dashboard-ops-views](./modules/dashboard-ops-views.aps.md)                   | DASHOPS  | Ready  | 0/7      | 3    | dashboard-foundation                                                     |
| [dashboard-ai-builder](./modules/dashboard-ai-builder.aps.md)                 | DASHAI   | Draft  | 0/6      | 4    | dashboard-foundation                                                     |

**Why Dashboard:** The CLI remains the primary developer interface; the
dashboard serves team leads, platform engineers, and compliance roles who need
persistent views, historical trends, and graphical visualisations that a
terminal cannot provide. See [brainstorm](./brainstorms/dashboard-web-ui.md) and
[json-render approach](./brainstorms/json-render-dashboard.md) for background.

### Policy Governance

Organisational policy governance: multi-level inheritance, lifecycle management,
compliance reporting, federation, and agent orchestration. Policy governance
tasks now reference Rust crates (anvil-kernel, anvil-policy, anvil-cli) as the
implementation targets.

| Module                                                                            | Scope   | Status   | Dependencies                                                                                                                                        |
| --------------------------------------------------------------------------------- | ------- | -------- | --------------------------------------------------------------------------------------------------------------------------------------------------- |
| [policy-engine](./archive/modules/policy-engine.aps.md)                                   | POLENG  | Complete | ADR-040 (Accepted 2026-05-13), crates/anvil-kernel, crates/anvil-policy — substrate for OPAE/ORGHIER/POLLC/COMPLY/POLFED/CPACKS; POLENG-001..009 Released/Shipped via v0.7.3-beta (skeleton PR #1485; engine substrate + `anvil policy eval` PR #1931, 2026-05-24; Go OPA parity gate PR #1942 PASS, 2026-05-25; engine hardening — determinism fence + resource bounds + findings-parse PR #1952, 2026-05-25 — ships preview-gated, output shape not yet a stable contract; all nine merge commits confirmed in the v0.7.3-beta tag). Module advanced to **Complete** 2026-06-08 per the v0.7.4-beta release-record post-tag note |
| [opa-enhancements](./modules/opa-enhancements.aps.md)                             | OPAE    | Draft    | opa-architecture-integration, crates/anvil-kernel, crates/anvil-tui                                                                                 |
| [org-policy-hierarchy](./modules/org-policy-hierarchy.aps.md)                     | ORGHIER | Draft    | opa-architecture-integration, policy-pack-validation, opa-enhancements, crates/anvil-policy                                                         |
| [policy-lifecycle](./modules/policy-lifecycle.aps.md)                             | POLLC   | Draft    | opa-architecture-integration, policy-pack-validation, org-policy-hierarchy, crates/anvil-policy                                                     |
| [compliance-reporting](./modules/compliance-reporting.aps.md)                     | COMPLY  | Draft    | org-policy-hierarchy, policy-lifecycle, drift-reporting, suppressions, crates/anvil-policy                                                          |
| [policy-federation](./modules/policy-federation.aps.md)                           | POLFED  | Draft    | opa-enhancements, org-policy-hierarchy, policy-lifecycle, policy-pack-validation, crates/anvil-policy                                               |
| [policy-pack-validation](./modules/policy-pack-validation.aps.md)                 | POLVAL  | Draft    | opa-architecture-integration, crates/anvil-policy                                                                                                   |
| [architecture-config-validation](./modules/architecture-config-validation.aps.md) | ARCHCFG | Draft    | opa-architecture-integration, architecture-safety, crates/anvil-kernel                                                                              |
| [ai-guardrail-profile](./archive/modules/ai-guardrail-profile.aps.md)                     | AIGUARD | Complete | crates/anvil-cli, crates/anvil-kernel-types, crates/anvil-kernel, crates/anvil-architecture, crates/anvil-checks, crates/anvil-policy; diagnostic envelope shared with RTAI/INTD/DRVR/RMCP |
| [opa-agent-orchestration](./modules/opa-agent-orchestration.aps.md)               | OPAG    | Ready    | opa-architecture-integration, opa-enhancements, architecture-safety, mcp-server                                                                     |
| [eval-harness-integration](./modules/eval-harness-integration.aps.md)             | EVAL    | Ready    | opa-enhancements, opa-agent-orchestration, drift-reporting                                                                                          |
| [compliance-evidence-workspace](./modules/compliance-evidence-workspace.aps.md)   | CEWS    | Draft    | compliance-reporting, policy-lifecycle, eval-harness-integration                                                                                    |
| [contextual-policy-assertions](./modules/contextual-policy-assertions.aps.md)     | CPOL    | Ready    | opa-enhancements, opa-agent-orchestration                                                                                                           |
| [io-risk-controls](./modules/io-risk-controls.aps.md)                             | IORISK  | Ready    | opa-enhancements, opa-agent-orchestration                                                                                                           |
| [gateway-control-plane-patterns](./modules/gateway-control-plane-patterns.aps.md) | GATE    | Ready    | opa-agent-orchestration, mcp-server                                                                                                                 |
| [adversarial-testing-catalog](./modules/adversarial-testing-catalog.aps.md)       | ATC     | Ready    | eval-harness-integration, opa-agent-orchestration                                                                                                   |
| [prompt-attack-regression-packs](./modules/prompt-attack-regression-packs.aps.md) | PATT    | Ready    | adversarial-testing-catalog, eval-harness-integration                                                                                               |
| [trust-center-automation](./modules/trust-center-automation.aps.md)               | TRUST   | Ready    | compliance-evidence-workspace, compliance-reporting                                                                                                 |
| [agent-governance-patterns](./modules/agent-governance-patterns.aps.md)           | AGOV    | Draft    | opa-enhancements, ember                                                                                                                             |
| [skill-discovery-observability](./modules/skill-discovery-observability.aps.md)   | SKOBS   | Draft    | AGOV (observability foundation for capability governance; AGOV-007 schema alignment)                                                                |
| [compliance-policy-packs](./modules/compliance-policy-packs.aps.md)               | CPACKS  | Draft    | opa-enhancements, policy-pack-validation                                                                                                            |
| [policy-action-taxonomy](./modules/policy-action-taxonomy.aps.md)                 | ACTAX   | Proposed | ADR-040, IORISK, AGOV, POLENG, CPOL (schema coordination) — action taxonomy + YAML policy DSL compiling to Rego; risk-score fusion into existing intercept routing                 |
| [policy-capability-discovery](./modules/policy-capability-discovery.aps.md)       | POLCAP  | Proposed | ACTAX-001, AGOV-007, IORISK, POLENG-001, INTD, MLP/MLP2 witness chain, DRVR; ADRs 001/002/037/040; pending Planning Council + ADR-051 — agent-facing signed capability view (`anvil policy capabilities`); advisory for planning, load-bearing for audit via cap_id binding to witness rows |
| [git-native-governance](./modules/git-native-governance.aps.md)                   | GITGOV  | In Progress | ADR-072/-073/-074 (Accepted 2026-06-08, full council); crates/anvil-witness (`WitnessLine`/`verify_chain_dag`), anvil-baseline, anvil-rules (`rules_sha`), anvil-policy (exceptions), anvil-cli SARIF (ADR-058) — Review Capsules wedge: file-first portable governance evidence, offline-verifiable. Decision gate cleared; capsule impl (GITGOV-003+) authorised; GITGOV-001/002 Done |
| [git-native-exceptions](./modules/git-native-exceptions.aps.md)                   | EXCEPT  | In Progress | ADR-073 (Accepted 2026-06-08, full council), crates/anvil-policy — move exceptions from gitignored `.anvil/exceptions.json` to tracked `anvil/exceptions/` so they travel with the repo + are PR-reviewable; sibling of `@anvil-ignore` (ADR-004). EXCEPT-001/002/003 Done; EXCEPT-007 write-path hardening gates the CLI write surface; CLI/L3-L4/capsule integration Proposed |

**Why Policy:** Builds on the single-repo OPA infrastructure from 0.1.0.
Requires multi-repo awareness, hierarchy resolution, and fleet-level aggregation
that only make sense after the core policy engine is battle-tested.

### Engineering Platform

Cross-cutting concerns that span all packages and releases. Promoted to Ready
when specific work is identified.

| Module                                                                                                | Scope      | Est. Tasks | Dependencies                                                                                                                                                                                                                                                                                                                                                                                                                                                   |
| ----------------------------------------------------------------------------------------------------- | ---------- | ---------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| [api-governance](./modules/api-governance.aps.md)                                                     | APGOV      | 7          | anvil-api (Hono), crates/anvil-cli — **Ready** (APGOV-001/-002/-003/-004/-005/-007 promoted Ready; APGOV-006 stays Draft — **needs design**: `/api/v1/health` already ships at `apps/anvil-api/src/index.ts:79` with a flat `{status,db,signingKey,verifyingKey}` shape; blocks on an owner call on (a) canonical response shape vs the original nested `checks:{}` draft and (b) the `/health` dependency-set vs OBS health-signal ownership)                                                                                                                                                                                                                                                                              |
| [feature-flagging](./archive/modules/feature-flagging.aps.md)                                         | FLAGS      | 9/9        | BAUTH, DOCSAUTH, OPAG, observability-foundation — **Complete**                                                                                                                                                                                                                                                                                                                                                                                                 |
| [feature-flag-migration](./archive/modules/feature-flag-migration.aps.md)                             | FLAGM      | 6/6        | FLAGS (complete), BAUTH, DOCSAUTH, RCLI — **Complete**                                                                                                                                                                                                                                                                                                                                                                                                         |
| [feature-flag-catalogue](./modules/feature-flag-catalogue.aps.md)                                     | FLAGCAT    | 8/9        | FLAGS (complete), FLAGM (complete); FLAGCAT-007 Complete via accepted ADR-041 (inline `flag_set`, manifest `key` join, ADR-019 unchanged; urgent authorised decision-only exception while module remains Draft); FLAGCAT-001 Complete via [`2026-05-18-feature-flag-catalogue-design.md`](./specs/2026-05-18-feature-flag-catalogue-design.md) pinning manifest layout, TS loader surface, Rust `build.rs` codegen, naming map, consistency check, and migration ordering; FLAGCAT-002..-006 promoted to Ready 2026-05-28 (release-freeze deferral spent — `v0.7.0-beta`..`v0.7.2-beta` shipped; FLAGCAT-004 Ready at `Confidence: low` with `build.rs`/sibling-crate fallback pinned in the design note); FLAGCAT-008 added 2026-05-21 — revisit `cli.licence-gate` membership (GH #1795), stays Draft pending planless-membership triage; FLAGCAT-002 In Progress 2026-06-01 (catalogue bootstrap + EnvironmentName rename + gating inventories) — **In Progress** |
| [check-language-and-onboarding](./archive/modules/check-language-and-onboarding.aps.md)               | CLAR       | 9/9        | discovery and alignment complete; `CLAR-006` -> `QLRUN-001`, `CLAR-007` -> `QLODX-001`, `CLAR-008` -> `QLODX-002` — **Complete**                                                                                                                                                                                                                                                                                                                               |
| [quality-language-runtime-alignment](./archive/modules/quality-language-runtime-alignment.aps.md)     | QLRUN      | 1/1        | CLAR (complete), rust-cli runtime/config surfaces — **Complete**                                                                                                                                                                                                                                                                                                                                                                                               |
| [quality-language-onboarding-and-docs](./archive/modules/quality-language-onboarding-and-docs.aps.md) | QLODX      | 2/2        | QLRUN, welcome/tutorial/docs surfaces — **Complete**                                                                                                                                                                                                                                                                                                                                                                                                           |
| [notification-framework](./archive/modules/notification-framework.aps.md)                             | NOTIFY     | 9/9        | CLAR, INTD, current CLI/TUI surfaces — **Complete** (doctor/audit alignment, shared TUI `NotificationSource`, telemetry contract, intercept integration spec)                                                                                                                                                                                                                                                                                                  |
| [command-safety-surfaces](./archive/modules/command-safety-surfaces.aps.md)                           | CMDSH      | 4/4        | CLAR, NOTIFY, INTD, anvil-checks command_safety — **Complete**                                                                                                                                                                                                                                                                                                                                                                                                 |
| [security](./modules/security.aps.md)                                                                 | SEC        | 2/9        | CI pipeline (`security.yml` Trivy/TruffleHog/Semgrep/license-check), cargo-deny advisories (`rust.yml`), dependabot — **In Progress** (SEC-007 token-revocation atomicity, GH #1672; SEC-008 named-pattern secret detection **Merged 2026-05-21 via PR #1815**, GH #1800; SEC-009 private docs entitlement gate, GH #1673, Done 2026-05-28; 2026-05-28 — SEC-001/-002/-003/-004 fleshed to Ready grounded in the as-built CI surface; SEC-005 security-headers stays **Proposed — needs APGOV↔SEC boundary call**; SEC-006 SBOM **deferred to SCA**, not duplicated) |
| [testing-strategy](./modules/testing-strategy.aps.md)                                                 | TEST       | 6          | eslint-plugin-anvil, e2e, Rust test suites                                                                                                                                                                                                                                                                                                                                                                                                                     |
| [release-management](./archive/modules/release-management.aps.md)                                     | RELMGMT    | 15/15      | CI pipeline, all packages and crates, DIST — **Complete**                                                                                                                                                                                                                                                                                                                                                                                                      |
| [operating-model-migration](./archive/modules/operating-model-migration.aps.md) | OPMODEL    | 12/12 (archived 2026-05-11) | Cross-cutting migration to the target Plan / Build / Release operating model — **Complete**. OPMODEL-001..-011 landed sequentially (see archived module for per-item detail). OPMODEL-012 completed the main-first cutover on 2026-05-11: `main` is now the only permanent product branch; `dev` retired as a dated compatibility branch (tag `dev-retired-2026-05-11`; deletion follow-up issue #1419 for on/after 2026-07-10); cutover SHA `b6f236e90dbc03338f17767202acf93f1449f8d2`; `pr-base-guard.yml` retired in PR #1417 (`62d85777`); `main` ruleset id 16217152 enforces 7 required checks + PR + non-FF + deletion. Module archived per `plans/aps-rules.md`. |
| [ci-cd-validation](./archive/modules/ci-cd-validation.aps.md)                                         | CICD       | 12/12 (archived 2026-05-12) | Specialist CI/CD + validation operating model (cost reporting, path/risk classifier, targeted gates, release-readiness reconciliation, drift checks, cutover readiness) — **Complete**, archived 2026-05-12. Per-item detail (CICD-001..-012) in the archived module. |
| [documentation-sync](./modules/documentation-sync.aps.md)                                             | DOCSYNC    | 11/16      | Public docs-site sync (`docs/public/anvil/`) — **In Progress** (Rust-migration phase 9/10 done; 5 Drafts remain — DOCSYNC-005 API reference, -011 Dashboard, -012 Policy governance, -013 Multi-language, -016 VSCode/CI warning divergence troubleshooting; 2026-05-22 scope sharpening dropped DOCSYNC-014 as superseded by DOCGOV-001 and reassigned -015/-017/-018/-019/-020 to DOCGOV-006 as internal-docs freshness; those absorbed notes are closed by DOCGOV-006)                                                                                                                                                                                                                                           |
| [documentation-governance](./archive/modules/documentation-governance.aps.md)                                 | DOCGOV     | 12/12      | APS-linked docs governance + agent closeout (docs-workflow, taxonomy, ADR integrity, `docs:check` / `docs:index`, metadata backfill) — **Complete**. Per-item detail (DOCGOV-001..-012) in the archived module. |
| [aps-canonical-alignment](./archive/modules/aps-canonical-alignment.aps.md)                           | APSCAN     | 11/11 (archived 2026-05-25) | Migration to canonical anvil-plan-spec v0.3.0 (Tasks → Work Items; Anvil lifecycle prose preserved) — **Complete**, archived 2026-05-25. Per-item detail (APSCAN-001..-011) in the archived module. |
| [schema-contracts](./modules/schema-contracts.aps.md)                                                 | SCHEMA     | 6          | anvil-core, anvil-kernel-types                                                                                                                                                                                                                                                                                                                                                                                                                                 |
| [git-config-hooks](./archive/modules/git-config-hooks.aps.md)                                         | GHOOK      | 6/6        | crates/anvil-cli, crates/anvil-tui, docs/public/anvil/, Git 2.54 hook API — **Complete** (GHOOK-001 baseline + rollout policy; GHOOK-002 `--config` install/uninstall landed; GHOOK-003 status/doctor/onboarding/tutorial detect config-mode entries; GHOOK-004 coexistence detection + duplicate-execution warnings; GHOOK-005 accepted **Option A — keep Husky** with dev runner on Git 2.51 as the decisive constraint; GHOOK-006 public docs sweep landed) |
| [eddacraft-tui-shared](./archive/modules/eddacraft-tui-shared.aps.md)                                 | TUIEXTRACT | 7/7        | eddacraft-tui, RATS (done) — **Complete**                                                                                                                                                                                                                                                                                                                                                                                                                      |
| [eddacraft-tui-canonical-source](./archive/modules/eddacraft-tui-canonical-source.aps.md)              | TUIMIRROR  | 0/8        | ADR-047 implementation plan — move `eddacraft-tui` canonical source back into Anvil, keep `eddacraft/eddacraft-tui` as a public read-only mirror, preserve crates.io as the external channel — **Superseded by TUIR; archived 2026-06-08 via TUIR-008** (0/8 — no work executed here; all implementation and history live under [tui-reintegration](./modules/tui-reintegration.aps.md))                                                                          |
| [tui-reintegration](./modules/tui-reintegration.aps.md)                                                | TUIR       | 10/10      | Supersedes TUIMIRROR; canonical eddacraft-tui source in crates/eddacraft-tui/, read-only mirror to eddacraft/eddacraft-tui, crates.io publish from here (ADR-047). TUIR-001..-007/-009/-010 Merged; TUIR-008 Done by operator evidence — legacy mirror `CARGO_REGISTRY_TOKEN` revoked and private `eddacraft/eddacraft-skills` `[patch.crates-io]` consumer check passed. Per-item detail in the module file. |
| [tui-next](./modules/tui-next.aps.md)                                                                  | TUIN       | 4/13       | Post-migration design deferred out of TUIR (parser policy, lifecycle ownership, runner-shell shape). TUIN-001/-011 docs Merged, TUIN-009 spike Done; TUIN-012 Done under operator override with feature-gated `lifecycle` + `runner` fallback CLI shell (`lexopt`, shared globals, first-level dispatch, config-path handoff, typed mode/theme hints; consumer-owned lifecycle/render-loop/domain semantics). Per-item detail in the module file. |
| [attribution-pipeline-v3](./archive/modules/attribution-pipeline-v3.aps.md)                                   | ATTRIB     | 15/16 (archived 2026-05-26) | tools/starters/acknowledgements/ kit + cargo-about + deny.toml — **Complete**, archived 2026-05-26 (anvil-code items shipped via v0.7.2-beta; ATTRIB-009 cross-repo; ATTRIB-005 rehomed to supply-chain-attestation). Full per-item history in the archived module. |
| [supply-chain-attestation](./modules/supply-chain-attestation.aps.md) | SCA | 0/3 | **Proposed** 2026-05-25 — home for the deferred ATTRIB-005 CycloneDX direction: SBOM generation (proper cyclonedx-* generators) + dependency mapping into the graph/witness layer + new-edges-only policy gating (L4) + SLSA/vuln. Gated on Anvil's graph layer ingesting a dependency graph; not Ready. Spawned from attribution-pipeline-v3 (ATTRIB-005 deferred here). |
| [acknowledgements-starter-releases](./modules/acknowledgements-starter-releases.aps.md) | ATTRIB | 1/1 | **Complete** — a deliberate semver-tag + GitHub-Release surface on the `eddacraft/acknowledgements-starter` mirror, layered on top of the unchanged rolling-`main` mirror (ATTRIB-011), so consumers get notified, read a changelog, and pin to an immutable version. Retains the ATTRIB lineage (ATTRIB-017) rather than re-opening archived attribution-pipeline-v3; modelled on the `eddacraft-tui` release flow. **ATTRIB-017 Merged 2026-06-08 via PR [#2418](https://github.com/eddacraft/anvil-001/pull/2418)** (release workflow + `check-version.sh` + kit self-test + runbook + consumer pinning docs; survived 3-lens Council + Copilot review). First cut **`v1.0.0`** shipped 2026-06-08 (release run 27128030923) — mirror tag + GitHub Release (latest) live, round-trip pin verified. Spec at [`plans/specs/2026-06-08-acknowledgements-starter-releases.md`](./specs/2026-06-08-acknowledgements-starter-releases.md); actions at [`plans/execution/ATTRIB-017.actions.md`](./execution/ATTRIB-017.actions.md). |
| [sarif-output](./modules/sarif-output.aps.md) | SARIFOUT | 6/6 | **Complete** — additive `--format sarif` on `anvil check`/`gate`/`audit`, promoted from CIB-014 after the [2026-05-29 design pass](./specs/2026-05-29-sarif-output-design.md). The three decisions (flag surface, module home, shared model) were **ratified 2026-05-29** ([ADR-056](./decisions/056-format-flag-output-selector.md) + [ADR-058](./decisions/058-sarif-shared-emitter-no-finding-model.md), both Accepted). Flag surface landed **per-command on check/gate/audit, not global** — `--format` already collides with `export`/`validate`'s domain flags; `--json` stays the global alias. Pinned to the GitHub Code Scanning subset of SARIF 2.1.0 (results/rules/locations/suppressions). All six work items Merged (SARIFOUT-001 via PR #2099; -002 #2105; -003 #2107; -004 #2112; -005 #2115; -006 #2116); Released/Shipped in v0.7.3-beta (tag 8bfd48c4d, 2026-05-31) — Complete. |

### Test Quality

CI infrastructure repair, coverage uplift to ≥80% for targeted packages/crates,
integration boundary testing, and external service contract tests. Implements
the strategy defined in TEST (Engineering Platform). TFIX is the prerequisite;
TCOV/TINT/TEXT depend on it.

| Module                                                                      | Scope | Status      | Progress                                                                                   | Dependencies            |
| --------------------------------------------------------------------------- | ----- | ----------- | ------------------------------------------------------------------------------------------ | ----------------------- |
| [test-infrastructure-fix](./archive/modules/test-infrastructure-fix.aps.md) | TFIX  | Complete    | 11/11                                                                                      | —                       |
| [test-coverage-uplift](./modules/test-coverage-uplift.aps.md)               | TCOV  | In Progress | 14/25 (Phase 1+2 complete: 13/13; Phase 3: 1 done + TCOV-015..-018 Ready, TCOV-019..-021 Blocked — needs decision: mcp-server archived under ADR-033 + excluded from workspace; Phase 4: TCOV-025 Ready, TCOV-022..-024 Blocked — scope drift, needs design refresh against the `eddacraft-tui` re-export + two-widget layout) | TFIX                    |
| [test-integration-surface](./modules/test-integration-surface.aps.md)       | TINT  | Proposed    | 0/15 (work items given Status fields 2026-05-28; Phases 1–2 TINT-001..-004 / -006..-011 individually **Ready** — grounded in the live `apps/e2e/` harness + `anvil-tui` insta snapshots; module stays Proposed because Phase 3 TINT-012..-015 needs re-scope vs the now-shipped intercept daemon; TINT-005 closed **Superseded** 2026-06-01 — ratified the shipped graceful-skip e2e CI design rather than adding a binary-build step) | TFIX, partial RCLI/KERN |
| [test-external-services](./modules/test-external-services.aps.md)           | TEXT  | Draft       | 0/14                                                                                       | TFIX                    |

### Language & Coverage

Coverage strategy is defined by the
[2026-04-08 Language and Coverage Design](./specs/2026-04-08-language-and-coverage-design.md)
(refreshed 2026-05-14). The flat "ten languages" placeholder list has been
replaced with **five parallel tracks**, ranked against demand × blast radius ×
strategic fit per spec §6. The original `lang-*.aps.md` placeholders for Dart,
Go, Java, Kotlin, .NET, C/C++, Swift, Zig have been **archived** now that their
content is folded into the new modules; `lang-rust.aps.md` and
`lang-python.aps.md` have been **rewritten in place** as Track 1 anchors.

This section is the canonical APS definition for the next Language & Coverage
target set. Treat the five tracks as a cross-cutting module family under
[ADR-034](./decisions/034-cross-cutting-modules-as-aps-primitive.md) and
[`plans/project-context.md#cross-cutting-modules`](./project-context.md#cross-cutting-modules)
(with the legacy forwarding anchor at
[`plans/aps-rules.md#cross-cutting-modules`](./aps-rules.md#cross-cutting-modules)):
each track module owns and counts its own work items, while cross-track
coordination uses prose callouts (`Coordinates with:`, `Blocks on:`,
`Supersedes:`, `Superseded by:`) that must be swept when tasks close. OPSUP owns
shared operational prerequisites for Track 3 surfaces and Track 4 packs; it does
not duplicate their rule-catalogue work.

**Next target set:** Phase 1 stays the first cut unless re-scored:
`LANGTS` (complete 6/6), `RSTLAN` (In Progress 8/8, all items merged — pending release tag), `SURFSQL`, `PACKPUL`, and `PACKLLM`, with the
needed OPSUP slices and FLAGCAT catalogue-bootstrap slice completed first or
cited as `Blocks on:` callouts in the owning tasks. Modules still marked
`Proposed` must be promoted to `Ready` with executable tasks before
implementation is authorised.

- **Phase 1 (MVP + Rust dogfood)**: TS audit + Rust → T3 + SQL migrations T2 +
  Pulumi pack + LLM Provider pack (warn-only). Spec §9 steps 1–5 after the
  2026-05-14 Rust reprioritisation.
- **Phase 2** (named deliverables complete): GH Actions T2, Drizzle pack, tail
  T1 wave, Python → T3, Python-substrate LLM Provider, Next.js, Hono, Tokio
  packs, Markdown M1. Spec §9 steps 6–14 after removing Rust from later-phase
  scope.
- **Phase 3 / open-ended**: remaining surfaces (Dockerfile, shell, `.env`),
  remaining packs (Django, FastAPI, Axum). Demand-pulled.
- **Cut entirely** (spec §13): Swift, Zig, Express, NestJS, Flask, Spring,
  Rails, tRPC, CloudFormation, Bicep, Ansible, Jenkins Groovy, Buildkite,
  CircleCI.

#### Track 1 — Anchors (TS, Rust, Python → T3)

Heavy, sequenced. TS audit produces the T3 acceptance checklist that Rust and
Python must hit. Spec §7, §8.1.

| Module                                          | Scope  | Status | Phase | Spec ref                                                                                                                                                   |
| ----------------------------------------------- | ------ | ------ | ----- | ---------------------------------------------------------------------------------------------------------------------------------------------------------- |
| [lang-ts-audit](./archive/modules/lang-ts-audit.aps.md) | LANGTS | Complete | 1     | §7.3, §8.1 — 6/6 (**Complete 2026-06-08** — LANGTS-002/-004/-005/-006 merge commits confirmed in the v0.7.3-beta tag, advanced to Released/Shipped; -001/-003 are Done audit/checklist artefacts); promoted to Ready 2026-04-26 after anchor re-scoring gate (TS still anchor zero; Rust catching up — flagged for separate RSTLAN re-eval); LANGTS-006 dynamic-eval antipattern Merged 2026-05-21 via PR #1820 `bcb96175` (AP-008 + AP-009 in new `dynamic-execution` family; `Function.prototype.constructor` deferred pending AST-aware filter); 2026-05-28 — two bounded OQs resolved inline (single module, no `lang-ts-prereq` split; K1 extractor-trait ADR deferred to RSTLAN per audit §8), so LANGTS-002/-004/-005 promoted from anticipated bullets to Ready work items; LANGTS-005 kernel-prereq refactor (K1–K4: extractor trait, grammar-versioned cache key, per-worker parser, non-panicking parse path) Merged 2026-05-29 via PR #2096 — unblocks the RSTLAN extractor wiring; LANGTS-002 TS extraction gaps (TS-G1 interface/type/enum + TS-G2 class-method symbols) Merged 2026-05-29 via PR #2106, advancing to 5/6; LANGTS-004 Zod-creep rules Merged 2026-05-30 via PR #2125 (AP-015 `z.any()`/`.passthrough()` on by default + AP-016 `z.unknown()` opt-in; renumbered off the retired AP-010..AP-013 range), advancing to 6/6 |
| [lang-rust](./modules/lang-rust.aps.md)         | RSTLAN | In Progress | 1     | §8.1 — RSTLAN-001/-002 (#2303) + -004 (#2319) + -005 (Rust boundary enforcement, #2321) Merged; RSTLAN-003 (AST antipattern catalogue — new gate-time `anvil-checks-ast` crate per ADR-071, `rust-reliability` RS-001..005), -007 (`architecture-validate` surface for Rust), and -008 (T3 dogfood: 571 files, 0 panics/parse-skips, 0% FP) Merged 2026-06-05 via PR #2329. Rust passes the T3 checklist + §16.5 #9 FP bar. RSTLAN-006 (`.rs` in default antipattern/drift scan set) Merged 2026-06-04 via PR #2324, reconciled 2026-06-07 — all 8 items now Merged (8/8), module In Progress pending release tag. `.clone()`-hot-loop + serde flatten/secret-field deferred to RSTLAN-003b. NBI re-eval complete 2026-06-03; ADR-065 (Rust-native) Accepted. Owner @aneki. (8/8) |
| [lang-python](./modules/lang-python.aps.md)     | PYLAN  | Draft  | 2     | §8.1                                                                                                                                                       |

#### Track 2 — Tail T1 wave (single batched sprint)

Bring tail languages to T1 (parsed + symbol graph inclusion) in one sprint.
Replaces the six per-language placeholder modules (now archived).

| Module                                            | Scope    | Status | Phase | Languages                                                             |
| ------------------------------------------------- | -------- | ------ | ----- | --------------------------------------------------------------------- |
| [lang-tail-wave](./modules/lang-tail-wave.aps.md) | LANGTAIL | Draft  | 2     | Dart, Go, Java, Kotlin, .NET/C#, C/C++ (C/C++ at-risk per spec §12.3) |

**Archived placeholder modules** (content folded into `lang-tail-wave`):
[lang-dart](./archive/modules/lang-dart.aps.md),
[lang-go](./archive/modules/lang-go.aps.md),
[lang-java](./archive/modules/lang-java.aps.md),
[lang-kotlin](./archive/modules/lang-kotlin.aps.md),
[lang-dotnet](./archive/modules/lang-dotnet.aps.md),
[lang-c-cpp](./archive/modules/lang-c-cpp.aps.md).

**Cut entirely** (spec §13, no demand):
[lang-swift](./archive/modules/lang-swift.aps.md),
[lang-zig](./archive/modules/lang-zig.aps.md). Re-enter only with a demand
signal.

#### Track 3 — Governance surfaces (pattern catalogues)

Pattern-catalogue work, not parser work. Surfaces ranked by demand × blast
radius × strategic per spec §8.3.

| Module                                                            | Scope    | Surface             | Target tier | Status      | Phase |
| ----------------------------------------------------------------- | -------- | ------------------- | ----------- | ----------- | ----- |
| [surface-sql-migrations](./modules/surface-sql-migrations.aps.md) | SURFSQL  | SQL migrations      | T2          | Draft       | 1     |
| [surface-github-actions](./modules/surface-github-actions.aps.md) | SURFGHA  | GitHub Actions YAML | T2          | Draft       | 2     |
| [surface-dockerfile](./modules/surface-dockerfile.aps.md)         | SURFDOCK | Dockerfile          | T2          | Draft       | 3     |
| [surface-shell](./modules/surface-shell.aps.md)                   | SURFSH   | Shell scripts       | T1          | Draft       | 3     |
| [surface-env-files](./archive/modules/surface-env-files.aps.md)   | SURFENV  | `.env` files        | T1          | Complete    | 6     |

Mostly deferred: Terraform / HCL (T1, demand=1 indirect via Pulumi), k8s YAML /
Helm (T1, no demand) — promotion gated on direct user demand.

#### Track 4 — Semantic packs (substrate-gated)

Domain-specific packs layered on anchor languages. Each pack declares its
substrate language and minimum substrate tier per spec §8.4.

| Module                                                  | Scope   | Substrate       | Min substrate tier     | Status                                 | Phase               |
| ------------------------------------------------------- | ------- | --------------- | ---------------------- | -------------------------------------- | ------------------- |
| [pack-pulumi](./modules/pack-pulumi.aps.md)             | PACKPUL | TS              | T3                     | Draft                                  | 1                   |
| [pack-llm-provider](./modules/pack-llm-provider.aps.md) | PACKLLM | TS, then Python | T3 (TS) → T2+ (Python) | Draft (warn-only by default per C-010) | 1 (TS) + 2 (Python) |
| [pack-drizzle](./modules/pack-drizzle.aps.md)           | PACKDRZ | TS              | T3                     | Draft                                  | 2                   |
| [pack-nextjs](./modules/pack-nextjs.aps.md)             | PACKNXT | TS              | T3                     | Draft                                  | 2                   |
| [pack-hono](./modules/pack-hono.aps.md)                 | PACKHON | TS              | T3                     | Draft                                  | 2                   |
| [pack-tokio](./modules/pack-tokio.aps.md)               | PACKTOK | Rust            | T2+                    | Draft                                  | 2                   |

**Phase 3 / open-ended packs** (spec §17.3 final paragraph): Django, FastAPI,
Axum — module files created only when promoted from Phase 3 to active work.
Django/FastAPI gated on User C's framework choice resolving.

#### Track 5 — Markdown governance

Markdown is its own track because it fits none of the other axes. Initial target
M1 = APS wellformedness + cross-reference integrity (spec §8.5). M2 (stale claim
detection) and M3 (capability-aware) queue for later.

| Module                                                      | Scope | Tier target | Status | Phase |
| ----------------------------------------------------------- | ----- | ----------- | ------ | ----- |
| [markdown-governance](./modules/markdown-governance.aps.md) | MDGOV | M1          | Draft  | 2     |

Crate assignment per [ADR-028](./decisions/028-markdown-governance-crate.md):
standalone Rust crate `crates/anvil-markdown-governance/` using `pulldown-cmark`
— **not** the Rust kernel.

#### Cross-track infrastructure

One module owns the operational concerns every Track 3/4 module needs. Without
it, each new module would re-design the same plumbing.

| Module                                                            | Scope | Status | Notes                                                                                                                                                                                                                                                                                          |
| ----------------------------------------------------------------- | ----- | ------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| [operational-supplement](./modules/operational-supplement.aps.md) | OPSUP | In Progress | 2/7 — OPSUP-001 check-ID registry slice complete; OPSUP-006 file-presence + wall-time framework complete; OPSUP-002/-003/-004/-005 Ready (fleshed to work-item detail 2026-05-28); OPSUP-007 Draft (FP-reporting telemetry destination unresolved). Stable check-ID registry building on `check_catalog.rs`, drift schema versioning + `anvil drift migrate`, per-track feature flags, CI wall-time budget + file-presence guards, FP reporting channel. Council §16.5 #7. Delivered in slices — surfaces can move to Ready against partial OPSUP. |

#### Supporting decisions

| ADR                                                        | Decision                                                                                      | Status   | Gates                       |
| ---------------------------------------------------------- | --------------------------------------------------------------------------------------------- | -------- | --------------------------- |
| [ADR-027](./decisions/027-pack-architecture.md)            | Per-pack crate, symbol-graph access, compiled-in activation                                   | Accepted | All Track 4 packs           |
| [ADR-028](./decisions/028-markdown-governance-crate.md)    | Standalone Rust crate `crates/anvil-markdown-governance/` with `pulldown-cmark`               | Accepted | MDGOV                       |
| [ADR-029](./decisions/029-suppression-parser-authority.md) | Rust suppression parser is authoritative for new surfaces; no new comment styles in TS parser | Accepted | All Track 3 surfaces, MDGOV |

#### Supporting process

- [Anchor re-scoring process](../docs/guides/anchor-rescoring-process.md) — gate
  run before each Track 1 anchor module starts. Required by council §16.5 #8.
  Permanent owner not yet named (each invocation names a session owner).

#### Reconciliation status (spec §17.3)

| #   | Action                                                            | Status                            |
| --- | ----------------------------------------------------------------- | --------------------------------- |
| 1   | Archive `lang-swift.aps.md`, `lang-zig.aps.md` (cut)              | ✅ Done                           |
| 2   | Merge six tail languages into `lang-tail-wave.aps.md`             | ✅ Done (placeholders archived)   |
| 3   | Rewrite `lang-rust.aps.md` for T3 (incorporates §16.5 #3, #5, #8) | ✅ Done (RSTLAN module rewritten) |
| 4   | Rewrite `lang-python.aps.md` for T3                               | ✅ Done (PYLAN module rewritten)  |
| 5   | Create five surface modules (Phase 1 priority: SURFSQL)           | ✅ Done                           |
| 6   | Create six pack modules (Phase 1 priority: PACKPUL, PACKLLM)      | ✅ Done                           |
| 7   | Create `markdown-governance.aps.md`                               | ✅ Done                           |
| 8   | Replace Multi-Language section in `index.aps.md`                  | ✅ Done                           |

#### Outstanding council §16.5 items

| Item                                                                                                                                                | Status                                                                           |
| --------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------- |
| §16.5 #3 — kernel prerequisite work (extractor refactor, grammar version in cache key, parser thread-safety, panic removal, grammar maturity audit) | Captured in LANGTS Ready Checklist; needs implementation                         |
| §16.5 #4 — pack architecture                                                                                                                        | ✅ ADR-027 (Accepted)                                                            |
| §16.5 #5 — Rust T3 architecture enforcement location                                                                                                | ✅ ADR-065 Accepted 2026-06-03 (Rust-native in anvil-architecture + kernel edges); RSTLAN promoted Ready; captured in lang-rust.aps.md Ready Checklist and work items. |
| §16.5 #7 — operational supplement                                                                                                                   | ✅ OPSUP module created                                                          |
| §16.5 #8 — anchor re-scoring process gate                                                                                                           | ✅ Process guide created; permanent owner still open                             |
| §16.5 #9 — acceptance bar revision (FP rate < N% AND ≥1 external codebase)                                                                          | Captured in each module's Ready Checklist; canonical wording not yet centralised |
| §16.5 #10 — Markdown M1 acceptance softening                                                                                                        | Captured inline in MDGOV                                                         |
| §16.5 #11 — Markdown crate assignment                                                                                                               | ✅ ADR-028 (Accepted)                                                            |
| §16.5 #12 — parallelism-is-logical-dependency clarification                                                                                         | Inline in spec §9; track modules inherit                                         |
| Council C-025 — suppression parser authority                                                                                                        | ✅ ADR-029 (Accepted)                                                            |

### Rust MCP Launch Path

Current-release Rust MCP launch shim plus next-release full parity port. The
current release ships only the narrow A1 path: `anvil mcp install` writes client
config, clients launch `anvil mcp serve --stdio`, and the Rust server validates
proposed writes before they land. Full TS MCP server parity is next-release work.

| Module | Scope | Status | Progress | Dependencies |
| ------ | ----- | ------ | -------- | ------------ |
| [rust-mcp-launch-shim](./archive/modules/rust-mcp-launch-shim.aps.md) | RMCP | Complete | 8/8 (A1 launch slice closed 2026-04-30 — RMCP-001..-008 shipped; RMCP-008 GUI dry-run recorded in `plans/specs/2026-04-26-rtai-demo-runbook.md` §8; follow-up gaps tracked as #1194/#1195/#1197) | RCLI3-016/-016b, RTAI, AIGUARD-002, anvil-checks; daemon preferred but embedded fallback allowed |
| [rust-mcp-full-port](./modules/rust-mcp-full-port.aps.md) | RMCPF | In Progress | 6/10 (RMCPF-001 inventory, RMCPF-002 architecture spec, RMCPF-003 Phase 1 readiness decisions, and RMCPF-010 check/gate/status MCP tool parity slice Complete; `anvil_check` ships as the daemon-RPC translator's correctness-equivalent embedded fallback and `anvil_gate` ships as MCP-driver-local composition with planless in-process and full subprocess modes. RMCPF-011 (fix/suppress/boundary tools) and RMCPF-012 (prompts retired) shipped via PR #1558 (merged 2026-05-14, commit `56d5fd89`); registry now exposes seven tools, `prompts` capability omitted, `prompts/list` returns -32601.) | RMCP, DRVR, `archive/anvil-mcp-server` (archived per ADR-033 — frozen reference) |

### Future

| Module | Scope | Description | Status |
| ------ | ----- | ----------- | ------ |
| [open-spec-adapter](./modules/open-spec-adapter.aps.md) | OPENSPEC | Parse open-spec format as planning source | Draft |
| ~~real-time-validation-simplified~~ | ~~RTVS~~ | Superseded 2026-04-24 by LAUNCH (watch polish) + RTAI (validation core, originally pointed at RTVF before RTVF itself was superseded); spec was written against retired Ink/TS stack — [archived](./archive/modules/real-time-validation-simplified.aps.md) | Superseded |
| ~~real-time-validation-full~~ | ~~RTVF~~ | Superseded 2026-04-24 by RTAI (in-flight validation against daemon + drivers), DRVR (per-surface integration), NOTIFY (notification channels); RTVF's "unified validation server" framing pre-dated ADR-030 — [archived](./archive/modules/real-time-validation-full.aps.md) | Superseded |
| [pocketflow-gateway](./modules/pocketflow-gateway.aps.md) | PFGW | Gateway integration with pocketflow | Draft |
| [early-access-migration](./modules/early-access-migration.aps.md) | EAMIG | Early access migration tooling | In Progress |
| [early-access-tests](./modules/early-access-tests.aps.md) | EATEST | Early access test infrastructure (6/38 complete) | In Progress |
| [intent-ledger-governance](./modules/intent-ledger-governance.aps.md) | ILGOV | Intent ledger governance model | Ready |
| [lineage-authorship-confidence](./modules/lineage-authorship-confidence.aps.md) | LAC | Lineage and authorship confidence tracking | Ready |
| [unified-config-format](./modules/unified-config-format.aps.md) | UCFG | Unified configuration format across surfaces | Proposed |

### Dormant: Not Yet Scheduled

Module families with no active (`Ready` / `In Progress`) work — all `Draft`,
`Proposed`, or `Blocked` — plus completed/archived pointers kept for
navigation. Parked below the active sections so the index leads with current
work; promote a family back up when it gains scheduled, executable work.

### Dev Tooling Bridge

Connect the LLM-powered council review flow to Anvil's deterministic attestation
format. Discovery-first: understand the interface before building.

| Module                                                                          | Scope | Status   | Progress | Dependencies |
| ------------------------------------------------------------------------------- | ----- | -------- | -------- | ------------ |
| [council-gate-bridge](./modules/council-gate-bridge.aps.md)                     | CGBDG | Blocked  | 0/6      | MLP-002      |
| [clawpatch-techniques-adoption](./modules/clawpatch-techniques-adoption.aps.md) | CPTA  | Proposed | 0/7      | CGBDG (sibling — overlap check via CPTA-001) |

### Observability Foundation

Domain ops: telemetry contracts, Neon health instrumentation, dashboard ops
data contract, alert thresholds, runbook pack. 5 tasks (post-launch
hardening). The cross-cutting tracing baseline originally scoped as OBS-006
moved to TRACE on 2026-04-30 per Planning Council session plan-b00c16c7;
see [ADR-035](./decisions/035-three-pipe-observability-rule.md) for the
three-pipe rule and [Tracing Foundation](#tracing-foundation) below.

| Module                                                                | Scope | Status | Progress | Dependencies                                                                                                                  |
| --------------------------------------------------------------------- | ----- | ------ | -------- | ----------------------------------------------------------------------------------------------------------------------------- |
| [observability-foundation](./modules/observability-foundation.aps.md) | OBS   | Proposed | 0/5    | DASHOPS (live-feed consumer), TRACE (namespace/redaction surface), `apps/anvil-api` (hosted health signals); archived kindling-integration re-sourced to live kernel/CLI emitters 2026-05-28. Post-launch domain-ops hardening; OBS-001..005 fleshed (Status/Files/Deps/Confidence) but module stays **Proposed** — not in a release wave, DASHOPS not started, OBS-003 live-feed premise conflicts with DASHOPS deferred-SSE scope. Tracing scope migrated to TRACE 2026-04-30 (OBS-006 → TRACE-001). |

### Config Intelligence

Extract dependency graphs and project structure from config files (package.json,
Cargo.toml, go.mod, tsconfig.json, etc.) without language-specific analysers.
Feeds the architecture edge detector with dependency graph data.

| Module                                                      | Scope  | Est. Tasks | Dependencies        |
| ----------------------------------------------------------- | ------ | ---------- | ------------------- |
| [config-intelligence](./modules/config-intelligence.aps.md) | CFGINT | 7          | architecture-safety |

### Agent Infrastructure

Thin, provider-agnostic agent runtime (weave, Apache-2.0) in standalone repo
(`eddacraft/weave-rs`) plus Anvil-specific harness (anvil-weave) with zero-copy
semantic graph access.

**Implementation state:** No `literate-core` or `anvil-agent` crates exist in
this repo. The upstream runtime lives at `~/Projects/src/weave-rs` (see memory:
reference_weave_rs). This module is a greenfield import plus harness build —
schedule after the intercept-loop thesis is proven.

| Module                          | Scope           | Status | Progress | Dependencies            |
| ------------------------------- | --------------- | ------ | -------- | ----------------------- |
| [weave](./modules/weave.aps.md) | WEAVE, AHARNESS | Draft  | 0/21     | KERN (anvil-weave only) |

**Architecture Decision:**
[D-024: Internal Agent Harness](./decisions/024-internal-agent-harness.md)

### Edda Stack — Memory System

Kindling (observation), Ember (interpretation), Edda (canonical memory),
integration layer, and review backlog.

See [completed-index.aps.md](./completed-index.aps.md) for task tables.

### Branch Recovery

Reconcile divergent `main`/`dev` histories by porting release-critical fixes
from `main` onto `dev`, validating as one integrated branch, then cutting over.
See `docs/runbooks/branch-reconciliation.md` and the freeze notice in
`RECONCILIATION-IN-PROGRESS.md`.

| Module                                                                  | Scope  | Status   | Progress |
| ----------------------------------------------------------------------- | ------ | -------- | -------- |
| [branch-reconciliation](./archive/modules/branch-reconciliation.aps.md) | BRECON | Complete | 14/14    |

### What's NOT in Scope (Yet)

- **Plan/APS execution** — Planless-first; APS is internal
- **Auto-fix** — Warnings only; don't be too clever

## Constraints

- Must deliver value **without requiring plans/APS** as a prerequisite
  (planless-first)
- Must not hard-block by default — warnings, not errors
- Must run on Node.js 20+
- Must integrate with existing linting/formatting tooling, not replace it
- Must acknowledge legacy drift without overwhelming developers with noise

## System Map

```mermaid
graph TD
    subgraph "Developer Flow"
        SAVE[File Save] --> RUNNER[Analysis Runner]
        RUNNER --> ARCH[Architecture Check]
        RUNNER --> ANTI[Anti-pattern Check]
        ARCH --> WARN[Warning Aggregator]
        ANTI --> WARN
    end

    subgraph "Feedback Channels"
        WARN --> IDE[IDE Diagnostics]
        WARN --> CLI[CLI Output]
        WARN --> CI[PR/CI Mirror]
    end

    subgraph "Accountability"
        WARN --> SUPP[Suppression Store]
        SUPP --> DRIFT[Drift Reports]
        ARCH --> DRIFT
    end

    BASELINE[(Architecture Baseline)] --> ARCH
    PATTERNS[(Anti-pattern Library)] --> ANTI
```

## Milestones

All milestones complete. See [completed-index.aps.md](./completed-index.aps.md).

## Modules

Active module tables live in the [Release Plan](#release-plan) above.
Completed modules are archived in
[completed-index.aps.md](./completed-index.aps.md). Per-task detail for any
module lives in that module's own `.aps.md` file — this index does not duplicate
it.

### Superseded

> ~~tui-enhancement~~ (TUIENH) — see D-005: Ink over OpenTUI, then ADR-011:
> Ratatui replaces Ink.

> ~~interactive-tutorial~~ (TUTOR) — absorbed into
> [WELCOME](./archive/modules/restore-welcome-screen.aps.md) (18/18 complete).
> All 13 TUTOR items mapped to WELCOME phases. See
> [archived plan](./archive/modules/interactive-tutorial.aps.md).

> ~~continuous-improvement~~ (CI) — retired 2026-04-18; meta-module without
> executable tasks. All concrete intents roll into MAINT.

## Risks & Mitigations

| Risk                              | Impact     | Likelihood | Mitigation                                                                  |
| --------------------------------- | ---------- | ---------- | --------------------------------------------------------------------------- |
| Warning noise kills adoption      | high       | medium     | High-signal patterns only; warn on NEW edges, not legacy                    |
| Analysis too slow (> 2s)          | high       | medium     | Incremental analysis; hash-based caching; warm daemon                       |
| Developers bypass with `--skip`   | medium     | medium     | Track skip usage; surface in drift reports                                  |
| Legacy drift overwhelms users     | medium     | high       | Baseline existing violations; focus warnings on new code                    |
| Over-claiming blast radius        | medium     | medium     | Careful language; surface confidence levels                                 |
| ~~Forge loops slow down commits~~ | ~~high~~   | ~~medium~~ | ~~Archived — Forge/Temper replaced by Council~~                             |
| ~~Temper creates bad fixes~~      | ~~high~~   | ~~low~~    | ~~Archived — Temper removed~~                                               |
| ~~Deferred findings pile up~~     | ~~medium~~ | ~~medium~~ | ~~Archived — Forge/Temper replaced by Council~~                             |
| ~~Bot review wars in CI~~         | ~~medium~~ | ~~low~~    | ~~Archived — Temper removed~~                                               |
| PGID TOCTOU race in intercept     | high       | medium     | Verify PGID ownership before signalling; fence on failure (D-015 AD-7)      |
| Intercept v1 scope creep          | medium     | medium     | Strict out-of-scope list; binary allow/interrupt; no driver framework in v1 |
| Shell wrapper bypass              | medium     | medium     | Hook side-channel + fence-on-unknown fallback (D-015 AD-2)                  |
| Secret content via `notification.context` (TRACE R1) | medium | low | Risk **accepted pre-launch** (Planning Council session plan-b00c16c7); revisit when INTD-015 reaches Ready OR first secret-detection rule ships, whichever first; TRACE-003 is the tracing-pipe side of the mitigation |
| `anvil.<domain>.*` namespace fragmentation (TRACE R2) | medium | medium | Namespace registry doc (TRACE-001 stub at `docs/observability/namespace-registry.md`) + founder-reviewed PR-to-add gate; ADR-035 governs pipe allocation |
| Dashboard cannot join traces day one (TRACE R3) | low | high | Documented in Known Gaps section of namespace registry; closes when TRACE-002 lands the TS-side `traceparent` parser |

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
- **D-008:** Ink vs Ratatui Assessment — evaluated both for Anvil TUI; Ratatui
  adopted with ADR-011 ([ADR](./decisions/008-ink-vs-ratatui-assessment.md)) —
  **Superseded**
- **D-009:** Ink vs Ratatui Watch Mode Performance — benchmark analysis of Ink
  vs Ratatui for watch dashboard rendering
  ([ADR](./decisions/009-ink-vs-ratatui-watch-mode-performance.md)) —
  **Superseded**
- **D-010:** Pulumi TypeScript IaC — TypeScript-first Pulumi with Azure backend
  ([ADR](./decisions/010-pulumi-typescript-iac.md))
- **D-011:** OPA Agent Orchestration — orchestration layer for checkpointed
  policy evaluation, remediation guidance, and auditable exception workflows
  ([ADR](./decisions/022-opa-agent-orchestration.md))
- **D-011a:** Rust Core Engine — Rust for performance-critical subsystems
  (engine, watcher, storage, TUI) while TypeScript CLI stays; gated on Phase 0
  spike ([ADR](./decisions/011a-rust-core-engine.md)) — **Proposed**
- **D-012:** Eval Harness Adoption — adopt external eval framework behind Anvil
  adapter contracts for CI-native trust regression testing
  ([ADR](./decisions/013-eval-harness-adoption.md))
- **D-015:** Intercept Loop Enforcement — driver-based host-local enforcement
  daemon with process-group control, configurable enforcement policy, and fence
  persistence ([ADR](./decisions/015-intercept-loop-enforcement.md))
- **D-034:** Cross-cutting modules as APS primitive — promoted from LAUNCH's
  local convention block to a normative `## Cross-Cutting Modules` section in
  `aps-rules.md`; LAUNCH (first trial), TRACE (second trial), and USAGE
  (third trial, founder-requested 2026-05-10) cite by anchor; `Blocks on:`
  callout type carried as provisional until exercised through a real close
  ([ADR](./decisions/034-cross-cutting-modules-as-aps-primitive.md))
  — **Accepted**
- **D-035:** Three-pipe observability rule — Kindling = governance facts (write
  -once, source-of-truth); Notification envelope = user-visible state (live
  feed, source-of-truth for the dashboard); tracing/OTEL = ephemeral debugging
  context (never source-of-truth); `traceparent` is the cross-pipe correlation
  key ([ADR](./decisions/035-three-pipe-observability-rule.md)) — **Accepted**
- **D-036:** Daemon scope, discovery, OS-boundary policy — per-execution-scope
  daemons (multi-daemon by design), `info.json` runtime sidecar with two-phase
  ready, hardened `os_locality_token`, cross-Windows/WSL boundary detect-and-
  refuse, forks inherit project_uuid by default
  ([ADR](./decisions/036-daemon-scope-discovery-and-boundaries.md)) —
  **Accepted** (2026-05-13)
- **D-037:** Witness chain + L4 policy framework — per-commit hash-chained
  witness in `anvil/witnessed.ndjson` (in-tree, travels via git), active +
  archive + manifest with rollover, `flock`-protected chain integrity, per-
  branch L4 policy with `validate_at_l4` server-side fallback in
  `refs/notes/anvil-l4` ([ADR](./decisions/037-witness-chain-and-l4-policy.md))
  — **Accepted** (2026-05-13)
- **D-038:** Hook surface + noise discipline (the Serena rule) — silent on
  success, single terse line on failure, repeat-suppressed; self-contained
  binary; non-destructive integration with husky / lefthook / pcf / plain;
  panic catcher demotes crashes to exit-0 + log
  ([ADR](./decisions/038-hook-surface-and-noise-discipline.md)) — **Accepted** (2026-05-13)
- **D-039:** Baseline policy + hard-pinned rule classes — `anvil baseline`
  scans + grandfathers per-class; `secrets` and `command-safety` cannot be
  config-disabled; fingerprint-based legacy-finding tracking; baseline-
  suspicious detection
  ([ADR](./decisions/039-baseline-policy-and-hard-pinned-classes.md)) —
  **Accepted** (2026-05-13)
- **D-043:** SSH remote host daemon — SSH remote support runs Anvil on the
  remote host where the checkout and writes happen; local surfaces are display
  and control only, and local daemons must not claim protection for remote files
  ([ADR](./decisions/043-ssh-remote-host-daemon.md)) — **Proposed**

## Open Questions

### Decided

- [x] VS Code extension vs CLI-only initially? — **CLI-first**, VS Code added in
      0.1.0
- [x] Provenance storage? — **Inline-only** for 0.1.0 (no central DB)
- [x] Onboarding TUI in 0.1.0? — **Yes** — critical for adoption
- [x] Command Safety (CMDSAF) initially? — Shipped in 0.1.0
- [x] OpenTUI vs Ink for TUI implementation? — **Ink** — OpenTUI requires Bun
      runtime (bun-ffi-structs for Zig FFI); Anvil requires Node.js 20+
- [x] Should first-run auto-run `anvil check` on sample files for demo? —
      **Yes** — implemented in IFR-003 (post-init automatic analysis)

### Open

- [ ] Which entry points define "public API" for boundary detection?
- [ ] Should drift reports include team/author attribution? (Privacy concern)
- [ ] How to handle monorepos with multiple architecture baselines?
- [ ] **OQ1 (EXPORT):** Production tracing sink choice — Tempo / Honeycomb /
      Grafana Cloud / self-hosted Jaeger / OTLP-to-Vercel-OTel — to be decided
      when first paying customer or first production incident motivates it.
      EXPORT module stays Draft until then. (Planning Council session
      plan-b00c16c7, 2026-04-30)
