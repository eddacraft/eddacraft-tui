# Rust Kernel Specification (H1)

**Status:** Proposed — H1 Implementation Target

**Relationship to other documents:**

- This spec implements the Rust kernel described in
  [ADR-011](../../plans/decisions/adr-011-rust-core-engine.md) (Proposed)
- The [Architecture Evolution](anvil-architecture-evolution.md) document
  supersedes ADR-011 and defines the phased rollout (Current → H1 → H2)
- The kernel's policy model draws from
  [Constitutional Engineering](../vision/constitutional-engineering.md)
  (structural/evolution/procedural law)
- [ADR-006](../../plans/decisions/adr-006-hybrid-dc-opa.md)
  (Dependency-Cruiser + OPA) defines the current policy approach; the kernel's
  policy engine is its long-term successor

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

- `trust_level` (enum)
- `boundary_label` (string)
- `visibility` (public/internal)

Trust is metadata, not a separate graph in H1.

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

---

## 8. Engine Event Emission

Kernel emits events via the Engine Protocol.

### 8.1 Required Event Types

- `Progress`
- `Snapshot`
- `Violation`

Events must:

- Be ordered deterministically
- Avoid full graph dumps
- Emit deltas only

### 8.2 Performance Requirements

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

- Fail fast on parse errors
- Emit structured error events
- Never panic across engine boundary
- Isolate malformed file impacts

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
