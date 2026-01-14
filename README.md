# Anvil

[![CI](https://github.com/EddaCraft/anvil-001/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/EddaCraft/anvil-001/actions/workflows/ci.yml)
[![pnpm](https://img.shields.io/badge/maintained%20with-pnpm-cc00ff.svg?style=flat-square)](https://pnpm.io/)
[![TypeScript](https://img.shields.io/badge/TypeScript-5.9-blue.svg?style=flat-square)](https://www.typescriptlang.org/)
[![Node.js](https://img.shields.io/badge/Node.js->=20.0.0-339933.svg?style=flat-square&logo=node.js&logoColor=white)](https://nodejs.org/)

> **Catch architecture drift and AI anti-patterns at file save — before they
> reach code review**

## What is Anvil?

Anvil makes AI-generated code safe for production by catching architecture
boundary violations and common anti-patterns **at the moment of creation** —
when fixing is cheap.

```bash
# Check changed files for issues
anvil check --changed

# Watch source files in real-time
anvil watch --source

# Get actionable warnings, not hard blocks
⚠ [AP-003] Explicit any type detected
  src/api/handler.ts:42
  Using 'any' defeats type safety
  Fix: Define a proper type or use 'unknown'
```

### Why Anvil?

AI coding tools produce code that compiles and passes tests, but **drifts from
your intended architecture**. By the time drift is noticed in review, it's
already merged or too expensive to fix.

Anvil catches it at file save:

- **Architecture violations** — New dependencies crossing boundaries you defined
- **AI escape hatches** — `any`, `@ts-ignore`, empty catch blocks
- **With accountability** — Suppress with `@anvil-ignore`, but explain why

### What It Doesn't Do

Anvil **warns**, it doesn't block. Your CI can enforce if you want, but the
default is informational. We trust developers to make informed decisions.

## Quick Start

```bash
# Clone and build (pre-release)
git clone https://github.com/EddaCraft/anvil-001.git
cd anvil-001 && pnpm install && pnpm build

# Link CLI globally
pnpm link:cli

# Set up in your project
cd /path/to/your/project
anvil init

# Run your first check
anvil check --changed
```

See the [Quick Start Guide](./docs/QUICK_START.md) for detailed instructions.

## Core Commands

### `anvil check` — Analyse files for issues

```bash
# Check specific files
anvil check src/api/*.ts

# Check git-changed files only
anvil check --changed

# Check staged files (pre-commit)
anvil check --changed --staged

# Check against a branch
anvil check --changed --since main

# Verbose output with fix suggestions
anvil check --changed --verbose
```

### `anvil watch` — Real-time feedback

```bash
# Watch source files and check on save
anvil watch --source

# Custom patterns
anvil watch --patterns "src/**/*.ts,lib/**/*.ts"

# With gate profile for fast iteration
anvil watch --source --profile dev
```

### `anvil status` — Quick health check

```bash
anvil status

# Shows:
# • HOOKS - pre-commit/pre-push status
# • CONFIGURATION - current settings
# • RECENT RESULTS - validation history
```

### `anvil doctor` — Diagnose setup

```bash
anvil doctor        # Check environment
anvil doctor --fix  # Auto-fix issues
```

## What Anvil Catches

### Anti-Patterns (7 high-signal patterns)

| ID     | Pattern                      | Why It Matters                     |
| ------ | ---------------------------- | ---------------------------------- |
| AP-001 | Broad `/* eslint-disable */` | Silences all linting for file      |
| AP-003 | Explicit `any` type          | Defeats TypeScript's purpose       |
| AP-004 | `@ts-ignore` directive       | Ignores type errors without fixing |
| AP-006 | Empty catch block            | Silently swallows errors           |
| AP-007 | Console in production code   | Should use proper logging (opt-in) |

### Architecture Boundaries

Anvil detects **new dependency edges** that cross architectural contexts:

```
⚠ [ARCH-001] New cross-boundary dependency
  src/api/handler.ts → src/database/queries.ts
  API layer should not directly access database layer
  Consider: Use the service layer as intermediary
```

### Suppression with Accountability

When you need to bypass a warning, explain why:

```typescript
// @anvil-ignore AP-003: Third-party SDK requires any for callback
const handler = sdk.createHandler(callback as any);
```

Suppressions are tracked. You'll see them in `anvil status` and drift reports.

## CI/CD Integration

### GitHub Action

```yaml
name: Anvil Check

on:
  pull_request:
    types: [opened, synchronize, reopened]

permissions:
  contents: read
  pull-requests: write
  checks: write

jobs:
  anvil:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: ./.github/actions/anvil-check
```

This provides:

- Automatic changed-file detection
- PR comment summaries
- Inline annotations
- Non-blocking by default (use `fail-on-warnings: true` for blocking)

See [GitHub Action docs](./.github/actions/anvil-check/README.md) for options.

## Configuration

### `.anvilrc`

```json
{
  "checks": {
    "architecture": {
      "enabled": true,
      "baseline": ".anvil/baseline.json"
    },
    "antipattern": {
      "enabled": true,
      "patterns": ["AP-001", "AP-003", "AP-004", "AP-006"]
    }
  },
  "watch": {
    "patterns": ["src/**/*.ts"],
    "debounceMs": 300
  }
}
```

### Gate Profiles

```bash
# Fast iteration (skip slow checks)
anvil check --changed --profile dev

# Full CI checks
anvil check --changed --profile ci

# Production release
anvil check --changed --profile production
```

## Project Status

| Component            | Status      |
| -------------------- | ----------- |
| Analysis Engine      | ✅ Complete |
| Architecture Safety  | ✅ Complete |
| Anti-pattern Library | ✅ Complete |
| Suppression System   | ✅ Complete |
| Git Integration      | ✅ Complete |
| Watch Mode           | ✅ Complete |
| TUI (init, status)   | ✅ Complete |
| GitHub Action        | ✅ Complete |
| OPA Policy Engine    | ✅ Complete |
| VS Code Extension    | ✅ Complete |

See [plans/index.aps.md](./plans/index.aps.md) for detailed roadmap.

## Documentation

| Document                                     | Description                 |
| -------------------------------------------- | --------------------------- |
| [Quick Start](./docs/QUICK_START.md)         | Get running in 5 minutes    |
| [User Guide](./docs/USER_GUIDE.md)           | Complete command reference  |
| [Examples](./docs/EXAMPLES.md)               | Real-world workflows        |
| [Troubleshooting](./docs/TROUBLESHOOTING.md) | Common issues and solutions |
| [CLI Reference](./apps/anvil-cli/README.md)  | All CLI commands            |
| [Architecture](./docs/ARCHITECTURE.md)       | System design               |

## Development

```bash
# Install dependencies
pnpm install

# Build all packages
pnpm build

# Run tests
pnpm test

# Type checking
pnpm typecheck

# Linting
pnpm lint
```

### Project Structure

```
anvil/
├── apps/
│   ├── anvil-cli/        # CLI application (Commander.js + Ink TUI)
│   └── e2e/              # E2E test suites
├── core/                 # Legacy core (being migrated)
├── packages/
│   ├── adapters/         # Format converters (SpecKit, BMAD)
│   ├── aps/              # APS document parser
│   ├── anvil/            # New modular core packages
│   │   ├── contracts/    # Shared interfaces & types
│   │   ├── ports/        # Adapter interfaces
│   │   ├── core/         # Pure domain logic
│   │   ├── runtime/      # Execution environment
│   │   └── policy/       # Policy evaluation
│   ├── platform/         # Platform services
│   │   ├── config/       # Configuration management
│   │   ├── storage/      # Storage abstractions
│   │   └── crypto/       # Cryptographic utilities
│   ├── tooling/          # Build & dev tooling
│   │   ├── tsconfig/     # Shared TypeScript configs
│   │   └── eslint-config/# Shared ESLint configs
│   └── vscode-extension/ # VS Code integration
├── tools/
│   ├── scripts/          # Build & utility scripts
│   ├── generators/       # Code generators
│   └── codemods/         # Codemod transformations
├── plans/                # Project planning (.aps.md specs)
└── docs/                 # Documentation
```

### Code Conventions

- **UK English** — organise, colour, behaviour
- **ESM with .js extensions** — `import { foo } from './bar.js'`
- **Zod-first schemas** — Define with Zod, export inferred types
- **Tests co-located** — `file.ts` + `file.test.ts`

## Contributing

1. Fork and clone
2. Create feature branch: `git checkout -b feature/my-feature`
3. Make changes, run `pnpm test && pnpm typecheck && pnpm lint`
4. Open PR

See [AGENTS.md](./AGENTS.md) for AI-assisted development instructions.

## License

[MIT](./LICENSE)

---

**Questions?** Open an
[issue](https://github.com/EddaCraft/anvil-001/issues/new) or see the
[troubleshooting guide](./docs/TROUBLESHOOTING.md).
