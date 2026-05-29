# PocketFlow Orchestration Gateway

| ID   | Owner  | Status |
|------|--------|--------|
| PFGW | @aneki | Draft  |

**Last reviewed:** 2026-04-26

> **Audit note (2026-04-26):** Module remains Draft, Tier C (parking lot,
> post-launch). Earlier audit pass mistakenly flagged this for archival on
> the basis that `kindling-integration`, `ember`, and `edda-stack-integration`
> are in `plans/archive/modules/`. That was a misread — those *planning
> modules* are archived because their work-item lists completed; the
> underlying components are live code in `packages/edda-stack/src/edda/`,
> `packages/edda-stack/src/ember/`, and `packages/kindling-integration/`.
>
> **Relationship to DRVR/INTD:** PFGW operates at the agent-task
> orchestration layer (capsule lifecycle, memory I/O routing, scope and
> attribution across many actions in a task). The intercept daemon (INTD)
> with surface drivers (DRVR) operates at the file-write enforcement layer
> (per-action mechanical policy enforcement). They are complementary, not
> substitutes. The daemon is opt-in (Anvil is planless-first, local-first);
> a memory/capsule API that operates whether or not the daemon is running
> still has standing.
>
> Not launch-blocker. Defer until RTAI ships, then revisit whether agent
> orchestration is needed in Rust (`crates/anvil-orchestrator`?) or whether
> the existing TS edda-stack surface is sufficient.
>
> **Substrate option:** PocketFlow has a Rust port upstream (PocketFlow-RS).
> A future Rust orchestrator crate could build on it directly rather than
> depending on the TS-vendored primitives in
> `packages/kindling-adapter-pocketflow/vendor/pocketflow/`. This removes
> "TS substrate dependency" as an objection if/when this module promotes.

## Purpose

Build PocketFlow as the runtime orchestration gateway for the eddacraft system. PocketFlow sits between agents/tools/humans and all four downstream components (Kindling, Ember, Edda, Anvil). It mediates all memory I/O, manages capsule lifecycle, enforces scope and attribution, and mechanically applies policy — without interpreting, ranking, or remembering.

This is the glue layer that turns four independent components into a unified system. Without it, agents access components directly and the separation-of-concerns architecture cannot be enforced at runtime.

## References

- [System Specification](../../docs/architecture/system-spec.md) — Full five-component system spec (PocketFlow section: lines 39-186)
- [PocketFlow Capabilities](../../docs/architecture/references/pocketflow-capabilities.md) — What the vendored PocketFlow library provides (Node/Flow/SharedStore primitives)
- [PocketFlow Vendoring](../../docs/architecture/references/pocketflow-vendoring.md) — Vendoring rationale, license, update procedure
- [Edda Stack Architecture](../../docs/architecture/edda-stack.md) — Three-layer memory system the gateway mediates

## Context

PocketFlow exists today in two forms, neither of which is the gateway:

1. **Vendored library** in Kindling (`kindling-adapter-pocketflow/vendor/pocketflow/`) — provides Node/Flow/SharedStore primitives for building workflows
2. **Kindling adapter** (`@eddacraft/kindling-adapter-pocketflow`) — lets users of the PocketFlow library get Kindling capture in their own workflows

The gateway described here is a new component that uses the PocketFlow primitives to build the runtime orchestration layer for the full eddacraft system. It is not a replacement for either existing piece.

## In Scope

### 1. Execution Orchestration

- Accept task or workflow invocations from agents/tools/humans
- Initialize execution context (task ID, capsule ID, agent identity, scope)
- Route actions to tools, agents, or subprocesses
- Enforce ordered execution
- Handle success, failure, and abort states

### 2. Capsule Lifecycle Management

- Open a new capsule at task start
- Bind all actions, tool calls, diffs, logs, and errors to the active capsule
- Close the capsule on completion, failure, or cancellation
- Guarantee that all observable activity occurs within a capsule

### 3. Memory I/O Mediation

- Unified API for memory interactions across all layers
- Route capture operations → Kindling
- Route retrieval operations → configured memory backends (Kindling mechanical or Ember ranked)
- Route promotion requests → Edda
- Prevent direct agent access to memory stores

### 4. Scope & Attribution Enforcement

- Enforce execution scope (repo, workspace, project, task)
- Attribute all actions to an agent, user, or process
- Prevent cross-scope access without explicit escalation
- Reject unauthenticated or unscoped actions

### 5. Policy Enforcement (Mechanical)

- Enforce preconditions defined by Anvil
- Enforce runtime invariants (e.g., active task required)
- Enforce postconditions (e.g., validation steps must run)
- Block or abort execution on violation
- Record violations as observations in Kindling

### 6. Validation & Check Execution

- Run deterministic validation steps (linters, scanners, invariants)
- Capture results as observations
- Do not interpret results or assign meaning

## Out of Scope

PocketFlow gateway does NOT:

- Infer intent (Ember's job)
- Rank or prioritise information (Ember's job)
- Summarise content (Ember's job)
- Detect patterns (Ember's job)
- Store durable memory (Edda's job)
- Curate knowledge (Edda's job)
- Define policy (Anvil's job)
- Perform semantic reasoning

## Interfaces

**Task Interface:**

- `task.start(metadata)` / `task.complete(result)` / `task.fail(error)` / `task.abort(reason)`

**Capsule Interface:**

- `capsule.open(context)` / `capsule.close(status)`

**Action Interface:**

- `action.execute(tool, input)` / `action.validate(output)` / `action.abort(reason)`

**Memory Interface (Abstracted):**

- `memory.capture(observation)` — routes to Kindling
- `memory.retrieve(query)` — routes to Kindling (mechanical) or Ember (ranked)
- `memory.propose(candidate)` — routes to Ember
- `memory.promote(reference)` — routes to Edda

**Policy Interface:**

- `policy.check_preconditions(context)` / `policy.check_postconditions(result)` / `policy.check_runtime(action)`

## Dependencies

**Depends on:**

- Kindling (observation capture, capsule storage, mechanical retrieval) — external dependency, open source
- Ember (candidate memory, ranking, interpretation) — `packages/edda-stack/src/ember/`
- Edda (canonical memory, promotion, versioning) — `packages/edda-stack/src/edda/`
- Anvil (policy definitions, preconditions/postconditions) — `packages/anvil/`
- PocketFlow primitives (Node, Flow, SharedStore) — vendored in Kindling or re-vendored here

**Exposes:**

- Gateway runtime that agents interact with instead of accessing components directly
- Execution context management
- Unified memory API

## Ready Checklist

- [x] Purpose and scope are clear
- [x] Dependencies identified
- [ ] Component boundaries with existing edda-stack code need clarification
- [ ] Decision needed: where does the gateway package live (new package vs. extension of existing)
- [ ] Decision needed: how does the gateway consume Kindling (npm dep vs. Rust binary IPC)

## Work Items

### PFGW-001: Define gateway package structure

- **Intent:** Establish where the gateway code lives and its dependency graph
- **Expected Outcome:** Package created with clear imports from Kindling, Ember, Edda, and Anvil
- **Validation:** Package builds with all four dependencies resolved
- **Status:** Draft

### PFGW-002: Execution context and capsule lifecycle

- **Intent:** Gateway manages task → capsule lifecycle end-to-end
- **Expected Outcome:** Starting a task opens a capsule, all actions are bound to it, completion closes it
- **Validation:** Integration test shows capsule opened/closed with observations attached
- **Status:** Draft
- **Dependencies:** PFGW-001

### PFGW-003: Memory I/O mediation layer

- **Intent:** Unified memory API that routes to the correct downstream component
- **Expected Outcome:** `memory.capture()` → Kindling, `memory.retrieve()` → Kindling or Ember, `memory.propose()` → Ember, `memory.promote()` → Edda
- **Validation:** Integration tests verify correct routing for each operation
- **Status:** Draft
- **Dependencies:** PFGW-002

### PFGW-004: Scope and attribution enforcement

- **Intent:** All actions are scoped and attributed; cross-scope access is blocked
- **Expected Outcome:** Actions without valid scope/identity are rejected; all observations carry attribution
- **Validation:** Tests verify rejection of unscoped actions and correct attribution on observations
- **Status:** Draft
- **Dependencies:** PFGW-002

### PFGW-005: Anvil policy enforcement integration

- **Intent:** Gateway mechanically enforces Anvil preconditions, postconditions, and runtime invariants
- **Expected Outcome:** Policy violations block or abort execution; violations are recorded as observations
- **Validation:** Integration test with Anvil policy that blocks an action, verifies observation recorded
- **Status:** Draft
- **Dependencies:** PFGW-003, PFGW-004

### PFGW-006: Agent access prevention

- **Intent:** Agents cannot bypass the gateway to access Kindling/Ember/Edda/Anvil directly
- **Expected Outcome:** The gateway is the only public API; downstream components are internal
- **Validation:** Package exports only the gateway interface; no direct component imports possible from consumer code
- **Status:** Draft
- **Dependencies:** PFGW-001

### PFGW-007: Standalone vs gateway mode

- **Intent:** Kindling continues to work standalone when PocketFlow gateway is not present
- **Expected Outcome:** Kindling detects whether the gateway is active; if not, manages its own capsule lifecycle
- **Validation:** Existing Kindling tests pass without the gateway installed
- **Status:** Draft
- **Dependencies:** PFGW-002

## Open Questions

- [ ] Should the gateway be a new top-level package or extend `packages/edda-stack/`?
- [ ] How does the gateway consume Kindling? npm dependency? Rust binary IPC? Both?
- [ ] Does the vendored PocketFlow library get re-vendored here, or is it consumed from the Kindling adapter package?
- [ ] What is the first integration target — Claude Code plugin? MCP server? CLI?
- [ ] How does the gateway interact with the existing `opa-agent-orchestration` module?
