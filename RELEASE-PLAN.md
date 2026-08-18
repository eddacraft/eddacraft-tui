# anvil Release Plan

| Type         | Authority | Owner       | Status | Freshness                                                                                                                                                                                                                              |
| ------------ | --------- | ----------- | ------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Release plan | Derived   | APS modules | Live   | 2026-08-18: **`v0.9.6-beta` claim locked** — beta field fixes after `v0.9.5-beta`, plus shared shell command-safety that grew out of that work. Override assess `v0.10.0-beta`. Not cut-ready until changelog curation + standing bar. |

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
- **Active window:** **`v0.9.6-beta`** — beta field fixes after live-heal, plus
  shared shell command-safety. Claim **locked** 2026-08-18. Patch on the v0.9
  line (assess defaulted to `v0.10.0-beta`; operator override keeps
  `0.9.6-beta`). Programme work (Graph Trust Surfaces Wave 0, `/settings`) runs
  **beside** this window and is not the cut claim.

---

## Active window — `v0.9.6-beta` (claim locked)

**Theme:** Beta field fixes after `v0.9.5-beta` — honest default hooks, quieter
daemon-down witness fallback, warning-flag escalation for surface checks, and
fewer false secret alarms on document filenames — plus the shared shell
command-safety rules that grew out of that field work.

**Status:** **Claim locked; not cut-ready.** Primary field fixes and the shell
extension are Merged on `main`. Remaining cut work is changelog curation,
standing bar, and preflight → prepare → readiness → tag.

**Customer one-liner:** After 0.9.5, hooks and warnings tell the truth again,
and shell scripts get the same dangerous-command coverage runtime already had.

**Authority:** Field intake from the 0.9.5 retest (default hooks, witness noise,
`--fail-on-warnings`, document-name entropy) plus the shell catalogue extension
that landed with that wave. Assess on `ac1bb9967495d96929b999561e107dc1022d8f8b`
recommended `v0.10.0-beta` / `direct` / `beta`; operator keeps **`v0.9.6-beta`**
as a named patch on the v0.9 line.

### Primary claim (beta field fixes)

| ID      | Item                                                        | Pri | State        | Notes                                     |
| ------- | ----------------------------------------------------------- | --- | ------------ | ----------------------------------------- |
| CIB-346 | Default hooks run the L3 witness path, not gate-only        | P1  | Merged #3982 | Install + start upgrade managed hooks     |
| CIB-345 | Hook witness fallback quiet when daemon is down             | P2  | Merged #3981 | Doctor / start --verify explain degraded  |
| CIB-347 | `--fail-on-warnings` escalates the four warn-only surfaces  | P1  | Merged #3980 | Default warn-only posture unchanged       |
| CIB-348 | Entropy does not fire on bare hyphenated document filenames | P3  | Merged #3979 | Opaque high-entropy values still reported |

### Extension that ships in this window (shell command-safety)

Grew out of the surface / command-safety field work; not a separate product
minor.

| ID         | Item                                             | Pri | State        | Notes                                            |
| ---------- | ------------------------------------------------ | --- | ------------ | ------------------------------------------------ |
| SURFSH-008 | Shared shell catalogue: pipe-to-shell, eval, 777 | P0  | Merged #3984 | Runtime Blocks pipe-to-shell; shell surface Warn |

### Not a claim of this window

- Live-heal supervisor/proxy soak (residual restart remains honest)
- Full **`/settings`** programme
- Graph Trust Surfaces / council-gate bridge discovery
- Intent-conformance product ADR
- Browser dashboard default-on
- Docs re-baseline migrations (authority + inventory already Merged; not a cut
  claim)
- Standing continuous-improvement drain beyond the primary rows above
- Unquoted-variable shell follow-ups

### Phase plan

| Phase              | Scope                                          | State                                      |
| ------------------ | ---------------------------------------------- | ------------------------------------------ |
| **0.9.5 closeout** | Record + APS advance + prune                   | Done 2026-08-17                            |
| **Field intake**   | Post-`v0.9.5-beta` signal → theme selection    | Done 2026-08-17 (retest + shell extension) |
| **Claim lock**     | Freeze primary/extension IDs for `v0.9.6-beta` | Done 2026-08-18 (this plan)                |
| **Implement**      | Primary field fixes + shell extension          | Done — claim rows Merged on `main`         |
| **Changelog**      | Curate `[Unreleased]` without APS ID bleed     | Next                                       |
| **Cut**            | Preflight → prepare → readiness → tag          | Not scheduled                              |

### Cut criteria

- Standing bar: full Cross matrix, release-readiness on source SHA,
  ACKNOWLEDGEMENTS fresh, dashboard openapi `check:api` green.
- Claim locked with Merged primary items (and extension Merged or waived).
- Changelog leads with the locked theme in customer language — no APS work-item
  IDs in theme, one-liner, or changelog entries.
- Strategy: **direct** unless readiness forces stabilisation.
- Version stays **`v0.9.6-beta`** (operator override of assess `v0.10.0-beta`).

### Risks

| Risk                                  | Mitigation                                                          |
| ------------------------------------- | ------------------------------------------------------------------- |
| Cutting without claim freeze          | Preflight/prepare blocked until RELEASE-PLAN status is claim-locked |
| Programme work mistaken for cut claim | Keep NBI / Not a claim list current at claim lock                   |
| APS IDs bleeding into public notes    | Curate changelog/theme in customer language only                    |
| Assess wants `v0.10.0-beta`           | Explicit override recorded above; prepare must use `0.9.6-beta`     |

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
