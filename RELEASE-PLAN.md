# anvil Release Plan

| Type         | Authority | Owner       | Status | Freshness                                                                                                                                                                                               |
| ------------ | --------- | ----------- | ------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Release plan | Derived   | APS modules | Live   | 2026-08-13: **`v0.9.5-beta` claim locked** — config unification and product deep clean (UCFG-001..014 primary; honesty/trust secondaries Merged on main). Not cut-ready until changelog + standing bar. |

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
- **Active window:** **`v0.9.5-beta`** — config unification and product deep
  clean. Claim **locked** 2026-08-13. Patch on the v0.9 line (not a new product
  minor). Programme work (Graph Trust Surfaces Wave 0, `/settings` SETCON+) runs
  **beside** this window and is not the cut claim.

---

## Active window — `v0.9.5-beta` (claim locked)

**Theme:** Config unification and product deep clean — one project config
surface that matches what commands actually do, plus honesty and edge fixes so
install, protect claims, and verify mean what they say.

**Status:** **Claim locked; not cut-ready.** Primary and secondary claim items
are Merged on `main`. Remaining cut work is changelog curation, standing base
bar, and preflight → prepare → readiness → tag.

**Customer one-liner:** One config file story that matches reality, plus a pass
of honesty and edge fixes around protection, install, and verify.

**Authority:** [ADR-120](./plans/decisions/120-config-surface-consolidation.md)
(config surface consolidation);
[UCFG](./plans/modules/unified-config-format.aps.md) (UCFG-001..014 all Merged).
Settings programme (SETCON/SETINS/SETPREF) is **not** this claim — it consumes
UCFG later.

### Primary claim (config unification)

| ID       | Item                                                  | Pri | State        | Notes                                      |
| -------- | ----------------------------------------------------- | --- | ------------ | ------------------------------------------ |
| UCFG-001 | `anvil init` writes canonical `.anvil.<ext>`          | P0  | Merged #3841 | No new `.anvilrc` creation paths           |
| UCFG-002 | migrate renames `.anvilrc`; doctor flags dual configs | P0  | Merged #3847 | Discover-first single precedence           |
| UCFG-003 | snake_case canonical key space                        | P0  | Merged #3824 | camelCase accepted on read                 |
| UCFG-004 | `gate` section schema in main config                  | P0  | Merged #3832 | Authoritative gate composition             |
| UCFG-005 | `gate-config` re-pointed; legacy JSON fold            | P0  | Merged #3834 | `.anvil/gate-config.json` retired as store |
| UCFG-006 | `SectionOrSource<T>` delegation                       | P0  | Merged #3833 | Path-safe one-level source                 |
| UCFG-007 | `architecture` section + migrate                      | P0  | Merged #3835 | Inline or delegated                        |
| UCFG-008 | gate / watch / architecture read resolved section     | P0  | Merged #3836 | Commands share resolution                  |
| UCFG-009 | Policy discovery via `anvil_config::discover`         | P0  | Merged #3831 | hook / l4_validate unified                 |
| UCFG-010 | MCP resources, config summary, doctor unified         | P0  | Merged #3852 | One surface for inspect                    |
| UCFG-011 | Documentation sweep to canonical name                 | P0  | Merged #3851 | Public docs / skill / runbook              |
| UCFG-012 | Fixture and CI sweep                                  | P0  | Merged #3848 | Tests pin canonical layout                 |
| UCFG-013 | Watch-time architecture for section configs           | P0  | Merged #3867 | Inline + delegated + standalone            |
| UCFG-014 | Descriptor-bound guard in bounded reader              | P0  | Merged #3867 | No FIFO hang on config open                |

Module disposition at claim lock: **UCFG Done (14/14 Merged)** — release
evidence still owed for Released/Shipped → Complete.

### Secondary claim (product deep clean)

Honesty and trust residuals landed on `main` in the same post-`0.9.4` window.
Ship in this tag so the config story is not undercut by false protect/install
claims.

| ID            | Item                                           | Pri | State        | Notes                                      |
| ------------- | ---------------------------------------------- | --- | ------------ | ------------------------------------------ |
| Deep-MCP-path | Claude workspace MCP → `.mcp.json`             | P0  | Merged       | Project scope writes the file Claude loads |
| Deep-LiveVal  | LiveValidation attributed per MCP client       | P0  | Merged #3866 | No mass-promote of unrelated clients       |
| Deep-Capsule  | Capsule verify binds witness to manifest range | P0  | Merged       | Range mismatch refuses verify              |
| Deep-Refresh  | Serialise refresh-token rotation               | P1  | Merged       | Exclusive lock across load/exchange/save   |
| Deep-PolicyTO | Policy suite timeout kills process group       | P1  | Merged       | No hang on grandchild pipe holds           |
| Deep-Review   | Post-release review defect batch               | P1  | Merged #3863 | Residual CLI honesty edges                 |

### Not a claim of this release

- Full **`/settings`** programme (**SETCON** / **SETINS** / **SETPREF** /
  **SETGOV** / **SETNL**) — foundation and inspect UI after UCFG; no
  `anvil settings` product claim
- Graph Trust Surfaces programme / **CGBDG** discovery (Wave 0 side track — see
  NBI and
  [`plans/specs/2026-07-28-graph-trust-surfaces.md`](./plans/specs/2026-07-28-graph-trust-surfaces.md))
- **CONF-001** intent-conformance ADR (Schedule; programme unlocker)
- Browser dashboard default-on / Team-Lead Surface (later minor when scoped)
- Full `rmcp` adoption (MCP26-012)
- **MCPLH** MCP live-heal without harness restart (Ready programme; not this
  cut)
- **FEFF** field-effectiveness evidence programme
- Standing CIB drain and Draft follow-ups (e.g. CIB-317/318)

### Phase plan

| Phase              | Scope                                          | State                                                       |
| ------------------ | ---------------------------------------------- | ----------------------------------------------------------- |
| **0.9.4 closeout** | Record + APS advance + prune                   | Done 2026-08-10                                             |
| **Field intake**   | Post-`v0.9.4-beta` signal → theme selection    | Done 2026-08-13 (operator: config unification + deep clean) |
| **Claim lock**     | Freeze primary/secondary IDs for `v0.9.5-beta` | Done 2026-08-13 (this plan)                                 |
| **Implement**      | Primary UCFG-001..014 + secondary deep clean   | Done — all claim rows Merged on `main`                      |
| **Changelog**      | Curate `[Unreleased]` for config + honesty     | Next                                                        |
| **Cut**            | Preflight → prepare → readiness → tag          | Not scheduled                                               |

### Cut criteria

- Standing base bar: full Cross matrix, release-readiness on source SHA,
  ACKNOWLEDGEMENTS fresh, dashboard openapi `check:api` green.
- Every **primary** claim item Merged with validation evidence (satisfied at
  claim lock; re-confirm on source SHA at preflight).
- Every **secondary** claim item Merged or explicitly waived with issue.
- Changelog leads with **config unification** (canonical file, migrate/doctor,
  gate/architecture sections) then **honesty/deep-clean** fixes — not programme
  work from Not a claim.
- Strategy: **direct** unless readiness forces stabilisation.
- APS closeout: advance UCFG items to Released/Shipped; roll this file to the
  next provisional window after tag.

### Risks

| Risk                                            | Mitigation                                                                                                |
| ----------------------------------------------- | --------------------------------------------------------------------------------------------------------- |
| Claiming "settings" when SETCON has not shipped | Theme and customer copy say **config** unification; settings modules listed under Not a claim             |
| Changelog under-reports 350+ main commits       | Curate only user-visible claim rows; rest stays Engineering History                                       |
| Programme work mistaken for cut work            | NBI keeps CGBDG/MCPLH/FEFF/CONF as programme; this plan lists them under Not a claim                      |
| Legacy dual-config still in the wild            | migrate + doctor dual-truth warnings ship in claim; legacy **read** fallbacks retained ≥1 minor (ADR-120) |

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
