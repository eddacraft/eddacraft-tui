# Ultimate Rust UI Runtime — Combined Specification

> Combined convenience copy. The numbered source documents remain the authoritative modular pack.


---

<!-- Source: 00-index.md -->

# Ultimate Rust UI Runtime — Documentation Pack

**Status:** Directional requirements and architecture specification  
**Date:** 3 August 2026  
**Scope:** CLI, terminal UI, shared semantic application runtime, and future web/native renderers

## Purpose

This pack documents the proposed category-defining Rust application framework discussed in the accompanying design session.

The proposal begins with the practical reality that Ratatui is currently the safest and most capable Rust terminal rendering foundation, but rejects the idea that a cell renderer, widget library, or command-line parser should define the architecture of a modern application.

The intended result is a **semantic application runtime** whose first exceptional expression is an advanced CLI/TUI experience, while the same underlying commands, entities, state, actions, documents, media, permissions and session history can support sibling web and native renderers.

## Core position

> Build the application model above the renderer. Treat terminal cells, the DOM and a native GPU scene as projection targets rather than sources of truth.

This means:

- Ratatui may be used as the initial terminal rendering substrate and compatibility layer.
- Clap may remain the default shell-argument adapter.
- Neither Ratatui nor Clap may define the core application model.
- Anvil is a high-value proving ground and eventual migration target.
- Existing Anvil architecture must not constrain the greenfield design.
- The framework must prove itself independently before Anvil migration becomes a design driver.

## Documents

| File | Purpose |
|---|---|
| [`01-vision-and-category-thesis.md`](01-vision-and-category-thesis.md) | North-star vision, category thesis, design principles and desired outcome. |
| [`02-session-context-and-research-synthesis.md`](02-session-context-and-research-synthesis.md) | Session history, research synthesis, current ecosystem assessment and reasoning behind the direction. |
| [`03-product-requirements.md`](03-product-requirements.md) | Comprehensive functional, non-functional, security, accessibility and cross-platform requirements. |
| [`04-technical-architecture-specification.md`](04-technical-architecture-specification.md) | Proposed layers, runtime model, rendering pipeline, terminal capabilities, Flow/Scene architecture and extension contracts. |
| [`05-experience-and-design-specification.md`](05-experience-and-design-specification.md) | Behavioural and interaction specification for an interface that feels calm, capable, coherent and genuinely delightful. |
| [`06-cross-platform-and-sibling-project-specification.md`](06-cross-platform-and-sibling-project-specification.md) | How the same semantic runtime can improve terminal, web and native products without forcing identical layouts. |
| [`07-delivery-roadmap-and-anvil-migration-strategy.md`](07-delivery-roadmap-and-anvil-migration-strategy.md) | Greenfield delivery plan, validation spikes, project boundaries and gradual Anvil migration strategy. |
| [`08-decisions-risks-and-open-questions.md`](08-decisions-risks-and-open-questions.md) | Confirmed decisions, hypotheses, unresolved questions, risks and proposed experiments. |
| [`09-reference-api-and-data-models.md`](09-reference-api-and-data-models.md) | Illustrative Rust APIs, wire formats, runtime objects, renderer contracts and command examples. |

## Recommended reading order

1. Read the vision and category thesis.
2. Read the experience specification to understand the intended product feel.
3. Read the product requirements.
4. Read the technical architecture.
5. Read the cross-platform and sibling-project specification.
6. Use the roadmap and decisions documents to plan implementation.
7. Use the API sketches as a design aid, not as frozen public API.

## Terminology

### Semantic runtime

The renderer-independent system that owns application identity, commands, actions, state, resources, tasks, permissions, session history, semantic UI nodes and renderer-neutral intent.

### Flow

A durable, sequential, append-friendly semantic document containing prose, code, command runs, findings, evidence, approvals, images, diagrams, progress and other blocks. Flow maps naturally to CLI output, terminal scrollback, activity streams and notebooks.

### Scene

A spatial workspace containing editors, inspectors, trees, panels, overlays, command palettes, diagrams, image viewers and other persistent interactive regions.

### Promotion

Moving or projecting a semantic node from Flow into Scene for deeper interaction without changing its identity or losing state.

### Collapse

Returning a Scene projection to its durable Flow representation while preserving history, state and relationships.

### Prepared graph

A validated, typed, sanitised and dependency-aware runtime representation compiled from Rust code, JSON specifications, agent output, plugins or stored artefacts.

### Renderer

A platform-specific adapter that turns semantic runtime state into terminal cells, DOM/CSS, a native scene graph, plain text, JSONL or another output format.

## Design status

The documents distinguish between:

- **Requirement:** expected behaviour the system must eventually support.
- **Foundation release:** the minimum coherent runtime needed to prove the architecture.
- **North-star capability:** an advanced outcome that may follow after the core has been validated.
- **Hypothesis:** an architectural idea requiring a focused spike or benchmark before commitment.

## Source context

The pack draws on:

- Morgan’s 2 August 2026 research catalogue of Rust and cross-language TUI frameworks.
- Ratatui, iocraft, R3BL, AppCUI, OpenTUI, Textual and related ecosystem research.
- Architectural patterns from Zed/GPUI and Warp/WarpUI.
- Existing `eddacraft-tui` work, especially its JSON Render interpretation and Pretext-inspired streaming text engine.
- The design discussion covering CLI/TUI convergence, governed generative UI, Flow and Scene, colour, images, accessibility, agents, web/native sibling renderers and Anvil migration.

## Change policy

This pack is a directional specification rather than an immutable contract. Major changes should be recorded as explicit decisions, particularly changes to:

- renderer independence;
- semantic identity;
- command and action ownership;
- Flow and Scene boundaries;
- specification and patch protocols;
- colour and media intent;
- security and trust boundaries;
- compatibility with Ratatui and existing `eddacraft-tui` consumers;
- cross-platform dependency direction.

---

<!-- Source: 01-vision-and-category-thesis.md -->

# Vision and Category Thesis

## Executive thesis

Ratatui is the strongest practical Rust TUI rendering foundation available today. It is not the ultimate contemporary application framework.

The missing product is not a larger widget catalogue or a React clone for terminal cells. It is a **semantic Rust application runtime** whose applications can be expressed through:

- a concise CLI;
- inline interactive terminal output;
- a full-screen terminal workspace;
- a headless or machine-readable mode;
- a web application;
- a native application;
- an agent or tool protocol.

The same application should preserve command semantics, stable identity, state, resources, actions, permissions, evidence, history and accessibility across these surfaces. Each renderer should remain free to create the best experience for its platform.

> A command is not separate from a TUI. A TUI is a live, inspectable, composable projection of commands, documents and application state.

## Why this category should exist now

Modern terminal applications increasingly need to support:

- long-running asynchronous work;
- agent-generated and streamed content;
- structured evidence and artefacts;
- approval and policy gates;
- rich code, diff and document interaction;
- images and diagrams;
- SSH and constrained environments;
- plain-text and machine-readable automation;
- future team and browser experiences.

The dominant Rust TUI model remains a rendering loop over a cell buffer. This is understandable and efficient, but leaves each application to rebuild:

- state ownership;
- focus and navigation;
- contextual actions;
- asynchronous lifetimes;
- terminal capability negotiation;
- command integration;
- persistence and replay;
- accessibility semantics;
- developer tooling;
- generative UI controls;
- renderer portability.

The opportunity is to own these concerns coherently.

## The verdict on current foundations

### Ratatui

Ratatui should be treated as:

- the current safest terminal rendering substrate;
- an important ecosystem compatibility target;
- a low-level escape hatch;
- a practical route to production during framework development.

Ratatui should not be treated as:

- the semantic application model;
- the owner of entity identity;
- the command runtime;
- the persistent document model;
- the cross-platform abstraction;
- the final limit of layout or media capability.

The terminal cell grid is the final rasterisation target, not the source of truth.

### Clap

Clap remains excellent at parsing command-line arguments. It should remain a supported and likely default adapter.

Clap should not own the application’s command model. A modern command contains more than arguments:

- typed inputs and provenance;
- preconditions and permissions;
- risk classification;
- preview and plan behaviour;
- execution and cancellation;
- typed event streams;
- artefacts and diagnostics;
- approval requirements;
- compensation or undo;
- schema and documentation.

From one semantic command definition, the system should be able to produce a Clap parser, shell completion, help, an inline prompt, a full-screen form, a command-palette entry, an agent tool schema, JSON/JSONL output and a remote invocation contract.

## Category definition

The proposed category is a:

> **Semantic application runtime with terminal, web and native expressions.**

Its defining capabilities are:

1. A stable semantic application graph.
2. Typed commands and contextual actions.
3. A Flow document model for durable sequential work.
4. A Scene model for spatial interaction.
5. Promotion and collapse between Flow and Scene.
6. Structured concurrency and owned asynchronous lifetimes.
7. A prepared and incrementally invalidated rendering graph.
8. Governed, catalogue-based generative UI.
9. Protocol-native terminal capability negotiation.
10. Perceptual colour and adaptive media intent.
11. Shared accessibility and agent semantics.
12. Deterministic testing, replay and deep developer tooling.
13. Multiple renderers without lowest-common-denominator design.

## Design centre

The design centre is **not Anvil as currently implemented** and **not Ratatui as currently designed**.

The design centre is a future application that must be able to:

- begin as an ordinary command;
- stream structured progress into terminal scrollback;
- expand into a rich workspace only when useful;
- present code, diffs, evidence, diagrams and approvals;
- preserve state through resize, suspend, reconnect and renderer changes;
- expose the same semantic operations to humans and agents;
- produce a browser or native experience from the same application model;
- remain safe, accessible and useful without colour, images or interactivity.

Anvil is exceptionally valuable as:

- a demanding reference workload;
- a source of real commands, evidence, policy and approvals;
- a migration target;
- a proving ground for developer and agent workflows.

It must not become a hidden constraint that reduces the framework to a refactor of Anvil’s existing TUI.

## Core principles

### 1. Semantics before cells

Applications describe entities, documents, actions, commands, media and meaning. Renderers decide how those appear.

### 2. Stable identity is non-negotiable

Nodes must preserve identity through updates, streaming, movement between Flow and Scene, renderer changes and session restoration.

### 3. Prepare once; project many times

Parsing, validation, sanitisation, schema compilation, resource dependency discovery and expensive measurement should occur before hot-path layout and painting.

### 4. Flow and Scene are equally first-class

The framework must support sequential durable work and spatial application work without forcing one to impersonate the other.

### 5. A simple operation stays simple

The framework must not force every command into a dashboard. Complexity should appear progressively as the work requires it.

### 6. The runtime owns lifecycle

State, tasks, focus, commands, resources, session history and invalidation must be coordinated by one runtime rather than stitched together through application conventions.

### 7. Richness is negotiated, not assumed

Colour depth, theme appearance, image protocols, keyboard protocols, mouse support, cell size and other capabilities must be detected and adapted.

### 8. Meaning survives degradation

No-colour, text-only, narrow, non-interactive and unsupported-capability modes must remain understandable and operable.

### 9. Humans and agents use the same semantic surface

Agents should invoke typed actions, not scrape screen positions. Accessibility tools should consume the same roles, labels, state and action model.

### 10. Generated interfaces are governed

An agent, plugin or server may compose approved catalogue capabilities, but cannot invent arbitrary executable authority.

### 11. Platform-specific excellence is allowed

The terminal, browser and native applications share semantics, not exact geometry or identical components.

### 12. The framework must be observable

The entity graph, action stream, task tree, focus route, specification patches, layout invalidations and rendered output must be inspectable and replayable.

## Desired emotional outcome

The framework should make terminal applications feel:

- immediate without being frantic;
- rich without being noisy;
- structured without being rigid;
- powerful without being obscure;
- adaptive without moving unexpectedly;
- visually expressive without depending on decoration;
- safe without feeling obstructive;
- modern without pretending the terminal is a browser.

Users should experience a deep sense that the interface understands the work and reshapes itself around that work.

## What makes it genuinely wonderful

The feeling should come from behaviour rather than effects:

- Nothing flickers.
- Focus does not jump when asynchronous data arrives.
- Selection and scroll anchors remain stable.
- Commands and shortcuts are discoverable.
- Output is structured data rather than dead coloured text.
- Long-running work can be detached and resumed.
- A live block can become a panel without losing identity.
- Exiting a workspace leaves useful scrollback and a durable record.
- Images can upgrade from a placeholder without causing layout jumps.
- Colour adapts to the terminal while preserving meaning.
- Errors remain visible, actionable and explainable.
- The same run can be understood by a person, an accessibility tool and an agent.

## North-star experience

```text
$ anvil assess .

Assessing repository…
Policy set       engineering-standard
Files discovered 1,284
Checks planned   17
```

A live semantic run block appears in ordinary scrollback. The user may continue watching it inline or promote it into a workspace.

In the workspace:

- findings appear without moving the current selection;
- streamed explanation flows beside live progress and media;
- a finding can open evidence, code and a proposed diff;
- a typed remediation action can require preview and approval;
- the same command continues running under structured concurrency;
- the user may collapse the workspace back into scrollback;
- the session leaves a stable run identifier and resume command.

The same run can later appear in a browser or native client with the same identity, permissions, evidence and action history.

## Non-goals

The project is not intended to be:

- a clone of React for terminal cells;
- a browser DOM implemented in Rust;
- a collection of decorative Ratatui widgets;
- a forced write-once-render-identically-everywhere system;
- a JSON-only application runtime;
- an Anvil-specific UI framework;
- a replacement for every command-line parser;
- a requirement that all applications use pixel graphics or animation;
- a plugin model that permits untrusted code to bypass runtime policy.

## Strategic outcome

If successful, the project provides three forms of leverage:

1. **Product leverage:** Anvil and other eddacraft products gain a more coherent local, terminal, web and future native experience.
2. **Platform leverage:** new applications can reuse commands, actions, Flow, Scene, media, colour, testing and runtime infrastructure.
3. **Category leverage:** Rust gains an application framework that treats the terminal as a first-class modern interface rather than a legacy display target.

---

<!-- Source: 02-session-context-and-research-synthesis.md -->

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

---

<!-- Source: 03-product-requirements.md -->

# Product Requirements

## 1. Purpose

This document defines the comprehensive product requirements for the proposed semantic Rust application runtime and its terminal-first implementation.

The requirements describe the intended system rather than merely the first release. Each requirement carries:

- **Priority:** Must, Should or Could.
- **Horizon:** Foundation, Expansion or North Star.

### Priority definitions

- **Must:** required to validate the architecture or preserve a core design promise.
- **Should:** strongly expected but may follow after the first coherent runtime.
- **Could:** valuable extension that must remain architecturally possible.

### Horizon definitions

- **Foundation:** required for the first credible end-to-end reference application.
- **Expansion:** required before positioning the system as a broadly usable framework.
- **North Star:** advanced capability that defines the ultimate direction.

## 2. System identity and boundaries

| ID | Requirement | Priority | Horizon | Acceptance |
|---|---|---:|---:|---|
| SYS-001 | The core runtime must be renderer-independent. | Must | Foundation | Core crates compile without Ratatui, Crossterm, browser DOM or native-GPU dependencies. |
| SYS-002 | The terminal implementation must be the first exceptional renderer, not the semantic core. | Must | Foundation | Terminal-specific types do not appear in public core entity, action, command or document contracts. |
| SYS-003 | Anvil must be treated as a proving workload and migration target rather than the design boundary. | Must | Foundation | A reference application can exercise the runtime without importing Anvil domain crates. |
| SYS-004 | Ratatui must remain usable as an implementation substrate and compatibility target. | Must | Foundation | Existing Ratatui widgets can be embedded through an explicit adapter without core dependencies on Ratatui. |
| SYS-005 | Clap must remain a supported command-line parser adapter. | Must | Foundation | A semantic command can generate or connect to a Clap parser without Clap owning runtime execution. |
| SYS-006 | The architecture must support terminal, headless, web and native renderer siblings. | Must | Foundation | Renderer contracts contain no terminal-only assumptions and at least one non-terminal proof is planned. |
| SYS-007 | Platform-specific renderers must be allowed to deliver native-quality experiences rather than identical layouts. | Must | Foundation | Shared conformance tests validate semantics and actions, not exact geometry across renderers. |
| SYS-008 | Public APIs must be versioned independently from wire-format versions. | Should | Expansion | Runtime crates and specification protocols can evolve on separate compatibility schedules. |

## 3. Semantic application graph

| ID | Requirement | Priority | Horizon | Acceptance |
|---|---|---:|---:|---|
| GRAPH-001 | Every durable application object must have stable identity. | Must | Foundation | Updates, movement and renderer changes preserve a stable `EntityId` or domain identifier. |
| GRAPH-002 | The runtime must own entity lifecycle. | Must | Foundation | Creation, disposal, task ownership and subscription cleanup occur through runtime APIs. |
| GRAPH-003 | Entities must expose typed state and mutations. | Must | Foundation | Normal application logic does not depend on stringly typed state bags. |
| GRAPH-004 | Views must be declarative projections of current entity state. | Must | Foundation | Components can regenerate projections while retained identity remains outside transient view objects. |
| GRAPH-005 | Runtime dependencies must be observable. | Should | Expansion | Devtools can show which resources, signals or entities invalidated a view. |
| GRAPH-006 | The runtime must support renderer-neutral semantic roles, labels, values, state and relationships. | Must | Foundation | Accessibility and agent adapters can inspect the graph without scraping rendered output. |
| GRAPH-007 | The runtime must support serialisable references to durable entities. | Should | Expansion | Sessions can persist and restore selected nodes, open documents and promoted regions. |
| GRAPH-008 | Entity mutation must be transactionally observable. | Should | Expansion | Action logs and replay can associate state changes with their initiating action or command. |
| GRAPH-009 | The runtime should support derived/computed state with dependency tracking. | Should | Expansion | Derived values recalculate only when their dependencies change. |
| GRAPH-010 | The runtime must not tie component state to hook call order. | Must | Foundation | State ownership uses explicit entities/resources rather than positional hook identity. |

## 4. Command and CLI model

| ID | Requirement | Priority | Horizon | Acceptance |
|---|---|---:|---:|---|
| CMD-001 | Commands must be typed semantic operations independent of CLI syntax. | Must | Foundation | A command type can be invoked from CLI, UI action and tests. |
| CMD-002 | A command must define typed inputs and validation. | Must | Foundation | Invalid input is rejected before execution with structured diagnostics. |
| CMD-003 | Input provenance must be retained. | Should | Expansion | The runtime can distinguish CLI argument, environment, config, prompt, pipeline and default sources. |
| CMD-004 | Commands must declare risk and required permissions. | Must | Foundation | The runtime can require preview or approval for risky operations. |
| CMD-005 | Commands must emit typed events rather than only writing text. | Must | Foundation | Progress, logs, diagnostics, prompts, diffs, artefacts, approvals and completion are distinct events. |
| CMD-006 | Commands must support cancellation. | Must | Foundation | A user or owning entity can request cancellation and observe final cancellation state. |
| CMD-007 | Commands should support retry policy. | Should | Expansion | Retry can be automatic or user-triggered and remains visible in history. |
| CMD-008 | Commands should support compensation or undo metadata. | Should | Expansion | Reversible commands expose a typed undo/compensation action. |
| CMD-009 | Commands must support non-interactive execution. | Must | Foundation | CI and pipes can run commands without terminal prompts. |
| CMD-010 | Commands must support stable plain-text output. | Must | Foundation | Human-readable output remains useful when no TUI is available. |
| CMD-011 | Commands must support structured machine output. | Must | Foundation | JSON or JSONL schemas are versioned and do not contain terminal control sequences. |
| CMD-012 | One command definition should drive help, completion, prompts and agent schemas. | Should | Expansion | Generated surfaces remain traceable to one semantic source. |
| CMD-013 | The runtime must support command detachment and reattachment. | Should | Expansion | Long-running command sessions can survive UI navigation and later be reopened. |
| CMD-014 | Command runs must have stable identifiers. | Must | Foundation | Logs, artefacts, evidence and resume operations reference a durable run ID. |
| CMD-015 | Command execution must be observable without relying on stdout capture. | Must | Foundation | The runtime can inspect command events and state directly. |
| CMD-016 | Shell parsing must remain adapter-specific. | Must | Foundation | Shell flags and syntax are not embedded in core command types. |
| CMD-017 | The framework should provide a lightweight fallback CLI envelope. | Could | Expansion | Simple applications can launch without committing to a parser, while serious apps can bring their own. |

## 5. Typed actions and input

| ID | Requirement | Priority | Horizon | Acceptance |
|---|---|---:|---:|---|
| ACT-001 | User interactions must resolve to typed actions before application mutation. | Must | Foundation | Application handlers do not switch directly on raw key codes for domain operations. |
| ACT-002 | Actions must be contextual. | Must | Foundation | The same key may resolve differently based on focus scope and active mode. |
| ACT-003 | Action availability must be discoverable. | Must | Foundation | Command palette, help, agent and accessibility surfaces can enumerate valid actions. |
| ACT-004 | Keybindings must be configurable independently of actions. | Should | Expansion | Users can rebind actions without changing application code. |
| ACT-005 | The input system must distinguish text input from command key input. | Must | Foundation | Printable characters are not intercepted as navigation inside text-entry contexts. |
| ACT-006 | Actions must carry permission and risk metadata where relevant. | Must | Foundation | A button, keybinding and agent invocation receive the same approval policy. |
| ACT-007 | Actions must be serialisable when used in replay or remote invocation. | Should | Expansion | Recorded interactions can be deterministically replayed. |
| ACT-008 | Mouse and pointer input must resolve to semantic actions. | Should | Expansion | Domain logic does not depend on raw cell coordinates after hit testing. |
| ACT-009 | Agents must invoke actions by identity and schema rather than simulated keystrokes. | Must | Expansion | An agent can select, open, approve or reject through typed calls. |
| ACT-010 | Action dispatch must expose capture, target and bubbling or an equivalent contextual model. | Should | Expansion | Nested components can override or delegate actions predictably. |

## 6. Flow document model

| ID | Requirement | Priority | Horizon | Acceptance |
|---|---|---:|---:|---|
| FLOW-001 | Flow must be a first-class semantic document, not formatted output text. | Must | Foundation | A Flow contains typed nodes with identity, metadata and actions. |
| FLOW-002 | Flow must support append-friendly streaming. | Must | Foundation | New content can arrive without rebuilding the entire document. |
| FLOW-003 | Flow must support prose, headings, code, diffs, logs, diagnostics, commands, findings, evidence, approvals, progress and media. | Must | Expansion | Each content type can preserve semantics and renderer-specific representations. |
| FLOW-004 | Flow nodes must be collapsible and expandable. | Must | Foundation | A node retains identity and internal state across collapse. |
| FLOW-005 | Flow must support selection and copy across mixed content. | Should | Expansion | Users can select text and semantic blocks without losing ordering. |
| FLOW-006 | Flow must retain source-to-rendered-position mapping. | Should | Expansion | Search, diagnostics and accessibility can map rendered content back to semantic source. |
| FLOW-007 | Flow must support stable scroll anchoring during streaming and insertions. | Must | Foundation | Content arriving above or inside the viewport does not unexpectedly move the user. |
| FLOW-008 | Flow must support virtualised layout for very large documents. | Must | Expansion | Memory and layout cost remain bounded for long sessions and logs. |
| FLOW-009 | Flow nodes must support deep links. | Should | Expansion | A finding, command event or image can be reopened by stable reference. |
| FLOW-010 | Flow must have a durable plain-text representation. | Must | Foundation | Exiting interactive mode can leave useful scrollback or export a transcript. |
| FLOW-011 | Flow should support annotations and relationships between nodes. | Should | Expansion | Evidence can reference findings, commands and artefacts. |
| FLOW-012 | Flow must support embedded live components with stable reserved geometry. | Must | Expansion | A progress, diagram or image can resolve without causing an uncontrolled layout jump. |
| FLOW-013 | Flow should support multiple available regions per row. | Should | North Star | Text and inline content can flow around embedded media or controls on both sides where appropriate. |
| FLOW-014 | Flow must adapt semantically to narrow renderers. | Must | Expansion | Side-by-side content can stack without losing meaning or actions. |

## 7. Scene workspace model

| ID | Requirement | Priority | Horizon | Acceptance |
|---|---|---:|---:|---|
| SCENE-001 | Scene must be a first-class spatial workspace. | Must | Foundation | Panels, editors, inspectors and overlays have stable identity and layout state. |
| SCENE-002 | Scene must support split regions and nested layouts. | Must | Foundation | A reference application can open code, evidence and findings simultaneously. |
| SCENE-003 | Scene must support overlays, popovers, modals and command palettes. | Must | Foundation | Layering and focus trapping are runtime-managed. |
| SCENE-004 | Scene must support virtualised lists, tables and trees. | Must | Expansion | Large datasets do not require rendering every row. |
| SCENE-005 | Scene must preserve focus and selection through data refresh. | Must | Foundation | Updates do not reset the active item when its identity remains present. |
| SCENE-006 | Scene layout must adapt to size and capability changes. | Must | Foundation | Narrow layouts reorganise semantically rather than merely clipping. |
| SCENE-007 | Scene must support promotion of Flow nodes. | Must | Foundation | A Flow node can become a panel without changing its underlying ID. |
| SCENE-008 | Scene must support collapse back to Flow. | Must | Foundation | Closing a promoted panel preserves relevant state and history. |
| SCENE-009 | Scene state should be serialisable. | Should | Expansion | Open panels, splits and selections can be restored across sessions. |
| SCENE-010 | Platform renderers may provide richer Scene affordances. | Must | Foundation | Web/native tabs, drag-and-drop or windows do not need terminal equivalents. |

## 8. Promotion and continuity

| ID | Requirement | Priority | Horizon | Acceptance |
|---|---|---:|---:|---|
| CONT-001 | Promotion must preserve semantic identity. | Must | Foundation | Flow and Scene projections refer to the same entity. |
| CONT-002 | Promotion must preserve operation and task ownership. | Must | Foundation | A running task does not restart because its projection changed. |
| CONT-003 | Promotion should preserve local view state where meaningful. | Should | Expansion | Selection, expansion, zoom and scroll can follow the promoted node. |
| CONT-004 | Promotion and collapse must be recorded as actions. | Should | Expansion | Replay can reproduce the workspace transition. |
| CONT-005 | Renderer changes must not alter command or permission semantics. | Must | Expansion | Opening the same run on web does not bypass terminal approval rules. |
| CONT-006 | The runtime should support simultaneous projections. | Could | North Star | A node can remain summarised in Flow while detailed in Scene. |

## 9. Structured concurrency and resources

| ID | Requirement | Priority | Horizon | Acceptance |
|---|---|---:|---:|---|
| ASYNC-001 | Tasks must have explicit owners and lifetimes. | Must | Foundation | Entity-scoped tasks cancel when the entity is disposed unless detached by policy. |
| ASYNC-002 | Cancellation must propagate through task trees. | Must | Foundation | Cancelling a run cancels owned child work consistently. |
| ASYNC-003 | Streams must support backpressure. | Must | Expansion | Fast producers cannot exhaust memory when renderers are slow. |
| ASYNC-004 | Resources must model loading, ready, stale, refreshing, failed and cancelled states. | Must | Foundation | Components do not invent incompatible loading-state conventions. |
| ASYNC-005 | The runtime must support foreground and background executors. | Should | Expansion | Blocking or CPU-heavy work cannot stall input and painting. |
| ASYNC-006 | The core should remain executor-neutral. | Should | Foundation | Tokio integration is first-class but not structurally mandatory in core types. |
| ASYNC-007 | A deterministic scheduler mode must exist for tests. | Must | Expansion | Seeded task interleavings can reproduce timing-sensitive failures. |
| ASYNC-008 | A virtual clock must exist for tests and replay. | Should | Expansion | Timers and animations can be advanced deterministically. |
| ASYNC-009 | Suspended or disconnected renderers must not lose runtime work. | Should | Expansion | Long-running operations continue according to explicit session policy. |
| ASYNC-010 | Resource dependencies must drive targeted invalidation. | Should | Expansion | A changed resource invalidates only dependent prepared nodes. |

## 10. Specification and governed generative UI

| ID | Requirement | Priority | Horizon | Acceptance |
|---|---|---:|---:|---|
| SPEC-001 | The system must support a portable semantic specification format. | Must | Foundation | A saved or generated spec can be compiled by terminal and sibling renderers. |
| SPEC-002 | Specifications must use stable node IDs. | Must | Foundation | Incremental patches can target nodes without replacing the whole tree. |
| SPEC-003 | Specifications must validate structure before activation. | Must | Foundation | Missing roots, cycles, dangling references and unknown capabilities produce diagnostics. |
| SPEC-004 | Arbitrary specification props must compile into typed prepared component data. | Must | Expansion | Hot-path rendering does not repeatedly interpret generic JSON. |
| SPEC-005 | The catalogue must define component schemas. | Must | Expansion | Invalid props are rejected or degraded with structured diagnostics. |
| SPEC-006 | The catalogue must define allowed actions and resources. | Must | Expansion | Generated UI cannot grant itself undeclared authority. |
| SPEC-007 | The catalogue must define renderer fallbacks. | Must | Expansion | Unsupported components have deliberate alternate representations. |
| SPEC-008 | Specifications must support data binding without embedding unrestricted code. | Must | Foundation | Data references resolve through host-approved contexts. |
| SPEC-009 | Specifications must support incremental transactional patches. | Must | Expansion | Add, remove, move and prop updates apply atomically against a revision. |
| SPEC-010 | Patch sources must be attributable. | Must | Expansion | Agent, server, plugin and local-author patches retain provenance. |
| SPEC-011 | Patch application must enforce trust and permission policy. | Must | Expansion | Untrusted sources cannot introduce unauthorised actions or media access. |
| SPEC-012 | Failed patches must not corrupt the active graph. | Must | Expansion | Invalid transactions are rejected atomically with diagnostics. |
| SPEC-013 | Specifications should support conditional visibility and variants. | Should | Expansion | Conditions are evaluated through a constrained expression model. |
| SPEC-014 | Specifications should support streaming construction. | Should | Expansion | A partially generated interface can become useful before completion. |
| SPEC-015 | Specifications must retain a plain semantic alternative. | Must | Expansion | Headless and accessibility renderers can express the same content. |
| SPEC-016 | Rust-authored components and generated specs must converge on the same prepared graph. | Must | Foundation | JSON is one input format, not a separate runtime. |

## 11. Text and Pretext-derived Flow layout

| ID | Requirement | Priority | Horizon | Acceptance |
|---|---|---:|---:|---|
| TEXT-001 | Expensive text measurement must be separated from layout. | Must | Foundation | Unchanged text is not remeasured on every frame. |
| TEXT-002 | Streaming appends must measure only new or changed fragments where possible. | Must | Foundation | Existing prepared content remains reusable. |
| TEXT-003 | Unicode grapheme and display-width handling must be correct. | Must | Foundation | Cursoring, selection and layout handle wide and combining characters. |
| TEXT-004 | Style and semantic spans must survive chunk boundaries. | Must | Foundation | Mid-word streamed styling is not lost. |
| TEXT-005 | The layout engine must support rich inline fragments. | Must | Expansion | Text, links, chips, actions and embedded nodes can share a Flow line. |
| TEXT-006 | The layout engine must support exclusion or reserved regions. | Must | Expansion | Text can flow around media or live components on capable layouts. |
| TEXT-007 | Layout must expose stable cursors or ranges. | Must | Expansion | Virtualisation and source mapping do not require materialising the full document. |
| TEXT-008 | Selection must be semantic and renderer-independent. | Should | Expansion | A copied range can produce plain, Markdown or structured representations. |
| TEXT-009 | Code, diff and log content must have specialised prepared models. | Should | Expansion | Large code or logs do not pass through a generic prose wrapper. |
| TEXT-010 | Text layout must remain calm during streaming. | Must | Foundation | Existing visible content does not visibly jitter without a meaningful structural cause. |

## 12. Layout, composition and responsiveness

| ID | Requirement | Priority | Horizon | Acceptance |
|---|---|---:|---:|---|
| LAYOUT-001 | The runtime must support flex and grid-style composition. | Must | Foundation | Common responsive layouts require no manual rectangle arithmetic. |
| LAYOUT-002 | The layout engine must support intrinsic sizing and min/max constraints. | Must | Foundation | Content can request natural size while respecting available space. |
| LAYOUT-003 | Layout must support named regions and portals/layers. | Must | Expansion | Overlays and promoted content can target stable regions. |
| LAYOUT-004 | Layout invalidation must be incremental. | Must | Expansion | Only dirty branches are recomputed when practical. |
| LAYOUT-005 | Painting must track damage regions. | Should | Expansion | The terminal renderer can minimise cell updates and native renderers can minimise redraw. |
| LAYOUT-006 | Responsiveness must be component- or container-relative. | Should | Expansion | Components adapt to their available region, not only global window width. |
| LAYOUT-007 | Responsive adaptation must preserve semantic priority. | Must | Expansion | Less important detail collapses before critical identity, severity or actions. |
| LAYOUT-008 | The system must support zero-work idle. | Must | Foundation | No frame loop runs continuously when state and animations are idle. |
| LAYOUT-009 | Animations must request frames explicitly. | Must | Foundation | The scheduler renders only while a transition or live effect is active. |
| LAYOUT-010 | Reduced-motion preference must be supported. | Must | Expansion | Motion can degrade to immediate or discrete transitions. |

## 13. Terminal runtime and modes

| ID | Requirement | Priority | Horizon | Acceptance |
|---|---|---:|---:|---|
| TERM-001 | The runtime must support plain/headless mode. | Must | Foundation | Commands work in CI, pipes and redirected output. |
| TERM-002 | The runtime must support inline interactive mode. | Must | Foundation | Interactive components can coexist with shell scrollback. |
| TERM-003 | The runtime must support full-screen workspace mode. | Must | Foundation | Alternate-screen Scene experiences are available where supported. |
| TERM-004 | The runtime should support remote/SSH-oriented mode. | Should | Expansion | Capability and latency policies adapt to remote sessions. |
| TERM-005 | A session must be able to transition between inline and workspace modes. | Must | Foundation | State and running commands are preserved across the transition. |
| TERM-006 | Terminal capability negotiation must be first-class. | Must | Foundation | A typed profile records keyboard, colour, graphics, mouse and geometry capabilities. |
| TERM-007 | Modern keyboard protocols should be supported where available. | Should | Expansion | Press, repeat and release events can be distinguished without breaking fallback terminals. |
| TERM-008 | Synchronised output should be used where supported. | Should | Expansion | Partial-frame display and flicker are reduced. |
| TERM-009 | Hyperlinks should be supported with safe fallbacks. | Should | Expansion | Semantic links become OSC 8 links only where appropriate. |
| TERM-010 | Terminal restoration must be guaranteed on normal exit and best-effort on panic. | Must | Foundation | Raw mode, cursor and alternate screen are restored. |
| TERM-011 | Terminal quirks and compatibility profiles must be inspectable. | Should | Expansion | Users can understand why a capability was enabled or disabled. |
| TERM-012 | Terminal brand checks must not be scattered through application code. | Must | Foundation | Applications query typed capabilities rather than environment strings. |

## 14. Colour and themes

| ID | Requirement | Priority | Horizon | Acceptance |
|---|---|---:|---:|---|
| CLR-001 | Applications must request semantic colour intent rather than literal terminal colours. | Must | Foundation | Components use tokens such as status, surface, text and diff roles. |
| CLR-002 | Theme definitions should use a perceptual colour model. | Should | Expansion | OKLCH or equivalent values can generate coherent light/dark variants. |
| CLR-003 | The runtime must resolve themes for truecolour, ANSI 256, ANSI 16 and monochrome. | Must | Expansion | All supported tiers preserve meaning and operability. |
| CLR-004 | The runtime should query terminal foreground, background and palette where safe. | Should | Expansion | Resolution can consider the actual terminal environment. |
| CLR-005 | Light/dark appearance must be detectable or user-configurable. | Must | Expansion | Themes can adapt without requiring application restart. |
| CLR-006 | Contrast must be validated after palette quantisation. | Must | Expansion | A colour passing in truecolour is retested in ANSI modes. |
| CLR-007 | Colour must never be the sole carrier of state. | Must | Foundation | Statuses also use text, glyph, weight, position or shape. |
| CLR-008 | Focus and selection must remain distinguishable in monochrome. | Must | Expansion | No-colour snapshots show clear focus and selection. |
| CLR-009 | Theme compilation must emit actionable diagnostics. | Should | Expansion | Insufficient contrast reports the failing token pair and suggested repair. |
| CLR-010 | The system should support colour-vision and greyscale simulation. | Should | Expansion | Devtools can preview common impairment modes. |
| CLR-011 | Existing `eddacraft-tui::Theme` consumers must have a compatibility adapter. | Must | Foundation | A resolved theme can implement or feed the current stable trait. |

## 15. Images and media

| ID | Requirement | Priority | Horizon | Acceptance |
|---|---|---:|---:|---|
| MEDIA-001 | Media assets must be semantic objects separate from placements. | Must | Expansion | One decoded asset can appear in multiple Flow and Scene locations. |
| MEDIA-002 | The runtime must negotiate the best supported representation. | Must | Expansion | Kitty, iTerm2, Sixel, cell raster and text alternatives can be selected. |
| MEDIA-003 | Media must have explicit fit, crop, focal-point and fallback policies. | Must | Expansion | Renderers can adapt the same asset to different regions. |
| MEDIA-004 | Meaningful media must provide text or structured alternatives. | Must | Expansion | Headless and accessibility renderers communicate equivalent purpose. |
| MEDIA-005 | Decorative media must be explicitly marked. | Should | Expansion | Accessibility output can omit non-informative decoration. |
| MEDIA-006 | Untrusted specifications must not access arbitrary paths or URLs. | Must | Expansion | Media acquisition goes through host-approved asset providers. |
| MEDIA-007 | Decoding must enforce byte, dimension, pixel and frame limits. | Must | Expansion | Malformed or hostile images cannot exhaust resources. |
| MEDIA-008 | Media placeholders must reserve stable geometry. | Must | Expansion | Decoding or protocol upgrade does not unexpectedly move surrounding content. |
| MEDIA-009 | Media must clean up protocol resources on removal, resize and exit. | Must | Expansion | No stale terminal graphics remain. |
| MEDIA-010 | Media should support thumbnails, zoom, crop and comparison. | Should | North Star | A screenshot or evidence image can be inspected without replacing its semantic identity. |
| MEDIA-011 | Image colour handling must integrate with theme and background resolution. | Must | Expansion | Alpha and fallback compositing use the resolved surface background. |
| MEDIA-012 | Diagrams should retain structured source where available. | Should | Expansion | A diagram can fall back to a tree, outline or text graph rather than a meaningless raster. |
| MEDIA-013 | Media nodes must participate in Flow and Scene promotion. | Must | Expansion | A thumbnail can become a viewer and collapse back while preserving identity. |
| MEDIA-014 | Existing `ImagePane` usage must remain available as a low-level compatibility path. | Should | Foundation | Applications with a prepared Ratatui protocol can still render it directly. |

## 16. Accessibility and agent semantics

| ID | Requirement | Priority | Horizon | Acceptance |
|---|---|---:|---:|---|
| A11Y-001 | Every interactive semantic node must expose role, name, state and actions. | Must | Foundation | Accessibility and agent adapters can inspect meaningful controls. |
| A11Y-002 | Complex visual content must expose descriptions or structured alternatives. | Must | Expansion | Diagrams, charts and media have non-visual representations. |
| A11Y-003 | Keyboard navigation must cover all interactive operations. | Must | Foundation | Mouse use is never required for core functionality. |
| A11Y-004 | Focus order must be deterministic and inspectable. | Must | Foundation | Devtools can display the active focus route. |
| A11Y-005 | Live updates must expose priority and announcement semantics. | Should | Expansion | Important failures can be surfaced without announcing every streamed token. |
| A11Y-006 | The runtime must support a sequential semantic renderer. | Must | Expansion | A no-layout representation can narrate or serialise the current application. |
| A11Y-007 | Agents must use the same action and permission model as humans. | Must | Expansion | An agent cannot bypass an approval required by a human-triggered action. |
| A11Y-008 | Agent interaction must not depend on cell coordinates. | Must | Expansion | Stable node and action IDs replace screen scraping. |
| A11Y-009 | Reduced-motion and no-colour preferences must be renderer-neutral settings. | Must | Expansion | Terminal, web and native renderers honour the same user preference intent. |

## 17. Security, trust and policy

| ID | Requirement | Priority | Horizon | Acceptance |
|---|---|---:|---:|---|
| SEC-001 | Display text must be safe by default. | Must | Foundation | Untrusted text cannot emit raw ANSI, OSC, control or spoofing characters. |
| SEC-002 | Trusted raw terminal output must use an explicit capability-bearing type. | Must | Foundation | Ordinary strings cannot silently become terminal control sequences. |
| SEC-003 | Specification sources must carry trust classification. | Must | Expansion | Local authored, signed, remote, plugin and agent specs can receive different policy. |
| SEC-004 | Actions must be authorised independently of where they are rendered. | Must | Foundation | A generated button does not gain execution authority from presentation. |
| SEC-005 | Media acquisition must be sandboxed or policy-controlled. | Must | Expansion | Generated UI cannot exfiltrate files or fetch arbitrary remote resources. |
| SEC-006 | Resource limits must apply to specs, trees, charts, text, logs and media. | Must | Foundation | Pathological inputs degrade or reject before unbounded work. |
| SEC-007 | Command approvals must be durable and attributable. | Must | Expansion | Approval records contain actor, command, inputs, preview and time. |
| SEC-008 | Remote renderer connections must not become implicit authority escalation. | Must | Expansion | Session permissions remain server/runtime-owned. |
| SEC-009 | Plugin extension points must have explicit trust boundaries. | Should | Expansion | Untrusted plugins cannot link arbitrary native code into the host process without opt-in. |
| SEC-010 | Security-relevant degradation must be visible. | Must | Foundation | Missing policy, unsupported verification or failed sanitisation cannot silently disappear. |

## 18. Persistence, provenance and replay

| ID | Requirement | Priority | Horizon | Acceptance |
|---|---|---:|---:|---|
| PERSIST-001 | Sessions must have stable identifiers. | Must | Foundation | Runs and application state can be reopened by ID. |
| PERSIST-002 | Command and action history must be recordable. | Must | Expansion | A session timeline can explain what changed state. |
| PERSIST-003 | Persisted state must distinguish semantic state from renderer-local state. | Must | Expansion | Web geometry does not pollute terminal restoration and vice versa. |
| PERSIST-004 | Runtime events should support deterministic replay. | Should | Expansion | A bug report can reproduce state transitions with a scheduler seed. |
| PERSIST-005 | Specification patches must retain provenance. | Must | Expansion | The source of generated or remote UI changes remains inspectable. |
| PERSIST-006 | Artefacts and evidence must be addressable. | Should | Expansion | Commands and Flow nodes can link durable outputs. |
| PERSIST-007 | Persistence formats must be versioned and migratable. | Must | Expansion | Runtime upgrades can detect and migrate older session data. |
| PERSIST-008 | Sensitive values must be redacted or excluded by policy. | Must | Expansion | Replay and exported transcripts do not leak secrets by default. |

## 19. Developer experience, testing and observability

| ID | Requirement | Priority | Horizon | Acceptance |
|---|---|---:|---:|---|
| DEV-001 | The framework must provide an entity and component inspector. | Must | Expansion | Developers can inspect identity, state and relationships live. |
| DEV-002 | The framework must expose the action and command event log. | Must | Expansion | Input-to-state transitions are visible. |
| DEV-003 | The framework must expose task and resource lifetimes. | Must | Expansion | Leaked, cancelled and detached tasks are inspectable. |
| DEV-004 | The framework must expose focus and navigation state. | Must | Expansion | Active scope, available actions and key resolution can be diagnosed. |
| DEV-005 | The framework must expose layout boxes, invalidation and damage. | Should | Expansion | Performance issues can be tied to specific nodes. |
| DEV-006 | The framework must expose terminal capability decisions. | Should | Expansion | Protocol and fallback selection include reasons. |
| DEV-007 | The framework must provide semantic snapshots. | Must | Foundation | Tests can compare roles, labels and content independent of cells. |
| DEV-008 | The terminal renderer must provide cell snapshots. | Must | Foundation | Terminal regressions are captured at defined sizes and profiles. |
| DEV-009 | Visual export snapshots should be supported. | Should | Expansion | SVG or HTML captures can be reviewed without a live terminal. |
| DEV-010 | PTY-level end-to-end tests must cover lifecycle and protocols. | Must | Expansion | Panic restoration, resize and input handling are tested in a real terminal process. |
| DEV-011 | The framework must support terminal-profile simulation. | Must | Expansion | Tests can emulate truecolour, ANSI 16, no images, narrow and remote modes. |
| DEV-012 | The framework should provide a component/workflow workbench. | Should | Expansion | Components and states can be explored without launching the full app. |
| DEV-013 | Interaction recording and deterministic replay should be first-class. | Should | Expansion | A compact trace reproduces input, time, task schedule and state. |
| DEV-014 | Framework diagnostics must be structured. | Must | Foundation | Errors can render in CLI, TUI, web and tests without parsing strings. |
| DEV-015 | Public extension APIs must include explicit stability grades. | Should | Expansion | Consumers can distinguish stable, unstable and experimental surfaces. |

## 20. Performance and reliability

| ID | Requirement | Priority | Horizon | Acceptance |
|---|---|---:|---:|---|
| PERF-001 | Idle applications must perform no periodic rendering work. | Must | Foundation | CPU usage approaches zero when no events or animations occur. |
| PERF-002 | Input-to-visible-update latency must remain perceptibly immediate. | Must | Foundation | Foundation benchmarks define and enforce a latency budget on reference hardware. |
| PERF-003 | Terminal writes must be diffed and batched. | Must | Foundation | Unchanged cells are not repeatedly emitted. |
| PERF-004 | Large Flow documents must be virtualised. | Must | Expansion | Layout and memory scale with visible/near-visible content rather than total history. |
| PERF-005 | Large lists, tables and trees must be virtualised. | Must | Expansion | Million-item synthetic tests remain interactive within documented limits. |
| PERF-006 | Resource limits must fail gracefully rather than panic. | Must | Foundation | Pathological trees, dimensions and data sizes yield diagnostics or truncation. |
| PERF-007 | The renderer must remain correct under resize storms. | Must | Expansion | Rapid size changes do not corrupt state or leave stale cells/media. |
| PERF-008 | The runtime must preserve terminal state after panics where technically possible. | Must | Foundation | Automated tests verify cursor, raw mode and alternate-screen restoration. |
| PERF-009 | Performance instrumentation must be opt-in low-overhead. | Should | Expansion | Production apps can collect frame, layout and task metrics without a debug build. |
| PERF-010 | Slow links must support frame coalescing or reduced update cadence. | Should | Expansion | SSH sessions remain usable without changing semantic state. |

## 21. Cross-platform renderers

| ID | Requirement | Priority | Horizon | Acceptance |
|---|---|---:|---:|---|
| XPLAT-001 | The shared core must define renderer-neutral component semantics. | Must | Foundation | Terminal and sibling renderers can implement the same catalogue entries. |
| XPLAT-002 | The web renderer may use DOM, CSS, browser routing and browser accessibility. | Must | Expansion | These details remain outside core crates. |
| XPLAT-003 | The native renderer may use GPU composition, native windows and OS services. | Must | North Star | Native-specific capabilities do not require terminal emulation. |
| XPLAT-004 | Renderer-local state must be namespaced. | Must | Expansion | A terminal scroll offset and web route can coexist without conflict. |
| XPLAT-005 | Shared conformance tests must validate action, command and semantic parity. | Must | Expansion | A catalogue entry cannot silently lose required actions in one renderer. |
| XPLAT-006 | Session state should be portable across renderers. | Should | Expansion | The same run and selection can be reopened elsewhere where meaningful. |
| XPLAT-007 | Renderer capability gaps must have explicit fallbacks. | Must | Expansion | Unsupported media or layout produces an intentional representation. |
| XPLAT-008 | The framework should provide a headless semantic renderer. | Must | Foundation | Tests, agents and accessibility tools can inspect the current application. |
| XPLAT-009 | A sibling renderer proof must be built before freezing the core API. | Must | Expansion | Terminal assumptions are exposed before 1.0 stability. |

## 22. Compatibility and migration

| ID | Requirement | Priority | Horizon | Acceptance |
|---|---|---:|---:|---|
| MIG-001 | Existing Ratatui widgets must be embeddable. | Must | Foundation | A compatibility component can render a Ratatui widget in a bounded region. |
| MIG-002 | Existing `eddacraft-tui` themes must remain usable. | Must | Foundation | Legacy themes can feed the new resolved-theme adapter. |
| MIG-003 | Existing `eddacraft-tui` JSON Render specs should have a migration path. | Must | Expansion | Current specs can compile directly or through a documented converter. |
| MIG-004 | Existing Pretext prepared text should inform, but not freeze, the new Flow API. | Must | Foundation | Compatibility is possible without preserving every current internal type. |
| MIG-005 | Anvil migration must be incremental. | Must | Expansion | Commands or surfaces can move independently through adapter boundaries. |
| MIG-006 | Anvil must not be required to migrate before the greenfield reference application validates the runtime. | Must | Foundation | Framework milestones precede broad Anvil rewrites. |
| MIG-007 | The compatibility layer must be removable by consumers over time. | Should | Expansion | Native components can replace embedded Ratatui widgets without architecture changes. |
| MIG-008 | Migration must preserve CLI automation contracts unless intentionally versioned. | Must | Expansion | Existing scripts are not broken by TUI architecture changes. |

## 23. Packaging and ecosystem

| ID | Requirement | Priority | Horizon | Acceptance |
|---|---|---:|---:|---|
| PKG-001 | The project should use a modular crate architecture. | Must | Foundation | Core, terminal renderer and compatibility layers have enforceable dependency direction. |
| PKG-002 | Optional heavy features must remain feature-gated. | Must | Foundation | Image decoding, native renderer and web integration do not burden minimal CLI binaries. |
| PKG-003 | The minimal headless runtime should remain small and auditable. | Should | Expansion | Simple command tools can avoid the full terminal/media dependency graph. |
| PKG-004 | Catalogue extensions must be composable. | Must | Expansion | Products can add domain components without forking the base renderer. |
| PKG-005 | Extension and plugin trust levels must be explicit. | Should | Expansion | In-process native extensions differ from declarative catalogue packs. |
| PKG-006 | Examples must include a small CLI, a streaming Flow app and a full workspace. | Must | Expansion | New users can learn the architecture progressively. |
| PKG-007 | Documentation must distinguish stable contracts from illustrative APIs. | Must | Foundation | Early sketches are not accidentally treated as compatibility promises. |

## 24. Foundation release exit criteria

The architecture is considered credibly proven only when a greenfield reference application can:

1. Define a typed command independent of Clap.
2. Invoke it through a CLI adapter.
3. Stream typed events into an inline Flow.
4. Preserve stable identity and scroll anchoring.
5. Promote a Flow node into a Scene panel.
6. Continue the same owned task after promotion.
7. Execute a typed action with permission metadata.
8. Collapse the node back into Flow.
9. Leave a durable plain-text transcript and run ID.
10. Render through a headless semantic renderer.
11. Embed at least one existing Ratatui widget through compatibility.
12. Demonstrate safe untrusted text handling.
13. Demonstrate no-colour and narrow-terminal operation.
14. Run deterministic semantic and terminal snapshots.
15. Avoid any dependency from the core runtime to Ratatui or Anvil.

## 25. Expansion release exit criteria

The system is ready for broad framework evaluation when it additionally supports:

- incremental specification patches;
- typed catalogue compilation;
- virtualised Flow and data views;
- terminal capability negotiation;
- adaptive colour compilation;
- negotiated image/media rendering;
- structured accessibility and agent adapters;
- deep runtime devtools;
- session persistence and replay;
- one non-terminal sibling renderer proof;
- the first incremental Anvil command or workflow migrated onto the runtime.

---

<!-- Source: 04-technical-architecture-specification.md -->

# Technical Architecture Specification

## 1. Purpose

This document specifies the proposed architecture for a renderer-independent semantic application runtime and its terminal-first renderer.

The design is intentionally greenfield. Existing Anvil and `eddacraft-tui` code are sources of validated ideas and migration requirements, but they do not define the core abstractions.

## 2. Architectural objectives

The architecture must:

1. Preserve semantic identity independently of rendered components.
2. Unify CLI commands, UI actions, agents and remote invocation.
3. Support durable sequential Flow and spatial Scene projections.
4. Own asynchronous lifetimes and cancellation.
5. Compile untrusted or dynamic specifications into typed prepared graphs.
6. Separate expensive preparation from hot-path layout and painting.
7. Negotiate terminal capabilities, colour and media.
8. Support terminal, headless, web and native renderers.
9. Provide deterministic testing, replay and deep inspection.
10. Accommodate Ratatui and current `eddacraft-tui` through compatibility adapters.

## 3. System context

```text
                         ┌──────────────────────────────┐
                         │ Application/domain packages  │
                         │ Anvil, reference app, others │
                         └──────────────┬───────────────┘
                                        │ typed commands,
                                        │ entities, catalogue
                                        ▼
┌──────────────┐              ┌──────────────────────────────┐
│ JSON/specs   │─────────────▶│ Semantic application runtime │
│ agents       │ compile/patch│                              │
│ plugins      │              │ entities • actions • tasks   │
│ stored UI    │              │ Flow • Scene • sessions      │
└──────────────┘              └──────────────┬───────────────┘
                                             │ prepared graph
                   ┌─────────────────────────┼─────────────────────────┐
                   ▼                         ▼                         ▼
       ┌────────────────────┐    ┌────────────────────┐    ┌────────────────────┐
       │ Terminal renderer  │    │ Headless renderer  │    │ Web/native sibling │
       │ inline/workspace   │    │ text/JSON/semantic │    │ DOM/GPU/platform   │
       └──────────┬─────────┘    └────────────────────┘    └────────────────────┘
                  │
         ┌────────┴────────┐
         ▼                 ▼
  Ratatui adapter   native terminal compositor
  and ecosystem     and protocol adapters
```

## 4. Dependency rules

### 4.1 Mandatory direction

```text
application/domain
      ↓
semantic runtime core
      ↓
renderer contracts
      ↓
terminal / web / native implementations
      ↓
Ratatui, DOM, GPUI, wgpu, protocol libraries
```

### 4.2 Forbidden dependencies

The core runtime must not depend on:

- Ratatui;
- Crossterm;
- Termwiz;
- Tokio-specific task handles;
- browser DOM types;
- React components;
- GPUI or wgpu types;
- Anvil domain types;
- terminal colour or cell geometry;
- raw JSON as the only internal component representation.

### 4.3 Allowed shared dependencies

The core may depend on lightweight foundational crates for:

- identifiers;
- serialisation traits behind optional features;
- schema description;
- time abstractions;
- futures traits;
- tracing interfaces;
- immutable or arena collections;
- diagnostics.

All heavy renderer and media dependencies should be feature-gated outside the minimal core.

## 5. Proposed crate topology

Names are illustrative and should not be frozen before the first spike.

```text
crates/
├── ui-core
│   ├── entity
│   ├── action
│   ├── command
│   ├── resource
│   ├── task
│   ├── session
│   ├── semantics
│   └── diagnostics
├── ui-spec
│   ├── raw format
│   ├── compiler
│   ├── catalogue
│   ├── patch protocol
│   └── provenance
├── ui-flow
│   ├── document
│   ├── prepared content
│   ├── cursor/range
│   ├── selection
│   └── renderer-neutral layout intent
├── ui-scene
│   ├── regions
│   ├── promotion
│   ├── focus
│   ├── overlays
│   └── renderer-neutral layout intent
├── ui-visual
│   ├── colour intent
│   ├── theme definition
│   ├── media assets
│   ├── accessibility alternatives
│   └── capability requirements
├── ui-renderer
│   ├── renderer contracts
│   ├── semantic tree
│   └── headless implementation
├── ui-terminal
│   ├── terminal session
│   ├── capability negotiation
│   ├── input
│   ├── layout/compositor
│   ├── colour resolver
│   ├── media resolver
│   └── output backends
├── ui-ratatui-compat
│   ├── legacy widget host
│   ├── theme adapter
│   └── buffer conversion
├── ui-cli-clap
│   ├── parser adapter
│   ├── completion
│   └── help
├── ui-web              # sibling proof or repository
└── ui-devtools
    ├── inspector
    ├── recorder
    ├── replay
    └── workbench
```

A smaller initial workspace is acceptable if module boundaries preserve this dependency direction.

## 6. Runtime ownership model

### 6.1 Runtime

The runtime owns:

- entity storage and identity;
- action dispatch;
- command runs;
- resource state;
- task trees;
- session state;
- semantic tree generation;
- invalidation;
- renderer subscriptions;
- replay and diagnostics hooks.

### 6.2 Entity

An entity is a durable typed state object with stable identity.

Conceptually:

```rust
EntityId
Entity<T>
WeakEntity<T>
EntityContext<T>
```

An entity must support:

- typed read and update operations;
- lifecycle hooks;
- action subscriptions;
- owned tasks and resources;
- renderer-neutral semantic projection;
- optional persistence keys;
- mutation attribution.

### 6.3 Identity

Identity should use opaque, globally unique or session-unique identifiers. Domain identifiers may be attached separately.

Required properties:

- stable through view regeneration;
- stable through Flow/Scene promotion;
- serialisable when persisted;
- not derived from array index or transient tree position;
- safe to expose to agents and remote renderers where policy permits.

### 6.4 Mutation model

Mutations should occur within runtime transactions:

```text
action/command event
        ↓
begin mutation transaction
        ↓
update entity/resource state
        ↓
record dependencies and diagnostics
        ↓
commit
        ↓
calculate invalidation
        ↓
notify renderers and observers
```

The runtime should preserve attribution from initiating action to resulting mutations.

## 7. Actions and contextual dispatch

### 7.1 Action definition

Actions are typed intents such as:

```text
OpenFinding
PinNode
PromoteToWorkspace
CollapseToFlow
ApproveRemediation
CancelRun
CopyAsMarkdown
```

An action definition may include:

- stable action name;
- typed payload;
- human label and description;
- default bindings;
- permission requirements;
- risk classification;
- availability predicate;
- accessibility description;
- agent/tool schema;
- optional undo relationship.

### 7.2 Dispatch path

```text
raw input / button / agent / RPC
               ↓
input adapter or invocation adapter
               ↓
typed ActionEnvelope
               ↓
contextual routing
capture → focused target → ancestors/global
               ↓
authorisation and availability
               ↓
handler and mutation transaction
```

### 7.3 Input maps

Raw terminal keys, mouse events, browser events and native shortcuts are renderer-local.

Bindings map these inputs to typed actions based on:

- focused semantic node;
- active mode;
- text-entry state;
- overlays and modal scope;
- application command context;
- user configuration.

### 7.4 Discoverability

The runtime must be able to enumerate currently available actions, enabling:

- command palettes;
- context menus;
- help bars;
- accessible action descriptions;
- agent tool listings;
- test assertions.

## 8. Command architecture

### 8.1 Semantic command

A command is a typed, potentially long-running operation.

It contains:

```text
identity
metadata
input schema
input provenance
validation
permissions and risk
preview/plan
execution
cancellation
retry
compensation/undo
output event schema
result schema
```

### 8.2 Command run

Each invocation creates a `CommandRun` entity with:

- stable run ID;
- command type and version;
- actor and invocation surface;
- resolved inputs and provenance;
- state machine;
- task tree;
- event stream;
- produced artefacts;
- approvals;
- final result;
- timestamps and diagnostics.

### 8.3 State machine

Minimum states:

```text
Created
Validating
AwaitingInput
AwaitingApproval
Queued
Running
Cancelling
Succeeded
Failed
Cancelled
Compensating
Compensated
```

Applications may add domain detail without changing the shared terminal states.

### 8.4 Event model

Recommended base events:

```rust
CommandEvent::Progress
CommandEvent::Log
CommandEvent::Diagnostic
CommandEvent::Prompt
CommandEvent::Diff
CommandEvent::Artifact
CommandEvent::ApprovalRequired
CommandEvent::StatusChanged
CommandEvent::Completed
```

Events remain structured. Renderers decide whether they appear as a line, block, panel, notification or API message.

### 8.5 CLI adapters

A Clap adapter should:

- derive argument parsing from command input schemas where practical;
- preserve explicit custom Clap configuration;
- map sources to input provenance;
- generate help and completion;
- invoke the semantic runtime;
- select plain, structured, inline or workspace rendering mode.

The command model must remain usable with other parsers.

## 9. Structured concurrency

### 9.1 Task tree

Every asynchronous task has:

- a task ID;
- an owner entity or command run;
- a parent task unless explicitly detached;
- cancellation token;
- status;
- optional progress channel;
- diagnostics and tracing context.

### 9.2 Ownership defaults

- Component/entity task: cancelled when owner is disposed.
- Command task: survives view changes and ends with command run.
- Session task: survives navigation within the session.
- Detached durable task: must be explicitly promoted and persisted.

### 9.3 Executor boundary

The core should expose executor traits for:

- spawning asynchronous work;
- spawning blocking work;
- sleeping against an abstract clock;
- yielding;
- cancellation.

Tokio should receive first-class integration, but core data models should not expose Tokio handles.

### 9.4 Backpressure

Stream channels must support:

- bounded buffers;
- coalescing progress updates;
- dropping or summarising low-priority logs by policy;
- preserving diagnostics, approvals and final state;
- renderer-specific consumption rates without changing command semantics.

## 10. Resource model

A resource is a typed value with lifecycle and freshness semantics.

Base state:

```rust
ResourceState<T, E> {
    Idle,
    Loading,
    Ready(T),
    Stale(T),
    Refreshing(T),
    Failed(E),
    Cancelled,
}
```

Resources may be:

- local files;
- database queries;
- network data;
- command-derived views;
- media assets;
- parsed documents;
- indexes;
- policy results.

The runtime records which prepared nodes depend on which resources.

## 11. Flow architecture

### 11.1 Flow document

A Flow is an ordered collection of stable semantic nodes.

```text
FlowDocument
├── FlowNodeId
├── parent/section relationships
├── ordered children or sequence
├── source and provenance
├── actions
├── semantic content
├── presentation hints
└── renderer-local projection state
```

### 11.2 Base node kinds

The core catalogue should eventually cover:

```text
Paragraph
Heading
CodeBlock
DiffBlock
LogBlock
DiagnosticBlock
CommandBlock
ProgressBlock
FindingBlock
EvidenceBlock
ApprovalBlock
ArtifactBlock
MediaBlock
TableBlock
TreeBlock
Callout
Separator
```

Products may define domain-specific nodes.

### 11.3 Prepared Flow content

Flow content should compile into a prepared representation:

```text
PreparedFlowNode
├── typed semantic data
├── sanitised display content
├── measured text fragments
├── action references
├── accessibility representation
├── media references
├── resource dependencies
└── layout capability requirements
```

### 11.4 Flow fragments

Illustrative fragment model:

```rust
TextRun
SoftBreak
HardBreak
InlineCode
Link
Chip
InlineAction
EmbeddedNode
MediaAnchor
```

Fragments retain source offsets and semantic roles.

### 11.5 Cursors and virtualisation

The Flow engine should expose logical cursors rather than only materialised lines:

```text
FlowCursor
FlowRange
LayoutCursor
VisibleRange
Anchor
```

The terminal renderer asks for layout over a viewport plus overscan. Web/native renderers may use their own layout while retaining the semantic cursor and source mapping.

### 11.6 Scroll anchoring

Anchors should identify semantic content rather than raw row numbers.

Examples:

- top visible node and offset;
- selected finding and local line;
- follow-tail mode;
- user-pinned historical position.

When content arrives, the renderer preserves the semantic anchor unless the user is explicitly following the tail.

## 12. Scene architecture

### 12.1 Scene graph

A Scene is a set of stable spatial regions and projections.

```text
Scene
├── WorkspaceId
├── regions
├── projections
├── focus graph
├── overlays
├── layout preferences
└── renderer-local geometry
```

### 12.2 Region types

Renderer-neutral intents may include:

```text
Primary
Secondary
Inspector
Navigation
BottomPanel
Overlay
Popover
CommandPalette
Modal
```

Renderers decide how these map to splits, drawers, routes, windows or tabs.

### 12.3 Projection

A projection references a semantic entity or Flow node and adds renderer/session-local state:

```text
ProjectionId
EntityId or FlowNodeId
region
mode
selection
scroll/zoom
visibility
local preferences
```

The projection does not own the underlying command or domain entity.

### 12.4 Promotion protocol

Promotion should be an action handled by the Scene coordinator:

```text
PromoteToScene {
    node_id,
    preferred_region,
    presentation_mode,
}
```

The coordinator:

1. verifies the node is promotable;
2. creates or reuses a projection;
3. transfers or maps relevant view state;
4. updates focus;
5. records the transition;
6. leaves the underlying node and task ownership unchanged.

### 12.5 Collapse protocol

Collapse removes or hides the projection, stores relevant local state, restores focus and leaves a durable Flow representation.

## 13. Specification architecture

### 13.1 Inputs

The runtime may receive semantic UI from:

- Rust-authored component definitions;
- JSON Render-compatible documents;
- agent-generated specifications;
- server-provided layouts;
- plugins;
- saved dashboards and templates.

All inputs converge on the same prepared graph.

### 13.2 Compilation stages

```text
raw spec
  ↓ parse and size limits
structural graph
  ↓ structural validation
catalogue resolution
  ↓ prop/schema validation
trust and permission analysis
  ↓ data/resource binding
sanitisation and normalisation
  ↓ component preparation
prepared semantic graph
```

Diagnostics are accumulated where safe rather than failing only on the first unrelated issue.

### 13.3 Catalogue definition

A component catalogue entry should define:

- stable component name and version;
- prop schema;
- child/content model;
- semantic roles;
- allowed actions;
- resource requirements;
- media permissions;
- trust policy;
- preparation function;
- renderer capability requirements;
- accessibility representation;
- fallback representations;
- migration rules.

### 13.4 Typed preparation

Generic raw properties must be compiled once into component-specific typed data.

```rust
trait ComponentDefinition {
    type Prepared: Send + Sync + 'static;

    fn prepare(
        &self,
        raw: &RawComponent,
        cx: &mut PrepareContext,
    ) -> Result<Self::Prepared, Diagnostics>;
}
```

A dynamic registry may erase `Prepared` behind a safe internal type-erasure boundary after compilation.

### 13.5 Patch protocol

Patches must be transactional and revisioned.

Base operations:

```text
AddNode
RemoveNode
ReplaceNode
SetProp
RemoveProp
InsertChild
RemoveChild
MoveNode
SetVisibility
AttachAction
DetachAction
```

Envelope metadata:

```text
spec ID
base revision
new revision
transaction ID
source identity
trust class
created time
provenance/evidence reference
optional signature
```

### 13.6 Patch application

```text
receive patch
    ↓
validate envelope and revision
    ↓
authorise source and operations
    ↓
apply to isolated candidate graph
    ↓
validate affected structure and catalogue contracts
    ↓
compile affected nodes
    ↓
commit atomically
    ↓
invalidate affected projections
```

Invalid patches leave the active graph unchanged.

## 14. Prepare, layout and paint pipeline

### 14.1 Universal pipeline

```text
semantic state/specification
            ↓
PREPARE
parse • validate • sanitise • type • measure • index
            ↓
prepared graph
            ↓
LAYOUT
renderer capabilities • viewport • focus • responsive policy
            ↓
layout graph
            ↓
PAINT
terminal cells • DOM • GPU primitives • plain text
            ↓
COMMIT
cell diff • DOM update • render pass • output stream
```

### 14.2 Preparation invariants

Preparation should be:

- deterministic for the same inputs and capabilities;
- side-effect-free except through explicit resource requests;
- safe against hostile content;
- cached by stable identity and dependency versions;
- independent of transient frame timing.

### 14.3 Layout invariants

Layout should:

- operate on prepared nodes;
- retain mapping to semantic IDs;
- support incremental invalidation;
- expose focus/hit-test geometry;
- respect renderer capability and responsive policy;
- avoid repeated text parsing or schema interpretation.

### 14.4 Paint invariants

Painting should:

- use resolved layout and visual tokens;
- avoid application state mutation;
- write only to the renderer’s output abstraction;
- identify damage where possible;
- provide semantic-to-output mapping for debugging.

## 15. Terminal renderer architecture

### 15.1 Terminal session

The terminal renderer owns:

- TTY detection;
- raw mode and cursor state;
- alternate-screen lifecycle;
- input protocol negotiation;
- colour and appearance probes;
- graphics capability negotiation;
- resize events;
- synchronised updates;
- output batching;
- restoration and panic hooks.

### 15.2 Terminal profile

```rust
TerminalProfile {
    identity,
    tty_kind,
    output_mode,
    dimensions_cells,
    dimensions_pixels,
    cell_pixels,
    colour_depth,
    foreground,
    background,
    ansi_palette,
    appearance,
    keyboard_protocol,
    mouse_capabilities,
    hyperlink_support,
    synchronised_output,
    graphics_capabilities,
    multiplexer_context,
    latency_profile,
    known_quirks,
}
```

Every field should distinguish known, unsupported and unknown where necessary.

### 15.3 Output modes

#### Plain/headless

- no cursor control;
- stable text or structured events;
- suitable for CI and pipes.

#### Inline

- preserves scrollback;
- owns a bounded live region where possible;
- can append durable Flow content;
- can transition to workspace.

#### Workspace

- alternate screen where supported;
- full Scene and Flow viewport;
- returns to a clean shell with durable summary.

#### Remote

- applies conservative update cadence;
- considers multiplexer and protocol passthrough;
- favours stable fallbacks.

### 15.4 Terminal compositor

The compositor should manage:

- scene layers;
- clipping;
- portals/overlays;
- Flow regions;
- cell painting;
- image placements;
- hit testing;
- damage tracking;
- frame diffing;
- synchronised commit.

A Ratatui-backed compositor may be used initially. A native terminal compositor can emerge if Ratatui’s buffer or widget contracts become limiting.

## 16. Ratatui compatibility architecture

### 16.1 Compatibility principle

Ratatui is a guest renderer inside a semantic node.

Illustrative adapter:

```rust
LegacyRatatuiNode {
    stable_id,
    render_callback,
    state_handle,
    semantic_fallback,
    action_adapter,
}
```

### 16.2 Required behaviour

The adapter must:

- allocate a bounded terminal region;
- provide a Ratatui frame or buffer facade;
- prevent writes outside the region;
- participate in focus and hit testing where declared;
- expose a semantic fallback;
- allow stateful widgets to retain state;
- avoid exposing Ratatui types to core APIs.

### 16.3 Theme compatibility

A `ResolvedTerminalTheme` should be adaptable to the existing `eddacraft-tui::Theme` trait.

Existing themes can also be imported as a legacy theme definition with reduced adaptive capability.

### 16.4 JSON Render compatibility

Current `eddacraft-tui` specifications should be accepted by an import/compiler layer that:

- preserves stable IDs;
- maps current catalogue types;
- retains data references;
- converts responsive hints;
- reports unsupported or changed semantics;
- compiles into the new prepared graph.

## 17. Colour architecture

### 17.1 Canonical colour intent

Core components use semantic tokens such as:

```text
text.primary
text.secondary
text.disabled
surface.canvas
surface.raised
surface.overlay
border.subtle
border.emphasis
focus.ring
selection.active
status.success
status.warning
status.failure
status.information
diff.added
diff.removed
diff.modified
evidence.verified
evidence.untrusted
action.destructive
```

### 17.2 Theme definition

Theme definitions should express:

- perceptual colour values, preferably OKLCH;
- light/dark variants or generation rules;
- semantic token mappings;
- typography/attribute intent;
- contrast targets;
- no-colour alternatives;
- chart/series generation constraints.

### 17.3 Resolution pipeline

```text
theme definition
      +
terminal visual profile
      +
user accessibility preferences
      ↓
perceptual gamut mapping
      ↓
colour-depth quantisation
      ↓
contrast validation and repair
      ↓
resolved terminal theme
```

### 17.4 Representation fallback

When hue cannot preserve distinction, the resolver may add:

- glyphs;
- labels;
- bold or underline;
- border patterns;
- position or shape changes.

The semantic meaning remains available to all renderers.

## 18. Media architecture

### 18.1 Asset and placement separation

`MediaAsset` owns source, decoded metadata and cache identity.

`MediaPlacement` owns fit, crop, focal point, region, z-order and renderer-local state.

### 18.2 Asset sources

Allowed source types may include:

```text
Embedded
ApprovedLocalFile
ContentAddressedBlob
CommandArtifact
ApprovedRemoteResource
GeneratedDiagram
InMemoryBytes
```

Untrusted specs reference approved asset IDs rather than arbitrary paths.

### 18.3 Decode pipeline

```text
source resolution
      ↓
trust and policy check
      ↓
bounded read
      ↓
format validation
      ↓
bounded decode
      ↓
colour normalisation
      ↓
thumbnail/representation cache
```

### 18.4 Terminal media resolver

Selection inputs:

- protocol support;
- terminal identity and quirks;
- multiplexer support;
- cell/pixel dimensions;
- alpha support;
- animation support;
- latency and size policy;
- user preference.

Potential outputs:

```text
Kitty placement
Unicode-placeholder Kitty placement
iTerm2 inline image
Sixel image
truecolour half-block raster
ANSI 256 raster
Braille or symbol representation
structured text alternative
```

### 18.5 Resource lifecycle

The media subsystem tracks transmitted image IDs and placements. It must delete, replace or reposition terminal resources during:

- node removal;
- resize;
- scrolling;
- promotion/collapse;
- renderer disconnect;
- session exit.

## 19. Accessibility semantic tree

The runtime should emit a renderer-neutral accessibility tree from the same prepared graph.

Node fields:

```text
role
name
description
value
state
relationships
position in set
available actions
shortcut
validation message
live priority
structured alternative
```

Terminal accessibility may use:

- a sequential narrated view;
- explicit focus announcements;
- exported semantic JSON;
- screen-reader integration where terminal/platform support permits.

The agent adapter consumes the same tree plus action schemas and permissions.

## 20. Persistence and replay architecture

### 20.1 Persistence layers

Separate:

1. Domain state.
2. Runtime session state.
3. Shared semantic projection state.
4. Renderer-local state.
5. Event/replay log.

### 20.2 Event envelope

Recorded events should include:

```text
event ID
session ID
causation ID
correlation ID
actor
source surface
action or command identity
payload version
timestamp or logical time
security/redaction classification
```

### 20.3 Replay

Deterministic replay requires:

- recorded semantic inputs;
- virtual clock;
- deterministic scheduler seed;
- captured external-resource responses or fixtures;
- specification patch history;
- renderer profile simulation.

Replay should be able to stop at semantic state even when exact terminal bytes are not reproduced.

## 21. Devtools architecture

### 21.1 Inspector panes

The developer experience should inspect:

- entity graph;
- semantic tree;
- prepared component graph;
- Flow document and anchors;
- Scene projections;
- focus graph;
- available actions;
- command runs and task trees;
- resources and dependencies;
- specification source and patches;
- colour resolution;
- media selection;
- layout boxes and invalidation;
- terminal frame damage and bytes.

### 21.2 Workbench

A component/workflow workbench should support:

- synthetic props and data;
- capability profiles;
- terminal sizes;
- light/dark and colour tiers;
- loading/error/empty states;
- animation and virtual clock controls;
- accessibility tree preview;
- semantic, cell and visual snapshots.

### 21.3 Recorder

The recorder captures:

- raw input;
- resolved actions;
- command events;
- state transactions;
- task scheduling decisions;
- time progression;
- renderer profile changes;
- specification patches.

## 22. Error and diagnostic model

Diagnostics must be structured:

```rust
Diagnostic {
    code,
    severity,
    summary,
    detail,
    source,
    labels,
    related,
    help,
    actions,
    security_class,
}
```

A diagnostic may render as:

- one CLI line;
- a rich Flow block;
- a Scene inspector;
- a browser card;
- structured JSON.

The system should prefer visible degradation over panic or silent omission.

## 23. Performance architecture

### 23.1 Idle model

The event loop wakes for:

- input;
- task/resource events;
- timer deadlines;
- resize;
- explicit animation frames;
- renderer connection changes.

No fixed frame polling should occur when idle.

### 23.2 Invalidation

Invalidation levels may include:

```text
semantic only
prepare node
Flow layout range
Scene layout branch
visual/style only
paint region
full renderer
```

The runtime should conservatively broaden invalidation when correctness is uncertain.

### 23.3 Budgets

The reference implementation should define budgets for:

- input-to-commit latency;
- Flow append preparation;
- visible-range layout;
- cell diff and output bytes;
- memory per retained node;
- maximum spec size and depth;
- maximum media dimensions and frames;
- task/event queue pressure.

Budgets must be measured on Windows, macOS and Linux, including at least one SSH scenario.

## 24. Security architecture

### 24.1 Trust boundaries

Trust classifications may include:

```text
CoreTrusted
ApplicationTrusted
SignedExtension
LocalUserAuthored
RemoteAuthenticated
AgentGenerated
UntrustedRepository
UntrustedNetwork
```

Policy evaluates:

- allowed component types;
- actions;
- data sources;
- media sources;
- raw terminal access;
- filesystem access;
- network access;
- maximum resource budgets.

### 24.2 Safe text types

Recommended distinction:

```text
DisplayText       # sanitised by default
TrustedMarkup     # parsed and controlled semantic styling
TrustedAnsi       # explicit high-risk terminal control capability
RawBytes          # never directly displayable
```

### 24.3 Capability security

Generated UI can reference only capabilities granted to its catalogue and source trust class. Presentation does not confer execution authority.

## 25. Web and native renderer boundaries

The shared renderer contract should expose:

- semantic node tree;
- stable IDs;
- typed actions;
- prepared content and media intent;
- renderer-neutral layout hints;
- focus/navigation semantics;
- change sets and invalidation;
- session and persistence hooks.

It should not expose:

- terminal rectangles;
- DOM nodes;
- CSS classes;
- GPU textures;
- native window handles;
- raw key events.

## 26. Reference implementation strategy

The first implementation should contain:

1. A minimal core runtime.
2. Entity and action systems.
3. Typed commands and command runs.
4. Structured task ownership.
5. A small Flow document and prepared text model.
6. A simple Scene coordinator.
7. Promotion and collapse.
8. A headless renderer.
9. A Ratatui-backed terminal renderer.
10. A Ratatui legacy-widget adapter.
11. Semantic and terminal snapshots.
12. One greenfield reference application.

The implementation should avoid prematurely building:

- a complete widget catalogue;
- a general plugin ABI;
- a full native renderer;
- a complete browser product;
- automatic Anvil migration;
- every modern terminal graphics protocol at once.

## 27. Architecture conformance tests

Automated architecture tests should enforce:

- core crates do not depend on terminal or Anvil crates;
- renderer crates implement required semantic fallback contracts;
- every catalogue component exposes required accessibility metadata;
- risky actions declare permissions;
- specification patches are atomic;
- command events remain structured;
- no-colour rendering preserves state labels;
- Flow/Scene promotion preserves IDs;
- task ownership survives projection changes;
- terminal lifecycle restores state after panic;
- current JSON Render fixtures can be imported or report explicit migration diagnostics.

---

<!-- Source: 05-experience-and-design-specification.md -->

# Experience and Design Specification

## 1. Purpose

This document defines how applications built on the runtime should behave and feel. It focuses on the qualities that create satisfaction, trust and wonder rather than only enumerating widgets.

The intended experience is contemporary but recognisably terminal-native. It should not imitate a browser inside character cells. It should make the shell, command and workspace feel like parts of one coherent environment.

## 2. Experience promise

> The interface composes itself around the work, reveals complexity only when useful, preserves the user’s place, and always leaves behind a durable, understandable record.

## 3. Experience principles

### 3.1 Calm over spectacle

The interface may be visually rich, but it must never become noisy merely to demonstrate terminal capabilities.

- Motion explains causality.
- Colour explains hierarchy and state.
- Images add information.
- Borders and chrome create structure only where needed.
- Idle surfaces remain still.

### 3.2 Progressive disclosure

A simple command should remain a simple command.

The experience may progressively develop:

```text
plain result
  ↓
inline live block
  ↓
expanded Flow details
  ↓
promoted workspace panel
  ↓
full multi-region Scene
```

The user must never be forced into a dashboard merely because the framework can render one.

### 3.3 Spatial memory

The system should protect the user’s mental map.

- Focus remains on the same semantic object through refresh.
- Selection survives sorting and streaming when the selected object still exists.
- Scroll anchors are semantic rather than raw row offsets.
- Promoted content retains its relationship to the originating Flow node.
- Resize changes arrangement without making the user rediscover their place.

### 3.4 Durable output

Interactive experiences should leave useful terminal history.

After an inline or full-screen operation, the user should retain:

- the final outcome;
- important diagnostics;
- artefact locations;
- a stable run identifier;
- a resume or reopen command;
- a clean terminal state.

### 3.5 Discoverability without clutter

Every action should be discoverable, but the interface should not show every possible shortcut at once.

Use:

- contextual help;
- a command palette;
- short active-action hints;
- searchable action descriptions;
- consistent semantic verbs;
- progressive detail.

### 3.6 Graceful degradation

The interface must remain understandable when:

- colour is unavailable;
- images are unavailable;
- the terminal is narrow;
- the session is remote;
- the user uses only a keyboard;
- the output is redirected;
- a component is unsupported;
- data is missing;
- an operation fails.

### 3.7 Trust through transparency

The interface should make clear:

- what is happening;
- what initiated it;
- what authority it has;
- what evidence supports a result;
- what will change if approved;
- how to cancel, undo or inspect;
- whether content was generated, remote or untrusted.

## 4. Application modes

## 4.1 Plain mode

Plain mode is the canonical non-interactive representation.

It must:

- work in CI, pipes and redirected output;
- avoid cursor movement and raw mode;
- provide stable human-readable output;
- provide explicit structured output when requested;
- never emit decorative animation frames;
- include durable identifiers and actionable next steps.

Example:

```text
Assessment complete: 14 passed, 2 warnings, 1 failure

FAIL F-214  Unreviewed model-generated migration
     Evidence: 4 entries
     Review:   anvil finding open F-214

Run: 01K2Q7…
```

## 4.2 Inline mode

Inline mode creates live interaction while respecting terminal scrollback.

It should:

- begin quickly without clearing the terminal;
- reserve only the region currently needed;
- append durable Flow nodes as they settle;
- avoid redrawing historical output unnecessarily;
- support compact prompts, progress and approvals;
- allow promotion into workspace mode;
- collapse cleanly back to ordinary output.

Inline mode should be the default interactive experience for commands that do not need a full workspace immediately.

## 4.3 Workspace mode

Workspace mode provides the full Scene experience.

It should:

- enter only when the user asks or the command genuinely requires it;
- preserve the current Flow and command state;
- provide focus-visible multi-region interaction;
- include a command palette and contextual actions;
- support code, diff, evidence, media and inspector surfaces;
- leave a clean terminal and durable summary on exit.

## 4.4 Remote mode

Remote mode adapts to latency and capability uncertainty.

It should:

- reduce unnecessary frames;
- coalesce progress updates;
- prefer stable text and conservative media protocols;
- preserve keyboard-first operation;
- explain disabled visual capabilities;
- remain fully operable over SSH and common multiplexers.

## 5. The command blooms into an application

The signature interaction is a command that grows only as necessary.

### Stage 1: invocation

```text
$ tool assess .
```

The shell responds immediately with identity and intent.

```text
Assessing repository…
Policy set       engineering-standard
Run              01K2Q7…
```

### Stage 2: live Flow

Structured progress appears as a live block. Logs and low-priority detail remain folded unless requested.

```text
◐ Assessing  9/17 checks
  secrets       passed
  dependencies  passed
  intent        running
```

### Stage 3: meaningful event

A finding appears as a semantic block rather than an arbitrary line.

```text
▲ High  F-214
Unreviewed model-generated migration
4 evidence entries · proposed remediation available
```

### Stage 4: promotion

The user opens or pins the finding. The application promotes it into Scene.

- The Flow node remains visible as its origin or compact summary.
- The detailed panel uses the same finding ID.
- Running work continues without restarting.
- Focus moves predictably.

### Stage 5: deep interaction

The workspace may show:

- finding summary;
- evidence timeline;
- related code;
- proposed diff;
- image or diagram evidence;
- policy explanation;
- approval actions.

### Stage 6: collapse and durable result

When the user exits or collapses the workspace:

- the run continues or finishes according to command policy;
- the Flow contains a settled summary;
- the shell remains clean;
- the user receives resume and artefact references.

## 6. Flow experience

### 6.1 Flow is not a chat transcript by default

Flow may contain conversation, but its primary model is a durable work document.

Nodes should communicate their type through structure, labels and actions rather than relying on chat bubbles for everything.

### 6.2 Node identity

Every significant node should expose a stable reference when useful:

```text
F-214        finding
R-01K2Q7     command run
E-88A        evidence entry
A-023        artefact
```

References should be copyable and usable in CLI or links.

### 6.3 Streaming behaviour

When content streams:

- existing visible words should not jitter;
- the viewport should remain anchored unless following the tail;
- partially generated structures should show a clear pending state;
- completed content should settle rather than disappear;
- the user should be able to pause automatic following;
- very fast low-value updates should be coalesced.

### 6.4 Expand and collapse

Collapsed nodes should communicate:

- identity;
- outcome or state;
- severity;
- quantity of hidden detail;
- key available actions.

Example:

```text
▸ Tool calls  12 completed · 1 warning
```

Expansion should preserve the user’s viewport anchor.

### 6.5 Mixed content

Flow may contain prose, code, chips, links, actions and media in one semantic sequence.

The renderer should avoid creating a bordered panel for every fragment. Use visual containers only when they clarify ownership, state or interaction.

### 6.6 Search and navigation

Flow must support:

- text search;
- semantic filtering by node type, status or severity;
- jump to next diagnostic/finding/action;
- deep-link navigation;
- return to previous anchor;
- follow-tail toggle.

### 6.7 Copy and export

A user should be able to copy or export:

- visible text;
- a complete node;
- Markdown;
- JSON;
- a command invocation;
- an evidence bundle;
- an artefact reference.

The chosen representation should preserve meaning rather than copy raw box-drawing characters.

## 7. Scene experience

### 7.1 Workspace composition

The default workspace should be sparse. Regions appear because content needs them.

A typical hierarchy:

```text
primary work region
secondary detail or evidence region
navigation/outline region when useful
bottom panel for logs or command activity when requested
overlay layer for palette, prompts and approvals
```

### 7.2 Focus

Focus must always be visible.

It should use multiple cues where needed:

- border emphasis;
- cursor/marker;
- title state;
- selection styling;
- semantic announcement.

Focus should never rely on subtle colour alone.

### 7.3 Navigation

Navigation conventions should be consistent but contextual:

- arrow keys always work;
- optional Vim-style bindings may be enabled;
- Tab or explicit focus actions move between regions;
- Escape returns through a predictable hierarchy;
- command palette searches all available actions;
- text-entry fields suspend character-based navigation bindings.

### 7.4 Promotion

Promotion should feel like the content is expanding into workspace, not like an unrelated screen replacement.

Recommended behaviour:

- preserve a visual or semantic origin marker;
- retain the node’s title and identity;
- focus the most useful detailed control;
- keep the originating command or Flow visible when space permits;
- record the transition for back navigation and replay.

### 7.5 Collapse

Collapse should:

- retain relevant local state;
- return focus to the originating node or nearest sensible location;
- update the Flow summary if the state changed;
- avoid losing running work or evidence.

### 7.6 Overlays and approvals

Overlays should be used for temporary decisions, not as the primary navigation model.

A high-risk approval should show:

- exact proposed operation;
- actor and source;
- affected resources;
- preview or diff;
- evidence and policy reason;
- approve, reject and revise actions;
- whether undo is available.

## 8. Colour experience

### 8.1 Semantic purpose

Colour should communicate:

- hierarchy;
- state;
- severity;
- focus;
- grouping;
- provenance or trust;
- diff meaning.

Colour should not be used merely to fill empty space.

### 8.2 Adaptive themes

The application should adapt to:

- dark and light terminal backgrounds;
- truecolour, ANSI 256 and ANSI 16;
- user-supplied palettes;
- no-colour preferences;
- high-contrast preferences.

The experience should be recognisably the same product without assuming exact RGB fidelity.

### 8.3 State redundancy

Every status must combine colour with another signal.

Examples:

```text
✓ Passed
▲ Warning
✕ Failed
◐ Running
○ Queued
! Approval required
```

### 8.4 Focus and selection

Focus, hover/pointer targeting and selection are distinct states and must not collapse into the same colour treatment.

### 8.5 Diff colour

Diffs should support:

- added, removed and modified semantic tokens;
- line and character-level emphasis;
- no-colour prefixes and patterns;
- contrast-safe backgrounds;
- optional syntax colouring that does not obscure diff meaning.

### 8.6 Generated series colours

Charts, agents and workstreams may require generated colours. The runtime should select perceptually distinct colours and add labels or patterns where the palette cannot preserve distinction.

## 9. Images and media experience

### 9.1 Use media when it adds information

Appropriate uses include:

- screenshots;
- architecture diagrams;
- charts whose shape matters;
- image evidence;
- generated artefacts;
- avatars or identity marks where useful;
- visual comparison.

Do not rasterise labels, code or ordinary interface text.

### 9.2 Progressive media resolution

Media should appear in stages without layout instability:

1. A reserved placeholder with name, purpose and dimensions.
2. A lightweight preview or cell representation.
3. The best native protocol representation.
4. An interactive promoted viewer when requested.

### 9.3 Flow media

In a wide Flow, prose may wrap beside a meaningful image or live diagram.

In a narrow Flow, media should stack at a deliberate point in the document.

The surrounding content should not jump when the media representation upgrades.

### 9.4 Media controls

Available actions may include:

- open/promote;
- zoom;
- fit/fill;
- inspect metadata;
- copy path or reference;
- save/export where authorised;
- view structured alternative;
- compare with another asset;
- reveal evidence source.

### 9.5 Media fallback

Fallback order should favour meaning rather than protocol novelty.

For a diagram:

```text
native graphic → cell preview → structured outline/tree → alt text
```

For a screenshot:

```text
native graphic → cell preview → metadata + file reference + purpose description
```

### 9.6 Animation

Animated media should:

- be paused by default when not essential;
- honour reduced motion;
- avoid consuming frames when off-screen;
- expose pause/replay controls;
- degrade to a representative still image.

## 10. Motion and transition experience

### 10.1 Motion communicates cause

Useful motion examples:

- a promoted node visually establishes its relationship to a new panel;
- a completed operation settles into a final state;
- an inserted block expands from its anchor;
- a panel resize interpolates when local and fast enough;
- a failure calls attention once.

### 10.2 Motion must be interruptible

User input, resize or a new state transition should be able to interrupt an animation without leaving invalid geometry.

### 10.3 Slow and remote terminals

The runtime may replace motion with discrete state changes under constrained latency. Semantic history must remain identical.

### 10.4 Reduced motion

Reduced-motion mode should eliminate non-essential interpolation and repeated animation while retaining progress and state changes.

## 11. Error, empty and degraded states

### 11.1 Errors remain in context

An error should appear where the operation or content belongs, with:

- concise summary;
- structured details;
- source location where applicable;
- cause and impact;
- available recovery actions;
- stable diagnostic code.

### 11.2 Unsupported content

Unsupported components should not vanish.

Example:

```text
[HeatMap unavailable in this renderer]
Data points: 348
Open as table · Export JSON
```

### 11.3 Missing data

Missing data should distinguish:

- not yet loaded;
- unavailable;
- permission denied;
- not applicable;
- failed to resolve;
- intentionally redacted.

A generic em dash may be used only when the distinction is not important.

### 11.4 Empty states

An empty state should explain:

- what the region normally contains;
- why it is empty if known;
- the most relevant action.

### 11.5 Disconnection

A disconnected renderer or data source should show:

- last known state;
- connection status;
- whether work continues;
- retry/reconnect actions;
- potential staleness.

## 12. Responsive experience

### 12.1 Semantic priority

Components should define priority tiers.

Example finding:

```text
Wide:
severity + title + evidence preview + owner + actions

Medium:
severity + title + evidence count + primary action

Narrow:
severity + title + compact status
```

### 12.2 Narrow terminals

Narrow mode should:

- stack rather than squeeze critical content;
- prioritise identity and state;
- collapse secondary metadata;
- keep actions reachable through palette or menu;
- avoid horizontal scrolling except for code, diff and data where unavoidable.

### 12.3 Height constraints

Low-height terminals should prioritise:

- active content;
- command state;
- focus and actions;
- temporary hiding of secondary chrome.

### 12.4 Ultrawide terminals

Wide terminals should not stretch prose to unreadable line lengths. Use:

- maximum reading widths;
- additional context panels;
- media or evidence alongside prose;
- multi-column data where semantically useful.

## 13. Keyboard, mouse and touchpad experience

### 13.1 Keyboard first

Every core operation must be available by keyboard.

### 13.2 Mouse support

Mouse and touchpad should improve:

- selection;
- scrolling;
- resizing;
- opening nodes;
- context menus;
- media interaction.

Mouse support must not create hidden actions unavailable elsewhere.

### 13.3 Keybinding philosophy

- Bind semantic actions, not component internals.
- Keep common navigation predictable.
- Allow application and user overrides.
- Display the active contextual binding.
- Never interpret text-entry characters as commands.

### 13.4 Chords

Multi-key chords may be supported, but the framework should provide timeout, discoverability and conflict diagnostics.

## 14. Command palette and help

The command palette is a universal action surface.

It should search:

- available actions;
- commands;
- nodes and entities;
- recent runs;
- settings;
- help topics.

Each result should show:

- action label;
- context;
- shortcut;
- risk/approval marker where relevant;
- why it is available or disabled.

Contextual help should derive from the same action registry rather than manually maintained footer text.

## 15. Accessibility experience

### 15.1 Semantic equivalence

The system must provide equivalent meaning without relying on:

- colour;
- image;
- spatial position alone;
- animation;
- mouse input.

### 15.2 Focus announcements

When focus changes, an accessibility representation should be able to communicate:

- role;
- name;
- state;
- position;
- relevant actions;
- important validation information.

### 15.3 Live updates

Do not announce every streamed token. Announce meaningful state transitions such as:

- command started;
- approval required;
- finding discovered;
- operation failed;
- operation completed.

### 15.4 Images and diagrams

Meaningful images must have:

- concise purpose-oriented alternative text;
- structured detail for complex diagrams or charts;
- access to underlying data where appropriate.

### 15.5 Sequential mode

A user should be able to switch to or export a sequential semantic representation of the active application.

## 16. Agent experience

### 16.1 Agents use capabilities, not pixels

An agent sees:

- semantic nodes;
- typed state;
- available actions;
- action schemas;
- permissions;
- diagnostics;
- structured media alternatives.

It does not need to press keys or infer meaning from colour.

### 16.2 Visible agency

Agent actions should appear in session history with:

- agent identity;
- requested action;
- inputs;
- authorisation result;
- produced changes;
- evidence and outcome.

### 16.3 Governed generative UI

An agent may compose catalogue components, but the interface should indicate generated or untrusted origin where relevant.

Agent-generated interfaces must not look authoritative solely because they use product styling.

### 16.4 Human override

A human should be able to:

- inspect why an agent action is available;
- reject or revise requests;
- pause streaming or automated progression;
- revoke permissions;
- return to a stable prior state where supported.

## 17. Cross-renderer experience consistency

Consistency means preserving:

- command and action names;
- entity identity;
- state and severity;
- permissions and approvals;
- Flow ordering and history;
- relationships;
- media purpose and alternatives;
- user preferences where portable.

Consistency does not require preserving:

- exact panel geometry;
- terminal keybindings in a browser;
- browser routes in a terminal;
- identical animations;
- the same density at every size;
- identical use of modals, tabs or windows.

## 18. Signature user journeys

### 18.1 Long-running assessment

1. User starts command in shell.
2. Inline Flow displays stable run identity and progress.
3. User continues working or opens workspace.
4. Findings stream without resetting focus.
5. User promotes one finding.
6. Evidence, code and diff appear in Scene.
7. User approves or rejects a typed remediation.
8. Workspace collapses to a durable summary.
9. Run can be reopened in terminal or web.

### 18.2 Generated dashboard

1. Agent or server proposes a specification patch.
2. Runtime validates source, catalogue and permissions.
3. New semantic nodes appear incrementally.
4. Unsupported media or components use deliberate fallbacks.
5. User inspects specification provenance in devtools or product UI.
6. Actions execute through the shared command runtime.

### 18.3 Image evidence

1. A finding references a screenshot.
2. Flow reserves stable media geometry.
3. Text alternative and metadata are immediately available.
4. Terminal resolves the best supported image protocol.
5. User promotes the screenshot into a comparison Scene.
6. Image identity and evidence relationship remain intact.
7. Text-only export includes purpose, source and artefact reference.

### 18.4 Renderer handoff

1. User starts a local command in terminal.
2. Run creates durable session ID.
3. User opens the same run in a browser.
4. Browser shows the same findings, evidence and approvals with web-native layout.
5. User performs an authorised action.
6. Terminal receives the state update without losing its current Flow anchor.

## 19. Anti-patterns

Applications built on the framework should avoid:

- clearing the screen for trivial commands;
- putting every item in a bordered card;
- global fixed key handling inside text fields;
- resetting selection after refresh;
- showing spinners with no operation identity;
- replacing structured diagnostics with coloured strings;
- using red/green as the only distinction;
- emitting raw logs as the primary data model;
- hiding unsupported components;
- loading arbitrary image paths from generated specs;
- recreating a node when promoting it;
- forcing web layouts into terminal cells;
- forcing terminal geometry into web/native renderers;
- continuous frame polling while idle;
- animation that delays user input;
- generic confirmation prompts that omit the actual impact.

## 20. “Wonderful” acceptance checklist

A reference experience should be judged successful when:

- It begins as quickly and simply as a good CLI.
- The user always knows what is running and how to stop it.
- The interface can grow into a workspace without restarting work.
- Focus, selection and scroll remain stable under streaming updates.
- Every action is discoverable and semantically named.
- A node can move between Flow and Scene while retaining identity.
- Colour is polished in truecolour and still clear in monochrome.
- Images feel integrated rather than pasted into a rectangle.
- A media placeholder can upgrade without a layout jump.
- Narrow and remote terminals remain first-class.
- Errors degrade visibly and helpfully.
- Exiting leaves useful scrollback and no terminal damage.
- A browser or native renderer can present the same run without imitating terminal layout.
- An agent can operate through typed actions without screen scraping.
- The experience is impressive because it is coherent, not because it is busy.

---

<!-- Source: 06-cross-platform-and-sibling-project-specification.md -->

# Cross-Platform and Sibling Project Specification

## 1. Purpose

This document defines how the semantic runtime should support terminal, web and native application experiences while preserving platform-specific excellence.

It also specifies the role of a sibling project or renderer effort that proves the application architecture outside the terminal.

## 2. Central proposition

Yes: building the terminal system correctly should make future web and native applications better and more consistent.

That benefit comes from sharing:

- identity;
- commands;
- actions;
- state;
- resources;
- task lifetimes;
- permissions;
- evidence and artefacts;
- Flow history;
- semantic components;
- media purpose;
- colour intent;
- accessibility information;
- session and provenance.

It does **not** come from forcing all platforms to render identical layouts.

> Share application semantics and behaviour. Specialise presentation, geometry and platform integration.

## 3. Shared versus renderer-specific concerns

| Shared semantic core | Terminal renderer | Web renderer | Native renderer |
|---|---|---|---|
| Entity identity | Cell and pixel geometry | DOM and CSS layout | GPU/native scene geometry |
| Typed commands | Shell and TTY modes | Browser/API invocation | Native command invocation |
| Typed actions | Keymap and mouse decoding | DOM events and shortcuts | OS input and shortcuts |
| Permission and risk policy | Terminal approval projection | Web approval projection | Native approval projection |
| Flow document | Scrollback and terminal viewport | Timeline/notebook/activity stream | Rich document or activity surface |
| Scene regions | Splits and overlays | Routes, panels, drawers and tabs | Windows, tabs and native panels |
| Resources and tasks | Terminal progress projection | Background jobs and notifications | Native background work |
| Media assets and alternatives | Kitty/Sixel/cell/text | Browser images/canvas/SVG | GPU textures and native viewers |
| Colour intent | ANSI/truecolour resolution | CSS colours and media queries | Native/GPU theme resolution |
| Accessibility semantics | Sequential/narrated terminal view | Browser accessibility tree | OS accessibility APIs |
| Session history | Local transcript/resume | Shared/team session UI | Persistent native workspace |

## 4. Why terminal-first improves the whole architecture

The terminal is an unusually useful forcing function.

### 4.1 It exposes identity failures

A browser can sometimes hide full component replacement behind a smooth transition. In a terminal, focus jumps, scroll movement and stale cells are immediately obvious.

Solving these properly creates better web/native behaviour:

- stable selection during refresh;
- predictable anchoring;
- incremental updates;
- reliable reconnection;
- preserved workspace state;
- easier replay and debugging.

### 4.2 It forces non-visual semantics

Terminal, headless and SSH use make it difficult to rely entirely on colour, animation or pixel layout.

This encourages:

- explicit roles and labels;
- structured diagnostics;
- typed actions;
- durable textual alternatives;
- accessibility-friendly content;
- agent-readable interfaces.

These qualities improve every renderer.

### 4.3 It forces command and application convergence

A terminal application must work both as automation and as an interactive experience.

This creates a strong shared command model that can also support:

- web API calls;
- browser forms;
- native command palettes;
- agents and MCP-style tools;
- background jobs;
- audit and evidence history.

### 4.4 It forces graceful degradation

A terminal renderer must handle no colour, no images, narrow size, pipes and latency. Designing semantic fallbacks early reduces brittleness in browser and native products as well.

## 5. Renderer contract

A renderer should receive:

- stable semantic node IDs;
- prepared node data;
- semantic roles and relationships;
- available typed actions;
- focus and navigation intent;
- renderer-neutral layout priorities;
- media intent and alternatives;
- colour/theme intent;
- change sets or invalidation information;
- renderer-local state namespace;
- session and diagnostics hooks.

A renderer should provide:

- capability profile;
- projection creation and disposal;
- input-to-action mapping;
- focus and hit-testing results;
- layout and paint lifecycle;
- renderer-local state persistence;
- accessibility integration;
- media and colour resolution;
- performance and diagnostics telemetry.

## 6. What must not leak into the core

The shared core must not expose:

- terminal `Rect` or cell types;
- Ratatui widgets or frames;
- raw ANSI or OSC sequences;
- DOM nodes;
- CSS class names;
- React hooks or component instances;
- browser history objects;
- GPUI entities as the canonical entity type;
- wgpu textures;
- native window handles;
- platform raw key events.

Renderer-neutral layout hints may express intent such as priority, region, minimum readable size or promotability, but not final geometry.

## 7. Sibling project model

A sibling project is recommended because it will expose terminal assumptions before the core API hardens.

### 7.1 Sibling project purpose

The sibling effort should prove that:

- the same command run can be projected outside the terminal;
- stable entity identity survives between renderers;
- actions and permissions remain identical;
- Flow and Scene remain meaningful in another interaction model;
- media and colour intent can be resolved differently;
- renderer-local state can differ without corrupting shared state.

### 7.2 Recommended first sibling: web

A web proof is the most practical first sibling because:

- existing JSON Render lineage already includes a web-side catalogue;
- browser accessibility and responsive layout provide a useful contrast to terminal constraints;
- a web surface is valuable for team and remote Anvil experiences;
- it can validate live session synchronisation and URL-addressable entities;
- it does not require committing early to a native Rust GUI framework.

The web proof should remain focused. It does not need to become the complete Anvil dashboard.

### 7.3 Native sibling

A native renderer may follow and could use GPUI, wgpu or another Rust-native framework.

Its purpose would be to validate:

- GPU text and media composition;
- native windows and tabs;
- OS accessibility;
- drag-and-drop;
- high-fidelity animation;
- local desktop integration;
- whether the shared entity/action model remains sufficiently general.

The native project should not be chosen merely because Zed uses GPUI. The renderer must be evaluated against the runtime contract.

## 8. Proposed repository shapes

### Option A: single workspace during incubation

```text
ui-runtime/
├── crates/ui-core
├── crates/ui-spec
├── crates/ui-flow
├── crates/ui-scene
├── crates/ui-terminal
├── crates/ui-ratatui-compat
├── crates/ui-web-prototype
├── crates/ui-devtools
└── examples/reference-app
```

Advantages:

- atomic changes;
- easy API experimentation;
- shared tests;
- avoids premature package release complexity.

Risks:

- renderer boundaries may blur without architecture tests;
- a large workspace can feel monolithic.

### Option B: core repository plus siblings

```text
ui-runtime-core
ui-runtime-terminal
ui-runtime-web
ui-runtime-native
```

Advantages:

- clear dependency and ownership boundaries;
- independent release cadence;
- easier external adoption.

Risks:

- cross-repository coordination during rapid design;
- premature API stability pressure;
- versioning and publish/bump overhead.

### Recommendation

Use a single incubation workspace with strict crate boundaries. Split repositories only after the core contract is proven by terminal and web renderers.

## 9. Shared semantic component catalogue

The catalogue should define component meaning rather than renderer implementation.

Example component:

```text
FindingCard
├── required semantic fields
│   ├── finding ID
│   ├── title
│   ├── severity
│   ├── state
│   └── evidence count
├── optional fields
│   ├── owner
│   ├── summary
│   └── proposed remediation
├── actions
│   ├── OpenFinding
│   ├── OpenEvidence
│   ├── ReviewDiff
│   └── RequestRemediation
├── accessibility representation
├── responsive priorities
└── renderer fallbacks
```

Renderer implementations:

- terminal: compact Flow block and promotable Scene panel;
- web: activity card and route/detail panel;
- native: list row and persistent inspector;
- headless: structured object and concise text.

## 10. Cross-renderer Flow

Flow is shared as ordering, identity, relationships and semantic content.

Renderer interpretation:

### Terminal

- native scrollback or virtualised Flow viewport;
- inline progress and command blocks;
- Pretext-derived rich text layout;
- media negotiated against terminal protocols.

### Web

- activity stream, conversation, notebook or investigation timeline;
- browser virtualisation;
- URL-addressable nodes;
- HTML text, code and media;
- browser selection and accessibility.

### Native

- rich document or event stream;
- GPU-accelerated text and media;
- native selection and drag interactions;
- persistent local workspace.

Flow source order and node identity remain common. Exact line wrapping and geometry do not.

## 11. Cross-renderer Scene

Scene is shared as semantic regions and projections rather than exact splits.

Example intent:

```text
Promote finding F-214 to primary review workspace.
Display evidence as secondary context.
Keep command run activity available.
```

Possible projections:

- terminal: primary/secondary split plus bottom activity panel;
- web: route with central detail and right evidence drawer;
- native: finding tab plus persistent evidence inspector;
- small screen: sequential detail with an evidence sub-route.

## 12. Renderer-local state

The runtime must distinguish:

### Portable semantic state

- selected finding ID;
- active command run;
- expanded/collapsed semantic sections;
- approval state;
- pinned semantic nodes;
- user preference intent such as reduced motion.

### Renderer-local state

- terminal column widths;
- terminal scroll offsets;
- web route and browser history;
- native window placement;
- exact split percentages;
- image protocol caches;
- DOM focus implementation.

Renderer-local state should be namespaced by renderer type and device/session where appropriate.

## 13. Session synchronisation

A future shared session may have multiple renderer connections.

### 13.1 Runtime authority

The runtime or control plane owns:

- domain state;
- command runs;
- action authorisation;
- shared semantic state;
- event history.

Renderers own only local projections.

### 13.2 Updates

```text
renderer action
      ↓
typed action envelope
      ↓
runtime authorisation and mutation
      ↓
semantic change set
      ↓
all connected renderers update their projections
```

### 13.3 Conflict policy

Shared semantic changes should use explicit conflict handling, revision checks or command rules. Renderer-local layout changes should not create shared conflicts.

## 14. Cross-platform colour

The shared layer expresses:

- semantic colour tokens;
- perceptual theme definition;
- contrast targets;
- no-colour representations;
- status and chart distinctions.

Each renderer resolves these against:

- terminal palette and depth;
- browser colour gamut and media queries;
- native display gamut and OS theme;
- user accessibility preferences.

The runtime should not require exact colour equality across devices. It should require preserved hierarchy, contrast and semantic distinction.

## 15. Cross-platform media

A shared `MediaAsset` should include:

- content identity;
- source and trust;
- intrinsic metadata;
- purpose;
- structured alternative;
- relationships to evidence or artefacts.

Renderer representations may be:

- terminal protocol placement;
- browser `<img>`, SVG, canvas or specialised viewer;
- native texture or document view;
- plain metadata and alternative text.

The media asset remains the same semantic object.

## 16. Cross-platform accessibility and agent access

The semantic tree should provide a common basis for:

- browser ARIA or accessibility APIs;
- native OS accessibility;
- terminal sequential/narrated rendering;
- agent inspection and action invocation;
- automated testing.

This convergence is intentional:

> The information needed to make an interface accessible is also the information needed to make it safely operable by an agent.

Agents may receive additional structured data not intended for visual users, but must not receive authority beyond the shared permission model.

## 17. Cross-renderer conformance

A component is conformant when every required renderer provides:

- semantic identity;
- required content;
- required actions;
- state and severity;
- accessibility representation;
- fallback for unsupported media or layout;
- required security and approval behaviour.

Conformance must not compare screenshots across platforms.

## 18. Sibling web proof specification

The first web proof should implement one complete journey:

1. Start or attach to a semantic command run.
2. Display typed progress events in a Flow timeline.
3. Show a finding with the same stable ID as the terminal.
4. Promote the finding into a web-native detail workspace.
5. Display evidence, code/diff and media using browser-native components.
6. Execute one typed action through the shared permission model.
7. Reflect the updated state back in the terminal renderer.
8. Preserve separate terminal and web local layout state.
9. Expose the same accessibility role and action names.
10. Pass shared semantic conformance tests.

This is enough to validate renderer independence without building a complete web product.

## 19. Sibling native proof specification

A later native proof should focus on capabilities that neither terminal nor web validates well:

- multiple native windows or tabs;
- high-fidelity text and image composition;
- OS accessibility;
- drag-and-drop evidence or artefacts;
- native notifications for long-running commands;
- smooth but interruptible promotion transitions;
- local persistence and resume.

## 20. Failure modes to avoid

### 20.1 Lowest-common-denominator components

Do not reduce every component to what all renderers can display identically.

Use shared semantics plus renderer-specific implementations and fallbacks.

### 20.2 Core-owned geometry

Do not store terminal rectangles or browser pixel positions in shared entities.

### 20.3 JSON as the permanent runtime

Do not force all Rust-authored state and interactions through generic JSON values. Compile external formats into typed runtime structures.

### 20.4 Renderer-specific action semantics

Do not create terminal-only approval behaviour and a separate web action path. Actions and policy must remain common.

### 20.5 Premature native framework commitment

Do not choose GPUI or another native toolkit before the renderer contract is proven.

### 20.6 Web project becomes the product core

The web sibling must consume the shared runtime, not redefine it through React state or browser APIs.

## 21. Exit criteria for renderer independence

The architecture can claim credible cross-platform foundations when:

- core crates contain no terminal or web dependencies;
- one command run appears concurrently in terminal and web;
- both renderers preserve the same node IDs and action semantics;
- promotion maps to platform-appropriate workspaces;
- an action executed in one renderer updates the other;
- permissions and approvals are identical;
- colour and media resolve independently;
- renderer-local layout state remains separate;
- semantic conformance and accessibility tests pass;
- no renderer is forced to imitate another’s geometry.

---

<!-- Source: 07-delivery-roadmap-and-anvil-migration-strategy.md -->

# Delivery Roadmap and Anvil Migration Strategy

## 1. Purpose

This document defines how to build the framework without allowing current Anvil or Ratatui constraints to determine the architecture, while still creating a credible path to production use and eventual Anvil migration.

## 2. Delivery principle

> Design greenfield, validate with a purpose-built reference application, preserve compatibility through adapters, and migrate Anvil only after the core boundaries have proven themselves.

This resolves the apparent tension:

- Anvil is the most valuable real workload.
- Anvil must not be allowed to become the design template.

## 3. Strategic workstreams

The programme should run as coordinated but separable workstreams.

### Workstream A — Semantic runtime

Owns:

- entities and identity;
- actions and commands;
- resources and tasks;
- sessions and transactions;
- semantic trees;
- renderer contracts;
- diagnostics and replay.

### Workstream B — Flow and Scene

Owns:

- Flow document model;
- prepared text and rich fragments;
- anchors, selection and virtualisation;
- Scene regions and projections;
- promotion and collapse;
- focus and navigation semantics.

### Workstream C — Terminal renderer

Owns:

- inline and workspace modes;
- terminal lifecycle;
- input and capability negotiation;
- layout/composition;
- colour and media resolution;
- Ratatui compatibility;
- terminal testing.

### Workstream D — Specification and governed UI

Owns:

- catalogue contracts;
- current JSON Render import;
- typed compilation;
- patch protocol;
- trust and provenance;
- generated UI policy.

### Workstream E — Sibling renderer proof

Owns:

- focused web proof first;
- shared semantic conformance;
- cross-renderer session updates;
- renderer-local state separation.

### Workstream F — Anvil migration

Owns:

- adapter seams;
- command-by-command adoption;
- surface migration;
- automation compatibility;
- risk-managed rollout.

Workstream F should begin with analysis and adapters, but broad product migration must lag the core proof.

## 4. Phase 0 — Charter and constraints

### Objective

Create the project boundary before implementation momentum hardens the wrong abstractions.

### Deliverables

- project charter;
- architecture dependency rules;
- terminology and glossary;
- initial ADRs;
- benchmark and reference-terminal matrix;
- reference application brief;
- compatibility inventory for `eddacraft-tui` and Anvil;
- public API stability policy.

### Required decisions

- incubation repository/workspace shape;
- provisional crate boundaries;
- entity ownership prototype direction;
- first terminal backend strategy;
- headless renderer contract;
- first sibling web technology boundary;
- licence and contribution approach.

### Exit criteria

- Core runtime cannot depend on Ratatui or Anvil by policy and build checks.
- Reference application use case is selected.
- Anvil migration is explicitly out of the critical path for the first proof.

## 5. Phase 1 — Runtime kernel spike

### Objective

Prove stable identity, typed actions, command runs and structured concurrency without building a UI framework yet.

### Scope

- `Runtime` and `Entity<T>` prototype;
- typed action registration and dispatch;
- command definition and run entity;
- task ownership and cancellation;
- resource state;
- semantic diagnostic model;
- abstract clock and deterministic test executor;
- headless semantic renderer.

### Reference scenario

A command performs several asynchronous checks, emits progress and diagnostics, accepts cancellation and produces an artefact.

### Deliberate exclusions

- no general widget library;
- no JSON specification compiler;
- no image protocol work;
- no Anvil domain code;
- no complex terminal layout.

### Exit criteria

- A command can be invoked from a test and CLI adapter.
- A command run has a stable ID and structured events.
- Owned tasks cancel correctly.
- Semantic state can be inspected headlessly.
- Deterministic tests can reproduce a seeded interleaving.

## 6. Phase 2 — First terminal expression

### Objective

Prove that a semantic command can become an excellent inline terminal experience and leave durable output.

### Scope

- terminal session lifecycle;
- plain and inline modes;
- basic terminal profile;
- input-to-action mapping;
- minimal Flow document;
- prepared streaming text;
- stable anchors;
- cell diff output;
- Ratatui-backed rendering allowed;
- semantic and cell snapshots.

### Reference scenario

A command streams progress, one diagnostic and one result block into inline Flow.

### Exit criteria

- No fixed frame loop while idle.
- Existing visible content remains stable during streaming.
- The user can cancel through a typed action.
- Exiting leaves a useful transcript and clean terminal.
- No-colour and redirected-output paths work.
- Core crates remain free of Ratatui.

## 7. Phase 3 — Flow/Scene continuity proof

### Objective

Prove the defining interaction: a semantic Flow node becomes a spatial Scene projection and collapses back without losing identity or task state.

### Scope

- Scene coordinator;
- focus graph;
- split regions;
- promotion and collapse actions;
- one code/diff or evidence panel;
- one overlay/command palette;
- preservation of selection and scroll;
- action discoverability.

### Reference scenario

A finding appears in Flow. The user promotes it to a workspace containing summary, evidence and a proposed diff. The command continues running. The finding then collapses back to Flow.

### Exit criteria

- Flow and Scene use the same entity/node ID.
- Task ownership does not change during promotion.
- Focus returns predictably on collapse.
- The settled transcript reflects actions performed in Scene.
- Resize does not reset selection.
- The headless renderer still represents the complete journey.

## 8. Phase 4 — Specification compilation and governed generation

### Objective

Turn existing JSON Render insight into a safe, typed and incremental application specification layer.

### Scope

- import of current `eddacraft-tui` JSON Render fixtures;
- catalogue schemas;
- typed preparation;
- trust classes;
- data/resource binding;
- transactional patch protocol;
- patch provenance;
- component and action permissions;
- deliberate unsupported-component fallback.

### Reference scenario

A simulated agent incrementally produces a dashboard or investigation view. The runtime validates and applies patches while preserving the active node and viewport.

### Exit criteria

- Existing compatible specs import successfully or produce precise diagnostics.
- Invalid patches cannot corrupt the live graph.
- Generated UI cannot attach undeclared risky actions.
- A patch changes only affected prepared nodes.
- Provenance is visible in the inspector.

## 9. Phase 5 — Visual system: colour and media

### Objective

Make adaptive colour and semantic media first-class rather than decorative additions.

### Scope

- perceptual theme definition;
- truecolour, ANSI 256, ANSI 16 and monochrome resolution;
- contrast validation after quantisation;
- light/dark detection and override;
- `MediaAsset` and placement model;
- bounded decode and cache;
- at least two terminal graphics protocols plus cell/text fallback;
- Flow media placeholder and upgrade;
- Scene media viewer;
- structured alternatives.

### Reference scenario

A streamed finding includes a generated architecture diagram and screenshot evidence. Both appear with stable placeholders, resolve to the best terminal representation, and can be promoted into Scene.

### Exit criteria

- No state meaning is lost in monochrome.
- Theme diagnostics identify contrast failures.
- Media upgrade causes no unexpected layout jump.
- Unsupported protocols use intentional fallback.
- Media resources clean up after resize, removal and exit.
- Untrusted specs cannot access arbitrary files or URLs.

## 10. Phase 6 — Sibling web proof

### Objective

Prove the core is genuinely renderer-independent before public API stabilisation.

### Scope

- web renderer for the reference journey;
- shared command session;
- Flow timeline;
- web-native detail workspace;
- shared typed action execution;
- renderer-local route/layout state;
- shared semantic and accessibility conformance tests.

### Exit criteria

- The same command run and finding IDs appear in terminal and web.
- An action in web updates terminal state.
- Permissions and approval behaviour are identical.
- Web uses browser-native layout rather than imitating terminal splits.
- Core APIs require no web- or terminal-specific exceptions.

## 11. Phase 7 — Framework hardening

### Objective

Prepare for external evaluation and initial production adoption.

### Scope

- virtualised Flow, lists, tables and trees;
- deep devtools;
- recorder and replay;
- capability simulation;
- robust SSH/multiplexer behaviour;
- API stability grading;
- documentation and examples;
- performance and security benchmarking;
- compatibility test matrix;
- release process.

### Exit criteria

- Reference performance budgets pass on Windows, macOS and Linux.
- PTY lifecycle tests pass.
- Public extension surfaces have explicit stability grades.
- At least one application outside the reference app can use the framework.
- Terminal and web conformance tests are automated.

## 12. Phase 8 — Incremental Anvil migration

### Objective

Adopt the proven runtime in Anvil without a high-risk rewrite.

### Migration principle

Use a strangler pattern:

```text
existing Anvil CLI/TUI
        │
        ├── unchanged legacy commands
        ├── adapter-backed commands
        └── new semantic-runtime commands
                         ↓
                 progressively expands
```

### Recommended migration order

#### Step 1: command event adapters

Wrap selected existing Anvil operations so they emit structured command events while retaining existing CLI output.

Good candidates:

- read-only status or doctor operations;
- assessment/gate runs;
- evidence browsing;
- long-running progress-heavy commands.

Avoid starting with the most stateful wizard or broadest dashboard.

#### Step 2: inline Flow projection

Render adapted command events through the new terminal Flow while preserving existing plain output contracts.

#### Step 3: finding/evidence promotion

Introduce the Flow-to-Scene experience for one high-value review workflow.

#### Step 4: semantic actions and approvals

Move domain operations from raw key handlers to typed actions and shared permission policy.

#### Step 5: JSON Render import

Run existing Anvil dashboard specifications through the new compiler/import layer.

#### Step 6: media and rich evidence

Adopt semantic media for screenshots, diagrams and artefacts where product value is clear.

#### Step 7: shared web projection

Expose selected Anvil runs or findings through the sibling renderer.

#### Step 8: retire legacy surfaces

Replace legacy Ratatui surfaces only when equivalent or better runtime-native experiences exist and automation contracts remain protected.

## 13. Anvil compatibility layers

### 13.1 CLI compatibility

Maintain:

- command names unless deliberately versioned;
- flags and exit codes;
- JSON/structured output schemas;
- non-interactive behaviour;
- environment and config precedence;
- scripts and CI use.

Interactive improvements must not silently break automation.

### 13.2 Ratatui surface host

Existing Anvil surfaces can be hosted as legacy Scene nodes while new commands migrate around them.

The host should provide:

- terminal region;
- legacy theme adapter;
- state ownership bridge;
- input/action bridge;
- semantic fallback label;
- lifecycle and cleanup.

### 13.3 Existing `eddacraft-tui` widgets

Current widgets remain useful during migration. They should be treated as:

- compatible renderer components;
- visual assets and reference behaviour;
- not the permanent semantic core.

### 13.4 Pretext migration

Existing Pretext code can seed the new Flow text preparation layer.

The new API may diverge to support:

- grapheme cursors;
- rich inline fragments;
- virtualised ranges;
- multiple row regions;
- source mapping;
- renderer-independent prepared text.

Compatibility wrappers may preserve current `PretextState` usage temporarily.

### 13.5 JSON Render migration

Current specs and catalogue names should be imported. Existing behaviour should be classified as:

- directly compatible;
- compatible with richer semantics;
- deprecated but convertible;
- unsupported with explicit diagnostic.

## 14. Greenfield reference application

### 14.1 Why it is mandatory

Without a greenfield application, the framework will tend to reproduce:

- Anvil’s current command boundaries;
- current surface traits;
- current flat action enum;
- Ratatui’s frame ownership;
- current JSON props and layout limitations;
- existing migration compromises.

The reference application creates permission to discover a better model.

### 14.2 Required workload characteristics

The app should include:

- one typed long-running command;
- multiple asynchronous checks;
- streamed prose and structured progress;
- at least one finding or diagnostic;
- evidence and artefacts;
- a proposed diff or change;
- a typed approval action;
- image or diagram media;
- Flow-to-Scene promotion;
- durable transcript and resume;
- headless output;
- a small web sibling projection.

### 14.3 Domain choice

The domain may resemble developer governance because it naturally stresses the right capabilities, but it should not import Anvil code or nomenclature as a requirement.

A neutral “repository assessment and remediation” app is suitable.

## 15. Decision gates

### Gate A — Kernel viability

Proceed only if stable identity, tasks and commands remain understandable and ergonomic in Rust.

### Gate B — Continuity value

Proceed only if Flow/Scene promotion feels materially better than navigation between unrelated screens.

### Gate C — Prepared Flow performance

Proceed only if streaming and virtualised layout meet clear performance and stability budgets.

### Gate D — Specification safety

Proceed only if generated patches can be constrained without making the catalogue unusably rigid.

### Gate E — Renderer independence

Do not freeze core APIs until the web proof succeeds without terminal exceptions.

### Gate F — Anvil adoption

Begin broad migration only after the reference app and at least one Anvil pilot command show lower complexity or better experience than the legacy path.

## 16. Suggested implementation sequence within the first proof

```text
1. IDs and runtime transactions
2. typed actions
3. typed command/run/event model
4. task ownership and cancellation
5. headless semantic renderer
6. terminal lifecycle and plain output
7. inline Flow with prepared streaming text
8. focus and action discovery
9. minimal Scene regions
10. promotion/collapse
11. Ratatui compatibility host
12. semantic and cell snapshots
```

This order prioritises architecture before visual breadth.

## 17. Testing strategy by phase

### Kernel

- unit tests for transactions and ownership;
- deterministic scheduler tests;
- property tests for ID and lifecycle invariants.

### Terminal

- semantic snapshots;
- cell snapshots at multiple sizes;
- PTY tests for lifecycle and resize;
- no-colour and redirected-output tests.

### Flow/Scene

- identity preservation tests;
- focus and anchor stability tests;
- promotion replay tests;
- task-survival tests.

### Specification

- fuzzing parser/compiler boundaries;
- transactional patch tests;
- hostile text and resource-limit tests;
- catalogue conformance tests.

### Visual

- contrast and palette simulations;
- image protocol compatibility tests;
- stale-placement cleanup tests;
- structured alternative tests.

### Cross-platform

- shared semantic fixture tests;
- action/permission parity;
- concurrent renderer session tests;
- renderer-local state separation.

## 18. Performance programme

Benchmarks should begin early and include:

- entity mutation and invalidation;
- command event throughput;
- streamed text append preparation;
- visible Flow layout;
- promotion/collapse cost;
- full and partial terminal paint;
- output bytes per update;
- large document memory;
- media decode and protocol transmission;
- remote latency behaviour.

Do not optimise only synthetic frame rate. Measure interaction latency, stability and transmitted bytes.

## 19. Risk register and mitigations

| Risk | Impact | Mitigation |
|---|---|---|
| Core becomes an over-abstract cross-platform framework | High | Build one concrete terminal reference journey and one small sibling proof; reject abstractions without two consumers. |
| Anvil quietly defines the core | High | Keep reference app free of Anvil imports; architecture tests; migration begins later. |
| Ratatui types leak into semantics | High | Separate crates; explicit compatibility adapter; dependency checks. |
| Flow engine becomes a browser-layout rewrite | High | Limit to terminal-relevant rich flow and semantic cursors; use renderer-specific layout elsewhere. |
| JSON remains stringly typed hot-path state | High | Compile raw specs into typed prepared nodes. |
| Generated UI creates security exposure | High | Catalogue capabilities, trust classes, atomic patches, host-controlled resources. |
| Media protocols are unreliable across terminals | Medium/High | Capability profile, compatibility matrix, conservative fallback, cleanup tests. |
| Colour adaptation changes brand identity | Medium | Semantic tokens, perceptual source theme, documented tier-specific resolution and review tools. |
| Framework work delays Anvil product delivery | High | Parallel tracks, compatibility host, narrow pilots, explicit gates. |
| Early public APIs freeze mistakes | High | Stability grades, incubation workspace, web proof before 1.0. |
| Multiple renderers create scope explosion | High | First sibling implements one end-to-end journey only. |
| Async runtime becomes difficult to use | High | Focused kernel spike, ergonomic task ownership APIs, strong examples. |
| Performance work overcomplicates architecture | Medium | Start conservative; instrument; optimise proven hot paths. |

## 20. What not to do

- Do not begin by rewriting all Anvil TUI surfaces.
- Do not begin by adding dozens of new widgets.
- Do not fork Ratatui immediately.
- Do not choose a native GUI framework before defining the renderer contract.
- Do not create separate command semantics for CLI, UI and agents.
- Do not expose generic JSON values throughout the runtime.
- Do not make the browser proof a complete product initiative.
- Do not chase every terminal protocol before the core interaction works.
- Do not promise identical cross-platform rendering.
- Do not freeze public APIs before a sibling renderer validates them.

## 21. Programme success measures

### Architecture

- Core builds without renderer dependencies.
- Reference app imports no Anvil code.
- Terminal and web share commands, IDs and actions.
- Compatibility adapters are isolated and measurable.

### Developer experience

- A new typed command can gain plain, inline and workspace projections without duplicated execution logic.
- Action discovery and task ownership require little application boilerplate.
- A bug can be reproduced from a compact semantic trace.

### User experience

- First useful output appears immediately.
- Promotion does not restart work or lose place.
- Exit leaves a clean terminal and durable result.
- No-colour and narrow modes remain clear.
- Terminal and web reflect the same run accurately.

### Migration

- The first Anvil pilot command reduces bespoke event-loop/state code.
- Existing automation remains intact.
- Legacy surfaces can coexist during migration.
- Migration can stop or roll back at a command boundary.

## 22. Immediate next planning artefacts

After this specification, the next concrete planning documents should be:

1. Project charter and naming brief.
2. Architecture test and dependency policy.
3. Reference application PRD.
4. Runtime kernel spike specification.
5. Flow/Scene continuity spike specification.
6. Current `eddacraft-tui` compatibility inventory.
7. Anvil command and surface migration inventory.
8. Terminal capability and media test matrix.
9. Sibling web proof PRD.
10. ADR set for identity, task ownership, command model and specification compilation.

---

<!-- Source: 08-decisions-risks-and-open-questions.md -->

# Decisions, Risks and Open Questions

## 1. Purpose

This document separates conclusions already established in the design session from hypotheses that still require evidence.

It should be maintained as the project evolves so exploratory ideas do not silently become permanent architecture.

## 2. Confirmed directional decisions

### DEC-001 — Build a semantic application runtime, not merely a TUI widget framework

**Status:** Confirmed direction

The project owns entities, commands, actions, resources, tasks, Flow, Scene, sessions and renderer contracts. Terminal rendering is one expression.

### DEC-002 — Ratatui is the current practical terminal foundation, not the semantic core

**Status:** Confirmed direction

Ratatui should accelerate implementation and preserve ecosystem compatibility. Core crates must remain independent.

### DEC-003 — Clap remains an adapter

**Status:** Confirmed direction

Clap should parse shell syntax. Semantic commands remain independent and may generate or consume Clap definitions.

### DEC-004 — Stable semantic identity is mandatory

**Status:** Confirmed direction

Entity and node identity survives view regeneration, streaming, promotion, collapse, persistence and renderer changes.

### DEC-005 — Flow and Scene are first-class concepts

**Status:** Confirmed direction

Flow represents durable sequential work. Scene represents spatial interaction. Neither is a fallback for the other.

### DEC-006 — Flow nodes may be promoted into Scene without recreation

**Status:** Confirmed direction

Promotion creates a projection of the same semantic object and must not restart tasks or lose history.

### DEC-007 — Use a universal prepare/layout/paint pipeline

**Status:** Confirmed direction

Parsing, validation, sanitisation, typing and expensive measurement occur in preparation. Renderers perform platform-specific layout and paint.

### DEC-008 — JSON/specifications are boundary formats, not the permanent runtime model

**Status:** Confirmed direction

Raw specifications compile into typed prepared nodes.

### DEC-009 — Generated UI is catalogue-governed

**Status:** Confirmed direction

Agents and remote sources may compose approved capabilities but cannot create executable authority through presentation.

### DEC-010 — Structured concurrency is part of the framework

**Status:** Confirmed direction

Tasks have explicit owners, cancellation, status and observability.

### DEC-011 — Colour is semantic and adaptive

**Status:** Confirmed direction

Applications request semantic tokens. Renderers resolve them for actual capability and accessibility constraints.

### DEC-012 — Media is semantic and negotiated

**Status:** Confirmed direction

Assets are distinct from placements and have protocol-specific plus text/structured representations.

### DEC-013 — Accessibility and agents consume the same semantic foundation

**Status:** Confirmed direction

Roles, names, state and actions should serve accessibility adapters, agents and automated tests.

### DEC-014 — Web/native sibling renderers share semantics, not exact layout

**Status:** Confirmed direction

Platform-specific experience is encouraged. Exact geometry is renderer-local.

### DEC-015 — Anvil is a proving ground and migration target, not a design boundary

**Status:** Confirmed direction

A greenfield reference application must validate the runtime before broad Anvil migration.

### DEC-016 — Preserve current `eddacraft-tui` investments through adapters and imports

**Status:** Confirmed direction

Current widgets, themes, JSON Render specs and Pretext concepts should inform and accelerate the new system without freezing its internals.

## 3. Decisions requiring focused validation

### HYP-001 — Entity model

**Question:** What ownership and borrowing model gives GPUI-like ergonomics without creating opaque runtime magic?

Candidates:

- runtime-owned typed arenas;
- `Arc`/lock-based entities;
- single-threaded UI entities plus message passing;
- generational IDs with explicit contexts;
- hybrid local and sendable entities.

Validation criteria:

- safe updates across `await`;
- low boilerplate;
- clear mutation attribution;
- deterministic disposal;
- renderer-neutral use;
- acceptable performance;
- comprehensible error messages.

### HYP-002 — Reactive dependency tracking

**Question:** Should the runtime use signals, explicit subscriptions, tracked reads, message/update architecture or a hybrid?

The preferred model should avoid:

- hook-order identity;
- hidden global reactivity;
- excessive cloning;
- coarse full-tree invalidation;
- difficult async interactions.

### HYP-003 — Layout engine

**Question:** Should terminal Scene layout use Taffy, Ratatui layout, a custom constraint engine or a hybrid?

Taffy is attractive for flex/grid and intrinsic sizing, but terminal cells, min-size semantics and Flow integration must be benchmarked.

### HYP-004 — Native terminal compositor

**Question:** How long can Ratatui remain the terminal compositor before its frame/widget contracts limit incremental retained rendering, media placement or Flow layout?

The project should avoid a premature fork while preserving an escape path.

### HYP-005 — Flow representation

**Question:** What is the correct prepared representation for rich terminal Flow?

It must support:

- grapheme-safe text;
- streaming appends;
- rich fragments;
- code and diff specialisation;
- embedded nodes and media;
- multiple row regions;
- source mapping;
- selection;
- virtualisation.

### HYP-006 — Promotion ownership

**Question:** Should promotion move projection state, create a second projection, or permit both depending on policy?

The model must remain understandable to users and developers.

### HYP-007 — Command definition mechanism

**Question:** Should commands be declared through traits, derive macros, builders, schema-first definitions or a combination?

The result must remain ergonomic for ordinary Rust applications and expressive for risk, preview, events and undo.

### HYP-008 — Specification patch format

**Question:** Use a custom typed patch protocol, JSON Patch, CRDT operations or an operation-log model?

Requirements include stable IDs, atomic validation, provenance, revision checks and constrained actions.

### HYP-009 — Cross-renderer synchronisation

**Question:** What is the minimum shared session protocol needed for terminal and web proofs?

Avoid building a distributed systems platform before the semantic model is proven.

### HYP-010 — Colour contrast model

**Question:** Which contrast standards and repair algorithms should be enforced?

Likely approach:

- WCAG 2.x ratios for recognised compliance targets;
- perceptual/APCA diagnostics as additional design information;
- validation after terminal palette quantisation;
- human review for brand-critical tokens.

### HYP-011 — Colour source model

**Question:** Is OKLCH the sole canonical source or one supported authoring model?

Consider interoperability, gamut mapping, terminal output and theme import.

### HYP-012 — Media protocol selection

**Question:** What capability probing and compatibility database is reliable enough across Kitty, Ghostty, WezTerm, iTerm2, Windows Terminal, tmux, screen and SSH?

### HYP-013 — Web renderer technology

**Question:** Should the web sibling use React, another framework, web components or a minimal custom renderer?

The answer should be based on proving the renderer contract, not ideology.

### HYP-014 — Native renderer technology

**Question:** GPUI, wgpu-based custom UI, Dioxus desktop, Iced, egui or another framework?

Defer until terminal and web proofs identify actual requirements.

### HYP-015 — Plugin model

**Question:** How should declarative catalogue extensions, trusted Rust crates, dynamic libraries, WASM and remote plugins differ?

Do not collapse all extension types into one trust model.

## 4. Open product questions

### Product identity and naming

- Is this an eddacraft-branded framework, an Anvil-adjacent project or an independent open-source identity?
- Should “TUI” appear in the name if the long-term category is broader?
- Does the project lead with terminal excellence or cross-platform semantic runtime?

### Target adopter

- Initially internal to Anvil and eddacraft products?
- Open to ambitious Rust CLI developers early?
- Intended eventually as a general application framework?
- Focused on developer and agent applications rather than forms/dashboard TUIs?

### First reference application

- Repository assessment and remediation?
- Agent tool execution environment?
- Evidence and investigation workspace?
- A neutral domain that still stresses code, diff, media and approvals?

### Public surface

- Which APIs should be stable before Anvil adoption?
- How much macro use is acceptable?
- Should application authors commonly interact with the entity runtime directly?
- Should JSON/spec support be optional or central in public positioning?

## 5. Open technical questions

## 5.1 Threading model

- Is the UI runtime single-threaded with sendable task results?
- Can entities be updated from background threads?
- How are renderer connections isolated?
- What is the cost and complexity of locks versus runtime scheduling?

## 5.2 Transaction model

- Are nested mutations allowed?
- How are async mutations sequenced?
- Can transactions fail and roll back?
- How much event sourcing is useful versus excessive?

## 5.3 Persistence

- What state is persisted by default?
- Which session data is sensitive?
- How are schema migrations defined?
- Is local SQLite appropriate for the reference app?
- How does a remote/team runtime differ from a local runtime?

## 5.4 Focus

- Is focus represented as a tree, graph or ordered route?
- How do Flow selection and Scene focus interact?
- Can one entity have focus in multiple renderer sessions?
- Which focus changes are shared versus renderer-local?

## 5.5 Flow selection

- How does selection cross text, inline chips, media and blocks?
- What should copy produce by default?
- How are code/diff rectangular selections represented?
- How does selection survive streamed updates?

## 5.6 Layout

- How much CSS-like behaviour is beneficial?
- Should layout constraints be typed Rust, declarative spec props or both?
- How are intrinsic measurements cached across renderers?
- Can Flow and Scene share any layout primitives without conflation?

## 5.7 Styling

- Are theme tokens compile-time, runtime or both?
- How should component variants and states be expressed?
- Is an external style language worth supporting?
- How is specificity kept predictable?

## 5.8 Text shaping

- How far should terminal text shaping go beyond `unicode-width`?
- How are ambiguous-width characters handled?
- What terminal assumptions can be queried versus user-configured?
- How do bidi and spoofing safety interact with legitimate language support?

## 5.9 Images

- Which formats are decoded in process?
- Should SVG be rasterised in a sandbox or constrained parser?
- How are animated formats budgeted?
- How are image caches bounded and invalidated?
- Can terminal placements be reliably anchored to virtualised Flow?

## 5.10 Remote operation

- Does the runtime run with the terminal client, with a local daemon or remotely?
- How are sessions attached and authenticated?
- What latency and disconnect semantics are required for the first version?

## 6. Risk register

| ID | Risk | Probability | Impact | Leading indicator | Mitigation |
|---|---|---:|---:|---|---|
| RISK-001 | The project becomes too broad to ship. | High | High | Multiple renderers and protocols advance before the first journey works. | Strict phase gates and one reference journey. |
| RISK-002 | Abstractions are elegant but unpleasant for ordinary Rust developers. | Medium | High | Excessive contexts, type erasure or macro errors. | Ergonomic spike, small examples, external design review. |
| RISK-003 | Anvil migration pressure compromises renderer independence. | High | High | Core APIs gain Anvil terms or legacy surface assumptions. | Greenfield app, dependency checks, migration workstream separation. |
| RISK-004 | Ratatui compatibility becomes permanent architecture debt. | Medium | Medium/High | New components are built only through legacy adapter. | Native component targets and adapter usage telemetry. |
| RISK-005 | Custom Flow layout consumes disproportionate effort. | High | High | Text edge cases block command/runtime progress. | Stage functionality, borrow Pretext work, benchmark focused primitives. |
| RISK-006 | Specification system duplicates a web framework. | Medium | High | Catalogue grows CSS/DOM-like behaviour without terminal value. | Keep semantics and constrained layout; compile to renderer-native systems. |
| RISK-007 | Generated UI security is underestimated. | Medium | Very high | Specs begin referencing arbitrary actions, paths or URLs. | Capability catalogue, trust classes, resource providers, threat model. |
| RISK-008 | Colour/media polish obscures core interaction problems. | Medium | Medium | Visual demo looks strong but promotion/task model remains weak. | Visual phase after continuity proof. |
| RISK-009 | Terminal protocol support is brittle. | High | Medium/High | Stale graphics, corrupted scrollback, multiplexer failures. | Compatibility matrix, conservative defaults, inspectable fallback. |
| RISK-010 | Cross-platform core becomes lowest common denominator. | Medium | High | Components omit platform-specific strengths to preserve sameness. | Semantic conformance only; renderer-specific components and variants. |
| RISK-011 | Runtime persistence creates privacy/security exposure. | Medium | High | Traces contain secrets or generated sensitive content. | Classification, redaction, retention policy, opt-in recording. |
| RISK-012 | Public release freezes premature APIs. | High | High | External users depend on experimental contracts. | Stability grades, 0.x policy, incubation docs and migration tooling. |
| RISK-013 | Web sibling becomes a separate implementation. | Medium | High | React state duplicates runtime state and command semantics. | Shared conformance fixtures and runtime-owned authority. |
| RISK-014 | Performance is optimised around frame rate rather than useful latency. | Medium | Medium | High FPS while input, streaming or SSH remains poor. | Measure input latency, bytes, idle cost and anchor stability. |
| RISK-015 | Native ambition distracts from terminal product. | Medium | High | Native toolkit evaluation begins before web proof. | Defer native selection and implementation. |

## 7. Required experiments

### EXP-001 — Entity ergonomics

Build the same small application state with two or three candidate ownership models.

Measure:

- lines of application code;
- async update ergonomics;
- error quality;
- deterministic disposal;
- mutation tracing;
- performance.

### EXP-002 — Command projection

Define one command and project it into:

- plain output;
- JSONL;
- inline Flow;
- a simple workspace;
- an agent schema.

Reject the model if it requires duplicated command logic.

### EXP-003 — Flow streaming and anchoring

Stream styled text, diagnostics and inserted blocks while:

- following tail;
- reading history;
- selecting text;
- resizing.

Measure layout cost and visible movement.

### EXP-004 — Promotion continuity

Promote one running command/finding node into Scene and collapse it repeatedly.

Verify:

- stable ID;
- no task restart;
- focus restoration;
- preserved scroll/selection;
- replay fidelity.

### EXP-005 — Taffy versus terminal-specific layout

Implement the same Scene layout using:

- Ratatui layout;
- Taffy;
- a minimal custom engine.

Evaluate intrinsic sizing, incremental invalidation, API ergonomics and terminal edge cases.

### EXP-006 — Ratatui compatibility host

Embed current `eddacraft-tui` widgets with state, input, focus and semantic fallback.

Determine whether the adapter can be cleanly isolated.

### EXP-007 — JSON Render import and compilation

Compile representative existing specs into typed prepared nodes and apply incremental updates.

Measure:

- migration compatibility;
- compile diagnostics;
- runtime overhead;
- catalogue ergonomics.

### EXP-008 — Colour compilation

Resolve one semantic theme across:

- dark/light truecolour;
- ANSI 256;
- multiple custom ANSI 16 palettes;
- monochrome.

Validate contrast, semantic distinction and brand recognisability.

### EXP-009 — Media placement lifecycle

Render, scroll, resize, promote, collapse and delete the same media asset across:

- Kitty;
- iTerm2 or Sixel;
- cell fallback.

Verify no stale graphics and stable layout.

### EXP-010 — Terminal/web concurrent session

Connect terminal and web renderers to one command run, execute an action from each, and verify shared state plus independent layout state.

## 8. Decision sequence

The project should decide in this order:

1. Runtime identity and ownership.
2. Action and command model.
3. Task ownership and deterministic test model.
4. Headless semantic renderer.
5. Flow representation and anchoring.
6. Scene projection and promotion.
7. Terminal compositor strategy.
8. Specification compiler and patches.
9. Colour system.
10. Media protocols.
11. Web renderer proof.
12. Native renderer investigation.
13. Plugin model.

This order avoids choosing downstream technologies before the semantic contract is understood.

## 9. ADR backlog

Recommended initial ADRs:

- ADR-001: renderer-independent core and dependency direction;
- ADR-002: entity identity and ownership;
- ADR-003: mutation transactions and attribution;
- ADR-004: typed action model and contextual dispatch;
- ADR-005: semantic command and event model;
- ADR-006: structured task ownership and cancellation;
- ADR-007: Flow and Scene distinction;
- ADR-008: promotion and projection identity;
- ADR-009: prepare/layout/paint pipeline;
- ADR-010: Ratatui compatibility boundary;
- ADR-011: raw spec compilation and catalogue authority;
- ADR-012: transactional patch protocol;
- ADR-013: colour intent and terminal resolution;
- ADR-014: media asset and placement model;
- ADR-015: accessibility and agent semantic tree;
- ADR-016: renderer-local versus shared session state;
- ADR-017: first sibling web renderer boundary;
- ADR-018: Anvil migration sequencing.

## 10. Review questions

Every architecture review should ask:

1. Does this type belong to semantics or to a renderer?
2. Does it preserve stable identity?
3. Can the headless renderer explain it?
4. Can an agent invoke it without screen scraping?
5. Does it have an accessibility representation?
6. What happens without colour or images?
7. What is the trust boundary?
8. Who owns its tasks and resources?
9. What state is shared versus renderer-local?
10. Does this decision accidentally optimise for current Anvil or Ratatui?
11. Has a second renderer validated the abstraction?
12. Can failure degrade visibly rather than panic or disappear?

---

<!-- Source: 09-reference-api-and-data-models.md -->

# Reference API and Data Models

## 1. Purpose and status

This document provides illustrative Rust and wire-format sketches to make the architecture concrete.

These examples are **not frozen public API**. They should inform spikes and discussion, then be replaced by validated designs.

## 2. Core identifiers

```rust
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct EntityId(pub uuid::Uuid);

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct SessionId(pub uuid::Uuid);

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct CommandRunId(pub uuid::Uuid);

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct NodeId(pub std::sync::Arc<str>);

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ProjectionId(pub uuid::Uuid);
```

IDs must not encode tree position or renderer geometry.

## 3. Runtime and entities

```rust
pub struct Runtime {
    // Private arenas, scheduler, action registry, sessions and observers.
}

pub struct Entity<T> {
    id: EntityId,
    _marker: std::marker::PhantomData<T>,
}

pub struct WeakEntity<T> {
    id: EntityId,
    _marker: std::marker::PhantomData<T>,
}

pub trait EntityState: Send + Sync + 'static {
    fn semantics(&self, cx: &SemanticContext<'_>) -> SemanticNode;
}

impl Runtime {
    pub fn create<T: EntityState>(&self, state: T) -> Entity<T>;

    pub fn read<T, R>(
        &self,
        entity: &Entity<T>,
        f: impl FnOnce(&T) -> R,
    ) -> Result<R, EntityError>;

    pub fn update<T, R>(
        &self,
        entity: &Entity<T>,
        cause: MutationCause,
        f: impl FnOnce(&mut T, &mut EntityContext<T>) -> R,
    ) -> Result<R, EntityError>;
}
```

Alternative ownership models should be tested before adoption.

## 4. Mutation attribution

```rust
pub enum MutationCause {
    Action(ActionInvocationId),
    CommandEvent(CommandRunId),
    Resource(ResourceId),
    System(SystemCause),
    Replay(ReplayEventId),
}

pub struct MutationRecord {
    pub transaction_id: uuid::Uuid,
    pub cause: MutationCause,
    pub changed_entities: Vec<EntityId>,
    pub invalidations: Vec<Invalidation>,
    pub diagnostics: Vec<Diagnostic>,
}
```

## 5. Typed actions

```rust
pub trait Action: serde::Serialize + Send + Sync + 'static {
    const NAME: &'static str;

    fn metadata() -> ActionMetadata;
}

pub struct ActionMetadata {
    pub label: &'static str,
    pub description: &'static str,
    pub risk: Risk,
    pub required_permissions: &'static [Permission],
    pub default_bindings: &'static [BindingHint],
}

#[derive(Clone, Copy, Debug)]
pub enum Risk {
    ReadOnly,
    LocalMutation,
    ExternalMutation,
    Destructive,
    Privileged,
}

pub struct ActionEnvelope<A: Action> {
    pub invocation_id: uuid::Uuid,
    pub session_id: SessionId,
    pub actor: Actor,
    pub target: EntityId,
    pub action: A,
    pub source: InvocationSource,
}
```

Example:

```rust
#[derive(serde::Serialize)]
pub struct PromoteToWorkspace {
    pub node: NodeId,
    pub preferred_region: Option<RegionIntent>,
}

impl Action for PromoteToWorkspace {
    const NAME: &'static str = "workspace.promote";

    fn metadata() -> ActionMetadata {
        ActionMetadata {
            label: "Open in workspace",
            description: "Promote this item into a detailed workspace",
            risk: Risk::ReadOnly,
            required_permissions: &[],
            default_bindings: &[BindingHint::Key("enter")],
        }
    }
}
```

## 6. Commands

```rust
pub trait Command: Send + Sync + 'static {
    type Input: CommandInput;
    type Output: CommandOutput;
    type Event: CommandEvent;
    type Error: std::error::Error + Send + Sync + 'static;

    const NAME: &'static str;
    const VERSION: u32;

    fn metadata() -> CommandMetadata;

    fn validate(
        input: &Self::Input,
        cx: &ValidationContext<'_>,
    ) -> Result<(), Diagnostics>;

    fn preview(
        input: &Self::Input,
        cx: &CommandContext<'_>,
    ) -> impl std::future::Future<Output = Result<Option<Preview>, Self::Error>> + Send;

    fn execute(
        input: Self::Input,
        cx: CommandContext<'_>,
        events: EventSink<Self::Event>,
    ) -> impl std::future::Future<Output = Result<Self::Output, Self::Error>> + Send;
}
```

A macro or builder may generate parts of this contract after ergonomic testing.

## 7. Command metadata

```rust
pub struct CommandMetadata {
    pub summary: &'static str,
    pub description: &'static str,
    pub risk: Risk,
    pub permissions: &'static [Permission],
    pub supports_preview: bool,
    pub supports_cancel: bool,
    pub retry: RetryPolicy,
    pub compensation: CompensationPolicy,
}

pub enum RetryPolicy {
    Never,
    UserInitiated,
    Automatic {
        attempts: u32,
        backoff: Backoff,
    },
}
```

## 8. Input provenance

```rust
pub struct ResolvedInput<T> {
    pub value: T,
    pub source: InputSource,
    pub explicitly_supplied: bool,
}

pub enum InputSource {
    CliArgument,
    Pipeline,
    Environment { name: String },
    Configuration { path: String, key: String },
    StoredPreference,
    InteractivePrompt,
    Agent,
    RemoteApi,
    Default,
}
```

## 9. Command events

```rust
pub enum StandardCommandEvent {
    Progress(ProgressEvent),
    Log(LogEvent),
    Diagnostic(Diagnostic),
    Prompt(PromptRequest),
    Diff(DiffArtifact),
    Artifact(ArtifactRef),
    ApprovalRequired(ApprovalRequest),
    StatusChanged(CommandStatus),
    Completed(CommandCompletion),
}

pub struct ProgressEvent {
    pub operation_id: String,
    pub label: String,
    pub completed: Option<u64>,
    pub total: Option<u64>,
    pub unit: Option<String>,
    pub importance: EventImportance,
}
```

## 10. Command run

```rust
pub struct CommandRun {
    pub id: CommandRunId,
    pub command_name: String,
    pub command_version: u32,
    pub actor: Actor,
    pub source: InvocationSource,
    pub status: CommandStatus,
    pub started_at: Option<Timestamp>,
    pub finished_at: Option<Timestamp>,
    pub events: FlowDocumentId,
    pub tasks: TaskTreeId,
    pub approvals: Vec<ApprovalId>,
    pub artifacts: Vec<ArtifactRef>,
}

pub enum CommandStatus {
    Created,
    Validating,
    AwaitingInput,
    AwaitingApproval,
    Queued,
    Running,
    Cancelling,
    Succeeded,
    Failed,
    Cancelled,
    Compensating,
    Compensated,
}
```

## 11. Structured tasks

```rust
pub struct TaskSpec {
    pub name: String,
    pub owner: TaskOwner,
    pub cancellation: CancellationPolicy,
    pub persistence: TaskPersistence,
}

pub enum TaskOwner {
    Entity(EntityId),
    Command(CommandRunId),
    Session(SessionId),
    Runtime,
}

pub enum TaskPersistence {
    Scoped,
    SurviveProjection,
    SurviveRendererDisconnect,
    Durable,
}
```

## 12. Resources

```rust
pub enum ResourceState<T, E> {
    Idle,
    Loading,
    Ready(T),
    Stale(T),
    Refreshing(T),
    Failed(E),
    Cancelled,
}

pub struct Resource<T, E> {
    pub id: ResourceId,
    pub version: u64,
    pub state: ResourceState<T, E>,
}
```

## 13. Semantic nodes

```rust
pub struct SemanticNode {
    pub id: NodeId,
    pub role: SemanticRole,
    pub name: Option<String>,
    pub description: Option<String>,
    pub value: Option<SemanticValue>,
    pub state: SemanticState,
    pub relationships: Vec<SemanticRelationship>,
    pub actions: Vec<ActionDescriptor>,
    pub children: Vec<NodeId>,
    pub content: SemanticContent,
    pub presentation: PresentationHints,
}

pub enum SemanticRole {
    Application,
    Document,
    Section,
    Heading,
    Paragraph,
    Code,
    Diff,
    List,
    ListItem,
    Table,
    Tree,
    Button,
    TextField,
    Progress,
    Status,
    Diagnostic,
    Finding,
    Evidence,
    Approval,
    Image,
    Diagram,
    Link,
    Custom(String),
}
```

## 14. Flow model

```rust
pub struct FlowDocument {
    pub id: FlowDocumentId,
    pub title: Option<String>,
    pub nodes: Vec<NodeId>,
    pub revision: u64,
}

pub enum FlowContent {
    RichText(PreparedRichText),
    Code(PreparedCode),
    Diff(PreparedDiff),
    Log(PreparedLog),
    Diagnostic(Diagnostic),
    Progress(ProgressModel),
    Finding(FindingSummary),
    Evidence(EvidenceSummary),
    Approval(ApprovalRequest),
    Artifact(ArtifactRef),
    Media(MediaReference),
    Container(FlowContainer),
}
```

## 15. Rich Flow fragments

```rust
pub enum FlowFragment {
    Text(TextRun),
    SoftBreak,
    HardBreak,
    InlineCode(TextRun),
    Link(LinkFragment),
    Chip(ChipFragment),
    InlineAction(ActionRef),
    EmbeddedNode(NodeId),
    MediaAnchor(MediaPlacementId),
}

pub struct TextRun {
    pub text: std::sync::Arc<str>,
    pub role: TextRole,
    pub source_range: Option<SourceRange>,
}
```

## 16. Prepared text and layout cursors

```rust
pub struct PreparedRichText {
    pub fragments: Vec<PreparedFragment>,
    pub source_index: SourceIndex,
    pub revision: u64,
}

pub struct FlowCursor {
    pub node: NodeId,
    pub fragment_index: usize,
    pub grapheme_offset: usize,
}

pub struct FlowRange {
    pub start: FlowCursor,
    pub end: FlowCursor,
}

pub trait FlowLayouter {
    fn layout_range(
        &mut self,
        prepared: &PreparedRichText,
        constraints: &FlowConstraints,
        start: &FlowCursor,
        viewport: ViewportExtent,
    ) -> FlowLayoutPage;
}
```

## 17. Flow constraints

```rust
pub struct FlowConstraints {
    pub inline_extent: LogicalLength,
    pub block_extent: Option<LogicalLength>,
    pub regions: Vec<FlowRegion>,
    pub typography: TypographyMetrics,
    pub capability: FlowCapability,
}

pub struct FlowRegion {
    pub block_start: LogicalLength,
    pub block_end: LogicalLength,
    pub available_segments: Vec<InlineSegment>,
}
```

The terminal implementation maps logical lengths to cells. Other renderers use their own units.

## 18. Scene model

```rust
pub struct Scene {
    pub id: WorkspaceId,
    pub regions: Vec<SceneRegion>,
    pub projections: Vec<Projection>,
    pub focus: FocusState,
    pub overlays: Vec<ProjectionId>,
}

pub struct Projection {
    pub id: ProjectionId,
    pub node: NodeId,
    pub region: RegionIntent,
    pub mode: ProjectionMode,
    pub shared_state: ProjectionSharedState,
    pub renderer_state_key: Option<RendererStateKey>,
}

pub enum RegionIntent {
    Primary,
    Secondary,
    Inspector,
    Navigation,
    BottomPanel,
    Overlay,
    Modal,
    Custom(String),
}
```

## 19. Promotion API

```rust
pub struct PromotionRequest {
    pub node: NodeId,
    pub preferred_region: Option<RegionIntent>,
    pub mode: ProjectionMode,
    pub retain_flow_summary: bool,
}

impl SceneCoordinator {
    pub fn promote(
        &mut self,
        request: PromotionRequest,
        cx: &mut RuntimeContext<'_>,
    ) -> Result<ProjectionId, Diagnostic>;

    pub fn collapse(
        &mut self,
        projection: ProjectionId,
        cx: &mut RuntimeContext<'_>,
    ) -> Result<(), Diagnostic>;
}
```

## 20. Raw specification

Illustrative JSON:

```json
{
  "specId": "assessment-view",
  "version": "1.0",
  "revision": 12,
  "root": "page",
  "nodes": {
    "page": {
      "type": "Stack",
      "props": { "gap": "large" },
      "children": ["summary", "findings"]
    },
    "summary": {
      "type": "RunSummary",
      "props": { "run": { "$resource": "run.current" } },
      "children": []
    },
    "findings": {
      "type": "FindingList",
      "props": { "items": { "$resource": "run.findings" } },
      "actions": ["finding.open"],
      "children": []
    }
  }
}
```

## 21. Catalogue contract

```rust
pub trait ComponentDefinition: Send + Sync + 'static {
    type Prepared: Send + Sync + 'static;

    fn descriptor(&self) -> ComponentDescriptor;

    fn prepare(
        &self,
        raw: &RawNode,
        cx: &mut PrepareContext<'_>,
    ) -> Result<Self::Prepared, Diagnostics>;

    fn semantics(
        &self,
        prepared: &Self::Prepared,
        cx: &SemanticContext<'_>,
    ) -> SemanticNode;
}

pub struct ComponentDescriptor {
    pub name: String,
    pub version: u32,
    pub prop_schema: Schema,
    pub allowed_actions: Vec<ActionName>,
    pub required_permissions: Vec<Permission>,
    pub media_policy: MediaPolicy,
    pub fallbacks: Vec<FallbackDescriptor>,
}
```

Renderer implementations are registered separately from semantic component definitions.

## 22. Renderer component contract

```rust
pub trait RendererComponent<R: Renderer>: Send + Sync {
    fn capabilities(&self) -> CapabilityRequirements;

    fn prepare_renderer_state(
        &self,
        node: &PreparedNode,
        cx: &mut R::PrepareContext<'_>,
    ) -> Result<R::NodeState, Diagnostics>;

    fn layout(
        &self,
        node: &PreparedNode,
        state: &mut R::NodeState,
        cx: &mut R::LayoutContext<'_>,
    ) -> R::LayoutNode;

    fn paint(
        &self,
        node: &PreparedNode,
        state: &R::NodeState,
        layout: &R::LayoutNode,
        cx: &mut R::PaintContext<'_>,
    );
}
```

Type erasure may be required for dynamic catalogues but should remain inside registry implementation.

## 23. Specification patches

```rust
pub struct SpecPatchEnvelope {
    pub spec_id: String,
    pub base_revision: u64,
    pub new_revision: u64,
    pub transaction_id: uuid::Uuid,
    pub source: PatchSource,
    pub trust: TrustClass,
    pub operations: Vec<SpecPatch>,
    pub provenance: Option<ProvenanceRef>,
}

pub enum SpecPatch {
    AddNode { node: RawNode },
    RemoveNode { id: NodeId },
    ReplaceNode { id: NodeId, node: RawNode },
    SetProp { id: NodeId, path: PropPath, value: RawValue },
    RemoveProp { id: NodeId, path: PropPath },
    InsertChild { parent: NodeId, index: usize, child: NodeId },
    RemoveChild { parent: NodeId, child: NodeId },
    MoveNode { id: NodeId, parent: NodeId, index: usize },
    SetVisibility { id: NodeId, expression: Option<Expression> },
    AttachAction { id: NodeId, action: ActionName },
    DetachAction { id: NodeId, action: ActionName },
}
```

## 24. Terminal profile

```rust
pub struct TerminalProfile {
    pub identity: TerminalIdentity,
    pub tty: TtyKind,
    pub cells: CellSize,
    pub pixels: Option<PixelSize>,
    pub cell_pixels: Option<PixelSize>,
    pub colour: TerminalColourProfile,
    pub appearance: Appearance,
    pub keyboard: KeyboardCapabilities,
    pub mouse: MouseCapabilities,
    pub hyperlinks: CapabilityState,
    pub synchronised_output: CapabilityState,
    pub graphics: GraphicsCapabilities,
    pub multiplexer: Option<MultiplexerProfile>,
    pub latency: LatencyProfile,
    pub quirks: Vec<TerminalQuirk>,
}
```

## 25. Colour model

```rust
pub struct OklchColour {
    pub lightness: f32,
    pub chroma: f32,
    pub hue_degrees: f32,
    pub alpha: f32,
}

pub enum ColourIntent {
    Fixed(OklchColour),
    Token(ColourToken),
    Derived {
        source: ColourToken,
        lightness_delta: f32,
        chroma_scale: f32,
    },
}

pub struct ThemeDefinition {
    pub name: String,
    pub dark: ThemeVariant,
    pub light: Option<ThemeVariant>,
    pub contrast_targets: ContrastTargets,
    pub no_colour: NoColourTheme,
}

pub struct ResolvedTerminalTheme {
    pub depth: ColourDepth,
    pub colours: std::collections::HashMap<ColourToken, ResolvedColour>,
    pub attributes: std::collections::HashMap<ColourToken, TextAttributes>,
    pub diagnostics: Vec<ThemeDiagnostic>,
}
```

## 26. Media model

```rust
pub struct MediaAsset {
    pub id: MediaAssetId,
    pub source: MediaSource,
    pub kind: MediaKind,
    pub intrinsic_size: Option<PixelSize>,
    pub colour_space: Option<ColourSpace>,
    pub content_hash: Option<ContentHash>,
    pub accessibility: MediaAccessibility,
    pub trust: TrustClass,
}

pub struct MediaPlacement {
    pub id: MediaPlacementId,
    pub asset: MediaAssetId,
    pub fit: MediaFit,
    pub focal_point: Option<NormalisedPoint>,
    pub crop: Option<NormalisedRect>,
    pub opacity: f32,
    pub z_index: i32,
    pub role: MediaRole,
    pub fallback: FallbackPolicy,
}

pub enum MediaSource {
    Embedded { bytes: std::sync::Arc<[u8]> },
    ApprovedLocalFile { capability: FileCapability, path: std::path::PathBuf },
    ContentAddressed { hash: ContentHash },
    CommandArtifact { artifact: ArtifactRef },
    ApprovedRemote { capability: NetworkCapability, url: String },
    GeneratedDiagram { source: DiagramSource },
}
```

## 27. Media accessibility

```rust
pub struct MediaAccessibility {
    pub purpose: MediaPurpose,
    pub alt_text: Option<String>,
    pub long_description: Option<String>,
    pub structured_alternative: Option<SemanticDocumentRef>,
}

pub enum MediaPurpose {
    Meaningful,
    Decorative,
    Evidence,
    Diagram,
    Chart,
    Identity,
}
```

## 28. Diagnostic model

```rust
pub struct Diagnostic {
    pub code: String,
    pub severity: DiagnosticSeverity,
    pub summary: String,
    pub detail: Option<String>,
    pub source: Option<DiagnosticSource>,
    pub labels: Vec<DiagnosticLabel>,
    pub related: Vec<DiagnosticRef>,
    pub help: Option<String>,
    pub actions: Vec<ActionDescriptor>,
    pub security_class: SecurityClass,
}
```

## 29. Headless renderer

```rust
pub trait HeadlessRenderer {
    fn render_semantics(&self, root: &SemanticNode) -> SemanticSnapshot;
    fn render_text(&self, root: &SemanticNode, options: TextRenderOptions) -> String;
    fn render_json(&self, root: &SemanticNode) -> serde_json::Value;
}
```

The headless renderer is not merely a test helper. It is a required expression for automation, accessibility and agents.

## 30. Terminal renderer mode selection

```rust
pub enum TerminalMode {
    Plain,
    Inline,
    Workspace,
    Remote,
}

pub struct ModePolicy {
    pub requested: Option<TerminalMode>,
    pub allow_workspace: bool,
    pub prefer_inline: bool,
    pub non_interactive: NonInteractivePolicy,
}

pub fn choose_mode(
    profile: &TerminalProfile,
    policy: &ModePolicy,
    command: &CommandMetadata,
) -> TerminalMode;
```

## 31. Ratatui compatibility sketch

```rust
#[cfg(feature = "ratatui-compat")]
pub struct RatatuiHost<State> {
    pub id: NodeId,
    pub state: State,
    pub semantics: fn(&State) -> SemanticNode,
    pub render: fn(&mut State, &mut ratatui::Frame<'_>, ratatui::layout::Rect),
    pub action: fn(&mut State, &ActionEnvelopeErased) -> ActionResult,
}
```

The host lives in a terminal compatibility crate. Core APIs never mention it.

## 32. Example north-star command

```rust
#[derive(CommandInput)]
pub struct AssessInput {
    #[source(cli, prompt, config, default = ".")]
    pub path: std::path::PathBuf,

    #[source(cli, env = "POLICY_SET", config, prompt)]
    pub policy: PolicySetId,
}

pub struct Assess;

impl Command for Assess {
    type Input = AssessInput;
    type Output = AssessmentResult;
    type Event = AssessmentEvent;
    type Error = AssessmentError;

    const NAME: &'static str = "assess";
    const VERSION: u32 = 1;

    fn metadata() -> CommandMetadata {
        CommandMetadata {
            summary: "Assess a repository against a policy set",
            description: "Runs checks, gathers evidence and produces findings",
            risk: Risk::ReadOnly,
            permissions: &[Permission::ReadWorkspace],
            supports_preview: false,
            supports_cancel: true,
            retry: RetryPolicy::UserInitiated,
            compensation: CompensationPolicy::None,
        }
    }

    // validate, preview and execute omitted
}
```

## 33. Example Flow projection

```rust
fn assessment_flow(run: Entity<CommandRun>, cx: &SemanticContext<'_>) -> FlowDocument {
    FlowDocument::builder()
        .node(run_summary(run.clone(), cx))
        .node(progress_block(run.clone(), cx))
        .nodes(finding_blocks(run, cx))
        .build()
}
```

## 34. Example promotion

```rust
runtime.dispatch(
    finding_entity,
    PromoteToWorkspace {
        node: finding_node_id,
        preferred_region: Some(RegionIntent::Primary),
    },
)?;
```

The same action may be invoked by:

- Enter in the terminal;
- clicking a web card;
- a native command palette;
- an agent with permission;
- an automated test.

## 35. Example generated specification patch

```json
{
  "specId": "assessment-view",
  "baseRevision": 12,
  "newRevision": 13,
  "transactionId": "52ec5b50-fbca-4aa5-81dc-45386a14d650",
  "source": {
    "kind": "agent",
    "id": "analysis-agent"
  },
  "trust": "agent-generated",
  "operations": [
    {
      "op": "addNode",
      "node": {
        "id": "finding-F-214",
        "type": "FindingCard",
        "props": {
          "finding": { "$resource": "findings.F-214" }
        },
        "actions": ["finding.open", "evidence.open"]
      }
    },
    {
      "op": "insertChild",
      "parent": "findings",
      "index": 0,
      "child": "finding-F-214"
    }
  ]
}
```

The runtime validates that the source may create `FindingCard` and attach only the declared actions.

## 36. Semantic snapshot example

```json
{
  "id": "finding-F-214",
  "role": "finding",
  "name": "Unreviewed model-generated migration",
  "state": {
    "severity": "high",
    "expanded": false
  },
  "value": {
    "evidenceCount": 4,
    "remediationAvailable": true
  },
  "actions": [
    "finding.open",
    "evidence.open",
    "remediation.request"
  ]
}
```

Semantic snapshots should be stable across terminal, web and native renderers.

## 37. API design guidelines

When converting these sketches into real APIs:

1. Prefer explicit typed state over generic property bags.
2. Keep renderer types at renderer boundaries.
3. Make lifecycle and ownership visible in method names and types.
4. Avoid requiring application authors to understand internal type erasure.
5. Preserve good compiler errors.
6. Use derive macros only where they remove repetition without hiding semantics.
7. Support dynamic catalogue entries without making static Rust components pay unnecessary runtime cost.
8. Separate stable IDs from display labels.
9. Make security-relevant operations explicit.
10. Provide low-level escape hatches in adapter crates rather than weakening the core.

## 38. API validation checklist

Before stabilising an API, verify:

- Is it used by the greenfield reference app?
- Is it used by the terminal renderer?
- Is it used or validated by the sibling web renderer?
- Does it work headlessly?
- Does it preserve stable identity?
- Is task ownership obvious?
- Can it be inspected and replayed?
- Does it have a trust and permission story?
- Can current Anvil/`eddacraft-tui` integrate through an adapter?
- Is the error message understandable when used incorrectly?
