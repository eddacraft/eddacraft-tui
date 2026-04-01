# Codebase Maintenance & Pattern Extraction

| ID | Owner | Status | Progress |
|----|-------|--------|----------|
| MAINT | @team | In Progress | 10/11 |

## Purpose

Ongoing maintenance module for identifying repeated patterns across the
codebase and extracting them into shared utilities, libraries, or generators.
Rather than letting ad-hoc solutions accumulate, this module tracks the
systematic discovery and consolidation of common patterns into reusable
abstractions.

This was created after CRB-020 revealed that every CLI command had its own
inline `parseInt` + guard pattern — a textbook case for extraction. The same
pattern-discovery approach should be applied across the entire monorepo.

## In Scope

- Identifying duplicated logic across commands, services, and packages
- Extracting shared utilities (option coercion, error formatting, output helpers)
- Creating Nx generators for repeated scaffolding patterns
- Consolidating divergent implementations of the same concept
- Standardising naming conventions where drift has occurred

## Out of Scope

- Feature work (belongs in feature modules)
- Bug fixes (belongs in the originating module or CRB)
- Architecture redesign (belongs in dedicated architecture modules)
- Performance optimisation (separate concern)

## Discovery Process

When working on any task across the codebase, note repeated patterns:

1. **Grep for the pattern** — how many sites exist?
2. **Assess consistency** — are the implementations identical or divergent?
3. **Evaluate extraction** — would a shared utility reduce duplication and
   improve consistency?
4. **File a MAINT task** — if extraction is worthwhile, add it here

## Tasks

### MAINT-001: CLI option coercion utility

- **Intent:** Extract inline `parseInt` + validation patterns from CLI commands
  into a shared coercion utility
- **Expected Outcome:** A `option-coerce.ts` utility in `apps/anvil-cli/src/utils/`
  replaces all inline `parseInt` guards in command files; zero `parseInt` calls
  remain in `apps/anvil-cli/src/commands/`
- **Validation:** `grep -rn "parseInt" apps/anvil-cli/src/commands/` returns 0
  matches
- **Files:** `apps/anvil-cli/src/utils/option-coerce.ts`,
  `apps/anvil-cli/src/commands/*.ts`
- **Confidence:** high
- **Priority:** High
- **Status:** Complete
- **Completed:** 2026-03-07
- **Notes:** Created `option-coerce.ts` with `coercePositiveInt`,
  `coerceNonNegativeInt`, `coercePort`. Replaced all 9 sites. 14 test cases
  in colocated test file. Zero parseInt calls remain in commands/.

---

### MAINT-002: Error formatting consistency

- **Intent:** Audit and consolidate error output patterns — some commands use
  `console.error()` + `CliError`, some use `error()` (chalk), some throw
  `CliError` directly
- **Expected Outcome:** A single error reporting convention is documented and
  followed; commands use `CliError` for flow control and a consistent output
  helper for user-facing messages
- **Validation:** `grep -rn "console.error" apps/anvil-cli/src/commands/` shows
  no direct console.error calls outside of a sanctioned helper
- **Files:** `apps/anvil-cli/src/commands/*.ts`, `apps/anvil-cli/src/utils/`
- **Confidence:** medium
- **Priority:** Medium
- **Status:** Complete
- **Completed:** 2026-03-08
- **Notes:** Resolved via CRB-019 console.*migration (commit 5a3882b2, PR #506).
  All CLI command implementation files now use shared output module (`output.ts`)
  with `print()`, `blank()`, `data()`, `json()` helpers. No direct console.error
  calls remain in command implementation files (test files may still reference
  console.* for spying/mocking purposes, which is expected).

---

### MAINT-003: Workspace root resolution patterns

- **Intent:** Identify and consolidate workspace root detection into a single
  utility — currently `getWorkspaceRoot()` is called inline with varying error
  handling
- **Expected Outcome:** One canonical workspace root utility with consistent
  error messaging; all commands delegate to it
- **Validation:** `grep -rn "getWorkspaceRoot" apps/anvil-cli/src/commands/ | wc -l`
  shows all usages go through the same path
- **Files:** `apps/anvil-cli/src/commands/*.ts`, `apps/anvil-cli/src/services/`
- **Confidence:** medium
- **Priority:** Low
- **Status:** Complete
- **Completed:** 2026-03-08
- **Notes:** Already consolidated. `getWorkspaceRoot()` in
  `apps/anvil-cli/src/utils/file-io.ts` (line 98) is the single canonical
  utility used by ~30+ command files. No divergent implementations exist.

---

### MAINT-004: Git operation wrappers

- **Intent:** Audit `execFile`/`spawn` calls to `git` across the monorepo and
  consolidate into typed, tested wrappers with consistent timeout and error
  handling
- **Expected Outcome:** A shared git operations module provides typed wrappers;
  direct `execFile('git', ...)` calls are replaced with wrapper calls
- **Validation:** Direct `execFile.*git` / `spawn.*git` calls outside the
  wrapper module are eliminated
- **Files:** `packages/anvil/runtime/src/concurrency/git-agent.ts` (extend),
  `apps/anvil-cli/src/services/release-changelog.ts`,
  `apps/anvil-cli/src/services/release-git.ts`,
  `apps/anvil-cli/src/commands/plan.ts`,
  `apps/anvil-cli/src/commands/init.ts`,
  `apps/anvil-cli/src/tui/components/SystemCheck.ts`,
  `packages/anvil/runtime/src/gate/checks/policy.check.ts`,
  `packages/aps/src/state/index.ts`
- **Confidence:** high
- **Priority:** Medium
- **Status:** Complete
- **Completed:** 2026-03-08
- **Notes:** Created `git-operations.ts` in `@eddacraft/anvil-core` with
  `gitExec` (async) and `gitExecSync` (sync) wrappers, typed `GitExecOptions`,
  `GitOperationError`, and convenience helpers. Migrated 14 production files
  across core, runtime, and CLI packages. Key design: `trimEnd()` preserves
  leading whitespace in git porcelain output. Packages `aps` and `edda-stack`
  excluded (independent dependency trees). 10 co-located tests.

---

### MAINT-005: JSON output formatting

- **Intent:** Audit `--json` output across CLI commands and standardise the
  envelope format
- **Expected Outcome:** All commands that support `--json` use a consistent
  response envelope (e.g., `{ ok, data, error }` or similar); a shared utility
  handles JSON serialisation and output
- **Validation:** `grep -rn "JSON.stringify" apps/anvil-cli/src/commands/` shows
  all usages go through a shared formatter
- **Files:** `apps/anvil-cli/src/commands/*.ts`, `apps/anvil-cli/src/utils/`
- **Confidence:** medium
- **Priority:** Low
- **Status:** Complete
- **Completed:** 2026-03-10
- **Notes:** Shared `json()` helper added to `output.ts` (PR #517, merged).
  All CLI command files migrated to use `json()` helper for JSON output.
  Zero `data(JSON.stringify(...))` call sites remain in commands/.

---

### MAINT-006: Nx generator for CLI commands

- **Intent:** Create an Nx generator that scaffolds a new CLI command with
  co-located test file, following all conventions (factory pattern, Commander.js,
  UK English, mocked test)
- **Expected Outcome:** `nx g command --name=foo` produces `commands/foo.ts` and
  `commands/foo.test.ts` with correct boilerplate
- **Validation:** Generated files pass typecheck and lint
- **Files:** `tools/generators/`
- **Confidence:** high
- **Priority:** Low
- **Status:** Complete
- **Completed:** 2026-03-08
- **Notes:** Generator merged via PR #516.
  `tools/generators/src/generators/command/` exists on main.

---

### MAINT-007: Nx generator for gate checks

- **Intent:** Create an Nx generator for new gate checks (BaseCheck subclass +
  co-located test + registration in gate-runner)
- **Expected Outcome:** `nx g gate-check --name=foo` produces check file, test
  file, and updates gate-runner registration
- **Validation:** Generated files pass typecheck and lint; check appears in gate
  runner
- **Files:** `tools/generators/`, `packages/anvil/runtime/src/gate/`
- **Confidence:** high
- **Priority:** Low
- **Status:** Complete
- **Completed:** 2026-03-08
- **Notes:** Generator merged via PR #516.
  `tools/generators/src/generators/gate-check/` exists on main.

---

### MAINT-008: Spinner/progress patterns

- **Intent:** Audit ora spinner usage across CLI commands and consolidate into
  consistent patterns — some commands create spinners inline, some use the
  `spinner.ts` utility, some mix both
- **Expected Outcome:** All commands use the shared spinner utility; inline
  `ora()` calls are eliminated from command files
- **Validation:** `grep -rn "ora(" apps/anvil-cli/src/commands/` shows no direct
  ora imports outside the utility
- **Files:** `apps/anvil-cli/src/commands/*.ts`, `apps/anvil-cli/src/utils/spinner.ts`
- **Confidence:** medium
- **Priority:** Low
- **Status:** Complete
- **Completed:** 2026-03-10
- **Notes:** Shared spinner utility added via PR #517 (merged). All CLI
  command files migrated to use `createSpinner()` from `utils/spinner.ts`.
  No direct `ora()` imports remain in command files.


---

### MAINT-009: Edda list filters parity with release claims

- **Intent:** Align `anvil edda list` filtering capabilities and help text with
  release-note claims (type, confidence, age), or update release/docs to match
  implemented behaviour
- **Expected Outcome:** One of:
  1) CLI adds explicit `--confidence` and age-style filtering flags with tests,
  2) release/docs are corrected to currently supported filters (`--type`,
     `--status`, `--limit`)
- **Validation:**
  - `anvil edda list --help` shows accurate supported filters
  - release notes/docs match command behaviour exactly
  - tests cover filter parsing and query behavior
- **Files:**
  - `apps/anvil-cli/src/commands/edda/list.ts`
  - `apps/anvil-cli/src/commands/__tests__/` (new/updated tests)
  - `apps/anvil-cli/README*` and release docs as needed
- **Confidence:** high
- **Priority:** Medium
- **Status:** Complete
- **Completed:** 2026-03-21
- **Notes:** Implemented Option A — added `--confidence` and `--since` flags
  to `anvil edda list`. Query API already supported both filters; wired up
  CLI flags with validation and tests.
- **Origin:** v0.2.1-beta release validation (2026-03-15)

---

### MAINT-010: Authenticated release smoke harness for Edda/Ember/Stack

- **Intent:** Add a repeatable authenticated smoke test path for release
  validation so command availability and runtime behavior can be verified in one
  pass before publish
- **Expected Outcome:** A documented smoke harness/checklist that validates:
  `edda list/show/promote/retire/trace`, `ember list/show/promote`,
  `stack status/validate`, and tutorial baseline in an authenticated session
- **Validation:**
  - `docs/testing/releases/` contains executable checklist
  - CI-safe or manual script exists for authenticated local validation
  - release checklist references this harness
- **Files:**
  - `docs/testing/releases/` (new)
  - release checklist docs under `docs/` and/or `.github/` as appropriate
- **Confidence:** medium
- **Priority:** Medium
- **Status:** Deferred — rebuild against new CLI once stable
- **Origin:** v0.2.1-beta release validation (2026-03-15)
- **Deferred:** PR #628 closed; harness tested a CLI about to be replaced.
  Rebuild against the new CLI when it lands.

---

### MAINT-011: Migrate to TypeScript 6.0

- **Intent:** Upgrade from TypeScript 5.9 to 6.0 — the last JS-based compiler
  release before the Go rewrite (TS 7.0). Remove deprecated options now so TS 7.0
  adoption is frictionless
- **Expected Outcome:** All packages compile, build, test, and lint cleanly on
  TypeScript 6.0. Deprecated `baseUrl` removed from all tsconfigs. Redundant
  `esModuleInterop: true` removed (now always-on). Target/lib bumped to es2024
- **Validation:**
  - `nx run-many -t typecheck` passes with zero errors
  - `nx run-many -t build` succeeds across all packages
  - `nx run-many -t test` passes with no regressions
  - `grep -r '"baseUrl"' --include='tsconfig*.json'` returns 0 matches
  - `grep -r '"esModuleInterop"' --include='tsconfig*.json'` returns 0 matches
- **Files:**
  - `package.json` (version bump)
  - `tsconfig.base.json` (remove baseUrl, bump target/lib)
  - `packages/adapters/tsconfig.lib.json` (remove baseUrl)
  - `packages/aps/tsconfig.lib.json` (remove baseUrl)
  - `packages/mcp-server/tsconfig.lib.json` (remove baseUrl)
  - `apps/anvil-cli/tsconfig.json` (remove baseUrl, esModuleInterop)
  - `apps/docs-site/tsconfig.json` (remove baseUrl)
  - `apps/website/tsconfig.json` (remove esModuleInterop)
  - `apps/anvil-api/tsconfig.json` (remove esModuleInterop)
  - `packages/eslint-plugin-anvil/tsconfig.json` (remove esModuleInterop)
  - `packages/vscode-extension/tsconfig.json` (remove esModuleInterop)
- **Confidence:** high
- **Priority:** High
- **Status:** Complete
- **Completed:** 2026-03-29 (PR #679)
- **Origin:** TypeScript 6.0 release (2026-03-17), TS 7.0 Go rewrite on horizon
- **Notes:** TypeScript ~6.0.2 configured with nodenext module resolution.
  `baseUrl` and `esModuleInterop` removed from all tsconfigs
