# Anvil Release Plan

| Type         | Authority | Owner       | Status | Freshness                                                                                 |
| ------------ | --------- | ----------- | ------ | ----------------------------------------------------------------------------------------- |
| Release plan | Derived   | APS modules | Live   | Updated 2026-06-11 at the `v0.8.1-beta` closeout; active window: `v0.9.0-beta` (scoping). |

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

- **Latest tag:** `v0.8.1-beta` "Headless GitHub Login" (shipped 2026-06-11) —
  the brokered GitHub device-flow `anvil auth login` (GHCLIAUTH 11/11, ADR-066):
  headless SSH/tmux login works end-to-end, activation page retired, `--otp`
  fallback retained; plus the eddacraft-tui 0.4.0 bump and a joi security
  override. Record:
  [`plans/releases/v0.8.1-beta.md`](./plans/releases/v0.8.1-beta.md).
  (`v0.8.0-beta` "The Save-Time Daemon" record:
  [`plans/releases/v0.8.0-beta.md`](./plans/releases/v0.8.0-beta.md).)
- **Cadence:** minors cut when ready + gates green, not on a calendar. See the
  [release-cadence policy](./docs/policies/release-cadence.md).
- **Active window:** `v0.9.0-beta` (below, **scoping** — carried from the
  ADR-075 deferral list; the operator confirms or re-scopes before coding
  starts). Records for `v0.6.x`–`v0.8.x` are in
  [`plans/releases/`](./plans/releases/).

---

## Active window — `v0.9.0-beta` "The Assistant-Facing Graph" (scoping)

Carried forward from the
[ADR-075](./plans/decisions/075-v080-graph-product-scope.md) deferral list:
`v0.8.0-beta` shipped the graph-backed save-time daemon to every user;
`v0.9.0-beta` is provisionally the window where the **resident graph becomes an
assistant-facing product** — context delivery to MCP/agent callers — plus the
daemon's warm-start persistence. **This scope is not yet operator-confirmed**;
it derives from Accepted ADR deferrals, and the gated entry decisions below must
land before any phase starts coding.

### Phase plan (provisional)

| Phase                                    | Scope                                                                                                                                                             | State                                                          |
| ---------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------- |
| **Entry decisions** (gate)               | GCTX-002 architectural decision; context-egress privacy review (council condition from ADR-075); v0.9 scope confirmation by the operator.                         | **pending** — blocks everything below                          |
| **Assistant graph product** (GCTX)       | `graph-context-delivery` module (0/13): context delivery to assistant callers over the existing MCP surface.                                                      | gated on entry decisions                                       |
| **Graph consumer surface** (GV2 balance) | GV2-020 (multi-graph registry), GV2-023 (consumer query contract), GV2-026 (reverse-impact lever), GV2-013 (control/session), GV2-014 (plan/provenance).          | gated on entry decisions                                       |
| **Persistence / warm-start**             | ADR-061 Sub-phase B: GV2-030 sealed-DTO no-leak guard + [ADR-069](./plans/decisions/069-graph-v2-persistence.md) warm-start.                                      | pending                                                        |
| **v0.8 follow-through** (parallel)       | Post-tag fixes from beta signal on the default-on daemon path (UJ-005/-007 + CIB-054 tracked fixes); WinGet publication confirm; CIB candidates from the cut log. | open — `v0.8.x` patches remain the vehicle for anything urgent |
| **Closeout hygiene**                     | All-Merged modules included in the `v0.8.0-beta` tag advance to `Complete` + archive (own PRs, per the APS archive cascade).                                      | pending                                                        |

### Cut criteria (provisional)

- Entry decisions recorded: GCTX-002 ADR Accepted, egress-privacy review passed,
  operator scope confirmation noted here.
- The GCTX slice ships behind the existing daemon trust boundary — no new policy
  engine on the hot path (ADR-061 §6 / ADR-064 invariants hold).
- ADR-031 latency gate (GV2-025 CI job) stays green — context delivery must not
  regress the save-time budget.
- The standing base bar: full `Cross` matrix green (incl. Windows),
  `release-readiness.yml` pass on the source SHA, `ACKNOWLEDGEMENTS` fresh.

### Deferred (beyond this window)

- `ssh-remote-host-daemon` (ADR-043 still Proposed).
- Marketplace track (MLP2-042..045) — blocked on licensing/pricing lock, not
  window timing.

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

| Risk                                                                                                                          | Mitigation                                                                                                                                                              |
| ----------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Default-on daemon routing surfaces field defects now that `v0.8.0-beta` reaches every installer/upgrader.                     | The DSV-021 rollout controls (documented `ANVIL_WATCH_DAEMON=0` opt-out, named revert signal) + the `v0.8.x` 48h-P0 patch lane.                                         |
| GCTX starts coding before its entry decisions (GCTX-002, egress-privacy review) — the exact risk the ADR-075 council flagged. | The entry-decisions phase is a hard gate in this plan; the window is marked **scoping** until the operator confirms.                                                    |
| Context delivery to assistants leaks privileged symbols or regresses the save-time budget.                                    | Egress-privacy review is an entry gate; the ADR-031 latency CI job (GV2-025) and the daemon trust-boundary invariants (ADR-061 §6 / ADR-064) are standing cut criteria. |
| The window accretes (this document rots back into a historical record).                                                       | "How this document works" + the closeout prune step keep it to one active window.                                                                                       |

## Records & roadmap

- **Shipped releases:** [`plans/releases/`](./plans/releases/) (per-tag
  records) + [`CHANGELOG.md`](./CHANGELOG.md).
- **Long-term direction / later windows** (incl. RMCPF Rust MCP full-port,
  team-lead browser surface, enterprise/language expansion):
  [`ROADMAP.md`](./ROADMAP.md).
