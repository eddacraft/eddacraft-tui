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
├── validator/          # Validation rules (745 lines)
│   └── index.ts        # 8 validation rules for APS documents
├── filter/             # Task/module filtering
│   └── index.ts        # Context bundle generation
├── state/              # Task state management (844 lines)
│   └── index.ts        # .anvil/state.json, locking
├── templates/          # Template generation (607 lines)
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

8 built-in rules in `validator/index.ts`:

| Rule                    | Purpose                              |
| ----------------------- | ------------------------------------ |
| `required-sections`     | Ensure mandatory sections exist      |
| `task-format`           | Validate task ID format (TASK-001)   |
| `task-intent`           | Tasks must have clear intent         |
| `broken-links`          | Detect references to missing modules |
| `duplicate-ids`         | No duplicate task/module IDs         |
| `circular-dependencies` | Detect circular module dependencies  |
| `scope-mismatch`        | Task scope matches module boundary   |
| `orphan-modules`        | Modules must be referenced           |

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

// Add to validateDocument():
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
import { parseAPSDocument } from '@eddacraft/anvil-aps/parser';

const doc = await parseAPSDocument(content, { path: 'plans/index.aps.md' });

// Access parsed structure
doc.metadata; // Title, version, status
doc.modules; // Array of Module objects
doc.tasks; // Array of Task objects
doc.dependencies; // Module dependency graph
```

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
