# Adversarial Code Review: apps/anvil-cli

**Date:** 2026-02-05 **Reviewer:** Claude (automated adversarial review)
**Scope:** Full `apps/anvil-cli` codebase (~224 TypeScript/TSX files, ~40k LOC)

---

## Executive Summary

The anvil-cli is a well-structured CLI tool with a React-based TUI layer. The
codebase follows reasonable security practices in most areas: no eval/dynamic
code execution, parameterized shell commands, and path traversal protection on
plan IDs. The auth layer is well-implemented with restrictive file permissions
and local token expiry checks. The review identified **3 high-severity
concerns** (unvalidated YAML parsing, shell exec, unguarded JSON.parse), **10
medium-severity issues** (code duplication, path escapes, input validation gaps,
auth design concerns), and a number of lower-severity design issues.

### Severity Counts

| Severity | Count |
| -------- | ----- |
| CRITICAL | 0     |
| HIGH     | 3     |
| MEDIUM   | 10    |
| LOW      | 6     |

---

## CRITICAL Issues

None.

---

## HIGH Issues

### H1. Unguarded YAML.parse on user-controlled config file

**File:** `src/services/policy-config.ts:119-121`

```typescript
const raw = readFileSync(this.configPath, 'utf-8');
const parsed = YAML.parse(raw);
return (parsed as AnvilConfig) ?? {};
```

The `.anvil/config.yml` file is parsed with `YAML.parse()` and immediately cast
to `AnvilConfig` without any schema validation. A malformed or malicious YAML
file could:

1. Produce an object with unexpected shapes that cause runtime type errors deep
   in the call chain (e.g., `resolvePolicies` iterates over `cfg.policies.team`
   assuming it is an array).
2. Exploit YAML-specific features (anchors, merge keys) to produce unexpected
   data structures.

The `yaml` library used here (v2.8.2) does not evaluate arbitrary JS by default,
so this is not an RCE vector, but it can cause confusing crashes.

**Recommendation:** Add Zod schema validation after `YAML.parse()`, consistent
with how other parts of the codebase validate JSON (e.g., `status-service.ts`
uses Zod).

---

### H2. `execSync('npx husky init')` in doctor fix command

**File:** `src/tui/commands/doctor/checks/HooksCheck.ts:64`

```typescript
execSync('npx husky init', {
  cwd: context.projectRoot,
  stdio: ['pipe', 'pipe', 'pipe'],
});
```

This uses `execSync` (shell mode, not `execFileSync`) which passes the command
through a shell interpreter. While the command string is hardcoded and not
vulnerable to injection, using `execSync` is inconsistent with the rest of the
codebase which correctly uses `execFileSync` with array arguments. If anyone
refactors this to accept user input, it becomes an injection vector.

**Recommendation:** Use `execFileSync('npx', ['husky', 'init'], { cwd, stdio })`
instead for defense in depth.

---

### H3. Unprotected JSON.parse at CLI startup

**File:** `src/index.ts:37`

```typescript
const packageJson = JSON.parse(
  readFileSync(join(__dirname, '..', 'package.json'), 'utf-8')
);
```

If `package.json` is missing, corrupted, or has invalid JSON (e.g., during a
partial npm install), this crashes the entire CLI with an unhelpful
`SyntaxError`. Since this runs unconditionally at startup, it blocks all
commands including `--help`.

**Recommendation:** Wrap in try/catch with a fallback version string.

---

## MEDIUM Issues

### M1. Auth token stored as plaintext JSON on disk

**File:** `src/services/auth-store.ts:56-66`

```typescript
writeFileSync(authPath, JSON.stringify(auth, null, 2), { mode: 0o600 });
```

The raw beta token is stored in `~/.anvil/auth.json` as plaintext JSON. The file
is correctly created with `0o600` permissions and the directory with `0o700` —
this is good. However, the token is in the clear, readable by any process
running as the same user. On shared systems or if `~/.anvil` is accidentally
committed or synced, tokens are exposed.

**Mitigating factors:** The permissions are set correctly, local expiry is
checked, and this is standard practice for CLI tools (cf. `~/.npmrc`,
`~/.docker/config.json`). This is a known tradeoff rather than a bug.

**Recommendation:** Consider using the system keychain (e.g., via `keytar`) for
production, or document the security model for users.

---

### M2. Auth API response cast without validation

**File:** `src/services/auth-client.ts:27`

```typescript
return (await res.json()) as VerifyResponse;
```

The response from `/api/v1/auth/verify` is cast to `VerifyResponse` without
runtime validation. A malicious or buggy server could return unexpected shapes.
The `login.ts` command does check for
`result.valid && result.user && result.scopes && result.expiresAt` (line 50),
which provides some defense, but a Zod schema would be more robust.

Same pattern in `admin-client.ts:67, 86, 103`.

---

### M3. Complete file duplication: historical-analyzer.ts / historical-analyser.ts

**Files:**

- `src/services/historical-analyzer.ts` (507 lines)
- `src/services/historical-analyser.ts` (507 lines)

These are **byte-for-byte identical** files (one uses American spelling, the
other British). Both are imported in different parts of the codebase:

- `src/services/repo-scanner.ts` imports from `historical-analyser.ts`
- `src/__tests__/` imports from both

This doubles bundle size for this module and creates a maintenance trap: changes
to one file won't appear in the other.

**Recommendation:** Delete one file. Export from a single canonical file and add
a re-export alias if both spellings are needed.

---

### ~~M4. Duplicate embedded shell scripts~~ ✅

**Files:**

- `src/services/hook-installer.ts:93-145` (embedded pre-commit and pre-push
  scripts)
- `src/commands/hooks.ts:27-83` (identical embedded scripts)

Both files contain identical copies of the pre-commit and pre-push hook shell
scripts. The `hook-installer.ts` service also has a `loadHookScript()` method
that tries to load from `scripts/` files first, falling back to embedded. The
`hooks.ts` command ignores the service entirely and uses its own standalone
functions.

**Fixed:** `hooks` now loads scripts via `HookInstaller`, removing duplicate
embedded scripts in the command.

---

### M5. `getWorkspaceRoot()` silently returns cwd on failure

**File:** `src/utils/file-io.ts:66-78`

When no `package.json` or `.git` directory is found anywhere in the path
hierarchy, the function silently returns `process.cwd()`. Multiple commands
depend on this to find `.anvilrc`, `.anvil/config.yml`, and `.anvil/policies/`.
If invoked from a directory that isn't a project root (e.g., a temp directory,
or `/tmp`), operations will silently read/write files in the wrong location.

**Recommendation:** Consider throwing an error or printing a warning when no
workspace root is found, rather than silently falling back.

---

### M6. `parseInt` without radix validation on CLI options

**Files:**

- `src/commands/gate.ts:243` — `parseInt(options.parallel, 10)` — no NaN check
- `src/commands/watch.ts:176` — `parseInt(options.debounce, 10)` — no NaN check
- `src/commands/policy.ts:1410` — `parseInt(options.refresh || '300000', 10)` —
  no NaN check

If a user passes `--parallel abc` or `--debounce foo`, `parseInt` returns `NaN`,
which then flows into runtime config. Depending on how downstream consumers
handle `NaN`, this could cause subtle bugs.

**Recommendation:** Validate parsed integers and exit with a clear error message
if invalid.

---

### M7. `mcp-config --write` writes outside workspace

**File:** `src/commands/mcp-config.ts:47-52`

```typescript
const expandedPath = config.configPath.startsWith('~/')
  ? config.configPath.replace('~', homedir())
  : config.configPath;
const fullPath = resolve(process.cwd(), expandedPath);
mkdirSync(dirname(fullPath), { recursive: true });
writeFileSync(
  fullPath,
  JSON.stringify(config.content, null, 2) + '\n',
  'utf-8'
);
```

The `--write` flag writes to a path returned by `generateMcpConfig()` which may
be outside the workspace (e.g., `~/.config/claude-code/`). The `~` expansion is
naive (only replaces the first `~`). The path is not validated against any
allowlist. While this is the intended behavior for MCP config, there is no user
confirmation before writing outside the workspace.

---

### M8. `policy doc --output` allows writing anywhere

**File:** `src/commands/policy.ts:794-800`

```typescript
const outputPath = join(workspaceRoot, options.output);
const outputDir = dirname(outputPath);
if (!existsSync(outputDir)) {
  mkdirSync(outputDir, { recursive: true });
}
writeFileSync(outputPath, markdown, 'utf-8');
```

The `--output` path is `join`'d with `workspaceRoot`, but if the user passes an
absolute path (e.g., `--output /etc/cron.d/evil`), `path.join` returns the
absolute path unchanged, allowing writes outside the workspace. Similarly for
relative paths with `../`.

**Recommendation:** Validate that the resolved output path is within the
workspace root.

---

### ~~M9. `policy scaffold --out` creates directories and files outside workspace~~ ✅

**File:** `src/commands/policy.ts:823`

```typescript
const outDir = join(workspaceRoot, options.out);
```

**Fixed:** `--out` now resolves and validates against the workspace root before
creating directories/files.

---

### M10. Assertion-free type casting on enforcement level

**File:** `src/commands/policy.ts:767`

```typescript
const enforcement = options.enforcement as EnforcementLevel;
```

The validation on line 768 checks
`['block', 'warn', 'info', 'off'].includes(enforcement)`, which is correct — but
only after the cast. If the type system is ever relaxed (e.g., in a refactor),
the cast would mask invalid values. The validation should precede the cast.

---

## LOW Issues

### L1. `ora` spinner v5.3.0 is CommonJS; rest of project is ESM

**File:** `package.json`

The `ora` dependency is pinned to `5.3.0`, which is a CommonJS-only version. The
project is ESM (`"type": "module"`). This works due to `esModuleInterop: true`,
but newer ora versions (v6+) are pure ESM. This may cause issues with bundlers
or stricter ESM environments.

---

### L2. Inconsistent error message output

Some commands use `console.error()` directly, some use the `error()` utility
from `utils/output.ts`, and some use both. Examples:

- `src/commands/mcp-config.ts:58` — `console.error('Error:', ...)`
- `src/commands/export.ts:41` — `console.error(chalk.red('Error: ...'))`
- `src/commands/gate.ts:447` — `error()` utility function

This makes it harder to filter stderr output in CI pipelines.

---

### L3. `ANVIL_SKIP_HOOKS` and `ANVIL_SKIP_GATES` bypass mechanisms are not logged

**Files:**

- `src/commands/gate.ts:103-110` — `ANVIL_SKIP_GATES` silently skips checks
- `src/services/hook-installer.ts:117` — `ANVIL_SKIP_HOOKS` silently skips hooks

When these environment variables are set, security checks are skipped without
any audit trail. In a CI environment, an attacker with environment variable
access could bypass all gates.

**Recommendation:** Log a warning (to stderr) when skip environment variables
are detected. The gate command does log when `--verbose` is set, but not by
default.

---

### L4. `export` command has unimplemented YAML support

**File:** `src/commands/export.ts:341-345`

```typescript
async function formatAsYaml(_plan: unknown): Promise<string> {
  throw new Error('YAML export is not yet implemented...');
}
```

The function exists and is callable, but always throws. The `yaml` package is a
declared dependency. This is confusing for users who pass `--to yaml`.

---

### L5. Test helpers use `mkdtempSync` without cleanup guarantees

**File:** `src/__tests__/helpers/test-workspace.ts`

Test workspace directories are created in `os.tmpdir()` but cleanup relies on
test lifecycle hooks. If tests crash, temp directories accumulate.

---

### L6. No timeout on `execFileSync` in `policy diff`

**File:** `src/commands/policy.ts:634-637`

```typescript
const configDiff = execFileSync(
  'git',
  ['diff', '--name-status', 'HEAD', '--', configPath],
  {
    cwd: workspaceRoot,
    encoding: 'utf-8',
  }
).trim();
```

If the git repository is very large or the index is corrupted, this could hang
indefinitely. Consider adding a `timeout` option.

---

## Architectural Observations

### Positive Patterns

1. **Path traversal protection** on plan IDs (`file-io.ts:41-52`) with regex
   validation and resolved path verification.
2. **`execFileSync` with array arguments** consistently used for git commands in
   `policy.ts`, preventing shell injection.
3. **No `eval()` or `Function()` constructor** usage anywhere in the codebase.
4. **No hardcoded credentials** — secrets are always referenced via environment
   variable names.
5. **Zod schema validation** used in some services for runtime type safety.
6. **Proper signal handling** in watch mode (`watch.ts:337-338`).
7. **Auth token file permissions** — `auth-store.ts` creates `~/.anvil/` with
   `0o700` and `auth.json` with `0o600`, with an explicit `chmodSync` fallback.
   This is correct practice.
8. **Local token expiry check** — `loadAuth()` checks `expiresAt` before
   returning, preventing obviously-expired tokens from hitting the network.

### Negative Patterns

1. **Service/command duplication** — hook management logic exists in both a
   service and a command without the command using the service.
2. **Spelling inconsistency** — British/American spelling varies:
   `analyser`/`analyzer`, `initialise`/`initialize`, `customise`/`customize`.
3. **God object tendency** — `policy.ts` at 1676 lines is doing too much. The
   embedded Rego templates, bundle management, and policy CRUD could be separate
   modules.
4. **Inconsistent API response validation** — Some code paths validate API
   responses before use (`login.ts:50`), while others blindly cast
   (`auth-client.ts:27`, `admin-client.ts:67`).

---

## Recommendations Priority

| Priority | Action                                                                |
| -------- | --------------------------------------------------------------------- |
| P0       | Add try/catch around `package.json` parse at startup (H3)             |
| P1       | Add Zod validation for `config.yml` parsing (H1)                      |
| P1       | Add Zod validation for API responses in auth-client/admin-client (M2) |
| P1       | Delete duplicate `historical-analyzer.ts` (M3)                        |
| P1       | Consolidate hook scripts into single source of truth (M4)             |
| P2       | Validate `--output` paths stay within workspace (M8, M9)              |
| P2       | Add `parseInt` validation for numeric CLI options (M6)                |
| P2       | Replace `execSync` with `execFileSync` in doctor checks (H2)          |
| P3       | Standardize error output to always use `utils/output.ts` (L2)         |
| P3       | Split `policy.ts` into smaller modules (Architectural)                |
