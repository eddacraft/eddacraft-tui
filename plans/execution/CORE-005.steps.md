# Steps: CORE-005

| Field      | Value                                                                  |
| ---------- | ---------------------------------------------------------------------- |
| Source     | [../modules/save-time-trust.aps.md](../modules/save-time-trust.aps.md) |
| Task(s)    | CORE-005 — Source file watch mode                                      |
| Created by | AI                                                                     |
| Status     | Complete                                                               |

## Prerequisites

- [x] CORE-003 complete (`anvil check` command exists)
- [x] `anvil watch` command exists with orchestrator infrastructure
- [ ] CORE-004 complete (git-aware detection) — recommended but not blocking

## Context

The current `anvil watch` command watches **plan files** and runs validate/gate
actions. For the "save-time trust" workflow, we need `--source` mode that
watches **source files** and runs `anvil check` on changes.

Existing infrastructure:

- `cli/src/commands/watch.ts` — watch command with orchestrator
- `core/src/watch/orchestrator.ts` — file watcher with debouncing
- `core/src/gate/gate-runner.ts` — `analyzeFiles()` for source analysis

## Steps

### 1. Add check action type to watch infrastructure

- **Files:** `core/src/watch/types.ts`, `core/src/watch/orchestrator.ts`
- **Changes:**
  - Extend `WatchAction` type: `'validate' | 'gate' | 'check'`
  - Add `setCheckHandler()` method to orchestrator
  - Update `WatchConfig` to support `action: 'check'`
- **Checkpoint:** `WatchConfig.action` accepts `'check'`
- **Validate:** Type check passes

### 2. Add --source flag to watch command

- **Files:** `cli/src/commands/watch.ts`
- **Changes:**
  - Add `.option('--source', 'Watch source files and run checks (not plans)')`
  - When `--source` is set:
    - Override action to `'check'`
    - Override patterns to
      `['src/**/*.ts', 'src/**/*.tsx', '**/*.ts', '**/*.tsx']` (or from config)
    - Override exclude to include `node_modules`, `dist`, etc.
- **Checkpoint:** `anvil watch --source` changes default patterns
- **Validate:** Manual test

### 3. Implement check handler in watch command

- **Files:** `cli/src/commands/watch.ts`
- **Changes:**
  - Add check handler using `GateRunner.analyzeFiles()`
  - Format output using existing warning formatters
  - Show file path, warning count, and brief summary
  - Support verbose mode for full warning details
- **Checkpoint:** File save triggers check and shows warnings
- **Validate:** Manual test with file containing anti-pattern

### 4. Add source watch configuration to .anvilrc

- **Files:** `core/src/gate/gate-config.ts`, docs
- **Changes:**
  - Add `watch.source` section to config schema:
    ```json
    {
      "watch": {
        "source": {
          "enabled": true,
          "patterns": ["src/**/*.ts", "src/**/*.tsx"],
          "exclude": ["**/*.test.ts", "**/__tests__/**"],
          "debounceMs": 300
        }
      }
    }
    ```
  - Load config in watch command when `--source` is used
- **Checkpoint:** Config file patterns are respected
- **Validate:** Manual test with custom config

### 5. Add incremental analysis optimisation

- **Files:** `cli/src/commands/watch.ts`
- **Changes:**
  - Only analyse the changed file(s), not all watched files
  - Leverage existing caching in `GateRunner`
  - Show timing in output (e.g., "Checked in 45ms")
- **Checkpoint:** Single file save analyses only that file
- **Validate:** Performance test (< 500ms for single file)

### 6. Update documentation

- **Files:** `cli/README.md`, `docs/USER_GUIDE.md`
- **Changes:**
  - Document `anvil watch --source` usage
  - Add configuration examples
  - Explain difference from plan watching
- **Checkpoint:** README shows source watch examples

## Acceptance Criteria

- [ ] `anvil watch --source` watches `.ts/.tsx/.js/.jsx` files
- [ ] File save triggers analysis and shows warnings inline
- [ ] Debouncing prevents rapid re-analysis (300ms default)
- [ ] Git filtering works (only analyse unstaged files by default)
- [ ] Performance: single file analysis < 500ms
- [ ] Config file patterns respected
- [ ] Verbose mode shows full warning details
- [ ] Ctrl+C gracefully stops watch

## Example Usage

```bash
# Watch source files with defaults
anvil watch --source

# Watch with verbose output
anvil watch --source --verbose

# Watch specific patterns
anvil watch --source --patterns "src/**/*.ts,lib/**/*.ts"

# Watch without git filtering (all changes)
anvil watch --source --no-git-filter
```

## Output Format

```
🔨 Anvil Watch (source mode)
  Patterns: src/**/*.ts, src/**/*.tsx
  Action: check
  Git filter: unstaged only

  Watching for changes... (Ctrl+C to stop)

  ─────────────────────────────────────────────
  📁 src/services/payment.ts (saved)
     ⚠ [AP-003] Explicit any type usage
       Line 42: const data: any = response.json()
     ✓ Checked in 127ms (1 warning)
  ─────────────────────────────────────────────

  📁 src/controllers/user.ts (saved)
     ✓ No warnings (89ms)
  ─────────────────────────────────────────────
```

## Notes

- Reuse existing watch orchestrator infrastructure
- Consider memory usage for long-running watch sessions
- Edge case: handle file deletion/rename gracefully
- Future: IDE extension will consume this same analysis
