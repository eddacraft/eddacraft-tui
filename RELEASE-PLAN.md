# anvil Release Plan

| Type         | Authority | Owner       | Status | Freshness                                                                                                                                                                                                                                                    |
| ------------ | --------- | ----------- | ------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Release plan | Derived   | APS modules | Live   | 2026-08-10: **`v0.9.4-beta` shipped** (clearer install advice and quieter false alarms). Closeout hygiene: durable record under `plans/releases/`. **Active window** is provisional **`v0.9.5-beta`** (field intake) — not cut-ready until claim solidifies. |

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

- **Latest tag:** `v0.9.4-beta` "Clearer install advice and quieter false
  alarms" (2026-08-10 on `165d33dfb`). Record:
  [`plans/releases/v0.9.4-beta.md`](./plans/releases/v0.9.4-beta.md).
- **Prior:** `v0.9.3-beta` honesty + Windows path
  ([record](./plans/releases/v0.9.3-beta.md)); `v0.9.2-beta` MCP reconnect;
  `v0.9.1-beta` daily path + MCP 2.0.
- **Cadence:** current-minor patches when user signal warrants. See
  [release-cadence policy](./docs/policies/release-cadence.md).
- **Active window:** provisional **`v0.9.5-beta`** — post-`v0.9.4-beta` field
  intake; claim not frozen. Programme work (Graph Trust Surfaces Wave 0) runs
  **beside** this window and is not the cut claim.

---

## Active window — `v0.9.5-beta` (provisional — field intake)

**Theme:** Post-`v0.9.4-beta` field signal on the v0.9 line. Not a new product
minor. Re-scope or promote to a named claim when operator signal warrants a cut.

**Status:** **Not cut-ready.** Placeholder after 0.9.4 closeout so
`RELEASE-PLAN.md` stays forward-looking (one active, untagged window).

### Claim draft (operator freeze before cut)

| ID        | Item                          | Pri | Notes                               |
| --------- | ----------------------------- | --- | ----------------------------------- |
| Field TBD | Post-`v0.9.4-beta` feedback   | —   | Intake before locking primary claim |
| Carry-in  | Unshipped `v0.9.4-beta` claim | —   | Re-triage at intake; do not assume  |

> Carry-in is deliberately unenumerated: which `v0.9.4-beta` candidates shipped
> is only known from the cut, so listing IDs here before closeout would assert
> what the record has not yet established.

### Not a claim of this release

- Graph Trust Surfaces programme / **CGBDG** discovery (Wave 0 side track — see
  NBI and
  [`plans/specs/2026-07-28-graph-trust-surfaces.md`](./plans/specs/2026-07-28-graph-trust-surfaces.md))
- **CONF-001** intent-conformance ADR (Schedule; programme unlocker)
- Browser dashboard default-on / Team-Lead Surface (`v0.10.0-beta` when scoped)
- Full `rmcp` adoption (MCP26-012)
- **CIB-305** and other clawpatch Drafts (internal / promote separately)

### Phase plan

| Phase              | Scope                                          | State                          |
| ------------------ | ---------------------------------------------- | ------------------------------ |
| **0.9.4 closeout** | Record + APS advance + prune                   | Done 2026-08-10 (this hygiene) |
| **Field intake**   | Post-`v0.9.4-beta` signal; triage into CIB     | Next                           |
| **Claim lock**     | Freeze primary/secondary IDs for `v0.9.5-beta` | Blocked on intake              |
| **Implement**      | Ready claim items                              | Not started                    |
| **Cut**            | Preflight → prepare → readiness → tag          | Not scheduled                  |

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
