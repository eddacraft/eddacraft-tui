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

| Phase                        | Status         | Progress |
| ---------------------------- | -------------- | -------- |
| Phase 1: Infrastructure      | ✅ Complete    | 100%     |
| Phase 2: APS Core            | ✅ Complete    | 100%     |
| Phase 2.5: Adapter Framework | ✅ Complete    | 100%     |
| Phase 2.5: SpecKit Adapter   | ✅ Complete    | 100%     |
| Phase 3: CLI Integration     | 🚧 In Progress | 80%      |
| Phase 4: Gate v1             | ✅ Complete    | 100%     |
| Phase 5: OPA/Rego            | ⏳ Planned     | 0%       |
| **Overall Progress**         | **~35%**       | **MVP**  |

See [TODO.md](./TODO.md) for detailed task tracking and [PLAN.md](./PLAN.md) for
the strategic roadmap.

### 🎉 Recent Milestones

- **2025-10-21**: CLI integration with SpecKit adapter (In Progress)
  - Format auto-detection service implemented
  - Plan loader with multi-format support
  - Enhanced `validate` and `gate` commands with adapter support
  - Type system for CLI integration complete
  - **Next**: Fix build errors and test end-to-end

- **2025-10-14**: SpecKit adapter complete
  - Full v1 and v2 format support
  - Import and export adapters
  - 51 tests (49 passing, 2 minor fixes pending)
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

- **Monorepo Structure**: Organized workspace with `cli/`, `ui/`, `core/`, and
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
├── cli/                      # Command-line interface application
│   ├── src/commands/         # CLI commands (validate, gate, etc.)
│   ├── src/services/         # Format detection, plan loading
│   └── src/types/            # TypeScript type definitions
├── ui/                       # User interface components
├── core/                     # Shared core functionality (APS schema, validation, hashing)
├── packages/adapters/        # Format adapters
│   ├── src/base/             # Adapter framework (registry, types)
│   ├── src/speckit/          # SpecKit adapter ✅
│   └── src/bmad/             # BMAD adapter (planned)
├── packs/                    # Package bundles (future)
└── e2e/                      # End-to-end tests
```

## 🛠️ Getting Started

### Prerequisites

- Node.js 18.x, 20.x, or 22.x
- pnpm 10.16.1 or higher

### Installation

```bash
# Install dependencies
pnpm install

# Build all packages
pnpm build
```

## 📜 Available Scripts

### Development

```bash
# Run tests
pnpm test              # Run unit tests
pnpm test:ui           # Run tests with UI
pnpm test:coverage     # Run tests with coverage
pnpm test:e2e          # Run Playwright E2E tests

# Code quality
pnpm lint              # Lint and fix code
pnpm lint:check        # Check linting issues
pnpm format            # Format code with Prettier
pnpm format:check      # Check formatting issues
pnpm typecheck         # Type check with TypeScript

# Build
pnpm build             # Build all packages
```

### Nx Commands

```bash
# Generate a new library
npx nx g @nx/js:lib packages/<name> --publishable --importPath=@anvil/<name>

# Run specific package commands
npx nx build <package-name>
npx nx test <package-name>

# Dependency graph
npx nx graph

# Run affected commands
npx nx affected -t lint
npx nx affected -t test
npx nx affected -t build
```

## 🧪 Testing

### Unit Tests (Vitest)

```bash
pnpm test
```

Tests are located next to source files with `.test.ts` or `.spec.ts` extensions.

### E2E Tests (Playwright)

```bash
pnpm test:e2e
```

E2E tests are located in the `e2e/` directory.

## 🔧 Configuration

### TypeScript

- Base configuration: `tsconfig.base.json`
- Project references configured for each package
- Strict mode enabled

### ESLint

- Configuration: `eslint.config.mjs`
- TypeScript support with `@typescript-eslint`
- Prettier integration

### Prettier

- Configuration: `.prettierrc`
- Consistent code formatting across the project

### Git Hooks (Husky)

- Pre-commit: Runs lint-staged for changed files
- Automatic formatting and linting before commits

## 🚀 CI/CD

GitHub Actions workflow includes:

- **Matrix Testing**: Tests against Node.js 18.x, 20.x, and 22.x
- **Caching**: pnpm store cached for faster builds
- **Parallel Jobs**: Lint, test, and build run efficiently
- **Coverage Reports**: Uploaded as artifacts
- **E2E Tests**: Playwright tests with report uploads
- **Nx Affected**: Runs only affected project tasks

## 📚 Learn More

- [Nx Documentation](https://nx.dev)
- [pnpm Documentation](https://pnpm.io)
- [TypeScript Documentation](https://www.typescriptlang.org/docs/)
- [Vitest Documentation](https://vitest.dev)
- [Playwright Documentation](https://playwright.dev)

## 🤝 Contributing

Contributions are welcome! Please ensure:

1. All tests pass (`pnpm test`)
2. Code is properly formatted (`pnpm format`)
3. No linting errors (`pnpm lint:check`)
4. TypeScript compiles without errors (`pnpm typecheck`)

## 💡 Nx Console

For an enhanced development experience, install the Nx Console extension for
your IDE:

- [VSCode Extension](https://marketplace.visualstudio.com/items?itemName=nrwl.angular-console)
- [IntelliJ Plugin](https://plugins.jetbrains.com/plugin/15000-nx-console)

## 🔗 Useful Links

- [Nx Cloud](https://nx.app) - Distributed caching and task execution
- [Nx Plugins](https://nx.dev/concepts/nx-plugins) - Extend Nx capabilities
- [Nx CI Setup](https://nx.dev/ci/intro/ci-with-nx) - Optimize CI pipelines
