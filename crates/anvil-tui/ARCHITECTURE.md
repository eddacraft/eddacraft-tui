# anvil TUI architecture

| Type         | Authority | Owner | Status | Freshness                                                                                                                                                     |
| ------------ | --------- | ----- | ------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Architecture | Derived   | TUI   | Live   | Last reviewed 2026-08-20 against `f0f834b39`, `src/surfaces/**`, `src/lib.rs`, `eddacraft-tui::Surface`, tutorial copy tests, snapshots, ADR-115, and ADR-123 |

| Upstream                                                                                | Downstream                                                                        |
| --------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------- |
| `crates/anvil-tui/src/**`, `crates/eddacraft-tui`, kernel event types, ADR-115, ADR-123 | interactive anvil commands through the [CLI runner](../anvil-cli/ARCHITECTURE.md) |

## Scope and boundaries

`anvil-tui` owns anvil-specific surface state and rendering. It does not own
terminal setup, filesystem/process I/O, or event polling; those belong to the
[CLI runner](../anvil-cli/ARCHITECTURE.md). It reuses the stable
[`eddacraft_tui::Surface` contract](../eddacraft-tui/src/surface.rs) and shared
theme, keyboard, shell, animation, widget, and snapshot facilities documented by
[`eddacraft-tui`](../eddacraft-tui/README.md).

The component exports surfaces through
[`src/surfaces/mod.rs`](src/surfaces/mod.rs). Current families are activation,
audit, browser, dashboard, doctor, fix request, gate, init, notifications,
onboarding, plan dashboard, status, tutorial, update hint, watch, welcome, and
wizard. The module list is canonical; release-era surface counts are not an
invariant.

## Surface contract and dispatch

[`src/surface.rs`](src/surface.rs) re-exports the generic shared trait. A
surface provides a name and help text, maps an abstract keyboard `Action`,
reports quit/back navigation, declares whether free-text entry is active, may
reset for re-entry, and renders into a caller-provided frame and rectangle.

That split keeps state deterministic and testable:

```text
CLI command / event source
        |
        v
anvil-cli terminal and event loop
        |
        | Action or typed text; domain snapshot/event
        v
anvil-tui surface state
        |
        | render(frame, area, theme)
        v
eddacraft-tui shell, theme, widgets, and test buffer
```

A nested surface must distinguish `should_back` from `should_quit`; the CLI
runner turns those flags into `SurfaceExit`. A text-entry step must opt into
`text_entry_active` so printable navigation keys are inserted rather than
interpreted. Surface code must not read the terminal, spawn commands, or mutate
the workspace during rendering.

## State and event adapters

Most surfaces receive complete view data from the command layer and mutate only
selection, navigation, or form state. Live families add explicit adapters:

- [`watch/event_adapter.rs`](src/surfaces/watch/event_adapter.rs) consumes
  kernel `EngineEvent` values and updates watch state;
- [`gate/event_adapter.rs`](src/surfaces/gate/event_adapter.rs) converts gate
  progress into renderable state; and
- [`plan_dashboard/event_adapter.rs`](src/surfaces/plan_dashboard/event_adapter.rs)
  owns plan-dashboard event translation.

Adapters are protocol consumers, not alternate business-logic engines. The
kernel, checks, and plan parsers remain authoritative for the underlying result.
A disconnected producer is a runner error or explicit terminal state, never an
implicit successful completion.

Dashboard surfaces render supplied specs or domain snapshots. Dashboard/operator
architecture remains in the central cross-system views; this document owns only
the TUI state/render boundary.

## anvil-specific widgets and shared widgets

[`src/widgets/`](src/widgets) contains anvil-specific composites and
presentation vocabulary. [`src/shell.rs`](src/shell.rs) and shared low-level
primitives are re-exported or composed from `eddacraft-tui`.

Ownership is deliberately split:

- `eddacraft-tui` owns the `Theme` and semantic role contract, keyboard mapping,
  `Surface` extension contract, generic widgets, shell branding primitives,
  animations, terminal lifecycle option, and buffer/snapshot test utilities;
- `anvil-tui` owns anvil surface composition, anvil-specific status and finding
  presentation, navigation state, and the copy rendered by those surfaces; and
- `anvil-cli` owns terminal sessions and external effects.

Consumers must not copy shared widget internals into this crate. ADR-115 governs
changes to the downstream-implemented shared `Surface` trait.

## Tutorial engine

[`src/surfaces/tutorial/`](src/surfaces/tutorial) is an anvil-specific state
machine, not the public tutorial corpus. It owns discovery, path definitions,
execution requests, fix and verify states, watch-demo state, showcase/first-win
steps, rendering, and snapshot/copy tests.

[`paths.rs`](src/surfaces/tutorial/paths.rs) supplies the ProtectionLoop default
and other learning paths. A `TutorialStep` carries description, instruction,
optional command and declared effect, optional verifier, optional watch path,
and optional in-TUI editor seed. Mutating structured operations are implemented
by the command/executor boundary after containment; render code does not execute
shell text.

The default ProtectionLoop path has two load-bearing honesty tests in
[`tutorial/mod.rs`](src/surfaces/tutorial/mod.rs):

- `protection_loop_copy_uses_activation_state_vocabulary` pins the shared
  labels; and
- `protection_loop_copy_does_not_claim_pre_write_protection` prevents the
  tutorial from promoting its own activation state.

Only `anvil start` and `anvil status --verify` produce evidence-backed
activation labels. The tutorial may teach those labels and invoke the read-only
verifier; it must not claim `protecting` from tutorial progress. The autoplay
path runs against an isolated fixture and declares command effects so consent,
containment, and reset behaviour stay observable.

File changes enter through the CLI runner and `handle_file_change`. Reset must
clear path-local state without leaking a prior run's result. Tutorial commands,
verification, and sandbox effects remain outside the render function.

## Snapshots and rendering invariants

Rendering tests use Ratatui `TestBackend` buffers and the shared snapshot
normaliser re-exported by [`src/test_utils.rs`](src/test_utils.rs). Snapshot
files live next to the tested surface hierarchy under `src/snapshots/` or module
snapshot directories.

Snapshots pin observable layout and copy for representative states; they do not
replace state-machine assertions. New or changed state must cover navigation and
failure behaviour directly, with snapshots added where layout/copy is part of
the contract.

Rendering invariants are:

- identical state, area, and theme produce identical output;
- surfaces render only inside the supplied rectangle;
- narrow layouts degrade deliberately rather than panicking;
- help text and keyboard actions agree;
- status and failure language comes from typed domain state rather than parsing
  previously rendered prose; and
- untrusted spec-derived display strings pass through the shared sanitiser
  before rendering.

## Failure invariants

- Rendering and action handling do not perform filesystem, network, or process
  I/O.
- A surface never converts a missing/error domain value into a healthy state.
- Quit and back remain distinct across nested flows.
- Free-text entry cannot trigger navigation actions for printable keys.
- Tutorial copy cannot claim pre-write protection without verifier evidence.
- Shared widget and theme contracts remain owned by `eddacraft-tui`.
- Terminal restoration and channel-disconnect handling remain owned by the CLI
  runner.

## Decisions, retained authorities, and history

- [ADR-115](../../plans/decisions/115-eddacraft-tui-surface-trait-evolution.md)
  governs evolution of the stable shared surface trait.
- [ADR-123](../../plans/decisions/123-documentation-authority-and-diagram-model.md)
  governs this component-local architecture authority.
- [The CLI architecture](../anvil-cli/ARCHITECTURE.md) is authoritative for
  terminal lifecycle, animation polling, and live event loops.
- [`eddacraft-tui` documentation](../eddacraft-tui/README.md) is authoritative
  for shared theme, keyboard, generic widget, shell, lifecycle, and snapshot
  contracts.
- The former TUI, widget-catalogue, and tutorial central as-builts remain at
  their original paths as compatibility and history records. Use
  `git log --follow -- <path>` for their pre-migration detail.
