# Rust Kernel Specification (H1)

| Type | Authority | Owner | Status   | Freshness                                        |
| ---- | --------- | ----- | -------- | ------------------------------------------------ |
| Spec | Derived   | KERN  | Proposed | Metadata backfilled 2026-05-24 during DOCGOV-009 |

| Upstream                                                                                                            | Downstream                                                           |
| ------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------- |
| ADR-011a, `docs/architecture/anvil-architecture-evolution.md`, `docs/vision/constitutional-engineering.md`, ADR-006 | `kernel-as-built.md`, benchmarking specs, Rust architecture overview |

**Status:** Proposed — H1 Implementation Target

**Relationship to other documents:**

- This spec refines the Rust kernel originally described in
  [ADR-011a](../../plans/decisions/011a-rust-core-engine.md) (Superseded)
- The [Architecture Evolution](anvil-architecture-evolution.md) document
  supersedes ADR-011 and defines the phased rollout (Current → H1 → H2)
- The kernel's policy model draws from
  [Constitutional Engineering](../vision/constitutional-engineering.md)
  (structural/evolution/procedural law)
- [ADR-006](../../plans/decisions/006-hybrid-dc-opa.md) (Dependency-Cruiser +
  OPA) defines the current policy approach; the kernel's policy engine is its
  long-term successor

---

## Purpose

Define the Rust Watcher Kernel for Anvil.

The Kernel is the high-performance semantic runtime responsible for:

- Maintaining a persistent in-memory model of repository structure
- Performing incremental recomputation on file changes
- Evaluating deterministic structural invariants
- Emitting structured semantic events via the Engine Protocol

The Kernel is not a CLI. The Kernel is not a renderer. The Kernel does not
define product philosophy.

It implements the Engine.

---

## 1. Design Goals

### 1. Deterministic

No AI in enforcement path. Same inputs → same outputs.

### 2. Incremental

Only recompute affected subgraphs on change.

### 3. Persistent

Graph state lives in memory across file changes.

### 4. Streaming

Emit events as soon as meaningful state changes occur.

### 5. Isolated

Surfaces must not depend on internal kernel types.

---

## 2. Responsibilities

The Rust Kernel owns:

- File system watching
- Change coalescing and backpressure
- Incremental parsing (tree-sitter)
- Persistent semantic graph
- Policy evaluation scheduling
- Event emission (Engine Protocol)

It does not own:

- CLI argument parsing
- TUI rendering
- VS Code integration
- Enterprise dashboards
- Remote distribution

---

## 3. High-Level Architecture

```
        ┌────────────────────┐
        │  Engine Protocol   │
        │  (events out)      │
        └─────────▲──────────┘
                  │
        ┌─────────┴──────────┐
        │   Policy Engine    │
        └─────────▲──────────┘
                  │
        ┌─────────┴──────────┐
        │  Semantic Graphs   │
        │  (persistent)      │
        └─────────▲──────────┘
                  │
        ┌─────────┴──────────┐
        │ Incremental Parser │
        │  (tree-sitter)     │
        └─────────▲──────────┘
                  │
        ┌─────────┴──────────┐
        │ Watcher + Queue    │
        │ Debounce / Merge   │
        └────────────────────┘
```

---

## 4. Watcher Subsystem

### 4.1 File Watcher

Use platform-native file notifications (e.g., `notify-rs`).

Must support:

- Recursive directory watching
- Ignore patterns (`node_modules`, build outputs)
- Git-aware filtering (optional in H1)

### 4.2 Debounce & Backpressure

On rapid file change bursts:

- Coalesce changes within a debounce window (e.g., 50–100ms)
- Merge multiple file updates into a single recompute batch
- Avoid recomputing same file multiple times

Queue must:

- Bound memory growth
- Drop redundant events
- Prevent re-entrant recompute loops

---

## 5. Incremental Parsing

### 5.1 Parser Strategy

Use tree-sitter for language parsing.

Maintain:

- AST cache keyed by file hash
- Symbol extraction cache

On file change:

1. Reparse only changed file
2. Replace AST subtree
3. Recompute symbol table entries for affected file

### 5.2 Language Support (H1)

H1 targets languages with mature tree-sitter grammars:

- **TypeScript/JavaScript** — primary target, covers most Anvil users
- **Rust** — dogfooding the kernel on itself

Languages are pluggable via tree-sitter grammar modules. Adding a language
requires:

1. A tree-sitter grammar crate
2. A symbol extraction adapter (maps AST nodes to `SymbolNode` types)
3. A trust annotation convention (how trust levels are declared in that
   language)

The symbol graph schema is language-agnostic — only the parser and extraction
adapter are language-specific.

### 5.3 Hashing

Each file:

- Content hash
- Optional structural hash (future)

Graph nodes maintain:

- Version counters
- Parent-child dependencies

---

## 6. Semantic Graph Model (H1 Scope)

H1 includes only:

### 6.1 Symbol Graph

Nodes:

- Functions
- Classes
- Modules
- Exports

Edges:

- Contains
- References

```
SymbolNode {
  id
  type
  visibility
  file
  trust_level
}

SymbolEdge {
  from
  to
  edge_type (contains, references, calls, imports)
}
```

Stored in-memory using `petgraph` or custom arena-based graph.

### 6.2 Dependency Graph

Derived from symbol graph.

Nodes:

- Modules/files

Edges:

- Import/require relationships
- External dependency calls

### 6.3 Trust Metadata (Minimal in H1)

Each node may include:

- `trust_level` (enum — see below)
- `boundary_label` (string)
- `visibility` (public/internal)

Trust is metadata, not a separate graph in H1.

#### Trust Level Enum

```rust
enum TrustLevel {
    Unknown,      // Default — unparsed or new
    Internal,     // No external exposure
    Boundary,     // Public API surface
    External,     // Calls external services
    Privileged,   // Accesses sensitive resources
}
```

- `Unknown` is the default for any symbol not yet analysed or newly introduced.
- `Boundary` marks symbols that form the public API surface (exported functions,
  public class methods).
- `External` marks symbols that call external services (HTTP clients, database
  drivers, third-party SDKs).
- `Privileged` marks symbols that access sensitive resources (credential stores,
  file system writes, process spawning).
- Trust levels are inferred by the parser's symbol extraction adapter (e.g., a
  function calling `fetch()` is `External`) and can be overridden via annotation
  comments or configuration.

### 6.4 Graph Persistence & Cold Start

**Cold start (H1):** The kernel rebuilds the graph from scratch on every start.
This is simple, correct, and sufficient for the H1 performance target (cold
graph build <3 seconds for 100k LOC repo). Graph snapshot to disk is a
fast-follow optimisation for post-H1.

**Stale state:** If files change while the kernel is stopped, a full rescan on
start detects all changes. Git diff optimisation (only rescan files changed
since last known commit) is post-H1.

**Memory budget:** "Medium repo" is defined as ~100k LOC / ~2000 files. The
<500MB memory target from section 8.3 applies to this size. The graph itself
(petgraph with SymbolNode/SymbolEdge) is expected to use <50MB for 2000 nodes;
the majority of the memory budget is consumed by AST caches and tree-sitter
parse trees.

---

## 7. Policy Engine

The Policy Engine:

- Evaluates invariants against changed subgraphs
- Produces deterministic Violation events
- Must operate on delta-only recomputation

### 7.1 H1 Invariant Scope (Minimal)

- Cross-layer boundary violation
- New external dependency introduction
- Public API surface expansion
- Privilege expansion heuristic (simple)

No drift modelling in H1. No entropy metrics. No behavioural trend tracking.

### 7.2 Policy Evaluation Model

**Invariant definition (H1):** Invariants are Rust functions that receive a
`GraphDelta` and return zero or more `Violation` events. Post-H1, a declarative
DSL may supplement or replace Rust functions for user-authored policies.

**Layer definitions:** Loaded from `.anvil/architecture.yaml` (existing format:
`clean.yaml`, `layered.yaml`, etc.). The kernel reads this configuration at
startup and uses it to annotate graph nodes with their architectural layer.

**Input:** `GraphDelta` — the set of added, removed, and modified nodes and
edges since the last evaluation. The policy engine operates on deltas, not the
full graph, to maintain incremental performance.

**Deduplication:** Violations are fingerprinted by `(policy_id, file, symbol)`.
If the same violation already exists in the current session's violation set, it
is not re-emitted. This prevents duplicate warnings when a file is re-saved
without fixing the violation.

---

## 8. Engine Event Emission

Kernel emits events via the Engine Protocol.

### 8.1 Event Envelope

All events are wrapped in a common envelope for routing, ordering, and
debugging:

```rust
struct EngineEvent {
    event_type: EventType,    // progress, snapshot, violation, error
    seq: u64,                 // monotonic sequence number
    timestamp: String,        // ISO 8601
    engine: EngineId,         // rust, legacy
    payload: EventPayload,    // type-specific enum
}
```

- `seq` is monotonically increasing per engine instance, enabling consumers to
  detect gaps and order events deterministically.
- `engine` identifies which engine produced the event (critical for dual-run
  mode).
- `timestamp` uses ISO 8601 with millisecond precision.

### 8.1.1 Canonical Diagnostic Envelope (`anvil.diagnostic.v1`)

`crates/anvil-kernel-types` owns the canonical `anvil.diagnostic.v1` diagnostic
shape used by gate, save-time, watch, and mid-edit validation surfaces. The AI
guardrail profile (`anvil gate --profile ai`), the RTAI-001 mid-edit
secret-detection loop, and the MCP `validate_write` tool all emit diagnostics in
this envelope so agent and editor consumers can parse results without bespoke
per-surface plumbing. The envelope coordination spec (in `docs/architecture/`)
records how AIGUARD, RTAI, INTD, and DRVR share it and how the schema version is
rolled forward.

### 8.2 Required Event Types

- `Progress` — parsing/evaluation progress
- `Snapshot` — graph state summary after recomputation
- `Violation` — policy invariant violation detected
- `Error` — structured error (parse failure, config error, internal fault)

Events must:

- Be ordered deterministically (via `seq`)
- Avoid full graph dumps
- Emit deltas only

#### Error Event Schema

```rust
struct ErrorPayload {
    code: ErrorCode,          // parse_error, config_error, internal
    file: Option<String>,     // file that triggered the error, if applicable
    message: String,          // human-readable description
    recoverable: bool,        // true if kernel can continue operating
}
```

The kernel isolates malformed file impacts: a parse error in one file does not
prevent analysis of other files. Recoverable errors are emitted as events;
non-recoverable errors (e.g., corrupt configuration) cause the kernel to emit
the error event and then shut down cleanly.

#### Heartbeat (Future — Daemon Mode)

```rust
struct HeartbeatPayload {
    uptime_ms: u64,
    files_watched: u64,
    graph_nodes: u64,
}
```

Defined now for forward compatibility. Not emitted in H1 embedded mode.

### 8.3 Performance Requirements

Target:

- Cold graph build <3 seconds for 100k LOC repo
- Incremental update <100ms for single-file change in medium repo
- Event emission overhead <10ms
- Memory footprint <500MB for medium repo
- No blocking on rendering

---

## 9. Execution Modes

The Kernel supports:

### 9.1 Embedded Mode

Called directly by CLI. Runs in-process or as subprocess. Exits after check.

### 9.2 Foreground Watch Mode

Long-lived process streaming events.

### 9.3 Daemon Mode (Optional in H1)

Runs as background service. Accepts local client connections. Maintains
persistent graph across sessions.

Daemon mode is not required for H1, but architecture must not prevent it.

---

## 10. Error Handling

Kernel must:

- Fail fast on non-recoverable parse errors (e.g. corrupted configuration,
  invalid kernel state)
- Emit structured error events for recoverable issues (e.g. per-file parse
  errors) while continuing processing
- Never panic across engine boundary
- Isolate malformed file impacts to the owning file and dependent analyses only

---

## 11. Non-Goals (H1)

The following are explicitly excluded:

- Distributed watcher mesh
- Cross-repo structural awareness
- Historical drift tracking
- Provenance storage engine
- WASM distribution
- AI-based enforcement

These belong to later phases.

---

## 12. Migration Constraints

- Must support dual-run comparison with legacy engine
- Must not change event semantics without protocol revision
- Must be swappable via `--engine rust`

---

## 13. Success Criteria

The Rust Kernel is considered production-ready when:

- Output parity with legacy engine for H1 rules
- Incremental performance targets met
- No false positive explosion in beta usage
- Surfaces remain unchanged when switching engines
- Behavioural diff prototype built on top of persistent graph

---

## Summary

The Rust Kernel is:

> A persistent, incremental, deterministic semantic runtime that continuously
> enforces structural invariants and streams governance events.

It is not:

- A CLI
- A UI
- A dashboard
- A distributed system

It is the core of the system. Everything else is a surface.
