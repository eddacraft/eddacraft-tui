<!--
APS Module: weave + anvil-weave
========================================
Thin, provider-agnostic agent runtime (weave) plus Anvil-specific
harness (anvil-weave) with zero-copy semantic graph access.

Scopes: WEAVE (weave crate, eddacraft/weave-rs + anvil-weave crate — Phase 3 items were originally drafted under an `AHARNESS-NNN` prefix; merged into the WEAVE namespace 2026-05-16 for tracking consistency)
-->

# weave — Agent Runtime

| ID    | Owner | Status | Progress |
| ----- | ----- | ------ | -------- |
| WEAVE | —     | Draft  | 0/21     |

**Last reviewed:** 2026-04-26

> **Audit note (2026-04-26):** Greenfield confirmed — no `weave` or
> `anvil-weave` crate exists in `crates/` yet. The standalone upstream at
> [`eddacraft/weave-rs`](https://github.com/eddacraft/weave-rs) is the
> source-of-truth for the runtime layer and is in place. ADR-024 reference
> and dependency on `anvil-kernel` / `anvil-kernel-types` crates verified
> against the current workspace.

Priority: medium.

## Purpose

Build a minimal, provider-agnostic agent runtime in Rust and an Anvil-specific
harness that gives LLM agents zero-copy access to the kernel's semantic graph.

**Problem:** Anvil's Rust kernel detects violations deterministically but cannot
*reason* about them. Several high-value features — violation remediation,
behavioural diff narration, policy authoring, headless CI review — require LLM
reasoning over the semantic graph. External agent runtimes (Claude Code, pi)
cannot access the kernel's live graph without serialisation overhead. No
existing Rust agent runtime is minimal enough to embed as a library dependency.

**Solution:** Two crates. `weave` captures the irreducible kernel of an
agent runtime (~2500 LOC, ~15 dependencies, Apache-2.0), living in the
standalone `eddacraft/weave-rs` repo. `anvil-weave` layers on domain-specific
tools (`graph_query`, `policy_eval`) with direct petgraph access, in
`crates/anvil-weave/`.

**Architecture Decision:** [ADR-024](../decisions/024-internal-agent-harness.md)

## In Scope

### weave (Apache-2.0, eddacraft/weave-rs)

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

### anvil-weave (source-proprietary, crates/anvil-weave/)

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
- Compaction algorithm in weave (trait hook only; strategy is
  consumer-provided)

## Interfaces

**Depends on:**

- `anvil-kernel` — semantic graph access, policy evaluation (anvil-weave only)
- `anvil-kernel-types` — EngineEvent, graph types (anvil-weave only)
- `tokio` — async runtime
- `serde` / `serde_json` — serialisation
- `reqwest` — HTTP client for provider implementations (feature-gated)

**Exposes:**

- `weave` — reusable agent runtime crate (Apache-2.0, zero anvil deps,
  published from eddacraft/weave-rs)
  - `Provider` trait — LLM provider abstraction
  - `Tool` trait — tool registration and execution
  - `SessionStore` trait — session persistence abstraction
  - `EventHandler` trait — lifecycle event subscription
  - `Compactor` trait — context compaction hook
  - `Agent` struct — state machine with steering/follow-up queues
  - `run_agent_loop()` — the core message loop
  - `JsonlSessionStore` — default JSONL-tree session implementation
- `anvil-weave` — Anvil-specific harness (depends on weave via path dep during
  dev, crates.io dep at release)
  - `GraphQueryTool` — zero-copy semantic graph queries
  - `PolicyEvalTool` — policy evaluation against live graph
  - `AnvilHarness` — pre-wired agent with Anvil tools and kernel access

## Constraints

- weave must have **zero** `anvil-*` dependencies (structurally enforced —
  separate repo)
- weave must be Apache-2.0 licensed from first commit
- Direct dependencies capped at ~15 crates (no image codecs, no JS runtimes,
  no compiler toolchains)
- Provider implementations are feature-gated (default features = no reqwest)
- Agent suggestions are **always advisory** — never block, never auto-apply
  (aligns with ADR-002: warnings over blocks)
- Session format must be JSONL for auditability and compliance
- Tokio is the async runtime (matches anvil-kernel)

## Acceptance Criteria

- [ ] `weave` compiles with zero `anvil-*` in its dependency tree
- [ ] `weave` Cargo.toml specifies `license = "Apache-2.0"`
- [ ] Agent loop completes a multi-turn conversation with tool calls using a
      mock provider
- [ ] JSONL session can be written, read back, branched, and context rebuilt
- [ ] Anthropic provider streams a real response (feature-gated integration test)
- [ ] `anvil-weave` GraphQueryTool returns correct results from a test graph
- [ ] `anvil-weave` PolicyEvalTool evaluates a policy and returns violations
- [ ] Kernel violation event triggers agent reasoning and produces a
      remediation suggestion
- [ ] weave-rs repo builds and tests independently

## Ready Checklist

Change status to **Ready** when:

- [ ] ADR-024 accepted
- [ ] Core trait signatures reviewed and agreed
- [ ] weave-rs standalone repo builds verified
- [ ] Provider API key availability confirmed for integration tests
- [ ] Dependency budget reviewed (~15 direct deps)

## Risks & Mitigations

| Risk | Likelihood | Impact | Mitigation |
| ---- | ---------- | ------ | ---------- |
| API churn delays stabilisation | Medium | Medium | Anvil-specific concerns in anvil-weave, not weave |
| Zero-dep invariant violated accidentally | Medium | High | Structurally enforced via separate repo; no cross-repo deps possible |
| Provider abstraction too thin for advanced features | Low | Medium | Feature-gated provider-specific extensions |
| Token costs for agent reasoning in CI | Medium | Low | Configurable: off by default, opt-in per pipeline |
| Agent suggestions conflict with deterministic kernel | Low | High | Agent is advisory only; kernel enforces. Clear UX separation |
| Two-repo coordination overhead | Medium | Low | Path dep during dev; only matters at release boundaries |
| crates.io name collision | Low | Low | Check availability; fallback: weave-agent, weave-core |

---

## Work Items

### Phase 0 — Spike (Validation)

Spike work happens in the `eddacraft/weave-rs` standalone repo, not in
`crates/`.

#### WEAVE-001: Validate minimal agent loop with mock provider

- **Intent:** Confirm the two-level loop architecture (inner: tool calls, outer:
  follow-ups) works correctly with a mock provider and mock tools
- **Expected Outcome:** Agent completes a multi-turn conversation including tool
  calls, steering messages, and follow-up messages
- **Validation:** `cargo test agent_loop`
- **Confidence:** high
- **Priority:** Critical
- **Dependencies:** None

---

#### WEAVE-002: Validate JSONL session tree with branching

- **Intent:** Confirm append-only JSONL tree supports write, read, branch, and
  context building without data loss
- **Expected Outcome:** Session entries form a tree via parentId links; branching
  creates a new leaf without modifying history; context building walks the tree
  correctly
- **Validation:** `cargo test session`
- **Confidence:** high
- **Priority:** Critical
- **Dependencies:** None

---

#### WEAVE-003: Validate standalone repo builds independently

- **Intent:** Confirm `weave` builds, tests, and lints cleanly as a standalone
  crate outside any Cargo workspace
- **Expected Outcome:** `cargo build`, `cargo test`, `cargo clippy` all pass in
  the `eddacraft/weave-rs` repo with no workspace context
- **Validation:** `cargo test && cargo clippy -- -D warnings`
- **Confidence:** high
- **Priority:** High
- **Dependencies:** None

---

### Phase 1 — Core Runtime (weave)

#### WEAVE-010: Core types (Message, Content, Context, Model, StreamEvent)

- **Intent:** Define the foundational type system for messages, tool calls, LLM
  context, model metadata, and streaming events
- **Expected Outcome:** Types are defined with serde Serialize/Deserialize,
  covering user messages, assistant messages (text + thinking + tool calls),
  tool result messages, and streaming deltas
- **Validation:** `cargo test types`
- **Confidence:** high
- **Priority:** Critical
- **Dependencies:** None

---

#### WEAVE-011: Tool trait and execution pipeline

- **Intent:** Define the Tool trait with async execution, read-only distinction,
  JSON Schema parameters, and a tool dispatch pipeline that parallelises
  read-only tools and sequences mutating tools
- **Expected Outcome:** Tools can be registered, dispatched by name, and
  executed with cancellation support
- **Validation:** `cargo test tool`
- **Confidence:** high
- **Priority:** Critical
- **Dependencies:** WEAVE-010

---

#### WEAVE-012: Provider trait and streaming abstraction

- **Intent:** Define the Provider trait that converts Context into an async
  stream of StreamEvents, with a provider registry for runtime selection
- **Expected Outcome:** Providers are registered by API type, resolved at
  runtime, and produce a normalised event stream regardless of backing LLM
- **Validation:** `cargo test provider` (mock provider)
- **Confidence:** high
- **Priority:** Critical
- **Dependencies:** WEAVE-010

---

#### WEAVE-013: Agent state and event system

- **Intent:** Define AgentState (system prompt, model, messages, tools),
  steering/follow-up queues, and the EventHandler trait for lifecycle
  notifications
- **Expected Outcome:** Agent emits typed events (agent_start, turn_start,
  tool_call, tool_result, turn_end, agent_end) that listeners can subscribe to
- **Validation:** `cargo test agent`
- **Confidence:** high
- **Priority:** Critical
- **Dependencies:** WEAVE-010, WEAVE-011, WEAVE-012

---

#### WEAVE-014: Agent loop (two-level iteration)

- **Intent:** Implement the core message loop: inner loop processes tool calls
  and steering messages, outer loop handles follow-up messages
- **Expected Outcome:** Loop correctly orchestrates prompt → stream → extract
  tool calls → execute → inject results → repeat, with steering interrupts and
  follow-up continuation
- **Validation:** `cargo test agent_loop`
- **Confidence:** high
- **Priority:** Critical
- **Dependencies:** WEAVE-011, WEAVE-012, WEAVE-013

---

#### WEAVE-015: SessionStore trait and JSONL-tree implementation

- **Intent:** Define the SessionStore trait and implement a default JSONL-tree
  backend with append-only persistence, parentId-based branching, and context
  building
- **Expected Outcome:** Sessions persist as line-delimited JSON, support
  branching without modifying history, and rebuild message context by walking
  the tree from leaf to root
- **Validation:** `cargo test session`
- **Confidence:** high
- **Priority:** High
- **Dependencies:** WEAVE-010

---

#### WEAVE-016: Compactor trait hook

- **Intent:** Define a Compactor trait that the agent loop calls when context
  approaches the model's window limit, allowing consumers to provide their own
  compaction strategy
- **Expected Outcome:** Trait defined with a default no-op implementation;
  consumers can inject LLM-driven summarisation or truncation strategies
- **Validation:** `cargo test compaction`
- **Confidence:** high
- **Priority:** Medium
- **Dependencies:** WEAVE-014, WEAVE-015

---

### Phase 2 — Providers

#### WEAVE-020: Anthropic provider (feature-gated)

- **Intent:** Implement the Anthropic Messages API provider behind a
  `provider-anthropic` feature flag, converting Context to Anthropic wire
  format and normalising streaming events
- **Expected Outcome:** Agent can complete conversations using Claude models
  with streaming, tool use, and thinking support
- **Scope:** `src/providers/anthropic.rs`
- **Validation:** `cargo test --features provider-anthropic anthropic`
  (integration test, requires API key)
- **Confidence:** high
- **Priority:** High
- **Dependencies:** WEAVE-012

---

#### WEAVE-021: OpenAI provider (feature-gated)

- **Intent:** Implement the OpenAI Chat Completions API provider behind a
  `provider-openai` feature flag, with the same normalised streaming interface
- **Expected Outcome:** Agent can complete conversations using OpenAI models;
  OpenAI-compatible endpoints (Groq, Together, etc.) work via base URL override
- **Scope:** `src/providers/openai.rs`
- **Validation:** `cargo test --features provider-openai openai`
- **Confidence:** high
- **Priority:** Medium
- **Dependencies:** WEAVE-012

---

### Phase 3 — Anvil Integration (anvil-weave)

#### WEAVE-030: anvil-weave crate scaffold

- **Intent:** Create the `crates/anvil-weave/` crate with dependencies on
  weave and anvil-kernel-types
- **Expected Outcome:** Crate compiles, is included in workspace, and has
  correct licence headers (source-proprietary, distinct from weave)
- **Validation:** `cargo build -p anvil-weave`
- **Confidence:** high
- **Priority:** High
- **Dependencies:** WEAVE-014

---

#### WEAVE-031: GraphQueryTool — semantic graph queries

- **Intent:** Implement a Tool that queries the kernel's petgraph semantic graph
  with zero-copy access, supporting queries like "what imports this symbol",
  "what are the transitive callers", "what layer does this belong to"
- **Expected Outcome:** Agent can reason about codebase structure by querying
  the live graph rather than parsing text
- **Validation:** `cargo test -p eddacraft-anvil-weave graph_query`
- **Confidence:** medium
- **Priority:** Critical
- **Dependencies:** WEAVE-030, KERN Phase 2 (semantic graph)

---

#### WEAVE-032: PolicyEvalTool — policy evaluation

- **Intent:** Implement a Tool that evaluates a structural policy against the
  current graph state and returns violations with context
- **Expected Outcome:** Agent can check "would this change violate any policy"
  or "what policies guard this boundary" programmatically
- **Validation:** `cargo test -p eddacraft-anvil-weave policy_eval`
- **Confidence:** medium
- **Priority:** High
- **Dependencies:** WEAVE-030, KERN Phase 3 (policy engine)

---

#### WEAVE-033: Standard tools (read, edit, bash)

- **Intent:** Implement standard file-operation and shell-execution tools for
  anvil-weave, with sandboxing appropriate for a governance context
- **Expected Outcome:** Agent can read files, propose edits, and execute
  commands within constrained scope
- **Validation:** `cargo test -p eddacraft-anvil-weave standard_tools`
- **Confidence:** high
- **Priority:** High
- **Dependencies:** WEAVE-030

---

#### WEAVE-034: Kernel event → agent trigger wiring

- **Intent:** Wire kernel EngineEvents (violations, snapshot completions) to
  trigger agent reasoning, so the agent reacts to structural changes
  automatically
- **Expected Outcome:** When the kernel emits a violation event, anvil-weave
  receives it and can initiate an agent turn with the violation as context
- **Validation:** `cargo test -p eddacraft-anvil-weave triggers`
- **Confidence:** medium
- **Priority:** High
- **Dependencies:** WEAVE-031, WEAVE-032

---

#### WEAVE-035: AnvilHarness — pre-wired agent configuration

- **Intent:** Provide a convenience struct that wires weave Agent with
  Anvil tools, a configured provider, and session persistence into a
  ready-to-use harness
- **Expected Outcome:** `AnvilHarness::new(kernel, config)` returns a
  fully-configured agent that can reason about the codebase
- **Validation:** `cargo test -p eddacraft-anvil-weave harness`
- **Confidence:** high
- **Priority:** Medium
- **Dependencies:** WEAVE-031, WEAVE-032, WEAVE-033, WEAVE-034

---

### Phase 4 — Validation

#### WEAVE-040: CI workflow for weave-rs repo

- **Intent:** Set up GitHub Actions CI for `eddacraft/weave-rs` with check,
  test, clippy, and dependency audit
- **Expected Outcome:** PRs to weave-rs are validated automatically; the
  zero-dep invariant is structurally guaranteed
- **Validation:** CI passes on a PR
- **Confidence:** high
- **Priority:** High
- **Dependencies:** WEAVE-003

---

#### WEAVE-041: Integration test — full agent conversation

- **Intent:** End-to-end test: mock provider, real tools, real session
  persistence, multi-turn conversation with tool calls and branching
- **Expected Outcome:** Full conversation round-trip works, session is
  persisted and recoverable
- **Validation:** `cargo test integration`
- **Confidence:** high
- **Priority:** High
- **Dependencies:** WEAVE-014, WEAVE-015

---

#### WEAVE-042: Integration test — anvil-weave violation remediation

- **Intent:** End-to-end test: kernel detects violation → anvil-weave receives
  trigger → agent queries graph → agent proposes remediation
- **Expected Outcome:** The full pipeline from detection to suggestion works
  with a mock provider
- **Validation:** `cargo test -p eddacraft-anvil-weave remediation_e2e`
- **Confidence:** medium
- **Priority:** High
- **Dependencies:** WEAVE-034, WEAVE-035

---

## Cross-Product Leverage (Future)

These are not tasks — they document how other products would consume
weave once the API stabilises.

| Product | Custom Tools | Use Case |
|---------|-------------|----------|
| Anvil | `graph_query`, `policy_eval` | Violation remediation, behavioural diff narration |
| Kindling | `memory_query`, `session_search` | Knowledge retrieval, pattern surfacing |
| Edda Stack | `ember_recall`, `knowledge_graph` | Memory-augmented agents |
| APS Tooling | `plan_read`, `plan_validate` | Plan-aware automation, spec validation |
| CI Pipeline | `diff_analyze`, `pr_comment` | Headless PR review, violation reporting |
| Personal | project-specific tools | General-purpose Rust agents |

## Performance Targets

| Metric | Target |
| ------ | ------ |
| Agent startup (no provider auth) | < 10ms |
| Tool dispatch overhead | < 1ms |
| Session append (JSONL) | < 5ms |
| Session context build (100 messages) | < 50ms |
| weave compile time (clean) | < 30s |
| weave binary size contribution | < 2MB |

## Stats

| Phase | Items | Status |
| ----- | ----- | ------ |
| 0 — Spike | 3 | Draft |
| 1 — Core Runtime | 7 | Draft |
| 2 — Providers | 2 | Draft |
| 3 — Anvil Integration | 6 | Draft |
| 4 — Validation | 3 | Draft |
| **Total** | **21** | **0/21** |
