# anvil Release Plan

| Type         | Authority | Owner       | Status | Freshness                                                                                                                                                                                                                                                                                          |
| ------------ | --------- | ----------- | ------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Release plan | Derived   | APS modules | Live   | 2026-08-09: **`v0.9.3-beta` shipped**. Active window provisional **`v0.9.4-beta`** — product delta on main is install-receipt honesty (**CIB-315**) + durable-membership wait (#3700); pre-release short pass found no cut-blockers. Claim draft locked below; freeze when operator schedules cut. |

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

- **Latest tag:** `v0.9.3-beta` "Honesty and Windows path" (2026-08-07 on
  `cfe0857cb`; binaries + signing published). Record:
  [`plans/releases/v0.9.3-beta.md`](./plans/releases/v0.9.3-beta.md). Public:
  https://github.com/eddacraft/anvil/releases/tag/v0.9.3-beta
- **Prior:** `v0.9.2-beta` MCP reconnect; `v0.9.1-beta` daily path + MCP 2.0.
- **Cadence:** current-minor patches when user signal warrants. See
  [release-cadence policy](./docs/policies/release-cadence.md).
- **Active window:** provisional **`v0.9.4-beta`** — field follow-up on the v0.9
  line. **Claim draft** below (operator freeze still required before cut).
  Programme work (Graph Trust Surfaces Wave 0) runs **beside** this window and
  is not the cut claim.

---

## Active window — `v0.9.4-beta` (provisional — field follow-up)

**Theme:** Post-`v0.9.3-beta` field honesty on the v0.9 line — install-method
truth on Windows/macOS, plus registration confirmation that does not race the
daemon. Not a new product minor.

**Status:** **Implement done on main; claim draft locked; cut not scheduled.**
Pre-release short pass (2026-08-09) found no cut-blocking product defects in
`v0.9.3-beta..main`. Standing base bar (Cross matrix, readiness,
ACKNOWLEDGEMENTS, openapi) still required at freeze/cut.

### Claim draft (operator freeze before cut)

| ID / ref    | Item                                                                                       | Pri | State on main                    |
| ----------- | ------------------------------------------------------------------------------------------ | --- | -------------------------------- |
| **CIB-315** | Install-receipt roots + platform-correct upgrade advice (Windows/macOS cargo-dist honesty) | P1  | Merged #3698 (+ #3703 pin tests) |
| **#3700**   | Wait for durable membership after register ack (false CIB-252 refusal race)                | P2  | Merged #3700                     |
| **CIB-305** | Concurrent CI-log tracked writers (internal data-loss)                                     | P0  | Draft — **not** in product claim |

### Already shipped — do not re-claim

| ID          | Item                                  | Notes                                                               |
| ----------- | ------------------------------------- | ------------------------------------------------------------------- |
| **CIB-281** | `audit` security scope on TUI + SARIF | Merged #3652; code in `v0.9.3-beta` tree; formal claim list omitted |

### Not a claim of this release

- Graph Trust Surfaces programme / **CGBDG** discovery (Wave 0 side track — see
  NBI and
  [`plans/specs/2026-07-28-graph-trust-surfaces.md`](./plans/specs/2026-07-28-graph-trust-surfaces.md))
- **CONF-001** intent-conformance ADR (Schedule; programme unlocker)
- Browser dashboard default-on / Team-Lead Surface (`v0.10.0-beta` when scoped)
- Full `rmcp` adoption (MCP26-012)
- **CIB-305** and other clawpatch Drafts (internal / promote separately)

### Phase plan

| Phase              | Scope                                   | State                                                |
| ------------------ | --------------------------------------- | ---------------------------------------------------- |
| **0.9.3 closeout** | Record + APS advance + prune            | Done 2026-08-09                                      |
| **Field intake**   | Post-0.9.3 beta signal; triage into CIB | Done for CIB-315 + #3700; further optional           |
| **Claim lock**     | Freeze primary/secondary IDs for v0.9.4 | **Draft locked 2026-08-09** — operator freeze at cut |
| **Implement**      | Ready claim items                       | Done on main (CIB-315, #3700)                        |
| **Cut**            | Preflight → prepare → readiness → tag   | Not scheduled                                        |

### Cut criteria (when claim locks)

- Standing base bar: full Cross matrix, release-readiness on source SHA,
  ACKNOWLEDGEMENTS fresh, dashboard openapi `check:api` green.
- Every **primary** claim item Merged with validation evidence (or explicit
  waive with issue).
- Changelog leads with user-visible honesty / field fixes, not new features.
- Strategy: **direct** unless readiness forces stabilisation.

### Risks

| Risk                                      | Mitigation                                                                |
| ----------------------------------------- | ------------------------------------------------------------------------- |
| Placeholder window drifts without a claim | Re-scope within one field cycle or drop to "no cut" and stay on main only |
| Programme work mistaken for cut work      | NBI keeps CGBDG/CONF as programme; this plan lists them under Not a claim |

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
