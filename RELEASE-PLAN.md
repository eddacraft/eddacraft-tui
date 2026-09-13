# anvil Release Plan

| Type         | Authority | Owner       | Status | Freshness                                                                                                                               |
| ------------ | --------- | ----------- | ------ | --------------------------------------------------------------------------------------------------------------------------------------- |
| Release plan | Derived   | APS modules | Live   | 2026-09-14: **`v0.10.0-beta` shipped** — closeout. Active window rolled to provisional `v0.10.1-beta` (field intake; claim not frozen). |

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

- **Latest tag:** `v0.10.0-beta` "Continuous journey honesty" (2026-09-13 on
  `bd6e4c98b`). Record:
  [`plans/releases/v0.10.0-beta.md`](./plans/releases/v0.10.0-beta.md).
- **Prior:** `v0.9.7-beta` first-session honesty
  ([record](./plans/releases/v0.9.7-beta.md)); `v0.9.6-beta` field fixes + shell
  command-safety; `v0.9.5-beta` MCP live-heal + config unification.
- **Cadence:** current-minor patches when user signal warrants. See
  [release-cadence policy](./docs/policies/release-cadence.md).
- **Active window:** provisional **`v0.10.1-beta`** — field intake after
  `v0.10.0-beta`. Theme and claim IDs are **not frozen**.

---

## Active window — `v0.10.1-beta` (provisional)

**Theme:** TBD from field intake after `v0.10.0-beta` (continuous journey
honesty ship).

**Status:** **Provisional; claim not locked.** Do not cut until claim freeze +
changelog + standing bar.

**Customer one-liner:** TBD.

**Authority:** Field signal + APS Ready/Accepted items after intake. Programme
work (Graph Trust Surfaces Wave 0, `/settings` SETCON+, GATT attestation,
live-heal soak) may run **beside** this window and is not automatically the cut
claim.

### Primary claim

_Not selected._ Promote only after operator intake names the theme and freezes
IDs.

### Known intake candidates

- Release packaging reliability follow-through after the `v0.10.0-beta` hollow
  first publish — the workflow repair itself lands in #4674; any residual
  hardening is intake, not yet a claim.
- WinGet community publication lag (microsoft/winget-pkgs#434141 open at
  `v0.10.0-beta` closeout).

### Not a claim of this window (default)

- **Ember** — inactive; generation unavailable; historical read only under
  `ANVIL_EMBER=1`
- **`anvil impact` / `anvil plan` / `anvil dashboard --web`** — present but
  default-off / hidden from `--help`
- Full **`/settings`** UI (SETCON foundations only)
- Intent conformance as a **product gate** — advisory `anvil conformance check`
  / CONF foundations; CEG remains Proposed
- Live-heal supervisor/proxy soak (residual restart remains honest)
- Graph Trust Surfaces / council-gate bridge discovery
- Browser dashboard default-on
- CCTX as shipped product (spike/advisory only)
- Standing CIB drain unless elevated to claim
- Unquoted-variable shell follow-ups
- Secret-detection truth (SDT) unless elevated
- CIB-353 tutorial depth (Draft editorial)
- First-run / docs prominence of telemetry disclosure (Elliot). Existing
  disclosed opt-out notice and `docs/public/anvil/operations/telemetry.md` stay
- Docs definition layer / DOCRB public-site programme
- Website decision-integrity redesign
- Graph attestation (GATT) as a headline unless intake elevates it

### Phase plan

| Phase               | Scope                                           | State         |
| ------------------- | ----------------------------------------------- | ------------- |
| **0.10.0 closeout** | Record + APS advance + prune                    | This change   |
| **Field intake**    | Post-`v0.10.0-beta` signal → theme selection    | Next          |
| **Claim lock**      | Freeze primary/secondary IDs for `v0.10.1-beta` | Not started   |
| **Implement**       | Claim items                                     | Not started   |
| **Changelog**       | Curate `[Unreleased]`                           | Not started   |
| **Cut**             | Preflight → prepare → readiness → tag           | Not scheduled |

### Cut criteria

- Standing bar: full Cross matrix, release-readiness on source SHA,
  ACKNOWLEDGEMENTS fresh, dashboard openapi `check:api` green.
- Claim locked with Merged primary items (and secondaries Merged or waived).
- Changelog leads with the locked theme only — not programme freight.
- Strategy: **direct** unless readiness forces stabilisation.
- Version stays `v0.10.1-beta` until intake names a different line.

### Risks

| Risk                                   | Mitigation                                                          |
| -------------------------------------- | ------------------------------------------------------------------- |
| Cutting without claim freeze           | Preflight/prepare blocked until RELEASE-PLAN status is claim-locked |
| Programme work mistaken for cut claim  | Keep NBI / Not a claim list current at claim lock                   |
| Hollow publish repeats on the next cut | Verify assets before closeout; #4674 carries the workflow repair    |

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
