# anvil Release Plan

| Type         | Authority | Owner       | Status | Freshness                                                                                                                                                                                                             |
| ------------ | --------- | ----------- | ------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Release plan | Derived   | APS modules | Live   | 2026-08-02: `v0.9.1-beta` tagged (publish recovery in flight). Active window advanced to **`v0.10.0-beta`** placeholder — full scope at closeout of the 0.9.1 cut. Prior preferred patch theme recorded in CHANGELOG. |

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

- **Latest tag:** `v0.9.1-beta` "Daily Path Polish and MCP 2.0 support" (tagged
  2026-08-02; cargo-dist publish recovery after dashboard render prebuild + ACK
  ARG_MAX fixes). Preferred claim was the **daily path** (bare ensure,
  activation TUI default, gate honesty) plus **MCP 2.0 support** (twelve-client
  install, dual-era protocol). Dashboard remained flag-gated and was not a
  claim. Durable record will land at
  [`plans/releases/v0.9.1-beta.md`](./plans/releases/v0.9.1-beta.md) on
  closeout. Prior:
  [`plans/releases/v0.9.0-beta.md`](./plans/releases/v0.9.0-beta.md).
- **Cadence:** minors cut when ready + gates green, not on a calendar. See the
  [release-cadence policy](./docs/policies/release-cadence.md).
- **Active window:** **`v0.10.0-beta`** — open placeholder after the `v0.9.1`
  tag. Theme and phase plans are **TBD at closeout** of the 0.9.1 publish
  recovery (do not invent a second narrative while the tag is still landing).

---

## Active window — `v0.10.0-beta` (placeholder)

**Operator note 2026-08-02** — `v0.9.1-beta` is already a git tag. This file
must not keep that version as the active window (docs `release-plan` surface).
Scope the next minor here once 0.9.1 publish + closeout finish; until then this
window is a deliberate placeholder so CI stays forward-looking.

### Phase plan

| Phase                       | Scope                                                                                         | State                                 |
| --------------------------- | --------------------------------------------------------------------------------------------- | ------------------------------------- |
| **0.9.1 publish recovery**  | Dashboard `@eddacraft/render` prebuild on tag; ACK Contents API ARG_MAX fix; cargo-dist green | In progress via release recovery PRs  |
| **0.9.1 closeout**          | Release record, tracking issue, APS Released/Shipped advance, prune narrative into ROADMAP    | Blocked on successful publish         |
| **Scope next minor** (gate) | Operator names theme, carry-in modules, and cut criteria for `v0.10.0-beta`                   | Not started — wait for 0.9.1 closeout |

### Cut criteria

- Standing base bar: full `Cross` matrix green (incl. Windows),
  `release-readiness.yml` pass on the source SHA, `ACKNOWLEDGEMENTS` fresh.
- Theme, claim honesty, and module carry-in named before prepare.
- No second active window accreted into this file.

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
| Tag exists while cargo-dist publish is incomplete — installers may be partial.  | Finish publish recovery before marketing the tag; verify private + public assets and signing before closeout.                                      |
| Release publication depends on a manually-rotated PAT (`ANVIL_RELEASES_TOKEN`). | Readiness runs `validate-publication-token.sh`; rotate per [`release-token-scope.md`](./docs/runbooks/release-token-scope.md) when the gate fails. |
| Placeholder active window rots without a real scope.                            | Closeout of 0.9.1 must replace this section with a named theme and phase plan (or an explicit "intake hold" note with owner).                      |

## Records & roadmap

- **Shipped releases:** [`plans/releases/`](./plans/releases/) (per-tag
  records) + [`CHANGELOG.md`](./CHANGELOG.md).
- **Long-term direction / later windows** (incl. RMCPF Rust MCP full-port,
  enterprise/language expansion): [`ROADMAP.md`](./ROADMAP.md).
