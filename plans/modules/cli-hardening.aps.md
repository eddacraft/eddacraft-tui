<!--
APS Module: Codebase Hardening
==============================
Addresses issues from the 2026-02-06 adversarial code reviews.
See: apps/anvil-cli/REVIEW.md, packages/anvil/core/REVIEW.md, apps/anvil-api/REVIEW.md, REVIEW.md
-->

# Codebase Hardening

| ID                                       | Owner | Status      |
| ---------------------------------------- | ----- | ----------- |
| CLIH, CORE, API, MCP, RT, POL, ADP, APS, VSIX, PLAT | — | In Progress |

## Branch Status

> **Note:** Branch `hardening/wave-0-foundation` (commit `333a291`) implemented
> 47 of 66 tasks. Additional P2 tasks completed on 2026-02-15: CLIH-009,
> CLIH-010, CORE-007, CORE-009, API-007, APS-PKG-002 (53 of 66 total).

## Purpose

Address the 129 issues identified across adversarial code reviews (2026-02-06):

- **anvil-cli** (scope CLIH): 3 high, 10 medium, 6 low → 18 tasks
- **anvil-core** (scope CORE): 2 high, 7 medium, 5 low → 14 tasks
- **anvil-api** (scope API): 2 high, 5 medium, 5 low → 12 tasks
- **mcp-server** (scope MCP): 3 crit, 3 high, 3 medium, 3 low → 6 tasks
- **anvil/runtime** (scope RT): 6 high, 7 medium, 6 low → 5 tasks
- **anvil/policy** (scope POL): 2 high, 8 medium, 5 low → 3 tasks
- **adapters** (scope ADP): 3 high, 4 medium, 3 low → 3 tasks
- **aps** (scope APS): 2 high, 7 medium, 4 low → 2 tasks
- **vscode-extension** (scope VSIX): 3 high, 4 medium, 3 low → 2 tasks
- **platform/storage** (scope PLAT): 1 high → 1 task
- **website, contracts, eslint-plugin, ports**: low-severity only → 0 tasks

Total: **66 tasks** across all packages.

**Sources:**

- [apps/anvil-cli/REVIEW.md](../../apps/anvil-cli/REVIEW.md)
- [packages/anvil/core/REVIEW.md](../../packages/anvil/core/REVIEW.md)
- [apps/anvil-api/REVIEW.md](../../apps/anvil-api/REVIEW.md)
- [REVIEW.md](../../REVIEW.md) (remaining packages)

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

### anvil-api (API)

- P0 CORS and rate limiting (2 items)
- P1 scope validation, transactions, and output validation (3 items)
- P2 input validation and audit improvements (3 items)
- P3 optional hardening and documentation (4 items)

### mcp-server (MCP)

- P0 authentication, workspace validation, newline injection (3 items)
- P1 race conditions, prompt injection (2 items)
- P3 optional (1 item)

> **See also:** [mcp-server-hardening.aps.md](./mcp-server-hardening.aps.md)
> (MCPH) covers additional MCP issues from the 2026-02-05 review. CLIH MCP-001–006
> and MCPH-001–009 have partial overlap on workspace validation and HTTP transport
> hardening — coordinate implementation to avoid duplicate work.

### anvil/runtime (RT)

- P0 OPA binary path, policy dir traversal (2 items)
- P1 cache integrity, temp dir safety, env var exfiltration (3 items)

### anvil/policy (POL)

- P0 tar extraction path traversal (1 item)
- P1 bundle manifest path traversal, URL validation (2 items)

### adapters (ADP)

- P1 path validation, input size limits, regex DoS (3 items)

### aps (APS)

- P1 path traversal in module loader (1 item)
- P2 hash verification (1 item)

### vscode-extension (VSIX)

- P1 CLI output validation, gate path validation (2 items)

### platform/storage (PLAT)

- P0 path traversal in FileStorage (1 item)

## Out of Scope

- New CLI features (handled by other modules)
- Auth UX improvements beyond the identified issues
- TUI visual changes
- New core features or API changes
- New API features or endpoints
- Website, contracts, eslint-plugin, ports (low-severity only)

## Interfaces

**Depends on:**

- `@eddacraft/anvil-cli` — Package being hardened
- `@eddacraft/anvil-core` — Package being hardened
- `@eddacraft/anvil-api` — Package being hardened
- `@eddacraft/anvil-runtime` — For gate runner and policy config

**Exposes:**

- Hardened CLI with improved input validation, path safety, and startup resilience
- Hardened core library with shell-safe exec, path sanitisation, and accurate docs
- Hardened API with restricted CORS, rate limiting, and output validation

## Ready Checklist

Change status to **Ready** when:

- [ ] All P0/P1 issues have clear implementation paths
- [ ] Team has reviewed path-escape findings (CLIH M8, M9)
- [ ] Team has reviewed exec→execFile migration scope (CORE H1, H2)
- [ ] Team has decided on CORS origin allowlist (API H1)
- [ ] At least one task defined per HIGH finding

## Tasks

### Shared Utilities (UTIL)

### UTIL-001: Extract path safety utilities

- **Intent:** Provide reusable path traversal validation for all packages
- **Expected Outcome:** `packages/anvil/core/src/utils/path-safety.ts` exports
  path safety utilities (e.g., `assertWithinBase()`, `sanitizePath()`) used by
  hardening tasks across PLAT, MCP, RT, POL, and other scopes
- **Validation:** `pnpm -F anvil-core typecheck`
- **Files:** `packages/anvil/core/src/utils/path-safety.ts`
- **Dependencies:** None
- **Confidence:** high
- **Priority:** P0
- **Status:** Complete (2026-02-09)

---

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
- **Status:** Complete (2026-02-09)

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
- **Status:** Complete (2026-02-09)

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
- **Status:** Complete (2026-02-09)

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
- **Status:** Complete (2026-02-09)

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
- **Status:** Complete (2026-02-09)

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
- **Status:** Complete (2026-02-09)

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
- **Status:** Complete (2026-02-09)

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
- **Status:** Complete (2026-02-09)

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
- **Status:** Complete (2026-02-15)

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
- **Status:** Complete (2026-02-15)

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
- **Status:** Complete (2026-02-09)

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
- **Status:** Complete (2026-02-09)

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
- **Status:** Complete (2026-02-09)

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
- **Status:** Complete (2026-02-09)

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
- **Status:** Complete (2026-02-09)

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
- **Status:** Complete (2026-02-09)

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
- **Status:** Complete (2026-02-15)

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
- **Status:** Complete (2026-02-09)

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
- **Status:** Complete (2026-02-15)

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

---

## API Hardening Tasks (API)

### API-001: Restrict CORS to known origins

- **Intent:** Prevent cross-origin abuse of admin endpoints from arbitrary websites
- **Expected Outcome:** `cors()` middleware is configured with an explicit origin
  allowlist; admin endpoints are only callable from approved origins; the
  `/auth/verify` endpoint may remain open or be restricted to known CLI user-agents
- **Validation:** `pnpm -F anvil-api test && curl -H "Origin: https://evil.com" -I https://<api>/api/v1/admin/invite` returns no `Access-Control-Allow-Origin` header
- **Files:** `apps/anvil-api/src/index.ts`
- **Dependencies:** None
- **Confidence:** high
- **Priority:** P0
- **Notes:** Team must decide the allowlist. If there is no web admin UI yet, CORS
  can be disabled entirely for admin routes (only CLI/server-to-server callers).
- **Status:** Complete (2026-02-09)

### API-002: Add rate limiting to all endpoints

- **Intent:** Prevent brute-force token testing, mass invite/revoke abuse, and
  database overload from request floods
- **Expected Outcome:** Rate limiting middleware is applied to all routes;
  `/auth/verify` allows ~60 req/min per IP; `/admin/*` allows ~30 req/min per
  admin key; exceeded limits return 429 with `Retry-After` header
- **Validation:** `pnpm -F anvil-api test -- --testNamePattern="rate"`
- **Files:** `apps/anvil-api/src/index.ts` (or new middleware file)
- **Dependencies:** None
- **Confidence:** high
- **Priority:** P0
- **Notes:** Vercel provides some DDoS protection but application-level limiting
  is needed for abuse prevention and cost control (Neon bills per query).
  Consider `hono-rate-limiter` or a simple in-memory/KV-backed counter.
- **Status:** Complete (2026-02-09)

### API-003: Validate scopes against known allowlist

- **Intent:** Prevent creation of tokens with unintended privilege levels
- **Expected Outcome:** The `scopes` field in `inviteSchema` validates each scope
  against a defined allowlist (e.g., `['beta', 'preview', 'internal']`);
  unknown scopes are rejected with a 400 error listing the valid options
- **Validation:** `pnpm -F anvil-api test -- --testNamePattern="invite|scope"`
- **Files:** `apps/anvil-api/src/routes/admin.ts`
- **Dependencies:** None
- **Confidence:** high
- **Priority:** P1
- **Status:** Complete (2026-02-09)

### API-004: Wrap invite/revoke in database transactions

- **Intent:** Prevent partial completion when audit log insert fails after
  the main operation succeeds
- **Expected Outcome:** The invite flow (upsert user → insert token → audit log)
  and revoke flow (revoke tokens → audit log) are wrapped in database
  transactions; if any step fails, the entire operation is rolled back;
  alternatively, audit log failures are caught and the response still succeeds
  with a warning flag
- **Validation:** `pnpm -F anvil-api test -- --testNamePattern="invite|revoke|transaction"`
- **Files:** `apps/anvil-api/src/routes/admin.ts`,
  `apps/anvil-api/src/db/queries.ts`
- **Dependencies:** None
- **Confidence:** medium
- **Priority:** P1
- **Notes:** Neon serverless client supports transactions via `sql.begin()`.
  If transactions add too much complexity, the simpler fix is to catch audit
  log errors and still return the successful response.
- **Status:** Complete (2026-02-09)

### API-005: Add Zod validation to database query results

- **Intent:** Catch DB schema drift and unexpected query results at runtime
- **Expected Outcome:** Each query function in `queries.ts` validates the result
  against a Zod schema before casting; the `rows()` helper is replaced with
  schema-aware parsing; mismatches throw descriptive errors rather than passing
  malformed data silently
- **Validation:** `pnpm -F anvil-api test -- --testNamePattern="queries|db"`
- **Files:** `apps/anvil-api/src/db/queries.ts`
- **Dependencies:** None
- **Confidence:** high
- **Priority:** P1
- **Status:** Complete (2026-02-09)

### API-006: Add email validation to GET /admin/user/:email

- **Intent:** Reject invalid email parameters before they reach the database
- **Expected Outcome:** The `GET /admin/user/:email` route validates the URL
  parameter against `z.string().email()` before querying; invalid emails
  return 400 with a clear error message
- **Validation:** `pnpm -F anvil-api test -- --testNamePattern="user|email"`
- **Files:** `apps/anvil-api/src/routes/admin.ts`
- **Dependencies:** None
- **Confidence:** high
- **Priority:** P2
- **Status:** Complete (2026-02-09)

### API-007: Track admin identity in audit logs

- **Intent:** Enable accountability when multiple people share admin access
- **Expected Outcome:** Audit log entries include the actual admin identity
  rather than the hardcoded string `'admin'`; implementation is one of:
  (a) per-admin API keys with an admin_users lookup,
  (b) a required `X-Admin-Actor` header recorded alongside the action, or
  (c) source IP + User-Agent logged in the audit metadata
- **Validation:** `pnpm -F anvil-api test -- --testNamePattern="audit"`
- **Files:** `apps/anvil-api/src/routes/admin.ts`,
  `apps/anvil-api/src/middleware/admin-auth.ts`
- **Dependencies:** None
- **Confidence:** medium
- **Priority:** P2
- **Notes:** Option (b) is simplest but unauthenticated. Option (a) is most
  robust but requires a new DB table. Option (c) is zero-effort but less
  reliable. Team should decide which level of accountability is needed.
- **Status:** Complete (2026-02-15)

### API-008: Add request body size limits

- **Intent:** Prevent oversized payloads from consuming memory and DB storage
- **Expected Outcome:** Zod schemas include `.max()` constraints on string
  fields (e.g., `notes: z.string().max(1000)`, `token: z.string().max(100)`);
  optionally, a body size middleware rejects payloads > 10KB before parsing
- **Validation:** `pnpm -F anvil-api test -- --testNamePattern="validation|size"`
- **Files:** `apps/anvil-api/src/routes/admin.ts`,
  `apps/anvil-api/src/routes/auth.ts`
- **Dependencies:** None
- **Confidence:** high
- **Priority:** P2
- **Status:** Complete (2026-02-09)

### API-009: Add database health check to /health endpoint

- **Intent:** Make health endpoint reflect actual service readiness
- **Expected Outcome:** `GET /health` performs a `SELECT 1` against the database;
  if the DB is unreachable, returns `503 { status: 'degraded', db: 'unreachable' }`
  instead of `200 { status: 'ok' }`
- **Validation:** `pnpm -F anvil-api test -- --testNamePattern="health"`
- **Files:** `apps/anvil-api/src/index.ts`
- **Dependencies:** None
- **Confidence:** high
- **Priority:** P3
- **Status:** Optional

### API-010: Document TOKEN_PEPPER rotation procedure

- **Intent:** Prevent accidental token invalidation during pepper rotation
- **Expected Outcome:** README or runbook documents how to rotate TOKEN_PEPPER
  without invalidating existing tokens; includes steps for dual-pepper support
  during migration or re-hashing existing tokens
- **Validation:** Manual review
- **Files:** `apps/anvil-api/README.md`
- **Dependencies:** None
- **Confidence:** high
- **Priority:** P3
- **Status:** Optional

### API-011: Gate setClient() behind NODE_ENV check

- **Intent:** Prevent test-only DB client override in production
- **Expected Outcome:** `setClient()` throws an error if `NODE_ENV !== 'test'`;
  or the function is moved to a `test-utils.ts` module not imported in production
- **Validation:** `pnpm -F anvil-api test`
- **Files:** `apps/anvil-api/src/db/client.ts`
- **Dependencies:** None
- **Confidence:** high
- **Priority:** P3
- **Status:** Optional

### API-012: Remove server timestamp from health endpoint

- **Intent:** Reduce information disclosure from health checks
- **Expected Outcome:** `GET /health` returns `{ status: 'ok' }` without a
  `timestamp` field, or the timestamp is only included in non-production
  environments
- **Validation:** `pnpm -F anvil-api test -- --testNamePattern="health"`
- **Files:** `apps/anvil-api/src/index.ts`
- **Dependencies:** None
- **Confidence:** high
- **Priority:** P3
- **Status:** Optional

---

## MCP Server Tasks (MCP)

### MCP-001: Add authentication to HTTP transport

- **Intent:** Prevent unauthenticated access to file-modifying MCP tools
- **Expected Outcome:** HTTP transport requires API key or mutual TLS; unauthenticated
  requests are rejected with 401; CORS is restricted to known origins
- **Validation:** `pnpm -F mcp-server test`
- **Files:** `packages/mcp-server/src/transports/streamable-http.ts`
- **Dependencies:** None
- **Confidence:** high
- **Priority:** P0
- **Status:** Complete (2026-02-09)

### MCP-002: Validate workspaceRoot against server-configured root

- **Intent:** Prevent MCP clients from accessing arbitrary filesystem directories
- **Expected Outcome:** All tools validate `workspaceRoot` is within a configured
  allowed root directory; arbitrary absolute paths are rejected
- **Validation:** `pnpm -F mcp-server test`
- **Files:** `packages/mcp-server/src/tools/check.tool.ts`,
  `packages/mcp-server/src/tools/gate.tool.ts`,
  `packages/mcp-server/src/tools/query-boundary.tool.ts`,
  `packages/mcp-server/src/tools/status.tool.ts`
- **Dependencies:** None
- **Confidence:** high
- **Priority:** P0
- **Status:** Complete (2026-02-09)

### MCP-003: Sanitise reason parameter in suppress tool

- **Intent:** Prevent newline injection into source file comments
- **Expected Outcome:** The `reason` parameter has `\r\n` characters stripped or
  rejected before interpolation into source file comments
- **Validation:** `pnpm -F mcp-server test -- --testNamePattern="suppress"`
- **Files:** `packages/mcp-server/src/tools/suppress.tool.ts`
- **Dependencies:** None
- **Confidence:** high
- **Priority:** P0
- **Status:** Complete (2026-02-09)

### MCP-004: Add file locking for concurrent modifications

- **Intent:** Prevent TOCTOU race conditions in fix and suppress tools
- **Expected Outcome:** File read-modify-write operations use advisory locking
  or atomic writes to prevent concurrent requests from overwriting each other
- **Validation:** `pnpm -F mcp-server test`
- **Files:** `packages/mcp-server/src/tools/fix.tool.ts`,
  `packages/mcp-server/src/tools/suppress.tool.ts`
- **Dependencies:** None
- **Confidence:** medium
- **Priority:** P1
- **Status:** Complete (2026-02-09)

### MCP-005: Sanitise prompt template inputs

- **Intent:** Prevent prompt injection via MCP client-supplied values
- **Expected Outcome:** User inputs interpolated into prompt templates are escaped
  or validated against expected patterns before interpolation
- **Validation:** `pnpm -F mcp-server test`
- **Files:** `packages/mcp-server/src/prompts/fix-violation.prompt.ts`,
  `packages/mcp-server/src/prompts/suppress-violation.prompt.ts`
- **Dependencies:** None
- **Confidence:** medium
- **Priority:** P1
- **Status:** Complete (2026-02-09)

### MCP-006: Add HTTP security headers and session cleanup

- **Intent:** Harden HTTP transport against common web attacks
- **Expected Outcome:** HTTP responses include standard security headers; idle
  sessions are cleaned up after configurable timeout
- **Validation:** `pnpm -F mcp-server test`
- **Files:** `packages/mcp-server/src/transports/streamable-http.ts`
- **Dependencies:** MCP-001
- **Confidence:** high
- **Priority:** P3
- **Status:** Optional

---

## Runtime Tasks (RT)

### RT-001: Validate ANVIL_OPA_PATH environment variable

- **Intent:** Prevent execution of arbitrary binaries via env var override
- **Expected Outcome:** `ANVIL_OPA_PATH` is validated: path must be a regular file
  (not symlink), must exist, and ideally must match an expected binary name pattern;
  or the env var override is removed entirely
- **Validation:** `pnpm -F anvil-runtime test -- --testNamePattern="opa|binary"`
- **Files:** `packages/anvil/runtime/src/gate/policy/opa-binary-manager.ts`
- **Dependencies:** None
- **Confidence:** high
- **Priority:** P0
- **Status:** Complete (2026-02-09)

### RT-002: Validate policy directory stays within workspace

- **Intent:** Prevent loading .rego files from outside the workspace
- **Expected Outcome:** `policy-loader.ts` validates the resolved policy directory
  is within `workspaceRoot` before scanning for .rego files
- **Validation:** `pnpm -F anvil-runtime test -- --testNamePattern="policy|loader"`
- **Files:** `packages/anvil/runtime/src/gate/policy/policy-loader.ts`
- **Dependencies:** None
- **Confidence:** high
- **Priority:** P0
- **Status:** Complete (2026-02-09)

### RT-003: Add HMAC integrity protection to cache entries

- **Intent:** Prevent cache poisoning via tampered .anvil/cache entries
- **Expected Outcome:** Cache entries are signed with HMAC-SHA256 using a
  per-workspace key; entries with invalid signatures are discarded
- **Validation:** `pnpm -F anvil-runtime test -- --testNamePattern="cache"`
- **Files:** `packages/anvil/runtime/src/cache/providers/file-cache.ts`
- **Dependencies:** None
- **Confidence:** medium
- **Priority:** P1
- **Status:** Complete (2026-02-09)

### RT-004: Use fs.mkdtemp() for OPA temp directories

- **Intent:** Eliminate TOCTOU race condition in temp directory creation
- **Expected Outcome:** OPA executor uses `fs.mkdtemp()` for atomic temp dir
  creation instead of `randomUUID()` + `mkdir`
- **Validation:** `pnpm -F anvil-runtime test -- --testNamePattern="opa|executor"`
- **Files:** `packages/anvil/runtime/src/gate/policy/opa-executor.ts`
- **Dependencies:** None
- **Confidence:** high
- **Priority:** P1
- **Status:** Complete (2026-02-09)

### RT-005: Whitelist allowed env var names in bundle verifier

- **Intent:** Prevent exfiltration of arbitrary environment variables
- **Expected Outcome:** Bundle verifier only reads env vars matching an allowlist
  pattern (e.g., `ANVIL_*`); attempts to read other vars are rejected with error
- **Validation:** `pnpm -F anvil-runtime test -- --testNamePattern="bundle|verifier"`
- **Files:** `packages/anvil/runtime/src/gate/policy/bundle-verifier.ts`
- **Dependencies:** None
- **Confidence:** high
- **Priority:** P1
- **Status:** Complete (2026-02-09)

---

## Policy Tasks (POL)

### POL-001: Validate paths in tar archive extraction

- **Intent:** Prevent zip-slip / path traversal during bundle extraction
- **Expected Outcome:** Tarball extraction validates all entry paths are within
  the destination directory; entries with `../` or absolute paths are rejected
- **Validation:** `pnpm -F anvil-policy test -- --testNamePattern="bundle"`
- **Files:** `packages/anvil/policy/src/bundle-manager.ts`
- **Dependencies:** None
- **Confidence:** high
- **Priority:** P0
- **Status:** Complete (2026-02-09)

### POL-002: Validate paths in bundle signature manifest

- **Intent:** Prevent path traversal via crafted .signatures.json
- **Expected Outcome:** `verifySignatureBlock` validates `fileEntry.name` is within
  the bundle directory before constructing file paths
- **Validation:** `pnpm -F anvil-policy test -- --testNamePattern="bundle|verifier"`
- **Files:** `packages/anvil/policy/src/bundle-verifier.ts`
- **Dependencies:** None
- **Confidence:** high
- **Priority:** P1
- **Status:** Complete (2026-02-09)

### POL-003: Enforce HTTPS and domain allowlist for bundle downloads

- **Intent:** Prevent bundle downloads from untrusted sources
- **Expected Outcome:** Bundle downloads enforce HTTPS and validate domains against
  a configurable allowlist; HTTP URLs are rejected
- **Validation:** `pnpm -F anvil-policy test -- --testNamePattern="bundle|download"`
- **Files:** `packages/anvil/policy/src/bundle-manager.ts`
- **Dependencies:** None
- **Confidence:** medium
- **Priority:** P1
- **Status:** Complete (2026-02-09)

---

## Adapters Tasks (ADP)

### ADP-001: Validate extracted file paths from external plan formats

- **Intent:** Prevent path traversal via malicious plan file content
- **Expected Outcome:** All parsers validate extracted file paths: reject absolute
  paths, normalise with `path.normalize()`, verify no `../` escapes
- **Validation:** `pnpm -F adapters test`
- **Files:** `packages/adapters/src/bmad/parser.ts`,
  `packages/adapters/src/speckit/parser.ts`,
  `packages/adapters/src/generic/parser.ts`
- **Dependencies:** None
- **Confidence:** high
- **Priority:** P1
- **Status:** Complete (2026-02-09)

### ADP-002: Add input size limits to all parsers

- **Intent:** Prevent DoS via massive input content
- **Expected Outcome:** All parsers reject content larger than a configurable max
  (default 2MB); recursive parsing has a depth limit (default 20 levels)
- **Validation:** `pnpm -F adapters test`
- **Files:** `packages/adapters/src/base/file-discovery.ts`, all parser files
- **Dependencies:** None
- **Confidence:** high
- **Priority:** P1
- **Status:** Complete (2026-02-09)

### ADP-003: Fix regex DoS vulnerabilities in parsers

- **Intent:** Prevent exponential backtracking on crafted input
- **Expected Outcome:** Complex regex patterns in bmad/speckit parsers are
  simplified or replaced with iterative parsing; input lines are length-limited
  before regex matching
- **Validation:** `pnpm -F adapters test`
- **Files:** `packages/adapters/src/bmad/utils.ts`,
  `packages/adapters/src/speckit/parser.ts`
- **Dependencies:** None
- **Confidence:** medium
- **Priority:** P1
- **Status:** Complete (2026-02-09)

---

## APS Tasks (APS-PKG)

### APS-PKG-001: Fix path traversal in module path resolution

- **Intent:** Prevent reading arbitrary files via malicious index Path fields
- **Expected Outcome:** `resolvePath()` rejects absolute paths and validates
  resolved paths are within `baseDir`; paths with `../` that escape are rejected
- **Validation:** `pnpm -F aps test`
- **Files:** `packages/aps/src/loader/index.ts`
- **Dependencies:** None
- **Confidence:** high
- **Priority:** P1
- **Status:** Complete (2026-02-09)

### APS-PKG-002: Add hash verification to execution plans

- **Intent:** Detect tampering with execution plan files
- **Expected Outcome:** `readExecutionPlan()` recomputes `content_hash` and
  compares against the stored value; mismatches produce a warning
- **Validation:** `pnpm -F aps test`
- **Files:** `packages/aps/src/state/index.ts`
- **Dependencies:** None
- **Confidence:** high
- **Priority:** P2
- **Status:** Complete (2026-02-15)

---

## VS Code Extension Tasks (VSIX)

### VSIX-001: Add schema validation to CLI output parsing

- **Intent:** Prevent untrusted CLI output from injecting arbitrary properties
- **Expected Outcome:** `parseValidationResult()` and `parseGateResults()` validate
  JSON output against Zod schemas before use; unknown properties are stripped
- **Validation:** Extension test suite
- **Files:** `packages/vscode-extension/src/services/anvilService.ts`
- **Dependencies:** None
- **Confidence:** high
- **Priority:** P1
- **Status:** Complete (2026-02-09)

### VSIX-002: Validate gate output file paths against workspace

- **Intent:** Prevent opening arbitrary files via crafted gate output
- **Expected Outcome:** Violation file paths from gate output are validated to be
  within the workspace root before being passed to `vscode.Uri.file()`
- **Validation:** Extension test suite
- **Files:** `packages/vscode-extension/src/providers/gateResultsProvider.ts`
- **Dependencies:** None
- **Confidence:** high
- **Priority:** P1
- **Status:** Complete (2026-02-09)

---

## Platform Tasks (PLAT)

### PLAT-001: Fix path traversal in FileStorage.resolvePath()

- **Intent:** Prevent reads/writes outside the configured baseDir
- **Expected Outcome:** `resolvePath()` uses `path.resolve()` and validates the
  result starts with `baseDir`; absolute paths and `../` escapes are rejected
- **Validation:** `pnpm -F platform-storage test`
- **Files:** `packages/platform/storage/src/file-storage.ts`
- **Dependencies:** None
- **Confidence:** high
- **Priority:** P0
- **Status:** Complete (2026-02-09)

---

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
| CORS restriction blocks legitimate clients | Medium | Inventory all API consumers before restricting; add origins incrementally |
| Rate limiting blocks CI automation      | Medium | Exempt known CI IPs or use higher limits for authenticated requests |
| DB transactions add latency             | Low    | Neon transactions have minimal overhead on serverless |
| Scope allowlist rejects valid tokens    | Medium | Audit existing tokens for scope values before restricting |
