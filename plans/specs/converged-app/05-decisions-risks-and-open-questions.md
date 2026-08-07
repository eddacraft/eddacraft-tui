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
