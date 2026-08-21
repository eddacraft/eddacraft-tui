# anvil Release Plan

| Type         | Authority | Owner       | Status | Freshness                                                                                                                             |
| ------------ | --------- | ----------- | ------ | ------------------------------------------------------------------------------------------------------------------------------------- |
| Release plan | Derived   | APS modules | Live   | 2026-08-21: **`v0.9.7-beta` shipped** — closeout. Active window rolled to provisional `v0.9.8-beta` (field intake; claim not frozen). |

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

- **Latest tag:** `v0.9.7-beta` "First-session honesty" (2026-08-21 on
  `89a6d2050`). Record:
  [`plans/releases/v0.9.7-beta.md`](./plans/releases/v0.9.7-beta.md).
- **Prior:** `v0.9.6-beta` field fixes + shell command-safety
  ([record](./plans/releases/v0.9.6-beta.md)); `v0.9.5-beta` MCP live-heal +
  config unification; `v0.9.4-beta` install advice + quieter FPs.
- **Cadence:** current-minor patches when user signal warrants. See
  [release-cadence policy](./docs/policies/release-cadence.md).
- **Active window:** provisional **`v0.9.8-beta`** — field intake after
  `v0.9.7-beta`. Theme and claim IDs are **not frozen**.

---

## Active window — `v0.9.8-beta` (provisional)

**Theme:** TBD from field intake after `v0.9.7-beta` (first-session honesty
ship).

**Status:** **Provisional; claim not locked.** Do not cut until claim freeze +
changelog + standing bar.

**Customer one-liner:** TBD.

**Authority:** Field signal + APS Ready/Accepted items after intake. Programme
work (Graph Trust Surfaces Wave 0, `/settings` SETCON+, live-heal soak) may run
**beside** this window and is not automatically the cut claim.

### Primary claim

_Not selected._ Promote only after operator intake names the theme and freezes
IDs.

### Not a claim of this window (default)

- Live-heal supervisor/proxy soak (residual restart remains honest)
- Full **`/settings`** programme (SETCON / SETINS / SETPREF / …)
- Graph Trust Surfaces / council-gate bridge discovery
- Intent-conformance product ADR
- Browser dashboard default-on
- Standing CIB drain unless elevated to claim
- Unquoted-variable shell follow-ups
- Secret-detection truth (SDT) unless elevated
- CIB-353 tutorial depth (Draft editorial)
- First-run / docs prominence of telemetry disclosure (Elliot). Existing
  disclosed opt-out notice and `docs/public/anvil/operations/telemetry.md` stay
- Docs definition layer / DOCRB public-site programme
- Website decision-integrity redesign

### Phase plan

| Phase              | Scope                                          | State         |
| ------------------ | ---------------------------------------------- | ------------- |
| **0.9.7 closeout** | Record + APS advance + prune                   | This change   |
| **Field intake**   | Post-`v0.9.7-beta` signal → theme selection    | Next          |
| **Claim lock**     | Freeze primary/secondary IDs for `v0.9.8-beta` | Not started   |
| **Implement**      | Claim items                                    | Not started   |
| **Changelog**      | Curate `[Unreleased]`                          | Not started   |
| **Cut**            | Preflight → prepare → readiness → tag          | Not scheduled |

### Cut criteria

- Standing bar: full Cross matrix, release-readiness on source SHA,
  ACKNOWLEDGEMENTS fresh, dashboard openapi `check:api` green.
- Claim locked with Merged primary items (and secondaries Merged or waived).
- Changelog leads with the locked theme only — not programme freight.
- Strategy: **direct** unless readiness forces stabilisation.
- Version stays `v0.9.8-beta` until intake names a different line.

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
