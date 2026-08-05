# anvil Release Plan

| Type         | Authority | Owner       | Status | Freshness                                                                                                                                                                                                                                                                                                |
| ------------ | --------- | ----------- | ------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Release plan | Derived   | APS modules | Live   | 2026-08-04: **`v0.9.3-beta`** = honesty pass (Morgan CIB-220..227 / #3510) + Windows install/update path (Dave pack-01 CIB-228..243 / #3514; auth wall excluded) + pack-02 commissioning/TUI intake **CIB-251..267** (RETRACT-1 + trust candidates). Prior cut `v0.9.2-beta` MCP reconnect is published. |

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
  Deus findings from 0.9.1) **and** Windows install/update path (Dave 0.9.1
  feedback; still live on 0.9.2). Not a new product theme; not Graph Trust
  Surfaces / dashboard.

---

## Active window — `v0.9.3-beta` (honesty + Windows path)

**Theme:** Daily-path honesty (what anvil says matches what it does) **and**
restore the Windows install / self-update path for cargo-dist users.

**Tracking:**

- Morgan honesty: [#3510](https://github.com/eddacraft/anvil-001/issues/3510) ·
  APS **CIB-220..227**
- Dave field report: [#3514](https://github.com/eddacraft/anvil-001/issues/3514)
  · APS **CIB-228..243** (auth wall excluded by operator)
- Dave pack-02 commissioning + TUI (2026-08-04): APS **CIB-251..267** ·
  preserves pack-01 disposition; **RETRACT-1** conditions hook-block anchors

### Primary claim (must ship)

| ID          | Item                                                                                                                                   | Pri |
| ----------- | -------------------------------------------------------------------------------------------------------------------------------------- | --- |
| **CIB-228** | PowerShell dual-install guard inject no longer makes `irm \| iex` a silent no-op on clean Windows machines                             | P0  |
| **CIB-229** | cargo-dist receipt layout: `update --check` + install-method (not cargo install) + correct receipt path/name                           | P0  |
| **CIB-220** | Interactive `anvil start --mcp-scope project` installs / offers MCP; never claims "MCP installation disabled" solely for project scope | P0  |
| **CIB-221** | No false `anvil auth login` nag for already-authenticated / pro users                                                                  | P1  |
| **CIB-222** | Start value receipt discloses machine-wide vs repo-scoped evidence                                                                     | P1  |

### Secondary claim (should ship in the same cut if cheap)

| ID          | Item                                                           | Pri |
| ----------- | -------------------------------------------------------------- | --- |
| **CIB-230** | No internal `GH #NNNN` in public installers / ship artefacts   | P2  |
| **CIB-224** | `--no-mcp` + `--mcp-client` / `--all-mcp-clients` fails loudly | P2  |
| **CIB-227** | User-facing copy not exclusive to Claude Code + Cursor         | P2  |
| **CIB-244** | Verdict Install reflects multi-client this-run choice          | P1  |
| **CIB-245** | Grouped multi-step consent + what-is-this on project bits      | P2  |
| **CIB-223** | Non-git init vs no-worktree messaging coherent                 | P2  |

### Nice-to-have in this cut (else next)

| ID          | Item                                        | Pri |
| ----------- | ------------------------------------------- | --- |
| **CIB-225** | `--format` warns when config already exists | P3  |
| **CIB-226** | Public CLI docs: flags + auth exit code 3   | P3  |

### Dave field follow-ups (filed; not primary 0.9.3 claim unless pulled in)

Full report: `Projects/tmp/20260804-anvil-beta-0.9.2-test-{josh,agent}.md`.
**AUTH day-zero wall intentionally not tracked.**

| ID          | Item                                                          | Pri | Dave ID   | Notes                                |
| ----------- | ------------------------------------------------------------- | --- | --------- | ------------------------------------ |
| **CIB-229** | (primary) receipt + classify + update --check                 | P0  | UPD-1+3   | Absorbs CIB-231                      |
| **CIB-231** | ~~cargo-dist as cargo install~~                               | —   | UPD-3     | **Done** — superseded by 229         |
| **CIB-232** | Disclose open admission (do not flip default)                 | P3  | CONF-1    | Intentional open                     |
| **CIB-233** | audit-chain coverage summary (keep field semantics)           | P2  | TRUST-1   | Presentation                         |
| **CIB-234** | audit domain disclosure (not count parity)                    | P2  | TRUST-2   | Presentation                         |
| **CIB-235** | status Protection:warming next step                           | P2  | TRUST-3   | Keep                                 |
| **CIB-236** | insights zeros disclose domain                                | P3  | TRUST-4   | Keep                                 |
| **CIB-237** | path/line rendering consistency                               | P3  | UX-1      | Polish                               |
| **CIB-238** | "Blocking warnings" vocabulary                                | P3  | UX-2      | Polish — not severity bug            |
| **CIB-239** | Label pre-existing tree debt (keep full-tree)                 | P2  | UX-3      | Label only                           |
| **CIB-240** | tutorial non-tty exit non-zero + accurate message             | P3  | UX-4      | Keep                                 |
| **CIB-241** | antipattern-scan naming (docs)                                | P3  | UX-5      | By design                            |
| **CIB-242** | status skew hint after upgrade                                | P3  | stack     | Enhancement, not defect              |
| **CIB-243** | skill install docs multi-client + move-outside                | P3  | stack     | Docs; require --client is correct    |
| **CIB-244** | Verdict Install reflects this-run multi-client choice         | P1  | start TUI | Dual-era Install vs registry consent |
| **CIB-245** | Grouped multi-step consent; blurbs on project/hooks/workflows | P2  | start TUI | MCP secondary; multi-screen OK       |

### Dave pack-02 intake (commissioning + TUI; 2026-08-04)

Sources: operator pack `/tmp/anvil-dave-pack02/` (Windows-only scope binding).
**Pack-01 disposition preserved** (do not re-file UPD/TRUST/UX/CONF/AUTH).
**RETRACT-1** (binding) conditions pack-01 hook-block regression anchors;
absorbed into **CIB-251** (Anchor A) + **CIB-255** (Anchor B). Pack-02 IDs are
**CIB-251..267**. Free **CIB-250** was claimed by **pack-03** tutorial safety
chain (not RETRACT-1).

| ID          | Item                                 | Pri      | Dave ID                 | Tonight?          | Notes                                                         |
| ----------- | ------------------------------------ | -------- | ----------------------- | ----------------- | ------------------------------------------------------------- |
| **CIB-251** | Config-mode hooks honesty (opt-in)   | P3       | HOOK-1                  | No                | Opt-in only; **out of normal-path cut**; default file-mode OK |
| **CIB-252** | Workspace register false success     | P0       | WS-1                    | **Yes**           | Coords CIB-160                                                |
| **CIB-253** | status vs intercept daemon agreement | P1       | STATUS-1                | Yes (if capacity) | Low-risk                                                      |
| **CIB-254** | Daemon save-time silent miss         | P0       | WATCH-1                 | **Yes**           | ND caveat                                                     |
| **CIB-255** | gate / check --all domain disclosure | P1       | GATE-1, CHECK-1, GATE-2 | Yes (if capacity) | Same stance as CIB-234                                        |
| **CIB-256** | start --verify meaning honesty       | P2       | START-1                 | If green          |                                                               |
| **CIB-257** | Init sample + language honesty       | P2       | INIT-2, INIT-3          | If green          |                                                               |
| **CIB-258** | ~~Tutorial progress repo scoping~~   | —        | TUI-2                   | —                 | **Done** — superseded by 250                                  |
| **CIB-259** | Learning-path overclaim copy         | P2       | TUI-8                   | If green          |                                                               |
| **CIB-260** | Welcome save-time promise            | P3       | WELCOME-1               | No                |                                                               |
| **CIB-261** | Windows policy-path idempotent       | P2       | TUI-4                   | If green          | Windows                                                       |
| **CIB-262** | --json for workspace list / tutorial | P3       | JSON-1                  | No                |                                                               |
| **CIB-263** | Init lists .gitignore                | P3       | INIT-1                  | No                |                                                               |
| **CIB-264** | status no project cache side effect  | P3       | STATUS-3                | No                |                                                               |
| **CIB-265** | ~~esc back vs exit~~                 | —        | TUI-1                   | —                 | **Done** — superseded by 250                                  |
| **CIB-266** | Watch dashboard local/relative time  | P3       | TUI-7                   | No                |                                                               |
| **CIB-267** | Pre-push silent pass                 | Proposed | PUSH-1                  | No                | Needs repro after 252                                         |

**Welcome follow-ups (separate):** CIB-268..274 (#3536; not pack-02).
**CIB-250:** claimed by pack-03 tutorial safety chain (2026-08-05); not
RETRACT-1.

**Absorbed / non-scope:** STATUS-2→CIB-235; PATH-1→CIB-237; TUI-9→CIB-248;

### Dave pack-03 intake (start + walkthrough first-timer; 2026-08-05)

Source: `/tmp/dave-beta-report-3.md`. **Normal-path cutline only.**

| ID          | Item                                                       | Pri | Tonight?          | Notes                  |
| ----------- | ---------------------------------------------------------- | --- | ----------------- | ---------------------- |
| **CIB-250** | Tutorial safety chain (esc → resume → wrong-repo activate) | P0  | **Yes**           | Supersedes 258, 265    |
| **CIB-276** | Prove fixture wording not "this repo"                      | P1  | Yes (if capacity) | Honesty                |
| **CIB-275** | Start result one help bar + full next:                     | P2  | If green          | Dual bars + truncation |
| CIB-261     | Windows policy mkdir re-run                                | P2  | If green          | Reconfirmed §5         |
| §4 teaching | curriculum editorial                                       | —   | No                | Deliberate non-scope   |

TUI-10, TUI-N1/N2, R1..R8 not automatic release work. (TUI-3 absorbed into
CIB-250; TUI-5/TUI-6 elevated to CIB-275.)

### Not a claim of this release

- Browser dashboard default-on
- Graph Trust Surfaces / CGBDG programme
- Full `rmcp` adoption (MCP26-012)
- Restoring cargo-dist `install-updater` sidecar (blocked on aarch64 Windows
  axoupdater)
- New feature narrative (that remains a later minor / `v0.10.0-beta` when
  scoped)

### Phase plan

| Phase                   | Scope                                                     | State                       |
| ----------------------- | --------------------------------------------------------- | --------------------------- |
| **0.9.2 publish**       | MCP reconnect + openapi retag                             | Done 2026-08-03             |
| **Intake honesty**      | File CIB-220..227 + #3510; scope this window              | Done 2026-08-04             |
| **Intake Windows path** | File CIB-228..230 + #3514 (Dave; still live on 0.9.2)     | Done 2026-08-04             |
| **Intake Dave field**   | File CIB-231..243; **exclude auth wall**                  | Done 2026-08-04             |
| **Re-triage Dave**      | Merge UPD-3→229; demote CONF/trust/UX per operator review | Done 2026-08-04             |
| **Intake Dave pack-02** | File CIB-251..267; RETRACT-1; preserve pack-01 map        | Done 2026-08-04             |
| **P0 implement**        | CIB-228, 229; then CIB-252, 254 (pack-02 trust); CIB-220  | Next                        |
| **P1 implement**        | CIB-221, 222                                              | Same cut                    |
| **P2 implement**        | CIB-230 (with 228), 223, 224, 227                         | Same cut if unblocked       |
| **P3**                  | CIB-225, 226                                              | Same cut or follow-up patch |
| **Cut**                 | Preflight → prepare → readiness → tag                     | After claim green           |

### Cut criteria

- Standing base bar: full Cross matrix, release-readiness on source SHA,
  ACKNOWLEDGEMENTS fresh, dashboard openapi `check:api` green (0.9.2 lesson).
- **CIB-228** validated (clean-PATH install body runs; dual-install still
  refuses; published pre-release asset checked when available).
- **CIB-229** validated (`update --check` + `version` install-method on
  cargo-dist receipt layout).
- **CIB-220** validated (TUI project-scope MCP path).
- **CIB-221** and **CIB-222** validated (or explicit waive with issue).
- **Pack-02 P0 if green tonight:** CIB-252 (register honesty), CIB-254
  (save-time daemon path) validated or explicitly waived with reason.
- **Pack-03 P0 if green tonight:** CIB-250 tutorial safety chain (wrong-repo
  activate) validated or explicitly waived with reason.
- Changelog leads with install/update + honesty fixes, not new features.
- Strategy: **direct** unless readiness forces stabilisation.
- Prepare regenerates dashboard openapi when version bumps (avoid 0.9.2 retag
  class).

### Risks

| Risk                                             | Mitigation                                                                                   |
| ------------------------------------------------ | -------------------------------------------------------------------------------------------- |
| Guard inject still breaks cargo-dist `param`     | Insert after cargo-dist `param` as fall-through; assembly fixture in CI                      |
| Receipt rename misses legacy names               | Try `eddacraft-anvil` then `anvil`; actionable error if neither configures                   |
| Project-scope MCP reopens installer edge cases   | Prefer thin path: stop `Skip` for project; reuse scope-aware installer already used headless |
| Auth false-negative hides real login need        | Fixture matrix: no token / expired / valid / pro                                             |
| Scope copy on value line confuses                | One short parenthetical; match insights scorecard language                                   |
| Pack-02 WS-1 register false success (Windows)    | Ship honesty first (CIB-252); durable path coords CIB-160 — never claim Registered if empty  |
| Pack-02 WATCH-1 daemon save-time non-determinism | Do not claim fixed on one happy path; repeat controlled write-while-watch (CIB-254)          |

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
