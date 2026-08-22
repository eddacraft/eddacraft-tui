# Contributing to anvil

| Type              | Authority | Owner | Status | Freshness                                                                                                                                                                            |
| ----------------- | --------- | ----- | ------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Contributor guide | Advisory  | DOCRB | Live   | Last reviewed 2026-08-21 for DOCRB-009 against `AGENTS.md`, `docs/guides/documentation-governance.md`, `scripts/docs/check-diagram-impact.mjs`, and `scripts/ci/classify-changes.sh` |

| Upstream                                                                                                                                                                                                | Downstream                  |
| ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------- |
| `AGENTS.md`, `docs/guides/documentation-governance.md`, `docs/guides/worktree-policy.md`, `docs/policies/release-cadence.md`, `scripts/docs/check-diagram-impact.mjs`, `scripts/ci/classify-changes.sh` | Contributors and PR authors |

Thank you for your interest in contributing to anvil! This guide will help you
get started.

## Getting Started

### Prerequisites

- **Node.js**: >=24.0.0
- **pnpm**: >=11.0.0
- **git**: >=2.54.0

These mirror `engines` in `package.json`, and
`scripts/ci/contributing-engines-parity.test.sh` fails if the two drift apart.
Older Node is not merely unsupported: pnpm 11 cannot run on it at all.

**Optional — direnv.** The committed `.envrc` relocates Rust build output to
`$HOME/.cache/anvil-targets/<worktree>` so it does not fill the mount holding
your checkout. Install direnv and run `direnv allow` once per worktree, or use
`wt`, whose post-start does the same. Without either, cargo builds into an
in-tree `target/` — reclaim one with `cargo clean`. See
[worktree policy](docs/guides/worktree-policy.md) for the full picture.

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
```

### Coverage

```bash
# Per-project coverage
pnpm exec nx test core --coverage

# Full monorepo coverage
pnpm test:coverage
```

### Running a candidate (`anvil-beta`) side-by-side with prod

Use `scripts/dev/run-candidate.sh` to dogfood a pre-release candidate without
uninstalling the production anvil install. The script builds the current HEAD
(or a specific git ref), stops the prod daemon so the candidate can bind the
socket, symlinks the candidate as `~/.local/bin/anvil-beta`, and pre-creates an
isolated scratch project under `/tmp/anvil-candidate-<sha>/`.

```bash
scripts/dev/run-candidate.sh             # build current HEAD + setup
scripts/dev/run-candidate.sh --ref <sha> # build a specific candidate
scripts/dev/run-candidate.sh --status    # show current install state
scripts/dev/run-candidate.sh --restore   # remove symlink, restart prod
```

Caveat: `~/.anvil/` user state is still shared between prod and candidate (no
`ANVIL_HOME` override exists yet — tracked as
[GH #1726](https://github.com/eddacraft/anvil-001/issues/1726)). Project state
is isolated by virtue of using a scratch directory. **Do not use this for Boring
Week** — the protocol explicitly requires real install paths so testers see what
a first-time user sees.

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

1. **Create a Worktrunk worktree and branch** from `main` for normal work with
   `wt switch --create <branch>`. Hotfixes also branch from `main`, or from the
   latest good tag only when `main` is unreleasable.
2. **Make your changes** following the code standards above.
3. **Run the full test suite**: `pnpm build && pnpm test && pnpm lint`.
4. **Submit a PR** with a clear description of changes.
5. **Offer or perform local cleanup** once the PR is opened, merged, abandoned,
   or paused with no near-term next action.

### Documentation and diagram impact

Before submitting, apply the mandatory
[change-impact review](docs/guides/documentation-governance.md#change-impact-review).
A triggered change updates the authoritative document or diagram, or records a
brief reason why that concern is unaffected. Run `pnpm validate:changed`; the
shared classifier selects the diff-scoped diagram check only for relevant
source, owner, governance, and enforcement paths.

## Branching and Worktrees

This repository uses a main-first model with Worktrunk-managed worktrees for
active streams.

- `main` is the only permanent product branch and PR target.
- `dev` is retired as of the OPMODEL-012 cutover.
- normal feature, fix, docs, and chore branches are created from `main` with
  `wt switch --create <branch>`.
- `release/*` branches are exceptional and short-lived when `main` cannot be
  tagged directly.
- hotfix branches are created from `main`, or from the latest good tag only when
  `main` is unreleasable.

Keep `main` as the only permanent worktree. Treat all other worktrees as
Worktrunk-managed task spaces and remove them with `wt remove` once the branch
is merged, replaced, abandoned, or paused without near-term action. Before
deleting a branch, confirm the worktree is clean and the branch is merged or
safely pushed.

Release guidance:

- normal releases tag an exact green `main` SHA
- use a short-lived `release/*` branch only for exceptional stabilisation
- merge release hardening back to `main`, tag from `main`, then delete the
  stabilisation branch
- release cadence, beta support windows, and hotfix expectations are documented
  in `docs/policies/release-cadence.md`

See the detailed guides for the full policy:

- `docs/guides/branching-strategy.md`
- `docs/guides/worktree-policy.md`
- `docs/runbooks/release-runbook.md`
- `docs/policies/release-cadence.md`

### Before Submitting

- [ ] All tests pass (`pnpm test`)
- [ ] No lint errors (`pnpm lint`)
- [ ] TypeScript compiles (`pnpm typecheck`)
- [ ] Changes are documented if needed
- [ ] Documentation and diagram impact has an update-or-unaffected disposition
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
| `plans/decisions/`                  | Architecture decisions   |

## Questions?

If you have questions or need help, please open an issue on GitHub.
