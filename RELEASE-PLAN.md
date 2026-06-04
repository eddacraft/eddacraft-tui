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

## Active window — `v0.8.0-beta` "The Save-Time Daemon"

The first **minor** since `v0.7.0-beta`, earned on architecture: it begins
moving save-time governance off per-save cold-spawned `check` and onto the
**persistent intercept daemon validating deltas**
([ADR-061](./plans/decisions/061-save-time-daemon-delta-validation.md), Accepted
2026-06-01) — the durable fix for the watch-CPU report (GH
[#2156](https://github.com/eddacraft/anvil-001/issues/2156)); `v0.7.4-beta`
shipped only the RLB-007 stopgap. **Cut when the sub-phase A slice is ready and
the gates are green — no calendar gate.**

### Phase plan

| Phase                                              | Scope                                                                                                                                                                                                                                                                                                                                        | State                                                                                                                                                                             |
| -------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Daemon sub-phase A** (headline)                  | `validate_paths` wire + watch/MCP re-point, backed by an interim `(SymbolGraph, DependencyGraph)` cache. [ADR-063](./plans/decisions/063-gv2-hot-path-boundary.md) closed the hot-path boundary + [ADR-064](./plans/decisions/064-intercept-graph-cache-crate-boundary.md) the crate boundary (both Accepted).                               | **Ready to code** — all council corrections (B1/B2/B3/B4/B5/B6/B7 + item 8) ✅ resolved 2026-06-02 (folded into ADR-061 + contract + plan); no pre-implementation blockers remain |
| **Ready freight** (parallel, no daemon dependency) | RLB-002/003/004/005/008 (resource benches + SLO gate, 5 Ready); TUIDASH-003..012 (dashboard renderer chain, rides `eddacraft-tui 0.2.4` — internally sequenced, ~2-3-wide); INSIGHTS-004 (Ready 2026-06-02); RTAI-007 (Ready 2026-06-02); RTAI-009 (Ready, doc-only, scoped to shipped surfaces — RTAI-005 editor path parked under ADR-033) | **Ready** to build now                                                                                                                                                            |
| **Closeout hygiene**                               | POLENG (9/9) + DISTRIB (6/6) → `Complete` after tag verification; GV2 gate-row → reflect ADR-063                                                                                                                                                                                                                                             | pending                                                                                                                                                                           |

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

ADR-061 §8 correctness bar — invalidation taxonomy + inode classification,
cross-path diagnostic parity, `workspace_root` auth
(`openat2`/`RESOLVE_NO_SYMLINKS`), privacy line — **plus** the council
corrections above, the full `Cross` matrix green (incl. Windows),
`release-readiness.yml` pass on the source SHA, and `ACKNOWLEDGEMENTS` fresh.

### Deferred (not in this window)

GV2 hot-read backing (sub-phase A′) + the `graph-v2-foundation` /
`graph-context-delivery` modules (need
`docs/architecture/graph-v2-foundation-spec.md` + GV2-002 stable identity);
ADR-061 sub-phase B (persistence); `ssh-remote-host-daemon` (ADR-043 still
Proposed).

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

| Risk                                                                                        | Mitigation                                                                                                                                                                                                                                                      |
| ------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Sub-phase A starts before its blockers are resolved and ships an unsound `certified` claim. | The council corrections are a hard gate in the sub-phase A plan; all (B1/B2/B3/B4/B5/B6/B7 + item 8) resolved 2026-06-02, so coding may begin. The cut criteria still require the §8 correctness gates (taxonomy + parity + auth) to pass before Phase 2 ships. |
| Scope creep pulls GV2 / sub-phase A′ into the window and slips the cut.                     | A′ and the GV2 modules are explicitly deferred; the window is the interim-cache slice only.                                                                                                                                                                     |
| Daemon save-time work re-introduces the CPU problem it exists to fix.                       | The 1-hop reverse-impact cap (ADR-063, a hard-capped lever) + the ADR-031 latency budget + the two-pool isolation keep the hot path bounded.                                                                                                                    |
| The window accretes (this document rots back into a historical record).                     | "How this document works" + the closeout prune step keep it to one active window.                                                                                                                                                                               |

## Records & roadmap

- **Shipped releases:** [`plans/releases/`](./plans/releases/) (per-tag
  records) + [`CHANGELOG.md`](./CHANGELOG.md).
- **Long-term direction / later windows** (incl. RMCPF Rust MCP full-port,
  team-lead browser surface, enterprise/language expansion):
  [`ROADMAP.md`](./ROADMAP.md).
