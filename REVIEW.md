# Adversarial Code Review: Remaining Packages

**Date:** 2026-02-06
**Reviewer:** Claude (automated adversarial review)
**Scope:** 11 packages not covered by prior CLI/Core/API reviews

---

## Executive Summary

This review covers the remaining 11 packages (~57k LOC total). The most critical
findings are in the **MCP server** (3 critical: unvalidated workspace root, newline
injection, missing HTTP auth), **anvil/runtime** (6 high: OPA binary path override,
policy dir traversal, cache poisoning, temp dir TOCTOU, env var exfiltration,
command parser weakness), and **anvil/policy** (2 high: tarball path traversal,
bundle manifest path traversal). The **platform/storage** package has a classic
path traversal vulnerability, and the **adapters** and **aps** packages have
unvalidated path handling from external input.

### Aggregate Severity Counts

| Package | CRIT | HIGH | MED | LOW |
|---------|------|------|-----|-----|
| anvil/runtime | 0 | 6 | 7 | 6 |
| anvil/policy | 0 | 2 | 8 | 5 |
| mcp-server | 3 | 3 | 3 | 3 |
| adapters | 0 | 3 | 4 | 3 |
| aps | 0 | 2 | 7 | 4 |
| vscode-extension | 0 | 3 | 4 | 3 |
| platform/storage | 0 | 1 | 0 | 0 |
| website | 0 | 0 | 1 | 1 |
| contracts | 0 | 0 | 2 | 0 |
| eslint-plugin | 0 | 0 | 0 | 0 |
| anvil/ports | 0 | 0 | 0 | 0 |
| **TOTAL** | **3** | **20** | **36** | **25** |

---

## MCP Server (packages/mcp-server)

### C1. Unvalidated workspace root — arbitrary directory access

**Files:** `src/tools/check.tool.ts:31`, `gate.tool.ts:37`, `query-boundary.tool.ts:32`, `status.tool.ts:26`

Four tools accept `workspaceRoot` directly from MCP client requests without
validation. A malicious client can pass arbitrary absolute paths to analyse any
directory on the server filesystem. Response metadata leaks filesystem structure.

**Recommendation:** Validate `workspaceRoot` against a server-configured allowed root.

### C2. Newline injection in suppress tool

**File:** `src/tools/suppress.tool.ts:87`

The `reason` parameter is interpolated into a source file comment without escaping
newlines. A client can inject `reason: "valid\n*/ malicious_code(); /*"` to inject
arbitrary code into the source file.

**Recommendation:** Strip `\r\n` from `reason` before interpolation.

### C3. Missing authentication on HTTP transport

**File:** `src/transports/streamable-http.ts:41-105`

The HTTP transport has no authentication, no CORS validation, no rate limiting.
Any HTTP client can connect and invoke all MCP tools including file-modifying ones.

**Recommendation:** Add authentication (API keys/mutual TLS) and CORS restrictions.

### H1. Race condition in file modification (fix/suppress tools)

TOCTOU between read and write. Concurrent requests can overwrite each other's changes.

### H2. Prompt injection via template interpolation

**File:** `src/prompts/fix-violation.prompt.ts:29`, `suppress-violation.prompt.ts:29`

User inputs (`warningId`, `filePath`, `message`) are interpolated into prompts without
escaping. Malicious clients can inject instructions.

---

## Runtime (packages/anvil/runtime)

### H1. ANVIL_OPA_PATH allows executing arbitrary binary

**File:** `src/gate/policy/opa-binary-manager.ts:95-102`

The `ANVIL_OPA_PATH` environment variable is used directly without validation.
An attacker who can set env vars can point to a trojanised binary.

**Recommendation:** Validate path is a regular file (not symlink), has correct
permissions, or remove the env var override.

### H2. Policy directory path traversal

**File:** `src/gate/policy/policy-loader.ts:71-72`

`policyDir` from config is joined with `workspaceRoot` without validation.
A config value of `../../../etc/` loads `.rego` files from system directories.

### H3. Cache entries have no integrity protection (HMAC)

**File:** `src/cache/providers/file-cache.ts:142-166`

Cache entries are loaded and trusted without integrity verification. An attacker
with write access to `.anvil/cache/` can inject false pass/fail results.

### H4. OPA temp directory TOCTOU race condition

**File:** `src/gate/policy/opa-executor.ts:271, 310`

Temp dirs created with `randomUUID()` at `/tmp/anvil-opa-{uuid}`. Between creation
and population, another process could replace files. Use `fs.mkdtemp()` instead.

### H5. Bundle verifier env var exfiltration

**File:** `src/gate/policy/bundle-verifier.ts:380-389`

When `keyConfig.source === 'env'`, any env var name is accepted. An attacker who
controls the config can specify `AWS_SECRET_ACCESS_KEY` and exfiltrate values.

### H6. Command parser regex-based detection is incomplete

**File:** `src/gate/parsers/command-parser.ts:160-176`

Simple regex patterns for detecting subprocess calls miss obfuscation, multi-line
constructs, and variable interpolation. Gives false sense of security.

---

## Policy (packages/anvil/policy)

### H1. Path traversal via bundle manifest filenames

**File:** `src/bundle-verifier.ts:285`

`join(bundleDir, fileEntry.name)` where `fileEntry.name` comes from parsed
`.signatures.json`. Attacker controls the manifest → path traversal.

### H2. Tar archive extraction without path validation

**File:** `src/bundle-manager.ts:666-674`

Tarball extraction uses `tar` library with `{ cwd: destDir }` but no path
validation. Malicious tarballs with `../../` entries extract outside `destDir`.

### M1-M8. URL validation, credential handling, silent failures, policy path validation, test file paths, credential leakage, version verification, error suppression.

---

## Adapters (packages/adapters)

### H1. Unvalidated path extraction from external formats

**Files:** `bmad/parser.ts:62`, `speckit/parser.ts:277`, `generic/parser.ts:37`

File paths extracted from user-controlled markdown content without validation
for path traversal. Paths like `../../etc/passwd` flow into downstream operations.

### H2. No file size limits — DoS via massive content

**Files:** All parsers, `base/file-discovery.ts:232`

No limits on input content size. `split('\n')` creates unbounded arrays. Recursive
markdown parsing has no depth limit.

### H3. Regex DoS vulnerabilities

**File:** `bmad/utils.ts:241`, `speckit/parser.ts:100, 279`

Complex regex patterns on user-controlled text can cause exponential backtracking.

---

## APS (packages/aps)

### H1. Path traversal via module path resolution

**File:** `src/loader/index.ts:235-244`

`resolvePath()` passes absolute paths through untouched and doesn't validate
`../` sequences in relative paths. Malicious index files can read arbitrary files.

### H2. Information disclosure via validator file probing

**File:** `src/validator/index.ts:420-421`

Validator uses `accessSync()` to check linked files. Malicious index files can
probe whether files like `/etc/passwd` or `~/.ssh/id_rsa` exist.

### M1-M7. Hash computed but never verified, missing input size limits, unbounded recursive directory scanning, task ID validation too permissive, path normalization gaps, missing task existence check, field parsing too permissive.

---

## VS Code Extension (packages/vscode-extension)

### H1. Symlink following in CLI path validation

**File:** `src/services/anvilService.ts:262-285`

Uses `statSync()` (follows symlinks) instead of `lstatSync()`. Symlink to
malicious binary accepted as valid CLI path.

### H2. Untrusted JSON from CLI parsed without schema validation

**File:** `src/services/anvilService.ts:371-443`

Gate/validation output is `JSON.parse()`'d and spread into objects without Zod
validation. Compromised CLI output can inject arbitrary properties.

### H3. Gate output file paths not workspace-bounded

**File:** `src/providers/gateResultsProvider.ts:422-430`

Violation file paths from gate output are passed to `vscode.Uri.file()` without
checking they're within the workspace. Can open arbitrary files.

---

## Platform Storage (packages/platform/storage)

### H1. Classic path traversal in FileStorage

**File:** `src/file-storage.ts:52-57`

```typescript
private resolvePath(path: string): string {
  if (path.startsWith('/')) return path;        // absolute paths bypass baseDir
  return `${this.baseDir}/${path}`;             // no normalization of ../
}
```

Both absolute paths and `../` sequences escape the `baseDir` sandbox.

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
5. **MCP server path traversal protection** — fix/suppress tools have defence-in-depth
6. **VS Code extension shell: false** — Subprocess args passed as array
7. **VS Code extension disposable pattern** — Proper resource cleanup
8. **APS Zod validation** — Strong schema validation on state files
9. **Adapters adapter pattern** — Clean FormatAdapter interface
10. **Contracts strict mode** — Top-level Zod `.strict()` on schemas
