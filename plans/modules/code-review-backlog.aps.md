<!--
APS Module: Code Review Backlog
================================
Architectural recommendations from the 2026-02-16 code review that do not
have immediate fixes but should be tracked for future work. These are
structural improvements, not security patches — the security surface was
addressed in cli-hardening.aps.md (2026-02-06 adversarial reviews, all
66 tasks complete).

Scopes: CRB (main), grouped by area: CLI, RT (runtime), INFRA
-->

# Code Review Backlog — Architectural Recommendations

| ID  | Owner | Status |
| --- | ----- | ------ |
| CRB | —     | Complete (29/29) |
<!-- Complete: CRB-001, CRB-002, CRB-003, CRB-004, CRB-005, CRB-006, CRB-007, CRB-008, CRB-009, CRB-010, CRB-011, CRB-012, CRB-013, CRB-014, CRB-015, CRB-016, CRB-017, CRB-018, CRB-019, CRB-020, CRB-021, CRB-022, CRB-023, CRB-024, CRB-025, CRB-026, CRB-027, CRB-028, CRB-029 -->

## Purpose

Track non-urgent architectural improvements identified during the 2026-02-16
code review. These items improve consistency, maintainability, and correctness
but do not represent security vulnerabilities or blocking defects. Each item
is independently actionable and can be promoted to Ready when prioritised.

## In Scope

- CLI output stream policy (stdout vs stderr)
- Hook script deduplication (further consolidation beyond CLIH-005)
- YAML schema validation in runtime parsers
- OPA binary manager safety and logging
- Dependency audit error surfacing
- Vitest configuration strategy
- Library-layer process.exit removal
- Output path containment (generalisation beyond CLIH-007)
- OPA checksum table correctness
- APS task locking atomicity
- APS loader parameter contract drift
- Config loader status accuracy
- MCP server test discoverability
- Shell command composition test coverage
- Symlink escape test coverage
- Windows separator test coverage for MCP path guards
- Platform/core config loader test coverage
- Monorepo developer workflow standardisation
- Logging and output conventions
- Option parsing/validation consistency
- Duplicated implementations and naming drift
- Large command module decomposition
- Silent fallback visibility
- Subprocess timeout enforcement
- Documentation/script drift from reality
- Spinner lifecycle safety in TUI-capable commands
- Input path containment (generalisation of output path containment)
- CLI command test coverage expansion

## Out of Scope

- Security hardening (covered by [cli-hardening](./cli-hardening.aps.md),
  all 66 tasks complete)
- New features or API changes
- Performance optimisation
- Dashboard or TUI work
- Multi-language support (post-1.0.0)

## Interfaces

**Depends on:**

- `@eddacraft/anvil-cli` — CLI commands and services being improved
- `@eddacraft/anvil-runtime` — Runtime YAML parsers and gate checks

**Exposes:**

- Cleaner separation of concerns across CLI and library layers
- Consistent output stream behaviour for CI/scripting consumers
- Validated YAML parsing with schema enforcement

## Prior Art

Some of these items partially overlap with completed hardening tasks:

- **CLIH-005** (Complete 2026-02-09) consolidated hook scripts from
  `commands/hooks.ts` into `services/hook-installer.ts`. CRB-002 tracks
  residual duplication or further consolidation if any remains.
- **CLIH-007** (Complete 2026-02-09) validated `--output` paths for `policy doc`
  and `policy scaffold`. CRB-008 generalises this to all output-path-accepting
  commands.

## Ready Checklist

Change status to **Ready** when:

- [ ] Team has reviewed and confirmed these are still relevant
- [ ] Priority ordering agreed (High items first)
- [ ] No overlap with in-flight work

---

## Tasks

### CLI (CRB-001, CRB-002, CRB-019 through CRB-021, CRB-023)

### CRB-001: Standardise stderr/stdout policy across CLI commands

- **Intent:** Establish a consistent output stream policy so machine-parseable
  output goes to stdout and human diagnostics go to stderr
- **Expected Outcome:** All CLI commands follow the convention: structured data
  (JSON, tables intended for piping) is written to stdout via `info()`; progress
  messages, warnings, and diagnostics are written to stderr; a brief policy
  document exists in the CLI contributing guide or CLAUDE.md
- **Validation:** `grep -rn "info(" apps/anvil-cli/src/commands/ | head -20`
  shows no diagnostic-only messages routed through stdout
- **Files:** `apps/anvil-cli/src/commands/*.ts`, `apps/anvil-cli/src/utils/output.ts`
- **Dependencies:** None (CLIH-013 standardised console.error to output.ts;
  this extends the policy to stdout/stderr split)
- **Confidence:** medium
- **Priority:** Medium
- **Status:** Complete
- **Risks:** Changing output streams may break scripts that parse stdout.
  Requires audit of downstream consumers before implementation.

---

### CRB-002: Consolidate hook scripts to single source of truth

- **Intent:** Eliminate any residual duplication between hook generation in
  commands and hook templates in services
- **Expected Outcome:** Hook script content has exactly one canonical source
  (likely `HookInstaller` in `services/hook-installer.ts`); `commands/hooks.ts`
  delegates to the service for all hook content generation; no inline hook
  script strings exist in command files
- **Validation:** `grep -c "#!/" apps/anvil-cli/src/commands/hooks.ts` returns 0
- **Files:** `apps/anvil-cli/src/commands/hooks.ts`,
  `apps/anvil-cli/src/services/hook-installer.ts`
- **Dependencies:** None (extends CLIH-005 which consolidated the initial
  duplication)
- **Confidence:** high
- **Priority:** Medium
- **Status:** Complete
- **Notes:** CLIH-005 (Complete 2026-02-09) addressed the initial consolidation.
  CRB-002 (Complete 2026-03-03) moved remaining standalone functions
  (installHook, uninstallHook, isAnvilManagedHook, injectMarker) from
  commands/hooks.ts into HookInstaller service. grep -c "#!/" returns 0.

---

### CRB-019: Consistent logging/output conventions

- **Intent:** Establish a unified logging approach so all CLI output follows
  consistent stream, format, and verbosity conventions
- **Expected Outcome:** A shared logger abstraction replaces ad-hoc console.*
  usage; stderr is used for diagnostics, stdout for structured data; JSON mode
  emits only valid JSON on stdout without interleaved human-readable text;
  a logging conventions guide documents the pattern
- **Validation:** `grep -rn "console\.\(log\|warn\|error\)" apps/anvil-cli/src/commands/`
  returns 0 matches (all routed through shared logger)
- **Files:** `apps/anvil-cli/src/commands/*.ts`, `apps/anvil-cli/src/utils/output.ts`
- **Dependencies:** CRB-001 (stderr/stdout policy is a prerequisite for logger
  design)
- **Confidence:** medium
- **Priority:** Medium
- **Status:** Complete
- **Notes:** Implemented shared output module (`output.ts`) with `print()`,
  `blank()`, `data()`, `json()` helpers. Migrated 54 source files + 10 test
  files. All console.* calls replaced with module functions. PR #506.

---

### CRB-020: Option parsing/validation inconsistency

- **Intent:** Standardise how CLI options are parsed and validated so numeric
  and enum options fail clearly on invalid input
- **Expected Outcome:** A consistent Commander coercion or validation utility is
  used for all numeric, enum, and path options; `parseInt` without guard is
  eliminated; invalid option values produce actionable error messages rather
  than NaN or silent fallback
- **Validation:** `grep -rn "parseInt(" apps/anvil-cli/src/commands/ | grep -v "coerce\|validate"`
  returns 0 matches
- **Files:** `apps/anvil-cli/src/commands/*.ts`,
  `apps/anvil-cli/src/utils/option-coerce.ts` (or similar shared utility)
- **Dependencies:** None
- **Confidence:** high
- **Priority:** Low
- **Status:** Complete
- **Completed:** 2026-03-07
- **Notes:** Created `option-coerce.ts` utility with `coercePositiveInt`,
  `coerceNonNegativeInt`, and `coercePort`. Replaced all 9 inline parseInt
  sites across authorship, mcp-config, policy, beta, watch, gate, edda/list,
  ember/list, and drift commands. Also tracked as MAINT-001 in
  [codebase-maintenance](./codebase-maintenance.aps.md).

---

### CRB-021: Duplicated implementations and naming drift

- **Intent:** Eliminate duplicated hook script/service logic and resolve
  analyser/analyzer naming inconsistency across the codebase
- **Expected Outcome:** Hook scripts and services have a single implementation
  path (no forked logic); the codebase consistently uses one spelling
  (British or American) for analyser/analyzer in identifiers, filenames, and
  comments; a brief naming conventions note is added to CLAUDE.md or
  CONTRIBUTING.md
- **Validation:** `grep -rn "analy[sz]er" apps/ packages/ --include="*.ts" | sort`
  shows consistent spelling throughout
- **Files:** `apps/anvil-cli/src/services/*.ts`, `apps/anvil-cli/src/commands/*.ts`,
  `packages/anvil/runtime/src/**/*.ts`
- **Dependencies:** CRB-002 (hook consolidation addresses part of the
  duplication)
- **Confidence:** medium
- **Priority:** Low
- **Status:** Complete
- **Completed:** 2026-03-07
- **Notes:** Renamed 4 classes to British spelling (HistoricalAnalyser,
  SampleAnalyser, DependencyAnalyser, ArchitectureAnalyser), renamed
  `sample-analyzer.ts` → `sample-analyser.ts`, updated 46 files across
  `packages/anvil/core`, `packages/anvil/runtime`, `apps/anvil-cli`, and
  `tools/codemods`. Hook duplication portion already resolved by CRB-002.

---

### CRB-023: Silent fallbacks without visibility

- **Intent:** Make fallback paths observable so developers and CI can detect
  when workspace root detection, skip env vars, or other heuristics silently
  deviate from expected behaviour
- **Expected Outcome:** All silent fallback paths emit a structured debug-level
  log message (visible via `--verbose` or `DEBUG=anvil:*`) explaining what was
  attempted, what failed, and what fallback was used; no fallback path is
  completely silent
- **Validation:** `grep -rn "ANVIL_SKIP\|findWorkspaceRoot\|fallback" apps/anvil-cli/src/ packages/ --include="*.ts"`
  — each match has a corresponding debug/warn log within 5 lines
- **Files:** `apps/anvil-cli/src/commands/*.ts`, `apps/anvil-cli/src/services/*.ts`,
  `packages/anvil/runtime/src/**/*.ts`
- **Dependencies:** CRB-019 (logging conventions should be established first)
- **Confidence:** medium
- **Priority:** Medium
- **Status:** Complete
- **Notes:** Added `debug()` function to output module with `ANVIL_DEBUG=1`
  env var support. Instrumented 42 silent catch blocks across 21 files with
  debug() calls. PR #508.

### CRB-003: Add Zod validation to core YAML parsers

- **Intent:** Validate parsed YAML structure at the boundary before it flows
  into the runtime, catching malformed configs early with clear error messages
- **Expected Outcome:** `yaml-parser.ts` and `templates/index.ts` in
  `packages/anvil/runtime/src/` validate YAML.parse() output against Zod
  schemas before returning typed objects; invalid YAML produces a descriptive
  error listing which fields failed validation rather than a downstream
  TypeError
- **Validation:** `pnpm -F anvil-runtime test -- --testNamePattern="yaml|template|parser"`
- **Files:** `packages/anvil/runtime/src/` (YAML parsing modules)
- **Dependencies:** None (Zod is already a project dependency)
- **Confidence:** medium
- **Priority:** Medium
- **Status:** Complete
- **Notes:** The spec referenced `yaml-parser.ts` and `templates/index.ts` in
  `packages/anvil/runtime/src/`, but no YAML parsing exists in the runtime
  package. All YAML parsing lives in `packages/anvil/core/src/architecture/`
  and already validates against Zod schemas: `ArchitectureDefinitionSchema`
  (in `definition-schema.ts`) for architecture YAML, and `TemplateFileSchema`
  (in `templates/index.ts`) for template YAML. CLI-level config YAML
  (`policy-config.ts`) also uses Zod via `AnvilConfigSchema`. The intent of
  this item — Zod validation at YAML parse boundaries — is already satisfied.

---

### CRB-004: OPA binary manager — safer PATH lookup and shared logger

- **Intent:** Reduce risk from OPA binary resolution and route warnings through
  a consistent logging interface
- **Expected Outcome:** All `console.warn` and `console.error` calls in
  `opa-binary-manager.ts` route through the existing `createDebugger('policy')`
  instance (already imported at line 19); download progress, checksum
  verification, and warning messages use the shared debug logger instead of
  writing directly to stderr; the binary validation from RT-001 is preserved
- **Validation:** `grep -rn "console\.\(warn\|error\)" packages/anvil/policy/src/opa-binary-manager.ts`
  returns 0 matches
- **Files:** `packages/anvil/policy/src/opa-binary-manager.ts`
- **Dependencies:** None (RT-001 already validates ANVIL_OPA_PATH)
- **Confidence:** high
- **Priority:** Low
- **Status:** Complete
- **Notes:** Original spec referenced `policy.check.ts` in the runtime package,
  but that file already uses `createDebugger` with zero `console.warn` calls.
  The actual issue was in `opa-binary-manager.ts` in `packages/anvil/policy/`,
  which already imported `createDebugger('policy')` but bypassed it with 10 raw
  `console.warn`/`console.error` calls in `downloadBinary()` and
  `verifyChecksum()`. All 10 calls replaced with `debug()` — download progress,
  checksum verification, and mismatch details now route through the shared
  logger. PATH lookup safety was already resolved (`execFileSync`, no shell
  expansion). Validation: `grep -rn "console\.\(warn\|error\)"` returns 0.

---

### CRB-005: Dependency audit — surface errors deterministically

- **Intent:** Prevent silent masking of audit tool failures as "no
  vulnerabilities found"
- **Expected Outcome:** When `npm audit` or `pnpm audit` exits non-zero for
  reasons other than found vulnerabilities (e.g., network error, registry
  unavailable, malformed lock file), the dependency check surfaces the error
  rather than reporting a clean audit; the check result includes a distinct
  "error" state separate from "pass" and "fail"
- **Validation:** `pnpm -F anvil-runtime test -- --testNamePattern="dependency"`
- **Files:** `packages/anvil/runtime/src/gate/checks/dependency.check.ts`
- **Dependencies:** None
- **Confidence:** medium
- **Priority:** Medium
- **Status:** Complete
- **Notes:** Parse failures in `runAudit()` now throw instead of returning `null`,
  so the caller's catch block surfaces them as `passed: false` with an error
  message rather than silently reporting "No vulnerabilities found." Two
  `console.error` calls replaced with `log()` (shared debugger). Two new tests
  verify parse failure and command error are surfaced as check failures.

---

### CRB-009: OPA checksum table contains placeholder hashes

- **Intent:** Replace synthetic-looking SHA-256 hashes in the OPA binary manager
  checksum table with real checksums from OPA release artifacts
- **Expected Outcome:** The checksum table at `opa-binary-manager.ts` lines 48-53
  contains verified SHA-256 hashes fetched from the official OPA GitHub release
  artifacts; the strict validation at line 314 and post-download enforcement at
  line 276 work correctly for all supported platforms; a script or documented
  procedure exists for updating checksums when bumping OPA versions
- **Validation:** `pnpm -F anvil-policy test -- --testNamePattern="opa|binary|checksum"`
  passes; manual comparison of at least one hash against the official OPA release
  page confirms correctness
- **Files:** `packages/anvil/policy/src/opa-binary-manager.ts`
- **Dependencies:** None
- **Confidence:** high
- **Priority:** High
- **Status:** Complete
- **Risks:** Policy check is likely unusable on platforms without a preinstalled
  OPA binary until this is fixed. Affects any user relying on automatic OPA
  download.

---

### CRB-014: Add tests for git command composition in watch and concurrency modules

- **Intent:** Add dedicated test coverage for shell command safety in git
  command composition paths that previously had injection vulnerabilities
- **Expected Outcome:** Test files exist for `watch/git-status.ts` and
  `concurrency/git-agent.ts` covering: argument escaping with special characters
  (spaces, quotes, semicolons, backticks), path arguments with traversal
  attempts, and command composition producing expected safe strings
- **Validation:** `pnpm -F anvil-runtime test -- --testNamePattern="git.*(command|shell|inject|escape)"`
  passes with at least 6 test cases
- **Files:** `packages/anvil/runtime/src/watch/git-status.test.ts` (new),
  `packages/anvil/runtime/src/concurrency/git-agent.test.ts` (new or extended)
- **Dependencies:** None (the security fixes are already in place; this adds
  regression coverage)
- **Confidence:** high
- **Priority:** Medium
- **Status:** Complete
- **Completed:** 2026-03-02
- **Notes:** 52 tests added across two files — 22 for git-status (argument
  composition with special paths, status code parsing, filtering) and 30 for
  git-agent (pure function tests for trailer parsing/extraction/formatting plus
  real temp repo tests for git-calling functions). Also fixed two source bugs
  uncovered by testing: `stdout.trim()` → `stdout.trimEnd()` in
  `getFileStatus` (was stripping status code leading space), and rewrote
  `parseCommitTrailers` bottom-up algorithm (was requiring trailing blank line
  after trailers).

---

### CRB-022: Large command modules need decomposition

- **Intent:** Break up oversized command files (e.g., `policy.ts`) into smaller,
  focused subcommand modules to improve navigation and reduce merge conflicts
- **Expected Outcome:** No single command file exceeds approximately 300 lines;
  large commands are split into a directory with an `index.ts` re-exporting
  subcommands; the public API (command names, options, behaviour) is unchanged
- **Validation:** `wc -l apps/anvil-cli/src/commands/*.ts | sort -rn | head -5`
  shows no file exceeding 300 lines
- **Files:** `apps/anvil-cli/src/commands/policy.ts` and other large command files
- **Dependencies:** None
- **Confidence:** high
- **Priority:** Low
- **Status:** Complete
- **Completed:** 2026-03-08
- **Notes:** `policy.ts` (1,400+ lines) decomposed into 14 focused subcommand
  files in `commands/policy/` directory. Public API unchanged. PR #514.

---

### CRB-024: Subprocess calls without timeouts in CI

- **Intent:** Ensure all subprocess calls (git, audit, OPA) have explicit
  timeouts to prevent indefinite hangs in CI environments
- **Expected Outcome:** All subprocess invocations include a timeout option; a
  shared utility wraps subprocess execution with configurable timeout (default
  suitable for CI); timeout produces a descriptive error including the command
  that timed out and the elapsed time
- **Validation:** All subprocess invocations in `apps/` and `packages/` either
  include a timeout parameter or use the shared timeout wrapper
- **Files:** `packages/anvil/runtime/src/**/*.ts`,
  `apps/anvil-cli/src/services/*.ts`
- **Dependencies:** None (some calls were fixed during the review; this is a
  systematic audit)
- **Confidence:** medium
- **Priority:** Medium
- **Status:** Complete
- **Completed:** 2026-03-07
- **Notes:** Added explicit timeout parameters to all 20 production files with
  subprocess calls: 30s for quick git metadata, 60s for push/fetch, 120s for
  package managers (npm/gh), 300s for build/test, 10s for which/where. Six files
  already had timeouts and were left untouched. No shared wrapper — direct
  timeout property on each call, matching Node.js child_process API.

---

### Infrastructure (CRB-006 through CRB-008, CRB-010 through CRB-013, CRB-015 through CRB-018, CRB-025)

### CRB-006: Monorepo-wide vitest config strategy

- **Intent:** Standardise the vitest configuration approach across all
  packages and apps to eliminate confusion about where and how tests run
- **Expected Outcome:** A documented decision on one of: (a) per-package vitest
  configs with a root config that delegates, or (b) root-only config with
  per-package includes. All packages/apps follow the chosen pattern; no mixed
  approach remains.
- **Validation:** Manual review — all vitest.config.ts files follow the
  chosen pattern
- **Files:** `vitest.config.ts` (root), `apps/anvil-cli/vitest.config.ts`,
  and any other per-package configs
- **Dependencies:** None
- **Confidence:** high
- **Priority:** Low
- **Status:** Complete
- **Completed:** 2026-03-04
- **Notes:** Dual-layer strategy implemented and documented (ADR-012, later
  removed during cleanup). Root vitest.config.ts provides monorepo-wide include
  globs and resolve aliases; per-package configs (anvil-cli, mcp-server) override
  environment and test-specific settings. The `@nx/vite/plugin` auto-infers test
  targets from per-package vitest.config.ts files. Pattern is consistent and
  working. Commit: 495059e3.

---

### CRB-007: Move process.exit from library code to CLI layer

- **Intent:** Ensure library and runtime code throws typed errors instead of
  calling process.exit(), so callers (CLI, MCP server, tests, VS Code
  extension) can handle failures appropriately
- **Expected Outcome:** No `process.exit()` calls exist in
  `packages/anvil/runtime/` or `packages/anvil/core/`; these modules throw
  typed error classes (e.g., `AnvilConfigError`, `AnvilRuntimeError`); only
  the CLI entry point (`apps/anvil-cli/src/index.ts`) and top-level command
  handlers call `process.exit()`
- **Validation:** `grep -rn "process.exit" packages/anvil/runtime/ packages/anvil/core/`
  returns 0 matches (excluding test files)
- **Files:** `packages/anvil/runtime/src/**/*.ts`,
  `apps/anvil-cli/src/services/**/*.ts`,
  `apps/anvil-cli/src/index.ts`
- **Dependencies:** None
- **Confidence:** medium
- **Priority:** High
- **Status:** Complete
- **Notes:** Verified 2026-02-27 — no process.exit() calls remain in
  packages/anvil/runtime/ or packages/anvil/core/ (excluding test files).
  Already resolved in prior work.

---

### CRB-008: Consistent workspace root containment for output paths

- **Intent:** Prevent any output-path-accepting command from writing outside
  the workspace root, generalising the protection added by CLIH-007
- **Expected Outcome:** All CLI commands that accept an output path option
  (not just `policy doc` and `policy scaffold`) validate the resolved path
  is within the workspace root; a shared utility (e.g.,
  `assertWithinWorkspace(resolvedPath)`) is used consistently; paths that
  escape via `../` or absolute paths outside the workspace produce a clear
  error
- **Validation:** `grep -rn "assertWithinWorkspace\|containsPath\|isWithinRoot" apps/anvil-cli/src/commands/`
  shows usage in all commands that write files
- **Files:** `apps/anvil-cli/src/commands/*.ts`,
  `apps/anvil-cli/src/utils/path-safety.ts` (or similar shared utility)
- **Dependencies:** None (CLIH-007 added per-command validation for two
  commands; UTIL-001 extracted path safety utilities to core)
- **Confidence:** high
- **Priority:** High
- **Status:** Complete
- **Notes:** CLIH-007 (Complete 2026-02-09) addressed `policy doc` and
  `policy scaffold`. Added `validatePathWithinRoot` to `plan.ts` (create
  subcommand) and `architecture.ts` (visualise subcommand) — the two
  remaining commands with unvalidated output paths. All output-accepting
  commands now use the shared utility from `@eddacraft/anvil-core`.

---

### CRB-010: APS task locking is not atomic despite first-lock-wins intent

- **Intent:** Make APS task locking truly atomic so concurrent lockers cannot
  both succeed, violating the first-lock-wins guarantee
- **Expected Outcome:** The lock path check-then-write at
  `packages/aps/src/state/index.ts` lines 600/631 uses an atomic file creation
  mechanism (e.g., `O_EXCL` flag or equivalent); the state read-modify-write at
  lines 227/229 is similarly protected; concurrent lock attempts from parallel
  agents correctly fail for all but the first locker
- **Validation:** A test spawns two concurrent lock attempts on the same task;
  exactly one succeeds and one fails
- **Files:** `packages/aps/src/state/index.ts`
- **Dependencies:** None
- **Confidence:** medium
- **Priority:** Medium
- **Status:** Complete
- **Notes:** Implemented O_EXCL-based atomic lock file creation. The lock
  path check-then-write now uses `wx` flag (O_CREAT | O_EXCL | O_WRONLY) which
  fails atomically with EEXIST if the file already exists, ensuring
  first-lock-wins at the kernel level. Backward-compat guard honours
  state.json for tasks locked before lockfiles existed.
- **Risks:** Race window is small but real in parallel agent execution.
  File-level locking behaviour varies across operating systems.

---

### CRB-011: APS loader maxDepth parameter documented but ignored

- **Intent:** Either implement the documented maxDepth parameter or remove it
  from the API to eliminate contract drift
- **Expected Outcome:** If maxDepth is useful: the parameter at
  `packages/aps/src/loader/index.ts` line 69 is respected during recursive
  loading at line 164, with tests verifying depth limits. If maxDepth is not
  useful: the parameter is removed from the function signature and documentation.
- **Validation:** If implemented: a test loads a nested plan structure and
  verifies loading stops at the specified depth. If removed: `grep -rn "maxDepth"
  packages/aps/src/` returns 0 matches.
- **Files:** `packages/aps/src/loader/index.ts`
- **Dependencies:** None
- **Confidence:** high
- **Priority:** Low
- **Status:** Complete
- **Notes:** Implemented depth tracking in `loadMultiModulePlan`. When a module
  path points to another index file (nested plan), the loader now recurses with
  `depth + 1` and throws `ParseError` when `maxDepth` is exceeded. Added nested
  index test fixtures (`examples/nested-index/`) and 4 tests covering recursive
  nested loading, task extraction from nested modules, depth limit enforcement,
  and successful loading within limits.

---

### CRB-012: Config loader marked placeholder while package reports Complete

- **Intent:** Resolve the contradiction between loader files declaring
  placeholder behaviour and README reporting the config package as Complete
- **Expected Outcome:** Either: (a) the config loaders at
  `packages/platform/config/src/loader.ts` and
  `packages/anvil/core/src/config/loader.ts` are implemented with file/env
  loading and the Complete status is accurate; or (b) the README and package
  status are corrected to reflect the actual placeholder state
- **Validation:** If implemented: `pnpm -F @eddacraft/platform-config test`
  passes with loader tests. If status corrected:
  `packages/platform/README.md` no longer claims config is Complete.
- **Files:** `packages/platform/config/src/loader.ts`,
  `packages/anvil/core/src/config/loader.ts`, `packages/platform/README.md`
- **Dependencies:** None
- **Confidence:** high
- **Priority:** Low
- **Status:** Complete
- **Completed:** 2026-03-02
- **Notes:** Corrected `packages/platform/README.md` status from "Complete" to
  "Placeholder". Integrators may assume file/env config loading exists when it
  does not; the status mismatch is now resolved.

---

### CRB-013: MCP server tests not in vitest include globs

- **Intent:** Make MCP server test files discoverable by vitest so they run as
  part of the standard test suite
- **Expected Outcome:** Running `npx vitest run packages/mcp-server/` discovers
  and executes all test files in the MCP server package; either the root
  `vitest.config.ts` include globs are updated to match `packages/mcp-server/`,
  or the package has its own `vitest.config.ts`
- **Validation:** `npx vitest run packages/mcp-server/ --reporter=verbose` lists
  all MCP server test files and passes
- **Files:** `vitest.config.ts` (root) or `packages/mcp-server/vitest.config.ts`
  (new)
- **Dependencies:** CRB-006 (vitest config strategy should inform the approach)
- **Confidence:** high
- **Priority:** Medium
- **Status:** Complete
- **Completed:** 2026-03-04
- **Notes:** Root vitest.config.ts include glob updated
  (`packages/mcp-server/src/**/*`), per-package vitest.config.ts broadened with
  resolve aliases. All 12 MCP test files now discovered and passing (199 tests).
  Commits: edc53765, 3f89c9b0.

---

### CRB-015: Add symlink escape tests to file-storage.test.ts

- **Intent:** Add test coverage for the recently added symlink guard in file
  storage to prevent regression
- **Expected Outcome:** `file-storage.test.ts` includes test cases that create
  symlinks pointing outside the storage root and verify that read/write
  operations through the symlink are rejected; both relative and absolute
  symlink targets are tested
- **Validation:** `pnpm -F @eddacraft/anvil-platform-storage test -- --testNamePattern="symlink|escape"`
  passes with at least 3 symlink-specific test cases
- **Files:** `packages/platform/storage/src/file-storage.test.ts`
- **Background:** Path-escape tests already exist (traversal via `../` and
  absolute paths). This item adds symlink-specific tests — creating symlinks
  that point outside the storage root and verifying operations through them are
  rejected.
- **Dependencies:** None (the symlink guard is already implemented; this adds
  test coverage)
- **Confidence:** high
- **Priority:** Medium
- **Status:** Complete
- **Notes:** Added 6 symlink-specific tests: symlink file pointing outside base
  (read, write, delete), symlinked intermediate directory, absolute symlink
  target outside base, and internal symlink (allowed). All tests skipped on
  Windows where symlinks require admin. Total tests in file: 39 (up from 33).

---

### CRB-016: Add Windows separator tests to MCP path guards

- **Intent:** Add test coverage for Windows-style path separators in MCP server
  path traversal guards to prevent bypass on Windows
- **Expected Outcome:** Test files `fix.tool.test.ts`, `suppress.tool.test.ts`,
  and `resources.test.ts` include test cases using backslash separators
  (`..\\..\\etc\\passwd`) and mixed separators (`../..\\etc/passwd`) to verify
  traversal guards reject them
- **Validation:** `npx vitest run packages/mcp-server/ --testNamePattern="windows|separator|backslash"`
  passes with at least 3 test cases per tool
- **Files:** `packages/mcp-server/src/tools/fix.tool.test.ts`,
  `packages/mcp-server/src/tools/suppress.tool.test.ts`,
  `packages/mcp-server/src/resources.test.ts`
- **Dependencies:** CRB-013 (MCP tests must be discoverable by vitest first)
- **Confidence:** high
- **Priority:** Low
- **Status:** Complete
- **Completed:** 2026-03-04
- **Notes:** Windows-style backslash and mixed separator tests added to 8 MCP
  test files (fix, suppress, resources, check, gate, query-boundary, status,
  server). Commit: edc53765.

---

### CRB-017: Add tests for platform/core config loaders

- **Intent:** Add test coverage for the config loader modules that currently
  have none
- **Expected Outcome:** Test files exist for
  `packages/platform/config/src/loader.ts` and
  `packages/anvil/core/src/config/loader.ts` covering: loading from file,
  loading from environment variables, missing config handling, and invalid
  config format errors (or, if loaders remain placeholders per CRB-012,
  tests verify the placeholder behaviour is explicit and documented)
- **Validation:** `pnpm -F @eddacraft/platform-config test` and
  `pnpm -F @eddacraft/anvil-core test` pass with loader test files included
- **Files:** `packages/platform/config/src/loader.test.ts` (new),
  `packages/anvil/core/src/config/loader.test.ts` (new)
- **Dependencies:** CRB-012 (loader status must be resolved to know what to
  test)
- **Confidence:** medium
- **Priority:** Low
- **Status:** Complete
- **Notes:** Test files already existed for both config loaders. Verified
  coverage is adequate. No additional work needed.

- **Intent:** Ensure all development commands (test, lint, build, check) work
  predictably from the repo root without requiring `cd` into specific packages
- **Expected Outcome:** A documented "how to run commands" section in
  CONTRIBUTING.md or CLAUDE.md explains the canonical way to run dev commands;
  `pnpm -F <pkg> test` works correctly for all packages; vitest globs and
  build scripts are consistent with the documented approach; misleading
  `pnpm -C <pkg> test` patterns are identified and either supported or
  documented as unsupported
- **Validation:** Manual review — running documented commands from repo root
  produces expected results for all packages
- **Files:** `CLAUDE.md` or `CONTRIBUTING.md`, `vitest.config.ts` (root),
  per-package `package.json` scripts
- **Dependencies:** CRB-006 (vitest config strategy is a prerequisite)
- **Confidence:** medium
- **Priority:** Medium
- **Status:** Complete
- **Notes:** .prettierignore expanded with build artifact patterns.
  CONTRIBUTING.md updated with comprehensive development workflow section.
  All documented commands verified working from repo root. PR #505.

---

### CRB-025: Docs and scripts drifting from reality

- **Intent:** Audit documentation and scripts for accuracy against the current
  codebase and fix or remove stale references
- **Expected Outcome:** README files, quickstart guides, and scripts reference
  correct paths, command names, and options that actually work; commands shown
  in docs can be copy-pasted and run successfully from the documented working
  directory; stale references to moved or renamed files are updated or removed
- **Validation:** Manual review — each command in README and docs-site
  quickstart can be executed as documented without error
- **Files:** `README.md`, `apps/docs-site/docs/**/*.md`,
  `packages/*/README.md`, `tools/**/*.sh`
- **Dependencies:** CRB-018 (workflow standardisation informs what the docs
  should say)
- **Confidence:** medium
- **Priority:** Low
- **Status:** Complete
- **Notes:** All 21 documentation issues identified and fixed across 8 files.
  Fixed Node.js version requirements (20.0.0 → 20.19.0) in 3 files, corrected
  package names and structure trees, removed stale references to non-existent
  directories (docker/, packages/shared, apps/docs-site/docs/), updated command
  and service counts in CLI AGENTS.md. All documented commands verified working:
  `pnpm build` (22 projects), `pnpm test` (220 files, 4798 tests), `pnpm lint`
  (0 errors), `pnpm typecheck` (22 projects). PR #510.

---

### Beta Review Gaps (CRB-026 through CRB-029)

### CRB-026: Fix spinner leak on TUI fallback path in audit command

- **Intent:** Ensure the ora spinner is cleaned up if an error is thrown during
  the scan or TUI rendering phase, preventing terminal garbling
- **Expected Outcome:** The spinner is wrapped in a try/finally so that any
  thrown error during `scanner.scan()` or `renderTUI()` stops the spinner before
  the error propagates; the spinner no longer runs concurrently with TUI output
  if TUI rendering is selected
- **Validation:** `pnpm -F anvil-cli test -- --testNamePattern="audit"` passes;
  manual test confirms spinner stops on scan error
- **Files:** `apps/anvil-cli/src/commands/audit.ts`
- **Dependencies:** None
- **Confidence:** high
- **Priority:** Medium
- **Status:** Complete
- **Notes:** Beta review item H-4. PBLU-032 added a related spinner test;
  PBLU-026 addressed CliExit patterns. Added finally block guaranteeing
  spinner?.stop() on all code paths.
- **Origin:** cli-beta-review.md H-4

---

### CRB-027: Add workspace path containment to policy validate subcommand

- **Intent:** Validate that the user-provided file path in `policy validate`
  is within the workspace root, consistent with other path-accepting commands
- **Expected Outcome:** `policy validate <file>` resolves the file argument
  against `getWorkspaceRoot()` using the existing `validatePathWithinRoot()`
  utility before reading it; paths outside the workspace produce a clear error;
  relative paths are resolved against the workspace root
- **Validation:** `pnpm -F anvil-cli test -- --testNamePattern="policy"` passes;
  manual test confirms `policy validate ../../etc/passwd` is rejected
- **Files:** `apps/anvil-cli/src/commands/policy.ts`
- **Dependencies:** None (`validatePathWithinRoot` is already imported)
- **Confidence:** high
- **Priority:** Medium
- **Status:** Complete
- **Notes:** Beta review item M-2. Added `validatePathWithinRoot()` call before
  `readFile()`, consistent with `policy doc`, `scaffold`, `export`, `plan create`.
- **Origin:** cli-beta-review.md M-2

---

### CRB-028: Annotate M-6 mcp-config symlink guard as fixed

- **Intent:** Update the beta review document to reflect that M-6 (symlink path
  traversal in `mcp-config --write`) was already fixed via PBLU-011 and PBLU-014
- **Expected Outcome:** `cli-beta-review.md` has a FIXED annotation on M-6
  noting that `realpathSync()` is now used on both paths (lines 118-137 of
  `mcp-config.ts`) with a parent-directory fallback for non-existent files
- **Validation:** Manual review of the review document
- **Files:** `plans/reviews/cli-beta-review.md`
- **Dependencies:** None (code fix already in place)
- **Confidence:** high
- **Priority:** Low
- **Status:** Complete
- **Notes:** The code at mcp-config.ts:118-137 already implements the exact
  fix recommended by the review (realpathSync on both paths before comparison).
  Annotated cli-beta-review.md with FIXED status.
- **Origin:** cli-beta-review.md M-6

---

### CRB-029: Expand test coverage for untested CLI commands

- **Intent:** Add baseline test coverage for CLI commands that currently have
  no tests, reducing regression risk during refactoring
- **Expected Outcome:** At minimum, command registration tests (subcommands,
  options, descriptions) exist for: `login`, `logout`, `whoami`, `authorship`,
  `drift`, `explain`, `audit`, `new`, `plan create`, `release`,
  `welcome/start`; ideally one behavioural test per command covering the happy
  path
- **Validation:** `pnpm -F anvil-cli test` discovers and passes tests for all
  previously untested commands
- **Files:** `apps/anvil-cli/src/commands/__tests__/` or colocated `.test.ts`
  files
- **Dependencies:** None
- **Confidence:** medium
- **Priority:** Medium
- **Status:** Complete
- **Completed:** 2026-03-07
- **Notes:** 12 colocated test files created for login, logout, whoami,
  authorship, drift, explain, new, plan, release, validate, welcome,
  architecture. Each file tests command registration (name, description,
  subcommands/options) and at least one happy-path execution. All 12 new
  suites pass; pre-existing failures in edda/ember suites are unrelated.
- **Origin:** cli-beta-review.md recommendation 13
