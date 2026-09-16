# anvil Release Plan

| Type         | Authority | Owner       | Status | Freshness                                                                                                                                                     |
| ------------ | --------- | ----------- | ------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Release plan | Derived   | APS modules | Live   | 2026-09-16: operator locked **SETPREF** as the `v0.12.0-beta` next-minor claim. Active window is **`v0.11.1-beta`** (CIB Ready honesty after `v0.11.0-beta`). |

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

- **Latest tag:** `v0.11.0-beta` "Windows readiness and inspect surfaces"
  (2026-09-15 on `1e07021a7`). Changelog: [`CHANGELOG.md`](./CHANGELOG.md).
  Per-tag record under `plans/releases/` is still owed (closeout gap).
- **Prior:** `v0.10.0-beta` continuous journey honesty
  ([record](./plans/releases/v0.10.0-beta.md)); `v0.9.7-beta` first-session
  honesty.
- **Cadence:** current-minor patches when user signal warrants; next minor when
  a named capability is Ready. See
  [release-cadence policy](./docs/policies/release-cadence.md).
- **Active window:** **`v0.11.1-beta`** — CIB Ready honesty after
  `v0.11.0-beta`.
- **After this window (not active):** **`v0.12.0-beta`** next-minor claim is
  **SETPREF** (Class A safe preferences; closes `/settings` v0.1). Do not open a
  second `## Active window` here.

---

## Active window — `v0.11.1-beta`

**Theme:** Gate and inspect honesty after `v0.11.0-beta`.

**Status:** **Claim named.** Primary ID frozen. Remaining Ready CIB items are
secondaries and may be waived at cut.

**Customer one-liner:** Secret detection and inspect tell the truth on the
languages and worktrees people actually use.

**Authority:** Operator 2026-09-16 — SETPREF locked as the next minor; this
patch drains the current CIB Ready set rather than starting Class A writes.

**Patch scope:** same behaviour intent, safer or clearer execution
([release-cadence](./docs/policies/release-cadence.md)). No new command, no
Class A mutation, no public-docs claim that `anvil settings` is a released
product until CLICT-008 is Done.

### Primary claim

- **[CIB-424](./plans/modules/continuous-improvement-backlog.aps.md)** — expand
  `anvil gate` secret-detection onto `.tsx`, `.jsx`, `.py`, `.go`, and `.sh`
  (Matt staff-portal / harvest B33). `check --all` discovery still omits
  `.go`/`.sh`; that is a residual, not this cut's claim.

### Secondaries

Current CIB Ready set besides the primary (waive at cut if still open). Merged
or superseded this window: CIB-425 (#4723), CIB-403 (#4722), CIB-404 (#4721),
CIB-401 (Superseded). Remaining:

- CIB-393 CI-only orphaned-socket flake
- CIB-202 flaky beacon reservation test
- CIB-209 worktree-safe local validation
- CIB-295 `aps` docs:check cannot express real APS drift
- CIB-296 `adr-integrity.test.sh` red on `main`
- CIB-333 scanner reports rule-token column
- CIB-334 `.anvil` compiler rejects stray rule-body H2
- CIB-330 WC-001 `usedforsecurity=False`
- CIB-204 Windows-only clippy backlog in anvil-intercept
- CIB-410 pin workspace Rust toolchain to 1.98.1
- **CLICT-008** — re-audit `anvil settings --help` before public docs claim the
  family
  ([cli-command-truth review](./docs/reviews/cli-command-truth-review.md))

### Not a claim of this window

- **SETPREF** — next-minor `v0.12.0-beta` claim; Class A writes
- **SHIPREP** — Draft; Ready checklist first
- **ABASE / SKOBS** — Draft; no authorised work items for ABASE
- **SKPKG-009 / 010 / 011** — new skill-install capability; park until 0.12 as
  secondary, do not finish silently on this patch
- **SETGOV / SETNL** — post-v0.1
- **Ember** — inactive; generation unavailable; historical read only under
  `ANVIL_EMBER=1`
- **`anvil impact` / `anvil plan` / `anvil dashboard --web`** — present but
  default-off / hidden from `--help`
- Intent conformance as a **product gate** — advisory `anvil conformance check`
  / CONF foundations; CEG remains Proposed
- Live-heal supervisor/proxy soak (residual restart remains honest)
- Graph Trust Surfaces / council-gate bridge discovery
- Browser dashboard default-on
- CCTX as shipped product (spike/advisory only)
- Standing CIB Proposed/Draft unless promoted to Ready
- CIB-353 tutorial depth (Draft editorial)
- First-run / docs prominence of telemetry disclosure (Elliot)
- Docs definition layer / DOCRB public-site programme
- Website decision-integrity redesign
- Graph attestation (GATT) as a headline

### After this window

`v0.12.0-beta` (minor, not this file's active window): **SETPREF** is the locked
headline — Class A interface preferences through the settings service, closing
`/settings` v0.1. The module is **Ready**. SKPKG-010/011 may ride as
secondaries. SHIPREP, ABASE, and SKOBS stay programme (Ready/ADR first). SETGOV
stays later.

### Phase plan

| Phase          | Scope                                  | State                                                                                                                                      |
| -------------- | -------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------ |
| **0.11.0 tag** | Windows readiness and inspect surfaces | Tagged                                                                                                                                     |
| **Claim lock** | CIB-424 primary; Ready CIB + CLICT-008 | Locked                                                                                                                                     |
| **Implement**  | Primary then secondaries               | Primary Merged (#4725); CIB-425 Merged (#4723); leftover Ready CIB waived at cut                                                           |
| **Changelog**  | Curate `[Unreleased]`                  | Done (this PR)                                                                                                                             |
| **Cut**        | Preflight → prepare → readiness → tag  | Prepare PR [#4748](https://github.com/eddacraft/anvil-001/pull/4748); tracking [#4749](https://github.com/eddacraft/anvil-001/issues/4749) |

### Cut criteria

- Standing bar: full Cross matrix, release-readiness on source SHA,
  ACKNOWLEDGEMENTS fresh, dashboard openapi `check:api` green.
- Primary CIB-424 Merged. Secondaries Merged or waived in the cut note.
- Changelog leads with the locked theme only — not SETPREF or skill-install
  freight.
- Strategy: **direct** unless readiness forces stabilisation.
- Version stays `v0.11.1-beta`.

### Risks

| Risk                                    | Mitigation                                                                     |
| --------------------------------------- | ------------------------------------------------------------------------------ |
| SETPREF starts in this patch window     | Keep Class A writes out of `v0.11.1-beta`; NBI ranks SETPREF as Ready for 0.12 |
| SKPKG-010/011 land as unclaimed freight | Park until 0.12; do not merge as 0.11.1 claim                                  |
| CIB Ready set is larger than the theme  | Primary is CIB-424; waive leftover secondaries at cut                          |
| Hollow publish repeats                  | Verify assets before closeout                                                  |

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
