# Post-Release Review — Changes Since v0.2.1-beta.0

**Date:** 2026-02-26
**Scope:** All 50 commits on `main` since the repo root (all post-release)
**Files changed:** 118 files, +5498 / -870 lines
**Reviewer:** Claude (automated deep review across 5 parallel agents)

---

## Executive Summary

The changes since the last release (`v0.2.1-beta.0`, 2026-02-22) fall into five
major areas:

1. **Forge & Temper pipeline** — New autonomous code review infrastructure
   (hooks, agents, commands, GitHub Actions workflow)
2. **CLI refactoring** — `process.exit()` replaced with `CliError`/`CliExit`
   thrown-error pattern across ~30 command files
3. **Security hardening** — Shell injection fix, API client URL validation,
   auth store Zod validation, path traversal mitigations
4. **Test coverage** — New test suites for tutorial continuation, picker, audit
   spinner, MCP config paths, and flaky test fixes
5. **Documentation & CI** — Docs accuracy fixes, caution banners for planned
   features, CI workflow updates, APS plan modules for Forge/Temper

Overall quality is good. The security fixes are substantive and correct. The CLI
refactoring is consistent. The Forge pipeline has shell-scripting issues that
need attention before enabling. Documentation has version drift that should be
fixed before sharing with beta testers.

---

## Findings by Severity

### CRITICAL (1)

| # | Area | File | Finding |
|---|------|------|---------|
| C-1 | Tests | `tutorial-picker.test.tsx:4,9` | **Shadowed `stripAnsi` import** — local `const stripAnsi` shadows the imported version from `test-utils.ts` with a weaker regex that only strips SGR sequences (`\x1B[...m`), missing OSC sequences and other escape types. The import on line 4 is dead code. Remove the local definition. |

### MAJOR (16)

| # | Area | File | Finding |
|---|------|------|---------|
| M-1 | Forge | `forge.sh:79,91-127` | **Shell injection via filenames in heredocs** — `STAGED_FILES` and `STAGED_DIFF_STAT` from `git diff` are interpolated into JSON and Markdown heredocs without escaping. A filename containing `"`, `` ` ``, or `$()` breaks JSON structure and could corrupt reports. Use `jq -n --arg` for JSON values. |
| M-2 | Forge | `forge.sh:88` | **Unsafe `echo` for diff file** — `echo "$STAGED_DIFF"` interprets flags (`-e`, `-n`) if the diff starts with those bytes. Use `printf '%s'` instead. |
| M-3 | Forge | `forge.sh:33-36` | **Bypass guard defeat via substring** — `--no-verify` and `--amend` guards are substring matches on the full command string. A string literal containing these words (e.g., in an echo or commit message) causes false positives/negatives. |
| M-4 | Forge | `forge-defer.sh:192-200` | **`get_repo_info` failure not guarded** — function returns 1 on failure but call site does not explicitly check, relying on `set -e` which can be fragile with inherited `set +e`. Add `|| exit 1`. |
| M-5 | Forge | `forge-defer.sh:165` | **Path traversal via `module_id`** — `find -name "*${module_id,,}*"` uses lowercased branch-extracted ID in a glob. While the regex limits to `[A-Z]{2,6}`, validate strictly after extraction. |
| M-6 | Temper | `temper.yml:63-64` | **Fragile cycle count** — counts all PR comments starting with `## Temper`, including human comments. Use a unique HTML comment marker instead. |
| M-7 | CLI | `plan.ts:133-140` | **`plan create` drops original error message** — `CliError('Failed to create plan')` discards the original `error.message`. Other commands forward the original; this should be consistent. |
| M-8 | CLI | `plan/load.ts:195-209` | **Validation errors throw plain `Error`, not `CliError`** — `throw new Error(...)` for invalid priority/confidence values bypasses the CliError pattern. These are thrown before the try block, relying on Commander's error handling. |
| M-9 | CLI | `mcp-config.ts:114,120` | **Uses `process.cwd()` instead of `getWorkspaceRoot()`** — the symlink-bypass check anchors to `process.cwd()` while all other commands use `getWorkspaceRoot()`. In monorepo subdirectories this produces inconsistent behavior. |
| M-10 | CLI | `policy.ts:809-814` | **Weaker path-escape check than other commands** — `policy doc` uses a home-rolled `relative().startsWith('..')` check instead of the shared `validatePathWithinRoot()` utility used by `export.ts`. |
| M-11 | Security | `api-client.test.ts` | **Missing IPv6 loopback test** — `::1` is in the `allowedLocalHosts` set but has no test case. Add `http://[::1]:3000` test alongside the `127.0.0.1` test. |
| M-12 | Security | `mcp-config-path.test.ts` | **Parent directory symlink not tested** — only tests symlink at final path component. The parent directory (`.cursor/`) being a symlink to outside the workspace is untested. |
| M-13 | Tests | `watch.test.ts:110-137` | **Source-text scan masquerading as a test** — reads `watch.ts` source and asserts on string contents. Breaks on rename/reformat, passes if `process.exit(0)` appears in a comment. Should mock `process.exit` and emit `SIGINT` instead. |
| M-14 | Docs | `changelog.md`, `upgrade-notes.md`, `beta/quickstart.md` | **Version number drift** — docs site shows `0.1.2-beta` as current; actual current release is `0.2.1-beta.0`. Three files affected. |
| M-15 | CI | `claude-code-review.yml:26` | **Unnecessary `id-token: write` permission** — grants OIDC token minting to a workflow that only needs `contents: read` and `pull-requests: write`. Remove this privilege. |
| M-16 | Tests | `tutorial-continuation.test.tsx`, `tutorial-picker.test.tsx` | **Duplicate `resolveTutorialKey` tests** — both files contain `describe('resolveTutorialKey', ...)` blocks testing the same function. Consolidate into `tutorial-picker.test.tsx`. |

### MINOR (25)

| # | Area | File | Finding |
|---|------|------|---------|
| m-1 | Forge | `forge.sh:64` | Hash fallback (when no sha/md5 available) strips non-alnum characters instead of hashing — collision-prone. |
| m-2 | Forge | `forge.sh:79,100` | `stagedFiles` stored as comma-separated string, not JSON array. |
| m-3 | Forge | `forge.sh:83-88` | Diff files in `agent-bus/diffs/` accumulate indefinitely — no cleanup. |
| m-4 | Forge | `forge-defer.sh:142` | Output JSON built via string interpolation, not `jq -n` — fragile. |
| m-5 | Forge | `forge-defer.sh:203-223` | Batch is sequential; `get_repo_info` result unused by `gh` commands. |
| m-6 | Forge | `forge-report.sh:47,66` | JSON via positional arg breaks if value contains single quotes. |
| m-7 | Forge | `forge-report.sh:60,74,100` | Inconsistent `|| true` guards under `set -e`. |
| m-8 | Temper | `temper.yml:8-9` | Fires on all review submissions (incl. approvals), not just change requests. |
| m-9 | CLI | `hooks.ts:306-311,380-384` | Double `spinner.fail()` + `error()` pattern fragile across inner/outer catch. |
| m-10 | CLI | `audit.ts:327-334` | `CliExit()` on natural return paths — should just `return`. |
| m-11 | CLI | `export.ts:224-228` | `process.cwd()` vs `getWorkspaceRoot()` inconsistency for constraint export. |
| m-12 | Security | `api-client.ts:115` | `/admin/` route detection is substring match — `items/admin/notes` would match. |
| m-13 | Security | `api-client.ts:91-97` | Non-TimeoutError DOMException falls to wrong error branch. |
| m-14 | Security | `auth-store.ts:38` | TOCTOU race: `existsSync` + `readFileSync` — redundant, rely on try/catch. |
| m-15 | Security | `api-client.ts:38` | `getApiUrl()` may return trailing slash, causing double-slash in paths. |
| m-16 | Security | `audit-spinner.test.ts` | No test for scan-throws-after-spinner-started scenario. |
| m-17 | Security | `mcp-config-path.test.ts` | `--yes` bypass path not tested. |
| m-18 | Tests | `tutorial-continuation.test.tsx:149` | `tick()` sleeps (50ms each) between steps — use `flushPromises()`. |
| m-19 | Tests | `Tutorial.tsx:118`, `NextStepsStep.tsx:77` | Hardcoded `'core'` as `currentTopic` — should be a prop or documented. |
| m-20 | Tests | `init.test.ts:343-449` | Interactive tests don't verify `TemplateGenerator` received prompt values. |
| m-21 | Tests | `test-utils.ts:57-73` | `waitForFrame` duplicates `vi.waitFor` — currently unused. |
| m-22 | Docs | `plans/beta-launch-checklist.aps.md:5-7` | Header says v0.1.0 and "Draft" status post-release. |
| m-23 | Docs | `plans/modules/03` / `04` | DEFER↔TEMPER circular dependency needs clarification note. |
| m-24 | Docs | `README.md:188` | `claude-code-review.yml` not listed in CI/CD section. |
| m-25 | Docs | `plans/reviews/cli-beta-review.md:383` | H-1 deferred item has no tracking issue link. |

### NIT (15)

| # | Finding |
|---|---------|
| N-1 | `audit.ts:9-10` — import after `const` assignment; unconventional order |
| N-2 | `hooks.ts:80-83` — orphaned double JSDoc block |
| N-3 | `plan/load.ts` — no `CliExit()` on JSON success (inconsistent with other plan subcommands) |
| N-4 | `forge.sh:56` — non-deterministic hash input (includes `$$`) not documented |
| N-5 | `forge-defer.sh:174` — no file locking for concurrent APS module appends |
| N-6 | `forge-report.sh:107` — `COMPLETED_AT` is redundant re-assignment of `TIMESTAMP` |
| N-7 | `forge.md:200` — `--no-verify` skips all hooks, not just Forge — undocumented |
| N-8 | `.gitignore:57,70` — duplicate `.claude/worktrees/` entry |
| N-9 | `.prettierignore:12` — terse reason for `claude-code-review.yml` exclusion |
| N-10 | `README.md:141-143` — footnote markers `^1/^2/^3` don't render as superscripts in GFM |
| N-11 | `tutorial-continuation.test.tsx:144` — file name doesn't match sole remaining describe block |
| N-12 | `tutorial-picker.test.tsx:93` — inline comment could be clearer about index mapping |
| N-13 | `init.test.ts:369` — `toHaveBeenCalled()` should be `toHaveBeenCalledTimes(3)` |
| N-14 | `temper.yml:1` — `name:` before `on:` is unconventional for Actions |
| N-15 | `forge-reviewer.md` — `codexAgreed` Required should be `yes (default false)` |

---

## Priority Recommendations

### Before Next Beta Push (Blocking)

1. **Fix docs version drift (M-14)** — Update `docs/public/anvil/releases/changelog.md`,
   `upgrade-notes.md`, and `beta/quickstart.md` to reflect `v0.2.1-beta.0`.
   Users will lose trust in docs that show the wrong version number.

2. **Remove `id-token: write` from CI (M-15)** — Unnecessary OIDC privilege in
   the code review workflow.

3. **Fix `plan create` error message loss (M-7)** — Forward `error.message` in
   the CliError constructor, consistent with all other commands.

4. **Fix `plan/load.ts` validation errors (M-8)** — Change `throw new Error()`
   to `throw new CliError()` for invalid priority/confidence values.

### Before Enabling Forge (Blocking for Forge)

5. **Fix shell injection in `forge.sh` heredocs (M-1)** — Escape filenames and
   diff stat before embedding in JSON/Markdown. Use `jq -n --arg`.

6. **Fix `--no-verify`/`--amend` bypass guard (M-3)** — Strengthen the regex to
   avoid substring matches in string literals within the command.

7. **Fix Temper cycle counting (M-6)** — Use a unique HTML comment marker
   instead of matching on `## Temper` prefix.

### Before GA

8. **Add IPv6 loopback test (M-11)** — `::1` is in the allowlist but untested.

9. **Add parent-directory symlink test (M-12)** — Test `.cursor/` itself being
   a symlink outside workspace.

10. **Consolidate duplicate `resolveTutorialKey` tests (M-16)** — Move to
    `tutorial-picker.test.tsx` only.

11. **Replace source-text scan in `watch.test.ts` (M-13)** — Use behavioral
    test with `process.exit` mock and SIGINT signal.

12. **Normalize path validation** — Ensure `mcp-config.ts` (M-9) and
    `policy.ts` (M-10) use `getWorkspaceRoot()` and the shared
    `validatePathWithinRoot()` utility respectively.

---

## Area-by-Area Summary

### Forge & Temper Pipeline
Well-conceived architecture. The agent, command, report, and defer modules are
well-documented. Shell scripting quality needs improvement: the main risks are
JSON/Markdown injection via filenames (M-1), unsafe `echo` for diff content
(M-2), and weak bypass guards (M-3). The Temper GitHub Actions workflow is
functional but should filter on review state and use a more specific cycle
counter marker.

### CLI Refactoring (CliError/CliExit)
Successfully migrated ~30 command files. One intentional `process.exit(0)`
survivor in `watch.ts` signal handler is correctly documented and tested. Two
consistency gaps: `plan create` drops error messages, `plan/load` uses plain
`Error` instead of `CliError`. The `--json` output feature is well-implemented
in hooks and plan subcommands. Barrel export removal is clean.

### Security Hardening
All critical and high-severity items from the beta review are fixed:
- C-1 shell injection: `exec` → `execFile` (**fixed**)
- C-2 401 handling: scoped to user routes, clears auth (**fixed**)
- HTTPS enforcement: URL parsing with localhost bypass (**fixed**)
- Auth store: Zod schema validation (**fixed**)
- Request timeout: `AbortSignal.timeout(30_000)` (**fixed**)

Remaining gaps: IPv6 test coverage, parent-directory symlink test, TOCTOU in
auth store, admin route substring match.

### Test Coverage
Good breadth of new tests. Tutorial continuation and picker tests are thorough.
The `test-utils.ts` module (`stripAnsi`, `KEY_SEQUENCES`, TTY mocks) is a
valuable addition. Main concerns: duplicate test blocks across files, source-text
scan in watch.test.ts, and missing edge cases for security-sensitive paths.

### Documentation & CI
Docs content is accurate and well-written with appropriate caution banners for
planned features. Critical issue: three docs files show `0.1.2-beta` as current
version when `0.2.1-beta.0` is released. APS plan modules for Forge/Temper are
well-structured with clear scope and dependency declarations.

---

## Stats

| Severity | Count |
|----------|-------|
| Critical | 1 |
| Major | 16 |
| Minor | 25 |
| Nit | 15 |
| **Total** | **57** |
