# APS and anvil Converged Application — Complete Planning Document

## Status

Draft product, requirements, and architecture pack for converging APS and anvil into a shared native, web, tray, CLI, and TUI application platform.

## Contents

1. App-layer requirements
2. Domain, command, event, and projection model
3. Surface and experience architecture
4. Convergence and migration plan
5. Decisions, risks, and open questions

---

# Part I — App-Layer Requirements

# Converged APS and anvil App-Layer Requirements

## Document status

- **Status:** Draft for architecture and product planning
- **Scope:** App layer for a converged APS and anvil product
- **Primary surfaces:** Native desktop application, web application, system tray, CLI, and TUI
- **Runtime centre:** Local anvil daemon
- **Planning model:** APS
- **Governance and evidence model:** anvil
- **Architecture posture:** Local-first, modular, capability-driven, and progressively convergent

---

## 1. Purpose

This document defines the app-layer requirements for converging APS and anvil into one coherent product family.

The target is not an APS Kanban application bolted onto anvil. The target is a shared application platform in which:

- APS defines, structures, and authorises intended work;
- anvil governs, observes, verifies, and records executed work;
- the daemon owns durable local application behaviour and runtime state;
- the native application is the primary rich human control plane;
- the web application exposes the same product model through an appropriate browser surface;
- the tray provides ambient status, attention, and fast controls;
- the CLI and TUI remain first-class interfaces over the same application layer;
- standalone APS continues to be published as an open, independently useful distribution.

The immediate APS need should be delivered as an early module of this architecture, not as a separate application that later requires a second convergence effort.

---

## 2. Product thesis

> **APS defines intended work. anvil governs and verifies its execution. The daemon provides the shared local control plane. Native, web, tray, CLI, and TUI surfaces let people and agents plan, supervise, approve, inspect, and automate the lifecycle.**

The core lifecycle is:

```text
Intent
  → specification
    → authorised work
      → workspace
        → run
          → sessions and actions
            → evidence
              → verification and decision
                → completion, revision, or rollback
```

The product must support plan-led work without requiring every anvil capability to begin with a plan. APS is an accelerant and source of authority when present; anvil must remain useful for planless observation, scanning, policy, and evidence capture.

---

## 3. Goals

### G-001: One application model

Provide a single app-layer model that can support APS and anvil without duplicating domain behaviour in each product or surface.

### G-002: One command path

All meaningful state changes must pass through a uniform command layer, regardless of whether they originate from native UI, web, tray, CLI, TUI, MCP, automation, or an agent.

### G-003: Shared projections

All surfaces must consume purpose-built projections of current state rather than reimplementing planning, workspace, policy, or evidence logic independently.

### G-004: Independent but convergable delivery

APS planning capabilities, workspace supervision, and anvil governance capabilities must be independently deliverable behind capabilities or feature flags while sharing the same kernel, command contracts, events, module system, and design language.

### G-005: Local-first operation

The product must remain fully useful with a local daemon and local repository state. Hosted services may extend the experience but must not be required for core planning, governance, execution, or evidence workflows.

### G-006: Durable work beyond the UI

Closing a window, hiding the native application, changing tabs, or disconnecting a browser must not terminate daemon-owned work.

### G-007: Standalone APS remains genuine

APS must remain independently installable and usable through its CLI and TUI. The converged monorepo must not make public APS depend on proprietary anvil modules.

### G-008: Future remote operation

The architecture must permit a future desktop or web client to connect to a daemon running elsewhere, without making remote operation a requirement for the first release.

---

## 4. Non-goals

The initial converged app layer is not required to:

- replace every existing CLI and TUI workflow at once;
- provide real-time multi-user collaboration in the first release;
- require a hosted account or central cloud control plane;
- embed a full IDE or compete with established code editors;
- make every plan item executable by an agent;
- make the Kanban board the source of planning truth;
- store critical execution state solely in the desktop application;
- unify every existing APS and anvil persistence mechanism in one migration;
- expose every daemon capability to the web surface;
- commit to Dioxus, Tauri, React, or another UI framework before the framework spike is completed.

---

## 5. North-star system context

```text
┌───────────────────────────────────────────────────────────────────────┐
│ Product surfaces                                                      │
│                                                                       │
│ Native desktop   Web client   Tray   CLI   TUI   MCP   Automations    │
└───────────────────────────────┬───────────────────────────────────────┘
                                │ commands, queries, subscriptions
┌───────────────────────────────▼───────────────────────────────────────┐
│ Shared application layer                                             │
│                                                                       │
│ command bus · query services · projections · capabilities · modules  │
│ notifications · preferences · identity · correlation · API contracts │
└───────────────────────────────┬───────────────────────────────────────┘
                                │
┌───────────────────────────────▼───────────────────────────────────────┐
│ anvil daemon                                                          │
│                                                                       │
│ APS · workspaces · runs · agents · policy · evidence · verification  │
│ process supervision · repositories · persistence · event stream       │
└───────────────────────────────┬───────────────────────────────────────┘
                                │
┌───────────────────────────────▼───────────────────────────────────────┐
│ Local and connected systems                                          │
│                                                                       │
│ Git · filesystem · terminals · agent harnesses · CI · issue systems  │
│ policy engines · telemetry · hosted control plane · customer systems │
└───────────────────────────────────────────────────────────────────────┘
```

The desktop application must be a client of the daemon, not a second application host containing separate domain behaviour.

---

## 6. Concentric capability circles

### Circle 1: Shared kernel

The shared kernel must define the cross-product primitives used by APS and anvil:

- stable identifiers;
- actors and identities;
- command and event envelopes;
- correlation and causation;
- optimistic concurrency and aggregate versions;
- timestamps and clocks;
- capability identifiers;
- module registration;
- error contracts;
- provenance references;
- serialisation and contract versioning.

The kernel must not depend on UI frameworks, filesystem adapters, daemon transports, or proprietary product modules.

### Circle 2: Shared platform services

The shared platform layer must provide:

- command dispatch;
- event publication and subscription;
- query handling;
- projection refresh;
- repository discovery;
- workspace lifecycle support;
- process and agent-session supervision;
- notifications and attention routing;
- configuration and preferences;
- audit and observability;
- capability and feature evaluation;
- module loading and contribution registration.

### Circle 3: Domain modules

The application must support separately owned domain modules, including:

- APS planning;
- workspace and repository execution;
- agent and runtime integration;
- anvil governance and policy;
- evidence and verification;
- delivery and pull-request state;
- system health and configuration.

Modules must integrate through commands, events, and public application services rather than direct access to one another’s storage tables.

### Circle 4: Product surfaces

Native, web, tray, CLI, TUI, MCP, and automation surfaces must use the same app-layer contracts while applying surface-appropriate interaction patterns.

---

## 7. Core domain distinctions

The app layer must preserve the following distinctions.

| Concept | Meaning | Must not be conflated with |
| --- | --- | --- |
| **Project** | A logical product or delivery context spanning one or more repositories | A physical checkout or worktree |
| **Repository** | A version-controlled source location | A project or workspace |
| **Plan** | Durable intent and initiative-level context | A backlog or run history |
| **Module** | A bounded area of planned capability | A process or agent session |
| **Work item** | A bounded unit of authorised change | A UI card, chat, or terminal |
| **Decision** | A durable resolution that constrains work | A transient message or approval click |
| **Workspace** | An execution environment used to attempt work | The work item itself |
| **Run** | A governed execution attempt | A workspace, chat thread, or process |
| **Session** | A durable interaction with an agent, human, terminal, or tool during work | A visible tab |
| **Tab** | A presentation view onto a domain object or tool surface | A run, work item, or session |
| **Evidence** | A durable, attributable observation supporting a claim or decision | A log line with no context |
| **Gate decision** | A governed judgement over evidence and policy | Raw check output |
| **Attention item** | A projected reason requiring human or agent attention | The source domain object |

The canonical relationship is:

```text
APS work item
  → zero or more workspace attempts
    → zero or more governed runs
      → one or more participating sessions
        → zero or more visible tabs and views
```

---

## 8. Application shell requirements

### APP-001: Unified shell

The native and web applications must expose a consistent top-level product structure while allowing surface-specific differences.

The default navigation model should support:

```text
Inbox
Projects
Workspaces
Runs
Evidence
System
```

Only capabilities available to the current installation and actor should be shown.

### APP-002: Module registration

Domain modules must be able to register:

- routes or views;
- navigation entries;
- command-palette actions;
- tab types;
- detail panels;
- context-menu actions;
- notifications;
- capability requirements;
- settings sections.

Module registration must not require the application shell to contain APS- or anvil-specific branching throughout its code.

### APP-003: Capability-driven composition

The shell must compose itself from declared capabilities rather than product-name checks such as `is_aps` or `is_anvil_enterprise`.

Capabilities may include:

```text
planning.read
planning.write
planning.authorise
workspace.read
workspace.manage
runs.execute
governance.evaluate
approvals.respond
evidence.read
evidence.submit
system.admin
```

### APP-004: Feature flags

Feature flags must control staged rollout and experimentation. They must not replace the durable capability model.

Feature flags must be evaluable by:

- build profile;
- local configuration;
- licence or entitlement;
- server or hosted policy;
- actor role;
- experimental channel.

### APP-005: Command palette

The native and web shells must provide a searchable command palette that:

- exposes commands permitted by the current context and capabilities;
- displays keyboard shortcuts;
- scopes commands by project, workspace, work item, or selection;
- issues the same app-layer commands as visible UI actions;
- does not bypass policy or confirmation requirements.

### APP-006: Deep links

Each durable object should have a stable application route or deep link where practical, including:

- project;
- plan;
- module;
- work item;
- workspace;
- run;
- session;
- evidence bundle;
- gate decision;
- attention item.

Deep links must be portable between native and web surfaces where the target capability exists.

---

## 9. Daemon and lifecycle requirements

### DAE-001: Authoritative local runtime

The daemon must be the authoritative local runtime for:

- command execution;
- durable domain state;
- workspace and process supervision;
- agent-session lifecycle;
- policy evaluation;
- evidence capture;
- projection generation;
- event subscriptions;
- system health.

### DAE-002: Independent lifecycle

The daemon must be able to continue running when:

- the desktop window is hidden;
- the desktop application exits, subject to user configuration;
- the browser disconnects;
- the tray window is closed;
- a client reconnects after failure.

### DAE-003: Single-instance coordination

The native application must detect and coordinate with the existing daemon instance rather than starting competing daemon processes.

### DAE-004: Health and compatibility

Clients must be able to query:

- daemon health;
- daemon version;
- contract version;
- enabled modules;
- capabilities;
- storage migrations;
- active jobs;
- degraded conditions.

A client must fail safely and explain compatibility problems when it cannot communicate with the daemon version in use.

### DAE-005: Resilient reconnection

Clients must support:

- daemon unavailable at startup;
- daemon restart;
- network or IPC interruption;
- projection refresh after reconnect;
- event resumption or safe re-query;
- duplicate event suppression.

### DAE-006: Transport abstraction

The app layer must isolate transport from application contracts so that the same commands and queries can be carried over:

- local IPC;
- loopback HTTP;
- server-sent events or WebSocket;
- future authenticated remote connections.

---

## 10. Command, query, event, and projection requirements

### CQEP-001: Command-only mutation

Every meaningful domain state change must be represented as a typed command.

UI components must not directly update domain persistence.

### CQEP-002: Uniform command envelope

Commands must include, at minimum:

- command identifier;
- command type and version;
- actor;
- target aggregate or context;
- payload;
- correlation identifier;
- causation identifier where applicable;
- expected aggregate version where applicable;
- requested timestamp;
- source surface;
- idempotency key for retryable operations.

### CQEP-003: Structured outcomes

Command responses must distinguish:

- accepted and completed;
- accepted and running asynchronously;
- rejected by validation;
- rejected by policy;
- conflicted by version;
- unavailable due to capability or transport;
- failed with a structured error.

### CQEP-004: Events as facts

Domain events must describe accepted facts rather than UI instructions.

Examples:

```text
planning.work_item_authorised
planning.work_item_started
workspace.created
run.started
session.output_appended
evidence.recorded
gate.decision_recorded
attention.created
```

### CQEP-005: Purpose-built projections

The app layer must provide projections including:

- project catalogue;
- operational inbox;
- planning board;
- plan hierarchy;
- workspace overview;
- run timeline;
- session status and output;
- evidence summary;
- gate and approval state;
- system health.

### CQEP-006: Projection refresh model

Events should notify clients that state changed. Queries and projections should describe the current state.

Clients must not need to replay an unbounded event history to reconstruct normal UI state.

### CQEP-007: Contract versioning

Commands, events, errors, and projection schemas must be versioned independently of the application release version.

Breaking changes must have explicit migrations or compatibility adapters.

---

## 11. APS planning requirements

### APS-APP-001: Canonical planning model

The application must expose APS concepts as first-class domain objects:

- index or initiative;
- module;
- work item;
- decision;
- issue or open question;
- action plan;
- dependency;
- validation requirement;
- learning;
- status and readiness.

### APS-APP-002: Markdown remains authoritative where configured

For repository-managed APS projects, Markdown files must remain the durable source format.

The daemon may maintain indexes, caches, projections, or supplemental application state, but it must not silently diverge from the repository documents.

### APS-APP-003: Safe document mutation

Commands that update APS documents must:

- preserve unrelated content and formatting where practical;
- validate expected document version or content hash;
- surface conflicts rather than overwrite concurrent edits;
- record actor and command provenance;
- support preview where the change is material;
- produce deterministic output.

### APS-APP-004: Planning views

The application must support at least:

- plan overview;
- module and work-item hierarchy;
- Kanban projection;
- dependency graph;
- decisions and issues;
- readiness and validation warnings;
- completion history;
- relevant context package.

### APS-APP-005: Board is a projection

The Kanban board must be a projection of APS and execution state, not a separate task database.

Dragging a card must issue a transition command. Invalid transitions must be rejected with an explanation.

### APS-APP-006: Recommended lanes

The initial board should support configurable mapping onto at least:

```text
Shaping
Decision needed
Awaiting approval
Ready
Running
Verification
Blocked
Done
```

### APS-APP-007: Plan scope and operational scope

The plan explorer and board must show durable work structure. The operational sidebar and inbox must show current activity and required attention.

The two views must not be collapsed into one overloaded hierarchy.

### APS-APP-008: Agent-readable planning API

Agents must consume APS through typed queries or MCP/CLI commands rather than scraping the visual board.

Required operations should include:

```text
list eligible work
inspect work context
claim or start work
propose a transition
submit completion evidence
record a learning
release or abandon work
```

### APS-APP-009: Standalone parity

Every core planning mutation exposed by the desktop must either:

- be available through the shared APS application layer to CLI/TUI; or
- be explicitly classified as an integrated anvil capability rather than standalone APS behaviour.

---

## 12. Workspace requirements

### WSP-001: Workspace as delivery envelope

A workspace must represent an environment used to progress a body of work. It may contain:

- one or more repositories;
- one or more worktrees or checkouts;
- branches;
- environment configuration;
- participating humans and agents;
- runs and sessions;
- tabs and layout preferences;
- changes and diffs;
- evidence and verification state;
- delivery and pull-request links.

### WSP-002: Logical project grouping

Multiple physical checkouts, worktrees, or remote environments of the same repository must be groupable beneath one logical project.

### WSP-003: Multi-repository workspaces

The domain model must permit a workspace to span multiple repositories where one outcome crosses repository boundaries.

The first release may limit this capability, but the model must not assume one workspace always equals one repository.

### WSP-004: Workspace attempts

A work item may have:

- no workspace while still being shaped;
- one active workspace;
- multiple historical workspace attempts;
- deliberately parallel alternative workspaces when explicitly authorised.

### WSP-005: Isolation policy

Workspace creation must support explicit isolation modes, including:

- shared current checkout;
- isolated Git worktree;
- remote environment;
- externally managed environment.

The application must display the isolation mode and enforce policy around unsafe shared execution.

### WSP-006: Lifecycle

Workspace lifecycle commands must include:

```text
create
open
attach repository
create worktree
start work
pause
resume
archive
abandon
restore
```

### WSP-007: Archive rather than destroy by default

Archiving a workspace must preserve attributable history, links, evidence, and decisions even when temporary execution resources are removed.

---

## 13. Run and session requirements

### RUN-001: Run as governed attempt

A run must represent a specific attempt to perform or verify work under a defined context, policy, actor, and set of capabilities.

### RUN-002: Multiple sessions per run

A run may include multiple sessions, such as:

- implementation agent;
- reviewer agent;
- verifier;
- human terminal;
- background tool;
- CI or external automation.

### RUN-003: Durable session identity

A session must survive process reconnects or UI closure where the underlying runtime permits it.

The durable session record must be distinct from the currently connected provider process.

### RUN-004: Session renderers

The same session may be rendered through multiple views, including:

- structured conversation;
- raw terminal;
- event stream;
- tool calls;
- evidence produced;
- policy decisions.

### RUN-005: Process ownership

The daemon must own supervised local processes. UI tabs must not be process owners.

Closing a tab must never implicitly stop a session unless the user explicitly chooses a stop action.

### RUN-006: Session control

Permitted actors must be able to:

- start;
- send input;
- pause where supported;
- resume;
- interrupt;
- stop;
- restart;
- detach;
- reattach;
- mark failed or abandoned.

### RUN-007: Provider-neutral adapters

Agent and terminal sessions must use provider-neutral app-layer contracts with adapters for harnesses such as Codex, Claude Code, OpenCode, Copilot, Hermes, Pi, and future runtimes.

---

## 14. anvil governance and evidence requirements

### GOV-001: Policy evaluation

The app layer must support policy evaluation before, during, and after execution where configured.

### GOV-002: Approvals

Approval requests must be first-class objects containing:

- requested action;
- requesting actor;
- reason;
- affected scope;
- policy basis;
- expiry;
- permitted responses;
- resulting decision and actor.

### GOV-003: Evidence capture

Evidence must be attributable and linked to:

- project;
- work item where applicable;
- workspace;
- run;
- session;
- actor;
- command or event correlation;
- claim or verification target;
- timestamp and source.

### GOV-004: Evidence types

The model must support evidence from:

- validation commands;
- tests and coverage;
- source and diff analysis;
- policy engines;
- CI systems;
- runtime logs;
- external systems;
- human attestations;
- screenshots or artefacts;
- agent reports.

### GOV-005: Gate decisions

A gate decision must record:

- inputs considered;
- policies and rules applied;
- evidence references;
- result;
- severity;
- conditions or exceptions;
- actor or engine;
- version and timestamp.

### GOV-006: Intent verification

The application must support verifying whether the implementation satisfies the authorised APS outcome, not merely whether checks passed.

### GOV-007: Planless governance

anvil governance, evidence, and scanning must remain usable when no APS work item is present.

### GOV-008: Provenance continuity

The application must preserve provenance across:

```text
command
→ run
→ session
→ action
→ change
→ evidence
→ decision
```

### GOV-009: Rollback and incident links

Where anvil supports rollback or incident workflows, the app layer must link them to the originating run, workspace, evidence, and gate decisions.

---

## 15. Inbox and attention requirements

### ATN-001: Unified operational inbox

The application must provide a flat operational inbox answering:

```text
What is happening?
What needs attention?
```

### ATN-002: Attention sources

Attention items may be projected from:

- approval requested;
- policy failure;
- verification failure;
- agent waiting for input;
- daemon or workspace failure;
- merge conflict;
- stale active work;
- unresolved decision;
- missing validation;
- new comment or external review;
- completed work awaiting review;
- expiring credential or permission.

### ATN-003: Active and settled density

Active, unread, blocked, and attention-requiring items should use richer rows. Settled or inactive items should collapse to compact rows without losing accessibility.

### ATN-004: Filtering

The inbox must support filtering by:

- logical project;
- status;
- attention reason;
- environment;
- provider or agent;
- actor;
- recent period;
- unread;
- settled or archived state.

### ATN-005: Reactivation

Settled items must reactivate when new meaningful activity occurs.

### ATN-006: Resolution

Resolving or dismissing an attention item must not mutate the underlying domain object unless the associated domain command is also executed.

---

## 16. Native desktop requirements

### NAT-001: Primary rich experience

The native desktop application must be the richest local human control surface.

### NAT-002: Window modes

The native application should support:

- full application window;
- compact or quick-status window;
- hide to tray;
- restore from tray or deep link;
- multiple windows or detached tabs where justified by the selected framework.

### NAT-003: Workspace tabs

The native application must support typed, persistent tabs for:

- overview;
- APS context;
- agent session;
- terminal;
- diff;
- evidence;
- verification;
- preview;
- system or diagnostic views.

### NAT-004: Tab behaviour

Tabs must support:

- open;
- close;
- pin;
- reorder;
- reopen;
- restore after restart;
- status badges;
- unsaved presentation state where applicable;
- the same session shown through multiple view types.

### NAT-005: Split panes

The native application must support at least:

- single pane;
- two-column split;
- primary pane with bottom terminal.

Layout state must be serialisable and restorable.

### NAT-006: Terminal integration

The native app must provide a credible terminal/session surface supporting:

- streamed ANSI output;
- keyboard input;
- resizing;
- selection and copying;
- large scrollback;
- long-running sessions;
- detachment and reattachment;
- background continuation.

### NAT-007: Diff and change review

The native app must provide a usable diff surface with:

- changed-file navigation;
- added, removed, and modified lines;
- syntax highlighting;
- large diff handling;
- links to evidence, plan authority, and review comments.

### NAT-008: Native system integration

Subject to platform support, the native app should integrate with:

- notifications;
- global shortcuts;
- protocol handlers and deep links;
- clipboard;
- file and folder reveal;
- native menus;
- application updates;
- crash reporting under user control.

### NAT-009: Cross-platform targets

The target platforms are:

- Windows;
- macOS;
- Linux, initially Ubuntu GNOME on Wayland and X11 where practical.

Platform limitations must be explicit rather than hidden behind inconsistent behaviour.

---

## 17. Web application requirements

### WEB-001: Shared product model

The web application must consume the same command, query, capability, and projection contracts as native clients.

### WEB-002: Appropriate web scope

The web application need not expose unsafe local capabilities by default. Capabilities should depend on the connected daemon or hosted control plane.

### WEB-003: Browser reconnect and continuity

Browser refresh or reconnect must restore durable navigation, selected project, and workspace views where the user still has access.

### WEB-004: Responsive experience

The web application must support desktop-class browser layouts first and degrade sensibly to narrower widths for review, approvals, and status workflows.

### WEB-005: No arbitrary local access

A browser client must never gain local filesystem, process, terminal, or daemon access merely because it can reach a page. Local access must be mediated by authenticated, capability-constrained APIs.

### WEB-006: Hosted future

The contracts must permit a future hosted control plane to provide organisation-wide projects, evidence, policy, and remote execution state without changing the core domain names.

---

## 18. Tray requirements

### TRY-001: Ambient status

The tray must display or make available:

- daemon health;
- active run count;
- active workspace count;
- attention-required count;
- paused or restricted state;
- current version or update state where useful.

### TRY-002: Fast actions

The tray should provide:

- open application;
- open inbox;
- open the most urgent item;
- pause or resume permitted execution;
- start or reconnect daemon where appropriate;
- open logs or diagnostics;
- quit the client;
- stop the daemon only through an explicit separate action.

### TRY-003: Non-authoritative presentation

The tray must consume the same projections and commands as other clients. It must not maintain a separate state machine.

### TRY-004: Platform-aware behaviour

Tray features must degrade gracefully where Linux desktop environments or other platforms do not support identical behaviour.

### TRY-005: Notifications

The tray/native notification layer should notify on high-value transitions such as:

- approval required;
- agent waiting;
- verification failed;
- run completed;
- daemon degraded;
- policy blocked an action.

Users must be able to tune notification categories and quiet periods.

---

## 19. CLI and TUI requirements

### CLI-001: First-class surfaces

CLI and TUI must remain first-class clients, not compatibility wrappers around desktop-specific behaviour.

### CLI-002: Shared application commands

Core operations must invoke the same application-layer command handlers used by native and web surfaces.

### CLI-003: Headless operation

All essential automation workflows must be possible without launching a graphical application.

### CLI-004: Structured output

Commands intended for automation must support stable machine-readable output and structured errors.

### CLI-005: TUI continuity

The TUI should share domain projections, interaction concepts, colours, semantic statuses, and navigation language with the native application where terminal constraints permit.

### CLI-006: Daemon modes

Commands must make clear whether they:

- operate directly on standalone APS files;
- use the local daemon;
- connect to a remote control plane;
- require anvil-only capabilities.

---

## 20. Shared component and design-system requirements

### DS-001: Semantic component system

The product must maintain a shared semantic design language across native, web, and TUI surfaces, even where implementation technologies differ.

### DS-002: Core components

The graphical design system must provide at least:

- button and icon button;
- text input and search;
- badge and status indicator;
- tooltip;
- menu and context menu;
- popover;
- dialog;
- toast and notification;
- tabs;
- card and list row;
- scroll area;
- resizable pane;
- command palette;
- tree and graph affordances;
- empty, loading, offline, degraded, and error states.

### DS-003: Domain components

Shared domain-level components should include:

- project identity;
- status and capability badges;
- work-item card;
- active and settled operational rows;
- run status;
- session status;
- evidence summary;
- gate decision;
- approval prompt;
- actor and agent identity;
- provenance trail;
- diff and validation result.

### DS-004: Theme and density

The graphical applications must support:

- light and dark appearance;
- system appearance;
- dense and comfortable layouts where appropriate;
- scalable text;
- persistent user preferences.

### DS-005: Technology isolation

Domain and application crates must not depend on a particular UI component framework.

---

## 21. State and persistence requirements

### STA-001: State ownership

State must be classified as:

- repository-authoritative domain state;
- daemon-authoritative local state;
- external-system state;
- durable user preference;
- ephemeral UI state.

### STA-002: Repository-authoritative examples

Repository-authoritative state includes, where configured:

- APS Markdown;
- source code;
- repository decisions and documentation;
- versioned policy and configuration.

### STA-003: Daemon-authoritative examples

Daemon-authoritative state may include:

- active workspaces;
- runs and sessions;
- local process state;
- evidence indexes;
- projections and caches;
- notifications and attention state;
- local approvals and audit data;
- connection metadata.

### STA-004: User-preference examples

Durable preferences may include:

- theme;
- density;
- sidebar filters;
- selected project scope;
- tab layout;
- notification settings;
- default agent or provider preferences.

### STA-005: UI-only state

Ephemeral state may include:

- hovered card;
- current drag preview;
- open transient menu;
- unsent filter text;
- temporary panel dimensions before persistence.

### STA-006: No silent divergence

When repository state and daemon projections diverge, the application must detect, explain, and reconcile the condition. It must not silently treat a stale projection as truth.

### STA-007: Migration discipline

Persistent schemas must use versioned migrations and must support safe backup and recovery.

---

## 22. Security requirements

### SEC-001: Local daemon authentication

Clients must authenticate to the daemon, even over loopback or local IPC, using an installation-appropriate mechanism.

### SEC-002: Capability enforcement server-side

The daemon must enforce capabilities and policy. Hiding a control in the UI is not authorisation.

### SEC-003: Least privilege

Each command must request only the capability and scope needed for that action.

### SEC-004: Untrusted repository content

The application must treat repository files, Markdown, terminal output, diffs, logs, and agent-generated content as untrusted input.

### SEC-005: Rendering safety

Markdown, ANSI, HTML-like output, links, images, and embedded content must be sanitised or constrained according to the surface.

### SEC-006: Process execution

Process spawning must pass through explicit app-layer commands, policy evaluation, working-directory controls, environment filtering, and auditable attribution.

### SEC-007: WebView restrictions

A native WebView must not navigate to arbitrary remote content or expose privileged bindings to untrusted pages.

### SEC-008: Secrets

Secrets must not be copied into APS documents, logs, evidence, UI state, crash reports, or event streams. Secret references and leases should be used where possible.

### SEC-009: Approval integrity

Approvals must be bound to the exact action, scope, version, actor, and expiry that were reviewed.

### SEC-010: Auditability

Security-sensitive commands and decisions must produce attributable audit records.

---

## 23. Reliability and performance requirements

### REL-001: Graceful degradation

The application must provide useful read-only or diagnostic behaviour when one module, adapter, provider, or external system is unavailable.

### REL-002: Idempotent retries

Retryable commands must use idempotency keys and safe conflict handling.

### REL-003: Large project support

The app layer must be designed for:

- hundreds of projects or repositories;
- thousands of plan items;
- hundreds of active or settled operational items;
- large session output streams;
- large diffs;
- many historical evidence records.

### REL-004: Virtualised rendering

Graphical surfaces must virtualise or incrementally render large lists, boards, logs, and diffs where required.

### REL-005: Backpressure

Event subscriptions must handle bursty terminal and agent output without freezing the application or exhausting memory.

### REL-006: Startup

The native application should become navigable before every secondary projection has fully loaded.

### REL-007: Offline operation

Local planning and available daemon capabilities must continue without internet access.

### REL-008: Recovery

Unexpected client termination must not corrupt daemon state or repository documents.

---

## 24. Accessibility requirements

### A11Y-001: Keyboard-complete operation

Core native and web workflows must be operable without a pointer.

### A11Y-002: Focus management

Dialogs, menus, tabs, split panes, boards, and command palettes must have visible and predictable focus behaviour.

### A11Y-003: Semantic status

Status must never be communicated by colour alone.

### A11Y-004: Screen-reader support

Interactive controls and meaningful status changes must expose appropriate semantic labels and announcements.

### A11Y-005: Reduced motion

Animations must respect reduced-motion preferences.

### A11Y-006: Text and contrast

The interface must support scalable text and acceptable contrast in light and dark themes.

### A11Y-007: Dense views

Dense operational layouts must remain understandable and keyboard-navigable.

---

## 25. Observability requirements

### OBS-001: Correlation

Commands, events, daemon operations, runs, sessions, evidence, and errors must carry correlation identifiers.

### OBS-002: User-visible diagnostics

The System area must expose:

- daemon health;
- connection state;
- module status;
- adapter status;
- recent errors;
- active jobs;
- storage and migration state;
- exportable diagnostic bundle.

### OBS-003: Privacy-aware telemetry

Any optional telemetry must be transparent, configurable, minimal, and must not collect repository content, plans, code, secrets, or terminal output without explicit consent.

### OBS-004: Support bundle

Users must be able to create a redacted support bundle containing relevant versions, health, configuration metadata, and logs.

---

## 26. Extension and integration requirements

### EXT-001: Adapter model

External integrations must use adapters behind stable ports for:

- Git providers;
- issue trackers;
- agent runtimes;
- policy engines;
- CI systems;
- evidence sources;
- hosted control planes;
- notifications.

### EXT-002: MCP and agent tools

The daemon should expose appropriate planning, workspace, evidence, and approval operations through governed MCP tools or equivalent agent APIs.

### EXT-003: No UI scraping

Agents and integrations must never rely on scraping the desktop or web UI to discover or mutate application state.

### EXT-004: External event ingestion

The app layer must permit external events and evidence to be ingested with source identity, deduplication, correlation, and trust metadata.

---

## 27. Standalone APS distribution requirements

### PUB-001: Canonical source

The canonical APS implementation should live inside the anvil monorepo once convergence is undertaken.

### PUB-002: Public extraction

The APS release process must extract a complete, coherent, buildable public source distribution into `eddacraft/anvil-plan-spec`.

### PUB-003: Public boundary

Exported APS packages must not transitively depend on proprietary anvil packages.

### PUB-004: Isolated verification

The extracted source tree must be built, tested, linted, and packaged outside the private monorepo context before publication.

### PUB-005: Release provenance

The public mirror must record:

- APS version;
- canonical source commit;
- extraction tool version;
- generation timestamp;
- public dependency manifest.

### PUB-006: Standalone surfaces

The public distribution must include the standalone APS CLI and TUI, documentation, templates, schemas, examples, and installation assets required by the release.

### PUB-007: Independent versioning

APS must retain its own version and release cadence even when developed inside the anvil monorepo.

### PUB-008: Contribution path

The public repository must document how external source contributions are reviewed and imported into the canonical monorepo before being republished.

---

## 28. Product modes

The same architecture should support modes such as:

| Mode | Typical capabilities |
| --- | --- |
| **APS standalone** | Planning files, validation, lifecycle, CLI, TUI, export |
| **anvil local** | Daemon, planning integration, local governance, workspaces, evidence |
| **anvil desktop** | Full native planning and operational control plane |
| **anvil web local** | Browser view over a local or reachable daemon |
| **anvil team** | Shared projects, remote state, team policy, organisation evidence |
| **customer-managed** | Customer-hosted control plane and storage |
| **air-gapped** | Local-only operation with offline licensing and updates |

Modes must be assembled from capabilities rather than separate application forks.

---

## 29. Initial acceptance scenarios

### AS-001: Plan and execute from the native app

```text
Given an APS project is registered
And a work item is Ready
When the user opens the project board
And moves the work item to Running
Then the application issues a planning transition command
And the daemon validates dependencies and authority
And the user may create an isolated workspace
And the resulting workspace and run appear in the operational inbox
```

### AS-002: Continue work after the window closes

```text
Given an agent session is running in a daemon-owned workspace
When the user closes the desktop window
Then the agent session continues
And the tray reports the active run
And reopening the application restores the workspace tabs and current status
```

### AS-003: Approval required

```text
Given a run requests a restricted capability
When policy requires human approval
Then an approval request appears in Inbox and the tray attention count
And the action remains paused
When an authorised user approves the exact request
Then the daemon records the decision and resumes only that authorised action
```

### AS-004: Invalid Kanban transition

```text
Given a work item has an incomplete dependency
When a user drags it from Shaping to Ready
Then the daemon rejects the transition
And the card returns to its authoritative lane
And the UI explains the unmet dependency
```

### AS-005: Web review of local work

```text
Given a web client is connected to an authenticated daemon
When a reviewer opens a completed run
Then they can inspect the plan authority, changes, evidence, and gate decisions
But they cannot access local process controls without the required capability
```

### AS-006: Agent crawls eligible work

```text
Given several APS work items exist
When an authorised agent queries eligible work
Then it receives only items whose dependencies, status, project scope, and policy permit execution
And claiming an item happens through a governed command
And the visual board updates from the resulting projection
```

### AS-007: Public APS build

```text
Given an APS release is initiated in the anvil monorepo
When the extraction pipeline runs
Then it produces a standalone public source tree
And the tree builds and tests without private monorepo files
And the public mirror is updated with provenance
And release binaries are built from the verified public tree
```

### AS-008: Planless anvil use

```text
Given a repository has no APS plan
When anvil observes or scans a change
Then findings, evidence, and policy decisions remain available
And the user may later associate the activity with a plan or work item without losing provenance
```

---

## 30. Success measures

The converged app layer should be judged against:

- one canonical implementation for each planning behaviour;
- no independent UI-owned planning or governance state machines;
- shared command handlers across native, web, CLI, TUI, and agent tools;
- daemon-owned work surviving client closure and reconnect;
- feature modules independently releasable but structurally convergent;
- standalone APS remaining buildable and independently useful;
- a user able to move from intent to authorised work, supervised execution, evidence, and verification without switching products;
- a user able to understand what is active and what needs attention from one operational inbox;
- clear architectural boundaries preventing public APS from depending on proprietary anvil modules.

---

## 31. Recommended first product slice

The first vertical slice should prove the shared architecture while satisfying an immediate APS need:

```text
Register local project
→ parse canonical APS files
→ render project overview and Kanban projection
→ issue a validated work-item transition command
→ create an isolated workspace
→ start a simulated or real session
→ stream status into the operational inbox
→ close the native window while work continues
→ reopen from tray and restore workspace tabs
→ attach validation evidence
→ move the item into Verification and Done
```

This slice must use the intended command, query, event, capability, and projection model. It should not depend on a temporary UI-only APS database.

---

## 32. Architectural north star

The enduring separation should remain:

```text
Intent and authority     APS
Execution environment    Workspaces and sessions
Governance and proof     anvil
Durable runtime          Daemon
Human experience         Native, web, tray, CLI, and TUI
Public distribution      Extracted standalone APS source and binaries
```

The product should converge through shared foundations, not by erasing useful product boundaries.


---

# Part II — Domain and Contract Model

# Domain, Command, Event, and Projection Model

## Purpose

This document proposes the app-layer domain model and interaction contracts for a converged APS and anvil product.

It is deliberately framework-neutral. Dioxus, Tauri, React, Ratatui, browser clients, MCP tools, and CLI commands should all sit outside this model.

---

## 1. Modelling principles

1. **Intent is not execution.** APS work describes what is authorised; workspaces and runs describe attempts to satisfy it.
2. **A workspace is not a task.** It is an environment used by one or more attempts.
3. **A session is not a tab.** A session is durable activity; a tab is one visual rendering of it.
4. **Evidence is not merely output.** It must be attributable and related to a claim or decision.
5. **Commands request change. Events record accepted facts. Projections describe current views.**
6. **The daemon owns durable runtime behaviour.** Clients may cache and render but do not become independent authorities.
7. **Repository documents may remain authoritative.** The daemon must reconcile rather than silently replace them.
8. **Modules own their aggregates.** Cross-module effects happen through commands, events, and stable ports.

---

## 2. Bounded contexts

### 2.1 Identity and capability

Owns:

- actors;
- humans;
- agents;
- service identities;
- roles;
- capabilities;
- credentials and leases;
- actor provenance.

### 2.2 Project and repository

Owns:

- logical projects;
- repository registrations;
- physical checkouts;
- repository identity grouping;
- environment associations;
- source-control provider references.

### 2.3 APS planning

Owns:

- plan/index;
- module;
- work item;
- decision;
- issue or question;
- action plan;
- dependencies;
- status;
- validation requirement;
- learning;
- planning warnings;
- planning document mutations.

### 2.4 Workspace

Owns:

- workspace;
- repository attachment;
- worktree or checkout;
- environment;
- layout reference;
- archive state;
- links to plan items and runs.

### 2.5 Run and session

Owns:

- run;
- session;
- provider runtime instance;
- process state;
- input and output stream;
- tool calls;
- run timeline;
- terminal attachment.

### 2.6 Governance and policy

Owns:

- policy evaluation;
- approval request;
- approval response;
- permission decision;
- gate decision;
- exception;
- enforcement mode.

### 2.7 Evidence and verification

Owns:

- evidence record;
- evidence bundle;
- claim;
- verification attempt;
- verification result;
- provenance chain;
- artefact references.

### 2.8 Attention and notification

Owns projections and preferences for:

- attention item;
- unread state;
- settlement;
- notification delivery;
- dismissal or snooze state.

It must not own the source domain state that caused attention.

### 2.9 UI workspace state

Owns durable user presentation preferences such as:

- open tabs;
- pinned tabs;
- tab order;
- split-pane layout;
- selected project scope;
- saved filters;
- theme and density.

It does not own runs, sessions, planning status, or process lifecycle.

---

## 3. Core entities and aggregates

## 3.1 Actor

```rust
pub struct ActorRef {
    pub actor_id: ActorId,
    pub actor_type: ActorType,
    pub display_name: String,
    pub organisation_id: Option<OrganisationId>,
    pub agent_id: Option<AgentId>,
}
```

Actor types may include:

```text
human
agent
service
system
external
```

An actor reference must be included in every mutating command and resulting auditable event.

---

## 3.2 Logical project

A logical project groups the planning, repositories, workspaces, and evidence for one product or delivery context.

```rust
pub struct Project {
    pub id: ProjectId,
    pub name: String,
    pub repositories: Vec<RepositoryRef>,
    pub planning_roots: Vec<PlanningRootRef>,
    pub status: ProjectStatus,
    pub version: AggregateVersion,
}
```

A physical checkout is not a project. Multiple checkouts of the same repository may appear beneath one logical project.

---

## 3.3 APS plan

```rust
pub struct Plan {
    pub id: PlanId,
    pub project_id: ProjectId,
    pub title: String,
    pub problem: String,
    pub success_criteria: Vec<Criterion>,
    pub modules: Vec<ModuleRef>,
    pub risks: Vec<RiskRef>,
    pub source: PlanningSource,
    pub version: AggregateVersion,
}
```

The plan should preserve a source reference to its Markdown document and content version or hash.

---

## 3.4 APS module

```rust
pub struct Module {
    pub id: ModuleId,
    pub plan_id: PlanId,
    pub title: String,
    pub purpose: String,
    pub scope: ScopeDefinition,
    pub constraints: Vec<Constraint>,
    pub work_items: Vec<WorkItemRef>,
    pub status: ModuleStatus,
    pub source: PlanningSource,
    pub version: AggregateVersion,
}
```

---

## 3.5 APS work item

```rust
pub struct WorkItem {
    pub id: WorkItemId,
    pub module_id: ModuleId,
    pub title: String,
    pub intent: String,
    pub expected_outcomes: Vec<Outcome>,
    pub validation: Vec<ValidationRequirement>,
    pub dependencies: Vec<WorkItemRef>,
    pub non_scope: Vec<String>,
    pub status: WorkItemStatus,
    pub authority: AuthorityState,
    pub source: PlanningSource,
    pub version: AggregateVersion,
}
```

Suggested statuses:

```text
Draft
Shaping
DecisionNeeded
AwaitingApproval
Ready
Running
Verification
Blocked
Complete
Cancelled
```

The exact stored APS vocabulary may differ. The projection layer may map several source statuses into common application lanes.

### Invariants

- A work item cannot become `Ready` while required dependencies are incomplete.
- A work item cannot become `Running` without execution authority.
- A work item cannot become `Complete` without satisfying the configured completion policy.
- A UI drag is not itself authority; only the accepted transition command changes status.

---

## 3.6 Decision

```rust
pub struct Decision {
    pub id: DecisionId,
    pub project_id: ProjectId,
    pub plan_id: Option<PlanId>,
    pub title: String,
    pub question: String,
    pub options: Vec<DecisionOption>,
    pub outcome: Option<DecisionOutcome>,
    pub status: DecisionStatus,
    pub decided_by: Option<ActorRef>,
    pub decided_at: Option<Timestamp>,
    pub source: PlanningSource,
    pub version: AggregateVersion,
}
```

A durable architectural decision is different from a runtime approval request.

---

## 3.7 Workspace

```rust
pub struct Workspace {
    pub id: WorkspaceId,
    pub project_id: ProjectId,
    pub purpose: WorkspacePurpose,
    pub work_item_id: Option<WorkItemId>,
    pub repositories: Vec<WorkspaceRepository>,
    pub isolation: IsolationMode,
    pub status: WorkspaceStatus,
    pub active_run_ids: Vec<RunId>,
    pub archived_at: Option<Timestamp>,
    pub version: AggregateVersion,
}
```

Isolation modes:

```text
SharedCheckout
GitWorktree
RemoteEnvironment
ExternallyManaged
EphemeralSandbox
```

### Invariants

- Destructive resource removal must not erase durable history.
- A workspace may be archived while its runs, evidence, and provenance remain inspectable.
- Shared checkout execution may require stronger approval or exclusive locking.

---

## 3.8 Run

```rust
pub struct Run {
    pub id: RunId,
    pub project_id: ProjectId,
    pub workspace_id: Option<WorkspaceId>,
    pub work_item_id: Option<WorkItemId>,
    pub run_type: RunType,
    pub status: RunStatus,
    pub actor: ActorRef,
    pub policy_context: PolicyContextRef,
    pub session_ids: Vec<SessionId>,
    pub evidence_bundle_id: EvidenceBundleId,
    pub started_at: Timestamp,
    pub finished_at: Option<Timestamp>,
    pub version: AggregateVersion,
}
```

Run types may include:

```text
Implementation
Review
Verification
Scan
Audit
Repair
Research
AdHoc
```

### Invariants

- Every run has an attributable initiating actor.
- A governed run records the policy context used at admission.
- A run may succeed operationally while failing verification.
- A run can be planless.

---

## 3.9 Session

```rust
pub struct Session {
    pub id: SessionId,
    pub run_id: RunId,
    pub workspace_id: Option<WorkspaceId>,
    pub session_type: SessionType,
    pub provider: ProviderRef,
    pub durable_handle: Option<String>,
    pub status: SessionStatus,
    pub started_at: Timestamp,
    pub last_activity_at: Timestamp,
    pub finished_at: Option<Timestamp>,
    pub version: AggregateVersion,
}
```

Session types may include:

```text
Agent
Terminal
HumanReview
ExternalJob
Tool
CI
```

A provider process may be replaced or reconnected without changing the durable session identity where continuity is possible.

---

## 3.10 Evidence record

```rust
pub struct EvidenceRecord {
    pub id: EvidenceId,
    pub bundle_id: EvidenceBundleId,
    pub evidence_type: EvidenceType,
    pub source: EvidenceSource,
    pub actor: ActorRef,
    pub claim_refs: Vec<ClaimRef>,
    pub content_ref: EvidenceContentRef,
    pub integrity: IntegrityMetadata,
    pub correlation_id: CorrelationId,
    pub recorded_at: Timestamp,
}
```

Evidence content should be referenced or stored according to size, sensitivity, and retention policy rather than embedded indiscriminately in events.

---

## 3.11 Evidence bundle

```rust
pub struct EvidenceBundle {
    pub id: EvidenceBundleId,
    pub project_id: ProjectId,
    pub work_item_id: Option<WorkItemId>,
    pub run_id: Option<RunId>,
    pub evidence_ids: Vec<EvidenceId>,
    pub claims: Vec<Claim>,
    pub verification_status: VerificationStatus,
    pub version: AggregateVersion,
}
```

---

## 3.12 Approval request

```rust
pub struct ApprovalRequest {
    pub id: ApprovalRequestId,
    pub run_id: RunId,
    pub requested_by: ActorRef,
    pub action: ProposedAction,
    pub scope: CapabilityScope,
    pub policy_basis: Vec<PolicyRef>,
    pub expires_at: Option<Timestamp>,
    pub status: ApprovalStatus,
    pub version: AggregateVersion,
}
```

An approval response must be bound to the exact request version and action digest.

---

## 3.13 Gate decision

```rust
pub struct GateDecision {
    pub id: GateDecisionId,
    pub gate_type: GateType,
    pub target: GateTarget,
    pub evidence_refs: Vec<EvidenceId>,
    pub policy_refs: Vec<PolicyRef>,
    pub outcome: GateOutcome,
    pub conditions: Vec<DecisionCondition>,
    pub decided_by: DecisionActor,
    pub decided_at: Timestamp,
}
```

---

## 3.14 Attention item

Attention is preferably a projection rather than a primary aggregate.

```rust
pub struct AttentionItemProjection {
    pub id: AttentionItemId,
    pub project_id: ProjectId,
    pub target: AttentionTarget,
    pub reason: AttentionReason,
    pub severity: AttentionSeverity,
    pub title: String,
    pub unread: bool,
    pub settled: bool,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}
```

A small durable preference record may track read, snoozed, dismissed, or settled presentation state.

---

## 3.15 Tab and layout state

```rust
pub struct TabState {
    pub id: TabId,
    pub owner_actor_id: ActorId,
    pub window_id: WindowId,
    pub title: String,
    pub kind: TabKind,
    pub target: TabTarget,
    pub pinned: bool,
    pub order: u32,
}

pub struct LayoutState {
    pub owner_actor_id: ActorId,
    pub window_id: WindowId,
    pub layout: LayoutNode,
    pub version: AggregateVersion,
}
```

A tab target might point to a workspace overview, plan, session renderer, diff, evidence bundle, or system view.

Closing a tab never implies stopping its target.

---

## 4. Command model

## 4.1 Command envelope

```rust
pub struct CommandEnvelope<T> {
    pub command_id: CommandId,
    pub command_type: String,
    pub command_version: u16,
    pub actor: ActorRef,
    pub target: CommandTarget,
    pub payload: T,
    pub correlation_id: CorrelationId,
    pub causation_id: Option<CausationId>,
    pub expected_version: Option<AggregateVersion>,
    pub idempotency_key: Option<IdempotencyKey>,
    pub source_surface: SourceSurface,
    pub requested_at: Timestamp,
}
```

Source surfaces may include:

```text
Native
Web
Tray
CLI
TUI
MCP
Automation
System
ExternalIntegration
```

## 4.2 Command result

```rust
pub enum CommandResult<T> {
    Completed(T),
    Accepted { operation_id: OperationId },
    Rejected(CommandRejection),
    Conflict(CommandConflict),
    Unavailable(CommandUnavailable),
    Failed(StructuredError),
}
```

A rejected command is not an exceptional transport failure. It should carry user-actionable reasons.

## 4.3 Planning command catalogue

```text
planning.register_project
planning.refresh
planning.create_plan
planning.update_plan
planning.create_module
planning.update_module
planning.create_work_item
planning.update_work_item
planning.transition_work_item
planning.authorise_work_item
planning.start_work_item
planning.complete_work_item
planning.record_learning
planning.create_decision
planning.resolve_decision
planning.record_issue
planning.export
planning.audit
```

## 4.4 Workspace command catalogue

```text
workspace.create
workspace.attach_repository
workspace.create_worktree
workspace.attach_environment
workspace.open
workspace.pause
workspace.resume
workspace.archive
workspace.abandon
workspace.restore
```

## 4.5 Run and session command catalogue

```text
run.create
run.admit
run.start
run.pause
run.resume
run.complete
run.fail
run.abandon
session.start
session.send_input
session.interrupt
session.stop
session.restart
session.detach
session.reattach
```

## 4.6 Governance command catalogue

```text
policy.evaluate
approval.request
approval.respond
gate.evaluate
gate.record_decision
exception.request
exception.revoke
```

## 4.7 Evidence command catalogue

```text
evidence.record
evidence.attach
evidence.create_bundle
evidence.submit_bundle
verification.start
verification.record_result
```

## 4.8 Presentation command catalogue

Presentation commands may persist user preferences but must remain clearly separated:

```text
ui.open_tab
ui.close_tab
ui.pin_tab
ui.reorder_tabs
ui.update_layout
ui.set_filter
ui.mark_attention_read
ui.settle_attention
```

---

## 5. Event model

## 5.1 Event envelope

```rust
pub struct EventEnvelope<T> {
    pub event_id: EventId,
    pub event_type: String,
    pub event_version: u16,
    pub aggregate_id: AggregateId,
    pub aggregate_version: AggregateVersion,
    pub actor: ActorRef,
    pub payload: T,
    pub correlation_id: CorrelationId,
    pub causation_id: Option<CausationId>,
    pub occurred_at: Timestamp,
}
```

## 5.2 Example events

### Planning

```text
planning.project_registered
planning.documents_refreshed
planning.work_item_created
planning.work_item_authorised
planning.work_item_status_changed
planning.learning_recorded
planning.decision_resolved
planning.conflict_detected
```

### Workspace

```text
workspace.created
workspace.repository_attached
workspace.worktree_created
workspace.status_changed
workspace.archived
workspace.resource_cleanup_failed
```

### Run and session

```text
run.created
run.admitted
run.started
run.paused
run.completed
run.failed
session.started
session.connected
session.output_appended
session.waiting_for_input
session.completed
session.disconnected
```

### Governance and evidence

```text
approval.requested
approval.responded
policy.evaluated
evidence.recorded
evidence.bundle_submitted
verification.completed
gate.decision_recorded
exception.granted
```

### System

```text
system.daemon_started
system.daemon_degraded
system.module_failed
system.migration_completed
system.adapter_disconnected
```

---

## 6. Query and projection model

Queries must be read-only and purpose-specific.

## 6.1 Project catalogue projection

```rust
pub struct ProjectCatalogueProjection {
    pub projects: Vec<ProjectSummary>,
    pub selected_project_id: Option<ProjectId>,
    pub total_attention_count: u32,
}
```

## 6.2 Operational inbox projection

```rust
pub struct OperationalInboxProjection {
    pub scope: InboxScope,
    pub active_items: Vec<OperationalRow>,
    pub settled_items: Vec<OperationalRow>,
    pub filters: AvailableFilters,
    pub unread_count: u32,
}
```

Rows may project workspaces, runs, sessions, approvals, or verification states into one operational form.

## 6.3 Planning board projection

```rust
pub struct PlanningBoardProjection {
    pub project_id: ProjectId,
    pub lanes: Vec<PlanningLane>,
    pub transition_rules: Vec<TransitionRuleSummary>,
    pub warnings: Vec<PlanningWarning>,
}
```

The projection should include permissible transitions or enough information for the UI to preview them, while the daemon remains authoritative.

## 6.4 Plan explorer projection

```rust
pub struct PlanExplorerProjection {
    pub plans: Vec<PlanTreeNode>,
    pub selected: Option<PlanningTarget>,
    pub dependency_summary: DependencySummary,
}
```

## 6.5 Workspace projection

```rust
pub struct WorkspaceProjection {
    pub workspace: WorkspaceSummary,
    pub repositories: Vec<WorkspaceRepositorySummary>,
    pub runs: Vec<RunSummary>,
    pub sessions: Vec<SessionSummary>,
    pub change_summary: ChangeSummary,
    pub evidence_summary: EvidenceBundleSummary,
    pub available_actions: Vec<AvailableAction>,
}
```

## 6.6 Run timeline projection

```rust
pub struct RunTimelineProjection {
    pub run: RunSummary,
    pub entries: Vec<RunTimelineEntry>,
    pub current_attention: Vec<AttentionReason>,
}
```

## 6.7 Evidence and gate projection

```rust
pub struct EvidenceReviewProjection {
    pub target: EvidenceTarget,
    pub claims: Vec<ClaimProjection>,
    pub evidence: Vec<EvidenceSummary>,
    pub gate_decisions: Vec<GateDecisionSummary>,
    pub gaps: Vec<EvidenceGap>,
}
```

## 6.8 System projection

```rust
pub struct SystemStatusProjection {
    pub daemon: DaemonStatus,
    pub modules: Vec<ModuleStatus>,
    pub adapters: Vec<AdapterStatus>,
    pub migrations: MigrationStatus,
    pub active_jobs: Vec<JobSummary>,
    pub recent_errors: Vec<ErrorSummary>,
}
```

---

## 7. Repository authority and file mutation

APS may remain Markdown-authoritative. The app layer should therefore use a deliberate write model.

### Read path

```text
APS files
  → bounded loader
    → canonical parser
      → domain model
        → indexed projection
```

### Write path

```text
transition or edit command
  → validate actor and expected version
    → apply domain rule
      → generate deterministic document patch
        → preview where required
          → atomic file write
            → emit event
              → refresh projections
```

### Required safeguards

- content hash or file version check;
- atomic replace;
- symlink and containment policy;
- backup where appropriate;
- no rewriting unrelated prose without need;
- structured conflict response;
- post-write parse and validation;
- actor and command provenance.

---

## 8. Cross-context interaction examples

## 8.1 Work item becomes Ready

```text
planning.transition_work_item
  → APS validates dependencies and document version
  → planning.work_item_status_changed
  → planning board projection refreshes
  → eligibility projection may add the item
```

No workspace is created automatically unless a policy or explicit automation subscribes to the event.

## 8.2 Workspace created from a work item

```text
workspace.create(work_item_id)
  → planning context queried
  → workspace aggregate created
  → repository adapter creates worktree if requested
  → workspace.created
  → work item projection gains workspace link
  → operational inbox gains active row
```

## 8.3 Agent asks for a restricted capability

```text
session tool request
  → policy.evaluate
  → approval.requested
  → run pauses at the action boundary
  → attention item appears
  → approval.respond
  → exact action digest validated
  → run resumes or action is denied
```

## 8.4 Completion and verification

```text
run completes
  → evidence bundle submitted
  → verification run starts
  → verification result recorded
  → gate decision recorded
  → planning.complete_work_item may be accepted or rejected
```

Operational completion and APS completion are related but not identical.

---

## 9. Idempotency, concurrency, and retries

### 9.1 Expected versions

Commands that update aggregates or repository documents should include an expected version or content hash.

### 9.2 Idempotency

Commands retried after transport interruption must use idempotency keys. The daemon should return the original result when the command was already accepted.

### 9.3 Long-running commands

Commands such as workspace creation, agent start, export, audit, or verification may return an operation identifier and emit progress events.

### 9.4 Conflict handling

The command layer must distinguish:

- domain rule rejection;
- policy rejection;
- aggregate version conflict;
- repository file conflict;
- resource collision;
- unavailable provider;
- transport failure.

Clients should not display every failure as a generic error toast.

---

## 10. Capability model

A capability is a durable named permission or feature contract.

```rust
pub struct CapabilityGrant {
    pub capability: CapabilityId,
    pub scope: CapabilityScope,
    pub actor_id: ActorId,
    pub source: CapabilitySource,
    pub constraints: Vec<CapabilityConstraint>,
    pub expires_at: Option<Timestamp>,
}
```

Capabilities may be granted by:

- local installation mode;
- licence or entitlement;
- organisation policy;
- project role;
- temporary approval;
- daemon configuration;
- runtime availability.

The UI may use capabilities to compose available actions, but the daemon must enforce them.

---

## 11. Public and private dependency boundaries

Suggested direction:

```text
public shared primitives
  → APS domain and application
    → standalone APS CLI/TUI
    → anvil integration adapters
      → proprietary governance/runtime/desktop modules
```

APS may depend on public shared command, event, identity, repository, and projection primitives.

APS must not depend on:

- proprietary policy modules;
- licensing;
- enterprise control-plane code;
- hosted-only storage;
- desktop framework packages;
- private integrations.

anvil may depend on APS through public interfaces and events.

---

## 12. Recommended first contracts to stabilise

Before major UI work, stabilise:

1. identifier newtypes;
2. actor and source-surface types;
3. command envelope and structured result;
4. event envelope;
5. capability identifiers and scope;
6. project, work-item, workspace, run, and session references;
7. planning board projection;
8. operational inbox projection;
9. workspace projection;
10. system status projection;
11. structured errors;
12. API contract version negotiation.

These are sufficient to build the first vertical slice without pretending the entire domain is already final.


---

# Part III — Surface and Experience Architecture

# Surface and Experience Architecture

## Purpose

This document defines how the converged APS and anvil product should divide responsibilities across native desktop, web, tray, CLI, TUI, and agent-facing surfaces.

The goal is one product model expressed through several appropriate experiences, not several applications that happen to share branding.

---

## 1. Surface roles

| Surface | Primary role | Strengths | Deliberate limits |
| --- | --- | --- | --- |
| **Native desktop** | Rich local control plane | Workspaces, tabs, terminals, diffs, planning, evidence, tray integration | Must not own domain state |
| **Web** | Review, planning, approvals, remote access, team visibility | Easy access, shareable links, responsive review | Local process and filesystem controls require explicit connected capability |
| **Tray** | Ambient status and urgent actions | Always available, low interruption, notifications | Not a mini duplicate of the full app |
| **CLI** | Automation and precise commands | Scriptable, headless, composable | Limited visual supervision |
| **TUI** | Terminal-native interactive work | Fast, SSH-friendly, keyboard-first | Cannot mirror every dense graphical interaction |
| **MCP/agent API** | Structured machine interaction | Governed discovery and action | Must not scrape or emulate human UI |

---

## 2. Shared information architecture

The shared conceptual navigation is:

```text
Inbox
Projects
Workspaces
Runs
Evidence
System
```

The graphical surfaces may add local navigation details, but these concepts should retain the same meaning everywhere.

### Inbox

Shows active work and anything requiring attention.

### Projects

Shows durable project context, APS plans, boards, decisions, and history.

### Workspaces

Shows execution environments and their participating runs and sessions.

### Runs

Shows governed execution attempts, including planless anvil scans and audits.

### Evidence

Shows claims, evidence bundles, verification, and gate decisions.

### System

Shows daemon, adapters, providers, configuration, diagnostics, and updates.

---

## 3. The three primary navigation layers

The product needs three separate navigation models rather than forcing one hierarchy to do every job.

## 3.1 Operational sidebar

The operational sidebar answers:

```text
What is happening?
What needs me?
```

It should borrow the strongest ideas from T3 Sidebar V2:

- flat list of current activity;
- logical project shown as metadata and filter scope;
- rich rows for active, unread, blocked, and attention states;
- compact rows for settled work;
- persistent filters;
- automatic reactivation when new activity appears.

Example:

```text
Inbox                                             4

● APS-014  Agent waiting for approval
  anvil · workspace: calm-galileo · Codex

● APS-008  Verification failed
  anvil-plan-spec · 2 failed claims

◐ Ad hoc security scan running
  customer-api · 3m 42s

Settled
○ APS-003  Merged and verified
○ AUD-019  Audit completed
```

The sidebar must not be the canonical plan hierarchy.

## 3.2 Project and plan explorer

The project area answers:

```text
What are we trying to achieve?
How is the work structured?
What is authorised or blocked?
```

It should support:

- project overview;
- repository set;
- plan and module tree;
- work-item details;
- decisions and issues;
- dependency graph;
- milestones and history;
- warnings and readiness.

## 3.3 Workspace tabs and panes

The workspace area answers:

```text
What is being done right now?
Which agents and tools are participating?
What changed?
What evidence exists?
```

Tabs are views, not work records.

Recommended tab types:

```text
Overview
APS context
Agent conversation
Terminal
Raw event stream
Diff
Evidence
Verification
Preview
Pull request
Diagnostics
```

---

## 4. Native desktop experience

## 4.1 Default layout

```text
┌──────────────────────────────────────────────────────────────────────┐
│ Top bar: project scope · command palette · status · actor            │
├─────────────────┬────────────────────────────────────────────────────┤
│ Operational     │ Workspace tabs                                    │
│ sidebar         │ [Overview] [Codex] [Terminal] [Diff] [Evidence]    │
│                 ├────────────────────────────────────────────────────┤
│ Active          │                                                    │
│ Attention       │ Main pane or split layout                          │
│ Settled         │                                                    │
│                 │                                                    │
│                 ├────────────────────────────────────────────────────┤
│                 │ Optional bottom terminal or activity pane          │
└─────────────────┴────────────────────────────────────────────────────┘
```

## 4.2 Native shell responsibilities

The native shell owns only presentation-level behaviour:

- windows;
- tabs and pane layout;
- menus;
- command palette;
- keyboard shortcuts;
- theme and density;
- tray integration;
- notifications;
- deep-link handling;
- client reconnection;
- local user preferences.

The daemon owns planning transitions, workspaces, sessions, policy, evidence, and processes.

## 4.3 Window and tab model

A future-capable model should permit:

- one main window initially;
- detached tabs or secondary windows later;
- picture-in-picture for a terminal, preview, or high-priority session where supported;
- a compact quick-status window from the tray;
- deep links that focus an existing tab or open a new one.

## 4.4 Split panes

Initial supported layouts:

1. single pane;
2. two equal columns;
3. primary pane with narrow secondary pane;
4. primary pane with bottom terminal.

Arbitrary recursive layout may be deferred until usage proves it necessary.

## 4.5 Terminal and agent session views

One durable session should be renderable as:

```text
Structured chat
Raw terminal
Tool-call timeline
Evidence produced
Policy and approval timeline
```

The user should be able to switch renderers without creating a new session.

## 4.6 Diff review

The diff view should place plan authority and evidence near the code review rather than forcing context switching.

Suggested layout:

```text
Changed files | Diff
              |---------------------------------
              | Authority: APS-014
              | Expected outcomes: 3
              | Validation: 2 passed, 1 pending
              | Gate: Awaiting verification
```

---

## 5. APS planning experience

## 5.1 Board purpose

The board is a visual projection of planned and executing work. It is not a separate project-management store.

## 5.2 Default board lanes

```text
Shaping
Decision needed
Awaiting approval
Ready
Running
Verification
Blocked
Done
```

Teams may configure lane mapping, but source statuses and transition rules remain controlled by the planning domain.

## 5.3 Card information

A card should show only scannable information:

- ID and title;
- module or plan identity;
- readiness or validation warning;
- dependencies;
- active workspace or run;
- assigned actor or agent;
- attention state;
- last meaningful activity.

The card detail view should expose full intent, outcomes, validation, non-scope, decisions, workspaces, runs, and evidence.

## 5.4 Drag behaviour

Dragging is a command preview.

```text
pointer drag
  → display candidate transition
    → issue command on drop
      → show pending state
        → accept authoritative projection
        or roll back and explain rejection
```

## 5.5 Plan versus operational state

The card may combine APS status with execution overlays, but must not rewrite the plan merely because a process is running.

For example:

- APS status: `Ready`;
- workspace overlay: `Creating`;
- run overlay: `Agent starting`;
- attention: `Permission required`.

The UI should make the distinction legible.

---

## 6. Web experience

## 6.1 Primary use cases

The initial web surface should prioritise:

- project and plan review;
- board and dependency visibility;
- approvals;
- evidence and gate review;
- run and workspace status;
- comments or decisions where supported;
- organisation-wide views in a future hosted mode.

## 6.2 Local web mode

A local browser view may connect to the same daemon API as the native application.

It must:

- authenticate explicitly;
- use origin and CSRF protections;
- expose only enabled capabilities;
- avoid direct privileged browser bindings;
- reconnect and refresh projections safely.

## 6.3 Remote web mode

A future remote client may connect through a hosted or customer-managed control plane. The domain names should remain the same even if persistence and identity become organisation-wide.

## 6.4 Responsive priority

Full workspace terminal and split-pane experiences may remain desktop-first. Narrow web layouts should still support:

- inbox;
- status;
- approval;
- evidence review;
- plan and work-item details;
- notifications.

---

## 7. Tray experience

## 7.1 Purpose

The tray is an ambient control and attention surface, not a second navigation system.

## 7.2 Recommended menu

```text
anvil                         Healthy
3 active runs · 1 needs attention

Open anvil
Open Inbox
Open APS-014: approval required

Pause new agent actions
Resume new agent actions

System status
Check for updates
Quit desktop client
Stop daemon…
```

Stopping the daemon should be separated and require explicit confirmation.

## 7.3 Quick-status window

Where supported, clicking the tray may open a compact view containing:

- urgent attention items;
- active runs;
- recent completions;
- daemon status;
- fast approval or deny actions only when enough context can be shown safely.

## 7.4 Notifications

High-value notifications:

- approval required;
- agent waiting for input;
- verification failed;
- run complete;
- workspace conflict;
- daemon degraded;
- restricted action denied.

Notifications should deep-link into the relevant object.

---

## 8. CLI experience

## 8.1 Command families

Suggested converged command families:

```text
anvil project …
anvil plan …
anvil workspace …
anvil run …
anvil session …
anvil approval …
anvil evidence …
anvil system …
```

Standalone APS may retain:

```text
aps init
aps lint
aps next
aps start
aps complete
aps audit
aps export
aps tui
```

Both should delegate to shared application behaviour where their capabilities overlap.

## 8.2 Structured automation

CLI commands must provide:

- stable JSON or JSONL where applicable;
- structured exit codes;
- machine-readable rejections;
- correlation identifiers;
- `--no-input` operation;
- explicit daemon or standalone mode.

## 8.3 Human command output

Human output should use the same semantic statuses and vocabulary as graphical surfaces.

---

## 9. TUI experience

## 9.1 Role

The TUI is the terminal-native interactive product for:

- SSH and remote environments;
- keyboard-first users;
- quick planning and status;
- headless systems;
- installation and setup;
- focused run or approval supervision.

## 9.2 Shared concepts

The TUI should share:

- project and workspace terminology;
- status vocabulary;
- command handlers;
- projections;
- capability checks;
- colours and semantic emphasis where terminal support permits;
- inbox and board concepts.

## 9.3 Surface-specific compromises

The TUI need not duplicate:

- arbitrary pane dragging;
- full graphical diffs;
- rich image previews;
- every tab layout;
- high-density visual graph exploration.

It should link or hand off to the native or web surface where that is better.

---

## 10. Agent and MCP experience

## 10.1 Structured discovery

Agents should be able to query:

```text
projects available to this actor
eligible APS work
work-item context
workspace state
active run status
required approvals
evidence gaps
verification result
```

## 10.2 Governed actions

Agents may request:

```text
claim work
create workspace
start run
submit evidence
propose transition
record learning
request approval
release work
```

The daemon remains authoritative and may reject or pause actions.

## 10.3 No visual dependency

The agent API must not depend on board lanes, pixel positions, visible labels, or current UI layout.

---

## 11. Cross-surface continuity

## 11.1 Stable targets

All surfaces should use the same stable object references and deep-link grammar.

Example conceptual URI:

```text
anvil://project/{project_id}/workspace/{workspace_id}/session/{session_id}
```

The final URI scheme is an implementation decision.

## 11.2 Handoff examples

- CLI command prints a deep link to open evidence in the native app.
- Tray notification opens an approval in the existing desktop window.
- Web review page links to the corresponding native workspace when installed.
- TUI can print or copy the remote web URL for a team reviewer.

## 11.3 State continuity

Durable object state must be consistent across clients. Presentation preferences may remain surface-specific unless deliberately synchronised.

---

## 12. Shared component architecture

Even if native and web use different implementation technologies, they should share a semantic component specification.

### Foundation tokens

- typography roles;
- spacing;
- radii;
- elevation;
- borders;
- status semantics;
- motion;
- focus appearance;
- light and dark palettes.

### Interaction primitives

- buttons;
- inputs;
- dialogs;
- menus;
- tooltips;
- popovers;
- tabs;
- tables;
- virtual lists;
- resizable panes;
- command palette;
- drag-and-drop affordances.

### Domain components

- project badge;
- work-item card;
- operational row;
- actor/agent badge;
- run indicator;
- approval card;
- evidence claim;
- gate decision;
- provenance timeline;
- system-health panel.

The native framework spike should measure how much of this needs custom implementation.

---

## 13. State ownership by surface

| State | Owner |
| --- | --- |
| APS status and content | Repository documents through planning application layer |
| Workspace lifecycle | Daemon |
| Run and session lifecycle | Daemon |
| Policy and approvals | Daemon/governance module |
| Evidence and gates | Daemon/evidence module |
| Operational inbox source state | Projections from daemon |
| Read/unread and settled preferences | User preference service |
| Open tabs and split layout | User presentation state |
| Hover, drag preview, open menu | Local client only |
| Tray count | Projection from daemon |

---

## 14. Framework implications

The architecture must remain valid whether the native application uses Dioxus, Tauri plus React, or another framework.

### Dioxus advantage to test

- shared Rust contracts and types;
- direct native integration;
- Rust-native shell and module registration;
- potential native/web component reuse.

### Dioxus risk to test

- terminal integration;
- diff and editor ecosystem;
- drag-and-drop and virtualisation;
- accessible component depth;
- framework churn;
- JavaScript interop complexity.

### Tauri plus React advantage

- mature web component ecosystem;
- shadcn/Radix/TanStack compatibility;
- terminal, diff, graph, and Kanban libraries;
- broad frontend familiarity.

### Tauri plus React risk

- Rust/TypeScript contract seam;
- duplicated view models and validation;
- tendency for business logic to drift into the frontend;
- more complex cross-language debugging.

The final choice should follow the spike evidence rather than ideology.

---

## 15. First experience slice

The first coherent experience should be:

1. launch native app and connect to daemon;
2. register or open a local APS project;
3. see its active work in the flat operational sidebar;
4. open the project board;
5. move a valid item to Running through a command;
6. create an isolated workspace;
7. open overview, session, terminal, diff, and evidence tabs;
8. begin a session and see streamed activity;
9. hide the window and observe status from tray;
10. restore the app with tabs intact;
11. submit validation evidence;
12. review verification and complete the work item.

This gives an immediate APS benefit while proving the future anvil control-plane experience.


---

# Part IV — Convergence and Migration Plan

# APS and anvil Convergence and Migration Plan

## Purpose

This document describes a low-risk path from the current separate and duplicated APS/anvil implementations to one canonical monorepo implementation with multiple intentional product distributions.

The approach is deliberately incremental. It supports immediate APS product needs while avoiding a tactical application that later needs to be rebuilt.

---

## 1. Target repository model

```text
anvil monorepo — canonical development
├── apps/
│   ├── anvil-desktop/
│   ├── anvil-web/
│   └── supporting apps
├── crates/
│   ├── shared kernel and contracts
│   ├── APS domain and application
│   ├── workspace and run modules
│   ├── anvil governance and evidence
│   ├── daemon and transports
│   ├── APS CLI/TUI
│   └── anvil CLI/TUI
├── packages/
│   ├── generated or client contracts
│   ├── graphical design system where applicable
│   └── compatibility packages
└── tooling/
    └── APS extraction and release

eddacraft/anvil-plan-spec — generated public source distribution
├── public APS crates
├── standalone CLI/TUI
├── templates and schemas
├── docs and examples
├── release and installer configuration
└── extraction provenance
```

The public APS repository remains coherent and buildable, but it is downstream of the canonical monorepo.

---

## 2. Current duplication to eliminate

The current repositories contain overlapping APS behaviour in:

- the standalone Rust APS CLI and TUI;
- the TypeScript `packages/aps` parser, loader, validator, state, and types;
- the Rust `anvil-plan-read-model` parser and dashboard projection;
- surface-specific planning logic in CLI and dashboard code.

The migration must converge these into:

```text
one canonical parser
one planning domain model
one planning application layer
one command implementation
one set of projections
multiple clients and distributions
```

---

## 3. Migration principles

1. **Move before redesigning.** Import the standalone APS source with minimal behavioural change before broad refactoring.
2. **One axis at a time.** Do not combine repository movement, domain redesign, daemon integration, desktop development, and public release changes in one large change.
3. **Preserve standalone behaviour.** Existing APS CLI/TUI commands and public installation paths need compatibility tests.
4. **Build the public boundary continuously.** Exportability must be a CI invariant, not a release-week surprise.
5. **No bidirectional automatic synchronisation.** The anvil monorepo is canonical; the public repository is published downstream.
6. **Shared does not automatically mean public.** Public APS dependencies must be explicitly allow-listed.
7. **Use capabilities, not product forks.** APS and anvil modules can ship independently from one application architecture.
8. **Avoid UI-led domain design.** Stabilise commands and projections around real workflows.

---

## 4. Proposed monorepo package shape

Names may change, but responsibilities should be explicit.

```text
crates/
├── edda-ids/                  # stable identifier newtypes
├── edda-commands/             # command envelope and results
├── edda-events/               # event envelope and subscription contracts
├── edda-capabilities/         # capability identifiers and scopes
├── edda-errors/               # structured app-layer errors
├── edda-repository/           # repository identities and ports
│
├── aps-domain/                # pure planning model and invariants
├── aps-parser/                # canonical Markdown parser/serializer
├── aps-application/           # planning command handlers and queries
├── aps-projections/           # board, tree, warning, and status projections
├── aps-storage-fs/            # safe Markdown filesystem adapter
├── aps-cli/                   # standalone `aps` surface
├── aps-tui/                   # standalone terminal UI
│
├── anvil-workspace/           # workspace aggregate and services
├── anvil-run/                 # run and session application layer
├── anvil-governance/          # policy, approvals, gates
├── anvil-evidence/            # evidence and verification
├── anvil-daemon/              # local runtime host
├── anvil-transport/           # IPC/HTTP/event streaming
├── anvil-cli/                 # integrated command surface
└── anvil-tui/                 # integrated terminal surfaces
```

Graphical apps and framework-specific component packages should depend on client/projection contracts rather than domain internals.

---

## 5. Phase 0 — establish decisions and guardrails

### Objectives

- agree that the anvil monorepo becomes canonical for APS source;
- define the public APS export boundary;
- define package dependency rules;
- define supported standalone commands and compatibility expectations;
- select how APS history will be imported;
- establish architecture tests.

### Deliverables

- architecture decision: canonical source and public mirror;
- public package allow-list;
- dependency boundary diagram;
- command and contract naming guidance;
- APS standalone acceptance test inventory;
- temporary freeze on adding new independent APS parsers.

### Exit criteria

- no ambiguity about source authority;
- CI can identify public versus private packages;
- new APS work is directed towards the convergence structure.

---

## 6. Phase 1 — import APS unchanged

### Objectives

Bring the standalone APS repository into the anvil monorepo with history or traceable provenance, while preserving its build and behaviour.

### Approach

- import into a bounded path;
- retain its existing Cargo manifest initially;
- preserve Apache 2.0 licensing for public APS code;
- make the standalone binary build inside the monorepo;
- run existing APS tests without using anvil private crates;
- do not immediately rewrite commands or parser behaviour.

### Suggested temporary shape

```text
crates/aps-standalone/
├── src/
├── scaffold/
├── templates/
└── Cargo.toml
```

### Exit criteria

- `aps` builds and passes its tests inside the monorepo;
- a clean-room copy of the imported subtree can still build;
- existing release fixtures are captured in CI.

---

## 7. Phase 2 — extract canonical APS libraries

### Objectives

Split the application out of the CLI and make one reusable planning implementation.

### Work

1. Extract stable identifiers and planning types.
2. Extract the canonical parser and serializer.
3. Extract planning invariants.
4. Move `next`, `start`, `complete`, `audit`, `export`, and related behaviour into application commands and queries.
5. Extract safe filesystem adapters.
6. Make the CLI thin.
7. Add projection crates for board, hierarchy, and warnings.

### Compatibility

- retain CLI command names and outputs unless a separately approved breaking change is warranted;
- add golden tests for APS document mutations;
- compare current and new exports;
- test real existing plan fixtures from both repositories.

### Exit criteria

- the `aps` executable delegates to `aps-application`;
- parser and mutation behaviour live outside the executable;
- application commands can be invoked without Clap or Ratatui;
- the public dependency graph is clean.

---

## 8. Phase 3 — consolidate internal APS implementations

### Objectives

Remove semantic duplication inside anvil.

### TypeScript APS package

Transition `packages/aps` from authoritative parser/domain behaviour into one of:

- generated transport and projection types;
- a compatibility client over the daemon;
- a temporary wrapper around the canonical Rust implementation.

Do not continue independently evolving planning rules in TypeScript.

### Rust plan read model

Move useful bounded loading and projection concepts from `anvil-plan-read-model` into `aps-projections` and `aps-storage-fs`.

Remove duplicate parsing once parity tests pass.

### Exit criteria

- one canonical parser is used for supported production paths;
- one set of status and warning rules exists;
- dashboard and CLI projections match on the same fixtures;
- no new independent planning state manager remains.

---

## 9. Phase 4 — establish shared command and projection infrastructure

### Objectives

Make native, web, tray, CLI, TUI, and agent APIs capable of using the same app layer.

### Work

- stabilise command and event envelopes;
- introduce actor, correlation, causation, and idempotency;
- define structured command outcomes;
- define capability checks;
- expose planning board, plan explorer, operational inbox, workspace, and system projections;
- establish contract version negotiation;
- wrap existing behaviour rather than requiring full event sourcing.

### Exit criteria

- the CLI can invoke at least one planning command through the shared dispatcher;
- a client can query the planning board projection;
- a client can subscribe to a planning or system change event;
- errors are structured and surface-independent.

---

## 10. Phase 5 — daemon integration

### Objectives

Make the daemon the shared local application host for converged workflows.

### Work

- register APS as a daemon module;
- add project registration and refresh;
- add planning commands and queries;
- add event subscription;
- add capability catalogue and health;
- support safe repository file mutation;
- preserve standalone direct-file mode for the public APS CLI.

### Exit criteria

- daemon-backed and standalone APS operations pass parity tests for common workflows;
- daemon restart and reconnect are safe;
- no graphical client is required for execution.

---

## 11. Phase 6 — native shell and tray

### Objectives

Build the future product shell before overinvesting in a tactical board.

### Work

- complete the Dioxus versus Tauri/React spike;
- create the native shell;
- connect to daemon health, capabilities, commands, and subscriptions;
- implement operational sidebar;
- implement module registration;
- implement tabs and basic layout persistence;
- add tray status and open/hide behaviour;
- make daemon work survive client closure.

### Exit criteria

- the shell is framework-decoupled from domain crates;
- the tray uses daemon projections;
- closing a tab or window does not stop a run;
- feature modules can be enabled independently.

---

## 12. Phase 7 — APS first vertical slice

### Objectives

Satisfy the immediate APS need through the future app architecture.

### Flow

```text
register project
→ render plans and board
→ transition a work item
→ create workspace
→ start session
→ show operational state
→ attach validation evidence
→ verify and complete
```

### Required surfaces

- Projects list;
- plan overview;
- Kanban projection;
- work-item detail;
- operational inbox;
- workspace overview;
- minimal session and terminal view;
- tray status.

### Exit criteria

- board mutations go through canonical planning commands;
- no UI-only task database exists;
- standalone APS still works;
- app state survives restart.

---

## 13. Phase 8 — workspace, run, and session depth

### Objectives

Build the operational control room shared by APS and planless anvil work.

### Work

- worktree creation and archive;
- multi-session runs;
- provider-neutral runtime adapters;
- terminal and structured chat renderers;
- diff and changed-file view;
- split panes;
- tab restoration;
- run timeline;
- worktree and provider status in the inbox.

### Exit criteria

- one workspace can host multiple agent and terminal sessions;
- sessions persist independently of tabs;
- planless runs and APS-linked runs use the same model.

---

## 14. Phase 9 — governance, evidence, and verification

### Objectives

Complete the visible convergence between APS intent and anvil trust.

### Work

- policy and capability display;
- approval inbox;
- evidence bundles;
- validation and verification views;
- gate decisions;
- intent verification;
- provenance timeline;
- rollback and incident links where supported.

### Exit criteria

- a reviewer can trace intent → run → change → evidence → gate decision;
- approval is bound to a specific action;
- completion can be rejected for inadequate evidence;
- governance remains available for planless runs.

---

## 15. Phase 10 — web surface

### Objectives

Expose the same product model through a browser-appropriate client.

### Initial scope

- inbox;
- project and plan review;
- board;
- approvals;
- run status;
- evidence and gate review;
- system connection status.

### Deferred or capability-constrained scope

- local terminal input;
- local process management;
- direct filesystem browsing;
- native window and tray functions.

### Exit criteria

- native and web clients agree on command and projection contracts;
- browser access is authenticated and capability-constrained;
- deep links are portable where practical.

---

## 16. Public APS extraction and release

## 16.1 Release flow

```text
canonical monorepo commit
  → validate public dependency graph
    → generate isolated APS source tree
      → rewrite standalone workspace manifests
        → include docs, templates, schemas, and installers
          → build and test outside monorepo
            → push snapshot to public mirror
              → tag public repository
                → build release artefacts from public tree
                  → publish crates and installers
```

## 16.2 Extraction manifest

Maintain an explicit manifest such as:

```toml
[distribution.aps]
roots = [
  "crates/aps-domain",
  "crates/aps-parser",
  "crates/aps-application",
  "crates/aps-projections",
  "crates/aps-storage-fs",
  "crates/aps-cli",
  "crates/aps-tui",
]

include = [
  "docs/aps",
  "templates",
  "scaffold",
  "schemas",
  "examples",
  "agents",
  "skills",
  "README.md",
  "CHANGELOG.md",
  "LICENSE",
]
```

## 16.3 Provenance file

The public tree should contain generated metadata:

```json
{
  "distribution": "aps",
  "version": "0.8.0",
  "source_commit": "<canonical commit>",
  "generator_version": "<tool version>",
  "generated_at": "<timestamp>"
}
```

## 16.4 Verification

At minimum:

```bash
cargo build --workspace --locked
cargo test --workspace --locked
cargo clippy --workspace --all-targets -- -D warnings
aps --version
aps lint <fixture>
aps export --json <fixture>
```

The public source must not rely on paths or environment only present in the private monorepo.

## 16.5 Contributions

Recommended policy:

- issues and discussions remain open publicly;
- documentation and example changes may be accepted directly if maintainable;
- source PRs are imported into a canonical monorepo branch with attribution;
- the next publication reflects the accepted change;
- no automatic bidirectional Git synchronisation.

---

## 17. Immediate workstreams

The migration can progress in parallel through bounded workstreams.

### Workstream A — domain and contracts

- IDs;
- actors;
- commands;
- events;
- capabilities;
- planning references;
- workspace/run/session references;
- structured errors.

### Workstream B — APS consolidation

- import;
- parser extraction;
- application commands;
- filesystem adapter;
- projections;
- CLI/TUI parity.

### Workstream C — daemon app layer

- module registration;
- command dispatcher;
- query services;
- subscriptions;
- health and capability catalogue.

### Workstream D — native framework spike

- Dioxus versus Tauri/React;
- terminal;
- diff;
- tabs and panes;
- accessibility;
- tray;
- packaging.

### Workstream E — first experience slice

- project registration;
- board;
- work-item transition;
- workspace creation;
- session status;
- evidence completion.

### Workstream F — release extraction

- public manifest;
- clean-room build;
- mirror publication;
- release provenance;
- compatibility checks.

---

## 18. Feature-flag strategy during convergence

Suggested staged flags:

```text
app_shell_v1
operational_inbox_v1
aps_project_views_v1
aps_board_v1
workspace_v1
session_tabs_v1
terminal_v1
evidence_review_v1
governance_approvals_v1
web_client_v1
```

Flags govern rollout. Capabilities still govern what the installation and actor may do.

The application should be able to run in combinations such as:

```text
shell + APS only
shell + APS + workspaces
shell + anvil planless runs
shell + full governance and evidence
```

---

## 19. Compatibility and release discipline

### APS compatibility

Maintain tests for:

- existing plan fixtures;
- status parsing;
- `next` resolution;
- `start` and `complete` mutations;
- context generation;
- lint output;
- export schema;
- installation assets;
- TUI launch and setup flows.

### anvil compatibility

Maintain tests for:

- existing daemon health and lifecycle;
- intercept and policy boundaries;
- existing CLI commands;
- planless operation;
- dashboard projections during transition;
- storage migrations.

### Contract compatibility

- version command and event schemas;
- snapshot serialisation fixtures;
- reject unsupported breaking versions explicitly;
- avoid coupling public APS version to anvil version.

---

## 20. Rollback strategy

Each phase should be independently reversible.

- The public APS repository remains canonical until the imported monorepo build reaches parity.
- The TypeScript APS package remains available behind compatibility adapters until consumers migrate.
- The old dashboard remains available until the new projection client is stable.
- The desktop app can ship APS modules behind flags without replacing CLI/TUI.
- The public mirror switch occurs only after repeated clean-room release rehearsals.

Avoid a single cut-over that simultaneously changes source authority, parser, CLI behaviour, desktop UI, and public release pipeline.

---

## 21. Recommended first implementation milestone

### Milestone: Converged planning spine

Deliver:

1. canonical APS parser and application crate inside the monorepo;
2. thin standalone `aps` CLI using it;
3. daemon module exposing project registration, board query, and work-item transition;
4. native shell showing operational sidebar and board;
5. tray showing daemon health and attention count;
6. clean-room extraction dry run producing a buildable public APS source tree.

This milestone proves the most important claim:

> One canonical planning implementation can serve standalone APS and the future converged anvil application.

---

## 22. Completion definition

The convergence is structurally complete when:

- APS source is canonical in the anvil monorepo;
- the public APS mirror is produced by a verified extraction process;
- no independent TypeScript or Rust APS parser remains on a production path;
- native, web, tray, CLI, TUI, and agent surfaces use the shared command and projection model;
- daemon-owned work survives client lifecycle;
- APS intent and anvil evidence can be traced through one application;
- planless anvil operation remains intact;
- public APS remains independently buildable, licensed, installable, and useful.


---

# Part V — Decisions, Risks, and Open Questions

# Decisions, Risks, and Open Questions

## Purpose

This document captures the decisions that should be made deliberately before or during the APS/anvil app-layer convergence. It also records the principal risks and the questions that should remain open until evidence exists.

---

## 1. Strong current direction

The following direction is sufficiently well formed to treat as the working baseline.

### D-001: One canonical implementation, multiple distributions

APS should be authored once inside the anvil monorepo and released as a standalone public source and binary distribution.

### D-002: The daemon is the local application centre

Native, web, tray, CLI, TUI, MCP, and automation clients should use the daemon-backed application layer when operating in integrated anvil mode.

Standalone APS may retain direct-file operation.

### D-003: APS and anvil remain distinct domains

APS owns intent, planning, authority, dependencies, and planning lifecycle.

anvil owns governance, policy, evidence, verification, provenance, and protected execution.

They converge through shared application foundations and related workflows rather than becoming one undifferentiated aggregate.

### D-004: Immediate APS capability ships inside the future shell

The first Kanban and project views should be feature-flagged modules inside the converged application shell, not a separate tactical APS desktop product.

### D-005: The board is a projection

APS Markdown and application commands remain authoritative. The board does not become another task database.

### D-006: Tabs are presentation state

Tabs and pane layouts must remain separate from work items, workspaces, runs, and sessions.

### D-007: Operational navigation is flat and attention-led

The persistent sidebar should favour active work and human attention, with projects used as scopes and metadata. The plan hierarchy belongs in project views.

### D-008: Framework choice follows a real spike

Dioxus is a serious candidate, but adoption depends on proving terminals, diffs, virtualisation, accessibility, cross-platform tray behaviour, and packaging.

---

## 2. Architecture decisions to record formally

The following should become ADRs or equivalent decisions.

## ADR-A: Canonical APS source and public mirror

Decide:

- canonical repository;
- extraction model;
- public source boundary;
- public contribution path;
- release provenance;
- versioning independence.

## ADR-B: Shared command and event contracts

Decide:

- envelope fields;
- versioning strategy;
- actor identity;
- correlation and causation;
- idempotency;
- long-running operation model;
- structured rejection model.

## ADR-C: Daemon transport

Compare:

- Unix domain sockets and Windows named pipes;
- loopback HTTP;
- SSE versus WebSocket;
- browser access;
- future remote access;
- authentication and origin protection.

A hybrid may be appropriate, but application contracts must remain transport-neutral.

## ADR-D: Planning document authority and mutation

Decide:

- Markdown authority;
- content hashes and version checks;
- deterministic patching;
- conflict handling;
- file watching and projection refresh;
- migration and schema compatibility.

## ADR-E: Workspace isolation and ownership

Decide:

- default isolation mode;
- shared checkout rules;
- worktree creation and naming;
- multi-repository workspaces;
- remote environment support;
- archive and cleanup policy.

## ADR-F: Run and session lifecycle

Decide:

- durable session identity;
- provider process reconnect;
- daemon process supervision;
- terminal attachment;
- output retention and backpressure;
- cancellation semantics.

## ADR-G: Native framework

Decide after the spike:

- Dioxus;
- Tauri plus React;
- constrained hybrid;
- package and update model;
- component system;
- JS interoperability policy.

## ADR-H: Capability and feature model

Decide:

- capability identifier naming;
- scope model;
- entitlement sources;
- local/hosted reconciliation;
- actor role integration;
- feature-flag evaluation and expiry.

## ADR-I: Evidence storage and retention

Decide:

- metadata versus content storage;
- large artefacts;
- sensitive evidence;
- integrity hashes;
- retention and deletion;
- export and portability;
- future hosted synchronisation.

## ADR-J: Native/web shared UI strategy

Decide whether sharing means:

- shared Rust Dioxus components;
- shared TypeScript React components;
- shared design tokens and semantic component specs only;
- generated contracts with separate renderers.

---

## 3. Principal risks

## R-001: Rebuilding the React ecosystem in Rust

### Risk

A Dioxus implementation may require substantial custom work for terminals, diffs, accessible primitives, complex drag-and-drop, virtual lists, split panes, and graphs.

### Mitigation

- run the difficult-component spike first;
- measure custom code rather than judging a polished static mock-up;
- permit a constrained hybrid for genuinely hard components;
- keep domain and client contracts framework-neutral;
- set explicit no-go criteria.

---

## R-002: The UI starts owning business logic

### Risk

Fast delivery pressure may push planning transitions, workspace state, or approvals into the desktop frontend.

### Mitigation

- command-only mutation;
- projection-only reads;
- no direct repository writes from UI packages;
- architecture dependency checks;
- acceptance tests exercised through CLI and graphical client.

---

## R-003: APS loses standalone quality

### Risk

Once canonical source moves into the proprietary monorepo, decisions may optimise only for the integrated desktop experience.

### Mitigation

- standalone APS acceptance suite;
- explicit public package boundary;
- clean-room extraction on every relevant change;
- independent APS versioning;
- CLI/TUI product ownership;
- public documentation and examples maintained as release inputs.

---

## R-004: Public code leaks private dependencies

### Risk

An APS crate may accidentally import proprietary anvil governance, licensing, telemetry, or hosted code.

### Mitigation

- allow-listed public dependency graph;
- transitive dependency validation in CI;
- licence checks;
- isolated public build;
- no release from a dirty or unverified export.

---

## R-005: Too many changes happen together

### Risk

Repository movement, parser rewrite, domain redesign, daemon protocol, desktop framework, and release pipeline could become one unreviewable programme.

### Mitigation

- phase the migration;
- preserve behaviour before redesign;
- maintain compatibility adapters;
- use narrow vertical slices;
- require exit criteria per phase;
- keep rollback paths.

---

## R-006: Command/event architecture becomes ceremony

### Risk

An overbuilt event-sourcing architecture could slow delivery without providing real value.

### Mitigation

- require commands for mutations and events for accepted facts;
- do not require full event-sourced persistence initially;
- build projections from existing storage where practical;
- add event retention only where provenance or asynchronous integration needs it;
- stabilise only contracts needed by real slices.

---

## R-007: Daemon becomes a monolith

### Risk

Putting all product capabilities behind one daemon could create tight coupling and poor failure isolation.

### Mitigation

- explicit modules and ports;
- unidirectional dependency rules;
- module health and degraded mode;
- background job isolation;
- provider adapters;
- capability-based registration;
- avoid direct cross-module database access.

---

## R-008: Repository documents and projections diverge

### Risk

External edits, Git changes, branch switches, or concurrent clients may make daemon indexes stale.

### Mitigation

- file watching and repository refresh;
- content hashes;
- expected versions;
- atomic writes;
- visible conflict state;
- authoritative refresh after reconnect and branch change;
- no silent overwrite.

---

## R-009: Workspaces, runs, and sessions remain ambiguous

### Risk

A collapsed model such as one card = one worktree = one agent chat will fail under review agents, retries, multi-repository work, human intervention, and verification.

### Mitigation

Preserve explicit concepts:

```text
work item
workspace
run
session
tab
```

Add architecture tests and naming guidance that prevent shortcuts at public API boundaries.

---

## R-010: Tray controls become unsafe

### Risk

Compact tray actions could approve or stop consequential work without sufficient context.

### Mitigation

- keep fast actions narrow;
- deep-link to full context for high-risk approval;
- bind approval to exact action digest;
- require confirmation for stopping daemon or broad execution;
- use capability and policy checks server-side.

---

## R-011: Browser access exposes local privilege

### Risk

A local web dashboard could accidentally expose daemon commands, files, or terminals to untrusted origins or local malware.

### Mitigation

- authenticated sessions;
- origin restrictions;
- CSRF protection;
- capability constraints;
- safe transport binding;
- no privileged JS bridge for arbitrary pages;
- explicit opt-in for local web exposure.

---

## R-012: Operational inbox becomes another source of truth

### Risk

Read, settle, and dismiss actions may begin changing actual work state or hiding unresolved risk.

### Mitigation

- attention is a projection;
- source object remains linked;
- dismissal is presentation state unless a domain command is also executed;
- severe unresolved states cannot be permanently hidden without policy.

---

## R-013: Multi-repository workspaces add early complexity

### Risk

Supporting multiple repositories too soon may complicate worktree lifecycle, diffs, branches, and pull requests.

### Mitigation

- design the domain to permit multiple repositories;
- implement one-repository workspaces first;
- add explicit capability flag for multi-repository operation;
- test with one genuine cross-repository slice before broad rollout.

---

## R-014: Framework churn

### Risk

Dioxus or other rapidly changing frameworks may introduce breaking changes and maintenance cost.

### Mitigation

- pin versions;
- isolate framework-specific code;
- avoid experimental renderer dependencies for production;
- document upgrade windows;
- keep app-layer contracts independent;
- compare actual migration cost during the spike.

---

## R-015: Public mirror confuses contributors

### Risk

External contributors may assume the public repository is canonical and be surprised when development history is generated from elsewhere.

### Mitigation

- explain the mirror model prominently;
- preserve readable source and release history;
- document contribution import;
- retain attribution;
- avoid pretending generated release snapshots are canonical development commits.

---

## 4. Open questions

These should remain open until the relevant spike or vertical slice provides evidence.

### Q-001: Which native framework wins?

Dioxus, Tauri plus React, or a constrained hybrid?

### Q-002: What is the primary local transport?

Local IPC, loopback HTTP, or both?

### Q-003: Does the local web surface ship with the desktop or later?

A local browser dashboard may be useful, but it should not distract from proving the native control room.

### Q-004: How much event history is retained?

Enough for provenance, reconnection, and timelines without committing to full event-sourced persistence everywhere.

### Q-005: What application state belongs in SQLite?

Candidates include workspaces, runs, sessions, evidence metadata, preferences, and projections. APS Markdown remains repository-authoritative.

### Q-006: What is the first real agent provider adapter?

The spike can simulate sessions, but the first production slice should choose one provider with durable session and worktree behaviour.

### Q-007: What is the minimum credible diff and terminal experience?

This should be answered by user experience and performance tests, not only technical integration.

### Q-008: How should the product name the combined native application?

The architecture can converge before marketing decides whether the app is simply `anvil`, `anvil desktop`, or another name.

### Q-009: How are APS-only and anvil features licensed?

Capabilities should support the split, but packaging and entitlement design require a product decision.

### Q-010: How should public APS changes be imported?

Manual patch tooling, generated import branches, or another reviewed workflow?

### Q-011: Which shared crates are publicly published?

The public mirror can vendor source or depend on separately published shared crates. The simplest trustworthy release may differ by crate.

### Q-012: How should remote and local projects merge?

A future team control plane may introduce organisation-scoped project identity, remote state, and local execution state.

### Q-013: What is the canonical status vocabulary?

APS source statuses, application board lanes, workspace status, run status, and session status should remain distinct but consistently named.

### Q-014: What is the first multi-user boundary?

Approvals and evidence review may require shared identity before full collaborative editing does.

### Q-015: Should tabs and layouts roam between devices?

Local persistence is enough initially. Hosted preference synchronisation can be deferred.

---

## 5. Recommended order of decisions

1. Canonical APS source and public mirror.
2. Public/private package boundary.
3. Core domain distinctions and command envelope.
4. Planning file authority and mutation model.
5. Native framework spike result.
6. Daemon transport and authentication.
7. Workspace and session lifecycle.
8. Evidence storage and retention.
9. Web scope and remote-control-plane path.
10. Licensing and entitlement composition.

This order minimises work that would otherwise need to be redone.

---

## 6. Decision test

For each major design choice, ask:

1. Does it preserve one canonical implementation?
2. Can standalone APS still work without proprietary anvil modules?
3. Can native, web, tray, CLI, TUI, and agents use the same command?
4. Does daemon-owned work survive the client lifecycle?
5. Are intent, workspace, run, session, and presentation still distinct?
6. Is policy enforced by the daemon rather than only the UI?
7. Can the choice be released incrementally behind capabilities?
8. Does it improve the future converged product rather than only the immediate board?

A tactical shortcut that fails several of these tests should be isolated and explicitly temporary.


---
