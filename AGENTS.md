# Anvil - AI Agent Instructions

> **Deterministic development automation platform that makes AI-generated code
> changes safe for production**

**Generated:** 2025-12-31 | **Commit:** 633939d | **Branch:** main

## Overview

Anvil validates AI-generated code changes through **APS (Anvil Plan
Specification)** – a hash-stable internal format enabling deterministic
validation. Users work in external formats (SpecKit, BMAD); Anvil translates
internally.

```
External Format → Adapter → APS (internal) → Gate → Execute → Evidence
```

## Planning Work

**When asked to plan, implement, build, complete, or execute ANY feature, task,
or work item:**

1. **READ `plans/aps-rules.md` FIRST** - Non-negotiable
2. **Check `plans/index.aps.md`** for existing task definitions (TUI-xxx,
   ARCH-xxx, etc.)
3. Follow the APS format for all planning documents
4. Create `.aps.md` files in `plans/modules/` for new features
5. Steps go in `plans/execution/[TASK-ID].steps.md`

**If a task ID exists (e.g., TUI-003, ARCH-001):**

- Find it in `plans/` before implementing
- Check task status and dependencies
- Create `plans/execution/[TASK-ID].steps.md` if complex

**Key rules from `plans/aps-rules.md`:**

- **Specs describe intent, not implementation**
- **Tasks authorise execution** - outcome + validation command
- **Steps are checkpoints** - max 12 words, observable state only
- **Never write HOW** - agent figures out implementation from patterns

```
❌ "Create middleware that extracts JWT, validates with jsonwebtoken..."
✅ "Auth middleware validates requests, attaches user to context"
```

## Structure

```
anvil/
├── core/                 # @anvil/core - Schema, validation, gates, crypto
├── cli/                  # @anvil/cli - Commander.js CLI with TUI (Ink)
├── packages/
│   ├── adapters/         # Format converters (SpecKit, BMAD, Generic)
│   ├── aps/              # APS document parser, validator, state management
│   └── vscode-extension/ # VS Code integration
├── plans/                # .aps.md specs, execution/*.steps.md, decisions/
├── docs/                 # Architecture, ADRs, guides, status
└── e2e/                  # Playwright E2E tests
```

## Commands

```bash
# Essential
pnpm build                    # Build all (REQUIRED before cross-package testing)
pnpm test                     # Unit tests (Vitest)
pnpm lint                     # ESLint + markdownlint with auto-fix
pnpm typecheck                # TypeScript strict mode

# Package-specific
nx test core --testNamePattern="validator"    # Test pattern
pnpm -F core run generate:schema              # Regenerate JSON schema from Zod
pnpm -F core run update-golden-hashes         # Update golden test hashes

# CLI development
pnpm link:cli                 # Build and link globally ('anvil' command)
pnpm unlink:cli               # Unlink when done
```

## Where to Look

| Task               | Location                        | Notes                                                  |
| ------------------ | ------------------------------- | ------------------------------------------------------ |
| APS schema changes | `core/src/schema/aps.schema.ts` | Run generate:schema + update-golden-hashes after       |
| Add gate check     | `core/src/gate/checks/`         | Extend BaseCheck, register in gate-runner.ts           |
| Add CLI command    | `cli/src/commands/`             | Use create{Name}Command() factory pattern              |
| Add format adapter | `packages/adapters/src/`        | Implement FormatAdapter, register with AdapterRegistry |
| TUI components     | `cli/src/tui/components/`       | Ink/React components with useInput hooks               |
| Validation rules   | `packages/aps/src/validator/`   | AST-based with remark-parse                            |
| **Planning/specs** | `plans/aps-rules.md`            | **READ FIRST** before creating/editing .aps.md files   |

## Conventions (Deviations from Standard)

### UK English Required

```
organised, recognised, colour, behaviour, optimise, centre, analyse
initialise (not initialize), serialise (not serialize)
```

### ESM with .js Extensions

```typescript
// ✅ Correct - NodeNext module resolution requires .js
import { CheckContext } from '../types/gate.types.js';

// ❌ Wrong - will fail at runtime
import { foo } from './utils';
```

### Zod-First Schemas (ADR-0001)

```typescript
// Define with Zod, export inferred type
export const ChangeSchema = z.object({
  type: ChangeTypeSchema,
  path: z.string().describe('File or resource path'),
});
export type Change = z.infer<typeof ChangeSchema>;
```

### Naming

- **Files**: kebab-case (`gate-runner.ts`, `format-detection.ts`)
- **Unused vars**: Prefix with `_` (`_unusedArg`, `_context`)
- **Tests**: Co-located as `.test.ts` or `.spec.ts`
- **Fixtures**: `__fixtures__/` directories

## Anti-Patterns (This Project)

| Pattern                                        | Why Forbidden                        |
| ---------------------------------------------- | ------------------------------------ |
| `as any`, `@ts-ignore`, `@ts-expect-error`     | Type safety is non-negotiable        |
| Empty catch blocks `catch(e) {}`               | Silently swallows errors             |
| Missing `.js` import extensions                | ESM runtime failure                  |
| Imports without path aliases crossing packages | Use `@anvil/core`, `@anvil/adapters` |
| `cd dir && command` in bash                    | Use workdir parameter instead        |

## Anti-Pattern Catalogue (Built-in)

| ID     | Pattern                      | Severity      |
| ------ | ---------------------------- | ------------- |
| AP-001 | Broad `/* eslint-disable */` | warning       |
| AP-003 | Explicit `any` type          | warning       |
| AP-004 | `@ts-ignore` directive       | warning       |
| AP-006 | Empty catch block            | warning       |
| AP-007 | Console in production code   | info (opt-in) |

## Gotchas

### 1. Build Before Test

TypeScript project references require explicit builds:

```bash
pnpm build        # Build all packages first
pnpm test         # Now cross-package imports work
```

### 2. After Schema Changes

```bash
pnpm -F core run generate:schema        # Regenerate JSON schema
pnpm -F core run update-golden-hashes   # Update golden test hashes
pnpm test                               # Verify all tests pass
```

### 3. Pre-existing Issues

When encountering issues **not related to your task**:

1. **Flag the issue** with specific details
2. **Ask for permission** before fixing
3. **Separate concerns** in a different commit if approved

### 4. Gate Check Pattern

All checks extend `BaseCheck` with consistent interface:

```typescript
class MyCheck extends BaseCheck {
  async run(context: CheckContext): Promise<GateResult> {
    // Use createSuccess() / createFailure() helpers
    return this.createSuccess({ message: 'Passed', score: 100 });
  }
}
```

## Key Architecture

### Pipeline Flow

1. **Parse**: External format → APS (via adapter)
2. **Validate**: Schema validation + SHA-256 hash generation
3. **Gate**: Quality checks (lint, test, coverage, secrets, dependencies)
4. **Execute**: Apply changes with snapshots (rollback capability)
5. **Evidence**: Immutable audit trail with provenance

### Gate Checks (`core/src/gate/checks/`)

- `architecture.check.ts` - Dependency analysis, layer violations
- `coverage.check.ts` - Code coverage thresholds
- `dependency.check.ts` - Vulnerability scanning (npm/pnpm audit)
- `eslint.check.ts` - Code quality enforcement
- `policy.check.ts` - OPA/Rego policy evaluation
- `secret.check.ts` - Pattern + entropy detection, git history

### Adapter Framework (`packages/adapters/`)

- `FormatAdapter` interface: detect(), parse(), serialize(), validate()
- `AdapterRegistry` singleton with auto-detection
- Confidence-based format detection (50% threshold, 30% for generic)

## Environment

- **Node.js**: >=20.0.0
- **pnpm**: >=10.20.0
- **Prettier**: Single quotes, trailing commas (es5), 100 char (80 for md)
- **Line endings**: LF (Unix-style)

## Documentation

| Document                      | Purpose                       |
| ----------------------------- | ----------------------------- |
| `docs/ARCHITECTURE.md`        | System design (1,575 lines)   |
| `docs/adr/`                   | Architecture decision records |
| `docs/planning/TODO.md`       | Task tracking                 |
| `core/API.md`                 | APS Core API reference        |
| `packages/adapters/README.md` | Adapter framework guide       |

## Package-Specific Instructions

See AGENTS.md in each package:

- `core/AGENTS.md` - Schema, validation, gates, crypto
- `cli/AGENTS.md` - Commands, services, TUI
- `packages/adapters/AGENTS.md` - Format adapter framework
- `packages/aps/AGENTS.md` - APS document management
