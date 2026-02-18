# Beta Testing Improvements

**Status:** In Progress
**Owner:** Engineering
**Priority:** High

## Context

Beta users on Windows, macOS, and Linux are about to use Anvil. This plan
identifies test gaps and cross-platform issues that need attention before
and during the beta period.

---

## Module: TEST-GAPS — Critical Test Coverage Gaps

### TEST-001: Add parse-task.ts unit tests

**Intent:** The APS task parser (`packages/aps/src/parser/parse-task.ts`, 275
lines) has zero test coverage. Every plan task creation flows through
`parseTaskHeading()`, `parseTaskFields()`, `assignField()`, and `parseTask()`.

**Expected Outcome:** Tests covering all 14 field types (Intent, ExpectedOutcome,
Validation, Confidence, Scopes, NonScope, Files, Tags, Dependencies, Risks,
Packages, Link, Status, Inputs), heading regex validation, error paths for
missing Intent, malformed headings, and edge cases (empty values, inline vs
list Inputs).

**Confidence:** High
**Status:** completed
**Tags:** testing, aps, parser
**Files:** packages/aps/src/parser/parse-task.ts

---

### TEST-002: Add file-storage.ts tests (security-critical)

**Intent:** `FileStorage.resolvePath()` at
`packages/platform/storage/src/file-storage.ts:52-58` is the only guard against
directory traversal. Every file operation in Anvil goes through this class. Zero
tests exist.

**Expected Outcome:** Tests for: path escaping detection (`../../../etc/passwd`),
normal relative paths, absolute paths, Windows-style paths (`..\\..\\`), the
base-directory equality edge case, and all CRUD methods (read, write, exists,
delete, list, mkdir).

**Confidence:** High
**Status:** completed
**Tags:** testing, security, storage
**Files:** packages/platform/storage/src/file-storage.ts

---

### TEST-003: Add hook-installer.ts tests

**Intent:** `HookInstaller` (229 lines,
`apps/anvil-cli/src/services/hook-installer.ts`) runs during `anvil init` — the
first thing every beta user does. `installHook()`, `uninstallHook()`,
`backupExistingHook()`, `loadHookScript()`, `isAnvilManagedHook()` are all
untested.

**Expected Outcome:** Tests for: hook installation (creates file, sets marker,
correct permissions), uninstallation (only Anvil-managed hooks), backup of
existing non-Anvil hooks, script loading with fallback to embedded scripts,
error when hook scripts directory not found.

**Confidence:** High
**Status:** completed
**Tags:** testing, cli, hooks
**Files:** apps/anvil-cli/src/services/hook-installer.ts

---

### TEST-004: Add architecture analyzer.ts tests

**Intent:** `ArchitectureAnalyzer.analyse()` at
`packages/anvil/core/src/architecture/analyzer.ts` (385 lines) is the main entry
point for architecture analysis — a primary Anvil feature. File collection,
filtering, baseline comparison, violation classification are all untested.

**Expected Outcome:** Tests for: basic analysis with sample project structure,
file filtering with include/exclude patterns, baseline creation and comparison,
violation classification (new vs existing), error handling for I/O failures.

**Confidence:** High
**Status:** completed
**Tags:** testing, core, architecture
**Files:** packages/anvil/core/src/architecture/analyzer.ts

---

### TEST-005: Add yaml-parser.ts and definition-schema.ts tests

**Intent:** `parseArchitectureDefinition()` and `writeArchitectureYaml()` at
`packages/anvil/core/src/architecture/yaml-parser.ts` (261 lines) plus
`validateArchitectureDefinition()` and all 9 template definitions at
`definition-schema.ts` (117 lines) have zero tests.

**Expected Outcome:** Tests for: valid YAML parsing, malformed YAML errors,
schema validation failures, all 9 template types (STARTER through CUSTOM),
template merging with user overrides, default options application, round-trip
write/read.

**Confidence:** High
**Status:** completed
**Tags:** testing, core, architecture, config
**Files:** packages/anvil/core/src/architecture/yaml-parser.ts, packages/anvil/core/src/architecture/definition-schema.ts

---

### TEST-006: Add concurrency lock-manager tests

**Intent:** The entire `packages/anvil/runtime/src/concurrency/` directory (7
files including lock-manager.ts, atomic.ts, queue-manager.ts) has zero test
coverage. Multi-agent coordination depends on correct locking behavior.

**Expected Outcome:** Tests for: lock acquisition and release, lock contention
(two agents acquiring same lock), stale lock detection and cleanup, atomic
JSON writes, queue manager ordering, timeout behavior.

**Confidence:** Medium
**Status:** completed
**Tags:** testing, runtime, concurrency
**Files:** packages/anvil/runtime/src/concurrency/lock-manager.ts, packages/anvil/runtime/src/concurrency/atomic.ts, packages/anvil/runtime/src/concurrency/queue-manager.ts

---

### TEST-007: Add config loader and core config tests

**Intent:** `packages/platform/config/src/loader.ts` and
`packages/anvil/core/src/config/` (5 files) have zero test coverage. Config
loading is used everywhere.

**Expected Outcome:** Tests for: loading from file, environment variable
overrides, missing config file handling, malformed config, default values.

**Confidence:** Medium
**Status:** Open
**Tags:** testing, config
**Files:** packages/platform/config/src/loader.ts, packages/anvil/core/src/config/

---

### TEST-008: Add anvil init error path tests

**Intent:** The init command (`apps/anvil-cli/src/commands/init.ts`) has
happy-path tests but no coverage for: architecture analysis failure
(line 190-197), smart defaults failure, TUI fallback when terminal detection
fails, hook installation errors, config file write failures.

**Expected Outcome:** Error-case tests for each failure point in the init flow.
Verify graceful degradation and useful error messages.

**Confidence:** Medium
**Status:** Open
**Tags:** testing, cli, init
**Files:** apps/anvil-cli/src/commands/init.ts

---

## Module: XPLAT — Cross-Platform Issues

### XPLAT-001: Fix pre-push.sh find command and quoting

**Intent:** `apps/anvil-cli/scripts/pre-push.sh:12` has ambiguous `find` OR
grouping (missing parentheses) and unquoted `$PLAN_FILES` on line 17.

**Expected Outcome:** `find` command uses explicit `\( \)` grouping. Loop uses
`while IFS= read -r` instead of unquoted `for file in $PLAN_FILES`.

**Confidence:** High
**Status:** completed
**Tags:** cross-platform, shell, hooks
**Files:** apps/anvil-cli/scripts/pre-push.sh, apps/anvil-cli/src/services/hook-installer.ts

---

### XPLAT-002: Replace UTF-8 symbols in shell scripts

**Intent:** `pre-commit.sh:13` and `pre-push.sh:21,30` use ✓ and ✗ characters
that render as garbage on Windows cmd.exe and PowerShell with non-UTF-8
codepages.

**Expected Outcome:** Replace `✓` with `[OK]` and `✗` with `[FAIL]` or similar
ASCII alternatives. Update both the script files AND the embedded copies in
`hook-installer.ts:92-146`.

**Confidence:** High
**Status:** completed
**Tags:** cross-platform, windows, hooks
**Files:** apps/anvil-cli/scripts/pre-commit.sh, apps/anvil-cli/scripts/pre-push.sh, apps/anvil-cli/src/services/hook-installer.ts

---

### XPLAT-003: Windows hook installation — warn or provide alternatives

**Intent:** `HookInstaller.installHook()` writes shell scripts with `#!/bin/sh`
shebangs. On Windows without Git Bash, these hooks silently do nothing. Only
`welcome.ts:94` has Windows platform detection in the entire CLI.

**Expected Outcome:** During `anvil init`, detect Windows and either: (a) warn
the user that hooks require Git Bash, (b) generate PowerShell hook equivalents,
or (c) skip hook installation with a clear message. Add `process.platform`
check in `HookInstaller`.

**Confidence:** Medium
**Status:** completed
**Tags:** cross-platform, windows, hooks
**Files:** apps/anvil-cli/src/services/hook-installer.ts, apps/anvil-cli/src/commands/init.ts

---

### XPLAT-004: Add CI matrix for macOS and Windows

**Intent:** `.github/workflows/ci.yml` runs only on `ubuntu-latest`. Shell
script issues and platform-specific Node.js behavior won't be caught until
beta users report them.

**Expected Outcome:** Add `macos-latest` and `windows-latest` to the `runs-on`
matrix for at least the unit test and build jobs. Gate the shell script tests
to only run on Linux/macOS.

**Confidence:** Medium
**Status:** completed
**Tags:** cross-platform, ci
**Files:** .github/workflows/ci.yml

---

### XPLAT-005: Architecture template glob patterns and Windows paths

**Intent:** Architecture templates in `yaml-parser.ts` use hardcoded forward
slashes in glob patterns (`src/controllers/**`). While `minimatch` handles
this, any code path that passes these patterns to raw `fs` operations or
string matching against `path.sep`-separated paths will fail on Windows.

**Expected Outcome:** Audit all glob pattern consumers to ensure they normalize
separators. Add a test that verifies template patterns match correctly on
Windows-style paths.

**Confidence:** Medium
**Status:** Done
**Tags:** cross-platform, windows, architecture
**Files:** packages/anvil/core/src/architecture/yaml-parser.ts, packages/anvil/core/src/architecture/layer-detector.ts

---

## Summary

| ID | Title | Priority | Status |
|----|-------|----------|--------|
| TEST-001 | parse-task.ts tests | P0 | Done |
| TEST-002 | file-storage.ts tests (security) | P0 | Done |
| TEST-003 | hook-installer.ts tests | P0 | Done |
| TEST-004 | analyzer.ts tests | P1 | Done |
| TEST-005 | yaml-parser + schema tests | P1 | Done |
| TEST-006 | concurrency lock-manager tests | P1 | Done |
| TEST-007 | config loader tests | P2 | Open |
| TEST-008 | init error path tests | P2 | Open |
| XPLAT-001 | Fix pre-push.sh find/quoting | P0 | Done |
| XPLAT-002 | Replace UTF-8 symbols | P0 | Done |
| XPLAT-003 | Windows hook installation | P1 | Done |
| XPLAT-004 | CI matrix for macOS/Windows | P1 | Done |
| XPLAT-005 | Glob patterns and Windows paths | P2 | Done |
