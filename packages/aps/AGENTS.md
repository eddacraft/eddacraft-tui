# APS Package (@eddacraft/anvil-aps)

> APS document parsing, validation, state management, and template generation

**Parent**: See root `AGENTS.md` for project-wide conventions.

## Structure

```
packages/aps/src/
├── parser/             # Markdown AST parsing (remark/unified)
│   └── index.ts        # Extract tasks, modules, metadata from .aps.md
├── loader/             # Document loading and graph resolution
│   └── index.ts        # Load plans, resolve dependencies
├── validator/          # Validation rules
│   └── index.ts        # See "Validation Rules" below
├── filter/             # Task/module filtering
│   └── index.ts        # Context bundle generation
├── state/              # Task state management
│   └── index.ts        # .anvil/state.json, locking
├── templates/          # Template generation
│   └── generator.ts    # Create .aps.md from prompts
├── types/              # Zod schemas and TypeScript types
│   └── index.ts        # APSDocument, Task, Module schemas
└── index.ts            # Barrel exports with subpath access
```

## Where to Look

| Task                | Location                 | Notes                        |
| ------------------- | ------------------------ | ---------------------------- |
| Add validation rule | `validator/index.ts`     | Follow existing rule pattern |
| Modify parsing      | `parser/index.ts`        | Uses remark AST visitors     |
| Add state operation | `state/index.ts`         | Modify state.json schema     |
| Add template        | `templates/generator.ts` | Template string functions    |
| Add filter          | `filter/index.ts`        | Task selection logic         |

## Validation Rules

Rules emitted by `validator/index.ts`. Severity is per-issue (`error` or
`warning`) — see the source for the exact threshold of each rule.

| Rule                       | Purpose                                                   |
| -------------------------- | --------------------------------------------------------- |
| `file-readable`            | Target file exists and is readable                        |
| `plan-loadable`            | Document parses without error                             |
| `required-sections`        | Index has `## Modules`, leaf has `## Tasks`, both have H1 |
| `task-format`              | Task ID matches `TASK_ID_REGEX` (`SCOPE-NNN`)             |
| `task-intent`              | Tasks declare a non-empty `Intent:`                       |
| `missing-expected-outcome` | Warn when a task omits `Expected Outcome:`                |
| `missing-validation`       | Warn when a task omits `Validation:` (alias: `Test:`)     |
| `missing-confidence`       | Warn when a task omits `Confidence:`                      |
| `broken-links`             | References to missing modules or task IDs                 |
| `duplicate-ids`            | No duplicate task or module IDs                           |
| `circular-dependencies`    | Detect circular module dependencies                       |
| `scope-mismatch`           | Task ID prefix matches the owning module's scope          |
| `orphan-modules`           | Modules must be referenced from an index                  |
| `orphan-scan-depth`        | Warn when orphan scan can't traverse the full plan graph  |
| `path-containment`         | Relative paths stay inside the planning root              |

## Adding a Validation Rule

```typescript
// In validator/index.ts
function validateMyRule(doc: APSDocument): ValidationIssue[] {
  const issues: ValidationIssue[] = [];

  // Check something...
  if (condition) {
    issues.push({
      type: 'error', // or 'warning'
      rule: 'my-rule',
      message: 'What went wrong',
      path: doc.path,
      line: lineNumber, // Optional
    });
  }

  return issues;
}

// Add to validatePlanningDoc() in validator/index.ts:
issues.push(...validateMyRule(doc));
```

## State Management

Task execution state stored in `.anvil/state.json`:

```typescript
import {
  loadState,
  saveState,
  lockTask,
  unlockTask,
} from '@eddacraft/anvil-aps/state';

// Load current state
const state = await loadState(projectRoot);

// Lock a task for execution
const lock = await lockTask(state, 'TASK-001', {
  executor: 'user@example.com',
  reason: 'Implementing feature',
});

// Update task status
await updateTaskStatus(state, 'TASK-001', 'in_progress');

// Unlock when done
await unlockTask(state, 'TASK-001');
```

## Parser Usage

```typescript
import { parseDocument, parseIndex } from '@eddacraft/anvil-aps/parser';

// Leaf spec — modules with tasks
const doc = await parseDocument(leafContent, 'plans/modules/my-feature.aps.md');
doc.title; //  string
doc.metadata; // ModuleMetadata (id, owner, status, …)
doc.tasks; //   Task[]

// Index file — the root plan listing modules
const index = await parseIndex(indexContent, 'plans/index.aps.md');
index.title; //   string
index.modules; // ModuleMetadata[] referenced from the index
```

Validation lives in the validator subpath:

```typescript
import { validatePlanningDoc } from '@eddacraft/anvil-aps/validator';
const result = await validatePlanningDoc('plans/modules/my-feature.aps.md');
```

## Document Shapes and Field Aliases

There are two parser entry-points; templates differ accordingly:

- **Leaf specs** (`parse-document.ts`) — H1 title, then an optional paragraph of
  inline `**Field:** value` metadata, then `## Tasks` or canonical
  `## Work Items` with H3 task entries. See `templates/leaf-*.md` and
  `templates/simple-*.md`.
- **Index files** (`parse-index.ts`) — H1 title, then `## Modules` with H3
  module entries followed by list-form `- **Field:** value` lines for module
  metadata. See `templates/index-*.md`.

Parser tolerances worth knowing when authoring or migrating docs:

| Surface              | Canonical tokens                                          | Legacy aliases accepted                                                                                                     | Effect                                              |
| -------------------- | --------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------- |
| Leaf section heading | `## Work Items`                                           | `## Tasks`                                                                                                                  | H3 entries parsed as tasks                          |
| Task field name      | `Expected Outcome:`                                       | temporary `Outcome:` alias                                                                                                  | Parsed as `expectedOutcome`                         |
| Task field name      | `Validation:`                                             | `Test:`                                                                                                                     | Parsed as `validation`                              |
| Task field name      | `Non-scope:`                                              | `NonScope:`                                                                                                                 | Parsed as `nonScope`                                |
| Module `Status:`     | `Proposed` / `Ready` / `In Progress` / `Done` / `Blocked` | `Draft` → `Proposed`; `Complete` → `Done`                                                                                   | Normalised; any other value leaves status unset     |
| Task `Status:` prose | `open` / `locked` / `completed` / `cancelled`             | `in progress` & `blocked` → `locked`; `complete`, `done` → `completed`; `draft`, `ready` → `open`; `canceled` → `cancelled` | Normalised — see `parseStatus()` in `parse-task.ts` |

Module status and task status use different vocabularies because they describe
different things: module status is _planning state_ (in `ModuleStatusSchema`),
task status is _execution state_ (in `TaskStatusSchema`, normally managed
externally in `.anvil/state.json`).

Task `Status:` is **fail-soft**: unknown prose defaults to `open` rather than
leaving the field unset, and the parser never errors on unrecognised status
text. Module `Status:` is **fail-silent**: unrecognised values are ignored
(status left unset). New text should use the canonical tokens listed above; the
aliases exist for migrations only.

## Template Generation

```typescript
import { generateTemplate } from '@eddacraft/anvil-aps/templates';

const content = await generateTemplate('module', {
  name: 'my-feature',
  description: 'Implements something cool',
  tasks: ['FEAT-001', 'FEAT-002'],
});

// Write to file
await writeFile('plans/modules/my-feature.aps.md', content);
```

## Scripts

```bash
nx test aps                              # All APS tests
nx test aps --testNamePattern="validator" # Validator tests only
pnpm -F aps run generate:templates       # Regenerate template examples
```

## Anti-Patterns (This Package)

- Never modify state.json directly - use state management functions
- Never skip validation before loading documents
- Always handle circular dependencies gracefully
- Always preserve AST positions for error reporting

## Testing

- Fixture-based with realistic APS documents
- Test files in `validator/__fixtures__/`, `parser/__fixtures__/`
- Comprehensive edge cases for validation rules
- State management tests with temp directories
