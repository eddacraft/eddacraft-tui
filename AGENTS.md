# Anvil - AI Agent Instructions

## What is Anvil?

Anvil is a **deterministic development automation platform** built by
[EddaCraft](https://eddacraft.dev). It catches architecture drift and AI
anti-patterns at file save — before they reach code review.

The core idea: probabilistic tools (LLMs, AI code agents) generate code, but
Anvil enforces **deterministic rules** over that output. Every change is
validated through quality gates, every decision is recorded in an immutable audit
trail, and every mutation is reversible.

Anvil works with the planning formats teams already use (SpecKit, BMAD) by
translating them into its own hash-stable internal format — **APS (Anvil Plan
Specification)** — which enables deterministic validation, drift detection, and
provenance tracking.

```
External Format → Adapter → APS (internal) → Gate → Execute → Evidence
```

**Key capabilities:**

- **Quality gates** — lint, test coverage, secret detection, dependency audit,
  architecture checks — run automatically before changes land
- **Plan validation** — specs describe intent, tasks authorise execution, steps
  are observable checkpoints
- **Audit trail** — SHA-256 hashed provenance for every change, immutable
  evidence chain
- **Rollback** — snapshots before execution, so every mutation is reversible
- **Format adapters** — SpecKit, BMAD, and generic formats convert to APS
  transparently

This is an **Nx monorepo** using **pnpm** workspaces, **TypeScript** throughout,
and **Vitest** for testing. The product ships as a CLI (`@eddacraft/anvil-cli`),
with a marketing website and documentation site deployed on Vercel.

## Technical Overview

Anvil validates AI-generated code changes through **APS (Anvil Plan
Specification)** – a hash-stable internal format enabling deterministic
validation. Users work in external formats (SpecKit, BMAD); Anvil translates
internally.

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
├── apps/
│   ├── anvil-cli/          # @eddacraft/anvil-cli - Commander.js CLI with TUI (Ink)
│   ├── website/            # Next.js 16 marketing site (Vercel)
│   ├── docs-site/          # Docusaurus 3.9 documentation hub (Vercel)
│   ├── anvil-api/          # API service (placeholder)
│   ├── anvil-ui/           # Web UI (placeholder)
│   └── e2e/                # Playwright E2E tests
├── packages/
│   ├── adapters/           # Format converters (SpecKit, BMAD, Generic)
│   ├── anvil/
│   │   ├── contracts/      # Schemas, types, events (zero deps)
│   │   ├── ports/          # Interface definitions
│   │   ├── core/           # @eddacraft/anvil-core - Schema, validation, crypto
│   │   ├── runtime/        # Gate checks, execution engine
│   │   └── policy/         # OPA/Rego policy wrappers
│   ├── aps/                # APS document parser, validator, state management
│   ├── platform/
│   │   ├── config/         # Configuration loading & validation
│   │   ├── storage/        # File system abstractions
│   │   └── crypto/         # Hashing, signing, verification
│   ├── edda-stack/         # Memory stack (Kindling · Ember · Edda)
│   ├── kindling-integration/ # Kindling memory contracts
│   ├── eslint-plugin-anvil/  # ESLint test quality rules
│   ├── vscode-extension/   # VS Code integration
│   └── tooling/
│       ├── tsconfig/       # Shared TypeScript configs
│       └── eslint-config/  # Shared ESLint configs
├── tools/
│   ├── scripts/            # Build & utility scripts
│   └── generators/         # Nx code generators
├── plans/                  # .aps.md specs, execution/*.steps.md, decisions/
└── docs/                   # Architecture, guides
```

## Commands

```bash
# Essential
pnpm build                    # Build all (REQUIRED before cross-package testing)
pnpm test                     # Unit tests (Vitest)
pnpm lint                     # ESLint + markdownlint with auto-fix
pnpm typecheck                # TypeScript strict mode
pnpm format                   # Prettier formatting with auto-fix

# Package-specific
nx test core --testNamePattern="validator"    # Test pattern
pnpm -F core run generate:schema              # Regenerate JSON schema from Zod
pnpm -F core run update-golden-hashes         # Update golden test hashes

# CLI development
pnpm link:cli                 # Build and link globally ('anvil' command)
pnpm unlink:cli               # Unlink when done
```

## Before Committing (Required)

**All changes must pass these checks before committing:**

```bash
pnpm build           # Build all packages
pnpm test            # Run unit tests
pnpm lint            # ESLint + markdownlint
pnpm format          # Prettier formatting
pnpm typecheck       # TypeScript strict mode
```

Or run the full verification in one go:

```bash
pnpm build && pnpm test && pnpm lint && pnpm format && pnpm typecheck
```

**CI will reject PRs that fail any of these checks.** Fix issues locally before
pushing to avoid failed CI runs.

## Where to Look

| Task               | Location                                       | Notes                                                  |
| ------------------ | ---------------------------------------------- | ------------------------------------------------------ |
| APS schema changes | `packages/anvil/core/src/schema/aps.schema.ts` | Run generate:schema + update-golden-hashes after       |
| Add gate check     | `packages/anvil/runtime/src/gate/checks/`      | Extend BaseCheck, register in gate-runner.ts           |
| Add CLI command    | `apps/anvil-cli/src/commands/`                 | Use create{Name}Command() factory pattern              |
| Add format adapter | `packages/adapters/src/`                       | Implement FormatAdapter, register with AdapterRegistry |
| TUI components     | `apps/anvil-cli/src/tui/components/`           | Ink/React components with useInput hooks               |
| Validation rules   | `packages/aps/src/validator/`                  | AST-based with remark-parse                            |
| **Planning/specs** | `plans/aps-rules.md`                           | **READ FIRST** before creating/editing .aps.md files   |

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

### Node.js Built-in Imports with `node:` Protocol

Always use the `node:` protocol prefix for Node.js built-in modules. This is
enforced by ESLint (`unicorn/prefer-node-protocol`).

```typescript
// ✅ Correct - use node: protocol
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { promisify } from 'node:util';

// ❌ Wrong - missing node: prefix
import { readFileSync } from 'fs';
import { join } from 'path';
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

| Pattern                                        | Why Forbidden                                            |
| ---------------------------------------------- | -------------------------------------------------------- |
| `as any`, `@ts-ignore`, `@ts-expect-error`     | Type safety is non-negotiable                            |
| Empty catch blocks `catch(e) {}`               | Silently swallows errors                                 |
| Missing `.js` import extensions                | ESM runtime failure                                      |
| Imports without path aliases crossing packages | Use `@eddacraft/anvil-core`, `@eddacraft/anvil-adapters` |
| `cd dir && command` in bash                    | Use workdir parameter instead                            |

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

1. **Fix issues as you discover them** – maintain codebase hygiene
2. **Keep fixes minimal and focused** – don't refactor while fixing
3. **Separate concerns** – use a different commit for unrelated fixes
4. **Note what you fixed** – mention in your summary so nothing is silently
   changed

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

### Gate Checks (`packages/anvil/runtime/src/gate/checks/`)

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
| `docs/ARCHITECTURE.md`        | System design                 |
| `docs/TESTING.md`             | Testing best practices        |
| `plans/decisions/`            | Architecture decision records |
| `packages/adapters/README.md` | Adapter framework guide       |

## Package-Specific Instructions

See AGENTS.md in each package:

- `apps/anvil-cli/AGENTS.md` - Commands, services, TUI
- `apps/website/AGENTS.md` - Next.js marketing site
- `apps/docs-site/AGENTS.md` - Docusaurus documentation hub
- `packages/adapters/AGENTS.md` - Format adapter framework
- `packages/aps/AGENTS.md` - APS document management
