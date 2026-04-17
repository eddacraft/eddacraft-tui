# weave-rs Standalone Agent Runtime

**Date:** 2026-04-17
**Branch:** feat/pi
**Amends:** ADR-024 (Internal Agent Harness)

## Problem

ADR-024 placed `literate-core` inside the Anvil monorepo with an "extract
later" strategy. The extractability invariant (zero anvil-* deps) guaranteed
this was always possible via `git subtree split`.

Since then, two factors changed:

1. **Earlier cross-product need** — other EddaCraft products and personal
   projects need a Rust agent runtime sooner than originally expected.
2. **Stable trait surface** — the 7 primitives (Message, Context, Tool,
   Provider, Loop, Session, Event) are well-understood from pi-mono analysis.
   The core traits will settle fast; churn will be in implementations, not
   signatures.

Keeping the crate in the monorepo means external consumers would need git
subtree splits, path overrides, or waiting for extraction. Since the API is
expected to stabilise quickly, the "extract later" trade-off no longer pays for
itself.

## Decision

Build `weave` as a standalone repo (`eddacraft/weave-rs`) from day one.

The Anvil-specific harness is renamed from `anvil-agent` to `anvil-weave` and
stays in the Anvil monorepo at `crates/anvil-weave/`. The APS scope tag is
renamed from `LCORE` to `WEAVE`.

## Design

### Naming

| Concept | Old name | New name |
|---------|----------|----------|
| Generic agent runtime crate | `literate-core` | `weave` |
| Crate repo | `crates/literate-core/` (monorepo) | `eddacraft/weave-rs` (standalone) |
| Anvil-specific harness | `anvil-agent` | `anvil-weave` |
| APS scope tag | `LCORE` | `WEAVE` |
| crates.io package | `literate-core` (planned) | `weave` |

### Repo structure — eddacraft/weave-rs

```
eddacraft/weave-rs/
├── src/
│   ├── lib.rs            # Public API re-exports
│   ├── types.rs          # Message, Content, Context, Model, StreamEvent
│   ├── tool.rs           # Tool trait, ToolResult, execution pipeline
│   ├── provider.rs       # Provider trait, registry
│   ├── stream.rs         # Streaming event types, async Stream normalisation
│   ├── agent.rs          # AgentState, event emission, steering/follow-up
│   ├── agent_loop.rs     # Two-level loop (inner: tool calls, outer: follow-ups)
│   └── session.rs        # SessionStore trait, default JSONL-tree impl
├── Cargo.toml
├── LICENSE               # Apache-2.0
├── README.md
└── .github/
    └── workflows/
        └── ci.yml        # check, test, clippy, dep-audit
```

~15 direct dependencies: serde, serde_json, tokio, futures, uuid, thiserror,
async-trait, tokio-util. Provider implementations (Anthropic, OpenAI) are
feature-gated behind optional `reqwest` dependency.

Default build has no HTTP dependency. Enable `provider-anthropic` or
`provider-openai` features to pull in reqwest.

### Crate layout — crates/anvil-weave/ (Anvil monorepo)

```
crates/anvil-weave/
├── src/
│   ├── tools/
│   │   ├── graph_query.rs    # Query petgraph semantic graph (zero-copy)
│   │   ├── policy_eval.rs    # Evaluate policy against current state
│   │   ├── read.rs           # File read
│   │   ├── edit.rs           # Propose edits
│   │   └── bash.rs           # Sandboxed shell execution
│   ├── harness.rs            # Pre-wired Agent with Anvil tools
│   ├── triggers.rs           # Kernel event → agent reasoning triggers
│   └── compaction.rs         # Graph-aware context compaction
└── Cargo.toml                # Depends on weave + anvil-kernel-types
```

Source-proprietary, same licence as the rest of Anvil.

### Core architecture (unchanged from ADR-024)

The 7 irreducible primitives carry over exactly:

1. **Message** — user | assistant | tool_result
2. **Context** — system_prompt + messages + tools (what the LLM sees)
3. **Tool** — name + JSON schema + async execute()
4. **Provider** — context → async stream of events
5. **Loop** — prompt → stream → extract tool calls → execute → repeat
6. **Session** — append-only JSONL tree (branching, context rebuilding)
7. **Event** — typed lifecycle notifications

Core traits (`Provider`, `Tool`, `SessionStore`, `EventHandler`) are unchanged.
See ADR-024 for full trait signatures.

**Two-level loop.** Inner loop: process tool calls and steering messages until
the model stops. Outer loop: check for follow-up messages and restart if needed.

**Read-only tool parallelisation.** Tools declare `is_readonly()`. Read-only
tools execute concurrently. Mutating tools execute sequentially.

**JSONL-tree sessions.** Append-only, branch-by-pointer, leaf-to-root context
building. Compaction inserts summary nodes.

### Dependency strategy

Three phases:

**Phase 1 — Active dev (spike):** Path dependency. Local checkout of both repos.

```toml
# crates/anvil-weave/Cargo.toml
[dependencies]
weave = { path = "../../weave-rs" }
```

Anvil CI clones `weave-rs` at a pinned git rev before `cargo build`. Other
consumers (personal projects, sibling EddaCraft repos) use git dep:

```toml
weave = { git = "https://github.com/eddacraft/weave-rs", rev = "abc1234" }
```

**Phase 2 — Pre-release (post-spike):** Publish `0.1.0-alpha.N` to crates.io.
Consumers switch to version deps. Breaking changes are versioned.

**Phase 3 — Stable:** Semver releases. `weave = "0.1"` everywhere.

### Zero-dep invariant

Naturally enforced by repository separation. `weave-rs` cannot import `anvil-*`
crates because they don't exist in its dependency graph. No CI cargo-metadata
check needed — the invariant is structural.

### Spike scope

The spike (WEAVE-001 through WEAVE-003) happens entirely in `weave-rs`:

1. Scaffold the crate with types, traits, and a no-op test provider
2. Implement the two-level agent loop with tool dispatch
3. End-to-end test: test provider + dummy tools → assert correct message flow

`anvil-weave` is scaffolded in the Anvil monorepo only after the spike lands.
It is the first real consumer, not a co-development target during the spike.

### Cross-product leverage (unchanged)

| Product     | Custom Tools                  | Use Case                       |
| ----------- | ----------------------------- | ------------------------------ |
| Anvil       | graph_query, policy_eval      | Remediation, behavioural diffs |
| Kindling    | memory_query, pattern_match   | Knowledge retrieval            |
| Edda Stack  | ember_recall, knowledge_graph | Memory-augmented agents        |
| CI Pipeline | diff_analyze, pr_comment      | Headless PR review             |
| Personal    | project-specific tools        | General-purpose Rust agents    |

## What changes vs ADR-024

| Aspect | ADR-024 (current) | This spec |
|--------|-------------------|-----------|
| Crate name | `literate-core` | `weave` |
| Repo location | `crates/literate-core/` in Anvil monorepo | `eddacraft/weave-rs` standalone |
| Harness name | `anvil-agent` | `anvil-weave` |
| APS scope | `LCORE` | `WEAVE` |
| Extraction strategy | Extract later via `git subtree split` | Standalone from day one |
| Zero-dep enforcement | CI cargo-metadata check | Structural (separate repo) |
| Publish strategy | When API stable | Pre-releases early |

## What does NOT change

- 7 primitives and core trait signatures
- Feature-gated providers (no HTTP by default)
- Two-level loop architecture
- JSONL-tree session format
- Read-only tool parallelisation
- Apache-2.0 licence for the generic runtime
- Source-proprietary licence for anvil-weave
- anvil-weave domain tools (graph_query, policy_eval, etc.)
- Cross-product leverage strategy

## Artefacts to update

1. **ADR-024** — amend in place: naming, repo location, distribution strategy
2. **plans/modules/literate-core.aps.md** — rename scope LCORE → WEAVE, update
   file paths to reference `eddacraft/weave-rs` instead of `crates/literate-core/`
3. **plans/index.aps.md** — update Agent Infrastructure table entry
4. **docs/internal/literate-core-feature-brief.md** — update naming throughout

## Risks

| Risk | Impact | Mitigation |
|------|--------|------------|
| API churn visible to external consumers during alpha | Low (pre-release semver) | Use `0.1.0-alpha.N` releases, document instability |
| Two-repo coordination overhead | Medium | Path dep during spike eliminates friction; only matters at release boundaries |
| crates.io name `weave` may be taken | Low | Check availability; fallback: `weave-agent`, `weave-core` |
| Personal projects diverge weave's direction from Anvil's needs | Low | Anvil-specific concerns go in anvil-weave, not weave |
