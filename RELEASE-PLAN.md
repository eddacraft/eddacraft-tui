# anvil Release Plan

| Type         | Authority | Owner       | Status | Freshness                                                                                                                             |
| ------------ | --------- | ----------- | ------ | ------------------------------------------------------------------------------------------------------------------------------------- |
| Release plan | Derived   | APS modules | Live   | 2026-08-17: **`v0.9.5-beta` shipped** — closeout. Active window rolled to provisional `v0.9.6-beta` (field intake; claim not frozen). |

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

- **Latest tag:** `v0.9.5-beta` "MCP live-heal and config unification"
  (2026-08-16 on `5c4b61a78`). Record:
  [`plans/releases/v0.9.5-beta.md`](./plans/releases/v0.9.5-beta.md).
- **Prior:** `v0.9.4-beta` install advice + quieter FPs
  ([record](./plans/releases/v0.9.4-beta.md)); `v0.9.3-beta` honesty + Windows
  path; `v0.9.2-beta` MCP reconnect; `v0.9.1-beta` daily path + MCP 2.0.
- **Cadence:** current-minor patches when user signal warrants. See
  [release-cadence policy](./docs/policies/release-cadence.md).
- **Active window:** provisional **`v0.9.6-beta`** — field intake after
  `v0.9.5-beta`. Theme and claim IDs are **not frozen**.

---

## Active window — `v0.9.6-beta` (provisional)

**Theme:** TBD from field intake after `v0.9.5-beta` (live-heal + config ship).

**Status:** **Provisional; claim not locked.** Do not cut until claim freeze +
changelog + standing bar.

**Customer one-liner:** TBD.

**Authority:** Field signal + APS Ready/Accepted items after intake. Programme
work (Graph Trust Surfaces Wave 0, `/settings` SETCON+, MCPLH-007 soak) may run
**beside** this window and is not automatically the cut claim.

### Primary claim

_Not selected._ Promote only after operator intake names the theme and freezes
IDs.

### Not a claim of this window (default)

- **MCPLH-007** supervisor/proxy soak (residual restart remains honest)
- Full **`/settings`** programme (SETCON / SETINS / SETPREF / …)
- Graph Trust Surfaces / **CGBDG** Wave 0
- **CONF-001** intent-conformance ADR
- Browser dashboard default-on
- Standing CIB drain unless elevated to claim

### Phase plan

| Phase              | Scope                                          | State           |
| ------------------ | ---------------------------------------------- | --------------- |
| **0.9.5 closeout** | Record + APS advance + prune                   | Done 2026-08-17 |
| **Field intake**   | Post-`v0.9.5-beta` signal → theme selection    | Next            |
| **Claim lock**     | Freeze primary/secondary IDs for `v0.9.6-beta` | Not started     |
| **Implement**      | Claim items                                    | Not started     |
| **Changelog**      | Curate `[Unreleased]`                          | Not started     |
| **Cut**            | Preflight → prepare → readiness → tag          | Not scheduled   |

### Cut criteria

- Standing bar: full Cross matrix, release-readiness on source SHA,
  ACKNOWLEDGEMENTS fresh, dashboard openapi `check:api` green.
- Claim locked with Merged primary items (and secondaries Merged or waived).
- Changelog leads with the locked theme only — not programme freight.
- Strategy: **direct** unless readiness forces stabilisation.
- Version stays provisional until claim freeze (may stay `v0.9.6-beta` or
  escalate if product minor criteria are met).

### Risks

| Risk                                  | Mitigation                                                          |
| ------------------------------------- | ------------------------------------------------------------------- |
| Cutting without claim freeze          | Preflight/prepare blocked until RELEASE-PLAN status is claim-locked |
| Programme work mistaken for cut claim | Keep NBI / Not a claim list current at claim lock                   |

---

## Hotfix Iteration Plan (post-tag)

| Cadence             | Channel                               | Scope                                               |
| ------------------- | ------------------------------------- | --------------------------------------------------- |
| Current-minor patch | Weekly while user signal is non-empty | Bug fixes, honesty, false-positive reductions, docs |
| Current-minor patch | Within 48h of any P0                  | Crash, data loss, false-claim, daemon corruption    |
| Next minor beta     | When ready                            | Feature additions                                   |

Authoritative source:
[release-cadence policy](./docs/policies/release-cadence.md) (DISTRIB-004).

## Records & roadmap

- **Shipped releases:** [`plans/releases/`](./plans/releases/) +
  [`CHANGELOG.md`](./CHANGELOG.md).
- **Long-term direction:** [`ROADMAP.md`](./ROADMAP.md).
