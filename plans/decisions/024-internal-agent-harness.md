# ADR-024: Internal Agent Harness (weave)

## Status

Proposed

### Amendment — 2026-04-17

Naming and hosting changed per `plans/specs/2026-04-17-weave-rs-standalone-design.md`:

- **literate-core** → **weave** (crate name), repo `eddacraft/weave-rs` (standalone)
- **anvil-agent** → **anvil-weave** (harness crate in monorepo)
- **APS scope** LCORE → WEAVE
- **Hosting** changed from monorepo-first to standalone from day one
- **Distribution** changed from extract-later to path dep → pre-releases → crates.io
- **Zero-dep invariant** tightened to **no `eddacraft-anvil-*` dependencies** in `weave`; standalone hosting reduces accidental coupling, but enforcement remains via automated CI / `cargo-deny` checks

## Date

2026-04-14

## Context

Anvil's Rust kernel maintains a persistent in-memory semantic graph with
sub-millisecond query times (800ns policy evaluation, 10μs incremental update).
Several aspirational features — violation remediation, behavioural diff
narration, policy authoring assistance, plan-to-code alignment, CI/CD headless
review — require LLM reasoning over this graph. No external agent runtime
(Claude Code, Cursor, pi) can access the kernel's live graph without
serialisation and protocol overhead.

Separately, the EddaCraft product family (Anvil, Kindling, Edda Stack, APS
tooling) would benefit from a shared, minimal agent runtime rather than each
product building ad-hoc LLM integration.

Two existing Rust implementations were evaluated:

- **pi-mono** (TypeScript, badlogic/pi-mono): Well-architected minimal agent
  with a ~3600 LOC irreducible core, but TypeScript-only.
- **pi_agent_rust** (Dicklesworthstone/pi_agent_rust): Rust port of pi, but
  heavily opinionated (Anthropic-first defaults, embedded QuickJS runtime,
  600-800 transitive crates, CLI-shaped not library-shaped, SDK contract
  incomplete). Not suitable as a dependency.

The gap: no minimal, opinion-free, library-first Rust agent runtime exists that
can be embedded in other binaries and composed with domain-specific tools.

## Decision

Build **weave**, a thin, provider-agnostic agent runtime crate in the standalone
repo `eddacraft/weave-rs`. It captures the irreducible kernel of an agent
runtime (message loop, tool dispatch, provider abstraction, session persistence)
with zero opinions about which LLM, which tools, or where sessions live.

Build **anvil-weave**, an Anvil-specific harness at `crates/anvil-weave/` that
depends on weave and adds domain tools (`graph_query`, `policy_eval`) with
direct, zero-copy access to the kernel's semantic graph.

### Key constraints

1. **Zero Anvil dependencies in weave** — the dependency arrow only points
   inward. `weave` must never import any `anvil-*` crate. This is structurally
   enforced by the separate repo boundary.
2. **Apache-2.0 licence** — weave is Apache-2.0 from commit one, distinct from
   Anvil's source-proprietary licence. This aligns with ADR-018's designation
   of foundational repos as OSS.
3. **Standalone from day one** — `eddacraft/weave-rs` is a separate repo from
   the start. Anvil consumes it via path dep during active development,
   transitioning to pre-releases and then crates.io as the API stabilises.
4. **~15 direct dependencies** — serde, tokio, futures, uuid, thiserror,
   async-trait, tokio-util, serde_json. Provider implementations (Anthropic,
   OpenAI) are feature-gated behind optional `reqwest` dependency.
5. **Trait-based composition** — `Provider`, `Tool`, `SessionStore`,
   `EventHandler` traits. No built-in tools, no built-in providers in the
   default feature set. Consumers bring their own.

### Architecture

```
weave (Apache-2.0, standalone repo: eddacraft/weave-rs, zero anvil-* deps)
├── types       — Message, Content, Context, Model, StreamEvent
├── tool        — Tool trait, ToolResult, execution pipeline
├── provider    — Provider trait, registry, feature-gated impls
├── stream      — Streaming event types, async Stream normalisation
├── agent       — AgentState, event emission, steering/follow-up queues
├── agent_loop  — Two-level loop (inner: tool calls, outer: follow-ups)
└── session     — SessionStore trait, default JSONL-tree implementation

anvil-weave (source-proprietary, depends on weave + anvil-kernel-types)
├── tools/
│   ├── graph_query   — Query semantic graph (petgraph, zero-copy)
│   ├── policy_eval   — Evaluate policy against current state
│   ├── read / edit   — Standard file operations
│   └── bash          — Sandboxed shell execution
├── harness           — Wire tools + provider + session → Agent
├── triggers          — Kernel event → agent reasoning triggers
└── compaction        — Anvil-specific context compaction strategy
```

### The 7 primitives (from pi-mono analysis)

An agent runtime is exactly 7 things:

1. **Message** — the unit of conversation (user | assistant | tool_result)
2. **Context** — system_prompt + messages + tools → what the LLM sees
3. **Tool** — name + schema + execute() → how the agent acts
4. **Provider** — context → stream of events → how the agent thinks
5. **Loop** — prompt → stream → extract tool calls → execute → repeat
6. **Session** — append-only message log → how the agent remembers
7. **Event** — typed notifications → how the agent communicates

Everything else is specialisation built on these primitives.

### Core traits

```rust
#[async_trait]
pub trait Provider: Send + Sync {
    async fn stream(
        &self,
        context: &Context,
        options: &StreamOptions,
    ) -> Result<Pin<Box<dyn Stream<Item = StreamEvent> + Send>>>;
}

#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn parameters(&self) -> &serde_json::Value;
    fn is_readonly(&self) -> bool { false }
    async fn execute(
        &self,
        call_id: &str,
        params: serde_json::Value,
        signal: CancellationToken,
    ) -> ToolResult;
}

#[async_trait]
pub trait SessionStore: Send + Sync {
    async fn append(&self, entry: SessionEntry) -> Result<EntryId>;
    async fn build_context(&self, leaf: EntryId) -> Result<Vec<Message>>;
    async fn branch(&self, from: EntryId) -> Result<EntryId>;
}

pub trait EventHandler: Send + Sync {
    fn on_event(&self, event: AgentEvent);
}
```

## Rationale

### Why not use pi_agent_rust as a dependency?

| Issue | Impact |
|-------|--------|
| 600-800 transitive crates (QuickJS, SWC, image codecs) | Supply chain risk, 5-10 min builds |
| CLI-shaped, not library-shaped | Session storage hardcoded to `~/.pi/agent/`, interactive auth flows |
| Tight coupling (loop ↔ session ↔ extension runtime) | Cannot use one layer without pulling all three |
| Anthropic-first opinions | Default model baked in, provider coverage biased |
| SDK contract incomplete | `create_agent_session()` not fully specified, RPC parity gaps |

### Why standalone from day one?

| Factor | Standalone OSS | Monorepo |
|--------|----------------|----------|
| Multiple consumers (EddaCraft products, personal projects) | Ships immediately, shared via crates.io path | Requires extract before other repos can consume |
| Zero-dep invariant | Structurally enforced by repo boundary | Requires CI cargo-metadata check discipline |
| API surface | Stable 7 primitives from pi-mono analysis | Same, but harder to signal stability |
| OSS positioning | Immediate | Delayed (extract when stable) |
| Cross-boundary refactors | Two repos, two PRs (acceptable given stable primitives) | One PR, one CI |

Standalone wins given multiple consumers need weave sooner than expected and the
core trait surface is stable (7 primitives from pi-mono analysis). The separate
repo is the pressure valve — Anvil-specific concerns stay in `anvil-weave`,
never in `weave`.

### Alternatives Considered

| Option | Pros | Cons |
|--------|------|------|
| **weave standalone** (chosen) | Clean separation, immediate OSS, multiple consumers, structural zero-dep enforcement | Two repos, two PRs for cross-boundary refactors |
| pi_agent_rust as dependency | No build effort | 600+ crates, CLI-shaped, opinionated, not embeddable |
| weave in monorepo (original) | Velocity, co-evolution | Cannot serve other consumers without extract; zero-dep relies on CI discipline |
| No internal harness (external agents only) | No build effort | Cannot access kernel graph directly, MCP overhead, no headless CI agent |
| TypeScript agent harness | Matches existing TS orchestration layer | Cannot call petgraph directly, FFI overhead, two runtimes in one binary |

## Consequences

- **Positive:** Anvil gains LLM reasoning with zero-copy graph access. Products
  share a common runtime. Apache-2.0 crate is a candidate for ADR-018's OSS
  foundational repos. CI/CD review works without Claude Code.
- **Positive:** The `graph_query` tool gives agents structural reasoning over
  a semantic graph — a capability no external agent runtime can replicate.
- **Positive:** `eddacraft/weave-rs` is immediately available to other EddaCraft
  products and personal projects without a monorepo extraction step.
- **Negative:** ~4-6 weeks of build effort for weave + anvil-weave MVP.
- **Negative:** Cross-boundary refactors require PRs in two repos.
- **Risk:** API may not stabilise quickly if Anvil's needs diverge from generic
  agent patterns.
- **Mitigation:** Anvil-specific concerns go in `anvil-weave`, never in `weave`.
  The boundary is the pressure valve.

## References

- Related ADRs: ADR-018 (product IP architecture, OSS foundational repos),
  ADR-012 (Rust CLI replacement), ADR-014 (TS vs Rust allocation),
  ADR-015 (intercept loop enforcement)
- APS modules: WEAVE (weave implementation), KERN (Rust kernel)
- External: [pi-mono](https://github.com/badlogic/pi-mono) (architecture
  reference), [pi_agent_rust](https://github.com/Dicklesworthstone/pi_agent_rust)
  (evaluated, rejected as dependency)
- Vision: `docs/vision/aspirational-ultimate-feature.md` (behavioural diff,
  plan-aware watching, provenance narration)
- Design spec: `plans/specs/2026-04-17-weave-rs-standalone-design.md`
