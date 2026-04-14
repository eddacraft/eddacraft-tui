<!--
APS Module: literate-core + anvil-agent
========================================
Thin, provider-agnostic agent runtime (literate-core) plus Anvil-specific
harness (anvil-agent) with zero-copy semantic graph access.

Scopes: LCORE (literate-core crate), AHARNESS (anvil-agent crate)
-->

# literate-core — Internal Agent Harness

| Scope | Owner | Priority | Status |
| ----- | ----- | -------- | ------ |
| LCORE | —     | medium   | Draft  |

## Purpose

Build a minimal, provider-agnostic agent runtime in Rust and an Anvil-specific
harness that gives LLM agents zero-copy access to the kernel's semantic graph.

**Problem:** Anvil's Rust kernel detects violations deterministically but cannot
*reason* about them. Several high-value features — violation remediation,
behavioural diff narration, policy authoring, headless CI review — require LLM
reasoning over the semantic graph. External agent runtimes (Claude Code, pi)
cannot access the kernel's live graph without serialisation overhead. No
existing Rust agent runtime is minimal enough to embed as a library dependency.

**Solution:** Two crates. `literate-core` captures the irreducible kernel of an
agent runtime (~2500 LOC, ~15 dependencies, Apache-2.0). `anvil-agent` layers
on domain-specific tools (`graph_query`, `policy_eval`) with direct petgraph
access.

**Architecture Decision:** [ADR-024](../decisions/024-internal-agent-harness.md)

## In Scope

### literate-core (Apache-2.0)

- Core types: Message, Content, Context, Model, StreamEvent
- `Provider` trait with async streaming and feature-gated implementations
  (Anthropic, OpenAI)
- `Tool` trait with read-only vs. mutating distinction for safe parallelisation
- Two-level agent loop (inner: tool dispatch, outer: follow-up messages)
- Steering and follow-up message queues
- `SessionStore` trait with default JSONL-tree implementation
  (append-only, branching, context building)
- `EventHandler` trait for typed agent lifecycle notifications
- `Compactor` trait hook for consumer-provided context compaction
- CI enforcement: zero `anvil-*` dependencies (extractability invariant)

### anvil-agent (source-proprietary)

- `graph_query` tool: query semantic graph (petgraph, zero-copy)
- `policy_eval` tool: evaluate policy against current graph state
- Standard tools: `read`, `edit`, `bash` (sandboxed)
- Kernel event → agent trigger wiring
- Anvil-specific context compaction (preserves graph context)
- Integration with `anvil-cli` for `anvil agent` subcommand

## Out of Scope

- TUI rendering (owned by RATS/RCLI)
- General-purpose coding agent features (file creation, refactoring)
- MCP server integration (future module)
- Extension/plugin system (Rust trait composition is sufficient)
- Model fine-tuning or training
- Credential management or OAuth flows (consumer's responsibility)
- Compaction algorithm in literate-core (trait hook only; strategy is
  consumer-provided)

## Interfaces

**Depends on:**

- `anvil-kernel` — semantic graph access, policy evaluation (anvil-agent only)
- `anvil-kernel-types` — EngineEvent, graph types (anvil-agent only)
- `tokio` — async runtime
- `serde` / `serde_json` — serialisation
- `reqwest` — HTTP client for provider implementations (feature-gated)

**Exposes:**

- `literate-core` — reusable agent runtime crate (Apache-2.0, zero anvil deps)
  - `Provider` trait — LLM provider abstraction
  - `Tool` trait — tool registration and execution
  - `SessionStore` trait — session persistence abstraction
  - `EventHandler` trait — lifecycle event subscription
  - `Compactor` trait — context compaction hook
  - `Agent` struct — state machine with steering/follow-up queues
  - `run_agent_loop()` — the core message loop
  - `JsonlSessionStore` — default JSONL-tree session implementation
- `anvil-agent` — Anvil-specific harness
  - `GraphQueryTool` — zero-copy semantic graph queries
  - `PolicyEvalTool` — policy evaluation against live graph
  - `AnvilHarness` — pre-wired agent with Anvil tools and kernel access

## Constraints

- literate-core must have **zero** `anvil-*` dependencies (CI-enforced)
- literate-core must be Apache-2.0 licensed from first commit
- Direct dependencies capped at ~15 crates (no image codecs, no JS runtimes,
  no compiler toolchains)
- Provider implementations are feature-gated (default features = no reqwest)
- Agent suggestions are **always advisory** — never block, never auto-apply
  (aligns with ADR-002: warnings over blocks)
- Session format must be JSONL for auditability and compliance
- Tokio is the async runtime (matches anvil-kernel)

## Acceptance Criteria

- [ ] `literate-core` compiles with zero `anvil-*` in its dependency tree
      (`cargo metadata` check)
- [ ] `literate-core` Cargo.toml specifies `license = "Apache-2.0"`
- [ ] Agent loop completes a multi-turn conversation with tool calls using a
      mock provider
- [ ] JSONL session can be written, read back, branched, and context rebuilt
- [ ] Anthropic provider streams a real response (feature-gated integration test)
- [ ] `anvil-agent` GraphQueryTool returns correct results from a test graph
- [ ] `anvil-agent` PolicyEvalTool evaluates a policy and returns violations
- [ ] Kernel violation event triggers agent reasoning and produces a
      remediation suggestion
- [ ] `git subtree split -P crates/literate-core` produces a clean,
      independently buildable repo

## Ready Checklist

Change status to **Ready** when:

- [ ] ADR-024 accepted
- [ ] Core trait signatures reviewed and agreed
- [ ] Cargo workspace integration verified (builds alongside existing crates)
- [ ] Provider API key availability confirmed for integration tests
- [ ] Dependency budget reviewed (~15 direct deps)

## Risks & Mitigations

| Risk | Likelihood | Impact | Mitigation |
| ---- | ---------- | ------ | ---------- |
| API churn delays stabilisation | Medium | Medium | Anvil-specific concerns in anvil-agent, not literate-core |
| Zero-dep invariant violated accidentally | Medium | High | CI check on every PR: `cargo metadata` for literate-core |
| Provider abstraction too thin for advanced features | Low | Medium | Feature-gated provider-specific extensions |
| Token costs for agent reasoning in CI | Medium | Low | Configurable: off by default, opt-in per pipeline |
| Agent suggestions conflict with deterministic kernel | Low | High | Agent is advisory only; kernel enforces. Clear UX separation |
| Extraction to standalone repo is never prioritised | Medium | Low | Trigger: API stable 4+ weeks AND second consumer exists |

---

## Phase 0 — Spike (Validation)

### LCORE-001: Validate minimal agent loop with mock provider

- **Intent:** Confirm the two-level loop architecture (inner: tool calls, outer:
  follow-ups) works correctly with a mock provider and mock tools
- **Expected Outcome:** Agent completes a multi-turn conversation including tool
  calls, steering messages, and follow-up messages
- **Validation:** `cargo test -p literate-core agent_loop`
- **Confidence:** high
- **Priority:** Critical
- **Dependencies:** None

---

### LCORE-002: Validate JSONL session tree with branching

- **Intent:** Confirm append-only JSONL tree supports write, read, branch, and
  context building without data loss
- **Expected Outcome:** Session entries form a tree via parentId links; branching
  creates a new leaf without modifying history; context building walks the tree
  correctly
- **Validation:** `cargo test -p literate-core session`
- **Confidence:** high
- **Priority:** Critical
- **Dependencies:** None

---

### LCORE-003: Validate Cargo workspace integration

- **Intent:** Confirm `crates/literate-core/` builds alongside existing
  `anvil-*` crates without conflicts or circular dependencies
- **Expected Outcome:** `cargo build --workspace` and `cargo test --workspace`
  pass with literate-core included
- **Validation:** `cargo build --workspace && cargo test --workspace`
- **Confidence:** high
- **Priority:** High
- **Dependencies:** None

---

## Phase 1 — Core Runtime (literate-core)

### LCORE-010: Core types (Message, Content, Context, Model, StreamEvent)

- **Intent:** Define the foundational type system for messages, tool calls, LLM
  context, model metadata, and streaming events
- **Expected Outcome:** Types are defined with serde Serialize/Deserialize,
  covering user messages, assistant messages (text + thinking + tool calls),
  tool result messages, and streaming deltas
- **Validation:** `cargo test -p literate-core types`
- **Confidence:** high
- **Priority:** Critical
- **Dependencies:** None

---

### LCORE-011: Tool trait and execution pipeline

- **Intent:** Define the Tool trait with async execution, read-only distinction,
  JSON Schema parameters, and a tool dispatch pipeline that parallelises
  read-only tools and sequences mutating tools
- **Expected Outcome:** Tools can be registered, dispatched by name, and
  executed with cancellation support
- **Validation:** `cargo test -p literate-core tool`
- **Confidence:** high
- **Priority:** Critical
- **Dependencies:** LCORE-010

---

### LCORE-012: Provider trait and streaming abstraction

- **Intent:** Define the Provider trait that converts Context into an async
  stream of StreamEvents, with a provider registry for runtime selection
- **Expected Outcome:** Providers are registered by API type, resolved at
  runtime, and produce a normalised event stream regardless of backing LLM
- **Validation:** `cargo test -p literate-core provider` (mock provider)
- **Confidence:** high
- **Priority:** Critical
- **Dependencies:** LCORE-010

---

### LCORE-013: Agent state and event system

- **Intent:** Define AgentState (system prompt, model, messages, tools),
  steering/follow-up queues, and the EventHandler trait for lifecycle
  notifications
- **Expected Outcome:** Agent emits typed events (agent_start, turn_start,
  tool_call, tool_result, turn_end, agent_end) that listeners can subscribe to
- **Validation:** `cargo test -p literate-core agent`
- **Confidence:** high
- **Priority:** Critical
- **Dependencies:** LCORE-010, LCORE-011, LCORE-012

---

### LCORE-014: Agent loop (two-level iteration)

- **Intent:** Implement the core message loop: inner loop processes tool calls
  and steering messages, outer loop handles follow-up messages
- **Expected Outcome:** Loop correctly orchestrates prompt → stream → extract
  tool calls → execute → inject results → repeat, with steering interrupts and
  follow-up continuation
- **Validation:** `cargo test -p literate-core agent_loop`
- **Confidence:** high
- **Priority:** Critical
- **Dependencies:** LCORE-011, LCORE-012, LCORE-013

---

### LCORE-015: SessionStore trait and JSONL-tree implementation

- **Intent:** Define the SessionStore trait and implement a default JSONL-tree
  backend with append-only persistence, parentId-based branching, and context
  building
- **Expected Outcome:** Sessions persist as line-delimited JSON, support
  branching without modifying history, and rebuild message context by walking
  the tree from leaf to root
- **Validation:** `cargo test -p literate-core session`
- **Confidence:** high
- **Priority:** High
- **Dependencies:** LCORE-010

---

### LCORE-016: Compactor trait hook

- **Intent:** Define a Compactor trait that the agent loop calls when context
  approaches the model's window limit, allowing consumers to provide their own
  compaction strategy
- **Expected Outcome:** Trait defined with a default no-op implementation;
  consumers can inject LLM-driven summarisation or truncation strategies
- **Validation:** `cargo test -p literate-core compaction`
- **Confidence:** high
- **Priority:** Medium
- **Dependencies:** LCORE-014, LCORE-015

---

## Phase 2 — Providers

### LCORE-020: Anthropic provider (feature-gated)

- **Intent:** Implement the Anthropic Messages API provider behind a
  `provider-anthropic` feature flag, converting Context to Anthropic wire
  format and normalising streaming events
- **Expected Outcome:** Agent can complete conversations using Claude models
  with streaming, tool use, and thinking support
- **Scope:** `crates/literate-core/src/providers/anthropic.rs`
- **Validation:** `cargo test -p literate-core --features provider-anthropic anthropic`
  (integration test, requires API key)
- **Confidence:** high
- **Priority:** High
- **Dependencies:** LCORE-012

---

### LCORE-021: OpenAI provider (feature-gated)

- **Intent:** Implement the OpenAI Chat Completions API provider behind a
  `provider-openai` feature flag, with the same normalised streaming interface
- **Expected Outcome:** Agent can complete conversations using OpenAI models;
  OpenAI-compatible endpoints (Groq, Together, etc.) work via base URL override
- **Scope:** `crates/literate-core/src/providers/openai.rs`
- **Validation:** `cargo test -p literate-core --features provider-openai openai`
- **Confidence:** high
- **Priority:** Medium
- **Dependencies:** LCORE-012

---

## Phase 3 — Anvil Integration (anvil-agent)

### LCORE-030: anvil-agent crate scaffold

- **Intent:** Create the `crates/anvil-agent/` crate with dependencies on
  literate-core and anvil-kernel-types
- **Expected Outcome:** Crate compiles, is included in workspace, and has
  correct licence headers (source-proprietary, distinct from literate-core)
- **Validation:** `cargo build -p anvil-agent`
- **Confidence:** high
- **Priority:** High
- **Dependencies:** LCORE-014

---

### LCORE-031: GraphQueryTool — semantic graph queries

- **Intent:** Implement a Tool that queries the kernel's petgraph semantic graph
  with zero-copy access, supporting queries like "what imports this symbol",
  "what are the transitive callers", "what layer does this belong to"
- **Expected Outcome:** Agent can reason about codebase structure by querying
  the live graph rather than parsing text
- **Validation:** `cargo test -p anvil-agent graph_query`
- **Confidence:** medium
- **Priority:** Critical
- **Dependencies:** LCORE-030, KERN Phase 2 (semantic graph)

---

### LCORE-032: PolicyEvalTool — policy evaluation

- **Intent:** Implement a Tool that evaluates a structural policy against the
  current graph state and returns violations with context
- **Expected Outcome:** Agent can check "would this change violate any policy"
  or "what policies guard this boundary" programmatically
- **Validation:** `cargo test -p anvil-agent policy_eval`
- **Confidence:** medium
- **Priority:** High
- **Dependencies:** LCORE-030, KERN Phase 3 (policy engine)

---

### LCORE-033: Standard tools (read, edit, bash)

- **Intent:** Implement standard file-operation and shell-execution tools for
  anvil-agent, with sandboxing appropriate for a governance context
- **Expected Outcome:** Agent can read files, propose edits, and execute
  commands within constrained scope
- **Validation:** `cargo test -p anvil-agent standard_tools`
- **Confidence:** high
- **Priority:** High
- **Dependencies:** LCORE-030

---

### LCORE-034: Kernel event → agent trigger wiring

- **Intent:** Wire kernel EngineEvents (violations, snapshot completions) to
  trigger agent reasoning, so the agent reacts to structural changes
  automatically
- **Expected Outcome:** When the kernel emits a violation event, anvil-agent
  receives it and can initiate an agent turn with the violation as context
- **Validation:** `cargo test -p anvil-agent triggers`
- **Confidence:** medium
- **Priority:** High
- **Dependencies:** LCORE-031, LCORE-032

---

### LCORE-035: AnvilHarness — pre-wired agent configuration

- **Intent:** Provide a convenience struct that wires literate-core Agent with
  Anvil tools, a configured provider, and session persistence into a
  ready-to-use harness
- **Expected Outcome:** `AnvilHarness::new(kernel, config)` returns a
  fully-configured agent that can reason about the codebase
- **Validation:** `cargo test -p anvil-agent harness`
- **Confidence:** high
- **Priority:** Medium
- **Dependencies:** LCORE-031, LCORE-032, LCORE-033, LCORE-034

---

## Phase 4 — Validation & Extraction Readiness

### LCORE-040: CI enforcement of zero-dep invariant

- **Intent:** Add a CI step that verifies literate-core has no `anvil-*`
  dependencies using `cargo metadata`
- **Expected Outcome:** PRs that accidentally add an Anvil dependency to
  literate-core are blocked
- **Validation:** CI passes with current code; intentional violation is caught
- **Confidence:** high
- **Priority:** High
- **Dependencies:** LCORE-003

---

### LCORE-041: Integration test — full agent conversation

- **Intent:** End-to-end test: mock provider, real tools, real session
  persistence, multi-turn conversation with tool calls and branching
- **Expected Outcome:** Full conversation round-trip works, session is
  persisted and recoverable
- **Validation:** `cargo test -p literate-core integration`
- **Confidence:** high
- **Priority:** High
- **Dependencies:** LCORE-014, LCORE-015

---

### LCORE-042: Integration test — anvil-agent violation remediation

- **Intent:** End-to-end test: kernel detects violation → anvil-agent receives
  trigger → agent queries graph → agent proposes remediation
- **Expected Outcome:** The full pipeline from detection to suggestion works
  with a mock provider
- **Validation:** `cargo test -p anvil-agent remediation_e2e`
- **Confidence:** medium
- **Priority:** High
- **Dependencies:** LCORE-034, LCORE-035

---

### LCORE-043: Extraction dry run

- **Intent:** Verify that `git subtree split -P crates/literate-core` produces
  a clean, independently buildable repository
- **Expected Outcome:** Extracted repo compiles, tests pass, licence is correct,
  no Anvil references in source
- **Validation:** Extract, build, and test in isolation
- **Confidence:** high
- **Priority:** Medium
- **Dependencies:** LCORE-040, LCORE-041

---

## Cross-Product Leverage (Future)

These are not tasks — they document how other products would consume
literate-core once the API stabilises.

| Product | Custom Tools | Use Case |
|---------|-------------|----------|
| Anvil | `graph_query`, `policy_eval` | Violation remediation, behavioural diff narration |
| Kindling | `memory_query`, `session_search` | Knowledge retrieval, pattern surfacing |
| Edda Stack | `ember_recall`, `knowledge_graph` | Memory-augmented agents |
| APS Tooling | `plan_read`, `plan_validate` | Plan-aware automation, spec validation |
| CI Pipeline | `diff_analyze`, `pr_comment` | Headless PR review, violation reporting |

## Performance Targets

| Metric | Target |
| ------ | ------ |
| Agent startup (no provider auth) | < 10ms |
| Tool dispatch overhead | < 1ms |
| Session append (JSONL) | < 5ms |
| Session context build (100 messages) | < 50ms |
| literate-core compile time (clean) | < 30s |
| literate-core binary size contribution | < 2MB |

## Stats

| Phase | Items | Status |
| ----- | ----- | ------ |
| 0 — Spike | 3 | Draft |
| 1 — Core Runtime | 7 | Draft |
| 2 — Providers | 2 | Draft |
| 3 — Anvil Integration | 6 | Draft |
| 4 — Validation | 4 | Draft |
| **Total** | **22** | **0/22** |
