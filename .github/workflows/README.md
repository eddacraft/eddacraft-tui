# GitHub Workflows

## Overview

This directory contains GitHub Actions workflows for CI/CD automation.

## Workflows

### `bench.yml` - Benchmarks

Runs Rust stress-test scenarios (graph memory, watcher saturation, incremental
throughput) on push to main/dev and on PRs that touch `crates/anvil-bench/**` or
related kernel crates. Also available via `workflow_dispatch`.

### `ci.yml` - Continuous Integration

**Optimisations:**

1. **Path-based Change Detection**: Detects which files changed to skip
   unnecessary jobs
2. **Docs-only Fast Path**: When only documentation changes (`.md`, `docs/`,
   `plans/`), runs only markdown linting and format checking
3. **Conditional E2E Tests**: Only runs E2E tests when relevant files change
4. **Matrix Strategy**: Tests on Node.js 20.x and 22.x in parallel

**Job Flow:**

```
detect-changes (always runs)
    ├── docs-lint (if docs-only)
    └── lint-and-test (if code changed)
            └── e2e-tests (if E2E files changed)
```

**Time Savings:**

- **Docs-only commits**: ~10 minutes → ~2 minutes (80% reduction)
- **Code without E2E changes**: ~15 minutes → ~12 minutes (20% reduction)
- **Full changes**: ~15 minutes (unchanged, but runs when needed)

### Docs-only Patterns

The following changes trigger the fast docs-only path:

- `*.md`, `*.txt` files
- `docs/**` directory
- `plans/**` directory
- `README.md`, `AGENTS.md`, `CLAUDE.md`
- `LICENSE` file

### E2E Trigger Patterns

E2E tests run when:

- `e2e/**` directory changes
- `playwright.config.ts` changes
- Other code changes exist (prevents docs-only from running E2E)

## Local Testing

Test workflow syntax:

```bash
# Install act (GitHub Actions locally)
brew install act  # or appropriate package manager

# Run CI workflow
act pull_request
```

## Adding New Workflows

1. Create `.yml` file in this directory
2. Follow naming convention: `{purpose}.yml`
3. Add documentation section to this README
4. Test with `act` before committing

## Troubleshooting

**Change detection not working in PRs:**

The workflow fetches base and head commits. If PRs are very old, increase fetch
depth:

```yaml
- uses: actions/checkout@v4
  with:
    fetch-depth: 100 # Increase if needed
```

**False positives in docs-only detection:**

Update the patterns in the `detect-changes` job's filter step.

## References

- [GitHub Actions Documentation](https://docs.github.com/en/actions)
- [pnpm/action-setup](https://github.com/pnpm/action-setup)
- [actions/checkout](https://github.com/actions/checkout)
