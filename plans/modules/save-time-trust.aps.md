<!-- APS: See https://github.com/EddaCraft/anvil-plan-spec for format reference -->
<!-- Executable only if tasks exist and status is Ready. -->

# Save-time Trust

| Scope | Owner | Priority | Status |
| ----- | ----- | -------- | ------ |
| CORE  | —     | high     | Draft  |

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

- [ ] `anvil check <file>` returns warnings in < 2s (cached)
- [ ] Warnings include explanation and suggestion
- [ ] JSON output mode for tooling integration
- [ ] Exit code reflects warning severity

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
- **Validation:** `pnpm test core`
- **Confidence:** high

### CORE-002: Check runner refactor

- **Intent:** Extend existing gate runner to support on-save analysis mode
- **Expected Outcome:** Runner can execute checks on single files with caching
- **Scope:** `core/src/gate/`
- **Non-scope:** New check implementations
- **Files:** `core/src/gate/runner.ts`, `core/src/gate/context.ts`
- **Dependencies:** CORE-001
- **Validation:** `pnpm test core`
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

## Decisions

- **D-001:** Reuse existing gate runner rather than new architecture

## Notes

- Existing `core/src/gate/` has check infrastructure we can extend
- Watch mode already exists (`anvil watch`) — can be adapted
