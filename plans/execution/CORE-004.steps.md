# Steps: CORE-004

| Field      | Value                                                                  |
| ---------- | ---------------------------------------------------------------------- |
| Source     | [../modules/save-time-trust.aps.md](../modules/save-time-trust.aps.md) |
| Task(s)    | CORE-004 — Git-aware changed file detection                            |
| Created by | AI                                                                     |
| Status     | Complete                                                               |

## Prerequisites

- [x] CORE-003 complete (`anvil check` command exists)
- [x] `core/src/watch/git-status.ts` has git utilities

## Context

The `anvil check` command currently requires explicit file paths. For developer
ergonomics, we need `--changed` to automatically detect and analyse files
changed in git.

Existing infrastructure:

- `core/src/watch/git-status.ts` — has `getGitStatus()` returning modified files
- `cli/src/commands/check.ts` — current check command implementation

## Steps

### 1. Expose git diff utilities from core

- **Files:** `core/src/watch/git-status.ts`
- **Changes:**
  - Add `getChangedFiles(options)` function with options:
    - `staged: boolean` — include staged files
    - `unstaged: boolean` — include unstaged files
    - `untracked: boolean` — include untracked files
    - `since?: string` — compare against ref (e.g., `main`, `HEAD~3`)
  - Export from `core/src/watch/index.ts`
- **Checkpoint:** `getChangedFiles({ staged: true, unstaged: true })` returns
  array of file paths
- **Validate:** Unit test in `core/src/watch/git-status.test.ts`

### 2. Add --changed flag to check command

- **Files:** `cli/src/commands/check.ts`
- **Changes:**
  - Add `.option('--changed', 'Analyse git-changed files only')`
  - Add `.option('--staged', 'Include only staged files (with --changed)')`
  - Add `.option('--since <ref>', 'Compare against git ref (e.g., main)')`
  - When `--changed` is set:
    - Call `getChangedFiles()` to get file list
    - Filter to analysable extensions (`.ts`, `.tsx`, `.js`, `.jsx`, `.mjs`,
      `.cjs`)
    - If no files, output "No changed files to analyse" and exit 0
    - Otherwise, analyse the detected files
- **Checkpoint:** `anvil check --changed` works on dirty worktree
- **Validate:** Manual test + unit test

### 3. Add --since flag for branch comparison

- **Files:** `cli/src/commands/check.ts`
- **Changes:**
  - `--since main` compares current HEAD against main branch
  - Useful for CI: check only files changed in PR
  - Combine with `--changed` for flexibility
- **Checkpoint:** `anvil check --changed --since main` shows files changed since
  main
- **Validate:** Manual test on feature branch

### 4. Update help text and examples

- **Files:** `cli/src/commands/check.ts`, `cli/README.md`
- **Changes:**
  - Add examples to command description
  - Update CLI README with new flags
- **Checkpoint:** `anvil check --help` shows new options with descriptions

## Acceptance Criteria

- [ ] `anvil check --changed` analyses git-modified files
- [ ] `anvil check --changed --staged` analyses only staged files
- [ ] `anvil check --changed --since main` analyses files changed since main
- [ ] Empty changeset outputs informative message and exits 0
- [ ] Non-analysable files (e.g., `.md`, `.json`) are filtered out
- [ ] Works in CI environment (GitHub Actions)

## Example Usage

```bash
# Check all changed files (staged + unstaged)
anvil check --changed

# Check only staged files (pre-commit hook)
anvil check --changed --staged

# Check files changed since main branch (CI/PR check)
anvil check --changed --since main

# Check files changed in last 3 commits
anvil check --changed --since HEAD~3
```

## Notes

- Existing `core/src/watch/git-status.ts` already has git integration
- Consider performance: git operations should be fast (< 100ms)
- Edge case: handle repos with no commits gracefully
