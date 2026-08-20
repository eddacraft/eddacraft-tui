# anvil Release Plan

| Type         | Authority | Owner       | Status | Freshness                                                                                                                                                                                          |
| ------------ | --------- | ----------- | ------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Release plan | Derived   | APS modules | Live   | 2026-08-20: **`v0.9.7-beta` claim locked** — first-session honesty after Chris Bridle's `v0.9.6-beta` run. Implementation already on `main`. Remaining cut work is standing bar + preflight → tag. |

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

- **Latest tag:** `v0.9.6-beta` "Beta field fixes and shell command-safety"
  (2026-08-18 on `07cd54c3a`). Record:
  [`plans/releases/v0.9.6-beta.md`](./plans/releases/v0.9.6-beta.md).
- **Prior:** `v0.9.5-beta` MCP live-heal + config unification
  ([record](./plans/releases/v0.9.5-beta.md)); `v0.9.4-beta` install advice +
  quieter FPs; `v0.9.3-beta` honesty + Windows path.
- **Cadence:** current-minor patches when user signal warrants. See
  [release-cadence policy](./docs/policies/release-cadence.md).
- **Active window:** **`v0.9.7-beta`** — first-session honesty after
  `v0.9.6-beta`. Claim **locked** 2026-08-20. Patch on the v0.9 line. Programme
  work (Graph Trust Surfaces Wave 0, `/settings`, live-heal soak) runs
  **beside** this window and is not the cut claim.

---

## Active window — `v0.9.7-beta` (claim locked)

**Theme:** First-session honesty after `v0.9.6-beta` — unsigned welcome does not
dead-end on gated Policy/Architecture commands, the hub gate shows live
progress, "Choose a learning path" opens the path picker, and audit Next Steps
jump to the matching issue.

**Status:** **Claim locked; not cut-ready.** Primary items are Merged on `main`.
Remaining cut work is the standing bar and preflight → prepare → readiness →
tag. Changelog `[Unreleased]` is curated in this lock.

**Customer one-liner:** First-session welcome, gate progress, learning path, and
Audit Next Steps tell the truth.

**Authority:** Chris Bridle first-session pack-09 on published `v0.9.6-beta`
(2026-08-19). Operator locked 2026-08-20 so the next tester gets the published
installer, not `main`.

### Primary claim (first-session honesty)

| ID      | Item                                                           | Pri | State        | Notes                                              |
| ------- | -------------------------------------------------------------- | --- | ------------ | -------------------------------------------------- |
| CIB-349 | Unsigned welcome must not dead-end on gated tutorial commands  | P1  | Merged #4004 | Sign-in bridge names `anvil auth login` first      |
| CIB-350 | Hub "Review gate decision" shows live progress on a large repo | P1  | Merged #4005 | Loading line updates per scan/check                |
| CIB-351 | "Choose a learning path" opens the path picker, not discovery  | P2  | Merged #4006 | First-run wow still allowed before the hub menu    |
| CIB-352 | Audit Next Steps jump to the matching Issues row               | P1  | Merged #4002 | Enter expands the issue; footer matches capability |

### Not a claim of this window (default)

- Live-heal supervisor/proxy soak (residual restart remains honest)
- Full **`/settings`** programme (SETCON / SETINS / SETPREF / …)
- Graph Trust Surfaces / council-gate bridge discovery
- Intent-conformance product ADR
- Browser dashboard default-on
- Standing CIB drain unless elevated to claim
- Unquoted-variable shell follow-ups
- Secret-detection truth (SDT) unless elevated
- CIB-353 tutorial depth (Draft editorial; two first-timers, not this cut)
- CIB-344 stale produce-lock reap (rides the tip as dogfood reliability; not the
  theme)
- First-run / docs prominence of telemetry disclosure (Elliot). Existing
  disclosed opt-out notice and `docs/public/anvil/operations/telemetry.md` stay;
  do not add a first-session lecture this cut
- Docs definition layer / DOCRB public-site programme
- Website decision-integrity redesign
- SEC-012 entitlement-claim honesty (API/docs-shell; not the CLI binary)

### Phase plan

| Phase              | Scope                                       | State                               |
| ------------------ | ------------------------------------------- | ----------------------------------- |
| **0.9.6 closeout** | Record + APS advance + prune                | Done 2026-08-18                     |
| **Field intake**   | Post-`v0.9.6-beta` signal → theme selection | Done 2026-08-20 (pack-09)           |
| **Claim lock**     | Freeze primary IDs for `v0.9.7-beta`        | This change                         |
| **Implement**      | Claim items                                 | Done on `main` (#4002, #4004–#4006) |
| **Changelog**      | Curate `[Unreleased]`                       | This change                         |
| **Cut**            | Preflight → prepare → readiness → tag       | Next                                |

### Cut criteria

- Standing bar: full Cross matrix, release-readiness on source SHA,
  ACKNOWLEDGEMENTS fresh, dashboard openapi `check:api` green.
- Claim locked with Merged primary items (and secondaries Merged or waived).
- Changelog leads with the locked theme only — not programme freight.
- Strategy: **direct** unless readiness forces stabilisation.
- Version stays `v0.9.7-beta` (patch on the v0.9 line).

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
