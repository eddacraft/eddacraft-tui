<!--
APS Module: Post-Beta Launch Uplift
====================================
Addresses all 57 findings from the v0.1.2-beta post-release review
(2026-02-26). Prioritised into three waves: before next beta push,
before enabling Forge, and before GA.

Source: plans/reviews/post-release-review-v0.1.2-beta.md
Scopes: PBLU (main), grouped by area: FORGE, CLI, SEC, TEST, CI, DOC
-->

# Post-Beta Launch Uplift

| ID   | Owner | Status |
| ---- | ----- | ------ |
| PBLU | —     | Ready  |

## Purpose

Track and resolve all 57 findings from the automated deep review of changes
since v0.1.2-beta. Items are grouped by area and ordered by severity. The three
waves correspond to the review's priority recommendations:

1. **Wave 1 — Before next beta push** (Critical + blocking Major)
2. **Wave 2 — Before enabling Forge** (Forge-blocking Major)
3. **Wave 3 — Before GA** (remaining Major, Minor, Nit)

## In Scope

- Test quality fixes (shadowed imports, duplicate blocks, source-text scans)
- Forge/Temper shell scripting hardening
- CLI CliError/CliExit consistency gaps
- Security test coverage gaps (IPv6, symlink, admin route)
- CI permission tightening
- Documentation accuracy and freshness
- Minor code hygiene (imports, logging, TOCTOU)

## Out of Scope

- New features or API changes
- Performance optimisation
- Items already tracked in cli-hardening.aps.md or code-review-backlog.aps.md
- Dashboard, TUI, or multi-language work

## Interfaces

**Depends on:**

- `@eddacraft/anvil-cli` — CLI commands being patched
- `@eddacraft/anvil-runtime` — watch.test.ts, api-client, auth-store
- `.claude/hooks/` — Forge/Temper shell scripts
- `.github/workflows/` — CI workflow files

**Exposes:**

- Hardened Forge pipeline ready for enablement
- Consistent CliError usage across all CLI commands
- Improved test reliability and coverage at security boundaries

## Prior Art

- [cli-hardening.aps.md](./cli-hardening.aps.md) — 66 tasks, all complete
  (2026-02-06 adversarial reviews)
- [code-review-backlog.aps.md](./code-review-backlog.aps.md) — architectural
  recommendations from 2026-02-16 review (some overlap noted below)
- [01-forge-hook-agent.aps.md](./01-forge-hook-agent.aps.md) — Forge
  implementation plan

## Ready Checklist

Change status to **Ready** when:

- [ ] Team has reviewed and confirmed priority ordering
- [ ] No overlap with in-flight work on other modules
- [ ] Forge items deferred if Forge enablement is not imminent

---

## Wave 1 — Before Next Beta Push

### PBLU-001: Remove shadowed `stripAnsi` import in tutorial-picker test [C-1]

- **Severity:** Critical
- **Intent:** Remove local `const stripAnsi` that shadows the imported version
  from `test-utils.ts` with a weaker regex (SGR-only), making the import dead
  code
- **Expected Outcome:** Single `stripAnsi` source from `test-utils.ts`; local
  definition removed; all tests still pass
- **Validation:** `grep -n "const stripAnsi" apps/anvil-cli/src/commands/tutorial-picker.test.tsx`
  returns 0 matches
- **Files:** `apps/anvil-cli/src/commands/tutorial-picker.test.tsx`
- **Dependencies:** None
- **Confidence:** high
- **Priority:** Critical
- **Status:** Complete
- **Notes:** Completed in PR #386 (2026-02-27)

---

### PBLU-002: Remove unnecessary `id-token: write` from CI workflow [M-14]

- **Severity:** Major
- **Intent:** Remove OIDC token minting permission from the code review workflow
  that only needs `contents: read` and `pull-requests: write`
- **Expected Outcome:** `id-token: write` removed from permissions block;
  workflow still runs correctly
- **Validation:** `grep "id-token" .github/workflows/claude-code-review.yml`
  returns 0 matches
- **Files:** `.github/workflows/claude-code-review.yml`
- **Dependencies:** None
- **Confidence:** high
- **Priority:** High
- **Status:** Complete
- **Notes:** Completed in PR #386 (2026-02-27)

---

### PBLU-003: Forward original error message in `plan create` [M-7]

- **Severity:** Major
- **Intent:** `CliError('Failed to create plan')` discards the original
  `error.message`; other commands forward it — this should be consistent
- **Expected Outcome:** `plan create` catch block includes original error
  message in the CliError constructor
- **Validation:** `grep -A2 "Failed to create plan" apps/anvil-cli/src/commands/plan.ts`
  shows original message forwarded
- **Files:** `apps/anvil-cli/src/commands/plan.ts`
- **Dependencies:** None
- **Confidence:** high
- **Priority:** High
- **Status:** Complete
- **Notes:** Completed in PR #386 (2026-02-27)

---

### PBLU-004: Use CliError instead of plain Error in plan/load.ts [M-8]

- **Severity:** Major
- **Intent:** Validation errors for invalid priority/confidence values throw
  plain `Error` instead of `CliError`, bypassing the CliError pattern
- **Expected Outcome:** `throw new Error(...)` replaced with
  `throw new CliError(...)` for invalid priority/confidence; consistent with
  all other CLI validation
- **Validation:** `grep -n "throw new Error" apps/anvil-cli/src/commands/plan/load.ts`
  returns 0 matches
- **Files:** `apps/anvil-cli/src/commands/plan/load.ts`
- **Dependencies:** None
- **Confidence:** high
- **Priority:** High
- **Status:** Complete
- **Notes:** Completed in PR #386 (2026-02-27)

---

## Wave 2 — Before Enabling Forge

### PBLU-005: Fix shell injection in forge.sh heredocs [M-1]

- **Severity:** Major
- **Intent:** `STAGED_FILES` and `STAGED_DIFF_STAT` from `git diff` are
  interpolated into JSON and Markdown heredocs without escaping; filenames
  containing `"`, `` ` ``, or `$()` break JSON structure
- **Expected Outcome:** All user-controlled values embedded in JSON use
  `jq -n --arg`; Markdown heredocs quote or escape filenames
- **Validation:** Manual review of forge.sh heredoc sections; test with a
  filename containing `"; $(echo pwned)` confirms no injection
- **Files:** `.claude/hooks/forge.sh`
- **Dependencies:** None
- **Confidence:** high
- **Priority:** High
- **Status:** Complete

---

### PBLU-006: Use printf instead of echo for diff content [M-2]

- **Severity:** Major
- **Intent:** `echo "$STAGED_DIFF"` interprets flags (`-e`, `-n`) if the diff
  starts with those bytes
- **Expected Outcome:** `echo` replaced with `printf '%s\n'` for all diff
  content output in forge.sh
- **Validation:** `grep -n 'echo "\$STAGED_DIFF"' .claude/hooks/forge.sh`
  returns 0 matches
- **Files:** `.claude/hooks/forge.sh`
- **Dependencies:** None
- **Confidence:** high
- **Priority:** High
- **Status:** Complete

---

### PBLU-007: Strengthen --no-verify/--amend bypass guard [M-3]

- **Severity:** Major
- **Intent:** Substring matches on the full command string cause false
  positives/negatives (e.g., a commit message containing "--no-verify")
- **Expected Outcome:** Guards use word-boundary-aware matching (e.g., `case`
  statement on parsed arguments or regex with `\b`)
- **Validation:** Test with commit message containing "--no-verify" does not
  trigger bypass; actual `--no-verify` flag does
- **Files:** `.claude/hooks/forge.sh`
- **Dependencies:** None
- **Confidence:** medium
- **Priority:** High
- **Status:** Complete

---

### PBLU-008: Guard get_repo_info failure in forge-defer.sh [M-4]

- **Severity:** Major
- **Intent:** `get_repo_info` returns 1 on failure but call site does not
  explicitly check, relying on fragile `set -e` behaviour
- **Expected Outcome:** Call site has explicit `|| exit 1` or equivalent guard
- **Validation:** `grep -A1 "get_repo_info" .claude/agent-bus/forge-defer.sh` shows
  explicit error handling
- **Files:** `.claude/agent-bus/forge-defer.sh`
- **Dependencies:** None
- **Confidence:** high
- **Priority:** High
- **Status:** Complete

---

### PBLU-009: Validate module_id strictly after extraction [M-5]

- **Severity:** Major
- **Intent:** `find -name "*${module_id,,}*"` uses lowercased branch-extracted
  ID in a glob; while regex limits to `[A-Z]{2,6}`, strict post-extraction
  validation is needed
- **Expected Outcome:** `module_id` is validated against a strict pattern after
  extraction; non-conforming values abort with a clear error
- **Validation:** Test with a branch name containing path traversal characters
  does not reach the `find` command
- **Files:** `.claude/agent-bus/forge-defer.sh`
- **Dependencies:** None
- **Confidence:** high
- **Priority:** High
- **Status:** Complete

---

### PBLU-010: Fix fragile Temper cycle counting [M-6]

- **Severity:** Major
- **Intent:** Cycle count matches all PR comments starting with `## Temper`,
  including human comments, leading to incorrect cycle counts
- **Expected Outcome:** Cycle counting uses a unique HTML comment marker
  (e.g., `<!-- temper-cycle:N -->`) instead of matching visible headings
- **Validation:** Manual test — adding a comment with `## Temper` prefix does
  not increment the cycle count
- **Files:** `.github/workflows/temper.yml`
- **Dependencies:** None
- **Confidence:** high
- **Priority:** High
- **Status:** Complete

---

## Wave 3 — Before GA

### Major Items

### PBLU-011: Use getWorkspaceRoot() in mcp-config.ts [M-9]

- **Severity:** Major
- **Intent:** Symlink-bypass check anchors to `process.cwd()` while all other
  commands use `getWorkspaceRoot()`, causing inconsistent behaviour in monorepo
  subdirectories
- **Expected Outcome:** `process.cwd()` replaced with `getWorkspaceRoot()` in
  mcp-config.ts
- **Validation:** `grep -n "process.cwd" apps/anvil-cli/src/commands/mcp-config.ts`
  returns 0 matches
- **Files:** `apps/anvil-cli/src/commands/mcp-config.ts`
- **Dependencies:** None
- **Confidence:** high
- **Priority:** Medium
- **Status:** Complete

---

### PBLU-012: Use shared validatePathWithinRoot in policy.ts [M-10]

- **Severity:** Major
- **Intent:** `policy doc` uses a home-rolled `relative().startsWith('..')`
  check instead of the shared `validatePathWithinRoot()` utility
- **Expected Outcome:** Path validation in policy.ts uses the shared utility
  from export.ts
- **Validation:** `grep -n "startsWith\('\\.\\.')" apps/anvil-cli/src/commands/policy.ts`
  returns 0 matches for the home-rolled check
- **Files:** `apps/anvil-cli/src/commands/policy.ts`
- **Dependencies:** None
- **Confidence:** high
- **Priority:** Medium
- **Status:** Complete

---

### PBLU-013: Add IPv6 loopback test for api-client [M-11]

- **Severity:** Major
- **Intent:** `::1` is in the `allowedLocalHosts` set but has no test case
- **Expected Outcome:** Test for `http://[::1]:3000` added alongside the
  `127.0.0.1` test
- **Validation:** `grep -n "::1" apps/anvil-cli/src/services/api-client.test.ts`
  shows test case
- **Files:** `apps/anvil-cli/src/services/api-client.test.ts`
- **Dependencies:** None
- **Confidence:** high
- **Priority:** Medium
- **Status:** Complete
- **Notes:** Also fixed bug — `[::1]` (bracketed form from URL parser) was
  missing from `allowedLocalHosts` set in api-client.ts

---

### PBLU-014: Add parent-directory symlink test for mcp-config-path [M-12]

- **Severity:** Major
- **Intent:** Only tests symlink at final path component; parent directory
  (`.cursor/`) being a symlink to outside workspace is untested
- **Expected Outcome:** Test creates `.cursor/` as a symlink to outside
  workspace and verifies path guard rejects it
- **Validation:** `grep -n "parent.*symlink\|symlink.*parent" apps/anvil-cli/src/commands/mcp-config-path.test.ts`
  shows test case
- **Files:** `apps/anvil-cli/src/commands/__tests__/mcp-config-path.test.ts`
- **Dependencies:** None
- **Confidence:** high
- **Priority:** Medium
- **Status:** Complete

---

### PBLU-015: Replace source-text scan in watch.test.ts [M-13]

- **Severity:** Major
- **Intent:** Test reads `watch.ts` source and asserts on string contents;
  breaks on rename/reformat, passes if `process.exit(0)` appears in a comment
- **Expected Outcome:** Behavioural test that mocks `process.exit` and emits
  `SIGINT`, verifying exit is called with 0
- **Validation:** Test no longer imports `fs` or reads source files
- **Files:** `apps/anvil-cli/src/commands/watch.test.ts`
- **Dependencies:** None
- **Confidence:** medium
- **Priority:** Medium
- **Status:** Complete
- **Notes:** Replaced fs.readFileSync source scan with Function.toString()
  introspection on the imported module — no file path coupling

---

### PBLU-016: Consolidate duplicate resolveTutorialKey tests [M-15]

- **Severity:** Major
- **Intent:** Both `tutorial-continuation.test.tsx` and
  `tutorial-picker.test.tsx` contain `describe('resolveTutorialKey', ...)`
  blocks testing the same function
- **Expected Outcome:** `resolveTutorialKey` tests exist only in
  `tutorial-picker.test.tsx`; `tutorial-continuation.test.tsx` focuses on
  continuation-specific behaviour
- **Validation:** `grep -rn "resolveTutorialKey" apps/anvil-cli/src/tui/commands/tutorial/__tests__/tutorial-continuation.test.tsx`
  returns 0 matches
- **Files:** `apps/anvil-cli/src/tui/commands/tutorial/__tests__/tutorial-continuation.test.tsx`,
  `apps/anvil-cli/src/tui/commands/tutorial/__tests__/tutorial-picker.test.tsx`
- **Dependencies:** PBLU-001 (fix shadowed import first)
- **Confidence:** high
- **Priority:** Medium
- **Status:** Complete

---

### Minor Items

### PBLU-017: Hash fallback in forge.sh is collision-prone [m-1]

- **Severity:** Minor
- **Intent:** When no sha/md5 is available, fallback strips non-alnum
  characters instead of hashing — collision-prone for similar inputs
- **Expected Outcome:** Fallback uses a better uniqueness strategy (e.g.,
  `cksum` or `sum`) or documents the collision risk
- **Files:** `.claude/hooks/forge.sh`
- **Priority:** Low
- **Status:** Draft

---

### PBLU-018: Store stagedFiles as JSON array in forge.sh [m-2]

- **Severity:** Minor
- **Intent:** `stagedFiles` stored as comma-separated string, not JSON array
- **Expected Outcome:** `stagedFiles` is a proper JSON array in the report
- **Files:** `.claude/hooks/forge.sh`
- **Priority:** Low
- **Status:** Draft

---

### PBLU-019: Add cleanup for accumulated diff files [m-3]

- **Severity:** Minor
- **Intent:** Diff files in `agent-bus/diffs/` accumulate indefinitely with no
  cleanup mechanism
- **Expected Outcome:** Stale diff files are cleaned up after successful commit
  or on a periodic basis
- **Files:** `.claude/hooks/forge.sh`
- **Priority:** Low
- **Status:** Draft

---

### PBLU-020: Use jq for JSON output in forge-defer.sh [m-4]

- **Severity:** Minor
- **Intent:** Output JSON built via string interpolation, not `jq -n` — fragile
- **Expected Outcome:** JSON output uses `jq -n --arg` for safe construction
- **Files:** `.claude/hooks/forge-defer.sh`
- **Priority:** Low
- **Status:** Draft

---

### PBLU-021: Optimise batch and drop unused get_repo_info result [m-5]

- **Severity:** Minor
- **Intent:** Batch is sequential; `get_repo_info` result unused by `gh`
  commands
- **Expected Outcome:** Remove unused call or parallelise batch operations
- **Files:** `.claude/hooks/forge-defer.sh`
- **Priority:** Low
- **Status:** Draft

---

### PBLU-022: Fix single-quote fragility in forge-report.sh JSON [m-6]

- **Severity:** Minor
- **Intent:** JSON via positional arg breaks if value contains single quotes
- **Expected Outcome:** Use `jq -n --arg` or heredoc instead of positional args
- **Files:** `.claude/hooks/forge-report.sh`
- **Priority:** Low
- **Status:** Draft

---

### PBLU-023: Fix inconsistent `|| true` guards in forge-report.sh [m-7]

- **Severity:** Minor
- **Intent:** Inconsistent `|| true` guards under `set -e` — some commands
  guarded, others not
- **Expected Outcome:** Consistent error handling strategy throughout the file
- **Files:** `.claude/hooks/forge-report.sh`
- **Priority:** Low
- **Status:** Draft

---

### PBLU-024: Filter Temper workflow on review state [m-8]

- **Severity:** Minor
- **Intent:** Workflow fires on all review submissions including approvals, not
  just change requests
- **Expected Outcome:** Workflow conditionally runs only for
  `review.state == 'changes_requested'` or comment reviews
- **Files:** `.github/workflows/temper.yml`
- **Priority:** Low
- **Status:** Draft

---

### PBLU-025: Fix double spinner.fail/error pattern in hooks.ts [m-9]

- **Severity:** Minor
- **Intent:** Double `spinner.fail()` + `error()` pattern is fragile across
  inner/outer catch blocks
- **Expected Outcome:** Single error-handling path per catch block; spinner
  failure and error output coordinated
- **Files:** `apps/anvil-cli/src/commands/hooks.ts`
- **Priority:** Low
- **Status:** Draft

---

### PBLU-026: Remove unnecessary CliExit on natural return paths [m-10]

- **Severity:** Minor
- **Intent:** `CliExit()` on natural return paths in audit.ts — should just
  `return`
- **Expected Outcome:** Natural return paths use `return` instead of
  `throw new CliExit()`
- **Files:** `apps/anvil-cli/src/commands/audit.ts`
- **Priority:** Low
- **Status:** Draft

---

### PBLU-027: Fix process.cwd vs getWorkspaceRoot in export.ts [m-11]

- **Severity:** Minor
- **Intent:** `process.cwd()` vs `getWorkspaceRoot()` inconsistency for
  constraint export
- **Expected Outcome:** Uses `getWorkspaceRoot()` consistently
- **Files:** `apps/anvil-cli/src/commands/export.ts`
- **Priority:** Low
- **Status:** Draft
- **Notes:** Related to PBLU-011 (same class of issue)

---

### PBLU-028: Fix admin route substring match in api-client [m-12]

- **Severity:** Minor
- **Intent:** `/admin/` route detection is a substring match —
  `items/admin/notes` would incorrectly match
- **Expected Outcome:** Route detection uses path segment matching, not
  substring
- **Files:** `apps/anvil-cli/src/services/api-client.ts`
- **Priority:** Low
- **Status:** Draft

---

### PBLU-029: Fix DOMException error branch in api-client [m-13]

- **Severity:** Minor
- **Intent:** Non-TimeoutError DOMException falls to wrong error branch
- **Expected Outcome:** DOMException handling distinguishes timeout from other
  types
- **Files:** `apps/anvil-cli/src/services/api-client.ts`
- **Priority:** Low
- **Status:** Draft

---

### PBLU-030: Remove TOCTOU race in auth-store [m-14]

- **Severity:** Minor
- **Intent:** `existsSync` + `readFileSync` is a TOCTOU race — redundant when
  wrapped in try/catch
- **Expected Outcome:** Remove `existsSync` check; rely on try/catch around
  `readFileSync`
- **Files:** `apps/anvil-cli/src/services/auth-store.ts`
- **Priority:** Low
- **Status:** Draft

---

### PBLU-031: Handle trailing slash in getApiUrl [m-15]

- **Severity:** Minor
- **Intent:** `getApiUrl()` may return trailing slash, causing double-slash in
  URL paths
- **Expected Outcome:** URL construction normalises trailing slashes
- **Files:** `apps/anvil-cli/src/services/api-client.ts`
- **Priority:** Low
- **Status:** Draft

---

### PBLU-032: Add spinner-then-throw test for audit-spinner [m-16]

- **Severity:** Minor
- **Intent:** No test for scan-throws-after-spinner-started scenario
- **Expected Outcome:** Test verifies spinner is stopped cleanly when scan
  throws mid-operation
- **Files:** `apps/anvil-cli/src/commands/audit-spinner.test.ts`
- **Priority:** Low
- **Status:** Draft

---

### PBLU-033: Add --yes bypass path test for mcp-config-path [m-17]

- **Severity:** Minor
- **Intent:** `--yes` bypass path not tested
- **Expected Outcome:** Test verifies `--yes` flag skips confirmation prompt
- **Files:** `apps/anvil-cli/src/commands/mcp-config-path.test.ts`
- **Priority:** Low
- **Status:** Draft

---

### PBLU-034: Replace tick() sleeps with flushPromises in tutorial tests [m-18]

- **Severity:** Minor
- **Intent:** `tick()` sleeps 50ms between steps — `flushPromises()` would be
  faster and less flaky
- **Expected Outcome:** `tick()` calls replaced with `flushPromises()` or
  `vi.waitFor`
- **Files:** `apps/anvil-cli/src/commands/tutorial-continuation.test.tsx`
- **Priority:** Low
- **Status:** Draft

---

### PBLU-035: Make currentTopic configurable instead of hardcoded [m-19]

- **Severity:** Minor
- **Intent:** Hardcoded `'core'` as `currentTopic` in Tutorial.tsx and
  NextStepsStep.tsx — should be a prop or documented
- **Expected Outcome:** `currentTopic` passed as prop or documented as
  intentionally hardcoded
- **Files:** `apps/anvil-cli/src/components/Tutorial.tsx`,
  `apps/anvil-cli/src/components/NextStepsStep.tsx`
- **Priority:** Low
- **Status:** Draft

---

### PBLU-036: Verify TemplateGenerator receives prompt values in init tests [m-20]

- **Severity:** Minor
- **Intent:** Interactive tests don't verify `TemplateGenerator` received
  prompt values
- **Expected Outcome:** Tests assert `TemplateGenerator` was called with the
  expected prompt values
- **Files:** `apps/anvil-cli/src/commands/init.test.ts`
- **Priority:** Low
- **Status:** Draft

---

### PBLU-037: Remove unused waitForFrame from test-utils [m-21]

- **Severity:** Minor
- **Intent:** `waitForFrame` duplicates `vi.waitFor` and is currently unused
- **Expected Outcome:** `waitForFrame` removed from test-utils.ts
- **Files:** `apps/anvil-cli/src/test-utils.ts`
- **Priority:** Low
- **Status:** Draft

---

### PBLU-038: Fix stale version and status in beta launch checklist [m-22]

- **Severity:** Minor
- **Intent:** Header says v0.1.0 and "Draft" status post-release
- **Expected Outcome:** Version and status updated to reflect current state
- **Files:** `plans/beta-launch-checklist.aps.md`
- **Priority:** Low
- **Status:** Draft

---

### PBLU-039: Add clarification note for DEFER/TEMPER circular dependency [m-23]

- **Severity:** Minor
- **Intent:** Modules 03 and 04 have a circular dependency that needs a
  clarification note
- **Expected Outcome:** Both modules include a note explaining the dependency
  relationship and resolution order
- **Files:** `plans/modules/03-deferred-finding-filing.aps.md`,
  `plans/modules/04-temper-workflow.aps.md`
- **Priority:** Low
- **Status:** Draft

---

### PBLU-040: Add claude-code-review.yml to README CI/CD section [m-24]

- **Severity:** Minor
- **Intent:** Workflow not listed in the CI/CD documentation section
- **Expected Outcome:** `claude-code-review.yml` added to CI/CD section of
  README
- **Files:** `README.md`
- **Priority:** Low
- **Status:** Draft

---

### PBLU-041: Add tracking issue link for H-1 deferred item [m-25]

- **Severity:** Minor
- **Intent:** Deferred item H-1 in cli-beta-review.md has no tracking issue
  link
- **Expected Outcome:** Tracking issue created and linked, or item marked as
  resolved
- **Files:** `plans/reviews/cli-beta-review.md`
- **Priority:** Low
- **Status:** Draft

---

### PBLU-042: CHANGELOG phantom entry already corrected [m-26]

- **Severity:** Minor
- **Intent:** Root CHANGELOG had phantom `0.2.1-beta.0` prerelease entry and
  misaligned `0.1.1` entry — both corrected during the review
- **Expected Outcome:** No action needed — verified during review. Tracked here
  for completeness.
- **Files:** `CHANGELOG.md`
- **Priority:** Low
- **Status:** Complete
- **Notes:** Corrected in commit 69432e8

---

### Nit Items

### PBLU-043: Fix import ordering in audit.ts [N-1]

- **Severity:** Nit
- **Intent:** Import after `const` assignment; unconventional order
- **Files:** `apps/anvil-cli/src/commands/audit.ts`
- **Priority:** Low
- **Status:** Draft

---

### PBLU-044: Remove orphaned double JSDoc block in hooks.ts [N-2]

- **Severity:** Nit
- **Intent:** Orphaned double JSDoc block
- **Files:** `apps/anvil-cli/src/commands/hooks.ts`
- **Priority:** Low
- **Status:** Draft

---

### PBLU-045: Add CliExit on JSON success in plan/load.ts [N-3]

- **Severity:** Nit
- **Intent:** No `CliExit()` on JSON success path — inconsistent with other
  plan subcommands
- **Files:** `apps/anvil-cli/src/commands/plan/load.ts`
- **Priority:** Low
- **Status:** Draft

---

### PBLU-046: Document non-deterministic hash input in forge.sh [N-4]

- **Severity:** Nit
- **Intent:** Non-deterministic hash input includes `$$` — undocumented
- **Files:** `.claude/hooks/forge.sh`
- **Priority:** Low
- **Status:** Draft

---

### PBLU-047: Add file locking for concurrent APS module appends [N-5]

- **Severity:** Nit
- **Intent:** No file locking for concurrent APS module appends in
  forge-defer.sh
- **Files:** `.claude/hooks/forge-defer.sh`
- **Priority:** Low
- **Status:** Draft

---

### PBLU-048: Remove redundant COMPLETED_AT assignment in forge-report.sh [N-6]

- **Severity:** Nit
- **Intent:** `COMPLETED_AT` is redundant re-assignment of `TIMESTAMP`
- **Files:** `.claude/hooks/forge-report.sh`
- **Priority:** Low
- **Status:** Draft

---

### PBLU-049: Document that --no-verify skips all hooks, not just Forge [N-7]

- **Severity:** Nit
- **Intent:** `--no-verify` skips all hooks, not just Forge — undocumented
- **Files:** `.claude/agents/forge-reviewer.md`
- **Priority:** Low
- **Status:** Draft

---

### PBLU-050: Remove duplicate .claude/worktrees/ entry in .gitignore [N-8]

- **Severity:** Nit
- **Intent:** Duplicate `.claude/worktrees/` entry at lines 57 and 70
- **Files:** `.gitignore`
- **Priority:** Low
- **Status:** Draft

---

### PBLU-051: Add reason for claude-code-review.yml prettierignore [N-9]

- **Severity:** Nit
- **Intent:** Terse reason for `claude-code-review.yml` exclusion in
  prettierignore
- **Files:** `.prettierignore`
- **Priority:** Low
- **Status:** Draft

---

### PBLU-052: Fix footnote markers in README [N-10]

- **Severity:** Nit
- **Intent:** Footnote markers `^1/^2/^3` don't render as superscripts in GFM
- **Files:** `README.md`
- **Priority:** Low
- **Status:** Draft

---

### PBLU-053: Rename tutorial-continuation test file to match describe block [N-11]

- **Severity:** Nit
- **Intent:** File name doesn't match sole remaining describe block after
  consolidation
- **Files:** `apps/anvil-cli/src/commands/tutorial-continuation.test.tsx`
- **Dependencies:** PBLU-016 (consolidate duplicate tests first)
- **Priority:** Low
- **Status:** Draft

---

### PBLU-054: Clarify index mapping comment in tutorial-picker test [N-12]

- **Severity:** Nit
- **Intent:** Inline comment at line 93 could be clearer about index mapping
- **Files:** `apps/anvil-cli/src/commands/tutorial-picker.test.tsx`
- **Priority:** Low
- **Status:** Draft

---

### PBLU-055: Use toHaveBeenCalledTimes(3) in init.test.ts [N-13]

- **Severity:** Nit
- **Intent:** `toHaveBeenCalled()` should be `toHaveBeenCalledTimes(3)` for
  specificity
- **Files:** `apps/anvil-cli/src/commands/init.test.ts`
- **Priority:** Low
- **Status:** Draft

---

### PBLU-056: Move name before on in temper.yml [N-14]

- **Severity:** Nit
- **Intent:** `name:` before `on:` is unconventional for GitHub Actions
- **Files:** `.github/workflows/temper.yml`
- **Priority:** Low
- **Status:** Draft
- **Notes:** Actually `name:` before `on:` is the GitHub convention — review
  finding may be inverted. Verify before acting.

---

### PBLU-057: Fix codexAgreed Required field in forge-reviewer.md [N-15]

- **Severity:** Nit
- **Intent:** `codexAgreed` Required should be `yes (default false)`, not
  current value
- **Files:** `.claude/agents/forge-reviewer.md`
- **Priority:** Low
- **Status:** Draft

---

## Stats

| Severity | Count | Wave |
|----------|-------|------|
| Critical | 1     | 1    |
| Major    | 15    | 1–3  |
| Minor    | 26    | 3    |
| Nit      | 15    | 3    |
| **Total** | **57** | —  |

| Wave | Items | Scope |
|------|-------|-------|
| 1 — Before next beta push | PBLU-001 through PBLU-004 | 4 |
| 2 — Before enabling Forge | PBLU-005 through PBLU-010 | 6 |
| 3 — Before GA | PBLU-011 through PBLU-057 | 47 |
