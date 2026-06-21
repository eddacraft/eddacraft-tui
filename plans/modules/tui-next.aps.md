<!-- APS: See https://github.com/eddacraft/anvil-plan-spec for format reference -->
<!-- Executable only if tasks exist and status is Ready. -->

# TUI Next

| ID   | Owner      | Status   | Progress |
| ---- | ---------- | -------- | -------- |
| TUIN | joshuaboys | In Progress | 4/13     |

**Last reviewed:** 2026-06-21 (gate reconciliation): the Ready-Checklist drift
gate is **met** — D-TUIR-018 mirror-drift-check green for 15+ consecutive daily
runs (2026-06-10..2026-06-21), well past the 7 required; the ADR-slot and
feature-flag-collision checklist items are also ticked (ADR-050 Accepted via
TUIN-001; `lifecycle`/`runner` shipped via TUIN-012 without collision). TUIN-004
reconciled to **Ready** — its implementation landed via TUIN-012
(`src/lifecycle.rs` + `lifecycle` feature + `TerminalGuard`/`restore_terminal`/
panic hook); only the dedicated `tests/lifecycle_panic.rs` test remains. TUIN-003
stays **open/Blocked** on D-TUIN-002 (mode-detection ownership), still `Proposed`
— accept that decision to unblock it. TUIR is now archived
([`plans/archive/modules/tui-reintegration.aps.md`](../archive/modules/tui-reintegration.aps.md)).
No done/total change (4/13). Prior: 2026-06-09 (TUIN-013 added Proposed: first-class public
docs-site section for `eddacraft-tui`, sibling to `/aps` and `/kindling`,
narrative-only with API reference linked to docs.rs; surfaced alongside the
runner BYO-parser docs PR #2462). Prior: 2026-06-08 (TUIN-012 Done:
feature-gated `lifecycle` and `runner` modules landed in
`crates/eddacraft-tui/`, using `lexopt` for shared global parsing, config-path
handoff, and typed mode/theme hints while preserving consumer-owned lifecycle,
render-loop, and command semantics; operator override accepted ADR-050 and
promoted TUIN-012 before the seven-consecutive-green mirror-drift observation
window completed; TUIR-008 is closed and the override is limited to the ADR-050
runner fallback CLI shell boundaries). Prior: 2026-06-08
(ADR-050 fallback CLI shell amendment; TUIN-011/-012 added). Prior: 2026-05-28 (opportunistic colour-harmonisation
spike; TUIN-009/010 added); 2026-05-23 (ADR-050 design pass).

> **Execution gate:** TUIN is gated on TUIR-008 (canonical source live in
> `crates/eddacraft-tui/`, mirror healthy, first crates.io publish from
> canonical source verified, TUIMIRROR archived). Until TUIR-008 closes,
> this module is planning context only — no task may be promoted from
> `Proposed` to `Ready`. **Exception (2026-05-23):** TUIN-001 is the
> ADR-drafting task (changeType: docs, releaseIntent: never) — it
> landed as planning material without violating the gate, and module
> header progress advances 0/8 → 1/8 to reflect the closed task.
> Implementation tasks (TUIN-003 onward) remain inert until the gate
> opens.
>
> **Exception (2026-05-28):** TUIN-009 is a spike/decision item
> (changeType: spike, releaseIntent: never) authorised by the operator
> (joshuaboys) to land as planning material under the gate — like
> TUIN-001 it merges no implementation, so it does not violate the gate;
> it records a ship/no-ship signal for opportunistic colour
> harmonisation and advances header progress to 2/10. TUIN-010 captures
> the remaining tuie port candidates as `Proposed` backlog. The
> colour-harmonisation **implementation** itself stays inert until
> TUIR-008 closes.
>
> **Exception (2026-06-08):** TUIN-011 is a docs-only planning
> amendment requested by the operator after two imminent downstream
> adopters surfaced a concrete need for CLI tools with subcommands,
> flags, and configuration. Like TUIN-001 and TUIN-009, it does not
> land implementation and therefore does not open TUIN's implementation
> gate. It amends ADR-050 and this module so TUIN-002 survey work and
> TUIN-012 implementation work target a fallback CLI shell rather than a
> single-app launcher; TUIN-004 remains the lifecycle dependency that
> `runner` composes.
>
> **Operator override (2026-06-08):** after TUIR-008 closed, the operator
> authorised TUIN-012 implementation before the D-TUIR-018 drift check observes
> seven consecutive green runs. The override does not waive the runner boundary:
> `runner` remains opt-in, parser-light, `clap`-free, and consumer-owned for
> command semantics; no default `[[bin]]` is introduced.
>
> **Relationship to TUIR:** TUIR moves the source of truth without
> behaviour change. TUIN is the first batch of design work that becomes
> *affordable* because the source moved. Every item below is deliberately
> deferred out of TUIR's Out-of-Scope list (items 25–28 of the
> post-migration checklist: CLI/fallback timing, CLI independence, clap
> policy, terminal lifecycle ownership) to keep migration blast radius
> narrow. Anything that would expand TUIR scope belongs here instead.

## Purpose

Resolve the design questions about CLI surface, argument-parser policy,
terminal lifecycle ownership, demo binary shape, and extension-surface
stability that the TUIR migration explicitly deferred. With canonical
source in Anvil and the release loop reduced from days to minutes, these
decisions can land with atomic widget + consumer + test changes — but
they should land *deliberately*, with their own decision record and CI
evidence, not as drive-by additions to TUIR or to ad-hoc release PRs.

**Why:** TUIR's discipline was zero behaviour change at the public API.
The moment work that is *not* migration starts landing inside TUIR
scope, both reviews and rollback get worse. TUIN exists so post-migration
design work has its own runway, its own ADRs, and its own validation
surface.

## In Scope

- Argument-parser policy in `eddacraft-tui` core (`clap`, `lexopt`,
  parser-agnostic helpers, or none).
- Fallback CLI shell shape for runner consumers: global flags,
  first-level subcommand dispatch, config-file handoff, and the
  boundary between shared shell plumbing and consumer-owned command
  semantics.
- CLI mode-detection helpers (TTY, alt-screen capability, terminfo
  probes) — does the crate ship them, and at what stability tier.
- Terminal lifecycle ownership: alt-screen entry/exit, raw mode
  toggle, panic-restore handler — `eddacraft-tui` responsibility or
  app-owned.
- Demo / fallback binary surface: `examples/`, `[[bin]]`, or neither.
- Downstream extension surface stability: widget trait, theme override
  hook, snapshot harness exposure.
- API stability checkpoint after N releases from canonical source —
  decision document, not a refactor commitment.
- Opportunistic colour / theming helpers that fit the post-migration
  affordance window — e.g. terminal-harmonised palette generation
  (TUIN-009 spike) and follow-on opt-in theme helpers that consumers can
  adopt without architectural change.

## Out of Scope

- Anything that belongs in TUIR (canonical-source migration mechanics,
  mirror automation, release plumbing, CI gate split). If a TUIN
  decision exposes a TUIR gap, file the gap against TUIR; do not
  expand TUIR scope from inside TUIN.
- Anvil-internal TUI surface redesign (`crates/anvil-tui/`,
  `crates/anvil-cli/`). TUIN owns the shared crate's contract, not
  its consumers' surfaces.
- New widgets or accessibility features unless they necessarily fall
  out of a TUIN decision (e.g. lifecycle ownership forcing an API
  addition). (Opportunistic colour / theming helpers are now in scope —
  see the corresponding In-Scope bullet.)
- Changes to crates.io publication policy (D-TUIR-005 / D-TUIR-006
  binding).
- Splitting `eddacraft-tui` into multiple published crates. ADR-050
  rejected the sibling-crate path (`eddacraft-tui-cli`) in favour of
  the in-crate `runner` feature flag; any future split would require
  its own ADR. The original framing left this option open under
  TUIN-001 / TUIN-003 — it is now closed.
- Sunsetting any existing public feature flag (`image`, `big-text`,
  `test-utils`). Deprecation requires a separate ADR.

## Interfaces

**Depends on:**

- TUIR — canonical source live in `crates/eddacraft-tui/`, mirror
  workflow healthy, drift check green (D-TUIR-018). TUIN cannot start
  until TUIR-008 closes.
- ADR-047 — accepted and load-bearing.
- ATTRIB-011 mirror precedent — TUIN may extend CI gates but must not
  duplicate mirror plumbing.
- Anvil consumers (`crates/anvil-tui/`, `crates/anvil-cli/`) — define
  the integration surface that drives lifecycle and CLI decisions.
- External downstream consumers (`eddacraft-skills`, the two imminent
  `eddacraft-tui` adopters raised on 2026-06-08, and any others
  enumerated in the TUIR-001 baseline) — surveyed during TUIN-002.

**Exposes:**

- A set of accepted ADRs covering parser/CLI policy, lifecycle
  ownership, and extension-surface stability. Next free ADR slot at
  drafting time is `050`; numbers are allocated when each ADR PR
  opens.
- New `eddacraft-tui` feature flags for opt-in helpers: `runner`
  (D-TUIN-001 / ADR-050) and `lifecycle` (D-TUIN-003, enabled
  transitively by `runner`).
- No sibling CLI crate in the current plan. ADR-050 rejects
  `eddacraft-tui-cli`; any future split requires a new ADR.
- API-stability checkpoint document
  (`docs/runbooks/eddacraft-tui-api-checkpoint.md`, new).

## Decisions

**D-TUIN-001:** Turn-key runner + argument-parser policy

- **Accepted resolution:** `eddacraft-tui` core has zero
  argument-parser dependency. The crate ships an opt-in `runner`
  feature flag (defaulted OFF) that bundles a small fallback CLI shell:
  `launch_cli<Cli: TerminalCli>(cli) -> ExitCode` for consumers that
  need subcommands, flags, and configuration handoff from a few
  lines of `main.rs`; a narrower `launch_default<App: TerminalApp>`
  adapter may remain for single-app consumers if TUIN-004 keeps it.
  Runner consumers inherit argument parsing for shared global flags
  (`--help`, `--version`, `--theme`, `--no-tui`, `--config`),
  first-level subcommand dispatch, config-file path handoff, and typed
  hints for theme and TUI/plain mode. Consumer crates own terminal
  lifecycle entry, render-loop behaviour, theme application, domain
  command semantics, command-specific parsing, nested command trees,
  completions, env binding, rich validation, and config format / merge
  semantics. The runner's minimal parser is `lexopt`
  (zero transitive deps, ~300 LoC); TUIN-003 / TUIN-004 may swap
  without re-ADR. The sibling-crate alternative (`eddacraft-tui-cli`) is
  rejected in favour of the feature-flag path for lower consumer
  friction; future split would require its own ADR. Default-on
  argument parsing and a default `[[bin]]` in core both remain
  rejected — the runner is opt-in via the feature flag, and the
  `[[bin]]` lives in each consumer crate (D-TUIN-004 unchanged). The
  TUIR Out-of-Scope clause forbidding `clap` in core is upheld and
  carried forward.
- **Status:** Accepted by ADR-050 and implemented by TUIN-012 (see
  [`050-eddacraft-tui-runner-and-cli-policy.md`](../decisions/050-eddacraft-tui-runner-and-cli-policy.md);
  amended 2026-06-08 by TUIN-011 to cover the fallback CLI shell and
  accepted under the TUIN-012 operator override).

**D-TUIN-002:** CLI mode-detection helpers ownership

- **Proposed resolution:** TTY / alt-screen / terminfo / colour-depth
  probes live in `eddacraft-tui` core as small, parser-free helpers
  with zero new dependencies (stdlib + `crossterm` or whatever the
  crate already pulls). They are NOT behind a feature flag — they form
  part of the core API surface. Anvil keeps its mode resolver but may
  delegate the *probe* to the shared crate. Helpers return typed
  enums, not raw capability bits — typed enums force consumers to
  handle the cases the crate decides matter (TtyKind variants,
  ColourDepth steps) rather than leaking probe internals.
- **Status:** Proposed.

**D-TUIN-003:** Terminal lifecycle ownership

- **Accepted resolution:** Lifecycle helpers (alt-screen enter/exit,
  raw mode set/clear, panic-restore handler, signal-driven cleanup)
  live in `eddacraft-tui` behind an opt-in `lifecycle` feature flag,
  which is *also* enabled transitively by the `runner` flag from
  D-TUIN-001 — runner consumers do not compose the two flags
  manually. The Anvil-side reference implementation
  (`crates/anvil-cli/src/tui.rs:46-101` — `TerminalGuard`,
  `install_panic_hook`, `restore_terminal`) drives the API shape
  TUIN-012 lifts into core. ADR-050 records that Anvil itself does
  not adopt the lifted helpers via the runner — it keeps its local
  `TerminalGuard` until a separate ADR authorises a migration.
  Defaults are conservative: the feature is OFF for any consumer
  that pulls `eddacraft-tui` for widget rendering only (apps that
  own their lifecycle, like Anvil, ignore it; apps that want
  managed lifecycle opt in directly or via `runner`).
- **Status:** Partially implemented by TUIN-012. `TerminalGuard` and
  panic-restore landed behind `lifecycle`, and `runner` enables the
  feature transitively. Signal-driven cleanup remains unimplemented and
  should stay in TUIN-004 or a follow-up lifecycle hardening item.

**D-TUIN-004:** Demo / fallback binary

- **Proposed resolution:** `eddacraft-tui` ships an `examples/` demo
  per widget family, runnable via `cargo run --example <name>`. It
  does NOT ship a `[[bin]]` target. Reason: `[[bin]]` pulls a CLI
  dependency surface (D-TUIN-001) into the published crate's default
  build; examples don't and remain opt-in. README links the examples
  list for downstream readers.
- **Status:** Proposed.

**D-TUIN-005:** Downstream extension-surface stability

- **Proposed resolution:** Widget trait surface and theme override hook
  are documented as stable post-TUIR. Breaking changes require a major
  version bump AND an ADR. Snapshot harness exposure (`test-utils`
  feature) stays explicitly unstable; consumers depend on it at their
  own risk. Stability markers land via docs convention initially
  (`# Stability` rustdoc section); switching to a stability crate is a
  separate decision.
- **Status:** Proposed.

**D-TUIN-006:** Post-migration API stability checkpoint

- **Proposed resolution:** After three crate releases from canonical
  source (baseline `0.2.5`, assuming `0.2.3` and `0.2.4` ship during
  migration shakedown), produce an API-stability checkpoint document
  reviewing what changed, what downstream consumers reported, and
  whether a `0.3.0` stabilisation pass is warranted. The checkpoint is
  a decision artefact, not a commitment to bump — refactors require
  their own modules and ADRs.
- **Status:** Proposed.

## Risks

- **Scope creep into TUIR.** Anything that touches mirror or publish
  plumbing belongs in TUIR, not TUIN. Mitigation: explicit
  Out-of-Scope clause; reviewer awareness when a TUIN PR strays into
  `.github/workflows/mirror-*.yml` or `publish-*.yml`.
- **Default-on CLI / lifecycle helpers break Anvil's existing
  ownership.** Mitigation: D-TUIN-001 (parser-off) and D-TUIN-003
  (lifecycle feature-off) defaults are conservative; Anvil consumers
  do not gain new behaviour without opting in.
- **`examples/` drift into a de facto bin.** Mitigation: D-TUIN-004
  forbids `[[bin]]`; TUIN-005 validation greps `Cargo.toml` to back
  it up.
- **API stability checkpoint becomes a slow-rolling refactor.**
  Mitigation: D-TUIN-006 treats the checkpoint as a document only.
  Any recommended action gets its own follow-up module.
- **Downstream consumers (`eddacraft-skills` etc.) build their own
  lifecycle helpers before TUIN-003 lands.** Mitigation: post-TUIR
  cadence is fast; TUIN-002 survey is an early item so consumer
  expectations land before code does.
- **Fallback shell expands into an unbounded CLI framework.** Mitigation:
  ADR-050's 2026-06-08 amendment pins the boundary at global flags,
  first-level subcommand dispatch, config handoff, lifecycle, and render
  loop. Nested commands, completions, env binding, rich validation, and
  domain command semantics stay consumer-owned unless a future ADR
  explicitly changes that boundary.
- **Stability annotations rot.** Rustdoc-comment stability markers
  drift from reality if not enforced. Mitigation: TUIN-006 outcome
  includes a CI grep that flags new public items missing a stability
  marker (warn-only initially per Anvil's warnings-over-blocks rule).
- **TUIN decisions get pre-empted by emergency feature work during
  migration shakedown.** Mitigation: if a downstream consumer files a
  P1 against a TUIN-scoped surface before TUIR-008 closes, escalate
  via a focused TUIR follow-up (TUIR-009 etc.), not via opening TUIN
  early.

## Work Items

### TUIN-001: Author ADR-050 — runner helper and CLI / parser policy

- **Status:** Merged 2026-05-23 via PR #1883

**Intent:** Lock D-TUIN-001..-003 into a numbered ADR before any
helper code lands. The ADR is the load-bearing artefact for "no
parser in core", "mode probes are core API", "lifecycle helpers
opt-in", and the turn-key `runner` feature surfaced during the
2026-05-23 design pass — all easy to re-litigate without it.

**Outcome:**
[`plans/decisions/050-eddacraft-tui-runner-and-cli-policy.md`](../decisions/050-eddacraft-tui-runner-and-cli-policy.md)
landed with status Proposed, covering parser-free core, the
turn-key `runner` feature contract (opt-in flag bundling lifecycle
+ minimal `lexopt`-based parser + `TerminalApp` trait +
`launch_default(app) -> ExitCode` entry point), mode-detection
helper shape (typed enums), the sibling-crate-vs-feature-flag
decision (chose feature flag), the Anvil-side non-adoption note
(Anvil keeps `crates/anvil-cli/src/tui.rs`), rejected alternatives
(`clap` default-on, `[[bin]]` in core, runner-in-sibling-crate, no
runner at all), and the carry-forward of the TUIR `clap` clause.

**Validation:** `pnpm adr:check` reports `next available ADR
number: 051` after ADR-050 lands; ADR appears in `DECISION-LOG.md`
under the Product and Distribution section (matching ADR-047);
`pnpm run format:check` passes; cross-links from ADR-050 back to
TUIN module resolve.

**changeType:** docs
**releaseIntent:** never
**releaseScope:** none

### TUIN-002: Survey downstream consumers for lifecycle, CLI, and runner expectations

- **Status:** open
- **Intent:** Before D-TUIN-003 / D-TUIN-001 land code, capture what
  each known downstream consumer (`eddacraft-skills` plus any
  documented external consumers from TUIR-001 baseline) expects from
  terminal lifecycle ownership, CLI helpers, AND the turn-key
  `runner` contract ADR-050 locks — do they own lifecycle, want it
  managed, or wrap it; do they bring their own parser, want one for
  free, or have no CLI today; what subcommands, global flags, and
  config-loading shape they need; what shape would their
  `Cli: TerminalCli` or `App: TerminalApp` implementation take.
- **Expected Outcome:**
  `plans/specs/2026-XX-XX-eddacraft-tui-post-migration-survey.md`
  with one section per consumer documenting current behaviour (CLI /
  TUI / either / neither), desired behaviour under TUIN (adopt runner /
  adopt lifecycle helpers only / stay fully bespoke), the rough shape
  of what the consumer would pass into `launch_cli(cli)` or
  `launch_default(app)` (command enum, config type, event type, render
  frame call, exit conditions), which shared global flags (`--help`,
  `--version`, `--theme`, `--no-tui`, `--config`) suffice vs need
  extension via `RunnerOptions`, whether first-level dispatch is enough
  or nested command trees / completions / rich validation require a
  consumer-owned parser, and any pain points or constraints
  (Windows-only widgets, custom panic-restore needs, alt-screen
  incompatibility, config format / merge ownership). The survey
  explicitly covers library-shaped consumers without a CLI today
  (`eddacraft-skills`, the two imminent `eddacraft-tui` adopters raised
  2026-06-08, future Rust ports of `anvil-plan-spec`, and other current
  TS-only packages) as the primary `runner` target, and Anvil-shaped
  consumers (`crates/anvil-tui/`, `crates/anvil-cli/`) as the
  already-has-its-own-CLI counter-example the runner must *not* absorb
  (per ADR-050's Anvil non-adoption note). Survey output drives the
  `TerminalCli` / `TerminalApp` trait shape that TUIN-003 / TUIN-004
  implement; gaps flagged for follow-up.
- **Validation:** survey doc lists each known consumer from the
  TUIR-001 baseline plus any new ones identified via the runner
  framing; gaps explicitly flagged for follow-up; ADR-050 referenced
  as the locking decision the survey informs.

**changeType:** docs
**releaseIntent:** never
**releaseScope:** none

### TUIN-003: Implement CLI mode-detection helpers in core

- **Status:** open — **Blocked:** D-TUIN-002 (mode-detection ownership) is
  still `Proposed`; its resolution (probes in `eddacraft-tui` core, parser-free,
  zero new deps, typed enums) must be Accepted before this promotes to Ready.
  `crates/eddacraft-tui/src/mode/` does not yet exist (genuinely unstarted).
- **Intent:** Land TTY / alt-screen capability / colour-depth / terminfo
  probes in `eddacraft-tui` core per D-TUIN-002, with zero new
  dependencies and typed-enum return values.
- **Expected Outcome:** New module under `crates/eddacraft-tui/src/mode/`
  with typed enums (`TtyKind`, `AltScreenSupport`, `ColourDepth`, etc.).
  Anvil's mode resolver delegates to the shared helpers behind a
  `cfg(feature = "lifecycle")`-free path (these are core, not behind
  the lifecycle flag). `crates/anvil-cli/` mode-detection paths
  delegate too.
- **Validation:** `cargo test -p eddacraft-tui --all-features`; `cargo
  test -p eddacraft-tui --no-default-features`; `cargo tree -p
  eddacraft-tui --prefix=none --edges normal` shows zero new
  dependencies vs the TUIR-001 baseline; Anvil mode resolver tests
  still pass.

**changeType:** internal
**releaseIntent:** candidate
**releaseScope:** minor

### TUIN-004: Implement lifecycle helpers behind opt-in feature flag

- **Status:** Ready — **implementation landed via TUIN-012** (D-TUIN-003
  Accepted): `crates/eddacraft-tui/src/lifecycle.rs` ships behind
  `#[cfg(feature = "lifecycle")]` with `TerminalGuard` (raw-mode + alt-screen
  enter/leave, `Drop` cleanup), `restore_terminal`, and an `install_panic_hook`
  panic-restore handler; `Cargo.toml` declares `lifecycle = []` (no transitive
  pulls); `examples/runner_shell.rs` exercises it transitively. **Remaining
  delta:** the dedicated panic-restore integration test
  (`tests/lifecycle_panic.rs`) named in the Validation criterion, and
  (optional) a lifecycle-specific example. Scoped, unblocked.
- **Intent:** Land the `lifecycle` feature flag in `eddacraft-tui` with
  alt-screen enter/exit, raw mode set/clear, panic-restore handler, and
  signal-driven cleanup helpers per D-TUIN-003.
- **Expected Outcome:** New module under
  `crates/eddacraft-tui/src/lifecycle/` exposed behind
  `#[cfg(feature = "lifecycle")]`. The `Cargo.toml` declares
  `lifecycle = []` with no transitive feature pulls. Anvil consumers
  unchanged (they keep their own lifecycle). At least one example under
  `crates/eddacraft-tui/examples/` demonstrates the helper so downstream
  readers see the wiring.
- **Validation:** `cargo test -p eddacraft-tui --features lifecycle`;
  `cargo test -p eddacraft-tui --no-default-features` (must still pass
  — the feature is genuinely opt-in); `cargo test --workspace`;
  panic-restore behaviour covered by a dedicated integration test
  under `crates/eddacraft-tui/tests/lifecycle_panic.rs` (or similar).

**changeType:** internal
**releaseIntent:** candidate
**releaseScope:** minor

### TUIN-005: Ship widget examples; forbid `[[bin]]`

- **Status:** open
- **Intent:** Cover each major widget family with a runnable example per
  D-TUIN-004, deliberately keeping the crate `[[bin]]`-free.
- **Expected Outcome:** `crates/eddacraft-tui/examples/` gains one
  example per widget family (counts seeded by TUIR-001 baseline). Each
  example is self-contained — no `clap`, no hidden runtime config.
  `Cargo.toml` declares no `[[bin]]` target. README links each example.
- **Validation:** `cargo build --examples -p eddacraft-tui` succeeds;
  `grep -F '[[bin]]' crates/eddacraft-tui/Cargo.toml` returns no
  matches; examples list reproduced in the published README (verified
  via `cargo package` extract + diff).

**changeType:** docs
**releaseIntent:** candidate
**releaseScope:** patch

### TUIN-006: Mark widget / theme extension surface stability

- **Status:** open
- **Intent:** Annotate stable vs unstable items on the widget trait
  surface and theme override hook per D-TUIN-005. Establish a docs
  convention so reviewers can spot drift.
- **Expected Outcome:** Public rustdoc items in `eddacraft-tui` carry a
  `# Stability` section: `stable`, `unstable`, or `experimental`.
  Snapshot harness items in `test-utils` are marked `experimental`.
  CHANGELOG notes the stability declaration as a soft commitment (no
  major version bump). `docs/runbooks/eddacraft-tui-release.md` gains a
  "breaking-change checklist" referencing the stability markers. A CI
  grep flags newly added public items lacking a stability marker
  (warn-only initially per Anvil's warnings-over-blocks rule).
- **Validation:** `cargo doc --no-deps -p eddacraft-tui` succeeds with
  stability sections visible in rustdoc output; stability-marker CI
  grep runs as a non-blocking check; CHANGELOG entry merged.

**changeType:** docs
**releaseIntent:** candidate
**releaseScope:** minor

### TUIN-007: Post-migration API stability checkpoint

- **Status:** open
- **Intent:** Capture the post-TUIR API state, downstream feedback from
  three shipped releases, and recommend (or reject) a `0.3.0`
  stabilisation pass per D-TUIN-006.
- **Expected Outcome:** `docs/runbooks/eddacraft-tui-api-checkpoint.md`
  documenting (a) what shipped in `0.2.x` post-TUIR, (b) downstream
  consumer feedback collected via TUIN-002 follow-up touchpoints, (c)
  breaking-change debt accumulated since TUIR-008, (d) recommendation:
  hold at `0.2.x`, stabilise to `0.3.0`, or pre-emptive `1.0`. The
  checkpoint is a decision document only — any recommended action
  becomes its own follow-up module.
- **Validation:** document review with at least one downstream consumer
  touchpoint cited; checkpoint linked from
  `plans/decisions/DECISION-LOG.md` under a "Pending follow-ups"
  section.

**changeType:** docs
**releaseIntent:** never
**releaseScope:** none

### TUIN-008: Retrospective and follow-up backlog

- **Status:** open
- **Intent:** After TUIN-001..TUIN-007 land, capture what worked, what
  didn't, what should be standing guidance, and what the next post-
  migration module ("TUI 3") should inherit.
- **Expected Outcome:** Short retrospective at
  `plans/retrospectives/tui-reintegration-and-next.md` covering both
  TUIR and TUIN, with cross-links to any new feedback memories worth
  extracting. Any items deferred from TUIN land in a clearly-named
  backlog section so they can be picked up without re-discovery.
- **Validation:** retrospective exists; cross-links resolve; deferred
  items have explicit "needs follow-up module" notes.

**changeType:** docs
**releaseIntent:** never
**releaseScope:** none

### TUIN-009: Spike — opportunistic terminal-harmonised colour palette

- **Status:** Done 2026-05-28 (spike; operator-authorised under the TUIN
  gate, no PR — see gate Exception 2026-05-28)

**Intent:** Decide whether to opportunistically port jake-stewart/tuie's
terminal-harmonised colour generation (CIELAB-space 256-colour palette
seeded from the terminal's real fg/bg + ANSI colours) into the
`eddacraft-tui` theme layer, by producing a ship / no-ship signal before
committing implementation effort.

**Outcome:** A throwaway spike crate ported tuie's CIELAB conversion and
palette-generation logic onto a `crossterm` front-end and added a
three-tier degrade (`from_theme_and_terminal`): tier 0 pure brand theme →
tier 1 terminal-true neutrals + brand accents → tier 2 fully
terminal-harmonised. Findings:

- Colour math is correct — 11 unit tests cover Lab round-trip
  (near-lossless), cube corners pinning to fg/bg, a monotonic grey ramp,
  the index-inversion involution, and all three degrade tiers.
- The live terminal colour query (OSC 10 / 11 / 4) round-trip survives
  `crossterm` raw-mode handling.
- tmux is **not** a hard ceiling: wrapping the palette queries in tmux DCS
  passthrough with `allow-passthrough on` returns the full palette
  (18/18) even on a phone terminal over SSH; the initial partial result
  was unwrapped queries being swallowed by tmux, not a terminal limit.
- An `is_terminal()` guard makes the query safe to skip when piped or
  redirected (degrades to the static brand theme).

Decision: **SHIP.** Target shape = port the CIELAB module verbatim +
`from_theme_and_terminal` three-tier seeding + a guarded terminal query
(hand-rolled stdin reader or the `terminal-colorsaurus` crate) that wraps
palette queries in tmux passthrough when `$TMUX` is set. Operational
caveat to document, not engineer around: tmux needs `allow-passthrough
on` (off by default since tmux 3.4); when off, consumers land in tier 1,
which is designed to look intentional. tuie is MIT — attribution carried
per house practice. The implementation is **not** merged into
`crates/eddacraft-tui/` (the prototype lives outside the repo); landing it
is `releaseIntent: candidate` work gated on TUIR-008 and is not part of
this spike's Done state.

**Validation:** spike crate `cargo test` → 11 passing; the demo binary
reports `18/18` in tmux with `allow-passthrough on` and degrades cleanly
to tier 1 / tier 0 otherwise.

**changeType:** spike
**releaseIntent:** never
**releaseScope:** none

### TUIN-010: Triage remaining tuie port candidates

- **Status:** Proposed
- **Intent:** Capture the non-colour tuie features surfaced during the
  TUIN-009 spike session as a backlog item so the assessment isn't lost,
  with a port / reimplement-the-idea / skip decision for each. No
  implementation authority until triaged and the TUIN gate opens.
- **Expected Outcome:** A triage note (this work-item body, promoted to
  its own spec if any candidate advances) recording, per candidate:
  - **Chord ergonomics** (tuie `input/chord.rs` + the `chord!` macro) —
    human-readable chord strings (`<C-a>`) and key+modifier matching.
    Recommendation: port the *idea* — map chord strings onto `crossterm`
    key events feeding the existing keyboard binding table — not tuie's
    proc-macro, which is bound to tuie's own key/trigger/modifier types.
  - **Image stack** (kitty / sixel / half-block + tmux passthrough,
    shared memory transport) — **skip:** `eddacraft-tui` already depends
    on `ratatui-image`, which covers this.
  - **Layout (flex / split / grid), virtualized list, async runtime,
    dirty tracking, GUI / GPU mode, editor (vi / emacs)** — **skip:**
    these are tuie's *architecture* (a retained widget tree), not
    detachable features; lifting them onto Ratatui is a rewrite, and
    Ratatui already provides `List` / `StatefulWidget` plus
    `tui-textarea`.
- **Validation:** triage note lists each candidate with a port/skip
  decision and rationale; any "port" decision spawns its own `Ready`
  work item once the gate opens.

**changeType:** docs
**releaseIntent:** never
**releaseScope:** none

### TUIN-011: Amend ADR-050 for fallback CLI shell consumers

- **Status:** Done 2026-06-08 (docs-only planning amendment; operator-
  requested under the TUIN gate)

**Intent:** Capture the new consumer signal that two imminent
`eddacraft-tui` adopters need CLI tools with subcommands, flags, and
configuration before TUIN-002 / TUIN-004 shape the runner API. Raise the
fallback contract from a single-app launcher to a small fallback CLI
shell without reopening the rejected paths (`clap` in core,
default-on parser behaviour, `[[bin]]` in the crate, or sibling
`eddacraft-tui-cli`).

**Outcome:** ADR-050 amended on 2026-06-08 to pin the runner as an
opt-in fallback CLI shell: global flags (`--help`, `--version`,
`--theme`, `--no-tui`, `--config`), first-level subcommand dispatch,
consumer-owned command payload parsing, consumer-owned config format /
merge semantics, lifecycle, panic restore, theme selection, mode
detection, and render-loop handoff. TUIN's D-TUIN-001 and TUIN-002 text
now survey and implement against `launch_cli<Cli: TerminalCli>` as the
command-shell entry point, with `launch_default<App: TerminalApp>` left
as an optional narrower adapter for single-app consumers.

**Validation:** `pnpm adr:check`, `pnpm docs:check`, `pnpm format:check`,
and targeted Prettier check passed locally on 2026-06-08 after the
ADR/TUIN amendment.

**changeType:** docs
**releaseIntent:** never
**releaseScope:** none

### TUIN-012: Implement runner fallback CLI shell

- **Status:** Done 2026-06-08

- **Intent:** Implement the amended ADR-050 `runner` feature contract once
the TUIN gate opens: a parser-light fallback CLI shell for consumers that
need shared global flags, first-level subcommand dispatch, and config handoff
without adopting a full CLI framework.

- **Expected Outcome:** `crates/eddacraft-tui/` exposes a feature-gated
`runner` module with working-name APIs equivalent to
`launch_cli<Cli: TerminalCli>(cli) -> ExitCode`, `launch_with`,
`RunnerOptions`, `CommandSet`, `ConfigSource`, and a runner-local event
envelope. The `runner` feature enables `lifecycle` transitively and adds
only the accepted minimal parser dependency unless TUIN-004 implementation
evidence proves a swap is necessary. The implementation draws the boundary
where ADR-050 pins it after the TUIN-012 implementation correction: global
flags (`--help`, `--version`, `--theme`, `--no-tui`, `--config`), first-level
command selection, config-path handoff, and typed mode/theme hints are shared;
terminal lifecycle entry, render loop, theme application, nested commands,
completions, env binding, rich validation, config format / merge semantics, and
domain command behaviour remain consumer-owned.

- **Outcome:** `crates/eddacraft-tui/` now exposes feature-gated `lifecycle`
  and `runner` modules. `lifecycle` provides `TerminalGuard` and
  `restore_terminal`; `runner` provides `launch_cli`, `launch_with`,
  `launch_with_args`, `TerminalCli`, `RunnerOptions`, `CommandSet`,
  `ConfigSource`, and `RunnerMode`. The parser uses `lexopt` for shared global
  flags and hands command-specific arguments through raw to the consumer after
  first-level command selection. `runner` enables `lifecycle` transitively for
  consumers that choose to enter TUI mode themselves; no `[[bin]]` target is
  introduced; `examples/runner_shell.rs` demonstrates a subcommand and
  `--config` handoff.

- **Validation:** `cargo test -p eddacraft-tui --features runner`; `cargo
test -p eddacraft-tui --all-features`; `cargo test -p eddacraft-tui
--no-default-features`; `cargo tree -p eddacraft-tui --features runner
--prefix=none --edges normal` reviewed against the TUIR-001 dependency
baseline; at least one example or fixture demonstrates `launch_cli` with
a subcommand and `--config` path handoff.

- **Validation Result (2026-06-08):** `cargo fmt --all -- --check`; `cargo
  test -p eddacraft-tui --features runner`; `cargo test -p eddacraft-tui
  --all-features`; `cargo test -p eddacraft-tui --no-default-features`; `cargo
  clippy -p eddacraft-tui --features runner --all-targets`; `cargo tree -p
  eddacraft-tui --features runner --prefix=none --edges normal`; `node
  scripts/aps/drift-check.mjs`; `pnpm docs:check` all passed. Docs check
  reported only baselined warnings.

**changeType:** internal
**releaseIntent:** candidate
**releaseScope:** minor

### TUIN-013: First-class public docs-site section for `eddacraft-tui`

- **Status:** Proposed
- **Intent:** Give `eddacraft-tui` its own public docs-site section, a sibling
  to `/aps` and `/kindling`, rather than a bullet under `/edda-stack`. The
  OSS crate currently has no narrative front door on the docs-site: its
  canonical reference lives in rustdoc (docs.rs) and the README that travels
  with the read-only mirror (ADR-047), but a discoverable getting-started /
  concepts surface is missing. Surfaced 2026-06-09 alongside the runner
  bring-your-own-parser docs (PR #2462), whose `runner` module narrative
  seeds the section's CLI page.
- **Expected Outcome:** New `docs/public/eddacraft-tui/` content tree following
  the APS/kindling section pattern (Docusaurus frontmatter — `id` / `title` /
  `sidebar_position`; no internal DOCGOV governance table for `docs/public/**`): `overview`,
  `getting-started`, a widgets tour, `theming`, `json-render`, `pretext`, and a
  `cli-runner` page seeded from the `runner` module rustdoc. Site wiring: a new
  plugin instance + navbar entry in `apps/docs-site/docusaurus.config.ts`, a
  `apps/docs-site/sidebars/eddacraft-tui.ts` sidebar, the `vercel.json`
  `ignoreCommand` path-list entry, and a row in `apps/docs-site/AGENTS.md`. API
  reference is **linked to docs.rs, not duplicated** — the section is narrative
  only, to avoid the multi-surface drift the mirror governance warns against
  (canonical reference stays on the crate + docs.rs).
- **Validation:** `pnpm docs:check` passes (new pages carry the metadata
  convention; relative links resolve); `pnpm docs:index` regenerated and
  committed; the docs-site builds with the new section; the section renders at
  `/eddacraft-tui` with a working sidebar. Cross-link from the crate README's
  Documentation section to the new site section resolves.

**changeType:** docs
**releaseIntent:** never
**releaseScope:** none

## Ready Checklist

- [x] TUIR-008 closed: canonical source live in
      `crates/eddacraft-tui/`, mirror healthy, first crates.io publish
      from canonical source verified.
- [x] Drift check (D-TUIR-018) green for at least 7 consecutive runs
      before TUIN promotes any task to Ready. **Met 2026-06-21** — 15+
      consecutive green daily runs (2026-06-10..2026-06-21).
- [ ] Downstream consumer list from TUIR-001 baseline imported into
      TUIN-002 survey scope.
- [x] Next ADR slot confirmed — ADR-050 Accepted; TUIN-001 Merged
      2026-05-23 via PR #1883.
- [x] Proposed feature-flag names checked against existing
      `crates/eddacraft-tui/Cargo.toml` features for collisions — `lifecycle`
      and `runner` landed via TUIN-012 with no collisions.
- [ ] Stability annotation mechanism (rustdoc convention vs stability
      crate) chosen before TUIN-006 starts.
- [ ] No emergency P1 against a TUIN-scoped surface is active in
      issues (open P1s get routed to a TUIR follow-up, not into TUIN
      pre-emptively).
