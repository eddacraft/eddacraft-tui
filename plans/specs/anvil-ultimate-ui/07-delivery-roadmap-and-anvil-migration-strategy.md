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
