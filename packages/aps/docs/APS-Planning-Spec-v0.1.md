# APS Planning Spec v0.1

> **Anvil Planning Spec (APS)** — A Markdown-based format for LLM-readable
> planning documents.

## Overview

The Anvil Planning Spec (APS) defines a structured Markdown format for planning
documents that are:

- **Human-readable** — Plain Markdown, familiar conventions
- **LLM-friendly** — Structured with clear headings and key-value fields
- **Graph-aware** — Supports modular plans with dependencies
- **Execution-ready** — Bridges from planning to task execution

### Design Goals

1. **Single source of truth** — Planning docs are canonical, not derived
2. **Separation of concerns** — Planning (source) vs Execution (derived state)
3. **Validation-first** — Clear rules, early feedback
4. **Scoped context** — LLMs see only what they need

### Non-Goals

See [APS-NonGoals.md](./APS-NonGoals.md) for explicit non-goals.

---

## Document Types

APS supports two document types:

1. **Leaf Spec** — Contains tasks for a single module or feature
2. **Index File** — Organises multiple leaf specs into a plan graph

### Single-File Plans

For simple plans, a single leaf spec is sufficient:

```markdown
# Feature: User Authentication

**Scope:** AUTH **Owner:** @alice **Priority:** high

## Tasks

### AUTH-001: Implement login endpoint

...
```

### Multi-File Plans

For complex plans, use an index file + leaf specs:

```
docs/planning/
  system.aps.md           # Index file
  modules/
    auth.aps.md           # Leaf spec
    payments.aps.md       # Leaf spec
```

---

## Index File Structure

Index files organise leaf specs and define module metadata.

### Required Sections

#### `# [Plan Title]` (H1)

The plan title (H1 heading). Only one H1 per file.

#### `## Modules` (H2)

Lists modules with metadata. Each module is an H3 heading.

**Example:**

```markdown
## Modules

### auth

- **Path:** [./modules/auth.aps.md](./modules/auth.aps.md)
- **Scope:** AUTH
- **Owner:** @alice
- **Priority:** high
- **Tags:** security, core
- **Dependencies:** (none)

### payments

- **Path:** [./modules/payments.aps.md](./modules/payments.aps.md)
- **Scope:** PAY
- **Owner:** @bob
- **Priority:** medium
- **Tags:** billing, stripe
- **Dependencies:** auth
```

### Module Metadata Fields

| Field             | Required | Format                       | Purpose                 |
| ----------------- | -------- | ---------------------------- | ----------------------- |
| **Path:**         | Yes      | Markdown link                | Link to leaf spec file  |
| **Scope:**        | No       | Uppercase prefix (e.g. AUTH) | Namespace for task IDs  |
| **Owner:**        | No       | @username                    | Person/team responsible |
| **Priority:**     | No       | `low`, `medium`, `high`      | Priority level          |
| **Tags:**         | No       | Comma-separated list         | Labels for filtering    |
| **Dependencies:** | No       | Comma-separated module IDs   | Modules this depends on |

### Optional Sections

- **`## Overview`** — High-level description
- **`## Open Questions`** — Unresolved questions
- **`## Decisions`** — Architectural decisions with dates

---

## Leaf Spec Structure

Leaf specs contain tasks for a single module or feature.

### Required Sections

#### `# [Module Title]` (H1)

The module title (H1 heading). Only one H1 per file.

#### Module Metadata Line

Immediately after the H1, include scope/owner/priority:

```markdown
# Authentication Module

**Scope:** AUTH **Owner:** @alice **Priority:** high
```

#### `## Tasks` (H2)

Contains task definitions. Each task is an H3 heading.

### Task Structure

Each task follows this format:

```markdown
### AUTH-001: Implement login endpoint

**Intent:** Create POST /auth/login endpoint that validates credentials and
returns JWT **Expected Outcome:** Working login endpoint with tests
**Confidence:** high **Scopes:** AUTH **Tags:** security, api **Dependencies:**
DB-001 **Inputs:**

- User credentials (email, password)
- Database connection

**Status:** open
```

### Task Fields

| Field                 | Required | Format                   | Purpose                               |
| --------------------- | -------- | ------------------------ | ------------------------------------- |
| **Intent:**           | Yes      | Single sentence          | What the task aims to achieve         |
| **Expected Outcome:** | No       | Text                     | Success criteria                      |
| **Confidence:**       | No       | `low`, `medium`, `high`  | Certainty about approach              |
| **Scopes:**           | No       | Comma-separated prefixes | What can be changed (LLM constraints) |
| **Tags:**             | No       | Comma-separated labels   | For filtering and search              |
| **Dependencies:**     | No       | Comma-separated task IDs | Tasks that must complete first        |
| **Inputs:**           | No       | Bulleted list            | Required inputs or context            |
| **Status:**           | No       | See status values below  | Current execution state               |

### Task Status Values

| Status      | Meaning                                 |
| ----------- | --------------------------------------- |
| `open`      | Not started (default)                   |
| `locked`    | Being worked on (via `anvil plan lock`) |
| `completed` | Finished successfully                   |
| `cancelled` | Abandoned or no longer needed           |

**Note:** Status is managed externally in `.anvil/state.json`, not in the
planning doc.

### Task ID Format

Task IDs must follow the format: `<SCOPE>-<NUMBER>`

**Examples:**

- `AUTH-001` ✓
- `PAY-042` ✓
- `auth-001` ✗ (lowercase scope)
- `AUTH001` ✗ (missing hyphen)

Task IDs must be unique across the entire plan graph.

### Optional Sections

- **`## Dependencies`** — Module-level dependencies
- **`## Notes`** — Additional context

---

## Conventions

See [APS-Conventions.md](./APS-Conventions.md) for detailed conventions on:

- File naming (`.aps.md` extension)
- Link rules (relative paths, in-repo only)
- ID uniqueness (error on duplicates)
- Heading hierarchy
- Root file discovery

---

## Scopes vs Tags

**Scopes** and **Tags** serve different purposes:

### Scopes (Hard Boundaries)

Scopes define **what code/files can be changed** during task execution. They
constrain the LLM's operating context.

**Example:**

```markdown
**Scopes:** AUTH, DB
```

This means the LLM can only modify files in the `AUTH` and `DB` scopes.

### Tags (Soft Labels)

Tags are labels for **filtering and search**. They don't constrain behaviour.

**Example:**

```markdown
**Tags:** security, high-risk, needs-review
```

This helps filter tasks but doesn't restrict what can be changed.

---

## Validation Rules

APS documents must pass validation before execution:

### Errors (Blocking)

- Missing required sections (`## Modules`, `## Tasks`)
- Missing required fields (`Path:` in modules, `Intent:` in tasks)
- Invalid task ID format
- Duplicate task IDs across plan graph
- Broken links
- Circular module dependencies

### Warnings (Non-blocking)

- Missing Confidence field
- Scope mismatch (task scope not in parent module scope)
- Orphan leaf specs (not referenced from index)

See Phase 6 implementation for full validation rules.

---

## Example: Simple Feature Plan

```markdown
# Feature: User Authentication

**Scope:** AUTH **Owner:** @alice **Priority:** high

> Implement basic username/password authentication.

## Tasks

### AUTH-001: Create user model

**Intent:** Define User model with email, password hash, created_at **Expected
Outcome:** User model with validation and tests **Confidence:** high **Scopes:**
AUTH, DB **Tags:** models, database

### AUTH-002: Implement login endpoint

**Intent:** Create POST /auth/login that validates credentials **Expected
Outcome:** Working endpoint with JWT generation **Confidence:** high **Scopes:**
AUTH **Tags:** api, security **Dependencies:** AUTH-001

## Notes

- Consider OAuth in future iteration
```

---

## Example: Multi-Module Plan

**Index file:** `docs/planning/system.aps.md`

```markdown
# System: E-commerce Platform

## Overview

Build core e-commerce functionality.

## Modules

### auth

- **Path:** [./modules/auth.aps.md](./modules/auth.aps.md)
- **Scope:** AUTH
- **Owner:** @alice
- **Priority:** high
- **Tags:** security, core

### payments

- **Path:** [./modules/payments.aps.md](./modules/payments.aps.md)
- **Scope:** PAY
- **Owner:** @bob
- **Priority:** medium
- **Tags:** billing, stripe
- **Dependencies:** auth
```

**Leaf file:** `docs/planning/modules/auth.aps.md`

```markdown
# Authentication Module

**Scope:** AUTH **Owner:** @alice **Priority:** high

## Tasks

### AUTH-001: Implement login

**Intent:** Create login endpoint **Confidence:** high **Scopes:** AUTH
```

---

## Integration with Anvil

See [APS-Anvil-Integration.md](./APS-Anvil-Integration.md) for how planning docs
become execution plans.

---

## Version History

- **v0.1** (2025-12-17) — Initial specification
