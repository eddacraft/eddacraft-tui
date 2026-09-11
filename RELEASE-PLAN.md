# anvil Release Plan

| Type         | Authority | Owner       | Status | Freshness                                                                                                                                                                                                                                 |
| ------------ | --------- | ----------- | ------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Release plan | Derived   | APS modules | Live   | 2026-09-11: **`v0.9.8-beta` claim locked** — continuous journey honesty (JSIMP + JREL attach/worktree/overrides; JOURNEY-015/-016 as gates only). Implementation already on `main`. Remaining cut work is standing bar + preflight → tag. |

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
- **Active window:** **`v0.9.8-beta`** — claim locked: continuous journey
  honesty. Primary items are Merged on `main`. Not cut-ready until standing
  bar + preflight.

---

## Active window — `v0.9.8-beta` (claim locked)

**Theme:** Continuous journey honesty — first use through daily ensure tells the
truth about activation, coverage, and protection.

**Status:** **Claim locked; not cut-ready.** Primary items are Merged on `main`.
Remaining cut work is the standing bar and preflight → prepare → readiness →
tag. Changelog `[Unreleased]` is curated theme-led in this lock.

**Customer one-liner:** From a never-activated checkout through quiet daily
recovery, anvil names what is proven (closing receipt, live protection attach,
worktree-pinned MCP) and does not treat silence, missing MCP, or a
configured-but-closed editor as success.

**Authority:** Operator-approved claim (2026-09-11) from the Librarian release
prep draft after JOURNEY-015/-016 evidence and JSIMP/JREL landings on `main`.
JOURNEY passes are gates only — they did **not** grant publication authority.

### Journey reliability readiness

The [JOURNEY programme](./plans/modules/release-user-journeys.aps.md) verified
[JREL reliability](./plans/archive/modules/journey-reliability.aps.md) and
[JSIMP simplification](./plans/modules/journey-simplification.aps.md) on pinned
main builds. Those gates are **Merged** and inform this claim; they are not
themselves publication authority. Use the existing release process; an internal
release channel is not a prerequisite.

**JOURNEY-015 disposition (2026-09-10):** pinned-main rehearsal **pass** on
`3f8890e15` for the required non-upgrade legs (claim #4572; evidence under
`plans/audits/2026-09-10-journey-015-*`). The recorded `--require-upgrade` pass
is **not** previous-public-build proof — the previous binary was not invoked
(#4591). Claim freeze and publication were **not** granted by that gate alone;
simplification was permitted and later landed.

**JOURNEY-016 disposition (2026-09-10):** simplification acceptance **pass** on
`5489c6112` (claim #4613; evidence under
`plans/audits/2026-09-10-journey-016-*`). No splash / always-on / dashboard.
Claim freeze, changelog lock for cut, standing gates, and publication authority
were **not** granted by that gate alone — this lock supplies the claim freeze.

### Primary claim (continuous journey honesty)

| ID                 | Item                                                 | Pri   | State                                           | Notes                                                                         |
| ------------------ | ---------------------------------------------------- | ----- | ----------------------------------------------- | ----------------------------------------------------------------------------- |
| JSIMP-001…006      | Continuous journey (ADR-145) through public guidance | P0–P1 | Merged #4580, #4585, #4599, #4604, #4607, #4611 | Never-activated bare, start consent, quiet restore, closing receipt, guidance |
| JREL-002           | Live MCP attach evidence for protection claims       | P0    | Merged #4416                                    | Handshake-attributed session; configured-but-closed is not live               |
| JREL-009           | Preserve explicit MCP launch choices during repair   | P1    | Merged #4556                                    | Daily ensure keeps overrides; only obsolete managed paths migrate             |
| JREL-010           | MCP worktree identity pin                            | P1    | Merged #4562                                    | Package subdirs / linked worktrees / symlinks share one project               |
| JOURNEY-015 / -016 | Reliability rehearsal + simplification acceptance    | —     | Merged #4574 / #4614                            | Gates only — not publication authority                                        |

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
- Secret-detection truth (SDT) as the cut theme (ships as tip freight)
- CIB-353 tutorial depth (Draft editorial)
- First-run / docs prominence of telemetry disclosure (Elliot). Existing
  disclosed opt-out notice and `docs/public/anvil/operations/telemetry.md` stay
- Docs definition layer / DOCRB public-site programme
- Website decision-integrity redesign
- **"JOURNEY published"** — JOURNEY-015/-016 explicitly did **not** grant
  publication authority
- Graph attestation (GATT) as the headline — optional secondary freight on the
  same tip

### Phase plan

| Phase              | Scope                                       | State                                                                      |
| ------------------ | ------------------------------------------- | -------------------------------------------------------------------------- |
| **0.9.7 closeout** | Record + APS advance + prune                | Done 2026-08-21                                                            |
| **Field intake**   | Post-`v0.9.7-beta` signal → theme selection | Done 2026-09-10 (JOURNEY-015/-016) + 2026-09-11 claim wording              |
| **Claim lock**     | Freeze primary IDs for `v0.9.8-beta`        | This change                                                                |
| **Implement**      | Claim items                                 | Done on `main` (JSIMP #4580–#4611; JREL-002/#4416, -009/#4556, -010/#4562) |
| **Changelog**      | Curate `[Unreleased]` theme-led             | This change (builds on #4615 / #4631)                                      |
| **Cut**            | Preflight → prepare → readiness → tag       | Next                                                                       |

### Cut criteria

- Standing bar: full Cross matrix, release-readiness on source SHA,
  ACKNOWLEDGEMENTS fresh, dashboard openapi `check:api` green.
- Claim locked with Merged primary items (and secondaries Merged or waived).
- Changelog leads with the locked theme only — not programme freight.
- Strategy: **direct** unless readiness forces stabilisation.
- Version stays `v0.9.8-beta` (patch on the v0.9 line).

### Risks

| Risk                                      | Mitigation                                                          |
| ----------------------------------------- | ------------------------------------------------------------------- |
| Cutting without claim freeze              | Preflight/prepare blocked until RELEASE-PLAN status is claim-locked |
| Programme work mistaken for cut claim     | Keep NBI / Not a claim list current at claim lock                   |
| JOURNEY pass misread as publish authority | JOURNEY-015/-016 dispositions stay explicit: gates only             |

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
