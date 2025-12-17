# APS-Anvil Integration

> How Anvil Planning Spec documents become executable plans.

## Overview

APS defines **what to do** (planning). Anvil provides **how to execute** it
(orchestration + LLM agents). This document explains how they integrate.

---

## Architecture

### Three Layers

```
┌─────────────────────────────────────────┐
│  Planning Docs (Source of Truth)       │
│  docs/planning/*.aps.md                 │
│  - Defines tasks, intent, dependencies  │
│  - Committed to Git                     │
│  - Human-editable Markdown              │
└─────────────────────────────────────────┘
                 │
                 │ parse + validate
                 ▼
┌─────────────────────────────────────────┐
│  State Management (Derived)             │
│  .anvil/state.json                      │
│  - Task status (open/locked/completed)  │
│  - Lock metadata (who, when, hash)      │
│  - Source file pointers                 │
└─────────────────────────────────────────┘
                 │
                 │ lock task
                 ▼
┌─────────────────────────────────────────┐
│  Execution Plans (Generated)            │
│  .anvil/executions/<task-id>.json       │
│  - Snapshot of task definition          │
│  - Scoped context bundle                │
│  - Provenance (hash, commit, timestamp) │
└─────────────────────────────────────────┘
                 │
                 │ execute
                 ▼
┌─────────────────────────────────────────┐
│  Anvil Execution Engine                 │
│  - LLM agents                           │
│  - Gate checks                          │
│  - Change validation                    │
└─────────────────────────────────────────┘
```

---

## Workflow: Planning to Execution

### Step 1: Write Planning Doc

Create or update a planning document:

```markdown
# Feature: User Authentication

**Scope:** AUTH **Owner:** @alice

## Tasks

### AUTH-001: Implement login endpoint

**Intent:** Create POST /auth/login that validates credentials and returns JWT
**Confidence:** high **Scopes:** AUTH **Tags:** security, api
```

Commit to Git:

```bash
git add docs/planning/auth.aps.md
git commit -m "Add auth planning doc"
```

---

### Step 2: Validate Planning Doc

Before locking tasks, validate the planning doc:

```bash
anvil plan validate docs/planning/auth.aps.md
```

**Output:**

```
✓ Structure valid
✓ All required sections present
✓ No duplicate task IDs
✓ All links resolve
⚠ AUTH-002 missing Confidence field

Validation passed with 1 warning
```

Validation checks:

- Required sections (`## Tasks`)
- Task has Intent
- Task ID format (`AUTH-001`)
- No duplicates
- Links resolve
- No circular dependencies

**Errors block execution. Warnings are non-blocking.**

---

### Step 3: Lock Task for Execution

When ready to work on a task:

```bash
anvil plan lock --task AUTH-001
```

**What happens:**

1. **Validate first** — Re-runs validation, fails if errors exist
2. **Check state** — Verify task is not already locked
3. **Snapshot task** — Extract full task definition from planning doc
4. **Resolve context** — Gather related tasks, dependencies, scoped files
5. **Generate execution plan** — Write to `.anvil/executions/AUTH-001.json`
6. **Update state** — Mark as `locked` in `.anvil/state.json`
7. **Compute provenance** — Hash, commit SHA, timestamp, user

**State update (`.anvil/state.json`):**

```json
{
  "AUTH-001": {
    "status": "locked",
    "locked_at": "2025-12-17T10:30:00Z",
    "locked_by": "alice",
    "execution_file": ".anvil/executions/AUTH-001.json",
    "source": {
      "file": "docs/planning/auth.aps.md",
      "line": 23,
      "commit": "abc123def"
    }
  }
}
```

**Execution plan (`.anvil/executions/AUTH-001.json`):**

```json
{
  "task_id": "AUTH-001",
  "intent": "Create POST /auth/login that validates credentials and returns JWT",
  "confidence": "high",
  "scopes": ["AUTH"],
  "tags": ["security", "api"],
  "dependencies": [],
  "context": {
    "related_tasks": ["AUTH-002", "AUTH-003"],
    "scoped_files": ["src/auth/**"],
    "planning_doc": "docs/planning/auth.aps.md"
  },
  "provenance": {
    "snapshot_hash": "sha256:...",
    "commit": "abc123def",
    "locked_at": "2025-12-17T10:30:00Z",
    "locked_by": "alice"
  }
}
```

---

### Step 4: Execute Task

Execution happens via Anvil agents (manually or automated):

```bash
anvil execute .anvil/executions/AUTH-001.json
```

**What the agent sees:**

- Task intent
- Scoped files (only `AUTH` scope visible)
- Related tasks
- Dependencies
- Current codebase state

**What the agent does:**

1. Reads the execution plan
2. Generates changes (code, tests, docs)
3. Runs gate checks (tests, linting, type checking)
4. Validates changes match scopes
5. Creates a change set

**Gate checks:**

- Do changes stay within declared scopes?
- Do tests pass?
- Does type checking pass?
- Is the intent satisfied?

---

### Step 5: Complete or Unlock Task

**On success:**

```bash
anvil plan complete --task AUTH-001
```

Updates state to `completed`:

```json
{
  "AUTH-001": {
    "status": "completed",
    "completed_at": "2025-12-17T11:45:00Z",
    "execution_file": ".anvil/executions/AUTH-001.json"
  }
}
```

**On failure or abandonment:**

```bash
anvil plan unlock --task AUTH-001
```

Updates state to `cancelled` and removes execution file.

---

### Step 6: View Status

Check status of all tasks:

```bash
anvil plan status
```

**Output:**

```
AUTH-001: ✓ completed
AUTH-002: ⏸  open
AUTH-003: 🔒 locked (by bob, 2h ago)
AUTH-004: ❌ cancelled
```

---

## Directory Structure

### Before Execution

```
repo/
  docs/
    planning/
      auth.aps.md          # Planning doc (source)
  src/
    auth/
      ...
```

### During Execution

```
repo/
  .anvil/
    state.json             # Task states
    executions/
      AUTH-001.json        # Execution plan for locked task
  docs/
    planning/
      auth.aps.md          # Planning doc (unchanged)
  src/
    auth/
      ...
```

### After Completion

```
repo/
  .anvil/
    state.json             # Updated with 'completed' status
    executions/            # Empty (or archived)
  docs/
    planning/
      auth.aps.md          # Planning doc (unchanged)
  src/
    auth/
      login.ts             # New code from execution
      login.test.ts        # New tests
```

---

## State Management

### State File Location

`.anvil/state.json` — Single source of truth for task execution state.

### State Schema

```typescript
type TaskState = {
  status: 'open' | 'locked' | 'completed' | 'cancelled';
  locked_at?: string; // ISO 8601 timestamp
  locked_by?: string; // Username
  completed_at?: string;
  cancelled_at?: string;
  execution_file?: string; // Path to execution plan
  source: {
    file: string; // Planning doc path
    line: number; // Line number of task heading
    commit?: string; // Git commit SHA
  };
};

type State = {
  [taskId: string]: TaskState;
};
```

### Concurrency Model: First Lock Wins

When multiple agents/users try to lock the same task:

1. Agent A runs `anvil plan lock --task AUTH-001`
2. Agent B runs `anvil plan lock --task AUTH-001` (simultaneously)
3. **First write to `.anvil/state.json` wins**
4. Second attempt fails with:
   `Error: Task AUTH-001 is already locked by agent-a`

**No distributed locking needed** — Git + filesystem atomicity handles this.

### State Transitions

```
open ──lock──> locked ──complete──> completed
               │
               └──unlock──> cancelled
```

**Valid transitions:**

- `open` → `locked`
- `locked` → `completed`
- `locked` → `cancelled`

**Invalid transitions:**

- `completed` → `locked` (cannot re-lock completed tasks)
- `open` → `completed` (must lock first)

---

## Execution Plan Structure

### File Location

`.anvil/executions/<task-id>.json`

**Example:** `.anvil/executions/AUTH-001.json`

### Schema

```typescript
type ExecutionPlan = {
  task_id: string;
  intent: string;
  confidence?: 'low' | 'medium' | 'high';
  scopes: string[];
  tags: string[];
  dependencies: string[]; // Task IDs
  inputs?: string[];
  expected_outcome?: string;
  context: {
    related_tasks: string[]; // Task IDs in same module
    scoped_files: string[]; // Glob patterns for scoped files
    planning_doc: string; // Path to source planning doc
    module?: string;
  };
  provenance: {
    snapshot_hash: string; // Hash of task definition
    commit: string; // Git commit SHA
    locked_at: string; // ISO 8601 timestamp
    locked_by: string; // Username
    plan_version?: string; // APS version
  };
};
```

### Context Bundle

The execution plan includes a **context bundle** — all information the LLM
needs:

**Included:**

- Task intent and metadata
- Related tasks in the same module
- Dependencies (with their definitions)
- Scoped files (glob patterns matching scopes)
- Planning doc content

**Excluded:**

- Files outside declared scopes
- Unrelated modules
- Completed or cancelled tasks (unless dependencies)

---

## Scope Enforcement

### What Are Scopes?

Scopes define **what can be changed** during task execution.

**Example:**

```markdown
**Scopes:** AUTH, DB
```

This task can modify:

- Files in the `AUTH` scope (e.g., `src/auth/**`)
- Files in the `DB` scope (e.g., `src/db/**`, `migrations/**`)

### Scope-to-File Mapping

Scope mapping is configured in `.anvilrc`:

```json
{
  "scopes": {
    "AUTH": ["src/auth/**", "tests/auth/**"],
    "DB": ["src/db/**", "migrations/**"],
    "API": ["src/api/**", "tests/api/**"]
  }
}
```

### Enforcement

During execution, Anvil agents:

1. Read task scopes from execution plan
2. Resolve scopes to file patterns
3. **Restrict file access** to only those patterns
4. Validate all changes stay within scopes

**If agent attempts to modify out-of-scope files:**

```
Error: Change validation failed
  Attempted to modify src/payments/charge.ts
  Task AUTH-001 only has scopes: AUTH, DB
  File src/payments/charge.ts is in scope: PAY
```

### Multi-Scope Tasks

Tasks can declare multiple scopes when necessary:

```markdown
### AUTH-001: Migrate user table

**Intent:** Add email_verified column to users table **Scopes:** AUTH, DB
```

This allows modifying both auth code and database schema.

---

## CLI Commands

### `anvil plan validate [path]`

Validates a planning document.

**Usage:**

```bash
anvil plan validate                        # Validates default (docs/planning/APS.md)
anvil plan validate docs/planning/auth.aps.md  # Validates specific file
```

**Options:**

- `--json` — Output as JSON
- `--strict` — Treat warnings as errors

---

### `anvil plan load [options]`

Loads a planning document and outputs context.

**Usage:**

```bash
anvil plan load --scope AUTH               # Load all AUTH tasks
anvil plan load --module auth --depth 2    # Load auth module + 2 levels of dependencies
anvil plan load --task AUTH-001            # Load single task
```

**Options:**

- `--scope <scope>` — Filter by scope
- `--module <module>` — Filter by module
- `--task <task-id>` — Load single task
- `--owner <owner>` — Filter by owner
- `--tag <tag>` — Filter by tag
- `--priority <priority>` — Filter by priority
- `--depth <n>` — Traverse N levels of dependencies (default: 1)
- `--json` — Output as JSON
- `--files-only` — Output only file paths

---

### `anvil plan lock --task <task-id>`

Locks a task for execution.

**Usage:**

```bash
anvil plan lock --task AUTH-001
```

**What it does:**

1. Validates planning doc
2. Checks task is not already locked
3. Generates execution plan
4. Updates state to `locked`

**Fails if:**

- Planning doc has validation errors
- Task already locked
- Task ID doesn't exist

---

### `anvil plan unlock --task <task-id>`

Unlocks a task (abandons work).

**Usage:**

```bash
anvil plan unlock --task AUTH-001
```

**What it does:**

1. Updates state to `cancelled`
2. Removes execution file
3. Allows re-locking later

---

### `anvil plan complete --task <task-id>`

Marks a task as completed.

**Usage:**

```bash
anvil plan complete --task AUTH-001
```

**What it does:**

1. Updates state to `completed`
2. Records completion timestamp
3. Archives execution file

---

### `anvil plan status [options]`

Shows status of all tasks.

**Usage:**

```bash
anvil plan status                          # All tasks
anvil plan status --module auth            # Tasks in auth module
anvil plan status --json                   # JSON output
```

**Output:**

```
AUTH-001: ✓ completed (2h ago)
AUTH-002: 🔒 locked (by alice, 30m ago)
AUTH-003: ⏸  open
AUTH-004: ❌ cancelled
```

---

## Integration Points

### With Git

- Planning docs committed to Git
- `.anvil/state.json` can be committed or gitignored
- `.anvil/executions/` typically gitignored (ephemeral)
- Provenance includes Git commit SHA

**Recommended `.gitignore`:**

```gitignore
.anvil/executions/
# .anvil/state.json  (optional - commit if tracking state)
```

---

### With Anvil Execution Engine

Anvil agents consume execution plans:

```typescript
import { executePlan } from '@anvil/execution';

const plan = await loadExecutionPlan('.anvil/executions/AUTH-001.json');
const result = await executePlan(plan, {
  scopeMapping: config.scopes,
  gateChecks: ['tests', 'lint', 'typecheck'],
});
```

---

### With CI/CD

Planning doc validation in CI:

```yaml
name: Validate Planning Docs
on: [pull_request]
jobs:
  validate:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: anvil plan validate docs/planning/APS.md
```

---

## Example: Full Workflow

### 1. Create Planning Doc

```markdown
# Feature: Password Reset

**Scope:** AUTH **Owner:** @alice

## Tasks

### AUTH-010: Add password reset endpoint

**Intent:** Create POST /auth/reset-password endpoint **Confidence:** high
**Scopes:** AUTH, EMAIL **Dependencies:** AUTH-001
```

### 2. Commit to Git

```bash
git add docs/planning/auth.aps.md
git commit -m "Add password reset tasks"
git push
```

### 3. Validate

```bash
anvil plan validate docs/planning/auth.aps.md
# ✓ Validation passed
```

### 4. Lock Task

```bash
anvil plan lock --task AUTH-010
# ✓ Task AUTH-010 locked
# Execution plan: .anvil/executions/AUTH-010.json
```

### 5. Execute (via Anvil agent)

```bash
anvil execute .anvil/executions/AUTH-010.json
# Agent runs, generates changes, passes gate checks
```

### 6. Complete

```bash
anvil plan complete --task AUTH-010
# ✓ Task AUTH-010 marked as completed
```

### 7. Check Status

```bash
anvil plan status --module auth
# AUTH-001: ✓ completed
# AUTH-010: ✓ completed
```

---

## Design Principles

1. **Planning docs are immutable during execution** — Source of truth never
   changes
2. **State is separate from planning** — `.anvil/state.json` tracks execution
3. **Execution plans are ephemeral** — Generated, used, then archived
4. **First lock wins** — Simple concurrency model
5. **Validation before execution** — Catch errors early
6. **Scope enforcement** — LLMs see only what they need

---

## Version History

- **v0.1** (2025-12-17) — Initial integration spec
