# Positioning: Anvil by EddaCraft

## Category Definition

**Anvil is a pre-commit governance engine for AI-assisted codebases.**

It is not:
- A linter (it governs architecture, not style)
- A code review tool (it enforces policy, not suggestions)
- An AI tool (it is deterministic, not probabilistic)
- A security scanner (it governs provenance and structure, not just vulnerabilities)

It is:
- A deterministic policy engine
- A file-save enforcement layer
- An authorship attribution system
- An architecture drift detector
- A continuous compliance generator

---

## Positioning Statement

**For engineering teams shipping AI-assisted code**, Anvil is the **governance engine that enforces policy at file save** -- the only tool that provides **deterministic analysis, line-level authorship attribution, and architecture drift detection** before code reaches review. Unlike post-commit scanners and AI-powered review tools, Anvil is **predictable, auditable, and operates at the speed of generation**.

---

## Category-Level Competitive Framing

Anvil does not compete with individual tools. It competes with categories -- and it occupies a position none of them cover.

### How to Frame Each Category (Without Naming Competitors)

**When asked about static analysis tools:**
> "Static analysis tools scan after commit. They catch code-level issues in CI. Anvil operates at file save -- it prevents architecture drift and enforces governance before code enters the pipeline. They are complementary, not competitive."

**When asked about AI code review tools:**
> "AI-powered code review uses one AI model to evaluate another AI model's output. That compounds uncertainty. Anvil is deterministic -- policy-as-code rules that produce the same result every time. We do not suggest; we enforce."

**When asked about supply chain security tools:**
> "Supply chain tools protect against malicious or vulnerable dependencies. Anvil governs what is generated inside the repository -- who or what wrote each line, whether the architecture is sound, and whether policy is met. Different attack surface."

**When asked about enterprise GRC platforms:**
> "Enterprise GRC operates at the organisational level -- risk registers, audit workflows, board reporting. Anvil is the developer-facing governance layer that generates the compliance evidence those platforms consume."

---

## The 2x2 That Defines the Category

```
                    Pre-commit                    Post-commit
                    (file save)                   (PR / CI)
                    ─────────────────────────────────────────
Deterministic       │ ANVIL              │ Static Analysis  │
(policy-based)      │                    │ (SonarQube,      │
                    │                    │  Semgrep, etc.)  │
                    ├────────────────────┼──────────────────┤
Probabilistic       │ [empty]            │ AI Code Review   │
(AI-powered)        │                    │ (various)        │
                    ─────────────────────────────────────────
```

**Anvil is the only player in the top-left quadrant.** This is the positioning anchor for the entire deck.

### Why the Top-Left Is Defensible

1. **Architectural moat**: Moving from post-commit to pre-commit requires fundamental re-architecture. Existing tools cannot bolt this on.
2. **Philosophy moat**: Being deterministic when your business model is built on AI review requires abandoning your core product thesis.
3. **Data moat**: Authorship attribution requires deep workflow integration. Retroactive attribution is unreliable.

---

## Audience-Specific Positioning

### For Investors
"Anvil is to AI-generated code what SonarQube was to human code -- the quality gate. But it operates earlier (file save vs CI), covers more (authorship + architecture + policy), and addresses a regulatory forcing function (EU AI Act August 2026) that creates mandatory spend."

### For CTOs / Engineering Leaders
"Your team is shipping AI-generated code at scale. Anvil tells you which lines are human, which are AI, and whether they meet your architecture and policy standards -- at the moment they are written, not after they are committed."

### For Compliance / Security Leaders
"The EU AI Act requires traceability of AI systems. Anvil provides line-level provenance, continuous policy enforcement, and a real-time audit trail. Compliance evidence is generated as a by-product of development."

### For Developers
"Anvil is the governance layer that runs in your terminal. It catches architecture drift and policy violations at file save -- before your PR is even opened. Policy-as-code (OPA/Rego) means your team controls the rules."

---

## Positioning Guardrails

### Do Say
- "Deterministic governance for AI-assisted codebases"
- "Policy enforcement at file save"
- "Line-level authorship attribution"
- "Architecture drift detection"
- "The adult in the room"
- "A flight recorder, not a chat app"

### Do Not Say
- "AI-powered" (Anvil is deliberately not AI)
- "Replaces code review" (Anvil complements human review)
- "Better than [competitor name]" (frame by category, not by name)
- "Revolutionary" / "game-changing" / "cutting-edge" (brand voice is understated)
- "Replaces your linter" (Anvil governs architecture, not style)

### Objection Handling

| Objection | Response |
|-----------|----------|
| "We already use SonarQube" | "Great -- Anvil is complementary. SonarQube scans after commit for code quality. Anvil governs at file save for architecture, authorship, and policy. They cover different stages and different concerns." |
| "Can't AI review tools do this?" | "AI review tools use probabilistic models to suggest improvements. Anvil uses deterministic policy-as-code to enforce governance. One suggests, the other enforces. One compounds AI uncertainty, the other eliminates it." |
| "Is this just another linter?" | "Linters check syntax and style. Anvil tracks who wrote each line of code, whether the architecture is drifting, and whether team policy is met. It is a governance engine, not a style checker." |
| "Do we need this if we have good code review?" | "Code review catches what reviewers notice. AI generates code faster than humans can review it. 46% of code is AI-generated, and fewer than half of developers review it before committing. Manual review does not scale." |
| "The EU AI Act does not apply to us" | "Regulatory pressure creates market pressure. Even if your organisation is not directly regulated, your enterprise customers and partners increasingly require evidence of AI code governance." |
