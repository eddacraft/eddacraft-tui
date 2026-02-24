# Anvil CLI Beta Code Review

**Date:** 2026-02-23
**Scope:** `apps/anvil-cli` — full CLI codebase review for beta readiness
**Version:** 0.1.2-beta
**Reviewer:** Claude (automated)

---

## Executive Summary

The Anvil CLI is a well-structured Commander.js-based tool with 25+ commands,
an Ink/React TUI layer, Zod-validated API interactions, and a comprehensive
service layer. The codebase demonstrates strong fundamentals: consistent error
handling, good separation of concerns, and thoughtful security practices. There
are, however, several issues that should be addressed before general
availability and a handful that warrant attention during beta.

**Verdict:** Ready for beta with caveats. The issues below are categorised by
severity.

---

## Architecture Overview

```
src/
  index.ts              Entry point, command registration, auth gate
  commands/             25+ command modules (Commander.js)
    plan/               Plan subcommand tree
    agent/              Agent subcommand tree
    __tests__/          Command unit tests
  services/             Business logic layer
    auth-store.ts       Token persistence (~/.anvil/auth.json)
    auth-client.ts      Token verification
    api-client.ts       HTTP client for EddaCraft API
    admin-client.ts     Admin operations (invite/revoke)
    kindling-bootstrap  Kindling stack initialisation
    ...12 more service modules
  tui/                  Ink/React terminal UI
    commands/           TUI command views
    components/         Shared UI components
    utils/              TTY detection, rendering, theme
  utils/                File I/O, env, output formatting
```

**Strengths:**
- Clean command factory pattern (`createXCommand() → Command`)
- Auth gate via Commander.js `preAction` hook with sensible exemptions
- Graceful degradation for TUI (TTY detection, CI detection, `--no-tui`)
- Path traversal guards on plan IDs and policy output paths
- Restrictive file permissions on auth tokens (0o600/0o700)
- Zod validation on API responses
- Debug logging via `createDebugger()` throughout

---

## Critical Issues

### C-1: Shell command injection via `exec` in historical analyser

**File:** `src/services/historical-analyser.ts:160,178-179,259`
**Severity:** HIGH (CWE-78)

The `HistoricalAnalyser` uses `exec` (which spawns a shell) with string
interpolation for git commands, rather than `execFile` (which passes args
directly):

```typescript
// Line 160: shell interpretation
await execAsync('git rev-parse --git-dir', { cwd: this.projectRoot });

// Line 178-179: daysBack/maxCommits are user-controlled
await execAsync(
  `git log --since="${since}" -${config.maxCommits} ...`,
  { cwd: this.projectRoot }
);

// Line 259: commit.hash from parsed git output
await execAsync(`git show ${commit.hash} --pretty="" --unified=0`, ...);
```

The `commit.hash` at line 259 is derived from parsing git log output. A
maliciously crafted git repository could supply content that confuses the
pipe-delimited parsing at line 200, causing attacker-controlled content to
appear in the `hash` field and execute via the shell.

By contrast, `release-git.ts` and the `policy diff` command correctly use
`execFileSync('git', [...args])` which is safe.

**Recommendation:** Replace all `exec`/`execAsync` calls with `execFile`
throughout `historical-analyser.ts`. This is a straightforward fix:

```typescript
import { execFile } from 'node:child_process';
const execFileAsync = promisify(execFile);

await execFileAsync('git', ['show', commit.hash, '--pretty=', '--unified=0'], {
  cwd: this.projectRoot, maxBuffer: 5 * 1024 * 1024,
});
```

### C-2: No token refresh or retry on 401

**File:** `src/services/api-client.ts:82-87`

The `apiRequest()` function throws on non-200 responses with no distinction
between 401 (expired/revoked token) and other failures. There is no automatic
token refresh, no prompt to re-login, and no retry with backoff. In beta, users
with expired tokens will see a generic `"failed: 401 ..."` error with no
guidance.

**Recommendation:** Detect 401/403 responses, call `clearAuth()`, and throw a
distinct error: "Authentication expired — run `anvil login` to re-authenticate."

### C-3: `authorship show` uses `process.cwd()` instead of `getWorkspaceRoot()`

**File:** `src/commands/authorship.ts:117`

The `authorship show`, `list`, and `stats` subcommands use
`const workspaceRoot = process.cwd()` directly, bypassing the
`getWorkspaceRoot()` function that all other commands use. This means they
won't work correctly when invoked from a subdirectory.

**Recommendation:** Replace `process.cwd()` with `getWorkspaceRoot()` across
all three subcommands.

---

## High Severity

### H-1: `process.exit()` throughout commands prevents cleanup

**Files:** Most command files (policy.ts, gate.ts, check.ts, audit.ts, etc.)

Almost every command error path calls `process.exit(1)` directly. This:
- Prevents Kindling from flushing observations
- Skips any cleanup registered via `finally` blocks
- Makes the CLI untestable in integration tests (process dies)
- Prevents Commander.js from running any post-action hooks

**Recommendation:** Throw typed errors and let the top-level `main().catch()`
handler set the exit code. Use `process.exitCode = 1` where immediate exit
is truly needed.

### H-2: `commands/index.ts` barrel export drift

**File:** `src/commands/index.ts`

The barrel file exports 17 commands, but `src/index.ts` imports 25. The barrel
is missing: `beta`, `plan`, `validate`, `init`, `hooks`, `policy`, `stack`,
`mcp-config`, `login`, `logout`, `whoami`, `welcome/start`. This means the
barrel isn't usable as a clean public API — consumers must import individual
files.

**Recommendation:** Either make the barrel exhaustive or remove it to avoid
confusion. For a CLI, the barrel isn't strictly needed since `index.ts` is the
sole consumer.

### H-3: Missing input validation on `--days-back` / `--max-commits` float edge

**File:** `src/commands/audit.ts:260-270`

`Number(options.daysBack)` parses `"1.5"` as `1.5`, then
`Number.isInteger(1.5)` correctly rejects it. However, `Number("1e3")` parses
to `1000` and passes the integer check and `<= 10000` bound. This is unlikely
to cause harm but is inconsistent with the `parseInt()` pattern used elsewhere.

**Recommendation:** Use `parseInt(value, 10)` consistently, and reject values
where `parseInt(v) !== Number(v)` (catches `1e3`, `0x10`, etc.).

### H-4: Spinner leak on TUI fallback path

**File:** `src/commands/audit.ts:254-286`

When `spinner` is created and then `isTUIAvailable()` returns true, the TUI
renderer is used but the spinner was already created and `stop()`ed. If the
TUI render fails _before_ the spinner is stopped (e.g., a thrown error between
lines 274-286), the spinner continues running and garbles the terminal.

**Recommendation:** Move spinner creation after the TUI availability check, or
use a try/finally to ensure cleanup.

---

## Medium Severity

### M-1: `.env` parser in `loadAnvilEnv()` doesn't handle multi-line values

**File:** `src/utils/env.ts:25-42`

The custom `.env` parser splits on newlines and processes each line
independently. Multi-line values, escaped characters, and export prefixes
(`export FOO=bar`) are not handled. This is fine for simple key=value pairs
but may surprise users familiar with `dotenv` semantics.

**Recommendation:** Document that only simple `KEY=VALUE` syntax is supported,
or switch to the `dotenv` package.

### M-2: `policy validate` doesn't validate path is within workspace

**File:** `src/commands/policy.ts:922`

The `policy validate <file>` command reads a file at the user-provided path
without verifying it's within the workspace root. While this is not exploitable
in a CLI context (the user controls the path), it's inconsistent with the
careful path validation done in `policy doc` and `policy scaffold`.

### M-3: Redundant `process.exit(0)` in audit command

**File:** `src/commands/audit.ts:322-332`

The audit command explicitly calls `process.exit(0)` for non-blocking results.
This is redundant (the process exits naturally) and prevents Commander.js
post-action hooks from running. It also abruptly kills any pending async work
(e.g., Kindling observation flush).

### M-4: Plan `create` subcommand has hardcoded `branch: 'main'`

**File:** `src/commands/plan.ts:72`

The provenance object in `plan create` hardcodes `branch: 'main'` with a
`// TODO: Get from git` comment. This generates incorrect provenance data.

**Recommendation:** Use `execFileSync('git', ['rev-parse', '--abbrev-ref',
'HEAD'])` or mark the field as empty/unknown rather than wrong.

### M-5: Missing `--json` output on several commands

Some commands lack `--json` for machine-readable output, which is important for
CI/CD integration:
- `hooks status`
- `authorship show/list/stats` (has `--json` — good)
- `plan create`
- `plan lock`/`unlock`

**Recommendation:** Add `--json` to all commands that produce structured output.

### M-6: `mcp-config --write` path traversal with `~` expansion

**File:** `src/commands/mcp-config.ts:104-107`

The `~` expansion uses `homedir()` which is correct, but the "is outside
workspace" check uses `pathRelative(cwd, fullPath)` which doesn't account for
symlinks. A symlink in the path could bypass the confirmation prompt.

**Recommendation:** Use `fs.realpathSync()` on both paths before comparison.

### M-7: No request timeout on API calls

**File:** `src/services/api-client.ts:64-68`

The `fetch()` call has no `signal` with `AbortController` timeout. A hanging
API server will cause the CLI to hang indefinitely.

**Recommendation:** Add a configurable timeout (e.g., 30s default) using
`AbortSignal.timeout(30_000)`.

---

## Low Severity / Code Quality

### L-1: `declare const __CLI_VERSION__` pattern is fragile

**File:** `src/index.ts:36-37`

The version is injected via esbuild `define` as `__CLI_VERSION__`. The fallback
`'0.0.0-dev'` works, but TypeScript's `typeof __CLI_VERSION__ !== 'undefined'`
check only works because `noEmit` is used. If someone compiles with `tsc`, the
declare would fail.

### L-2: Several commands import `ora` but don't use it in all paths

E.g., `explain.ts` imports `chalk` but not `ora`, while `policy.ts` imports
`ora` and uses it heavily. Some commands (like `explain`) write directly to
`console.log` with manual formatting rather than using the shared `output.ts`
helpers. Minor inconsistency.

### L-3: TUI ErrorBoundary swallows render errors

**File:** `src/tui/components/ErrorBoundary.tsx` (referenced by renderer)

The ErrorBoundary catches render errors and calls `onExit()`. If `onExit` is
not provided, errors are silently swallowed. Consider logging to stderr.

### L-4: Git hook scripts don't validate `ANVIL_SKIP_HOOKS` strictly

The hooks command tells users to use `ANVIL_SKIP_HOOKS=1` to bypass, but the
hook content generation is delegated to `HookInstaller` — couldn't verify the
exact check. Ensure it's a strict `"1"` or `"true"` check, not just
"variable exists".

### L-5: Mixed test file locations

Tests are split between:
- `src/commands/__tests__/` (proper `__tests__` directory)
- `src/commands/*.test.ts` (colocated)
- `src/services/__tests__/`
- `src/utils/*.test.ts` (colocated)
- `src/__tests__/` (integration)
- `src/__tests__/e2e/` (end-to-end)

This is inconsistent. Pick one convention.

### L-6: `formatRelativeTime()` and `formatSize()` are defined in `policy.ts` but are generic utilities

**File:** `src/commands/policy.ts:1190-1210`

These helper functions are useful beyond the policy command. They could live in
`utils/output.ts` for reuse.

---

## Security Assessment

### Positive Findings
- Auth tokens stored with `0o600` permissions, directory with `0o700`
- Plan ID validation uses strict regex (`/^aps-[a-f0-9]{8,}$/i`) preventing
  path traversal
- `findPlanById()` double-checks resolved path starts with plans directory
- `policy scaffold` validates `--out` path with `validatePathWithinRoot()`
- `policy doc` validates output path with relative path check
- Zod schema validation on API responses
- Most git operations use `execFileSync` with argument arrays (safe)
- `.env` loading does not override existing env vars
- No hardcoded secrets anywhere in source
- Email validation uses Zod (`z.string().email()`) in beta invite

### Concerns
- **C-1 (HIGH):** `historical-analyser.ts` uses `exec` with string
  interpolation for git commands — shell injection vector (see above)
- `loadAuth()` uses `JSON.parse() as StoredAuth` without Zod validation —
  a corrupted `~/.anvil/auth.json` could cause unexpected behaviour
- `login --token` flag exposes token in process list (`ps aux`) and shell
  history — interactive prompt is the safe path
- `beta invite` prints token to stdout — consider suppressing when not a TTY
- `ANVIL_API_URL` override accepts non-HTTPS URLs, enabling SSRF/phishing
  if an attacker controls `~/.anvil/.env`
- Full API response body included in error messages (`api-client.ts:84`) —
  could leak verbose server details
- `export` command writes to user-provided `--output` path without workspace
  validation (inconsistent with `policy doc`)
- `SECURITY.md` documents `--keychain` feature that is not implemented
- Admin key (`ANVIL_ADMIN_KEY`) via env var — standard but document as
  sensitive

---

## Test Coverage Assessment

44 test files found. Coverage spans:
- **Commands:** check, gate, hooks, mcp-config, policy, watch, export, stack,
  init, beta, tutorial, check-interactive
- **Services:** api-client, auth-store, architecture, environment-detector,
  evidence-writer, format-detection, first-run, hook-installer, historical
  analyser, kindling-bootstrap, project-detector, policy-config, sample
  analyser, quick-wins, smart-defaults, template-loader, recent-warnings
- **TUI:** tty-detection, policy-checks, e2e TUI stories
- **Utils:** file-io, env, plan-resolution, spinner, tool-detection
- **Integration:** cli-aps, cli-speckit, cli-gate

**Gaps:**
- No tests for: `login`, `logout`, `whoami`, `authorship`, `drift`,
  `explain`, `audit`, `new`, `plan create`, `release`, `welcome/start`,
  `mcp-config --write` path
- No tests for the `api-client` retry/error paths
- No tests for the `loadAnvilEnv()` parser edge cases

---

## Recommendations for Beta

### Must Fix Immediately
1. ~~**C-1 (HIGH):** Replace `exec` with `execFile` in `historical-analyser.ts`
   — shell injection vulnerability~~ **FIXED** (2026-02-24)
2. ~~**C-2:** Add 401 detection with actionable error message + `clearAuth()`~~
   **FIXED** (2026-02-24)
3. ~~**C-3:** Fix `authorship` workspace root (`process.cwd()` →
   `getWorkspaceRoot()`)~~ **FIXED** (2026-02-24)
4. ~~**M-7:** Add request timeout to API calls (`AbortSignal.timeout()`)~~
   **FIXED** (2026-02-24)

### Must Fix Before GA
5. **H-1:** Replace `process.exit()` with thrown errors (phased — start with
   most-used commands)
6. **M-4:** Fix hardcoded `branch: 'main'` in plan create
7. Validate `ANVIL_API_URL` requires HTTPS scheme
8. Add Zod validation to `loadAuth()` for stored auth integrity
9. Truncate API error response bodies to prevent information leakage

### Should Fix During Beta
10. **H-2:** Clean up barrel export
11. **M-1:** Document `.env` parser limitations
12. **M-5:** Add `--json` to remaining commands
13. Expand test coverage for untested commands
14. Add workspace validation to `export --output` and `new --output`
15. Remove or document `SECURITY.md` keychain claim

### Nice to Have
16. **L-5:** Standardise test file locations
17. **L-6:** Extract shared utility functions
18. **H-3:** Normalise number parsing across commands
19. Deprecate `login --token` flag in favour of `ANVIL_TOKEN` env var

---

## Command Inventory (25 commands)

| Command | Auth Required | JSON Output | TUI Mode | Tests |
|---------|:---:|:---:|:---:|:---:|
| `login` | No | - | - | No |
| `logout` | No | - | - | No |
| `whoami` | No | Yes | - | No |
| `beta invite/revoke` | No* | - | - | Yes |
| `init` | Yes | - | Yes | Yes |
| `check` | Yes | Yes | Yes | Yes |
| `gate` | Yes | Yes | Yes | Yes |
| `gate-config` | Yes | - | - | No |
| `validate` | Yes | Yes | - | No |
| `plan *` | Yes | - | - | Partial |
| `new` | Yes | - | Yes | No |
| `explain` | Yes | Yes | - | No |
| `export` | Yes | Yes | - | Yes |
| `policy *` | Yes | Partial | - | Yes |
| `hooks *` | Yes | - | - | Yes |
| `audit` | Yes | Yes | Yes | No |
| `doctor` | Yes | Yes | Yes | Partial |
| `drift` | Yes | Yes | - | No |
| `watch` | Yes | - | - | Yes |
| `status` | Yes | Yes | Yes | Partial |
| `stack` | Yes | Yes | - | Yes |
| `architecture` | Yes | Yes | - | Partial |
| `authorship *` | Yes | Yes | - | No |
| `tutorial` | No | - | Yes | Partial |
| `mcp-config` | Yes | - | - | Yes |
| `release` | No | - | - | Partial |

\* `beta` requires `ANVIL_ADMIN_KEY` instead of user auth

---

*End of review.*
