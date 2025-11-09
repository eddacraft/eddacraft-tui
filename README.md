# Anvil

[![CI](https://github.com/EddaCraft/anvil-001/actions/workflows/ci.yml/badge.svg)](https://github.com/EddaCraft/anvil-001/actions/workflows/ci.yml)
[![pnpm](https://img.shields.io/badge/maintained%20with-pnpm-cc00ff.svg?style=flat-square)](https://pnpm.io/)
[![TypeScript](https://img.shields.io/badge/TypeScript-5.9-blue.svg?style=flat-square)](https://www.typescriptlang.org/)
[![Node.js](https://img.shields.io/badge/Node.js->=18.0.0-339933.svg?style=flat-square&logo=node.js&logoColor=white)](https://nodejs.org/)
[![Nx](https://img.shields.io/badge/Nx-21.5.2-143055.svg?style=flat-square)](https://nx.dev)
[![PRs Welcome](https://img.shields.io/badge/PRs-welcome-brightgreen.svg?style=flat-square)](http://makeapullrequest.com)

> **Deterministic development automation platform that makes AI-generated code
> changes safe for production**

## 🎯 What is Anvil?

Anvil is a validation and execution pipeline that transforms AI and human intent
into validated, auditable, and reversible changes. It solves a critical problem:
**making AI-generated code changes safe for production** by enforcing quality
gates, maintaining complete audit trails, and ensuring every change is
reversible.

### What Makes Anvil Different

- **Format Agnostic**: Work with SpecKit, BMAD, or native APS – your choice
- **Comprehensive Gates**: ESLint, Vitest, coverage thresholds, secret scanning,
  OPA policies
- **Production Safety**: Snapshot-based rollback with transactional execution
- **Full Provenance**: Track who, what, when, why for every change
- **GitHub Native**: First-class PR integration that blocks unsafe merges

### Core Design Principles

1. **Interoperability First**: Users keep their existing planning formats. Anvil
   works with them through pluggable adapters rather than forcing format
   adoption.
2. **Determinism**: Same input → same output, always. Hash-stable plans enable
   reproducible validation, tamper detection, and confident reviews.
3. **Safety by Default**: Every change creates a snapshot before application.
   Rollback is a first-class operation, not an afterthought.
4. **Transparency**: Immutable, append-only evidence bundles provide complete
   audit trails for compliance and debugging.
5. **Composability**: Parse → Validate → Execute are independent, composable
   stages. Each component has clear interfaces and can be used in library mode.

## 🏗️ Architecture & Pipeline Flow

Anvil operates as a validation and execution pipeline built around the **Anvil
Plan Specification (APS)** – a hash-stable, internal interchange format that
enables deterministic validation and governance:

```
User Formats (SpecKit, BMAD, etc.)
    ↓
Adapters (parse to APS)
    ↓
Quality Gates (lint, test, coverage, secrets, policies)
    ↓
Safe Execution (apply with snapshots, rollback capability)
    ↓
Immutable Audit Trail
```

**Key Insight**: APS is the moat. It's hash-stable (SHA-256), schema-validated
(Zod), and enables deterministic validation. Users work in their preferred
format; Anvil translates everything to APS internally.

## 📊 Project Status

| Phase                        | Status      | Progress |
| ---------------------------- | ----------- | -------- |
| Phase 1: Infrastructure      | ✅ Complete | 100%     |
| Phase 2: APS Core            | ✅ Complete | 100%     |
| Phase 2.5: Adapter Framework | ✅ Complete | 100%     |
| Phase 2.5: SpecKit Adapter   | ✅ Complete | 100%     |
| Phase 2.5: BMAD Adapter      | ✅ Complete | 100%     |
| Phase 3: CLI Integration     | ✅ Complete | 100%     |
| Phase 4: Gate v1             | ✅ Complete | 100%     |
| Phase 5: OPA/Rego            | ⏳ Planned  | 0%       |
| **Overall Progress**         | **~48%**    | **MVP**  |

See [docs/planning/TODO.md](./docs/planning/TODO.md) for detailed task tracking
and [docs/planning/PLAN.md](./docs/planning/PLAN.md) for the strategic roadmap.

### 🎉 Recent Milestones

- **2025-10-23**: ✅ BMAD FormatAdapter implementation complete
  - Full FormatAdapter interface compliance
  - Format detection with 100% confidence (5 weighted indicators)
  - Complete BMAD → APS → BMAD conversion
  - CLI integration verified (validate, gate, export commands)
  - Registry auto-registration
  - Serves as reference for SpecKit adapter migration
  - **Ready for**: Customer #2 pilot demos (PRD/Architecture format)

- **2025-10-23**: ✅ CLI integration with SpecKit adapter complete
  - All 69 adapter tests passing
  - Format auto-detection service (FormatDetectionService)
  - Plan loader with multi-format support (PlanLoader)
  - `anvil validate` command with SpecKit/APS support
  - `anvil gate` command with multi-format adapter support
  - `anvil export` command for format conversion (SpecKit ↔ APS)
  - Evidence collection integrated (injection deferred to post-MVP)
  - **Ready for**: Customer #1 pilot demos

- **2025-10-14**: SpecKit adapter complete
  - Full v1 and v2 format support
  - Import and export adapters
  - 69 tests (all passing)
  - Comprehensive parser for spec.md, plan.md, tasks.md
  - Registry integration with auto-detection

- **2025-10-13**: Adapter framework complete
  - FormatAdapter interface and base types
  - AdapterRegistry with auto-detection
  - Testing utilities and documentation
  - 22 framework tests (100% passing)

- **2025-10-10**: CLI + APS integration complete
  - Core package exports all APS utilities
  - CLI successfully imports and uses core functionality
  - TypeScript configuration fixed for proper build outputs
  - Gate types aligned with APS schema v0.1.0

## 🚀 Features

- **Monorepo Structure**: Organised workspace with `cli/`, `ui/`, `core/`, and
  `packs/` packages
- **TypeScript**: Full TypeScript support with project references
- **Testing**: Vitest for unit tests, Playwright for E2E tests
- **Code Quality**: ESLint, Prettier, and Husky pre-commit hooks
- **CI/CD**: GitHub Actions workflow with caching and matrix testing
- **Developer Experience**: Fast builds with Nx, efficient package management
  with pnpm

## 📦 Project Structure

```
anvil/
├── .claude/                  # Claude Projects Lite configuration
│   ├── addons/               # Reusable agent extensions (git-workflow, repository, etc.)
│   ├── agents/               # AI agent definitions (planner, architect, coder, etc.)
│   ├── commands/             # Slash commands (/feature, /ship, /new-project)
│   ├── docs-templates/       # Documentation templates (PRD, ADR, etc.)
│   ├── skills/               # Reusable agent skills (debugging, refactoring, etc.)
│   └── USAGE.md              # Guide to using Claude agents
├── cli/                      # Command-line interface application
│   ├── src/commands/         # CLI commands (validate, gate, plan)
│   ├── src/services/         # Format detection, plan loading
│   └── src/types/            # TypeScript type definitions
├── core/                     # APS schema, validation, hashing, gate runner
│   ├── src/schema/           # aps.schema.ts - Zod schema (v0.1.0)
│   ├── src/crypto/           # SHA-256 deterministic hashing
│   ├── src/gate/             # Quality gate checks (ESLint, coverage, secrets)
│   └── src/validation/       # APS validator with rich error messages
├── packages/adapters/        # Format conversion (SpecKit ✅, BMAD ✅)
│   ├── src/base/             # FormatAdapter interface, AdapterRegistry
│   ├── src/speckit/          # GitHub spec-kit adapter (2.5k LOC, 51 tests)
│   └── src/bmad/             # BMAD PRD/architecture adapter (~800 LOC)
├── packs/                    # [Future] Feature bundles (flags, telemetry)
├── docs/                     # Project documentation
│   ├── ARCHITECTURE.md       # System design (1,575 lines)
│   ├── planning/             # Strategic planning and task tracking
│   ├── guides/               # Development guides and workflows
│   ├── status/               # Project status and known issues
│   ├── prd/                  # Product requirements
│   └── adr/                  # Architecture decision records
├── e2e/                      # End-to-end tests (Playwright)
└── ui/                       # [Future] User interface components
```

## 🛠️ Getting Started

### Prerequisites

- **Node.js**: 18.x, 20.x, or 22.x
- **pnpm**: 10.17.1 or higher (enforced by `packageManager` in package.json)
- **Git**: For version control and Claude agent workflows

### Installation

```bash
# Clone the repository
git clone https://github.com/EddaCraft/anvil-001.git
cd anvil-001

# Install dependencies
pnpm install

# Build all packages
pnpm build

# Verify installation
pnpm test
```

## 🤖 AI-Assisted Development

### Claude Projects Lite

Anvil includes [Claude Projects Lite](https://github.com/pchaganti/bx-sophia)
for AI-assisted development. This provides:

- **AI Agents**: Pre-configured agents for planning, architecture, coding,
  review, testing
- **Slash Commands**: Quick workflows like `/feature`, `/ship`, `/new-project`
- **Documentation Templates**: PRD, ADR, test plans, etc.
- **Skills**: Reusable patterns for debugging, refactoring, tracing data flows

#### Quick Start with Claude

1. Open this repository in [Claude Code](https://claude.ai/code)
2. Activate addons for your workflow (see `.claude/addons/ACTIVATION.md`)
3. Use slash commands:
   ```
   /feature "add BMAD adapter"
   /ship "review CLI integration PR"
   /prime-repo "scan entire codebase"
   ```
4. See `.claude/USAGE.md` for complete documentation

#### Available Agents

Located in `.claude/agents/`:

- **planner** – Breaks goals into steps with success criteria
- **architect** – Defines interfaces, file changes, schema tweaks
- **product-manager** – Writes crisp PRDs with use cases
- **coder** – Implements minimal changes with diffs
- **tester** – Produces test plans and code
- **reviewer** – Pragmatic code review
- **docs-writer** – Updates READMEs, ADRs, usage notes
- **security-auditor** – Checks auth, PII, dependency risks

#### Useful Skills

Located in `.claude/skills/`:

- `debug-adapter.md` – Debug format adapter issues
- `feature-adapter.md` – Add new format adapters
- `fix-build.md` – Resolve TypeScript/build errors
- `refactor-safe.md` – Safe refactoring patterns
- `trace-data-flow.md` – Understand data transformations

### GitHub Copilot

For VS Code/GitHub Copilot users, see
[`.github/copilot-instructions.md`](.github/copilot-instructions.md) for:

- Language conventions (UK English)
- Architecture patterns
- Code conventions
- Testing strategies
- Common gotchas

## 📜 Available Scripts

### Development Workflow

```bash
# Install dependencies (first time or after package.json changes)
pnpm install

# Build all packages (required before testing cross-package changes)
pnpm build

# Run tests
pnpm test              # Unit tests with Vitest
pnpm test:ui           # Tests with interactive UI
pnpm test:coverage     # Tests with coverage reports
pnpm test:e2e          # Playwright E2E tests

# Code quality
pnpm lint              # Lint and auto-fix code
pnpm lint:check        # Check linting without fixing
pnpm format            # Format code with Prettier
pnpm format:check      # Check formatting without fixing
pnpm typecheck         # TypeScript strict mode validation
```

### Nx Commands

```bash
# Build specific package
npx nx build core                    # Build @anvil/core
npx nx build adapters                # Build @anvil/adapters
npx nx build cli                     # Build CLI

# Test specific package
npx nx test core                     # Test core package
npx nx test adapters                 # Test adapters
npx nx test core --testNamePattern="validator"  # Run specific tests

# Visualise dependency graph
npx nx graph

# Run affected commands (only changed packages)
npx nx affected -t lint
npx nx affected -t test
npx nx affected -t build

# Generate new library
npx nx g @nx/js:lib packages/<name> --publishable --importPath=@anvil/<name>
```

### Package-Specific Commands

```bash
# APS Core utilities
pnpm -F core run generate:schema          # Regenerate JSON schema from Zod
pnpm -F core run update-golden-hashes     # Update golden test hashes

# CLI Development
cd cli
pnpm build
node dist/index.js validate <plan>        # Run validate command
node dist/index.js gate <plan>            # Run gate command
```

### Important: Build Before Testing

**Critical**: TypeScript project references require explicit builds before
cross-package imports work:

```bash
pnpm build        # Build all packages first
pnpm test         # Now tests can import from other packages
```

## 🧪 Testing

### Unit Tests (Vitest)

Tests are co-located with source files using `.test.ts` or `.spec.ts`
extensions.

```bash
# Run all tests
pnpm test

# Run with coverage
pnpm test:coverage

# Run with UI
pnpm test:ui

# Run specific package tests
npx nx test core
npx nx test adapters

# Run specific test pattern
npx nx test core --testNamePattern="validator"
npx nx test adapters --testNamePattern="SpecKit"
```

**Test Patterns**:

- **Golden Files**: `core/src/__fixtures__/golden-plans/` contain hash-verified
  reference plans
- **Fixtures**: Use `__fixtures__/` directories for test data
- **Integration Tests**: See `cli/src/__tests__/cli-aps-integration.test.ts`

### E2E Tests (Playwright)

```bash
# Run E2E tests
pnpm test:e2e

# Run with UI
pnpm test:e2e:ui

# View last test report
npx playwright show-report
```

E2E tests are located in the `e2e/` directory.

## 🔧 Configuration

### TypeScript

- **Base Config**: `tsconfig.base.json` with strict mode enabled
- **Module System**: ES2022 with `"module": "nodenext"` (requires `.js`
  extensions in imports)
- **Project References**: Each package references dependencies for proper build
  order
- **Path Aliases**: `@anvil/adapters` → `packages/adapters/src/index.ts`

**Important**: All imports must use `.js` extensions even for TypeScript files:

```typescript
// ❌ Wrong
import { foo } from './utils';

// ✅ Correct
import { foo } from './utils.js';
```

### ESLint

- **Configuration**: `eslint.config.mjs` (flat config format)
- **TypeScript Support**: `@typescript-eslint` with strict rules
- **Prettier Integration**: `eslint-plugin-prettier` for formatting
- **Auto-fix**: Pre-commit hook via Husky

### Prettier

- **Configuration**: `.prettierrc`
- **Enforced On**: Pre-commit via `lint-staged`
- **Rules**: 100 character line length, single quotes, trailing commas

### Git Hooks (Husky)

- **Pre-commit**: Runs `lint-staged` on changed files
- **Includes**: ESLint auto-fix and Prettier formatting
- **Configuration**: `.lintstagedrc.json`

## 🚀 CI/CD

GitHub Actions workflow (`.github/workflows/ci.yml`) includes:

- **Matrix Testing**: Node.js 18.x, 20.x, and 22.x
- **Caching**: pnpm store cached for faster builds (~1-2 minute builds)
- **Parallel Jobs**: Lint, test, typecheck, and build run in parallel
- **Coverage Reports**: Uploaded as artifacts for analysis
- **E2E Tests**: Playwright tests with HTML report uploads
- **Nx Affected**: Only runs tasks for changed packages

### Current CI Status

All checks must pass before merging:

- ✅ Lint (ESLint)
- ✅ Format check (Prettier)
- ✅ Type check (TypeScript strict)
- ✅ Unit tests (Vitest)
- ✅ E2E tests (Playwright)
- ✅ Build (all packages)

## 📚 Documentation

### Core Documentation

- **[docs/ARCHITECTURE.md](./docs/ARCHITECTURE.md)** – System design,
  principles, data flows (1,575 lines)
- **[docs/planning/PLAN.md](./docs/planning/PLAN.md)** – Strategic plan and
  three-act vision
- **[docs/planning/TODO.md](./docs/planning/TODO.md)** – Comprehensive task list
  with progress tracking
- **[docs/planning/ROADMAP.md](./docs/planning/ROADMAP.md)** – High-level
  milestones and phases
- **[docs/guides/CLAUDE.md](./docs/guides/CLAUDE.md)** – Project overview and
  commands
- **[docs/DOCUMENTATION.md](./docs/DOCUMENTATION.md)** – Documentation
  organisation and standards

### Package Documentation

- **[core/API.md](./core/API.md)** – APS Core API reference
- **[core/EXAMPLES.md](./core/EXAMPLES.md)** – Usage examples
- **[core/MIGRATION.md](./core/MIGRATION.md)** – Migration guide
- **[packages/adapters/README.md](./packages/adapters/README.md)** – Adapter
  framework
- **[packages/adapters/ADAPTER_WORKFLOW_GUIDE.md](./packages/adapters/ADAPTER_WORKFLOW_GUIDE.md)**
  – Creating new adapters

### AI Development Guides

- **[.github/copilot-instructions.md](./.github/copilot-instructions.md)** –
  GitHub Copilot instructions
- **[.claude/USAGE.md](./.claude/USAGE.md)** – Claude Projects Lite usage guide
- **[.claude/addons/ACTIVATION.md](./.claude/addons/ACTIVATION.md)** –
  Activating Claude addons

### Workflows & Patterns

- **[docs/GIT_WORKTREE_WORKFLOW.md](./docs/GIT_WORKTREE_WORKFLOW.md)** –
  Parallel plan development
- **[docs/adr/](./docs/adr/)** – Architecture decision records

## 🤝 Contributing

### Code Conventions

**Language**: Use UK English for all documentation, comments, variables, and
user-facing text.

**TypeScript**:

- Strict mode enabled – all type errors must be resolved
- ES2022 + Node.js – use modern syntax (`??`, optional chaining, top-level
  await)
- ES modules – use `.js` extensions in all imports

**Schema & Validation**:

- Always use Zod for schema definition, not manual JSON schemas
- Export TypeScript types via `z.infer<typeof Schema>`
- Update JSON schema after Zod changes: `pnpm -F core run generate:schema`

**Testing**:

- Co-locate tests with source files (`.test.ts` or `.spec.ts`)
- Use fixtures in `__fixtures__/` directories
- Update golden hashes after schema changes:
  `pnpm -F core run update-golden-hashes`

### Contribution Workflow

1. Fork and clone the repository
2. Create a feature branch: `git checkout -b feature/my-feature`
3. Make your changes
4. Run quality checks:
   ```bash
   pnpm build          # Build all packages
   pnpm test           # Run all tests
   pnpm typecheck      # Check types
   pnpm lint           # Lint code
   pnpm format:check   # Check formatting
   ```
5. Commit with conventional commits (use `/commit` with Claude)
6. Push and open a pull request

### Pull Request Requirements

- ✅ All tests pass (`pnpm test`)
- ✅ Code properly formatted (`pnpm format`)
- ✅ No linting errors (`pnpm lint:check`)
- ✅ TypeScript compiles (`pnpm typecheck`)
- ✅ Documentation updated if needed
- ✅ UK English used throughout

## 💡 Development Tools

### Nx Console

Enhanced development experience with IDE extensions:

- **[VS Code Extension](https://marketplace.visualstudio.com/items?itemName=nrwl.angular-console)**
  – Run tasks, visualise graph, generate code
- **[IntelliJ Plugin](https://plugins.jetbrains.com/plugin/15000-nx-console)** –
  JetBrains IDE support

### Claude Code Integration

This repository is optimised for [Claude Code](https://claude.ai/code):

- Pre-configured AI agents for common workflows
- Slash commands for rapid development
- Documentation templates for PRDs, ADRs, etc.
- See [`.claude/USAGE.md`](.claude/USAGE.md) for details

### Useful Links

- **[Nx Documentation](https://nx.dev)** – Monorepo build system
- **[pnpm Documentation](https://pnpm.io)** – Fast, disk space efficient package
  manager
- **[TypeScript Handbook](https://www.typescriptlang.org/docs/)** – TypeScript
  language reference
- **[Vitest Documentation](https://vitest.dev)** – Vite-native unit test
  framework
- **[Playwright Documentation](https://playwright.dev)** – E2E testing framework
- **[Zod Documentation](https://zod.dev)** – TypeScript-first schema validation

## 🐛 Troubleshooting

### Common Issues

**TypeScript "Cannot find module" errors**:

```bash
# Build packages first
pnpm build
```

**Tests failing after schema changes**:

```bash
# Update golden hashes
pnpm -F core run update-golden-hashes
```

**ESM import errors (missing .js extension)**:

```typescript
// Change this:
import { foo } from './utils';
// To this:
import { foo } from './utils.js';
```

**Vitest config not in rootDir**:

- Adjust `rootDir` in `tsconfig.spec.json` to `"."` instead of `"src"`
