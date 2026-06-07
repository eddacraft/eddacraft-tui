# ADR-050: `eddacraft-tui` runner helper and CLI / parser policy

## Status

Proposed

## Date

2026-05-23

## Amendments

- **2026-06-08:** Near-term downstream demand changed from one named
  library-shaped consumer to two imminent `eddacraft-tui` adopters that both
  need CLI tools with subcommands, flags, and configuration. The runner contract
  is amended from a single-app fallback launcher to a small fallback CLI shell:
  still opt-in, still parser-light, still no `clap` in core, still no `[[bin]]`
  in the crate, but now explicitly capable of global flags, first-level
  subcommand dispatch, and consumer-owned config loading hooks.

## Context

After ADR-047 / TUIR moves `eddacraft-tui` back into Anvil as canonical
source, the next class of design question — deferred out of TUIR's
Out-of-Scope clause and parked under the
[`tui-next` module (TUIN)](../modules/tui-next.aps.md) — is what the
crate offers downstream consumers in terms of CLI surface, argument
parsing, and terminal lifecycle ownership.

Two consumer shapes drive the decision:

1. **Anvil itself** (`crates/anvil-cli/`, `crates/anvil-tui/`) — already
   owns its full CLI: a `clap::Parser` derive tree with dozens of
   subcommands, an `EXIT_*` constants surface (see
   `crates/anvil-cli/src/main.rs:36-87` for the exit-code map and
   forward-position reservations), an Anvil-shaped error vocabulary,
   and a hand-rolled `TerminalGuard` lifecycle wrapper
   (`crates/anvil-cli/src/tui.rs:46-101`). Anvil consumes
   `eddacraft-tui` for widgets and themes only; any default behaviour
   the crate takes on must not collide with what Anvil already owns.

2. **Library-shaped consumers without their own CLI**
   (`eddacraft-skills` today; `anvil-plan-spec` and other future Rust
   ports of currently-TS surfaces) — they want to ship a usable
   terminal experience without re-implementing argument parsing,
   alt-screen lifecycle, panic restore, theme selection, and mode
   detection from scratch. The "library wins, app loses" framing
   (TUIR D-TUIR-014's spirit) leaves these consumers paying the same
   integration tax Anvil pays at the widget layer — but for the
   *application* layer instead of the *library* layer.

   As of the 2026-06-08 amendment, this is no longer hypothetical:
   two projects are about to adopt `eddacraft-tui`, and both need CLI
   tools with subcommands, flags, and configuration. That does not
   justify turning `eddacraft-tui` into a full CLI framework, but it does
   raise the fallback bar above "one TUI app plus four global flags".

TUIN's currently-proposed decisions partly address (2) but stop short
of an explicit turn-key runner:

- **D-TUIN-001** says "no parser in core; helpers behind opt-in feature
  flag OR sibling crate" — a parser-policy decision, framed defensively
  against `clap` weight.
- **D-TUIN-002** keeps mode-detection probes in core, parser-free.
- **D-TUIN-003** puts lifecycle helpers behind an opt-in `lifecycle`
  feature flag, defaulted OFF.
- **D-TUIN-004** forbids a `[[bin]]` in core; widget examples only.

Each piece is correct in isolation but leaves the assembly to every
consumer. A library-shaped consumer must:

- enable two or three feature flags (`lifecycle` + a parser flag if
  one exists);
- discover the right helper APIs from rustdoc and stitch them
  together;
- own its own `[[bin]]` plus a hand-rolled `fn main()` that wires
  parser → mode detection → terminal lifecycle → render loop →
  panic-restore;
- replicate this for every new consumer.

The forces TUIN must balance:

- **No `clap` in core, no default-on parsing, no `[[bin]]` in core.**
  All three are load-bearing TUIR / TUIN commitments. ADR-050 must
  uphold them.
- **Consumers without their own CLI should reach a usable CLI-backed
  terminal experience with ~3 lines of code.** "Library wins, app
  loses" is not acceptable for the *application* layer the way it is
  for the *widget* layer — apps are precisely what `eddacraft-tui`
  exists to enable. The 2026-06-08 consumer signal means "usable"
  includes first-level subcommand dispatch, shared global flags, and
  config-loading hooks, not only a single TUI launch path.
- **Anvil's existing ownership must not regress.** Anvil's
  `clap::Parser` tree, `EXIT_*` constants, and `TerminalGuard` are
  the reference implementation; whatever ships in `eddacraft-tui`
  must coexist without conflict.
- **The runner must be opt-in.** Default-on terminal takeover for
  any consumer that pulls `eddacraft-tui` for widget rendering only
  is the failure mode D-TUIN-003 already guards against; the runner
  inherits that posture.

The decision must be locked before TUIN promotes any item from
Proposed to Ready, because TUIN-001 (the ADR draft), TUIN-002 (the
consumer survey), and TUIN-003 / TUIN-004 (the helper implementations)
all key off the runner contract.

## Decision

`eddacraft-tui` ships a **turn-key runner helper** behind a new opt-in
`runner` feature flag. The runner is the public face of the
post-TUIR CLI policy:

```toml
# downstream consumer Cargo.toml
[dependencies]
eddacraft-tui = { version = "0.x", features = ["runner"] }
```

```rust
// downstream consumer src/main.rs
fn main() -> std::process::ExitCode {
    eddacraft_tui::runner::launch_cli(MyTool::new())
}
```

The runner is a **small fallback CLI shell**, not a full CLI framework.
It owns the common application-layer plumbing that otherwise gets
rewritten by every small consumer: global flag parsing, mode selection,
terminal lifecycle, panic restore, theme selection, config-file handoff,
and first-level subcommand dispatch. It deliberately does **not** own
domain command semantics, nested command trees, shell completions,
environment-variable binding, or rich validation. Consumers that need
those bring their own parser and call lower-level runner / lifecycle
helpers.

The exact API surface lands in TUIN-003 / TUIN-004; the policy this
ADR locks is the contract:

| Decision | Resolution |
| --- | --- |
| **No `clap` (or equivalent heavy parser) in core's default build** | Upheld from TUIR Out-of-Scope. The `runner` feature MAY pull a minimal parser (`lexopt` is the working assumption — zero-dep, ~300 LoC) for global args (`--help`, `--version`, `--theme`, `--no-tui`, `--config`) plus first-level subcommand dispatch. |
| **`runner` is opt-in** | Defaulted OFF. Consumers that want only widgets and themes (Anvil today) pay zero cost. |
| **`runner` composes other opt-in features, not replaces them** | `runner` enables `lifecycle` (D-TUIN-003) transitively. Mode-detection (D-TUIN-002, core / unflagged) is available regardless. |
| **`runner` takes a consumer-supplied app, not a pre-baked one** | Public `TerminalCli` / `TerminalApp` (working names) traits the consumer implements. `launch_cli<C: TerminalCli>(cli: C) -> ExitCode` is the command-shell entry point; single-app consumers may still use a narrower `launch_default<A: TerminalApp>(app: A) -> ExitCode` adapter if TUIN-004 keeps it. |
| **Subcommands are first-level and consumer-owned** | The runner parses the global shell envelope and selects one declared command. The consumer owns the command enum / payload, command-specific parsing, execution, and help copy beyond the shared envelope. Nested subcommands or rich validation are an explicit signal to use a consumer-owned parser. |
| **Config loading is a hook, not a format mandate** | `--config <path>` is a shared global flag, but `eddacraft-tui` does not pick TOML/YAML/JSON, config discovery paths, schema validation, or merge semantics. The consumer receives the optional path and returns its own config type. |
| **No `[[bin]]` in core** | Upheld from D-TUIN-004. The `[[bin]]` lives in each consumer crate; `eddacraft-tui` ships the library helper they call. |
| **Anvil does not adopt the runner** | Anvil keeps its `clap::Parser` tree, `EXIT_*` constants, and `TerminalGuard` (`crates/anvil-cli/src/tui.rs`). The runner is for consumers without an existing CLI, not a migration target for those with one. |
| **`TerminalGuard` migrates to `eddacraft-tui::lifecycle`** | The Anvil-side implementation (`anvil-cli/src/tui.rs:46-101`) is the reference; TUIN-004 moves it to `crates/eddacraft-tui/src/lifecycle/`. Whether Anvil re-exports the moved type or keeps its local one is an Anvil-internal call deferred to TUIN-004 implementation (or a follow-up ADR if the decision needs to be load-bearing); the consumer survey in TUIN-002 covers external consumers, not Anvil's own posture. |
| **A sibling `eddacraft-tui-cli` crate is NOT created** | D-TUIN-001 left both "opt-in feature flag" and "sibling crate" on the table. ADR-050 picks the feature-flag path: lower friction for consumers, no extra publish/release surface, and `runner`-feature consumers naturally opt out of the parser weight if they don't enable it. A future split into a sibling crate would require its own ADR. |

### Renames in TUIN

This ADR also fixes a TUIN scope-framing issue surfaced during draft:

- **D-TUIN-001's framing is amended** — the opt-in is the *whole runner*,
  not just "a helper that needs argument parsing". The current text
  treats parser-policy as the centrepiece; ADR-050 reframes it as a
  subset of the runner contract.
- **TUIN-001 outcome filename** updates from `050-eddacraft-tui-cli-policy.md`
  to this file (`050-eddacraft-tui-runner-and-cli-policy.md`).
- **TUIN-002 outcome scope** extends to surveying what each consumer
  would actually pass into `launch_cli` / `launch_default` (command
  shape, config shape, App shape, default-vs-configurable surface,
  lifecycle ownership preference). The survey feeds back into the
  `TerminalCli` / `TerminalApp` trait shape that TUIN-003 / TUIN-004
  land.

### Reference runner contract (illustrative — not normative for this ADR)

```rust
// crates/eddacraft-tui/src/runner/mod.rs   (under feature = "runner")

use std::ops::ControlFlow;          // stdlib
use std::process::ExitCode;         // stdlib
use ratatui::Frame;                 // ratatui re-export
use crate::runner::event::Event;    // runner-local event envelope wrapping
                                    // crossterm input + the consumer's
                                    // user-event type (C::Event)

/// A consumer-supplied CLI-backed terminal application.
///
/// Implementers describe their command set, config loading, state,
/// event handling, and render pass. The runner owns: global flag
/// parsing, first-level command dispatch, terminal lifecycle,
/// panic-restore, theme selection, and the render loop.
pub trait TerminalCli {
    /// Consumer-defined command payload. A simple app can use `()` or a
    /// small enum; richer apps can parse command-specific args inside
    /// `parse_command`.
    type Command;

    /// Consumer-defined config shape. The runner only passes through an
    /// optional `--config` path; it does not mandate a file format.
    type Config;

    /// User-defined event payload (typically `()` for simple apps; a
    /// channel-carried `enum` for apps with background work).
    type Event: Send + 'static;

    /// Describe the shared command names for help and dispatch.
    fn commands(&self) -> CommandSet;

    /// Parse a selected first-level command after global flags have
    /// been consumed. Consumers own command-specific flags and payloads.
    fn parse_command(
        &self,
        command: &str,
        args: &[std::ffi::OsString],
    ) -> Result<Self::Command, RunnerError>;

    /// Load consumer config from the optional global `--config` path.
    fn load_config(
        &self,
        source: ConfigSource<'_>,
    ) -> Result<Self::Config, RunnerError>;

    /// Render one frame against the supplied Ratatui frame.
    fn render(&self, frame: &mut Frame<'_>);

    /// Handle a single event. Return `ControlFlow::Break(exit_code)`
    /// to exit the run loop with the specified `ExitCode`; return
    /// `ControlFlow::Continue(())` to keep running.
    fn handle_event(
        &mut self,
        event: Event<Self::Event>,
    ) -> ControlFlow<ExitCode>;

    /// Run the selected command. Command handlers can choose to enter
    /// the TUI render loop, perform a non-interactive action, or return
    /// an immediate exit code.
    fn run_command(
        &mut self,
        command: Self::Command,
        config: Self::Config,
    ) -> ControlFlow<ExitCode>;
}

/// The default fallback CLI shell.
///
/// Parses shared global flags (`--help`, `--version`, `--theme NAME`,
/// `--no-tui`, `--config PATH`) and a first-level subcommand from the
/// process command line; asks the consumer to parse the selected
/// command payload and load config; sets up `TerminalGuard`
/// (alt-screen + raw mode + panic restore via the `lifecycle` feature)
/// when TUI mode is selected; resolves a `Theme` (default
/// `EddaCraftTheme`); runs the selected command and, if requested,
/// enters the render loop. Consumers wanting more control bypass this
/// and call lower-level helpers directly.
pub fn launch_cli<C: TerminalCli>(cli: C) -> ExitCode { /* ... */ }

/// As `launch_cli`, but accepts pre-parsed global options and command
/// payload so consumers can integrate with their own parser if they
/// want to.
pub fn launch_with<C: TerminalCli>(
    cli: C,
    opts: RunnerOptions,
) -> ExitCode { /* ... */ }
```

`Event<U>` is a runner-local envelope (defined alongside the trait
in `crates/eddacraft-tui/src/runner/event.rs` per TUIN-004) wrapping
`crossterm::event::Event` from the terminal plus a user-defined
`U = C::Event` for app-internal events delivered via a channel. Its
exact shape is a TUIN-003 / TUIN-004 deliverable; this snippet only
fixes the trait surface the consumer sees.

The exact trait shape, the parser choice (`lexopt` working
assumption), and the precise global-flag / subcommand surface are
TUIN-003 / TUIN-004 deliverables — this ADR only locks the policy and
the opt-in shape.

## Rationale

### Why a turn-key runner at all?

The bare TUIN-001..-004 surface lands the *primitives* (mode
detection, lifecycle, examples), but each consumer must still write
the same ~50 lines of glue: parse args, install panic hook, enter
alt-screen, build terminal, instantiate theme, run loop, restore.
For Anvil that's fine — Anvil has the glue and a strong reason to
own it. For library-shaped consumers (`eddacraft-skills`,
`anvil-plan-spec` future Rust port, any consumer-without-CLI), the
glue is undifferentiated infrastructure that gets re-implemented
slightly wrong each time. A turn-key runner makes the obvious thing
trivial without preventing the bespoke thing.

The 2026-06-08 consumer signal strengthens that rationale: two
imminent consumers need CLI tools as well as TUI surfaces. If the
fallback stops at "single render loop plus `--theme`", both consumers
still duplicate the same command envelope, config handoff, and mode
selection. The runner should absorb that shared shell, while leaving
each consumer's command semantics in the consumer crate.

The "library wins, app loses" framing TUIR uses to justify zero
in-core Anvil dependencies is fully consistent here: the runner is
a library helper that an *app crate* invokes. The crate stays a
library; the app stays an app.

### Why a feature flag rather than a sibling crate?

D-TUIN-001 left both paths open. The sibling crate
(`eddacraft-tui-cli`) has lower coupling at the manifest level but
higher friction for consumers — they would depend on two crates,
track two version bumps, and reason about feature compatibility
between them. A feature flag inside `eddacraft-tui` keeps the
consumption surface single-crate while preserving the opt-in
posture. If the runner ever needs to ship at a different cadence
from the widget surface (or to depend on something the widget
surface cannot pull), splitting to a sibling crate is a future ADR
— not a precondition.

### Why not adopt the runner inside Anvil too?

Anvil's CLI is dozens of subcommands deep with a stable exit-code
contract, `clap`-derived shape, and Anvil-specific error vocabulary
that the runner's minimal-parser surface would not preserve. Forcing
a `runner`-shaped migration on Anvil would either trivialise the
runner (to accommodate all of Anvil's needs it stops being minimal)
or break Anvil's CLI shape (it grows incompatible). The runner is
deliberately scoped to consumers who *don't* have an existing CLI;
Anvil's existence as a counter-example is the constraint that keeps
the runner minimal.

### Why `lexopt` (working assumption) over `clap`, `argh`, or hand-rolled?

| Parser | Pulled by `runner` flag? | Why / why not |
| --- | --- | --- |
| `clap` (derive) | No | ~80kB compile-time, large transitive surface, exactly what TUIR's Out-of-Scope clause forbids. Even gated, importing `clap` into the published crate metadata is visible weight. |
| `clap` (builder, minimal) | No | Same transitive footprint as derive in practice. |
| `argh` | Maybe | Smaller than `clap` but still pulls `proc-macro2`/`syn` via derive. |
| `lexopt` | **Yes (proposed)** | Zero-dep, ~300 LoC, parser-free style (callbacks). Matches the runner's needs for global flags plus first-level subcommand dispatch. If a consumer needs nested command trees, completions, env binding, or rich validation, they pass `RunnerOptions` from their own parser. |
| Hand-rolled | No | Cheaper than it sounds for 4-6 flags, but `lexopt` is well-tested and the LoC saving is trivial. |

The parser choice is the only non-policy detail this ADR
provisionally fixes; if TUIN-003 implementation surfaces a reason
to swap (e.g. `lexopt` doesn't handle a needed pattern cleanly),
the swap is a TUIN-003 implementation note rather than a new ADR.

### Alternatives Considered

| Option | Pros | Cons |
| --- | --- | --- |
| **Chosen: opt-in `runner` feature in core, small fallback CLI shell, no `[[bin]]` in core** | Single-crate consumption surface; opt-in posture preserves widget-only consumers' weight budget; global flags, first-level subcommand dispatch, config handoff, lifecycle, and render-loop plumbing stop being duplicated across near-term consumers; consumer ships its own `[[bin]]` so domain CLI shape stays under consumer control. | Couples runner cadence to widget cadence; `runner` feature surface grows the published `Cargo.toml` even when the feature is off; first-level dispatch may be too small for some consumers. |
| **Sibling crate `eddacraft-tui-cli`** | Stronger separation of cadences; widget consumers never see the runner surface. | Two-crate consumption; version-skew risk; another publish workflow to maintain (TUIR-005 currently scopes one publish workflow only). |
| **No runner — ship lifecycle + parser helpers individually** | Smallest surface area; maximally composable. | Every consumer rewrites the glue; failure mode is "consumers stop reaching for `eddacraft-tui` for CLI-shaped projects and grow their own incompatible scaffolding". |
| **Ship a `[[bin]]` in core that runs a demo** | Trivially discoverable. | D-TUIN-004 forbids this for good reason: `[[bin]]` pulls the CLI dependency surface into the default build, breaks the library contract. The demo case is well served by `examples/` (D-TUIN-004 / TUIN-005). |
| **Adopt the runner inside Anvil too** | Single CLI scaffold across the ecosystem. | Forces either runner bloat or Anvil regression; rejected above. |

## Consequences

- **Positive:**
  - Library-shaped consumers (`eddacraft-skills`, future Rust ports of
    `anvil-plan-spec` and similar) reach a usable CLI shell with ~3
    lines of glue.
  - The "library wins, app loses" trade-off TUIR carries forward
    stops costing consumers their application layer.
  - The runner sets a single, opinionated default for terminal
    lifecycle, panic restore, theme selection, mode detection, global
    flags, first-level command dispatch, and config handoff that
    downstream consumers don't have to re-derive.
  - Anvil's existing CLI is untouched; widget-only consumers pay zero
    cost; the runner is opt-in throughout.

- **Negative:**
  - `eddacraft-tui` grows a runner cadence on top of the widget
    cadence; release notes must distinguish.
  - The runner is now another public-API surface to keep stable
    (TUIN-006 covers stability annotations).
  - The fallback shell may become attractive enough that consumers ask
    it to grow into a full CLI framework; TUIN-004 must keep the
    boundary explicit.
  - The `runner` feature being off-by-default means consumers must
    discover it (README + crate-level rustdoc); discoverability is
    a docs problem rather than an API problem.

- **Risks:**
  - **`lexopt` choice ages poorly.** Mitigation: parser choice is
    not normative in this ADR — TUIN-003 / TUIN-004 implementation
    can swap without an ADR amendment.
  - **`TerminalApp` / `TerminalCli` trait turns out to be wrong shape
    after first real consumer.** Mitigation: TUIN-002 survey scope
    extended to capture what each known consumer would actually pass in
    before the trait lands, including command and config shapes. Traits
    are annotated `# Stability: experimental` per D-TUIN-005 / TUIN-006
    until at least one external consumer ships against them.
  - **Fallback shell grows into a full CLI framework by accretion.**
    Mitigation: ADR-050 draws the boundary at global flags,
    first-level subcommand dispatch, config handoff, lifecycle, and
    render loop. Nested command trees, completions, env binding, rich
    validation, and domain command semantics stay consumer-owned unless
    a future ADR explicitly changes that boundary.
  - **Anvil reviewers reach for the runner anyway during refactors.**
    Mitigation: this ADR explicitly names Anvil as a non-adopter;
    `crates/anvil-cli/src/tui.rs` keeps its own `TerminalGuard`
    until a future ADR explicitly authorises a migration.
  - **Feature-flag matrix grows.** Mitigation: `runner` transitively
    enables `lifecycle` so consumers don't compose flags manually.
    Per-feature test gates already exist (D-TUIR-007).

- **Mitigations:** captured above per risk.

## References

- Related ADRs: ADR-047 (TUIR canonical-source-mirror, parent of TUIN)
- APS modules:
  [`tui-next` (TUIN)](../modules/tui-next.aps.md) — items
  amended by this ADR:
  - D-TUIN-001 (parser/CLI policy) — reframed: runner is the
    centrepiece, parser policy is a subset.
  - D-TUIN-003 (lifecycle ownership) — `runner` feature
    transitively enables `lifecycle`.
  - TUIN-001 — outcome filename updated to this ADR's path.
  - TUIN-002 — survey scope extended to capture per-consumer
    `launch_cli` / `launch_default` expectations, including command
    and config shapes.
  - TUIN-003 / TUIN-004 — implementation deliverables for the
    contract this ADR locks.
- Reference implementation:
  - `crates/anvil-cli/src/tui.rs:46-101` — `TerminalGuard` +
    `install_panic_hook` (TUIN-004 will lift to
    `crates/eddacraft-tui/src/lifecycle/`).
  - `crates/anvil-cli/src/main.rs:36-87` — `EXIT_*` constants the
    runner's minimal `ExitCode` surface should align with where it
    overlaps (`EXIT_OK`, `EXIT_ERROR`).
- Out-of-scope clauses:
  [TUIR Out of Scope](../modules/tui-reintegration.aps.md#out-of-scope)
  — "Adding `clap` (or any argument-parser) as a dependency of
  `eddacraft-tui` core" — upheld and carried forward by this ADR.
