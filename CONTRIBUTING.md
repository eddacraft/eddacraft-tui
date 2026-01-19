# Contributing to Anvil

Thank you for your interest in contributing to Anvil! This guide will help you
get started.

## Getting Started

### Prerequisites

- **Node.js**: >=20.0.0
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

### Essential Commands

```bash
pnpm build          # Build all packages
pnpm test           # Run unit tests
pnpm lint           # Lint and auto-fix
pnpm typecheck      # TypeScript validation
pnpm test:e2e       # End-to-end tests
```

### Package-Specific Development

```bash
npx nx build core          # Build specific package
npx nx test adapters       # Test specific package
pnpm link:cli              # Link CLI for local development
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

- **Prettier**: Single quotes, trailing commas (es5), 100 char width
- **Line endings**: LF (Unix-style)

## Testing

See **[docs/TESTING.md](docs/TESTING.md)** for comprehensive testing guidelines,
including:

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

1. **Create a branch** from `main`
2. **Make your changes** following the code standards above
3. **Run the full test suite**: `pnpm build && pnpm test && pnpm lint`
4. **Submit a PR** with a clear description of changes

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

| Document               | Purpose                  |
| ---------------------- | ------------------------ |
| `AGENTS.md`            | AI agent instructions    |
| `CLAUDE.md`            | Claude Code instructions |
| `docs/ARCHITECTURE.md` | System design            |
| `docs/TESTING.md`      | Testing best practices   |
| `docs/adr/`            | Architecture decisions   |

## Questions?

If you have questions or need help, please open an issue on GitHub.
