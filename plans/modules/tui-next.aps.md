<!-- APS: See https://github.com/eddacraft/anvil-plan-spec for format reference -->
<!-- Executable only if tasks exist and status is Ready. -->

# TUI Next

| ID   | Owner      | Status   | Progress |
| ---- | ---------- | -------- | -------- |
| TUIN | joshuaboys | Proposed | 1/8      |

**Last reviewed:** 2026-05-23 (ADR-050 design pass)

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

## Out of Scope

- Anything that belongs in TUIR (canonical-source migration mechanics,
  mirror automation, release plumbing, CI gate split). If a TUIN
  decision exposes a TUIR gap, file the gap against TUIR; do not
  expand TUIR scope from inside TUIN.
- Anvil-internal TUI surface redesign (`crates/anvil-tui/`,
  `crates/anvil-cli/`). TUIN owns the shared crate's contract, not
  its consumers' surfaces.
- New widgets, themes, or accessibility features unless they
  necessarily fall out of a TUIN decision (e.g. lifecycle ownership
  forcing an API addition).
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
- External downstream consumers (`eddacraft-skills` and any others
  enumerated in the TUIR-001 baseline) — surveyed during TUIN-002.

**Exposes:**

- A set of accepted ADRs covering parser/CLI policy, lifecycle
  ownership, and extension-surface stability. Next free ADR slot at
  drafting time is `050`; numbers are allocated when each ADR PR
  opens.
- New `eddacraft-tui` feature flags if any decision lands an opt-in
  helper (`lifecycle` proposed by D-TUIN-003).
- Optional sibling crate(s) if a decision rejects shipping a surface
  inside core (e.g. `crates/eddacraft-tui-cli/` if D-TUIN-001 rules
  out an in-core parser).
- API-stability checkpoint document
  (`docs/runbooks/eddacraft-tui-api-checkpoint.md`, new).

## Decisions

**D-TUIN-001:** Turn-key runner + argument-parser policy

- **Proposed resolution:** `eddacraft-tui` core has zero
  argument-parser dependency. The crate ships an opt-in `runner`
  feature flag (defaulted OFF) that bundles a turn-key
  `launch_default<App: TerminalApp>(app) -> ExitCode` entry point —
  consumer crates without their own CLI add 3 lines to `main.rs` and
  inherit argument parsing for default flags (`--help`, `--version`,
  `--theme`, `--no-tui`), terminal lifecycle (alt-screen + raw mode +
  panic restore), theme selection, mode detection, and the render
  loop. The runner's minimal parser is provisionally `lexopt` (zero
  transitive deps, ~300 LoC); TUIN-003 / TUIN-004 may swap without
  re-ADR. The sibling-crate alternative (`eddacraft-tui-cli`) is
  rejected in favour of the feature-flag path for lower consumer
  friction; future split would require its own ADR. Default-on
  argument parsing and a default `[[bin]]` in core both remain
  rejected — the runner is opt-in via the feature flag, and the
  `[[bin]]` lives in each consumer crate (D-TUIN-004 unchanged). The
  TUIR Out-of-Scope clause forbidding `clap` in core is upheld and
  carried forward.
- **Status:** Proposed (ratified by ADR-050 — see
  [`050-eddacraft-tui-runner-and-cli-policy.md`](../decisions/050-eddacraft-tui-runner-and-cli-policy.md)).

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

- **Proposed resolution:** Lifecycle helpers (alt-screen enter/exit,
  raw mode set/clear, panic-restore handler, signal-driven cleanup)
  live in `eddacraft-tui` behind an opt-in `lifecycle` feature flag,
  which is *also* enabled transitively by the `runner` flag from
  D-TUIN-001 — runner consumers do not compose the two flags
  manually. The Anvil-side reference implementation
  (`crates/anvil-cli/src/tui.rs:46-101` — `TerminalGuard`,
  `install_panic_hook`, `restore_terminal`) drives the API shape
  TUIN-004 lifts into core. ADR-050 records that Anvil itself does
  not adopt the lifted helpers via the runner — it keeps its local
  `TerminalGuard` until a separate ADR authorises a migration.
  Defaults are conservative: the feature is OFF for any consumer
  that pulls `eddacraft-tui` for widget rendering only (apps that
  own their lifecycle, like Anvil, ignore it; apps that want
  managed lifecycle opt in directly or via `runner`).
- **Status:** Proposed (sanity-check during TUIN-002 survey;
  composition with `runner` ratified by ADR-050).

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

**Intent:** Before D-TUIN-003 / D-TUIN-001 land code, capture what
each known downstream consumer (`eddacraft-skills` plus any
documented external consumers from TUIR-001 baseline) expects from
terminal lifecycle ownership, CLI helpers, AND the turn-key
`runner` contract ADR-050 locks — do they own lifecycle, want it
managed, or wrap it; do they bring their own parser, want one for
free, or have no CLI today; what shape would their `App: TerminalApp`
implementation take.

**Outcome:**
`plans/specs/2026-XX-XX-eddacraft-tui-post-migration-survey.md`
with one section per consumer documenting:
- current behaviour (does it have a CLI / TUI / either / neither);
- desired behaviour under TUIN — adopt runner / adopt lifecycle
  helpers only / stay fully bespoke;
- the rough shape of what the consumer would pass into
  `launch_default(app)` — what `App: TerminalApp` impl carries
  (event type, render frame call, exit conditions);
- which default flags (`--help`, `--version`, `--theme`, `--no-tui`)
  are sufficient vs need extension via `RunnerOptions`;
- any pain points or constraints (Windows-only widgets, custom
  panic-restore needs, alt-screen incompatibility).

The survey explicitly covers **library-shaped consumers without a
CLI today** (`eddacraft-skills`, future Rust ports of
`anvil-plan-spec` and other current TS-only packages) as the
primary `runner` target, and Anvil-shaped consumers
(`crates/anvil-tui/`, `crates/anvil-cli/`) as the
already-has-its-own-CLI counter-example that the runner must
*not* try to absorb (per ADR-050's Anvil non-adoption note).
Survey output drives the `TerminalApp` trait shape that TUIN-003 /
TUIN-004 implement; gaps flagged for follow-up.

**Validation:** survey doc lists each known consumer from the
TUIR-001 baseline plus any new ones identified via the runner
framing; gaps explicitly flagged for follow-up; ADR-050 referenced
as the locking decision the survey informs.

**changeType:** docs
**releaseIntent:** never
**releaseScope:** none

### TUIN-003: Implement CLI mode-detection helpers in core

- **Status:** open

**Intent:** Land TTY / alt-screen capability / colour-depth / terminfo
probes in `eddacraft-tui` core per D-TUIN-002, with zero new
dependencies and typed-enum return values.

**Outcome:** New module under `crates/eddacraft-tui/src/mode/` with
typed enums (`TtyKind`, `AltScreenSupport`, `ColourDepth`, etc.).
Anvil's mode resolver delegates to the shared helpers behind a
`cfg(feature = "lifecycle")`-free path (these are core, not behind
the lifecycle flag). `crates/anvil-cli/` mode-detection paths
delegate too.

**Validation:** `cargo test -p eddacraft-tui --all-features`; `cargo
test -p eddacraft-tui --no-default-features`; `cargo tree -p
eddacraft-tui --prefix=none --edges normal` shows zero new
dependencies vs the TUIR-001 baseline; Anvil mode resolver tests
still pass.

**changeType:** internal
**releaseIntent:** candidate
**releaseScope:** minor

### TUIN-004: Implement lifecycle helpers behind opt-in feature flag

- **Status:** open

**Intent:** Land the `lifecycle` feature flag in `eddacraft-tui` with
alt-screen enter/exit, raw mode set/clear, panic-restore handler, and
signal-driven cleanup helpers per D-TUIN-003.

**Outcome:** New module under `crates/eddacraft-tui/src/lifecycle/`
exposed behind `#[cfg(feature = "lifecycle")]`. The `Cargo.toml`
declares `lifecycle = []` with no transitive feature pulls. Anvil
consumers unchanged (they keep their own lifecycle). At least one
example under `crates/eddacraft-tui/examples/` demonstrates the
helper so downstream readers see the wiring.

**Validation:** `cargo test -p eddacraft-tui --features lifecycle`;
`cargo test -p eddacraft-tui --no-default-features` (must still pass
— the feature is genuinely opt-in); `cargo test --workspace`;
panic-restore behaviour covered by a dedicated integration test
under `crates/eddacraft-tui/tests/lifecycle_panic.rs` (or similar).

**changeType:** internal
**releaseIntent:** candidate
**releaseScope:** minor

### TUIN-005: Ship widget examples; forbid `[[bin]]`

- **Status:** open

**Intent:** Cover each major widget family with a runnable example per
D-TUIN-004, deliberately keeping the crate `[[bin]]`-free.

**Outcome:** `crates/eddacraft-tui/examples/` gains one example per
widget family (counts seeded by TUIR-001 baseline). Each example is
self-contained — no `clap`, no hidden runtime config. `Cargo.toml`
declares no `[[bin]]` target. README links each example.

**Validation:** `cargo build --examples -p eddacraft-tui` succeeds;
`grep -F '[[bin]]' crates/eddacraft-tui/Cargo.toml` returns no
matches; examples list reproduced in the published README (verified
via `cargo package` extract + diff).

**changeType:** docs
**releaseIntent:** candidate
**releaseScope:** patch

### TUIN-006: Mark widget / theme extension surface stability

- **Status:** open

**Intent:** Annotate stable vs unstable items on the widget trait
surface and theme override hook per D-TUIN-005. Establish a docs
convention so reviewers can spot drift.

**Outcome:** Public rustdoc items in `eddacraft-tui` carry a
`# Stability` section: `stable`, `unstable`, or `experimental`.
Snapshot harness items in `test-utils` are marked `experimental`.
CHANGELOG notes the stability declaration as a soft commitment (no
major version bump). `docs/runbooks/eddacraft-tui-release.md` gains a
"breaking-change checklist" referencing the stability markers. A CI
grep flags newly added public items lacking a stability marker
(warn-only initially per Anvil's warnings-over-blocks rule).

**Validation:** `cargo doc --no-deps -p eddacraft-tui` succeeds with
stability sections visible in rustdoc output; stability-marker CI
grep runs as a non-blocking check; CHANGELOG entry merged.

**changeType:** docs
**releaseIntent:** candidate
**releaseScope:** minor

### TUIN-007: Post-migration API stability checkpoint

- **Status:** open

**Intent:** Capture the post-TUIR API state, downstream feedback from
three shipped releases, and recommend (or reject) a `0.3.0`
stabilisation pass per D-TUIN-006.

**Outcome:** `docs/runbooks/eddacraft-tui-api-checkpoint.md`
documenting (a) what shipped in `0.2.x` post-TUIR, (b) downstream
consumer feedback collected via TUIN-002 follow-up touchpoints, (c)
breaking-change debt accumulated since TUIR-008, (d) recommendation:
hold at `0.2.x`, stabilise to `0.3.0`, or pre-emptive `1.0`. The
checkpoint is a decision document only — any recommended action
becomes its own follow-up module.

**Validation:** document review with at least one downstream consumer
touchpoint cited; checkpoint linked from `plans/decisions/DECISION-LOG.md`
under a "Pending follow-ups" section.

**changeType:** docs
**releaseIntent:** never
**releaseScope:** none

### TUIN-008: Retrospective and follow-up backlog

- **Status:** open

**Intent:** After TUIN-001..TUIN-007 land, capture what worked, what
didn't, what should be standing guidance, and what the next post-
migration module ("TUI 3") should inherit.

**Outcome:** Short retrospective at
`plans/retrospectives/tui-reintegration-and-next.md` covering both
TUIR and TUIN, with cross-links to any new feedback memories worth
extracting. Any items deferred from TUIN land in a clearly-named
backlog section so they can be picked up without re-discovery.

**Validation:** retrospective exists; cross-links resolve; deferred
items have explicit "needs follow-up module" notes.

**changeType:** docs
**releaseIntent:** never
**releaseScope:** none

## Ready Checklist

- [ ] TUIR-008 closed: canonical source live in
      `crates/eddacraft-tui/`, mirror healthy, first crates.io publish
      from canonical source verified.
- [ ] Drift check (D-TUIR-018) green for at least 7 consecutive runs
      before TUIN promotes any task to Ready.
- [ ] Downstream consumer list from TUIR-001 baseline imported into
      TUIN-002 survey scope.
- [ ] Next ADR slot confirmed (slot `050` at drafting; verify in
      `DECISION-LOG.md` before opening TUIN-001).
- [ ] Proposed feature-flag names (`lifecycle`, plus any names
      introduced by ADR-050) checked against existing
      `crates/eddacraft-tui/Cargo.toml` features for collisions.
- [ ] Stability annotation mechanism (rustdoc convention vs stability
      crate) chosen before TUIN-006 starts.
- [ ] No emergency P1 against a TUIN-scoped surface is active in
      issues (open P1s get routed to a TUIR follow-up, not into TUIN
      pre-emptively).
