# Session Context and Research Synthesis

## Purpose

This document preserves the design context that led to the requirements and architecture in this pack. It is intentionally broader than a formal specification so later contributors can understand why particular boundaries exist.

## Initial request

The session began with a request to imagine the ultimate modern Rust-based TUI framework, potentially including a CLI framework if Clap constrained the design.

The instruction was explicitly adventurous:

- do not be constrained by what has always been done;
- determine honestly whether Ratatui is already the best possible foundation;
- imagine a framework that creates genuine satisfaction and wonder;
- consider contemporary editors and terminal products rather than only TUI libraries.

Morgan’s research provided a broad August 2026 survey of Rust and cross-language frameworks.

## Morgan research: key findings

The research identified Ratatui as the safest Rust default because it is:

- actively maintained;
- cross-platform;
- the centre of gravity for the Rust TUI ecosystem;
- supported by the deepest collection of widgets, integrations and examples.

It also identified higher-level alternatives:

- iocraft for React/SwiftUI-like declarative composition;
- Cursive for retained widgets and a managed event loop;
- tui-realm for message/update architecture over Ratatui;
- R3BL for asynchronous reactive applications;
- AppCUI-rs for a desktop-style complete toolkit;
- emerging projects such as Rooibos, Reratui, RxTUI, termuix and frankentui.

The research’s most important conclusion was that the key choice is not merely Rust versus TypeScript. It is:

> rendering library versus application framework.

Ratatui supplies rendering, layout and widgets. The application still owns state, focus, navigation, event routing and asynchronous orchestration.

## Initial architectural conclusion

The first synthesis concluded:

- Ratatui is the best practical foundation for shipping today.
- Ratatui is not the ultimate contemporary TUI framework.
- The missing system is a semantic application runtime whose first renderer is the terminal.

The initial direction introduced several durable ideas that remain the backbone of this specification.

### Semantic application graph

The core primitive should not be a transient widget. It should be a stable entity, action, command, resource, document, task or session.

### Hybrid retained and declarative composition

The runtime retains identity, state, focus, tasks, layout caches and semantic nodes, while components declaratively describe their current projection.

### Terminal cells as an output format

The cell buffer should be the final rasterisation target rather than the application’s source of truth.

### Protocol-native terminal runtime

The framework should negotiate modern keyboard, mouse, colour, synchronised output, hyperlinks, graphics and terminal geometry capabilities.

### Structured concurrency

Tasks should be owned by entities and cancelled or detached according to explicit lifetime policy rather than allowed to mutate dead screens.

### Typed command system above Clap

One semantic command should support CLI parsing, interactive prompting, command palettes, agent tools, structured output, previews, approvals and undo.

### Devtools as a framework feature

Entity trees, task lifetimes, focus, action dispatch, specification patches, layout, damage and emitted terminal output should be inspectable.

### Ratatui as compatibility, not landlord

Existing Ratatui widgets should be embeddable, but Ratatui’s `Widget` contract should not define the semantic core.

## Zed and GPUI as references

Zed and GPUI were considered because they demonstrate what happens when a product owns a coherent application runtime.

Relevant ideas include:

- framework-owned entities with stable identity;
- typed actions rather than raw-key application logic;
- contextual key dispatch;
- retained application state with declarative view construction;
- lower-level rendering escape hatches for complex surfaces;
- asynchronous contexts connected to entity lifetimes;
- deterministic scheduling and testing.

The useful lesson is not GPU rendering itself. It is the coordination of state, actions, focus, async work, rendering and testing.

## Warp and WarpUI as references

Warp’s open-source architecture strengthened the idea that a terminal cell grid should not define the product model.

Important patterns include:

- a shared semantic and entity core across graphical and headless terminal experiences;
- a typed block model for commands, output and agent interactions;
- divergence at input, layout and rendering rather than at domain semantics;
- stable identities for operations that can be presented in different front ends.

Warp’s block model is particularly relevant to developer and agent applications. A command, tool call, result, artefact or approval should be a typed object rather than an undifferentiated stream of characters.

## Existing eddacraft-tui work

The user highlighted `eddacraft/eddacraft-tui`, particularly two re-imaginings:

1. JSON Render for declarative, cross-surface terminal UI specifications.
2. Pretext-inspired streaming text layout.

This materially changed the direction because those capabilities are already early parts of the proposed runtime rather than ordinary widget features.

## JSON Render implications

The existing JSON Render work demonstrates:

- a flat, stable-ID element graph;
- structural validation;
- a catalogue and registry;
- generic and domain-specific component separation;
- data binding;
- responsive breakpoints;
- safe placeholders for missing renderers;
- sanitisation of hostile terminal control sequences;
- resource limits and defensive rendering;
- catalogue parity between web and terminal components.

This is not yet a complete application runtime, but it establishes the basis for:

- portable semantic specifications;
- governed agent-generated interfaces;
- progressive patches;
- shared component meaning across renderers;
- a compilation boundary between untyped JSON and typed runtime nodes.

The key architectural refinement is that JSON should remain a wire, storage and generation format. It should compile into typed runtime structures rather than remain the permanent hot-path representation.

## Pretext implications

The existing Pretext-inspired engine demonstrates:

- measurement separated from layout;
- cached Unicode display widths;
- incremental streamed appends;
- preservation of style runs across token boundaries;
- exclusion zones around moving or embedded content;
- persistent state separate from the transient Ratatui widget;
- streamed agent content and masonry demonstrations.

This suggested a broader principle:

> prepare expensive or trust-sensitive material once, then project it cheaply into the current layout.

Pretext also led to the concept of Flow as a first-class semantic document rather than treating all output as rows in a widget.

## Flow and Scene

The session introduced two fundamental application modes.

### Flow

A sequential, append-friendly, durable document containing:

- prose;
- code;
- diffs;
- commands;
- diagnostics;
- findings;
- evidence;
- approvals;
- progress;
- images and diagrams;
- agent and tool interactions.

Flow maps naturally to CLI output, terminal scrollback, web timelines, conversations and notebooks.

### Scene

A spatial workspace containing:

- editors;
- inspectors;
- navigation trees;
- sidebars;
- overlays;
- image viewers;
- command palettes;
- multi-pane review surfaces.

The distinctive interaction is the ability to promote a Flow node into Scene and collapse it back without recreating or losing the underlying semantic object.

## Colour and media expansion

The user asked for colours and images to be covered well.

The resulting direction was:

- colour is semantic intent, not a literal terminal code;
- themes should be authored in a perceptual colour space such as OKLCH;
- a runtime resolver should adapt to truecolour, ANSI 256, ANSI 16 and monochrome;
- contrast and accessibility should be validated after quantisation;
- colour must never be the only signal;
- images should be semantic media assets with negotiated representations;
- the runtime should support Kitty, iTerm2, Sixel, cell-based and text-only fallbacks;
- image identity should be separate from placement;
- media should participate in Flow and Scene;
- meaningful media should have structured alternatives;
- untrusted specifications must not receive unrestricted file or network access.

## Cross-platform realisation

The user then validated that the system should improve future web and native products.

The conclusion was yes, with an important distinction:

- share semantics, state, commands, actions, media intent, permissions and history;
- do not force exact shared layout, raw input handling or renderer-specific types.

This led to the sibling-project model:

```text
shared semantic core
├── terminal renderer
├── web renderer
├── native renderer
├── headless/plain renderer
└── agent/tool adapter
```

The terminal implementation remains the first exceptional expression and the hardest proving ground, while web or native work can validate that the semantic core is genuinely independent.

## Anvil constraint clarification

The final session clarification was critical:

- Anvil must eventually migrate to the framework.
- The architecture must not be constrained by how Anvil or Ratatui works today.

This produces a deliberate strategy:

1. Design the semantic runtime greenfield.
2. Build a focused reference application that exercises the hardest ideas.
3. Keep Ratatui and existing `eddacraft-tui` as compatibility and acceleration layers.
4. Validate one sibling renderer early.
5. Migrate Anvil incrementally after the runtime boundaries have proven themselves.

## Resolved direction

The session converged on the following statement:

> The project is a renderer-independent semantic application runtime, with an exceptional terminal expression built first. It combines typed commands, stable entities, structured concurrency, Flow and Scene, prepared rendering, governed generative UI, adaptive colour and media, and shared human/agent/accessibility semantics. Anvil is a proving ground and migration target, not the architectural boundary.

## Important unproven hypotheses

The following remain hypotheses requiring implementation spikes:

- A Flow layout engine can efficiently support rich inline nodes, media, selection and virtualisation.
- Promotion between Flow and Scene can preserve identity without confusing application ownership.
- A specification patch protocol can support agent-generated UI safely and incrementally.
- One command model can serve CLI, TUI, web, native and agent surfaces without becoming excessively abstract.
- A shared semantic core can remain renderer-independent while supporting platform-specific excellence.
- Perceptual colour compilation can reliably preserve semantic distinctions across real terminal palettes.
- Media placements can behave correctly across modern protocols, multiplexers, resize and scrollback.
- Ratatui compatibility can remain ergonomic without leaking Ratatui types into the core.

## Reference sources and projects

### Rust and terminal frameworks

- Ratatui
- Crossterm
- Termwiz
- iocraft
- R3BL TUI
- Cursive
- tui-realm
- AppCUI-rs
- Rooibos
- frankentui
- ratatui-image
- TachyonFX

### Cross-language references

- OpenTUI
- Ink
- Textual
- Bubble Tea
- FTXUI
- Brick
- Terminal.Gui
- notcurses

### Product and application-runtime references

- Zed and GPUI
- Warp and WarpUI
- Nushell’s typed command model
- Vercel JSON Render
- Cheng Lou’s Pretext
- Kitty graphics and keyboard protocols

### Existing eddacraft assets

- `eddacraft-tui`
- its JSON Render engine and component registry
- its Pretext-inspired layout engine
- its theme roles and terminal mode detection
- its Ratatui image wrapper
- Anvil’s current TUI surfaces, commands, evidence and dashboard work
