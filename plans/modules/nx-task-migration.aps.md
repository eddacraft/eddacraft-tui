<!-- APS Module: nx-task-migration -->
<!-- Status: Complete -->

# Nx Task Migration

Migrate root-level lint, typecheck, and test scripts from monolithic processes to
Nx-orchestrated per-project targets for parallelism, caching, and affected-only
CI runs.

## Purpose

The monorepo has 26 Nx projects with inferred per-project `lint`, `typecheck`,
and `test` targets (via `@nx/eslint/plugin`, `@nx/js/typescript`, and
`@nx/vite/plugin`), but the root `package.json` scripts bypass Nx entirely:

- `lint` / `lint:check` run a single ESLint process over the entire workspace
- `typecheck` runs a single `tsc --noEmit` from the root
- `test` runs a single `vitest` process with a root config that manually lists
  per-project globs

The `build` script already uses `nx run-many -t build` correctly. The others
should follow the same pattern.

**Why this matters:**

1. **Parallelism** -- Nx runs targets across projects in parallel, respecting
   the dependency graph. A single ESLint/tsc/vitest process is inherently
   serial.
2. **Caching** -- Nx caches per-project results (local + Azure remote). Changing
   one package only re-runs affected targets. Today every script runs
   everything.
3. **Affected-only CI** -- `nx affected -t lint test typecheck` skips unchanged
   projects entirely. The current CI runs everything when any code changes.
4. **Consistency** -- `build` uses Nx but `lint`/`test`/`typecheck` do not.
   This inconsistency confuses contributors and undermines the monorepo setup.

## In Scope

- Fix `nx sync` if TypeScript project references are stale
- Migrate `lint`, `lint:check`, `typecheck`, and `test` root scripts to
  `nx run-many`
- Handle the `eslint-plugin-anvil` build dependency (currently `prelint` script)
- Handle markdownlint (root-level, no per-project targets)
- Update CI workflow (`.github/workflows/ci.yml`) to use `nx affected`
- Verify lint-staged still works after migration
- Verify per-project vitest configs are complete (all 20 test-bearing projects
  have `vitest.config.ts`)

## Out of Scope

- Migrating `format` / `format:check` (prettier) -- these are intentionally
  root-level, no dependency graph benefit
- Migrating E2E test scripts -- these have separate CI jobs and different
  concerns
- Adding Nx Cloud or changing the remote cache provider (Azure is already
  configured)
- Refactoring eslint config structure (the shared root `eslint.config.mjs` works
  with per-project inference)
- Removing the root `vitest.config.ts` entirely -- it may be kept as fallback
  for ad-hoc `pnpm vitest` runs during development

## Interfaces

### Depends On

- Nx plugins already configured in `nx.json` (`@nx/js/typescript`,
  `@nx/eslint/plugin`, `@nx/vite/plugin`)
- Per-project `vitest.config.ts` files (18 projects already have them)
- Root `eslint.config.mjs` (used by all projects via Nx inference)

### Exposes

- `pnpm lint` -- runs `nx run-many -t lint` + markdownlint
- `pnpm lint:check` -- runs `nx run-many -t lint` (check mode) + markdownlint
- `pnpm typecheck` -- runs `nx run-many -t typecheck`
- `pnpm test` -- runs `nx run-many -t test`
- CI jobs use `nx affected` for PR builds

## Constraints

- Zero regressions: all currently-passing lint, typecheck, and test results must
  continue to pass
- The `eslint-plugin-anvil` must be built before any ESLint target runs (it
  provides custom rules used across the workspace)
- markdownlint has no per-project inference -- it must either remain a root
  command or be added as an Nx target via `nx:run-commands`
- lint-staged (pre-commit hook) runs `eslint --fix` and `markdownlint --fix`
  directly on staged files -- this should NOT go through Nx (file-level
  operations, not project-level)
- The root `vitest.config.ts` contains include patterns and resolve aliases that
  serve as the "monolithic" configuration; per-project configs may need the same
  aliases if they don't already have them
- CI must continue to work on PRs, pushes to main/develop, and docs-only changes

## Ready Checklist

- [x] Nx plugins confirmed: `@nx/eslint/plugin`, `@nx/js/typescript`,
  `@nx/vite/plugin` all present in `nx.json`
- [x] Per-project targets verified: 24 projects have `lint`, 23 have
  `typecheck`, 20 have `test`
- [x] Per-project `vitest.config.ts` files exist for all test-bearing projects
- [x] CI workflow structure understood (7 jobs, change detection, matrix builds)
- [x] lint-staged config reviewed (file-level, should remain as-is)
- [x] `nx sync` status checked (currently reports up to date)

---

## Work Items

### NXTASK-001: Ensure nx sync is clean and TypeScript references are current

- **Status:** Complete
- **Intent:** Guarantee that `nx sync` reports no drift so per-project
  `typecheck` targets have correct project references.
- **Expected Outcome:** `npx nx sync --dry-run` exits 0 and reports "already up
  to date". Any generated `tsconfig.json` reference changes are committed.
- **Validation:** `npx nx sync --dry-run 2>&1 | grep -q "already up to date"`
- **Confidence:** high
- **Files:** `tsconfig.json`, `tsconfig.base.json`, per-project `tsconfig.json`
  and `tsconfig.lib.json` files
- **Non-scope:** Changing compiler options or path aliases
- **Risks:** If references were stale, fixing them may surface new type errors
  that were previously hidden by the monolithic `tsc --noEmit` approach

### NXTASK-002: Wire eslint-plugin-anvil as an Nx build dependency for lint targets

- **Status:** Complete
- **Dependencies:** NXTASK-001
- **Intent:** Ensure the custom ESLint plugin is built before any project's lint
  target runs, replacing the `prelint` npm script hack.
- **Expected Outcome:** Running `nx run-many -t lint` automatically builds
  `eslint-plugin-anvil` first, without a separate `prelint` script.
- **Validation:** `npx nx run-many -t lint --dry-run 2>&1 | grep -q eslint-plugin-anvil`
- **Confidence:** high
- **Files:** `nx.json` (targetDefaults for lint), possibly
  `packages/eslint-plugin-anvil/project.json`
- **Non-scope:** Changing the eslint-plugin-anvil source code or rules
- **Risks:** If the dependency is misconfigured, lint targets will fail with
  "cannot find module eslint-plugin-anvil" errors. Easy to detect and fix.

### NXTASK-003: Migrate root lint scripts to nx run-many

- **Status:** Complete
- **Dependencies:** NXTASK-002
- **Intent:** Replace monolithic `eslint . --fix` with `nx run-many -t lint` so
  linting runs per-project with caching and parallelism.
- **Expected Outcome:**
  - `pnpm lint` runs `nx run-many -t lint` (with fix mode) followed by
    markdownlint with `--fix`
  - `pnpm lint:check` runs `nx run-many -t lint` (check mode) followed by
    markdownlint without `--fix`
  - `prelint` / `prelint:check` scripts are removed (superseded by NXTASK-002)
  - markdownlint runs as a separate step (root-level) since it has no
    per-project targets
- **Validation:** `pnpm lint:check` exits 0 and `npx nx run-many -t lint -- --dry-run` shows all 24 projects
- **Confidence:** high
- **Files:** `package.json` (scripts section)
- **Non-scope:** Changing ESLint rules, markdownlint config, or per-project
  eslint configs
- **Risks:** Nx-inferred lint targets may use different ESLint CLI flags than the
  root script. Need to verify that `--fix` passthrough works with Nx. The
  `@nx/eslint/plugin` already supports this.

### NXTASK-004: Migrate root typecheck script to nx run-many

- **Status:** Complete
- **Dependencies:** NXTASK-001
- **Intent:** Replace monolithic `tsc --noEmit` with `nx run-many -t typecheck`
  for per-project type checking with caching.
- **Expected Outcome:** `pnpm typecheck` runs `nx run-many -t typecheck` and
  all 23 projects type-check successfully.
- **Validation:** `pnpm typecheck` exits 0
- **Confidence:** high
- **Files:** `package.json` (scripts section)
- **Non-scope:** Changing tsconfig compiler options or path aliases
- **Risks:** Per-project typecheck may surface errors that the monolithic
  `tsc --noEmit` masked (e.g., files not included in any project's tsconfig).
  The root `tsconfig.json` includes `vitest.config.ts`, `playwright.config.ts`,
  `eslint.config.mjs`, and `e2e/**/*` -- these root-level files need a typecheck
  home.

### NXTASK-005: Migrate root test script to nx run-many

- **Status:** Complete
- **Dependencies:** NXTASK-001
- **Intent:** Replace the monolithic `vitest` invocation with
  `nx run-many -t test` so tests run per-project with caching.
- **Expected Outcome:**
  - `pnpm test` runs `nx run-many -t test`
  - `pnpm test:coverage` runs `nx run-many -t test -- --coverage`
  - All 20 test-bearing projects pass
  - The root `vitest.config.ts` is retained but documented as a development
    convenience (for running `pnpm vitest path/to/file` directly)
- **Validation:** `pnpm test -- --run` exits 0
- **Confidence:** medium
- **Files:** `package.json` (scripts section)
- **Non-scope:** Changing per-project vitest configs, test code, or coverage
  thresholds
- **Risks:**
  - The root vitest.config.ts has resolve aliases that per-project configs may
    not all replicate. Some per-project test configs (e.g., anvil-cli) have their
    own aliases but others may rely on the root.
  - The root config has selective MCP server test includes (only passing suites).
    The per-project `packages/mcp-server/vitest.config.ts` may include all tests,
    which could surface known failures.
  - Coverage report aggregation will change -- each project produces its own
    report instead of one merged report. May need `nx run-many -t test --
    --coverage` or a separate aggregation step.

### NXTASK-006: Update CI to use nx affected

- **Status:** Complete
- **Dependencies:** NXTASK-003, NXTASK-004, NXTASK-005
- **Intent:** Maximise CI efficiency by running only affected project targets on
  PRs instead of running everything.
- **Expected Outcome:**
  - PR CI jobs use `nx affected -t lint typecheck test` (with appropriate base
    ref) instead of separate monolithic commands
  - Push-to-main CI uses `nx run-many -t lint typecheck test` (run everything)
  - The `detect-changes` job is simplified or removed -- Nx's own affected
    detection replaces manual file-pattern matching for code jobs
  - docs-only job stays as-is (markdownlint + prettier)
  - Build job continues to use `nx run-many -t build` (already correct)
  - Nx SHAs are set using `nrwl/nx-set-shas` action for accurate affected
    calculation
- **Validation:** CI workflow parses as valid YAML and a test PR triggers only
  affected targets
- **Confidence:** medium
- **Files:** `.github/workflows/ci.yml`
- **Non-scope:** Changing E2E test jobs, TUI test jobs, or the build job
  structure. Changing Azure cache configuration.
- **Risks:**
  - `nx affected` requires correct base/head SHA configuration. The
    `nrwl/nx-set-shas` action handles this but needs to be added.
  - The current CI has a matrix of Node 20.x + 22.x for tests and builds.
    `nx affected` needs to work within this matrix.
  - Cross-platform smoke tests (macOS, Windows) currently run the same monolithic
    test. May need to keep them as-is or also migrate to `nx affected`.
  - Coverage artifact upload currently expects a single `coverage/` directory.
    Per-project coverage changes the directory structure.
