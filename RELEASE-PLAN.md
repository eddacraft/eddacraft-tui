# anvil Release Plan

| Type         | Authority | Owner       | Status | Freshness                                                                                                                                                                        |
| ------------ | --------- | ----------- | ------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Release plan | Derived   | APS modules | Live   | 2026-09-17: closeout record for **`v0.11.1-beta`** published; active window remains **`v0.12.0-beta`** (remaining `/settings` work after SETPREF landed Done on the 0.11.1 tip). |

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

- **Latest tag:** `v0.11.1-beta` "Gate and inspect honesty" (2026-09-16 on
  `e7024b7a6`). Changelog: [`CHANGELOG.md`](./CHANGELOG.md). Per-tag record:
  [`plans/releases/v0.11.1-beta.md`](./plans/releases/v0.11.1-beta.md).
- **Prior:** `v0.11.0-beta` Windows readiness and inspect surfaces
  (`1e07021a7`); `v0.10.0-beta` continuous journey honesty
  ([record](./plans/releases/v0.10.0-beta.md)); `v0.9.7-beta` first-session
  honesty.
- **Cadence:** current-minor patches when user signal warrants; next minor when
  a named capability is Ready. See
  [release-cadence policy](./docs/policies/release-cadence.md).
- **Active window:** **`v0.12.0-beta`** — remaining `/settings` work after
  SETPREF (Class A safe preferences) landed Done on the `v0.11.1-beta` tip.

---

## Active window — `v0.12.0-beta`

**Theme:** Close `/settings` v0.1 — the remaining settings work after SETPREF
landed on `v0.11.1-beta`.

**Status:** **Claim named.** SETPREF remains the locked next-minor headline;
SETPREF-001..006 Merged via #4741 on the `v0.11.1-beta` tip, so the module is
Done. This window is the remaining `/settings` work around it, not a second
write path.

**Customer one-liner:** Interface preferences you change in settings stay
changed, with a visible scope and a safe write.

**Authority:** Operator 2026-09-16 locked SETPREF as the next minor. The 0.11.1
tag then included the whole module; the remaining `/settings` work stays this
window (operator 2026-09-17).

**Minor scope:** complete `/settings` v0.1 through the settings service. No
Class B/C mutation (SETGOV). No public-docs claim that `anvil settings` is a
released product until CLICT-008 is Done.

### Primary claim

- **[SETPREF](./plans/modules/settings-safe-preferences.aps.md)** — Done:
  SETPREF-001..006 Merged via #4741 on `v0.11.1-beta`. What this window ships is
  the `/settings` v0.1 closeout around it: CLICT-008, then the public docs claim
  for `anvil settings`, a curated changelog, and the cut.

### Secondaries

- **CLICT-008** — re-audit `anvil settings --help` before public docs claim the
  family
  ([cli-command-truth review](./docs/reviews/cli-command-truth-review.md))
- SKPKG-010/011 may ride; do not silently become the headline
- Leftover CIB Ready items from the 0.11.1 drain (waive or park)

### Not a claim of this window

- **SHIPREP** — Draft; Ready checklist first
- **ABASE / SKOBS** — Draft; no authorised work items for ABASE
- **SETGOV / SETNL** — post-v0.1
- **Ember** — inactive; generation unavailable; historical read only under
  `ANVIL_EMBER=1`
- **`anvil impact` / `anvil plan` / `anvil dashboard --web`** — present but
  default-off / hidden from `--help`
- Intent conformance as a **product gate**
- Graph Trust Surfaces / council-gate bridge discovery
- Browser dashboard default-on
- CCTX as shipped product

### After this window

SHIPREP, ABASE, and SKOBS stay programme (Ready/ADR first). SETGOV stays later.

### Phase plan

| Phase          | Scope                                              | State                          |
| -------------- | -------------------------------------------------- | ------------------------------ |
| **0.11.1 tag** | Gate and inspect honesty                           | Tagged `e7024b7a6`             |
| **Claim lock** | SETPREF + `/settings` v0.1 closeout                | Locked                         |
| **Implement**  | Remaining `/settings` work (CLICT-008, docs claim) | SETPREF Done on 0.11.1 (#4741) |
| **Changelog**  | Curate `[Unreleased]`                              | Not started                    |
| **Cut**        | Preflight → prepare → readiness → tag              | Not started                    |

### Cut criteria

- Standing bar: full Cross matrix, release-readiness on source SHA,
  ACKNOWLEDGEMENTS fresh, dashboard openapi `check:api` green.
- Remaining `/settings` work (CLICT-008, public docs claim) Merged or waived in
  the cut note.
- Strategy: **direct** unless readiness forces stabilisation.

### Risks

| Risk                                      | Mitigation                                                    |
| ----------------------------------------- | ------------------------------------------------------------- |
| Double-claim persist that already shipped | Changelog 0.11.1 already names it; do not re-announce in 0.12 |
| SKPKG-010/011 become the headline         | Keep SETPREF as the locked claim                              |
| Hollow publish repeats                    | Verify assets before closeout                                 |

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
