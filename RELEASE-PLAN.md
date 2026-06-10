# Anvil Release Plan

| Type         | Authority | Owner       | Status | Freshness                                                                      |
| ------------ | --------- | ----------- | ------ | ------------------------------------------------------------------------------ |
| Release plan | Derived   | APS modules | Live   | Rewritten 2026-06-02 to be forward-looking only; active window: `v0.8.0-beta`. |

| Upstream                                                                                                                                                        | Downstream                                                  |
| --------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------- |
| [`plans/index.aps.md`](./plans/index.aps.md), `git tag`, [`ROADMAP.md`](./ROADMAP.md), [`docs/policies/release-cadence.md`](./docs/policies/release-cadence.md) | Release runbooks, PR planning, [`ROADMAP.md`](./ROADMAP.md) |

## How this document works

This is a **forward-looking** plan, not a historical record. It scopes the **one
active release window** — its theme, scope, phase plans, and cut criteria —
nothing else.

- **Closed releases are not kept here.** Each shipped tag has an immutable
  record under [`plans/releases/<tag>.md`](./plans/releases/) (created at cut).
  On closeout, the active window is **pruned** from this file and the **next
  window is scoped** with phase plans. The release `closeout` step owns the
  prune (see
  [`docs/policies/release-cadence.md`](./docs/policies/release-cadence.md)).
- **Long-term direction** (later windows, big bets) lives in
  [`ROADMAP.md`](./ROADMAP.md), not here.
- This plan is **`Derived`** — it follows `Ready`/`Accepted` APS modules and
  ADRs; it does not lead them.
- **Enforced:** `pnpm docs:check` (the `release-plan` surface) fails CI if this
  file accretes a second window, a `Shipped`/`Next Release Window` header, an
  active window whose version is already a git tag, or an `## Active window`
  heading missing a `vX.Y.Z` version string. Run it via
  `pnpm release-plan:check`.

## Current state

- **Latest tag:** `v0.7.4-beta` (shipped 2026-06-01) — watch save-time CPU fix
  (RLB-007) + `ANVIL_HOME` install-root override (DISTRIB-006). Post-tag `main`
  also carries the Windows home-resolution + `eddacraft-tui` deep-tree fixes.
- **Cadence:** the six-week sit-on hold is **retired** (2026-06-01) — minors cut
  when ready + gates green, not on a calendar. See the
  [release-cadence policy](./docs/policies/release-cadence.md).
- **Active window:** `v0.8.0-beta` (below). Records for `v0.6.x`/`v0.7.x` are in
  [`plans/releases/`](./plans/releases/).

---

## Active window — `v0.8.0-beta` "The Graph-Backed Save-Time Daemon"

The first **minor** since `v0.7.0-beta`. It moves save-time governance off
per-save cold-spawned `check` and onto the **persistent intercept daemon
validating deltas**
([ADR-061](./plans/decisions/061-save-time-daemon-delta-validation.md), Accepted
2026-06-01) — the durable fix for the watch-CPU report (GH
[#2156](https://github.com/eddacraft/anvil-001/issues/2156)) — and backs that
daemon with the **Graph V2 resident model**, delivering it to **every user by
default** rather than behind an opt-in flag.

**Scope set 2026-06-08 by
[ADR-075](./plans/decisions/075-v080-graph-product-scope.md) (Accepted via
council, accept-with-changes).** The interim-cache slice (Sub-phase A) is
Merged, but as scoped it reached only opt-in users on one check family. The
window now carries the **GV2 A′-critical-path foundation** + the **A→A′ hot-read
backing swap** (GV2-027, under the unchanged frozen wire), and flips
`ANVIL_WATCH_DAEMON` to **default-on** (with rollout controls) once the §8 bar +
A′ swap are green — so every user gets a graph-backed save-time daemon. The
**assistant-facing graph product** (GCTX context delivery, multi-graph registry,
consumer query contract) and warm-start persistence are **deferred to v0.9**
(council recommendation: do not put GCTX, 0/13 with an unresolved GCTX-002
architectural decision and an unmet egress-privacy review, on this critical
path). **Cut when the A′ slice is ready and the gates are green — no calendar
gate.**

### Phase plan

| Phase                                         | Scope                                                                                                                                                                                                                                                                                          | State                                                                                                                                       |
| --------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------- |
| **Daemon Sub-phase A** (interim slice)        | `validate_paths` wire + watch/MCP re-point, backed by an interim `(SymbolGraph, DependencyGraph)` cache. [ADR-063](./plans/decisions/063-gv2-hot-path-boundary.md) closed the hot-path boundary + [ADR-064](./plans/decisions/064-intercept-graph-cache-crate-boundary.md) the crate boundary. | **Merged** — DSV Sub-phase A 9/9 + A-W 2/2 (cross-matrix, run 27102943706). Superseded at runtime by the A′ GV2 backing/default-on routing below.        |
| **Ready freight** (parallel)                  | RLB-002/003/004/005/008, TUIDASH-003..013, RTAI-007/-009, INSIGHTS-004.                                                                                                                                                                                                                        | **Merged** — landed via #2226/#2227/#2228/#2229/#2246.                                                                                      |
| **GV2 A′-critical-path foundation** (ADR-075) | The GV2-027 dependency closure: GV2-010 (semantic schema), 011 (incremental hot indexes), 012 (trust/policy contract), 022 (hot-read API + guardrails), 024 (hot-read type split), 025 (Criterion CI gate), 028 (production parser feed), 029 (privilege containment).                         | **In progress** — frontier GV2-010 `Ready` (deps Merged); rest dep-blocked along the 7-deep chain. GV2 4/19.                                |
| **A→A′ backing swap** (ADR-075)               | GV2-027 retires the interim re-derive; `validate_paths`/`save_time` read the resident GV2 hot-index under the **unchanged wire**; `backing_schema_version` → `gv2-hotindex-v1`.                                                                                                                | **Blocked** on GV2-022/024/028/029 (chain bottoms out at GV2-010). Gated on verdict-parity + the GV2-025 Criterion gate + **GV2-028 Done**. |
| **Default-on daemon routing** (ADR-075)       | Flip `ANVIL_WATCH_DAEMON` to default-on for `check` watches so the save-time fix reaches every user — **with rollout controls** (opt-out env, daemon-presence/auto-start guard, revert signal, staged rollout).                                                                                | **Done in-tree** — unset routes only when a live daemon answers, `ANVIL_WATCH_DAEMON=0` opts out, and `=1` preserves forced diagnostics; release runbook must exercise the opt-out/revert signal before tagging.                                                                 |
| **Closeout hygiene**                          | POLENG (9/9) + DISTRIB (6/6) → `Complete` after tag verification; GV2 gate-row → reflect ADR-063.                                                                                                                                                                                              | pending                                                                                                                                     |

### Sub-phase A is gated on the architecture review council

A review council
([verdict](./plans/reviews/2026-06-01-daemon-graph-council-verdict.md)) returned
**do not start as written**. The blocking corrections (recorded in the
[sub-phase A plan](./plans/execution/2026-06-01-daemon-save-time-subphase-a.md)):

- **B1** — ✅ **Resolved 2026-06-02**: the cache holds
  `(SymbolGraph, DependencyGraph)` and `certify` takes both; the net-new reverse
  index is folded into ADR-061 §6, contract §3, and Tasks 6/7.
- **B2** — ✅ **Resolved 2026-06-02**: `check_families: ["antipattern"]` added
  to the frozen wire; `coverage: certified` + the parity gate scoped to that
  family (do not run the policy engine on the hot path); folded into ADR-061
  §6 + contract §1/§3/§7.
- **B5** — ✅ **Resolved** by
  [ADR-064](./plans/decisions/064-intercept-graph-cache-crate-boundary.md)
  (Accepted 2026-06-02): extract `anvil-graph-cache`; the daemon depends on it
  for the `(SymbolGraph, DependencyGraph)` cache without linking the parser.
- **B6** — ✅ **Resolved 2026-06-02**: initial state
  `Stale(cross-file-resolution-needed)` (never `Clean`) + `watch` auto-scan on
  connect; folded into ADR-061 §9 + contract §6 + Tasks 7/9.
- **B4** — ✅ **Resolved 2026-06-02**: export-surface decision driven off the
  `GraphDelta.previously_public` set-diff; any modify touching a
  public/privileged symbol defaults to `partial`/`Stale` until a real
  export-diff helper lands (no dedicated helper mandated for Sub-phase A — the
  conservative default is). Folded into ADR-061 §6 + contract §3 + Task 6
  edge-case fixtures; `delta.removed_edges` is always empty so importer
  discovery uses `dependents_of` exclusively.
- **B7** — ✅ **Resolved 2026-06-02** (PR #2233): Task 3 openat2 read-safety
  made a hard predecessor of Task 8; `run_antipattern_check` re-shaped to scan
  pre-read guarded bytes on Task 10's interactive rayon pool (not the global
  pool); Task 2 reworded — the workspace-root auth handshake is net-new, not a
  reuse. Folded into the sub-phase A plan (Tasks 2/8 + File Map + sequencing
  notes).
- **B3** — ✅ **Resolved 2026-06-02** (PR #2235): the shared diagnostic envelope
  (`DiagnosticEnvelope = Vec<Diagnostic>`) now lives in `anvil-intercept-proto`,
  removing the phantom-`ScanDiagnostics` compile blocker so Task 1's wire can be
  frozen against a real type; the remaining sub-parts (typing
  `ValidatePathsResponse` + the cross-surface parity test) are Task 1 coding
  work.
- **item 8** — ✅ **Resolved 2026-06-02**: (a) `confinement.rs` stays in
  `anvil-intercept`, reusing the daemon's own `anvil_home_prefix()` (no
  wrong-direction `anvil-cli` dep — the council premise was stale); (b)
  assurance-transition logging routed via the ADR-035 Notification envelope with
  named fields (`class=FenceState`, `grouping.transition`, `context`); (c)
  mid-session daemon death/reconnect spec for `watch` (truncated in-flight ⇒
  scoped fallback, warn-once resets on reconnect, reconnect re-issues
  `request_full_scan`). Folded into ADR-061 §7/§9 + contract §4/§6 + Tasks
  9/12/14.
- **No corrections remain before coding** — all council blockers
  (B1/B2/B3/B4/B5/B6/B7 + item 8) are resolved; Sub-phase A coding may begin.
  See the sub-phase A plan's correction gate.

### Cut criteria

Base bar (carried from the interim slice): ADR-061 §8 correctness bar —
invalidation taxonomy + inode classification, cross-path diagnostic parity,
`workspace_root` auth (`openat2`/`RESOLVE_NO_SYMLINKS`), privacy line — the full
`Cross` matrix green (incl. Windows), `release-readiness.yml` pass on the source
SHA, and `ACKNOWLEDGEMENTS` fresh.

A′ bar (added by [ADR-075](./plans/decisions/075-v080-graph-product-scope.md)):

- **A→A′ swap proven** — GV2-027 verdict-parity property test green vs the
  interim backing; the **GV2-025 Criterion gate** (ADR-031 save-time budget,
  named CI job on a quiet-box runner) green on the canonical corpus; **and
  GV2-028 (production parser feed) Done** (else `ContentModify` stays
  `partial`). All three are hard gates, not prose.
- **Default-on daemon routing shipped with controls** — `ANVIL_WATCH_DAEMON`
  default-on for `check` watches behind the §8 bar, **plus**: a documented
  `ANVIL_WATCH_DAEMON=0` opt-out exercised in the runbook; default-on
  conditional on a live daemon / auto-start (no `daemon-absent` warning storm
  for non-daemon users; Windows gated on the DSV-010b served-verb set); a named
  revert signal (p95 over ADR-031 budget or WARN-rate threshold) + staged
  rollout (beta before GA).

### Deferred (to v0.9 or later)

- **Assistant graph product → v0.9:** `graph-context-delivery` (GCTX, +
  context-egress privacy review) and the non-critical-path GV2 items — GV2-013
  (control/session), 014 (plan/provenance), 020 (multi-graph registry), 023
  (consumer query contract), 026 (reverse-impact lever).
- **Persistence:** ADR-061 Sub-phase B / warm-start (GV2-030 sealed-DTO no-leak
  guard, [ADR-069](./plans/decisions/069-graph-v2-persistence.md)).
- `ssh-remote-host-daemon` (ADR-043 still Proposed).

---

## Hotfix Iteration Plan (post-tag)

**The six-week sit-on hold is retired (2026-06-01).** Releases are gated by
quality (releasable `main`, green gates, APS authorisation), not by a calendar.

| Cadence                | Channel                               | Scope                                                                 |
| ---------------------- | ------------------------------------- | --------------------------------------------------------------------- |
| `v0.7.x` patch         | Weekly while user signal is non-empty | Bug fixes, false-positive reductions, doc corrections.                |
| `v0.7.x` patch         | Within 48h of any P0 bug              | Crash, data loss, false-claim regression, daemon corruption.          |
| Next minor beta        | When ready — green gates + APS auth   | Feature additions. No calendar gate; cut when the slice is ready.     |
| Breaking beta or major | Demand-pulled                         | Driven by a real adopter requirement, not by completion of a backlog. |

Authoritative source:
[release-cadence policy](./docs/policies/release-cadence.md) (DISTRIB-004).

## Risks (active window)

| Risk                                                                                                                                                | Mitigation                                                                                                                                                                                                                                                                     |
| --------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Default-on `ANVIL_WATCH_DAEMON` degrades or destabilises stock installs (daemon-absent warning storm; daemon-defect blast radius across all users). | Rollout controls are cut criteria (ADR-075): `ANVIL_WATCH_DAEMON=0` opt-out, default-on conditional on a live daemon / auto-start (Windows gated on DSV-010b), a named revert signal, and staged rollout (beta → GA). The §8 correctness bar still gates the flip.             |
| The 7-deep GV2-027 A′ chain slips the cut — esp. GV2-028 (parser feed, medium-confidence).                                                          | Scope is the A′ slice only (council): GCTX + multi-graph registry + persistence are deferred to v0.9, off the critical path. GV2-028 Done is an explicit cut gate, so a slip can't silently ship `partial` verdicts. v0.7.x remains the P0 patch vehicle if the cut runs long. |
| Daemon save-time work re-introduces the CPU problem it exists to fix.                                                                               | The 1-hop reverse-impact cap (ADR-063, a hard-capped lever) + the ADR-031 latency budget + the two-pool isolation keep the hot path bounded.                                                                                                                                   |
| The window accretes (this document rots back into a historical record).                                                                             | "How this document works" + the closeout prune step keep it to one active window.                                                                                                                                                                                              |

## Records & roadmap

- **Shipped releases:** [`plans/releases/`](./plans/releases/) (per-tag
  records) + [`CHANGELOG.md`](./CHANGELOG.md).
- **Long-term direction / later windows** (incl. RMCPF Rust MCP full-port,
  team-lead browser surface, enterprise/language expansion):
  [`ROADMAP.md`](./ROADMAP.md).
