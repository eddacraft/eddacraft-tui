# Anvil Release Plan

| Type         | Authority | Owner       | Status | Freshness                                                                                                                                                                                                                                                                                                         |
| ------------ | --------- | ----------- | ------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Release plan | Derived   | APS modules | Live   | Reconciled 2026-06-29 to module truth and operator beta-usefulness review: the original assistant-graph scope is complete, but the `v0.9.0-beta` cut-line now needs a default-on daemon usefulness addendum before release. Candidate additions are ACTMO-013 (subsequent worktree registration UX) and DSV-046 (headless background save-time driver), pending APS promotion. (Prior: 2026-06-26 scoped modules complete; 2026-06-13 operator scope confirmation.) |

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
- **Active window:** `v0.9.0-beta` (below, **assistant-graph scope complete;
  default-on daemon usefulness addendum under APS review**).
  Direction confirmed by the operator 2026-06-13 — assistant-facing graph
  (ADR-075 deferrals) + bug-fixing, with USAGE added as additive scope. The
  ADR-075 entry decisions landed 2026-06-15 (ADR-083 Accepted + the
  context-egress privacy review (PV-9) filed); the scoped feature work then
  completed — as of 2026-06-26 every scoped module is Merged/Done (see the phase
  plan). What remains is the cut itself: the standing base bar + latency gate
  (cut criteria below) and operator go. Records for `v0.6.x`–`v0.8.x` are in
  [`plans/releases/`](./plans/releases/).

---

## Active window — `v0.9.0-beta` "The Assistant-Facing Graph" (usefulness addendum under review)

Carried forward from the
[ADR-075](./plans/decisions/075-v080-graph-product-scope.md) deferral list:
`v0.8.0-beta` shipped the graph-backed save-time daemon to every user;
`v0.9.0-beta` is the window where the **resident graph becomes an
assistant-facing product** — context delivery to MCP/agent callers — plus the
daemon's warm-start persistence. The operator confirmed this direction on
2026-06-13 (see **Operator priorities** below). The ADR-075 entry gate (GCTX-002
ADR, context-egress privacy review) applies specifically to the **GCTX
assistant-facing egress surface** — it must land before GCTX coding and is a cut
prerequisite. The **internal GV2 substrate items** were deferred to v0.9 by
_scope_, not entry-gated (ADR-075), and were delivered once their dependencies
Merged; the GCTX entry decisions landed **2026-06-15**
([ADR-083](./plans/decisions/083-gctx-mcp-delivery-target.md) Accepted + the
[context-egress privacy review (PV-9)](./plans/reviews/2026-06-15-gctx-context-egress-privacy-review-verdict.md),
APPROVE-WITH-CONDITIONS).

**Operator priorities (2026-06-13):** the assistant-facing graph (above) and
ongoing **bug-fixing / beta-signal quality** are the headline for this window;
the `v0.8.x` patch lane and the v0.8 follow-through phase carry the latter.
**Usage analytics (USAGE,
[`usage-analytics`](./plans/modules/usage-analytics.aps.md))** was additionally
scoped into this window as founder-requested additive work; it landed in full —
USAGE-001..005 all Merged (module Done-but-for-release, 5/5). See the phase
plan.

**Operator usefulness review (2026-06-29):** do not cut `v0.9.0-beta` as a
pure assistant-graph release if the default-on daemon still feels like passive
infrastructure. The next release should make `anvil start` useful without MCP
and without a visible `anvil watch` terminal: start/reuse the per-user daemon,
register the current worktree when possible, provide an obvious way to register
later-created worktrees, and honestly surface background save-time state. The
candidate APS additions are
[`ACTMO-013`](./plans/modules/activation-mcp-optional.aps.md) and
[`DSV-046`](./plans/modules/daemon-save-time-validation.aps.md); promote them
before treating this addendum as committed release scope.

### Phase plan (provisional)

| Phase                                    | Scope                                                                                                                                                                                                                                                                                                                                                                                                                               | State                                                                                                                                                                                                                                                                                                    |
| ---------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Entry decisions** (gate)               | GCTX-002 architectural decision ([ADR-083](./plans/decisions/083-gctx-mcp-delivery-target.md) "GCTX-002 — MCP target for assistant graph context delivery", **Accepted 2026-06-15**; targets Rust RMCPF `anvil mcp serve` primary per ADR-033/ RMCPF); context-egress privacy review ([PV-9](./plans/reviews/2026-06-15-gctx-context-egress-privacy-review-verdict.md)); v0.9 scope confirmation by the operator (done 2026-06-13). | **done 2026-06-15** — ADR-083 Accepted (Josh) + the PV-9 egress review filed (APPROVE-WITH-CONDITIONS, 4/4; CE-1..CE-12 fold into GCTX-001). GCTX-002 + GCTX module + GV2-020/-023 Ready-promoted.                                                                                                       |
| **Assistant graph product** (GCTX)       | `graph-context-delivery` module (14/14): context delivery to assistant callers over the existing MCP surface.                                                                                                                                                                                                                                                                                                                       | **complete 2026-06-26** — module 14/14, all Merged: contract (GCTX-001/-002) → Phase-1 tools (010..014) → `graph://` resources (030) → snippet slicing (020..023) → token-reduction benchmark (031, #2942) → user guide (032, #2952).                                                                    |
| **Graph consumer surface** (GV2 balance) | GV2-013 (control/session), GV2-014 (plan/provenance), GV2-026 (reverse-impact lever) — internal substrate, deps Merged; GV2-020 (multi-graph registry) + GV2-023 (consumer query contract) layer on top.                                                                                                                                                                                                                            | **complete** — GV2 module **Done 21/21**: GV2-013/-014 Merged #2578/#2579, GV2-026 #2594, GV2-020 #2622, GV2-023 #2621, GV2-031 #2627 (and the deferred GV2-032 span/hash substrate Merged).                                                                                                             |
| **Persistence / warm-start**             | ADR-061 Sub-phase B: GV2-030 sealed-DTO no-leak guard + [ADR-069](./plans/decisions/069-graph-v2-persistence.md) warm-start.                                                                                                                                                                                                                                                                                                        | **complete** — GV2-030 sealed-DTO no-leak guard Merged #2595; DSV-030 warm-start persistence Merged #2688 (ADR-061 Sub-phase B reconciled 2026-06-24, ADR-069 Accepted)                                                                                                                                  |
| **Usage analytics** (USAGE, additive)    | [`usage-analytics`](./plans/modules/usage-analytics.aps.md) (5/5): `command.invoked` Kindling observation kind + CLI/JSON-RPC producers (privacy contract, conformance fixture), flag-context join, dev-investment query views, and flag-driven licence-gate enforcement.                                                                                                                                                           | **complete** — `usage-analytics` **5/5**, all Merged (USAGE-001 command-invocation kind/producer, -002 flag-context join, -003 dev-investment views, -004 JSON-RPC producer, -005 flag-driven licence-gate)                                                                                              |
| **Default-on daemon usefulness addendum** | Candidate cut-line additions: ACTMO-013 (clear registration UX for later-created worktrees, `anvil start` outside a worktree, duplicate registration as heartbeat/update, guided opt-in for automatic `anvil start --no-mcp` when a Worktrunk/Git worktree is created, optional Worktrunk auto-registration, investigation of a global opt-in mode that discovers only configured in-scope apps/workspaces, and a scoped local tray/menu-bar daemon vehicle design) plus DSV-046 (headless background save-time driver so `anvil start` can drive validation without a foreground `anvil watch` terminal). | **under APS review 2026-06-29** — not yet committed scope until ACTMO-013/DSV-046 are promoted. Minimum useful release shape: `anvil start` from a worktree registers it and starts/ensures unattended save-time validation; `anvil workspace register [path]` or equivalent attaches later worktrees; status reports daemon, registration, watching/protecting, last validation, and fence state truthfully. |
| **v0.8 follow-through** (parallel)       | Post-tag fixes from beta signal on the default-on daemon path (UJ-005/-007 + CIB-054 tracked fixes); WinGet publication confirm; CIB candidates from the cut log.                                                                                                                                                                                                                                                                   | open — `v0.8.x` patches remain the vehicle for anything urgent                                                                                                                                                                                                                                           |
| **Closeout hygiene**                     | All-Merged modules included in a release tag advance to `Complete` + archive (own PRs, per the APS archive cascade).                                                                                                                                                                                                                                                                                                                | v0.8.0 closeout **done** (reconcile #2573 + archive cascade #2575, 2026-06-13). **v0.9 closeout pending the tag:** GCTX (14/14), GV2 (Done 21/21), USAGE (5/5) and the other all-Merged graph/daemon modules stay `In Progress` until `v0.9.0-beta` includes them, then advance to `Complete` + archive. |

### Cut criteria (provisional)

- Entry decisions recorded: GCTX-002 ADR Accepted, egress-privacy review passed,
  operator scope confirmation noted here.
- The GCTX slice ships behind the existing daemon trust boundary — no new policy
  engine on the hot path (ADR-061 §6 / ADR-064 invariants hold).
- ADR-031 latency gate (GV2-025 CI job) stays green — context delivery must not
  regress the save-time budget.
- If the usefulness addendum is promoted into the `v0.9.0-beta` cut-line:
  `anvil start --no-mcp` from a worktree must register that worktree and run
  unattended save-time validation with no visible `anvil watch` terminal; a
  later-created worktree must have one obvious registration path; duplicate
  registration must be a heartbeat/update; there must be a guided opt-in path for
  automatic `anvil start --no-mcp` when a Worktrunk/Git worktree is created; any
  global auto-discovery mode must be opt-in and limited by explicit config that
  identifies in-scope apps/workspaces; and status output must distinguish
  `watching` from MCP-backed `protecting`.
- The standing base bar: full `Cross` matrix green (incl. Windows),
  `release-readiness.yml` pass on the source SHA, `ACKNOWLEDGEMENTS` fresh.

### Deferred (beyond this window)

- `ssh-remote-host-daemon` (ADR-043 still Proposed).
- Marketplace track (MLP2-042..045) — blocked on licensing/pricing lock, not
  window timing.
- Sandbox-grade fence/session containment, daemon-to-session write-back, and
  interrupt/fence/kill lifecycle expansion (MLP2-077..079) — important for the
  protection model, but not required for the minimum useful `v0.9.0-beta` daemon
  experience.
- Full desktop product UI. A small local app may be designed as a scoped daemon
  control vehicle under ACTMO-013, but implementation should not block the cut
  unless APS explicitly promotes it.

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

| Risk                                                                                                                          | Mitigation                                                                                                                                                                                                                                  |
| ----------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Default-on daemon routing surfaces field defects now that `v0.8.0-beta` reaches every installer/upgrader.                     | The DSV-021 rollout controls (documented `ANVIL_WATCH_DAEMON=0` opt-out, named revert signal) + the `v0.8.x` 48h-P0 patch lane.                                                                                                             |
| GCTX starts coding before its entry decisions (GCTX-002, egress-privacy review) — the exact risk the ADR-075 council flagged. | **Discharged** — the entry decisions landed 2026-06-15 (ADR-083 Accepted + PV-9 egress review); GCTX was then built entirely behind them (module 14/14 Merged — not yet released — on the sealed-DTO `GctxProjector` + CE-5 no-leak spine). |
| Context delivery to assistants leaks privileged symbols or regresses the save-time budget.                                    | Egress-privacy review is an entry gate; the ADR-031 latency CI job (GV2-025) and the daemon trust-boundary invariants (ADR-061 §6 / ADR-064) are standing cut criteria.                                                                     |
| `v0.9.0-beta` ships impressive assistant graph APIs but the default user still sees no useful background behaviour after `anvil start`. | Treat ACTMO-013 + DSV-046 as the candidate usefulness addendum: registration UX + headless save-time driver, with truthful status copy and no over-claiming of `protecting` without live MCP.                                                 |
| The window accretes (this document rots back into a historical record).                                                       | "How this document works" + the closeout prune step keep it to one active window.                                                                                                                                                           |

## Records & roadmap

- **Shipped releases:** [`plans/releases/`](./plans/releases/) (per-tag
  records) + [`CHANGELOG.md`](./CHANGELOG.md).
- **Long-term direction / later windows** (incl. RMCPF Rust MCP full-port,
  team-lead browser surface, enterprise/language expansion):
  [`ROADMAP.md`](./ROADMAP.md).
