# Codebase Maintenance & Pattern Extraction

| ID | Owner | Status | Progress |
|----|-------|--------|----------|
| MAINT | @team | In Progress | 4/8 |

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
- **Status:** In Progress
- **Notes:** Shared `json()` helper added to `output.ts` (PR #517, open).
  Migration of existing `data(JSON.stringify(...))` call sites is incomplete —
  ~21 files in `apps/anvil-cli/src/commands/` still use the old pattern.

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
- **Status:** In Progress
- **Notes:** Generator scaffolded in PR #516 (open, not yet merged).
  `tools/generators/src/generators/command/` does not exist on main yet.

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
- **Status:** In Progress
- **Notes:** Generator scaffolded in PR #516 (open, not yet merged).
  `tools/generators/src/generators/gate-check/` does not exist on main yet.

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
- **Status:** In Progress
- **Notes:** Shared spinner utility planned in PR #517 (open, not yet merged).
  ~27 command files still import and call `ora()` directly; no
  `createSpinner()` usages exist in `apps/anvil-cli/src/commands/` yet.
