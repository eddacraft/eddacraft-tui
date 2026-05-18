<!--
APS Module: Continuous Improvement Backlog
==========================================
Standing intake for concrete improvement work identified across the project.
This module intentionally remains active while the project is active.
-->

# Continuous Improvement Backlog

| ID  | Owner | Status      | Progress |
| --- | ----- | ----------- | -------- |
| CIB | —     | In Progress | 3/7      |

## Purpose

Capture concrete improvement opportunities identified anywhere in the project so
they are not lost between feature modules, reviews, releases, incidents, and
documentation closeout.

## Standing Module Policy

This is a standing APS module. It does not close merely because all currently
listed items are done. Keep it active while the project is active, append new
items as they are identified, and only archive it if a future APS decision
explicitly replaces the intake model.

Progress stays numeric for APS drift tooling. Use `0/0` while no items are
listed, then update it to `done/total` as `CIB-NNN` items are added.

## In Scope

- Cross-project cleanup and quality improvements that do not fit a more specific
  active module
- Follow-up work discovered during reviews, releases, debugging, docs closeout,
  or routine implementation
- Small operational, developer-experience, test, documentation, and maintenance
  improvements with clear expected outcomes
- Candidate work that needs triage before promotion into a specialist module

## Out of Scope

- Vague ideas without an observable outcome
- Product features large enough to need their own APS module
- Work already owned by a specific active module
- Duplicating issue trackers or PR review threads without distilling an
  executable APS item

## Intake Rules

Add a new `CIB-NNN` item when an improvement has:

- A one-sentence intent
- An expected outcome or observable acceptance condition
- A validation command, manual check, or explicit reason validation is not yet
  known
- Best-effort source context, such as the review, release, incident, file path,
  or module where it was identified

When a cluster becomes large or domain-specific, promote it into a dedicated APS
module and leave a short `Superseded by:` note on the original CIB item.

## Cross-Cutting Convention

This is a cross-cutting APS module and follows the rules in
[`plans/aps-rules.md#cross-cutting-modules`](../aps-rules.md#cross-cutting-modules).
Task closeout must sweep `Coordinates with:`, `Blocks on:`, `Supersedes:`, and
`Superseded by:` callouts rather than carrying unresolved references into
archive.

## Item Template

```markdown
### CIB-NNN: Short outcome-focused title

- **Status:** Draft
- **Intent:** One sentence describing the improvement outcome.
- **Expected Outcome:** Observable result or acceptance condition.
- **Validation:** `command` or manual check.
- **Identified From:** Review, release, incident, module, or file path.
- **Confidence:** low | medium | high
```

## Tasks

### CIB-001: Sweep global `dev-workflow` skill for post-cutover and current-council drift

- **Status:** Complete
- **Intent:** Bring the global `dev-workflow` routing skill into alignment with
  the main-first cutover and the current risk-tiered council architecture.
- **Expected Outcome:** `~/Projects/src/code-env/.claude/skills/dev-workflow/SKILL.md`
  no longer instructs branching from `dev`, the Stage Map references the current
  council and skill set (risk-tiered `council`, `local-review-council` for the
  streaming flow, `planning-council`), and adjacent stages
  (`addressing-pr-reviews`, `finishing-a-branch`, `release`) are linked where
  relevant. Skill is consistent with `AGENTS.md` and current APS lifecycle.
- **Validation:** Manual diff of the updated skill against the main-first
  cutover artefacts (`docs/guides/branching-strategy.md`,
  `docs/guides/worktree-policy.md`) and the current council skill; `pnpm
  format:check` if any in-repo doc is touched alongside.
- **Identified From:** Session review 2026-05-11 — OPMODEL-012 archive closed
  without sweeping `dev-workflow`; skill still says "Branch from `dev`" at line
  32 and points "Review" only at the legacy `code-review` skill plus `/council`
  command despite the newer streaming/batch council model.
- **Coordinates with:** DOCGOV-008 (stale entrypoints), CIB-002 (canonical skill
  list), `plans/specs/2026-05-09-agentic-execution-ecosystem-architecture.md`
  (skill authority boundaries).
- **Evidence:** Anvil PR #1443 (vendored repo-local copy at
  `.claude/skills/dev-workflow/SKILL.md`, merged 2026-05-11); follow-up review
  fixes in commit `ce4091cf` aligned the skill to the repo-local `quick|mini|full`
  council tiers and added a Surface Inventory section. Companion code-env PR
  `joshuaboys/code-env#20` covers the upstream global skill — open at closeout
  time; tracked separately.
- **Confidence:** high

### CIB-002: Establish definitive skill and agent list for the anvil repo

- **Status:** In Progress
- **Intent:** Produce a single authoritative inventory of the skills and agents
  this repository expects to be available, distinguishing repo-local from global
  surfaces and recording authority and source for each entry.
- **Expected Outcome:** A checked-in list (location decided during triage —
  candidates: `docs/guides/agent-surface-inventory.md` or a section inside
  `AGENTS.md`) names every skill and agent the anvil workflow depends on, marks
  repo-local versus global, identifies the canonical source for each global
  entry (e.g. `joshuaboys/code-env`), and is linked from `AGENTS.md`. Drift
  between this list and `.claude/` plus external skill repos is detectable by a
  documented manual check until automated validation is added.
- **Validation:** Manual inventory cross-check against `.claude/agents/`,
  `.claude/skills/` (where present), the global Claude skill directory, and
  current `AGENTS.md` references; `pnpm format:check` for any in-repo docs
  touched.
- **Identified From:** Session review 2026-05-11 — repeated drift between
  expected skills (e.g. `dev-workflow`, `council`, `release`) and what is
  current or correct, with no single source of truth available to detect it.
- **Coordinates with:** CIB-001 (drift sweep informs entries), DOCGOV-002
  (taxonomy and metadata),
  `plans/specs/2026-05-09-agentic-execution-ecosystem-architecture.md`
  (Phase 1: Inventory And Declare Authority).
- **Confidence:** medium

### CIB-003: Harden PR remediation against partial closure

- **Status:** Complete
- **Intent:** Ensure PR review remediation does not stop after fixing only CI,
  only review comments, or only merge conflicts.
- **Expected Outcome:** Repo-local `addressing-pr-reviews` skills for Claude,
  OpenCode, and Codex require a bounded closure loop that re-inventories CI,
  unresolved review threads, and mergeability after every push/rebase/thread
  resolution, and the dev workflow routes PR feedback through that loop.
- **Validation:** Manual cross-check of `.claude/skills/addressing-pr-reviews/SKILL.md`,
  `.opencode/skills/addressing-pr-reviews/SKILL.md`,
  `.codex/skills/addressing-pr-reviews/SKILL.md`, and both `dev-workflow` skill
  variants; targeted `pnpm exec oxfmt --check <changed files>` and
  `git diff --check` for formatting/whitespace. Full `pnpm format:check` was
  blocked by unrelated untracked OpenCode files in the local worktree.
- **Identified From:** User report 2026-05-13 that agents addressing a stream of
  PRs were fixing one blocker class while leaving CI, review comments, or merge
  conflicts unresolved, causing repeated token-heavy reruns.
- **Coordinates with:** CIB-002 (agent surface inventory) and repo-local
  `addressing-pr-reviews` skill authority.
- **Confidence:** high

### CIB-004: Simplify admin-key retrieval with credential-source config

- **Status:** Complete
- **Intent:** Make routine admin CLI use easier without storing plaintext admin
  keys by letting operators configure where the key should be retrieved from.
- **Expected Outcome:** `anvil admin auth set 1password <op-reference>` writes
  an owner-only local credential-source config, `anvil admin auth status` reports
  the configured source without revealing the key, `anvil admin auth unset`
  removes it, and normal `anvil admin` commands resolve the key from
  `ANVIL_ADMIN_KEY` first or from `op read <reference>` otherwise.
- **Validation:** `cargo test -p eddacraft-anvil commands::admin`; `cargo fmt
  --check`; `pnpm format:check`; manual source check against issue #952.
- **Identified From:** GitHub issue #952, deferred from ADMINCLIH v1 as
  "Admin CLI: keychain-integrated local key storage".
- **Coordinates with:** Archived `ADMINCLIH` module out-of-scope note; this item
  intentionally records retrieval metadata only, not plaintext admin keys or OS
  keychain integration.
- **Files:** `crates/anvil-cli/src/commands/admin.rs`,
  `docs/runbooks/admin-cli.md`.
- **Evidence:** `anvil admin auth set 1password <op-reference>` stores only the
  retrieval reference, normal admin commands resolve it with `op read` when
  `ANVIL_ADMIN_KEY` is absent, and the runbook documents setup/status/unset.
- **Confidence:** high

### CIB-005: Pre-write validator patch-mode support

- **Status:** In Progress
- **Intent:** Stop forcing agents to ship full `proposedContent` to
  `anvil_validate_write` when they only need to apply a small edit; accept a
  diff/patch payload instead so token cost scales with the change, not the file.
- **Expected Outcome:** `anvil_validate_write` invoked with `patch` and no
  `content` no longer returns the `patch-only-unsupported` error currently
  hard-coded at
  `crates/anvil-cli/src/mcp/tools/validate_write.rs:621-625`. The validator
  reads the on-disk file at the `workspaceRoot`-relative path, applies the
  patch to produce the post-image buffer, then feeds that buffer through the
  existing `ProposedChange` (`crates/anvil-intercept/src/enforcement.rs:32`)
  and `EnforcementPipeline::diagnostics_for_proposed_changes()`
  (`crates/anvil-intercept/src/enforcement.rs:70`) pipeline — every existing
  rule continues to fire against the post-image, with no rule-side changes.
  Patch format (unified diff vs. structured edit ops), line-ending behaviour,
  and the read-then-apply race window have documented semantics. The existing
  rejection test (`patch_only_blocks_as_unsupported` at
  `crates/anvil-cli/src/mcp/tools/validate_write.rs:1312`) flips to a positive
  acceptance test, and a new test covers the original screenshot case: a
  one-string rename inside a 2770-line JSON file succeeds without
  `proposedContent`.
- **Validation:** `cargo test -p anvil-cli mcp::tools::validate_write`;
  `cargo test -p anvil-intercept enforcement`; manual end-to-end check via the
  MCP tool with a patch-only payload against a >20k-token fixture, confirming
  the call succeeds, the diagnostics returned match what full-content
  validation would have produced, and the disk file is unchanged (validator
  is read-only).
- **Identified From:** Beta tester screenshot 2026-05-18 — agent hit its
  single-Read budget on a 2770-line (~25.6k token) JSON metadata file for a
  one-string tag rename at idx 394 and was forced into a stop-and-ask. The
  validator already parses the `patch` field (detection at
  `crates/anvil-cli/src/mcp/tools/validate_write.rs:589-602`) but rejects it
  deliberately rather than for any architectural reason; the `contentSha256` +
  `preview` slim-payload design intent in the same file's descriptor comment
  signals patch-mode was planned and deferred, not abandoned.
- **Files:** `crates/anvil-cli/src/mcp/tools/validate_write.rs`,
  `crates/anvil-cli/src/mcp/validation.rs` (entry at line 432),
  `crates/anvil-intercept/src/enforcement.rs` (consumer; no expected changes).
- **Coordinates with:** CIB-006 (risk-tiered validation builds on this — once
  patches are first-class, the safelist tier can dispatch on patch shape).
- **Out of Scope:** The same screenshot also surfaced the
  `untrusted-workspace-root` gate
  (`crates/anvil-cli/src/mcp/tools/validate_write.rs:699-702`). Tracked
  separately as CIB-007 and intended to ship in the same hotfix tag so the
  beta tester's full friction surface clears in one cut.
- **Confidence:** high

### CIB-006: Risk-tiered validation for trivial edits

- **Status:** Proposed
- **Intent:** Even with patch-mode (CIB-005) in place, full pipeline
  validation is overkill for genuinely trivial changes. Add a lightweight
  validator tier so a defined safelist of change shapes short-circuits the
  rules that cannot possibly apply, dropping both latency and token cost on
  the hot path.
- **Expected Outcome:** A documented safelist (initial entries: single-string
  value rename inside a JSON file at a stable path, no key add/remove, no
  structural change) is matched against the parsed patch before the full
  pipeline runs. When a change matches, only rules whose declared inputs
  overlap the touched node fire; the remainder are skipped with a recorded
  reason. Out-of-safelist edits continue to run the full pipeline unchanged.
  The decision (full vs. tiered) is surfaced in the validator response so
  callers can audit. Out-of-safelist behaviour and safelist criteria are
  documented in `crates/anvil-cli/src/mcp/tools/validate_write.rs` and
  cross-linked from the validator README.
- **Validation:** `cargo test -p anvil-cli mcp::tools::validate_write`
  covering safelist hits and near-miss cases that must fall through to full
  validation. Benchmark in `crates/anvil-bench` showing meaningful
  wall-time drop on the safelist path against a representative JSON metadata
  fixture; regression test ensuring no rule that *should* fire is skipped on
  any safelist shape.
- **Identified From:** Beta tester screenshot 2026-05-18 — same incident as
  CIB-005. The change was a one-string rename at idx 394; even patch-mode
  alone still validates the whole post-image, so risk-tiering is the natural
  follow-up to push trivial cases below the cost floor entirely.
- **Files:** `crates/anvil-cli/src/mcp/tools/validate_write.rs`,
  `crates/anvil-intercept-rules/src/lib.rs` (rule input declarations at
  `RuleInput` line 51 will likely need a "touched-node" predicate to support
  selective firing).
- **Coordinates with:** CIB-005 (must land first — without patch-mode there
  is no cheap way to know which node was touched).
- **Confidence:** medium — the plumbing is straightforward but the policy
  decision (which shapes are "safe enough" to skim) carries real
  under-validation risk and needs sign-off before the safelist grows.

### CIB-007: Untrusted-workspace-root preflight gate is unrecoverable for legitimate callers

- **Status:** In Progress
- **Triage Decision (2026-05-18):** Option **(b)** — keep the strict
  equality check, add an `expectedWorkspaceRoot` field to the
  `untrusted-workspace-root` error payload so the caller can
  self-correct on the next call. Option (a) (worktree-aware accept) is
  the more leveraged fix but widens the trust boundary and needs an
  ADR per `docs/guides/adr-process.md`, which is out of scope for the
  CIB-005 hotfix this item is paired with. Option (a) can be opened
  as a follow-up CIB if (b)'s recoverability proves insufficient in
  practice.
- **Intent:** Stop the `untrusted-workspace-root` MCP preflight from
  rejecting agents that pass a legitimate sibling `workspaceRoot`
  (worktree siblings, monorepo sub-packages, macOS `/private`-prefixed
  symlink variants) without giving the caller enough information to
  recover without a human round-trip.
- **Expected Outcome:** The strict equality check at
  `crates/anvil-cli/src/mcp/tools/validate_write.rs:696-703` is updated
  along one of two paths, chosen during triage:
  - **(a) Relax to worktree-aware accept:** any path that canonicalises
    to a Git worktree linked to the shim's primary working tree is
    accepted. The trust boundary (no traversal outside the shim's tree)
    is preserved; symlink resolution still goes through
    `canonical_workspace_root()` at line 713.
  - **(b) Keep the strict check but return a recoverable error:** the
    `ToolProblem` payload includes the expected `workspaceRoot` (the
    shim's canonicalised cwd) so the caller can retry with the right
    value on the next call without operator intervention.
  Either way the existing positive case (no `workspaceRoot` provided, or
  exact match after canonicalisation) continues to pass, and the
  rejection test at
  `crates/anvil-cli/src/mcp/tools/validate_write.rs:1470` is updated to
  cover whichever resolution lands. The
  `docs/architecture/mcp-shim-as-built.md` request-shape note (line 160)
  is updated to match.
- **Validation:** `cargo test -p anvil-cli mcp::tools::validate_write`;
  manual end-to-end check from a worktree at
  `~/Projects/src/anvil-001-<branch>` against an MCP shim launched in
  `~/Projects/src/anvil-001`, confirming the call now either succeeds
  (option a) or fails with a recoverable payload that names the expected
  root (option b); `pnpm format:check` for any in-repo doc touched.
- **Identified From:** Beta tester screenshot 2026-05-18 — same incident
  as CIB-005. After patch-mode was worked around, the same agent then
  tripped `untrusted-workspace-root`
  (`crates/anvil-cli/src/mcp/tools/validate_write.rs:699-702`) because
  its understood `workspaceRoot` did not canonicalise to the shim's
  launch cwd. The gate is correct as a trust boundary, but its current
  error text is unrecoverable: the agent cannot know what value would
  satisfy the shim short of asking the operator, so the friction surface
  rolls back to "stop and ask".
- **Files:**
  `crates/anvil-cli/src/mcp/tools/validate_write.rs` (`workspace_root()`
  at line 680, `canonical_workspace_root()` at line 713, rejection test
  at line 1470); `docs/architecture/mcp-shim-as-built.md` (request-shape
  table at line 160); `plans/decisions/DECISION-LOG.md` if option (a) is
  chosen, because that path widens the trust boundary and needs an ADR
  per `docs/guides/adr-process.md`.
- **Coordinates with:** CIB-005 (same beta incident, same file; both
  should ship in the same hotfix tag so the tester's full friction
  surface clears in one cut, not two), CIB-006 (out of scope here —
  risk-tiering does not depend on this gate).
- **Out of Scope:** Broader MCP trust-boundary review (auth, multi-root
  workspaces, daemon-mediated workspace registration). Anything beyond
  worktree-sibling recognition and recoverable error payloads belongs in
  a dedicated APS module, not a hotfix-eligible CIB item.
- **Confidence:** medium — option (b) is high-confidence and small;
  option (a) is higher-leverage but its trust-boundary widening needs an
  ADR before code lands. Triage selects between them.
