<!-- APS: See https://github.com/EddaCraft/anvil-plan-spec for format reference -->
<!-- Executable only if tasks exist and status is Ready. -->

# Save-time Trust

| Scope | Owner | Priority | Status   |
| ----- | ----- | -------- | -------- |
| CORE  | —     | high     | Complete |

## Purpose

Provide the on-save analysis runner that forms the foundation of Anvil's
developer feedback loop. This is the core engine that orchestrates checks and
delivers warnings at file-save time.

## In Scope

- On-save file analysis trigger
- Check orchestration and parallel execution
- Warning aggregation and formatting
- CLI entry point for manual runs
- Integration hooks for IDE extensions

## Out of Scope

- Specific check implementations (see architecture-safety, antipattern-library)
- IDE extension UI (see ide-integration)
- CI/PR integration (see ci-integration)

## Interfaces

**Depends on:**

- `@anvil/core` — existing gate runner infrastructure
- File system watcher (chokidar, already in codebase)

**Exposes:**

- `analyzeFile(path)` — analyse single file, return warnings
- `analyzeChanges(paths[])` — batch analysis
- `WarningSchema` — structured warning format

## Boundary Rules

- CORE must not depend on IDE-specific APIs
- CORE must not depend on CI/GitHub APIs
- All checks receive the same `CheckContext` interface

## Acceptance Criteria

- [x] `anvil check <file>` returns warnings in < 2s (cached)
- [x] Warnings include explanation and suggestion
- [x] JSON output mode for tooling integration
- [x] Exit code 0 for warnings (non-blocking), non-zero only for errors
- [ ] `anvil check --changed` analyses git-changed files only
- [ ] `anvil watch --source` watches source files and runs checks on save

## Risks & Mitigations

| Risk              | Mitigation                          |
| ----------------- | ----------------------------------- |
| Slow analysis     | Use existing caching infrastructure |
| Too many warnings | Severity filtering, new-only mode   |

## Tasks

### CORE-001: Warning schema definition

- **Intent:** Define the structured warning format that all checks produce
- **Expected Outcome:** Zod schema for warnings with severity, location,
  explanation, suggestion
- **Scope:** `core/src/schema/`
- **Non-scope:** Check implementations
- **Files:** `core/src/schema/warning.schema.ts`
- **Dependencies:** —
- **Validation:** `nx test core`
- **Confidence:** high

### CORE-002: Check runner refactor

- **Intent:** Extend existing gate runner to support on-save analysis mode
- **Expected Outcome:** Runner can execute checks on single files with caching
- **Scope:** `core/src/gate/`
- **Non-scope:** New check implementations
- **Files:** `core/src/gate/runner.ts`, `core/src/gate/context.ts`
- **Dependencies:** CORE-001
- **Validation:** `nx test core`
- **Confidence:** medium

### CORE-003: CLI check command

- **Intent:** Add `anvil check` command for manual analysis
- **Expected Outcome:** CLI command that analyses files and outputs warnings
- **Scope:** `cli/src/commands/`
- **Non-scope:** Watch mode, IDE integration
- **Files:** `cli/src/commands/check.ts`
- **Dependencies:** CORE-002
- **Validation:** `anvil check --help`, manual test
- **Confidence:** high
- **Status:** Complete

### CORE-004: Git-aware changed file detection

- **Intent:** Add `--changed` flag to `anvil check` for git-aware analysis
- **Expected Outcome:** `anvil check --changed` analyses only files changed in
  git (staged, unstaged, or since a ref)
- **Scope:** `cli/src/commands/check.ts`, `core/src/watch/git-status.ts`
- **Non-scope:** Full codebase scanning
- **Files:**
  - `cli/src/commands/check.ts` — add `--changed`, `--since` flags
  - `core/src/watch/git-status.ts` — expose git diff utilities
- **Dependencies:** CORE-003
- **Validation:** `anvil check --changed` on dirty worktree
- **Confidence:** high
- **Status:** Complete

### CORE-005: Source file watch mode

- **Intent:** Extend watch command to support source file analysis on save
- **Expected Outcome:** `anvil watch --source` watches `.ts/.tsx/.js/.jsx` files
  and runs `anvil check` on changes
- **Scope:** `cli/src/commands/watch.ts`
- **Non-scope:** IDE extension (separate module)
- **Files:**
  - `cli/src/commands/watch.ts` — add `--source` flag and check handler
  - `core/src/watch/orchestrator.ts` — add check action type
- **Dependencies:** CORE-003, CORE-004
- **Validation:** `anvil watch --source` on active development
- **Confidence:** high
- **Status:** Complete

## Decisions

- **D-001:** Reuse existing gate runner rather than new architecture

## Notes

- Existing `core/src/gate/` has check infrastructure we can extend
- Watch mode already exists (`anvil watch`) — can be adapted
