# APS Planning Spec — "APS Spin-out + Plan Loader Foundations" (v0.2)

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

- APS Planning Spec v0.1 is defined and committed (templates + conventions)
- Supports **Index + Leaf Spec** structure for large projects
- Repository contains **3 concrete examples** showing APS in real use
- Anvil has an **internal Plan Loader** that can:
  - Discover APS planning doc root
  - Resolve in-repo linked leaf specs
  - Build a PlanGraph
  - Output a scoped context bundle (depth-limited)
- Task-level locking primitive exists (`anvil plan lock --task <id>`)
- A CLI command exists to load plans by scope
- New `packages/aps` package (Nx-managed) contains planning spec assets and
  loader

---

## Non-Goals

- No policy enforcement, approvals, signatures, or provenance features (handled
  by existing gate)
- No UI
- No embeddings/vector search
- No "use our agent" coupling
- No full Markdown AST parser unless required
- No extraction of Zod schema from `@anvil/core` (stays where it is)

---

## Assumptions

- Package scaffolded via Nx generator, then adjusted to match existing patterns
- Templates in `packages/aps` are generated from schema in `core`
- Existing CLI can be extended with new `plan` subcommands
- Planning docs live in `planning/` directory (distinct from `.anvil/` for
  execution artefacts)

Confidence: high

---

## Directory Conventions

| Artefact Type   | Location                                  | Format   | Mutability           |
| --------------- | ----------------------------------------- | -------- | -------------------- |
| Planning docs   | `planning/APS.md`, `planning/**/*.aps.md` | Markdown | Mutable              |
| Execution plans | `.anvil/executions/<task-id>.json`        | JSON     | Immutable (per-task) |
| Evidence        | `.anvil/evidence/`                        | JSON     | Append-only          |
| Cache           | `.anvil/cache/`                           | JSON     | Ephemeral            |

**Root discovery order**: `planning/APS.md` → `APS.md` →
`.anvil/planning/APS.md`

---

## System / Repo Structure

```
packages/aps/                    # Nx-managed package
  src/
    loader/
      plan-graph.ts              # Graph model (nodes, edges, task states)
      plan-loader.ts             # Root discovery, markdown scanning
      scope-resolver.ts          # --scope, --task resolution
      context-bundle.ts          # Output bundler
      task-locker.ts             # Lock task, generate execution plan
    templates/
      generator.ts               # Generate templates from core schema
    index.ts                     # Public API
  docs/
    APS-Planning-Spec-v0.1.md    # The spec itself
    APS-Conventions.md           # IDs, naming, links, confidence
    APS-NonGoals.md              # What APS deliberately doesn't do
    APS-Anvil-Integration.md     # How planning docs become execution plans
  examples/
    feature.aps.md               # Single-file example
    system/
      APS.md                     # Index example
      modules/
        auth.aps.md
        payments.aps.md
    refactor.aps.md              # Uncertainty + scoped tasks example
  project.json                   # Nx project config (generated)
  package.json
  tsconfig.json
  tsconfig.lib.json
  README.md
```

---

## Execution Plan

### Phase 1 — Package Setup (Nx Hybrid)

- Generate package using Nx:

  ```bash
  nx g @nx/js:library aps \
    --directory=packages/aps \
    --publishable \
    --importPath=@anvil/aps \
    --unitTestRunner=vitest \
    --bundler=tsc
  ```

- Adjust generated files to match existing patterns (adapters)
- Add `@anvil/core` as workspace dependency
- Verify Nx graph shows correct dependencies
- Add docs/ and examples/ folder structure

**Deliverables**: Scaffolded `packages/aps` with Nx integration

---

### Phase 2 — Spec Definition (APS Planning Spec v0.1)

- Define Index sections (metadata, modules, decisions, open questions)
- Define Leaf Spec sections (scope, tasks, dependencies, notes)
- Define Task shape:
  - ID (namespaced: `AUTH-001`)
  - Intent
  - Inputs / Expected Outcome
  - Status: `open | locked | completed | cancelled`
  - Confidence: `low | medium | high`
  - Optional: Scope constraints, dependencies
- Define conventions:
  - Root file discovery
  - File naming (`*.aps.md` for leaf specs)
  - Link rules (relative, in-repo only)
  - ID uniqueness (warn on duplicates, recommend namespacing)

**Deliverables**: `APS-Planning-Spec-v0.1.md`, `APS-Conventions.md`,
`APS-NonGoals.md`, `APS-Anvil-Integration.md`

---

### Phase 3 — Examples (adoption fuel)

Create 3 example plans demonstrating real use:

1. **Feature plan** (single file) — simple, self-contained
2. **System plan** (Index + 2–3 leaf specs) — shows graph structure
3. **Refactor plan** (uncertainty + scoped tasks) — shows confidence markers,
   task states

**Deliverables**: `examples/` directory

---

### Phase 4 — Plan Loader (internal primitive)

Implement Plan Loader components:

- Root discovery + override (`--plan-root`)
- Markdown scanning (headings, links, task blocks)
- Relative path resolution + in-repo safety checks
- Cycle detection
- Depth-limited traversal (`--depth`, default 1)
- PlanGraph model:
  - Nodes (specs, tasks)
  - Edges (links, dependencies)
  - Task states (open, locked, completed)
- Scope selection:
  - `--scope <module-id|file-path>`
  - `--task <task-id>`
- Context Bundle output:
  - Ordered files
  - Raw contents
  - JSON format option

**Deliverables**: `PlanGraph`, `PlanLoader`, `ScopeResolver`,
`ContextBundleBuilder`

---

### Phase 5 — Task Locking (execution bridge)

Implement task-level locking:

- `anvil plan lock --task AUTH-001`
  - Snapshot task definition from planning doc
  - Generate execution plan JSON (per-task)
  - Compute hash, capture provenance (who, when, plan version/commit)
  - Write to `.anvil/executions/<task-id>.json`
  - Track task state (locked)
- `anvil plan unlock --task AUTH-001` (abandon incomplete work)
- `anvil plan status` — show task states across the plan

**Deliverables**: `TaskLocker`, CLI commands, execution plan schema extension

---

### Phase 6 — CLI Surface + Dogfood

Add CLI commands:

```bash
anvil plan load --scope auth --depth 1      # Load scoped context
anvil plan lock --task AUTH-001             # Lock task for execution
anvil plan status                           # Show task states
anvil plan unlock --task AUTH-001           # Abandon locked task
```

Dogfood against system example:

- Verify scoping works correctly
- Verify depth limiting
- Verify lock/unlock cycle
- Verify broken links and duplicates are surfaced

**Deliverables**: CLI commands, README section, dogfood notes

---

### Phase 7 — Template Generation

- Script to generate planning spec sections from `core/src/schema/aps.schema.ts`
- Ensure templates stay in sync with schema changes
- Add Nx target for regeneration

**Deliverables**: `generator.ts`, Nx target `nx run aps:generate-templates`

---

## Tasks

### Package Setup (Nx Hybrid)

- [ ] Run
      `nx g @nx/js:library aps --directory=packages/aps --publishable --importPath=@anvil/aps --unitTestRunner=vitest --bundler=tsc`
- [ ] Adjust generated files to match existing package patterns
- [ ] Add `@anvil/core` as workspace dependency in package.json
- [ ] Add tsconfig reference to core
- [ ] Add docs/ and examples/ folder structure
- [ ] Add basic README explaining purpose and boundaries
- [ ] Verify `nx graph` shows correct dependencies

### APS Planning Spec (v0.1)

- [ ] Write `APS-Planning-Spec-v0.1.md` (Index + Leaf + Task patterns)
- [ ] Write `APS-Conventions.md` (IDs, naming, links, confidence, task states)
- [ ] Write `APS-NonGoals.md`
- [ ] Write `APS-Anvil-Integration.md` (planning doc → execution plan flow)

### Examples

- [ ] Example 1: feature plan (single file)
- [ ] Example 2: system plan (index + leaf specs)
- [ ] Example 3: refactor plan (uncertainty + tasks + scope)

### Plan Loader

- [ ] Implement PlanGraph model (nodes, edges, task states)
- [ ] Implement root discovery + override
- [ ] Implement markdown scanning (headings, links, task blocks)
- [ ] Implement safe link resolution (no escaping repo root)
- [ ] Implement cycle detection
- [ ] Implement depth-limited traversal
- [ ] Implement scope resolver (`--scope`, `--task`)
- [ ] Implement context bundle output (text + JSON)
- [ ] Add duplicate ID detection (warn only)
- [ ] Add broken reference reporting

### Task Locking

- [ ] Define execution plan schema (per-task, extends existing APS schema)
- [ ] Implement TaskLocker (snapshot, hash, provenance)
- [ ] Implement lock command
- [ ] Implement unlock command
- [ ] Implement status command
- [ ] Decide task state storage: `.anvil/state.json` vs in-doc markers

### CLI

- [ ] Add `anvil plan load` command
- [ ] Add `anvil plan lock` command
- [ ] Add `anvil plan unlock` command
- [ ] Add `anvil plan status` command
- [ ] Add output modes: default, `--json`, `--files-only`
- [ ] Document usage in README

### Template Generation

- [ ] Create generator script in `src/templates/generator.ts`
- [ ] Add Nx target for regeneration
- [ ] Document regeneration process

### Dogfooding

- [ ] Run loader on example system plan
- [ ] Test lock/unlock cycle
- [ ] Capture issues and adjust

---

## Risks & Open Questions

| Question                                 | Proposed Resolution                                        |
| ---------------------------------------- | ---------------------------------------------------------- |
| How strict should link following be?     | Follow `.md` but recommend `.aps.md` for leaf specs        |
| Task ID uniqueness: global or per-scope? | Warn on duplicates; recommend namespacing (`AUTH-###`)     |
| Markdown parsing complexity?             | Start with scanner; upgrade only if needed                 |
| Where to track task state?               | `.anvil/state.json` (avoid modifying source docs)          |
| Concurrent locks on same task?           | First lock wins; subsequent attempts fail with clear error |

---

## Relationship to Existing Code

| Existing                          | Relationship                                                                 |
| --------------------------------- | ---------------------------------------------------------------------------- |
| `core/src/schema/aps.schema.ts`   | **Stays in place**. Execution plan schema. `packages/aps` references it.     |
| `cli/src/services/plan-loader.ts` | **Remains**. Handles JSON/adapter-based loading. New loader is for Markdown. |
| `cli/src/commands/plan.ts`        | **Extend**. Add `load`, `lock`, `unlock`, `status` subcommands.              |
| `.anvil/plans/`                   | **Rename to `.anvil/executions/`** for clarity.                              |

---

## Future Considerations

- **Custom Nx generator**: Could create `nx g @anvil/aps:planning-doc` to
  scaffold new planning docs from templates
- **Linter**: Future `anvil plan lint` to validate planning doc structure
- **VS Code extension**: Syntax highlighting and task state indicators for
  `.aps.md` files

---

_Version: 0.2 | Last updated: December 2025_
