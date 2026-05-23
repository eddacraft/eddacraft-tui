# ADR-050: `eddacraft-tui` runner helper and CLI / parser policy

## Status

Proposed

## Date

2026-05-23

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
- **Consumers without their own CLI should reach a usable terminal
  experience with ~3 lines of code.** "Library wins, app loses" is
  not acceptable for the *application* layer the way it is for the
  *widget* layer — apps are precisely what `eddacraft-tui` exists
  to enable.
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
    eddacraft_tui::runner::launch_default(MySkillsApp::new())
}
```

The exact API surface lands in TUIN-003 / TUIN-004; the policy this
ADR locks is the contract:

| Decision | Resolution |
| --- | --- |
| **No `clap` (or equivalent heavy parser) in core's default build** | Upheld from TUIR Out-of-Scope. The `runner` feature MAY pull a minimal parser (`lexopt` is the working assumption — zero-dep, ~300 LoC) for default args (`--help`, `--version`, `--theme`, `--no-tui`). |
| **`runner` is opt-in** | Defaulted OFF. Consumers that want only widgets and themes (Anvil today) pay zero cost. |
| **`runner` composes other opt-in features, not replaces them** | `runner` enables `lifecycle` (D-TUIN-003) transitively. Mode-detection (D-TUIN-002, core / unflagged) is available regardless. |
| **`runner` takes a consumer-supplied app, not a pre-baked one** | Public `TerminalApp` (working name) trait the consumer implements. `launch_default<A: TerminalApp>(app: A) -> ExitCode` is the entry point. |
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
  would actually pass into `launch_default` (App shape, default-vs-
  configurable surface, lifecycle ownership preference). The survey
  feeds back into the `TerminalApp` trait shape that TUIN-003 /
  TUIN-004 land.

### Reference runner contract (illustrative — not normative for this ADR)

```rust
// crates/eddacraft-tui/src/runner/mod.rs   (under feature = "runner")

use std::ops::ControlFlow;          // stdlib
use std::process::ExitCode;         // stdlib
use ratatui::Frame;                 // ratatui re-export
use crate::runner::event::Event;    // runner-local event envelope wrapping
                                    // crossterm input + the consumer's
                                    // user-event type (A::Event)

/// A consumer-supplied terminal application.
///
/// Implementers describe their own state, event handling, and render
/// pass. The runner owns: argument parsing for default flags,
/// terminal lifecycle, panic-restore, theme selection, and the
/// render loop.
pub trait TerminalApp {
    /// User-defined event payload (typically `()` for simple apps; a
    /// channel-carried `enum` for apps with background work).
    type Event: Send + 'static;

    /// Render one frame against the supplied Ratatui frame.
    fn render(&self, frame: &mut Frame<'_>);

    /// Handle a single event. Return `ControlFlow::Break(exit_code)`
    /// to exit the run loop with the specified `ExitCode`; return
    /// `ControlFlow::Continue(())` to keep running.
    fn handle_event(
        &mut self,
        event: Event<Self::Event>,
    ) -> ControlFlow<ExitCode>;
}

/// The default runner.
///
/// Parses `--help`, `--version`, `--theme NAME`, `--no-tui` from the
/// process command line; sets up `TerminalGuard` (alt-screen + raw
/// mode + panic restore via the `lifecycle` feature); resolves a
/// `Theme` (default `EddaCraftTheme`); enters the render loop; returns
/// an `ExitCode`. Consumers wanting more control bypass this and call
/// the lower-level helpers directly.
pub fn launch_default<A: TerminalApp>(app: A) -> ExitCode { /* ... */ }

/// As `launch_default`, but accepts pre-parsed args + theme so
/// consumers can integrate with their own parser if they want to.
pub fn launch_with<A: TerminalApp>(
    app: A,
    opts: RunnerOptions,
) -> ExitCode { /* ... */ }
```

`Event<U>` is a runner-local envelope (defined alongside the trait
in `crates/eddacraft-tui/src/runner/event.rs` per TUIN-004) wrapping
`crossterm::event::Event` from the terminal plus a user-defined
`U = A::Event` for app-internal events delivered via a channel. Its
exact shape is a TUIN-003 / TUIN-004 deliverable; this snippet only
fixes the trait surface the consumer sees.

The exact trait shape, the parser choice (`lexopt` working
assumption), and the precise default-flags surface are TUIN-003 /
TUIN-004 deliverables — this ADR only locks the policy and the
opt-in shape.

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
| `lexopt` | **Yes (proposed)** | Zero-dep, ~300 LoC, parser-free style (callbacks). Matches the runner's needs (4-6 default flags). If a consumer needs more, they pass `RunnerOptions` from their own parser. |
| Hand-rolled | No | Cheaper than it sounds for 4-6 flags, but `lexopt` is well-tested and the LoC saving is trivial. |

The parser choice is the only non-policy detail this ADR
provisionally fixes; if TUIN-003 implementation surfaces a reason
to swap (e.g. `lexopt` doesn't handle a needed pattern cleanly),
the swap is a TUIN-003 implementation note rather than a new ADR.

### Alternatives Considered

| Option | Pros | Cons |
| --- | --- | --- |
| **Chosen: opt-in `runner` feature in core, `TerminalApp` trait, no `[[bin]]` in core** | Single-crate consumption surface; opt-in posture preserves widget-only consumers' weight budget; consumer ships their own `[[bin]]` so per-consumer CLI shape stays under consumer control. | Couples runner cadence to widget cadence; `runner` feature surface grows the published `Cargo.toml` even when the feature is off. |
| **Sibling crate `eddacraft-tui-cli`** | Stronger separation of cadences; widget consumers never see the runner surface. | Two-crate consumption; version-skew risk; another publish workflow to maintain (TUIR-005 currently scopes one publish workflow only). |
| **No runner — ship lifecycle + parser helpers individually** | Smallest surface area; maximally composable. | Every consumer rewrites the glue; failure mode is "consumers stop reaching for `eddacraft-tui` for CLI-shaped projects and grow their own incompatible scaffolding". |
| **Ship a `[[bin]]` in core that runs a demo** | Trivially discoverable. | D-TUIN-004 forbids this for good reason: `[[bin]]` pulls the CLI dependency surface into the default build, breaks the library contract. The demo case is well served by `examples/` (D-TUIN-004 / TUIN-005). |
| **Adopt the runner inside Anvil too** | Single CLI scaffold across the ecosystem. | Forces either runner bloat or Anvil regression; rejected above. |

## Consequences

- **Positive:**
  - Library-shaped consumers (`eddacraft-skills`, future Rust ports of
    `anvil-plan-spec` and similar) reach a usable CLI with ~3 lines
    of glue.
  - The "library wins, app loses" trade-off TUIR carries forward
    stops costing consumers their application layer.
  - The runner sets a single, opinionated default for terminal
    lifecycle, panic restore, theme selection, and mode detection
    that downstream consumers don't have to re-derive.
  - Anvil's existing CLI is untouched; widget-only consumers pay zero
    cost; the runner is opt-in throughout.

- **Negative:**
  - `eddacraft-tui` grows a runner cadence on top of the widget
    cadence; release notes must distinguish.
  - The runner is now another public-API surface to keep stable
    (TUIN-006 covers stability annotations).
  - The `runner` feature being off-by-default means consumers must
    discover it (README + crate-level rustdoc); discoverability is
    a docs problem rather than an API problem.

- **Risks:**
  - **`lexopt` choice ages poorly.** Mitigation: parser choice is
    not normative in this ADR — TUIN-003 / TUIN-004 implementation
    can swap without an ADR amendment.
  - **`TerminalApp` trait turns out to be wrong shape after first
    real consumer.** Mitigation: TUIN-002 survey scope extended to
    capture what each known consumer would actually pass in before
    the trait lands. Trait is annotated `# Stability: experimental`
    per D-TUIN-005 / TUIN-006 until at least one external consumer
    ships against it.
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
    `launch_default` expectations.
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
