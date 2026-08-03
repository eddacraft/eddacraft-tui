# anvil Release Plan

| Type         | Authority | Owner       | Status | Freshness                                                                                                                                                               |
| ------------ | --------- | ----------- | ------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Release plan | Derived   | APS modules | Live   | 2026-08-03: operator scoped **`v0.9.2-beta`** as a patch for the MCP 2.0 reconnect regression (MCP26-013). `v0.10.0-beta` stays the next unscoped minor after this cut. |

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

- **Latest tag:** `v0.9.1-beta` "Daily Path Polish and MCP 2.0 support"
  (published and verified 2026-08-02). Durable record:
  [`plans/releases/v0.9.1-beta.md`](./plans/releases/v0.9.1-beta.md). Prior:
  [`plans/releases/v0.9.0-beta.md`](./plans/releases/v0.9.0-beta.md).
- **Cadence:** minors cut when ready + gates green, not on a calendar. Current-
  minor patches land when user signal warrants (here: MCP client breakage after
  the dual-era host). See the
  [release-cadence policy](./docs/policies/release-cadence.md).
- **Active window:** **`v0.9.2-beta`** — patch on the v0.9 line for the **MCP
  2.0 reconnect bug** (MCP26-013). Operator override of assess's default
  `v0.10.0-beta` (semver minor from mixed `feat:` commits) because the product
  reason for the cut is a hotfix, not a new minor theme.

---

## Active window — `v0.9.2-beta` (MCP 2.0 reconnect patch)

**Theme:** MCP 2.0 reconnect — Codex and similar assistants work again after the
dual-era host rejected normal progress metadata.

**Primary claim (must ship, honest):**

- **MCP26-013** — restore legacy request-metadata interoperability so clients
  that send standard `_meta.progressToken` (and similar) are not rejected as if
  they spoke a newer reserved-metadata protocol. Merged via PR #3487. Customer-
  facing wording is already in `CHANGELOG.md` `[Unreleased]` under Fixed.

**Not a claim of this release:**

- Browser dashboard (`dashboard.web` remains default-off)
- Graph Trust Surfaces / CGBDG and other NBI tracks
- Full `rmcp` adoption (MCP26-012 stays Ready follow-up)
- A new minor product narrative (that is the later `v0.10.0-beta` window)

**Secondary carry-in already on `main` (ship if source SHA includes them; do not
reframe the release around them):**

- ACTTUI-018..021 — quiet re-run consent, start/status posture alignment, Prove
  honesty on the activation result screen (drafted in `[Unreleased]` Added /
  Changed)
- Acknowledgements starter kit 1.1.0 and ATTRIB CI hardening (operator / kit
  surfaces; not an anvil binary claim)
- Public docs rewrites for the already-shipped daily path

### Phase plan

| Phase                         | Scope                                                                                        | State           |
| ----------------------------- | -------------------------------------------------------------------------------------------- | --------------- |
| **0.9.1 publish + closeout**  | Publish, sign, verify, and record the `v0.9.1-beta` cut                                      | Done 2026-08-02 |
| **MCP reconnect land**        | MCP26-013 on `main` (PR #3487)                                                               | Done 2026-08-03 |
| **Scope patch + assess**      | Operator names `v0.9.2-beta` / MCP reconnect theme; assess base `v0.9.1-beta`                | Done 2026-08-03 |
| **Preflight → prepare → tag** | `preflight` (pre-prepare), publication token, `prepare`, readiness, pre-tag, `tag`, verify   | Next            |
| **Closeout**                  | Immutable `plans/releases/v0.9.2-beta.md`; prune this window; open next intake (likely 0.10) | After publish   |

### Cut criteria

- Standing base bar: full `Cross` matrix green (incl. Windows),
  `release-readiness.yml` pass on the source SHA, `ACKNOWLEDGEMENTS` fresh.
- **MCP26-013** is on the tagged SHA and covered by the stdio / metadata
  regression fixtures that landed with #3487.
- Claim honesty: headline is the MCP reconnect fix; do not market ACTTUI polish
  or kit work as the reason for the patch.
- Changelog promotion leads with the Fixed (Codex / MCP metadata) entry;
  secondary Unreleased bullets may ride along.
- Strategy: **`direct`** on a selected green `main` SHA (assess recommendation;
  no stabilisation branch unless preflight/readiness forces hardening).
- No second active window accreted into this file.

### Semver note

`assess.sh` proposes `v0.10.0-beta` because the window contains `feat:` commits.
Operator overrides to **`v0.9.2-beta`** (current-minor patch) — same pattern as
prior hotfix patches when the product reason is reconnect / friction, not a new
feature slate.

---

## Hotfix Iteration Plan (post-tag)

Releases are gated by quality (releasable `main`, green gates, APS
authorisation), not by a calendar.

| Cadence                | Channel                               | Scope                                                                 |
| ---------------------- | ------------------------------------- | --------------------------------------------------------------------- |
| Current-minor patch    | Weekly while user signal is non-empty | Bug fixes, false-positive reductions, doc corrections.                |
| Current-minor patch    | Within 48h of any P0 bug              | Crash, data loss, false-claim regression, daemon corruption.          |
| Next minor beta        | When ready — green gates + APS auth   | Feature additions. No calendar gate; cut when the slice is ready.     |
| Breaking beta or major | Demand-pulled                         | Driven by a real adopter requirement, not by completion of a backlog. |

Authoritative source:
[release-cadence policy](./docs/policies/release-cadence.md) (DISTRIB-004).

## Risks (active window)

| Risk                                                                            | Mitigation                                                                                                                                         |
| ------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------- |
| Release publication depends on a manually-rotated PAT (`ANVIL_RELEASES_TOKEN`). | Readiness runs `validate-publication-token.sh`; rotate per [`release-token-scope.md`](./docs/runbooks/release-token-scope.md) when the gate fails. |
| Mixed post-0.9.1 commits tempt an over-broad claim.                             | Keep the headline to MCP26-013; list ACTTUI/docs/kit only as secondary or omit from the customer summary.                                          |
| Assess default version (`v0.10.0-beta`) disagrees with operator patch.          | Document override in this plan and in the tracking issue; pass `--version v0.9.2-beta` to preflight/prepare.                                       |

## Records & roadmap

- **Shipped releases:** [`plans/releases/`](./plans/releases/) (per-tag
  records) + [`CHANGELOG.md`](./CHANGELOG.md).
- **Long-term direction / later windows** (incl. next minor after this patch,
  RMCPF full-port, enterprise/language expansion): [`ROADMAP.md`](./ROADMAP.md).
