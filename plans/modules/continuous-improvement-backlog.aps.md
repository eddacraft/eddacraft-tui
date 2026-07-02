<!--
APS Module: Continuous Improvement Backlog
==========================================
Standing intake for concrete improvement work identified across the project.
This module intentionally remains active while the project is active.
-->

# Continuous Improvement Backlog

| ID  | Owner | Status      | Progress |
| --- | ----- | ----------- | -------- |
| CIB | —     | In Progress | 109/154  |

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
[`plans/aps-rules.md#module-types-vertical-and-conductor`](../aps-rules.md#module-types-vertical-and-conductor).
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

## Work Items

> **Completed items are compacted.** Items in a done state (`Done` / `Complete`
> / `Merged` / `Released/Shipped` — the `DONE_PATTERNS` in
> `scripts/aps/lib/modules.mjs`) are kept as a heading + `Status:` + a one-line
> `Summary:`.
> Their full Intent / Validation / Resolution detail lives in git history and,
> in brief, in the `CIB` row of [`plans/index.aps.md`](../index.aps.md). The
> `done/total` count is derived from these statuses by
> `scripts/aps/index-counts.mjs` (CIB-022), so a compacted item still counts.

### CIB-001: Sweep global `dev-workflow` skill for post-cutover and current-council drift

- **Status:** Done
- **Summary:** Aligned the global `dev-workflow` skill with the main-first
  cutover and the risk-tiered council model (Anvil PR #1443; review fixes
  `ce4091cf`). Upstream global skill tracked via `joshuaboys/code-env#20`.

### CIB-002: Establish definitive skill and agent list for the anvil repo

- **Status:** Merged 2026-05-12 via PR #1453
- **Summary:** Authoritative skill/agent/command inventory shipped at
  `docs/guides/agent-surface-inventory.md` (PR #1453, `7c59b2ee`) — it marks
  repo-local versus global surfaces, names the canonical source for each global
  entry (`joshuaboys/code-env`), and is linked from `AGENTS.md`. Drift is caught
  by a documented manual cross-check; automated inventory validation is separate
  follow-up. Status reconciled 2026-05-29 (the work shipped but was left
  `In Progress`).

### CIB-003: Harden PR remediation against partial closure

- **Status:** Done
- **Summary:** Repo-local `addressing-pr-reviews` skills (Claude/OpenCode/Codex)
  now require a bounded closure loop that re-inventories CI → review threads →
  mergeability after every push/rebase, so remediation can't stop after one
  blocker class.

### CIB-004: Simplify admin-key retrieval with credential-source config

- **Status:** Done
- **Summary:** `anvil admin auth set/status/unset` configure a credential
  *source* (e.g. `op read <reference>`) so routine admin use never stores a
  plaintext key; commands resolve `ANVIL_ADMIN_KEY` first, else the source
  (issue #952).

### CIB-005: Pre-write validator patch-mode support

- **Status:** Released/Shipped via v0.7.0-beta (d7873161 · 2026-05-21)
- **Summary:** `anvil_validate_write` accepts a `patch` payload with no full
  `content` — it applies the patch to the on-disk file and runs the post-image
  through the normal enforcement pipeline (no rule changes), so token cost
  scales with the edit, not the file. PR #1692 (`3a647d4b`). CIB-006 (risk
  tiering) builds on it.

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

- **Status:** Released/Shipped via v0.7.0-beta (d7873161 · 2026-05-21)
- **Summary:** The `untrusted-workspace-root` MCP preflight error now returns
  the expected `workspaceRoot` (option **(b)**, strict check kept) so a caller
  passing a sibling/worktree root can self-correct without a human round-trip.
  PR #1692 (`3a647d4b`), same hotfix as CIB-005. Worktree-aware accept (option
  a) deferred — it widens the trust boundary and needs an ADR.

### CIB-008: `anvil check` planless path runs only `architecture`, ignoring `.anvilrc`

- **Status:** Merged 2026-05-21 via PR #1817 (issue #1797)
- **Summary:** The planless `anvil check` dispatcher now routes through the same
  `.anvilrc#checks` reader as `anvil gate` and runs the planless-eligible set
  (`secret-detection` + `antipattern-scan`), so the marquee `anvil check
  src/smelly.ts` demo no longer silently passes a hardcoded `sk-…` key
  (`crates/anvil-cli/src/commands/check.rs`, `PLANLESS_ELIGIBLE_CHECKS`).

### CIB-009: `anvil audit` and `anvil gate` disagree on the same repo

- **Status:** Merged 2026-05-21 via PR #1814 (issue #1798)
- **Summary:** `anvil audit` now runs the canonical `secret-detection` check
  over the same tree as `anvil gate` (with gate-aligned file extensions), so the
  two can no longer disagree — audit no longer reports "0 issues" on a repo with
  a planted key (`crates/anvil-cli/src/commands/audit.rs`). Sibling of CIB-008.

### CIB-010: `anvil watch` first-scan emits a wall of `public-api-expansion` against existing symbols

- **Status:** Merged 2026-05-21 via PR #1816 (issue #1802; behaviour fixed by WATCHUX-001, PR #1816 adds the regression test)
- **Summary:** On a never-baselined repo, the initial graph is now treated as
  the baseline (behaviour fixed by WATCHUX-001 — only post-scan modifications go
  through the policy engine), so `anvil watch`'s first scan no longer flags every
  pre-existing public symbol as `public-api-expansion`. PR #1816 added the
  multi-file regression test `audit_1802_multi_file_initial_scan_emits_no_public_api_violations`
  in `crates/anvil-kernel/src/watch.rs`.

### CIB-011: `anvil gate -p ai` fails strict-mode checks on missing configs without next-step guidance

- **Status:** Merged via PR [#1818](https://github.com/eddacraft/anvil-001/pull/1818) (merged 2026-05-21 at `acc4db6f`)
- **Summary:** On a fresh repo, `anvil gate -p ai` no longer FAILs purely
  because config files don't exist yet — missing-config is info-level and the
  score grades against available checks, with a `next:` hint (issue #1803,
  new-user journey audit finding #9).

### CIB-012: `anvil check --staged` errors with "`--changed` required"

- **Status:** Merged via PR [#1813](https://github.com/eddacraft/anvil-001/pull/1813) (merged 2026-05-21 at `ce0bd32b`)
- **Summary:** `anvil check --staged` (and `--since`) now imply `--changed`
  instead of erroring with "`--changed` required" (issue #1804, new-user
  journey audit finding #10; `crates/anvil-cli/src/commands/check.rs`).

### CIB-013: Add agent continuous-improvement closeout to dev-workflow

- **Status:** Done
- **Summary:** Repo-local OpenCode + Claude `dev-workflow` skills now require a
  compact continuous-improvement note before the final response on non-trivial
  tasks and point at the shared evidence log
  (`plans/reviews/continuous-improvement-log.md`) rather than a second backlog.

### CIB-014: SARIF output for `anvil check` / `anvil gate` / `anvil audit`

- **Status:** Done
- **Superseded by:** [`sarif-output`](../archive/modules/sarif-output.aps.md) (SARIFOUT module,
  Proposed) on 2026-05-29 — promoted into a dedicated APS module per the CIB
  intake rule; the `Superseded by:` label matches this module's cross-cutting
  convention sweep.
- **Summary:** Promoted out of the backlog into a dedicated APS module per the
  CIB intake rule ("promote into a dedicated APS module"). The 2026-05-29
  design pass
  ([`plans/specs/2026-05-29-sarif-output-design.md`](../specs/2026-05-29-sarif-output-design.md))
  resolved the three readiness gates that deferred this item: flag surface
  (`--format` value-enum with `--json` kept as a backward-compatible alias;
  SARIF never auto-selected by TTY), module home (new dedicated SARIFOUT module
  under Engineering Platform — explicitly not EXPORT, which is a telemetry sink
  per ADR-035, and not COMPLY, which SARIF is upstream of), and shared finding
  model (a thin shared SARIF emitter plus per-command adapters; no
  `anvil-checks`/`anvil-policy-engine`/`anvil-rules` refactor — SARIF itself is
  the shared target shape). Scoped to the GitHub Code Scanning subset of SARIF
  2.1.0 (results/rules/locations/suppressions) and split into six single-purpose
  work items across four waves (SARIFOUT-001..-006); schema-validation tests are
  in-repo/CI, the GitHub upload check is manual/out-of-band. Two candidate ADRs
  (`--format` value-enum convention; shared-emitter/no-finding-model decision)
  flagged for sign-off. Execution tracking now lives in SARIFOUT-001..-006.

### CIB-015: Triage `anvil bom` surface before filing as APS

- **Status:** Merged 2026-05-26 via PR #1995
- **Summary:** Triaged 2026-05-26 →
  [`anvil-bom-surface`](../brainstorms/2026-05-26-anvil-bom-surface.md).
  **Decline to file an APS item now.** Of five slices, three survive the
  scope-guard as a read-only _view_ over existing production-wired collectors
  (agents via the detected-agents cache; policy refs via `anvil policy
  list`/bundles; witness/protection structural summary); MCP-server inventory
  and credential-reference registry are **rejected** (both need new,
  scope-creeping collectors — credentials additionally sensitive);
  controlled-actions **defers to AGOV-007**. The shape that earns its place is a
  view + `--diff` drift gate (new-edges-only), not the inventory itself. View,
  not collector: it must add no detectors, and the §8 "view over the witness
  chain" option is unavailable for the agent slice today (`ProtectionClaim`
  carries no agent attribution; witness-line `agent_tag` is not reliably
  persisted). Slot when filed: a new `AGOV-NNN` under
  `agent-governance-patterns.aps.md` — not a new module, not AGOV-007; ADOPT
  (`adoption-friction`) is Complete/archived, so not there either. Trigger to file: AGOV leaves the launch parking
  lot **and** a concrete `--json`/drift consumer appears.

### CIB-016: Name "current posture vs new regression" in baseline output

- **Status:** Merged 2026-06-03 via PR #2270
- **Intent:** `anvil baseline` + `cutoff_commit` already
  distinguish first-scan posture from new regressions mechanically.
  The UX doesn't name that distinction. Adding the phrasing turns
  a hidden invariant into a teachable principle.
- **Expected Outcome:** First scan on an established repo reads
  "current posture — N findings, baselined as-is." Subsequent
  scans read "new regressions — M findings since baseline." Maps
  onto Anvil's stated "warnings over blocks; new edges only"
  principle (per `.claude/rules/architecture.md`). Wording lands
  in `anvil baseline`, `anvil check` (when run against a baselined
  repo), and the wow-start tutorial copy.
- **Validation:** Manual cross-check that the three surfaces emit
  the new phrasing; `pnpm format:check`; CLI snapshot tests on
  baseline/check output updated to the new wording.
- **Identified From:** [2026-05-24 Drako borrow assessment](../brainstorms/2026-05-24-drako-borrow-assessment.md)
  §4 Borrow C — pure framing borrow, no code mechanics change.
- **Files:** `crates/anvil-cli/src/commands/baseline.rs`,
  `crates/anvil-cli/src/commands/check.rs`,
  wow-start tutorial copy (location TBD at implementation time).
- **Coordinates with:** ADOPT (wow-start surface, Merged 6/6),
  CIB-010 (Merged — `anvil watch` first-scan public-api-expansion wall;
  same first-scan-vs-steady-state class of UX gap).
- **Confidence:** high — docs / output-string change; no behaviour
  change. Lowest-risk of the three Drako borrows.

### CIB-017: Tracing on the `anvil policy eval` path

- **Status:** Merged 2026-05-26 via PR #1983
- **Summary:** `eval::run` carries `#[tracing::instrument]` (policy/query span)
  and a `debug!` summary (policy_bytes / input_bytes / eval_ms / findings /
  exit_code), with `warn!`s on the gate-relevant failure paths and on engine
  abnormal conditions (caught panic, poisoned lock). Surfaces under
  `ANVIL_LOG=debug` via `anvil-observability` (JSON to stdout). Added `tracing`
  to the policy-engine crate (already in the binary tree; no new crates).

### CIB-018: `catch_unwind` at the policy-engine facade boundary

- **Status:** Merged 2026-05-26 via PR #1980
- **Summary:** `Engine::guard` wraps every regorus call in
  `catch_unwind(AssertUnwindSafe(..))`, converting a panic on an adversarial
  policy into `EngineError::Regorus` + poisoning the engine, so `anvil policy
  eval` returns a structured error/non-zero exit instead of aborting. Required
  flipping the CLI from `panic = "abort"` to `"unwind"` (**ADR-051**) — without
  it `catch_unwind` is a no-op in the shipped binary.

### CIB-019: Surface Go OPA stderr in the parity gate

- **Status:** Merged 2026-05-26 via PR #1990
- **Summary:** `scripts/bench-vs-go-opa.sh` now captures `opa bench` stderr to a
  temp file (trap-cleaned) and surfaces it before the `require_pos_num` bail, so
  an OPA error (parse failure, crash, version skew) reaches the operator instead
  of a bare "no positive measurement". `require_pos_num` gained an optional 4th
  arg pointing at the stderr file. New fixture test `bench-vs-go-opa.test.sh`
  stubs `opa` + the harness and asserts both the happy-path `GATE: PASS` and the
  opa-error path (exit 2, OPA's text surfaced); wired into CI script-fixtures.
  From the POLENG full council (operations + adversarial seats), 2026-05-25.

### CIB-020: Release-prep must refresh version-embedding TUI snapshots

- **Status:** Merged 2026-05-25 via PR #1961
- **Summary:** anvil-tui's shell watermark renders a fixed `X.Y.Z` placeholder
  in test builds (`VERSION` is `CARGO_PKG_VERSION` under `cfg(not(test))`, the
  placeholder under `cfg(test)`), so ~38 snapshots are version-agnostic and a
  release version bump no longer reddens `main`. `version_matches_workspace` →
  `production_watermark_uses_cargo_pkg_version`. Surfaced reactively by PR #1959
  during the POLENG-009 rebase.

### CIB-021: Append-only CI log should not produce merge conflicts

- **Status:** Merged 2026-05-26 via PR #1967
- **Summary:** `plans/reviews/continuous-improvement-log.md` is `merge=union` in
  `.gitattributes`, so concurrent appends from parallel agents/worktrees merge
  without conflict markers (reuses the witness-NDJSON-log pattern). Entry
  convention requires a trailing blank line so unioned entries don't abut.

### CIB-022: Derive APS index progress counts instead of hand-editing

- **Status:** Merged 2026-05-26 via PR #1969
- **Summary:** `scripts/aps/index-counts.mjs` derives each managed module's
  `done/total` from its work-item `Status:` lines and rewrites the module header
  + the `plans/index.aps.md` count token; `aps:index:check` originally enforced
  freshness in Docs Lint (exit 1 on drift). **Superseded for freshness by
  ADR-053/CIB-025 (2026-06-27):** `--check` now reports drift advisedly (exit 0).
  Parser shared with `drift-check.mjs` via
  `scripts/aps/lib/modules.mjs`. Headerless/archived rows untouched; annotation
  prose preserved (so it kills count drift, not same-module prose conflicts).

### CIB-023: Detect implemented-but-unreconciled APS items

- **Status:** Draft
- **Intent:** Surface APS items that are done in code but still marked `Draft` /
  `Proposed` / `In Progress`. This drift is currently invisible: the
  CIB-008/009/010 fixes from the 2026-05-21 new-user-journey audit had landed
  (CIB-008 #1817, CIB-009 #1814; CIB-010's behaviour came from WATCHUX-001 with
  regression coverage in #1816) but the items stayed `Draft` until a human
  grepped the dispatcher while picking the next item.
- **Expected Outcome:** An advisory check (extend `scripts/aps/drift-check.mjs` or
  a sibling) that, for each non-done item carrying a `Tracking:` GH issue, looks
  for evidence the issue was resolved — a merged PR linked to the issue, or a
  `Fixes #N` / `fix(...): … #N` reference in merged history — and warns
  `aps-item-implemented-but-draft` so the item can be reconciled. Advisory
  (exit 0), consistent with the rest of `drift-check`.
- **Validation:** a fixture where a non-done item's `Tracking:` issue has a
  merged-PR reference emits the warning; an item with no such reference does not;
  done items are never flagged.
- **Identified From:** CIB-018 session 2026-05-26 — reconciling CIB-008/009/010
  (PR #1977) surfaced that their fixes had merged but the items were left Draft;
  the drift was invisible to every existing check until grepped by hand.
- **Files:** `scripts/aps/drift-check.mjs` (or a new `scripts/aps/` check); likely
  needs GH API access (issue→PR links) or a merged-commit-message scan in CI.
- **Coordinates with:** CIB-022 (APS-count derivation — same "keep the index
  honest about reality" goal, different drift class).
- **Confidence:** low — the resolution heuristic needs a reliable signal
  (GH issue↔PR links vs commit-message grep) and may false-positive on issues
  that are referenced but not actually resolved; needs design before building.
  Filed per operator request after the single observed instance, not a confirmed
  recurring pattern yet.

### CIB-024: CLI tracing logs to stderr, not stdout

- **Status:** Merged 2026-05-26 via PR #1987
- **Summary:** `anvil-observability::init_tracing` routes `BinaryKind::Cli`'s
  fmt layer to **stderr** so stdout stays reserved for command output —
  `anvil … --json` is now a single clean JSON document even under
  `ANVIL_LOG=debug` or when a default-filter `warn!` fires. The daemon (stdout)
  and the `file=` sink are untouched; the quiet-by-default filter is kept. Two
  integration tests cover the debug and default-filter-`warn!` cases. Closes the
  footgun CIB-017 surfaced.

### CIB-025: Make APS count freshness advisory-derived, not PR-maintained

- **Status:** Done 2026-06-27
- **Intent:** Implement ADR-053's accepted same-module collision fix: feature PRs
  stop editing aggregate `N/M` counts, and count freshness becomes
  advisory-derived rather than a blocking per-PR maintenance obligation.
- **Expected Outcome:** `scripts/aps/index-counts.mjs --check` still derives and
  reports module-header/index count drift, but exits 0 for freshness mismatches
  so concurrent same-module feature PRs can flip only their own `Status:` lines.
  Write mode (`pnpm aps:index`) remains the single-writer reconcile. Structural
  failures, malformed parser inputs, and unsupported rows remain visible through
  notes/errors rather than silently disappearing. Agent-facing APS guidance
  (`AGENTS.md`, `.claude/rules/aps-index.md`, `plans/project-context.md`, and
  repo-local workflow skill text if needed) is updated so it no longer instructs
  every feature PR to bump header/index counts.
- **Non-scope:** full generated APS index rows, prose custody, heterogeneous
  table-schema generation, and per-module index fragments. Those remain tracked
  separately in CIB-107.
- **Validation:** fixture proves `--check` exits 0 while naming stale counts and
  suggesting `pnpm aps:index`; write mode still reconciles; `scripts/aps/_test/index-counts.test.sh`;
  `pnpm aps:index:check`; `pnpm docs:check`; `pnpm format:check`.
- **Identified From:** CIB-019/-024 session 2026-05-26 (4 serialised rebases);
  planning council 2026-05-27; ADR-053 (accepted advisory count model).
- **Files:** `scripts/aps/index-counts.mjs`, `scripts/aps/_test/index-counts.test.sh`,
  `AGENTS.md`, `.claude/rules/aps-index.md`, `plans/project-context.md`,
  `.codex/skills/dev-workflow/SKILL.md`, `.claude/skills/dev-workflow/SKILL.md`,
  `.opencode/skills/dev-workflow/SKILL.md` if their bookkeeping wording still
  treats index counters as mandatory per feature PR.
- **Confidence:** high — the decision is already accepted in ADR-053; the change
  is a narrow gate/guidance update with fixture coverage.

### CIB-107: Generate APS index rows or fragments without prose/count conflicts

- **Status:** Proposed 2026-06-27
- **Intent:** Revisit the larger CIB-025 shapes after the advisory count model is
  in place: fully generated module-status rows or per-module index fragments
  that remove broader index-row contention without losing curated narrative.
- **Expected Outcome:** A design pass resolves prose custody, section/schema
  heterogeneity, cell escaping, failure-closed parsing, archive handling, and a
  waved migration plan before any 100+ row index rewrite. The implementation
  either generates only a safe subset or provides structured homes for narrative
  that currently lives in `plans/index.aps.md` Progress cells.
- **Validation:** generator fixtures cover `|`/newline escaping, unparseable
  module failure, section membership, heterogeneous table schemas, and a staged
  migration path that does not rewrite the whole index in one conflict-heavy PR.
- **Identified From:** CIB-025 planning council (2026-05-27) Gates 2–4 and the
  CIB-025 scope split on 2026-06-27.
- **Coordinates with:** CIB-025, CIB-022, CIB-021, CIB-023.
- **Confidence:** medium — valuable, but still a real restructure; keep Proposed
  until the design gates are settled.

### CIB-026: Isolate cwd-mutating tests across the Rust workspace

- **Status:** Merged 2026-05-29 via PR #2063
- **Summary:** Routed all `set_current_dir` tests through one workspace-wide
  serialisation guard so `cargo test --workspace` is deterministic under CI
  parallelism.
- **Intent:** Stop tests that mutate process-global cwd from creating sporadic
  failures when unrelated subprocess or MCP tests add scheduling pressure.
- **Expected Outcome:** Every test path that calls `std::env::set_current_dir`
  either uses one workspace-wide serialisation guard or is refactored to thread
  cwd explicitly, so `cargo test --workspace` is deterministic even when the
  pre-push subprocess tests run alongside doctor and MCP validation suites.
- **Validation:** `cargo test --workspace` repeated under the same parallelism as
  CI; targeted tests covering `doctor`, `mcp::tools::validate_write`, and
  `crates/anvil-cli/tests/pre_push_subprocess.rs` pass together without cwd
  leakage.
- **Identified From:** Continuous-improvement log entry 2026-05-25 for MLP2-047:
  new subprocess tests surfaced pre-existing cwd races in doctor and MCP tests.
- **Files:** `crates/anvil-cli/src/commands/doctor.rs`,
  `crates/anvil-cli/src/mcp/tools/validate_write.rs`, shared Rust test helper
  location chosen during implementation.
- **Confidence:** high — root cause and affected surfaces are named; fix shape
  is straightforward but needs careful test-helper placement.

### CIB-027: Define a lightweight review path for cross-repo implementation work

- **Status:** Merged 2026-06-03 via PR #2271
- **Intent:** Give agents a first-class pre-PR review surface when implementation
  work happens in a downstream or sibling repository where Anvil's `/council`
  command is not available.
- **Expected Outcome:** The dev workflow or agent-surface inventory documents a
  cross-repo review fallback, such as a focused `code-reviewer`/Council-agent
  pass plus the target repository's CI and automated review checks, and records
  when full Anvil Council is not applicable.
- **Validation:** Manual dry run on a non-Anvil repository task confirms the
  documented path produces review evidence before PR publication without
  pretending Anvil-specific Council commands exist there.
- **Identified From:** Continuous-improvement log entry 2026-05-25 for ATTRIB
  cross-repo `little-termi` work: `/council` was Anvil-scoped, so review relied
  on self-review, target-repo CI, and Copilot.
- **Files:** `docs/guides/agent-surface-inventory.md` and/or repo-local
  `dev-workflow` skill copies, depending on where CIB-002 lands the canonical
  inventory.
- **Coordinates with:** CIB-002 (definitive skill and agent list).
- **Confidence:** medium — process-only improvement, but needs care not to imply
  Anvil commands are portable across repositories.
- **TDD waiver (for docs-only item):** No executable product behaviour or test
  surface to drive red/green for the process documentation wording itself (see
  `test-driven-development` skill: record why when cannot reasonably test-first;
  dev-workflow/AGENTS guidance for docs/config-only changes prefers schema, lint,
  formatting, links, or manual validation over inventing irrelevant tests).
  Replacement evidence: explicit manual dry-run (non-Anvil target using general
  `code-reviewer` path), `pnpm format:check && pnpm lint:check`,
  verification-before-completion gate, and full pre-PR checks. The change adds
  no new code paths.

### CIB-028: Add a safe post-merge worktree cleanup sweep

- **Status:** Done 2026-06-27 — added a conservative dry-run-first Worktrunk cleanup assistant (`scripts/dev/wt-cleanup-sweep.sh`) with fixture coverage and documented operator use.
- **Intent:** Reduce accumulated Worktrunk worktrees left behind after batches of
  merged PRs, without deleting unmerged or still-needed local work.
- **Expected Outcome:** A documented operator command or script lists
  Worktrunk-managed worktrees whose branches are merged or deleted remotely,
  verifies clean local state, and offers/removes them with explicit safety
  checks.
- **Validation:** Run against a fixture or local repository state containing one
  merged clean worktree, one unmerged worktree, and one dirty worktree; only the
  merged clean worktree is eligible for removal.
- **Identified From:** Continuous-improvement log entries from APSCAN and ATTRIB
  batches: repeated `wt remove` follow-ups remained after PR branches merged and
  remote branches were deleted.
- **Files:** `docs/guides/worktree-policy.md` and optionally a helper under
  `scripts/` if an executable sweep is chosen.
- **Confidence:** medium — valuable ergonomics improvement, but must preserve the
  existing rule to ask before deleting unmerged, unpushed, or dirty work.

### CIB-029: Fix required Anvil version documentation to use exact semver

- **Status:** Merged 2026-05-29 via PR #2059
- **Summary:** Aligned `required_anvil_version` examples with the exact-semver
  parser contract: fixed the `anvil-l4` schema docstring + `policy.rs` fixture,
  swept range-syntax examples out of `plans/decisions/037` and the multilayer
  spec, and added `parse_rejects_semver_range_syntax` to lock range ops →
  `InvalidFloor`.
- **Intent:** Stop examples from teaching operators and test authors to write
  semver range expressions where the parser accepts only exact versions.
- **Expected Outcome:** Documentation and inline examples for
  `required_anvil_version` show exact semver values such as `0.6.0`, not range
  requirements such as `>=0.6.0`, or the parser/documentation contract is
  explicitly changed if range support is intentionally added later.
- **Validation:** Search for `required_anvil_version` examples; any examples in
  active docs or docstrings align with `RequiredAnvilVersion::parse` behaviour;
  targeted tests that cover the version-floor path still pass.
- **Identified From:** Continuous-improvement log entry 2026-05-25 for MLP2-047:
  a subprocess fixture copied the `anvil-l4` docstring's `>=0.6.0` example and
  hit `InvalidFloor` because the implementation parses exact semver only.
- **Files:** `crates/anvil-l4/src/lib.rs` and any active docs found by the
  implementation search.
- **Confidence:** high — small documentation/API-contract alignment with a named
  parser behaviour.

### CIB-030: Harden `eddacraft-tui` publish doc gate parity (PR-side `-D warnings`, all-features match docs.rs)

- **Status:** Merged 2026-06-27 via PR #2967 (PR-side `-D warnings` gate; publish-side `--all-features` gate merged 2026-06-16 via PR #2682)
- **Correction 2026-05-29:** A readiness review found the original point 3's
  premise did not hold on `main`. The `Create GitHub Release on anvil-001`
  step in `publish-eddacraft-tui.yml` is ALREADY the final state-mutating
  step — it runs after `cargo publish` AND after the mirror tag-propagation
  step (only a non-mutating `Summary` step follows it), and has
  been positioned there since the workflow's creation
  (`24884c1de`). There is no early `gh release create` to move, so point 3 is
  dropped. The stray `eddacraft-tui-v0.2.3` GitHub Release was therefore NOT
  caused by early release creation in this workflow; its true origin needs
  separate re-tracing before any remediation. This item is re-scoped to the
  two sound doc-gate sub-points.
- **Intent:** Close two latent gaps in the `eddacraft-tui` publish workflow
  and PR-side doc gate that let two `broken_intra_doc_links` errors reach the
  live publish run (`26549955604`, 2026-05-28) and fail it at the `cargo doc`
  step, after `#2018` had been merged green. The fix PR `#2029` repaired the
  immediate breakage; this item closes the recurrence surface.
- **Expected Outcome:**
  1. **PR-side `cargo doc` gate enforces `-D warnings`.** The `rust.yml`
     workflow's `cargo doc` step runs with `RUSTDOCFLAGS=-D warnings`
     (matching the publish-side gate) so rustdoc regressions block at PR
     review, not at publish. Today it does not — which is how the two
     broken links slipped through `#2018`.
  2. **Publish doc gate truly proxies docs.rs.**
     `publish-eddacraft-tui.yml`'s `cargo doc --no-deps -p eddacraft-tui`
     step uses `--all-features` (matching
     `crates/eddacraft-tui/Cargo.toml` `[package.metadata.docs.rs]
     all-features = true`) so the gate validates the same surface docs.rs
     builds. Today it uses default features, so feature-gated rustdoc
     links (e.g. the `test_utils` link de-linked by `#2029`) and
     feature-gated lints (e.g. an existing `redundant_explicit_links` in
     `widgets/image_pane.rs` under the `image` feature) are invisible to
     it. Switching to `--all-features` requires fixing the pre-existing
     all-features rustdoc lints in `image`-gated code first.
- **Validation:**
  - `.github/workflows/rust.yml`'s `cargo doc` step has
    `env: RUSTDOCFLAGS: -D warnings` (or equivalent).
  - `.github/workflows/publish-eddacraft-tui.yml`'s doc step passes
    `--all-features`; `RUSTDOCFLAGS='-D warnings' cargo doc --no-deps
    -p eddacraft-tui --all-features` is green on `main`.
  - A deliberately broken intra-doc link in a doc-only PR fails the
    PR-side `cargo doc` gate (manual smoke).
- **Identified From:** TUIR-008 first publish attempt (run `26549955604`,
  2026-05-28) failed at the doc gate with two `broken_intra_doc_links` after
  `#2018` merged green. Surfaced while opening `#2029`. A stray
  `eddacraft-tui-v0.2.3` GitHub Release (`github-actions[bot]`,
  2026-05-28T01:57:43Z) was observed at the time and originally attributed to
  early release creation in this workflow; the 2026-05-29 correction above
  showed that attribution was wrong (the release step is already the final
  step), so the stray Release's origin is untraced and tracked separately.
- **Files:** `.github/workflows/rust.yml`,
  `.github/workflows/publish-eddacraft-tui.yml`,
  `crates/eddacraft-tui/src/widgets/image_pane.rs` (and any sibling
  feature-gated rustdoc lints uncovered by `--all-features -D warnings`).
- **Confidence:** high — each gap is small and well-isolated; the
  all-features lint cascade is the only unknown-scope subpoint.

### CIB-031: Scope the dependency-audit gate so Rust-only lockfile changes skip the npm Trivy audit

- **Status:** Merged 2026-05-30 via PR
  [#2128](https://github.com/eddacraft/anvil-001/pull/2128) — the `lockfile`
  classifier case is now scoped to npm manifests/lockfiles only; a Rust-only
  `Cargo.lock` change no longer adds `dependency-audit` (and therefore skips
  the whole-repo Trivy scan + `license-check`). Three new classifier test
  cases pin the truth table. Awaiting release-tag evidence to advance to
  Released/Shipped.
- **Intent:** A PR that changes only `Cargo.lock` (or a crate `Cargo.toml`) is
  classified as a generic `lockfile` change, which the classifier maps to the
  `dependency-audit` signal. In `.github/workflows/security.yml` that signal
  gates **two** npm-oriented jobs — the Trivy `Dependency Audit` *and*
  `license-check` (both keyed on `dependency-audit-required`). Trivy runs with
  `scan-ref: '.'`, scanning the whole repo filesystem, so it reports the repo's
  standing `pnpm-lock.yaml` HIGH/CRITICAL advisories regardless of what changed.
  Net effect: a Rust-only dependency change runs two npm-facing gates and
  re-surfaces unrelated npm advisories, while `cargo-deny` already covers the
  Rust side. Route Rust-only lockfile changes to `cargo-deny` and reserve the
  Trivy + `license-check` gates for npm manifest/lockfile changes.
- **Expected Outcome:** A PR whose only dependency-file change is `Cargo.lock`
  or a crate `Cargo.toml` triggers `cargo-deny` but NOT the Trivy
  `Dependency Audit` or `license-check` jobs (or those jobs no-op when no npm
  lockfile/manifest changed). A `pnpm-lock.yaml` / `package.json` change still
  triggers Trivy + `license-check` as today. The change-classifier
  distinguishes Rust lockfiles from npm lockfiles.
- **Validation:** `scripts/ci/classify-changes.test.sh` gains cases asserting
  (a) a `Cargo.lock`-only diff routes to the Rust audit and does NOT add the
  `dependency-audit` requirement (which gates Trivy + `license-check`),
  (b) a `pnpm-lock.yaml` diff still does, and (c) a mixed `Cargo.lock` +
  `pnpm-lock.yaml` diff DOES add `dependency-audit` (the Rust-only suppression
  must not silence the npm audit when npm also changed). A Rust-only-dependency
  PR shows green `cargo-deny` and no `Dependency Audit` / `license-check`
  failure attributable to unrelated npm advisories.
- **Dependencies:** None (`cargo-deny` job already exists in
  `.github/workflows/rust.yml`; the discovery PR SCAN-005 #2034 is merged).
- **Identified From:** SCAN-005 PR #2034 (2026-05-28). Adding `ignore` to
  `anvil-bench` dev-dependencies changed only `Cargo.lock` (no npm files), but
  the `lockfile` classification ran the Trivy `Dependency Audit` (whole-repo
  `scan-ref: '.'`), which failed on pre-existing `pnpm-lock.yaml` HIGH/CRITICAL
  vulns unrelated to the change. `cargo-deny` passed. The Trivy job is advisory
  (not a required check) so it did not block, but it is persistent red-X noise
  on Rust-only PRs and can mask a genuinely new npm advisory.
- **Files:** `scripts/ci/classify-changes.sh` (the `lockfile)` classification
  case and its path→class mapping), `scripts/ci/classify-changes.test.sh`
  (three new contract cases). Implementation note: the Files list also named
  `.github/workflows/security.yml` and `.github/actions/detect-changes/action.yml`,
  but the action.yml is a pure consumer of the bash classifier (calls
  `classify-changes.sh` and reads `has_check dependency-audit`), and
  security.yml's gating clause was already correct — so the actual fix needed
  only the classifier + tests.
- **Confidence:** medium — must keep the Rust audit (`cargo-deny`) and the
  npm-facing gates (Trivy + `license-check`) correctly routed; touches the
  change-classifier contract, which has its own test suite to extend.

### CIB-032: Fresh worktrees fall back to a stale global oxfmt, producing false format failures

- **Status:** In Progress
- **Intent:** A freshly created `git worktree` has no `node_modules`, so
  `pnpm run format:check` / `pnpm run lint` resolve `oxfmt` from a stale
  **global** install instead of the workspace-pinned `oxfmt@^0.51.0`. The older
  global binary reports format "failures" on files the current change never
  touched (observed repeatedly on `crates/eddacraft-tui/CHANGELOG.md`), sending
  the developer chasing a non-issue until they realise `pnpm install` is needed.
- **Expected Outcome:** A fresh worktree's first `pnpm run format:check` either
  runs the workspace-pinned `oxfmt` or fails **loudly with an actionable
  message** ("oxfmt X found, workspace pins Y — run `pnpm install`"), instead of
  silently linting with a stale global binary. Candidate mechanisms: a tiny
  version-guard prepended to the `format`/`lint` scripts; the worktree-creation
  flow (`wt` / a setup hook) running `pnpm install`; or documenting the
  install-first requirement in the worktree policy and failing closed otherwise.
- **Validation:** In a worktree with no `node_modules`, `pnpm run format:check`
  does not emit false positives on unmodified files — it either uses the pinned
  oxfmt or exits non-zero with the version-mismatch guidance. A worktree that
  has run `pnpm install` checks clean.
- **Identified From:** Recurring friction across this session's worktrees
  (SCAN-004/-005/-006, CIB-031, #1735) — the stale-global-oxfmt false positive
  on `CHANGELOG.md` appeared on every fresh worktree until `pnpm install` ran.
- **Files:** `package.json` (`format`/`format:check`/`lint` scripts), worktree
  policy docs (`docs/guides/worktree-policy.md`), and the worktree-creation
  tooling if a setup hook is the chosen path.
- **Confidence:** medium — clear problem and validation; the cleanest fix
  mechanism (script guard vs setup hook vs docs) needs a small design call.

### CIB-033: Sweep for open GitHub issues already resolved on main

- **Status:** Draft
- **Intent:** Open issues that have already been fixed on `main` (often by a
  later, separately-tracked change) linger in the tracker, so anyone picking
  work from the issue list wastes a verification round per stale item before
  finding a still-real one.
- **Expected Outcome:** A lightweight, repeatable way to surface
  likely-already-resolved open issues for human confirmation + closure —
  e.g. a documented triage procedure, or a best-effort script that, for each
  open issue, greps the file/symbol/behaviour it cites against current `main`
  and flags candidates whose described gap no longer exists. Output is a
  candidate list a human closes; it does not auto-close.
- **Validation:** Running the sweep over the current open-issue list surfaces a
  candidate set; spot-checking confirms the flagged items are genuinely
  resolved (no false "still open" misses on a small known sample). Closure
  decisions stay human.
- **Identified From:** 2026-05-28 session picking a GitHub issue to work — 2 of the
  first 3 candidates (#1873 resolved by CIB-020; #1976 resolved by the
  drift-check `--release-record` gating) were already fixed on `main` but still
  open. Both were closed manually; a sweep would have surfaced them up front.
- **Files:** A new triage helper under `scripts/` and/or a procedure note in
  the dev-workflow / issue-triage docs — mechanism chosen at implementation.
- **Confidence:** low — the "is this issue resolved?" heuristic is inherently
  fuzzy (cited symbols move, behaviour is hard to grep); the win is a
  human-reviewed candidate list, not automated closure.

### CIB-034: Publish sanitised release evidence for the public release mirror

- **Status:** Merged 2026-06-16 via PR #2684
- **Intent:** Give users of the public release mirror a concise, sanitised trust
  record for each shipped Anvil release, even though the source project itself is
  not public.
- **Expected Outcome:** Each public mirror release includes a generated
  `release-evidence.md` or equivalent manifest that names the release version,
  tag or release ref, artefact identities, checksums or digests, publish
  timestamps, blocking/advisory validation summary, and known accepted risks. The
  evidence proves that the published artefacts map to the exact release ref that
  passed blocking gates, and omits raw logs, secrets, internal hostnames,
  customer data, provider payloads, and sensitive private-repo details.
- **Validation:** Dry-run a release evidence generation against an existing beta
  release and manually verify that the public mirror record is useful without
  exposing private operational data; release verification confirms artefact
  digest/checksum, package version, and release ref alignment.
- **Identified From:** 2026-05-28 operator review of OpenClaw's public
  `release-evidence.md` pattern and discussion of whether a private-source
  project with a public release mirror benefits from public release evidence.
- **Files:** `scripts/release/`, `plans/releases/`, public release mirror
  publication workflow or repository, and any release runbook section that owns
  public artefact publishing.
- **Coordinates with:** Release orchestration / closeout flow; distribution
  trust work under DISTRIB; future SBOM or provenance work if added.
- **Confidence:** medium — the trust value is clear for a public mirror, but the
  implementation must draw the sanitisation boundary carefully and prove exact
  artefact-to-ref alignment rather than merely reporting nearby CI.

### CIB-035: drift-check crashes on invalid release records instead of staying advisory

- **Status:** Merged 2026-06-02 via PR #2241
- **Intent:** Keep `scripts/aps/drift-check.mjs` true to the warnings-over-blocks
  / exit-0 architecture principle when handed a malformed input, so a bad release
  record degrades to an advisory finding rather than an uncaught crash.
- **Expected Outcome:** When the release record passed to drift-check is missing,
  unreadable, or not valid JSON, the check emits a JSON advisory finding such as
  `release-record-unreadable` or `release-record-invalid-json` and preserves
  exit 0, consistent with the rest of `drift-check`. A genuine read/parse failure
  never aborts the advisory run.
- **Validation:** A drift-check fixture that passes an invalid release-record
  path/content asserts exit status 0 plus the JSON advisory finding; a valid
  record is unaffected.
- **Identified From:** 2026-05-29 clawpatch periodic scan
  (`plans/audits/2026-05-29-clawpatch-periodic-scan.json`, finding
  `fnd_sig-feat-library-ae662c437a-fe21_879e585035`); triage at
  `plans/reviews/2026-05-29-clawpatch-triage.md`. Distinct from CIB-023, which
  covers the implemented-but-draft drift class rather than input robustness.
- **Files:** `scripts/aps/drift-check.mjs` release-record loading/error handling;
  a drift-check fixture test.
- **Coordinates with:** CIB-023 (same script, different drift class); the
  architecture warnings-over-blocks principle in `.claude/rules/architecture.md`.
- **Confidence:** high — the defect and the minimal fix scope are both concrete
  and the advisory exit-0 contract is unambiguous.

### CIB-036: Active APS modules fail canonical `aps lint` (missing `## Work Items` section)

- **Status:** Done
- **Summary:** Resolved the repo-wide divergence between Anvil's module corpus
  and the canonical `aps lint` structural contract. Per maintainer decision the
  corpus was bulk-migrated to canonical `## Work Items` (not forking the
  validator): PR #2095 mechanically migrated 84 modules (`## Tasks`→`## Work
  Items`, casing, phase-nesting, `Scope`→`ID`); the cleanup PR added the
  remaining metadata tables, per-item field fills, and fixed `issues.md`
  section headings. Where `aps` was genuinely unfit it was changed instead of
  contorting modules: anvil-plan-spec PR #56 exempts terminal
  (Done/Complete/Merged/Released/Shipped) work items from E005, matching Anvil's
  done-item compaction. `clawpatch-pre-tag-v0.7.0-beta` (a release-findings
  tracker, not a feature module) is excluded from `aps:active-lint` with
  rationale, pending archival once its findings close out. `pnpm aps:active-lint`
  is now green across the active corpus and wired into CI.


### CIB-037: `aps lint` validates only one file when given multiple arguments

- **Status:** Done
- **Summary:** Canonical `aps lint <f1> <f2> …` only honoured its *last* path
  argument (reported `1 file checked`), so `scripts/aps/active-lint.mjs` — which
  passed the whole active set in one `spawnSync(apsBin, ['lint', ...files])` —
  silently validated a single module (`weave.aps.md`, last in sort order). Filed
  in PR #2084; fixed in PR #2089 by invoking `aps lint` per file and aggregating
  exit status (uniform non-zero propagates, mixed → 1, spawn error → 2);
  `pnpm aps:active-lint` now reports `102 files checked` across the full
  `--list-files` scope. `scripts/aps/_test/active-lint.test.sh` updated to assert
  one invocation per file. Un-masked CIB-036's true scope (89/102 files with
  findings). Pending merge — PR #2089 is blocked by CIB-038.

### CIB-038: Skip-filler duplicate check names block ruleset merge on docs-path PRs

- **Status:** Merged 2026-06-03 via PR #2295
- **Intent:** Let docs/plans-path PRs merge — the `main` ruleset must resolve
  required status checks that currently report a duplicate `success`+`skipped`
  pair under one name.
- **Expected Outcome:** A docs-only PR with a green check rollup reaches
  `mergeStateStatus: CLEAN` and `gh pr merge --auto` fires without `--admin`.
  Each required context (`Docs Lint`, `Lint & Format`, `Type Check`,
  `Unit Tests (Node 22.x, ubuntu-latest)`) reports a single conclusion per
  commit.
- **Defect:** On docs-path PRs the `CI` workflow emits both the real job and its
  skip-filler under the same required check name, producing one `success` and
  one `skipped` check-run per name on the head commit. GitHub's rollup reads
  `SUCCESS`, but the `main` ruleset cannot disambiguate the duplicate and
  reports `BLOCKED` ("base branch policy prohibits the merge"), so `--auto`
  never fires. Observed 2026-05-29 on #2084 and #2083; non-docs PRs
  (#2081/#2082) are unaffected.
- **Fix options:** make the skip-filler and the real job mutually exclusive per
  check name (only one reports a given required context per run), or stop naming
  the filler with the required context name. Relates to the CICD-005 `*-skip`
  filler design.
- **Validation:** a docs-only PR shows exactly one check-run per required
  context name on its head SHA, reaches `CLEAN`, and `--auto` merges without
  `--admin`.
- **Identified From:** Surfaced 2026-05-29 — CIB-036's own PR #2084 (and sibling
  #2083) sat `BLOCKED` with a green rollup; GraphQL showed `success`+`skipped`
  duplicates for four required contexts from the `CI` workflow.
- **Coordinates with:** the `main` ruleset (required status checks);
  `.github/workflows/ci.yml` skip-filler jobs; CICD-005 (the `*-skip` filler
  design).
- **Confidence:** high — the block is reproduced and the duplicate-name cause is
  pinned via GraphQL; the exact filler-dedup approach needs a small design call.
- **Implementation:** Chose "consolidate into primaries" (mutually exclusive
  by having only the primary job report under each required name). Primary
  jobs' `if:` now ensure execution on PRs (to own the name); added guarded
  "Skip (filler) when ..." early step + `if:` on every heavy step (checkout,
  setup, run, etc) so filler path is cheap success with no work. Removed the
  four twin `*-skip:` jobs + stale comments. Updated contract test
  (`scripts/ci/integration-validation.test.sh`) and
  `.github/workflows/README.md`. TDD: yml edit made contract test fail (red);
  test update made it pass (green). format/lint/typecheck + dedicated
  `test:ci-*` green (full `pnpm test` has known pre-existing cargo-flag
  inheritance friction; used targeted ci fixtures instead). APS status kept
  current in same PR. (PR will supply the merge evidence.)

### CIB-039: Archive clawpatch-pre-tag-v0.7.0-beta once its findings are closed out

- **Status:** Merged 2026-06-03 via this PR
- **Resolution (2026-06-03):** all CLAWP-NNN findings were dispositioned to
  terminal (53 Merged, 11 Ship, 1 Deferred-tracked) by the #1740 test-hardening
  batch (PRs #2261 / #2265 / #2267, reconcile #2268) and the disposition pass
  (#2274). The tracker was then archived: `git mv`-d to
  `plans/archive/modules/`, the `clawpatch` entry removed from
  `NON_CANONICAL_MODULES` in `scripts/aps/active-lint.mjs` (the archived file
  falls out of the active walk scope, so the carve-out is no longer needed), the
  index row repointed to the archive path, and inbound links updated.
  `pnpm aps:active-lint` stays green without the exclusion. Release-tag inclusion
  is moot — this is a planning-history doc, not shipped code.
- **Intent:** Retire the v0.7.0-beta pre-tag release-findings tracker once its
  findings are closed, and drop the canonical-lint carve-out added for it.
- **Expected Outcome:** `plans/modules/clawpatch-pre-tag-v0.7.0-beta.aps.md` is
  `git mv`-d to `plans/archive/modules/`, its entry is removed from
  `NON_CANONICAL_MODULES` in `scripts/aps/active-lint.mjs` (the exclusion exists
  only because it was an active non-module doc), and the index row is repointed
  to the archive path. `pnpm aps:active-lint` stays green without the exclusion.
- **Validation:** after archival, `pnpm aps:active-lint` is green with no
  `clawpatch` entry in `NON_CANONICAL_MODULES`; the archive cascade passes
  (`docs:check`/check-links, `aps:index:check`); the module is confirmed
  included in (or moot for) the relevant release tag before marking Complete.
- **Identified From:** CIB-036 — the canonical-lint migration carved clawpatch
  out as a release-findings tracker (CLAWP-NNN = bug findings, not work items)
  rather than forcing it into the work-item shape; archival is the proper
  end-state once the findings close.
- **Coordinates with:** CIB-036 (the exclusion this removes);
  `scripts/aps/active-lint.mjs` `NON_CANONICAL_MODULES`; the APS module-archive
  cascade in `plans/aps-rules.md`.
- **Confidence:** medium — the archival mechanics are well-understood
  (`git mv` + relink + index repoint + docs index regen), but the trigger
  (all findings closed) is not yet met and is owned outside this item.
### CIB-040: Full CLIC-010 help-text layout pass for all commands

- **Status:** Done 2026-06-27
- **Intent:** Apply the uniform CLIC-010 layout to every command's `--help`
  output: one-line imperative summary, a when-to-use hint, common flag
  descriptions, and a pointer to the relevant runbook or docs page.
- **Expected Outcome:** `cargo run -p anvil-cli -- <cmd> --help` for every
  non-hidden command follows the same four-section structure that answers "what
  does this do, when should I run it, and where do I learn more?" — with no
  internal identifiers, ADR references, or work-item IDs in user-visible text. A
  new CLI lint (proposed in the coherence spec as CLIC-010) asserts the layout
  in CI.
- **Validation:** `cargo run -p anvil-cli -- --help` and spot-checks of at least
  10 commands show the four-section layout. The CLIC-010 CI lint passes without
  exclusions.
- **Identified From:** Specified in
  `plans/specs/2026-05-07-cli-surface-coherence.md` §9 CLIC-010 as the full
  layout rewrite, explicitly deferred until after the light consistency pass.
  The light pass and `docs/runbooks/cli-surface.md` landed first in the
  `chore/cli-help-standardization` branch (2026-05-29).
- **Coordinates with:** `plans/specs/2026-05-07-cli-surface-coherence.md` §9;
  `docs/runbooks/cli-surface.md` (the companion runbook that documents current
  command intent and can serve as the source of truth for when-to-use hints).
- **Confidence:** high — the spec is clear on scope; the blocker was the light
  pass landing first.

### CIB-041: Stop `release.yml` from triggering on `eddacraft-tui-v*` library tags

- **Status:** Merged 2026-06-02 via PR #2221
- **Summary:** `release.yml`'s broad `push.tags` glob
  (`'**[0-9]+.[0-9]+.[0-9]+*'`) also matched `eddacraft-tui-v*`, so library tags
  ran the CLI cargo-dist pipeline and published a binary-less release to the
  public `eddacraft/anvil` repo that stole the GitHub "Latest" pointer and
  404'd `install.eddacraft.ai`'s `/releases/latest/download/` installer.
  Guarded the `plan` job with an allowlist —
  `github.event_name == 'pull_request' || startsWith(github.ref_name, 'v')` —
  so only PR dry-runs and CLI `v*` tags run; the skip cascades to `host`
  (gated on `needs.plan.result == 'success'`). Allowlist over per-crate denylist
  (Council, 2026-06-02) so future prefixed crate tags (`napi-v*`,
  `eddacraft-anvil-graph-cache-v*` per ADR-064) need no further guard. Immediate
  incident mitigation was a manual `gh release edit v0.7.4-beta --repo
  eddacraft/anvil --latest`. **Post-merge validation (pending next library
  release):** push an `eddacraft-tui-v*` tag → `plan` skips, no release on
  `eddacraft/anvil`; a CLI `v*` tag still runs the full pipeline;
  `curl -sIL .../releases/latest/download/eddacraft-anvil-installer.sh` → 200.
  Deferred follow-ups (Council): installer-URL synthetic health check (CIB-042);
  anvil-001 `--latest=false` on the tui release (CIB-043);
  `release-sign-artefacts.yml` no-op on tui releases (CIB-044).

### CIB-042: Synthetic health check on the public installer URL

- **Status:** Merged 2026-06-16 via PR #2679
- **Intent:** Catch a broken `install.eddacraft.ai` installer proactively
  instead of waiting for a user to report a 404.
- **Expected Outcome:** A post-publish probe in the `announce` job of
  `release.yml` (and/or a scheduled workflow) fetches
  `https://github.com/eddacraft/anvil/releases/latest/download/eddacraft-anvil-installer.sh`
  and the `.ps1`, asserts HTTP `200`, and fails loudly (`::error::`) otherwise.
  A green release run guarantees the public installer resolves; a regression
  surfaces as a red CI signal, not a support ticket.
- **Validation:** the probe step returns non-zero when pointed at a known-404
  URL (negative test) and `200` against the real latest CLI release; a release
  dry-run shows the probe executing.
- **Identified From:** 2026-06-02 install.eddacraft.ai investigation + CIB-041
  Council (operations reviewer) — the installer 404 was discovered reactively;
  there is no synthetic monitoring on the installer asset URL.
- **Coordinates with:** `.github/workflows/release.yml` `announce` job;
  CIB-041 (the fix for this specific incident); `install.sh` (the local
  installer that fetches the same `/releases/latest/download/` asset).
- **Confidence:** high — a `curl -sIL … | grep 200` probe is small and the
  failure semantics are clear; the only design choice is in-release probe vs.
  external uptime monitor (the in-release probe is the cheaper first step).

### CIB-043: Set `--latest=false` on the eddacraft-tui anvil-001 release

- **Status:** Merged 2026-06-16 via PR #2679
- **Intent:** Stop library (`eddacraft-tui-v*`) releases from contending for the
  GitHub "Latest" pointer on the private `anvil-001` repo, so the CLI release is
  always uncontested as Latest there.
- **Expected Outcome:** The `gh release create` step in
  `publish-eddacraft-tui.yml` passes `--latest=false`, so a tui release never
  auto-promotes to Latest on `anvil-001`. CIB-041 already removed the
  public-repo exposure; this closes the residual anvil-001 internal-pointer race
  against `scripts/release/closeout.sh`'s `--latest` promotion.
- **Validation:** after merge, publish (or dry-run) an `eddacraft-tui-v*` tag and
  confirm the anvil-001 release is created with Latest unset; the most recent
  CLI `v*` release remains Latest on `anvil-001`.
- **Identified From:** CIB-041 Council (operations reviewer) — `anvil-001`
  Latest hygiene; lower severity than the public-repo bug since the installer is
  served from `eddacraft/anvil`, not `anvil-001`.
- **Coordinates with:** `.github/workflows/publish-eddacraft-tui.yml`
  (`gh release create`); `scripts/release/closeout.sh:274` (the public
  `--latest` promotion).
- **Confidence:** high — single flag on an existing command; no behavioural risk
  to the CLI release path.

### CIB-044: Skip `release-sign-artefacts.yml` for non-CLI (library) releases

- **Status:** Done 2026-06-27 — `release-sign-artefacts.yml` now gates release-published runs to non-prerelease CLI `v*` tags while keeping manual dispatch; workflow contract tests pin the library-tag skip.
- **Intent:** Stop the artefact-signing workflow from spending a runner on a
  ~10-minute no-op for every `eddacraft-tui-v*` release on `anvil-001`.
- **Expected Outcome:** `release-sign-artefacts.yml`'s job `if:` gates on the CLI
  tag convention (e.g. `&& startsWith(github.event.release.tag_name, 'v')`), so
  it does not run for library releases (which carry no `*-installer.*` /
  `anvil-*-provenance.json` assets and currently sign nothing under
  `shopt -s nullglob`).
- **Validation:** publish (or dry-run) an `eddacraft-tui-v*` release and confirm
  the sign job is skipped; a CLI `v*` release still runs signing and uploads the
  signed assets.
- **Identified From:** CIB-041 Council (operations reviewer) — pre-existing
  inefficiency surfaced by the install.eddacraft.ai investigation; the tui
  release is non-prerelease, so the sign job's `!prerelease` condition currently
  evaluates true and it runs to no effect.
- **Coordinates with:** `.github/workflows/release-sign-artefacts.yml` (the
  signing job `if:`); CIB-041 (allowlist precedent — same `v*` CLI convention).
- **Confidence:** high — additive `if:` clause; worst case the job is skipped
  when it would have been a no-op anyway.

### CIB-045: Add Codex dev-workflow config and routing surface

- **Status:** Done
- **Summary:** Added repo-local Codex configuration and a Codex `dev-workflow`
  skill so Codex can run the Anvil APS -> Worktrunk -> code -> Council -> PR ->
  cleanup loop with workspace-write permissions, network access, sibling
  worktree write roots, and auto-reviewed approvals while staying out of
  `danger-full-access`. Updated the agent-surface inventory to make the Codex
  lifecycle surface discoverable.
- **Hardening:** Taught the CI change classifier
  (`scripts/ci/classify-changes.sh`) to recognise agent-tooling config
  directories (`.codex` / `.claude` / `.opencode`) as a dedicated
  `agent-config` class. Previously a `.codex` file fell through to the
  conservative `unknown` fallback and forced the full unit-test / typecheck
  matrix on a pure agent-config bookkeeping change. These dirs carry no
  compiled source and are oxfmt-excluded (`.codex` added to `.prettierignore`
  alongside `.claude` / `.opencode`, whose skill files embed copy/paste fenced
  markdown templates the formatter corrupts), so they now require only an
  operations review; markdown within still routes to `docs` for markdownlint.
- **Root-cause fix:** Also hardened the `Unit Tests` pre-test build step in
  `.github/workflows/ci.yml`. `nx affected -t test` runs against the PR
  merge-ref, so it pulls in `main`'s FLAGCAT migration where `apps/anvil-api`
  imports `@eddacraft/anvil-flags-catalogue` by its published `dist/` entry.
  The test target declares `^build`, but on an nx cache hit the build marker
  is restored without the `dist/` outputs present, so the import resolved to
  nothing and the suites collected 0 tests. Added `flags-catalogue` (its nx
  project name) to the explicit "Build test dependencies" force-build list
  alongside `@eddacraft/transactional`, guaranteeing the entry exists on every
  run. Verified locally: `apps/anvil-api` goes from 0-test import failures to
  411 passing tests with the catalogue dist built.

### CIB-046: Gate the `anvil plan dashboard` APS surface behind an internal-developer feature flag

- **Status:** Merged 2026-06-08 via PR #2440
- **Intent:** The APS dashboard (`anvil plan dashboard`) renders Anvil's own
  plan internals but ships always-on, unauthenticated, and documented as
  **Class: User-explicit**. That posture is not a deliberate access decision —
  the command was deliberately added to the auth-bypass set
  (`bypass_auth_plan_dashboard`) so it could be dogfooded locally while
  `anvil auth` was unavailable; the execution plan
  (`plans/execution/2026-05-24-aps-tui-dashboard.md` §104/§110) recorded that
  workaround as an "unauthenticated classification matching local planning
  commands". Feature-flagging was never considered in the spec, module, or
  ADR-055. Bring the surface under the FLAGCAT catalogue as an
  internal-developer-only feature.
- **Expected Outcome:**
  - New flag `tui-dashboard.aps-dashboard` defined once in `flags/manifest.json`
    under a new `tui-dashboard` group in `flags/groups.json` (the existing
    `dashboard` group is the **web** dashboard surface and must not be reused),
    scoped to a new `staff-internal-developer` audience (staff axis) added to
    `flags/audiences.json`. Flag flows through the TS surfaces, the Rust
    `build.rs` codegen, and the manifest<->TS<->Rust drift gates (nx
    `test:flags-catalogue` + `cargo test` kernel-types) with no drift.
  - `anvil plan dashboard` is hidden/refused unless the flag resolves enabled
    for the caller, gated at dispatch. (Correction, captured during
    implementation: there is no explicit auth-bypass *set* to remove it from —
    it was unauthenticated only because `plan` is absent from the licence-gate
    set `CLI_GATED_COMMANDS`; see the Implementation note below.)
  - A local escape hatch that is **not** "open to everyone" survives `anvil auth`
    being down — gate acceptance on the `tui-dashboard.aps-dashboard` flag or on
    `ANVIL_ADMIN_KEY` (the `admin` command already authenticates this way
    without personal credentials, per `bypass_auth_admin`), so the original
    dogfooding need is met without an unauthenticated public command.
  - `docs/runbooks/cli-surface.md` reclassifies `anvil plan` from
    **User-explicit** to an internal/staff class.
- **Validation:** flag-catalogue drift tests green (TS + Rust); with the flag
  disabled (and no `ANVIL_ADMIN_KEY`), `anvil plan dashboard` is hidden/refused;
  with the flag enabled for an internal-developer caller **or** with
  `ANVIL_ADMIN_KEY` set, the dashboard opens with `anvil auth` down; a
  binary-level test asserts `plan dashboard` is refused (exit 3) when the gate
  is closed (the licence-gate `requires_auth` test is unchanged — `plan` stays
  out of that set by design); `cli-surface.md` no longer lists it as
  User-explicit.
- **Decision (resolved 2026-06-03):** add a new `staff-internal-developer`
  audience (staff axis) to `flags/audiences.json` rather than reuse
  `staff-anvil-internal` — "internal developer" is intentionally narrower than
  "all Anvil staff".
- **Identified From:** 2026-06-03 review of public/operator docs for APS
  dashboard references — confirmed no public-docs leak, but the command is
  unflagged, unauthenticated, and operator-classed by way of the local
  auth-down workaround. Good flag-by-default exemplar (a code-defined feature,
  flip via PR).
- **Coordinates with:** `flags/manifest.json`, `flags/groups.json`,
  `flags/audiences.json` (FLAGCAT catalogue); `crates/anvil-cli/src/commands/plan.rs`
  and `crates/anvil-cli/src/main.rs` (subcommand wiring + `bypass_auth_plan_dashboard`
  / `requires_auth`); `crates/anvil-cli/src/feature_flags.rs` (command-gating
  metadata path used by `cli.licence-gate`); `docs/runbooks/cli-surface.md`.
- **Confidence:** medium — mechanically additive (new flag + group + audience +
  a gating check); audience resolved (`staff-internal-developer`), the remaining
  implementation choice is the existing licence-gate command-metadata path vs. a
  dedicated check.
- **Implementation note (2026-06-08, In Progress):** chose the **dedicated check
  at dispatch** over the licence-gate command-metadata path — the licence gate
  (`CLI_GATED_COMMANDS` / `requires_auth`) keys on customer plan tier, a
  different axis from a staff-internal surface, and would gate all of `plan`
  rather than the `dashboard` subcommand. Correction to the item framing: there
  is **no explicit auth-bypass set** to remove `plan dashboard` from — it was
  unauthenticated simply because `"plan"` is absent from `CLI_GATED_COMMANDS`;
  gating is therefore an *added* check, not a deletion. New flag
  `tui-dashboard.aps-dashboard` is **default-disabled**; runtime open paths are
  `ANVIL_DEV=1` (local override, extended in `local_overrides_from_env`) and a
  non-empty `ANVIL_ADMIN_KEY`; a closed gate returns `output::AuthRequired` →
  `EXIT_AUTH_REQUIRED` (3), mirroring `admin`. The `staff-internal-developer`
  audience is declared in the inventory and on the `tui-dashboard` group, but
  the MVP does **not** plumb a staff-axis signal into the CLI evaluation context
  (`/auth/verify` carries no staff claim today), so the flag cannot yet resolve
  `enabled` for a real authenticated caller via targeting — **deferred
  follow-up**: plumb a staff/role claim so the flag targets
  `staff-internal-developer` directly. `cli-surface.md` reclassifies `anvil
  plan` from `User-explicit` to a new `Internal` class. Drift gates green
  (`pnpm nx test flags-catalogue`, `cargo test -p eddacraft-anvil-kernel-types`).

### CIB-047: Surface the save-time daemon-absent fallback in the watch TUI

- **Status:** Done 2026-06-27 — watch TUI now shows a daemon-unavailable scoped-fallback footer notice on the first fallback of a disconnect and clears it when daemon-backed validation reconnects.
- **Intent:** DSV-007 made `anvil watch` a thin save-time-daemon client with a
  scoped fallback (safe default-on via DSV-021, with `ANVIL_WATCH_DAEMON=0`
  opt-out and `=1` forced diagnostics). The "warn once per
  disconnect" contract is honoured on the plain/non-TUI surface
  (`tracing::warn!` plus an advisory stderr line), but in TUI mode the warn is
  suppressed — the alt-screen owns stdout/stderr — so a TUI user gets a
  correct-but-silent demotion to a scoped subprocess check when the daemon dies
  mid-session, with no in-TUI indicator. Surface it so the warn-once contract
  holds on the TUI surface too.
- **Expected Outcome:** when watch is in TUI mode and a save-time cycle falls
  back (daemon absent / mid-session death), the watch action footer (or status
  strip) shows a once-per-disconnect indicator (e.g. `daemon: unavailable —
  scoped fallback`) that resets on reconnect. No change to the non-TUI surface
  or to the DSV-021 default-on/opt-out routing gate.
- **Validation:** a render/unit test that a TUI fallback cycle produces the
  footer indicator once per disconnect and clears on reconnect.
- **Identified From:** DSV-007 (PR #2284) batch council — kernel-maintainer
  MINOR: the warn-once-per-disconnect contract currently holds only for the
  non-TUI path (`crates/anvil-cli/src/commands/watch.rs` gates the advisory on
  `!self.tui_parent`).
- **Coordinates with:** `crates/anvil-cli/src/commands/watch.rs` (the `FellBack`
  dispatch arm + the TUI `ActionResultLine` path);
  `crates/anvil-cli/src/commands/watch_save_time.rs` (the `SaveTimeDecision`
  `warned` flag is already computed); the `anvil-tui` watch surface.
- **Confidence:** medium — small additive UI signal; the `warned` flag already
  exists, the work is plumbing it to the TUI footer.

### CIB-048: Worktree cargo `target/` dirs oversubscribe the shared Projects disk

- **Status:** Draft
- **Intent:** Each agent worktree carries its own full Rust `target/` (~100G), so
  a dozen-plus live worktrees on `/home/aneki/Projects` (a separate disk) sum to
  ~1.7T and intermittently fill the disk to 100%, ENOSPC-blocking every agent
  mid-task — even a single source-file write fails. Make worktrees share one
  build-output location (or redirect it onto the roomy disk) so worktree count no
  longer multiplies build-cache footprint.
- **Expected Outcome:** A documented, default-on mechanism so a new worktree does
  not allocate its own ~100G `target/`. Candidate shapes (pick during planning):
  a shared `CARGO_TARGET_DIR` exported by the post-`wt-new` / post-start `rust`
  hook pointing at a single cache on the larger disk; or a per-machine
  `.cargo/config.toml` `build.target-dir` override. Sibling worktrees must never
  be touched to reclaim space; only a worktree's own regenerable artefacts are
  fair game. Behaviour is opt-outable for anyone who wants isolated targets.
- **Validation:** create two worktrees from `main`, build each, and confirm
  build output lands in one shared location (single `target/` grows; the second
  worktree adds no second ~100G tree); `df` on the Projects disk stays well under
  capacity across N worktrees. Document the guarantee in
  `docs/guides/worktree-policy.md`.
- **Identified From:** continuous-improvement-log 2026-05-29 (CIB-026 and #2068 —
  the latter explicitly flagged "shared-target worktree config is a real
  recurring infra fix — candidate CIB item if it recurs"; it had already recurred
  one entry earlier). Recurring across sessions; worked around each time by
  reclaiming only the current worktree's own `target/debug/incremental` and
  building to an external target dir on the roomy disk.
- **Coordinates with:** `scripts/dev/wt-new.sh`, the post-start `rust` worktree
  hook, `docs/guides/worktree-policy.md`; any per-worktree `CARGO_TARGET_DIR`
  conventions already in `wt.toml`.
- **Confidence:** medium — the problem and its impact are well-evidenced and
  recurring; the exact mechanism (shared env var vs cargo config vs hook) is a
  planning decision, and a shared target dir can change incremental-rebuild
  behaviour across worktrees, so it needs a deliberate choice not a blind flip.

### CIB-049: `anvil start --verify` is auth-gated and `--json` auth envelopes go to stderr

- **Status:** Merged 2026-06-10 via PR #2474
- **Intent:** Two related CLI auth-gate defects in
  `crates/anvil-cli/src/main.rs`, both verified against `main` on 2026-06-09
  (clawpatch run `20260609T111844-b83fca`, feature `feat_cli-command_ba5ccdd3a6`):
  1. `StartArgs` carries a documented read-only `verify: bool` — the
     `anvil start --verify` sibling of `anvil status --verify` — but
     `skips_auth_for_local_probe` matches `Commands::Status(args) if args.verify`
     only. So `start --verify`, despite being a read-only probe, hits the auth
     wall, breaking air-gapped and scripted consumers.
  2. `auth_required_response` coerces action commands
     (start/welcome/init/gate/audit/watch) to `EXIT_OK` with a success-shaped
     `{"state":"authRequired", …}` envelope (issue #1822), but the call site
     `eprintln!`s every envelope to stderr. Under `--json`, `anvil start`
     unauthenticated exits 0 with its structured payload on stderr — a JSON
     consumer reading stdout gets nothing.
- **Expected Outcome:**
  1. `skips_auth_for_local_probe` also matches
     `Commands::Start(args) if args.verify`; full `anvil start` stays
     auth-gated.
  2. The JSON envelope is only ever produced under `--json`, so the call-site
     `eprintln!` becomes a stdout write for **every** envelope — the action
     `authRequired` payload *and* the probe / non-auth error envelopes —
     per the `--json` stream policy in `docs/guides/cli-output-streams.md`
     (structured JSON on stdout, human diagnostics on stderr). Exit-code
     routing is unchanged: the action command coerces to `EXIT_OK`, the probe
     keeps its non-zero code.
- **Validation:** air-gapped subprocess regressions — (1) `anvil start --verify`
  runs unauthenticated without the auth wall; (2) `anvil start --json`
  unauthenticated emits the `authRequired` envelope on stdout with exit 0. Both
  must first fail against the current code (proven-mutant bar).
- **Identified From:** clawpatch review 2026-06-09 — findings `…-_8f848cf67e`
  (api-contract, medium/high) and `…-_00e37f07d6` (api-contract, medium/high),
  both verified real.
- **Coordinates with:** `crates/anvil-cli/src/main.rs`
  (`skips_auth_for_local_probe`, `auth_required_response`, the auth-gate block
  ~L991–1006); `crates/anvil-cli/tests/air_gapped.rs`.
- **Confidence:** high — both root causes located and confirmed; the fix is a
  predicate arm plus routing the `--json` envelope to stdout, tightly scoped to
  one file.

### CIB-050: AST registry load failures silently disable scanning

- **Status:** Merged 2026-06-10 via PR #2475
- **Intent:** In `crates/anvil-checks-ast/src/lib.rs`, `load_rules` early-returns
  `LoadOutcome { rules: [], init_errors: [] }` when
  `load_compiled_registry` yields no registry, discarding
  `LoadRegistryResult.warnings` (documented as "missing file, parse error,
  schema mismatch"). `scan_bytes` then reports a default clean output — no
  warnings, no `patterns_checked`, no init error — so a bad or unloadable
  registry looks like a passing scan and gate-time AST rules are silently
  disabled. This contradicts the ADR-071 §3 "fail loudly, never silently produce
  nothing" guarantee the same function enforces for per-rule failures (missing
  predicate, malformed query, no `@target` capture). Verified against `main` on
  2026-06-09 (clawpatch run `20260609T120358-c18565`, feature
  `feat_library_f65cfd1ba9`).
- **Expected Outcome:** a registry that cannot be loaded or parsed surfaces the
  loader warnings through `AstScanOutput` (folded into `init_errors`, or a new
  explicit `registry_errors`/`registry_warnings` field the CLI surfaces like AST
  query/predicate init failures) rather than producing a silent clean result.
- **Validation:** a focused test that a missing / malformed registry yields a
  non-empty error channel in `AstScanOutput` (must fail against current code,
  which returns an empty `init_errors`).
- **Identified From:** clawpatch review 2026-06-09 — finding
  `fnd_sig-feat-library-f65cfd1ba9-d89a_7c56581562` (bug, medium/high), verified
  real.
- **Coordinates with:** `crates/anvil-checks-ast/src/lib.rs` (`load_rules`
  `None`-branch + `AstScanOutput` shape); `crates/anvil-checks/src/antipattern/registry_loader.rs`
  (`LoadRegistryResult.warnings`).
- **Confidence:** high — root cause located (dropped `loaded.warnings`); the fix
  is propagating an already-computed warning channel plus one test.

### CIB-051: `anvil start --verify --format` is silently ignored

- **Status:** Merged 2026-07-02 via PR #3078
- **Intent:** `anvil start --verify` computes `read_only = args.verify ||
  global.json` and the `--format <ext>` first-run config write
  (`pre_write_anvil_config`) is gated on `!read_only`, so
  `start --verify --format yaml` silently drops `--format` with no feedback.
  The other mutating-flag combinations are rejected explicitly —
  `--watch --verify` and `--new-identity --verify` both `bail!` before any
  side effect, and `--new-identity` documents "Incompatible with `--verify`
  (read-only)" — but `--format` has no analogous rejection, no doc note on
  the arg, and no test pinning the silent-ignore, so the no-op can silently
  stop being a no-op if the gating or ordering changes.
- **Expected Outcome:** the combination is made consistent with the sibling
  flags — either `start --verify --format <ext>` bails like `--new-identity`
  (preferred for consistency), or the `--format` arg doc explicitly states it
  is ignored under `--verify`. Either way a regression test pins the chosen
  contract.
- **Validation:** a test for `start --verify --format yaml` asserting the
  chosen behaviour (clean bail before side effects, or no-op with no config
  file written).
- **Identified From:** council review of PR #2474 (CIB-049),
  adversarial-reviewer MINOR — flag-combination sweep of the
  `skips_auth_for_local_probe` expansion.
- **Coordinates with:** `crates/anvil-cli/src/commands/start.rs` (`read_only`
  gating, `pre_write_anvil_config`, the `--watch`/`--new-identity` bails);
  `crates/anvil-cli/tests/start.rs`.
- **Confidence:** high — behaviour confirmed from the code; the open choice
  (bail vs document) is small and local.

### CIB-052: admin JSON auth errors still go to stderr

- **Status:** Merged 2026-07-02 via PR #3080
- **Intent:** PR #2474 (CIB-049) routed the pre-dispatch auth-gate `--json`
  envelopes to stdout per the stream policy
  (`docs/guides/cli-output-streams.md`). `admin.rs::print_auth_required`
  is now the only place in the CLI that emits a JSON-shaped auth error to
  stderr. The admin surface authenticates via `ANVIL_ADMIN_KEY` (it is not in
  the gated-command set, so it never hits the main auth wall), which makes the
  divergence functionally harmless today — but it is a maintenance trap for
  anyone extending the admin surface under `--json`, and an inconsistency for
  scripted admin consumers.
- **Expected Outcome:** the admin JSON auth-error output follows the stream
  policy (structured JSON on stdout, exit codes unchanged), or — if the admin
  surface is deliberately exempt — `print_auth_required` carries an explicit
  comment documenting the divergence and why.
- **Validation:** if the stream changes, a test asserting the admin
  auth-error JSON lands on stdout with the existing exit code; if
  documented-divergence is chosen, the comment is the deliverable (doc-only,
  no test).
- **Identified From:** council review of PR #2474 (CIB-049),
  adversarial-reviewer NIT — consistency sweep after the envelope stream
  change.
- **Coordinates with:** `crates/anvil-cli/src/commands/admin.rs`
  (`print_auth_required`).
- **Confidence:** medium — mechanics are trivial; the open question is
  policy (align vs document), and admin consumers parsing stderr today would
  see a stream change.

### CIB-053: disposition the dogfood repo's tracked `.anvil/` paths

- **Status:** Merged 2026-06-10 via PR #2481
- **Intent:** the GITGOV-014 doctor `state-boundary` check (ADR-073) warns in
  this repo because four `.anvil/` runtime paths are git-tracked:
  `.anvil/baseline.json` (pre-boundary legacy — ADR-073 places the durable
  baseline at `anvil/baseline.json`), `.anvil/config.yml` (stale policies
  example), `.anvil/plans/aps-test123.json` (old test fixture), and
  `.anvil/dashboards/gate-summary.dashboard.json` (deliberate — the
  `include_str!` single-source spec embedded by `init.rs`). The first three
  look like accidents; the fourth is justified but contradicts ADR-073's "no
  tracked sub-path is justified today" line.
- **Expected Outcome:** each of the four paths is either untracked (with any
  consumer repointed), relocated (e.g. the embedded dashboard spec moved to a
  source dir outside `.anvil/`), or recorded as a justified exception in
  ADR-073; the dogfood `anvil doctor` state-boundary warn reflects only
  recorded deviations.
- **Validation:** `git ls-files .anvil` lists only paths with a recorded
  ADR-073 justification; `cargo test -p eddacraft-anvil seeds_gate_summary`
  stays green if the embedded spec moves.
- **Identified From:** GITGOV-014 implementation — first real-world run of
  the new doctor state-boundary check on this repo.
- **Coordinates with:** `crates/anvil-cli/src/commands/init.rs`
  (`include_str!` path), ADR-073, GITGOV-014.
- **Confidence:** medium — the three legacy paths need a quick consumer
  sweep before untracking; the dashboard-spec move touches the init seeding
  test and the dogfood dashboard workflow.

### CIB-054: pre-tag v0.8.0 changelog claims must match shipped behaviour

- **Status:** Merged 2026-06-10 via PR #2493
- **Intent:** two draft v0.8.0-beta changelog entries over-claim against the
  code: the `anvil watch` "Changed" entry never says daemon routing needs a
  live daemon (`anvil start`) and silently falls back without one
  (DSV-021 `DefaultOnWhenLive` contract), and the MCP entry says
  `validate_write` uses "the same daemon save-time path … converge on the
  same verdict assembly" when MCP deliberately uses the daemon `scan_buffer`
  verb, not watch's `validate_paths` (DSV-007 Task 13 decision).
- **Expected Outcome:** the watch entry names the live-daemon condition and
  `anvil start`; the MCP entry says "daemon-backed" rather than "same
  path"/"same verdict assembly"; the public docs changelog mirrors both
  corrections. Lands before the `v0.8.0-beta` tag cut.
- **Validation:** `pnpm docs:check` green; entries reviewed against
  `crates/anvil-cli/src/commands/watch_save_time.rs` (routing modes) and
  `crates/anvil-cli/src/mcp/validation.rs` (`scan_buffer`).
- **Files:** `CHANGELOG.md`, `docs/public/anvil/releases/changelog.md`
- **Identified From:** 2026-06-10 v0.8.0-beta user-journey completeness
  review (operator session).
- **Coordinates with:** UJ-006 (watch help/advisory wording), the NBI rank-1
  release-cut gate.
- **Confidence:** high — text-only, behaviour contract already verified
  against code.

### CIB-055: reconcile the stale RELEASE-PLAN.md v0.8.0 phase table

- **Status:** Merged 2026-06-10 via PR #2496
- **Intent:** the active-window phase table still shows the A→A′ backing swap
  as **Blocked** and "GV2 4/19", but the slice is complete — GV2-022/027/028/029
  landed on the #2442/#2446 wave, GV2-024 (#2470) + GV2-025 (#2459) closed the
  A′ hardening, and DSV-021 (#2473) flipped default-on routing (the index NBI
  note and GV2 13/20 already say so). Anyone reading the release plan to judge
  cut readiness gets the wrong answer.
- **Expected Outcome:** the v0.8.0 phase-table rows reflect the
  Merged/Done state with PR references, consistent with `plans/index.aps.md`;
  no second window or header-shape change is introduced.
- **Validation:** `pnpm release-plan:check` green; phase-table states match
  the index NBI note.
- **Files:** `RELEASE-PLAN.md`
- **Identified From:** 2026-06-10 v0.8.0-beta user-journey completeness
  review (operator session).
- **Coordinates with:** the NBI rank-1 release-cut gate.
- **Confidence:** high — bookkeeping reconcile against already-verified
  merge state.

### CIB-056: Driver-client Windows pipe gate must verify the current-user SID

- **Status:** Merged 2026-06-10 via PR #2485
- **Intent:** the `Transport.connect` contract
  (`packages/anvil-driver-client/src/transport/types.ts`) promises a
  platform owner gate before `connect()` resolves, but the Windows
  implementation (`validateWindowsPipeName` in
  `packages/anvil-driver-client/src/transport/windows.ts`) only shape-checks
  the `\\.\pipe\anvil-intercept-<sid>` pattern. It never compares the SID
  suffix to the current user's SID, so a misconfigured or
  attacker-influenced `pipeName` targeting another user's daemon pipe
  connects successfully and is treated as trusted. The Unix transport has
  the real gate (mode-0600 + current-uid stat); the daemon side already
  binds owner-only (INTD-002) and checks client SIDs (DSV-010b/ADR-070) —
  this is the missing client half. Clawpatch finding
  `fnd_sig-feat-cli-command-a4f9ddbd8c-_55d076e44e`, promoted from umbrella
  #1826 to focused issue #2484.
- **Expected Outcome:** on win32, `connect()` rejects with
  `anvil-daemon-wrong-owner` when the pipe-name SID suffix does not match
  the current user's SID (derived via an injectable provider; default
  implementation shells out to `whoami /user`, fails closed on resolution
  failure). The shape check, error codes, and public API stay
  backward-compatible. The deeper pipe-squat defence (server SID via
  security descriptor or handshake attestation) stays a documented
  follow-up on #2484, not silently claimed.
- **Validation:** vitest unit tests (runnable on Linux via injected SID
  provider) covering: suffix == current SID accepted; mismatched SID
  rejected with `anvil-daemon-wrong-owner`; SID-resolution failure rejects
  (fail closed); existing shape-check cases unchanged.
- **Identified From:** clawpatch v0.7.0-beta sweep (2026-05-21), top of the
  open-findings queue in the 2026-06-10 triage; promoted per #1826's
  load-bearing-consumer rule (surface-drivers DRVR-001 is live, ADR-030).
- **Coordinates with:** issue #2484, umbrella #1826,
  `crates/anvil-intercept-win32/src/lib.rs::pipe_name_for_current_user`
  (canonical `S-1-…` SID string both sides must agree on), INTD-012
  successor work for Windows CI coverage.
- **Confidence:** high — narrow, well-understood gate with an established
  Unix analogue; the only platform-sensitive piece (SID lookup) is
  injection-seamed for tests.

### CIB-057: capsule subsystem as-built architecture doc

- **Status:** Merged 2026-06-10 via PR #2512
- **Intent:** the git-native governance capsule wedge (GITGOV-003..013,
  `crates/anvil-capsule` + the `anvil capsule` CLI lane, ~8k source lines,
  ADRs 072/073/074/078) shipped with zero internal architecture coverage —
  the only "capsule" mentions in `docs/architecture/` are Kindling capture
  capsules, a different concept. The subsystem rides the v0.8.0-beta window;
  the next maintainer or reviewer has no source-pinned map of the manifest
  formats, verify engine, exit-code contract, or retention semantics.
- **Expected Outcome:** `docs/architecture/capsule-as-built.md` following
  `_as-built-template.md` — lifecycle (create/verify/prune) with file:line
  pins, surfaces table, invariants (canonical-JSON digesting,
  present-but-empty discipline, verdict-laundering guard, full-chain
  witness), known-gaps register, source references — plus the
  `docs/architecture/README.md` index entry.
- **Validation:** `pnpm docs:check` green (validates code-wrapped source
  paths in governed as-builts); `pnpm run format:check` green;
  `node scripts/aps/index-counts.mjs --check` green.
- **Identified From:** 2026-06-10 internal-architecture-docs drift
  assessment (f5-dev-workflow session): all as-builts stamped 2026-05-07
  against `v0.6.0-beta`; the 2026-06-08 sweep (#2371) fixed paths only;
  capsule subsystem had no as-built at all.
- **Coordinates with:** GITGOV module closeout (release-evidence gate),
  ADR-078, the public concepts page
  `docs/public/anvil/concepts/review-capsules.md`.
- **Confidence:** high — documentation of already-merged, tested behaviour;
  every claim line-pinned against main `d6e7b4189`.

### CIB-058: as-built delta refresh for the v0.8.0 window

- **Status:** Merged 2026-06-10 via PR #2515
- **Intent:** five internal as-builts (kernel, intercept, activation,
  driver-framework, tui) are stamped 2026-05-07 against `v0.6.0-beta` and
  predate the v0.8.0 window's architecture changes: the DSV save-time
  validation arc (`validate_paths` via the GV2 hot-read certify, three new
  IPC verbs, confinement/admission, Windows peer-SID accept gate), DSV-021
  `ANVIL_WATCH_DAEMON` default-on routing, GV2-024 hot-read seal + ADR-077
  depth cap (merged AFTER the 2026-06-08 kernel §7 re-verify), INTR-003..-007
  rule set + config, the TUIDASH dashboard surface family, and ADR-080
  welcome ungate. driver-framework-as-built also carries a factually
  inverted panic-policy claim (says `panic="abort"`; workspace ships
  `panic="unwind"` per ADR-051). The 2026-06-08 sweep (#2371) fixed source
  paths only.
- **Expected Outcome:** targeted delta updates to the five docs — stale
  claims corrected, missing subsystems documented with verified file:line
  pins, freshness stamps bumped with honest delta-review scoping —
  plus matching `docs/architecture/README.md` blurb updates. Not a full
  re-verification of untouched sections.
- **Validation:** `pnpm docs:check` green; `pnpm run format:check` green;
  `node scripts/aps/index-counts.mjs --check` green; adversarial pin
  fact-check pass on the edited sections.
- **Identified From:** 2026-06-10 internal-architecture-docs drift
  assessment (same f5-dev-workflow session as CIB-057); five parallel
  per-doc drift agents verified the deltas against source at `a1c41e284`.
- **Coordinates with:** DSV/GV2/INTR/UJ module closeouts, ADR-077/-080,
  the v0.8.0-beta release-cut readiness gate (NBI rank 1).
- **Confidence:** high — documentation of already-merged behaviour;
  every correction carries verified pins.

### CIB-059: quickstart leads with the ungated `anvil welcome` demo

- **Status:** Merged 2026-06-22 via PR #2873
- **Intent:** the quickstart fronts "2. Authenticate" before "3. Take a
  Path" and never says `anvil welcome` runs without logging in — it hides
  the ADR-080 ungated demo surface from exactly the invite-less users it
  was created for (UJ-003 / PR #2503 landed before ADR-080 / PR #2509 and
  never got the back-edit).
- **Expected Outcome:** `docs/public/anvil/quickstart.md` reflects the
  ADR-080 posture: `anvil welcome` is presented as runnable immediately
  after install with no login, and authentication is introduced where it
  is actually required — at `anvil start` and the other durable surfaces.
  No change to the documented auth flows themselves.
- **Files:** `docs/public/anvil/quickstart.md`
- **Validation:** `pnpm docs:check` green; the rewritten section order
  matches the live gate behaviour (`welcome` ungated, `start` gated)
  verified in the 2026-06-10 walkthrough.
- **Identified From:** 2026-06-10 beta user-journey live walkthrough
  (operator session); cross-checked against ADR-080 and
  `CLI_GATED_COMMANDS` in `crates/anvil-cli/src/feature_flags.rs`.
- **Coordinates with:** UJ-003/UJ-004 (shipped), FLAGCAT-008 (GA gate
  revisit), the v0.8.0-beta release-cut readiness gate.
- **Confidence:** high — docs-only, behaviour already shipped.

### CIB-060: auth wall points users without beta access at the request channel

- **Status:** Merged 2026-06-22 via PR #2858
- **Intent:** the not-logged-in gate message says only "Authentication
  required. Run `anvil auth login` to authenticate." — a user without a
  beta invite dead-ends with no pointer to how to get access.
- **Expected Outcome:** the unauthenticated (`Ok(None)` credentials) gate
  surface names the early-access channel (`https://eddacraft.ai`) on both
  the human stderr message and, additively, the `--json` envelope. The
  expired-session and invalid-edict messages stay unchanged (those users
  already have access). Exit-code routing (#1822) unchanged.
- **Files:** `crates/anvil-cli/src/main.rs`
- **Validation:** `cargo test -p eddacraft-anvil` message/envelope
  assertions; manual unauthenticated transcript shows the pointer.
- **Identified From:** 2026-06-10 beta user-journey live walkthrough
  (operator session); request channel per `README.md` ("Early access at
  eddacraft.ai").
- **Coordinates with:** UJ-004/ADR-080 (gate placement), issue #1822
  (exit-0 posture).
- **Confidence:** high — message-surface change with existing test
  coverage to extend.

### CIB-061: as-built delta refresh — remaining eight docs + runbook tail

- **Status:** Merged 2026-06-10 via PR #2526
- **Intent:** the eight as-builts not covered by CIB-058 (mcp-shim, checks,
  auth, api, tutorial, widgets, adapter-packages, observability) carry stale
  or inverted central claims: the MCP shim doc says "one tool" (registry now
  ships 8, RMCPF-010..-012); the observability doc says the redaction
  deny-list is uncalled (the redacting formatter is live in `init_tracing`,
  TRACE-003); the widgets doc says eddacraft-tui is consumed from crates.io
  v0.1.0 with 13 widgets (it is the in-monorepo path crate v0.3.0 per
  ADR-047, 22 widgets); the auth doc omits the merged GitHub OAuth flow
  (GHCLIAUTH-003) and claims a hardcoded identity claim; the api doc misses
  `/admin/broadcast`, the trace-context + admin-rate-limit middleware, and
  four migrations; the checks doc says 18 rules/5 families (27/7, plus the
  undocumented `anvil-checks-ast` AST tier, ADR-071) and 18 secret patterns
  (21); the tutorial doc claims showcase wiring "not yet connected" (wired
  via welcome, G-04 resolved); the adapters doc says 8 validator rules (15)
  and flags a resolved schema-drift gap (DOCGOV-003). Also: activation
  as-built's watch-fallback section carries v0.6.0-era `start.rs` pins, and
  `docs/runbooks/cli-surface.md` lacks the `anvil capsule prune` row
  (capsule-as-built G-06).
- **Expected Outcome:** targeted delta updates to the eight docs (stale
  claims corrected, missing surfaces documented with verified pins, gaps
  resolved/narrowed honestly, dual-date freshness stamps), the activation
  pin fix, the runbook prune row, and matching `docs/architecture/README.md`
  blurbs. Widgets gets corrections + a new-widget summary table, not 9
  authored deep-dives (deep-dives recorded as a known gap).
- **Validation:** `pnpm docs:check` green; `pnpm run format:check` green;
  `node scripts/aps/index-counts.mjs --check` green; adversarial pin
  fact-check on the diff.
- **Identified From:** 2026-06-10 internal-architecture-docs drift run
  (continuation of CIB-057/-058); eight parallel per-doc drift agents
  verified deltas against source at `45dd1047a`.
- **Coordinates with:** RMCPF/TRACE/GHCLIAUTH/EMAIL/RSTLAN/LANGTS/ADR-071
  module closeouts; capsule-as-built G-06.
- **Confidence:** high — documentation of already-merged behaviour; every
  correction carries verified pins.

### CIB-062: auth-gate tracing WARN leaks raw JSON onto golden-path stderr

- **Status:** Merged 2026-06-11 via PR #2529
- **Intent:** running a gated command unauthenticated (`anvil status`;
  `anvil welcome` too on pre-ADR-080 installed builds) prints a raw JSON
  tracing line — `{"timestamp":…,"level":"WARN","fields":{"message":"cli
  command authentication required"},…}` — to stderr directly under the
  human auth message, duplicating it as machine noise on the beta golden
  path.
- **Expected Outcome:** the pre-dispatch auth-gate event in
  `crates/anvil-cli/src/main.rs` is emitted at `info`, not `warn` —
  auth-required is an *expected state* per issue #1822 (exit 0,
  informational) — so the CLI's default `warn` filter (CIB-024) keeps
  stderr clean while `ANVIL_LOG=info` still surfaces the event for
  operators. The human stderr message from `check_auth` is unchanged.
- **Files:** `crates/anvil-cli/src/main.rs`
- **Validation:** integration test asserting an unauthenticated gated
  command's stderr carries no `"level":"WARN"` JSON line at the default
  filter; `cargo test -p eddacraft-anvil`.
- **Identified From:** 2026-06-10/11 beta golden-path walkthrough;
  reproduced live against a fresh debug build (`status`) and the
  installed pre-ungate binary (`welcome`).
- **Coordinates with:** CIB-024 (CLI stream policy), CIB-060 (same gate
  surface, message content — independent edits), issue #1822.
- **Confidence:** high — one-line severity change with the stream policy
  already documented.

### CIB-063: install.sh docs URL disagrees with the product surfaces

- **Status:** Merged 2026-06-11 via PR #2528
- **Intent:** `install.sh` closes with `https://eddacraft.dev/docs` while
  `README.md`, `anvil welcome`, and the what's-new banner all point at
  `https://docs.eddacraft.ai` — the first URL a new user sees disagrees
  with every later one.
- **Expected Outcome:** `install.sh` prints `https://docs.eddacraft.ai`,
  matching the canonical docs origin used by the product surfaces.
- **Files:** `install.sh`
- **Validation:**
  `grep -q "docs.eddacraft.ai" install.sh && ! grep -q "eddacraft.dev/docs" install.sh`;
  `bash -n install.sh`.
- **Identified From:** 2026-06-10/11 beta golden-path walkthrough
  (install → welcome transcript comparison).
- **Coordinates with:** DISTRIB-002 (install surface), CIB-059
  (quickstart wording).
- **Confidence:** high — one-line string change.

### CIB-064: one planted secret reports under two SECRET rule IDs

- **Status:** Merged 2026-06-11 via PR #2530
- **Intent:** a planted `OPENAI_API_KEY = "sk-proj-…"` line reports twice
  — `SECRET-API-KEY` (low-confidence keyword pattern) and
  `SECRET-OPENAI-API-KEY` (high-confidence shape pattern) — on the same
  string, double-counting one credential in the demo/golden-path scan
  output.
- **Expected Outcome:** in
  `crates/anvil-checks/src/secret/scanner.rs`, a low-confidence keyword
  match whose range overlaps a high-confidence shape match on the same
  line is suppressed — the same credential reports once, under the
  precise provider rule ID. Non-overlapping matches and lines with only
  low-confidence matches are unchanged; custom patterns (always
  low-confidence) follow the same rule.
- **Files:** `crates/anvil-checks/src/secret/scanner.rs`
- **Validation:** unit tests — overlap dedups to the high-confidence
  finding; non-overlapping pairs still report both; `cargo test -p
  eddacraft-anvil-checks` and `cargo test -p eddacraft-anvil` (the
  consuming `check`/`gate` surfaces).
- **Identified From:** 2026-06-10/11 beta golden-path walkthrough;
  reproduced via `anvil check` on a planted OpenAI project key.
- **Coordinates with:** issue #1800 (high-confidence pattern class),
  SARIFOUT-003 (`SECRET-*` rule-id projection).
- **Confidence:** high — root cause located (per-pattern scan loop has
  no cross-pattern overlap dedup); fix is local to the scanner.

### CIB-065: schema `$id` URIs still use the retired eddacraft.dev domain

- **Status:** Merged 2026-06-11 via PR #2539 (owner decision: target
  domain is `https://docs.eddacraft.ai`; filename mismatch reconciled by
  renaming the file to `workflow-session-event.v1.schema.json` to match
  the `$id` and the siblings' v1-in-filename convention)
- **Intent:** the three published JSON Schemas declare `$id` URIs under
  `https://eddacraft.dev/anvil/schemas/…` while every product surface
  (README, `anvil welcome`, what's-new, install.sh as of CIB-063) uses
  the eddacraft.ai domain family — the schemas' network identity points
  at a domain the product no longer presents.
- **Expected Outcome:** the `$id` values in `schemas/anvil-status.v1.json`,
  `schemas/anvil-insights.v1.json`, and
  `schemas/workflow-session-event.v1.schema.json` move to the canonical
  domain `https://docs.eddacraft.ai/anvil/schemas/…` (owner decision
  2026-06-11; `eddacraft.ai` was the considered alternative). Blast
  radius is the three files plus prose path references: all in-repo
  consumers (`status.rs` doc comment, `status_json_contract.rs`)
  reference the schemas by file path, and no code constructs or resolves
  the `$id` URLs (verified by grep, 2026-06-11). The
  `workflow-session-event` filename/`$id` mismatch is reconciled by
  renaming the file to `…v1.schema.json` to match the `$id`.
- **Files:** `schemas/anvil-status.v1.json`,
  `schemas/anvil-insights.v1.json`,
  `schemas/workflow-session-event.v1.schema.json` (renamed from
  `…schema.json` to match the `$id`), `.claude/commands/council.md`
  (path reference)
- **Validation:** `git grep -c "eddacraft.dev" -- schemas/` = 0;
  `cargo test -p eddacraft-anvil --test status_json_contract` green
  (schema still parses and validates the emitted payload).
- **Identified From:** CIB-063 reviewer sweep (PR #2528) — flagged as
  out of scope for the one-line install.sh fix.
- **Coordinates with:** CIB-063 (docs-domain alignment), DISTRIB-002
  (shipped-surface consistency).
- **Confidence:** medium — mechanical edit, but the target domain (and
  whether `$id` stability matters to any external consumer) is an owner
  call before execution.

---

### CIB-066: `/auth/verify` rejects the licence-JWT credential interactive logins store

- **Status:** Merged 2026-06-11 via PR #2566 (deployed + verified live
  2026-06-12: `/health` 200 `verifyingKey: "ok"`; real production smoke on
  the released v0.8.1-beta binary — device-flow `anvil auth login` →
  `anvil auth whoami` returns the authenticated identity with plan `pro`
  served by the new licence path)
- **Intent:** a freshly logged-in user's first `anvil auth whoami` fails with
  "Stored credentials are invalid or expired" — interactive logins (GitHub
  device flow and OTP) store the ES256 licence JWT as the credential and the
  CLI posts it to `/api/v1/auth/verify`, but that endpoint only accepted
  `anvil_beta_…` access tokens (and its zod schema capped `token` at 200
  chars, rejecting any JWT with a 400 before verification ran). Found during
  the pre-signup production sweep after v0.8.1-beta; the GHCLIAUTH-011 E2E
  could not catch it because it mocks the server (cross-boundary contract
  gap). Related root cause: `LICENSE_PUBLIC_KEY` was wired to the docs apps
  only, never anvil-api, so the API could sign licences but not verify them —
  also why prod `/health` 503s with `verifyingKey: "unavailable"`.
- **Expected Outcome:** `/auth/verify` accepts both credential forms: the
  access-token path is unchanged, and a non-`anvil_beta_` token is verified
  as a licence via `verifyLicence` with an account-status gate
  (`findUserById`, `status === 'active'`) for revocation parity — returning
  `{valid, isEdict: false, user: {email, plan: tier}, scopes}`; a
  verifying-key load failure returns 503 `verification_unavailable` (server
  misconfiguration, never "your credentials are invalid"); the schema cap
  rises to 4096; `infra/src/vercel.ts` wires `LICENSE_PUBLIC_KEY` into the
  anvil-api env (fixing the `/health` degraded 503 on deploy).
- **Files:** `apps/anvil-api/src/routes/auth.ts`,
  `apps/anvil-api/src/__tests__/auth.test.ts`, `infra/src/vercel.ts`
- **Validation:** deploy order — `LICENSE_PUBLIC_KEY` must reach the
  anvil-api Vercel env (pulumi up or UI) **before or with** the code deploy,
  else the licence path 503s in the window (loud and distinguishable from
  "invalid credentials", but avoidable). `pnpm nx test @eddacraft/anvil-api`
  — licence-path tests
  cover valid/active, suspended, unknown subject, expired, tampered
  signature, non-credential string, and key-unavailable→503, signed and
  verified through real ES256 keys; after deploy + Pulumi apply:
  `curl https://api.eddacraft.ai/api/v1/health` returns 200 with
  `verifyingKey: "ok"`, and a real `anvil auth login` → `anvil auth whoami`
  round-trip succeeds on the released v0.8.1-beta binary.
- **Identified From:** pre-signup production sweep, 2026-06-12 (operator
  question "anything before people start signing up?").
- **Coordinates with:** GHCLIAUTH (the flow that surfaces it), GHCLIAUTH-011
  (the E2E that needs a follow-up contract leg), DOCSAUTH (existing
  `LICENSE_PUBLIC_KEY` consumers).
- **Confidence:** high — bounded route change with end-to-end-keyed tests.

---

### CIB-067: production email failures are silent — Resend key probe on /health

- **Status:** Merged 2026-06-12 via PR #2568
- **Intent:** a revoked Resend API key produced a ~15-day production email
  outage (no invites, no OTP codes, no waitlist confirmations) that no
  surface reported — email senders are best-effort by design and
  `/auth/otp/request` reports success regardless for anti-enumeration.
  Discovered 2026-06-12 by the pre-signup invite/OTP smoke: the CLI said
  "code sent", Resend's dashboard showed nothing sent in 15 days, and the
  Key Vault key answered 401 `validation_error` to a direct send.
- **Expected Outcome:** the GHCLIAUTH-002 credential-probe pattern extends
  to email: `verifyResendKey()` validates the key with a cheap
  authenticated read (`GET /domains`, 5s timeout, result cached 5 minutes
  so health polling cannot hammer Resend), distinguishing `ok`
  (including sending-only keys, whose distinct `restricted_api_key`
  rejection proves the key is alive), `invalid` (dead key), `unconfigured`
  (missing env), and `unverifiable` (Resend/network failure). Boot logs a
  non-ok status; `/health` carries a `resendKey` field and gates
  `degraded` (503) on `invalid`/`unconfigured` only — `unverifiable` is
  reported without gating since it is not our misconfiguration.
- **Files:** `apps/anvil-api/src/lib/resend-credentials.ts` (new),
  `apps/anvil-api/src/lib/__tests__/resend-credentials.test.ts` (new),
  `apps/anvil-api/src/index.ts`,
  `apps/anvil-api/src/__tests__/health.test.ts`
- **Validation:** `pnpm nx test @eddacraft/anvil-api` — probe tests cover
  accepted/restricted/dead keys, Resend 5xx, network failure,
  missing-env (no network touch, not cached), and cache behaviour; health
  tests cover the three gating outcomes. After the key replacement
  deploys: `curl https://api.eddacraft.ai/api/v1/health` shows
  `resendKey: "ok"` and the invite + OTP smokes deliver real email.
- **Identified From:** invite/OTP production smoke, 2026-06-12 (operator
  report: "resend has sent no emails in the past 15 days").
- **Coordinates with:** CIB-066 (same pre-signup sweep), GHCLIAUTH-002
  (the probe pattern), EMAIL (the sender surfaces).
- **Confidence:** high — mirrors an established pattern with real-key
  failure modes captured in tests.

---

### CIB-068: invite/OTP email copy — install step, lowercase brand, larger prose

- **Status:** Merged 2026-06-12 via PR #2569
- **Intent:** operator review of the first real invite + OTP sends
  (2026-06-12, after the CIB-067 key replacement) found the invite assumes
  anvil is already installed, both subjects capitalise the brand
  ("Anvil" — the product name is lowercase everywhere user-facing), and
  the prose sizes read small.
- **Expected Outcome:** the invite gains a "First, install anvil" step
  (the quickstart's curl one-liner + the Windows variant) ahead of the
  sign-in instructions, in both the React template and the plain-text
  body; both subjects and body prose use lowercase "anvil"; prose font
  sizes in both templates step up (14→15 body/code, 13→14 labels/muted).
- **Files:** `packages/transactional/emails/beta-invite.tsx`,
  `packages/transactional/emails/otp-code.tsx`,
  `packages/transactional/emails/__tests__/render.test.tsx`,
  `apps/anvil-api/src/lib/email.ts`
- **Validation:** `pnpm nx test @eddacraft/transactional` (render test
  asserts the install line) and `pnpm nx test @eddacraft/anvil-api`; a
  real invite + OTP send shows the new copy.
- **Identified From:** operator copy review of the live smoke emails,
  2026-06-12.
- **Coordinates with:** CIB-067 (the smoke that produced the emails),
  GHCLIAUTH-007 (the invite rewrite this amends).
- **Confidence:** high — copy and style only.

---

### CIB-069: invite email copy v2 — operator-supplied structure

- **Status:** Merged 2026-06-12 via PR #2570
- **Intent:** the operator reviewed the live CIB-068 invite and supplied a
  restructured body (2026-06-12): lead with the beta guide and an install
  options page, then quick-install commands per platform, then sign-in,
  then a documentation pointer.
- **Expected Outcome:** the BetaInvite template and plain-text body carry
  the operator's structure verbatim, with the beta-guide link corrected to
  the live path `https://docs.eddacraft.ai/anvil/beta-testing-guide`
  (the draft's `/beta-testing-guide` 404s; the `/anvil/` path serves the
  docs auth wall a freshly-invited user can pass); section headings,
  inline links, and per-platform code blocks render in the existing
  template idiom; the render test asserts both install commands and all
  three links.
- **Files:** `packages/transactional/emails/beta-invite.tsx`,
  `packages/transactional/emails/__tests__/render.test.tsx`,
  `apps/anvil-api/src/lib/email.ts`
- **Validation:** `pnpm nx test @eddacraft/transactional` +
  `pnpm nx test @eddacraft/anvil-api`; a real invite send shows the new
  structure.
- **Identified From:** operator copy review, 2026-06-12.
- **Coordinates with:** CIB-068 (the copy iteration this supersedes).
- **Confidence:** high — operator-specified copy.

---

### CIB-070: `anvil admin auth set key` — store the admin key without per-shell export

- **Status:** Merged 2026-06-13 via PR #2577
- **Intent:** the admin key has only two resolution paths today — the
  `ANVIL_ADMIN_KEY` env var or a 1Password reference (`anvil admin auth set
  1password`). Operators without the 1Password CLI must `export
  ANVIL_ADMIN_KEY` in every shell and routinely forget how (operator report,
  2026-06-13). There is no "store the key once" option.
- **Expected Outcome:** `anvil admin auth set key <key>` (and `set key -` to
  read from stdin so the secret avoids argv/shell history) persists the admin
  key in the existing owner-only `admin-auth.json` (mode 0600) under a new
  `key` source; `resolve_admin_key` returns it directly with `ANVIL_ADMIN_KEY`
  still taking precedence (CI unaffected); the key is redacted to a trailing
  fingerprint (`****` + last 4) in `auth status`, the `set` confirmation, and
  the JSON form — the raw value lives only in the 0600 file; all
  authentication-required hints name the new option; the admin-cli runbook
  documents all three patterns (env / stored key / 1Password) with a
  copy-paste quick start and the plaintext-at-rest tradeoff stated honestly.
- **Files:** `crates/anvil-cli/src/commands/admin.rs`,
  `docs/runbooks/admin-cli.md`
- **Validation:** `cargo test -p eddacraft-anvil -- admin` (parse, resolve,
  env-precedence, empty-key rejection, redaction, no-raw-key-in-status-JSON);
  manual: `anvil admin auth set key -` then `anvil admin auth status` shows a
  masked fingerprint and `anvil admin list` authenticates.
- **Identified From:** operator request, 2026-06-13 ("I need to remember how
  to export it and I always forget").
- **Coordinates with:** CIB-004 (the credential-source config this extends),
  ADMINCLIH (per-operator key model). A future `anvil admin login` over the
  GitHub device flow (admin-scoped licence, staff allowlist) would unify admin
  onto the GitHub identity — ADR-territory, deliberately out of scope here.
- **Confidence:** high — additive CLI source reusing existing config plumbing.

### CIB-071: migrate user-facing diagnostics from `anyhow` to `miette`

- **Status:** Merged 2026-06-14 via PR #2597 (Phase A) + PR #2611 (Phase B)
- **Intent:** Anvil reports violations with file path and line number
  (`from_file`, `to_file`, `import_line` in `anvil-architecture`; file +
  line in AST check findings) but renders them as plain prose strings. As a
  static analyser, Anvil's primary output _is_ per-location diagnostics;
  `miette` provides source-span context, labels, and help text at the
  terminal — the same rendering model as `cargo`/`clippy`. `anyhow` stays
  appropriate for internal/infrastructure errors (`?`-propagation, config
  parse, IO); `miette` replaces it on the user-facing diagnostic surface.
- **Expected Outcome:** `miette` added to the workspace; violation/finding
  output types (`BoundaryViolation`, AST findings, etc.) implement
  `miette::Diagnostic` with source spans and labels; the CLI renders them
  via `miette`'s reporter rather than hand-formatted strings; `anyhow`
  retained for internal error propagation; no behaviour change for
  machine-readable (`--json`) output.
- **Files:** `Cargo.toml` (workspace dep), `crates/anvil-architecture/src/`,
  `crates/anvil-checks-ast/src/`, `crates/anvil-cli/src/` (output
  formatting paths).
- **Validation:** `cargo test --workspace`; golden-path `anvil check` on a
  repo with a known violation shows a source-span block (file, line,
  offending import highlighted); `--json` output unchanged.
- **Identified From:** conversation 2026-06-14 — `anyhow` was the default
  choice at project inception; Anvil now reports per-location findings that
  benefit from rich terminal diagnostic rendering.
- **Coordinates with:** CIB-062 (stderr cleanliness on the golden path).
- **Confidence:** medium — `miette` composing with `anyhow` across crate
  boundaries needs care; machine-readable output path must be audited to
  stay unaffected.

### CIB-072: clear `ready_restart_required` on Windows when daemon attestation is Unreachable

- **Status:** Merged 2026-06-22 via PR #2859
- **Intent:** the same Windows/Scoop/PowerShell beta user who raised #1831
  still hits a stuck `ready_restart_required` on Anvil 0.8.1 (#2583). #1840
  (MLP2-051f) added the daemon-evidence promotion path that graduates a
  handshake-verified MCP client `RestartRequired -> LiveValidation`, which
  cleared the loop on Linux/macOS. On the affected Windows box the daemon
  attestation comes back `Unreachable`, so there is no promotion path and the
  state parks permanently. #2583 was closed by #2590 with copy only - the
  "explain why it cannot clear" half of its acceptance criteria - leaving the
  "clear the state when possible" half open.
- **Expected Outcome:** on Windows native (Scoop, PowerShell) with the MCP
  client installed and the editor restarted, `anvil start --verify` either
  reports `Protecting` (daemon attests live enforcement and the state
  graduates to `LiveValidation`) or gives an actionable, terminating reason
  the daemon is unreachable rather than an open-ended restart instruction;
  daemon attestation succeeds - or fails diagnosably - over Windows
  named-pipe IPC for a genuinely attached client.
- **Files:** `crates/anvil-cli/src/activation/diagnostic.rs` (state
  computation, ~L230-258), `crates/anvil-cli/src/activation/daemon_evidence.rs`
  (attestation variants / promotion path),
  `crates/anvil-cli/src/activation/render.rs` (#2590 copy - reference only).
- **Validation:** `cargo test -p eddacraft-anvil -- activation`; regression
  coverage for the Windows attachment -> `LiveValidation` path; manual on
  Windows/Scoop/PowerShell: install MCP client, restart editor, confirm
  `anvil start --verify` no longer parks at `ready_restart_required`.
- **Identified From:** issue #2609 (filed 2026-06-14), recurrence of #1831
  via beta feedback #2583; #2590 fixed the copy half only.
- **Coordinates with:** #1831 / #1840 (MLP2-051f, the Linux/macOS promotion
  path this extends to Windows), #2583 / #2590 (copy half), CIB-056
  (driver-client Windows pipe SID gate - same Windows named-pipe IPC surface).
- **Confidence:** medium - root cause is understood, but Windows named-pipe
  IPC reachability needs reproduction on a real Windows box before the fix
  shape is certain; may surface a deeper attachment-detection gap.

### CIB-073: cumulative "value caught" scoreboard and shareable scorecard

- **Status:** Draft
- **Intent:** Anvil has no surface that answers "what has Anvil saved me?" —
  `anvil insights` reports a rolling 7-day window (witness events, saves,
  findings, suppressions) plus a Unicode sparkline, but there is no cumulative
  count of risky writes blocked and nothing a user can share. That number is
  the single most compelling artifact for the beta waitlist and the
  fundraise, and the underlying data already lives in the witness chain.
- **Expected Outcome:** a cumulative aggregate (e.g. "writes blocked",
  "secrets caught", "boundary violations prevented" since first run, plus
  rolling 30/90-day) surfaced in `anvil insights`; a `anvil insights --share`
  (or `--format card`) that emits a clean, self-contained shareable artifact
  (PNG or single-file HTML) of the headline numbers with no PII and no repo
  internals leaked; `--json` schema extended (versioned, e.g.
  `anvil.insights.v2`) with the cumulative fields; existing rolling-window
  output unchanged by default.
- **Files:** `crates/anvil-cli/src/commands/insights.rs` (and the insights
  schema), witness-chain aggregation source.
- **Validation:** `cargo test -p eddacraft-anvil -- insights` (cumulative
  aggregation, schema version, redaction); manual: run against a repo with
  recorded blocks and confirm `anvil insights` shows a cumulative total and
  `anvil insights --share` writes a leak-safe card.
- **Identified From:** demo-readiness review 2026-06-14 (beta waitlist +
  fundraise) — `docs/strategy/beta-demo-script.md`; gap: no cumulative or
  shareable value surface.
- **Coordinates with:** witness-chain / provenance surface; CIB-074 (the
  human-readable audit report shares the redaction + rendering concern).
- **Confidence:** high — additive surface over data Anvil already records;
  the only real care item is leak-safe redaction in the shareable artifact.

### CIB-074: human-readable provenance / audit report export

- **Status:** Draft
- **Intent:** the compliance story ("every block is recorded, with the rule
  and the reason, auditable for the EU AI Act") has no on-screen proof today.
  `anvil audit` emits text / JSON / SARIF and `anvil export` emits
  llms.txt / markdown / MCP-resource, but nothing renders the decision
  history as a document a CISO buyer or an auditor could read or hand over.
- **Expected Outcome:** an `anvil audit --format html` (and/or a
  markdown-to-PDF path) that produces a self-contained, human-readable
  provenance report — each enforcement decision with rule id, severity,
  file/line, the reason, and timestamp, grouped and summarised — sourced from
  the witness chain; redaction so secret _values_ never appear in the report
  (only the fact that a secret was caught); existing `--format json|sarif`
  output unchanged.
- **Files:** `crates/anvil-cli/src/commands/audit.rs` (format dispatch),
  witness-chain read path, a report template asset.
- **Validation:** `cargo test -p eddacraft-anvil -- audit` (html/markdown
  rendering, redaction, deterministic output); manual: `anvil audit
  --format html` on a repo with recorded decisions opens a readable report
  with no secret values present.
- **Identified From:** demo-readiness review 2026-06-14 — Scene 6
  (compliance) of `docs/strategy/beta-demo-script.md` has no artifact to
  show; ties to the EU AI Act enforcement positioning.
- **Coordinates with:** CIB-073 (shared redaction + rendering), witness-chain
  / capsule export surface.
- **Confidence:** medium — rendering and redaction are tractable, but the
  report's audit-grade claims should be checked against the witness chain's
  actual integrity guarantees before the compliance framing is published.

### CIB-075: surface a visible always-on protection indicator (macOS menubar)

- **Status:** Draft
- **Intent:** Anvil is purely terminal-based — no menubar, tray, or GUI
  presence. Protection is invisible between commands, and a block lives only
  in the editor's chat until it scrolls away. A persistent visible indicator
  ("Anvil · protecting · N sessions · M blocked today", flashing on a block)
  would make always-on protection tangible — the biggest single lift for the
  fundraise demo, where a visible artifact lands far harder than CLI text.
- **Expected Outcome:** triage + a thin spike: decide the surface (native
  macOS menubar item reading existing daemon/intercept status over the local
  socket vs. a heavier app shell), confirm it can render live session count
  and a today's-blocked count from data the daemon already exposes, and scope
  the smallest shippable version. Likely graduates to a dedicated APS module
  and an ADR (net-new product surface, packaging, and platform story) — this
  CIB item is the triage gate, not the build.
- **Files:** TBD by triage; reads from `anvil intercept` status surface
  (`crates/anvil-intercept`), daemon socket at `$ANVIL_HOME/intercept.sock`.
- **Validation:** spike acceptance — a menubar item that reflects live daemon
  state and increments on a real block; promotion decision recorded.
- **Identified From:** demo-readiness review 2026-06-14 — biggest demo gap is
  the absence of any visible artifact (`docs/strategy/beta-demo-script.md`).
- **Coordinates with:** `anvil intercept status` surface; CIB-073 (the
  today's-blocked count it would display).
- **Promotion note:** likely **Superseded by** a dedicated module + ADR once
  triaged — net-new GUI surface exceeds CIB sizing; filed here for triage per
  the intake rules.
- **Confidence:** low — net-new surface; platform, packaging, signing
  (cf. eddacraft Windows-signing precedent), and maintenance cost are
  unscoped until the spike.

### CIB-076: zero-config architecture/boundary rule so the differentiated feature demos without setup

- **Status:** Draft
- **Intent:** secret detection fires zero-config, but the capability that
  makes Anvil a _category_ rather than a secret scanner — architecture /
  boundary enforcement — only runs through the live daemon against a
  configured baseline, and silently does nothing in a fresh repo. The easy
  demo is the commodity feature and the moat feature is the fragile one,
  which is backwards for the fundraise and for first-run user value.
- **Expected Outcome:** at least one boundary/architecture rule that fires
  from a sensible zero-config default (e.g. an obvious cross-layer import
  detectable without a hand-authored baseline), routed through the same MCP
  `validate_write` path with a `remediation_hint` so an agent can
  self-correct; clear documentation of what is caught with no config vs. what
  needs a baseline; no false-positive regressions on the embedded validator's
  existing default rules.
- **Files:** `crates/anvil-intercept/src/enforcement.rs`
  (`default_rule_registry`), `crates/anvil-architecture/`, the
  `validate_write` response path.
- **Validation:** `cargo test -p eddacraft-anvil` + architecture-crate tests;
  manual: in a fresh repo with no baseline, an obvious cross-layer import via
  the agent returns a `block` with a remediation hint.
- **Identified From:** demo-readiness review 2026-06-14 — the architecture
  block in `docs/strategy/beta-demo-script.md` Scene 4 does not fire
  out-of-the-box (embedded validator runs secret-detection +
  reasoning-pattern only).
- **Coordinates with:** CIB-005 (pre-write validator), the embedded-validator
  rule registry; may need an ADR for the default-on rule set.
- **Promotion note:** the zero-config-defaults question may be ADR territory
  (which rules are safe to run with no baseline) — raise an ADR if triage
  finds the default-rule-set decision contentious.
- **Confidence:** low — picking a default-on architecture rule that is
  genuinely low-false-positive without a baseline is the hard part and may
  need an ADR before implementation.
### CIB-077: resolve `preflight.sh` version-gate vs `prepare.sh` bump ordering

- **Status:** Draft
- **Intent:** the release flow's `preflight.sh` runs a `cargo-version` gate
  (`require_workspace_version_match`) that fails when the workspace version
  equals the latest release tag — treating it as "the engineer forgot to
  bump". But `prepare.sh`, which performs the version bump, runs _after_
  `preflight.sh`. On a hotfix cut from a tagged release the workspace version
  legitimately still equals the latest tag at preflight time, so the gate
  aborts the release before `prepare` ever gets to bump it — a chicken-and-egg
  ordering wall.
- **Expected Outcome:** `preflight.sh` gains a deferred/skip mode (e.g.
  `SKIP_VERSION_GATE=1` or a `--pre-prepare` flag) that bypasses the
  workspace-equals-latest-tag check when the bump is intentionally deferred to
  `prepare.sh`; default behaviour and gate ordering are unchanged so a genuine
  forgotten bump on a normal cut still fails; the release runbook documents
  when the skip is appropriate and the hotfix-from-tag flow it unblocks.
- **Files:** `scripts/release/preflight.sh` (cargo-version gate, ~L195–325),
  `scripts/release/prepare.sh` (bump stage — ordering reference),
  `docs/runbooks/release-runbook.md`.
- **Validation:** manual — re-run a hotfix-from-tag flow with the skip mode and
  confirm `preflight` passes, `prepare` bumps the version, and `promote`/`tag`
  proceed; confirm a normal cut with a genuinely missing bump still fails the
  gate.
- **Identified From:** release attempt 2026-06-14 — v0.8.2-beta hotfix (from
  v0.8.1-beta) aborted twice on the `cargo-version` preflight gate (workspace
  `0.8.1-beta` == latest tag); recovered from a release-attempt log note that
  was lost when a sibling branch switch discarded the uncommitted entry.
- **Coordinates with:** the `scripts/release/*` flow
  (preflight/prepare/promote/tag); release runbook.
- **Confidence:** medium — the ordering tension is clearly diagnosed, but the
  right fix shape (skip flag vs. reordering the gate vs. moving the check into
  `prepare`) is a release-owner call.

### CIB-078: freeze the `anvil policy eval --json` output contract at v1 before EVAL binds

- **Status:** Merged 2026-06-17 via PR #2717
- **Intent:** POLENG-007 shipped `anvil policy eval --json` preview-gated, with
  the wire shape explicitly _not_ a stable contract. The
  [eval-harness-integration](eval-harness-integration.aps.md) module (EVAL) is
  next to bind an adapter (`EvalRunSummary`/`EvalRegressionReport`) to that
  output. Freeze the shape at v1 _before_ EVAL locks onto it so the adapter has
  a durable contract and a later eval-output refactor cannot silently break
  trust-regression gates — the output dual of the existing
  [`PolicyInput` v1 contract](../../docs/specs/policy-input-v1.md).
- **Expected Outcome:** an embedded `schema_version` (`"1.0.0"`, emitted first)
  on `EvalOutput`; a schema-stability snapshot test pinning the **gate-critical**
  surface (`schema_version`/`policy`/`query`/`findings`/`exit_code` plus the
  `Finding`/`Severity` shapes) so an accidental change fails CI; the diagnostic
  fields (`value`/`coverage`/`trace`/`why`) documented as outside the contract
  and free to evolve; an authoritative spec at
  [`docs/specs/policy-eval-output-v1.md`](../../docs/specs/policy-eval-output-v1.md)
  with a deprecation policy; and the EVAL module annotated to bind to the frozen
  surface.
- **Files:** `crates/anvil-cli/src/commands/policy/eval.rs` (`EvalOutput`,
  `EVAL_OUTPUT_SCHEMA_VERSION`, snapshot test),
  `crates/anvil-cli/src/commands/policy/snapshots/`,
  `docs/specs/policy-eval-output-v1.md`,
  `plans/modules/eval-harness-integration.aps.md`.
- **Validation:** `cargo test -p eddacraft-anvil eval::tests`; `pnpm docs:check`.
- **Identified From:** the 2026-06-17 OPA/policy-cluster readiness review — the
  one outstanding Regorus follow-up was that the eval output shape is not yet a
  stable contract, and EVAL was about to harden against it.
- **Coordinates with:** EVAL (eval-harness-integration), POLENG (engine
  substrate, archived), ADR-040.
- **Confidence:** high — direct mirror of the shipped `PolicyInput` v1 +
  `watch-output-contract` patterns; additive field, no behaviour change.

### CIB-079: Rust serde-hygiene + clone-in-hot-loop AST antipattern rules (RSTLAN-003b)

- **Status:** Done 2026-06-27 — opt-in RS-006/RS-007/RS-008 AST rules now cover catch-all `serde(flatten)` validation gaps, plaintext secret fields on `Deserialize` structs, and `.clone()` in syntactic loops; default Rust dogfood output is unchanged for the new rules.
- **Intent:** Land the three AST-dependent Rust reliability rules deferred from
  RSTLAN-003 (the "RSTLAN-003b" follow-on) onto the now-shipped
  `anvil-checks-ast` gate-time mechanism, so the `rust-reliability` catalogue
  covers the serde-hygiene and hot-loop-allocation shapes the regex scanner
  could not express.
- **Expected Outcome:** Three new gate-time AST rules in the `rust-reliability`
  family (provisional ids continuing the shipped RS-001..005 sequence):
  `#[serde(flatten)]` without validation; `Deserialize` on secret-bearing
  types; and `.clone()` inside a hot loop. Each rule fires on representative bad
  Rust, is clean on the good equivalent, and is suppressible via
  `// @anvil-ignore RS-00x -- <reason>`. Default-on/opt-in posture decided per
  rule against the §16.5 #9 FP bar at authoring time (mirroring RS-004 shipping
  opt-in). Daemon save-time path stays regex-only + parser-free (ADR-064/ADR-061
  hot/non-hot split unchanged); these are `anvil check`/`gate` only.
- **Scope:** New rule definitions + tree-sitter queries + Rust predicate tables
  in `crates/anvil-checks-ast` (`Detection::Ast`), registry registration, and
  good/bad fixture pairs.
- **Non-scope:** New detection infrastructure (the `anvil-checks-ast` crate and
  `Detection::Ast` variant already shipped under RSTLAN-003/ADR-071); framework
  packs; changing severity defaults of the shipped RS-001..005 rules.
- **Files:** `crates/anvil-checks-ast/src/`, rule/fixture directories under
  `crates/anvil-checks-ast/`, the compiled rule registry.
- **Validation:** `cargo test -p eddacraft-anvil-checks-ast` (fixture pairs
  assert each rule fires on bad Rust, is clean + suppressible on good Rust);
  end-to-end `anvil check` on a temp Rust file exercising the three shapes.
- **Identified From:** RSTLAN-003 detection-mechanism finding (2026-06-04) and
  [ADR-071](../decisions/071-ast-aware-antipattern-detection.md) — the
  "needs real AST" remainder (serde flatten / secret-field `Deserialize` /
  `.clone()`-in-hot-loop) was explicitly carved out as RSTLAN-003b when
  lang-rust (RSTLAN) shipped 8/8 in v0.8.0-beta and was archived.
- **Coordinates with:** archived [lang-rust](../archive/modules/lang-rust.aps.md)
  (RSTLAN-003), ADR-071 (`anvil-checks-ast`), ADR-064/ADR-061 (daemon hot/non-hot
  split), RSTLAN-008 FP bar.
- **Confidence:** medium — the AST mechanism is proven and the rule shapes are
  well-specified, but the precise hot-loop predicate and secret-field heuristic
  need tuning against the FP bar.

### CIB-080: secret-detection test-vector / fixture false positives

- **Status:** Done 2026-06-26 — secret-detection fixture suppressions are now context-bound and observable; zod/vite SECRET dogfood findings dropped to zero and excalidraw retains only two Google Drive URL-ID entropy residuals.
- **Intent:** Suppress the residual secret-detection false positives the
  external-codebase FP dogfood surfaced beyond the credit-card class (fixed in
  PR #2747): high-entropy base64 test vectors, `mongodb://` validator fixtures
  (zod), and JWT / database-URL / API-key literals that are test data, not
  leaked credentials.
- **Expected Outcome:** the secret detectors gain the masking / test-path /
  field-proximity gating the `AP-*` / `GS-*` rules already have — e.g. skip
  obvious test-vector contexts and require a credential-shaped key in proximity
  — so the language-agnostic secret scan no longer dominates the FP count on
  fixture-heavy repos, without weakening detection of real high-confidence
  patterns (issue #1800 textbook keys must still fire).
- **Files:** `crates/anvil-checks/src/secret/scanner.rs`,
  `crates/anvil-checks/src/secret/entropy.rs`, `crates/anvil-checks/src/secret/patterns.rs`.
- **Validation:** `cargo test -p eddacraft-anvil-checks secret::`; re-run
  `scripts/dogfood/external-fp` over the pinned corpus and confirm the
  `SECRET-HIGH-ENTROPY-STRING` / `SECRET-DATABASE-URL` / `SECRET-JWT-TOKEN`
  counts drop while real-credential fixtures still fire.
- **Identified From:** the 2026-06-18 external-codebase FP dogfood
  ([plans/reviews/2026-06-18-langts-external-fp.md](../reviews/2026-06-18-langts-external-fp.md)
  cross-cutting secret-detection section).
- **Confidence:** medium — the gating patterns are well understood, but
  balancing test-vector suppression against issue #1800's "textbook keys must
  fire" stance needs care.

### CIB-081: RS-005 doc-config (`cfg(doc)` / `cfg(docsrs)`) residual

- **Status:** Done 2026-06-26 — RS-005 now treats `cfg(doc)` / `cfg(docsrs)` as non-shipped doc-only code while preserving `cfg(not(doc))` runtime findings.
- **Intent:** Decide how RS-005 should treat `todo!()` / `unimplemented!()`
  stubs gated by `#[cfg(doc)]` / `#[cfg(docsrs)]` — doc-build-only signature
  stubs that are not shipped runtime code but are still flagged after the
  AST conversion (PR #2743) because `in_cfg_test` only matches `cfg(test)`.
- **Expected Outcome:** either extend the AST cfg predicate to treat
  `doc` / `docsrs` as non-shipped (excluding such stubs) or document the
  decision to keep flagging them as suppressible via
  `// @anvil-ignore RS-005 -- doc-only stub`; whichever is chosen, a fixture
  pins the behaviour.
- **Files:** `crates/anvil-checks-ast/src/predicates.rs`,
  `crates/anvil-checks-ast/src/lib.rs`, `crates/anvil-checks-ast/src/tests.rs`.
- **Validation:** `cargo test -p eddacraft-anvil-checks-ast`.
- **Identified From:** the 2026-06-18 external-codebase FP dogfood
  ([plans/reviews/2026-06-18-rstlan-external-fp.md](../reviews/2026-06-18-rstlan-external-fp.md))
  — tokio `process/mod.rs` / `macros/*.rs` doc-only stubs.
- **Confidence:** medium — small, well-scoped; mostly a policy decision on
  whether doc-only configs count as shipped code.

### CIB-082: tree-sitter-rust parse-skip coverage gap

- **Status:** Proposed 2026-06-18
- **Intent:** Close (or upstream) the tree-sitter-rust 0.24 grammar gaps that
  make real Rust files parse-skip, leaving the `RS-*` AST catalogue silently
  blind to them. The external-FP dogfood hit a 2.3% parse-skip rate on
  alacritty (`src/config/bindings.rs` — bare `~Ident::CONST` macro-matcher
  tokens; `src/display/window.rs` — attributes on function parameters), above
  the internal dogfood's < 2% clean-parse bar.
- **Expected Outcome:** a grammar bump that parses these constructs (or a filed
  upstream issue + a tracked pin), and a parse-skip-rate assertion in the
  external-FP harness so a regression is visible.
- **Files:** `crates/anvil-checks-ast/` (grammar dependency / cache key),
  `scripts/dogfood/external-fp/`.
- **Validation:** re-run `scripts/dogfood/external-fp` over alacritty and
  confirm `anvil-ast-parse-skip` count drops to 0 (or the rate stays logged
  under the bar).
- **Identified From:** the 2026-06-18 external-codebase FP dogfood
  ([plans/reviews/2026-06-18-rstlan-external-fp.md](../reviews/2026-06-18-rstlan-external-fp.md)
  parse-skips section).
- **Confidence:** low — depends on upstream tree-sitter-rust grammar coverage;
  may resolve to a documented limitation rather than a code fix.

### CIB-083: context-stack lexer for template-literal interpolation masking

- **Status:** Done 2026-06-26 — template interpolation masking now uses a carried context stack; strings/comments/regexes inside `${…}` are masked, nested templates remain visible where they are real code, and TS/JS external FP dogfood dropped the eight known GS-001 template-text false positives (AP/GS default findings 2,228 → 2,220; 0 panics).
- **Intent:** Replace the lexer-light interpolation brace-counter in the AP-/GS-
  comment/string masker (`mask.rs`) with a context stack so a `${ … }`
  interpolation is fully re-lexed — masking a `!` / `any` / brace that sits
  inside an interpolation *string or comment* — while still handling nested
  template literals correctly.
- **Expected Outcome:** strings/comments inside an interpolation are masked (no
  residual `!`/`any` false positives there) and a `}` inside such a literal no
  longer affects interpolation depth, with **no** regression on nested template
  literals (the naive single-counter re-lex leaks template-text state to
  end-of-file and mass-masks real assertions — verified against vite
  `proxy.ts`, which is why PR #2746 kept the brace-count version). Fixtures
  cover nested templates, interpolation strings, and brace-in-string.
- **Files:** `crates/anvil-checks/src/antipattern/mask.rs`.
- **Validation:** `cargo test -p eddacraft-anvil-checks`; re-run
  `scripts/dogfood/external-fp` over the TS/JS corpus and confirm GS-001/AP-003
  counts hold or drop with no new false negatives.
- **Identified From:** Copilot review of PR #2746 (template-literal masking) —
  the suggested full re-lex is correct but needs the nested-template-safe stack.
- **Confidence:** medium — the correct design is known (a frame stack); the risk
  is the false-negative class a naive version reintroduces, so it needs careful
  fixtures.

### CIB-084: bound read size/count for baseline + drift-snapshot readers

- **Status:** Merged 2026-06-20 via PR #2815
- **Intent:** Cap read size and file count in the architecture-baseline and
  drift-snapshot readers, which currently use bare `std::fs::read_to_string`
  with no bound — unlike `anvil_config::parse_file`, which caps reads via
  `read_to_string_bounded` (1 MiB, MLP2-060). `load_baseline`
  (`crates/anvil-architecture/src/baseline.rs`) and `load_snapshot_file`
  (`crates/anvil-cli/src/commands/drift.rs`) read whole files unbounded, and
  `list_snapshot_files` reads + deserialises *every* snapshot in
  `.anvil/snapshots/` to sort by `created_at` with no count cap. A hostile or
  corrupt local workspace (a 500 MB `architecture.json`, or 10k snapshot files)
  forces an unbounded read into process memory — reachable over the
  `anvil://baseline` / `anvil://drift` MCP resources as well as the CLI.
- **Expected Outcome:** `load_baseline` and `load_snapshot_file` use the existing
  `read_to_string_bounded` pattern (8–16 MiB cap), and `list_snapshot_files`
  caps the number of files scanned before sorting (e.g. 1000) and logs a warning
  when it truncates (no silent cap). Both the CLI (`anvil drift`, baseline
  loading) and
  the `anvil://baseline`/`anvil://drift` MCP resources benefit.
- **Files:** `crates/anvil-architecture/src/baseline.rs`,
  `crates/anvil-cli/src/commands/drift.rs`.
- **Validation:** unit tests asserting an over-cap baseline/snapshot file is
  rejected with a bounded error (not OOM) and a snapshot dir over the count cap
  is truncated with a logged warning; `cargo test -p eddacraft-anvil
  commands::drift` and `-p eddacraft-anvil-architecture` green.
- **Identified From:** RMCPF-020 council review (adversarial reviewer, MAJOR
  #3/#4) while porting the `anvil://` MCP resources (PR #2809). The readers are
  shared with the CLI and the unbounded read pre-dates the resources, so the fix
  belongs at the reader level rather than the MCP layer.
- **Confidence:** medium — the bounded-read primitive already exists
  (`read_to_string_bounded`); the count cap needs a sensible default and a
  truncation log.
- **Closeout (2026-06-20, PR #2815):** added `anvil_architecture::read_to_string_capped`
  (one helper reused across both crates, no new dep); `load_baseline`,
  `load_snapshot_file`, and `read_created_at` cap at 16 MiB; `list_snapshot_files`
  caps the scan at 1000 files keeping the **most recent by mtime** before the
  `created_at` sort (council fix — the first cut truncated in arbitrary `read_dir`
  order and could drop the newest); `anvil://drift` reports the true total via a
  new `count_snapshot_files`. Council (kernel + adversarial) one MAJOR fixed
  in-PR. Pre-existing unbounded reads in `architecture.rs::parse_architecture`
  (CLI-only) and `collect_sql_findings` are out of scope and noted for future
  intake.

### CIB-085: Clawpatch Rust contract batch (gctx, policy-engine, sarif)

- **Status:** Merged 2026-06-20 via PR #2823
- **Summary:** Closed four medium-severity Rust contract findings from the
  2026-06-20 clawpatch triage: gctx impact cap is now identity-order independent;
  `find_dependents` surfaces walk budget truncation via `partial`; empty
  `ImpactQuery` deserialises to structured `InvalidQuery`; SARIF regions reject
  zero line/column via `NonZeroU32`. Policy-engine multi-expression queries are
  rejected with a contract error; multi-row comprehensions keep the first binding
  for `EvalResult.value` (full set in trace) — clawpatch finding
  `fnd_sig-feat-library-cf04c15e28-21c6_b62c4d5a74` triaged `wont-fix` with that
  documented contract.
- **Intent:** Close the five medium-severity contract/correctness findings left
  on the shipping Rust product after the 2026-06-20 clawpatch triage and JS/TS
  retirement verdict — deterministic gctx impact reporting, structured invalid
  query handling, multi-query Rego eval, and SARIF region invariants.
- **Expected Outcome:**
  1. **`anvil-gctx-egress`** — affected-symbol cap is input-order independent
     (stable sort or canonical ordering before truncation); when
     `find_dependents` hits the node cap it surfaces truncation explicitly
     instead of silently omitting dependents.
  2. **`anvil-gctx-types`** — deserialising an empty `ImpactQuery` routes to
     the structured `InvalidQuery` path, not an opaque serde failure.
  3. **`anvil-policy-engine`** — `Eval` processes every Rego query expression
     (or returns a contract error if multi-query input is unsupported —
     document the chosen behaviour).
  4. **`anvil-sarif`** — region builders reject or normalise zero line/column
     values so emitted SARIF cannot violate the schema.
  5. **Clawpatch closeout** — revalidate the five finding IDs below; mark
     `fixed` or `wont-fix` with note when merged.
- **Out of scope (separate intake):** the 56 open `crates/` `test-gap` tail
  (future hardening batch); CLAWP-005 / `midedit_contract.rs` (#1737); napi
  `.node` freshness residuals (verify against CLAWP-019 / PR #2181 before any
  fix); `start --verify` auth bypass (`fnd_sig-feat-cli-command-ba5ccdd3a6-_8f848cf67e`)
  — CIB-049 merged via PR #2474; revalidate and close the clawpatch finding if
  still open.
- **Clawpatch finding IDs:**
  - `fnd_sig-feat-library-ab71aa0329-31f3_a7852997e3` — gctx-egress cap ordering
  - `fnd_sig-feat-library-ab71aa0329-323b_24572b45a5` — gctx-egress silent truncation
  - `fnd_sig-feat-library-d12dc3b89f-d086_132cc413ae` — gctx-types empty query
  - `fnd_sig-feat-library-cf04c15e28-21c6_b62c4d5a74` — policy-engine multi-query
  - `fnd_sig-feat-library-9fdc01f86c-c265_1f2e438476` — sarif zero line/column
- **Files:** `crates/anvil-gctx-egress/src/lib.rs`,
  `crates/anvil-gctx-types/src/lib.rs`, `crates/anvil-policy-engine/src/lib.rs`,
  `crates/anvil-sarif/src/lib.rs`; focused unit/integration tests per crate.
- **Validation:** `cargo test -p eddacraft-anvil-gctx-egress -p eddacraft-anvil-gctx-types -p eddacraft-anvil-policy-engine -p eddacraft-anvil-sarif`;
  each fix has a failing test proven against current `main` before implementation.
- **Identified From:** `plans/reviews/2026-06-20-clawpatch-triage.md` §A (Rust
  product-actionable); audit `plans/audits/2026-06-20-clawpatch-periodic-scan.json`.
- **Coordinates with:** `feat/gctx-020` and other in-flight gctx work — reconcile
  file overlap before implementation; do not race duplicate fixes.
- **Confidence:** medium — root causes are localised; gctx-egress cap/truncation
  behaviour needs an explicit product call on truncation signalling.

### CIB-086: False-positive report off-machine egress bridge

- **Status:** Proposed
- **Intent:** Build the deferred, **opt-in** path for transmitting locally
  recorded false-positive reports off the machine, so Anvil can learn from
  aggregate FP signal (the second half of OPSUP-007's stated purpose).
- **Expected Outcome:** A new ADR decides the egress mechanism and destination
  (e.g. an `anvil report-fp export --to <dest>` bridge, or a managed upload)
  under an explicit opt-in contract that preserves the air-gap guarantee —
  egress only on deliberate operator action, never default-on. Reads the
  already-anonymised local `false-positives.ndjson` record (hashed paths, no
  source by default), so redaction is fixed at record time, not re-derived at
  the egress boundary.
- **Out of scope:** changing the local-record format or the anonymisation
  policy (fixed by ADR-089); analytics over the collected data.
- **Files:** new ADR under `plans/decisions/`; `crates/anvil-cli/src/commands/`
  (egress subcommand); the egress destination/transport.
- **Validation:** egress is off by default and requires explicit opt-in; the
  air-gap test harness still passes for `report-fp` and the default path.
- **Identified From:** OPSUP milestone Council review
  (`plans/reviews/2026-06-21-opsup-council.md`, pragmatic-lead MAJOR); deferred
  by ADR-089.
- **Confidence:** medium — destination/transport is a product + privacy
  decision that the ADR must settle first.

### CIB-087: False-positive report local read path

- **Status:** Done 2026-06-26 — `anvil report-fp --list` lists local reports with check ID, hashed path, line, and timestamp in plain or JSON output without plaintext paths or snippets.
- **Intent:** Give operators a way to see what `anvil report-fp` has recorded
  locally — today the `false-positives.ndjson` sidecar is write-only with no
  CLI read surface.
- **Expected Outcome:** A read command (e.g. `anvil report-fp --list` or a
  Kindling query view) lists the local FP reports (check ID, hashed path, line,
  timestamp; never the plaintext path), with `--json`. Useful independently of
  CIB-086 egress: it lets a user verify their reports and assemble a support
  bundle.
- **Files:** `crates/anvil-cli/src/commands/report_fp.rs` (or a Kindling read
  view); reuses the existing sidecar.
- **Validation:** new test: the read path lists recorded reports and never
  surfaces a plaintext path; empty/absent sidecar is a clean "none".
- **Identified From:** OPSUP milestone Council review (operations MAJOR).
- **Confidence:** high — small local read over an existing NDJSON sidecar.

### CIB-088: `anvil drift migrate` operability — backup retention + partial-failure reporting

- **Status:** Done 2026-06-26 — `drift migrate` now reports skipped baselines by reason, exits non-zero for partial runs after emitting the report, and supports explicit `--prune-backups` count retention that keeps the latest rollback per live snapshot.
- **Intent:** Close two operability gaps in `drift migrate`: (a) `.bak`/`.bak.N`
  backups accumulate unbounded ("retained for one release" is comment-only, not
  enforced); (b) skipped corrupt/unreadable/future-version baselines are not
  reported in the count or exit code, so CI can't detect a partial migration.
- **Expected Outcome:**
  1. A prune path for stale backups (e.g. `anvil drift migrate --prune-backups`
     or an age/count bound) so backups can't grow without limit across releases;
     the "one release" retention is enforced or the limitation is documented.
  2. `MigrateReport` carries a `skipped` count (by reason); `run_migrate`
     surfaces it in both plain and `--json` output and returns a non-zero exit
     when any baseline was skipped, so a CI step can tell clean from partial.
- **Files:** `crates/anvil-cli/src/commands/drift.rs` (`MigrateReport`,
  `migrate_snapshots`, `run_migrate`, backup helpers).
- **Validation:** new tests: a skipped corrupt baseline is counted and changes
  the exit code; a prune run removes only eligible backups and never the live
  baseline.
- **Identified From:** OPSUP milestone Council review (operations MAJOR×2;
  adversarial flagged the disk-full `.bak.N` chain).
- **Confidence:** high — localised to the migrate command.

### CIB-089: Reconcile unknown-check-ID resolution semantics across surfaces

- **Status:** Done 2026-06-26 — `.anvilrc#checks` warn-and-continue is formalised for configs with a known subset; explicit `--only-checks` / `--skip-checks` remain fatal on unknown IDs.
- **Intent:** `--skip-checks` / `--only-checks` reject an unknown check ID with
  a fatal error, while `.anvilrc#checks` warns-and-continues with the known
  subset. The same class of user error (a typo'd check ID) has divergent
  outcomes; OPSUP-002's "deterministic error" intent doesn't distinguish the
  config-file path.
- **Expected Outcome:** A deliberate, documented decision: either make
  `.anvilrc#checks` fatal on unknown IDs (matching the CLI flags), or formalise
  the warn-and-continue posture for the config file in the spec and code
  comment. Both paths already emit the OPSUP-002 did-you-mean suggestion; the
  decision is about fatal-vs-warn, tested either way.
- **Files:** `crates/anvil-cli/src/commands/gate.rs`
  (`resolve_anvilrc_check_filter` vs `validate_check_names`); the OPSUP-002 spec
  note.
- **Validation:** a test pins the chosen behaviour (and the suggestion text) for
  each surface.
- **Identified From:** OPSUP milestone Council review (general-quality MAJOR).
- **Confidence:** high — small; mostly a product call on fatal-vs-warn.

### CIB-090: `O_NOFOLLOW` hardening for the Kindling NDJSON sidecar writes

- **Status:** Done 2026-06-26 — Unix sidecar writes are parent-dirfd anchored with leaf `O_NOFOLLOW`; trim reads are no-follow; trim rewrites use unique create-new temp files; existing parents are tightened to `0700`. Windows reparse-point parity is tracked separately in CIB-108.
- **Intent:** Close the TOCTOU symlink race in `append_observation_to`: it does
  `symlink_metadata` then a separate `open`, with no `O_NOFOLLOW` between them,
  so a writer to the parent dir on a multi-user host could redirect the append.
  Affects both `usage.ndjson` and the OPSUP-007 `false-positives.ndjson` (shared
  helper), plus the trim temp file and `write_private_file`.
- **Expected Outcome:** The Unix append/write opens use parent-directory fd
  anchoring plus leaf `O_NOFOLLOW` so the open atomically refuses a symlinked
  target, and trim reads use the same no-follow discipline before retention
  housekeeping. Trim rewrites use unique `create_new` temp files rather than the
  deterministic `.trim.tmp` path. The `0600`/`0700` posture is unchanged or
  tightened. Platform-equivalent Windows reparse-point handling is deliberately
  not claimed here and is tracked by CIB-108.
- **Files:** `crates/anvil-cli/src/usage.rs` (`append_observation_to`,
  `trim` temp path, sidecar parent/open helpers), `crates/anvil-cli/Cargo.toml`
  (Unix `nix` safe syscall wrappers).
- **Validation:** `cargo test -p eddacraft-anvil usage::tests::`; integration
  checks for usage observation/views and `report-fp`; `cargo clippy -p
  eddacraft-anvil --all-targets -- -D warnings`.
- **Identified From:** OPSUP milestone Council review (adversarial MAJOR;
  security-adjacent). Pre-existing in the shared usage path, surfaced by the FP
  reuse.
- **Confidence:** high — focused Unix hardening on the existing sidecar paths;
  Windows parity is a separate platform-specific item.

### CIB-091: GCTX assistant-facing egress hardening (v0.9.0 council, cut-blocker)

- **Status:** Merged 2026-06-22 via PR #2852
- **Intent:** Close the GCTX egress findings that survived the 2026-06-21
  v0.9.0-beta release council's skeptic. Headline is **CE-3**: the
  sensitive-path egress deny-list is entirely unimplemented, so on a fresh
  install the workspace-relative paths of secret/credential files (`.env*`,
  `*.pem`, `secrets/`, `id_rsa`, `.aws/`, `.ssh/`) reach the connected assistant
  as identity (the substrate scans with `standard_filters(false)`). This is a
  PV-9 APPROVE-WITH-CONDITIONS condition and the v0.9.0 cut-blocker.
- **Expected Outcome:** `is_sensitive_egress_path` applied across all six
  `collect_*` projection paths, matches dropped before the DTO is sealed,
  counted in `RedactionSummary.omitted_sensitive_paths`, with a structural
  never-appears test (091a). Plus: `workspace_root` 512B/NUL validation →
  `InvalidQuery` (091b); `collect_impact_with_budget` sort moved off the cache
  Mutex (091c); a per-MCP-session byte ceiling for `graph://` reads, or a
  tracked Phase-1 limitation (091d).
- **Files:** `crates/anvil-gctx-egress/src/lib.rs`,
  `crates/anvil-intercept/src/save_time.rs`,
  `crates/anvil-cli/src/mcp/resources/mod.rs`.
- **Validation:** structural test that sensitive paths never appear in any
  projection; `InvalidQuery` classification test; impact-path latency unchanged.
- **Identified From:** v0.9.0-beta release council (CE-3 verifier-confirmed high;
  details in `plans/audits/2026-06-21-v090-council-survivors.md`).
- **Confidence:** high — CE-3 is well-scoped; 091d may be deferred with a note.

### CIB-092: Persistence / warm-start wire-integrity & observability (v0.9.0 council)

- **Status:** Merged 2026-06-22 via PR #2852
- **Intent:** Close the warm-start persistence findings (default-off,
  `ANVIL_PERSIST_GRAPH`) from the v0.9.0 council. Headline highs: the ADR-069 §6
  golden wire-bytes fixture is missing (writer+reader drift together, so a
  postcard/codec change silently loads a valid-but-wrong warm graph with no CI
  signal), and the §10 metric counters are absent (the §7 graduation gate is
  unverifiable — graduation-blocker for the default-on flip, not the cut).
- **Expected Outcome:** golden `&[u8]` fixture asserts `to_bytes()` of the
  standard fixture, forcing a `SNAPSHOT_BACKING_SCHEMA_VERSION` bump on drift
  (092a); `snapshot_load_result`/`snapshot_write_result` counters via the
  `TelemetryEmitter` fanout (092b); orphan `.snap` startup sweep (092c); ADR-069
  §4 openat2 discipline on snapshot I/O (092d); drop the undeclared `sha2` dep
  for a non-crypto hash (092e); §3 verdict-gate end-to-end test (092f);
  fsync_dir-after-rename semi-success (092g); ADR-035 notification on persist
  write failure (092h).
- **Files:** `crates/anvil-graph-cache/src/{snapshot.rs,snapshot_io.rs}`,
  `crates/anvil-graph-cache/Cargo.toml`,
  `crates/anvil-intercept/src/{save_time.rs,full_scan_executor.rs}`.
- **Validation:** committed wire-bytes fixture + drift test; counter-emission
  test; orphan-sweep test; verdict-gate restore-window test.
- **Identified From:** v0.9.0-beta release council (092a/092b verifier-confirmed
  high; details in `plans/audits/2026-06-21-v090-council-survivors.md`).
- **Confidence:** high for 092a/092b/092c/092e; medium for the rest.

### CIB-093: GV2 substrate hot-path & trust correctness (v0.9.0 council)

- **Status:** Merged 2026-06-22 via PR #2852
- **Intent:** Close the GV2 graph-substrate mediums from the v0.9.0 council. The
  load-bearing one is the privilege gate: `PRIVILEGED_MODULES` omits
  spawn/sandbox-escape Node built-ins (`worker_threads`, `vm`, `v8`, `dns`,
  `tls`, `dgram`), so a newly imported/re-exported privileged capability
  certifies CLEAN. The rest are hot-path constant-factor regressions under the
  cache Mutex and a snapshot-version aliasing bug.
- **Expected Outcome:** `PRIVILEGED_MODULES` extended (093a); reexport-privilege
  read off `trust_level` / memoised rather than re-walked per file (093b);
  `known_files` built via O(files) `file_names()` not O(symbols)
  `node_weights()` (093c); a latency span + WARN threshold on `annotate_trust`
  (093d); an independent `SNAPSHOT_BACKING_SCHEMA_VERSION` const (093e).
- **Files:** `crates/anvil-graph-cache/src/{trust.rs,certify.rs,incremental.rs,snapshot.rs}`,
  `crates/anvil-intercept/src/kernel_cache.rs`.
- **Validation:** trust-gate test covering the new privileged modules;
  micro-benchmark/parity unchanged; snapshot-version independence test.
- **Identified From:** v0.9.0-beta release council (all medium; details in
  `plans/audits/2026-06-21-v090-council-survivors.md`).
- **Confidence:** high — 093a is a product call + list edit; the rest are
  localised hot-path swaps.

### CIB-094: USAGE producer controls & robustness (v0.9.0 council)

- **Status:** Merged 2026-06-22 via PR #2852
- **Intent:** Close the USAGE-analytics mediums from the v0.9.0 council. The CLI
  `command.invoked` producer has no operator kill-switch (asymmetric with the
  daemon DPO opt-out), a non-UTF-8 byte permanently defeats retention trimming,
  the conformance test overstates its coverage, a daemon-down unblock goes
  unrecorded, and the operator-control docs are stale.
- **Expected Outcome:** `ANVIL_USAGE_DISABLE` kill-switch consulted by
  `record_invocation` (094a); `trim_usage_sidecar_at` switched to a
  line-skipping reader + non-UTF-8 test (094b); conformance test iterates the
  registered command list or the claim is corrected (094c); CLI row emitted on
  daemon-down unblock (094d); retention + env-knob docs refreshed (094e).
- **Files:** `crates/anvil-cli/src/{usage.rs,usage_views.rs,main.rs,commands/intercept.rs}`,
  `docs/observability/usage-analytics.md`.
- **Validation:** kill-switch test; non-UTF-8 mid-file trim test; full-coverage
  conformance iteration; daemon-down row test.
- **Identified From:** v0.9.0-beta release council (all medium; details in
  `plans/audits/2026-06-21-v090-council-survivors.md`).
- **Confidence:** high — small, well-scoped CLI changes.

### CIB-095: Intercept hot-path follow-through (v0.9.0 council)

- **Status:** Merged 2026-06-22 via PR #2852
- **Intent:** Close the intercept/save-time follow-through mediums from the
  v0.9.0 council (surface gate was PASS — no blockers). `search_symbols` omits
  the UNC-path filter its sibling verbs enforce; the restore→reconcile window
  needs a certify-verdict guard; the new implicit background scan needs a scoped
  opt-out + doc; the listener-failure exit loses warm graphs; a per-job watchdog
  thread and a shutdown write-failure counter are loose ends.
- **Expected Outcome:** `search_symbols` routed through
  `invalid_relative_path_reason` (095a); confirm + guard non-certifiable
  restore window (095b); `ANVIL_WATCH_DAEMON_SCAN=0` scoped opt-out + release
  note (095c); `persist_all_on_shutdown()` on the listener-failure exit (095d);
  watchdog via the existing cancel channel (095e); `dropped_snapshot_writes`
  observation pairing with CIB-092b (095f).
- **Files:** `crates/anvil-intercept/src/{save_time.rs,kernel_cache.rs,lib.rs,full_scan_executor.rs}`.
- **Validation:** UNC-rejection test for `search_symbols`; restore-window
  verdict test; listener-failure persist test.
- **Identified From:** v0.9.0-beta release council (all medium/low; details in
  `plans/audits/2026-06-21-v090-council-survivors.md`).
- **Confidence:** high — localised, each with a clear test.

### CIB-096: Wire the orphan `.snap` startup sweep into the daemon (092c follow-up)

- **Status:** Merged 2026-06-22 via PR #2870
- **Intent:** the orphan-`.snap` sweep was provided in CIB-092c but **not called
  anywhere in the daemon**, so snapshots for worktrees deleted while the daemon
  was down accumulate in `graph-cache/` indefinitely. The keep-set sweep had no
  safe cold-boot call site: snapshot filenames are FNV hashes of the canonical
  root, so an empty-at-boot registry can't tell a true orphan from a
  not-yet-reattached warm snapshot. **Resolved (owner-approved design) by storing
  the root, so no keep-set is needed.**
- **Expected Outcome:** (implemented) each snapshot gets a sibling `<hash>.root`
  companion (0600, written via the CIB-097 dirfd discipline) holding the canonical
  root. `sweep_orphan_snapshots_on_start(dir)` reads each `.root` and reclaims a
  snapshot **only** when its root is **proven gone** (`symlink_metadata` →
  `NotFound`); it KEEPS (fail-safe) on an absent/unreadable/oversized/symlinked
  companion **or** any non-`NotFound` stat error (EACCES/EIO/transient mount). The
  companion read is dirfd-anchored (`O_NOFOLLOW`) + `MAX_ROOT_BYTES`-capped; the
  unlinks are anchored basenames. No keep-set ⇒ **safe at cold boot** — wired at
  `run_foreground` startup beside the temp sweep. The old keep-set
  `sweep_stale_snapshots_on_start` + its `SaveTimeState` wrapper are removed.
- **Files:** `crates/anvil-intercept/src/snapshot_io.rs` (companion write/read +
  `sweep_orphan_snapshots_on_start`), `crates/anvil-intercept/src/save_time.rs`
  (parameterless wrapper), `crates/anvil-intercept/src/lib.rs` (`run_foreground`
  wiring beside `sweep_snapshot_temps_on_start`).
- **Validation:** reclaims a `.snap`+`.root` whose root is gone; keeps it when the
  root exists, the companion is missing/unreadable/oversized/symlinked, or the
  stat is EACCES (not `NotFound`); cleans a stray `.root`.
- **Identified From:** CIB-092 v0.9.0 council survivor follow-up — deferred at
  merge (PR #2852) for lack of a faithful registered-set source; tracked here per
  `plans/reviews/post-merge/fix-v090-council-survivors.md`.
- **Confidence:** high — implemented via the companion root-file + existence
  check (no warm-set/registry dependency); council-reviewed.

### CIB-097: Anchor the snapshot WRITE path to a validated directory fd (092d follow-up)

- **Status:** Merged 2026-06-22 via PR #2865
- **Intent:** CIB-092d (PR #2852) anchored the snapshot **read** path to a
  validated `O_PATH` dirfd via `open_workspace_dirfd` + `openat2`
  (`RESOLVE_NO_SYMLINKS|RESOLVE_BENEATH`) with an `O_NOFOLLOW` fallback, but the
  **write** path (temp create + `rename`) is still path-based. ADR-069 §4 mandates
  the same dirfd discipline on both sides. The residual gap is dir-component-swap
  atomicity (the leaf is already `O_EXCL|O_NOFOLLOW` and `validate_secure_dir`
  blocks a symlinked/non-owned dir before any write), so this is hardening, not a
  live hole. (The write anchor must be a **real `O_DIRECTORY` fd**, not `O_PATH` —
  an `O_PATH` fd cannot be `fsync`'d, and the publish must `fsync` the directory.)
- **Expected Outcome:** the temp create + `renameat` + a dirfd `fsync` are anchored
  to one validated **`O_DIRECTORY|O_RDONLY|O_NOFOLLOW`** fd on the state dir (a
  real, fsync-able fd that also serves as the `openat`/`renameat`/`unlinkat`
  anchor; the read path keeps `O_PATH` for its `openat2` anchor), closing the
  dir-component-swap window. The fd is `fstat`-validated (owner-only, owned by us)
  after open to close the validate→open TOCTOU. The shipped `path_safety` helpers
  cover only the read side, so a new `openat`/`renameat` ladder is required.
- **Files:** `crates/anvil-intercept/src/snapshot_io.rs` (`write_snapshot`, around
  the deferred `CIB-092d` create/rename note), reusing/extending
  `crates/anvil-intercept/src/path_safety.rs`.
- **Validation:** a test that the write refuses a symlinked intermediate directory
  component (parity with the read path's symlinked-leaf test).
- **Identified From:** CIB-092 v0.9.0 council survivor follow-up — write side
  deferred at merge (PR #2852) as a larger change than the read fix; tracked here
  per `plans/reviews/post-merge/fix-v090-council-survivors.md`.
- **Confidence:** medium — Linux-specific `openat`/`renameat` ladder; needs cross-
  platform `#[cfg]` gating consistent with the read path.

### CIB-098: Deliver the persist-failure degradation signal to opted-in operators (092h follow-up)

- **Status:** Merged 2026-06-22 via PR #2887
- **Intent:** CIB-092h (PR #2852) builds an ADR-035 persist-failure
  `NotificationEnvelope` on a snapshot write failure when `persistence_enabled()`,
  but the INTD-015 fanout **hard-denies any envelope without an
  `originating_session_id`**, and daemon-internal writes (shutdown flush /
  background scan) carry none — so the envelope is never delivered to any
  operator. Today the only real signal is the `tracing::warn!` per failure plus the
  cumulative `SnapshotMetrics` shutdown log. An operator who set
  `ANVIL_PERSIST_GRAPH=1` and hits a full/EROFS state dir gets no user-visible
  degradation notification.
- **Expected Outcome:** a legitimate delivery path for daemon-originated health
  envelopes (a daemon-local health sink, or a sanctioned session-less delivery
  lane that respects the INTD-015 boundary) so an opted-in operator actually sees
  the persist-failure degradation — without weakening the cross-session deny
  invariant. Decide deliberately whether this needs an ADR (INTD-015 amendment).
- **Files:** `crates/anvil-intercept/src/save_time.rs` (`notify_persist_write_failure`),
  `crates/anvil-intercept/src/fanout.rs` (the `decide` session-deny),
  `crates/anvil-intercept/src/telemetry.rs`.
- **Validation:** a test that an enabled-persistence write failure produces a
  *delivered* notification (not merely a built envelope) via the new lane.
- **Identified From:** CIB-092 v0.9.0 council survivor follow-up — delivery
  deferred at merge (PR #2852); honest WARN+metrics signal shipped meanwhile;
  tracked here per `plans/reviews/post-merge/fix-v090-council-survivors.md`.
- **Confidence:** medium — may need an INTD-015/ADR-035 decision on session-less
  health delivery before implementation.

### CIB-099: GCTX cross-surface hardening (GCTX-010..014 council follow-ups)

- **Status:** Done 2026-06-24 — shared GCTX daemon client extracted for tools/resources; non-Linux/macOS Unix peer-validation failures now degrade to `Unavailable` instead of hard `Failure`.
- **Intent:** Close the council follow-ups shared across the merged GCTX Phase-1
  tool surface (GCTX-010..014) that were deliberately deferred at merge — not
  regressions, but real hardening debt before wider assistant-facing rollout.
- **Expected Outcome:** (a) extract the duplicated `GctxRpcError`/socket-client
  between `search_symbols.rs` and `find_dependents.rs` (and siblings) into a
  shared GCTX client module; (b) on non-Linux/macOS Unix peer-validation
  failures, return `Unavailable` rather than `Failure` for consistency with the
  sibling tools. N5 (`SaveTimeError::Io` wire leak) is **closed** in PR #2852 —
  do not reopen here.
- **Split:** the cursor-fingerprint hardening (HMAC or equivalent) is broken out
  to **CIB-103** — it is a product/security call that gates implementation,
  unlike the mechanical shared-client and peer-validation work here.
- **Files:** `crates/anvil-cli/src/mcp/gctx_client.rs`,
  `crates/anvil-cli/src/mcp/tools/*.rs`, `crates/anvil-cli/src/mcp/resources/mod.rs`
  (shared client extraction and peer-validation classification only).
- **Validation:** `cargo test -p eddacraft-anvil mcp::`;
  `cargo clippy -p eddacraft-anvil --all-targets -- -D warnings`.
- **Identified From:** GCTX-011 council review; tracked in
  `plans/reviews/post-merge/feat-gctx-011-find-dependents.md` until filed here
  (2026-06-22 release-window hygiene).
- **Coordinates with:** CIB-103 (split-out cursor-fingerprint decision; rides the
  same shared GCTX client module).
- **Confidence:** high — shared-client extraction is mechanical and the
  peer-validation classification is localised; the gated cursor-fingerprint
  decision is split to CIB-103.

### CIB-100: Windows named-pipe GCTX client transport

- **Status:** In Progress 2026-06-24 — Windows named-pipe client path implemented in the shared GCTX client; remains open pending a Windows toolchain/matrix check.
- **Intent:** The GCTX Phase-1 tools (`anvil_search_symbols`,
  `anvil_find_dependents`, `anvil_impact_of_change`, `anvil_affected_tests`,
  `anvil_find_callers`) degrade to `unavailable` on non-Unix because the GCTX
  client uses a Unix-domain socket only. Windows operators running
  `anvil mcp serve` against a local daemon cannot use graph-context tools.
- **Expected Outcome:** a Windows named-pipe GCTX client transport mirroring the
  existing save-time pipe client, wired through the shared GCTX client module
  (coordinate with CIB-099), so the five tools return real outcomes on Windows when
  the daemon is running.
- **Files:** `crates/anvil-cli/src/mcp/gctx_client.rs`,
  `crates/anvil-cli/src/mcp/tools/*.rs`, `crates/anvil-cli/src/mcp/resources/mod.rs`.
- **Validation:** non-Windows validation: `cargo test -p eddacraft-anvil mcp::`;
  `cargo clippy -p eddacraft-anvil --all-targets -- -D warnings`. Windows
  matrix still required: local `cargo check -p eddacraft-anvil --target
  x86_64-pc-windows-gnu` and `--target x86_64-pc-windows-msvc` attempts were
  blocked by missing Windows C toolchain components (`x86_64-w64-mingw32-gcc` /
  `lib.exe`).
- **Identified From:** GCTX-014 post-merge note; DSV Windows parity split
  (DSV-010/011). Filed 2026-06-22 release-window hygiene.
- **Confidence:** medium — transport exists for save-time; GCTX projection path is
  new wiring.

### CIB-101: `anvil uninstall --global` cleans the active `ANVIL_HOME` user root

- **Status:** Done 2026-06-27 — `anvil uninstall --global` now cleans `<ANVIL_HOME>/user/` under an install-root override, preserves production user state, reports the scoped path in dry-run JSON, and uses the safer intercept stop path.
- **Intent:** Close the local DISTRIB-006 uninstall gap: when the active install
  root is re-rooted by `ANVIL_HOME`, global uninstall must clean the candidate's
  install-owned user state instead of production's user state.
- **Expected Outcome:** `anvil uninstall --global` removes `<ANVIL_HOME>/user/`
  when an install-root override is active, preserves production `~/.anvil/` and
  default credentials under that override, reports the same path in dry-run JSON,
  keeps default no-`ANVIL_HOME` behaviour unchanged, and refuses symlinked
  cleanup targets. Windows named-pipe side-by-side endpoint re-rooting is split
  to CIB-106.
- **Files:** `crates/anvil-cli/src/install_root.rs`,
  `crates/anvil-cli/src/commands/uninstall.rs`,
  `crates/anvil-cli/tests/anvil_home.rs`.
- **Validation:** uninstall unit tests; `anvil_home` integration tests proving
  `<ANVIL_HOME>/user/` cleanup and production home preservation; docs/APS checks.
- **Identified From:** `plans/reviews/post-merge/feat-distrib-006-anvil-home-override.md`;
  DISTRIB module is Complete — intake via CIB. Filed 2026-06-22 release-window
  hygiene; split after CIB-101 mini Council on 2026-06-27.
- **Coordinates with:** CIB-106 (Windows `ANVIL_HOME` named-pipe endpoint re-root).
- **Confidence:** high — localised CLI planner/executor change with existing
  install-root primitives.


### CIB-106: Windows `ANVIL_HOME` named-pipe endpoint re-root

- **Status:** Merged 2026-07-02 via PR #3083
- **Intent:** Complete the Windows half of DISTRIB-006 side-by-side daemon
  coexistence by deriving the intercept named-pipe endpoint from the active
  install root when `ANVIL_HOME` is set.
- **Expected Outcome:** Windows keeps the legacy `\\.\pipe\anvil-intercept-<sid>`
  pipe name when `ANVIL_HOME` is unset/blank, and adds a stable bounded hashed
  install-root namespace when `ANVIL_HOME` is active so two same-user candidate
  daemons can coexist. All daemon/client surfaces use the canonical resolver
  (`ensure`, `intercept status`, MCP protection claim, watch save-time transport,
  GCTX), while owner-only DACL / local-only / SQOS behaviour remains unchanged.
- **Files:** `crates/anvil-intercept-win32/src/lib.rs`,
  `crates/anvil-intercept/src/ipc.rs`, `crates/anvil-intercept/src/ensure.rs`,
  `crates/anvil-cli/src/commands/intercept.rs`,
  `crates/anvil-cli/src/mcp/validation.rs`,
  `crates/anvil-cli/src/commands/watch_save_time.rs`, optional GCTX client tests.
- **Validation:** pure resolver tests for unset/blank/default compatibility,
  absolutised `ANVIL_HOME`, stable hash, distinct prefixes, no raw path leakage;
  Windows matrix tests/checks proving daemon bind and clients use the same
  resolver.
- **Identified From:** CIB-101 mini Council (operations, security/adversarial,
  implementation maintainer) on 2026-06-27; split from CIB-101 because it overlaps
  CIB-100 and requires Windows validation.
- **Coordinates with:** CIB-100, CIB-101, CIB-105.
- **Confidence:** medium — behaviour is clear, but Windows matrix coverage is
  required before shipping.

### CIB-102: Anchor the snapshot delete/sweep paths to a validated dirfd (CIB-097 follow-up)

- **Status:** Merged 2026-06-22 via PR #2884
- **Intent:** CIB-097 (PR #2865) anchored the snapshot **write** path to a
  validated directory fd, but the **delete** paths are still path-based:
  `remove_snapshot` (`fs::remove_file(&path)`), `sweep_orphan_temps`, and
  `sweep_stale_snapshots_on_start` (`fs::read_dir(dir)` + `fs::remove_file(&path)`)
  in `crates/anvil-intercept/src/snapshot_io.rs`. A same-uid attacker who swaps
  the `dir` (or plants a symlink at a `.snap`/`.tmp` path) between the `read_dir`
  and the `unlink` can redirect the delete — a data-destruction vector (the daemon
  deletes a file outside the cache), distinct from the write-through gap CIB-097
  closed. Flagged by the CIB-097 council as out of scope for that PR.
- **Expected Outcome:** the delete/sweep paths open the state dir once via
  `open_workspace_dir_for_fsync` (the validated, fstat-checked real dirfd from
  CIB-097) and use `unlinkat` relative to that fd (and `readdir` anchored to it
  where the sweep enumerates), so a swapped component cannot redirect the unlink.
  Mirrors the write path's dirfd discipline.
- **Files:** `crates/anvil-intercept/src/snapshot_io.rs` (`remove_snapshot`,
  `sweep_orphan_temps`, `sweep_stale_snapshots_on_start`), reusing
  `crate::path_safety::open_workspace_dir_for_fsync`.
- **Validation:** a test that a symlink planted at a `.snap` path is not followed
  by the sweep/remove (the symlink target is left intact; only the entry under the
  cache dir is removed).
- **Identified From:** CIB-097 council review (adversarial, low/out-of-scope) —
  the delete-path counterpart to the write-path anchoring.
- **Confidence:** high — the validated dirfd + `unlinkat` primitives already exist
  from CIB-097; this extends them to the delete paths.

### CIB-103: GCTX cursor fingerprint hardening (HMAC) — decision

- **Status:** Merged 2026-06-24 via PR #2903
- **Decision (2026-06-24, [ADR-091](../decisions/091-gctx-cursor-fingerprint-integrity.md), Accepted):**
  option **(b)** — keep the reproducible FNV-1a filter fingerprint; do **not** add
  an HMAC now. The opaque keyset cursor is a server-minted *seek position*, not an
  authorisation token: a forged cursor only reseeks within the caller's own
  already-authorised, re-fingerprinted query (identity-only via the CE-5 choke
  point, no source, bounded by `MAX_CURSOR_BYTES`), so HMAC would add per-daemon
  key material + a wire change to defend a non-threat. **Execution (delivered):** a
  crate-level threat-model note + `CursorPayload` pointer in `anvil-gctx-egress`,
  the binding revisit trigger (flip to a keyed MAC the moment a cursor encodes
  snippet/source, cross-tenant/trust-scope, or non-re-authorised results —
  Phase-2 CE-1), and the `forged_cursor_stays_within_the_querys_own_authorised_results`
  pinning test (a forged cursor reseeks only within the query's own authorised set,
  never leaks/panics). **ADR-091 Accepted 2026-06-24** (owner; council
  `council-f8ed314e` ratified). Item completes on PR #2903 merge.
- **Intent:** Decide whether to replace the current FNV cursor fingerprint on the
  GCTX identity-paging cursor with a keyed construction (HMAC or equivalent). The
  FNV fingerprint reseeks identity-only pages — no data leak today, but it is
  forgeable, so a client could craft a cursor that resumes from an arbitrary
  identity. Split from CIB-099 because, unlike the mechanical shared-client and
  peer-validation work there, this is a product/security call that must be settled
  before implementation.
- **Expected Outcome:** An explicit decision (recorded inline; an ADR if it
  changes the cursor wire contract): either (a) adopt a keyed fingerprint (HMAC
  over the `{filter-fingerprint, last_identity}` tuple with a per-daemon key) so a
  forged cursor is rejected at decode time, or (b) document that identity-only
  paging needs no integrity guarantee and keep FNV, with the threat model written
  down. If (a), the CE-6 keyset cursor (hex `{fnv1a-filter-fingerprint,
  last_identity}`) gains a MAC and verification rejects a tampered cursor; old
  cursors are invalidated cleanly, not mis-parsed.
- **Files:** `crates/anvil-gctx-egress/src/lib.rs` (cursor encode/decode +
  fingerprint); an ADR under `plans/decisions/` only if the wire contract changes.
- **Validation:** if (a), a test that a tampered/forged cursor is rejected and a
  round-tripped cursor verifies; if (b), the threat-model note lands and a test
  pins that identity-only paging tolerates an opaque cursor.
- **Identified From:** CIB-099 triage split (PR #2890 follow-up); GCTX-011 council
  review (cursor-fingerprint sub-decision).
- **Coordinates with:** CIB-099 (shared GCTX client — the cursor code rides the
  same module); CE-6 keyset cursor.
- **Confidence:** medium — the keyed-fingerprint mechanics are straightforward,
  but whether forgeable identity-only paging warrants an HMAC is a
  product/security call that gates the work.

### CIB-104: Forged-cursor pinning tests for the GCTX dependents/callers/edges surfaces

- **Status:** Merged 2026-06-25 via PR #2912
- **Intent:** Extend the ADR-091 cursor-integrity guarantee from the search
  surface to its three siblings. CIB-103 pinned the `search_symbols` cursor with
  `forged_cursor_cannot_seek_across_a_filter_boundary` (containment) and
  `cursor_payload_shape_is_pinned_to_seek_position_only` (the revisit-trigger
  shape guard), but `find_dependents`, `find_callers`, and `graph://edges` —
  which share the identical forgeable-by-design cursor construction — have no
  equivalent tests. The ADR-091 council (`council-f8ed314e`, finding C-004)
  verified the siblings are structurally safe today (the cursor is decoded only
  after the bounded walk materialises a sorted candidate set), but that safety is
  unpinned: a future change could regress one surface with no failing test.
- **Expected Outcome:** Each of the three sibling surfaces gains (a) a forged-
  cursor containment test — a client-minted cursor with an arbitrary `last` plus a
  recomputed matching fingerprint reseeks only within the query's own
  already-authorised, identity-only result set (empty page past the end,
  strictly-after mid-set, no leak/panic) — and (b) a payload-shape guard asserting
  the decoded cursor JSON carries exactly its expected keys, so adding a
  snippet/scope field to `DependentsCursorPayload` / `CallersCursorPayload` /
  `EdgesCursorPayload` breaks CI and forces an ADR-091 re-open, mirroring the
  search-surface guards.
- **Files:** `crates/anvil-gctx-egress/src/lib.rs` (the `mod tests` block; reuses
  the existing dependents/callers/edges graph fixtures).
- **Validation:** `cargo test -p eddacraft-anvil-gctx-egress` — the new
  per-surface forged-cursor + shape-guard tests pass; a deliberately added stray
  field on any sibling cursor payload fails the shape guard.
- **Identified From:** ADR-091 council review (`council-f8ed314e`, finding C-004,
  deferred); recorded in ADR-091 Mitigations.
- **Coordinates with:** CIB-103 (search-surface precedent), ADR-091.
- **Confidence:** high — the search-surface tests are a direct template, the
  sibling fixtures already exist, and the change is purely additive test coverage.

### CIB-105: Windows reparse-point hardening for Kindling sidecar writes

- **Status:** Proposed
- **Intent:** Provide the Windows platform equivalent to CIB-090's Unix
  `O_NOFOLLOW`/dirfd discipline for `usage.ndjson` and `false-positives.ndjson`
  sidecar writes, so a reparse point, junction, or symlink cannot redirect the
  local Kindling sidecar on Windows hosts.
- **Expected Outcome:** Windows sidecar append/read/temp-write paths refuse
  reparse-point leaves and redirected parent components using Windows-specific
  safe file APIs or a helper crate, preserving the existing local-only privacy
  posture and `ANVIL_HOME` isolation. The Unix CIB-090 implementation remains
  unchanged.
- **Files:** `crates/anvil-cli/src/usage.rs`; optional Windows helper crate if
  direct Win32 calls would otherwise require `unsafe` in `anvil-cli`.
- **Validation:** Windows-only regression tests or CI matrix coverage proving
  sidecar leaf and parent reparse points are refused and an outside target is
  not modified; existing Unix CIB-090 tests still pass.
- **Identified From:** CIB-090 mini-Council security/adversarial review
  (2026-06-26) — Unix `O_NOFOLLOW` hardening must not overclaim platform
  equivalence.
- **Coordinates with:** CIB-090, CIB-100, CIB-101 Windows transport/install-root
  parity work.
- **Confidence:** medium — behaviour is clear, but the safe Windows API shape
  likely needs a helper to keep `anvil-cli` under `unsafe_code = "forbid"`.

### CIB-108: Restrict network-capable OPA built-ins during policy evaluation

- **Status:** Merged 2026-07-02 via PR #3084
- **Intent:** Prevent untrusted workspace policies from using OPA network-capable
  built-ins during `anvil policy eval` execution.
- **Expected Outcome:** `OPAExecutor` evaluates loaded workspace policies with a
  restricted OPA capabilities profile, or an equivalent fail-closed guard, that
  removes `http.send` and other runtime/network-sensitive built-ins. The eval path
  rejects or safely fails policies that require those built-ins instead of making
  outbound requests from developer or CI environments.
- **Files:** `packages/anvil/policy/src/opa-executor.ts`, policy eval fixtures and
  tests under `packages/anvil/policy/src/`.
- **Validation:** Targeted policy-package tests prove a policy using `http.send`
  cannot make an outbound request and receives a deterministic denied/unsupported
  result; existing `anvil policy eval --json` contract tests still pass.
- **Identified From:** Deepsec P0 true-positive triage, run
  `20260629190245-caf2a4b60b2715fe`; finding `ssrf` in
  `packages/anvil/policy/src/opa-executor.ts` (`Untrusted Rego policies run with
  unrestricted OPA built-ins`), revalidated true-positive.
- **Coordinates with:** EVAL-001..005 (`anvil policy eval --json` v1 consumers),
  CIB-078 (frozen eval output contract).
- **Confidence:** high — the vulnerable trust boundary is confirmed; the remaining
  choice is the exact OPA capabilities/sandbox mechanism.

### CIB-109: Bind bundle-auth environment credentials to trusted configuration

- **Status:** Merged 2026-07-02 via PR #3082
- **Intent:** Stop workspace-controlled bundle configuration from selecting and
  exfiltrating arbitrary process environment variables as bundle credentials.
- **Expected Outcome:** Bundle auth no longer accepts arbitrary `password_env` or
  `token_env` names from untrusted bundle config. Credential references are
  restricted to operator-owned names, an allowlisted prefix, or a trusted
  credential registry, and credentials are bound to the intended bundle host so an
  attacker-controlled HTTPS URL cannot receive unrelated CI secrets.
- **Files:** `packages/anvil/policy/src/bundle-manager.ts`, bundle manager auth
  tests under `packages/anvil/policy/src/`.
- **Validation:** Targeted bundle-manager tests prove a malicious bundle config
  cannot read sensitive env vars such as `GITHUB_TOKEN` by name, cannot send an
  allowed credential to an unbound host, and preserves the documented trusted
  credential path.
- **Identified From:** Deepsec P0 true-positive triage, run
  `20260629190245-caf2a4b60b2715fe`; finding `secret-env-var` in
  `packages/anvil/policy/src/bundle-manager.ts` (`Bundle auth can exfiltrate
  arbitrary environment variables`), revalidated true-positive.
- **Coordinates with:** CIB-108 (same policy package trust-boundary hardening),
  EVAL-001..005 if eval harness fixtures consume remote bundles.
- **Confidence:** high — arbitrary env-var selection is confirmed; host-binding
  details need implementation review to avoid breaking trusted operator config.

### CIB-110: Harden GitHub Actions trust boundaries before running PR-controlled code

- **Status:** Done 2026-07-02 — superseded by CIB-136..CIB-139 (decomposed; readiness verdict: oversized)
- **Intent:** Close the deepsec P1 CI findings where PR-controlled checkout,
  local actions, branch selectors, or manual dispatch inputs can influence checks
  that hold secrets, required-check authority, release authority, or self-hosted
  runner execution.
- **Expected Outcome:** Pull-request jobs do not authenticate to Azure or expose
  deployment secrets before executing PR-controlled code; local actions that
  receive secrets are loaded from trusted refs or replaced with trusted reusable
  workflows; change detection cannot be controlled by PR code; self-hosted runner
  workflows refuse untrusted refs; release/signing workflows enforce trusted tag
  ancestry, quoted inputs, and least-privilege target repositories.
- **Files:** `.github/actions/setup-workspace/action.yml`,
  `.github/actions/anvil-check/action.yml`, `.github/actions/detect-changes/action.yml`,
  `.github/workflows/{ci.yml,ci-nightly.yml,infra.yml,bench-nightly.yml,resource-budget.yml,rust.yml,security.yml,homebrew-bump.yml,release.yml,release-sign-artefacts.yml,napi.yml}`.
- **Validation:** GitHub Actions static review plus targeted workflow tests or
  dry-run checks proving PR jobs skip cloud login, required checks cannot be
  skipped by changed-file spoofing, and release/signing jobs reject untrusted tags
  or manual-dispatch targets.
- **Identified From:** Deepsec P1 true-positive clusters, run
  `20260629190245-caf2a4b60b2715fe`, across CI bypass, secrets exposure,
  self-hosted runner, release-signing, and command-injection findings.
- **Coordinates with:** release workflow governance, branch-protection required
  checks, CIB-031 dependency-audit scoping.
- **Confidence:** high — many findings share one trust-boundary shape: secrets or
  required-check decisions cross into PR-controlled code too early.

### CIB-111: Close API auth, token-rotation, and rate-limit race gaps

- **Status:** Done 2026-07-02 — superseded by CIB-140..CIB-143 (decomposed; readiness verdict: oversized)
- **Intent:** Remediate the active deepsec P1 findings in the Anvil API and docs
  shell around fail-open access, token rotation races, spoofable client identity,
  and unauthenticated expensive endpoints.
- **Expected Outcome:** Missing entitlement rows fail closed; refresh-token family
  revocation and replacement insertion are atomic; OTP active-code and attempt
  limits cannot be bypassed by concurrent requests; rate limits key on trusted
  client identity rather than spoofable `X-Forwarded-For`; public waitlist and
  OAuth callback endpoints have abuse throttles; private docs require the intended
  docs entitlement, not merely any valid licence JWT.
- **Files:** `apps/anvil-api/src/db/queries.ts`, `apps/anvil-api/src/index.ts`,
  `apps/anvil-api/src/middleware/rate-limit.ts`,
  `apps/anvil-api/src/routes/{auth-otp.ts,waitlist.ts}`,
  `apps/anvil-api/src/lib/licence.ts`,
  `apps/docs-shell/app/auth/callback/route.ts`, `apps/docs-shell/proxy.ts`.
- **Validation:** API tests covering fail-closed entitlement lookup, concurrent OTP
  request/verify races, refresh-token reuse under parallel rotation, trusted rate
  limit keys, waitlist/email throttling, callback replay throttling, and docs
  entitlement enforcement.
- **Identified From:** Deepsec P1 true-positive clusters, run
  `20260629190245-caf2a4b60b2715fe`, across auth-bypass, JWT handling,
  rate-limit-bypass, race-condition, and expensive-API-abuse findings.
- **Coordinates with:** auth-wall and docs-shell access policy; fixed device-flow
  P0 closure evidence should remain regression coverage, not active scope.
- **Confidence:** high — each finding has concrete affected files and observable
  concurrency or fail-closed tests.

### CIB-112: Gate mutating and script-executing MCP/CLI tools at the daemon boundary

- **Status:** Done 2026-07-02 — superseded by CIB-144..CIB-148 (decomposed; readiness verdict: oversized)
- **Intent:** Ensure mutating MCP tools, fence-unblock authority, and gate/fix
  execution paths cannot be invoked without the intended authentication and
  containment checks.
- **Expected Outcome:** Mutating MCP tools are registered behind the auth gate;
  `anvil_fix` write paths cannot escape via symlink races; read-only `gate` either
  cannot execute project-controlled scripts or requires explicit authenticated
  consent; CLI-gated commands fail closed when auth is missing; fence-unblock
  authority is enforced by the daemon protocol, not only by the CLI wrapper; path
  query tools normalise source/target paths before policy evaluation.
- **Files:** `crates/anvil-cli/src/main.rs`,
  `crates/anvil-cli/src/commands/intercept.rs`,
  `crates/anvil-cli/src/mcp/tools/{fix.rs,gate.rs,registry.rs,suppress.rs,query_boundary.rs}`.
- **Validation:** Rust tests proving unauthenticated MCP sessions cannot call
  mutating tools, symlink-swap write attempts are refused, gate execution posture
  matches the chosen consent policy, missing auth fails closed, and daemon
  fence-unblock rejects unauthorised clients.
- **Identified From:** Deepsec P1 true-positive clusters, run
  `20260629190245-caf2a4b60b2715fe`, across missing-auth, auth-bypass, RCE,
  path-boundary, and source-code-injection findings.
- **Coordinates with:** MCP auth model, intercept daemon protocol, suppression
  comment safety rules.
- **Confidence:** high — localised CLI/MCP boundary work, but gate execution
  posture may need an explicit product/security decision.

### CIB-113: Harden intercept daemon workspace admission, lineage, and session state

- **Status:** Done 2026-07-02 — superseded by CIB-149..CIB-154 (decomposed; readiness verdict: oversized)
- **Intent:** Close active deepsec P1/P2 daemon findings where the first IPC client
  or wire-declared metadata can influence trusted roots, lineage, guarded reads,
  or persisted fence state.
- **Expected Outcome:** Allowlist confinement never treats an attacker-chosen first
  save-time root as trusted authority; IPC clients cannot mint trusted lineage
  tags; wire change kind cannot suppress guarded reads or content scanning; fence
  updates are atomic and cannot lose security state; session lifecycle operations
  are bound to the owning peer; workspace admission budgets are enforced by live
  callers and cannot be loosened by project-controlled config.
- **Files:** `crates/anvil-intercept/src/{confinement.rs,save_time.rs,workspace_admission.rs,registry.rs,validate_paths.rs,fence.rs,dos.rs,workspace_pool.rs,lib.rs,fanout.rs,path_safety.rs}`.
- **Validation:** Daemon tests covering first-root rejection under allowlist
  confinement, forged lineage rejection, guarded-read enforcement independent of
  wire change kind, concurrent fence update preservation, session-owner checks,
  enforced workspace budgets, and FIFO/special-file read refusal or timeout.
- **Identified From:** Deepsec P1/P2 true-positive clusters, run
  `20260629190245-caf2a4b60b2715fe`, across ACL, trust-boundary,
  enforcement-bypass, race-condition, local-DoS, and false-attestation findings.
- **Coordinates with:** DSV daemon save-time guarantees, ACTMO durable worktree
  registration, CIB-090/CIB-097 filesystem hardening patterns.
- **Confidence:** high for the trust-boundary fixes; medium for concurrency tests
  that may require deterministic daemon harness support.

### CIB-114: Authenticate Windows named-pipe peers across Anvil clients

- **Status:** Ready
- **Intent:** Close active Windows IPC findings where CLI, driver-client, or
  intercept clients can connect without proving the server is the Anvil daemon and
  where trusted config files can be writable by other principals.
- **Expected Outcome:** Windows named-pipe clients authenticate the daemon peer and
  preserve local-only/SQOS expectations; driver-client validation no longer
  accepts an attacker-controlled pipe as the daemon; trusted config reads reject
  files writable by non-owner principals; Windows lineage/registration semantics
  match daemon expectations.
- **Files:** `crates/anvil-intercept-win32/src/lib.rs`,
  `packages/anvil-driver-client/src/transport/windows.ts`,
  `packages/anvil-driver-client/src/midedit/validate-mid-edit.ts`,
  `crates/anvil-run/src/ipc.rs`.
- **Validation:** Windows matrix tests or platform-specific unit tests proving pipe
  peer authentication, rejected writable config ACLs, spoof-block propagation, and
  valid registration lineage on Windows.
- **Identified From:** Deepsec P1/P2/skip clusters, run
  `20260629190245-caf2a4b60b2715fe`, across IPC impersonation, ACL, spoof-block
  suppression, and Windows registration findings.
- **Coordinates with:** CIB-100 and CIB-106 Windows named-pipe work.
- **Confidence:** medium — behaviour is clear, but reliable Windows validation is
  the gating risk.

### CIB-115: Centralise workspace path containment for TS adapters and APS loaders

- **Status:** Ready
- **Decomposition note (2026-07-02):** held back from the
  oversized-item split on purpose — every target directory sits in the
  retiring JS/TS `packages/` tree, so filing per-consumer children before
  the owner decides invest-vs-retire would allocate ids for work that may
  never run. Decide the retirement posture first, then split (suggested
  children: adapters, aps loader+validator, architecture scanner,
  policy-loader, file-cache — each routing through the existing
  `path-safety.ts` helper).
- **Intent:** Close the deepsec P1 path-traversal cluster caused by copied,
  inconsistent path validation across APS, SpecKit, BMAD, architecture, policy,
  and cache code.
- **Expected Outcome:** A single containment helper rejects dot segments,
  backslash traversal, absolute paths, symlink escapes, and untrusted task/scenario
  identifiers before they become filesystem paths. APS adapters/importers,
  architecture scanners, policy discovery, runtime cache, and APS state helpers use
  the shared helper or prove equivalent behaviour.
- **Files:** `packages/anvil/core/src/utils/path-safety.ts`,
  `packages/adapters/src/**`, `packages/aps/src/{loader,state,validator}/**`,
  `packages/anvil/core/src/architecture/**`,
  `packages/anvil/policy/src/policy-loader.ts`,
  `packages/anvil/runtime/src/cache/providers/file-cache.ts`.
- **Validation:** Cross-package tests covering POSIX and Windows separators,
  symlinked directories, generated scenario/task IDs, duplicate basename handling,
  and cache-entry names; existing adapter and APS validator tests still pass.
- **Identified From:** Deepsec P1/P2 true-positive clusters, run
  `20260629190245-caf2a4b60b2715fe`, dominated by path-traversal findings in
  adapters, APS state/loader, architecture analysis, policy loading, and cache
  providers.
- **Coordinates with:** CIB-108 (policy eval trust boundary), APS parser/validator
  governance.
- **Confidence:** high — root cause is duplicated validation; broad file touch
  warrants careful staged tests.

### CIB-116: Redact provenance and debug secrets before persistence or logs

- **Status:** Ready
- **Intent:** Prevent Copilot tokens, credential-bearing Git remotes, raw debug
  payloads, and admin secrets from being persisted to notes, provenance records,
  logs, or command histories.
- **Expected Outcome:** Git remote URLs and AI session identifiers are redacted or
  rejected before persistence; Copilot tokens are never treated as session IDs;
  structured debug payloads pass through the same redaction path as string logs;
  admin and revoke tokens are not accepted through command-line arguments where
  process listings can expose them.
- **Files:** `packages/anvil/core/src/provenance/**`,
  `apps/anvil-api/src/lib/debug.ts`, `apps/admin-cli/src/index.ts`,
  `apps/admin-cli/src/commands/revoke.ts`.
- **Validation:** Tests proving credential-bearing remotes, Copilot-like tokens,
  structured debug payloads, and CLI argument secrets are redacted, rejected, or
  moved to safer input channels; existing provenance output remains stable for
  non-secret values.
- **Identified From:** Deepsec P1/P2 true-positive clusters, run
  `20260629190245-caf2a4b60b2715fe`, across secret-in-log, secrets-exposure,
  info-disclosure, and secret CLI argument findings.
- **Coordinates with:** provenance/Git AI standard outputs, admin CLI operator
  runbook.
- **Confidence:** high — secret shapes are concrete; compatibility risk is limited
  to documented input channels and provenance schema expectations.

### CIB-117: Fence TS runtime and APS state transitions against lost updates

- **Status:** Merged 2026-07-02 via PR #3087
- **Intent:** Close deepsec P1 race findings in TypeScript lock/state helpers that
  can lose task records, lock records, or mutual-exclusion guarantees.
- **Expected Outcome:** Expired/stale lock takeover is fenced and atomic; APS state
  updates cannot overwrite concurrent task or lock records; file-backed state
  helpers use atomic write/compare or a process-level lock appropriate to the
  runtime; segment targeting logic is reconciled across TypeScript and any other
  feature-flag runtimes.
- **Files:** `packages/anvil/runtime/src/concurrency/lock-manager.ts`,
  `packages/aps/src/state/index.ts`,
  `packages/anvil/runtime/src/feature-flags/resolver.ts`.
- **Validation:** Concurrency tests proving stale lock takeover is single-winner,
  simultaneous APS state writes preserve both records, and TypeScript segment
  targeting matches the intended flag contract.
- **Identified From:** Deepsec P1 true-positive clusters, run
  `20260629190245-caf2a4b60b2715fe`, across race-condition and
  cross-runtime-logic-drift findings.
- **Coordinates with:** feature flag governance, APS state tooling.
- **Confidence:** high for lock/state tests; medium for feature-flag parity if
  external consumers depend on current behaviour.

### CIB-118: Make Edda and Kindling state transitions atomic and payload-consistent

- **Status:** Merged 2026-07-02 via PR #3088
- **Intent:** Close deepsec P1/P2 findings where Edda memory/proposal transitions
  and Kindling observation writes can be duplicated, overwritten, or validated
  against a different payload than the one stored.
- **Expected Outcome:** Edda promotion, supersede, memory index updates, and
  proposal resolution are idempotent and atomic; terminal proposal states cannot
  be re-resolved or promoted twice; Kindling writes exactly the payload that was
  validated; human-input observation IDs returned by emitters are persisted or the
  API is corrected.
- **Files:** `packages/edda-stack/src/edda/{memory-store.ts,promotion-service.ts,evolution-service.ts}`,
  `packages/edda-stack/src/ember/proposal-store.ts`,
  `packages/kindling-integration/src/{kindling-service.ts,emitters/human-input-emitter.ts}`.
- **Validation:** Concurrency/idempotency tests for duplicate promotions,
  supersede/update races, terminal proposal resolution, validated-payload equality,
  and persisted human-input observation linkage.
- **Identified From:** Deepsec P1/P2/skip clusters, run
  `20260629190245-caf2a4b60b2715fe`, across race-condition,
  non-atomic-state-transition, validation-bypass, and observation-linkage findings.
- **Coordinates with:** Edda/Kindling storage semantics and any existing proposal
  lifecycle tests.
- **Confidence:** medium — fixes are localised but concurrency semantics may need
  explicit product decisions around conflict resolution.

### CIB-119: Gate infrastructure secrets and production resources by trusted stack context

- **Status:** Merged 2026-07-02 via PR #3086
- **Intent:** Stop non-production or PR-controlled infrastructure paths from
  reading production secrets or defining/mutating production resources.
- **Expected Outcome:** Pulumi preview/apply paths do not fetch production Key
  Vault secrets for untrusted previews; non-prod stacks cannot create production
  Vercel, signing, or admin-key resources; concurrent admin-key creation preserves
  the active-key invariant.
- **Files:** `infra/src/{keyvault.ts,vercel.ts,signing.ts,admin-keys.ts}`,
  `infra/scripts/admin-key-manage.mjs`, `.github/workflows/infra.yml`.
- **Validation:** Infrastructure unit tests or Pulumi preview tests proving stack
  gating for production-only resources, no production secret reads in untrusted
  previews, and atomic active-key creation under concurrent attempts.
- **Identified From:** Deepsec P1/P2/skip clusters, run
  `20260629190245-caf2a4b60b2715fe`, across secrets-exposure,
  cross-stack-resource-ownership, admin-key-provisioning, and race-condition
  findings.
- **Coordinates with:** CIB-110 for workflow-side secret exposure; infrastructure
  release/deployment runbooks.
- **Confidence:** high for stack guards; medium for concurrency depending on the
  backing admin-key store.

### CIB-120: Pin release-time installers and signing-job supply-chain inputs

- **Status:** Merged 2026-07-02 via PR #3077
- **Intent:** Close active deepsec P2 supply-chain findings where release or
  signing jobs execute unpinned installer scripts before using release authority or
  private signing keys.
- **Expected Outcome:** Release workflows pin and verify `rsign2`, `rustup`, and
  any mutable installer inputs before execution; signing jobs install tools from
  trusted, integrity-checked artefacts; release/signing jobs document the
  verification mechanism and fail closed on mismatch.
- **Files:** `.github/workflows/{release.yml,release-sign-artefacts.yml}`.
- **Validation:** Workflow review plus a dry-run or shellcheck-style validation
  proving pinned URLs/checksums are used and checksum mismatch fails before any
  private key or publish token is available.
- **Identified From:** Deepsec P2 true-positive clusters, run
  `20260629190245-caf2a4b60b2715fe`, across unpinned `rsign2` and `rustup`
  installer findings.
- **Coordinates with:** CIB-110 release workflow hardening; release skill/runbook.
- **Confidence:** high — small workflow hardening with clear fail-closed evidence.

### CIB-121: Sweep P2 product-surface correctness and information-disclosure findings

- **Status:** Proposed
- **Intent:** Disposition lower-priority deepsec findings in public/admin surfaces
  that are not covered by the P1 hardening clusters but still have observable user
  or operator impact.
- **Expected Outcome:** Each remaining P2 public/admin finding is either fixed,
  accepted with rationale, or promoted to a more specific Ready item. Initial
  candidates include waitlist membership enumeration, OTP timing differences,
  early-access throttling, placeholder PGP disclosure instructions, non-ASCII
  secret-header comparison crashes, admin dry-run copy drift, and invite/approval
  polling-secret loss.
- **Files:** `apps/anvil-api/src/routes/{admin.ts,auth-otp.ts,waitlist.ts}`,
  `apps/website/app/api/early-access/install/route.ts`,
  `apps/website/app/security/page.tsx`,
  `apps/{anvil-docs-private,docs-public}/middleware.ts`,
  `apps/admin-cli/src/commands/send-migration.ts`,
  `apps/docs-shell/app/auth/error/page.tsx`.
- **Validation:** Targeted tests or documented accepted-risk decisions for each
  disposition; no unresolved P2 finding in these surfaces remains without a
  tracking reference.
- **Identified From:** Deepsec P2/untriaged/skip clusters, run
  `20260629190245-caf2a4b60b2715fe`.
- **Coordinates with:** CIB-111 and CIB-116 where fixes share API or secret-handling
  primitives.
- **Confidence:** medium — this is a disposition sweep; some findings may collapse
  into existing items after closer review.

### CIB-122: Disposition remaining deepsec P2/skip/untriaged quality findings

- **Status:** Proposed
- **Intent:** Ensure the residual deepsec triage output does not become a parallel
  backlog outside APS.
- **Expected Outcome:** The remaining non-P1 findings in adapters, generic parser,
  docs metadata, eslint rules, render validation, codemods, CLI process handling,
  and security-scan reporting are reviewed and either fixed, accepted, closed as
  duplicate of CIB-110..121, or split into one Ready item per executable root
  cause.
- **Files:** `packages/adapters/src/**`, `packages/docs-meta/src/**`,
  `packages/eslint-plugin-anvil/src/**`, `packages/libs/render/src/**`,
  `tools/{codemods,nx-rust,generators}/**`, `.github/workflows/security.yml`,
  `crates/anvil-cli/src/{commands/intercept.rs,mcp/enforcement.rs,mcp/tools/gate.rs,mcp/tools/validate_write.rs}`.
- **Validation:** A reconciliation note in this item or split child items lists the
  disposition for each residual P2/skip/untriaged finding; deepsec export no
  longer contains untracked residuals for those clusters.
- **Identified From:** Deepsec P2/skip/untriaged residual clusters, run
  `20260629190245-caf2a4b60b2715fe`.
- **Coordinates with:** CIB-110..121; split any finding that proves larger or more
  urgent than a sweep.
- **Confidence:** medium — this deliberately starts as triage/disposition work to
  avoid over-filing low-priority duplicates.

### CIB-123: Reconcile the language-profile registry with shipped parser/check coverage

- **Status:** Merged 2026-06-30 via PR #3011
- **Decision (owner tier call, 2026-06-30):** **Python → Supported** (PYLAN
  shipped the catalogue + scanning + boundary, same bar as Rust; the
  "PYLAN parked" entry was stale — a Python-only repo no longer maps to
  `Unsupported`). **Tail T1 languages stay Unsupported but are now listed**
  (added C#/`.cs`, Dart/`.dart`, Zig/`.zig`, WebAssembly-text/`.wat`,`.wast` +
  full C/C++ extensions) — parsed but no language-specific catalogue. **No new
  tier**: a tier reflects shipped language-specific governance, not parser
  capability.
- **Intent:** The CLI's user-facing language profile
  (`supported`/`partial`/`unsupported`) reflects current coverage instead of a
  hand-maintained list that has drifted from the kernel parser.
- **Expected Outcome:** The hardcoded `LANGUAGE_REGISTRY` in
  `crates/anvil-cli/src/activation/language_profile.rs` is reconciled with
  reality. At minimum **Python is re-tiered** — PYLAN shipped a T3 reliability
  catalogue, but the registry still reports Python `unsupported` ("PYLAN parked").
  A decision is recorded on how **parser-only** languages — parsed by the kernel
  (T1) but carrying no governance checks yet — should be reported: a distinct
  "parsed"/preview signal versus plain `unsupported`. The LANGTAIL/LTW2 tail (Go,
  Java, Kotlin, C#, C/C++, Dart, Zig, WebAssembly-text) is classified per that
  decision, and the currently-**unlisted** extensions (`.cs`, `.dart`, `.zig`,
  `.wat`/`.wast`) are added to the registry instead of falling into
  `unclassified_files_seen`.
- **Files:** `crates/anvil-cli/src/activation/language_profile.rs`,
  `crates/anvil-cli/src/activation/render.rs`,
  `crates/anvil-cli/src/activation/diagnostic.rs`,
  `crates/anvil-cli/tests/status_verify_languages.rs`.
- **Validation:** `status_verify_languages` tests updated to the reconciled
  tiers; `anvil status` reports them; `ProtectionState` transitions reviewed (a
  repo with only Python/tail files no longer necessarily resolves to
  `Unsupported`).
- **Identified From:** LTW2-005 (PR #3006) — a doc edit claimed parser
  capability; a reviewer caught that the user-facing registry diverges (the
  parser parses Python + the tail, the registry reports them unsupported). See
  `lang-tail-wave-2.aps.md` Open Questions.
- **Coordinates with:** `lang-tail-wave-2` (LTW2), `lang-python` (PYLAN), and the
  [2026-04-08 Language and Coverage Design](../specs/2026-04-08-language-and-coverage-design.md)
  tier definitions (T1/T2/T3 vs the registry's three tiers).
- **Risk:** re-tiering changes `ProtectionState` for affected repos (a flip from
  `Unsupported`), so it needs **owner sign-off** on the tier mapping — not a
  silent change.
- **Confidence:** medium — the Python re-tier is clear-cut; the "how to report
  parser-only languages" question is a design decision.

### CIB-124: Witness `acquire_lock` timeout + `Drop`-guard

- **Status:** Merged 2026-07-01 via PR #3021 (full 5-reviewer council; blockers
  fixed in-PR). `WitnessWriter::acquire_lock` polls `try_lock_exclusive`
  with capped backoff (5ms→100ms) up to a `DEFAULT_LOCK_ACQUIRE_TIMEOUT` (**5s**
  per acquire; see follow-on (a) for the env override), returning the new
  `WriterError::LockTimeout(Duration)` instead of
  blocking indefinitely; the held lock is a `LockGuard` RAII wrapper that releases
  on `Drop` (including on panic). `acquire_lock_with_timeout(dur)` is split out so
  the timeout path is testable without a multi-second wait. Contention is detected
  via `fs2::lock_contended_error()` (raw-OS-error compare), not the Rust-version-
  dependent `WouldBlock` `ErrorKind` — so the retry loop works on Windows on older
  toolchains too. Both the embedded hook and the daemon
  (`save_time.rs::witness_append`) route through `append_chained` → `acquire_lock`,
  so both are bounded, each with a distinct `LockTimeout` log (the hook mapping is
  the pure, unit-tested `classify_append_error`; `LockTimeout` → `WriteFailed`).
  Note: MLP2-005 phase 3 landed first (the ordering the phase-1 council flagged was
  inverted), so this bounds the lock **before beta** rather than before the
  hook-fallback wiring — the net protection is the same.
- **Council follow-ons (2026-07-01, tracked as GA hardening, not blocking this
  PR):** (a) **`ANVIL_WITNESS_LOCK_TIMEOUT` env override — DONE 2026-07-01 via PR
  #3027.** The default is now `DEFAULT_LOCK_ACQUIRE_TIMEOUT` (5s), overridable by
  the env var (whole seconds); the pure `anvil_witness::lock_timeout_from_env`
  (crate stays env/log-free) is resolved by the hook embedded leg per-call and by
  the daemon **once at start** (stored on `SaveTimeState`, so a malformed value is
  warned once, not per append — a change needs a daemon restart). Both pass it via
  `append_chained_with_lock_timeout`; warning-and-defaulting on a malformed value.
  Documented in the witness runbook (compound worst-case + restart caveats).
  (b) **Non-blocking daemon leg** —
  the daemon's `acquire_lock` still `thread::sleep`s on a tokio worker thread up to
  the timeout, and on a wedged lock the commit path compounds (~2s daemon RPC + the
  embedded 5s ≈ 7s); a short daemon-side lock timeout (defer to embedded on
  contention) would cut both (adversarial F1). (c) **Runbook entry** for "witness
  lock wedged" — how to find the holder (`lsof`/`fuser anvil/witness/.lock`),
  whether killing it is safe, where the tracing log lives (operations).
- **Intent:** Stop a stalled witness writer from wedging concurrent committers,
  and make the flock release explicit on panic.
- **Expected Outcome:** `WitnessWriter::acquire_lock`
  (`crates/anvil-witness/src/writer.rs`) replaces the unbounded
  `fs2::FileExt::lock_exclusive` (which blocks indefinitely — a stalled holder or
  an NFS hang blocks every concurrent committer) with `try_lock_exclusive` + a
  bounded retry/backoff, returning a timeout error (mapped to `WriteFailed` at the
  hook) rather than hanging. The held lock is wrapped in a small RAII guard so the
  `flock` releases on the `Drop` path (including panics), not only via the explicit
  `unlock`.
- **Validation:** a test holding the lock from one handle proves a second
  `acquire_lock` times out (not hangs) and maps to `WriteFailed`; a
  panic-in-closure test proves the lock is released.
- **Files:** `crates/anvil-witness/src/writer.rs`,
  `crates/anvil-cli/src/commands/hook.rs` (error mapping).
- **Identified From:** MLP2-005 phase-1 Council (2026-06-30) — operations HIGH
  (lock blast radius) + adversarial LOW (Drop-guard). **Do before the MLP2-005
  hook-fallback phase wires daemon + CLI cross-process writes.**
- **Confidence:** medium.

### CIB-125: Cross-process `append_chained` linearisation test

- **Status:** Merged 2026-07-01 via PR #3024 — `crates/anvil-witness/tests/
  cross_process_linearisation.rs` (custom `harness = false`) re-execs itself via
  `current_exe()` as 6 worker processes, released together on a go-signal, each
  doing 5 `append_chained` calls against one shared chain; the parent asserts
  `verify_chain_dag` yields one Healthy, strictly-linear chain (genesis + 30
  records, each present exactly once). Validated by temporarily reverting
  `append_chained` to an out-of-lock head read → the test fails with a `ChainBreak`
  (fork detected). Closes the last MLP2-005 "No divergence" validation bullet.
- **Intent:** Prove the witness-append atomicity holds across **separate
  processes** (a daemon vs a CLI fallback), not just threads.
- **Expected Outcome:** an integration test in `crates/anvil-witness/tests/`
  spawns N separate processes (e.g. re-exec via `std::process::Command` /
  `std::env::current_exe()` with a worker mode) that each `append_chained` against one shared
  root, then asserts `verify_chain_dag` yields a single linear chain — exercising
  the real cross-process flock path the phase-1 thread test only approximates.
- **Validation:** the new test passes; deliberately reverting to an out-of-lock
  head read makes it fail (fork detected).
- **Files:** `crates/anvil-witness/tests/`.
- **Identified From:** MLP2-005 phase-1 Council (2026-06-30) — adversarial MEDIUM
  (cross-process correctness asserted but only thread-tested).
- **Confidence:** high.

### CIB-126: Witness chain-init marker (zero-byte-active reseed detection)

- **Status:** Merged 2026-07-01 via PR #3026 (full 5-reviewer council; blockers
  fixed in-PR). A durable witness-root sentinel
  `anvil/witness/.chain-initialised` (sibling of `.lock`, non-`.ndjson` so
  `witness_paths` skips it) is written under the flock by `append_chained` the
  first time a chain is seeded, and **backfilled** on the next `append_chained` for
  any pre-existing chain. `read_chain_head` returns `ChainBroken` (not `Empty`)
  whenever the walk yields zero lines with all segments empty/absent **but** the
  marker exists — covering both a **truncated** and a **deleted** active file
  (council F1) — while a marker-less empty/absent active stays `Empty` (fresh repo).
  The manifest was not usable as the marker: it is only written at rollover, so a
  young un-rolled chain has none.
- **Git posture (owner decision):** the marker is a **local runtime artefact**
  (gitignored via `anvil init`, exempt in the `doctor` durable-state sweep),
  matching `.lock` — its presence, not its history, is load-bearing, and it
  self-heals via backfill. Not committed.
- **Symlink hardening (council F2):** the marker path is symlink-refused up-front
  in `acquire_lock` alongside root/`.lock`/active, and `refuse_if_symlink` now uses
  `symlink_metadata` (not `path.exists()`) so a **dangling** symlink is caught too
  — closing the silent-disable a squatted symlink would otherwise allow (this also
  hardens the pre-existing root/`.lock`/active checks).
- **Recovery:** a `ChainBroken` from an emptied/deleted active is restored with
  `git checkout -- anvil/witness/active.ndjson` (or, if never committed, by
  removing the marker to permit a reseed) — `--witness-recent` no-ops here.
  Documented in `docs/runbooks/anvil-witness-chain.md` §5b.
- **Residuals (acknowledged):** (a) **fresh-clone window** — the marker is
  gitignored, so a clone that zeroes the active *before* its first commit still
  reseeds; protection is active from the first commit onward. (b) **two-failure
  window** — a crash between the genesis write and the marker write, *then* a
  truncation, reseeds; needs a transactional FS to close. (c) a deliberate actor
  with witness-dir write access can pre-place a marker + zero-byte active to DoS a
  fresh repo — equivalent to the existing garbled-active DoS.
- **Intent:** Detect a truncated-to-zero `active.ndjson` with no archives, which is
  currently indistinguishable from a fresh repo and silently reseeds genesis over
  erased history.
- **Expected Outcome:** a durable "chain initialised" marker (a witness-root
  sentinel written at genesis) so `WitnessWriter::read_chain_head` returns
  `ChainBroken` for a zero-byte active when the chain is known to have existed,
  closing the residual the phase-1 non-empty-unparseable hardening does not cover,
  without regressing the legitimate fresh-repo path.
- **Design note (ADR-038 alignment):** the marker's PRESENCE is the only
  load-bearing bit (body is a versioned sentinel). It lives on a separate inode
  from `active.ndjson`, so it survives the accidental event — a crash mid-write, a
  disk glitch, a stray truncation — that zeroes the active file, letting the writer
  distinguish "erased established chain" (refuse, ADR-038) from "fresh repo" (seed).
  It deliberately does **not** defend against a determined actor who deletes the
  marker AND truncates the active file (that actor can rewrite the whole chain, and
  the hash-chain verifier already catches a substituted chain); it closes the
  simple/accidental silent-reseed the non-empty-unparseable hardening left open.
  The chain-broken refusal blocks the commit exactly as any other `ChainBroken`
  does (recovery above).
- **Validation:** zero-byte active after a prior genesis → `ChainBroken` (build not
  run, no reseed); **deleted** active after genesis → `ChainBroken`; marker-less
  empty active → `Empty` (seeds); marker backfilled for a legacy Healthy chain;
  rollover + empty active → `Healthy` (archive carries the chain, no false break);
  a symlinked marker (dangling) → `SymlinkRoot`; marker excluded from the chain
  walk. All covered by `writer.rs` unit tests.
- **Files:** `crates/anvil-witness/src/writer.rs`;
  `crates/anvil-cli/src/commands/{init.rs,doctor.rs,hook.rs}` (gitignore + doctor
  exemption + recovery comment); `docs/runbooks/anvil-witness-chain.md` (§5b).
- **Identified From:** MLP2-005 phase-1 Council (2026-06-30) — adversarial residual
  (the proposed `any_nonempty` fix does not close the zero-byte case).
- **Confidence:** low — needs a design decision before execution.

### CIB-127: Sync public docs to current release delta

- **Status:** Done
- **Intent:** Align public user documentation with release-facing behaviour landed
  since the last tagged baseline.
- **Expected Outcome:** Public release notes, MCP docs, activation/language-support
  copy, daemon troubleshooting, config reference, and insights docs no longer
  contradict the current release delta.
- **Validation:** `pnpm run docs:check`
- **Files:** `docs/public/anvil/releases/changelog.md`,
  `docs/public/anvil/integrations/mcp.md`,
  `docs/public/anvil/quickstart.md`,
  `docs/public/anvil/beta-testing-guide.md`,
  `docs/public/anvil/guides/wow-start-demo.md`,
  `docs/public/anvil/tutorials/rust-project.md`,
  `docs/public/anvil/overview.md`,
  `docs/public/anvil/operations/config.md`,
  `docs/public/anvil/operations/troubleshooting.md`,
  `docs/public/anvil/guides/insights.md`.
- **Identified From:** Public-doc review after `v0.8.2-beta` showing stale release
  baseline, missing GCTX MCP context docs, stale Python support copy, and
  conflicting daemon-stop guidance.
- **Confidence:** high.

### CIB-128: Parse `anvil-intercept` CLI before installing daemon tracing

- **Status:** Merged 2026-07-02 via PR #3076
- **Intent:** Let clap handle help/version/usage exits before the daemon tracing
  subscriber is installed, so `--help`/`--version` never emit a trace record or
  create a trace sink.
- **Expected Outcome:** `anvil-intercept --help` with tracing configured prints
  the clap help banner to stdout with empty stderr and no trace-sink file
  created; tracing init stays immediately before the foreground daemon path.
- **Validation:** `cargo test -p eddacraft-anvil-intercept` (extend
  `tests/binary_contract.rs` to reject pre-help tracing output).
- **Files:** `crates/anvil-intercept/src/main.rs`,
  `crates/anvil-intercept/tests/binary_contract.rs`.
- **Identified From:** clawpatch 2026-07-02 triage
  (`fnd_sig-feat-cli-command-43c5f1e5c2`, low / contract-mismatch);
  `plans/reviews/2026-07-02-clawpatch-triage.md`.
- **Confidence:** high.

### CIB-129: Cover the `anvil-rayon-init` half-cores pool cap with a test

- **Status:** Draft
- **Intent:** Add regression coverage for the half-cores global rayon pool cap,
  which is currently untested even though `cap_threads` was factored out as a
  pure function specifically to be unit-testable.
- **Expected Outcome:** a unit test pins `cap_threads == (num_cpus / 2).max(1)`
  across representative core counts; optionally a subprocess/integration smoke
  asserts `init_global()` applies the cap before any `par_iter`.
- **Validation:** `cargo test -p eddacraft-anvil-rayon-init`.
- **Files:** `crates/anvil-rayon-init/src/lib.rs`.
- **Identified From:** clawpatch 2026-07-02 triage
  (`fnd_sig-feat-library-8a1266b4d7`, low / test-gap);
  `plans/reviews/2026-07-02-clawpatch-triage.md`.
- **Confidence:** high.

### CIB-130: Rust test-hardening batch (2026-07-02 clawpatch tail)

- **Status:** Draft
- **Intent:** Burn down the low-severity Rust test-hygiene tail from the
  2026-07-02 clawpatch scan (assertions that pass for the wrong reason plus two
  confirmed test-harness bugs), the same pattern as the CLAWP #1740 batch.
- **Expected Outcome:** the two confirmed-bugs fixed — (a) `status_render.rs`
  fixture-update is single-owner/deterministic and only `ANVIL_UPDATE_FIXTURES=1`
  enables rewriting; (b) `langtail_external_validation.rs` includes
  `total.panics` in the not-cleanly-parsed rate and printed breakdown — and each
  strengthened test-gap assertion proven non-vacuous against a deliberate mutant.
- **Validation:** `cargo test -p eddacraft-anvil` and per touched crate; full
  item list via
  `jq -r '.items[] | select((.evidence[0].path|startswith("crates/")) and .status=="open" and (.evidence[0].path|contains("/tests/"))) | "\(.evidence[0].path)\t\(.title)"' plans/audits/2026-07-02-clawpatch-periodic-scan.json`.
- **Files:** `crates/anvil-cli/tests/status_render.rs`,
  `crates/anvil-kernel/tests/langtail_external_validation.rs`, plus the test-gap
  tail enumerated in `plans/audits/2026-07-02-clawpatch-periodic-scan.json`.
- **Identified From:** clawpatch 2026-07-02 triage (63 test-gap + 2 confirmed-bug
  in `crates/**/tests/`); `plans/reviews/2026-07-02-clawpatch-triage.md`.
- **Confidence:** medium.

### CIB-131: Harden dogfood FP classifier path handling

- **Status:** Draft
- **Intent:** Stop `scripts/dogfood/external-fp/classify.py` from accepting
  warning file paths that escape the checked-out repository, and make the
  worksheet build tolerate an absent output directory.
- **Expected Outcome:** paths resolving outside the repo root are rejected or
  normalised (no traversal); the worksheet build creates its output directory
  when missing instead of failing.
- **Validation:** manual dogfood run of `classify.py` against a repo-escaping
  warning path and an absent output dir; add a focused unit test if the harness
  supports it.
- **Files:** `scripts/dogfood/external-fp/classify.py`.
- **Identified From:** clawpatch 2026-07-02 triage (`classify.py`
  medium / confirmed-bug path-escape + low / confirmed-bug output-dir-absent);
  `plans/reviews/2026-07-02-clawpatch-triage.md`.
- **Confidence:** high.

### CIB-132: Precise SSL detection in `admin-key-manage`

- **Status:** Merged 2026-07-02 via PR #3079
- **Intent:** Replace the naive connection-string substring match that disables
  SSL in `infra/scripts/admin-key-manage.mjs` with precise parameter parsing,
  and align the documented create-output field name with what the script emits.
- **Expected Outcome:** SSL is disabled only when the connection string
  genuinely requests it (parsed, not substring-matched); the documented output
  field and the emitted field agree (`hashed_key` vs `hashedKey`).
- **Validation:** run `admin-key-manage.mjs` against `sslmode`-varied connection
  strings and confirm SSL toggling plus the emitted output field.
- **Files:** `infra/scripts/admin-key-manage.mjs`.
- **Identified From:** clawpatch 2026-07-02 triage (`admin-key-manage.mjs`
  medium / confirmed-bug SSL-substring + low / contract-mismatch field name);
  `plans/reviews/2026-07-02-clawpatch-triage.md`.
- **Confidence:** high.

### CIB-133: Gate the first-week insights nudge under `project_writes_gated` in `status` and `watch`

- **Status:** Proposed
- **Re-filed:** 2026-07-02 — originally filed as CIB-105; that entry was
  removed by an accidental stale-base revert (`e57a65fdf`, 2026-06-26) and the
  CIB-105 id was subsequently reused by "Windows reparse-point hardening for
  Kindling sidecar writes" (`97e00b0ed`, 2026-06-26). Body restored verbatim
  from the pre-revert state; the work was never implemented
  (`first_week_insights_hint` still takes no gate parameter).
- **Intent:** Stop `anvil status` and `anvil watch` from reading-and-recording
  the real project's first-week-nudge state under a gated `ANVIL_HOME`
  (DISTRIB-006 / ADR-060). Both call `first_week_insights_hint` ungated today, so
  a candidate / side-by-side install burns the real install's once-per-week
  marker (and writes `.anvil/insights-hint.json` into the real project).
- **Expected Outcome:** All three nudge surfaces (`status`, `watch`, `welcome`)
  honour the gate uniformly. The cleanest shape is to thread the gate into the
  canonical function — `first_week_insights_hint(root, now, project_writes_gated)`
  returning `None` with no read and no write when gated — and drop INSIGHTS-005's
  `welcome`-specific `welcome_insights_hint` wrapper, so no surface can regress by
  forgetting the guard.
- **Files:** `crates/anvil-cli/src/insights/first_week_hint.rs`,
  `crates/anvil-cli/src/commands/status.rs`,
  `crates/anvil-cli/src/commands/watch.rs`,
  `crates/anvil-cli/src/commands/welcome.rs`.
- **Validation:** `cargo test -p eddacraft-anvil` — a gated-root test per surface
  asserts the nudge is suppressed and `.anvil/insights-hint.json` is not written;
  the existing INSIGHTS-004/-005 in-window tests still pass.
- **Identified From:** INSIGHTS-005 pre-PR Council (PR #2957) — code-reviewer
  MINOR + NIT: `status.rs` and `watch.rs` call the hint ungated; `welcome.rs` is
  the correct reference but its gate is surface-specific boilerplate for a
  universal concern.
- **Coordinates with:** INSIGHTS-004 (PR #2226, the hint mechanism), INSIGHTS-005
  (PR #2957, the welcome wiring + gated wrapper this would absorb).
- **Confidence:** high — small and additive; the gating logic already exists in
  `welcome.rs` and just needs lifting into the canonical function with the two
  call sites updated.

### CIB-134: Widen the rustdoc `-D warnings` gate to the whole workspace (clear the pre-existing all-features cascade)

- **Status:** Draft
- **Re-filed:** 2026-07-02 — originally filed as CIB-106; that entry was
  removed by an accidental stale-base revert (`e57a65fdf`, 2026-06-26) and the
  CIB-106 id was subsequently reused by the 2026-06-27 Windows named-pipe
  intake. Body restored verbatim from the pre-revert state.
- **Intent:** Extend the CIB-030 PR-side rustdoc gate from `eddacraft-tui` to
  the whole workspace so a rustdoc regression in any crate fails at PR review,
  once the pre-existing all-features rustdoc errors are cleared.
- **Expected Outcome:**
  `RUSTDOCFLAGS='-D warnings' cargo doc --workspace --no-deps --all-features` is
  green on `main`, and the `rust.yml` `doc` job is widened from
  `-p eddacraft-tui` to `--workspace`.
- **Known failures (2026-06-27, `main` @ 45c0e9edd):**
  - `eddacraft-anvil-kernel-types`: `parse_str` doc links the private
    `enforce_depth_cap`; `protection_claim` doc links the private
    `ProtectionClaimRaw` (`rustdoc::private_intra_doc_links`).
  - `eddacraft-anvil-config`: unresolved intra-doc link
    `crate::watch_event::tests::error_code_wire_strings_are_pascal_case_and_pinned`
    (links a `#[cfg(test)]` item).
  - unresolved link to `tree_sitter::Node::byte_range` — a dependency item, not
    resolvable under `--no-deps`; convert to a code span.
- **Validation:** the widened workspace doc build is green with `-D warnings`; a
  deliberately broken intra-doc link in any crate fails the `doc` job.
- **Identified From:** CIB-030 completion (PR #2967) — the
  `--workspace --all-features -D warnings` probe surfaced six pre-existing
  rustdoc errors outside CIB-030's `eddacraft-tui` scope.
- **Files:** `crates/anvil-kernel-types/`, `crates/anvil-config/`, the crate
  carrying the `tree_sitter::Node::byte_range` link, and
  `.github/workflows/rust.yml` (`-p eddacraft-tui` → `--workspace`).
- **Coordinates with:** CIB-030 (the `eddacraft-tui`-scoped gate this widens).
- **Confidence:** medium — the six known errors are small fixes, but the full
  all-features surface may surface more once those clear.

### CIB-135: Gate live prod DNS records out of the untrusted dev stack

- **Status:** Proposed
- **Intent:** `infra/src/dns/eddacraft-ai.ts` (imported unconditionally in
  `infra/index.ts`) creates real `RecordSet` resources for `eddacraft.ai`
  inside `rg-prd-ap-public-web`, and `azure-dns:resourceGroupName` is
  configured identically in BOTH `Pulumi.dev.yaml` and `Pulumi.prod.yaml` — so
  the untrusted `dev` stack (the PR-preview stack) still manages live
  production DNS records, ungated by CIB-119's `isTrustedStack()`.
- **Expected Outcome:** the DNS record definitions are gated by the
  `stack-trust.ts` mechanism (or moved behind a prod-only module boundary), so
  an untrusted-stack preview/up defines zero live DNS resources; a test pins
  the untrusted-stack resource count, mirroring `untrusted-stack.test.ts`.
- **Files:** `infra/src/dns/eddacraft-ai.ts`, `infra/index.ts`,
  `infra/Pulumi.dev.yaml` (resource-group config split, if taken).
- **Validation:** a Pulumi-mock test with stack `dev` asserts zero DNS
  `RecordSet` resources are registered (mirroring
  `untrusted-stack.test.ts`), and the `prod`-stack sibling asserts the
  records are still defined; full infra vitest suite stays green.
- **Identified From:** CIB-119 pre-merge review (PR #3086) — same
  cross-stack-resource-ownership class as CIB-119's own findings, but outside
  its Files list; pre-existing, not introduced by that PR.
- **Confidence:** high — the gating mechanism already exists
  (`infra/src/stack-trust.ts`); this applies it to one more module.

### CIB-136: Stop PR preview jobs exposing production Azure/Key Vault secrets to PR-controlled Pulumi code

- **Status:** Ready
- **Intent:** `infra.yml`'s `preview` job (same-repo `pull_request`, `infra.yml:62-64`) checks out the PR branch, runs `pnpm install --frozen-lockfile` from PR-controlled `pnpm-lock.yaml`/`package.json` (`infra.yml:69-82`), authenticates to Azure with `secrets.ARM_CLIENT_ID`/`ARM_CLIENT_SECRET`/... (`infra.yml:84-90`), fetches the production `vercel-token` Key Vault secret via a raw `az keyvault secret show` call inside the workflow itself (`infra.yml:92-96` — this bypasses `infra/src/keyvault.ts` entirely), then runs `pulumi/actions@...` "Pulumi Preview" (`infra.yml:98-116`) with `ARM_*`, `VERCEL_API_TOKEN`, `PULUMI_CONFIG_PASSPHRASE`, and `AZURE_STORAGE_KEY` all exported as step env against the PR's own checked-out `infra/` Pulumi program. CIB-119 (PR #3086, merged) only edits `infra/src/*.ts` stack gating and `infra/scripts/admin-key-manage.mjs`; it never touches this workflow file (confirmed via `git show edb8895fb --stat` — no `infra.yml` in the diff), so this ordering gap is unaddressed on main.
- **Expected Outcome:** The PR preview path no longer exposes Azure Key Vault or Pulumi secrets to PR-controlled `infra/` code; the Vercel token fetch is either removed from the raw workflow step or routed through the already-gated `infra/src/keyvault.ts` path so untrusted-stack previews get the `<untrusted-stack-secret:name>` marker instead of a live secret.
- **Files:** `.github/workflows/infra.yml`.
- **Validation:** A workflow dry-run (or targeted Pulumi preview test) showing the preview job's Pulumi Preview step no longer receives a live Vercel/Key-Vault token when run against the dev/preview stack, plus static review confirming no `az keyvault secret show` call reads the production vault outside the trusted path.
- **Identified From:** Split from CIB-110 (deepsec sweep 20260629190245); decomposition readiness pass 2026-07-02.
- **Coordinates with:** CIB-119 (infra/src stack-trust gating, PR #3086, merged — its own "Coordinates with" note already defers workflow-side secret exposure to CIB-110; this item is that deferred half); CIB-139 (release/signing tag trust, related but separate secrets path).
- **Confidence:** high — the exposure is visible directly in the current workflow YAML, and no fork check protects internal (non-fork) PR branches from it.

### CIB-137: Make required-check classification tamper-resistant to PR-controlled code

- **Status:** Ready
- **Intent:** `detect-changes`'s classification is computed entirely from code checked out at the PR's own head (`ci.yml:52-59` checks out the default `pull_request` ref, then `uses: ./.github/actions/detect-changes`, itself PR-controlled). The composite action shells out to `scripts/ci/classify-changes.sh` (`.github/actions/detect-changes/action.yml:182`) and derives every `*-required` gate (`lint-required`, `typecheck-required`, `unit-tests-required`, `dependency-audit-required`, etc., `action.yml:227-272`) purely from that PR-editable script's output. A PR that edits `.github/actions/detect-changes/action.yml` or `scripts/ci/classify-changes.sh` to always report `docs-only`/no required checks makes `ci.yml`'s per-job skip conditions (e.g. `ci.yml:449-457`, `ci.yml:529-536`) short-circuit to a passing filler conclusion for every required-check name, without lint/typecheck/tests/dependency-audit ever running on that same PR.
- **Expected Outcome:** The composite action and script that decide which required checks run are resolved from a trusted ref (PR base SHA or `main`) rather than the PR's own head, so PR-controlled edits to the classifier cannot suppress required checks on that same PR.
- **Files:** `.github/actions/detect-changes/action.yml`, `scripts/ci/classify-changes.sh`.
- **Validation:** A test PR that edits the classifier to force `docs-only=true` while touching source files still runs the full required-check set (or the workflow fails closed), verified via a dry-run/fixture harness plus static review of the classifier's checkout ref.
- **Identified From:** Split from CIB-110 (deepsec sweep 20260629190245); decomposition readiness pass 2026-07-02.
- **Coordinates with:** CIB-038 (Lint & Format / Type Check filler-job consolidation, which consumes this classifier's outputs); CIB-136/CIB-138/CIB-139 siblings.
- **Confidence:** high — the spoof path is a direct read of currently-committed workflow logic, not a hypothetical.

### CIB-138: Restrict bench-nightly self-hosted runner to trusted refs

- **Status:** Ready
- **Intent:** `bench-nightly.yml` runs on a dedicated self-hosted runner (`runs-on: [self-hosted, bench]`, line 42) and is triggered only by `workflow_dispatch` (lines 20-26) with no ref restriction — unlike `mirror-eddacraft-tui.yml` and `mirror-drift-check.yml`, which already carry a "Guard — only mirror/check from refs/heads/main" step rejecting any other `GITHUB_REF` before doing anything privileged (both confirmed present on `origin/main`, and both run on `ubuntu-latest`, not self-hosted, so they are already fine and out of scope here). A dispatch against any branch runs that branch's own `bench-nightly.yml` content (including `cargo build`/`cargo run -p anvil-bench --release`) directly on the self-hosted box with no equivalent guard.
- **Expected Outcome:** `bench-nightly.yml` refuses to run unless dispatched against `refs/heads/main` (or another explicitly trusted ref), mirroring the guard pattern already proven in the two mirror workflows.
- **Files:** `.github/workflows/bench-nightly.yml`.
- **Validation:** A `workflow_dispatch` dry-run against a non-main branch fails fast at the new guard step before the self-hosted runner executes any build/run step; static review confirming the guard matches the mirror-workflow pattern.
- **Identified From:** Split from CIB-110 (deepsec sweep 20260629190245); decomposition readiness pass 2026-07-02.
- **Coordinates with:** existing ref guards in `mirror-eddacraft-tui.yml` / `mirror-drift-check.yml` (precedent, already merged — no change needed there).
- **Confidence:** high — small, mechanical, directly modelled on an existing merged pattern.

### CIB-139: Verify release tag ancestry before exercising signing/publish authority

- **Status:** Proposed — needs an owner decision on what counts as a "trusted" ancestor ref for hotfix/backport tags that may legitimately not descend from the current `main` tip (see the repo's hotfix-backport-risk precedent) before the ancestry check can be written without breaking a valid release path.
- **Intent:** `release.yml` triggers on any pushed tag matching `**[0-9]+.[0-9]+.[0-9]+*` (`release.yml:49-50`) and its `plan` job runs `dist host --steps=create --tag=${{ github.ref_name }}` (`release.yml:122`) using `secrets.GITHUB_TOKEN`; `release-sign-artefacts.yml` triggers on `release: published` or an operator-supplied `workflow_dispatch` `tag` input (`release-sign-artefacts.yml:10-17`) and resolves the commit to sign purely from `github.event.release.target_commitish` / `gh release view $TAG --json targetCommitish` (`release-sign-artefacts.yml:98-100`) before materialising the private minisign key (`release-sign-artefacts.yml:78`). Neither workflow checks that the tagged commit is reachable from `main` before exercising release-creation or signing authority. CIB-120 (Merged, PR #3077) pinned the `rsign2`/`rustup` installer supply chain inside these same two files but added no ancestry check — confirmed present-and-unchanged on `origin/main` (`release-sign-artefacts.yml:69` now `cargo install rsign2 --version =0.6.6 --locked`; `release.yml` rustup step now pinned/checksummed) — so the tag-trust gap is a distinct, still-open concern in the same files.
- **Expected Outcome:** Both workflows verify (e.g. `git merge-base --is-ancestor <tag-commit> origin/main`, or an equivalent trusted-ref check per the owner decision above) that the tag/release target commit is an ancestor of a trusted branch before running `dist host --steps=create` or materialising the signing key, and fail closed with a clear error if not.
- **Files:** `.github/workflows/release.yml`, `.github/workflows/release-sign-artefacts.yml`.
- **Validation:** A dry-run against a synthetic tag pointing at a commit not reachable from the trusted ref is rejected by both workflows before any publish/signing step runs; existing legitimate release tags continue to pass.
- **Identified From:** Split from CIB-110 (deepsec sweep 20260629190245); decomposition readiness pass 2026-07-02.
- **Coordinates with:** CIB-120 (rsign2/rustup pinning, Merged via PR #3077 — same two files, adjacent but distinct supply-chain vs. tag-trust concerns); ADR-045 signing key-custody runbook.
- **Confidence:** medium — the exposure is real and evidenced, but the fix shape depends on the pending trusted-ref decision, not just mechanics.

### CIB-140: Key API rate limiting on a trusted client-identity signal

- **Status:** Ready
- **Intent:** Stop the per-IP rate limiter from being keyed on a header a
  client can set directly, which lets an attacker evade or frame another
  client's limit.
- **Expected Outcome:** `rateLimiter`/`globalRateLimiter` key on the client
  identity the Vercel edge itself establishes (the edge-appended hop, or a
  platform header Vercel overwrites rather than merely appends to) instead of
  blindly trusting the leftmost, client-suppliable entry in
  `X-Forwarded-For`. The trust boundary and expected hop count are documented
  inline so a future change to the header source doesn't silently regress
  back to trusting client input.
- **Files:** `apps/anvil-api/src/middleware/rate-limit.ts`,
  `apps/anvil-api/src/index.ts`.
- **Validation:** A test proving a request that sets its own
  `X-Forwarded-For` value (e.g. `evil, 203.0.113.7` to imitate a two-hop
  chain) is keyed on the platform-trusted IP, not the attacker-chosen prefix;
  existing rate-limit unit tests updated to cover multi-hop `X-Forwarded-For`
  fixtures.
- **Identified From:** Split from CIB-111 (deepsec sweep 20260629190245);
  decomposition readiness pass 2026-07-02. Re-verified live at
  `apps/anvil-api/src/middleware/rate-limit.ts:32` — still keys on
  `.split(',')[0]` with no trusted-proxy configuration anywhere in the app.
- **Coordinates with:** CIB-142 (waitlist throttle), which currently relies
  on this same spoofable key for its baseline protection.
- **Confidence:** high — localised middleware fix; Vercel's edge-proxy
  header semantics are documented, no product decision needed.

### CIB-141: Fail-closed entitlement default and atomic refresh-token rotation

- **Status:** Proposed
- **Intent:** Close two related gaps in `apps/anvil-api`'s licence/session
  issuance: an entitlement lookup that fails open by design, and a
  refresh-token rotation that isn't fully atomic.
- **Expected Outcome:** `findActiveScopesForUser` (`queries.ts:174-204`) no
  longer folds "user never issued a token" and "user's tokens were all
  revoked/expired" into the same `['beta']` fallback — a decision is needed
  on how to distinguish them (see Open Question). Separately, and
  independent of that decision: `consumeRefreshToken` and the replacement
  `insertRefreshToken` in `/session/refresh`
  (`apps/anvil-api/src/routes/auth-session.ts:73-87`) are wrapped in one
  transaction, matching the pattern already used by
  `revokeRefreshFamilyAndAccessTokensForUser`, so a partial failure can't
  leave a consumed token with no replacement.
- **Open Question (product decision):** Is defaulting an active, zero-token
  user to `['beta']` the intended self-signup entry point (as the existing
  code comment states), and if so, what signal should distinguish that from
  a user whose access was explicitly revoked and should get `[]`/deny
  instead of being silently re-granted `beta`? Answering this determines
  whether a schema change (e.g. an explicit `revoked`/lifecycle marker) is
  needed, or whether the current behaviour is correct as documented and this
  half of the item should be closed as a false positive.
- **Files:** `apps/anvil-api/src/db/queries.ts`,
  `apps/anvil-api/src/routes/auth-session.ts`,
  `apps/anvil-api/src/lib/session.ts`.
- **Validation:** Tests proving (1) an active user with zero access_token
  rows resolves to the agreed default per the decision above, distinct from
  a revoked user; (2) two concurrent `/session/refresh` calls against the
  same valid refresh token — the loser gets 401 and the winner's consume +
  new-token insert commit or roll back together, never partially.
- **Identified From:** Split from CIB-111 (deepsec sweep 20260629190245);
  decomposition readiness pass 2026-07-02. Re-verified: family-revocation
  (`revokeRefreshFamilyAndAccessTokensForUser`, `queries.ts:566-584`) was
  already transactional pre-dating the sweep — narrowed scope to the
  consume+insert pairing and the entitlement default.
- **Coordinates with:** CIB-143 (docs-shell also reads `scopes`/`tier` off
  the same licence this item mints).
- **Confidence:** medium — the atomicity fix is mechanical; the entitlement
  half is gated on a product decision named above.

### CIB-142: Atomic OTP attempt limiting and a dedicated waitlist abuse throttle

- **Status:** Ready
- **Intent:** Remove the check-then-increment race on OTP verification
  attempts and add a dedicated throttle to the public waitlist signup
  endpoint, which today relies only on the shared (and spoofable) global
  rate limiter.
- **Expected Outcome:** `POST /auth/otp/verify` enforces `MAX_ATTEMPTS` with
  an atomic conditional update (e.g. `UPDATE ... WHERE attempts < $max
  RETURNING attempts`, evaluated before the code comparison) so N concurrent
  guesses against the same code cannot all read a stale attempts count and
  proceed past the cap. `POST /waitlist` gets a per-email and/or per-key
  throttle independent of the global limiter, closing the email-bombing /
  signup-abuse gap.
- **Files:** `apps/anvil-api/src/routes/auth-otp.ts`,
  `apps/anvil-api/src/routes/waitlist.ts`, `apps/anvil-api/src/db/queries.ts`
  (attempt-increment query).
- **Validation:** A test firing concurrent `/auth/otp/verify` requests
  against the same active code proves at most `MAX_ATTEMPTS` guesses are
  ever evaluated; a test proving repeated `/waitlist` submissions for the
  same email are throttled independent of source IP.
- **Identified From:** Split from CIB-111 (deepsec sweep 20260629190245);
  decomposition readiness pass 2026-07-02. Re-verified live at
  `apps/anvil-api/src/routes/auth-otp.ts:111-128` (stale-snapshot check
  before increment) and `apps/anvil-api/src/routes/waitlist.ts:13` (no
  per-email throttle).
- **Coordinates with:** CIB-140 (shares/depends on the rate-limit primitive
  this item's waitlist throttle sits alongside).
- **Confidence:** high — self-contained query/route fixes, no product
  decision needed.

### CIB-143: Scope-based docs entitlement and callback abuse throttling

- **Status:** Proposed
- **Intent:** Stop `apps/docs-shell` from granting private-docs access to
  any authenticated licence regardless of actual entitlement, and add abuse
  throttling to the OAuth callback route, which currently has none of its
  own.
- **Expected Outcome:** `apps/docs-shell/lib/jwt.ts`'s `verifyLicense` checks
  an actual entitlement signal in `scopes` rather than `tier` (`tier` is
  hardcoded to `'pro'` for every licence at mint time — see CIB-141's
  `lib/session.ts:67` — so the current tier check is always true and
  entitlement-blind). `apps/docs-shell/app/auth/callback/route.ts` gets a
  throttle on repeated callback attempts, independent of `apps/anvil-api`'s
  own limits, to bound abuse of the upstream GitHub exchange and DB writes
  it triggers.
- **Open Question (product decision):** What scope value (or set) denotes
  "entitled to private docs"? Both `apps/docs-shell/lib/jwt.ts` (the check)
  and `apps/anvil-api/src/lib/session.ts` (`tier` hardcoding — shared file
  with CIB-141) need to agree on this before either can be safely fixed;
  this JS/TS surface (`anvil-api`, `docs-shell`) is a **live product
  surface**, not the retiring `packages/` tree, so the fix should land as a
  real product decision, not a stopgap.
- **Files:** `apps/docs-shell/lib/jwt.ts` (corrects the original split's file
  list — this is where the vacuous check actually lives, not
  `proxy.ts`/`route.ts` directly), `apps/docs-shell/app/auth/callback/route.ts`,
  `apps/docs-shell/proxy.ts`.
- **Validation:** A test proving a licence with `scopes: ['beta']` and no
  docs entitlement is rejected by `proxy.ts`'s `/anvil/*` gate even though
  `tier` is `'pro'`; a test proving repeated `/auth/callback` requests from
  the same source are throttled.
- **Identified From:** Split from CIB-111 (deepsec sweep 20260629190245);
  decomposition readiness pass 2026-07-02. Re-verified live at
  `apps/docs-shell/lib/jwt.ts:4,28` and `apps/anvil-api/src/lib/session.ts:67`.
- **Coordinates with:** CIB-141 (shares the licence-minting file and the
  `scopes` semantics this item's entitlement check depends on).
- **Confidence:** medium — the throttling half is mechanical; the
  entitlement half is gated on the product decision named above.

### CIB-144: Flip requires_auth for anvil_fix/anvil_suppress/anvil_gate MCP tools

- **Status:** Ready
- **Intent:** Close the RMCPF-011-deferred auth gap — mutating (`anvil_fix`,
  `anvil_suppress`) and execution-triggering (`anvil_gate`) MCP tools currently
  register as `requires_auth: false`, unlike `anvil_validate_write` /
  `anvil_apply_patch`.
- **Expected Outcome:** `fix`, `suppress`, and `gate` flip to
  `requires_auth: true` in the tool registry; an unauthenticated MCP session
  calling any of the three receives the existing `gatewayUnavailable` /
  `allow-with-warning` auth-required envelope instead of the tool executing.
  **Blast radius (must be documented in the PR):** the enforcement path
  already exists (`tools_call_response` in `commands/mcp.rs`) and fails open
  with a structured warning, not a hard error, so no client crashes — but any
  unauthenticated caller that relies on `anvil_fix`/`anvil_suppress` actually
  mutating files, or `anvil_gate` actually running, will silently stop getting
  that side effect until `anvil auth login` succeeds. `ANVIL_DEV=1` /
  local dev-bypass sessions are unaffected (`mcp_tool_auth_ok` short-circuits
  true under `cli_dev_bypass_active()`). No live documentation promises
  unauthenticated use of these three tools — `docs/public/anvil/integrations/mcp.md`
  only describes them under the "Frozen legacy Node MCP catalogue" section,
  explicitly marked non-authoritative for the Rust shim.
- **Files:** `crates/anvil-cli/src/mcp/tools/registry.rs` (flip the three
  `requires_auth` fields, update/remove the stale RMCPF-011-deferral comment
  at `registry.rs:73-77`), `crates/anvil-cli/src/commands/mcp.rs` (add
  regression tests).
- **Validation:** Unit test on `registry::all()` asserting `fix`/`suppress`/
  `gate` are `requires_auth: true`; integration test in `mcp.rs` proving an
  unauthenticated `tools/call` for each of the three returns the
  `auth-required` envelope without invoking the underlying tool's side effect
  (no file write, no subprocess spawn); existing dev-bypass and edict-cache
  tests continue to pass unchanged.
- **Identified From:** Split from CIB-112 (deepsec sweep 20260629190245);
  decomposition readiness pass 2026-07-02.
- **Coordinates with:** RMCPF-011 disposition (`plans/modules/rust-mcp-full-port.aps.md:311`),
  MCP auth model.
- **Confidence:** high — mechanical flag flip with an enforcement path
  already proven in production; the only risk is under-communicating the
  behavioural change to existing unauthenticated integrations.

### CIB-145: Harden anvil_fix against symlink-swap TOCTOU races

- **Status:** Ready
- **Intent:** `anvil_fix`'s canonicalise-then-write sequence has a
  check-then-use window an attacker with concurrent filesystem access could
  exploit to redirect a write outside the workspace.
- **Expected Outcome:** `anvil_fix` (and, for parity, `anvil_suppress`, which
  shares the same pattern) no longer trusts a path resolved at check time for
  a write that happens later — either by re-verifying containment against an
  open file handle/inode rather than a path string, or by an equivalent
  fail-closed mitigation; at minimum, `fix.rs` gains the symlink-escape test
  `suppress.rs` already has, closing the current test-parity gap.
- **Files:** `crates/anvil-cli/src/mcp/tools/fix.rs` (`canonicalise_inside_workspace`
  at `fix.rs:147`, `apply_fix` write at `fix.rs:381`), `crates/anvil-cli/src/mcp/tools/suppress.rs`
  (apply the same hardening for consistency).
- **Validation:** New test proving a symlink swapped between the
  canonicalize check and the write (or a symlink pointing outside the
  workspace present at check time, matching `suppress.rs:635`) is rejected,
  not silently followed; existing fix/suppress functional tests stay green.
- **Identified From:** Split from CIB-112 (deepsec sweep 20260629190245);
  decomposition readiness pass 2026-07-02.
- **Coordinates with:** `anvil_suppress` path-containment contract (shares
  the same design).
- **Confidence:** medium — the fix-on-read/write pattern is well understood,
  but a true fd-based re-check may need a small refactor of `apply_fix`'s
  read/write split; a pure test-parity fix alone would only close the
  check-time case, not the race.

### CIB-146: Decide and implement anvil_gate script-execution consent policy

- **Status:** Proposed
- **Intent:** `anvil_gate`'s full-mode path executes project-controlled
  `package.json` scripts (`pnpm lint:check` at `crates/anvil-cli/src/commands/gate.rs:540-541`,
  `pnpm test` at `gate.rs:574-575`) with no consent gate, letting an
  unauthenticated (pre-CIB-144) or even authenticated MCP session trigger
  arbitrary workspace-defined code merely by calling `anvil_gate` without
  `targetFiles`.
- **Product question requiring an explicit decision before implementation:**
  *Should `anvil_gate`'s full-mode (config-driven) run be allowed to execute
  project-controlled lint/test/coverage scripts at all from an MCP session,
  or should it require explicit authenticated consent (e.g. a
  session-scoped opt-in flag, a confirmation round-trip, or restricting MCP
  `anvil_gate` to the planless/antipattern-only mode and pushing full-mode
  gate runs to the CLI-only surface)?* This cannot be resolved by engineering
  judgement alone — it trades off "gate parity with the CLI" against
  "MCP sessions should not have RCE-equivalent authority over an untrusted
  workspace," and needs owner/security sign-off.
- **Expected Outcome:** Once the policy is decided, `anvil_gate`'s full-mode
  path enforces it (e.g. `requires_auth: true` is necessary but not
  sufficient — CIB-144 alone does not close this, since an authenticated
  session would still trigger the scripts unconditionally).
- **Files:** `crates/anvil-cli/src/mcp/tools/gate.rs`, `crates/anvil-cli/src/commands/gate.rs`
  (`run_check_lint`, `run_check_test`, `run_full_gate`).
- **Validation:** TBD pending the design decision; must include a test
  proving the chosen consent gate blocks full-mode gate execution absent the
  required signal, and a test proving planless mode (antipattern-only, no
  subprocess) is unaffected.
- **Identified From:** Split from CIB-112 (deepsec sweep 20260629190245);
  decomposition readiness pass 2026-07-02.
- **Coordinates with:** CIB-144 (auth flip is a prerequisite, not a
  substitute), gate-config subsystem.
- **Confidence:** low until the product question above is answered —
  correctly scoped as Proposed, not Ready.

### CIB-147: Re-scope fence-unblock authority against the settled MLP2-026 trust model

- **Status:** Proposed
- **Intent:** CIB-112 assumed fence-unblock authority is "enforced only by
  the CLI wrapper." Re-verification shows this is not quite accurate for
  Unix: the daemon protocol's real authority gate is Unix socket file
  permissions set at bind time (`crates/anvil-intercept/src/ipc.rs:1103,1108`
  — 0600 socket / 0700 directory, owner-checked at `ipc.rs:771-847`), and
  `--acknowledge-cascade` is explicitly documented as UX-only
  (`crates/anvil-intercept-proto/src/lib.rs:148`) per a **settled v1 design
  decision** (`plans/specs/2026-05-16-mlp2-026-fence-cascade-control-lane.md:513-515`:
  "same-UID peers can issue any IPC command... intentionally permissive...
  out of scope for v1").
- **Product question requiring a decision:** *Does the existing same-UID
  trust-zone model (any local peer owning the socket may clear any fence or
  cascade) remain acceptable, or should `UnblockCascade`/`UnblockWorktree`
  gain a stricter per-operator authorisation check (the ratchet path the
  MLP2-026 spec already sketches — a `requires_ack` flag tied to
  `OperatorContext.uid`)?* Re-opening this duplicates a decision already made
  once; do not implement a "fix" without an explicit new sign-off overriding
  MLP2-026 §5.5.
- **Expected Outcome:** Either (a) the trust model is reaffirmed as-is and
  this item closes as a documentation/regression-test task confirming socket
  permission enforcement is regression-pinned, or (b) a new decision record
  authorises the stricter per-operator gate and this item implements it.
- **Files:** `crates/anvil-intercept/src/ipc.rs` (`dispatch_command` for
  `unblock-cascade`/`unblock-worktree`, `ipc.rs:4053-4073`),
  `crates/anvil-cli/src/commands/intercept.rs`.
- **Validation:** At minimum, a regression test pinning that a
  non-owner-permission socket connection cannot reach `dispatch_command` at
  all (permission enforcement, not per-command); if the stricter model is
  chosen, tests proving `UnblockCascade`/`UnblockWorktree` fail without a
  valid operator/ack context.
- **Identified From:** Split from CIB-112 (deepsec sweep 20260629190245);
  decomposition readiness pass 2026-07-02.
- **Coordinates with:** CIB-113 (session-owner-peer binding — check for
  overlap before scoping work), CIB-114 (Windows named-pipe peer
  authentication owns the equivalent gap on Windows), MLP2-026 spec.
- **Confidence:** low — this child needs a decision-log entry before any
  code changes; treat the parent's "wire-protocol change" framing as
  provisional pending that decision.

### CIB-148: Normalise source/target paths in anvil_query_boundary before policy evaluation

- **Status:** Ready
- **Intent:** `anvil_query_boundary` matches `sourceFile`/`targetFile`
  directly against layer glob patterns with only an emptiness check
  (`crates/anvil-cli/src/mcp/tools/query_boundary.rs:86-100`); unassigned
  layers fail open to `"allowed": true` (`query_boundary.rs:167-179`), so a
  path written as `./src/x.ts`, `src//x.ts`, or with backslash separators
  silently fails to match and bypasses boundary policy instead of being
  rejected or correctly evaluated.
- **Expected Outcome:** `sourceFile`/`targetFile` are normalised (collapse
  `./`, redundant separators, and reject `..`/absolute paths, matching the
  containment checks `fix.rs`/`suppress.rs` already apply) before being
  passed to `match_layer`, so representation tricks cannot cause a
  false "unassigned-layer, allowed by default" verdict.
- **Files:** `crates/anvil-cli/src/mcp/tools/query_boundary.rs`
  (`query_payload:86-100`, `match_layer:247-250`).
- **Validation:** Tests proving `./src/x.ts` and `src//x.ts` resolve to the
  same layer as `src/x.ts`; a test proving `..`-escaping or absolute inputs
  are rejected rather than silently unassigned; existing boundary-resolution
  tests (same-layer, cross-layer violation, no-baseline) stay green.
- **Identified From:** Split from CIB-112 (deepsec sweep 20260629190245);
  decomposition readiness pass 2026-07-02.
- **Coordinates with:** `anvil-architecture` validator (`assign_layers`/
  `matches_layer`), shared path-validation helpers in
  `crates/anvil-cli/src/mcp/tools/shared.rs`.
- **Confidence:** high — read-only, no filesystem mutation, no daemon
  coordination; a contained string-normalisation fix.

### CIB-149: Stop treating an unverified first wire root as the confinement primary

- **Status:** Ready
- **Intent:** Close the Allowlist-confinement bypass where a same-uid client's
  first self-declared `workspace_root` becomes the connection's implicitly
  admitted primary root, regardless of the operator's allow list.
- **Expected Outcome:** In `Allowlist` mode, a connection's implicitly admitted
  primary root is derived from a daemon-verified source (e.g. the
  `RegisterSession` worktree already bound to the authenticated peer), not from
  the first arbitrary `workspace_root` a later wire request happens to name;
  an unverified root is refused unless it independently matches an operator
  allow entry.
- **Files:** `crates/anvil-intercept/src/{confinement.rs,save_time.rs}`.
- **Validation:** Daemon test proving that, in `Allowlist` mode, a connection
  cannot get an unlisted root implicitly admitted merely by naming it first in
  a `validate_paths` call; existing `primary_root_implicitly_admitted` and
  `allowlist_refuses_unlisted` tests continue to pass with the tightened
  source.
- **Identified From:** Split from CIB-113 (deepsec sweep 20260629190245);
  decomposition readiness pass 2026-07-02.
- **Coordinates with:** DSV-008 confinement CLI (`anvil workspace`), ADR-061
  §7 same-uid trust boundary, `save_time.rs::authorise_root`.
- **Confidence:** high — root cause and fix are mechanically clear; the only
  risk is over-tightening legitimate first-touch usage in `Open` mode (which
  this item does not change).

### CIB-150: Verify the wire `agent_tag` claim before honouring durable membership

- **Status:** Ready
- **Intent:** Close the trust-boundary gap where any same-uid IPC client can
  mint an `AgentTag` claiming `claimed_agent_id: "activation-spine"` and be
  treated as durable worktree membership, bypassing the live per-worktree cap
  and consuming the separate registered-worktree budget.
- **Expected Outcome:** A `RegisterSession` claiming durable
  (activation-spine) membership is only honoured when the connection's
  authenticated peer is independently authorised to register durable
  membership (mirroring the `verify_lineage_claim` peer-derivation pattern
  already applied to `lineage`); an unauthorised claim is downgraded to an
  ordinary (non-durable, capped, TTL-bound) session rather than rejected
  outright, so a benign mis-tagged client still registers.
- **Files:** `crates/anvil-intercept/src/{registry.rs,ipc.rs}`.
- **Validation:** Daemon tests proving an unauthorised same-uid peer's
  `activation-spine` claim is downgraded to a live (capped) session, a
  legitimately authorised caller's durable claim still succeeds, and the
  `registered_worktree_cap` cannot be exhausted by forged durable claims.
- **Identified From:** Split from CIB-113 (deepsec sweep 20260629190245);
  decomposition readiness pass 2026-07-02.
- **Coordinates with:** MLP2-070/MLP2-074 lineage-verification pattern
  (`verify_lineage_claim`, `verify_report_process_starttime`), ACTMO-014/019
  durable-registration state.
- **Confidence:** high — the fix mirrors an existing, tested verification
  pattern in the same file; the main design decision is the safe downgrade
  behaviour rather than a hard reject.

### CIB-151: Verify on-disk state before trusting a wire-declared delete/rename

- **Status:** Ready
- **Intent:** Close the bypass where a client's self-declared `ChangeKindWire`
  (`Deleted`/`Renamed`) suppresses the guarded read and antipattern content
  scan even when the path still holds live, unscanned bytes on disk.
- **Expected Outcome:** `validate_paths` no longer gates the guarded read and
  antipattern scan purely on the client's claimed change kind; a path that
  still resolves to readable content is read and scanned regardless of the
  claimed kind, and only an actually-vanished path is treated as
  content-free.
- **Files:** `crates/anvil-intercept/src/validate_paths.rs`.
- **Validation:** Daemon test proving a path declared `Deleted`/`Renamed` but
  still present with live bytes on disk is still guarded-read and
  antipattern-scanned (and cannot silently evade a blocking finding);
  existing `change_has_bytes`/`per_path_outcome` coverage-cap and taxonomy
  tests continue to pass.
- **Identified From:** Split from CIB-113 (deepsec sweep 20260629190245);
  decomposition readiness pass 2026-07-02.
- **Coordinates with:** DSV-005/006 save-time verdict assembly, the
  antipattern check family, `CertifyStale` taxonomy.
- **Confidence:** high — the bypass is a single conditional
  (`change_has_bytes`); the main risk is the added read cost for legitimate
  delete/rename traffic, which is bounded by the existing parse-size cap.

### CIB-152: Serialise the fence store's load-mutate-save cycle

- **Status:** Draft
- **Intent:** Close the lost-update race where two concurrent fence engages
  (or an engage racing an unblock/clear-cascade) can each `load()` the same
  on-disk `FenceState`, mutate independently, and `save()` — silently
  dropping one caller's security-relevant change.
- **Expected Outcome:** `FenceStore::fence_worktree`, `unblock_worktree`, and
  `clear_cascade` execute their load-mutate-save sequence under a single
  serialising lock per store instance, so no concurrent pair of calls can
  lose a persisted fence or cascade record; the existing atomic
  temp-then-rename write (`store_io.rs`) is unchanged.
- **Files:** `crates/anvil-intercept/src/fence.rs`.
- **Validation:** A concurrency test proving two simultaneous
  `fence_worktree`/`unblock_worktree`/`clear_cascade` calls against the same
  store never lose either caller's record; existing single-threaded fence
  tests continue to pass unmodified.
- **Identified From:** Split from CIB-113 (deepsec sweep 20260629190245);
  decomposition readiness pass 2026-07-02.
- **Coordinates with:** MLP2-026 fence-cascade control lane, `store_io.rs`
  atomic-write primitives. **Depends on deterministic concurrent-write test
  support**: `tests/` has a live in-process daemon harness
  (`daemon_config_wired.rs::spawn_daemon_with_config`) but no
  interleave-control/race-injection scaffolding for this class of race; the
  regression test needs either a barrier-based in-crate unit test seam or
  new harness support before this can be verified deterministically rather
  than by timing-dependent flakiness.
- **Confidence:** medium — the fix (a mutex around the critical section) is
  mechanically simple, but proving the fix deterministically (not by a
  flaky sleep-based race) needs the harness support noted above.

### CIB-153: Bind session lifecycle operations to the registering peer

- **Status:** Draft
- **Intent:** Close the gap where any same-uid IPC client that knows or
  guesses another session's id can heartbeat-keep-alive or force-unregister
  a session it never registered, since `Heartbeat`/`UnregisterSession` carry
  no peer-credential check today.
- **Expected Outcome:** A session records the authenticated peer identity
  (uid/pid/starttime) that registered it; `Heartbeat` and `UnregisterSession`
  are rejected when the calling peer does not match the recorded owner
  (mirroring the existing `ReportProcess` peer-pid contract), independent of
  the separate telemetry-subscriber binding.
- **Files:** `crates/anvil-intercept/src/{registry.rs,ipc.rs}`.
- **Validation:** Dispatch-level tests (mirroring the existing
  `dispatch_command_register_lineage_*` injected-`peer_pid` pattern) proving
  a session registered under peer A's credentials rejects `Heartbeat`/
  `UnregisterSession` from a different injected peer B, and still accepts
  them from peer A; legacy/no-peer-credential paths continue to fail closed.
- **Identified From:** Split from CIB-113 (deepsec sweep 20260629190245);
  decomposition readiness pass 2026-07-02.
- **Coordinates with:** MLP2-070/MLP2-074 peer-credential verification
  pattern (`verify_lineage_claim`, `mint_subscriber_id`); CIB-114
  Windows-peer authentication (sibling, different platform layer).
  **Depends on deterministic multi-peer test coverage**: the existing
  `dispatch_command` unit tests already inject synthetic `peer_pid` values
  without a live second process, which is sufficient for the dispatch-level
  contract above but does not prove true cross-process denial end-to-end;
  no multi-process (distinct real-PID) daemon harness exists in `tests/`
  today for that stronger proof.
- **Confidence:** medium — the dispatch-level fix and its test are
  low-risk and reuse an existing pattern, but the end-to-end
  (real-process, real `SO_PEERCRED`) proof is gated on harness work not yet
  in place.

### CIB-154: Cap the number of workspace roots a connection may admit

- **Status:** Ready
- **Intent:** Close the unbounded per-connection resource vector where
  `Open`-mode admission holds one real file descriptor
  (`WorkspaceAnchor`/`OwnedFd`) per distinct admitted root with no ceiling,
  letting a same-uid peer exhaust the daemon's descriptor table by naming
  many distinct roots.
- **Expected Outcome:** `AdmittedRoots` enforces a per-connection budget on
  the number of distinct admitted roots (a new `DoS`-family cap alongside
  the existing connection/RPS/frame budgets); a connection past the budget
  is refused further admission with a structured error rather than allowed
  to keep opening anchors, in both `Open` and `Allowlist` modes.
- **Files:** `crates/anvil-intercept/src/{workspace_admission.rs,dos.rs}`.
- **Validation:** Test proving a connection that admits more than the
  budgeted number of distinct roots is refused on the next admission (not
  crashed or silently unbounded), while staying within budget continues to
  admit normally; existing admission tests (`root_set_grows_on_first_touch`,
  allowlist refusal) continue to pass.
- **Identified From:** Split from CIB-113 (deepsec sweep 20260629190245);
  decomposition readiness pass 2026-07-02.
- **Coordinates with:** `dos.rs` `IpcLimits`/`DosCaps` budget model
  (already stricter-wins project/user merge for the sibling caps),
  `workspace_anchor.rs` held-fd resource.
- **Confidence:** high — the fix is an additive counter/cap with no
  behavioural change below the new budget; the only judgement call is
  picking a default ceiling generous enough for real multi-root workflows.
