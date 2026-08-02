# anvil Release Plan

| Type         | Authority | Owner       | Status | Freshness                                                                                                                                                                                                                                                                                    |
| ------------ | --------- | ----------- | ------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Release plan | Derived   | APS modules | Live   | Pre-release started 2026-08-02: preferred tag **`v0.9.1-beta`** "Daily Path Polish and MCP 2.0 support" (operator). Fallback if patch framing is rejected: **`v0.10.0-beta`** "Daily Path Upgrade and MCP 2.0 support" — same content. Prior window was scoped as `v0.10.0-beta` 2026-07-13. |

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

- **Latest tag:** `v0.9.0-beta` "First-Run Wins and the Assistant Graph"
  (shipped 2026-07-12) — the assistant-facing graph over MCP (GCTX 14/14,
  ADR-083), warm-start persistence (ADR-069 + the ADR-105 base+delta store,
  `ANVIL_PERSIST_GRAPH` default-on), the MCP-optional useful daemon (ACTMO + the
  ADR-101 headless save-time driver), USAGE analytics, and the JOURNEY-conducted
  first-run/daily-confidence cut. Record:
  [`plans/releases/v0.9.0-beta.md`](./plans/releases/v0.9.0-beta.md).
  (`v0.8.1-beta` "Headless GitHub Login" record:
  [`plans/releases/v0.8.1-beta.md`](./plans/releases/v0.8.1-beta.md).)
- **Cadence:** minors cut when ready + gates green, not on a calendar. See the
  [release-cadence policy](./docs/policies/release-cadence.md).
- **Active window:** **`v0.9.1-beta`** "Daily Path Polish and MCP 2.0 support"
  (operator 2026-08-02). Same payload as the earlier multi-harness/daily-ensure
  window, reframed as a **patch on the v0.9 line** so the upgrade stays honest:
  bare `anvil`, default activation TUI, multi-client MCP + ratified protocol
  support, managed skills, gate honesty. Flag-gated dashboard rides along and is
  **not** a claim. **Fallback tag** if patch framing gets massive pushback:
  `v0.10.0-beta` "Daily Path Upgrade and MCP 2.0 support" (identical content).
  Records for `v0.6.x`–`v0.9.x` are in [`plans/releases/`](./plans/releases/).

---

## Active window — `v0.9.1-beta` "Daily Path Polish and MCP 2.0 support"

**Operator decision 2026-08-02** — cut preferred as a **v0.9.x patch** so we do
not oversell. Customer claim is the **daily path** (bare ensure + activation TUI
default + gate warnings-over-blocks) plus **MCP 2.0 support** (multi-client
install, dual-era protocol including the 2026-07-28 era, managed skills). The
earlier "Team-Lead Surface Foundations" and "Multi-Harness MCP and Daily Ensure"
titles are retired for this cut. Browser dashboard foundation (DASH/DASHCORE)
remains **merged and flag-gated** for internal testing only — not a release
claim.

**Fallback:** if patch-vs-minor framing is rejected at review, retag the same
SHA as `v0.10.0-beta` "Daily Path Upgrade and MCP 2.0 support" without changing
scope.

Primary delivery for this window:

- **Bare ensure (ONSW / JOURNEY-011 / ADR-114)** — bare `anvil` as the daily
  on-switch after activation; `anvil start` remains activate/reconfigure.
- **Activation polish** — interactive `anvil start` opens the consent-first TUI
  by default; celebration / quiet healthy repeats; related post-`v0.9` path
  work.
- **MCP 2.0 support** — twelve `anvil mcp install --client` targets; interactive
  start offers the full registry (consent-first); ratified MCP `2026-07-28` with
  sealed legacy eras retained; managed skill install + doctor freshness.
- **Gate honesty** — warnings-over-blocks restored; must-block crypto (WC-002 /
  WC-003) still errors.

MCPX and SKPKG merged to `main` in #3328 on 2026-07-15 — three days after the
`v0.9.0-beta` cut — and therefore in no tag yet. As of `v0.9.0-beta`,
`anvil mcp install --client` accepts two clients; on `main` it accepts twelve.
Naming that carry-in makes cut evidence real.

DASH-001..012 and DASHCORE-001..009 are Merged and may ride the tag as
flag-gated foundation only (`dashboard.web` default-off). They are not
user-facing highlights for this release. JOURNEY-009 remains on hold;
JOURNEY-010 remains blocked on later DASH view waves.

### Phase plan

| Phase                                | Scope                                                                                                                                                                | State                                                                                                                                                        |
| ------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| **Scope confirmation** (gate)        | Operator confirms preferred tag/theme (`v0.9.1-beta` daily path polish + MCP 2.0; fallback `v0.10.0-beta`) and that DASH is not a claim.                             | **done 2026-08-02** — preferred `v0.9.1-beta`; fallback on pushback only                                                                                     |
| **v0.9 closeout hygiene**            | All-Merged modules included in `v0.9.0-beta` advance to `Released/Shipped` + archive (own PRs, per the APS archive cascade); CIB intake from the cut log.            | done 2026-07-13 — release record + tracking-issue closeout 2026-07-12; 17 tag-complete modules archived via the cascade PR                                   |
| **MCP 2.0 / MCPX/SKPKG carry-in**    | Multi-harness MCP install (twelve `--client` targets) + dual-era protocol + skill packaging; interactive start offers full registry; verify at cut.                  | merged 2026-07-15 via #3328 — in no tag; rides this window; start multi-offer extended 2026-08-01                                                            |
| **Bare ensure** (ONSW / JOURNEY-011) | Bare `anvil` daily ensure vs `anvil start` reconfigure (ADR-114). First-class public docs for the daily path.                                                        | ONSW-001..006 + JOURNEY-011 Merged via #3474; public docs aligned to daily path                                                                              |
| **JOURNEY post-cut expansion**       | JOURNEY-007..010 coordinated enhancements. JOURNEY-011 closed with ONSW.                                                                                             | JOURNEY-007 + WOW-006 Merged via #3441; JOURNEY-008 Merged #3408; JOURNEY-009 on hold; JOURNEY-010 blocked on later DASH waves; **JOURNEY-011 Merged #3474** |
| **Dashboard foundation** (DASH)      | Flag-gated foundation only — not a release claim.                                                                                                                    | all 12 Merged (PRs #3261, #3321, #3421); `dashboard.web` default-off                                                                                         |
| **Dashboard core views** (DASHCORE)  | Flag-gated core routes only — not a release claim.                                                                                                                   | 9/9 Merged (PR #3363, PR #3379, PR #3436)                                                                                                                    |
| **v0.9 follow-through** (lane)       | Beta-signal fixes on the shipped first-run/daemon/graph surfaces (48h-P0 patch lane on `v0.9.x`); CIB-193/-194/-195/-196 and the release-recovery hardening (#3309). | all closed: CIB-193/-194/-195/-196 merged (2026-07-12, 2026-07-26 via #3422) and #3309 merged 2026-07-12; the lane stays the vehicle for anything urgent     |
| **Pre-release**                      | assess → preflight → prepare → readiness → pre-tag review → tag. Candidate SHA frozen on a stabilisation or prep branch as needed.                                   | **started 2026-08-02** — assess green on `main` `b971f4ea7`; preflight next                                                                                  |

### Cut criteria

- The standing base bar: full `Cross` matrix green (incl. Windows),
  `release-readiness.yml` pass on the source SHA, `ACKNOWLEDGEMENTS` fresh.
- ADR-031 latency gate (GV2-025 CI job) stays green.
- Bare ensure: public docs + help describe bare `anvil` vs `anvil start`;
  `cargo test -p eddacraft-anvil --test bare_invocation` green; JOURNEY-011
  evidence recorded.
- MCP 2.0 carry-in is verified **from the release artefact, not from `main`**:
  `anvil mcp install --client <client> --verify` succeeds for a non-Cursor,
  non-Claude-Code client using the tagged binary. Reading `--help` on a stale
  installed binary is what hid the gap across the whole `v0.9.0-beta` window.
- Interactive `anvil start` offers the full MCP registry (consent-first).
- Changelog title matches the chosen tag (`v0.9.1-beta` preferred;
  `v0.10.0-beta` only if fallback is taken).

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

| Risk                                                                                                                                                                                                            | Mitigation                                                                                                                                                                                                                                                                                                                                 |
| --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Default-on graph persistence and the first-run surfaces reach every upgrader — field defects surface now.                                                                                                       | The `v0.9.x` 48h-P0 patch lane; `ANVIL_PERSIST_GRAPH=0` and `ANVIL_WATCH_DAEMON=0` documented opt-outs; CIB intake from beta signal.                                                                                                                                                                                                       |
| Release publication depends on a manually-rotated PAT (`ANVIL_RELEASES_TOKEN`) — the v0.9 cut stalled ~6h on its expiry.                                                                                        | Closed on both halves: #3309 (merged 2026-07-12) hardened recovery after a failed publication, and `validate-publication-token.sh` now fails the readiness gate when the credential is absent, rejected, unpermitted, or expiring within 14 days — so the v0.9 failure mode is caught before a cut starts rather than at the publish step. |
| The window accretes (this document rots back into a historical record).                                                                                                                                         | "How this document works" + the closeout prune step keep it to one active window.                                                                                                                                                                                                                                                          |
| Work merged between a cut and the next window's scoping is invisible here, so a module can read `Done` while delivering nothing to users (MCPX/SKPKG, merged 2026-07-15, untagged since the `v0.9.0-beta` cut). | Name merged-but-untagged modules as explicit carry-in when a window is scoped; verify them from the release artefact at cut, not from `main`.                                                                                                                                                                                              |

## Records & roadmap

- **Shipped releases:** [`plans/releases/`](./plans/releases/) (per-tag
  records) + [`CHANGELOG.md`](./CHANGELOG.md).
- **Long-term direction / later windows** (incl. RMCPF Rust MCP full-port,
  enterprise/language expansion): [`ROADMAP.md`](./ROADMAP.md).
