---
id: overview
title: What anvil Does
description:
  anvil catches architecture drift and AI anti-patterns at save-time, before
  they reach code review.
sidebar_position: 1
---

# What anvil Does

anvil is a deterministic development automation platform that makes AI-generated
code changes safe for production.

## The Problem anvil Solves

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

## How anvil Works

anvil validates changes **at save-time**—before they reach review.

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│   Change    │ ──▶ │    Gate     │ ──▶ │  Evidence   │
│  (AI/Human) │     │ (Validate)  │     │  (Audit)    │
└─────────────┘     └─────────────┘     └─────────────┘
```

### 1. Watch Mode

anvil runs in the background, watching for file changes:

```bash
anvil watch
```

### 2. Gate Validation

When files change, anvil runs quality gates:

- **Architecture boundaries** — catches new dependency edges crossing contexts
- **Anti-pattern detection** — 13 built-in patterns (4 default, 9 opt-in)
- **Policy evaluation** — custom rules via OPA/Rego
- **Secret detection** — pattern + entropy analysis

### 3. Immediate Feedback

Issues surface instantly in your terminal or editor—not in a PR comment hours
later.

```
  ⚠ [AP-003] Explicit any type usage
    src/utils/parser.ts:42
    Consider using a more specific type or generic.
```

### 4. Evidence Trail

Every validation run produces evidence: what was checked, what passed, what
failed, and when.

## Key Features

| Feature                  | Description                                         |
| ------------------------ | --------------------------------------------------- |
| **Architecture Safety**  | Detects dependency violations using import analysis |
| **Anti-Pattern Library** | 13 built-in patterns (4 on by default, 9 opt-in)    |
| **Watch Mode**           | Real-time validation on file save                   |
| **Suppression System**   | Allow exceptions with mandatory explanations        |
| **GitHub Integration**   | PR checks and inline comments                       |
| **VS Code Extension**    | In-editor diagnostics and quick fixes               |

## Anti-Patterns Detected

### Default patterns (enabled out of the box)

| ID     | Pattern                      | Severity |
| ------ | ---------------------------- | -------- |
| AP-001 | Broad `/* eslint-disable */` | warning  |
| AP-003 | Explicit `any` type          | warning  |
| AP-004 | `@ts-ignore` directive       | warning  |
| AP-006 | Empty catch block            | warning  |

### Opt-in patterns

Enable these in your `.anvilrc` when relevant to your project.

| ID     | Pattern                        | Category     | Severity |
| ------ | ------------------------------ | ------------ | -------- |
| AP-002 | Rule-specific `eslint-disable` | escape hatch | info     |
| AP-005 | `@ts-expect-error` directive   | type safety  | info     |
| AP-007 | Console in production code     | code quality | info     |
| AP-008 | Inline `style` attribute       | HTML         | warning  |
| AP-009 | Inline `<script>` block        | HTML         | warning  |
| AP-010 | Inline event handler           | HTML         | warning  |
| AP-011 | Deprecated HTML tag            | HTML         | warning  |
| AP-012 | `!important` in CSS            | CSS          | warning  |
| AP-013 | CSS `@import`                  | CSS          | info     |

## What anvil Doesn't Do

anvil is focused. It doesn't:

- **Run your tests** — use your existing test runner
- **Format your code** — use Prettier/ESLint
- **Deploy your code** — use your existing CI/CD
- **Replace code review** — it augments review, not replaces it

anvil catches _structural_ and _architectural_ issues that other tools miss.

---

**Ready to start?** [Request access](https://eddacraft.ai/#waitlist) or
[go to the quickstart →](/anvil/quickstart) if you already have an invite.
