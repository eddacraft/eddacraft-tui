# Anvil Kernel (Rust)

### Technical Specification — v1 + Strategic Extensions

**Relationship to other documents:**

- This spec implements the Rust kernel described in
  [ADR-011](../../plans/decisions/adr-011-rust-core-engine.md) (Proposed)
- The [Architecture Evolution](anvil-architecture-evolution.md) document
  supersedes ADR-011 and defines the phased rollout
- The kernel's policy model draws from
  [Constitutional Engineering](../vision/constitutional-engineering.md)
  (structural/evolution/procedural law)
- [ADR-006](../../plans/decisions/adr-006-hybrid-dc-opa.md)
  (Dependency-Cruiser + OPA) defines the current policy approach; the kernel's
  policy engine is its long-term successor

---

## 1. Purpose

The Anvil Watcher Kernel is a high-performance, persistent, incremental semantic
analysis engine written in Rust.

Its purpose is to:

- Maintain a live structural model of a repository
- Detect invariant violations in real time
- Evaluate architectural and trust policies continuously
- Provide deterministic governance feedback to developers, CI, and agents

This is not a linter. It is a **semantic guardrail runtime**.

---

## 2. Design Principles

1. Deterministic over probabilistic
2. Incremental over full rescans
3. Structural over textual
4. Streaming over batch
5. Language-agnostic via pluggable parsers
6. Zero reliance on AI for core enforcement

AI may assist interpretation. It never defines truth.

---

## 3. Core Capabilities (v1 – Buildable Now)

---

## 3.1 File System Event Engine

### Responsibility

Watch repository changes and trigger minimal semantic updates.

### Implementation

- `notify` crate (cross-platform inotify/kqueue/FSEvents)
- Debounce batching (50–200ms window)
- Event queue with backpressure control

### Output

```
FileChanged { path, change_type }
```

---

## 3.2 Incremental AST Parsing

### Responsibility

Parse only changed files into syntax trees.

### Implementation

- tree-sitter (language plugins)
- Maintain AST cache
- Hash-based change detection

### Data Structure

```
FileNode {
  path
  language
  ast
  symbol_table
  last_hash
}
```

---

## 3.2.1 Language Support (v1)

v1 targets languages with mature tree-sitter grammars and high relevance to the
Anvil user base:

- **TypeScript/JavaScript** — primary target, covers most Anvil users
- **Rust** — dogfooding the kernel on itself
- **Go** — common in infrastructure codebases

Languages are pluggable via tree-sitter grammar modules. Adding a language
requires:

1. A tree-sitter grammar crate
2. A symbol extraction adapter (maps AST nodes to `SymbolNode` types)
3. A trust annotation convention (how trust levels are declared in that
   language)

Later phases may add Python, Java, and C#. The symbol graph schema is
language-agnostic — only the parser and extraction adapter are
language-specific.

---

## 3.3 Symbol Graph

### Responsibility

Track:

- Functions
- Classes
- Exports
- Imports
- Interfaces
- Public boundaries

### Data Structure

Directed graph:

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
  edge_type (call, import, inherit, etc.)
}
```

Stored in-memory using:

- petgraph
- or custom arena-based graph for performance

---

## 3.4 Dependency Graph

Derived from symbol graph.

Tracks:

- Inter-module dependencies
- External dependencies
- Cross-layer violations

---

## 3.5 Trust Graph (Core Governance Layer)

Each symbol/node may have:

- Trust classification
- Sensitivity level
- External exposure status
- Privilege scope

Policy engine evaluates:

- Unauthorised external calls
- Privilege escalation
- Boundary violations
- Data crossing restricted layers

---

## 3.6 Policy Engine (Deterministic Rules)

Policies defined declaratively (YAML/DSL). Each policy maps to one of the three
invariant categories from
[Constitutional Engineering](../vision/constitutional-engineering.md).

### Structural Law Examples

Hard invariants — non-negotiable boundary rules:

```yaml
policy: external-traffic-restricted
category: structural
match:
  call: fetch
  trust_level: HIGH
require:
  route: approved-gateway

policy: no-cross-layer-imports
category: structural
match:
  import_from: infrastructure/*
  import_in: domain/*
deny: true
message: "Domain layer must not import from infrastructure"
```

### Evolution Law Examples

Rules about the rate and direction of change:

```yaml
policy: public-api-growth-gate
category: evolution
match:
  visibility: public
  change_type: added
require:
  plan_alignment: true
message: 'New public exports require an active plan reference'
```

### Procedural Law Examples

Rules about how change is introduced:

```yaml
policy: boundary-shift-declaration
category: procedural
match:
  trust_level_changed: true
require:
  annotation: 'anvil:trust-change'
message: 'Trust level changes must be explicitly declared'
```

### Engine Behaviour

- Re-evaluates only affected graph subtrees
- Emits structured violation events
- Policies are additive — multiple policies can match the same symbol

---

## 3.7 Invariant Violation Streaming

When a violation occurs:

```
Violation {
  policy_id
  severity
  symbol
  file
  reasoning
  suggested_remediation
}
```

Output modes:

- CLI streaming
- JSON stream (machine readable)
- WebSocket server (optional)

---

## 3.8 Performance Goals (v1)

- Cold graph build under 3 seconds for 100k LOC
- Incremental update under 100ms for single-file change
- Memory footprint under 500MB for medium repo

---

## 4. v1 Architecture

```
[ File Watcher ]
        ↓
[ Event Queue ]
        ↓
[ Incremental Parser ]
        ↓
[ Symbol Graph ]
        ↓
[ Dependency Graph ]
        ↓
[ Trust Graph ]
        ↓
[ Policy Engine ]
        ↓
[ Streaming Output ]
```

Single process. Multi-threaded parsing + evaluation.

---

## 5. CLI Modes

1. `anvil watch`
   - Live streaming

2. `anvil scan`
   - One-shot evaluation

3. `anvil graph`
   - Export graph state

4. `anvil policy test`
   - Validate policy definitions

---

## 6. APS Integration (v1.5 Extension)

Watcher can optionally load active APS plan.

When enabled:

- Detect code changes outside declared plan scope
- Detect plan drift
- Validate declared structural change against actual diff

Example output:

```
PlanDrift {
  file
  symbol
  expected_module
  actual_location
}
```

---

## 7. Future Capabilities (Strategic Roadmap)

Now the fun part.

---

## 7.1 Behavioural Diff Engine

Instead of text diff, compute:

- Call graph delta
- Public API surface delta
- Async path introduction
- New side-effect surface

Output:

```
BehaviouralDelta {
  new_external_calls: 1
  new_public_methods: 2
  privilege_scope_expansion: true
}
```

---

## 7.2 Structural Drift Modelling

Maintain:

- Expected architecture boundaries
- Layer enforcement rules
- Drift tolerance thresholds

Track cumulative drift over time.

Warn before architecture collapses.

---

## 7.3 Provenance Tracking

For each symbol:

- Introduced in commit
- Introduced under plan
- Modified by whom
- Policy state history

This turns Anvil into a code lineage engine.

---

## 7.4 Distributed Watcher Mesh

Compile to:

- Native binary
- WASM

Run in:

- CI
- Agent VMs
- Dev machines
- Pre-commit hooks

Aggregate telemetry into Edda.

---

## 7.5 Semantic Risk Scoring

Compute:

- Risk index per module
- Architectural entropy score
- Privilege concentration index

Trend over time.

---

## 7.6 Data Flow Classification

Advanced static analysis:

- Track data origin
- Track data sinks
- Enforce classification boundaries

Example:

```
PII → logging → violation
```

---

## 7.7 Real-Time Agent Feedback Channel

Agents connected via:

- IPC socket
- JSON stream

Agents receive structured semantic deltas instead of raw git diff.

---

## 7.8 Cross-Repo Structural Awareness

Enterprise mode:

- Multiple repos
- Shared contracts
- Service boundary validation

---

## 7.9 Architecture Snapshots

Persist graph snapshots.

Enable:

- Time-travel architecture diff
- Architectural regression detection

---

## 7.10 Live Architecture Visualisation API

Expose:

- GraphQL
- JSON export
- Real-time graph streaming

Feed into UI or TUI.

---

## 8. Non-Goals (For Sanity)

- No runtime code execution tracing in v1
- No AI interpretation in core enforcement
- No deep whole-program static analysis beyond symbol + call graph initially
- No cross-language full semantic unification (phase 2+)

---

## 9. Build Phases

Phase 1:

- Watcher
- AST parsing
- Symbol graph
- Basic dependency graph
- Basic policy engine

Phase 2:

- Trust graph
- Incremental policy evaluation
- Streaming output

Phase 3:

- APS integration
- Behavioural diff

Phase 4:

- Drift modelling
- Provenance engine

Phase 5:

- Distributed mesh
- Enterprise features

---

## 10. The Real Strategic Outcome

If you execute this correctly:

Anvil becomes:

- A semantic operating system for the repository
- A deterministic governance runtime
- A structural immune system
- The missing layer between intention and implementation

That is not a linter. That is constitutional engineering.
