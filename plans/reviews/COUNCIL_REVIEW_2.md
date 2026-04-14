# Council Review 2: fix/rcli-tier1-surgical

**Branch:** fix/rcli-tier1-surgical **Date:** 2026-03-31 **Reviewer:**
Independent second review **Base:** 77f73221..HEAD (8 commits incl. prior
council fixup)

## Build & Test

- `cargo build --workspace` — clean
- `cargo test --workspace` — 767 passed, 0 failed

## Findings

### Important

| #   | Severity  | File           | Lines        | Issue                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           | Recommendation                                                                                                                                                                                                                                                                                                                                                                                                                                                   |
| --- | --------- | -------------- | ------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1   | Important | credentials.rs | 36–46, 67–68 | **macOS fallback migration path doesn't work as described.** The first review accepted this as "Intentional migration path: read from old location, write to new." However, `save()` calls `credentials_dir()` which returns the _fallback_ path when XDG doesn't exist. So writes also go to the old `~/Library/Application Support/anvil/` — the user never migrates to XDG. For migration to work, `save()` would need to always write to the XDG (`primary`) path regardless of what `credentials_dir()` returns for reads. | Not a regression (before this change macOS fallback users were simply broken/logged out), but the documented behaviour is inaccurate. Either (a) update `save()` to always target the primary XDG path so migration actually happens, or (b) update the doc comment and review notes to say "fallback is permanent, not a migration." Recommend option (a) as a follow-up, not a merge blocker — the current code is strictly better than what was there before. |

### Minor

| #   | Severity | File           | Lines   | Issue                                                                                                                                                                                                                                                                                                                                              | Recommendation                                                                                                                                                                           |
| --- | -------- | -------------- | ------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 2   | Minor    | util.rs        | 8–19    | **No fsync before rename.** `atomic_write` calls `std::fs::write` then `std::fs::rename` without `File::sync_all()`. This protects against process crashes but not OS crashes — the tmp file data may not be durable on disk before the rename commits. For a CLI state file this is acceptable, but worth documenting.                            | Add a brief doc comment noting the guarantee is process-crash atomicity, not power-loss durability. Not a merge blocker.                                                                 |
| 3   | Minor    | credentials.rs | 75–90   | **Unix `save()` doesn't fsync before rename either.** Same issue as #2 — the `write_all` to tmp followed by rename without sync. For credentials this is slightly more concerning since a corrupt credential file after power loss means the user is logged out.                                                                                   | Same as #2 — document the guarantee level. Could add `file.sync_all()?` before the rename in a follow-up.                                                                                |
| 4   | Minor    | gate.rs        | 246–302 | **`run_check_architecture` has no unit test.** The new architecture check function is only exercised indirectly through the kernel's own `run_embedded` tests. There's no gate-level test that verifies the architecture check path (e.g., that it constructs `EmbeddedConfig` correctly, or that it formats violation messages properly).         | Add a test using `tempfile` that writes a minimal workspace and calls `run_check_architecture` directly. Not a merge blocker — the kernel's own tests give confidence in `run_embedded`. |
| 5   | Minor    | gate.rs        | 246     | **`workspace_root()` called inside `run_check_architecture` is redundant.** When invoked from `run_single_check` → `run_checks`, the workspace root is already available as context. Each check function independently calls `workspace_root()` (which spawns `git rev-parse`), so running the architecture check shells out to git unnecessarily. | Pre-existing pattern across all check functions. Could be refactored to pass root as a parameter, but that's a broader cleanup.                                                          |

### Observations (no action required)

| #   | Severity | File              | Finding                                                                                                                                                                                                                                                                                                       |
| --- | -------- | ----------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 6   | Nit      | engine_mode.rs    | Clean removal. No remaining references to `Legacy`, `Dual`, `validate_mode()`, or `is_implemented()` anywhere in the workspace outside the test assertions that verify they are rejected.                                                                                                                     |
| 7   | Nit      | watch.rs (kernel) | `include_patterns` / `exclude_patterns` fields are wired but unconsumed. Tests correctly initialise them as empty vecs. The watch loop and `initial_scan` don't reference them. This is consistent with the `plan` field pattern — scaffolding for future work.                                               |
| 8   | Nit      | filter.rs         | `FileFilter::should_ignore` does case-sensitive path component matching. On Linux this means `Node_Modules` wouldn't be filtered. On macOS (HFS+/APFS case-insensitive) the filesystem normalises case before path components are compared, so this works correctly in practice. No issue for production use. |
| 9   | Nit      | embedded.rs       | `plan: None` added to all 7 test instances of `EmbeddedConfig`. Mechanical and correct.                                                                                                                                                                                                                       |
| 10  | Nit      | util.rs           | If `atomic_write` fails on rename, the `.tmp` file is left behind. Next call will overwrite it. Acceptable for a CLI.                                                                                                                                                                                         |

## Agreement with First Review

The first review's findings were well-identified. The three issues it fixed (doc
comment on `plan`, `unwrap()` in fail_fast loop, `with_extension` vs `push`)
were genuine and correctly resolved. The deferred items are reasonable.

**One disagreement:** Finding #1 above — the macOS migration path
characterisation in the first review's "Accepted / Deferred" table says
"Intentional migration path: read from old location, write to new." The code
does not implement this. `save()` writes to the fallback (old) location because
it uses `credentials_dir()` which returns the fallback. This should be corrected
either in code or in documentation.

## Risk Assessment

**Low risk, merge-ready.** No Critical findings. The Important finding (#1) is
not a regression — it's a design gap in a new feature that is strictly better
than the previous state (where macOS fallback users were broken). The migration
path can be fixed in a follow-up commit.

All 767 tests pass. No new `unwrap()` on fallible paths in production code. The
`EngineMode` breaking change only affects internal crate API with no external
consumers. Atomic writes are correctly implemented for process-crash safety.
