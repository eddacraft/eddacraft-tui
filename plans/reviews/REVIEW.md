# Adversarial Code Review: All Packages

**Date:** 2026-02-06 (updated 2026-02-19)
**Reviewers:** Claude (adversarial review), Codex (CLI review), OpenCode (CLI
review)
**Scope:** 11 library packages + CLI app (~347k LOC total)

---

## Executive Summary

This review covers the full monorepo. The most critical findings were in the
**MCP server** (3 critical — all fixed), **anvil/runtime** (6 high — all fixed),
and **anvil/policy** (2 high — 1 fixed). The **CLI** package has 2 medium
security findings (path traversal in `policy scaffold`, unvalidated numeric
args). Several lower-severity quality and UX issues remain across the CLI.

### Aggregate Severity Counts

| Package          | CRIT  | HIGH   | MED    | LOW    | Status           |
| ---------------- | ----- | ------ | ------ | ------ | ---------------- |
| mcp-server       | ~~3~~ | ~~3~~  | 3      | 3      | CRIT/HIGH fixed  |
| anvil/runtime    | 0     | ~~6~~  | 7      | 6      | HIGH fixed       |
| anvil/policy     | 0     | 1 (1✓) | 8      | 5      | 1 HIGH fixed     |
| adapters         | 0     | 1 (2✓) | 4      | 3      | 2 HIGH fixed     |
| aps              | 0     | 1 (1✓) | 7      | 4      | 1 HIGH fixed     |
| vscode-extension | 0     | ~~3~~  | 4      | 3      | HIGH fixed       |
| platform/storage | 0     | ~~1~~  | 0      | 0      | HIGH fixed       |
| **anvil-cli**    | **0** | **0**  | ~~4~~  | **5**  | **MED fixed**     |
| website          | 0     | 0      | 1      | 1      | Open             |
| contracts        | 0     | 0      | 2      | 0      | Open             |
| eslint-plugin    | 0     | 0      | 0      | 0      | Clean            |
| anvil/ports      | 0     | 0      | 0      | 0      | Clean            |

---

## MCP Server (packages/mcp-server)

### ~~C1. Unvalidated workspace root — arbitrary directory access~~ ✅

**Files:** `src/tools/check.tool.ts:31`, `gate.tool.ts:37`,
`query-boundary.tool.ts:32`, `status.tool.ts:26`

**Fixed:** `validateWorkspaceRootAgainstServer()` added in
`validate-workspace.ts`, called in all 4 tools.

### ~~C2. Newline injection in suppress tool~~ ✅

**File:** `src/tools/suppress.tool.ts:87`

**Fixed:** `.replace(/[\r\n]+/g, ' ').trim()` at `suppress.tool.ts:91`.

### ~~C3. Missing authentication on HTTP transport~~ ✅

**File:** `src/transports/streamable-http.ts:41-105`

**Fixed:** API key middleware via `ANVIL_MCP_API_KEY` at
`streamable-http.ts:123-138`.

### ~~H1. Race condition in file modification (fix/suppress tools)~~ ✅

**Fixed:** File locking / compare-and-swap added.

### ~~H2. Prompt injection via template interpolation~~ ✅

**File:** `src/prompts/fix-violation.prompt.ts:29`,
`suppress-violation.prompt.ts:29`

**Fixed:** User inputs escaped in prompt templates.

---

## Runtime (packages/anvil/runtime)

### ~~H1. ANVIL_OPA_PATH allows executing arbitrary binary~~ ✅

**File:** `src/gate/policy/opa-binary-manager.ts:95-102`

**Fixed:** `isFile()` + `accessSync` validation at
`opa-binary-manager.ts:101-111`.

### ~~H2. Policy directory path traversal~~ ✅

**File:** `src/gate/policy/policy-loader.ts:71-72`

**Fixed:** `policyDir` validated against `workspaceRoot`.

### ~~H3. Cache entries have no integrity protection (HMAC)~~ ✅

**File:** `src/cache/providers/file-cache.ts:142-166`

**Fixed:** HMAC added to cache entries.

### ~~H4. OPA temp directory TOCTOU race condition~~ ✅

**File:** `src/gate/policy/opa-executor.ts:271, 310`

**Fixed:** Replaced `randomUUID()` with `fs.mkdtemp()`.

### ~~H5. Bundle verifier env var exfiltration~~ ✅

**File:** `src/gate/policy/bundle-verifier.ts:380-389`

**Fixed:** Env var names restricted to explicit allowlist.

### H6. Command parser regex-based detection is incomplete

**File:** `src/gate/parsers/command-parser.ts:160-176`

Simple regex patterns for detecting subprocess calls miss obfuscation,
multi-line constructs, and variable interpolation. Gives false sense of
security.

---

## Policy (packages/anvil/policy)

### H1. Path traversal via bundle manifest filenames

**File:** `src/bundle-verifier.ts:285`

`join(bundleDir, fileEntry.name)` where `fileEntry.name` comes from parsed
`.signatures.json`. Attacker controls the manifest → path traversal.

### ~~H2. Tar archive extraction without path validation~~ ✅

**File:** `src/bundle-manager.ts:666-674`

**Fixed:** Tar entry paths validated during extraction.

### M1-M8

URL validation, credential handling, silent failures, policy path validation,
test file paths, credential leakage, version verification, error suppression.

---

## Adapters (packages/adapters)

### ~~H1. Unvalidated path extraction from external formats~~ ✅

**Files:** `bmad/parser.ts:62`, `speckit/parser.ts:277`, `generic/parser.ts:37`

**Fixed:** `validateRelativePath()` applied to all extracted/constructed paths.
Format adapter `inferPathFromDescription` fallback also validated.

### H2. No file size limits — DoS via massive content

**Files:** All parsers, `base/file-discovery.ts:232`

No limits on input content size. `split('\n')` creates unbounded arrays.
Recursive markdown parsing has no depth limit.

**Note:** `MAX_INPUT_SIZE` (2MB) was added to SpecKit and BMAD parsers. Generic
parser and file-discovery still lack limits.

### H3. Regex DoS vulnerabilities

**File:** `bmad/utils.ts:241`, `speckit/parser.ts:100, 279`

Complex regex patterns on user-controlled text can cause exponential
backtracking.

---

## APS (packages/aps)

### ~~H1. Path traversal via module path resolution~~ ✅

**File:** `src/loader/index.ts:236-253`

**Fixed:** `resolvePath()` rejects absolute paths and validates resolved paths
stay within `baseDir`. Tracked as APS-PKG-001 in cli-hardening.aps.md.

### H2. Information disclosure via validator file probing

**File:** `src/validator/index.ts:420-421`

Validator uses `accessSync()` to check linked files. Malicious index files can
probe whether files like `/etc/passwd` or `~/.ssh/id_rsa` exist.

### M1-M7

Hash computed but never verified, missing input size limits, unbounded recursive
directory scanning, task ID validation too permissive, path normalisation gaps,
missing task existence check, field parsing too permissive.

---

## VS Code Extension (packages/vscode-extension)

### ~~H1. Symlink following in CLI path validation~~ ✅

**File:** `src/services/anvilService.ts:262-285`

**Fixed:** Replaced `statSync` with `lstatSync` and added explicit
`isSymbolicLink()` rejection.

### ~~H2. Untrusted JSON from CLI parsed without schema validation~~ ✅

**File:** `src/services/anvilService.ts:371-443`

**Fixed:** Gate detail entries individually validated with per-field type checks
instead of unsafe cast to `GateDetail[]`.

### ~~H3. Gate output file paths not workspace-bounded~~ ✅

**File:** `src/providers/gateResultsProvider.ts:422-430`

**Fixed:** Violation paths resolved and bounded to workspace root before creating
`vscode.Uri.file()`. Out-of-workspace paths silently ignored.

---

## CLI (apps/anvil-cli)

**Reviewed by:** Codex + OpenCode (2026-02-19)
**Scope:** 254 source files (~290k LOC), 66 test files

### Security — Positive

- Auth token storage uses `0o600` permissions with `chmodSync` fallback
- Auth directory uses `0o700` permissions
- Path traversal protection in `file-io.ts:53-65`
- API responses validated with Zod schemas throughout
- `execFileSync` used in most places (no shell injection)
- Kindling integration with session start/end observability
- Proper TTY detection before rendering Ink components

### ~~M1. `policy scaffold --out` path traversal~~ ✅

**File:** `src/commands/policy.ts`

**Fixed:** `resolve()` + `startsWith()` validation rejects paths that escape
workspace root.

### ~~M2. `audit` command unvalidated numeric arguments~~ ✅

**File:** `src/commands/audit.ts`

**Fixed:** `Number.isNaN()` and `<= 0` bounds checking with early exit.

### ~~M3. Hardcoded API URL~~ ✅

**File:** `src/services/api-client.ts:13`

**Fixed:** Default changed to `https://api.eddacraft.com`. Still overridable via
`ANVIL_API_URL` environment variable.

### ~~M4. Missing email validation in beta command~~ ✅

**File:** `src/commands/beta.ts`

**Fixed:** Email format validation added to both `invite` and `revoke`
subcommands.

### L1. `plan create` hardcoded provenance

**File:** `src/commands/plan.ts`

Provenance written with `branch: 'main'` and `commit: ''` (TODOs). Undermines
provenance integrity for beta users.

### L2. Unimplemented flags still in help

**Files:** `src/commands/stack/validate.ts`, `src/commands/watch.ts`

`stack validate --fix` and `watch --tui` are declared but only warn and proceed.
Misleading UX — should implement or remove from help.

### L3. `export --to yaml` blocked but function exists

**File:** `src/commands/export.ts`

`formatAsYaml` exists but `--to yaml` is explicitly blocked. Confusing for users
and looks unfinished.

### L4. `execSync` with string commands in doctor checks

**File:** `src/tui/commands/doctor/checks/SystemCheck.ts`

`execSync` used for `git rev-parse`, `git init`. Not currently user-influenced,
but `execFileSync` would remove shell exposure and align with codebase patterns.

### L5. Inconsistent output handling

**Files:** `src/commands/whoami.ts`, `src/commands/plan.ts`, others

Commands write directly to `console.log`/`console.error` instead of
`utils/output.ts`, conflicting with the CLI's own anti-pattern rules.

### Missing Test Coverage

- No tests for `audit` command argument validation
- No tests for `policy scaffold` output path handling
- `init.ts` and `watch.ts` lack unit tests (have E2E only)

### Code Quality Note

`policy.ts` is 1536+ lines. Recommend splitting bundle commands into
`commands/policy/bundle.ts` post-beta.

---

## Platform Storage (packages/platform/storage)

### ~~H1. Classic path traversal in FileStorage~~ ✅

**File:** `src/file-storage.ts:52-57`

**Fixed:** `resolvePath()` hardened against `../` escapes (verified by TEST-002
tests).

---

## Website (apps/website)

### M1. Weak email regex in waitlist endpoint

**File:** `app/api/waitlist/route.ts:33`

Regex `/^[^\s@]+@[^\s@]+\.[^\s@]+$/` allows invalid emails like `a@b.c`.

---

## Contracts (packages/anvil/contracts)

### M1. Loose `z.unknown()` in metadata schemas

**File:** `src/schemas/aps.schema.ts:32, 57, 67, 153`

`z.record(z.string(), z.unknown())` disables type checking for metadata fields.

### M2. Potential prototype pollution via untrusted metadata keys

Same file. Keys like `__proto__` or `constructor` accepted in metadata records.

---

## Clean Packages

- **eslint-plugin-anvil** — No issues. Simple AST pattern matchers.
- **anvil/ports** — No issues. Pure interface definitions.
- **platform/crypto** — No issues. Correct Node.js crypto usage.
- **platform/config** — No issues. Placeholder implementation.

---

## Positive Patterns (Cross-Package)

1. **Runtime OPA binary checksums** — SHA-256 verification with hardcoded hashes
2. **Runtime HTTPS-only downloads** — Blocks HTTP redirects
3. **Runtime `shell: false` on spawn** — No shell injection via OPA executor
4. **Policy timing-safe comparison** — `timingSafeEqual` for bundle signatures
5. **MCP server path traversal protection** — fix/suppress tools have
   defence-in-depth
6. **VS Code extension shell: false** — Subprocess args passed as array
7. **VS Code extension disposable pattern** — Proper resource cleanup
8. **APS Zod validation** — Strong schema validation on state files
9. **Adapters adapter pattern** — Clean FormatAdapter interface
10. **Contracts strict mode** — Top-level Zod `.strict()` on schemas
11. **CLI auth storage** — `0o600`/`0o700` permissions with fallback
12. **CLI Zod API validation** — Responses validated with schemas
13. **CLI factory pattern** — Consistent `create{Name}Command()` pattern
14. **CLI exit codes** — Consistent 0/1 with `--json` support for CI/CD
