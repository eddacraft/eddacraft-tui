# Activation TUI — `anvil start` as the flagship eddacraft-tui surface

| ID     | Owner | Status | Progress |
| ------ | ----- | ------ | -------- |
| ACTTUI | Josh  | Done | 22/22 |

**Last reviewed:** 2026-08-03 — items **000–017 Merged via PR #3478/#3488**
([spec](../specs/2026-08-03-activation-tui-completion.md)); completion programme
015–017 in #3488. **Usability follow-ups ACTTUI-018..021 Merged via PR #3499**
(quiet re-run consent, shared posture model, settled Install/Languages honesty,
MCP pre-write prove honesty). Escape hatches: `--no-tui` / `ANVIL_NO_TUI=1`.
Module **Done** (all 22 items Merged); release evidence still owed for
Released/Shipped.

The 2026-07-25 ADR-103 acceptance ([ADR-103](../decisions/103-tty-default-activation-tui.md))
remains the governing rollout ladder. Post-flip continuous surface (ACTTUI-014)
plus the 2026-08-03 completion programme (015–017) close the deferred thin-v1
smoke gap and daily re-run friction without reopening machine contracts.

Earlier review (2026-07-11) — the operator-approved
[`JOURNEY` conductor](./release-user-journeys.aps.md) makes ACTTUI-009/-010/-012
and WOW-005 release-cut gates, while retaining celebration and richer diagnostics
as coordinated enhancements. The 2026-07-10 ACTTUI-009..011 implementation
milestone Merged via PR #3263. Consent now binds the
exact selected targets and applies only after explicit submission; the contract matrix includes
real PTY restoration and all-phase snapshots; verdict/evidence models are built
from typed activation data. Targeted Rust, Clippy, PTY, snapshot, and activation
e2e checks pass locally. ACTTUI-012 has since Merged via PR #3284. The earlier
post-ACTTUI first-run council review
([`2026-07-09-acttui-first-run-journeys.md`](../reviews/2026-07-09-acttui-first-run-journeys.md))
blocked the TTY-default flip because the opt-in `--tui` consent path was then a
dead end; ACTTUI-009..012 tracked that remediation wave and all Merged.
ACTTUI-000 planning gate merged (PR #3232); ACTTUI-001 activation-surface
scaffold (#3234), ACTTUI-002 progress events (#3236), ACTTUI-003 working
progress (#3237), ACTTUI-004 consent chrome (#3238), ACTTUI-005 verdict tree
(#3254), ACTTUI-006 tier-evidence LogPanel (#3256), and ACTTUI-007 fixture
hardening (#3257) all Merged across the stacked ACTTUI branches. ADR-103
Accepted 2026-07-25, fixture spec + fixture home created, public scripting contract
documented, and PR #3231 WOW-005/006 dependency acknowledged. Module originally
created via planning-workflow + planning-council direction-validate (four-lens
stress test). Replaces the
plain-text activation dossier and `demand` pickers with a single interactive
surface built on `eddacraft-tui` widgets. **Start first, welcome second** —
execution waves land activation before welcome convergence; the module plans the
whole first-run pair but does not block WOW tutorial work.

**Council decision:** `proceed` with `amend` — ACTTUI-001 may move **In Progress** after ACTTUI-000 lands because ADR-103 / DECISION-LOG entry and the pinned plain-output fixture spec now exist (see Planning Council record below).

## Purpose

Make `anvil start` the flagship TUI moment: progressive disclosure, honest
consent pickers, collapsible diagnostics, and a visible activation spine — using
`eddacraft-tui` widgets as intended rather than hand-rolled `Paragraph` menus and
`demand::MultiSelect` overlays.

Closes the re-run verbosity and picker-scaling gaps from the 2026-07-04
welcome/start user-journey audit
([`plans/audits/2026-07-04-anvil-start-welcome-user-journey.md`](../audits/2026-07-04-anvil-start-welcome-user-journey.md))
and realises the operator investment in `eddacraft-tui` for end-user wow.

## Strategic sequencing

**Release cohort (operator decision 2026-07-08):** `anvil start` and
`anvil welcome` both ship in the **same release** — no partial UX where
activation is polished and welcome is left hand-rolled. Waves below optimise
implementation order within that cohort, not across releases.

| Phase | Scope | Rationale |
| ----- | ----- | --------- |
| **A — Foundation** | ACTTUI-000, 001, 002 + shared widget extract | ADR, fixtures, orchestrator events, and shared `HelpBar`/`Select` chrome used by both surfaces |
| **B — Parallel surfaces** | ACTTUI-003..005 ∥ ACTTUI-008 ∥ WOW-001..005 | Start activation phases and welcome uplift on separate files; converge visually in the same train |
| **C — Ship gate** | ACTTUI-009 → ACTTUI-010 → ACTTUI-012 ∥ WOW-005 | Working consent, executable contract matrix, default-path polish, and repository-specific first win all pass |

Welcome tutorial narrative uplift stays in
[`first-run-wow.aps.md`](./first-run-wow.aps.md) (WOW-001..006). ACTTUI-008
covers chrome/widget adoption on the welcome hub and shared primitives — not
tutorial story changes (WOW owns narrative).

## In Scope

- TTY-default interactive activation surface for `anvil start`
- Collapsible diagnostic tree (`Tree`), tier badges (`StatusBadge`), multi-step
  progress (`ParallelProgress`), consent pickers (`Select`, `Confirm`,
  `OverlayStack`), ephemeral feedback (`Toast`), optional splash (`BigBanner`
  via `big-text` feature), contextual keys (`HelpBar`)
- Orchestrator presentation decoupling: logic stays; `demand` pickers and
  `render_human` wall removed from the interactive path
- Plain-text compact fallback for piped, CI, `--no-tui`, and scripted consumers
- Byte-stable `--verify` / `--json` surfaces (unchanged contracts)
- Auto-expand repair sections on non-`protecting` states; collapsed repeat-success
- Quiet re-run: skip Consent when no actionable offers remain (still unticked when
  offers exist — CIB-165)
- In-surface **Prove** (formerly thin-v1 smoke `t` key): real ADTRUST-006 recipe
  execution with honest finding display (ACTTUI-016); honesty hotfix first
  (ACTTUI-015) so a placeholder cannot over-claim
- Shared posture / next-step language with `anvil status` (ACTTUI-017) without
  replacing the status command
- `LogPanel` for tier-evidence / daemon skip detail (replaces JSON WARN leak on
  human path — CIB-162)
- Snapshot + e2e regression matrix for TTY vs plain vs verify
- **Usability follow-ups (post-#3488):** quiet re-run consent when installs are
  settled (ACTTUI-018); deeper start↔status shared posture model (ACTTUI-019);
  settled Install section + no fake language-row actions (ACTTUI-020); optional
  honest MCP pre-write prove path (ACTTUI-021)

## Out of Scope

- Replacing `anvil status` as a command (sharing posture fields is in scope via
  ACTTUI-017; a merged start/status product surface is not)
- Per-language Prove actions on every Languages tree row
- Full-repo `gate` / full scan from the activation surface
- Claiming MCP pre-write intercept from a CLI Prove result
- Web tutorial changes (WOW / UJ shipped scope)
- MCP protocol or daemon spine behaviour changes (ACTMO owns semantics; ACTTUI
  owns presentation)
- New protection claims or LAUNCH-014 honesty-pin relaxations
- `json_render` activation catalogue in wave 1 (ACTTUI-006 optional; default
  path is hand-wired widgets first for faster wow, catalogue second for layout
  agility)
- Renaming `anvil start` to `anvil connect` (wow-start brainstorm — separate
  product decision)

## Constraints

- **Honesty pins:** `ProtectionState` labels, tier vocabulary, and
  `protecting`/`watching`/`ready_restart_required` gates unchanged (LAUNCH-014,
  ADR-092). TUI must not list L0 as "active" when only wired (CIB-164).
- **Consent — repo-scoped writes:** Workflows, hooks, `.anvilrc`, baseline, and
  witness paths require explicit tick; default unticked (CIB-165). Plain Enter
  writes nothing.
- **Consent — MCP (interactive):** MCP `Select` defaults **unticked** for every
  offered client (CIB-165 parity with workflows — current `demand` pre-selects
  `NotPresent`/`SafeDrift` and must not carry into TUI). `UnsafeDrift` never
  appears in multi-select; `OverlayStack` + `Confirm` only.
- **Consent — non-interactive exception:** CI/piped/`--no-tui` may auto-install
  MCP per existing orchestrator policy; never auto-write workflows/hooks/project
  seeding under gated `ANVIL_HOME`.
- **`ANVIL_HOME` gated posture:** Persistent shell banner when
  `project_writes_gated`; wow affordances (`BigBanner`, in-surface Prove) are
  suppressed or gated when they would write the project; consent phase skips
  repo-scoped offers with visible reason.
- **Deterministic degradation:** Non-TTY path prints a fixed compact summary;
  no keypress hang; exit codes unchanged.
- **Byte stability:** `anvil start --verify` and `--json` stdout contracts
  preserved; TUI is additive on the default TTY path only.
- **Terminal safety:** `TerminalGuard` / panic restore via `eddacraft-tui`
  `lifecycle` or existing `anvil-cli` guard — no raw-mode leaks between phases.
- **Widget-first:** New UI in `anvil-tui` surfaces must use `eddacraft-tui`
  prelude widgets where one exists; hand-rolled spinners/menus/overlays require
  a non-scope justification in the work item.

## Coordination

| Module / audit | Relationship |
| -------------- | ------------ |
| ACTMO | Spine semantics frozen; ACTTUI consumes `ActivationDiagnostic` + `InstallReport` |
| WOW | Tutorial/welcome story; ACTTUI-008 lands after WOW-001..002 or in parallel on non-overlapping files. PR #3231 records new downstream gates: WOW-005 depends on ACTTUI-000 + ACTTUI-004 consent posture; WOW-006 depends on ACTTUI foundation + shared-widget extract. |
| CIB-162..179 | Many repair items become easier or obsoleted by TUI path; do not close CIB items in ACTTUI PRs unless the fix is verified |
| ADTRUST-006 | Plain first-run recipe remains the source of Prove fixture copy; ACTTUI-016 executes it in-surface; CIB-183 re-runs omit the plain recipe by design |
| TUIN / eddacraft-tui | Enable `big-text` on `anvil-tui`; consider promoting `Tree`/`ParallelProgress` to `stable` if activation depends on them |
| DSV-046..051 | Daemon attestation copy in `LogPanel`; no change to attestation rules |
| JOURNEY | Release conductor; ACTTUI-009/-010/-012 were cut gates; completion programme (014–017) is post-cut polish for daily confidence |
| Completion spec | [`2026-08-03-activation-tui-completion.md`](../specs/2026-08-03-activation-tui-completion.md) — WP0–WP3 product contract |

## Work Items

### ACTTUI-000: UX contract + plain-output fixtures (planning gate)

- **Status:** Done 2026-07-08 (ADR-103 proposed; DECISION-LOG row added; fixture spec + fixture home created; public scripting contract documented)
- **Intent:** Load-bearing public UX change and scripting contracts are documented
  before code lands.
- **Expected Outcome:** ADR or DECISION-LOG entry for TTY-default activation
  TUI (with trust-boundary table: interactive vs `--no-tui`/piped); pinned
  fixture spec for `anvil start --verify`, `--json`, and compact plain
  (`tests/fixtures/start-verify/` or equivalent); release-note template
  ("scripting: `--no-tui`; probing: `--verify`/`--json`"); rollout ladder:
  opt-in (`--tui` or `ANVIL_ACTIVATION_TUI=1`) for first release, flip
  TTY-default only after contract matrix green. **Closeout:** met by
  [`ADR-103`](../decisions/103-tty-default-activation-tui.md),
  [`2026-07-08-activation-tui-contract-fixtures.md`](../specs/2026-07-08-activation-tui-contract-fixtures.md),
  `crates/anvil-cli/tests/fixtures/start-activation/README.md`, and
  [`start-output-contracts.md`](../../docs/public/anvil/guides/start-output-contracts.md). PR #3231 WOW dependency noted as downstream coordination, without editing the WOW file on this branch.
- **Files:** `plans/decisions/`, `docs/public/anvil/guides/`, fixture directory
- **Validation:** `pnpm docs:check`; `pnpm adr:check`; `pnpm format:check`; council checklist ticked
- **Confidence:** high

### ACTTUI-001: Activation surface scaffold + plain fallback contract

- **Status:** Merged 2026-07-08 via PR #3234 — surface scaffold + opt-in
  dispatch. New
  `crates/anvil-tui/src/surfaces/activation/` (`ActivationPhase` enum +
  `ActivationSurface` + renderer with gated-`ANVIL_HOME` banner); `anvil start`
  gains `--tui` and honours `ANVIL_ACTIVATION_TUI`, gated behind genuine
  interactivity (stdin+stdout+stderr TTY, not CI/`ANVIL_NO_PROMPT`) and
  suppressed by `--verify`/`--json`/`--watch`/`--no-tui`/`ANVIL_NO_TUI`;
  `anvil-tui` enables the `eddacraft-tui` `big-text` feature. `render_compact`
  path unchanged (verdict text composed identically for both surfaces).
- **Intent:** `anvil start` on TTY enters an `anvil-tui` activation surface;
  all other contexts get a compact plain summary without hanging.
- **Expected Outcome:** New `crates/anvil-tui/src/surfaces/activation/` with
  `Surface` impl, shell chrome, and phased state enum (`Preflight`, `Working`,
  `Consent`, `Verdict`, `Done`); `start.rs` dispatches TTY → surface only when
  `orchestrator::is_interactive` (stdin **and** stderr TTY, not `CI`, not
  `ANVIL_NO_PROMPT`) **and** opt-in flag set per ACTTUI-000 rollout ladder;
  else `render_compact()` unchanged until ACTTUI-007 fixtures land;
  `ANVIL_NO_TUI=1` mirrors global `--no-tui`; `anvil-tui` enables
  `eddacraft-tui` `big-text` feature; gated `ANVIL_HOME` banner in shell chrome.
- **Non-scope:** Changing `render_compact()` output before ACTTUI-007 fixtures
  exist.
- **Files:** `crates/anvil-tui/src/surfaces/activation/`,
  `crates/anvil-tui/Cargo.toml`, `crates/anvil-cli/src/commands/start.rs`,
  `apps/e2e/src/cli/activation.e2e.test.ts` (compact path)
- **Validation:** `cargo test -p anvil-tui -p anvil-cli`; `pnpm e2e -- --testPathPattern activation`
- **Confidence:** high

### ACTTUI-002: Orchestrator progress events (decouple presentation)

- **Status:** Merged 2026-07-08 via PR #3236 — activation progress event
  contract implemented (stacked on ACTTUI-001). Added typed
  `ActivationStep`/`ActivationStepEvent`/`ActivationRun` primitives,
  `StartRenderMode`, TUI-mode lifecycle/log buffering, and a consent deferral
  seam so `demand` pickers are not invoked on the TUI path. Full in-surface
  consent widgets remain ACTTUI-004 scope.
- **Dependencies:** ACTTUI-001
- **Intent:** The orchestrator reports step lifecycle to the surface without
  printing pickers or human diagnostic strings on the TUI path.
- **Expected Outcome:** Typed `ActivationStep` enum + `ActivationRun`
  accumulator with golden test fixture for happy-path ordering; orchestrator
  exposes step lifecycle to the surface (callback/channel or polled struct);
  `demand` pickers not invoked when `StartRenderMode::Tui`; daemon attestation
  skip detail routed to surface log buffer (CIB-162 TTY coverage); workflow picker
  reordered or modelled as explicit sub-steps so `Working → Consent → Verdict`
  matches real spine ordering (workflow consent may precede or consolidate with
  MCP consent — documented in work-item closeout).
- **Files:** `crates/anvil-cli/src/activation/orchestrator/`,
  `crates/anvil-cli/src/commands/start.rs`
- **Validation:** `cargo test -p anvil-cli -- start`; unit tests for
  `StartRenderMode` branch parity
- **Confidence:** high

### ACTTUI-003: Working phase — `ParallelProgress` + `Spinner`

- **Status:** Merged 2026-07-08 via PR #3237 — activation surface renders
  orchestrator progress rows via shared `eddacraft-tui` `ParallelProgress` and
  shows an Anvil spinner for daemon ensure (stacked on
  ACTTUI-002). Progress is fed from `ActivationRun` events; deferred TUI consent
  transitions the surface to `Consent`, otherwise it lands on `Verdict`.
- **Dependencies:** ACTTUI-002
- **Intent:** Users see activation work happening (daemon ensure, init, baseline
  sample) instead of a silent hang or stderr preamble.
- **Expected Outcome:** Working phase renders `ParallelProgress` fed by orchestrator
  step events; daemon ensure shows branded `Spinner`; overall progress animates
  via `animate_tick`; completion transitions to Consent or Verdict based on
  pending pickers.
- **Files:** `crates/anvil-tui/src/surfaces/activation/render.rs`,
  `crates/anvil-cli/src/tui.rs` (tick wiring if needed)
- **Validation:** `cargo test -p anvil-tui` snapshot: working phase mid-run;
  `insta` redaction for durations
- **Confidence:** high

### ACTTUI-004: Consent phase — `Select`, `Confirm`, `OverlayStack`

- **Status:** Merged 2026-07-09 via PR #3238 — consent model and renderer in
  `crates/anvil-tui/src/surfaces/activation/consent.rs` use `Select`,
  `Confirm`, and `OverlayStack`; rows default unticked, gated `ANVIL_HOME`
  disables repo-scoped writes, and unsafe drift opens an acknowledgement overlay
  without implicit selection. Orchestrator handoff remains through the
  ACTTUI-002 deferral seam. The end-to-end wiring gap flagged by Copilot review
  on #3238 — `commands/start.rs` never constructing a `ConsentState`, so the
  picker was unreachable and had no apply-selection path — was reparented to
  **ACTTUI-009**, Merged 2026-07-10 via PR #3263.
- **Dependencies:** ACTTUI-003
- **Intent:** MCP, workflow, and hook consent use the shared picker chrome with
  descriptions, drift badges, and explicit tick-to-install — replacing `demand`.
- **Expected Outcome:** MCP multi-select via `Select` with **all items
  `selected = false` by default** (CIB-165 parity — tests mirror workflow
  unticked contract); drift state in description only, not as implicit consent;
  workflows default unticked (CIB-165); hook install gets explicit consent step
  or documented APS exemption with constraint narrowed; unsafe MCP drift via
  `OverlayStack` + `Confirm`; repo-scoped pickers disabled when
  `project_writes_gated`; no `RawModeGuard` on TUI path.
- **Files:** `crates/anvil-tui/src/surfaces/activation/consent.rs`,
  `crates/anvil-cli/src/activation/orchestrator/install.rs`,
  `crates/anvil-cli/src/activation/orchestrator/mod.rs` (workflow picker),
  `crates/anvil-cli/src/commands/start.rs` (remaining: build `ConsentState`
  from `InstallReport` + `pending_workflows`, call `with_consent`, and apply
  the selection after the surface exits)
- **Validation:** `cargo test -p anvil-tui -p anvil-cli`; e2e: workflow Enter-without-tick writes nothing
- **Confidence:** high

### ACTTUI-005: Verdict phase — `Tree`, `StatusBadge`, `BigBanner`, `Toast`, smoke test

- **Status:** Merged 2026-07-09 via PR #3254 — structured verdict model/view in
  `crates/anvil-tui/src/surfaces/activation/verdict.rs` renders state labels
  through `StatusBadge`, collapsible evidence through `Tree`, contextual keys
  through `HelpBar`, optional `BigBanner`, and a thin-v1 honest `Toast` for the
  `t` smoke key. The view currently derives its sections from the composed
  verdict string so the TUI cannot drift from the plain contract; richer
  diagnostic-fed evidence remains ACTTUI-006/007 scope.
- **Dependencies:** ACTTUI-004
- **Intent:** The protection verdict is one glance on repeat runs and expandable
  on demand; first protecting run delivers the wow beat.
- **Expected Outcome:** Verdict renders `Tree` sections (activation, layers, install,
  languages, config) collapsed by default on `protecting` re-run, auto-expanded on
  repair states; headline uses `StatusBadge` chips; first `protecting` shows
  `BigBanner` once per project (marker or `baseline: absent` heuristic);
  `t` runs in-surface smoke test with `Toast` result; single `HelpBar` next step
  (CIB-166 arbiter); duplicate stdout `verify:` block removed on TUI path.
- **Files:** `crates/anvil-tui/src/surfaces/activation/verdict.rs`,
  `crates/anvil-cli/src/commands/start.rs` (remove `render_first_run_recipe` on TUI path)
- **Validation:** `cargo test -p anvil-tui` snapshots: collapsed protecting,
  expanded `ready_restart_required`, first-run banner; e2e smoke key optional;
  LAUNCH-014 honesty-pin tests extended to TUI render path
- **Non-scope (thin v1):** `BigBanner`, in-surface smoke (`t` key) — defer if
  schedule bites; retain compact plain recipe pointer
- **Confidence:** medium

### ACTTUI-006: Tier evidence — `LogPanel` + optional `json_render` catalogue

- **Status:** Merged 2026-07-09 via PR #3256 — ACTTUI evidence rows now feed an
  in-surface `LogPanel`: compact verdict rows, orchestrator lifecycle lines, and
  `render_human_verbose` (`--why`) text are parsed into `LogEntry` rows. The
  `l` key toggles tier evidence in the activation surface, and TUI `--why`
  attaches the same verbose evidence in-surface instead of printing a post-exit
  stderr block. No `json_render` catalogue lands in this slice.
- **Dependencies:** ACTTUI-005
- **Intent:** Operators who need `--why` depth get it in-surface without stderr
  JSON noise; declarative layout available for future activation panels.
- **Expected Outcome:** `l` toggles `LogPanel` with daemon attestation skips,
  install detail, and tier evidence; supersedes `anvil start --why` stderr block
  on TUI path (stdout/`--verify` `--why` unchanged); optional thin
  `activation_catalog` registering `Alert`/`MetricCard` rows bound to
  `ActivationDiagnostic` — behind feature flag or compile-time module if scope
  threatens wave 1.
- **Files:** `crates/anvil-tui/src/surfaces/activation/`,
  `crates/anvil-tui/src/dashboard_catalog/` (if catalogue lands),
  `crates/anvil-cli/src/activation/render.rs` (`render_human_verbose` reuse)
- **Validation:** `cargo test -p anvil-tui`; manual: no JSON WARN on TTY start
- **Confidence:** medium

### ACTTUI-007: Contract hardening — verify/json/CI matrix

- **Status:** Merged 2026-07-09 via PR #3257 — fixture-backed read-only/JSON/no-TUI
  contract tests now pin the reachable `ready_restart_required` path in
  `crates/anvil-cli/tests/fixtures/start-activation/`, with e2e parity coverage
  for `--no-tui` and `ANVIL_NO_TUI=1`. The `protecting` and PTY transcript
  matrix was reparented to **ACTTUI-010**, Merged 2026-07-10 via PR #3263.
- **Dependencies:** ACTTUI-005
- **Intent:** Scripted and CI consumers never regress when TUI becomes default.
- **Expected Outcome:** `anvil start --verify` and `--json` byte-stable against
  ACTTUI-000 fixtures; piped stdout compact ≤10 lines on `protecting` re-run;
  `ANVIL_NO_TUI=1` / `--no-tui` forces plain; `read_only` code path isolated
  (byte-diff test vs pre-ACTTUI fixtures); PTY integration test: TTY opt-in
  enters surface, `q` exits without orphan raw mode; `--watch` tears down
  alternate screen before `watch_cmd::run` (bounded test); air-gapped closed-set
  passes; MCP install failure non-zero preserved; plain `--no-tui` path satisfies
  CIB-162/165/169 independently of TUI.
- **Files:** `crates/anvil-cli/tests/start.rs`,
  `crates/anvil-cli/tests/air_gapped.rs`, `apps/e2e/src/cli/activation.e2e.test.ts`
- **Validation:** `cargo test -p anvil-cli -- start air_gapped`; `pnpm e2e -- --testPathPattern activation`
- **Confidence:** high

### ACTTUI-008: Welcome widget convergence (release cohort — phase B)

- **Status:** Merged 2026-07-08 via PR #3235 — welcome widget convergence implemented on
  `feat/acttui-008-welcome-widgets` (stacked on ACTTUI-001). Welcome menu now
  renders via shared `eddacraft-tui` `Select`; discovery scanning uses shared
  `Spinner` + `ParallelProgress`; shell/footer help remains the shared chrome;
  tutorial narrative unchanged (WOW-owned). Snapshot drift reviewed and
  accepted for the Select-based menu.
- **Dependencies:** ACTTUI-001 (shared chrome); coordinates with WOW-001..004;
  may start once 001 lands — does not wait for 005
- **Intent:** Welcome hub and activation share the same eddacraft-tui vocabulary
  so the first-run pair feels like one product in the same release.
- **Expected Outcome:** Welcome menu uses `Select` instead of hand-rolled
  paragraphs; discovery scan uses `Spinner` + `ParallelProgress` instead of DIY
  frames; shared `HelpBar` component extracted to `anvil-tui/src/widgets/` (or
  `eddacraft-tui` if generally useful); visual continuity with activation shell
  (same header/footer rules). Tutorial path unchanged (WOW owns narrative).
- **Files:** `crates/anvil-tui/src/surfaces/welcome/`,
  `crates/anvil-tui/src/surfaces/tutorial/discovery_render.rs`,
  `crates/anvil-cli/src/commands/welcome.rs`
- **Validation:** `cargo test -p anvil-tui`; welcome snapshot drift review
- **Confidence:** medium

### ACTTUI-009: Wire activation TUI consent end to end

- **Status:** Merged 2026-07-10 via PR #3263
- **Source:** First-run council review C-001, C-002, C-003, C-005
  ([`2026-07-09-acttui-first-run-journeys.md`](../reviews/2026-07-09-acttui-first-run-journeys.md))
- **Dependencies:** ACTTUI-004
- **Intent:** The opt-in `anvil start --tui` path can actually collect consent
  and perform selected MCP/workflow writes, without silently skipping install or
  bypassing consent.
- **Expected Outcome:** `commands/start.rs` builds a `ConsentState` from
  pending GitHub Actions workflow offers and MCP install candidates before
  opening the surface; `ActivationSurface::with_consent(...)` is used on the
  production TUI path; the returned surface state is captured; ticked selections
  drive the same workflow/MCP install primitives as the plain path; unticked
  selections write nothing; gated `ANVIL_HOME` keeps repo-scoped offers disabled
  with an explicit reason. The renderer rejects or visibly guards
  `phase == Consent` with no consent model so the phase strip cannot show
  Consent over a verdict body. Add a positive integration test proving
  "tick ⇒ file write" and a negative test proving "Enter/no tick ⇒ no write".
- **Files:** `crates/anvil-cli/src/commands/start.rs`,
  `crates/anvil-cli/src/activation/orchestrator/mod.rs`,
  `crates/anvil-cli/src/activation/orchestrator/install.rs`,
  `crates/anvil-tui/src/surfaces/activation/{mod,consent,render}.rs`,
  `crates/anvil-cli/tests/start.rs`,
  `apps/e2e/src/cli/activation.e2e.test.ts`
- **Validation:** `cargo test -p eddacraft-anvil start`; `cargo test -p
  eddacraft-anvil-tui activation`; `pnpm --filter @eddacraft/anvil-e2e exec
  vitest run src/cli/activation.e2e.test.ts`
- **Confidence:** high

### ACTTUI-010: Complete activation TUI contract matrix

- **Status:** Merged 2026-07-10 via PR #3263 — CIB-182's existing repair-hint
  fixture change is accepted as the sanctioned contract.
- **Source:** First-run council review C-006, C-007, C-011, C-012
  ([`2026-07-09-acttui-first-run-journeys.md`](../reviews/2026-07-09-acttui-first-run-journeys.md))
- **Dependencies:** ACTTUI-009
- **Intent:** The rollout/default-flip contract is backed by executable fixtures
  and PTY coverage rather than README-only fixture intent.
- **Expected Outcome:** `anvil start --verify`, `--json`, compact plain, and
  `--no-tui` / `ANVIL_NO_TUI=1` outputs are pinned by real fixtures, including
  the opt-in flag on non-TTY stdio. A PTY test proves `--tui` enters the surface,
  `q` exits cleanly, and raw mode is restored; snapshot tests cover Preflight,
  Working, Consent, Verdict, and Done render states; the CIB-182 repair-hint copy
  change is either explicitly accepted as a sanctioned fixture update or rolled
  back to the byte-stable contract. This item is the hard gate before any
  TTY-default flip.
- **Files:** `crates/anvil-cli/tests/start.rs`,
  `crates/anvil-cli/tests/fixtures/start-activation/`,
  `apps/e2e/src/cli/activation.e2e.test.ts`,
  `crates/anvil-tui/src/surfaces/activation/{mod,render}.rs`
- **Validation:** `cargo test -p eddacraft-anvil start`; `cargo test -p
  eddacraft-anvil-tui activation`; `pnpm --filter @eddacraft/anvil-e2e exec
  vitest run src/cli/activation.e2e.test.ts`
- **Confidence:** high

### ACTTUI-011: Drive verdict and evidence panes from typed activation data

- **Status:** Merged 2026-07-10 via PR #3263 — the production path builds the
  verdict/evidence panes with `from_typed_with_progress` from a typed
  `VerdictModel` and typed `LogEntry` rows.
- **Source:** First-run council review C-004, C-010
  ([`2026-07-09-acttui-first-run-journeys.md`](../reviews/2026-07-09-acttui-first-run-journeys.md))
- **Dependencies:** ACTTUI-009
- **Intent:** Activation TUI sections and tier evidence do not depend on
  substring-parsing the human plain-output copy.
- **Expected Outcome:** The TUI verdict tree is built from typed
  `ActivationDiagnostic`, `InstallReport`, and `ActivationRun` data, using
  `with_verdict_model(...)` or an equivalent typed constructor on the production
  path. `LogPanel` rows are fed by typed lifecycle/evidence records instead of
  indentation heuristics over `render_human_verbose`. Plain output remains the
  authority for non-TUI users, but copy-only edits cannot silently misfile TUI
  tree sections or log severity.
- **Files:** `crates/anvil-cli/src/commands/start.rs`,
  `crates/anvil-cli/src/activation/render.rs`,
  `crates/anvil-tui/src/surfaces/activation/{mod,verdict,log_panel}.rs`
- **Validation:** `cargo test -p eddacraft-anvil-tui activation`; `cargo test -p
  eddacraft-anvil start`
- **Confidence:** medium

### ACTTUI-012: Activation TUI polish before default flip

- **Status:** Merged 2026-07-11 via PR #3284
- **Shipped:** Promoted into the release cut by the operator-accepted JOURNEY
  conductor ([`release-user-journeys.aps.md`](./release-user-journeys.aps.md));
  tier-evidence pane rebound to `e` and pinned flag-free, one `esc/q` exit-key
  story, dead consent helpers removed, `ANVIL_NO_TUI=` semantics documented as
  aligned, `celebrate()`/`big-text` deferred with the unused dependency dropped.
- **Source:** First-run council review C-013, C-014, C-015, C-017, C-018
  ([`2026-07-09-acttui-first-run-journeys.md`](../reviews/2026-07-09-acttui-first-run-journeys.md))
- **Dependencies:** ACTTUI-010
- **Intent:** Clear the remaining low-risk inconsistencies before making the
  activation TUI the default terminal path.
- **Expected Outcome:** Tier evidence can be opened in-surface without requiring
  `--why` at process start; activation and welcome use one exit-key story;
  dead consent helpers are either used by ACTTUI-009 or removed; `ANVIL_NO_TUI=`
  empty-value semantics are either aligned with sibling env hatches or explicitly
  documented as the exception; the `big-text`/`celebrate()` path is either wired
  to a real first-run state or deferred with no unused default dependency. This
  item does not block ACTTUI-009, but it blocks the TTY-default flip.
- **Files:** `crates/anvil-cli/src/commands/start.rs`,
  `crates/anvil-tui/src/surfaces/activation/`,
  `crates/anvil-tui/Cargo.toml`,
  `docs/public/anvil/guides/start-output-contracts.md` (only if env semantics
  change)
- **Validation:** `cargo test -p eddacraft-anvil-tui activation`; `cargo test -p
  eddacraft-anvil start`; `pnpm docs:check` if public env semantics change
- **Confidence:** medium

### ACTTUI-013: TTY-default flip — activation TUI on the default interactive path

- **Status:** Merged 2026-07-26 via PR #3411
- **Progress:** `start_render_mode` derives
  from `activation_tui_allowed` (pure argument/environment policy, unit-testable
  without a PTY) plus the stdio terminal probe, with the opt-in gate removed.
  `--tui` is retained as a hidden, accepted no-op and `ANVIL_ACTIVATION_TUI` is
  inert. New coverage: flag-free PTY entry, `--no-tui` holding in a real PTY,
  and byte-identical output with the retired aliases passed.
- **Source:** [ADR-103](../decisions/103-tty-default-activation-tui.md) §4
  rollout ladder, Release 2 (Accepted 2026-07-25); JOURNEY-002 closed the
  just-works gate but deferred the flip itself as a phase-C decision
  ([`release-user-journeys.aps.md`](./release-user-journeys.aps.md))
- **Dependencies:** ACTTUI-008, ACTTUI-009, ACTTUI-010, ACTTUI-012 (all Merged)
- **Intent:** A genuinely interactive `anvil start` opens the activation TUI
  with no flag, while every machine and non-interactive contract stays on the
  deterministic plain path.
- **Expected Outcome:** `start_render_mode` returns `Tui` whenever the
  `activation_tui_eligible` trust boundary holds, without consulting
  `activation_tui_requested`. The trust boundary itself is unchanged: read-only,
  `--watch`, `--json`, `--verify`, `--no-tui`, `ANVIL_NO_TUI`, non-interactive
  environments, and any non-TTY stdio handle still resolve to `Plain`. `--tui`
  and `ANVIL_ACTIVATION_TUI=1` are retired to accepted no-op aliases per
  ADR-103 — still parsed, no deprecation error, no behaviour change when passed.
  `--no-tui` / `ANVIL_NO_TUI=1` remain the permanent escape hatches. `anvil
  start --verify` and `--json` stdout stay byte-stable against the pinned
  ACTTUI-010 fixtures, and the compact plain fixtures continue to describe the
  piped/CI default. Fixtures and tests that assert the plain path only because
  the opt-in flag was absent are re-pinned to the TUI path, and the PTY test
  covers flag-free entry, clean `esc`/`q` exit, and raw-mode restoration. The
  public start-output contract guide documents the default as TUI-on-TTY with
  the escape hatches named.
- **Non-scope:** Consent posture (unticked defaults, gated `ANVIL_HOME`
  suppression), `ProtectionState` vocabulary and honesty pins, orchestrator or
  daemon semantics, removal of the `--tui` flag or the `ANVIL_ACTIVATION_TUI`
  env var, and `celebrate()`/`big-text` (deferred by ACTTUI-012).
- **Files:** `crates/anvil-cli/src/commands/start.rs`,
  `crates/anvil-cli/tests/start.rs`,
  `crates/anvil-cli/tests/fixtures/start-activation/`,
  `apps/e2e/src/cli/activation.e2e.test.ts`,
  `docs/public/anvil/guides/start-output-contracts.md`
- **Validation:** `cargo test -p eddacraft-anvil start`; `cargo test -p
  eddacraft-anvil-tui activation`; `pnpm --filter @eddacraft/anvil-e2e exec
  vitest run src/cli/activation.e2e.test.ts`; `pnpm docs:check`
- **Confidence:** medium

### ACTTUI-014: Continuous live activation surface

- **Status:** Merged 2026-08-02 via PR #3478 — continuous live session; expanded acceptance residuals closed with ACTTUI-015..016 via PR #3488
- **Source:** Owner direction on 2026-08-01;
  [ADR-103](../decisions/103-tty-default-activation-tui.md); consistency
  reference in `crates/anvil-tui/src/surfaces/tutorial/`; completion programme
  [spec](../specs/2026-08-03-activation-tui-completion.md) WP1
- **Dependencies:** ACTTUI-013 and its release gates (all Merged)
- **Intent:** Make interactive `anvil start` feel like the tutorial: one
  branded, continuously updating terminal surface from preflight through
  consent and the final protection verdict — quiet on healthy re-runs, loud on
  repair — using the existing `eddacraft-tui` component vocabulary.
- **Expected Outcome:**
  - The eligible TTY path enters the activation TUI before orchestration work
    begins and keeps one alternate-screen session alive until the user leaves
    the final verdict.
  - Typed `ActivationStepEvent` updates drive `ParallelProgress` / `Spinner`
    while work is running; consent uses unticked `Select`/`Confirm`; apply
    transitions in place to the structured verdict and `LogPanel` evidence
    without a second TUI.
  - **Quiet consent:** when the consent plan has no actionable offers (already
    settled install / nothing to write), skip the Consent phase and land on
    Verdict; when offers exist, defaults remain unticked (CIB-165).
  - **Hand-off:** Working progress chrome is not left dominating the Verdict
    pane; phase strip and help match the active phase.
  - **Single help chrome:** one help surface per phase (shell footer *or*
    in-pane HelpBar/Select footer — not overlapping duplicate key legends).
  - **Pinned Next:** Verdict always surfaces the single arbitrated next step
    (CIB-166 / `arbitrated_next_step`); repair states auto-expand the relevant
    tree sections; healthy `protecting` re-runs keep noise collapsed.
  - Shell branding, progressive phase language, and responsive layout stay
    consistent with the tutorial experience. No new widget unless an existing
    component cannot express a required state.
  - `--verify`, `--json`, piped/CI output, `--watch`, `--no-tui`, and
    `ANVIL_NO_TUI=1` retain deterministic contracts and exit semantics.
- **Non-scope:** Activation-spine semantics; changing consent defaults when
  offers exist; `ProtectionState` vocabulary; machine/plain output fixtures;
  in-surface Prove execution (ACTTUI-016); start↔status field sharing
  (ACTTUI-017); JSON-render / Pretext; widgets for visual novelty only.
- **Files:** `crates/anvil-cli/src/commands/start.rs`,
  `crates/anvil-cli/src/activation/orchestrator/`,
  `crates/anvil-cli/src/tui.rs`, `crates/anvil-cli/tests/start.rs`,
  `crates/anvil-tui/src/surfaces/activation/`,
  `crates/eddacraft-tui/src/widgets/log_panel.rs`
- **Validation:** `cargo test -p eddacraft-anvil-tui activation`; `cargo test
  -p eddacraft-anvil start`; focused flag-free PTY test proving one
  alternate-screen session through consent (when offered) and verdict, and a
  re-run path that skips Consent when offers are empty; `pnpm docs:check`;
  `pnpm aps:active-lint`; `pnpm aps:index:check`; focused clippy for the three
  changed Rust crates
- **Confidence:** high

### ACTTUI-015: Honesty hotfix for Smoke/Prove and help

- **Status:** Merged 2026-08-03 via PR #3488
- **Source:** Live Homebrew validation 2026-08-02/03; completion programme
  [spec](../specs/2026-08-03-activation-tui-completion.md) WP0
- **Dependencies:** none (may land before or during ACTTUI-014)
- **Intent:** Stop teaching users a live-looking Smoke control that neither
  executes nor points at a recipe they will see on healthy re-runs.
- **Expected Outcome:**
  - The verdict `t` control no longer toasts “contract-hardening slice” or
    claims `anvil start --no-tui` always shows the ADTRUST-006 recipe (CIB-183
    re-runs collapse that recipe by design).
  - Until ACTTUI-016 ships real Prove: either (a) hide/remove the Smoke key and
    help binding, or (b) show a toast that states honestly that in-surface Prove
    is not available yet and optionally embeds the three recipe lines in-surface
    without claiming they were executed.
  - Help chrome on Consent and Verdict does not render two competing footers
    that garble key legends.
  - No change to machine/plain contracts; no real check execution in this item
    (that is ACTTUI-016).
- **Non-scope:** Implementing real Prove; consent skip logic (ACTTUI-014);
  status surface changes.
- **Files:** `crates/anvil-tui/src/surfaces/activation/verdict.rs`,
  `crates/anvil-tui/src/surfaces/activation/mod.rs`,
  `crates/anvil-tui/src/surfaces/activation/consent.rs`,
  `crates/anvil-tui/src/surfaces/activation/render.rs`,
  related activation unit/snapshot tests
- **Validation:** `cargo test -p eddacraft-anvil-tui smoke_key`; `cargo test -p
  eddacraft-anvil-tui activation`; assert toast/help copy never contains
  `contract-hardening` or an unconditional `--no-tui` recipe claim
- **Confidence:** high

### ACTTUI-016: Prove protection in-surface

- **Status:** Merged 2026-08-03 via PR #3488
- **Source:** Deferred ACTTUI-005 thin-v1 smoke; ADTRUST-006 recipe;
  completion programme [spec](../specs/2026-08-03-activation-tui-completion.md)
  WP2
- **Dependencies:** ACTTUI-015 (honest control surface); preferred after
  ACTTUI-014 quiet Verdict hand-off
- **Intent:** Give skeptics and first-run users one key that **actually** proves
  the check pipeline can catch a secret-shaped finding — without over-claiming
  MCP pre-write live status.
- **Expected Outcome:**
  - Verdict key (recommended label **Prove**, key may remain `t`) runs the
    ADTRUST-006 throwaway fixture through the real check engine path used by
    `anvil check` (or an equivalent library entry point), reports a real
    `secret-detection` finding on success, and always cleans up the file.
  - Gates disable Prove with an explicit reason when unsupported languages /
    missing check / write-gated project / unwritable disk make proof impossible.
  - Success copy claims **check pipeline** proof only — never “MCP pre-write is
    live” from CLI Prove alone.
  - Global on Verdict (not per Languages row). Fixture language may follow the
    best supported language that can exercise secret-detection.
  - Prefer OS temp (or non-durable path) so daily Prove does not require
    repo-write consent; if the file must live in-repo, one explicit confirm per
    session.
  - Unit + PTY coverage: success path finds the secret; unsupported path is
    suppressed; cleanup runs after success and after failure.
- **Non-scope:** Full `gate` / full-repo scan; language-row actions; auto-ticking
  consent; inventing demo findings; changing ProtectionState vocabulary.
- **Files:** `crates/anvil-cli/src/commands/start.rs` (recipe constants /
  shared prove helper), `crates/anvil-tui/src/surfaces/activation/verdict.rs`,
  `crates/anvil-tui/src/surfaces/activation/mod.rs`, check-engine call site(s),
  `crates/anvil-cli/tests/start.rs` (PTY optional), unit tests for gates and
  cleanup
- **Validation:** `cargo test -p eddacraft-anvil-tui` Prove/smoke filters;
  `cargo test -p eddacraft-anvil` recipe + prove filters; optional PTY prove
  path; secret fixture still matches secret-detection (existing
  `first_run_recipe_smoke_string_triggers_secret_detection` stays green)
- **Confidence:** medium

### ACTTUI-017: Align start and status posture

- **Status:** Merged 2026-08-03 via PR #3488
- **Source:** Live divergence (start `--verify` protecting vs status warming);
  completion programme [spec](../specs/2026-08-03-activation-tui-completion.md)
  WP3
- **Dependencies:** ACTTUI-014 preferred (stable Verdict next-step model); can
  parallel Prove (ACTTUI-016)
- **Intent:** Operators see one story of health whether they open `anvil start`
  or `anvil status`.
- **Expected Outcome:**
  - Start Verdict and status share the same diagnostic fields for protection
    state, MCP client tiers (wired vs live), daemon attestation, save-time
    attached/stale reason, and the single arbitrated next step where both
    surfaces show a next step.
  - No new protection vocabulary; no merging of the two commands into one UI.
  - Documented or tested mapping so “protecting” on start cannot silently mean a
    different claim than status’s primary protection line without an explained
    layer (e.g. save-time stale vs MCP live).
- **Non-scope:** Replacing `anvil status`; redesigning L0–L5 status layout
  wholesale; daemon spine behaviour changes.
- **Files:** `crates/anvil-cli/src/commands/start.rs`,
  `crates/anvil-cli/src/commands/status.rs` (or status render path),
  `crates/anvil-tui/src/surfaces/status/`,
  `crates/anvil-tui/src/surfaces/activation/verdict.rs`, focused tests
- **Validation:** `cargo test -p eddacraft-anvil` status/start filters that pin
  shared next-step / posture fields; `cargo test -p eddacraft-anvil-tui status`
  if TUI status changes; manual: same repo cannot show contradictory primary
  protection claims without an explicit subordinate reason line
- **Confidence:** medium

### ACTTUI-018: Quiet re-run consent — filter settled offers

- **Status:** Merged 2026-08-03 via PR #3499 — registry MCP dry-run filters
  settled clients out of Consent into `settled_mcp`; empty offer set opens
  Verdict without a consent parade
- **Source:** Live validation + post-#3488 usability residual; completion
  programme WP1 gap (empty-offer skip only)
  ([spec](../specs/2026-08-03-activation-tui-completion.md) follow-ups)
- **Dependencies:** ACTTUI-014, ACTTUI-015 (Merged)
- **Intent:** A healthy daily `anvil start` lands on Verdict in seconds without a
  multi-client consent parade when nothing actionable remains.
- **Expected Outcome:**
  - Consent offers exclude **settled** items: MCP clients already at
    safe/live/`AlreadyUpToDate` with no write needed; hooks already installed
    and anvil-managed; workflows already present without drift.
  - When the filtered offer set is empty, skip Consent and open Verdict (same as
    today's empty-plan path).
  - Verdict **Install** (or equivalent) still shows settled clients/hooks as
    read-only posture rows so operators can see what is already wired.
  - Unsafe drift and truly missing clients still appear as unticked offers
    (CIB-165 preserved).
  - PTY/unit: activated repo with Cursor+Claude already configured and no
    hook/workflow delta → no Consent phase; one missing client still offers only
    that client.
- **Non-scope:** Auto-ticking any client; changing non-interactive MCP install
  policy; removing the ability to re-offer a client after deliberate uninstall.
- **Files:** `crates/anvil-cli/src/activation/orchestrator/` (consent plan
  builder), `crates/anvil-cli/src/commands/start.rs`,
  `crates/anvil-tui/src/surfaces/activation/`, `crates/anvil-cli/tests/start.rs`
- **Validation:** `cargo test -p eddacraft-anvil` consent-plan / start filters;
  `cargo test -p eddacraft-anvil-tui activation`; focused PTY re-run without
  Consent when settled
- **Confidence:** high

### ACTTUI-019: Shared start/status posture model

- **Status:** Merged 2026-08-03 via PR #3499 — `SharedPostureFacts` /
  `McpPosture` under `crates/anvil-cli/src/activation/posture.rs` feed start
  Verdict layers and status meaning lines with byte-identical subordinate facts
- **Source:** Residual of ACTTUI-017 (meaning lines only); live protecting vs
  warming divergence
  ([spec](../specs/2026-08-03-activation-tui-completion.md) follow-ups)
- **Dependencies:** ACTTUI-017 (Merged)
- **Intent:** Operators get one coherent protection story across `anvil start`
  and `anvil status`, not just an explanatory footnote.
- **Expected Outcome:**
  - A shared typed posture projection (or equivalent pure helper) feeds both
    start Verdict next-step/layers and status Protection/Next/L0–L2 lines for
    the overlapping fields: protection state or claim, MCP wired vs live, daemon
    attestation, save-time attached/stale, single arbitrated next step.
  - Where vocabularies necessarily differ (`protecting` vs `warming`), both
    surfaces name the **same subordinate facts** so they cannot contradict
    without an explicit layer line.
  - Unit tests pin the shared mapping; no new ProtectionState words.
- **Non-scope:** Merging start and status into one command; rewriting L1/L5
  unknown layers; daemon spine behaviour.
- **Files:** `crates/anvil-cli/src/commands/start.rs`,
  `crates/anvil-cli/src/commands/status.rs`, optional shared module under
  `crates/anvil-cli/src/activation/`, focused tests
- **Validation:** `cargo test -p eddacraft-anvil` shared-posture / status / start
  filters; manual pair of `anvil start --verify` and `anvil status` on a
  warming+L0-on repo shows consistent facts
- **Confidence:** medium

### ACTTUI-020: Settled Install section and honest Languages leaves

- **Status:** Merged 2026-08-03 via PR #3499 — Verdict Install lists settled MCP
  rows; Languages inventory copy states Prove is global, not per-language
- **Source:** Live TUI — Languages expand + `t` felt language-scoped; Install
  noise on re-run
  ([spec](../specs/2026-08-03-activation-tui-completion.md) follow-ups)
- **Dependencies:** ACTTUI-016 (Merged — Prove global); preferred after
  ACTTUI-018 so settled Install rows match filtered consent
- **Intent:** Tree sections set correct expectations: Install shows what is
  already wired; Languages never imply a per-language action that does not exist.
- **Expected Outcome:**
  - Verdict Install (or new Settled subsection) lists configured MCP clients and
    hook posture as non-actionable status rows on re-runs.
  - Languages rows remain coverage inventory only; help does not imply that
    focusing a language then Prove smokes that language.
  - Optional: focusing a language leaf shows a one-line basis (supported /
    partial / unsupported) in the toast or detail strip without running checks.
  - No per-language Prove execution in this item.
- **Non-scope:** Language packs; per-language secret fixtures; changing coverage
  tiers.
- **Files:** `crates/anvil-tui/src/surfaces/activation/verdict.rs`,
  `crates/anvil-cli/src/commands/start.rs` (verdict model builder), snapshots
- **Validation:** `cargo test -p eddacraft-anvil-tui activation`; snapshot or unit
  asserts Install settled rows; help/Prove copy remains global
- **Confidence:** medium

### ACTTUI-021: Optional MCP pre-write prove (honest intercept demo)

- **Status:** Merged 2026-08-03 via PR #3499 — Prove toast always appends MCP
  pre-write honesty from `mcp_pre_write_live()` (refuse/honest path; no false
  live claim from check-pipeline results alone)
- **Source:** Explicit gap after ACTTUI-016 (check-pipeline Prove ≠ MCP live)
  ([spec](../specs/2026-08-03-activation-tui-completion.md) follow-ups)
- **Dependencies:** ACTTUI-016 (Merged); coordinates with ACTMO / daemon
  attestation truth
- **Intent:** When the operator asks, prove **editor pre-write interception**
  honestly — or refuse with a reason — without claiming CLI check results mean
  MCP is live.
- **Expected Outcome:**
  - A distinct control or second mode (not overloading check-pipeline Prove
    success copy) that either: (a) exercises a documented, safe intercept path
    and reports real evidence (daemon + client tier / handshake), or (b) states
    clearly that MCP prove requires a supported editor attach and lists Next.
  - Never upgrades check-pipeline Prove toast to “MCP pre-write is live”.
  - Gates: no live client → unavailable with reason; gated `ANVIL_HOME` respected.
  - Tests for refuse path; success path only if a deterministic harness fixture
    exists (otherwise ship refuse + guidance only and record residual).
- **Non-scope:** Driving a real GUI editor from the CLI; claiming full L0 from
  CLI-only fixtures; replacing `anvil intercept status`.
- **Files:** `crates/anvil-cli/src/commands/start.rs`,
  `crates/anvil-tui/src/surfaces/activation/`, intercept/status helpers as needed
- **Validation:** unit tests for refuse/honest copy; optional integration if a
  fixture harness exists; must not break ACTTUI-016 check-pipeline Prove
- **Confidence:** low


## Sequencing

```text
Phase A (foundation — serial) — shipped:
  ACTTUI-000 → 001 → 002
  (+ shared HelpBar / Select chrome → 008)

Phase B (surfaces — parallel after 002) — shipped:
  Start track:    ACTTUI-003 → 004 → 005
  Welcome track:  ACTTUI-008 ∥ WOW-001 → 002 → 003 → 004 → 005

Phase C (ship gate) — shipped:
  ACTTUI-007 ∥ ACTTUI-006
  Review remediation: ACTTUI-009 → 010; 011 after 009; 012 before default flip
  Default flip: ACTTUI-008 ∧ 009 ∧ 010 ∧ 012 → ACTTUI-013 (Merged)

Completion programme (owner-approved 2026-08-03) — Merged via #3478/#3488:
  ACTTUI-015 (honesty) ──parallel──► ACTTUI-014 (continuous)
           │                                  │
           └──────────► ACTTUI-016 (Prove) ◄──┘
  ACTTUI-017 (start↔status meaning) Merged

Usability follow-ups (filed 2026-08-03) — Merged via #3499:
  ACTTUI-018 (quiet re-run consent) → ACTTUI-020 (settled Install / Languages)
  ACTTUI-019 (shared posture model) parallel after 017
  ACTTUI-021 (MCP pre-write prove honesty; refuse/honest path)
```

**Release history:** ACTTUI-009 → 010 → 012 were JOURNEY cut gates for the
TTY-default flip (ACTTUI-013). Completion items 014–017 Merged via #3478/#3488.
Follow-ups 018–021 Merged via #3499; they do not reopen machine contracts.

## Planning Council record (2026-07-08, direction-validate)

**Decision:** `proceed` with `amend` (unanimous across pragmatic, operations,
security, adversarial lenses).

| Lens | Verdict | Key amendment |
| ---- | ------- | --------------- |
| Pragmatic | Proceed | ACTTUI-002 orchestrator event contract is programme risk; 007 minimum before TTY-default flip |
| Operations | Proceed | `--no-tui`-only CI today; PTY matrix required; `ANVIL_NO_TUI` must exist; fixture-first |
| Security | Proceed | MCP unticked parity (CIB-165); `ANVIL_HOME` persistent banner; hook consent |
| Adversarial | Conditional proceed | Opt-in rollout first; `--watch` teardown mandatory; consider plain CIB fixes in parallel |

**Risks accepted:**

- First production load on `Tree` / `ParallelProgress` (eddacraft-tui burn-down)
- E2e/snapshot churn when compact plain and TTY paths diverge
- Welcome hub lags activation chrome until ACTTUI-008 (mitigate: extract shared widgets early)

**Checks before ACTTUI-001 → In Progress:**

- [x] Planning council direction-validate complete (this record)
- [x] ACTTUI-000 ADR + fixture spec landed
- [ ] `pnpm aps:index` after first implementation item completes
