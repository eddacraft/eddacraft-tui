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
