# Activation TUI — `anvil start` as the flagship eddacraft-tui surface

| ID     | Owner | Status | Progress |
| ------ | ----- | ------ | -------- |
| ACTTUI | Josh  | Ready  | 0/9      |

**Last reviewed:** 2026-07-08 — module created via planning-workflow +
planning-council direction-validate (four-lens stress test). Replaces the
plain-text activation dossier and `demand` pickers with a single interactive
surface built on `eddacraft-tui` widgets. **Start first, welcome second** —
execution waves land activation before welcome convergence; the module plans the
whole first-run pair but does not block WOW tutorial work.

**Council decision:** `proceed` with `amend` — do not mark ACTTUI-001 **In
Progress** until ADR/decision-log entry for TTY-default activation TUI and a
pinned plain-output fixture spec exist (see Planning Council record below).

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
| **B — Parallel surfaces** | ACTTUI-003..005 ∥ ACTTUI-008 ∥ WOW-001..004 | Start activation phases and welcome uplift on separate files; converge visually in the same train |
| **C — Ship gate** | ACTTUI-006 ∥ ACTTUI-007 | Contracts, PTY matrix, TTY-default flip only when **both** surfaces pass |

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
  a Non-scope justification in the work item.

## Coordination

| Module / audit | Relationship |
| -------------- | ------------ |
| ACTMO | Spine semantics frozen; ACTTUI consumes `ActivationDiagnostic` + `InstallReport` |
| WOW | Tutorial/welcome story; ACTTUI-008 lands after WOW-001..002 or in parallel on non-overlapping files |
| CIB-162..179 | Many repair items become easier or obsoleted by TUI path; do not close CIB items in ACTTUI PRs unless the fix is verified |
| ADTRUST-006 | First-run recipe moves in-surface; compact plain fallback retains a one-line "run `anvil status`" next step |
| TUIN / eddacraft-tui | Enable `big-text` on `anvil-tui`; consider promoting `Tree`/`ParallelProgress` to `stable` if activation depends on them |
| DSV-046..051 | Daemon attestation copy in `LogPanel`; no change to attestation rules |

## Work Items

### ACTTUI-000: UX contract + plain-output fixtures (planning gate)

- **Status:** Ready
- **Intent:** Load-bearing public UX change and scripting contracts are documented
  before code lands.
- **Expected Outcome:** ADR or DECISION-LOG entry for TTY-default activation
  TUI (with trust-boundary table: interactive vs `--no-tui`/piped); pinned
  fixture spec for `anvil start --verify`, `--json`, and compact plain
  (`tests/fixtures/start-verify/` or equivalent); release-note template
  ("scripting: `--no-tui`; probing: `--verify`/`--json`"); rollout ladder:
  opt-in (`--tui` or `ANVIL_ACTIVATION_TUI=1`) for first release, flip
  TTY-default only after contract matrix green.
- **Files:** `plans/decisions/`, `docs/public/anvil/guides/`, fixture directory
- **Validation:** `pnpm docs:check`; council checklist ticked
- **Confidence:** high

### ACTTUI-001: Activation surface scaffold + plain fallback contract

- **Status:** Ready
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

- **Status:** Ready
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

- **Status:** Ready
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

- **Status:** Ready
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
  `crates/anvil-cli/src/activation/orchestrator/mod.rs` (workflow picker)
- **Validation:** `cargo test -p anvil-tui -p anvil-cli`; e2e: workflow Enter-without-tick writes nothing
- **Confidence:** high

### ACTTUI-005: Verdict phase — `Tree`, `StatusBadge`, `BigBanner`, `Toast`, smoke test

- **Status:** Ready
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

- **Status:** Ready
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

- **Status:** Ready
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

- **Status:** Ready
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

## Sequencing

```text
Phase A (foundation — serial):
  ACTTUI-000 → 001 → 002
  (+ early extract: shared HelpBar / Select chrome → consumed by 008)

Phase B (surfaces — parallel after 002):
  Start track:    ACTTUI-003 → 004 → 005
  Welcome track:  ACTTUI-008 ∥ WOW-001 → 002 → 003 → 004
  (WOW-005/006 remain Draft — out of this release cohort unless design gates close)

Phase C (ship gate — both surfaces must be green):
  ACTTUI-007 ∥ ACTTUI-006
  TTY-default flip only when start + welcome snapshots/e2e pass together
```

**Release blocker:** ACTTUI-008 + WOW-001..004 complete, not optional for the
cohort release. ACTTUI-006 (LogPanel / json_render) remains deferrable within
the cohort if schedule bites.

ACTTUI-000 is the first item to mark **In Progress** when implementation starts.

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
- [ ] ACTTUI-000 ADR + fixture spec landed
- [ ] `pnpm aps:index` after first implementation item completes