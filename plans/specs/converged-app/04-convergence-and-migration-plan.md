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
