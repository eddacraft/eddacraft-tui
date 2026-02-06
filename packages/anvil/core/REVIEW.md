# Adversarial Code Review: packages/anvil/core

**Date:** 2026-02-06 **Reviewer:** Claude (automated adversarial review)
**Scope:** Full `packages/anvil/core` codebase (~144 TypeScript files across 11
modules)

---

## Executive Summary

The anvil-core package is labelled as "pure domain logic with NO I/O
operations", but in practice at least 8 modules perform direct filesystem reads,
writes, or shell command execution. Security-wise the codebase is mixed: the
`drift/snapshot-storage` module has exemplary path traversal protection, and
`git-notes.ts` has proper input validation for git refs — but
`provenance/collector.ts` executes 7+ shell commands per invocation via the
unsafe `promisify(exec)` pattern, and `provenance/store.ts` uses unsanitised
record IDs in file paths. The review identified **2 high-severity concerns**
(shell injection surface area), **7 medium-severity issues** (path traversal,
inconsistent hashing, race conditions), and **5 low-severity items**.

### Severity Counts

| Severity | Count |
| -------- | ----- |
| CRITICAL | 0     |
| HIGH     | 2     |
| MEDIUM   | 7     |
| LOW      | 5     |

---

## CRITICAL Issues

None.

---

## HIGH Issues

### H1. Shell injection surface via `promisify(exec)` in provenance collector

**File:** `src/provenance/collector.ts:21, 96-118, 133, 225, 301`

```typescript
const execAsync = promisify(exec);
// ...
const [branch, commit, ...] = await Promise.all([
  execAsync('git rev-parse --abbrev-ref HEAD', { cwd: workspaceRoot }),
  execAsync('git rev-parse HEAD', { cwd: workspaceRoot }),
  execAsync('git log -1 --format=%s', { cwd: workspaceRoot }),
  execAsync('git log -1 --format=%an <%ae>', { cwd: workspaceRoot }),
  execAsync('git status --porcelain', { cwd: workspaceRoot }),
  execAsync('git diff --name-only --cached', { cwd: workspaceRoot }),
]);
```

`promisify(exec)` passes every command through a shell interpreter. The
`collectGitContext` function alone fires 6 parallel shell commands, with
additional ones in `detectAITool` (line 225) and `createProvenanceRecord` (line
301). While the command strings themselves are hardcoded and `workspaceRoot` is
only used as `cwd`, this is a large shell surface area. If any future refactor
interpolates user-supplied data into these strings, it becomes an injection
vector.

This is inconsistent with the CLI layer (`apps/anvil-cli`) which correctly uses
`execFileSync` with array arguments throughout.

**Recommendation:** Replace all `promisify(exec)` calls with
`promisify(execFile)` and pass arguments as arrays. Example:

```typescript
const execFileAsync = promisify(execFile);
const { stdout } = await execFileAsync(
  'git',
  ['rev-parse', '--abbrev-ref', 'HEAD'],
  { cwd: workspaceRoot }
);
```

---

### H2. Shell commands in drift and git-notes modules use `exec` not `execFile`

**Files:**

- `src/drift/snapshot-capture.ts:24, 43`
- `src/provenance/git-ai-standard/git-notes.ts:9, 70, 104, 124, 157, 192, 225, 247, 273, 313`

```typescript
// snapshot-capture.ts
const execAsync = promisify(exec);
const { stdout } = await execAsync('git rev-parse HEAD', {
  cwd: workspaceRoot,
});

// git-notes.ts
await execAsync(
  `git notes --ref=${NOTES_REF} add -f -F "${tempFile}" -- ${commitSha}`,
  {
    cwd: workspaceRoot,
  }
);
```

The `git-notes.ts` module has proper input validation functions
(`isValidGitRef`, `isValidRemoteName`, `isValidRevisionRange`) that whitelist
allowed characters — this is good. However, it still uses `execAsync` (shell
mode) with string interpolation. The validation layer is defense-in-depth
against injection but using `execFile` with array arguments would make the
validation a belt-and-suspenders approach rather than the sole protection.

**Recommendation:** Migrate to `execFile` with arguments array. The existing
validation functions should be retained as an additional layer.

---

## MEDIUM Issues

### M1. Provenance store uses record.id directly in filenames without sanitisation

**File:** `src/provenance/store.ts:106, 143, 270`

```typescript
// save()
const recordPath = join(this.historyDir, `${record.id}.json`);
writeFileSync(recordPath, JSON.stringify(record, null, 2));

// get(id) — accepts arbitrary string
const recordPath = join(this.historyDir, `${id}.json`);
```

The `save()` method uses `record.id` (formatted as `prov-${randomUUID()}`, which
is safe) but the `get(id)` method accepts an arbitrary string from callers. If
`id` contains `../`, this allows path traversal to read any JSON file on disk.

Contrast with `drift/snapshot-storage.ts` which has explicit
`sanitizeSnapshotIdentifier()` protection (lines 50-66). The provenance store
lacks equivalent protection.

**Recommendation:** Add `path.basename()` validation (matching the pattern in
`snapshot-storage.ts`) or validate the `id` format with a regex before
constructing the file path.

---

### M2. `canonicalizeJSON` has inconsistent `undefined` handling

**File:** `src/crypto/hash.ts:36-38, 64-67`

```typescript
// Top-level undefined → returns literal string "undefined"
if (obj === undefined) {
  return 'undefined';
}

// But inside objects, undefined values are correctly skipped:
if (value === undefined) {
  return null; // filtered out
}
```

At the top level, `canonicalizeJSON(undefined)` returns the string
`"undefined"`, but an object like `{a: undefined}` produces `{}` (matching
`JSON.stringify` behavior). This inconsistency means:

- `canonicalizeJSON(undefined)` → `"undefined"`
- `canonicalizeJSON({a: undefined})` → `{}`
- Standard `JSON.stringify(undefined)` → `undefined` (the value, not a string)

While unlikely to be triggered in practice (callers typically pass objects),
this breaks the contract that canonicalization is a deterministic superset of
`JSON.stringify`.

---

### M3. `ProvenanceStore.clear()` writes empty strings instead of deleting files

**File:** `src/provenance/store.ts:266-281`

```typescript
clear(): void {
  const index = this.loadIndex();
  for (const record of index.records) {
    const recordPath = join(this.historyDir, `${record.id}.json`);
    try {
      if (existsSync(recordPath)) {
        writeFileSync(recordPath, '');  // ← writes empty string, does not delete
      }
    } catch (error) {
      debug(`Failed to clear record file ${record.id}`, error);
    }
  }
  this.saveIndex(this.createEmptyIndex());
}
```

This leaves zero-byte `.json` files on disk. The `get()` method handles this
gracefully (the `JSON.parse('')` throws, caught and returns `null`), but:

1. It wastes disk space (up to 1000 empty files if MAX_RECORDS reached)
2. It leaves artifacts visible in directory listings
3. Files that were only partially in the index (from concurrent writes) are
   missed

**Recommendation:** Use `unlinkSync` to actually delete the files, or use
`fs.rm(historyDir, { recursive: true })` followed by `ensureDirectories()`.

---

### M4. Package header claims "NO I/O operations" — inaccurate

**File:** `src/index.ts:8`

```typescript
* This package has NO I/O operations - all I/O is handled by @eddacraft/anvil-runtime.
```

Modules that perform direct I/O:

| Module                           | I/O Type                                                                          |
| -------------------------------- | --------------------------------------------------------------------------------- |
| `provenance/collector.ts`        | Shell commands (`exec`), `readFileSync`, `existsSync`                             |
| `provenance/store.ts`            | `readFileSync`, `writeFileSync`, `mkdirSync`, `existsSync`                        |
| `drift/snapshot-capture.ts`      | `fs.readFile`, `exec`                                                             |
| `drift/snapshot-storage.ts`      | `fs.readFile`, `fs.writeFile`, `fs.mkdir`, `fs.readdir`, `fs.unlink`, `fs.access` |
| `architecture/analyzer.ts`       | `readdirSync`, `statSync`                                                         |
| `architecture/edge-detector.ts`  | `readFileSync`                                                                    |
| `architecture/entry-detector.ts` | `readFileSync`, `existsSync`                                                      |
| `architecture/yaml-parser.ts`    | `readFile`, `writeFile`, `existsSync`                                             |
| `suppression/store.ts`           | `fs.readFile`, `fs.writeFile`, `fs.mkdir`                                         |

This is a documentation/architecture violation rather than a security bug, but
it misleads consumers about the package's dependency profile and testability.

**Recommendation:** Either move I/O to `@eddacraft/anvil-runtime` (as the header
implies), or update the documentation to accurately describe the package.

---

### M5. `generatePlanId()` has only 32 bits of entropy

**File:** `src/crypto/hash.ts:102-107`

```typescript
export function generatePlanId(): string {
  const buffer = randomBytes(4); // 4 bytes = 32 bits
  const hexString = buffer.toString('hex');
  return `aps-${hexString}`;
}
```

4 random bytes = 2^32 ≈ 4.3 billion possibilities. By the birthday paradox,
collision probability reaches 50% at approximately √(2^32) ≈ 65,536 IDs. For
plan IDs that need to be unique across a team, repository, or organization, this
may be insufficient.

The `provenance/collector.ts:24` correctly uses `randomUUID()` (128 bits),
showing the project already has the right pattern available.

**Recommendation:** Increase to `randomBytes(8)` (64 bits, collision at ~4
billion) or use `randomUUID()` for consistency.

---

### M6. `entry-detector.ts` has unguarded JSON parsing and unbounded recursion

**File:** `src/architecture/entry-detector.ts`

The `checkExports` function recursively traverses JSON structures without a
depth limit. A crafted `package.json` with deeply nested exports could cause
stack overflow:

```json
{ "exports": { ".": { ".": { ".": { "...500 levels...": "./index.js" } } } } }
```

Additionally, some `JSON.parse(readFileSync(...))` calls lack try-catch
protection in production code paths, and could crash on malformed `package.json`
files.

**Recommendation:** Add a depth limit to recursive traversal (e.g., max 10
levels) and wrap all JSON parse calls in try-catch.

---

### M7. No file locking in suppression store — concurrent write race condition

**File:** `src/suppression/store.ts:40-71`

```typescript
async load(): Promise<void> {
  const content = await fs.readFile(this.filePath, 'utf-8');
  // ...
}

async save(): Promise<void> {
  await fs.writeFile(this.filePath, JSON.stringify(this.data, null, 2), 'utf-8');
}
```

There is no file locking mechanism. If multiple processes (e.g., git pre-commit
hook + `anvil watch` mode) call `load()` → modify → `save()` concurrently, one
write can overwrite the other's changes (lost update). The same issue applies to
`provenance/store.ts`.

**Recommendation:** Use `proper-lockfile` or `fd-lock`, or implement atomic
writes via write-to-temp-then-rename.

---

## LOW Issues

### L1. Pervasive silent error swallowing across architecture modules

**Files:**

- `src/architecture/analyzer.ts` — `collectSourceFiles()` ignores all fs errors
- `src/architecture/edge-detector.ts` — returns empty array on read failure
  (line 115)
- `src/architecture/entry-detector.ts` — silently ignores parse errors (lines
  186, 244)
- `src/drift/snapshot-capture.ts` — silently drops unreadable files (line 225)

Multiple modules catch errors and either return defaults or skip items without
any logging or user notification. This makes debugging difficult: if a file
can't be read due to permissions or encoding issues, the analysis silently
produces incomplete results.

**Recommendation:** At minimum, log skipped files via the `debug` utility that
exists in the codebase. Consider propagating read errors as warnings in the scan
results.

---

### L2. Architecture analyzer violations detection is a placeholder

**File:** `src/architecture/analyzer.ts`

The violations detection returns an empty array — the implementation is
incomplete. This means architecture analysis reports zero violations regardless
of actual boundary crossings, giving a false sense of compliance.

---

### L3. `expandLineRanges` silently ignores malformed input

**File:** `src/provenance/git-ai-standard/serializer.ts:179-194`

```typescript
export function expandLineRanges(ranges: string): number[] {
  for (const part of ranges.split(',')) {
    if (part.includes('-')) {
      const [start, end] = part.split('-').map(Number);
      for (let i = start; i <= end; i++) {
        lines.push(i);
      }
    } else {
      lines.push(Number(part));
    }
  }
```

If `ranges` contains non-numeric strings (e.g., `"abc-def"`), `Number()` returns
`NaN`. The for-loop `for (let i = NaN; NaN <= NaN; ...)` never executes (safe)
but NaN values from non-range parts are pushed into the output array (line 189).
No error is reported.

---

### L4. `getAuthorshipStats` processes commits sequentially

**File:** `src/provenance/git-ai-standard/git-notes.ts:311-340`

Each commit in a revision range is checked one-by-one with a separate shell
command. For ranges with hundreds of commits, this creates hundreds of child
processes sequentially, which is very slow.

**Recommendation:** Batch note lookups or use `git notes list` to get all note
references in one command, then filter locally.

---

### L5. Debug utility has no log sanitisation

**File:** `src/utils/debug.ts`

The debug logger outputs arguments directly to console without sanitising
sensitive data. If a caller passes tokens, passwords, or API keys as debug
arguments, these appear in the output when `DEBUG=*` is set.

---

## Architectural Observations

### Positive Patterns

1. **Exemplary path traversal protection** in `drift/snapshot-storage.ts` —
   `sanitizeSnapshotIdentifier()` uses `path.basename()`, validates no directory
   separators, and checks for `.` / `..` / null bytes.
2. **Git ref validation** in `git-notes.ts` — `isValidGitRef()`,
   `isValidRemoteName()`, `isValidRevisionRange()` provide regex-based input
   whitelisting.
3. **Zod schema validation** consistently applied on data load in
   `snapshot-storage.ts`, `provenance/store.ts`, `suppression/store.ts`,
   `yaml-parser.ts`.
4. **Timing-safe hash comparison** in `crypto/hash.ts` using `timingSafeEqual`.
5. **Temp file cleanup** in `git-notes.ts` — writes to OS temp directory with
   UUID names and cleans up in `finally` block.
6. **Commit SHA validation** in `createAuthorshipLog` — requires full 40-char
   hex SHA.

### Negative Patterns

1. **Inconsistent exec patterns** — `git-notes.ts` validates inputs then uses
   shell exec; `collector.ts` uses shell exec with no validation; the CLI uses
   `execFileSync` with arrays. Three different security postures for the same
   operation.
2. **I/O leaking into "pure" package** — 9 of 11 modules perform direct I/O
   despite the package header claiming otherwise.
3. **Inconsistent path safety** — `snapshot-storage.ts` has proper sanitisation,
   `provenance/store.ts` has none. Two modules in the same package with opposite
   security approaches.
4. **Silent failures everywhere** — architecture analysis modules return
   empty/default results on errors, making it impossible to distinguish "no
   issues found" from "couldn't read the files".

---

## Recommendations Priority

| Priority | Action                                                               |
| -------- | -------------------------------------------------------------------- |
| P0       | Migrate all `exec` calls to `execFile` with array arguments (H1, H2) |
| P1       | Add path sanitisation to `ProvenanceStore.get()` and `clear()` (M1)  |
| P1       | Fix `ProvenanceStore.clear()` to actually delete files (M3)          |
| P1       | Add file locking or atomic writes to stores (M7)                     |
| P2       | Increase `generatePlanId()` entropy to 64+ bits (M5)                 |
| P2       | Add depth limit to `entry-detector.ts` recursive traversal (M6)      |
| P2       | Fix `canonicalizeJSON` undefined handling (M2)                       |
| P2       | Update package header to reflect actual I/O usage (M4)               |
| P3       | Add debug logging for silently skipped files (L1)                    |
| P3       | Batch `getAuthorshipStats` commit processing (L4)                    |
| P3       | Add input validation to `expandLineRanges` (L3)                      |
