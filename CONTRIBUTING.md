# Contributing to Anvil

Thank you for your interest in contributing to Anvil! This guide will help you
get started.

## Getting Started

### Prerequisites

- **Node.js**: >=22.13.0
- **pnpm**: >=10.20.0

### Setup

```bash
# Clone the repository
git clone https://github.com/your-org/anvil.git
cd anvil

# Install dependencies
pnpm install

# Build all packages (required before testing)
pnpm build

# Run tests
pnpm test
```

## Development Workflow

All commands are designed to run from the **repository root**. You should never
need to `cd` into a package directory — use the patterns below instead.

### Essential Commands (repo root)

```bash
pnpm build          # Build all packages (Nx orchestrated, honours dependency graph)
pnpm test           # Run all unit tests (excludes E2E)
pnpm lint           # oxlint + ESLint + Rust lint + markdownlint auto-fix
pnpm lint:check     # Same as lint but without auto-fix (CI mode)
pnpm typecheck      # TypeScript strict mode across all packages (excludes anvil-vscode)
pnpm format         # oxfmt format (write mode)
pnpm format:check   # oxfmt format (check mode, CI)
```

### Build Before Test

TypeScript project references require packages to be built before cross-package
imports resolve at test time:

```bash
pnpm build        # Required once after clone or dependency changes
pnpm test         # Now cross-package imports work
```

### Package-Specific Commands

Use `pnpm -F` (filter) or `nx` to target individual packages:

```bash
# Using pnpm filter (by package name)
pnpm -F @eddacraft/anvil-core test
pnpm -F @eddacraft/anvil-aps test
pnpm -F @eddacraft/anvil-cli test

# Using Nx (by project name)
pnpm exec nx test core
pnpm exec nx test @eddacraft/anvil-aps
pnpm exec nx build @eddacraft/anvil-cli

# Test with pattern filter
pnpm exec nx test core --testNamePattern="validator"

# Run a specific package script
pnpm -F @eddacraft/anvil-aps run generate-templates
```

Both `pnpm -F <package-name>` and `pnpm -C <relative-path>` work for targeting
packages. The canonical form is `pnpm -F` (by package name) because it does not
depend on directory structure.

### E2E and CLI Testing

```bash
pnpm test:e2e       # Playwright E2E tests
pnpm test:e2e:cli   # CLI-only E2E tests
pnpm link:cli       # Build + link 'anvil' command globally
pnpm unlink:cli     # Remove global link
```

### Coverage

```bash
# Per-project coverage
pnpm exec nx test core --coverage

# Full monorepo coverage
pnpm test:coverage
```

## Code Standards

### Language

Use **UK English** throughout:

- `colour`, `behaviour`, `organise`, `initialise`, `serialise`

### TypeScript

- **ESM with .js extensions** (NodeNext module resolution)
- **Zod-first schemas** - define with Zod, export inferred types
- **Strict mode** - no `any`, no `@ts-ignore`

```typescript
// Correct - .js extension required
import { foo } from './utils.js';

// Zod-first pattern
export const MySchema = z.object({ field: z.string() });
export type MyType = z.infer<typeof MySchema>;
```

### Formatting

- **oxfmt**: Single quotes, trailing commas (es5), 100 char width
- **Line endings**: LF (Unix-style)

## Testing

See **[docs/guides/testing.md](docs/guides/testing.md)** for comprehensive
testing guidelines, including:

- Vitest mocking best practices
- Fixture and factory patterns
- Package-specific testing approaches
- Quick checklist for PR review

### Key Testing Principles

1. **Co-locate tests** with source code as `.test.ts`
2. **Don't mock unless necessary** - prefer real implementations
3. **Mock at your boundary** - not deep inside vendors
4. **Clean up** temp directories and restore `process.cwd()`

## Pull Request Process

1. **Create a branch** from `dev` for normal work, or from `main` only for
   production hotfixes
2. **Make your changes** following the code standards above
3. **Run the full test suite**: `pnpm build && pnpm test && pnpm lint`
4. **Submit a PR** with a clear description of changes

## Branching and Worktrees

This repository uses a `main`/`dev` model that supports multiple active streams
in parallel worktrees.

- `main` is the stable release branch.
- `dev` is the active integration branch.
- normal feature, fix, docs, and chore branches are created from `dev`.
- hotfix branches are created from `main` or the active `release/*` branch.

Keep `main` and `dev` as the only permanent worktrees. Treat all other worktrees
as disposable and remove them once the branch is merged, replaced, or paused.

Release guidance:

- small releases may promote directly from `dev` to `main`
- larger releases should use a short-lived `release/*` branch
- any fix that lands during release stabilisation must be merged back to `dev`
  immediately after release

See the detailed guides for the full policy:

- `docs/guides/branching-strategy.md`
- `docs/guides/worktree-policy.md`
- `docs/guides/release-runbook.md`

### Before Submitting

- [ ] All tests pass (`pnpm test`)
- [ ] No lint errors (`pnpm lint`)
- [ ] TypeScript compiles (`pnpm typecheck`)
- [ ] Changes are documented if needed
- [ ] Commit messages are clear and descriptive

## Project Structure

```
anvil/
├── apps/
│   ├── anvil-cli/        # CLI application (Commander.js + Ink TUI)
│   ├── docs-site/        # Docusaurus documentation site
│   └── e2e/              # E2E test suites
├── packages/
│   ├── adapters/         # Format converters (SpecKit, BMAD)
│   ├── anvil/            # Core packages (contracts, core, runtime, policy)
│   ├── aps/              # APS document parser and tooling
│   ├── edda-stack/       # Memory system contracts
│   └── vscode-extension/ # VS Code integration
├── docs/                 # Internal engineering documentation
└── plans/                # Project planning (.aps.md specs)
```

## Additional Resources

| Document                            | Purpose                  |
| ----------------------------------- | ------------------------ |
| `AGENTS.md`                         | AI agent instructions    |
| `CLAUDE.md`                         | Claude Code instructions |
| `docs/architecture/overview.md`     | System design            |
| `docs/guides/branching-strategy.md` | Branching model          |
| `docs/guides/worktree-policy.md`    | Worktree hygiene         |
| `docs/guides/testing.md`            | Testing best practices   |
| `docs/adr/`                         | Architecture decisions   |

## Questions?

If you have questions or need help, please open an issue on GitHub.
