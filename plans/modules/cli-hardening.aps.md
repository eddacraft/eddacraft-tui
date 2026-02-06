<!--
APS Module: Codebase Hardening
==============================
Addresses issues from the 2026-02-06 adversarial code reviews.
See: apps/anvil-cli/REVIEW.md, packages/anvil/core/REVIEW.md
-->

# Codebase Hardening

| ID         | Owner | Status |
| ---------- | ----- | ------ |
| CLIH, CORE | —     | Draft  |

## Purpose

Address the 33 issues identified across two adversarial code reviews (2026-02-06):

- **anvil-cli** (scope CLIH): 3 high, 10 medium, 6 low → 18 tasks
- **anvil-core** (scope CORE): 2 high, 7 medium, 5 low → 14 tasks

These range from high-severity security hardening (P0/P1) through code quality
improvements (P2) to optional cleanups (P3). This module tracks the work needed
to resolve each finding and ensure both packages are production-ready.

**Sources:**

- [apps/anvil-cli/REVIEW.md](../../apps/anvil-cli/REVIEW.md)
- [packages/anvil/core/REVIEW.md](../../packages/anvil/core/REVIEW.md)

## In Scope

### anvil-cli (CLIH)

- P0 startup resilience (1 item)
- P1 security and correctness issues (4 items)
- P2 code quality, input validation, and path safety (5 items)
- P3 optional cleanups and improvements (6 items)
- Architectural refactoring (2 items)

### anvil-core (CORE)

- P0 shell injection remediation (1 item)
- P1 path safety, file operations, and locking (3 items)
- P2 crypto, validation, and documentation fixes (4 items)
- P3 optional cleanups and improvements (6 items)

## Out of Scope

- New CLI features (handled by other modules)
- Auth UX improvements beyond the identified issues
- TUI visual changes
- New core features or API changes

## Interfaces

**Depends on:**

- `@eddacraft/anvil-cli` — Package being hardened
- `@eddacraft/anvil-core` — Package being hardened
- `@eddacraft/anvil-runtime` — For gate runner and policy config

**Exposes:**

- Hardened CLI with improved input validation, path safety, and startup resilience
- Hardened core library with shell-safe exec, path sanitisation, and accurate docs

## Ready Checklist

Change status to **Ready** when:

- [ ] All P0/P1 issues have clear implementation paths
- [ ] Team has reviewed path-escape findings (CLIH M8, M9)
- [ ] Team has reviewed exec→execFile migration scope (CORE H1, H2)
- [ ] At least one task defined per HIGH finding

## Tasks

### CLIH-001: Guard JSON.parse at CLI startup

- **Intent:** Prevent the entire CLI from crashing on corrupted package.json
- **Expected Outcome:** `src/index.ts` wraps the `JSON.parse(readFileSync(...))` call
  in a try/catch with a fallback version string (e.g., `"0.0.0-unknown"`); `--help`
  and all commands remain functional even if package.json is missing or malformed
- **Validation:** `pnpm -F anvil-cli test -- --testNamePattern="startup|version"`
- **Files:** `apps/anvil-cli/src/index.ts`
- **Dependencies:** None
- **Confidence:** high
- **Priority:** P0
- **Status:** Pending

### CLIH-002: Add Zod schema validation for config.yml parsing

- **Intent:** Prevent runtime type errors from malformed YAML config
- **Expected Outcome:** `policy-config.ts` validates `YAML.parse()` output against
  a Zod schema before casting to `AnvilConfig`; invalid config produces a clear
  error message listing which fields failed validation
- **Validation:** `pnpm -F anvil-cli test -- --testNamePattern="policy-config"`
- **Files:** `apps/anvil-cli/src/services/policy-config.ts`
- **Dependencies:** None
- **Confidence:** high
- **Priority:** P1
- **Status:** Pending

### CLIH-003: Add Zod validation for auth API responses

- **Intent:** Prevent unexpected API response shapes from causing runtime errors
- **Expected Outcome:** `auth-client.ts` and `admin-client.ts` validate API
  responses with Zod schemas before returning; invalid responses throw a clear
  error rather than silently passing malformed data
- **Validation:** `pnpm -F anvil-cli test -- --testNamePattern="auth"`
- **Files:** `apps/anvil-cli/src/services/auth-client.ts`,
  `apps/anvil-cli/src/services/admin-client.ts`
- **Dependencies:** None
- **Confidence:** high
- **Priority:** P1
- **Status:** Pending

### CLIH-004: Delete duplicate historical-analyser.ts

- **Intent:** Eliminate maintenance trap from identical duplicate files
- **Expected Outcome:** One of `historical-analyzer.ts` / `historical-analyser.ts`
  is deleted; the surviving file is the single source of truth; a re-export alias
  is added if both import paths are needed; all imports updated
- **Validation:** `pnpm -F anvil-cli typecheck && pnpm -F anvil-cli test`
- **Files:** `apps/anvil-cli/src/services/historical-analyzer.ts`,
  `apps/anvil-cli/src/services/historical-analyser.ts`,
  `apps/anvil-cli/src/services/repo-scanner.ts`
- **Dependencies:** None
- **Confidence:** high
- **Priority:** P1
- **Status:** Pending

### CLIH-005: Consolidate hook scripts into single source

- **Intent:** Eliminate duplicate embedded shell scripts between service and command
- **Expected Outcome:** `commands/hooks.ts` delegates to `services/hook-installer.ts`
  for hook script content instead of maintaining its own copies; the
  `getPreCommitHook()` and `getPrePushHook()` standalone functions in `hooks.ts`
  are removed
- **Validation:** `pnpm -F anvil-cli test -- --testNamePattern="hook"`
- **Files:** `apps/anvil-cli/src/commands/hooks.ts`,
  `apps/anvil-cli/src/services/hook-installer.ts`
- **Dependencies:** None
- **Confidence:** high
- **Priority:** P1
- **Status:** Pending

### CLIH-006: Replace execSync with execFileSync in doctor HooksCheck

- **Intent:** Use shell-safe exec pattern consistent with rest of codebase
- **Expected Outcome:** `HooksCheck.ts` fix method uses
  `execFileSync('npx', ['husky', 'init'], ...)` instead of
  `execSync('npx husky init', ...)`
- **Validation:** `pnpm -F anvil-cli test -- --testNamePattern="doctor"`
- **Files:** `apps/anvil-cli/src/tui/commands/doctor/checks/HooksCheck.ts`
- **Dependencies:** None
- **Confidence:** high
- **Priority:** P2
- **Status:** Pending

### CLIH-007: Validate --output paths stay within workspace

- **Intent:** Prevent `policy doc --output` and `policy scaffold --out` from
  writing outside the workspace root via absolute paths or `../` traversal
- **Expected Outcome:** Both `policy doc` and `policy scaffold` resolve the output
  path and verify it starts with the workspace root; absolute paths or paths that
  escape via `../` produce a clear error and exit non-zero
- **Validation:** `pnpm -F anvil-cli test -- --testNamePattern="policy"`
- **Files:** `apps/anvil-cli/src/commands/policy.ts`
- **Dependencies:** None
- **Confidence:** high
- **Priority:** P2
- **Status:** Pending

### CLIH-008: Add parseInt validation for numeric CLI options

- **Intent:** Prevent NaN from flowing into runtime config when users pass
  non-numeric values to `--parallel`, `--debounce`, or `--refresh`
- **Expected Outcome:** After `parseInt`, each command checks `Number.isNaN()` and
  exits with a clear error message if the value is not a valid integer
- **Validation:** `pnpm -F anvil-cli test -- --testNamePattern="gate|watch|policy"`
- **Files:** `apps/anvil-cli/src/commands/gate.ts`,
  `apps/anvil-cli/src/commands/watch.ts`,
  `apps/anvil-cli/src/commands/policy.ts`
- **Dependencies:** None
- **Confidence:** high
- **Priority:** P2
- **Status:** Pending

### CLIH-009: Add workspace root detection warning

- **Intent:** Alert users when no workspace root is found instead of silently
  falling back to cwd
- **Expected Outcome:** `getWorkspaceRoot()` emits a stderr warning when no
  `package.json` or `.git` directory is found; the warning suggests running from
  a project directory or running `anvil init`
- **Validation:** `pnpm -F anvil-cli test -- --testNamePattern="file-io|workspace"`
- **Files:** `apps/anvil-cli/src/utils/file-io.ts`
- **Dependencies:** None
- **Confidence:** medium
- **Priority:** P2
- **Notes:** May be intentionally silent for use in non-project directories. Check
  with team before adding warning.
- **Status:** Pending

### CLIH-010: Add user confirmation for mcp-config --write outside workspace

- **Intent:** Prevent accidental writes to paths outside the workspace
- **Expected Outcome:** When `--write` resolves to a path outside the workspace,
  the command prompts for confirmation before writing (skippable with `--yes`);
  the naive `~` expansion is replaced with proper home directory resolution
- **Validation:** `pnpm -F anvil-cli test -- --testNamePattern="mcp-config"`
- **Files:** `apps/anvil-cli/src/commands/mcp-config.ts`
- **Dependencies:** None
- **Confidence:** medium
- **Priority:** P2
- **Notes:** Writing to `~/.config/` is the intended behavior for MCP config.
  This finding may be acceptable as-is if the team decides the path is always
  controlled by `generateMcpConfig()` and not user-supplied.
- **Status:** Pending

### CLIH-011: Validate enforcement level before type cast

- **Intent:** Ensure runtime validation precedes TypeScript type assertion
- **Expected Outcome:** The `includes()` check on enforcement level runs before
  the `as EnforcementLevel` cast; invalid values produce a clear error message
- **Validation:** `pnpm -F anvil-cli test -- --testNamePattern="policy"`
- **Files:** `apps/anvil-cli/src/commands/policy.ts`
- **Dependencies:** None
- **Confidence:** high
- **Priority:** P3
- **Status:** Optional

### CLIH-012: Upgrade ora to ESM-native version

- **Intent:** Align spinner dependency with ESM module system
- **Expected Outcome:** `ora` upgraded from v5.3.0 (CommonJS) to v6+ (ESM);
  all import sites updated if API changed
- **Validation:** `pnpm -F anvil-cli typecheck && pnpm -F anvil-cli test`
- **Files:** `apps/anvil-cli/package.json`, all files importing `ora`
- **Dependencies:** None
- **Confidence:** medium
- **Priority:** P3
- **Notes:** Low risk but low value. Only needed if bundler issues arise.
- **Status:** Optional

### CLIH-013: Standardize error output to use utils/output.ts

- **Intent:** Make stderr output consistent and filterable in CI
- **Expected Outcome:** All commands use `error()`, `warn()`, `info()`, `success()`
  from `utils/output.ts` instead of raw `console.error(chalk.red(...))` calls
- **Validation:** `grep -r "console.error" apps/anvil-cli/src/commands/` returns
  zero matches (excluding test files)
- **Files:** `apps/anvil-cli/src/commands/mcp-config.ts`,
  `apps/anvil-cli/src/commands/export.ts`,
  `apps/anvil-cli/src/commands/gate.ts`,
  and other command files with direct console.error calls
- **Dependencies:** None
- **Confidence:** high
- **Priority:** P3
- **Status:** Optional

### CLIH-014: Log warning when ANVIL_SKIP_GATES/HOOKS is active

- **Intent:** Create audit trail when security checks are bypassed
- **Expected Outcome:** When `ANVIL_SKIP_GATES` or `ANVIL_SKIP_HOOKS` environment
  variables are set, a warning is emitted to stderr (not just in --verbose mode)
  so CI logs capture the bypass
- **Validation:** `pnpm -F anvil-cli test -- --testNamePattern="gate|hook"`
- **Files:** `apps/anvil-cli/src/commands/gate.ts`,
  `apps/anvil-cli/src/services/hook-installer.ts`
- **Dependencies:** None
- **Confidence:** high
- **Priority:** P3
- **Status:** Optional

### CLIH-015: Implement YAML export or remove --to yaml option

- **Intent:** Eliminate confusing runtime error for users who pass `--to yaml`
- **Expected Outcome:** Either implement YAML export using the existing `yaml`
  dependency, or remove `yaml` from the `--to` option's accepted values with
  a deprecation notice
- **Validation:** `pnpm -F anvil-cli test -- --testNamePattern="export"`
- **Files:** `apps/anvil-cli/src/commands/export.ts`
- **Dependencies:** None
- **Confidence:** high
- **Priority:** P3
- **Status:** Optional

### CLIH-016: Add timeout to execFileSync git calls in policy diff

- **Intent:** Prevent indefinite hangs on corrupted or very large git repos
- **Expected Outcome:** `execFileSync('git', ...)` calls in `policy.ts` include
  a `timeout` option (e.g., 30 seconds)
- **Validation:** `pnpm -F anvil-cli test -- --testNamePattern="policy"`
- **Files:** `apps/anvil-cli/src/commands/policy.ts`
- **Dependencies:** None
- **Confidence:** high
- **Priority:** P3
- **Status:** Optional

### CLIH-017: Split policy.ts into smaller modules

- **Intent:** Reduce cognitive load and improve maintainability of the 1676-line
  policy command
- **Expected Outcome:** `policy.ts` is split into focused modules: policy CRUD
  operations, Rego template generation, bundle management, and the command
  registration glue. Each module is < 400 lines.
- **Validation:** `pnpm -F anvil-cli typecheck && pnpm -F anvil-cli test`
- **Files:** `apps/anvil-cli/src/commands/policy.ts` (split into multiple files)
- **Dependencies:** None
- **Confidence:** medium
- **Priority:** P3
- **Notes:** Large refactor with risk of merge conflicts. Schedule during a quiet
  period.
- **Status:** Optional

### CLIH-018: Document auth token security model

- **Intent:** Inform users about plaintext token storage tradeoffs
- **Expected Outcome:** README or security documentation explains that tokens are
  stored in `~/.anvil/auth.json` with `0o600` permissions, consistent with
  standard CLI tool practice; notes the option of system keychain for
  higher-security environments
- **Validation:** Manual review
- **Files:** `apps/anvil-cli/README.md` or new `SECURITY.md`
- **Dependencies:** None
- **Confidence:** high
- **Priority:** P3
- **Status:** Optional

---

## Core Hardening Tasks (CORE)

### CORE-001: Migrate provenance collector from exec to execFile

- **Intent:** Eliminate shell injection surface in the most shell-heavy module
- **Expected Outcome:** All `promisify(exec)` calls in `collector.ts` are replaced
  with `promisify(execFile)` using array arguments; the 7 parallel git commands in
  `collectGitContext` use `execFileAsync('git', ['rev-parse', ...])` form; the
  standalone `git config user.name` call in `createProvenanceRecord` is also migrated
- **Validation:** `pnpm -F anvil-core test -- --testNamePattern="collector|provenance"`
- **Files:** `packages/anvil/core/src/provenance/collector.ts`
- **Dependencies:** None
- **Confidence:** high
- **Priority:** P0
- **Status:** Pending

### CORE-002: Migrate drift and git-notes modules from exec to execFile

- **Intent:** Eliminate shell injection surface in remaining shell-using modules
- **Expected Outcome:** `snapshot-capture.ts` and `git-notes.ts` use
  `promisify(execFile)` with array arguments; the existing input validation
  functions (`isValidGitRef`, `isValidRemoteName`, `isValidRevisionRange`) are
  retained as an additional safety layer
- **Validation:** `pnpm -F anvil-core test -- --testNamePattern="drift|git-notes|snapshot"`
- **Files:** `packages/anvil/core/src/drift/snapshot-capture.ts`,
  `packages/anvil/core/src/provenance/git-ai-standard/git-notes.ts`
- **Dependencies:** None
- **Confidence:** high
- **Priority:** P0
- **Status:** Pending

### CORE-003: Add path sanitisation to ProvenanceStore.get() and related methods

- **Intent:** Prevent path traversal via unsanitised record IDs
- **Expected Outcome:** `ProvenanceStore.get(id)`, `findByCommit()`, and `clear()`
  validate the `id` parameter using the same `sanitizeSnapshotIdentifier` pattern
  from `drift/snapshot-storage.ts` (basename extraction + directory separator check);
  IDs containing `../` or path separators throw an error
- **Validation:** `pnpm -F anvil-core test -- --testNamePattern="provenance|store"`
- **Files:** `packages/anvil/core/src/provenance/store.ts`
- **Dependencies:** None
- **Confidence:** high
- **Priority:** P1
- **Status:** Pending

### CORE-004: Fix ProvenanceStore.clear() to actually delete files

- **Intent:** Ensure clear() removes record files instead of leaving empty artifacts
- **Expected Outcome:** `clear()` uses `unlinkSync()` instead of
  `writeFileSync(path, '')` to delete record files; alternatively uses
  `fs.rmSync(historyDir, { recursive: true })` followed by `ensureDirectories()`
- **Validation:** `pnpm -F anvil-core test -- --testNamePattern="provenance|store|clear"`
- **Files:** `packages/anvil/core/src/provenance/store.ts`
- **Dependencies:** CORE-003 (sanitisation should be in place before deletion logic)
- **Confidence:** high
- **Priority:** P1
- **Status:** Pending

### CORE-005: Add file locking or atomic writes to store modules

- **Intent:** Prevent data corruption from concurrent read-modify-write cycles
- **Expected Outcome:** `SuppressionStore` and `ProvenanceStore` use atomic writes
  (write to temp file, then rename) to prevent partial writes; optionally add
  advisory file locking via `proper-lockfile` or similar to prevent lost updates
  from concurrent processes
- **Validation:** `pnpm -F anvil-core test -- --testNamePattern="store|suppression"`
- **Files:** `packages/anvil/core/src/suppression/store.ts`,
  `packages/anvil/core/src/provenance/store.ts`
- **Dependencies:** None
- **Confidence:** medium
- **Priority:** P1
- **Notes:** Atomic rename is the minimum; full locking adds a dependency. Team
  should decide which level of protection is needed based on expected concurrency.
- **Status:** Pending

### CORE-006: Increase generatePlanId() entropy

- **Intent:** Reduce collision probability for plan IDs across teams/repos
- **Expected Outcome:** `generatePlanId()` uses `randomBytes(8)` (64 bits, 16 hex
  chars) or `randomUUID()` instead of `randomBytes(4)` (32 bits); the
  `isValidPlanId` regex is updated to match the new format; existing plan IDs
  remain valid (backward compatible)
- **Validation:** `pnpm -F anvil-core test -- --testNamePattern="hash|planId|crypto"`
- **Files:** `packages/anvil/core/src/crypto/hash.ts`
- **Dependencies:** None
- **Confidence:** high
- **Priority:** P2
- **Notes:** Changing the plan ID format is a breaking change for existing plans.
  Consider supporting both old and new formats in `isValidPlanId`.
- **Status:** Pending

### CORE-007: Fix canonicalizeJSON undefined handling

- **Intent:** Make canonicalization consistent for all input types
- **Expected Outcome:** `canonicalizeJSON(undefined)` either throws an error
  (matching `JSON.stringify` which returns `undefined` the value) or returns
  `"null"` for consistency; the behavior is documented in JSDoc; existing tests
  are updated
- **Validation:** `pnpm -F anvil-core test -- --testNamePattern="hash|canonical|crypto"`
- **Files:** `packages/anvil/core/src/crypto/hash.ts`
- **Dependencies:** None
- **Confidence:** medium
- **Priority:** P2
- **Notes:** Changing this affects hash output for any data that contained
  top-level undefined. Verify no existing hashes depend on this behavior.
- **Status:** Pending

### CORE-008: Add depth limit to entry-detector recursive traversal

- **Intent:** Prevent stack overflow from deeply nested package.json exports
- **Expected Outcome:** `checkExports()` in `entry-detector.ts` accepts a depth
  parameter (default 10) and stops recursing beyond the limit; JSON parse calls
  are wrapped in try-catch
- **Validation:** `pnpm -F anvil-core test -- --testNamePattern="entry|detector|architecture"`
- **Files:** `packages/anvil/core/src/architecture/entry-detector.ts`
- **Dependencies:** None
- **Confidence:** high
- **Priority:** P2
- **Status:** Pending

### CORE-009: Update package header to reflect actual I/O usage

- **Intent:** Accurately document the package's dependency profile
- **Expected Outcome:** The `src/index.ts` module comment is updated to reflect
  that provenance, drift, architecture, and suppression modules perform I/O;
  either the comment is corrected or I/O is extracted to `@eddacraft/anvil-runtime`
- **Validation:** Manual review
- **Files:** `packages/anvil/core/src/index.ts`
- **Dependencies:** None
- **Confidence:** high
- **Priority:** P2
- **Notes:** If the team's intent is truly to keep core I/O-free, this becomes a
  larger refactoring task to move I/O into anvil-runtime. Mark as P2 for the
  documentation fix; a full I/O extraction would be a separate module.
- **Status:** Pending

### CORE-010: Add debug logging for silently skipped files

- **Intent:** Make architecture analysis failures visible for debugging
- **Expected Outcome:** `analyzer.ts`, `edge-detector.ts`, `entry-detector.ts`,
  and `snapshot-capture.ts` use the existing `createDebugger` utility to log
  when files are skipped due to read errors, parse failures, or permission issues
- **Validation:** `DEBUG=anvil:* pnpm -F anvil-core test -- --testNamePattern="architecture"`
- **Files:** `packages/anvil/core/src/architecture/analyzer.ts`,
  `packages/anvil/core/src/architecture/edge-detector.ts`,
  `packages/anvil/core/src/architecture/entry-detector.ts`,
  `packages/anvil/core/src/drift/snapshot-capture.ts`
- **Dependencies:** None
- **Confidence:** high
- **Priority:** P3
- **Status:** Optional

### CORE-011: Batch getAuthorshipStats commit processing

- **Intent:** Improve performance for large revision ranges
- **Expected Outcome:** `getAuthorshipStats` uses `git notes list` to get all
  note references in one command, then reads only the commits that have notes,
  rather than checking each commit individually
- **Validation:** `pnpm -F anvil-core test -- --testNamePattern="git-notes|authorship|stats"`
- **Files:** `packages/anvil/core/src/provenance/git-ai-standard/git-notes.ts`
- **Dependencies:** CORE-002 (exec migration should happen first)
- **Confidence:** medium
- **Priority:** P3
- **Status:** Optional

### CORE-012: Add input validation to expandLineRanges

- **Intent:** Report errors on malformed line range input instead of silent NaN
- **Expected Outcome:** `expandLineRanges` validates each part is a valid number
  or range before processing; malformed input throws a descriptive error or
  returns an empty array with a warning
- **Validation:** `pnpm -F anvil-core test -- --testNamePattern="serializer|lineRange"`
- **Files:** `packages/anvil/core/src/provenance/git-ai-standard/serializer.ts`
- **Dependencies:** None
- **Confidence:** high
- **Priority:** P3
- **Status:** Optional

### CORE-013: Complete architecture violations detection

- **Intent:** Make boundary violation detection functional
- **Expected Outcome:** The violations detection in `analyzer.ts` produces actual
  results based on the layer definitions and dependency rules, instead of
  returning an empty array
- **Validation:** `pnpm -F anvil-core test -- --testNamePattern="analyzer|violation|architecture"`
- **Files:** `packages/anvil/core/src/architecture/analyzer.ts`
- **Dependencies:** None
- **Confidence:** low
- **Priority:** P3
- **Notes:** This may be intentionally deferred. Confirm with team whether
  violations detection is in scope for the current release.
- **Status:** Optional

### CORE-014: Add log sanitisation to debug utility

- **Intent:** Prevent sensitive data from appearing in debug output
- **Expected Outcome:** The debug utility redacts known sensitive patterns
  (tokens, passwords, API keys) from logged arguments when `DEBUG=*` is enabled;
  at minimum, values matching common token formats are replaced with `[REDACTED]`
- **Validation:** `pnpm -F anvil-core test -- --testNamePattern="debug"`
- **Files:** `packages/anvil/core/src/utils/debug.ts`
- **Dependencies:** None
- **Confidence:** medium
- **Priority:** P3
- **Notes:** Redaction heuristics can produce false positives. Keep the approach
  simple (e.g., redact values longer than 20 chars that look like hex/base64).
- **Status:** Optional

## Risks

| Risk                                    | Impact | Mitigation                                       |
| --------------------------------------- | ------ | ------------------------------------------------ |
| Path validation breaks MCP config       | Medium | CLIH M7 writes outside workspace intentionally   |
| Workspace root warning is too noisy     | Low    | Gate behind --verbose or only warn once           |
| policy.ts refactor causes conflicts     | Medium | Schedule during quiet period; feature-flag        |
| Zod validation rejects valid config     | Medium | Start with loose schema, tighten incrementally    |
| exec→execFile migration breaks on edge cases | Medium | Run full test suite; some git commands may need shell features (pipes, redirects) — verify each call |
| Plan ID format change is breaking       | High   | Support both old (8-char) and new (16-char) formats in `isValidPlanId` |
| File locking adds new dependency        | Low    | Atomic rename (no new dep) is sufficient minimum  |
| canonicalizeJSON change alters hashes   | High   | Audit existing stored hashes before changing      |
