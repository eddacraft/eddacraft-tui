# Activation TUI — `anvil start` as the flagship eddacraft-tui surface

| ID     | Owner | Status | Progress |
| ------ | ----- | ------ | -------- |
| ACTTUI | Josh  | In Progress | 6/14      |

**Last reviewed:** 2026-07-25 — [ADR-103](../decisions/103-tty-default-activation-tui.md)
**Accepted** by the owner, and the Release 2 TTY-default flip is filed as
**ACTTUI-013**. Its named gates — ACTTUI-008 welcome convergence,
ACTTUI-009 consent wiring, ACTTUI-010 contract/PTY matrix, ACTTUI-012 polish —
are all Merged, so the 2026-07-09 council block on the flip is cleared and the
phase-C decision JOURNEY-002 deferred now has an owning work item. On `main`,
`anvil start` remains opt-in behind `--tui` / `ANVIL_ACTIVATION_TUI=1`; the flip
itself is in flight as ACTTUI-013 (In Progress, PR #3411), which retires those
two to inert aliases and keeps `--no-tui` / `ANVIL_NO_TUI=1` as the permanent
escape hatches.

Earlier review (2026-07-11) — the operator-approved
[`JOURNEY` conductor](./release-user-journeys.aps.md) makes ACTTUI-009/-010/-012
and WOW-005 release-cut gates, while retaining celebration and richer diagnostics
as coordinated enhancements. The 2026-07-10 ACTTUI-009..011 implementation
milestone is active on `feat/acttui-009-consent-wiring`. Consent now binds the
exact selected targets and applies only after explicit submission; the contract matrix includes
real PTY restoration and all-phase snapshots; verdict/evidence models are built
from typed activation data. Targeted Rust, Clippy, PTY, snapshot, and activation
e2e checks pass locally. ACTTUI-012 has since Merged via PR #3284. The earlier
post-ACTTUI first-run council review
([`2026-07-09-acttui-first-run-journeys.md`](../reviews/2026-07-09-acttui-first-run-journeys.md))
blocked the TTY-default flip because the opt-in `--tui` consent path is still a
dead end. ACTTUI-009..012 now track the remediation wave. ACTTUI-000 planning
gate merged (PR #3232); ACTTUI-001 activation-surface scaffold, ACTTUI-002
progress events, ACTTUI-003 working progress, ACTTUI-004 consent chrome,
ACTTUI-005 verdict tree, ACTTUI-006 tier-evidence LogPanel, and ACTTUI-007
fixture hardening are In Progress across the stacked ACTTUI branches. ADR-103
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
- In-surface smoke-test recipe (optional `t` key) with honest finding display
- `LogPanel` for tier-evidence / daemon skip detail (replaces JSON WARN leak on
  human path — CIB-162)
- Snapshot + e2e regression matrix for TTY vs plain vs verify

## Out of Scope

- Replacing `anvil status` TUI (may later share `Tree` renderer; not this module's
  first slice)
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
  `project_writes_gated`; wow affordances (`BigBanner`, in-surface smoke)
  suppressed; consent phase skips repo-scoped offers with visible reason.
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
| ADTRUST-006 | First-run recipe moves in-surface; compact plain fallback retains a one-line "run `anvil status`" next step |
| TUIN / eddacraft-tui | Enable `big-text` on `anvil-tui`; consider promoting `Tree`/`ParallelProgress` to `stable` if activation depends on them |
| DSV-046..051 | Daemon attestation copy in `LogPanel`; no change to attestation rules |
| JOURNEY | Release conductor; ACTTUI-009/-010/-012 are cut gates, while ACTTUI-005 celebration and -006/-011 diagnostic depth remain coordinated enhancements |

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

- **Status:** In Progress 2026-07-08 — surface scaffold + opt-in dispatch landed
  on `feat/acttui-001-scaffold`. New
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

- **Status:** In Progress 2026-07-08 — activation progress event contract implemented
  on `feat/acttui-002-events` (stacked on ACTTUI-001). Added typed
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

- **Status:** In Progress 2026-07-08 — activation surface renders orchestrator
  progress rows via shared `eddacraft-tui` `ParallelProgress` and shows an Anvil
  spinner for daemon ensure on `feat/acttui-003-working-phase` (stacked on
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

- **Status:** In Progress 2026-07-08 — consent model and renderer in
  `crates/anvil-tui/src/surfaces/activation/consent.rs` use `Select`,
  `Confirm`, and `OverlayStack`; rows default unticked, gated `ANVIL_HOME`
  disables repo-scoped writes, and unsafe drift opens an acknowledgement overlay
  without implicit selection. Orchestrator handoff remains through the
  ACTTUI-002 deferral seam. **Remaining gap (flagged by Copilot review on
  #3238):** `commands/start.rs` never constructs a `ConsentState` or calls
  `ActivationSurface::with_consent(...)`, so even though the deferred
  MCP/workflow progress rows correctly drive `phase == Consent`, the surface
  still falls through to the verdict renderer — the consent picker is not yet
  reachable, and there is no apply-selection path to actually install after
  the picker interaction. That wiring is the remaining scope for this item.
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

- **Status:** In Progress 2026-07-08 — structured verdict model/view in
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

- **Status:** In Progress 2026-07-09 — ACTTUI evidence rows now feed an
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

- **Status:** In Progress 2026-07-09 — fixture-backed read-only/JSON/no-TUI
  contract tests now pin the reachable `ready_restart_required` path in
  `crates/anvil-cli/tests/fixtures/start-activation/`, with e2e parity coverage
  for `--no-tui` and `ANVIL_NO_TUI=1`. The `protecting` and PTY transcript
  matrix remains gated on a harness that can synthesise live MCP/daemon
  attestation without over-claiming.
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

- **Status:** In Progress
- **Progress:** Implementation open as PR #3411 — `start_render_mode` derives
  from `activation_tui_allowed` (pure argument/environment policy, unit-testable
  without a PTY) plus the stdio terminal probe, with the opt-in gate removed.
  `--tui` is retained as a hidden, accepted no-op and `ANVIL_ACTIVATION_TUI` is
  inert. New coverage: flag-free PTY entry, `--no-tui` holding in a real PTY,
  and byte-identical output with the retired aliases passed. Flip to
  `Merged YYYY-MM-DD via PR #3411` on merge.
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

## Sequencing

```text
Phase A (foundation — serial):
  ACTTUI-000 → 001 → 002
  (+ early extract: shared HelpBar / Select chrome → consumed by 008)

Phase B (surfaces — parallel after 002):
  Start track:    ACTTUI-003 → 004 → 005
  Welcome track:  ACTTUI-008 ∥ WOW-001 → 002 → 003 → 004 → 005
  (WOW-006 remains Proposed as JOURNEY-007 post-cut expansion)

Phase C (ship gate — both surfaces must be green):
  ACTTUI-007 ∥ ACTTUI-006
  TTY-default flip only when start + welcome snapshots/e2e pass together

Review remediation (post council-d4c804e6):
  ACTTUI-009 → ACTTUI-010
  ACTTUI-011 may run after 009
  ACTTUI-012 blocks only the default flip

Default flip (ADR-103 Release 2, unblocked 2026-07-25):
  ACTTUI-008 ∧ 009 ∧ 010 ∧ 012 (all Merged) → ACTTUI-013
```

**Release blocker (JOURNEY conductor, 2026-07-11):** ACTTUI-009 → ACTTUI-010 →
ACTTUI-012, plus WOW-005 and the JOURNEY candidate rehearsal. ACTTUI-006
(LogPanel / json_render) and ACTTUI-011 typed diagnostic depth remain coordinated
enhancements; they do not block the cut unless required to close a correctness
defect found by the contract matrix.

ACTTUI-000 is complete; ACTTUI-001 is In Progress (scaffold + opt-in dispatch on `feat/acttui-001-scaffold`). ACTTUI-002 (orchestrator progress events) is the next serial foundation item.

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
