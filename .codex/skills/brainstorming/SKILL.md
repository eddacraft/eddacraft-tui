---
name: brainstorming
description: Use before any creative work — creating features, building components, adding functionality, or modifying behaviour. Explores intent, requirements, and design before implementation. Hard gate — no code until design is approved.
---

# Brainstorming

Explore intent and produce an approved design before touching code.

## Hard Gate

Do NOT write code, scaffold files, or invoke any implementation tool until the user has approved a design. No exceptions — even for "simple" tasks.

## Process (in order)

1. **Read project context** — files, docs, recent commits
2. **Ask clarifying questions** — one at a time; purpose, constraints, success criteria
3. **Propose 2–3 approaches** — with trade-offs and your recommendation
4. **Present design** — section by section, get approval at each stage
5. **Write design doc** — save to `plans/specs/YYYY-MM-DD-<topic>.md`, commit
6. **Hand off to `writing-plans`** — this is the only valid next step

## Decomposition Rule

If the request spans multiple independent subsystems, flag it before asking detailed questions. Help the user decompose into sub-projects first. Each sub-project gets its own spec → plan → implementation cycle.

## Design Principles

- One question per message
- Prefer multiple-choice over open-ended
- Propose alternatives before recommending
- Design for isolation: each unit has one clear purpose and a well-defined interface
- YAGNI: remove anything not needed for the stated goal
- In existing codebases: follow established patterns; don't refactor beyond the current goal

## Terminal State

The only valid exit from brainstorming is invoking the `writing-plans` skill. Do not invoke any other skill or write any code.
