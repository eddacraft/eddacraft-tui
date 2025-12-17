# APS Planning Spec — "APS Spin-out + Plan Loader Foundations" (v0.3)

> Purpose: Establish APS as an adoptable planning format that teams can use
> independently, while building the loader primitives Anvil needs for scoped,
> task-level execution.

---

## Problem Statement

We want a lightweight planning artefact (APS Planning Docs) that sits between
intent and execution for mixed human + AI delivery. Today APS exists as an
internal JSON execution schema in `@anvil/core`, but:

1. There's no standalone, human-readable planning format teams can adopt without
   Anvil
2. Anvil can't load planning docs as a graph for scoped (sharded) agent/human
   execution
3. The relationship between planning docs (mutable) and execution plans
   (immutable at task level) isn't formalised

Without clear structure, examples, and loading support, APS adoption will remain
ad-hoc.

---

## Key Mental Model: Planning vs Execution

```
APS Planning Doc (Markdown)     ← Source of truth, human-editable, mutable
    │
    │  contains
    ▼
Tasks (units of work)           ← Can be locked individually for execution
    │
    │  anvil plan lock --task AUTH-001
    ▼
APS Execution Plan (JSON)       ← Per-task, immutable once locked, has hash + provenance
    │
    │  anvil gate / anvil apply
    ▼
Evidence + Audit Trail          ← Appended as work completes
```

**Critical insight**: The plan as a whole remains **mutable** (living document).
Immutability and hashing apply at the **task level** once a task enters
execution. This allows:

- Ongoing planning while work is in flight
- Multiple tasks executing in parallel with independent provenance
- Clear separation between "open for editing" and "locked for execution"

---

## Success Criteria

| Criteria                       | Measurable Target                                                                                                      |
| ------------------------------ | ---------------------------------------------------------------------------------------------------------------------- |
| APS Planning Spec v0.1 defined | All 4 docs committed: `APS-Planning-Spec-v0.1.md`, `APS-Conventions.md`, `APS-NonGoals.md`, `APS-Anvil-Integration.md` |
| Index + Leaf Spec structure    | Loader correctly parses index with ≥3 linked leaf specs                                                                |
| Concrete examples              | 3 examples committed: single-file, multi-module (index + 2 leaves), refactor with uncertainty                          |
| Plan Loader                    | >90% test coverage                                                                                                     |
| Task-level locking             | Lock/unlock commands work with concurrent lock conflict handling tested                                                |
| CLI scope commands             | `anvil plan load` supports `--scope`, `--task`, `--depth`, `--owner`, `--tag`, `--module` flags                        |
| New package                    | `packages/aps` builds, lints, and is visible in `nx graph`                                                             |
| Performance                    | Load and parse 100-task plan in <500ms                                                                                 |
| Validation                     | Duplicate task ID detection works across leaf specs                                                                    |
| Error handling                 | Broken links reported with file:line reference                                                                         |
| State management               | `.anvil/state.json` correctly tracks all 4 status values                                                               |

---

## Non-Goals

- No policy enforcement, approvals, signatures, or provenance features (handled
  by existing gate)
- No UI
- No embeddings/vector search
- No "use our agent" coupling
- No YAML frontmatter (keep pure Markdown for human readability)
- No extraction of Zod schema from `@anvil/core` (stays where it is)

---

## Assumptions

- Package scaffolded via Nx generator, then adjusted to match existing patterns
- Templates in `packages/aps` are generated from schema in `core`
- Existing CLI can be extended with new `plan` subcommands
- Planning docs live in `docs/planning/` directory (distinct from `.anvil/` for
  execution artefacts)
- ESM-only stance (no CJS support)

Confidence: high

---

## Directory Conventions

| Artefact Type   | Location                                            | Format   | Mutability  |
| --------------- | --------------------------------------------------- | -------- | ----------- |
| Planning docs   | `docs/planning/APS.md`, `docs/planning/**/*.aps.md` | Markdown | Mutable     |
| Execution plans | `.anvil/executions/<id>.json`                       | JSON     | Immutable   |
| Task state      | `.anvil/state.json`                                 | JSON     | Mutable     |
| Evidence        | `.anvil/evidence/`                                  | JSON     | Append-only |
| Cache           | `.anvil/cache/`                                     | JSON     | Ephemeral   |

**Root discovery order**: `docs/planning/APS.md` → `APS.md`

---

## Task State Schema (`.anvil/state.json`)

```json
{
  "version": "1.0.0",
  "tasks": {
    "AUTH-001": {
      "status": "locked",
      "locked_at": "2025-12-17T10:30:00.000Z",
      "locked_by": "aneki",
      "execution_file": ".anvil/executions/AUTH-001.json",
      "source": {
        "file": "docs/planning/modules/auth.aps.md",
        "line": 42
      }
    }
  }
}
```

### Status Values

| Status      | Meaning                                               |
| ----------- | ----------------------------------------------------- |
| `open`      | Task exists in planning doc, not yet locked           |
| `locked`    | Task locked for execution, immutable snapshot created |
| `completed` | Work finished, evidence collected                     |
| `cancelled` | Task abandoned (unlocked without completion)          |

---

## Markdown Conventions

### Parser

Use `remark` (part of `unified` ecosystem) for robust Markdown parsing.

Dependencies: `remark-parse`, `unified`, `unist-util-visit`

### Task Block Format (Heading-based)

```markdown
### AUTH-001: Implement login endpoint

**Intent:** Create POST /auth/login endpoint with JWT response **Status:** open
**Confidence:** high **Scopes:** AUTH, PAY **Tags:** security, api **Inputs:**
User credentials (email, password) **Expected Outcome:** Returns JWT token on
success, 401 on failure **Dependencies:** DB-001, DB-002
```

### Task Field Definitions

| Field                 | Required | Format                                                             |
| --------------------- | -------- | ------------------------------------------------------------------ |
| ID + Title (H3)       | Yes      | `### <ID>: <Title>`                                                |
| **Intent:**           | Yes      | Free text                                                          |
| **Status:**           | No       | `open` \| `locked` \| `completed` \| `cancelled` (default: `open`) |
| **Confidence:**       | No       | `low` \| `medium` \| `high` (default: `medium`)                    |
| **Scopes:**           | No       | Comma-separated scope namespaces (for LLM constraints)             |
| **Tags:**             | No       | Comma-separated labels (for search/filtering)                      |
| **Inputs:**           | No       | Free text or list                                                  |
| **Expected Outcome:** | No       | Free text                                                          |
| **Dependencies:**     | No       | Comma-separated task IDs                                           |

### Scopes vs Tags

| Field       | Purpose                                | Example                |
| ----------- | -------------------------------------- | ---------------------- |
| **Scope:**  | Module-level namespace                 | `AUTH`                 |
| **Scopes:** | Task-level boundaries (constrains LLM) | `PAY, AUTH`            |
| **Tags:**   | Flexible labels for search/filtering   | `security, stripe, v2` |

- **Scopes** = hard boundaries (what can be changed)
- **Tags** = soft labels (how to find/group things)

### Index File Structure (`docs/planning/APS.md`)

```markdown
# Project Name ← Required: H1 title

> Purpose statement ← Optional: blockquote summary

## Modules ← Required: links to leaf specs

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

## Open Questions ← Optional

- How do we handle rate limiting?

## Decisions ← Optional

- Using JWT for auth (decided 2025-12-01)
```

### Module Metadata Fields

| Field             | Required | Purpose                     |
| ----------------- | -------- | --------------------------- |
| **Path:**         | Yes      | Link to leaf file           |
| **Scope:**        | No       | Namespace for task IDs      |
| **Owner:**        | No       | Responsible party           |
| **Priority:**     | No       | `low` \| `medium` \| `high` |
| **Tags:**         | No       | Comma-separated labels      |
| **Dependencies:** | No       | Module IDs this depends on  |

### Leaf Spec Structure (`docs/planning/**/*.aps.md`)

```markdown
# Authentication Module ← Required: H1 title

**Scope:** AUTH **Owner:** @alice **Priority:** high

> Handles user authentication and session management. ← Optional

## Tasks ← Required section heading

### AUTH-001: Implement login ← Required: ID + title

**Intent:** Create POST /auth/login endpoint **Confidence:** high ...

### AUTH-002: Add password reset ← Another task

**Intent:** ...

## Dependencies ← Optional

- Depends on database schema (DB-001)

## Notes ← Optional

- Consider OAuth later
```

---

## Scope Resolution

### Definitions

**Module ID:** The H3 heading under `## Modules` in the index file (e.g.,
`auth`, `payments`).

**Scope:** The namespace prefix for task IDs (e.g., `AUTH`, `PAY`). Defined via
`**Scope:**` field.

**Task ID:** The ID portion of the H3 task heading (e.g., `AUTH-001`).

### Resolution Rules

| Flag              | Resolves to                                           |
| ----------------- | ----------------------------------------------------- |
| `--module auth`   | All tasks in the `auth` module (by module-id)         |
| `--scope AUTH`    | All tasks with `AUTH` in their Scopes field           |
| `--task AUTH-001` | Single task by exact ID                               |
| `--owner @alice`  | All modules/tasks owned by `@alice`                   |
| `--tag security`  | All modules/tasks tagged with `security`              |
| `--priority high` | All high-priority modules/tasks                       |
| `--depth N`       | Traverse N levels of module dependencies (default: 1) |

---

## System / Repo Structure

### Spinout Package (`packages/aps`)

```
packages/aps/                    # Nx-managed package
  src/
    validator/
      validate.ts               # validatePlanningDoc()
      rules/
        required-sections.ts
        task-format.ts
        duplicate-ids.ts
        broken-links.ts
    parser/
      parse.ts                  # parsePlanningDoc()
      task-parser.ts
      link-resolver.ts
    graph/
      plan-graph.ts             # PlanGraph model
      scope-resolver.ts         # resolveScope()
    templates/
      generator.ts              # Generate templates from core schema
    index.ts                    # Public API exports
  docs/
    APS-Planning-Spec-v0.1.md   # The spec itself (canonical)
    APS-Conventions.md          # IDs, naming, links, confidence
    APS-NonGoals.md             # What APS deliberately doesn't do
    APS-Anvil-Integration.md    # How planning docs become execution plans
  examples/
    feature.aps.md              # Single-file example
    system/
      APS.md                    # Index example
      modules/
        auth.aps.md
        payments.aps.md
    refactor.aps.md             # Uncertainty + scoped tasks example
  project.json                  # Nx project config
  package.json
  tsconfig.json
  tsconfig.lib.json
  tsconfig.spec.json
  eslint.config.mjs
  README.md
```

### Anvil CLI Integration

```
cli/src/commands/
  plan.ts                       # Update to add subcommands
  plan/
    validate.ts                 # anvil plan validate
    load.ts                     # anvil plan load
    lock.ts                     # anvil plan lock (validates first)
    unlock.ts
    status.ts
```

---

## Execution Plan

### Phase 1 — Package Setup (Nx Generator + Definition of Done)

**Pre-generation (Definition of Done):**

ESM-only stance: | Decision | Value | | -------- | ----- | | Module format | ESM
only | | `"type"` | `"module"` | | Target consumers | Node 18+, modern bundlers
| | CJS support | None (intentional) |

Package.json exports shape:

```json
{
  "name": "@anvil/aps",
  "version": "0.0.1",
  "type": "module",
  "main": "./dist/index.js",
  "types": "./dist/index.d.ts",
  "exports": {
    ".": {
      "types": "./dist/index.d.ts",
      "import": "./dist/index.js"
    }
  },
  "files": ["dist"]
}
```

Output shape:

```
packages/aps/dist/
  index.js              # Main entrypoint
  index.d.ts            # Type declarations
  validator/
    validate.js
    validate.d.ts
    ...
  parser/
    ...
  graph/
    ...
```

**Generation:**

```bash
nx g @nx/js:library aps \
  --directory=packages/aps \
  --publishable \
  --importPath=@anvil/aps \
  --unitTestRunner=vitest \
  --bundler=tsc
```

**Post-generation adjustments:**

- Add `"type": "module"` to package.json
- Add explicit `exports` field to package.json
- Add `@anvil/core` as `workspace:*` runtime dependency
- Add remark dependencies as runtime deps: `remark-parse`, `unified`,
  `unist-util-visit`
- Update tsconfig with `references: [{ "path": "../../core" }]`
- Ensure `composite: true` is set
- Create `tsconfig.spec.json` for tests
- Update eslint config to exclude `docs/`, `examples/`
- Update project.json to exclude `docs/`, `examples/` from build inputs
- Create `docs/` directory with placeholder README
- Create `examples/` directory structure
- Create symlink or README in `docs/guides/aps/` pointing to
  `packages/aps/docs/`

**Verification:**

- `nx graph` shows `aps` → `core` dependency
- `nx build aps` produces expected dist/ shape
- `nx test aps` runs successfully
- `nx lint aps` passes
- Imports from `@anvil/aps` resolve correctly in another package

**Deliverables**: Scaffolded `packages/aps` with Nx integration

---

### Phase 2 — Spec Definition (APS Planning Spec v0.1)

- Define Index sections (metadata, modules, decisions, open questions)
- Define Leaf Spec sections (scope, tasks, dependencies, notes)
- Define Task shape:
  - ID (namespaced: `AUTH-001`)
  - Intent (required)
  - Inputs / Expected Outcome
  - Status: `open | locked | completed | cancelled`
  - Confidence: `low | medium | high`
  - Scopes (for LLM constraints)
  - Tags (for filtering)
  - Dependencies
- Define conventions:
  - Root file discovery
  - File naming (`*.aps.md` for leaf specs)
  - Link rules (relative, in-repo only)
  - ID uniqueness (error on duplicates)
- Define task block format (H3 heading + key-value fields)
- Define module metadata fields (Scope, Owner, Priority, Tags, Dependencies)

**Deliverables**: `APS-Planning-Spec-v0.1.md`, `APS-Conventions.md`,
`APS-NonGoals.md`, `APS-Anvil-Integration.md`

---

### Phase 3 — Template Generation

- Script to generate planning spec templates from
  `core/src/schema/aps.schema.ts`
- Generate index template
- Generate leaf spec template
- Ensure templates stay in sync with schema changes
- Add Nx target for regeneration

**Deliverables**: `generator.ts`, Nx target `nx run aps:generate-templates`

---

### Phase 4 — Examples (adoption fuel)

Create 3 example plans demonstrating real use:

1. **Feature plan** (single file) — simple, self-contained
2. **System plan** (Index + 2–3 leaf specs) — shows graph structure, scopes,
   module metadata
3. **Refactor plan** (uncertainty + scoped tasks) — shows confidence markers,
   task states, multi-scope tasks

**Deliverables**: `examples/` directory

---

### Phase 5 — Plan Loader (phased implementation)

#### Phase 5a — Single-File Parser (MVP)

- Parse one `.aps.md` file using remark
- Extract tasks with all fields (Intent, Scopes, Tags, etc.)
- No graph traversal, no links
- Returns flat list of tasks

#### Phase 5b — Index + Links

- Parse index file (`## Modules` section)
- Extract module metadata (Scope, Owner, Priority, Tags, Dependencies)
- Resolve links to leaf specs
- Build basic PlanGraph (nodes + edges)
- Broken link detection with file:line

#### Phase 5c — Graph Features

- Cycle detection
- Depth-limited traversal (`--depth`)
- Module dependency resolution

#### Phase 5d — Filtering & Scoping

- `--scope`, `--module`, `--task` filters
- `--owner`, `--tag`, `--priority` filters
- Context bundle output (text + JSON)

**Deliverables**: `PlanGraph`, `PlanLoader`, `ScopeResolver`,
`ContextBundleBuilder`

---

### Phase 6 — Validation

Implement planning doc validation:

- `validatePlanningDoc(path)` function in `packages/aps`
- Required sections present (Index: `## Modules`, Leaf: `## Tasks`)
- Task has Intent
- Task ID format matches `<SCOPE>-<NUMBER>`
- Duplicate task IDs across plan graph
- Broken links
- Scope mismatch warnings
- Missing Confidence warnings
- Orphan leaf specs warnings
- Circular module dependencies

**Deliverables**: `validator/validate.ts`, validation rules

---

### Phase 7 — Task Locking (execution bridge)

Implement task-level locking:

- `anvil plan lock --task AUTH-001`
  - Validate planning doc first (fail if invalid)
  - Snapshot task definition from planning doc
  - Generate execution plan JSON (per-task)
  - Compute hash, capture provenance (who, when, plan version/commit)
  - Write to `.anvil/executions/<task-id>.json`
  - Update `.anvil/state.json` (locked)
- `anvil plan unlock --task AUTH-001` (abandon incomplete work)
  - Move to `cancelled` state
  - Remove execution file
- `anvil plan status` — show task states across the plan
- First lock wins; subsequent attempts fail with clear error

**Deliverables**: `TaskLocker`, CLI commands, state management

---

### Phase 8 — CLI Surface + Dogfood

Add CLI commands:

```bash
anvil plan validate [path]                   # Validate planning doc
anvil plan load --scope auth --depth 1       # Load scoped context
anvil plan lock --task AUTH-001              # Lock task for execution
anvil plan status                            # Show task states
anvil plan unlock --task AUTH-001            # Abandon locked task
```

Output modes:

- Default (human-readable)
- `--json` (structured output)
- `--files-only` (just file paths)

Dogfood against system example:

- Verify validation catches errors with file:line references
- Verify scoping works correctly
- Verify depth limiting
- Verify lock/unlock cycle
- Verify broken links and duplicates are surfaced

**Deliverables**: CLI commands, README section, dogfood notes

---

## Migration Tasks

- [ ] Move `.anvil/plans/aps-spinout-v0.2.aps.md` →
      `docs/planning/aps-spinout-v0.3.aps.md`
- [ ] Rename `.anvil/plans/` → `.anvil/executions/` (directory and contents)
- [ ] Update `cli/src/utils/file-io.ts` — change `.anvil/plans` references to
      `.anvil/executions`
- [ ] Update `cli/src/commands/plan.ts` — change output path
- [ ] Update any test fixtures referencing `.anvil/plans/`

---

## Tasks

### Package Setup (Phase 1)

- [ ] Document ESM-only stance and Definition of Done
- [ ] Confirm `composite: true` in core tsconfig
- [ ] Run
      `nx g @nx/js:library aps --directory=packages/aps --publishable --importPath=@anvil/aps --unitTestRunner=vitest --bundler=tsc`
- [ ] Add `"type": "module"` to package.json
- [ ] Add explicit `exports` field to package.json
- [ ] Add `@anvil/core` as `workspace:*` dependency
- [ ] Add `remark-parse`, `unified`, `unist-util-visit` as dependencies
- [ ] Update tsconfig with `references: [{ "path": "../../core" }]`
- [ ] Ensure `composite: true` is set
- [ ] Create `tsconfig.spec.json` for tests
- [ ] Update eslint config to exclude `docs/`, `examples/`
- [ ] Update project.json to exclude `docs/`, `examples/` from build inputs
- [ ] Create `docs/` directory with placeholder README
- [ ] Create `examples/` directory structure
- [ ] Create symlink/README in `docs/guides/aps/` pointing to
      `packages/aps/docs/`
- [ ] Verify `nx graph` shows correct dependencies
- [ ] Verify `nx build aps` produces expected dist/ shape
- [ ] Verify `nx test aps` runs successfully
- [ ] Verify `nx lint aps` passes

### APS Planning Spec (Phase 2)

- [ ] Write `APS-Planning-Spec-v0.1.md` (Index + Leaf + Task patterns)
- [ ] Write `APS-Conventions.md` (IDs, naming, links, confidence, scopes, tags)
- [ ] Write `APS-NonGoals.md`
- [ ] Write `APS-Anvil-Integration.md` (planning doc → execution plan flow)

### Template Generation (Phase 3)

- [ ] Create generator script in `src/templates/generator.ts`
- [ ] Generate index template
- [ ] Generate leaf spec template
- [ ] Add Nx target for regeneration
- [ ] Document regeneration process

### Examples (Phase 4)

- [ ] Example 1: feature plan (single file)
- [ ] Example 2: system plan (index + leaf specs with module metadata)
- [ ] Example 3: refactor plan (uncertainty + tasks + multi-scope)

### Plan Loader (Phase 5)

**5a — Single-File Parser:**

- [ ] Add remark dependencies
- [ ] Implement task heading parser (H3 with ID)
- [ ] Implement key-value field extractor (Intent, Confidence, etc.)
- [ ] Implement Scopes and Tags parsing
- [ ] Return flat task list

**5b — Index + Links:**

- [ ] Implement index parser (`## Modules` section)
- [ ] Implement module metadata extraction
- [ ] Implement link resolution (relative paths)
- [ ] Implement PlanGraph model (nodes, edges)
- [ ] Add broken reference reporting with file:line

**5c — Graph Features:**

- [ ] Implement cycle detection
- [ ] Implement depth-limited traversal
- [ ] Implement module dependency resolution

**5d — Filtering & Scoping:**

- [ ] Implement scope resolver (`--scope`, `--module`, `--task`)
- [ ] Implement metadata filters (`--owner`, `--tag`, `--priority`)
- [ ] Implement context bundle output (text + JSON)

### Validation (Phase 6)

- [ ] Implement `validatePlanningDoc()` function
- [ ] Required sections rule
- [ ] Task Intent required rule
- [ ] Task ID format rule
- [ ] Duplicate ID detection (across graph)
- [ ] Broken link detection
- [ ] Scope mismatch warning
- [ ] Missing Confidence warning
- [ ] Orphan leaf spec warning
- [ ] Circular dependency detection

### Task Locking (Phase 7)

- [ ] Define `.anvil/state.json` schema
- [ ] Implement state read/write utilities
- [ ] Implement TaskLocker (validate, snapshot, hash, provenance)
- [ ] Implement lock command (first lock wins)
- [ ] Implement unlock command
- [ ] Implement status command

### CLI (Phase 8)

- [ ] Add `anvil plan validate` command
- [ ] Add `anvil plan load` command
- [ ] Add `anvil plan lock` command
- [ ] Add `anvil plan unlock` command
- [ ] Add `anvil plan status` command
- [ ] Add output modes: default, `--json`, `--files-only`
- [ ] Document usage in README

### Migration

- [ ] Move spinout plan to `docs/planning/`
- [ ] Rename `.anvil/plans/` to `.anvil/executions/`
- [ ] Update `cli/src/utils/file-io.ts` paths
- [ ] Update `cli/src/commands/plan.ts` output path
- [ ] Update test fixtures

### Dogfooding

- [ ] Run validation on example plans
- [ ] Run loader on system example
- [ ] Test all filter flags
- [ ] Test lock/unlock cycle
- [ ] Capture issues and adjust

---

## Risks & Open Questions

| Question                             | Resolution                                                                    |
| ------------------------------------ | ----------------------------------------------------------------------------- |
| How strict should link following be? | Follow `.md` but recommend `.aps.md` for leaf specs                           |
| Task ID uniqueness                   | Error on duplicates across entire plan graph                                  |
| Markdown parsing complexity?         | Use remark for robust AST parsing                                             |
| Where to track task state?           | `.anvil/state.json` (avoid modifying source docs)                             |
| Concurrent locks on same task?       | First lock wins; subsequent attempts fail with clear error                    |
| Multi-scope tasks?                   | Supported via `**Scopes:**` field; task appears in all matching scope queries |

---

## Relationship to Existing Code

| Existing                          | Change                                                                                                          |
| --------------------------------- | --------------------------------------------------------------------------------------------------------------- |
| `core/src/schema/aps.schema.ts`   | **Stays in place**. Execution plan schema. `packages/aps` references it.                                        |
| `cli/src/services/plan-loader.ts` | **Remains**. Handles JSON/adapter-based loading. New loader is for Markdown.                                    |
| `cli/src/commands/plan.ts`        | **Update output path** to `.anvil/executions/`. Add `validate`, `load`, `lock`, `unlock`, `status` subcommands. |
| `cli/src/utils/file-io.ts`        | **Update paths** from `.anvil/plans/` to `.anvil/executions/`.                                                  |
| `.anvil/plans/`                   | **Rename to `.anvil/executions/`**.                                                                             |

---

## Spinout vs Anvil Integration

### Spinout (`packages/aps`) — Standalone Library

| Feature                        | Description                                              |
| ------------------------------ | -------------------------------------------------------- |
| `validatePlanningDoc(path)`    | Validate a Markdown planning doc, return errors/warnings |
| `parsePlanningDoc(path)`       | Parse Markdown into PlanGraph structure                  |
| `resolveScope(graph, options)` | Filter graph by scope/tag/owner                          |
| Templates                      | Markdown templates for index/leaf specs                  |

This is the **library** that anyone can use without Anvil.

### Anvil CLI — Consumes Spinout

| Command               | Uses Spinout                           | Anvil-Specific                            |
| --------------------- | -------------------------------------- | ----------------------------------------- |
| `anvil plan validate` | `validatePlanningDoc()`                | CLI wrapper, output formatting            |
| `anvil plan load`     | `parsePlanningDoc()`, `resolveScope()` | CLI flags, context bundling               |
| `anvil plan lock`     | `validatePlanningDoc()` + parse        | State management, execution plan creation |
| `anvil plan unlock`   | —                                      | State management                          |
| `anvil plan status`   | —                                      | State display                             |

---

## Future Considerations

- **Custom Nx generator**: Could create `nx g @anvil/aps:planning-doc` to
  scaffold new planning docs from templates
- **Linter**: Future `anvil plan lint` for style/convention checks
- **VS Code extension**: Syntax highlighting and task state indicators for
  `.aps.md` files
- **MCP integration**: Expose planning doc parsing as MCP tools

---

_Version: 0.3 | Last updated: December 2025_
