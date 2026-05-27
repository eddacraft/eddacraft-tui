# weave: Internal Agent Harness

| Type | Authority | Owner | Status | Freshness                                        |
| ---- | --------- | ----- | ------ | ------------------------------------------------ |
| Spec | Advisory  | WEAVE | Draft  | Metadata backfilled 2026-05-24 during DOCGOV-009 |

| Upstream                                  | Downstream                             |
| ----------------------------------------- | -------------------------------------- |
| Rust kernel semantic graph, weave runtime | Future internal agent harness planning |

## Technical Feature Description (Internal Engineering)

### What Is This?

Two new Rust crates that embed LLM reasoning directly inside the Anvil binary:

- **weave** — A minimal, provider-agnostic agent runtime library. Apache-2.0
  licensed, zero Anvil dependencies, lives in a standalone open-source repo
  (`eddacraft/weave-rs`). Captures the irreducible kernel of an agent: message
  loop, tool dispatch, provider abstraction, session persistence. ~2500 LOC, ~15
  dependencies.

- **anvil-weave** — An Anvil-specific harness that depends on weave and adds
  tools with zero-copy access to the kernel's live semantic graph.
  Source-proprietary, same licence as the rest of Anvil.

### Why Does This Exist?

The Rust kernel maintains a persistent in-memory semantic graph — symbols,
dependency edges, trust boundaries, policy violations — with sub-millisecond
query times (800ns policy eval, 10us incremental update). Several high-value
features require LLM reasoning over this graph:

| Feature                    | What It Needs                                               |
| -------------------------- | ----------------------------------------------------------- |
| Violation remediation      | "Here's the violation. Here's the fix."                     |
| Behavioural diff narration | "This PR expanded the trust surface of your auth boundary." |
| Policy authoring           | "Describe what you want to prevent" -> working policy       |
| Headless CI review         | Agent-powered PR review without Claude Code                 |
| Plan-to-code alignment     | "Does this diff match the declared APS work item?"          |
| Onboarding / `anvil init`  | Agent analyses codebase, suggests initial rules             |

No external agent runtime can do this. Claude Code, Cursor, pi — they would all
need to serialise the graph, send it over a protocol, parse it, reason about it,
and send back a response. Anvil's internal agent skips all of that. It calls
`petgraph` methods directly on the kernel's live graph. Zero serialisation, zero
IPC, zero protocol overhead.

That is the architectural moat.

### How Does It Work?

The agent runtime is 7 primitives:

1. **Message** — user | assistant | tool_result
2. **Context** — system prompt + messages + tools (what the LLM sees)
3. **Tool** — name + JSON schema + async execute()
4. **Provider** — context -> async stream of events (Anthropic, OpenAI, etc.)
5. **Loop** — prompt -> stream -> extract tool calls -> execute -> repeat
6. **Session** — append-only JSONL tree (branching, context rebuilding)
7. **Event** — typed lifecycle notifications (agent_start, tool_call, etc.)

Everything is a trait. Consumers bring their own tools, providers, and session
backends. weave provides the loop and the types. anvil-weave provides Anvil's
specific tools.

### Crate Layout

```
eddacraft/weave-rs/         # Apache-2.0. ZERO eddacraft-anvil-* deps.
├── src/
│   ├── types.rs            # Message, Context, Model, StreamEvent
│   ├── tool.rs             # Tool trait, execution pipeline
│   ├── provider.rs         # Provider trait, registry
│   ├── stream.rs           # Streaming event normalisation
│   ├── agent.rs            # AgentState, event emission
│   ├── agent_loop.rs       # Two-level loop (tools + follow-ups)
│   └── session.rs          # SessionStore trait, JSONL-tree impl
└── Cargo.toml              # deps: serde, tokio, futures, uuid, thiserror

crates/anvil-weave/         # Source-proprietary. Depends on weave.
├── src/
│   ├── tools/
│   │   ├── graph_query.rs  # Query petgraph semantic graph
│   │   ├── policy_eval.rs  # Evaluate policy against live state
│   │   ├── read.rs         # File read
│   │   ├── edit.rs         # Propose edits
│   │   └── bash.rs         # Sandboxed shell
│   ├── triggers.rs         # Kernel event -> agent reasoning
│   ├── harness.rs          # Pre-wired Agent with Anvil tools
│   └── compaction.rs       # Graph-aware context compaction
└── Cargo.toml
```

### Key Design Decisions

**Trait-based, not framework-based.** No extension system, no plugin loading, no
dynamic dispatch. Rust's type system replaces all of that. You implement `Tool`,
you implement `Provider`, you wire them together.

**Two-level loop.** Inner loop: process tool calls and steering messages until
the model stops. Outer loop: check for follow-up messages and restart if needed.
Iterative, not recursive — no stack exhaustion risk.

**Read-only tool parallelisation.** Tools declare `is_readonly()`. Read-only
tools execute concurrently (up to N). Mutating tools execute sequentially. The
loop handles the scheduling.

**JSONL-tree sessions.** Each entry has an `id` and `parentId` forming a tree.
Branching moves the leaf pointer without modifying history. Compaction inserts a
summary node. Context building walks leaf-to-root. Append-only for auditability.

**Feature-gated providers.** Default build has no HTTP dependency. Enable
`provider-anthropic` or `provider-openai` features to pull in reqwest. This
keeps the base crate light for consumers who bring their own provider.

**Structural separation.** weave lives in its own repo (`eddacraft/weave-rs`),
which reduces accidental coupling but does not by itself prevent
`eddacraft-anvil-*` crates from being added as normal cargo dependencies.
Enforcement of the zero-dep invariant remains via automated CI / `cargo-deny`
checks.

### How to Contribute

- weave code goes in `eddacraft/weave-rs`. Never import from `eddacraft-anvil-*`
  crates. If you need an Anvil type, that code belongs in `anvil-weave` instead.
- All source files in weave carry Apache-2.0 headers.
- Provider implementations live behind feature gates.
- Follow the APS plan: `plans/modules/weave.aps.md` (WEAVE scope).
- Architecture decision: `plans/decisions/024-internal-agent-harness.md`.

### Cross-Product Leverage

weave is designed to be consumed by any EddaCraft product:

| Product     | Custom Tools                  | Use Case                       |
| ----------- | ----------------------------- | ------------------------------ |
| Anvil       | graph_query, policy_eval      | Remediation, behavioural diffs |
| Kindling    | memory_query, pattern_match   | Knowledge retrieval            |
| Edda Stack  | ember_recall, knowledge_graph | Memory-augmented agents        |
| CI Pipeline | diff_analyze, pr_comment      | Headless PR review             |

Same runtime, same session format, different tools.

---

## Marketing Brief

### Headline

**Anvil now understands your architecture — and explains what it finds.**

### The Shift

Today, Anvil catches violations. It says: _"cross-layer boundary violation in
payments.service.ts."_

That is valuable. But it leaves the developer asking: _why does this matter? how
do I fix it?_

With the agent harness, Anvil bridges that gap. When it catches a violation, it
can now reason about the structural context — what the violation means for your
architecture, which trust boundaries are affected, and what the fix looks like:

> **Violation:** `payments.service.ts` imports directly from `runtime/cache`.
>
> **Why it matters:** This bypasses the contracts layer, creating a direct
> dependency between the payments domain and an infrastructure concern. If the
> cache implementation changes, payments breaks silently.
>
> **Suggested fix:** Import `CacheKey` from `contracts/types` instead — it's
> already re-exported there.

That is the difference between a linter and a governance system that
_understands your codebase_.

### Key Differentiator

**Anvil is the only governance tool where the AI reasons over a semantic graph,
not text diffs.**

Other tools send your code to an LLM and ask "is this OK?" Anvil's agent has
direct access to a live structural model of your codebase — every symbol, every
dependency edge, every trust boundary, every policy rule — updated in
microseconds on every file save. It reasons about _structure and relationships_,
not just the text on screen.

This is not bolt-on AI. The semantic graph is the kernel. The agent is a
reasoning layer built on top of that kernel. They share the same memory space.
No serialisation, no protocol overhead, no approximation.

### Positioning (One Sentence)

Anvil catches architecture violations at save-time and now explains what they
mean and how to fix them — because its AI has direct access to a live semantic
model of your codebase.

### What This Unlocks (Customer-Facing)

| Capability             | Before                             | After                                                                                                   |
| ---------------------- | ---------------------------------- | ------------------------------------------------------------------------------------------------------- |
| **Violation response** | Warning message with file and line | Contextual explanation + suggested fix                                                                  |
| **PR review**          | Text-based diff review             | Behavioural diff: "this PR expanded the auth trust surface"                                             |
| **Policy authoring**   | Write Rego by hand                 | Describe intent in English, get a working policy                                                        |
| **Onboarding**         | Read docs, write config            | `anvil init` analyses your codebase and suggests rules                                                  |
| **CI integration**     | Pass/fail gate                     | Agent-powered review that posts structural findings on PRs                                              |
| **Drift explanation**  | "Boundary stress increasing"       | "Your payments module has added 4 new cross-layer imports this sprint, trending toward the cache layer" |

### Personas

**For developers:** Anvil stops being the tool that tells you "no" and starts
being the tool that tells you "here's why, and here's the fix." Faster
resolution, less friction, more trust in the guardrails.

**For platform engineers:** Policy authoring drops from "learn Rego and
understand the graph model" to "describe what you want to prevent." The agent
translates intent into working policy, validates it against the live graph, and
shows what it would catch.

**For engineering leaders:** PR reviews include structural impact analysis, not
just code diffs. "This PR adds 2 new cross-boundary dependencies and expands the
public API surface by 3 methods" — automatically, on every PR, in CI.

**For compliance teams:** Every agent reasoning session is persisted as an
auditable JSONL log. Who asked, what was analysed, what was suggested, what was
accepted. Provenance from detection through remediation.

### Narrative Arc (For Deck / Blog)

1. **The problem:** AI coding tools generate code fast but don't understand your
   architecture. Governance tools catch violations but leave developers stranded
   with cryptic warnings.

2. **The insight:** Anvil already maintains a live semantic graph of your
   codebase — symbols, dependencies, trust boundaries — updated in microseconds.
   What if an AI could reason directly over that graph?

3. **The solution:** An embedded agent runtime where the LLM has zero-copy
   access to the same structural model that powers enforcement. It doesn't
   approximate your architecture from text — it queries the actual graph.

4. **The result:** Violations come with explanations. PRs come with structural
   impact analysis. Policy authoring becomes conversational. Onboarding becomes
   automatic. And every interaction is auditable.

5. **The moat:** No external tool can replicate this. The graph lives in Anvil's
   memory. The agent lives in Anvil's binary. There is no serialisation layer,
   no API call, no approximation. The reasoning happens _inside the kernel_.

### Competitive Framing

| Tool                       | Approach                             | Limitation                                        |
| -------------------------- | ------------------------------------ | ------------------------------------------------- |
| SonarQube, Semgrep         | Pattern matching on AST              | No structural graph, no reasoning, no remediation |
| GitHub Copilot Code Review | LLM reads text diff                  | No architecture model, no policy awareness        |
| CodeScene                  | Historical analysis                  | Retrospective, not real-time; no enforcement      |
| Anvil + agent harness      | LLM reasons over live semantic graph | Real-time, structural, enforceable, auditable     |

### What This Is NOT

- Not a general-purpose AI coding assistant (it doesn't write features)
- Not replacing Claude Code or Cursor (it governs, they generate)
- Not making enforcement probabilistic (the kernel is deterministic; the agent
  explains and remediates)
- Not opt-in AI (the kernel enforces regardless; the agent layer is the
  value-add)

### Open Source Strategy

weave (the generic agent runtime) ships as Apache-2.0 open source in its own
standalone repo (`eddacraft/weave-rs`) from day one — not extracted later. It is
an opinion-free Rust crate that anyone can use to build agents. This positions
EddaCraft as a platform company, not just a product company, and creates
ecosystem gravity. The Anvil-specific harness (anvil-weave) remains proprietary.

### Timeline

Draft status. ADR-024 proposed (amended 2026-04-17 for standalone strategy). 21
work items across 5 phases. Estimate: 4-6 weeks to MVP (working agent loop with
Anvil graph tools). Marketing-ready when the first demo shows a violation ->
explanation -> fix flow end-to-end.
