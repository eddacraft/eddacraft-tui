# ADR 012: Dual-Layer Vitest Configuration Strategy

**Date:** 2026-03-02
**Status:** Accepted
**Context:** CRB-006

## Decision

The monorepo uses a **dual-layer** vitest configuration:

1. **Root config** (`vitest.config.ts`) — includes globs for all packages/apps,
   with shared aliases and coverage settings. Running `npx vitest run` from the
   repo root discovers and runs every test across the monorepo.

2. **Per-package configs** (`packages/*/vitest.config.ts`,
   `apps/*/vitest.config.ts`) — each package has its own config for targeted
   execution. Running `npx vitest run -c packages/foo/vitest.config.ts` or
   `pnpm -F @eddacraft/foo test` runs only that package's tests.

## Rationale

- **Root config** gives CI a single entry point (`npx vitest run`) and provides
  a global alias map so cross-package imports resolve correctly in tests.
- **Per-package configs** enable fast local development cycles where developers
  only run tests for the package they're editing. They also allow packages to
  override environment settings (e.g., `environment: 'node'` vs `'happy-dom'`).
- This is not a "mixed" approach — both layers serve distinct purposes and all
  20 packages/apps follow this pattern consistently.

## Conventions

- Every new package should have its own `vitest.config.ts` and be added to the
  root config's `include` globs.
- Per-package configs should duplicate the alias map for any cross-package
  imports used in that package's tests.
- The root config is the canonical include list — if a package isn't in the root
  config, its tests won't run in CI.
