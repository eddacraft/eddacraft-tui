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
