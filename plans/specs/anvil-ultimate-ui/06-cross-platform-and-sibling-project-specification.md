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
