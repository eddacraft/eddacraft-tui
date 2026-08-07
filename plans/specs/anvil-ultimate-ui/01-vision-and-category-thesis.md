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
