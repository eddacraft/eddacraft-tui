# Converged APS and anvil App-Layer Requirements

## Document status

- **Status:** Directional specification for architecture and product planning
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
