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
