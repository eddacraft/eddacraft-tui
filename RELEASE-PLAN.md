# anvil Release Plan

| Type         | Authority | Owner       | Status | Freshness                                                                                                                                                     |
| ------------ | --------- | ----------- | ------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Release plan | Derived   | APS modules | Live   | 2026-08-04: **`v0.9.3-beta` honesty pass** scoped from Morgan Deus 0.9.1 findings (CIB-220..227 / #3510). Prior cut `v0.9.2-beta` MCP reconnect is published. |

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

- **Latest tag:** `v0.9.2-beta` "MCP 2.0 reconnect" (retagged 2026-08-03 on
  `22f6a9bec` after openapi dashboard fix; binaries + signing published). Prior
  headline: `v0.9.1-beta` daily path + MCP 2.0. Records under
  [`plans/releases/`](./plans/releases/) when closeout writes them; public
  release: https://github.com/eddacraft/anvil/releases/tag/v0.9.2-beta
- **Cadence:** current-minor patches when user signal warrants. See
  [release-cadence policy](./docs/policies/release-cadence.md).
- **Active window:** **`v0.9.3-beta`** — honesty pass on the daily path (Morgan
  Deus findings from 0.9.1). Not a new product theme; not Graph Trust Surfaces /
  dashboard.

---

## Active window — `v0.9.3-beta` (honesty pass)

**Theme:** Daily-path honesty — what anvil says matches what it does (MCP scope,
auth, value receipt, flags, multi-client docs).

**Tracking:** [#3510](https://github.com/eddacraft/anvil-001/issues/3510) · APS
**CIB-220..227**

### Primary claim (must ship)

| ID          | Item                                                                                                                                   | Pri |
| ----------- | -------------------------------------------------------------------------------------------------------------------------------------- | --- |
| **CIB-220** | Interactive `anvil start --mcp-scope project` installs / offers MCP; never claims "MCP installation disabled" solely for project scope | P0  |
| **CIB-221** | No false `anvil auth login` nag for already-authenticated / pro users                                                                  | P1  |
| **CIB-222** | Start value receipt discloses machine-wide vs repo-scoped evidence                                                                     | P1  |

### Secondary claim (should ship in the same cut if cheap)

| ID          | Item                                                           | Pri |
| ----------- | -------------------------------------------------------------- | --- |
| **CIB-224** | `--no-mcp` + `--mcp-client` / `--all-mcp-clients` fails loudly | P2  |
| **CIB-227** | User-facing copy not exclusive to Claude Code + Cursor         | P2  |
| **CIB-223** | Non-git init vs no-worktree messaging coherent                 | P2  |

### Nice-to-have in this cut (else next)

| ID          | Item                                        | Pri |
| ----------- | ------------------------------------------- | --- |
| **CIB-225** | `--format` warns when config already exists | P3  |
| **CIB-226** | Public CLI docs: flags + auth exit code 3   | P3  |

### Not a claim of this release

- Browser dashboard default-on
- Graph Trust Surfaces / CGBDG programme
- Full `rmcp` adoption (MCP26-012)
- New feature narrative (that remains a later minor / `v0.10.0-beta` when
  scoped)

### Phase plan

| Phase               | Scope                                        | State                       |
| ------------------- | -------------------------------------------- | --------------------------- |
| **0.9.2 publish**   | MCP reconnect + openapi retag                | Done 2026-08-03             |
| **Intake honesty**  | File CIB-220..227 + #3510; scope this window | Done 2026-08-04             |
| **P0/P1 implement** | CIB-220, 221, 222                            | Next                        |
| **P2 implement**    | CIB-223, 224, 227                            | Same cut if unblocked       |
| **P3**              | CIB-225, 226                                 | Same cut or follow-up patch |
| **Cut**             | Preflight → prepare → readiness → tag        | After claim green           |

### Cut criteria

- Standing base bar: full Cross matrix, release-readiness on source SHA,
  ACKNOWLEDGEMENTS fresh, dashboard openapi `check:api` green (0.9.2 lesson).
- **CIB-220** validated (TUI project-scope MCP path).
- **CIB-221** and **CIB-222** validated (or explicit waive with issue).
- Claim honesty: changelog leads with honesty fixes, not new features.
- Strategy: **direct** unless readiness forces stabilisation.
- Prepare regenerates dashboard openapi when version bumps (avoid 0.9.2 retag
  class).

### Risks

| Risk                                           | Mitigation                                                                                   |
| ---------------------------------------------- | -------------------------------------------------------------------------------------------- |
| Project-scope MCP reopens installer edge cases | Prefer thin path: stop `Skip` for project; reuse scope-aware installer already used headless |
| Auth false-negative hides real login need      | Fixture matrix: no token / expired / valid / pro                                             |
| Scope copy on value line confuses              | One short parenthetical; match insights scorecard language                                   |

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
