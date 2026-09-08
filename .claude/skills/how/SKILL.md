---
name: how
description: Explain how something works in this codebase by exploring code and producing a clear architectural explanation. Optionally critique the architecture for issues. Use when asked "how does X work?", "walk me through Y", "explain the Z subsystem", or "how is X structured? critique it."
---

# How

Explore the codebase to answer "how does X work?" questions. Produce clear architectural explanations at the level of a senior engineer onboarding onto a subsystem — enough to build a working mental model, not so much that it reads like annotated source code.

This skill is read-only: never edit files; report suggested changes as findings.

Two modes:

1. **Explain** (default) — explore the code and produce a clear explanation
2. **Critique** — explain first, then identify architectural issues

## Explain Mode

### Step 1 — Understand the question

Parse what the user is asking about. Common shapes:

- "How does message virtualization work?" — a subsystem
- "How do we handle billing for on-demand usage?" — a feature flow
- "How is the auth service structured?" — an architectural overview
- "Walk me through what happens when a user sends a message" — a runtime trace

Identify the scope. If it's ambiguous, state your interpretation before exploring. Don't ask — explore and let the user redirect if you're off.

**Assess complexity to decide the approach:**

- **Simple** — a single module, a small utility, a narrow question. One pass: read, then explain.
- **Complex** — a subsystem spanning multiple files/services, a cross-cutting feature, a full architectural overview. Decompose into 2–4 bounded angles and explore each in turn. If the active runtime exposes native delegation, the active harness adapter may assign read-only angles concurrently; otherwise keep them sequential.

When in doubt, lean toward the simple path — escalate if the first pass hits a wall.

### Step 2 — Explore

For each exploration angle:

- Start broad: enumerate relevant directories and use the harness's text search
  for key types, interfaces, and class names
- Follow the thread: trace the call chain — callers, callees, data flow, type definitions
- Read the actual code, don't guess from file names
- Stop when you can describe the full path from input to output without hand-waving
- Note things that are surprising, non-obvious, or that a newcomer would get wrong

Capture findings as you go: components found, flow traced, files read, anything non-obvious.

### Step 3 — Synthesise

If you explored multiple angles, reconcile findings into one coherent explanation. Drop duplicate observations. When two findings disagree, read the code one more time and resolve it explicitly.

### Step 4 — Present

Output format:

- **Overview** — 1–2 paragraphs. What is this, what does it do, why does it exist.
- **Key Concepts** — Important types, services, abstractions. Brief definitions.
- **How It Works** — The core explanation. Prose with file/function references. Include a Mermaid diagram for complex flows.
- **Where Things Live** — Brief file/directory map of the relevant code.
- **Gotchas** — Non-obvious things, sharp edges, historical context. Skip if nothing worth calling out.

## Critique Mode

Triggered when the user asks "how does X work? critique it" or "critique the architecture of Y" or explicitly says "critique" alongside a how question.

### Step 1 — Explain first

Run the full Explain mode first. The critique is grounded in the explanation.

### Step 2 — Critique

Critique through the lenses in [`references/critique-rubric.md`](references/critique-rubric.md) — the canonical rubric; not every lens applies to every subsystem. For added rigour, run two passes with different framings (for example, resilience then testability) — concurrently if the runtime supports native delegation, sequentially otherwise — and de-duplicate findings.

### Step 3 — Present findings

Group findings by severity:

- `structural` — fundamental architectural problems
- `concern` — real issues, not fundamental
- `observation` — tradeoffs, inconsistencies, technical debt

If the architecture is sound and you found nothing, say so — an empty critique is a valid outcome.

## References

Used only when delegating exploration or critique to subagents:

- Build each explorer prompt from `references/explorer-prompt.md`
- Synthesise multi-angle findings via `references/explainer-prompt.md`
- Build critic prompts from `references/critic-prompt.md`, with `references/critique-rubric.md` inlined

On the sequential (non-delegated) path, skip these prompt references entirely and follow the steps above directly, reading `references/critique-rubric.md` yourself for Critique mode.
