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

