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
