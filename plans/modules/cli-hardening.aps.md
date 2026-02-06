<!--
APS Module: CLI Hardening
=========================
Addresses issues from the 2026-02-06 adversarial code review.
See: apps/anvil-cli/REVIEW.md
-->

# CLI Hardening

| ID    | Owner | Status |
| ----- | ----- | ------ |
| CLIH  | —     | Draft  |

## Purpose

Address the 19 issues identified in the anvil-cli adversarial code review
(2026-02-06). These range from high-severity security hardening (P0/P1) through
code quality improvements (P2) to optional cleanups (P3). This module tracks the
work needed to resolve each finding and ensure the CLI is production-ready.

**Source:** [apps/anvil-cli/REVIEW.md](../../apps/anvil-cli/REVIEW.md)

## In Scope

- P0 startup resilience (1 item)
- P1 security and correctness issues (4 items)
- P2 code quality, input validation, and path safety (5 items)
- P3 optional cleanups and improvements (6 items)
- Architectural refactoring (2 items)

## Out of Scope

- New CLI features (handled by other modules)
- Auth UX improvements beyond the identified issues
- TUI visual changes

## Interfaces

**Depends on:**

- `@eddacraft/anvil-cli` — Package being hardened
- `@eddacraft/anvil-runtime` — For gate runner and policy config

**Exposes:**

- Hardened CLI with improved input validation, path safety, and startup resilience

## Ready Checklist

Change status to **Ready** when:

- [ ] All P0/P1 issues have clear implementation paths
- [ ] Team has reviewed path-escape findings (M8, M9)
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

## Risks

| Risk                               | Impact | Mitigation                                    |
| ---------------------------------- | ------ | --------------------------------------------- |
| Path validation breaks MCP config  | Medium | M7 writes outside workspace intentionally     |
| Workspace root warning is too noisy| Low    | Gate behind --verbose or only warn once        |
| policy.ts refactor causes conflicts| Medium | Schedule during quiet period; feature-flag     |
| Zod validation rejects valid config| Medium | Start with loose schema, tighten incrementally |
