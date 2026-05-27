# Council Review Synthesis

**Tier:** COUNCIL (6 agents)
**Mode:** internal
**Branch:** prep (f0d34257..HEAD, 7 commits)
**Date:** 2026-03-27
**Verdict:** SHIP with 1 fix required

## Scope

| Commit | Summary |
|--------|---------|
| d133a1d6 | feat(RCLI-015a): legacy credential path fallback with XDG migration |
| 65d68ca9 | fix(RCLI-015a): do not persist env var credentials to disk |
| e438843c | feat(RCLI-015b): pre-action auth enforcement middleware |
| d02891e0 | fix(RCLI-015b): normalise JSON error format across auth failure path |
| 5862b2a0 | feat(RCLI-022): OutputMode enum and gate integration |
| e67313e8 | feat(RCLI-030): gate and watch menu items in welcome surface |
| 0009b6b7 | chore(RCLI-023): archive Node.js CLI and Ink TUI |

**Changed files (non-rename):** ~15 files, +1407/-1088 lines
**Test count:** ~65 new/modified tests

## Consolidated Findings

### Must Fix (block merge)

| ID | Severity | Finding | File | Reviewers |
|----|----------|---------|------|-----------|
| F-001 | **major** | `gate::run()` calls `std::process::exit(2)` directly, bypassing main()'s error handling and terminal teardown | `gate.rs:759` | kernel-maintainer, pragmatic-lead |

**Fix:** Replace `std::process::exit(i32::from(crate::EXIT_GATE_FAIL))` with a
return that main() can map to exit code 2. Options: (a) return a typed error, or
(b) use a `GateFailure` error variant that main() matches to EXIT_GATE_FAIL.

### Should Fix (before merge if practical)

| ID | Severity | Finding | File | Reviewers |
|----|----------|---------|------|-----------|
| F-002 | **major** | Credential writes are not atomic (partial write on crash) | `credentials.rs:144-153,182-189` | adversarial-reviewer, security-analyst |

**Fix:** Write to a temp file in the same directory, then `std::fs::rename()`.

### Defer (follow-up issues)

| ID | Severity | Finding | Reviewers |
|----|----------|---------|-----------|
| D-001 | minor | Duplicated file-write logic between `migrate_to_xdg` and `save` | code-reviewer |
| D-002 | minor | Redundant `set_permissions` call in `save()` | code-reviewer |
| D-003 | minor | `create_first_run_marker` takes `&PathBuf` not `&Path` | code-reviewer |
| D-004 | minor | `workspace_root()` called multiple times per gate run | kernel-maintainer |
| D-005 | minor | Architecture gate check presents YAML-parse-only as full validation | kernel-maintainer |
| D-006 | minor | Secret scan has only 3 patterns; message implies comprehensive scan | adversarial-reviewer |
| D-007 | minor | walkdir has no depth limit in secret scan | adversarial-reviewer |
| D-008 | minor | `evaluate_auth` swallows the underlying error | adversarial-reviewer |
| D-009 | minor | `first_run_marker_path` is relative to CWD not project root | adversarial-reviewer |
| D-010 | minor | No deprecation notice for old credential files after migration | operations-reviewer |
| D-011 | minor | Exit codes not documented externally for CI consumers | operations-reviewer |
| D-012 | minor | Non-Unix credential write has no permission restriction | security-analyst |
| D-013 | minor | Secret scan reads entire files into memory (no size cap) | security-analyst |

### Positive Observations

- **Credential security:** 0o600 permissions, HTTPS enforcement, env-var
  credentials never persisted to disk. Well-tested invariants.
- **Test quality:** ~65 tests with thorough coverage of priority ordering,
  migration, auth enforcement, and output mode resolution.
- **Archival execution:** Clean build system separation -- all Nx plugins,
  pnpm workspace, ESLint, vitest, and tsconfig references properly updated.
  Lockfile shrinks by ~1000 lines.
- **Auth middleware design:** Clean `requires_auth` + `evaluate_auth` separation
  with exhaustive matching over all command variants.
- **OutputMode:** Simple, well-tested 3-variant enum with clear priority chain
  (--json > --no-tui > TTY detection).

## Reviewer Verdicts

| Reviewer | Verdict | Key Concern |
|----------|---------|-------------|
| code-reviewer | PASS (minor) | DRY violations in credential writes |
| kernel-maintainer | PASS (1 major) | process::exit bypasses error handling |
| pragmatic-lead | SHIP (1 fix) | process::exit must be fixed |
| adversarial-reviewer | PASS | TOCTOU race in migration (low likelihood) |
| operations-reviewer | PASS | Clean rollback posture |
| security-analyst | PASS | Atomic writes recommended |

## Recommendation

**Ship after fixing F-001** (process::exit in gate.rs). F-002 (atomic credential
writes) is strongly recommended before merge but acceptable as an immediate
follow-up given the low likelihood of the failure mode.

All 13 deferred items are minor quality improvements suitable for a follow-up
batch.
