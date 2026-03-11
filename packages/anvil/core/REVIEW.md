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

| Severity | Count                        |
| -------- | ---------------------------- |
| CRITICAL | 0                            |
| HIGH     | ~~2~~ (fixed)                |
| MEDIUM   | ~~6~~ (fixed), 1 in progress |
| LOW      | ~~5~~ (fixed)                |

---

## CRITICAL Issues

None.

---

## HIGH Issues

### ~~H1. Shell injection surface via `promisify(exec)` in provenance collector~~ ✅

**File:** `src/provenance/collector.ts:21, 96-118, 133, 225, 301`

**Fixed (2026-03-11):** All git calls now use `gitExec` wrapper which delegates
to `execFile` with array arguments. No shell interpolation remains.

---

### ~~H2. Shell commands in drift and git-notes modules use `exec` not `execFile`~~ ✅

**Files:**

- `src/drift/snapshot-capture.ts:24, 43`
- `src/provenance/git-ai-standard/git-notes.ts:9, 70, 104, 124, 157, 192, 225, 247, 273, 313`

**Fixed (2026-03-11):** All git calls now use `gitExec` wrapper which delegates
to `execFile` with array arguments. Existing validation functions retained as
defence-in-depth.

---

## MEDIUM Issues

### ~~M1. Provenance store uses record.id directly in filenames without sanitisation~~ ✅

**File:** `src/provenance/store.ts:106, 143, 270`

**Fixed (2026-03-11):** `sanitizeIdentifier()` applied to all record ID inputs,
matching the pattern from `drift/snapshot-storage.ts`.

---

### ~~M2. `canonicalizeJSON` has inconsistent `undefined` handling~~ ✅

**File:** `src/crypto/hash.ts:36-38, 64-67`

**Fixed (2026-03-11):** Top-level `undefined` now throws `TypeError` instead of
returning a misleading string, matching `JSON.stringify` semantics.

---

### ~~M3. `ProvenanceStore.clear()` writes empty strings instead of deleting files~~ ✅

**File:** `src/provenance/store.ts:266-281`

**Fixed (2026-03-11):** `clear()` now uses `unlinkSync` to delete record files
instead of writing empty strings.

---

### ~~M4. Package header claims "NO I/O operations" — inaccurate~~ ✅

**File:** `src/index.ts:8`

**Fixed (2026-03-11):** Comment updated to accurately describe the package's I/O
profile.

---

### ~~M5. `generatePlanId()` has only 32 bits of entropy~~ ✅

**File:** `src/crypto/hash.ts:102-107`

**Fixed (2026-03-11):** Now uses `randomBytes(8)` (64 bits of entropy).

---

### ~~M6. `entry-detector.ts` has unguarded JSON parsing and unbounded recursion~~ ✅

**File:** `src/architecture/entry-detector.ts`

**Fixed (2026-03-11):** `MAX_EXPORTS_DEPTH` guard added to `checkExports`
recursive traversal.

---

### M7. No file locking in suppression store — concurrent write race condition (fix in progress)

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

**Note (2026-03-11):** Fix in progress — being addressed in Unit 1 of the
security review backlog.

---

## LOW Issues

### ~~L1. Pervasive silent error swallowing across architecture modules~~ ✅

**Files:**

- `src/architecture/analyzer.ts` — `collectSourceFiles()` ignores all fs errors
- `src/architecture/edge-detector.ts` — returns empty array on read failure
- `src/architecture/entry-detector.ts` — silently ignores parse errors
- `src/drift/snapshot-capture.ts` — silently drops unreadable files

**Fixed (2026-03-11):** Debug logging added to all silent catch blocks via the
`debug` utility.

---

### ~~L2. Architecture analyzer violations detection is a placeholder~~ ✅

**File:** `src/architecture/analyzer.ts`

**Fixed (2026-03-11):** Real violation detection implemented — no longer returns
an empty array.

---

### ~~L3. `expandLineRanges` silently ignores malformed input~~ ✅

**File:** `src/provenance/git-ai-standard/serializer.ts:179-194`

**Fixed (2026-03-11):** Validates parsed values with `Number.isInteger()` before
pushing into the output array.

---

### ~~L4. `getAuthorshipStats` processes commits sequentially~~ ✅

**File:** `src/provenance/git-ai-standard/git-notes.ts:311-340`

**Fixed (2026-03-11):** Commit processing batched instead of sequential
per-commit shell commands.

---

### ~~L5. Debug utility has no log sanitisation~~ ✅

**File:** `src/utils/debug.ts`

**Fixed (2026-03-11):** `sanitizeForLog` helper added that redacts sensitive
patterns (tokens, passwords, API keys) from debug output.

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

| Priority | Action                                                               | Status      |
| -------- | -------------------------------------------------------------------- | ----------- |
| P0       | Migrate all `exec` calls to `execFile` with array arguments (H1, H2) | ✅ Fixed    |
| P1       | Add path sanitisation to `ProvenanceStore.get()` and `clear()` (M1)  | ✅ Fixed    |
| P1       | Fix `ProvenanceStore.clear()` to actually delete files (M3)          | ✅ Fixed    |
| P1       | Add file locking or atomic writes to stores (M7)                     | In progress |
| P2       | Increase `generatePlanId()` entropy to 64+ bits (M5)                 | ✅ Fixed    |
| P2       | Add depth limit to `entry-detector.ts` recursive traversal (M6)      | ✅ Fixed    |
| P2       | Fix `canonicalizeJSON` undefined handling (M2)                       | ✅ Fixed    |
| P2       | Update package header to reflect actual I/O usage (M4)               | ✅ Fixed    |
| P3       | Add debug logging for silently skipped files (L1)                    | ✅ Fixed    |
| P3       | Batch `getAuthorshipStats` commit processing (L4)                    | ✅ Fixed    |
| P3       | Add input validation to `expandLineRanges` (L3)                      | ✅ Fixed    |
