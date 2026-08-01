# anvil Release Plan

| Type         | Authority | Owner       | Status | Freshness                                                                                                                                                                                                        |
| ------------ | --------- | ----------- | ------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Release plan | Derived   | APS modules | Live   | Turned over 2026-07-12 at the `v0.9.0-beta` closeout (record: [`plans/releases/v0.9.0-beta.md`](./plans/releases/v0.9.0-beta.md)); the `v0.10.0-beta` window scope was **confirmed by the operator 2026-07-13**. |

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

- **Latest tag:** `v0.9.0-beta` "First-Run Wins and the Assistant Graph"
  (shipped 2026-07-12) — the assistant-facing graph over MCP (GCTX 14/14,
  ADR-083), warm-start persistence (ADR-069 + the ADR-105 base+delta store,
  `ANVIL_PERSIST_GRAPH` default-on), the MCP-optional useful daemon (ACTMO + the
  ADR-101 headless save-time driver), USAGE analytics, and the JOURNEY-conducted
  first-run/daily-confidence cut. Record:
  [`plans/releases/v0.9.0-beta.md`](./plans/releases/v0.9.0-beta.md).
  (`v0.8.1-beta` "Headless GitHub Login" record:
  [`plans/releases/v0.8.1-beta.md`](./plans/releases/v0.8.1-beta.md).)
- **Cadence:** minors cut when ready + gates green, not on a calendar. See the
  [release-cadence policy](./docs/policies/release-cadence.md).
- **Active window:** `v0.10.0-beta` (below, **confirmed by the operator
  2026-07-13**): the DASH dashboard-foundation wave (the team-lead browser
  surface, Horizon 2) plus the `v0.9.x` follow-through patch lane, the residual
  v0.9 closeout hygiene, and the **MCPX/SKPKG carry-in** already merged to
  `main` after the `v0.9.0-beta` cut. Records for `v0.6.x`–`v0.9.x` are in
  [`plans/releases/`](./plans/releases/).

---

## Active window — `v0.10.0-beta` "Team-Lead Surface Foundations"

**Scope confirmed by the operator 2026-07-13** — the team-lead browser surface
theme and the DASH wave's place in it, as scoped at the `v0.9.0-beta` turnover.
Implementation planning may treat this window as approved.

The JOURNEY release accepted the browser surface as explicitly **post-cut**
expansion; this window picks that thread up. The
[`dashboard-foundation`](./plans/modules/dashboard-foundation.aps.md) wave
(DASH-001..012, all Merged) built `apps/dashboard/` (Vite 8 + React + TanStack
Router/Query/Table + shadcn/ui + Tailwind v4) backed by
`crates/anvil-dashboard-server/`, unblocking **DASHCORE**, **DASHARCH**, and
**DASHOPS**. Wave 1 (PR #3261, PR #3321) left the surface unreachable — a repo
checkout, a Node toolchain, and two processes — and **DASH-012** (PR #3421,
2026-07-26) closed that: `anvil dashboard --web` now serves the embedded UI from
the installed binary.

[`dashboard-core-views`](./plans/modules/dashboard-core-views.aps.md) has landed
all nine items: DASHCORE-001 via PR #3363, DASHCORE-003..009 via PR #3379, and
DASHCORE-002 retained history and trends via PR #3436 on 2026-07-27. The module
is 9/9 Merged and awaits release evidence in this window. JOURNEY post-cut
expansion remains non-blocking for the cut: JOURNEY-007 + WOW-006 Merged via
#3441 (2026-07-30); JOURNEY-008 Merged via #3408; JOURNEY-009 on hold;
JOURNEY-010 blocked on DASHARCH/DASHOPS.

This window also carries **MCPX** and **SKPKG**, both merged to `main` in #3328
on 2026-07-15 — three days after the `v0.9.0-beta` cut — and therefore in no
tag. They need no further implementation, but they are named here because
nothing of them is reachable from the latest tagged release: as of
`v0.9.0-beta`, `anvil mcp install --client` accepts two clients, against twelve
on `main`, so every user on the released binary still wires at most Claude Code
and Cursor. Untracked carry-in ships by accident of being on `main`; naming it
makes the cut evidence real and gives the closeout a `Released/Shipped`
transition to make. Module statuses are unchanged by this document — advancing
them is the closeout step's job, once a tag exists.

### Phase plan

| Phase                               | Scope                                                                                                                                                                         | State                                                                                                                                                                                      |
| ----------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| **Scope confirmation** (gate)       | Operator confirms the `v0.10.0-beta` theme (team-lead browser surface vs. an alternative priority) and the DASH wave's place in it.                                           | **done 2026-07-13** — confirmed by the operator                                                                                                                                            |
| **v0.9 closeout hygiene**           | All-Merged modules included in `v0.9.0-beta` advance to `Released/Shipped` + archive (own PRs, per the APS archive cascade); CIB intake from the cut log.                     | done 2026-07-13 — release record + tracking-issue closeout 2026-07-12; 17 tag-complete modules archived via the cascade PR                                                                 |
| **Dashboard foundation** (DASH)     | DASH-001..012: scaffold, server crate, auth posture, core routing/data layer, first role views, and `anvil dashboard --web` delivery from the installed binary.               | all 12 Merged (PRs #3261, #3321, #3421) — module In Progress pending release evidence                                                                                                      |
| **Dashboard core views** (DASHCORE) | Product routes on latest-gate evidence: DASHCORE-001 and DASHCORE-003 through DASHCORE-009; DASHCORE-002 adds retained gate history and honest trends from its approved spec. | 9/9 Merged (PR #3363, PR #3379, PR #3436); pending release evidence                                                                                                                        |
| **JOURNEY post-cut expansion**      | JOURNEY-007..011 as coordinated enhancements alongside the DASH wave. JOURNEY-011 / ONSW (bare `anvil` ensure) targeted for this cut.                                         | JOURNEY-007 + WOW-006 Merged via #3441 (2026-07-30); JOURNEY-008 Merged #3408; JOURNEY-009 on hold; JOURNEY-010 blocked on DASHARCH/DASHOPS; **JOURNEY-011 / ADR-114 / ONSW Merged #3474** |
| **MCPX/SKPKG carry-in**             | Merged-but-untagged multi-harness MCP install (twelve `--client` targets) and skill packaging; no implementation left, verify at cut and advance at closeout.                 | merged 2026-07-15 via #3328 — in no tag; rides this window                                                                                                                                 |
| **v0.9 follow-through** (lane)      | Beta-signal fixes on the shipped first-run/daemon/graph surfaces (48h-P0 patch lane on `v0.9.x`); CIB-193/-194/-195/-196 and the release-recovery hardening (#3309).          | all closed: CIB-193/-194/-195/-196 merged (2026-07-12, 2026-07-26 via #3422) and #3309 merged 2026-07-12; the lane stays the vehicle for anything urgent                                   |

### Cut criteria

- The standing base bar: full `Cross` matrix green (incl. Windows),
  `release-readiness.yml` pass on the source SHA, `ACKNOWLEDGEMENTS` fresh.
- ADR-031 latency gate (GV2-025 CI job) stays green — the dashboard server must
  not regress the save-time budget.
- DASH acceptance gates as defined by the owning modules once the wave is
  implementation-planned.
- The MCPX carry-in is verified **from the release artefact, not from `main`**:
  `anvil mcp install --client <client> --verify` succeeds for a non-Cursor,
  non-Claude-Code client using the tagged binary. Reading `--help` on a stale
  installed binary is what hid the gap across the whole `v0.9.0-beta` window.

---

## Hotfix Iteration Plan (post-tag)

Releases are gated by quality (releasable `main`, green gates, APS
authorisation), not by a calendar.

| Cadence                | Channel                               | Scope                                                                 |
| ---------------------- | ------------------------------------- | --------------------------------------------------------------------- |
| Current-minor patch    | Weekly while user signal is non-empty | Bug fixes, false-positive reductions, doc corrections.                |
| Current-minor patch    | Within 48h of any P0 bug              | Crash, data loss, false-claim regression, daemon corruption.          |
| Next minor beta        | When ready — green gates + APS auth   | Feature additions. No calendar gate; cut when the slice is ready.     |
| Breaking beta or major | Demand-pulled                         | Driven by a real adopter requirement, not by completion of a backlog. |

Authoritative source:
[release-cadence policy](./docs/policies/release-cadence.md) (DISTRIB-004).

## Risks (active window)

| Risk                                                                                                                                                                                                            | Mitigation                                                                                                                                                                                                                                                                                                                                 |
| --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Default-on graph persistence and the first-run surfaces reach every upgrader — field defects surface now.                                                                                                       | The `v0.9.x` 48h-P0 patch lane; `ANVIL_PERSIST_GRAPH=0` and `ANVIL_WATCH_DAEMON=0` documented opt-outs; CIB intake from beta signal.                                                                                                                                                                                                       |
| Release publication depends on a manually-rotated PAT (`ANVIL_RELEASES_TOKEN`) — the v0.9 cut stalled ~6h on its expiry.                                                                                        | Closed on both halves: #3309 (merged 2026-07-12) hardened recovery after a failed publication, and `validate-publication-token.sh` now fails the readiness gate when the credential is absent, rejected, unpermitted, or expiring within 14 days — so the v0.9 failure mode is caught before a cut starts rather than at the publish step. |
| The window accretes (this document rots back into a historical record).                                                                                                                                         | "How this document works" + the closeout prune step keep it to one active window.                                                                                                                                                                                                                                                          |
| Work merged between a cut and the next window's scoping is invisible here, so a module can read `Done` while delivering nothing to users (MCPX/SKPKG, merged 2026-07-15, untagged since the `v0.9.0-beta` cut). | Name merged-but-untagged modules as explicit carry-in when a window is scoped; verify them from the release artefact at cut, not from `main`.                                                                                                                                                                                              |

## Records & roadmap

- **Shipped releases:** [`plans/releases/`](./plans/releases/) (per-tag
  records) + [`CHANGELOG.md`](./CHANGELOG.md).
- **Long-term direction / later windows** (incl. RMCPF Rust MCP full-port,
  enterprise/language expansion): [`ROADMAP.md`](./ROADMAP.md).
