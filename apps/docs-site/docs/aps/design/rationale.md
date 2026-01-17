---
id: rationale
title: Design Rationale
description: Why APS is designed the way it is.
sidebar_position: 1
---

# Design Rationale

This page explains the design decisions behind APS.

## Core Principles

### 1. Intent Over Implementation

**Decision:** Plans describe _what_, not _how_.

**Rationale:**

- Implementation details change frequently
- Intent is stable and reviewable
- Executors (human or AI) can choose best approach
- Reduces coupling between plan and code

**Example:**

```markdown
❌ "Create middleware using Express that extracts Bearer token from
Authorization header, validates with jsonwebtoken..."

✓ "Auth middleware validates requests, attaches user to context"
```

### 2. Observable State Only

**Decision:** Steps describe verifiable conditions.

**Rationale:**

- Unverifiable steps can't be validated
- "Refactor X" is subjective
- "X has property Y" is testable
- Enables automated progress tracking

**Example:**

```markdown
❌ "Make the code cleaner" ✓ "Functions are under 50 lines"
```

### 3. Hash Stability

**Decision:** Content changes = hash changes. Format changes don't.

**Rationale:**

- Enables caching and deduplication
- Proves plan hasn't changed
- Supports audit requirements
- Allows formatting flexibility

### 4. Markdown-First

**Decision:** Plans are Markdown with YAML frontmatter.

**Rationale:**

- Human readable and writable
- Renders nicely on GitHub
- Easy to diff and review
- Familiar to developers
- No special tooling required to read

## Format Choices

### Why Markdown?

Alternatives considered:

- **YAML only** — verbose, less readable for prose
- **JSON** — not human-friendly for editing
- **Custom DSL** — learning curve, tooling needed

Markdown provides:

- Familiar syntax
- Built-in structure (headings, lists)
- Good tooling support
- GitHub rendering

### Why YAML Frontmatter?

Alternatives considered:

- **HTML comments** — parsing complexity
- **Code blocks** — conflicts with content
- **Separate files** — harder to keep in sync

YAML frontmatter:

- Standard pattern (Jekyll, Hugo, etc.)
- Clear separation of metadata
- Parser libraries available

### Why Task IDs?

Pattern: `AUTH-001`, `PAY-002`

Alternatives considered:

- **UUID** — not memorable, poor DX
- **Slugs** — collision risk, renaming issues
- **Numbers only** — no module context

Prefix + number provides:

- Module context at a glance
- Stable references
- Sortable within module
- Low collision risk

## Structural Decisions

### Four-Level Hierarchy

Index → Module → Task → Step

**Why not deeper?**

- Cognitive overhead increases
- Harder to navigate
- Diminishing returns

**Why not flatter?**

- Large projects need organisation
- Modules enable boundaries
- Steps enable progress tracking

### Module Boundaries

Modules can declare file scope:

```yaml
files:
  - src/auth/**
```

**Why optional?**

- Not all projects need enforcement
- Progressive adoption
- Simple projects stay simple

### Dependencies

Tasks can depend on other tasks:

```markdown
**Dependencies:**

- AUTH-001
```

**Why explicit?**

- Clear execution order
- Blockers visible
- Enables dependency graph
- Catches cycles

**Why not implicit?**

- Implicit deps are fragile
- Order of definition != dependency
- Explicit is auditable

## What APS Doesn't Do

### No Implementation Details

Plans don't specify _how_ to implement:

```markdown
❌ **Implementation:** 1. Create file src/auth/login.ts 2. Import express... 3.
...
```

**Why not?**

- Becomes outdated quickly
- Constrains executor
- Duplicates code
- Not the plan's job

### No Time Estimates

Plans don't include:

```markdown
❌ **Estimate:** 2 days
```

**Why not?**

- Estimates are notoriously unreliable
- Better tracked in project management
- Plans focus on _what_, not _when_
- Reduces plan maintenance

### No Assignments

Plans don't specify:

```markdown
❌ **Assignee:** @alice
```

**Why not?**

- Assignments change frequently
- Better tracked elsewhere (Jira, Linear)
- Plans are about work definition
- Reduces coupling to team structure

## Trade-offs

### Simplicity vs Expressiveness

APS chooses simplicity:

- Fewer concepts to learn
- Easier to adopt
- Harder to misuse

Cost:

- Can't express everything
- May need external tools
- Less flexibility

### Human-First vs Machine-First

APS is human-first:

- Readable without tools
- Editable in any text editor
- Reviewable in PRs

Cost:

- Parsing is more complex
- Less strict than pure data formats
- Validation needed

### Explicit vs Implicit

APS prefers explicit:

- Dependencies declared
- Outcomes stated
- Validation specified

Cost:

- More to write
- More to maintain
- Steeper initial effort

---

**Next:** [Alternatives considered →](/docs/aps/design/alternatives)
