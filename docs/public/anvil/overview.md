---
id: overview
title: What Anvil Does
description:
  Anvil catches architecture drift and AI anti-patterns at save-time, before
  they reach code review.
sidebar_position: 1
---

# What Anvil Does

Anvil is a deterministic development automation platform that makes AI-generated
code changes safe for production.

## The Problem Anvil Solves

AI coding assistants produce code that compiles and passes tests. But they also:

- **Drift from architecture** — introducing dependency edges that violate your
  intended boundaries
- **Introduce anti-patterns** — broad `eslint-disable`, explicit `any` types,
  empty catch blocks
- **Erode quality gradually** — each small compromise compounds over time

Code review _should_ catch these issues. But:

- Reviewers are overwhelmed by AI-generated volume
- Architectural violations are subtle and easy to miss
- By the time issues reach review, the cognitive load to fix them is high

## How Anvil Works

Anvil validates changes **at save-time**—before they reach review.

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│   Change    │ ──▶ │    Gate     │ ──▶ │  Evidence   │
│  (AI/Human) │     │ (Validate)  │     │  (Audit)    │
└─────────────┘     └─────────────┘     └─────────────┘
```

### 1. Watch Mode

Anvil runs in the background, watching for file changes:

```bash
anvil watch
```

### 2. Gate Validation

When files change, Anvil runs quality gates:

- **Architecture boundaries** — catches new dependency edges crossing contexts
- **Anti-pattern detection** — identifies 7 high-signal patterns
- **Policy evaluation** — custom rules via OPA/Rego
- **Secret detection** — pattern + entropy analysis

### 3. Immediate Feedback

Issues surface instantly in your terminal or editor—not in a PR comment hours
later.

```
⚠️  AP-003: Explicit 'any' type at src/utils/parser.ts:42
    Consider using a more specific type or generic.
```

### 4. Evidence Trail

Every validation run produces evidence: what was checked, what passed, what
failed, and when.

## Key Features

| Feature                  | Description                                         |
| ------------------------ | --------------------------------------------------- |
| **Architecture Safety**  | Detects dependency violations using import analysis |
| **Anti-Pattern Library** | 7 built-in patterns (AP-001 through AP-007)         |
| **Watch Mode**           | Real-time validation on file save                   |
| **Suppression System**   | Allow exceptions with mandatory explanations        |
| **GitHub Integration**   | PR checks and inline comments                       |
| **VS Code Extension**    | In-editor diagnostics and quick fixes               |

## Anti-Patterns Detected

| ID     | Pattern                      | Severity |
| ------ | ---------------------------- | -------- |
| AP-001 | Broad `/* eslint-disable */` | warning  |
| AP-003 | Explicit `any` type          | warning  |
| AP-004 | `@ts-ignore` directive       | warning  |
| AP-006 | Empty catch block            | warning  |
| AP-007 | Console in production code   | info     |

> AP-002 (rule-specific `eslint-disable`) and AP-005 (`@ts-expect-error`
> directive) are implemented opt-in patterns. They are disabled by default to
> reduce noise, but can be enabled explicitly in your Anvil configuration.

## What Anvil Doesn't Do

Anvil is focused. It doesn't:

- **Run your tests** — use your existing test runner
- **Format your code** — use Prettier/ESLint
- **Deploy your code** — use your existing CI/CD
- **Replace code review** — it augments review, not replaces it

Anvil catches _structural_ and _architectural_ issues that other tools miss.

---

**Ready to start?** [Request access](https://eddacraft.ai/#waitlist) or
[go to the quickstart →](/anvil/quickstart) if you already have an invite.
